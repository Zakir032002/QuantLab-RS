#!/usr/bin/env bash
# Repeatable setup for the managed Linux cloud workspace, not a global user home.
set -euo pipefail
export RUSTUP_HOME=/workspace/.qlrs-tools/rustup
export CARGO_HOME=/workspace/.qlrs-tools/cargo
export PATH="/workspace/.qlrs-tools/bin:$CARGO_HOME/bin:$PATH"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-3}"
if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
    qlrs_installer=$(mktemp /tmp/qlrs-rustup.XXXXXX)
    trap 'rm -f "$qlrs_installer"' EXIT
    curl --fail --silent --show-error https://sh.rustup.rs -o "$qlrs_installer"
    sh "$qlrs_installer" -y --no-modify-path --profile minimal --default-toolchain 1.99.1 --component rustfmt --component clippy
elif ! rustup run 1.99.1 rustc --version >/dev/null 2>&1; then
    rustup toolchain install 1.99.1 --profile minimal --component rustfmt --component clippy
fi
rustup component add --toolchain 1.99.1 rustfmt clippy
while read -r qlrs_tool qlrs_version; do
    if [ "$qlrs_tool" = gungraun-runner ]; then
        qlrs_installed=$("$CARGO_HOME/bin/$qlrs_tool" --version 2>/dev/null || true)
    else
        qlrs_installed=$(cargo "${qlrs_tool#cargo-}" --version 2>/dev/null || true)
    fi
    # Cargo plugins differ in their displayed command names; compare the exact
    # space-delimited release token, not a guessed executable-name prefix.
    if [[ " $qlrs_installed " != *" $qlrs_version "* ]]; then
        cargo install "$qlrs_tool" --version "$qlrs_version" --locked --force
    fi
done <<'TOOLS'
cargo-nextest 0.9.151
cargo-deny 0.20.2
cargo-audit 0.22.2
cargo-mutants 27.1.0
gungraun-runner 0.20.0
TOOLS
cd /workspace/QuantLab-RS
cargo fetch --locked
cargo build --workspace --locked
./ci/check-all.sh
python3 ci/catalogue.py
