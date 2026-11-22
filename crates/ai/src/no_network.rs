//! The promise that this crate cannot reach the network, checked instead of believed.
//!
//! "Local only" is the central claim of the application. A comment saying so is worthless: what
//! matters is that no code path in the AI crate can open a socket, and that no dependency can do it on
//! the crate's behalf. Two source-level checks enforce that, and they fail the build rather than a
//! review.
//!
//! This is deliberately a source scan and not a build-time lint: a `#![forbid]` cannot express
//! "no networking crates", and a reviewer reading `Cargo.toml` is exactly the thing that gets skipped
//! under deadline.

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Crates that exist to open sockets, or that open them as a side effect of being a runtime.
    const NETWORK_CRATES: &[&str] = &[
        "reqwest",
        "hyper",
        "ureq",
        "curl",
        "isahc",
        "surf",
        "attohttpc",
        "tokio",
        "async-std",
        "smol",
        "tungstenite",
        "quinn",
        "h2",
        "rustls",
        "native-tls",
        "openssl",
        "webpki-roots",
    ];

    /// Names that only appear when something is about to talk to a network.
    const NETWORK_SYMBOLS: &[&str] = &[
        "std::net",
        "TcpStream",
        "TcpListener",
        "UdpSocket",
        "ToSocketAddrs",
        "reqwest::",
        "hyper::",
        "ureq::",
        "tokio::",
        "std::process::Command",
    ];

    fn crate_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
    }

    fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                out.push(path);
            }
        }
    }

    #[test]
    fn the_ai_crate_depends_on_nothing_that_can_open_a_socket() {
        let manifest_path = crate_root().join("Cargo.toml");
        let text = fs::read_to_string(&manifest_path).expect("read Cargo.toml");
        // Only the dependency tables, so a mention in a comment or a description does not trip this.
        let mut in_dependencies = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_dependencies = trimmed.starts_with("[dependencies")
                    || trimmed.starts_with("[dev-dependencies")
                    || trimmed.starts_with("[build-dependencies");
                continue;
            }
            if !in_dependencies || trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let name = trimmed.split('=').next().unwrap_or_default().trim().to_string();
            assert!(
                !NETWORK_CRATES.contains(&name.as_str()),
                "{name} can reach the network, so it does not belong in sortlens-ai:\n  {trimmed}"
            );
        }
    }

    #[test]
    fn no_source_file_in_the_ai_crate_mentions_a_network_api() {
        let mut files = Vec::new();
        rust_files(&crate_root().join("src"), &mut files);
        assert!(!files.is_empty(), "the scan found the sources");
        for file in files {
            // This file names the symbols on purpose, so it is the one exception, and it is named
            // explicitly rather than being skipped by a pattern that could hide a real hit.
            if file.ends_with("no_network.rs") {
                continue;
            }
            let text = fs::read_to_string(&file).unwrap_or_default();
            for symbol in NETWORK_SYMBOLS {
                assert!(
                    !text.contains(symbol),
                    "{} mentions {symbol}, which is how a crate starts talking to a network",
                    file.display()
                );
            }
        }
    }

    #[test]
    fn the_scan_would_actually_catch_something() {
        // A check that cannot fail is not a check: prove the symbol list matches real code.
        let suspicious = "fn fetch() { let stream = std::net::TcpStream::connect(addr); }";
        assert!(
            NETWORK_SYMBOLS.iter().any(|symbol| suspicious.contains(symbol)),
            "the symbol list must catch a socket"
        );
        let dependency = "reqwest = { version = \"0.12\" }";
        assert!(
            NETWORK_CRATES.iter().any(|name| dependency.starts_with(name)),
            "the dependency list must catch an http client"
        );
    }
}
