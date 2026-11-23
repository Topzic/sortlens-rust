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
        "screenshots" => screenshots(),
        "version" => version(),
        "attribution" => attribution(),
        "a11y" => a11y(),
        "perf" => perf(),
        "docs" => docs(),
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
  snapshot golden CLI output; --update rewrites the goldens, then read the diff
  screenshots  paint the shell through egui_kittest and diff the baselines; --update rewrites them
  version  one version for the workspace, declared once
  attribution  every dependency attributed, and the document honest about what is linked
  a11y     every custom-painted clickable region registers a name
  perf     measurements for docs/PERF.md (not in ci: a timing that fails on a loaded box gets ignored)
  docs     the documents agree with the tracker, and every crate and document is named"
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
    attribution()?;
    a11y()?;
    docs()?;
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

/// Paint the shell and compare it with the committed baselines (ROADMAP Phase 3 item 9).
///
/// Opt-in on purpose: `cargo xtask ci` is headless, and this needs a GPU adapter. The test file is
/// gated on `sortlens-ui`'s `screenshots` feature, so it does not even compile into a normal run.
/// Tests run one at a time: each one creates a wgpu device, and six devices at once is a way to make
/// a screenshot fail for a reason that has nothing to do with the UI.
fn screenshots() -> Result<(), String> {
    let update = std::env::args().any(|arg| arg == "--update");
    if update {
        println!("xtask screenshots: rewriting baselines — read `git diff --stat` and open the images");
    }
    let mut command = Command::new("cargo");
    command.args([
        "test",
        "-p",
        "sortlens-ui",
        "--features",
        "screenshots",
        "--test",
        "screenshots",
        "--",
        "--test-threads=1",
    ]);
    if update {
        command.env("UPDATE_SNAPSHOTS", "1");
    }
    let status = command.status().map_err(|error| format!("could not run cargo: {error}"))?;
    if !status.success() {
        return Err("the screenshot baselines do not match (see crates/ui/tests/snapshots)".to_string());
    }
    Ok(())
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

/// Every dependency is attributed, and the document says which ones are actually linked.
///
/// Two failure modes matter and this catches both. A crate can be linked into the app and missing from the
/// document, which is a licence nobody read; or the document can name crates that no longer appear in any
/// manifest, which is a table that has stopped describing the software. Crates declared for a later
/// milestone but not linked yet are listed separately, because "declared" and "shipped" are different
/// claims and a reader deserves to know which is which.
fn attribution() -> Result<(), String> {
    let doc = fs::read_to_string("docs/ATTRIBUTION.md").map_err(|error| format!("ATTRIBUTION.md: {error}"))?;
    let mut problems = Vec::new();

    let workspace = fs::read_to_string("Cargo.toml").map_err(|error| format!("Cargo.toml: {error}"))?;
    let mut declared: Vec<String> = Vec::new();
    let mut inside = false;
    for line in workspace.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed == "[workspace.dependencies]";
            continue;
        }
        if inside
            && let Some((name, _)) = trimmed.split_once(" = ")
            && !name.starts_with("sortlens-")
        {
            declared.push(name.to_string());
        }
    }

    let mut manifests: Vec<String> = Vec::new();
    for folder in ["crates", "apps", "xtask"] {
        let Ok(entries) = fs::read_dir(folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let manifest = entry.path().join("Cargo.toml");
            if let Ok(text) = fs::read_to_string(&manifest) {
                manifests.push(text);
            }
        }
    }
    // A crate is linked when some manifest asks for it from the workspace.
    let linked: Vec<&String> = declared
        .iter()
        .filter(|name| {
            manifests.iter().any(|text| {
                text.lines().any(|line| {
                    // Both spellings count: `name = { workspace = true }` and `name.workspace = true`.
                    let Some((key, rest)) = line.trim().split_once('=') else {
                        return false;
                    };
                    let key = key.trim();
                    if let Some(dotted) = key.strip_suffix(".workspace") {
                        return dotted == name.as_str() && rest.trim() == "true";
                    }
                    key == name.as_str() && rest.contains("workspace = true")
                })
            })
        })
        .collect();
    let pending: Vec<&String> = declared.iter().filter(|name| !linked.contains(name)).collect();

    for name in &linked {
        if !doc.contains(&format!("`{name}`")) {
            problems.push(format!("{name} is linked but docs/ATTRIBUTION.md does not attribute it"));
        }
    }
    // The "Data and models" section is a table of pinned model files, not dependencies; its names
    // are checked against the manifest's pinned URLs below instead of against Cargo.toml.
    let dependency_doc = doc
        .split("## ")
        .filter(|section| !section.trim_start().starts_with("Data and models"))
        .collect::<Vec<_>>()
        .join("## ");
    let (linked_section, pending_section) = split_attribution(&dependency_doc);
    for name in &pending {
        if !pending_section.contains(&format!("`{name}`")) {
            problems.push(format!(
                "{name} is declared in Cargo.toml but not linked; list it under \"Declared, not linked yet\""
            ));
        }
    }
    // The other direction: a table entry that no longer describes anything.
    for table in [&linked_section, &pending_section] {
        for name in crate_names_in(table) {
            if !declared.contains(&name) {
                problems.push(format!("docs/ATTRIBUTION.md lists `{name}`, which is not a dependency any more"));
            }
        }
    }

    // Pinned model weights: every release URL the manifest pins must be attributed, and a manifest
    // with no pins must say plainly that it has none.
    let manifest_source = fs::read_to_string("crates/ai/src/manifest.rs").map_err(|error| format!("manifest.rs: {error}"))?;
    let pinned: Vec<&str> = manifest_source
        .lines()
        .filter_map(|line| line.split("https://github.com/Topzic/sortlens-rust/releases/download/").nth(1))
        .filter_map(|rest| rest.split('"').next())
        .collect();
    for url in &pinned {
        let file = url.rsplit('/').next().unwrap_or("");
        if !file.is_empty() && !doc.contains(file) {
            problems.push(format!("the manifest pins {file}, but docs/ATTRIBUTION.md does not attribute it"));
        }
    }
    if !manifest_source.contains("models: Vec::new()") && pinned.is_empty() {
        problems.push(
            "the manifest pins real weights but none of their release URLs appear in crates/ai/src/manifest.rs's expected form".into(),
        );
    }

    // Every family an embedded build carried has to be attributed, once each.
    if let Ok(fonts) = fs::read_to_string("fonts/embedded.txt") {
        let mut families: Vec<&str> = fonts
            .lines()
            .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
            .map(|line| line.split('|').next().unwrap_or("").trim())
            .filter(|family| !family.is_empty())
            .collect();
        families.sort_unstable();
        families.dedup();
        for family in families {
            if !doc.contains(family) {
                problems.push(format!("the embedded font family \"{family}\" is not attributed"));
            }
        }
    }

    // And the honest entry for the half that ships no third-party data at all.
    if !doc.contains("No model weights") {
        problems.push("docs/ATTRIBUTION.md must say that no model weights, corpora or datasets ship with this project".into());
    }

    println!(
        "xtask attribution: {} linked, {} declared but not linked",
        linked.len(),
        pending.len()
    );
    report("attribution", problems)
}

