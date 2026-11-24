//! Worked examples: one callable argument set per command, proven by execution.
//!
//! The tool list is the command registry, which says nothing about argument names. An agent then has
//! to guess, and guessing is not reasonable: the same engine takes `ids` in one place and `id` in
//! another, and a wrong guess costs a round trip and reads like a bug.
//!
//! These examples are **proven, not written from memory**: every one is executed against a fixture
//! library in a test, and an example that the engine refuses fails the build. A name in this file is
//! therefore a name the engine really accepts. Commands that need a dialog or a path the caller must
//! choose carry a note instead of an example, and the coverage test insists that every ready command
//! is in one of the two lists — no ready tool is left undocumented.
//!
//! Optimistic concurrency (SL-D11): every response carries `rev`, the catalog revision it
//! describes. A caller that read the library at revision N passes `"expect": N` on any command and
//! is refused with a stale-revision error the moment the catalog has moved on, instead of
//! overwriting annotations that arrived in between; re-read, then retry with the new `rev`.

use serde_json::{Value, json};

/// A worked call: the command, the arguments, and what it is for.
#[derive(Debug, Clone, Copy)]
pub struct Example {
    /// Command id, which is also the tool name.
    pub command: &'static str,
    /// Arguments that the engine accepts, as JSON.
    pub args: &'static str,
    /// One line an agent can read before calling.
    pub note: &'static str,
}

/// Commands whose arguments include something only the caller can supply: a path they chose, a
/// dialog, or a window action. They are still listed, with the shape spelled out.
pub const NEEDS_CALLER: &[(&str, &str)] = &[
    ("library.addFolder", "path: the folder to index"),
    ("library.rescan", "path: a folder already in the library (omit for the first one)"),
    ("library.removeRoot", "root: an indexed folder to forget"),
    ("library.exportHistory", "file: where to write the report"),
    ("library.backup", "file: where to write the archive"),
    ("library.restoreBackup", "file: an archive, dest: an empty folder to restore into"),
    ("library.remapRoot", "from: the old folder, to: the new one"),
    ("edit.movePicks", "dest: the folder to move into, ids: photos to move"),
    (
        "edit.export",
        "dest: an absolute folder; ids: photos; optional format (original|jpeg), maxEdge, quality, metadata, collision (skip|rename|overwrite)",
    ),
    ("photo.setCaptured", "changes: [{ id, captured }], captured in Unix seconds or null"),
    ("photo.reveal", "id: one photo; returns the command to run, does not run it"),
    ("photo.open", "id: one photo; returns the command to run, does not run it"),
    (
        "packs.list",
        "no arguments; the keyword packs in the library, with their licence and switch state",
    ),
    ("packs.set", "name: the pack; on: true or false"),
    (
        "model.install",
        "id: the model to download (see model.status). Refused until consent is on (model.consent). Downloads tens of MB on a worker; resumable across attempts and restarts",
    ),
    (
        "model.consent",
        "on: true or false; the owner-approved opt-in for the one network call Sortlens makes, downloading models",
    ),
    (
        "update.check",
        "no arguments; asks the project's GitHub releases for a newer Sortlens on a worker. Refused until consent is on (update.consent)",
    ),
    (
        "update.consent",
        "on: true or false; the owner-approved opt-in for update checks, the other approved network call",
    ),
    (
        "update.status",
        "no arguments; the running version, the consent switch, and the last check's answer",
    ),
    (
        "suggest.start",
        "no arguments; proposes keywords for the whole library on a worker, writes nothing",
    ),
    ("suggest.progress", "no arguments; done and total for the running pass"),
    (
        "suggest.cancel",
        "no arguments; stops the pass, which has written nothing to begin with",
    ),
    ("suggest.clear", "no arguments; forgets the last pass's proposals"),
    ("suggest.list", "ids: photos; each photo's keyword suggestions from its own EXIF"),
    ("suggest.accept", "id: one photo; tag: a suggestion to write as an ordinary keyword"),
    (
        "app.diagnostics",
        "file: where to write the report; omitted returns the text. Paths are redacted.",
    ),
    ("app.quit", "no arguments; asks the host to close the window"),
];

