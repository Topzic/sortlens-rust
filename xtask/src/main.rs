//! Repository tasks.
//!
//! `cargo xtask ci` is what CI runs, so a green local run means a green build. Every check is
//! cheap enough to run on each commit; none of them need network access.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

mod intake;
mod workflow;
mod yaml;

// The manifest reader crates/ui's build script uses, so the phone trim and the font report
// cannot disagree with the build about which faces a `CRAFT_FONTS_SCRIPTS` filter keeps.
#[path = "../../crates/ui/src/font_manifest.rs"]
mod font_manifest;

const LIB_CRATES: &[&str] = &[
    "core", "hash", "quality", "testkit", "meta", "media", "sidecar", "geo", "ai", "thumbs", "scan", "catalog", "cull", "insights",
    "similar", "remote", "engine", "ui", "mcp",
];

const APP_CRATES: &[&str] = &["sortlens", "sortlens-cli", "sortlens-mobile", "sortlens-server"];

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
        "mobile" => mobile(),
        "layers" => layers(),
        "roots" => roots(),
        "assets" => assets(),
        "icons" => icons(),
        "fonts" => fonts(),
        "deny" => deny(),
        "snapshot" => snapshot(),
        "screenshots" => screenshots(),
        "version" => version(),
        "attribution" => attribution(),
        "a11y" => a11y(),
        "perf" => perf(),
        "docs" => docs(),
        "templates" => templates(),
        "workflows" => workflows(),
        "intake" => intake_check(),
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
  mobile   check the phone app and the crates under it for Android and iOS (`mobile android|ios` for one),
           asking the CRAFT_FONTS_SCRIPTS trim a phone build embeds with
  layers   enforce the dependency layering rules
  roots    enforce the crate-level lint header
  assets   enforce that every asset has a provenance entry
  icons    check assets/icon.ico and assets/icon.icns against the drawing; --update rewrites them
  fonts    report the embedded face set and refuse one that misses a script the interface ships
  deny     check licences, advisories, bans and sources against deny.toml
  snapshot golden CLI output; --update rewrites the goldens, then read the diff
  screenshots  paint the shell through egui_kittest and diff the baselines; --update rewrites them
  version  one version for the workspace, declared once
  attribution  every dependency attributed, and the document honest about what is linked
  a11y     every custom-painted clickable region registers a name
  perf     measurements for docs/PERF.md (not in ci: a timing that fails on a loaded box gets ignored)
  docs     the documents agree with the tracker, and every crate and document is named
  templates  the GitHub issue forms and the pull-request checklist are the shape GitHub accepts
  workflows  the gates a pull request meets, the intake that reads a report, and the playbook's rules
  intake   check a filed report (SORTLENS_ISSUE_BODY, SORTLENS_ISSUE_LABELS) against its form"
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
    templates()?;
    workflows()?;
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

/// Check the phone app (SL-J10) for the targets it ships on: `cargo xtask mobile [android|ios]`, both
/// when no platform is named. The crates under it are the desktop's own, so a target that stops
/// compiling is a regression in shared code, caught here rather than at the release build. Android
/// needs the NDK (`ANDROID_NDK_HOME`) and iOS needs Xcode, because `ring` and `tract-linalg` compile
/// C and assembly; the Rust targets come from `scripts/bootstrap.sh`.
fn mobile() -> Result<(), String> {
    let platform = std::env::args().nth(2);
    let targets: &[&str] = match platform.as_deref() {
        None => &["aarch64-linux-android", "aarch64-apple-ios"],
        Some("android") => &["aarch64-linux-android"],
        Some("ios") => &["aarch64-apple-ios"],
        Some(other) => return Err(format!("mobile knows android and ios, not {other}")),
    };
    // The phone's trimmed font set (roadmap 3b): a phone build that embeds craft-fonts keeps only
    // the faces the shipped interface languages need, not the whole manifest — a desktop luxury
    // a phone binary does not pay for. Without a checkout to embed from this is a no-op.
    let env: Vec<(String, String)> = if std::env::var_os("CRAFT_FONTS_DIR").is_some() {
        vec![("CRAFT_FONTS_SCRIPTS".to_string(), font_manifest::PHONE_SCRIPTS.join(","))]
    } else {
        Vec::new()
    };
    for target in targets {
        run_env("cargo", &["check", "-p", "sortlens-mobile", "--target", target], &env)?;
    }
    Ok(())
}

