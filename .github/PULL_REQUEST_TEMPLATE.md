## Linked issue

Closes #

<!-- One issue, one pull request (CONTRIBUTING.md). Say "Closes #123" so the issue closes
     when this lands, and explain the decision here if the issue is only part of the story. -->

## What changed

<!-- The why, not the what: the diff says what. Name the files a reviewer would not think to
     open, and any format, command id or on-disk behaviour that changed. -->

- [ ] Clean room: if a behaviour was studied in another product it is named here, and no source, asset or string was copied (CONTRIBUTING.md §3).

## The test that failed first

<!-- The test name and the exact command. It must have failed before the fix, for the reason the
     issue reports, and pass now. A test written after the fix and never seen red proves less. -->

- [ ] The test above failed for the reported reason before this change, and passes now.

## Gates

Run from the repository root; after `. scripts/env.sh` when the toolchain is project-local.

- [ ] `cargo xtask ci` — fmt, clippy -D warnings, the workspace tests, layers, roots, assets, fonts, version, attribution, a11y, docs, templates
- [ ] `cargo xtask snapshot` — no diff, or the diff was read and is explained in "What changed"
- [ ] `cargo xtask deny` — licences, advisories, bans, sources
- [ ] `cargo xtask wasm` — required when `crates/core`, `crates/ui` or a wasm-facing crate changed

## Performance (scan, catalog, thumbnails)

<!-- Required when this pull request touches scanning, catalog replay or thumbnail work.
     Paste before and after from the same machine and the same arguments, or say why a timing
     cannot be trusted. Numbers are read, not generated: `cargo xtask perf` prints the table. -->

| Measurement | Before | After |
| --- | --- | --- |
| `cargo xtask perf --photos 2000 --thumbs 200` | | |

- [ ] `cargo xtask perf` run and the delta above is explained, or: not applicable — this pull request does not touch scan, catalog or thumbnails.

## UI evidence (crates/ui or any painted surface)

<!-- Required when a painted surface changed. Attach before/after captures, or name the
     `egui_kittest` baselines that moved and say in one sentence what a person sees differently.
     `cargo xtask screenshots --update` rewrites baselines; the image diff is the review. -->

- Before:
- After:
- [ ] Screenshots attached or the moved `egui_kittest` baselines named, or: not applicable — no painted surface changed.