/// The examples, in a sensible order for an agent learning the library.
pub const EXAMPLES: &[Example] = &[
    Example {
        command: "library.stats",
        args: "{}",
        note: "Totals: photos, rated, picks, rejects, keywords, bytes, missing.",
    },
    Example {
        command: "library.page",
        args: r#"{"offset": 0, "limit": 5, "sort": "rating"}"#,
        note: "One page of photos; sort: natural, name, rating, date, size, kind or edited. Add filter {\"added_after\": epoch} for what Sortlens added in the last 30 days (SL-A14), best read with sort \"recently-added\"; a photo with no recorded add time is never invented from its mtime. filter {\"tag_source\": \"ai\"} narrows to photos whose keywords include one an accepted suggestion added (SL-D12): manual, ai, xmp, or unrecorded for a library written before sources were kept. filter {\"missing\": true} asks the other way round from include_missing: only files that are gone (the sidebar's Missing scope), not the visible ones plus the absent.",
    },
    Example {
        command: "photo.rate",
        args: r#"{"ids": [1, 2], "stars": 4}"#,
        note: "Stars 0-5 on several photos at once; 0 clears.",
    },
    Example {
        command: "photo.flag",
        args: r#"{"ids": [1], "flag": "pick"}"#,
        note: "flag: pick, reject or none.",
    },
    Example {
        command: "photo.label",
        args: r#"{"ids": [1], "colour": "red"}"#,
        note: "colour: red, yellow, green, blue, purple, or null to clear.",
    },
    Example {
        command: "photo.keyword",
        args: r#"{"ids": [1], "tag": "beach", "add": true}"#,
        note: "add false removes the keyword instead.",
    },
    Example {
        command: "photo.tagFromPrev",
        args: r#"{"ids": [1]}"#,
        note: "re-applies the keyword added earlier this session; there is no tag argument, which is the point.",
    },
    Example {
        command: "app.diagnostics",
        args: "{}",
        note: "file: where to write the report; omitted returns the text. Paths are redacted.",
    },
    Example {
        command: "library.roots",
        args: "{}",
        note: "no arguments; every indexed folder with its photo count",
    },
    Example {
        command: "tag.keywords",
        args: "{}",
        note: "no arguments; every keyword with how many photos carry it",
    },
    Example {
        command: "tag.registry",
        args: "{}",
        note: "Every keyword with its count, the names it swallowed and orphaned aliases.",
    },
    Example {
        command: "tag.rename",
        args: r#"{"from": "strand", "to": "beach"}"#,
        note: "Renames everywhere and merges when the new name already exists.",
    },
    Example {
        command: "tag.delete",
        args: r#"{"tag": "strand"}"#,
        note: "Removes the keyword from every photo and forgets its aliases; one undo brings it back.",
    },
    Example {
        command: "tag.alias",
        args: r#"{"alias": "seaside", "canonical": "beach"}"#,
        note: "Omit canonical to forget the link.",
    },
    Example {
        command: "collection.new",
        args: r#"{"name": "Trip"}"#,
        note: "Returns the new collection id.",
    },
    Example {
        command: "collection.addTo",
        args: r#"{"id": 1, "ids": [1, 2]}"#,
        note: "Adds photos to a collection.",
    },
    Example {
        command: "collection.removeFrom",
        args: r#"{"id": 1, "ids": [2]}"#,
        note: "Removes photos from a collection; the photos stay in the library.",
    },
    Example {
        command: "collection.rename",
        args: r#"{"id": 1, "name": "Holiday"}"#,
        note: "Renames a collection.",
    },
    Example {
        command: "folder.mark",
        args: r#"{"path": "/photos/2024", "favourite": true, "colour": "blue", "sort": "largest"}"#,
        note: "the key is a path identity; omitted fields keep their value, and colour/sort/search null clears them.",
    },
    Example {
        command: "folder.marks",
        args: "{}",
        note: "every marked folder as {key, favourite, colour}; keys are normalised paths.",
    },
    Example {
        command: "collection.colour",
        args: r#"{"collections": [1, 2], "colour": "blue"}"#,
        note: "one id or several as `collections`; the whole batch is a single undoable change.",
    },
    Example {
        command: "collection.favourite",
        args: r#"{"id": 1, "favourite": true}"#,
        note: "Pins a collection to the top of the sidebar.",
    },
    Example {
        command: "collection.sort",
        args: r#"{"id": 1, "sort": "Rating"}"#,
        note: "Remembered per collection.",
    },
    Example {
        command: "collection.search",
        args: r#"{"id": 1, "search": "beach"}"#,
        note: "Remembered per collection.",
    },
    Example {
        command: "collection.delete",
        args: r#"{"id": 1}"#,
        note: "Deletes the collection; the photos stay in the library.",
    },
    Example {
        command: "cull.start",
        args: r#"{"ids": [1, 2, 3]}"#,
        note: "Stages a review session over these photos.",
    },
    Example {
        command: "cull.decide",
        args: r#"{"decision": "reject"}"#,
        note: "decision: pick, reject, deferred or unjudged.",
    },
    Example {
        command: "cull.summary",
        args: "{}",
        note: "Counts so far, and what applying would do.",
    },
    Example {
        command: "cull.apply",
        args: r#"{"mode": "tag"}"#,
        note: "mode: trash, tag or move (with target); one undoable op.",
    },
    Example {
        command: "cull.clear",
        args: "{}",
        note: "Discards a session without applying it.",
    },
    // Last, because stepping back undoes the staged decision the example above depends on.
    Example {
        command: "cull.back",
        args: "{}",
        note: "Steps back one photo in an open session.",
    },
    // Redo is published before Undo on purpose: these examples run in order against one library,
    // and the undo below clears a decision the later examples need cleared — redoing it back would
    // hand them a state they refuse. On its own a redo with nothing behind it answers, not errors.
    Example {
        command: "edit.redo",
        args: "{}",
        note: "Redoes the last undone change; a new change abandons the redo branch.",
    },
    Example {
        command: "edit.undo",
        args: "{}",
        note: "Undoes the last change; 20 steps are kept, and each undo is itself reversible.",
    },
    Example {
        command: "library.history",
        args: r#"{"limit": 10}"#,
        note: "The most recent changes, newest first.",
    },
    Example {
        command: "library.audit",
        args: "{}",
        note: "Checks the catalog files without touching them.",
    },
    Example {
        command: "library.compact",
        args: "{}",
        note: "Rewrites the log, refusing if the audit found damage.",
    },
    Example {
        command: "library.snapshot",
        args: "{}",
        note: "The whole library as one snapshot JSON object, the exact state a paired client opens with Engine::open_memory and saves back between sessions.",
    },
    Example {
        command: "library.health",
        args: "{}",
        note: "Missing, restored and changed files.",
    },
    Example {
        command: "library.changed",
        args: "{}",
        note: "Files whose size or modification time moved since the scan.",
    },
    Example {
        command: "library.refreshChanged",
        args: "{}",
        note: "Re-reads those files, as one undoable step per photo.",
    },
    Example {
        command: "library.retryMetadata",
        args: r#"{"all": false}"#,
        note: "all true also re-reads photos that had no camera and no date.",
    },
    Example {
        command: "library.readOnly",
        args: "{}",
        note: "True when another window holds the library lock.",
    },
    Example {
        command: "library.jobs",
        args: "{}",
        note: "Recent scans, hash passes and counts, newest first.",
    },
    Example {
        command: "library.deleted",
        args: "{}",
        note: "The rejects bin: batches with their restore records.",
    },
    Example {
        command: "library.forgetBatch",
        args: r#"{"id": 1}"#,
        note: "Forgets one restore record; nothing on disk changes.",
    },
    Example {
        command: "library.restoreBatch",
        args: r#"{"id": 1}"#,
        note: "Puts a batch's files back, when they are still in the Trash.",
    },
    Example {
        command: "library.clearDeleted",
        args: "{}",
        note: "Forgets every restore record; nothing on disk changes.",
    },
    Example {
        command: "library.relocate",
        args: "{}",
        note: "Finds missing files elsewhere under their root.",
    },
    Example {
        command: "library.removeMissing",
        args: "{}",
        note: "Drops missing photos from the catalog.",
    },
    Example {
        command: "library.subfolders",
        args: r#"{"root": "/photos", "prefix": ""}"#,
        note: "root: an indexed folder, prefix: a subpath to drill into.",
    },
    Example {
        command: "insights.start",
        args: "{}",
        note: "Counts the library on a worker; poll insights.progress.",
    },
    Example {
        command: "photo.delete",
        // Tag-only in the example on purpose: the harness runs every example against one scratch library, and an
        // example that really deleted files would break the ones after it — and, if the mode were `trash`, would
        // move files into the machine's real Trash while a test was running.
        args: r#"{"ids": [3], "mode": "tag"}"#,
        note: "mode: trash (default), tag (flags only), or move with a target folder; one audited batch, undoable.",
    },
    Example {
        command: "photo.sharpness",
        args: "{}",
        note: "Measures every photo that has no score from the current algorithm; poll photo.sharpnessProgress.",
    },
    Example {
        command: "photo.sharpnessProgress",
        args: "{}",
        note: "How far the sharpness pass has come, and how many frames could not be judged (left unscored).",
    },
    Example {
        command: "insights.progress",
        args: "{}",
        note: "How far the count has come.",
    },
    Example {
        command: "view.insights",
        args: "{}",
        note: "The counted statistics, including months and highlights.",
    },
    Example {
        command: "memories.list",
        args: r#"{"today": 1719230400}"#,
        note: "Resurfacing cards; today is Unix seconds, default now.",
    },
    Example {
        command: "similar.start",
        args: "{}",
        note: "Hashes previews on a worker; poll similar.groups.",
    },
    Example {
        command: "similar.groups",
        args: r#"{"mode": "families", "colour": 42}"#,
        note: "mode: families, colour or either.",
    },
    Example {
        command: "view.similar",
        args: "{}",
        note: "The groups found so far.",
    },
    Example {
        command: "suggest.start",
        args: "{}",
        note: "no arguments; proposes keywords for the whole library on a worker, writes nothing",
    },
    Example {
        command: "suggest.progress",
        args: "{}",
        note: "no arguments; done and total for the running pass",
    },
    Example {
        command: "suggest.results",
        args: "{}",
        note: "no arguments; the photos that have something to suggest, and what",
    },
    Example {
        command: "packs.list",
        args: "{}",
        note: "no arguments; the keyword packs in the library, with their licence and switch state",
    },
    Example {
        command: "packs.set",
        args: r#"{"name": "Travel", "on": true}"#,
        note: "name: the pack; on: true or false",
    },
    Example {
        command: "suggest.list",
        args: r#"{"ids": [1]}"#,
        note: "ids: photos; each photo's keyword suggestions from its own EXIF",
    },
    Example {
        command: "suggest.accept",
        args: r#"{"id": 1, "tag": "low light"}"#,
        note: "id: one photo; tag: a suggestion to write as an ordinary keyword",
    },
    Example {
        command: "suggest.dismiss",
        args: r#"{"ids": [1], "tag": "low light"}"#,
        note: "ids: photos; tag: a suggestion to stop proposing for them; one undo brings it back",
    },
    Example {
        command: "edit.cancelExport",
        args: "{}",
        note: "Stops a running export; files already written stay written.",
    },
    Example {
        command: "edit.moves",
        args: "{}",
        note: "Receipts for photos moved by edit.movePicks.",
    },
    Example {
        command: "edit.undoMove",
        args: r#"{"id": 1}"#,
        note: "Puts a moved batch back, files and records together.",
    },
    Example {
        command: "app.shortcuts",
        args: "{}",
        note: "Commands that have a keyboard shortcut.",
    },
    Example {
        command: "app.about",
        args: "{}",
        note: "Version and command counts.",
    },
    Example {
        command: "app.supportReport",
        args: "{}",
        note: "A plain-text report to paste into a bug report.",
    },
    Example {
        command: "app.settings",
        args: "{}",
        note: "Asks the host to open settings.",
    },
    Example {
        command: "view.detail",
        args: "{}",
        note: "Asks the host to open the detail view for the current selection.",
    },
    Example {
        command: "view.compare",
        args: "{}",
        note: "Asks the host to put the next selected or neighbouring photo beside the primary.",
    },
    Example {
        command: "view.softFocus",
        args: "{}",
        note: "Asks the host to open the sharpness review, answering with how many photos are scored.",
    },
    Example {
        command: "view.places",
        args: "{}",
        note: "Asks the host to open the map of geotagged photos.",
    },
    Example {
        command: "model.status",
        args: "{}",
        note: "Every pinned model with its licence, size, digest-checked install state and the consent flag.",
    },
    Example {
        command: "model.cancel",
        args: "{}",
        note: "Stops a running download; the partial file stays and a later install resumes it.",
    },
    Example {
        command: "model.remove",
        args: r#"{"id": "yolos-tiny"}"#,
        note: "Removes a verified model, reporting bytes reclaimed; 0 when nothing was installed.",
    },
];

