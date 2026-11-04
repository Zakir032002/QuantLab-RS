//! Embed toolchain and source provenance without wall-clock timestamps.
#![forbid(unsafe_code)]
use std::process::Command;
fn main() {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
    let version = Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".into());
    let git = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "uncommitted".into());
    println!("cargo:rustc-env=QLRS_RUSTC_VERSION={}", version.trim());
    println!("cargo:rustc-env=QLRS_GIT_COMMIT={}", git.trim());
    println!("cargo:rerun-if-changed=build.rs");
    // Source archives and mutation copies have no Git metadata. Watching a
    // nonexistent path makes Cargo rebuild the bundle and its dependents on
    // every invocation. Resolve real paths for ordinary clones/worktrees.
    for name in ["HEAD", "refs"] {
        if let Ok(output) = Command::new("git")
            .args(["rev-parse", "--git-path", name])
            .output()
            && output.status.success()
            && let Ok(path) = String::from_utf8(output.stdout)
            && std::path::Path::new(path.trim()).exists()
        {
            println!("cargo:rerun-if-changed={}", path.trim());
        }
    }
}
