//! The GitHub intake: the issue forms and the pull-request checklist.
//!
//! The forms are the only door a stranger's report comes through, and GitHub answers a malformed
//! one by refusing to render it — from the reporter's side, silently. So `cargo xtask templates`
//! checks them against the shape GitHub accepts: the keys an issue form carries, the element types it
//! renders, an id on every field, a label on everything a person fills in, options where the type
//! needs them, at least one required field, and — for the bug form — the diagnostics bundle this
//! project actually prints. The pull-request template is checked for the markers a reviewer needs:
//! the linked issue, the failing-then-passing test, every gate as a checkbox, the performance table
//! for scan/catalog/thumbs and the before-and-after for `crates/ui`.
//!
//! Deliberately not a prose linter: it checks facts that have one right answer, not how a sentence
//! is phrased, and it never claims what GitHub itself will decide.

use std::fs;
use std::path::Path;

use crate::yaml::{self, Node};

const FORM_KEYS: &[&str] = &["name", "description", "title", "labels", "assignees", "projects", "type", "body"];
const ELEMENT_KEYS: &[&str] = &["type", "id", "attributes", "validations"];
const ELEMENT_TYPES: &[&str] = &["markdown", "input", "textarea", "dropdown", "checkboxes", "upload"];
const OPTION_KEYS: &[&str] = &["label", "required", "description"];
const GATES: &[&str] = &[
    "cargo xtask ci",
    "cargo xtask snapshot",
    "cargo xtask deny",
    "cargo xtask wasm",
    "cargo xtask screenshots",
];

/// Which form a file is, so the checks only a bug report needs can run.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Bug,
    Feature,
}

/// Every problem the intake has, each naming the file it was found in.
pub fn check(root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    let forms = [
        (
            "bug_report.yml",
            Form::Bug,
            &["version", "os", "os_version", "repro", "expected", "actual", "diagnostics"][..],
        ),
        ("feature_request.yml", Form::Feature, &["problem", "proposal", "area"][..]),
    ];
    for (file, kind, required) in forms {
        let label = format!(".github/ISSUE_TEMPLATE/{file}");
        match fs::read_to_string(root.join(".github/ISSUE_TEMPLATE").join(file)) {
            Err(error) => problems.push(format!("{label}: {error}")),
            Ok(text) => problems.extend(form_problems(&label, &text, required, kind)),
        }
    }
    let label = ".github/PULL_REQUEST_TEMPLATE.md";
    match fs::read_to_string(root.join(label)) {
        Err(error) => problems.push(format!("{label}: {error}")),
        Ok(text) => problems.extend(pull_request_problems(label, &text)),
    }
    problems
}

fn form_problems(label: &str, text: &str, required: &[&str], kind: Form) -> Vec<String> {
    let node = match yaml::parse(text) {
        Ok(node) => node,
        Err(error) => return vec![format!("{label}: {error}")],
    };
    let mut problems = shape_problems(label, &node, required);
    if kind == Form::Bug {
        problems.extend(diagnostics_problems(label, &node));
    }
    problems
}

fn shape_problems(label: &str, node: &Node, required: &[&str]) -> Vec<String> {
    let mut problems = Vec::new();
    let Some(fields) = node.as_map() else {
        return vec![format!("{label}: the document is not a mapping")];
    };
    for (key, _) in fields {
        if !FORM_KEYS.contains(&key.as_str()) {
            problems.push(format!("{label}: `{key}` is not a key an issue form carries"));
        }
    }
    check_text(&mut problems, label, node, "name", 64);
    check_text(&mut problems, label, node, "description", 200);
    if let Some(title) = node.field("title")
        && title.as_str().is_none()
    {
        problems.push(format!("{label}: `title` must be a string"));
    }
    match node.field("labels").and_then(Node::as_list) {
        None => problems.push(format!("{label}: no `labels` list, so a filed report lands unlabelled")),
        Some([]) => problems.push(format!("{label}: `labels` is empty")),
        Some(labels) => {
            for entry in labels {
                if entry.as_str().is_none_or(|text| text.trim().is_empty()) {
                    problems.push(format!("{label}: every entry in `labels` must be a non-empty string"));
                }
            }
        }
    }
    let Some(body) = node.field("body") else {
        problems.push(format!("{label}: no `body`, so the form asks nothing"));
        return problems;
    };
    let Some(elements) = body.as_list() else {
        problems.push(format!("{label}: `body` must be a list of fields"));
        return problems;
    };
    if elements.is_empty() {
        problems.push(format!("{label}: `body` is empty"));
        return problems;
    }
    let mut ids: Vec<String> = Vec::new();
    let mut required_fields = 0;
    for (index, element) in elements.iter().enumerate() {
        let at = format!("{label}: body[{index}]");
        let (element_problems, required_here) = one_element(&at, element, &mut ids);
        problems.extend(element_problems);
        required_fields += required_here;
    }
    if required_fields == 0 {
        problems.push(format!("{label}: no field is required, so an empty issue is a valid report"));
    }
    for wanted in required {
        if !ids.iter().any(|id| id == wanted) {
            problems.push(format!("{label}: no field with id `{wanted}`"));
        }
    }
    problems
}

