# Project-local Rust toolchain. Source this before cargo/xtask commands:
#   . scripts/env.sh
# Installed with `brew install rustup` + project-local RUSTUP_HOME/CARGO_HOME
# (or, on Windows, rustup-init.exe from https://rustup.rs into the same paths);
# nothing user-wide is modified and .toolchain/ is gitignored.
export RUSTUP_HOME="$PWD/.toolchain/rustup"
export CARGO_HOME="$PWD/.toolchain/cargo"
# The project-local cargo bin first; the Homebrew rustup bin only where it exists.
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*)
    export PATH="$PWD/.toolchain/cargo/bin:$PATH"
    ;;
  *)
    if [ -d "/opt/homebrew/opt/rustup/bin" ]; then
      export PATH="/opt/homebrew/opt/rustup/bin:$PWD/.toolchain/cargo/bin:$PATH"
    else
      export PATH="$PWD/.toolchain/cargo/bin:$PATH"
    fi
    ;;
esac
