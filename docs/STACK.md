# Verified stack

Verified against official crates.io release metadata on 2026-11-04.
Rust 1.99.1: https://static.rust-lang.org/dist/channel-rust-1.99.1.toml
docs.rs requests remain denied by the running policy; see ADR-0004.

| Package | Version | Published UTC | Registry | Documentation |
|---|---|---|---|---|
| blake3 | 1.8.7 | 2026-08-20 | https://crates.io/crates/blake3/1.8.7 | https://docs.rs/crate/blake3/1.8.7 (access blocked) |
| cargo-audit | 0.22.2 | 2026-06-05 | https://crates.io/crates/cargo-audit/0.22.2 | https://docs.rs/crate/cargo-audit/0.22.2 (access blocked) |
| cargo-deny | 0.20.2 | 2026-07-09 | https://crates.io/crates/cargo-deny/0.20.2 | https://docs.rs/crate/cargo-deny/0.20.2 (access blocked) |
| cargo-mutants | 27.1.0 | 2026-06-02 | https://crates.io/crates/cargo-mutants/27.1.0 | https://docs.rs/crate/cargo-mutants/27.1.0 (access blocked) |
| cargo-nextest | 0.9.151 | 2026-11-03 | https://crates.io/crates/cargo-nextest/0.9.151 | https://docs.rs/crate/cargo-nextest/0.9.151 (access blocked) |
| clap | 4.6.7 | 2026-09-14 | https://crates.io/crates/clap/4.6.7 | https://docs.rs/crate/clap/4.6.7 (access blocked) |
| criterion | 0.8.2 | 2026-02-04 | https://crates.io/crates/criterion/0.8.2 | https://docs.rs/crate/criterion/0.8.2 (access blocked) |
| foldhash | 0.2.0 | 2025-08-23 | https://crates.io/crates/foldhash/0.2.0 | https://docs.rs/crate/foldhash/0.2.0 (access blocked) |
| gungraun | 0.20.0 | 2026-09-26 | https://crates.io/crates/gungraun/0.20.0 | https://docs.rs/crate/gungraun/0.20.0 (access blocked) |
| num-bigint | 0.4.8 | 2026-07-05 | https://crates.io/crates/num-bigint/0.4.8 | https://docs.rs/crate/num-bigint/0.4.8 (access blocked) |
| num-traits | 0.2.19 | 2024-05-03 | https://crates.io/crates/num-traits/0.2.19 | https://docs.rs/crate/num-traits/0.2.19 (access blocked) |
| proptest | 1.12.0 | 2026-10-10 | https://crates.io/crates/proptest/1.12.0 | https://docs.rs/crate/proptest/1.12.0 (access blocked) |
| rayon | 1.12.0 | 2026-04-14 | https://crates.io/crates/rayon/1.12.0 | https://docs.rs/crate/rayon/1.12.0 (access blocked) |
| serde | 1.0.229 | 2026-07-18 | https://crates.io/crates/serde/1.0.229 | https://docs.rs/crate/serde/1.0.229 (access blocked) |
| serde_json | 1.0.154 | 2026-10-11 | https://crates.io/crates/serde_json/1.0.154 | https://docs.rs/crate/serde_json/1.0.154 (access blocked) |

Not all verified tools are runtime dependencies. num-traits is transitive only.
Optional mmap, sonic-rs, smallvec, tracing and mimalloc are omitted without measured need.
All transitive Rust dependency versions and checksums are in Cargo.lock.

Stable-API probe: `u128::carrying_mul` compiles on pinned Rust 1.99.1. The
source and successful command result are in
[carrying-mul-probe.txt](../validation-results/carrying-mul-probe.txt). Existing
safe, independently tested limb arithmetic is retained; switching to the newer
primitive without a demonstrated benefit is deferred (ADR-0001/0006).

Part 2 recheck: Rayon 1.12.0, released 2026-04-14, not yanked, verified
2026-11-05 Asia/Calcutta (2026-11-04 UTC) via
https://crates.io/api/v1/crates/rayon . The direct testkit edge reuses this exact
workspace pin; no package version was added or bumped. Raw metadata:
bench/results/CP2.8/rayon-release.json. Rust remains 1.99.1; CP2.0 contains the
official channel recheck.

Part 2 tooling recheck, 2026-11-05 Asia/Calcutta: cargo-mutants 27.1.0
(released 2026-06-02) and optional kani-verifier 0.68.0 (2026-09-16), neither
yanked, verified via https://crates.io/api/v1/crates/cargo-mutants and
https://crates.io/api/v1/crates/kani-verifier . Raw metadata is recorded with
CP2.11. Kani is configured as an optional job; no local proof pass is claimed.

