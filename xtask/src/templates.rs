//! The GitHub intake templates, checked as facts rather than trusted.
//!
//! GitHub parses the YAML; this does not pretend to. It checks the part GitHub cannot check: that
//! the bug form still asks for the facts a report needs (OS, version, reproduction, expected vs
//! actual, diagnostics) and that every one of those fields is still required. A field quietly
//! relaxed to optional is how an intake form turns into a paragraph box, and a pull-request
//! template that stops naming a gate is how a red tree gets merged.
//!
//! Same bargain as `docs()`: facts with one right answer, never prose.

use std::fs;
use std::path::Path;

const BUG_FORM: &str = ".github/ISSUE_TEMPLATE/bug_report.yml";
const FEATURE_FORM: &str = ".github/ISSUE_TEMPLATE/feature_request.yml";
const PULL_REQUEST: &str = ".github/PULL_REQUEST_TEMPLATE.md";

/// The facts a bug report cannot be triaged without. `os-version` sits beside `os` because a
/// platform name alone does not reproduce a rendering or filesystem bug.
const BUG_REQUIRED: &[&str] = &["version", "os", "os-version", "repro", "expected", "actual", "diagnostics"];
const FEATURE_REQUIRED: &[&str] = &["problem", "proposal", "area"];

/// Headings and names the pull-request template may not lose. The commands are checked as
/// `- [ ]` lines below, so they cannot decay into prose a reviewer scrolls past.
const PULL_REQUEST_FACTS: &[&str] = &[
    "## Linked issue",
    "Closes #",
    "## The test that failed first",
    "## Gates",
    "## Performance",
    "## UI evidence",
    "egui_kittest",
    "crates/ui",
    "cargo xtask screenshots",
];

const PULL_REQUEST_CHECKS: &[&str] = &[
    "Clean room",
    "cargo xtask ci",
    "cargo xtask snapshot",
    "cargo xtask deny",
    "cargo xtask wasm",
    "cargo xtask perf",
];

pub fn run() -> Result<(), String> {
    crate::report("templates", check_root(Path::new(".")))
}

/// Every problem with the templates under `root`, without printing or failing: `run` reports
/// them from the checkout, and the tests below hand it fixtures and `CARGO_MANIFEST_DIR`.
pub fn check_root(root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    for (path, required) in [(BUG_FORM, BUG_REQUIRED), (FEATURE_FORM, FEATURE_REQUIRED)] {
        match fs::read_to_string(root.join(path)) {
            Ok(text) => problems.extend(issue_form_problems(&text, path, required)),
            Err(_) => problems.push(format!("{path} is missing; the intake loop reads it")),
        }
    }
    match fs::read_to_string(root.join(PULL_REQUEST)) {
        Ok(text) => problems.extend(pull_request_problems(&text)),
        Err(_) => problems.push(format!("{PULL_REQUEST} is missing; the gates are its checklist")),
    }
    problems
}

fn issue_form_problems(text: &str, path: &str, required: &[&str]) -> Vec<String> {
    let mut problems = Vec::new();
    for key in ["name:", "description:", "title:", "labels:", "body:"] {
        if !text.lines().any(|line| line.starts_with(key)) {
            problems.push(format!("{path} has no top-level `{key}`"));
        }
    }

    let mut asked: Vec<(String, bool)> = Vec::new();
    for entry in issue_form_entries(text) {
        match entry_problems(&entry, path) {
            Ok(Some((id, is_required))) => asked.push((id, is_required)),
            Ok(None) => {}
            Err(mut entry_problems) => problems.append(&mut entry_problems),
        }
    }

    for id in required {
        match asked.iter().find(|(asked, _)| asked == id) {
            None => problems.push(format!("{path} never asks for `{id}`")),
            Some((_, true)) => {}
            Some((_, false)) => problems.push(format!("{path}'s `{id}` is optional; it must be required")),
        }
    }
    problems
}

/// The `body:` entries of an issue form, one block of lines each. A block starts at the form's
/// own two-space `- type:` line; everything indented further belongs to the entry above it.
fn issue_form_entries(text: &str) -> Vec<Vec<&str>> {
    let mut entries: Vec<Vec<&str>> = Vec::new();
    for line in text.lines() {
        if line.starts_with("  - type: ") {
            entries.push(vec![line]);
        } else if let Some(entry) = entries.last_mut() {
            entry.push(line);
        }
    }
    entries
}

/// What one entry asks, or why it cannot be trusted. `Ok(None)` is a `markdown` block, which
/// asks nothing and is allowed to.
fn entry_problems(entry: &[&str], path: &str) -> Result<Option<(String, bool)>, Vec<String>> {
    let Some(first) = entry.first() else {
        return Ok(None);
    };
    let kind = first.trim_start_matches("  - type: ").trim();
    if kind == "markdown" {
        return Ok(None);
    }
    let Some(id) = field(entry, "id:") else {
        return Err(vec![format!("{path} has a `{kind}` element with no `id:`")]);
    };

    let mut problems = Vec::new();
    let is_required = match kind {
        "input" | "textarea" | "dropdown" => required_value(entry, &id, path, &mut problems),
        "checkboxes" => {
            if !entry.iter().any(|line| line.trim() == "required: true") {
                problems.push(format!("{path}'s `{id}` checkbox group has no box a reporter must tick"));
            }
            if entry.iter().any(|line| line.trim() == "validations:") {
                problems.push(format!(
                    "{path}'s `{id}` checkbox group carries `validations`, which GitHub ignores; require the options instead"
                ));
            }
            true
        }
        other => {
            problems.push(format!("{path}'s `{id}` uses `{other}`, which is not an issue-form element"));
            true
        }
    };

    if problems.is_empty() {
        Ok(Some((id, is_required)))
    } else {
        Err(problems)
    }
}

