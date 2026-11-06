//! Deterministic, offline-verifiable evidence directories with verbatim claims.
#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]
use qlrs_check::{Checker, Finding, InstanceKey};
use qlrs_io::{Limits, ParsedTrace, atomic_file, decode, digest, encode};
use qlrs_reduce::Certificate;
use qlrs_reduce::Reduction;
use qlrs_schema::{AliveMask, Model, Status, SyntheticCheckpoint};
use rustix::fs::{AtFlags, FileType, Mode, OFlags};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::{collections::BTreeSet, fs, path::Path};

/// Whether an immutable observed record is asserted or supplies context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WitnessRole {
    #[doc = "Observed opening, never asserted."]
    Anchor,
    #[doc = "Target assertion."]
    Asserted,
    #[doc = "Observed supporting event."]
    Evidence,
    #[doc = "Independent expected state."]
    Synthetic,
}
/// A role and its original byte/sequence provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleEntry {
    /// Observed sequence, absent for synthetic state.
    pub original_seq: Option<u64>,
    /// Original source byte range, absent for synthetic state.
    pub source_range: Option<[usize; 2]>,
    /// Evidence role.
    pub role: WitnessRole,
}
fn roles(trace: &ParsedTrace, reduction: &Reduction) -> Result<Vec<RoleEntry>, String> {
    let mut out = Vec::new();
    for event in 0..trace.store.len() {
        if !reduction.alive.contains_event(&trace.store, event) {
            continue;
        }
        let seq = *trace.store.spine.seq.get(event).ok_or("missing sequence")?;
        let range = trace
            .line_range(event.saturating_add(1))
            .ok_or("missing source range")?;
        out.push(RoleEntry {
            original_seq: Some(seq),
            source_range: Some([range.start, range.end]),
            role: if seq == reduction.target.window.end_seq {
                WitnessRole::Asserted
            } else if Some(seq) == reduction.target.window.start_seq
                && reduction
                    .target
                    .instance_key
                    .as_ref()
                    .is_some_and(|key| key.invariant_id != qlrs_check::Invariant::I3)
            {
                WitnessRole::Anchor
            } else {
                WitnessRole::Evidence
            },
        });
    }
    if reduction.checkpoint.is_some() {
        out.push(RoleEntry {
            original_seq: None,
            source_range: None,
            role: WitnessRole::Synthetic,
        });
    }
    Ok(out)
}
/// File content commitment; paths are restricted to the v0 allowlist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileEntry {
    #[doc = "path."]
    pub path: String,
    #[doc = "blake3."]
    pub blake3: String,
    #[doc = "byte length."]
    pub byte_length: u64,
}
/// Original input commitment; the source trace is not copied into the bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceTrace {
    #[doc = "blake3."]
    pub blake3: String,
    #[doc = "byte length."]
    pub byte_length: u64,
}
/// Bundle manifest, serialized canonically with stable field order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[doc = "schema."]
    pub schema: String,
    #[doc = "qlrs version."]
    pub qlrs_version: String,
    #[doc = "git commit."]
    pub git_commit: String,
    #[doc = "rustc version."]
    pub rustc_version: String,
    #[doc = "model profile."]
    pub model_profile: Model,
    #[doc = "verdict."]
    pub verdict: Status,
    #[doc = "original trace."]
    pub original_trace: SourceTrace,
    /// State provenance, retained when the input is already a reduced trace.
    pub checkpoint_source_digest: Option<String>,
    #[doc = "instance key."]
    pub instance_key: InstanceKey,
    #[doc = "closure 1 minimal."]
    pub closure_1_minimal: bool,
    /// Actual candidate calls; absent in older bundles.
    #[serde(default, skip_serializing_if = "zero_calls")]
    pub predicate_calls: u64,
    /// Final removable closures actually tested (zero is vacuous).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_candidates_tested: Option<u64>,
    /// Reserved full-check slots, including canceled speculative work.
    #[serde(default, skip_serializing_if = "zero_calls")]
    pub full_calls: u64,
    /// Algebraic queries.
    #[serde(default, skip_serializing_if = "zero_calls")]
    pub fast_probes: u64,
    /// Exhausted search budget, when minimality has not been certified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<qlrs_reduce::LimitReason>,
    /// Present when wall-clock search limits may affect this output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub determinism: Option<String>,
    #[doc = "minimality guarantee."]
    pub minimality_guarantee: String,
    #[doc = "retained events."]
    pub retained_events: usize,
    #[doc = "files."]
    pub files: Vec<FileEntry>,
}
// serde skip_serializing_if requires a borrowed field.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn zero_calls(calls: &u64) -> bool {
    *calls == 0
}
fn minimality_text(certified: bool) -> &'static str {
    if certified {
        "closure-1-minimal; not a global minimum"
    } else {
        "best verified witness; minimality not certified"
    }
}
fn put(dir: &Path, name: &str, bytes: &[u8]) -> Result<FileEntry, String> {
    atomic_file(dir.join(name), |file| file.write_all(bytes)).map_err(|e| e.to_string())?;
    Ok(FileEntry {
        path: name.into(),
        blake3: digest(bytes),
        byte_length: u64::try_from(bytes.len()).map_err(|_| "file too large")?,
    })
}
/// Write a new directory. Refuse to overwrite any existing user directory.
pub fn write(
    dir: impl AsRef<Path>,
    trace: &ParsedTrace,
    reduction: &Reduction,
) -> Result<Manifest, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = dir.as_ref();
    if fs::symlink_metadata(dir).is_ok() {
        return Err("bundle directory already exists".into());
    }
    let parent = dir
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let temporary = parent.join(format!(
        ".qlrs.bundle.{}.{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&temporary).map_err(|e| e.to_string())?;
    let result = (|| {
        let manifest = write_directory(&temporary, trace, reduction)?;
        fs::File::open(&temporary)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        if fs::symlink_metadata(dir).is_ok() {
            return Err("bundle directory already exists".into());
        }
        rustix::fs::renameat_with(
            rustix::fs::CWD,
            &temporary,
            rustix::fs::CWD,
            dir,
            rustix::fs::RenameFlags::NOREPLACE,
        )
        .map_err(|e| e.to_string())?;
        fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(manifest)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&temporary);
    }
    result
}
fn write_directory(
    dir: &Path,
    trace: &ParsedTrace,
    reduction: &Reduction,
) -> Result<Manifest, String> {
    let mut files = Vec::new();
    let mut reduced = Vec::new();
    trace
        .write_kept(&reduction.alive, &mut reduced)
        .map_err(|e| e.to_string())?;
    files.push(put(dir, "reduced_trace.jsonl", &reduced)?);
    files.push(put(
        dir,
        "violation.json",
        &encode(&reduction.target).map_err(|e| e.to_string())?,
    )?);
    if let Some(checkpoint) = &reduction.checkpoint {
        files.push(put(
            dir,
            "synthetic_checkpoint.json",
            &encode(checkpoint).map_err(|e| e.to_string())?,
        )?);
    }
    files.push(put(
        dir,
        "minimality_certificate.json",
        &encode(&reduction.certificate).map_err(|e| e.to_string())?,
    )?);
    files.push(put(
        dir,
        "witness_roles.json",
        &encode(&roles(trace, reduction)?).map_err(|e| e.to_string())?,
    )?);
    let mut log = Vec::new();
    for entry in &reduction.steps {
        log.extend(encode(entry).map_err(|e| e.to_string())?);
    }
    files.push(put(dir, "reduction_log.jsonl", &log)?);
    let checkpoint = if reduction.checkpoint.is_some() {
        " --checkpoint synthetic_checkpoint.json"
    } else {
        ""
    };
    let reproduce = format!(
        "# Replay this evidence\n\nFrom this bundle directory, using qlrs {}:\n\n```sh\nqlrs verify reduced_trace.jsonl{checkpoint}\nqlrs bundle verify .\n```\n\nThe first command must return VIOLATION (exit 2); the second must exit 0.\nThe synthetic checkpoint, when present, is independently computed expected\nstate replacing removed history. It is not an observed engine claim.\nThis is a Mode T output witness, not an engine-input reproduction.\nMinimality: {}. Fingerprint relations are hints, not diagnoses.\n",
        env!("CARGO_PKG_VERSION"),
        minimality_text(reduction.closure_1_minimal)
    );
    files.push(put(dir, "REPRODUCE.md", reproduce.as_bytes())?);
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let manifest = Manifest {
        schema: "qlrs-bundle/v1".into(),
        qlrs_version: env!("CARGO_PKG_VERSION").into(),
        git_commit: env!("QLRS_GIT_COMMIT").into(),
        rustc_version: env!("QLRS_RUSTC_VERSION").into(),
        model_profile: trace.store.header.model.clone(),
        verdict: Status::Violation,
        original_trace: SourceTrace {
            blake3: trace.source_digest(),
            byte_length: trace.source_len(),
        },
        checkpoint_source_digest: reduction
            .checkpoint
            .as_ref()
            .map(|cp| cp.source_digest.clone()),
        instance_key: reduction
            .target
            .instance_key
            .clone()
            .ok_or("missing target key")?,
        closure_1_minimal: reduction.closure_1_minimal,
        predicate_calls: reduction.predicate_calls,
        certificate_candidates_tested: Some(reduction.certificate.candidates_tested),
        full_calls: reduction.full_calls,
        fast_probes: reduction.fast_probes,
        stop_reason: reduction.stop_reason,
        determinism: reduction.time_limited.then(|| "time-limited".into()),
        minimality_guarantee: minimality_text(reduction.closure_1_minimal).into(),
        retained_events: reduction.retained_events,
        files,
    };
    let bytes = encode(&manifest).map_err(|e| e.to_string())?;
    put(dir, "manifest.json", &bytes)?;
    atomic_file(dir.join("manifest.blake3"), |file| {
        file.write_all(format!("{}  manifest.json\n", digest(&bytes)).as_bytes())
    })
    .map_err(|e| e.to_string())?;
    verify(dir)?;
    Ok(manifest)
}
/// Rehash every byte, reject unexpected paths/files, replay and match the key.
/// Integrity detects corruption; it does not authenticate a malicious author.
#[allow(clippy::too_many_lines)]
pub fn verify(dir: impl AsRef<Path>) -> Result<Manifest, String> {
    verify_with_options(dir, false)
}
/// Verify integrity and optionally replay every certificate candidate offline.
#[allow(clippy::too_many_lines)]
pub fn verify_with_options(
    dir: impl AsRef<Path>,
    recheck_certificate: bool,
) -> Result<Manifest, String> {
    // Legacy entry keeps existing defaults until the R9 resource review closes.
    verify_with_limits(
        dir,
        recheck_certificate,
        Limits {
            max_entries: usize::MAX,
            max_entry_bytes: u64::MAX,
            ..Limits::default()
        },
    )
}
/// Verify with explicit resource limits.
pub fn verify_with_limits(
    dir: impl AsRef<Path>,
    recheck_certificate: bool,
    limits: Limits,
) -> Result<Manifest, String> {
    verify_with_observer(dir, recheck_certificate, limits, |_| {})
}
/// Observe every validated file access; used by the confinement fuzz oracle.
#[allow(clippy::too_many_lines)]
pub fn verify_with_observer(
    dir: impl AsRef<Path>,
    recheck_certificate: bool,
    limits: Limits,
    mut observe: impl FnMut(&Path),
) -> Result<Manifest, String> {
    let dir = dir.as_ref();
    if !fs::symlink_metadata(dir)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_dir()
    {
        return Err("E-BUNDLE-PATH: root must be a real directory".into());
    }
    let root = fs::File::from(
        rustix::fs::open(
            dir,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|e| format!("E-BUNDLE-PATH: {e}"))?,
    );
    let mut actual = BTreeSet::new();
    for entry in rustix::fs::Dir::read_from(&root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry
            .file_name()
            .to_str()
            .map_err(|_| "E-BUNDLE-PATH: non-UTF-8 name")?;
        if matches!(name, "." | "..") {
            continue;
        }
        if !actual.insert(name.to_owned()) || actual.len() > limits.max_entries {
            return Err("E-BUNDLE-ENTRIES: too many entries".into());
        }
        let stat = rustix::fs::statat(&root, name, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|e| e.to_string())?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
            return Err("E-BUNDLE-PATH: non-regular entry".into());
        }
        if u64::try_from(stat.st_size).map_err(|e| e.to_string())? > limits.max_entry_bytes {
            return Err("E-BUNDLE-ENTRY-BYTES: entry exceeds byte limit".into());
        }
    }
    let mut read = |name: &str| regular_file(&root, dir, name, limits, &mut observe);
    let manifest_bytes = read("manifest.json")?;
    let commitment = read("manifest.blake3")?;
    if commitment != format!("{}  manifest.json\n", digest(&manifest_bytes)).as_bytes() {
        return Err("manifest digest mismatch".into());
    }
    let manifest: Manifest = decode(&manifest_bytes).map_err(|e| e.to_string())?;
    if encode(&manifest).map_err(|e| e.to_string())? != manifest_bytes {
        return Err("manifest is not canonical".into());
    }
    if !matches!(
        manifest.schema.as_str(),
        "qlrs-bundle/v0" | "qlrs-bundle/v1"
    ) || manifest.verdict != Status::Violation
        || manifest.minimality_guarantee != minimality_text(manifest.closure_1_minimal)
        || manifest.closure_1_minimal == manifest.stop_reason.is_some()
        || manifest.original_trace.blake3.len() != 64
    {
        return Err("unsupported or invalid bundle manifest".into());
    }
    if manifest
        .determinism
        .as_deref()
        .is_some_and(|label| label != "time-limited")
    {
        return Err("invalid determinism label".into());
    }
    let allowed = [
        "REPRODUCE.md",
        "reduced_trace.jsonl",
        "reduction_log.jsonl",
        "synthetic_checkpoint.json",
        "violation.json",
        "minimality_certificate.json",
        "witness_roles.json",
    ];
    let mut names = BTreeSet::from(["manifest.json".to_owned(), "manifest.blake3".to_owned()]);
    let mut previous = "";
    for file in &manifest.files {
        if !allowed.contains(&file.path.as_str())
            || file.path.as_str() <= previous
            || !names.insert(file.path.clone())
        {
            return Err("E-BUNDLE-PATH: invalid, duplicate or unsorted bundle path".into());
        }
        previous = &file.path;
        let bytes = read(&file.path)?;
        if digest(&bytes) != file.blake3
            || u64::try_from(bytes.len()).map_err(|_| "file too large")? != file.byte_length
        {
            return Err(format!("content hash or length mismatch: {}", file.path));
        }
    }
    for required in [
        "REPRODUCE.md",
        "reduced_trace.jsonl",
        "reduction_log.jsonl",
        "violation.json",
    ] {
        if !names.contains(required) {
            return Err(format!("missing file: {required}"));
        }
    }
    if manifest.schema == "qlrs-bundle/v1"
        && (!names.contains("minimality_certificate.json") || !names.contains("witness_roles.json"))
    {
        return Err("v1 proof files missing".into());
    }
    if actual != names {
        return Err("E-BUNDLE-PATH: unexpected bundle contents".into());
    }
    let checkpoint: Option<SyntheticCheckpoint> = if names.contains("synthetic_checkpoint.json") {
        Some(decode(&read("synthetic_checkpoint.json")?).map_err(|e| e.to_string())?)
    } else {
        None
    };
    if checkpoint.as_ref().map(|cp| cp.source_digest.as_str())
        != manifest.checkpoint_source_digest.as_deref()
    {
        return Err("checkpoint source digest differs from its declared provenance".into());
    }
    let mut trace = ParsedTrace::parse(read("reduced_trace.jsonl")?).map_err(|e| e.to_string())?;
    Checker::prepare_with_checkpoint(&mut trace.store, checkpoint.as_ref())?;
    if trace.store.header.model != manifest.model_profile
        || trace.store.len() != manifest.retained_events
    {
        return Err("trace and manifest metadata disagree".into());
    }
    let alive = AliveMask::all(trace.store.units.len());
    let report = if manifest.schema == "qlrs-bundle/v1" {
        Checker::check_witness(
            &trace.store,
            &alive,
            checkpoint.as_ref(),
            &manifest.instance_key,
        )
    } else {
        Checker::check(&trace.store, &alive, checkpoint.as_ref(), true)
    };
    if report
        .findings
        .iter()
        .any(|f| matches!(f.status, Status::Unsupported | Status::Inconclusive))
    {
        return Err("replay lacks supported complete evidence".into());
    }
    let expected: Finding = decode(&read("violation.json")?).map_err(|e| e.to_string())?;
    if expected.instance_key.as_ref() != Some(&manifest.instance_key)
        || !report.findings.contains(&expected)
    {
        return Err("target instance or evidence not reproduced".into());
    }
    if manifest.schema == "qlrs-bundle/v1" {
        let certificate: Certificate =
            decode(&read("minimality_certificate.json")?).map_err(|e| e.to_string())?;
        if certificate.target != manifest.instance_key
            || certificate.complete != manifest.closure_1_minimal
            || Some(certificate.candidates_tested) != manifest.certificate_candidates_tested
            || certificate.vacuous != (certificate.candidates_tested == 0)
        {
            return Err("certificate and manifest disagree".into());
        }
        let entries: Vec<RoleEntry> =
            decode(&read("witness_roles.json")?).map_err(|e| e.to_string())?;
        if entries.len()
            != trace
                .store
                .len()
                .saturating_add(usize::from(checkpoint.is_some()))
        {
            return Err("role coverage mismatch".into());
        }
        let mut previous_end = 0;
        for (event, seq) in trace.store.spine.seq.iter().enumerate() {
            let entry = entries.get(event).ok_or("missing role")?;
            let wanted = if *seq == expected.window.end_seq {
                WitnessRole::Asserted
            } else if Some(*seq) == expected.window.start_seq
                && manifest.instance_key.invariant_id != qlrs_check::Invariant::I3
            {
                WitnessRole::Anchor
            } else {
                WitnessRole::Evidence
            };
            let [first, last] = entry.source_range.ok_or("missing observed range")?;
            let local = trace
                .line_range(event.saturating_add(1))
                .ok_or("missing observed bytes")?;
            if entry.original_seq != Some(*seq)
                || entry.role != wanted
                || first < previous_end
                || last < first
                || u64::try_from(last).map_err(|_| "invalid source range")?
                    > manifest.original_trace.byte_length
                || last.saturating_sub(first) != local.len()
            {
                return Err("invalid witness role or range".into());
            }
            previous_end = last;
        }
        if checkpoint.is_some()
            && entries.last()
                != Some(&RoleEntry {
                    original_seq: None,
                    source_range: None,
                    role: WitnessRole::Synthetic,
                })
        {
            return Err("missing synthetic role".into());
        }
        if recheck_certificate {
            qlrs_reduce::recheck_certificate(&trace.store, checkpoint.as_ref(), &certificate)?;
        }
    } else if recheck_certificate {
        return Err("legacy v0 bundle has no closure certificate".into());
    }
    let log_bytes = read("reduction_log.jsonl")?;
    let mut entries = 0_usize;
    for line in log_bytes.split_inclusive(|b| *b == b'\n') {
        if manifest.schema == "qlrs-bundle/v1" {
            let entry: qlrs_reduce::StepLog = decode(line).map_err(|e| e.to_string())?;
            if entry.schema != "qlrs-reduce-log/v1"
                || entry.step != u64::try_from(entries).map_err(|_| "too many steps")?
                || !matches!(entry.phase.as_str(), "input" | "search" | "certificate")
            {
                return Err("invalid compact reduction log".into());
            }
        } else {
            let _: qlrs_reduce::LogEntry = decode(line).map_err(|e| e.to_string())?;
        }
        entries = entries.saturating_add(1);
    }
    if entries == 0 {
        return Err("empty reduction log".into());
    }
    Ok(manifest)
}
fn regular_file(
    root: &fs::File,
    dir: &Path,
    name: &str,
    limits: Limits,
    observe: &mut impl FnMut(&Path),
) -> Result<Vec<u8>, String> {
    if name.is_empty()
        || Path::new(name).components().count() != 1
        || !matches!(
            Path::new(name).components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return Err("E-BUNDLE-PATH: invalid relative file name".into());
    }
    let path = dir.join(name);
    observe(&path);
    let file = fs::File::from(
        rustix::fs::openat(
            root,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|e| format!("E-BUNDLE-PATH: {e}"))?,
    );
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err("E-BUNDLE-PATH: non-regular entry".into());
    }
    if metadata.len() > limits.max_entry_bytes {
        return Err("E-BUNDLE-ENTRY-BYTES: entry exceeds byte limit".into());
    }
    let mut bytes = Vec::new();
    file.take(limits.max_entry_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if u64::try_from(bytes.len()).map_err(|e| e.to_string())? > limits.max_entry_bytes {
        return Err("E-BUNDLE-ENTRY-BYTES: entry grew past byte limit".into());
    }
    Ok(bytes)
}

/// Source revision embedded when the product was built.
#[must_use]
pub const fn source_commit() -> &'static str {
    env!("QLRS_GIT_COMMIT")
}