/// [`run`] with environment variables set for the child only.
fn run_env(program: &str, args: &[&str], env: &[(String, String)]) -> Result<(), String> {
    let mut command = Command::new(program);
    command.args(args);
    for (key, value) in env {
        command.env(key, value);
    }
    let status = command.status().map_err(|error| format!("could not run {program}: {error}"))?;
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

        // Only the ui crate may know that a window exists. `craft-update` is not a window crate:
        // the engine takes it with default features (no egui) for the update check, and the pin's
        // egui feature is exactly what it must not enable.
        if *name != "ui" {
            for dep in &deps {
                if UI_ONLY.contains(&dep.as_str()) && !(*name == "engine" && dep == "craft-update") {
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
/// silent success — but a set that *does* embed faces and misses a script the interface ships (its
/// own two, or one a shipped translation is written in) is refused, and so is a `CRAFT_FONTS_SCRIPTS`
/// filter the manifest cannot satisfy. The rules are the manifest reader's, shared with the build
/// script so a report and a build cannot disagree.
fn fonts() -> Result<(), String> {
    let manifest = Path::new("fonts/embedded.txt");
    let dir = std::env::var_os("CRAFT_FONTS_DIR").map(std::path::PathBuf::from);
    let filter_value = std::env::var("CRAFT_FONTS_SCRIPTS").ok();
    let filter = font_manifest::parse_filter(filter_value.as_deref());
    let mut report = Vec::new();
    match &dir {
        None => {
            let trimmed = if filter.is_empty() {
                String::new()
            } else {
                format!("; CRAFT_FONTS_SCRIPTS={} would trim a checkout build", filter.join(","))
            };
            println!("xtask fonts: not embedded (CRAFT_FONTS_DIR is unset) — egui's own faces are used{trimmed}");
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
            let parsed = font_manifest::parse(&text)?;
            let kept = font_manifest::select(&parsed, &filter).map_err(|error| format!("CRAFT_FONTS_SCRIPTS: {error}"))?;
            let mut families: Vec<&str> = kept.iter().map(|face| face.family).collect();
            families.sort_unstable();
            families.dedup();
            if filter.is_empty() {
                report.push(format!(
                    "{} face(s) in {} family(ies): {}",
                    kept.len(),
                    families.len(),
                    families.join(", ")
                ));
            } else {
                report.push(format!(
                    "{} of {} face(s) in {} family(ies) after the CRAFT_FONTS_SCRIPTS filter ({}): {}",
                    kept.len(),
                    parsed.len(),
                    families.len(),
                    filter.join(","),
                    families.join(", ")
                ));
            }
            // The same rule the build enforces on itself: the embedded set must cover the
            // interface's own scripts and every shipped language's — a release that offers a
            // language it cannot paint is a defect, not a note.
            if let Some(problem) = font_manifest::validate(&text, &filter)?.into_iter().next() {
                return Err(problem);
            }
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

/// Check the GitHub intake: the issue forms GitHub will parse, and the pull-request checklist.
///
/// A malformed issue form fails from the reporter's side, where nobody here sees it, and the
/// checklist is the only place the gates are written down for a contributor. Both are checked in
/// `ci` for the reason the documents are: a promise nobody verifies has already drifted.
fn templates() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let problems = intake::check(&root);
    report("templates", problems)
}

/// Check the GitHub workflows and the autonomous-agent playbook (ROADMAP Phase 5 items 2 and 3).
///
/// Two promises live only in these files: that every gate the pull-request template names is run by
/// a workflow that can fail, and that the intake that reads a stranger's issue can label it and
/// reach nothing else. The playbook is checked for the rules it is the only copy of. All three are
/// facts, not phrasing, and every one of them is a promise nobody would notice going stale.
fn workflows() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let problems = workflow::check(&root);
    report("workflows", problems)
}

/// Check a filed report against the form it came through, for `agent-intake.yml`.
///
/// The body and labels arrive through the environment because that is how a workflow can pass a
/// stranger's text without interpolating it into a script line. The output is the comment the
/// intake posts: which required headings are absent or empty. Exit status is the answer; the
/// workflow labels the issue rather than failing a run a reporter cannot act on.
fn intake_check() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let labels = std::env::var("SORTLENS_ISSUE_LABELS").unwrap_or_default();
    let body = std::env::var("SORTLENS_ISSUE_BODY").unwrap_or_default();
    if crate::intake::form_for_labels(&labels).is_none() {
        println!("intake: neither a bug nor a feature report; nothing to check");
        return Ok(());
    }
    let problems = crate::intake::report_problems(&root, &labels, &body);
    if problems.is_empty() {
        println!("intake: the report carries every field its form asks for");
        return Ok(());
    }
    println!("Thanks for the report. Its form asks for the fields below, and these are missing or empty:");
    for problem in &problems {
        println!("- {problem}");
    }
    Err(format!("intake: {} field(s) missing or incomplete", problems.len()))
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

/// Check the committed packaging icons against the drawing, or rewrite them with `--update`.
///
/// The Windows `.ico` and the macOS `.icns` are generated bytes — renders of the one drawing in
/// `crates/icon`, packed per format — so they are checked the way the SVG is: the `sortlens-icon`
/// tests compare them byte for byte and run in `cargo test --workspace`, meaning `cargo xtask ci`
/// already refuses a stale file. This task is the discoverable front door: without `--update` it
/// reports and fails on a mismatch, with `--update` it rewrites both files, and the diff is left to
/// `git diff` and a human eye, the way `snapshot --update` works.
fn icons() -> Result<(), String> {
    let update = std::env::args().any(|arg| arg == "--update");
    let mut problems = Vec::new();
    for (path, bytes) in [
        ("assets/icon.ico", sortlens_icon::ico()),
        ("assets/icon.icns", sortlens_icon::icns()),
    ] {
        match fs::read(path) {
            Ok(on_disk) if on_disk == bytes => println!("xtask icons: {path} matches the drawing ({} bytes)", bytes.len()),
            Ok(_) if update => {
                fs::write(path, &bytes).map_err(|error| format!("{}: {error}", path))?;
                println!(
                    "xtask icons: rewrote {path} ({} bytes) — read `git diff` and open the file",
                    bytes.len()
                );
            }
            Ok(_) => problems.push(format!(
                "{path} does not match the drawing in crates/icon; run: cargo xtask icons --update"
            )),
            Err(_) if update => {
                fs::write(path, &bytes).map_err(|error| format!("{}: {error}", path))?;
                println!("xtask icons: wrote {path} ({} bytes)", bytes.len());
            }
            Err(_) => problems.push(format!("{path} is missing; run: cargo xtask icons --update")),
        }
    }
    if update {
        println!("xtask icons: the committed copies live in git; a wrong file should be reverted, not re-edited");
        return Ok(());
    }
    report("icons", problems)
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