/// The example for one command, if there is one.
pub fn example(command: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|example| example.command == command)
}

/// What to say about a command's arguments: an example, a note about what only the caller can
/// supply, or nothing at all when the command takes none and none are documented.
pub fn arguments(command: &str) -> Option<Value> {
    if let Some(example) = example(command) {
        return Some(json!({
            "example": serde_json::from_str::<Value>(example.args).unwrap_or(Value::Null),
            "note": example.note,
        }));
    }
    NEEDS_CALLER
        .iter()
        .find(|(id, _)| *id == command)
        .map(|(_, note)| json!({ "note": note }))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn every_example_is_json_and_pairs_with_a_ready_command() {
        let registry = sortlens_engine::command::Registry::v0();
        for example in EXAMPLES {
            let parsed: Value = serde_json::from_str(example.args).unwrap_or_else(|error| panic!("{}: {error}", example.command));
            assert!(parsed.is_object(), "{} needs an object", example.command);
            assert!(!example.note.is_empty(), "{} has no note", example.command);
            let command = registry
                .all()
                .iter()
                .find(|command| command.id == example.command)
                .unwrap_or_else(|| panic!("{} is not a command", example.command));
            assert!(command.is_ready(), "{} is not ready yet", example.command);
        }
        // Two examples for one command would mean one of them is stale.
        let mut seen: Vec<&str> = EXAMPLES.iter().map(|example| example.command).collect();
        let count = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(count, seen.len(), "a command is listed twice");
    }

    #[test]
    fn no_ready_command_is_left_without_an_answer_about_its_arguments() {
        let registry = sortlens_engine::command::Registry::v0();
        let mut missing = Vec::new();
        for command in registry.ready() {
            if arguments(command.id).is_none() {
                missing.push(command.id);
            }
        }
        assert!(
            missing.is_empty(),
            "these ready tools say nothing about their arguments: {missing:?}"
        );
        for (id, _) in NEEDS_CALLER {
            assert!(
                registry.all().iter().any(|command| command.id == *id && command.is_ready()),
                "{id} is listed as needing a caller but is not a ready command"
            );
        }
    }

    #[test]
    fn an_example_is_what_an_agent_sees() {
        let shown = arguments("photo.rate").expect("photo.rate is documented");
        assert_eq!(shown["example"]["stars"], 4);
        assert_eq!(shown["example"]["ids"], json!([1, 2]));
        let note = arguments("library.backup").expect("backup needs a caller path");
        assert!(note["note"].as_str().expect("note").contains("file"));
        assert!(arguments("no.suchCommand").is_none());
    }
}