/// One `body` entry: what GitHub will render, and whether it must be filled in.
fn one_element(at: &str, element: &Node, ids: &mut Vec<String>) -> (Vec<String>, usize) {
    let mut problems = Vec::new();
    let Some(fields) = element.as_map() else {
        return (vec![format!("{at} is not a mapping")], 0);
    };
    for (key, _) in fields {
        if !ELEMENT_KEYS.contains(&key.as_str()) {
            problems.push(format!("{at} carries `{key}`, which an issue-form element does not accept"));
        }
    }
    let Some(kind) = element.field("type").and_then(Node::as_str) else {
        return (vec![format!("{at} has no `type`")], 0);
    };
    if !ELEMENT_TYPES.contains(&kind) {
        return (vec![format!("{at} has type `{kind}`, which GitHub does not render")], 0);
    }
    if kind != "markdown" {
        match element.field("id").and_then(Node::as_str) {
            None => problems.push(format!("{at} ({kind}) has no `id`")),
            Some(id) if !valid_id(id) => {
                problems.push(format!("{at} has id `{id}`; use lowercase letters, digits, `-` and `_`"));
            }
            Some(id) if ids.iter().any(|known| known == id) => problems.push(format!("{at} repeats the id `{id}`")),
            Some(id) => ids.push(id.to_string()),
        }
    }
    match element.field("attributes") {
        None => problems.push(format!("{at} has no `attributes`")),
        Some(attributes) => problems.extend(attributes_problems(at, kind, attributes)),
    }
    let required = match element.field("validations") {
        None => 0,
        Some(validations) => match validations.as_map() {
            None => {
                problems.push(format!("{at}: `validations` must be a mapping"));
                0
            }
            Some(entries) => {
                let mut required = 0;
                for (key, value) in entries {
                    match (key.as_str(), value.as_str()) {
                        ("required", Some("true")) => required += 1,
                        ("required", Some("false")) => {}
                        ("required", _) => problems.push(format!("{at}: `validations.required` is `true` or `false`")),
                        _ => problems.push(format!("{at}: `validations.{key}` does not exist")),
                    }
                }
                required
            }
        },
    };
    (problems, required)
}

fn attributes_problems(at: &str, kind: &str, attributes: &Node) -> Vec<String> {
    let Some(attributes) = attributes.as_map() else {
        return vec![format!("{at}: `attributes` must be a mapping")];
    };
    let mut problems = Vec::new();
    let allowed = allowed_attributes(kind);
    for (key, _) in attributes {
        if !allowed.contains(&key.as_str()) {
            problems.push(format!("{at}: `attributes.{key}` is not something a {kind} accepts"));
        }
    }
    if kind == "markdown" {
        if !text_field(attributes, "value") {
            problems.push(format!("{at} (markdown) needs a non-empty `value`"));
        }
    } else if !text_field(attributes, "label") {
        problems.push(format!("{at} ({kind}) needs a non-empty `label`"));
    }
    if kind == "dropdown" || kind == "checkboxes" {
        problems.extend(option_problems(at, kind, attributes));
    }
    problems
}

