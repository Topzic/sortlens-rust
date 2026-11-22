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

/// Crates that only the shell may use. Format and platform craft crates (metadata, jpeg, tiff,
/// raw, settings, atomic) are allowed at any layer; these are the ones that assume a window.
const UI_ONLY: &[&str] = &[
    "egui",
    "eframe",
    "craft-ui",
    "craft-window",
    "craft-menu",
    "craft-dialogs",
    "craft-keymap",
    "craft-update",
    "craft-quit",
    "craft-extensions",
];

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
        "fonts" => fonts(),
        "deny" => deny(),
        "snapshot" => snapshot(),
        "version" => version(),
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

  ci       fmt --check, clippy, tests, layer rules, crate roots, asset provenance, fonts
  fmt      check formatting
  clippy   lint every target with -D warnings
  test     run the workspace tests
  wasm     check the wasm-safe crates for wasm32
  layers   enforce the dependency layering rules
  roots    enforce the crate-level lint header
  assets   enforce that every asset has a provenance entry
  fonts    report the embedded face set and refuse to ship one that covers no Latin text
  deny     check licences, advisories, bans and sources against deny.toml
  snapshot golden CLI output; --update rewrites the goldens, then read the diff"
    );
}

fn ci() -> Result<(), String> {
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run("cargo", &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])?;
    run("cargo", &["test", "--workspace"])?;
    layers()?;
    roots()?;
    assets()?;
    fonts()?;
    version()?;
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

        // Only the ui crate may know that a window exists.
        if *name != "ui" {
            for dep in &deps {
                if UI_ONLY.contains(&dep.as_str()) {
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

/// Check the font stack: the family list, and whether the required scripts are covered.
///
/// The craft-fonts checkout is optional, so "not embedded" is a pass with a clear line rather than a
/// silent success — but a build that *does* embed faces and covers none of the interface's own
/// scripts is refused, because that is a table built from the wrong manifest.
fn fonts() -> Result<(), String> {
    let manifest = Path::new("fonts/embedded.txt");
    let dir = std::env::var_os("CRAFT_FONTS_DIR").map(std::path::PathBuf::from);
    let mut report = Vec::new();
    match &dir {
        None => {
            println!("xtask fonts: not embedded (CRAFT_FONTS_DIR is unset) — egui's own faces are used");
            println!("  to embed: git clone https://github.com/storytold/craft-fonts .fonts/craft-fonts");
            println!("            CRAFT_FONTS_DIR=$PWD/.fonts/craft-fonts cargo xtask fonts");
            return Ok(());
        }
        Some(dir) => {
            let checkout = dir.join("fonts/manifest.txt");
            if !checkout.exists() {
                return Err(format!("{} is not a craft-fonts checkout", dir.display()));
            }
            let text = fs::read_to_string(&checkout).map_err(|error| error.to_string())?;
            let mut families: Vec<&str> = text
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .filter_map(|line| line.split(" | ").next())
                .collect();
            families.sort_unstable();
            families.dedup();
            report.push(format!(
                "{} face(s) in {} family(ies): {}",
                text.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty() && !line.starts_with('#'))
                    .count(),
                families.len(),
                families.join(", ")
            ));
            let has_latin = text.lines().any(|line| line.contains("Latn"));
            let has_symbols = text.contains("Zsym");
            if !has_latin {
                return Err("the manifest lists no Latin face".to_string());
            }
            report.push(format!(
                "required scripts: Latn {} · Zsym {}",
                "covered",
                if has_symbols { "covered" } else { "MISSING" }
            ));
        }
    }
    if let Ok(written) = fs::read_to_string(manifest) {
        report.push(format!("last written record: {}", written.trim()));
    }
    for line in &report {
        println!("  {line}");
    }
    println!("xtask fonts: ok ({} note(s))", report.len());
    Ok(())
}

/// Diff (or rewrite) the golden outputs in `apps/sortlens-cli/tests/golden`.
///
/// Read-only by default, because that is what CI runs: the goldens are a claim about what the tool
/// says, and a check that can update its own expectation checks nothing. `--update` exists for the
/// person making an intended change, and printing the diff is left to `git diff`.
fn snapshot() -> Result<(), String> {
    if std::env::args().any(|arg| arg == "--update") {
        println!("xtask snapshot: rewriting goldens — read `git diff` before committing");
        let status = Command::new("cargo")
            .args(["test", "-p", "sortlens-cli", "--test", "golden"])
            .env("SORTLENS_UPDATE_GOLDEN", "1")
            .status()
            .map_err(|error| format!("could not run cargo: {error}"))?;
        if !status.success() {
            return Err("rewriting the goldens failed".to_string());
        }
        return Ok(());
    }
    run("cargo", &["test", "-p", "sortlens-cli", "--test", "golden"])
}

/// Run cargo-deny against the allow-list in `deny.toml`.
///
/// A missing cargo-deny is a failure, not a pass: a check that quietly does nothing when its tool is
/// absent is worse than no check, because the badge still goes green. CI installs it; locally it is
/// `cargo install cargo-deny --locked`.
fn deny() -> Result<(), String> {
    let installed = Command::new("cargo")
        .args(["deny", "--version"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    if !installed {
        return Err("cargo-deny is not installed; run: cargo install cargo-deny --locked".to_string());
    }
    run("cargo", &["deny", "check"])
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

/// One version for the whole workspace, declared once.
///
/// A release is the moment a hand-written version string goes stale, and a crate that carries its own
/// number is a crate that will disagree with the app's About window a year later. This checks that every
/// member inherits the workspace version and that the constant the app prints comes from the same place —
/// a check that cannot fail would be worse than none, so it names the crate that broke the rule.
fn version() -> Result<(), String> {
    let mut problems = Vec::new();
    let workspace = fs::read_to_string("Cargo.toml").map_err(|error| format!("Cargo.toml: {error}"))?;
    let declared = workspace
        .lines()
        .find_map(|line| line.strip_prefix("version = "))
        .map(|value| value.trim().trim_matches('"').to_string());
    let Some(declared) = declared else {
        return Err("the workspace Cargo.toml declares no version".into());
    };

    let mut manifests: Vec<(String, String)> = Vec::new();
    manifests.push(("Cargo.toml".to_string(), workspace.clone()));
    for folder in ["crates", "apps", "xtask"] {
        let Ok(entries) = fs::read_dir(folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let manifest = entry.path().join("Cargo.toml");
            if manifest.is_file()
                && let Ok(text) = fs::read_to_string(&manifest)
            {
                manifests.push((manifest.display().to_string(), text));
            }
        }
    }
    for (path, text) in &manifests {
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with("version = ") && !line.contains("workspace = true") && path != "Cargo.toml" {
                problems.push(format!("{path} declares its own version: {line}"));
            }
        }
        if !text.contains("version.workspace = true") && path != "Cargo.toml" {
            problems.push(format!("{path} does not inherit the workspace version"));
        }
    }

    let core = fs::read_to_string("crates/core/src/lib.rs").map_err(|error| format!("core: {error}"))?;
    if !core.contains("env!(\"CARGO_PKG_VERSION\")") {
        problems.push("sortlens-core::VERSION does not come from CARGO_PKG_VERSION".into());
    }

    println!("xtask version: {declared}");
    report("version", problems)
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
