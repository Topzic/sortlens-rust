#!/bin/sh
# One-time setup for a machine without Rust: installs rustup (Homebrew) and the
# pinned toolchain into ./.toolchain so the project is self-contained.
set -eu
cd "$(dirname "$0")/.."
if ! command -v rustup >/dev/null 2>&1; then
  if command -v brew >/dev/null 2>&1; then
    brew install rustup
  else
    echo "Install rustup first: https://rustup.rs" >&2
    exit 1
  fi
fi
export PATH="$(brew --prefix rustup 2>/dev/null)/bin:$PATH"
export RUSTUP_HOME="$PWD/.toolchain/rustup"
export CARGO_HOME="$PWD/.toolchain/cargo"
rustup toolchain install 1.95.0 --profile minimal
rustup component add rustfmt clippy --toolchain 1.95.0
rustup target add wasm32-unknown-unknown --toolchain 1.95.0
echo "Ready. Run: . scripts/env.sh && cargo build -p sortlens"