fn option_problems(at: &str, kind: &str, attributes: &[(String, Node)]) -> Vec<String> {
    let Some(options) = attributes.iter().find(|(name, _)| name == "options").map(|(_, value)| value) else {
        return vec![format!("{at} ({kind}) needs `options`")];
    };
    let Some(options) = options.as_list() else {
        return vec![format!("{at}: `options` must be a list")];
    };
    if options.is_empty() {
        return vec![format!("{at}: `options` is empty")];
    }
    let mut problems = Vec::new();
    for option in options {
        if kind == "dropdown" {
            if option.as_str().is_none_or(|text| text.trim().is_empty()) {
                problems.push(format!("{at}: every dropdown option must be a non-empty string"));
            }
            continue;
        }
        let Some(fields) = option.as_map() else {
            problems.push(format!("{at}: every checkbox option is a mapping with a `label`"));
            continue;
        };
        for (key, _) in fields {
            if !OPTION_KEYS.contains(&key.as_str()) {
                problems.push(format!("{at}: a checkbox option carries `{key}`, which GitHub does not accept"));
            }
        }
        if !text_field(fields, "label") {
            problems.push(format!("{at}: every checkbox option needs a non-empty `label`"));
        }
    }
    problems
}

/// The bug form is only actionable if the bundle arrives with it.
fn diagnostics_problems(label: &str, node: &Node) -> Vec<String> {
    let Some(body) = node.field("body").and_then(Node::as_list) else {
        return Vec::new();
    };
    let Some(field) = body
        .iter()
        .find(|element| element.field("id").and_then(Node::as_str) == Some("diagnostics"))
    else {
        return Vec::new();
    };
    let mut problems = Vec::new();
    if field.field("type").and_then(Node::as_str) != Some("textarea") {
        problems.push(format!("{label}: the `diagnostics` field must be a textarea"));
    }
    let required = field.field("validations").and_then(|node| node.field("required"));
    if required.and_then(Node::as_str) != Some("true") {
        problems.push(format!("{label}: the `diagnostics` field must be required"));
    }
    let description = field
        .field("attributes")
        .and_then(|node| node.field("description"))
        .and_then(Node::as_str)
        .unwrap_or_default();
    if !description.contains("sortlens-cli diagnostics") {
        problems.push(format!(
            "{label}: the `diagnostics` description must name `sortlens-cli diagnostics`"
        ));
    }
    problems
}

fn pull_request_problems(label: &str, text: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let lower = text.to_lowercase();
    if !lower.contains("closes #") {
        problems.push(format!("{label}: no `Closes #` line, so the pull request names no issue"));
    }
    if !lower.contains("test that fails without this change") {
        problems.push(format!("{label}: no failing-then-passing test prompt"));
    }
    for gate in GATES {
        let required = text
            .lines()
            .any(|line| line.trim_start().starts_with("- [ ]") && line.contains(gate));
        if !required {
            problems.push(format!("{label}: `{gate}` is not a required checkbox"));
        }
    }
    for marker in ["cargo xtask perf", "scan", "catalog", "thumbs"] {
        if !lower.contains(marker) {
            problems.push(format!("{label}: the performance section never says `{marker}`"));
        }
    }
    for marker in ["crates/ui", "baseline"] {
        if !lower.contains(marker) {
            problems.push(format!("{label}: the screenshot section never says `{marker}`"));
        }
    }
    problems
}

fn check_text(problems: &mut Vec<String>, label: &str, node: &Node, key: &str, limit: usize) {
    match node.field(key).and_then(Node::as_str) {
        None => problems.push(format!("{label}: no `{key}`")),
        Some(text) if text.trim().is_empty() => problems.push(format!("{label}: `{key}` is empty")),
        Some(text) if text.chars().count() > limit => {
            problems.push(format!(
                "{label}: `{key}` is {} characters, past GitHub's limit of {limit}",
                text.chars().count()
            ));
        }
        Some(_) => {}
    }
}