fn required_value(entry: &[&str], id: &str, path: &str, problems: &mut Vec<String>) -> bool {
    let Some(index) = entry.iter().position(|line| line.trim() == "validations:") else {
        problems.push(format!("{path}'s `{id}` never says whether it is required"));
        return false;
    };
    let Some(line) = entry[index + 1..].iter().find(|line| line.trim().starts_with("required:")) else {
        problems.push(format!("{path}'s `{id}` has a `validations:` block with no `required:`"));
        return false;
    };
    match line.trim().trim_start_matches("required:").trim() {
        "true" => true,
        "false" => false,
        other => {
            problems.push(format!("{path}'s `{id}` says `required: {other}`, which is neither true nor false"));
            false
        }
    }
}

fn field(entry: &[&str], key: &str) -> Option<String> {
    entry.iter().find_map(|line| {
        line.trim()
            .strip_prefix(key)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn pull_request_problems(text: &str) -> Vec<String> {
    let mut problems = Vec::new();
    for fact in PULL_REQUEST_FACTS {
        if !text.contains(fact) {
            problems.push(format!("{PULL_REQUEST} no longer says `{fact}`"));
        }
    }
    let checks: Vec<&str> = text.lines().map(str::trim).filter(|line| line.starts_with("- [ ]")).collect();
    for command in PULL_REQUEST_CHECKS {
        if !checks.iter().any(|line| line.contains(command)) {
            problems.push(format!("{PULL_REQUEST} has no `- [ ]` naming `{command}`"));
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    const A_COMPLETE_FORM: &str = r#"name: Bug report
description: Something is wrong
title: "[Bug]: "
labels: ["bug"]
body:
  - type: markdown
    attributes:
      value: |
        Read this first.
  - type: input
    id: version
    attributes:
      label: Version
    validations:
      required: true
  - type: textarea
    id: diagnostics
    attributes:
      label: Diagnostics
    validations:
      required: true
  - type: textarea
    id: extra
    attributes:
      label: Extra
    validations:
      required: false
  - type: checkboxes
    id: constraints
    attributes:
      label: Constraints
      options:
        - label: Works offline
          required: true
"#;

    const A_COMPLETE_PULL_REQUEST: &str = r#"## Linked issue

Closes #

## The test that failed first

- [ ] It failed first.

## Gates

- [ ] Clean room.
- [ ] `cargo xtask ci`
- [ ] `cargo xtask snapshot`
- [ ] `cargo xtask deny`
- [ ] `cargo xtask wasm`

## Performance

- [ ] `cargo xtask perf` run.

## UI evidence

`cargo xtask screenshots` and `egui_kittest`; `crates/ui` only.
"#;

    #[test]
    fn a_complete_issue_form_is_accepted() {
        let problems = issue_form_problems(A_COMPLETE_FORM, "form", &["version", "diagnostics"]);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn a_form_that_stops_asking_for_a_required_fact_is_named() {
        let broken = A_COMPLETE_FORM.replace(
            "  - type: textarea
    id: diagnostics",
            "  - type: textarea
    id: notes",
        );
        let problems = issue_form_problems(&broken, "form", &["version", "diagnostics"]);
        assert!(
            problems.iter().any(|problem| problem.contains("never asks for `diagnostics`")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_field_relaxed_to_optional_is_named() {
        let broken = A_COMPLETE_FORM.replace(
            "    id: diagnostics
    attributes:
      label: Diagnostics
    validations:
      required: true",
            "    id: diagnostics
    attributes:
      label: Diagnostics
    validations:
      required: false",
        );
        let problems = issue_form_problems(&broken, "form", &["version", "diagnostics"]);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("`diagnostics` is optional; it must be required")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_checkbox_group_with_no_required_box_is_named() {
        let broken = A_COMPLETE_FORM.replace(
            "- label: Works offline
          required: true",
            "- label: Works offline
          required: false",
        );
        let problems = issue_form_problems(&broken, "form", &["version", "diagnostics"]);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("checkbox group has no box a reporter must tick")),
            "{problems:?}"
        );
    }

    #[test]
    fn an_element_github_does_not_have_is_named() {
        let broken = A_COMPLETE_FORM.replace("  - type: input", "  - type: slider");
        let problems = issue_form_problems(&broken, "form", &["version", "diagnostics"]);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("which is not an issue-form element")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_complete_pull_request_template_is_accepted() {
        let problems = pull_request_problems(A_COMPLETE_PULL_REQUEST);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn a_pull_request_without_the_perf_checkbox_is_named() {
        let broken = A_COMPLETE_PULL_REQUEST.replace("- [ ] `cargo xtask perf` run.", "- `cargo xtask perf` run.");
        let problems = pull_request_problems(&broken);
        assert!(
            problems.iter().any(|problem| problem.contains("`cargo xtask perf`")),
            "{problems:?}"
        );
    }

    #[test]
    fn a_pull_request_that_forgets_the_issue_link_is_named() {
        let broken = A_COMPLETE_PULL_REQUEST.replace("Closes #", "See the issue.");
        let problems = pull_request_problems(&broken);
        assert!(
            problems.iter().any(|problem| problem.contains("no longer says `Closes #`")),
            "{problems:?}"
        );
    }

    #[test]
    fn the_repositorys_templates_are_valid() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let problems = check_root(&root);
        assert!(problems.is_empty(), "{problems:?}");
    }
}
