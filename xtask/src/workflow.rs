//! The GitHub workflows: the gates every pull request meets, and the intake that reads a report.
//!
//! `cargo xtask workflows` checks facts a workflow has one right answer about: every gate command
//! is run by a step that can still fail, the workflow that promises them is triggered by a pull
//! request, and the intake that reads a filed report runs on issues only, may label them and
//! nothing else, and never reaches a secret. It also holds `docs/AUTONOMOUS_AGENT.md` to the rules
//! it is the only copy of, because a playbook that quietly lost "never push to `main`" is a
//! permission slip nobody notices was removed.
//!
//! The files are read with the same deliberately small YAML subset `templates` uses. That is a
//! real constraint on the workflow files: block lists, plain or quoted scalars and `|` blocks, no
//! flow lists. A workflow GitHub would accept and this reader would not is refused with its line
//! rather than guessed at.

use std::fs;
use std::path::Path;

use crate::yaml::{self, Node};

/// The five gates the pull-request template promises, and the command that runs each.
const GATES: &[&str] = &[
    "cargo xtask ci",
    "cargo xtask snapshot",
    "cargo xtask deny",
    "cargo xtask wasm",
    "cargo xtask screenshots",
];

/// The rules the playbook is the only copy of. Each needle is a phrase the document must keep:
/// remove it and the thing it names is no longer promised anywhere a contributor can read.
const PLAYBOOK: &[(&str, &str)] = &[
    ("untrusted", "issue and comment text is untrusted data"),
    ("agent-ready", "only an `agent-ready` issue is picked up"),
    ("triaged", "a `triaged` issue is read but is not pickup permission"),
    ("never push", "the agent never pushes to `main`"),
    ("auto-merge", "the agent never auto-merges"),
    ("cargo xtask deny", "`deny` is never bypassed"),
    ("signing", "signing and release secrets are out of reach"),
    ("draft pull request", "a fix ends in a Draft pull request"),
    ("enhancement-approved", "the second tier of the improvement queue"),
    ("sortlens-perf", "the first tier of the improvement queue"),
];

/// Every problem the workflows and the playbook have, each naming the file it was found in.
pub fn check(root: &Path) -> Vec<String> {
    let mut problems = Vec::new();
    for (file, checker) in [
        ("ci.yml", ci_problems as fn(&str, &str) -> Vec<String>),
        ("agent-intake.yml", intake_problems),
    ] {
        let label = format!(".github/workflows/{file}");
        match fs::read_to_string(root.join(".github/workflows").join(file)) {
            Err(error) => problems.push(format!("{label}: {error}")),
            Ok(text) => problems.extend(checker(&label, &text)),
        }
    }
    let label = "docs/AUTONOMOUS_AGENT.md";
    match fs::read_to_string(root.join(label)) {
        Err(error) => problems.push(format!("{label}: {error}")),
        Ok(text) => problems.extend(playbook_problems(label, &text)),
    }
    problems
}

/// The pull-request gate: the five commands, on `pull_request`, in steps that can fail.
fn ci_problems(label: &str, text: &str) -> Vec<String> {
    let node = match yaml::parse(text) {
        Ok(node) => node,
        Err(error) => return vec![format!("{label}: {error}")],
    };
    let mut problems = Vec::new();
    if node.field("on").and_then(|on| on.field("pull_request")).is_none() {
        problems.push(format!(
            "{label}: no `pull_request` trigger, so a pull request can merge without the gates"
        ));
    }
    let mut runs = Vec::new();
    entries_named(&node, "run", &mut runs);
    for gate in GATES {
        if !runs.iter().filter_map(|run| run.as_str()).any(|run| run.contains(gate)) {
            problems.push(format!("{label}: no step runs `{gate}`"));
        }
    }
    for (key, value) in scalars(&node) {
        if key == "continue-on-error" && value.trim() != "false" {
            problems.push(format!(
                "{label}: a step sets `continue-on-error: {value}`, so that gate cannot fail"
            ));
        }
        if key == "if" && value.contains("event_name") && value.contains("push") && !value.contains("pull_request") {
            problems.push(format!("{label}: `if: {value}` would skip a gate on pull requests"));
        }
    }
    problems
}