/// The parts of the document the checks care about, split by heading.
fn split_attribution(doc: &str) -> (String, String) {
    let mut linked = String::new();
    let mut pending = String::new();
    let mut current: &mut String = &mut linked;
    for line in doc.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            current = if trimmed.to_lowercase().contains("not linked") {
                &mut pending
            } else {
                &mut linked
            };
        }
        current.push_str(line);
        current.push('\n');
    }
    (linked, pending)
}

/// Backticked names at the start of a table row, which is how a dependency row is written.
fn crate_names_in(section: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let Some(start) = trimmed.find('`') else {
            continue;
        };
        let rest = &trimmed[start + 1..];
        let Some(end) = rest.find('`') else {
            continue;
        };
        let name = &rest[..end];
        if name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            names.push(name.to_string());
        }
    }
    names
}

/// Every custom-painted clickable region has to say what it is.
///
/// A grid cell, a contact sheet and a cull strip are painted rather than built from widgets, so nothing
/// gives them a name: without `a11y::label_widget` a screen reader sees empty space where the photo is.
/// This walks the shell function by function — function granularity, because a line-level rule would break
/// the first time a call is reformatted — and fails when a region that can be clicked registers nothing.
///
/// It cannot check the emitted tree: this egui version has no public way to read accessibility nodes back,
/// so the labels themselves are tested as pure functions and this check covers the plumbing. Both halves
/// are named in ROADMAP so nobody mistakes one for the other.
fn a11y() -> Result<(), String> {
    let mut problems = Vec::new();
    for file in ["crates/ui/src/shell.rs"] {
        let text = fs::read_to_string(file).map_err(|error| format!("{file}: {error}"))?;
        let mut current: Option<(String, bool, bool)> = None;
        let mut functions: Vec<(String, bool, bool)> = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim_start();
            let starts_function = (trimmed.starts_with("fn ") || trimmed.starts_with("pub fn ")) && trimmed.contains('(');
            if starts_function {
                if let Some(done) = current.take() {
                    functions.push(done);
                }
                let name = trimmed
                    .split('(')
                    .next()
                    .unwrap_or(trimmed)
                    .trim_start_matches("pub ")
                    .trim_start_matches("fn ")
                    .to_string();
                current = Some((name, false, false));
                continue;
            }
            if let Some((_, clickable, named)) = current.as_mut() {
                if line.contains("Sense::click()") || line.contains("Sense::click_and_drag()") {
                    *clickable = true;
                }
                if line.contains("a11y::label_widget(") {
                    *named = true;
                }
            }
        }
        if let Some(done) = current.take() {
            functions.push(done);
        }
        for (name, clickable, named) in functions {
            if clickable && !named {
                problems.push(format!(
                    "{file}: {name} makes a region clickable without naming it for assistive technology"
                ));
            }
        }
    }
    report("a11y", problems)
}