## Part 3 version inventory — 2026-11-05

Official registry recheck: [raw metadata](../evidence/part3/cp3.0/registry.json).
Pinned product dependencies are retained. Candidates listed here are not all adopted.
Older samply/ahash releases are inventory only; no maintained-status exception or use is claimed.
Rust stable remains 1.99.1; the [official channel](../evidence/part3/cp3.0/channel-rust-stable.toml)
reports 2026-10-15, so no 1.100 toolchain bump is made. Existing dated nightly
2026-11-04 is retained for Miri; the fuzz nightly will be independently checked at CP3.4.

| Package/tool | Exact version | Published UTC | Official source |
|---|---|---|---|
| cargo-fuzz | 0.13.2 | 2026-06-09 | https://crates.io/api/v1/crates/cargo-fuzz |
| kani-verifier | 0.68.0 | 2026-09-16 | https://crates.io/api/v1/crates/kani-verifier |
| cargo-mutants | 27.1.0 | 2026-06-02 | https://crates.io/api/v1/crates/cargo-mutants |
| cargo-nextest | 0.9.151 | 2026-11-03 | https://crates.io/api/v1/crates/cargo-nextest |
| cargo-deny | 0.20.2 | 2026-07-09 | https://crates.io/api/v1/crates/cargo-deny |
| cargo-audit | 0.22.2 | 2026-06-05 | https://crates.io/api/v1/crates/cargo-audit |
| gungraun | 0.20.0 | 2026-09-26 | https://crates.io/api/v1/crates/gungraun |
| gungraun-runner | 0.20.0 | 2026-09-26 | https://crates.io/api/v1/crates/gungraun-runner |
| hyperfine | 2.0.0 | 2026-10-07 | https://crates.io/api/v1/crates/hyperfine |
| samply | 0.13.1 | 2025-02-01 | https://crates.io/api/v1/crates/samply |
| serde_json | 1.0.154 | 2026-10-11 | https://crates.io/api/v1/crates/serde_json |
| rayon | 1.12.0 | 2026-04-14 | https://crates.io/api/v1/crates/rayon |
| foldhash | 0.2.0 | 2025-08-23 | https://crates.io/api/v1/crates/foldhash |
| ahash | 0.8.12 | 2025-05-08 | https://crates.io/api/v1/crates/ahash |
| rustc-hash | 2.1.3 | 2026-07-02 | https://crates.io/api/v1/crates/rustc-hash |
| mimalloc | 0.1.52 | 2026-05-22 | https://crates.io/api/v1/crates/mimalloc |
| tikv-jemallocator | 0.7.0 | 2026-05-25 | https://crates.io/api/v1/crates/tikv-jemallocator |
| sonic-rs | 0.5.10 | 2026-09-11 | https://crates.io/api/v1/crates/sonic-rs |
| simd-json | 0.19.0 | 2026-10-27 | https://crates.io/api/v1/crates/simd-json |
| arbitrary | 1.5.0 | 2026-10-06 | https://crates.io/api/v1/crates/arbitrary |
| libfuzzer-sys | 0.4.13 | 2026-06-04 | https://crates.io/api/v1/crates/libfuzzer-sys |
| cargo-auditable | 0.7.7 | 2026-10-02 | https://crates.io/api/v1/crates/cargo-auditable |
| cargo-vet | 0.10.3 | 2026-10-22 | https://crates.io/api/v1/crates/cargo-vet |
| blake3 | 1.8.7 | 2026-08-20 | https://crates.io/api/v1/crates/blake3 |

GNU time: Debian package `time 1.9-0.2`, packaging changelog dated 2022-09-22,
verified from the official Debian archive and extracted into the workspace.
The binary reports upstream version UNKNOWN; the package version and SHA256,
not an invented binary version, are recorded in evidence/part3/cp3.0/gnu-time-package.json.
The prescribed RSS tool was absent from /usr/bin; its workspace path is recorded
in every command. Hyperfine 2.0.0 was installed with --locked after registry verification.

Part 3 CI pins: actions/checkout v4 at 11d5960a326750d5838078e36cf38b85af677262
(commit date 2026-07-16), actions/upload-artifact v4 at
ea165f8d65b6e75b540449e92b4886f43607fa02 (2025-03-19; existing major retained).
Official repository refs and exact dates: evidence/part3/cp3.0/actions-refs.json.
Dated fuzz nightly verified against its official manifest: 2026-11-04,
1.101.0-nightly; cargo-fuzz 0.13.2. Root Rust remains 1.99.1.