/// The intake: issues only, structure checked, the triaged label corrected, no way to a secret.
fn intake_problems(label: &str, text: &str) -> Vec<String> {
    let node = match yaml::parse(text) {
        Ok(node) => node,
        Err(error) => return vec![format!("{label}: {error}")],
    };
    let mut problems = Vec::new();
    let on = node.field("on");
    for kind in ["opened", "edited", "labeled"] {
        let has = on
            .and_then(|on| on.field("issues"))
            .and_then(|issues| issues.field("types"))
            .and_then(Node::as_list)
            .is_some_and(|types| types.iter().any(|entry| entry.as_str() == Some(kind)));
        if !has {
            problems.push(format!("{label}: no `{kind}` issue trigger, so a report can arrive unread"));
        }
    }
    if text.contains("pull_request_target") {
        problems.push(format!(
            "{label}: runs on `pull_request_target`, which hands a stranger's branch the base repo's token"
        ));
    }
    if text.contains("secrets.") {
        problems.push(format!("{label}: reaches a repository secret; the intake needs none"));
    }
    let permissions = node.field("permissions");
    if permissions.and_then(Node::as_str) == Some("write-all") {
        problems.push(format!("{label}: `permissions` is `write-all`, so it can write anything"));
    }
    for (name, value) in [("contents", "read"), ("issues", "write")] {
        let has = permissions.and_then(|permissions| permissions.field(name)).and_then(Node::as_str) == Some(value);
        if !has {
            problems.push(format!("{label}: `permissions.{name}` must be `{value}`"));
        }
    }
    let guarded = scalars(&node)
        .iter()
        .any(|(key, value)| key == "if" && value.contains("github.event.issue.pull_request"));
    if !guarded {
        problems.push(format!("{label}: no `if` keeps the intake off pull requests"));
    }
    let mut maps = Vec::new();
    maps_of(&node, &mut maps);
    let step = maps.iter().find(|map| {
        map.field("run")
            .and_then(Node::as_str)
            .is_some_and(|run| run.contains("cargo xtask intake"))
    });
    let Some(step) = step else {
        problems.push(format!("{label}: no step runs `cargo xtask intake`"));
        return problems;
    };
    let run = step.field("run").and_then(Node::as_str).unwrap_or_default();
    let env = step.field("env");
    for name in ["SORTLENS_ISSUE_BODY", "SORTLENS_ISSUE_LABELS"] {
        if env.and_then(|env| env.field(name)).is_none() {
            problems.push(format!("{label}: the intake step does not pass `{name}`"));
        }
    }
    for marker in [
        "gh issue comment",
        "gh issue edit",
        "--remove-label triaged",
        "--add-label needs-info",
    ] {
        if !run.contains(marker) {
            problems.push(format!("{label}: the intake step never says `{marker}`"));
        }
    }
    problems
}

/// The playbook: every rule it is the only copy of is still a sentence in it.
fn playbook_problems(label: &str, text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut problems = Vec::new();
    for (needle, what) in PLAYBOOK {
        if !lower.contains(&needle.to_lowercase()) {
            problems.push(format!("{label}: never says `{needle}` ({what})"));
        }
    }
    problems
}

/// Every map in the document, deepest first order untouched.
fn maps_of<'a>(node: &'a Node, out: &mut Vec<&'a Node>) {
    match node {
        Node::Map(entries) => {
            out.push(node);
            for (_, value) in entries {
                maps_of(value, out);
            }
        }
        Node::List(items) => {
            for item in items {
                maps_of(item, out);
            }
        }
        Node::Scalar(_) => {}
    }
}

/// Every value written under `key`, wherever it sits.
fn entries_named<'a>(node: &'a Node, key: &str, out: &mut Vec<&'a Node>) {
    match node {
        Node::Map(entries) => {
            for (name, value) in entries {
                if name == key {
                    out.push(value);
                }
                entries_named(value, key, out);
            }
        }
        Node::List(items) => {
            for item in items {
                entries_named(item, key, out);
            }
        }
        Node::Scalar(_) => {}
    }
}

/// Every `key: value` pair where the value is a scalar.
fn scalars(node: &Node) -> Vec<(String, String)> {
    let mut out = Vec::new();
    collect_scalars(node, &mut out);
    out
}

