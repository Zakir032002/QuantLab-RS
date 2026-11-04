//! Deterministic, offline-verifiable evidence directories with verbatim claims.
#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]
use qlrs_check::{Checker, Finding, InstanceKey};
use qlrs_io::{ParsedTrace, decode, digest, encode};
use qlrs_reduce::Reduction;
use qlrs_schema::{AliveMask, Model, Status, SyntheticCheckpoint};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

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
    /// Exhausted search budget, when minimality has not been certified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<qlrs_reduce::LimitReason>,
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
    fs::write(dir.join(name), bytes).map_err(|e| e.to_string())?;
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
    let dir = dir.as_ref();
    fs::create_dir(dir).map_err(|e| format!("cannot create new bundle directory: {e}"))?;
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
    let mut log = Vec::new();
    for entry in &reduction.log {
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
        schema: "qlrs-bundle/v0".into(),
        qlrs_version: env!("CARGO_PKG_VERSION").into(),
        git_commit: env!("QLRS_GIT_COMMIT").into(),
        rustc_version: env!("QLRS_RUSTC_VERSION").into(),
        model_profile: trace.store.header.model.clone(),
        verdict: Status::Violation,
        original_trace: SourceTrace {
            blake3: digest(&trace.source),
            byte_length: u64::try_from(trace.source.len()).map_err(|_| "input too large")?,
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
        stop_reason: reduction.stop_reason,
        minimality_guarantee: minimality_text(reduction.closure_1_minimal).into(),
        retained_events: reduction.retained_events,
        files,
    };
    let bytes = encode(&manifest).map_err(|e| e.to_string())?;
    put(dir, "manifest.json", &bytes)?;
    fs::write(
        dir.join("manifest.blake3"),
        format!("{}  manifest.json\n", digest(&bytes)),
    )
    .map_err(|e| e.to_string())?;
    verify(dir)?;
    Ok(manifest)
}
/// Rehash every byte, reject unexpected paths/files, replay and match the key.
/// Integrity detects corruption; it does not authenticate a malicious author.
#[allow(clippy::too_many_lines)]
pub fn verify(dir: impl AsRef<Path>) -> Result<Manifest, String> {
    let dir = dir.as_ref();
    let manifest_bytes = regular_file(dir, "manifest.json")?;
    let commitment = regular_file(dir, "manifest.blake3")?;
    if commitment != format!("{}  manifest.json\n", digest(&manifest_bytes)).as_bytes() {
        return Err("manifest digest mismatch".into());
    }
    let manifest: Manifest = decode(&manifest_bytes).map_err(|e| e.to_string())?;
    if encode(&manifest).map_err(|e| e.to_string())? != manifest_bytes {
        return Err("manifest is not canonical".into());
    }
    if manifest.schema != "qlrs-bundle/v0"
        || manifest.verdict != Status::Violation
        || manifest.minimality_guarantee != minimality_text(manifest.closure_1_minimal)
        || manifest.closure_1_minimal == manifest.stop_reason.is_some()
        || manifest.original_trace.blake3.len() != 64
    {
        return Err("unsupported or invalid bundle manifest".into());
    }
    let allowed = [
        "REPRODUCE.md",
        "reduced_trace.jsonl",
        "reduction_log.jsonl",
        "synthetic_checkpoint.json",
        "violation.json",
    ];
    let mut names = BTreeSet::from(["manifest.json".to_owned(), "manifest.blake3".to_owned()]);
    let mut previous = "";
    for file in &manifest.files {
        if !allowed.contains(&file.path.as_str())
            || file.path.as_str() <= previous
            || !names.insert(file.path.clone())
        {
            return Err("invalid, duplicate or unsorted bundle path".into());
        }
        previous = &file.path;
        let bytes = regular_file(dir, &file.path)?;
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
    let actual: Result<BTreeSet<_>, _> = fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .map(|entry| entry.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect();
    if actual.map_err(|e| e.to_string())? != names {
        return Err("unexpected bundle contents".into());
    }
    let checkpoint: Option<SyntheticCheckpoint> = if names.contains("synthetic_checkpoint.json") {
        Some(decode(&regular_file(dir, "synthetic_checkpoint.json")?).map_err(|e| e.to_string())?)
    } else {
        None
    };
    if checkpoint.as_ref().map(|cp| cp.source_digest.as_str())
        != manifest.checkpoint_source_digest.as_deref()
    {
        return Err("checkpoint source digest differs from its declared provenance".into());
    }
    let mut trace =
        ParsedTrace::parse(regular_file(dir, "reduced_trace.jsonl")?).map_err(|e| e.to_string())?;
    Checker::prepare_with_checkpoint(&mut trace.store, checkpoint.as_ref())?;
    if trace.store.header.model != manifest.model_profile
        || trace.store.len() != manifest.retained_events
    {
        return Err("trace and manifest metadata disagree".into());
    }
    let report = Checker::check(
        &trace.store,
        &AliveMask::all(trace.store.units.len()),
        checkpoint.as_ref(),
        true,
    );
    if report
        .findings
        .iter()
        .any(|f| matches!(f.status, Status::Unsupported | Status::Inconclusive))
    {
        return Err("replay lacks supported complete evidence".into());
    }
    let expected: Finding =
        decode(&regular_file(dir, "violation.json")?).map_err(|e| e.to_string())?;
    if expected.instance_key.as_ref() != Some(&manifest.instance_key)
        || !report.findings.contains(&expected)
    {
        return Err("target instance or evidence not reproduced".into());
    }
    let log_bytes = regular_file(dir, "reduction_log.jsonl")?;
    let mut entries = 0_usize;
    for line in log_bytes.split_inclusive(|b| *b == b'\n') {
        let _: qlrs_reduce::LogEntry = decode(line).map_err(|e| e.to_string())?;
        entries = entries.saturating_add(1);
    }
    if entries == 0 {
        return Err("empty reduction log".into());
    }
    Ok(manifest)
}
fn regular_file(dir: &Path, name: &str) -> Result<Vec<u8>, String> {
    let path = dir.join(name);
    let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !metadata.file_type().is_file() {
        return Err(format!("not a regular bundle file: {name}"));
    }
    fs::read(path).map_err(|e| e.to_string())
}
