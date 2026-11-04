Use the existing isolated checkout at `/workspace/QuantLab-RS`. Do not create a
worktree unless the user requests one. No service, database or secret is needed.

Activate the installed tools in each shell:

```sh
export RUSTUP_HOME=/workspace/.qlrs-tools/rustup
export CARGO_HOME=/workspace/.qlrs-tools/cargo
export PATH="/workspace/.qlrs-tools/bin:$CARGO_HOME/bin:$PATH"
cd /workspace/QuantLab-RS
```

Read AGENTS.md, docs/HANDOFF_PART_01.md and docs/BLOCKERS.md. The full synthetic
Part 1 workflow is implemented; use the handoff for measured gates and limits.
The setup script builds the locked workspace and runs ci/check-all.sh plus the
protected catalogue CLI check. Run those checks after meaningful changes.

Quick readiness check:

```sh
cargo run -p qlrs-cli --locked -- verify fixtures/clean/C01-worked-example.jsonl
```

It must return CONSISTENT and expected cash 7990.00. README.md contains the full
generate, verify, reduce and bundle-verify demonstration. Run the scale_gate
example in release mode for the million-event gate. Bench commands, source and
fixture commitments, raw samples and machine information are in bench-results.
Linux instruction benches require Valgrind and gungraun-runner. This snapshot
has a workspace Valgrind wrapper in /workspace/.qlrs-tools/bin; it supplies the
library/debug paths after the benchmark runner clears its environment. Use
GUNGRAUN_ALLOW_ASLR=yes because the container forbids setarch personality changes.
Hosted CI installs Valgrind through its OS package manager.

Miri edge and allocation commands are in the quality workflow and handoff;
high-volume properties run natively. Do not claim hosted macOS success or fresh
environment restoration without observing it. Whole-process memory exceeds the
original engineering target because original JSONL evidence is retained.

Versions, dates and checksums were verified against official crates.io metadata
and downloaded source. docs.rs remains denied by the running destination policy;
ADR-0004 records the authorized fallback. A saved docs.rs allowlist entry is not
proof that runtime policy has been applied. Follow the pinned stack and lockfile;
do not change protected expectations, tolerances or golden files to pass checks.
