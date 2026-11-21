//! Repository tasks.
//!
//! `cargo xtask ci` is what CI runs, so a green local run means a green build. Every check is
//! cheap enough to run on each commit; none of them need network access.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const LIB_CRATES: &[&str] = &[
    "core", "hash", "quality", "testkit", "meta", "media", "sidecar", "geo", "ai", "thumbs", "scan", "catalog", "cull", "insights",
    "similar", "remote", "engine", "ui", "mcp",
];

const APP_CRATES: &[&str] = &["sortlens", "sortlens-cli", "sortlens-web", "sortlens-server"];

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_else(|| "help".into());
    let result = match task.as_str() {
        "ci" => ci(),
        "fmt" => run("cargo", &["fmt", "--all", "--", "--check"]),
        "clippy" => run("cargo", &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
        "test" => run("cargo", &["test", "--workspace"]),
        "wasm" => run(
            "cargo",
            &[
                "check",
                "-p",
                "sortlens-ui",
                "-p",
                "sortlens-core",
                "--target",
                "wasm32-unknown-unknown",
            ],
        ),
        "layers" => layers(),
        "roots" => roots(),
        "assets" => assets(),
        _ => {
            help();
            return ExitCode::SUCCESS;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}

fn help() {
    println!(
        "cargo xtask <task>

  ci       fmt --check, clippy, tests, layer rules, crate roots, asset provenance
  fmt      check formatting
  clippy   lint every target with -D warnings
  test     run the workspace tests
  wasm     check the wasm-safe crates for wasm32
  layers   enforce the dependency layering rules
  roots    enforce the crate-level lint header
  assets   enforce that every asset has a provenance entry"
    );
}

fn ci() -> Result<(), String> {
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run("cargo", &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])?;
    run("cargo", &["test", "--workspace"])?;
    layers()?;
    roots()?;
    assets()?;
    Ok(())
}

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("could not run {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed", args.join(" ")))
    }
}

/// Read the dependency section of a crate manifest, excluding dev-dependencies and build-dependencies.
fn dependencies(manifest: &Path) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(manifest).map_err(|error| format!("{}: {error}", manifest.display()))?;
    let mut deps = Vec::new();
    let mut in_deps = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line == "[dependencies]";
            continue;
        }
        if !in_deps || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = line.split_once('=') {
            deps.push(name.trim().to_string());
        }
    }
    Ok(deps)
}

/// The layering rules from `docs/ARCHITECTURE.md`, enforced rather than documented.
fn layers() -> Result<(), String> {
    let mut problems = Vec::new();

    for name in LIB_CRATES {
        let manifest = PathBuf::from(format!("crates/{name}/Cargo.toml"));
        if !manifest.exists() {
            problems.push(format!("missing crate: {name}"));
            continue;
        }
        let deps = dependencies(&manifest)?;

        // Only the ui crate may know about egui or any craft UI crate.
        if *name != "ui" {
            for dep in &deps {
                if dep == "egui" || dep == "eframe" || dep.starts_with("craft-") {
                    problems.push(format!("crates/{name} depends on {dep}; only crates/ui may"));
                }
            }
        }

        // An app crate may never be a dependency.
        for dep in &deps {
            if dep.starts_with("sortlens-app") || APP_CRATES.contains(&dep.as_str()) {
                problems.push(format!("crates/{name} depends on application crate {dep}"));
            }
        }
    }

    for name in APP_CRATES {
        let manifest = PathBuf::from(format!("apps/{name}/Cargo.toml"));
        if !manifest.exists() {
            problems.push(format!("missing app: {name}"));
        }
    }

    // Applications may not depend on other applications (the suite rule).
    for name in APP_CRATES {
        let manifest = PathBuf::from(format!("apps/{name}/Cargo.toml"));
        for dep in dependencies(&manifest)? {
            if dep != *name && APP_CRATES.contains(&dep.as_str()) {
                problems.push(format!("apps/{name} depends on app {dep}"));
            }
        }
    }

    report("layers", problems)
}

fn roots() -> Result<(), String> {
    let mut problems = Vec::new();
    for name in LIB_CRATES {
        let root = PathBuf::from(format!("crates/{name}/src/lib.rs"));
        let text = fs::read_to_string(&root).map_err(|error| format!("{}: {error}", root.display()))?;
        if !text.contains("forbid(unsafe_code)") {
            problems.push(format!("{} has no forbid(unsafe_code)", root.display()));
        }
        for lint in ["clippy::unwrap_used", "clippy::expect_used", "clippy::panic"] {
            if !text.contains(lint) {
                problems.push(format!("{} does not deny {}", root.display(), lint));
            }
        }
    }
    report("roots", problems)
}

fn assets() -> Result<(), String> {
    let dir = Path::new("assets");
    if !dir.exists() {
        return Ok(());
    }
    let attribution = fs::read_to_string("docs/ATTRIBUTION.md").unwrap_or_default();
    let mut problems = Vec::new();
    let entries = fs::read_dir(dir).map_err(|error| format!("assets: {error}"))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("assets: {error}"))?;
        if !entry.path().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !attribution.contains(&name) && !attribution.contains("project-owned") {
            problems.push(format!("assets/{name} has no provenance entry in docs/ATTRIBUTION.md"));
        }
    }
    report("assets", problems)
}

fn report(name: &str, problems: Vec<String>) -> Result<(), String> {
    if problems.is_empty() {
        println!("xtask {name}: ok");
        return Ok(());
    }
    for problem in &problems {
        eprintln!("  {problem}");
    }
    Err(format!("{name}: {} problem(s)", problems.len()))
}
