# Project-local Rust toolchain. Source this before cargo/xtask commands:
#   . scripts/env.sh
# Installed with `brew install rustup` + project-local RUSTUP_HOME/CARGO_HOME;
# nothing user-wide is modified and .toolchain/ is gitignored.
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
export RUSTUP_HOME="$PWD/.toolchain/rustup"
export CARGO_HOME="$PWD/.toolchain/cargo"
