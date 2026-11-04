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