fn collect_scalars(node: &Node, out: &mut Vec<(String, String)>) {
    match node {
        Node::Map(entries) => {
            for (key, value) in entries {
                if let Some(text) = value.as_str() {
                    out.push((key.clone(), text.to_string()));
                } else {
                    collect_scalars(value, out);
                }
            }
        }
        Node::List(items) => {
            for item in items {
                collect_scalars(item, out);
            }
        }
        Node::Scalar(_) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CI: &str = "name: CI\non:\n  push:\n    branches:\n      - main\n  pull_request:\njobs:\n  ci:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo xtask ci\n      - run: cargo xtask wasm\n  snapshot:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo xtask snapshot\n  deny:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo xtask deny\n  screenshots:\n    runs-on: ubuntu-latest\n    steps:\n      - run: cargo xtask screenshots\n";

    const INTAKE: &str = "name: Agent intake\non:\n  issues:\n    types:\n      - opened\n      - edited\n      - labeled\npermissions:\n  contents: read\n  issues: write\njobs:\n  intake:\n    if: ${{ github.event.issue.pull_request == null }}\n    runs-on: ubuntu-latest\n    steps:\n      - name: Check the report\n        env:\n          SORTLENS_ISSUE_BODY: ${{ github.event.issue.body }}\n          SORTLENS_ISSUE_LABELS: ${{ github.event.issue.labels }}\n        run: |\n          if cargo xtask intake > intake.md; then\n            exit 0\n          fi\n          cat intake.md\n          gh issue comment \"$ISSUE\" --body-file intake.md\n          gh issue edit \"$ISSUE\" --remove-label triaged --add-label needs-info\n";

    #[test]
    fn the_shipped_workflows_and_playbook_pass() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let problems = check(&root);
        assert!(problems.is_empty(), "{problems:#?}");
    }

    #[test]
    fn the_fixtures_are_what_the_checkers_want() {
        assert!(ci_problems("test", CI).is_empty(), "{:#?}", ci_problems("test", CI));
        assert!(intake_problems("test", INTAKE).is_empty(), "{:#?}", intake_problems("test", INTAKE));
    }

    #[test]
    fn a_workflow_without_the_screenshots_gate_is_refused() {
        let without = CI.replace("cargo xtask screenshots", "cargo xtask screenshot");
        let problems = ci_problems("test", &without);
        assert!(
            problems.iter().any(|problem| problem.contains("`cargo xtask screenshots`")),
            "{problems:#?}"
        );
    }

    #[test]
    fn a_gate_that_cannot_fail_is_refused() {
        let weakened = CI.replace(
            "  deny:\n    runs-on: ubuntu-latest\n",
            "  deny:\n    runs-on: ubuntu-latest\n    continue-on-error: true\n",
        );
        let problems = ci_problems("test", &weakened);
        assert!(problems.iter().any(|problem| problem.contains("cannot fail")), "{problems:#?}");
    }

    #[test]
    fn a_gate_skipped_on_pull_requests_is_refused() {
        let skipped = CI.replace(
            "  screenshots:\n    runs-on: ubuntu-latest\n",
            "  screenshots:\n    if: github.event_name == 'push'\n    runs-on: ubuntu-latest\n",
        );
        let problems = ci_problems("test", &skipped);
        assert!(problems.iter().any(|problem| problem.contains("skip a gate")), "{problems:#?}");
    }

    #[test]
    fn an_intake_that_does_not_run_on_filed_issues_is_refused() {
        let without = INTAKE.replace("      - opened\n", "");
        let problems = intake_problems("test", &without);
        assert!(problems.iter().any(|problem| problem.contains("`opened`")), "{problems:#?}");
    }

    #[test]
    fn an_intake_that_would_run_on_a_pull_request_is_refused() {
        let without = INTAKE.replace("    if: ${{ github.event.issue.pull_request == null }}\n", "");
        let problems = intake_problems("test", &without);
        assert!(
            problems.iter().any(|problem| problem.contains("off pull requests")),
            "{problems:#?}"
        );
    }

    #[test]
    fn an_intake_that_reaches_a_secret_is_refused() {
        let leaky = INTAKE.replace(
            "SORTLENS_ISSUE_BODY: ${{ github.event.issue.body }}",
            "SORTLENS_ISSUE_BODY: ${{ secrets.ISSUE_BODY }}",
        );
        let problems = intake_problems("test", &leaky);
        assert!(
            problems.iter().any(|problem| problem.contains("repository secret")),
            "{problems:#?}"
        );
    }

    #[test]
    fn an_intake_without_the_label_bookkeeping_is_refused() {
        let without = INTAKE.replace("--remove-label triaged ", "");
        let problems = intake_problems("test", &without);
        assert!(
            problems.iter().any(|problem| problem.contains("--remove-label triaged")),
            "{problems:#?}"
        );
    }

    #[test]
    fn a_playbook_that_lost_the_no_auto_merge_rule_is_refused() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let text = fs::read_to_string(root.join("docs/AUTONOMOUS_AGENT.md")).expect("the playbook is checked in");
        let rewritten = text.replace("auto-merge", "automerging");
        let problems = playbook_problems("test", &rewritten);
        assert!(problems.iter().any(|problem| problem.contains("auto-merge")), "{problems:#?}");
    }
}