fn allowed_attributes(kind: &str) -> &'static [&'static str] {
    match kind {
        "markdown" => &["value"],
        "input" => &["label", "description", "placeholder", "value"],
        "textarea" => &["label", "description", "placeholder", "value", "render"],
        "dropdown" => &["label", "description", "multiple", "options", "default"],
        "checkboxes" => &["label", "description", "options"],
        "upload" => &["label", "description"],
        _ => &[],
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-' || character == '_')
}

fn text_field(attributes: &[(String, Node)], key: &str) -> bool {
    attributes
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(_, value)| value.as_str())
        .is_some_and(|text| !text.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
name: A form
description: What it is for.
title: "[Bug]: "
labels:
  - bug
body:
  - type: markdown
    attributes:
      value: |
        Start here.
  - type: input
    id: os
    attributes:
      label: Operating system
    validations:
      required: true
"#;

    fn shape(document: &str, required: &[&str]) -> Vec<String> {
        let node = yaml::parse(document).expect("the fixture is YAML this reader accepts");
        shape_problems("test", &node, required)
    }

    #[test]
    fn the_shipped_intake_passes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let problems = check(&root);
        assert!(problems.is_empty(), "{problems:#?}");
    }

    #[test]
    fn a_valid_form_has_no_problems() {
        assert!(shape(MINIMAL, &["os"]).is_empty(), "{:#?}", shape(MINIMAL, &["os"]));
    }

    #[test]
    fn a_missing_required_field_is_named() {
        let problems = shape(MINIMAL, &["os", "diagnostics"]);
        assert!(problems.iter().any(|problem| problem.contains("`diagnostics`")), "{problems:#?}");
    }

    #[test]
    fn a_form_with_no_required_field_is_refused() {
        let document = MINIMAL.replace("    validations:\n      required: true\n", "");
        let problems = shape(&document, &["os"]);
        assert!(
            problems.iter().any(|problem| problem.contains("no field is required")),
            "{problems:#?}"
        );
    }

    #[test]
    fn a_duplicate_id_is_refused() {
        let document = MINIMAL.replace(
            "- type: input\n    id: os",
            "- type: input\n    id: os\n  - type: input\n    id: os",
        );
        let problems = shape(&document, &["os"]);
        assert!(problems.iter().any(|problem| problem.contains("repeats the id")), "{problems:#?}");
    }

    #[test]
    fn an_unrenderable_element_type_is_refused() {
        let document = MINIMAL.replace("type: input", "type: slider");
        let problems = shape(&document, &["os"]);
        assert!(
            problems.iter().any(|problem| problem.contains("GitHub does not render")),
            "{problems:#?}"
        );
    }

    #[test]
    fn an_unknown_attribute_is_refused() {
        let document = MINIMAL.replace("label: Operating system", "lable: Operating system");
        let problems = shape(&document, &["os"]);
        assert!(problems.iter().any(|problem| problem.contains("attributes.lable")), "{problems:#?}");
        assert!(
            problems.iter().any(|problem| problem.contains("needs a non-empty `label`")),
            "{problems:#?}"
        );
    }

    #[test]
    fn a_dropdown_without_options_is_refused() {
        let document = MINIMAL.replace(
            "  - type: input\n    id: os\n    attributes:\n      label: Operating system",
            "  - type: dropdown\n    id: os\n    attributes:\n      label: Operating system",
        );
        let problems = shape(&document, &["os"]);
        assert!(problems.iter().any(|problem| problem.contains("needs `options`")), "{problems:#?}");
    }

    #[test]
    fn a_bug_form_without_the_diagnostics_command_is_refused() {
        let document = MINIMAL.replace(
            "  - type: input\n    id: os",
            "  - type: textarea\n    id: diagnostics\n    attributes:\n      label: Diagnostics\n      description: Paste it.\n    validations:\n      required: true\n  - type: input\n    id: os",
        );
        let node = yaml::parse(&document).expect("the fixture is readable");
        let problems = diagnostics_problems("test", &node);
        assert!(
            problems.iter().any(|problem| problem.contains("sortlens-cli diagnostics")),
            "{problems:#?}"
        );
        let named = document.replace("Paste it.", "Paste the output of `sortlens-cli diagnostics`.");
        let node = yaml::parse(&named).expect("the fixture is readable");
        assert!(diagnostics_problems("test", &node).is_empty());
    }

    #[test]
    fn a_pull_request_template_without_a_gate_is_refused() {
        let template = concat!(
            "Closes #1\n\n## The test that fails without this change\n\n- Test: `one`\n\n## Gates\n\n",
            "- [ ] `cargo xtask ci`\n",
            "- [ ] `cargo xtask snapshot`\n",
            "- [ ] `cargo xtask deny`\n",
            "- [ ] `cargo xtask wasm`\n",
            "- [ ] `cargo xtask screenshots`\n\n",
            "`cargo xtask perf` before and after for scan, catalog, thumbs. `crates/ui` baselines.\n",
        );
        assert!(
            pull_request_problems("test", template).is_empty(),
            "{:#?}",
            pull_request_problems("test", template)
        );
        let without_deny = template.replace("- [ ] `cargo xtask deny`\n", "");
        let problems = pull_request_problems("test", &without_deny);
        assert!(problems.iter().any(|problem| problem.contains("cargo xtask deny")), "{problems:#?}");
    }
}
