//! Compact source offsets and checked positioned reads, without mmap.
use crate::{InputError, err, limits};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    os::unix::fs::FileExt,
};

const BLOCK: usize = 64 << 10;

/// Location of an original adapter or JSONL record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRef {
    /// Source file identifier; JSONL currently uses zero.
    pub file_id: u32,
    /// Absolute byte offset.
    pub offset: u64,
    /// Record length including its newline.
    pub len: u32,
    /// One-based source line or record.
    pub line_or_record: u64,
}
/// Random access boundary shared with future engine adapters.
pub trait SourceReader {
    /// Fetch the original record and verify its source commitment.
    fn read_record(&self, reference: &SourceRef) -> Result<Vec<u8>, InputError>;
}

#[derive(Debug)]
pub(crate) struct FileSource {
    pub file: File,
    pub starts: Vec<u64>,
    pub lengths: Vec<u32>,
    blocks: Vec<blake3::Hash>,
    pub digest: String,
    pub bytes: u64,
    remove_on_drop: Option<std::path::PathBuf>,
}
impl Drop for FileSource {
    fn drop(&mut self) {
        if let Some(path) = &self.remove_on_drop {
            let _ = std::fs::remove_file(path);
        }
    }
}
impl FileSource {
    pub fn scan(mut file: File, limits: Option<crate::Limits>) -> Result<Self, InputError> {
        file.seek(SeekFrom::Start(0))
            .map_err(|e| err(0, e.to_string()))?;
        let bytes = file.metadata().map_err(|e| err(0, e.to_string()))?.len();
        if limits.is_some_and(|l| bytes > l.max_input_bytes) {
            return Err(limits::error(
                0,
                "E-LIMIT-INPUT-BYTES",
                "input exceeds byte limit",
            ));
        }
        let mut result = Self {
            file: file.try_clone().map_err(|e| err(0, e.to_string()))?,
            starts: Vec::new(),
            lengths: Vec::new(),
            blocks: Vec::new(),
            digest: String::new(),
            bytes,
            remove_on_drop: None,
        };
        let mut block = vec![0_u8; BLOCK];
        let mut hasher = blake3::Hasher::new();
        let mut offset = 0_u64;
        let mut start = 0_u64;
        let mut last = None;
        loop {
            let mut used = 0;
            while used < BLOCK {
                let count = file
                    .read(
                        block
                            .get_mut(used..)
                            .ok_or_else(|| err(0, "invalid block"))?,
                    )
                    .map_err(|e| err(0, e.to_string()))?;
                if count == 0 {
                    break;
                }
                used += count;
            }
            if used == 0 {
                break;
            }
            let data = block.get(..used).ok_or_else(|| err(0, "invalid block"))?;
            hasher.update(data);
            result.blocks.push(blake3::hash(data));
            last = data.last().copied();
            for (index, _) in data.iter().enumerate().filter(|(_, byte)| **byte == b'\n') {
                let end = offset
                    .checked_add(u64::try_from(index).map_err(|e| err(0, e.to_string()))?)
                    .and_then(|n| n.checked_add(1))
                    .ok_or_else(|| limits::error(0, "U-LIMIT-INDEX", "offset overflow"))?;
                let length = end.saturating_sub(start);
                if limits.is_some_and(|l| length > l.max_line_bytes as u64) {
                    return Err(limits::error(
                        result.starts.len() + 1,
                        "E-LIMIT-LINE-BYTES",
                        "line exceeds byte limit",
                    ));
                }
                let length = u32::try_from(length).map_err(|_| {
                    limits::error(
                        result.starts.len() + 1,
                        "U-LIMIT-INDEX",
                        "line length exceeds u32",
                    )
                })?;
                if limits
                    .and_then(|l| l.memory_budget_bytes)
                    .is_some_and(|budget| {
                        (result.starts.len() as u64)
                            .saturating_add(1)
                            .saturating_mul(240)
                            .saturating_add(64 << 20)
                            > budget
                    })
                {
                    return Err(limits::error(
                        0,
                        "I-BUDGET-MEMORY",
                        "pre-scan allocation estimate exceeds budget",
                    ));
                }
                result.starts.push(start);
                result.lengths.push(length);
                start = end;
                if limits.is_some_and(|l| result.starts.len().saturating_sub(1) > l.max_events) {
                    return Err(limits::error(
                        result.starts.len(),
                        "U-LIMIT-EVENTS",
                        "event count exceeds limit",
                    ));
                }
            }
            offset = offset
                .checked_add(u64::try_from(used).map_err(|e| err(0, e.to_string()))?)
                .ok_or_else(|| err(0, "offset overflow"))?;
            if limits.is_some_and(|l| offset.saturating_sub(start) > l.max_line_bytes as u64) {
                return Err(limits::error(
                    result.starts.len() + 1,
                    "E-LIMIT-LINE-BYTES",
                    "line exceeds byte limit",
                ));
            }
        }
        if last != Some(b'\n') {
            return Err(limits::error(
                result.starts.len() + 1,
                "E-NEWLINE",
                "trace must end with a newline",
            ));
        }
        if offset != bytes {
            return Err(limits::error(
                0,
                "E-INPUT-CHANGED",
                "input size changed during scan",
            ));
        }
        result.digest = hasher.finalize().to_hex().to_string();
        result.starts.shrink_to_fit();
        result.lengths.shrink_to_fit();
        result
            .file
            .seek(SeekFrom::Start(0))
            .map_err(|e| err(0, e.to_string()))?;
        if limits
            .and_then(|l| l.memory_budget_bytes)
            .is_some_and(|budget| result.estimate() > budget)
        {
            return Err(limits::error(
                0,
                "I-BUDGET-MEMORY",
                "pre-scan allocation estimate exceeds budget",
            ));
        }
        Ok(result)
    }
    pub fn estimate(&self) -> u64 {
        // Conservative model before event columns are allocated, including
        // temporary chunks and checker work arrays; no source-byte retention.
        u64::try_from(self.starts.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(240)
            .saturating_add(64 << 20)
    }
    pub fn heap_bytes(&self) -> usize {
        self.starts.capacity() * 8 + self.lengths.capacity() * 4 + self.blocks.capacity() * 32
    }
    pub fn reference(&self, line: usize) -> Option<SourceRef> {
        Some(SourceRef {
            file_id: 0,
            offset: *self.starts.get(line)?,
            len: *self.lengths.get(line)?,
            line_or_record: u64::try_from(line).ok()?.checked_add(1)?,
        })
    }
    pub fn read(&self, reference: &SourceRef) -> Result<Vec<u8>, InputError> {
        let line = usize::try_from(reference.line_or_record)
            .ok()
            .and_then(|n| n.checked_sub(1))
            .ok_or_else(|| err(0, "invalid source reference"))?;
        if self.reference(line).as_ref() != Some(reference) {
            return Err(err(line + 1, "invalid source reference"));
        }
        if self
            .file
            .metadata()
            .map_err(|e| err(0, e.to_string()))?
            .len()
            != self.bytes
        {
            return Err(limits::error(
                line + 1,
                "E-INPUT-CHANGED",
                "source length changed",
            ));
        }
        let end = reference
            .offset
            .checked_add(u64::from(reference.len))
            .ok_or_else(|| err(line + 1, "invalid source range"))?;
        let first = usize::try_from(reference.offset / (BLOCK as u64))
            .map_err(|e| err(line + 1, e.to_string()))?;
        let last = usize::try_from(end.saturating_sub(1) / (BLOCK as u64))
            .map_err(|e| err(line + 1, e.to_string()))?;
        let mut out = Vec::with_capacity(reference.len as usize);
        let mut block = vec![0_u8; BLOCK];
        for index in first..=last {
            let offset = u64::try_from(index)
                .map_err(|e| err(line + 1, e.to_string()))?
                .checked_mul(BLOCK as u64)
                .ok_or_else(|| err(line + 1, "invalid block offset"))?;
            let length = usize::try_from(self.bytes.saturating_sub(offset).min(BLOCK as u64))
                .map_err(|e| err(line + 1, e.to_string()))?;
            let data = block
                .get_mut(..length)
                .ok_or_else(|| err(line + 1, "invalid block length"))?;
            self.file.read_exact_at(data, offset).map_err(|_| {
                limits::error(
                    line + 1,
                    "E-INPUT-CHANGED",
                    "source truncated during positioned read",
                )
            })?;
            if self.blocks.get(index) != Some(&blake3::hash(data)) {
                return Err(limits::error(
                    line + 1,
                    "E-INPUT-CHANGED",
                    "source block digest mismatch",
                ));
            }
            let from = usize::try_from(reference.offset.saturating_sub(offset))
                .map_err(|e| err(line + 1, e.to_string()))?;
            let to = usize::try_from(end.saturating_sub(offset).min(length as u64))
                .map_err(|e| err(line + 1, e.to_string()))?;
            out.extend_from_slice(
                data.get(from..to)
                    .ok_or_else(|| err(line + 1, "invalid record slice"))?,
            );
        }
        if out.last() != Some(&b'\n') {
            return Err(limits::error(
                line + 1,
                "E-INPUT-CHANGED",
                "record terminator changed",
            ));
        }
        Ok(out)
    }
    pub fn remove_on_drop(&mut self, path: std::path::PathBuf) {
        self.remove_on_drop = Some(path);
    }
}