/// Run the measurement binary in release mode and let it print its own table.
///
/// Deliberately outside `ci`. A timing that fails on a loaded CI box teaches people to ignore the check,
/// and a timing that passes there says nothing about a library on someone's laptop. The numbers live in
/// `docs/PERF.md` next to the machine they came from; this only makes them reproducible.
fn perf() -> Result<(), String> {
    let passthrough: Vec<String> = std::env::args().skip(2).collect();
    let mut args = vec!["run", "-q", "--release", "-p", "sortlens-perf", "--"];
    args.extend(passthrough.iter().map(String::as_str));
    run("cargo", &args)
}

/// The documents must agree with the tracker, and every crate and document must be named.
///
/// Three drifts this exists to catch, all of which happened at least once while the plan ran:
///
/// 1. **README's status said M0–M8** while M10 had shipped. The milestone range is now computed from
///    `docs/ROADMAP.md` — the highest milestone with a ticked row — and required in both `README.md` and
///    `docs/PLAN.md`, so prose cannot quietly fall behind the checkboxes.
/// 2. **New crates and apps went unlisted** (`sortlens-icon`, `sortlens-perf` were added to the workspace
///    and the layer diagram in `docs/ARCHITECTURE.md` only by hand, if at all).
/// 3. **`docs/PERF.md` existed and the README's document table never mentioned it**, so the measurements
///    were reachable only if you already knew the file name.
///
/// Deliberately not a prose linter: it checks facts that have one right answer (a range, a path, a name),
/// not how they are phrased.
fn docs() -> Result<(), String> {
    let mut problems = Vec::new();
    let readme = fs::read_to_string("README.md").map_err(|error| format!("README.md: {error}"))?;
    let plan = fs::read_to_string("docs/PLAN.md").map_err(|error| format!("docs/PLAN.md: {error}"))?;
    let roadmap = fs::read_to_string("docs/ROADMAP.md").map_err(|error| format!("docs/ROADMAP.md: {error}"))?;
    let architecture = fs::read_to_string("docs/ARCHITECTURE.md").map_err(|error| format!("docs/ARCHITECTURE.md: {error}"))?;

    let Some(shipped) = shipped_through(&roadmap) else {
        problems.push("docs/ROADMAP.md has no milestone with a ticked row".to_string());
        return report("docs", problems);
    };
    let range = format!("M0–M{shipped}");
    // The badge is percent-encoded for the dash, which is how it silently kept saying M0–M7.
    let badge = format!("M0%E2%80%93M{shipped}");
    if !readme.contains(&range) {
        problems.push(format!("README.md never says {range}, which is what docs/ROADMAP.md has shipped"));
    }
    if !readme.contains(&badge) {
        problems.push(format!("README.md's status badge does not say {range} (encoded as {badge})"));
    }
    if !plan.contains(&range) {
        problems.push(format!("docs/PLAN.md §12 never says {range}"));
    }

    // Every crate and app in the workspace is named in the layer diagram.
    for entry in fs::read_dir("crates").map_err(|error| format!("crates: {error}"))? {
        let path = entry.map_err(|error| format!("crates: {error}"))?.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let package = format!("sortlens-{name}");
        if !architecture.contains(&package) {
            problems.push(format!("crates/{name} is not in docs/ARCHITECTURE.md's layer diagram ({package})"));
        }
    }
    for entry in fs::read_dir("apps").map_err(|error| format!("apps: {error}"))? {
        let path = entry.map_err(|error| format!("apps: {error}"))?.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !architecture.contains(name) {
            problems.push(format!("apps/{name} is not named in docs/ARCHITECTURE.md"));
        }
    }

    // Every document is reachable from the README.
    for entry in fs::read_dir("docs").map_err(|error| format!("docs: {error}"))? {
        let path = entry.map_err(|error| format!("docs: {error}"))?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !readme.contains(name) {
            problems.push(format!("docs/{name} is not mentioned anywhere in README.md"));
        }
    }
    report("docs", problems)
}

/// The highest milestone in `docs/ROADMAP.md` with at least one ticked row.
fn shipped_through(roadmap: &str) -> Option<u32> {
    let mut shipped = None;
    let mut current: Option<u32> = None;
    for line in roadmap.lines() {
        if let Some(rest) = line.strip_prefix("## M")
            && let Some(number) = rest
                .split_whitespace()
                .next()
                .and_then(|word| word.trim_end_matches('—').parse::<u32>().ok())
        {
            current = Some(number);
            continue;
        }
        if line.starts_with("## ") {
            current = None;
            continue;
        }
        if line.trim_start().starts_with("- [x]")
            && let Some(number) = current
        {
            shipped = Some(shipped.map_or(number, |best: u32| best.max(number)));
        }
    }
    shipped
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
