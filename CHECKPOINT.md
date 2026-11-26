# Checkpoint

## 2026-11-26 (hundred-and-thirty-first) — the host's library-lifecycle notices speak the interface language

- **Why this came up.** The hundred-and-thirtieth entry's Next named the host's remaining notices.
  The `CI` run for `3fc3905` (`45777738171`) was green — all eight jobs (`ci` on Ubuntu, Windows
  and macOS, `screenshots`, `snapshot`, `deny`, both `mobile` legs) — so this is a roadmap slice
  rather than a repair.
- **What was English.** The startup settings report, the library-open failure, both shortcut-set
  dialogs (`Import a shortcut set`, `Export a shortcut set`, the `Shortcut set` filter) and their
  read/write failures plus `Wrote “{name}”`, the backup and restore dialogs (`Back up the Sortlens
  library`, `Sortlens backup`, `Choose a Sortlens backup`, `Where should the restored library
  go?`) and their toasts, the sync server's start, stop and revoke lines, and both sidecar toggles.
- **What is routed.** All nineteen through `i18n::tr`/`i18n::fmt`, with `i18n::count` for the two
  counts, so the Arabic backup toast reads `نُسّخ ٣ ملف احتياطيًا`. A failed operation's own error
  sentence stays as it is, appended as `{error}`: the engine's and the system's words are not this
  seam's to translate. Nineteen rows per catalogue, 114 in all six.
- **Tests.** The host's source-reading listing gains the nineteen keys; the new
  `the_hosts_library_notices_read_the_interface_language` pins the English `Backed up 3 file(s)`,
  the Arabic library-open frame (Arabic verb, no `opened`), the restored toast's `٣` and folder,
  and the Arabic backup title and sync line differing from English. It fails on the old literals in
  two ways: `routed_keys` would not see them, and the Arabic lookups would return English.
- **Docs.** UX.md's i18n paragraph records the library-lifecycle set; ARCHITECTURE.md names the new
  test; ROADMAP's SL-L07 row states what remains (the suggest, pack, tag, undo, change-report,
  remap, rescan, consent and remove-root notices, and the last two `rfd` titles).
- **Verified.** `cargo xtask ci` exit 0 (fmt, workspace clippy, the workspace suite including the
  catalogue audit and the host's three tests, `layers`, `roots`, `assets`, `fonts`, `version`,
  `attribution`, `a11y`, `docs`, `templates`, `workflows`); `cargo test -p sortlens the_hosts`
  3 passed; `cargo xtask snapshot` (5 goldens), `cargo xtask deny` (advisories, bans, licenses,
  sources) and `cargo xtask screenshots` (29 passed, 1 ignored, no baseline moved — no committed
  frame paints a host notice) all exit 0. No `cargo xtask wasm` exists in this workspace;
  `cargo xtask mobile` cannot run here (no NDK, no Xcode) and no phone code changed. No release
  workflow was dispatched: this is source, catalogue rows and docs, no build or release surface.
- **Files.** `apps/sortlens/src/main.rs` (routing, the listing and the new test),
  `crates/ui/src/i18n/{ar,ja,ko,th,vi,zh-TW}.tsv` (19 rows each), `docs/UX.md`,
  `docs/ARCHITECTURE.md`, `docs/ROADMAP.md` and this file.
- **Next.** Read this commit's `CI` run first with one `gh run view` (`screenshots` is the job that
  matters; no committed frame paints these notices). Then the host's last notices — the suggest
  pass, pack and tag toasts, undo-move, the change report, remap and rescan, both consent lines and
  remove-root — plus the two remaining `rfd` titles (`Save the change report`, `Where does this
  folder live now?`), through the same seam. No release run is in flight.

## 2026-11-26 (hundred-and-twenty-ninth) — the `Moved` receipt speaks the interface language, and no English is left on the canvas
## 2026-11-26 (hundred-and-thirtieth) — the desktop's own pick, move and export words speak the interface language

- **Why this came up.** The hundred-and-twenty-ninth entry's Next named the English the shell could not
  reach: the host's own status lines and the native file-dialog titles in `apps/sortlens`. The `CI` run
  for the newest commit (`45775311062`) was in progress at the first step, not red, so this is a roadmap
  slice rather than a repair.
- **What the host said in English.** `Action::PickFolder` opened `Add a folder to Sortlens` and reported
  `Indexing <folder>…`; `MovePicks` and `Export` answered an empty selection with the literal
  `Pick a photo first`, titled their native dialogs `Move the picks into…` and `Export into…`, and
  reported a move as `Moved {count} photo(s) to <folder>` and a batch export as
  `Exporting {planned} file(s)…` — strings the shell's catalogue audit reads only in `crates/ui`, so no
  row ever covered them.
- **The seam.** `sortlens_ui::i18n` gained `tr` (a literal in the current language, the same English-key
  lookup `tl!` makes), `count` (the reader's digits, `format::count` re-exported) and
  `moved_headline(count, folder)`. `shell::move_receipt_headline` is now `pub` and delegates to
  `moved_headline`, which is the point: the receipt dialog and the host's status line word a move from
  the same two templates, so they cannot disagree. The host's `MovePicks` asks the just-made receipt's
  headline first and falls back to `moved_headline(count, dest)` only if the engine kept no receipt.
- **What is routed.** All six above: the add-folder title (reusing the existing
  `Add a folder to the library` row), `Indexing {folder}…`, both `Pick a photo first` notices, both
  move/export titles and `Exporting {n} file(s)…`. The add-folder title's English changed from *Add a
  folder to Sortlens* to the row the shell already paints, which is the same dialog under the app's name.
- **The host's own audit.** Four new rows landed in each of the six catalogues (24):
  `Move the picks into…`, `Export into…`, `Indexing {folder}…` and `Exporting {n} file(s)…`.
  Because `crates/ui`'s audit cannot `include_str!` another crate's source, `apps/sortlens/src/main.rs`
  now carries `the_hosts_words_have_a_row_in_every_catalogue`: it reads `main.rs` back, collects every
  `i18n::tr` literal and `i18n::fmt` template (whitespace after the parenthesis is skipped, and the
  scanner's own marker literals are split as `["i18n", "::tr("]` so they do not list themselves), and
  requires a row for each in all six catalogues. `the_hosts_move_notice_reads_the_interface_language`
  pins the English headline, the Arabic one (`١` and `صورة`, no `photo`), the Arabic
  `Pick a photo first` and the Arabic `Indexing {folder}…` template.
- **Not routed here, named.** The host's other notices — `Settings could not be read in full…`, the
  library-open failure, backup and restore, the sync server's three lines, the shortcut-set errors, the
  tag/put-back words and the sidecars toggles — and the remaining `rfd` titles are still English; the
  same seam takes them one slice at a time, and the host's listing is where each one lands.
- **Verified.** `cargo xtask ci` exit 0 (fmt, workspace clippy, the workspace suite including the
  catalogue audit and both host tests, `layers`, `roots`, `assets`, `fonts`, `version`, `attribution`,
  `a11y`, `docs`, `templates`, `workflows`); `cargo xtask snapshot` (5 goldens), `cargo xtask deny`
  (advisories, bans, licenses, sources) and `cargo xtask screenshots` (29 passed, 1 ignored, no baseline
  moved — no committed frame paints a native dialog title or a host notice) all exit 0. No
  `cargo xtask wasm` exists in this workspace; `cargo xtask mobile` cannot run here (no NDK, no Xcode)
  and no phone code changed. No release workflow was dispatched: this is source and docs, no build or
  release surface.
- **Files.** `crates/ui/src/i18n.rs` (`tr`, `count`, `moved_headline`), `crates/ui/src/shell.rs`
  (the public headline helper), `apps/sortlens/src/main.rs` (routing and two tests),
  `crates/ui/src/i18n/{ar,ja,ko,th,vi,zh-TW}.tsv` (four rows each), `docs/UX.md`,
  `docs/ARCHITECTURE.md`, `docs/ROADMAP.md` and this file.
- **Next.** Read this commit's `CI` run first with one `gh run view` (`screenshots` is the job that
  matters; no committed frame paints these words). Then the host's remaining notices, starting with the
  library open and backup/restore lines, through the same `i18n::tr`/`i18n::fmt` seam. No release run is
  in flight.



- **Why this came up.** The hundred-and-twenty-eighth entry's Next named SL-L07's last English on the
  canvas: the `Moved` receipt's title, the sentence under it, its `and {n} more` line and its two
  buttons. The `CI` run for the newest commit, `45773031564` (`03148d3`), was in progress at the
  tick's first step, not red, so this is a roadmap slice rather than a repair.
- **What was English.** `move_receipt_dialog` built the window from the literal `"Moved"`, painted the
  engine's English `MoveReceipt::headline()` (`"Moved 1 photo(s) to <folder>"`), the literal sentence
  under it, `format!("and {} more", …)` and both buttons from literals, while every neighbouring
  dialog already looked its words up. The engine's `headline()` stays: it is the sentence the record's
  other callers print, and the test pins it.
- **What is routed.** `move_receipt_headline` words the receipt's own pieces — `items.len()` and
  `sortlens_engine::moves::folder_name(&receipt.dest)` — through `{n} photo moved to {folder}` /
  `{n} photos moved to {folder}` with `format_count`, so an Arabic receipt reads `نُقلت ١ صورة إلى moved`.
  The title, the sentence under it (`Ratings, labels and keywords travelled with the files.`),
  `and {n} more` and both buttons (`Put them back`, `Done`) are `tl!`/`fmt` keys; the last two rows
  already existed for the Batch applied receipt. Five new rows landed in each of the six catalogues
  (30 rows): the two headline templates, the title, the sentence and `and {n} more`.
- **The test makes one real move.** `the_move_receipt_speaks_the_interface_language` indexes one JPEG,
  calls `Engine::move_picks` into `moved/`, sets `state.move_receipt` to that receipt and paints the
  dialog through `Context::run_ui` in English, Japanese and Arabic. English must paint `Moved`,
  `1 photo moved to moved`, the sentence and `Put them back`; Japanese must paint `移動しました`,
  `枚の写真を`, `元に戻す` and leave `1 photo moved to moved` and `Put them back` absent; the Arabic
  headline must carry `١` and `صورة` and no `photo`; and the `and {n} more` template must format to
  `و٥ أخرى`. (The first `run_ui` frame after `set_current` paints no shapes — the same quirk the Batch
  applied test works around — so both language switches paint twice.)
- **Docs reconciled.** UX.md's "What is still English on the canvas is the `Moved` receipt" paragraph now
  records the receipt as routed and leaves only the bin row's engine sentences as the exception;
  ARCHITECTURE.md gains the paragraph naming `move_receipt_headline` and the paint test; ROADMAP.md's
  stale "Open:" paragraph (which still named the search hint and the bottom bar's count as unrouted,
  both shipped by the hundred-and-twenty-seventh entry) is corrected, this entry is recorded on the
  SL-L07 row, and what remains on that row is stated instead: the host's own notices and native
  file-dialog titles in `apps/sortlens`, which the source-reading catalogue audit cannot see.
- **Verified.** The first `cargo xtask ci` failed only at `cargo fmt --check` (two `assert!` lines);
  `cargo fmt --all` then `cargo xtask fmt` exit 0 and the full `cargo xtask ci` re-run exit 0 (fmt,
  workspace clippy, the workspace suite including the catalogue audit, `layers`, `roots`, `assets`,
  `fonts`, `version`, `attribution`, `a11y`, `docs`, `templates`, `workflows`); `cargo xtask
  snapshot` (5 goldens), `cargo xtask deny` (advisories, bans, licenses, sources) and `cargo xtask
  wasm` all exit 0. `cargo xtask screenshots` exit 0: 29 passed, 1 ignored, no baseline moved — the
  receipt dialog is not one of the committed scenes, and `git status` shows no `.new.png`/`.diff.png`
  left behind.
- **Files.** `crates/ui/src/shell.rs` (routing, helper, test), `crates/ui/src/i18n/{ar,ja,ko,th,vi,zh-TW}.tsv`
  (five rows each), `docs/UX.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md` and this file.
- **Next.** Read this commit's `CI` run first with one `gh run view` (`screenshots` is the job that
  matters; the paint change is in a dialog no frame paints). Then SL-L07's remaining scope, in order:
  the host-level notices and the native file-dialog titles in `apps/sortlens` (`Pick a photo first`,
  `Moving the picks into…`), which no catalogue reaches today. No release run is in flight.

## 2026-11-26 (hundred-and-twenty-eighth) — the lavapipe set adopts the whole-row paint, and the adapter set is current again

- **Why this came up.** The eighty-second entry — committed as `6bd6b53`, which is the newest commit but whose
  entry sits beside the eighty-first rather than at the top of this file — repainted the 40 *reference* frames
  for the sidebar's whole-row clip and predicted that the `lavapipe/` set would be stale for the same scenes,
  naming the artifact adoption as the next tick's first job. Its `CI` run, `45771651973`, had exactly one
  failing job, `screenshots`, and `gh run view 45771651973 --json jobs` named it with the other seven green
  (`ci` on Ubuntu and macOS, `deny`, `snapshot`, both `mobile` legs, `ci` on Windows still in progress). A red
  trunk is the tick's slice, not a new roadmap row, so nothing else was started.
- **One artifact, 39 frames.** `gh run download 45771651973 -n screenshot-set -D <tmp>\sortlens-ci-shots` wrote
  exactly `lavapipe/*.new.png` for the 39 of the adapter set's 49 frames the runner painted differently, and
  nothing else. `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt <tmp>\sortlens-ci-shots` copied
  each one over its baseline (the tool copies bytes, so every committed frame *is* the paint the comparing run
  reported and not a second paint from this machine). `git status --short` then listed exactly those 39 PNGs.
- **The diff is the row band, computed not eyeballed.** All 39 old→new pairs were diffed pixel by pixel before
  adoption. 33 of them differ only inside the band the eighty-second entry named: `18,706–245,710` (42 pixels,
  the tops of the skipped keyword row's glyphs) at 1440×900, `18,417–247,428` (297 pixels, a whole skipped row)
  in the four 900×620 scenes `narrow-canvas-*`, `settings-short-*`, `short-sidebar-*`, `10,598–253,610` (2,904
  pixels, the selected folder row's band) in `filters-*` and `no-matches-*` at 1200×800, and
  `1,195,706–1,420,710` mirrored in `rtl-library-*`. The same bands are what `6bd6b53` repainted in the reference
  set, so the adapter set adopted exactly the change the reference set already carries.
- **Six pixels are the adapter, not the slice.** Outside those bands the 39 pairs differ by six isolated pixels
  in six frames — `compare-charcoal` (638,650), `filters-charcoal` and `no-matches-charcoal` (394,178),
  `palette-charcoal` (799,230) and `rtl-library-charcoal` (181,94) and (181,95). Each is a single pixel in a
  canvas or panel region, several hundred pixels from the row band, and is the paint Mesa produced this run
  against a baseline from an earlier one: the size deltas are those bands' worth of bytes (e.g.
  `library-justified-charcoal` 665426→665207, `no-matches-charcoal` 109199→108517) and no frame moved anything
  else.
- **The frames were read.** `library-justified-charcoal` and `rtl-library-charcoal` were opened at 1440×900. The
  sections in the LTR frame end on a whole `sea  2` row, the hairline and the four utility rows below it, with no
  cut glyph; the RTL frame mirrors the same ending on its own side (`sea  2` at the foot of the right-hand
  sections), the Arabic strip, header and bar unchanged, and no other column moved.
- **Not verified by hand**: the 37 frames that were not opened; no chip was clicked. The gate that compares this
  set is `screenshots` on GitHub and it is read on the next tick, not waited on.
- **Verified.** `cargo xtask ci` exit 0 (fmt, clippy, the workspace suite including the catalogue audit,
  `layers`, `roots`, `assets`, `fonts`, `version`, `attribution`, `a11y`, `docs`, `templates`, `workflows`),
  `cargo xtask snapshot` (5 goldens pass) and `cargo xtask deny` (advisories, bans, licenses, sources all ok,
  with `CARGO_HOME=<tmp>/sortlens-cargo-deny`). No `cargo xtask screenshots` run here: the set this commit
  changes is painted only by the runner, and the local adapter cannot compare it. No release dispatch: this
  slice is 39 PNGs and this file, and touches no workflow, `apps/**`, `Cargo.toml` or release code.
- **Files.** `crates/ui/tests/snapshots/lavapipe/*.png` (39 frames) and this file. No source, no ROADMAP row:
  3B.6 (d)'s revisit shipped in `6bd6b53` and this commit only makes the adapter set agree with it.
- **Next.** Read this commit's `CI` run first with one `gh run view` — `screenshots` is expected green now that
  the set is the runner's own paint. Then SL-L07's last English on the canvas: the `Moved` receipt, whose title,
  the sentence under it, `and {n} more` line and two buttons (`Put them back`, `Done`) are still literals at
  `shell.rs:10239` and its call site, with a paint test built on one indexed photo and one real move. The search
  field's hint and the bar's `No selection` count are already routed (hundred-and-twenty-seventh entry), so they
  are not in that queue. No release run is in flight.
## 2026-11-26 (hundred-and-twenty-seventh) — the sidebar's quick actions, the search field's hint and the bottom bar's count read Arabic

- **Why this came up.** The hundred-and-twenty-fifth entry's Next named the last English words on the
  default-set RTL frames: the Quick Actions rows `Start Cull` and `Export Picks…`, the search field's
  hint and the bottom bar's `No selection` count. No `CI` run was red at the tick's first step — the run
  for `1800781` (`45764901256`) was still in flight — so this is a roadmap slice, not a repair. (The
  number follows ROADMAP's own citations, which already call the top bar's words the
  hundred-and-twenty-sixth entry; that tick left no CHECKPOINT heading.)
- **One of the named surfaces was already routed.** `Local library` is a row in all six catalogues and
  `the_top_bar_names_the_library_in_the_interface_language` proves the bar looks it up, so the
  hundred-and-twenty-fifth entry's "still English" note was stale for that line and this tick changed
  nothing there. The quick-actions rows were routed (`tl!(spec.label)`) but their labels are looked up
  at run time, so the source-reading catalogue audit could never see `Start Cull` or `Export Picks…` —
  and neither had a row. The hint and the bar's count were English formatted in place.
- **What is routed.** `Start Cull` and `Export Picks…` are rows in all six catalogues, and the new
  `the_sidebar_command_rows_speak_the_interface_language` walks `SIDEBAR_QUICK` and `SIDEBAR_FOOT`
  against every catalogue, which is the guard the audit cannot be. `palette_hint` is now
  `fmt("Search photos or {keys}")` over the platform's own key cap (`⌘K`/`Ctrl+K`, never translated);
  the field's two fallbacks are `tl!("Search photos")`/`tl!("Search")`; the collection scope is
  `fmt("Search in {name}")`. The bar's count is the new `selection_count_text`: `tl!("No selection")`
  at zero and `fmt("{n} selected")` over `format_count` above it, shared with the marquee's toast,
  which formatted `{count} selected` in place.
- **The frame proves it.** `the_canvas_chrome_reads_in_the_interface_language` now requires
  `بدء الفرز` and `تصدير المختارة…`, the Arabic hint and the Arabic count on the painted frame, and
  requires `Start Cull`, `Export Picks…`, `No selection`, `Search photos or Ctrl+K` and
  `Search photos` to be absent. `the_search_hint_and_the_selection_count_speak_the_interface_language`
  pins English (`Search photos or Ctrl+K`, `No selection`, `3 selected`) and Arabic
  (`لم يتم تحديد شيء`, `محددة`, `٣`) as values, so a lost row fails without a paint.
- **The repaint moved more than the words.** `UPDATE_SNAPSHOTS=1` on
  `the_arabic_interface_paints_right_to_left_in_both_themes` rewrote
  `rtl-library-{charcoal,paper}.png` (663346→663849 and 665851→666267 bytes); a second run of that one
  test without the variable compared them byte for byte and passed. Both frames were opened at
  1440×900: the sidebar reads `بدء الفرز` and `تصدير المختارة…`, the field reads
  `ابحث عن الصور أو Ctrl+K`, and the bar reads `لم يتم تحديد شيء`; the paper theme matches modulo the
  palette, and no other pixel moved. The `Photos` line under the canvas header is still the folder's
  own name, as the hundred-and-twenty-fifth entry recorded.
- **Verified.** `cargo xtask ci` exit 0 (fmt, clippy, the workspace suite including the catalogue audit,
  `layers`, `roots`, `assets`, `fonts`, `version`, `attribution`, `a11y`, `docs`, `templates`,
  `workflows`), `cargo xtask snapshot` (5 goldens pass) and `cargo xtask deny` (advisories, bans,
  licenses, sources all ok, with `CARGO_HOME=/tmp/sortlens-cargo-deny`). The screenshots gate was run
  for the one scene this slice changed and re-compared byte for byte; no full default-set run, because
  22 of the 29 scenes are red on this machine for the pre-existing rasteriser difference the
  hundred-and-twenty-first entry recorded. No `release` dispatch: this slice touches `crates/ui/src`
  and the catalogues, not `.github/workflows/**`, `xtask/src/workflow.rs`, `apps/**`, a `Cargo.toml` or
  release code.
- **Files.** `crates/ui/src/shell.rs` (the hint, the two fallbacks, the collection scope, the bar's
  count helper, the marquee toast, the frame assertion and the two new tests),
  `crates/ui/src/i18n/{ar,ja,ko,th,vi,zh-TW}.tsv` (eight rows each),
  `crates/ui/tests/snapshots/rtl-library-{charcoal,paper}.png`, `docs/ROADMAP.md` (the SL-L07 Open
  line) and `docs/UX.md` (the canvas-prose reconciliation).
- **Next.** Read this commit's `CI` run first: its `screenshots` job is expected red on the two
  `lavapipe/rtl-library-*` frames, which this machine cannot paint — `gh run download <id> -n
  screenshot-set -D .ci-shots`, then `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt
  .ci-shots`, and read the pair. Then SL-L07's last English on the canvas: the `Moved` receipt, whose
  title, sentence under it, `and {n} more` line and two buttons are still literals at the call site —
  unless that `CI` run is red for another reason, which it then is.
## 2026-11-26 (hundred-and-twenty-fifth) — the canvas header's two scope chips and the sidebar's new-collection button read Arabic

- **Why this came up.** The hundred-and-twenty-fourth entry's Next named SL-L07's remaining canvas
  chrome, smallest first: the top bar's `Library` and `All Photos`, the `+ New collection` button,
  then the canvas head's `Photos` and `1 active`. No `CI` run was red at the tick's first step — the
  run for `9485f66` (`45757388716`) was still in flight, and the six commits before it were the Places
  work another task shipped — so this is a roadmap slice, not a repair.
- **Two of the named words were already rows, and one is not chrome.** `Library` is `tl!("Library")` in
  the sidebar header and `view_word(View::Library)` in the canvas title, so both already read `المكتبة`
  in the committed frame. The `Photos` line under the canvas header is `breadcrumbs` painting
  `folder_name(root)` — the scoped folder's own name, a value, not a label — so the frame must keep it
  as the folder spells it. The words actually left English were the header's `All Photos` reset chip,
  its `1 active` badge (a `format!` in place, so no catalogue row could ever catch it), and the
  sidebar's `+ New collection`, whose lookup key missed the existing `New collection` row by the plus.
- **What is routed.** `All Photos` looks up the row the sidebar's `Source::All` already had; the badge
  is the new template `{n} active` filled with `format_count`, so Arabic writes `١ مفعّل`; the badge's
  `Clear every filter` hover is a row too; and the button's key is `+ New collection` — the plus inside
  the key, the way the strip's `+ keyword` row works — with a row in all six catalogues.
- **The frame proves it.** `the_canvas_chrome_reads_in_the_interface_language` now scopes its fixture
  (`state.root`) the way the RTL baseline does, which is what makes the header paint both chips at all,
  and requires `كل الصور`, the formatted badge `١ مفعّل` and `+ مجموعة جديدة` on the painted frame while
  requiring `All Photos`, `1 active` and `+ New collection` to be absent. The source-reading catalogue
  audit demands the three new keys in all six languages, so a chip that loses its row fails the suite.
- **The repaint moved more than the words.** `UPDATE_SNAPSHOTS=1` on that one test rewrote
  `rtl-library-{charcoal,paper}.png` (661726→663123 and 664381→665647 bytes); a second run of the same
  test without the variable compared them byte for byte and passed. Both frames were opened at
  1440×900: the header reads `كل الصور` and `١ مفعّل` where it read `All Photos` and `1 active`. They
  also show the sidebar's Quick Actions section that shipped in `19b3b8a`, which the committed pair
  predated — this is the first default-set frame with it, and its `Start Cull`, `Export Picks…` and the
  `Ctrl+O`/`C`/`E` shortcuts are still English: the next slice, not this one.
- **Verified.** `cargo xtask ci` exit 0 (fmt, clippy, the workspace suite, `layers`, `roots`, `assets`,
  `fonts`, `version`, `attribution`, `a11y`, `docs`, `templates`, `workflows`), `cargo xtask snapshot`
  (5 goldens pass) and `cargo xtask deny` (advisories, bans, licenses, sources all ok, run with
  `CARGO_HOME=/tmp/sortlens-cargo-deny`). `cargo xtask wasm` is not a task in this tree — the command
  printed the task list, whose phone gate is `cargo xtask mobile`, so it proved nothing and is not
  claimed. No full default-set `cargo xtask screenshots` run: 22 of the 29 scenes are red on this
  machine for the pre-existing rasteriser difference the hundred-and-twenty-first entry recorded, so
  only the two scenes this slice changed were repainted and re-compared.
- **Files.** `crates/ui/src/shell.rs` (the two header chips, the badge template, the button, the
  extended frame test), `crates/ui/src/i18n/{ar,ja,ko,th,vi,zh-TW}.tsv` (three rows each),
  `crates/ui/tests/snapshots/rtl-library-{charcoal,paper}.png`, `docs/ROADMAP.md` (the SL-L07 open line
  and a shipped paragraph) and `docs/UX.md` (the RTL-prose reconciliation) and this file.
- **Not verified by hand**: neither chip nor the button was clicked; the painted frame proves the
  at-rest words only.
- **Next.** Read this commit's `CI` run first: the `screenshots` job is expected red on the two
  `lavapipe/rtl-library-*` frames, so `gh run download <id> -n screenshot-set -D /tmp/sl-lava` then
  `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt /tmp/sl-lava`, and read the pair.
  Then SL-L07's remaining reported surfaces: the Quick Actions labels (`Start Cull`, `Export Picks…`)
  in the sidebar's new section, the top bar's `Local library` and `Search photos or Ctrl+K` hint, and
  the bottom bar's `No Selection`. No release workflow was dispatched: this slice touches no workflow,
  `apps/**`, a `Cargo.toml` or release code. No release run is in flight.
## 2026-11-26 (hundred-and-twenty-fourth) — the lavapipe set adopts the filter strip's Arabic, and the trunk goes green

- **Why this came up.** `CI` on `main` was red on `eeb254b` (run `45669464175`) with exactly one failing
  job, `screenshots`: `the_arabic_interface_paints_right_to_left_in_both_themes` reported both of its
  scenes differing, `lavapipe/rtl-library-{charcoal,paper}`. The hundred-and-twenty-third entry repainted
  the *reference* `rtl-library-*` frames for the strip's translation and deliberately left the adapter set
  for an artifact; `62fb266`'s own run (`45667132463`) was red on the same pair, so the next commit landed
  on a red trunk. The other seven jobs — `ci` on Windows, Ubuntu and macOS, `deny`, `snapshot` and both
  `mobile` legs — were green, so a red trunk is the tick's slice rather than a new roadmap row.
- **One artifact, two frames.** `gh run download 45669464175 -n screenshot-set -D /tmp/sl-lava2` wrote
  exactly `lavapipe/rtl-library-{charcoal,paper}.new.png` and nothing else. `SORTLENS_SNAPSHOT_SET=lavapipe
  cargo xtask screenshots --adopt /tmp/sl-lava2` copied them into `crates/ui/tests/snapshots/lavapipe/`;
  each adopted file hashes byte-identical to the artifact's frame (`deb93d0d…`, `e49c3c0b…`), so the
  committed baseline is the paint the comparing run reported and not a second paint from this machine.
- **The diff is the strip.** `661658→661675` and `664409→664405` bytes. Both the old and the new frame were
  opened at 1440×900: the strip still painted `Clear`, `Missing`, `Edited`, `Source`, `All kinds`,
  `Flags`, `Rating` and `+ keyword` in the old baseline, while the new one paints `مسح`, `مفقودة`,
  `معدّلة`, `المصدر`, `كل الأنواع`, `اللون`, `العلامات`, `التقييم` and `+ كلمة`, with the bidi
  placement of the strip mirrored to match. The right rail's `المستبعدات` (the rejects bin), the grid, the
  counts and the still-English top bar are identical in the two frames; the adapter set was one tick behind
  on the strip, not the rejects bin.
- **What is still English is the next slice.** `Library`, `Search photos or Ctrl+K`, `All Photos`,
  `Photos`, `1 active`, `+ New collection` and `No selection` — the hundred-and-twenty-third entry's list,
  unchanged.
- **Verified.** `cargo xtask ci` exit 0 (fmt, clippy, the workspace suite, `layers`, `roots`, `assets`,
  `fonts`, `version`, `attribution`, `a11y`, `docs`, `templates`, `workflows`), `cargo xtask snapshot`
  (5 pass) and `cargo xtask deny` (advisories, bans, licenses, sources all ok). `deny` was run with
  `CARGO_HOME=/tmp/sortlens-cargo-deny` because the advisory database under `.toolchain/` holds AppleDouble
  `._*` siblings on this volume and cargo-deny stops on their invalid UTF-8. No default-set
  `cargo xtask screenshots` run: no code changed, and 22 of the 29 default-set scenes are red on this
  machine for the pre-existing rasteriser difference. No release workflow was dispatched: two baselines and
  this file touch no workflow, `apps/**`, a `Cargo.toml` or release code.
- **Not verified by hand**: the frames were read and nothing past them; no chip was clicked.
- **Next.** Read this commit's `CI` run first; if `screenshots` is green again, SL-L07's remaining canvas
  chrome, smallest first: the top bar's `Library`, `All Photos` and `+ New collection`, then the canvas
  head's `Photos` and `1 active`. Every one needs a frame, so extend a headless `run_ui` assertion,
  repaint the two `rtl-library-*` scenes with `UPDATE_SNAPSHOTS=1` on that one test, and adopt the
  `lavapipe` pair from that commit's own `CI` artifact in the same tick. No release run is in flight.

## 2026-11-26 (hundred-and-twenty-third) — the filter strip's chips name themselves in the interface language

- **Why this came up.** The hundred-and-twenty-second entry's Next named SL-L07's open canvas chrome,
  smallest first: the filter strip's chips. `CI` on `main` was green on `5685d9b` (run `45664188618`,
  all eight jobs), so this is a roadmap slice rather than a repair.
- **What was routed.** Every word the strip paints is now a catalogue row: at rest `Rating`, `Flags`,
  `Colour`, `All kinds`, `Source`, `Edited`, `Missing`, `Clear` and `+ keyword`; and every state
  the chips cycle through — `Picks`/`Rejects`/`Unflagged`, `Photos`/`RAW`/`Videos`,
  `Edited only`/`Unedited`, the keyword menu's `Find a keyword` and `No keywords`, the
  `match {mode}` toggle with its `any`/`all` values, the `{source} keywords` template and
  the two hovers. The flag, kind, source, edit and keyword-mode labels moved out of inline
  `match` expressions into `flag_filter_word`, `kind_filter_word`, `source_filter_word`,
  `edited_filter_word`, `tag_mode_word` and `match_chip_word`, so each state is a value a test can
  ask for rather than a literal buried in the paint. The source value itself (`Manual`, `AI`, `XMP`)
  stays the crate's recorded data word, the same one the filter JSON carries.
- **Nineteen rows in six catalogues.** `ar`, `ja`, `ko`, `th`, `vi` and `zh-TW` each gained
  `Flags`, `Unflagged`, `All kinds`, `Videos`, `RAW`, `Source`, `Edited`, `Edited only`,
  `Unedited`, `+ keyword`, `Find a keyword`, `No keywords`, `match {mode}`, `any`, `all`,
  `{source} keywords`, the source hover, the rating hover and the five-star hover template; the
  words that already had rows (`Picks`, `Rejects`, `Missing`, `Clear`, `Photos`, `Rating`,
  `Colour`) reuse them instead of growing a second spelling. The source-reading audit now demands
  all of them, so a chip that falls back to English fails the suite.
- **The frame proves it.** `the_canvas_chrome_reads_in_the_interface_language` now also requires
  the Arabic strip's `التقييم`, `العلامات`, `اللون`, `كل الأنواع`, `المصدر`, `معدّلة`, `مفقودة`
  and `+ كلمة` on the painted frame, and requires the English words to be absent. A new
  `every_filter_chip_state_speaks_the_interface_language` walks every cycle position in Arabic
  (including the bidi-isolated `match any`/`match all`) and checks English still reads its own
  words.
- **Verified.** `cargo xtask ci` green after one `cargo fmt --all` pass (the first run stopped at
  `fmt --check`); `cargo xtask snapshot` (5 pass) and `cargo xtask deny` green with
  `CARGO_HOME=/tmp/sortlens-cargo-deny`. The two `rtl-library-*` scenes were repainted with
  `UPDATE_SNAPSHOTS=1` and the same test then compared them byte for byte; both frames were opened
  at 1440×900: the strip reads `مسح`, `مفقودة`, `معدّلة`, `المصدر`, `كل الأنواع`, `اللون`,
  `العلامات`, `التقييم` and `+ كلمة`, while the top bar's `Library`, `1 active`, `All Photos`,
  the canvas head's `Photos` and `+ New collection` are still English — the next slice, unchanged.
  No full default-set `cargo xtask screenshots` run: 22 of the other scenes are red on this machine
  for the known rasteriser difference, and only these two scenes changed. The `lavapipe` pair is
  deliberately left for the next tick's CI artifact, because this machine has no Mesa adapter.
- **Files.** `crates/ui/src/shell.rs` (the six helpers, the strip's call sites, the two hovers, the
  extended frame test and the new state test), `crates/ui/src/i18n/{ar,ja,ko,th,vi,zh-TW}.tsv`
  (nineteen rows each), `crates/ui/tests/snapshots/rtl-library-{charcoal,paper}.png`,
  `docs/ROADMAP.md` (the SL-L07 Open line and a shipped paragraph), `docs/UX.md` (the RTL-prose
  reconciliation) and this file.
- **Not verified by hand**: the open keyword menu, the flag/kind/edit cycles and the source chip were
  tested as values, not clicked; the painted frame proves the at-rest words only.
- **Next.** Read this commit's `CI` run first: its `screenshots` job is expected red on the two
  `lavapipe/rtl-library-*` frames. `gh run download <id> -n screenshot-set -D .ci-shots`, then
  `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt .ci-shots`, and read the two frames.
  After that, SL-L07's remaining canvas chrome, smallest first: the top bar's `Library`, `Photos` and
  `New collection`, then the canvas head's `Photos` and `1 active`. No release run was dispatched:
  this slice touches no workflow, `apps/**`, a `Cargo.toml` or release code.

## 2026-11-26 (hundred-and-twenty-second) — the lavapipe set adopts the sort value's paint, and the trunk goes green

- **Why this came up.** `CI` on `main` was red on `0d68218` (run `45662554285`) with exactly one failing job,
  `screenshots`: `the_arabic_interface_paints_right_to_left_in_both_themes` reported both of its scenes differing,
  `lavapipe/rtl-library-{charcoal,paper}`. The hundred-and-twenty-first entry repainted the *reference*
  `rtl-library-*` frames and left the adapter's set at the paint from before the sort value changed. The other
  seven jobs — `ci` on Windows, Ubuntu and macOS, `deny`, `snapshot` and both `mobile` legs — were green, so
  `screenshots` was the only red job and a red trunk is the tick's slice rather than a new roadmap row.
- **One artifact, two frames.** `gh run download 45662554285 -n screenshot-set -D /tmp/sl-lava` wrote exactly
  `lavapipe/rtl-library-{charcoal,paper}.new.png` and nothing else. `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask
  screenshots --adopt /tmp/sl-lava` copied them into `crates/ui/tests/snapshots/lavapipe/`; each adopted file
  hashes byte-identical to the artifact's frame (`4d5c69d6…`, `32bedf58…`), so the committed baseline is the
  paint the comparing run reported and not a second paint from this machine.
- **The diff is the sort value.** `661372→661658` and `664139→664409` bytes. Both frames were opened at
  1440×900: the canvas header now reads `ترتيب الإضافة` where it painted `Added order`, the count still reads
  `١٢ صور` and the strip `١٢ معروضة`, and nothing else in either frame moved.
- **What is still English is the next slice.** `Photos`, `1 active`, `All Photos`, the filter chips
  (`+ keyword`, `Rating`, `Flags`, `اللون`, `All kinds`, `Source`, `Edited`, `Missing`, `Clear`),
  `No Selection` and `+ New collection` — the hundred-and-twenty-first entry's list, unchanged.
- **Verified.** `cargo xtask ci` exit 0 (fmt, clippy, the workspace suite, `layers`, `roots`, `assets`, `fonts`,
  `version`, `attribution`, `a11y`, `docs`, `templates`, `workflows`), `cargo xtask snapshot` (5 pass) and
  `cargo xtask deny` (advisories, bans, licenses, sources all ok; run with `CARGO_HOME=/tmp/sortlens-cargo-deny`
  because cargo-deny cannot read the advisory database under `.toolchain/` on this non-native volume). No
  `cargo xtask screenshots` run of the default set: no code changed, the two adopted baselines live in the
  adapter set only, and 22 of the 29 default-set scenes are red on this machine for the pre-existing
  rasteriser difference the hundred-and-twenty-first entry recorded. No release workflow was dispatched: two
  baselines and this file touch no workflow, `apps/**`, a `Cargo.toml` or release code.
- **Not verified by hand**: the frames were read and nothing past them; the combo's open menu in Arabic is
  still unclicked.
- **Next.** SL-L07's open canvas chrome, smallest first: the filter strip's chips (`All`, `Picks`, `Rated`,
  `Missing`, `Rejects`, `All kinds`, `Source`, `Edited`, `Clear`), then the top bar's `Library`, `Photos`
  and `New collection`, then the canvas head's remaining `Photos` and `1 active`. Every one needs a frame, so
  extend a headless `run_ui` assertion, repaint the two `rtl-library-*` scenes with `UPDATE_SNAPSHOTS=1` on
  that one test, and adopt the `lavapipe` pair from this commit's `CI` artifact. No release run is in flight.

## 2026-11-26 (hundred-and-twenty-first) — the canvas header's sort value reads in the interface language

- **Why this came up.** The hundred-and-twentieth entry's Next named SL-L07's open canvas chrome,
  smallest first: the header's sort value (`Added order`, `Sort::label()` from `sortlens-catalog`).
  `CI` on `main` was still running on `690c48f` when this tick began — `screenshots`, `deny`,
  `snapshot` and both `mobile` legs had passed, and the two `ci` legs were still going — while the
  last completed run (`d4469ab`, `45652802395`) was the red the 120th entry had just fixed, so this
  is a roadmap slice rather than a repair.
- **One lookup, one value.** `shell::sort_word` is `tl!(sort.label())`, the shape `view_word` already
  uses: `Sort::label` stays the English source of truth (the JSON and reports name the order by it)
  and the catalogue row is looked up by it. Both the combo's selected text and its seven menu rows go
  through it, so the closed and open states cannot disagree.
- **Seven rows in six catalogues.** `Added order`, `Recently added first`, `Name`, `Newest first`,
  `Oldest first`, `Largest first` and `Highest rated` answer in `ar`, `ja`, `ko`, `th`, `vi` and
  `zh-TW`; Arabic now paints `ترتيب الإضافة` where it painted `Added order`.
- **The tests prove it.** `the_canvas_chrome_reads_in_the_interface_language` (the 119th entry's
  count test, renamed) indexes three photos, paints the shell headless at 1440×900 in Arabic, and
  requires `ترتيب الإضافة` on the frame while `Added order` is absent — the old `state.sort.label()`
  call fails it. `every_sort_order_has_a_catalogue_row` walks `Sort::ALL` against all six
  catalogues, so a new order that arrives without a row fails rather than showing English.
- **Verified.** `cargo test -p sortlens-ui canvas_chrome` and
  `cargo test -p sortlens-ui every_sort_order_has_a_catalogue_row` pass;
  `cargo clippy -p sortlens-ui --all-targets -- -D warnings` is green. `cargo xtask ci` passed
  fmt, clippy, the workspace suite, `layers`, `roots`, `assets`, `fonts`, `version`, `attribution`
  and `a11y`, then its `docs` step failed on `docs/._ROADMAP.md` — an AppleDouble sidecar this
  non-native volume writes beside every file it touches, not a document. The docs scanner now skips
  `._*` (the same rule the `.gitignore` line states), and `cargo xtask docs` passes. `cargo xtask
  snapshot` passes all five golden tests. `cargo xtask deny` cannot read the advisory database under
  `.toolchain/` on this volume: cargo-deny re-extracts it with an AppleDouble `._RUSTSEC-*.md`
  sidecar, which it refuses as invalid UTF-8. The same `cargo deny check` therefore ran with
  `CARGO_HOME=/tmp/sortlens-cargo-deny` (a native APFS path, with the toolchain's `cargo-deny` on
  PATH) and reports advisories, bans, licenses and sources all ok. `cargo xtask screenshots` was
  red on 22 of 29 default-set scenes
  here, and `filters-charcoal`'s diff is 105 pixels of the top bar's app title — the pre-existing
  local rasteriser difference, not the sort value — so only the two `rtl-library-*` scenes were
  repainted (`UPDATE_SNAPSHOTS=1` on that one test) and both were read: the header now reads
  `ترتيب الإضافة`, the strip `١٢ معروضة`, and the still-English chrome is `Photos`, `1 active`,
  `All Photos`, the filter chips, `No Selection` and `New collection`.
- **Files.** `crates/ui/src/shell.rs` (the helper, the two call sites, the renamed frame test),
  `crates/ui/src/i18n.rs` (the coverage test), `crates/ui/src/i18n/{ar,ja,ko,th,vi,zh-TW}.tsv`
  (seven rows each), `crates/ui/tests/snapshots/rtl-library-{charcoal,paper}.png`, `docs/ROADMAP.md`,
  `xtask/src/main.rs` (the docs scan skips AppleDouble sidecars), `.gitignore` (the same `._*`
  files), this file.
- **Not verified by hand**: the combo's open menu in Arabic was not clicked; the frame proves only
  the closed value, and the menu rows share `sort_word` with it.
- **Next.** Adopt the two `lavapipe/rtl-library-*` frames from this commit's `CI` artifact and read
  them, because this machine has no Mesa adapter and only the runner can prove that set. Then the
  filter strip's chips (`All`, `Picks`, `Rated`, `Missing`, `Rejects`, `All kinds`, `Source`,
  `Edited`, `Clear`), then the top bar's `Library`, `Photos` and `New collection`. No release run
  was dispatched: this slice touches no workflow, `apps/**`, `Cargo.toml` or release code.

## 2026-11-26 (hundred-and-twentieth) — the lavapipe set adopts the counts' own paint, and the trunk goes green on the RTL canvas

- **Why this came up.** `CI` on `main` was red on `d4469ab` (run `45652802395`) with exactly one failing job,
  `screenshots`: `the_arabic_interface_paints_right_to_left_in_both_themes` reported both of its scenes
  differing, `lavapipe/rtl-library-charcoal` by 293 pixels and `lavapipe/rtl-library-paper` by 284. The
  hundred-and-nineteenth entry repainted the *reference* `rtl-library-*` frames and left the adapter's set at
  the paint from before its count change. The other seven jobs — `ci` on Windows, Ubuntu and macOS, `deny`,
  `snapshot` and both `mobile` legs — were green in this run, so `screenshots` was the only red job. A red trunk
  is the tick's slice, not a new roadmap row.
- **One artifact, two frames.** `gh run download 45652802395 -n screenshot-set -D .ci-shots` wrote exactly
  `lavapipe/rtl-library-{charcoal,paper}.new.png` and nothing else, so the only scenes that differ are the two the
  count change repainted, and only under Mesa. `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots
  --adopt .ci-shots` copied them into `crates/ui/tests/snapshots/lavapipe/`; each adopted file hashes
  byte-identical to the artifact's frame (`16820c15…`, `8a490eb0…`), so the committed baseline is the paint the
  comparing run reported and not a second paint from this machine.
- **The diff is the adapter's colour, not the layout.** `661526→661372` and `664312→664139` bytes; both frames
  were opened at 1440×900 and carry the hundred-and-nineteenth entry's fix intact — the header reads `١٢ صور`,
  the strip's right-hand column `١٢ معروضة`, both in Arabic-Indic digits, and no English count survives. What
  they still paint in English is the next slice: `Added order`, `1 active`, `All Photos`, `Photos`,
  `No Selection`, `+ New collection`.
- **Verified, step by step.** `cargo xtask fmt` and `cargo xtask clippy` each exit 0; `cargo test --workspace`
  written to a file exits 0 with every binary and doc-test `ok`; the ten post-test checks (`layers`, `roots`,
  `assets`, `fonts`, `version`, `attribution`, `a11y`, `docs`, `templates`, `workflows`) each report `ok`. The
  aggregate `cargo xtask ci 2>&1` in a background job settled exit 1 with its capture ending mid doc-tests and no
  failing assertion in it — the same harness artefact the hundred-and-eighteenth entry recorded — which is why
  the gate is recorded as its steps rather than once. `cargo xtask screenshots` (29 scenes, the default set,
  untouched by this commit), `cargo xtask snapshot` (5 pass) and `cargo xtask deny` (advisories, bans, licenses,
  sources) green.
- **Files.** `crates/ui/tests/snapshots/lavapipe/rtl-library-{charcoal,paper}.png`; this file. No `docs/ROADMAP.md`
  row moves: the adoption repaints a baseline rather than shipping a row. No release workflow dispatched — two
  baselines and this file touch no workflow, `apps/**`, a `Cargo.toml` or release code.
- **Not verified by hand**: the frames were read and nothing past them — nobody has driven the Arabic shell with
  a mouse.
- **Next.** SL-L07's open canvas chrome, smallest first: the header's sort value (`Added order`, `Sort::label()`
  from `sortlens-catalog`), then the filter strip's chips (`All`, `Picks`, `Rated`, `Missing`, `Rejects`,
  `All kinds`, `Source`, `Edited`, `Clear`), then the top bar's `Library`, `Photos` and `New collection`. Every
  one needs a frame, so extend a headless `run_ui` assertion and repaint the two `rtl-library-*` scenes in both
  the default and the `lavapipe` sets — and read the `CI` run for that commit before starting the row after it,
  since a repainted baseline is only proven on the runner. No release run is in flight.
## 2026-11-26 (hundred-and-nineteenth) — the canvas's two counts read the interface language

- **Why this came up.** The hundred-and-eighteenth entry's Next named the SL-L07 row's open canvas
  chrome, smallest first: the filter strip's `{n} shown` summary and the canvas header's count
  (`12 photos`). `CI` on `main` was green on `0ae11e7` (run `45650770648`), so this is a roadmap
  slice rather than a repair.
- **Two counts, four templates.** The header's line was `a11y::library_status`, which formatted
  English in place. The shell now has `canvas_status(total, selected)` (`crates/ui/src/shell.rs`),
  which takes the catalogue's existing `{n} photo`/`{n} photos` rows and, when a selection exists,
  two new rows `{n} photo, {selected} selected` and `{n} photos, {selected} selected`, so a
  language can order the number, its noun and the selection clause itself. The strip's summary is
  `crate::i18n::fmt("{n} shown", …)` with the same `format_count` digits. `a11y::library_status` is
  deleted with its three assertions: the string the header paints and the live region speaks now
  comes from one function, so the two cannot drift.
- **Six catalogues.** Each of `ar`, `ja`, `ko`, `th`, `vi`, `zh-TW` answers the three templates —
  so `١٢ صور` and `١٢ معروضة` in Arabic, `写真 12 枚` and `12 件を表示` in Japanese — and the
  catalogue audit reads the new `fmt` templates because every one is an English literal directly
  after the call, not an `if` the audit cannot see.
- **The test proves it.** `the_canvas_counts_read_in_the_interface_language` indexes a three-photo
  fixture, paints the shell headless through `Context::run_ui` at 1440×900 in Arabic, and requires
  both `٣ صور` (header) and `٣ معروضة` (strip) on the frame while `3 photos` and `3 shown` are
  absent — the old `format!` calls fail it, and so does a catalogue row that falls back to English.
- **Verified.** `cargo xtask ci` green (fmt, clippy on every target, the workspace suite, layers,
  roots, assets, fonts, version, attribution, a11y, docs, templates, workflows); `cargo xtask wasm`,
  `cargo xtask snapshot` (5 pass) and `cargo xtask deny` (advisories, bans, licenses, sources) green.
  `cargo xtask screenshots` first named exactly the two `rtl-library` scenes as differing (293 and
  289 pixels), then `--update` painted and rewrote exactly them
  (`661443→661280`, `664151→663977` bytes); both were opened: the header reads `١٢ صور`, the strip's
  right-hand column reads `١٢ معروضة`, and no English count survives. What the frames still paint in
  English is the next slice: `Added order`, `1 active`, `All Photos`, `Photos`, `No Selection` and
  `+ New collection`.
- **Files.** `crates/ui/src/shell.rs` (the helper, its call site, the strip summary, the test),
  `crates/ui/src/a11y.rs` (the removed English sentence and its test), the six
  `crates/ui/src/i18n/*.tsv` catalogues, `crates/ui/tests/snapshots/rtl-library-{charcoal,paper}.png`,
  `docs/ROADMAP.md` (the SL-L07 row) and this file. No release workflow dispatched: no workflow,
  `apps/**`, `Cargo.toml` or release code changed.
- **Not verified by hand**: nobody has switched the language with a mouse since this — the Settings
  path is unchanged and the headless test drives the same thread-local language.
- **Next.** Read this commit's `CI` run first — `gh run list --workflow CI --limit 1 --json
  databaseId,conclusion,headSha`. Its `screenshots` job is expected red on the two lavapipe RTL
  frames (this machine has no Mesa adapter): `gh run download <id> -n screenshot-set -D .ci-shots`,
  then `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt .ci-shots`, and read the two
  frames. After that, SL-L07's open canvas chrome, smallest first: the header's sort value
  (`Added order`, `Sort::label()` from `sortlens-catalog`), then the filter strip's chips (`All`,
  `Picks`, `Rated`, `Missing`, `Rejects`, `All kinds`, `Source`, `Edited`, `Clear`), then the top
  bar's `Library`, `Photos` and `New collection`. Every one needs a frame, so extend a headless
  `run_ui` assertion and repaint the two `rtl-library-*` scenes in both the default and the
  `lavapipe` sets. No release run is in flight.
## 2026-11-26 (hundred-and-eighteenth) — the lavapipe set adopts CI's own paint, and the trunk goes green on the RTL sidebar

- **Why this came up.** `CI` on `main` was red on `3104271` (run `45649689136`) with exactly one failing
  job, `screenshots`; the other seven — `ci` on Windows, Ubuntu and macOS, `deny`, `snapshot` and both
  `mobile` legs — were green. A red trunk is the tick's slice, not a new roadmap row.
- **One artifact, two frames.** `gh run download 45649689136 -n screenshot-set -D .ci-shots` wrote exactly
  `lavapipe/rtl-library-{charcoal,paper}.new.png` and nothing else, so the only scenes that differ are the two
  the hundred-and-seventeenth entry repainted, and only under Mesa. `SORTLENS_SNAPSHOT_SET=lavapipe cargo
  xtask screenshots --adopt .ci-shots` copied them into `crates/ui/tests/snapshots/lavapipe/`; each adopted
  file hashes byte-identical to the artifact's frame (`7680b185…`, `c459891f…`), so the committed baseline is
  the paint the comparing run reported and not a second paint from this machine.
- **The diff is the adapter's colour, not the layout.** `618402→661526` and `620988→664312` bytes; both frames
  were opened at 1440×900 and carry the hundred-and-seventeenth entry's fix intact — `الكلمات المفتاحية` at the
  sidebar's right edge, the toggle `الأكثر استخدامًا` at its left with no overlap, and every Arabic-Indic count
  (`١٢`, `٢٧`, `٢٥`) wholly inside the panel, the keyword rows below unmoved. The default set does not move:
  these two scenes are compared only when `SORTLENS_SNAPSHOT_SET=lavapipe`, and the `rtl-library` frames this
  machine paints in the default set are untouched by the adoption.
- **Verified.** `cargo xtask ci` green from start to finish (fmt, clippy on every target, the workspace suite,
  layers, roots, assets, fonts, version, attribution, a11y, docs, templates, workflows); `cargo xtask snapshot`
  (5 pass) and `cargo xtask deny` (advisories, bans, licenses, sources) green. The first invocation ran as a
  background job and settled exit 1 with its captured output ending mid doc-tests and no failing assertion in
  it; the same command re-run with its whole log written to a file exited 0 with every gate at `ok`, so this is
  recorded as the harness cutting a capture rather than a red gate. No `cargo xtask screenshots` run of the
  default set: no code changed. No release workflow dispatched — two baselines and this file touch no workflow,
  `apps/**`, a `Cargo.toml` or release code.
- **Files.** `crates/ui/tests/snapshots/lavapipe/rtl-library-{charcoal,paper}.png`; this file. No `docs/ROADMAP.md`
  row moves: the adoption repaints a baseline rather than shipping a row.
- **Not verified by hand**: nothing past the frames — nobody has driven the Arabic shell with a mouse.
- **Next.** Read this commit's `CI` run first — `gh run list --workflow CI --limit 1 --json
  databaseId,conclusion,headSha` — then the SL-L07 row's open canvas chrome, smallest first: route the filter
  strip's `{n} shown` summary and the canvas header's count (`12 photos`) through the catalogues, then its sort
  value (`Added order`), then the top bar's `Library` and `New collection`. Every one needs a frame, so extend
  `the_arabic_interface_paints_right_to_left_in_both_themes`'s assertions or add a headless `run_ui` test, and
  repaint the two `rtl-library-*` scenes in both the default and the `lavapipe` set. No release run is in flight.
## 2026-11-26 (hundred-and-seventeenth) — the RTL sidebar's Keywords row gives its heading and its toggle their own ends

- **Why this came up.** The hundred-and-sixteenth entry's Next was the RTL frame's two visible defects, and
  `CI` on `main` was green on `442011a` (run `45645099645`), so this is a roadmap slice rather than a repair.
- **The row.** `keyword_section_header` (`crates/ui/src/shell.rs`) now owns the Keywords line. Left to right it
  makes the calls the row always made — `nav_section` inside `ui.horizontal`, the button in a `right_to_left`
  trailing layout — in the same order, so no left-to-right frame moves by a pixel. Right to left that shape
  cannot be mirrored as it stands: `nav_section` puts the heading at the reading start by wrapping it in a
  `top_down(Align::Max)` child, that child takes the whole row, and the trailing layout then paints the toggle
  from the reading end inside the space the heading already owns — which is why the two words collided. The
  mirrored branch lays the row with `direction::horizontal` (right to left) and gives the toggle the space
  after the heading in a `direction::trailing` child (left to right), so each end owns its own room: heading
  at the right, toggle at the left. The heading is the same `tl!("Keywords")` literal, painted with the same
  size and colour, and the row's top and bottom spacing are the ones `nav_section` adds.
- **The words.** The toggle painted `state.tag_order.label()` — `most used` or `A–Z`, English in every
  language. The shell now matches the order onto the two catalogue keys `most used` and `A–Z`, so the audit in
  `i18n.rs` reads them and all six catalogues answer: `الأكثر استخدامًا`/`أ–ي`, `よく使う順`/`あ–ん`,
  `사용 빈도순`/`가–하`, `ใช้บ่อยที่สุด`/`ก–ฮ`, `dùng nhiều nhất`/`A–Z theo tên`, `最常用`/`依字母`.
  `TagOrder::label()` stays in `sortlens-catalog` as the English key the enum names — the shell simply no
  longer paints it, the way the cull screen words itself from the engine's pieces rather than its sentences.
- **The second defect was not one.** The entry claimed every section count was clipped at the panel's left
  edge. Measured on the committed frame, the count's ink runs x 1079..1088 while the sidebar's background
  starts at 1072, so the numbers are cramped against the edge, not cut by it — `nav_row`'s right-aligned anchor
  at `end_x - 6` already mirrors correctly. Nothing in `nav_row` changed, and the new test pins the counts
  wholly inside their clip so a later mirroring that does move them out fails there.
- **The test proves the collision.** `the_rtl_sidebar_keeps_its_heading_and_counts_inside_the_panel` paints the
  1280×800 shell headless through `Context::run_ui` in Arabic and reads the text shapes' own rects: the heading
  must lie wholly at the reading start of the toggle, and every Arabic-Indic count in the sidebar must be
  wholly inside the clip its panel paints with. Put the old shape back (`if false && is_rtl`) and it fails
  naming both rects — heading `[1183,330]-[1262,341]` against toggle `[1202,334]-[1272,345]`, a 60-point
  overlap — so the assertion is about the defect, not about a shape that happens to look right.
- **Verified.** `cargo xtask ci` green (fmt, clippy on every target, the workspace suite, layers, roots,
  assets, fonts, version, attribution, a11y, docs, templates, workflows); `cargo xtask screenshots` painted
  29 scenes green and rewrote exactly the two frames that changed, `rtl-library-{charcoal,paper}.png`
  (`618581→661443` and `620935→664151` bytes), both read at 3× on the Keywords band: the heading sits at the
  right, the toggle at the left with its Arabic word, no overlap, and the keyword rows below did not move;
  `cargo xtask snapshot` (5 pass) and `cargo xtask deny` (advisories, bans, licenses, sources) green.
- **Files.** `crates/ui/src/shell.rs` (the header helper, its call site, the test), the six
  `crates/ui/src/i18n/*.tsv` catalogues, `crates/ui/tests/snapshots/rtl-library-{charcoal,paper}.png`,
  `docs/ROADMAP.md` (the SL-L07 row) and this file.
- **Not verified by hand**: nobody has pressed the Arabic toggle with a mouse — the click path is the same
  `clicked()` branch it was, and the only change on it is where the widget is laid out and which word it
  paints. The two `lavapipe` baselines are not repainted here: this machine has no Mesa adapter.
- **Next.** Read this commit's `CI` run first — `gh run list --workflow CI --limit 1 --json
  databaseId,conclusion,headSha`. Its `screenshots` job is expected red on the two RTL frames: download the
  artifact (`gh run download <id> -n screenshot-set -D .ci-shots`) and adopt it with
  `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt .ci-shots`, then read the two frames. After
  that, the SL-L07 row's open canvas chrome, smallest first: route the filter strip's `{n} shown` summary and
  the canvas header's count (`12 photos`) through the catalogues, then its sort value (`Added order`), then
  the top bar's `Library` and `New collection`. Every one needs a frame, so extend
  `the_arabic_interface_paints_right_to_left_in_both_themes`'s assertions or add a headless `run_ui` test.
  No release workflow is in flight.
## 2026-11-26 (hundred-and-sixteenth) — the trunk goes green on the Arabic shell's own paint, and the RTL frame names the panel's next two defects

- **Why this came up.** `CI` on `main` was red on `3bd8ecc` (run `45643003091`) with exactly one failing
  job, `screenshots`: the hundred-and-fifteenth entry committed a changed reference set, so this is the
  adoption that entry's Next named, and a red trunk is the tick's slice rather than a new roadmap row. The
  other seven jobs — `ci` on Windows, Ubuntu and macOS, `deny`, `snapshot` and both `mobile` legs — were
  green.
- **One artifact, one command.** `gh run download 45643003091 -n screenshot-set` wrote exactly six frames
  and nothing else — `lavapipe/rtl-library-{charcoal,paper}.new.png` (the two new scenes, reported missing)
  and `lavapipe/settings-{short,sync}-{charcoal,paper}.new.png` (the four the vendored Arabic face
  repainted) — so no scene that matched was handed over and none was left behind.
  `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt <dir>` copied them into
  `crates/ui/tests/snapshots/lavapipe/`, and every adopted file hashes byte-identical to the artifact's
  frame (`c1322770…`, `4c415dad…`, `87c70dca…`, `15175684…`, `da952137…`, `71a3177d…`), so each
  committed baseline is the paint the comparing run reported and not a second paint from this machine.
- **Read, all six.** The two `rtl-library` scenes paint the Arabic shell right-to-left at 1440×900: sources
  sidebar at the right edge with `المصادر` and the scope rows right-aligned, inspector and its tool rail
  mirrored to the left, the bottom bar and utility row Arabic. The four Settings scenes paint the Language
  row with `العربية` in real glyphs while the untranslated scripts stay boxes (`CRAFT_FONTS_DIR` unset,
  exactly the harness's documented limit). The diff is the face: four PNGs `91776→90770`, `96760→95522`,
  `235207→236874`, `248269→249822` bytes.
- **Two defects the RTL frame puts on record**, read at 3× on the reference frame and present in this set
  the same way: (a) the Keywords heading and its sort button collide and the button's word is the
  untranslated `most used` — `shell.rs:3125-3136` paints `nav_section(tl!("Keywords"))` and a
  right-to-left `Layout` in one `ui.horizontal`, and the label comes from
  `crates/catalog/src/query.rs:283` `TagOrder::label()`, so mirroring the row and routing the toggle
  through the catalogue are one slice; (b) every section count (`١٢`, `٢٧`, `٢٥`) is clipped at the
  panel's left edge, because `nav_row` (`shell.rs:10609`) anchors the count at `end_x -` 6 points and
  aligns it `RIGHT_CENTER`: in RTL the anchor is the row's left edge and right-aligned text extends past
  it. Neither is a baseline mismatch; both are painting defects the new frame now proves.
- **Verified.** `cargo xtask ci` green (fmt, clippy, the workspace suite, layers, roots, assets, fonts,
  version, attribution, a11y, docs, templates, workflows), `cargo xtask snapshot` (5 pass) and
  `cargo xtask deny` (advisories, bans, licenses, sources) green. No `cargo xtask screenshots` run: no
  code changed and the default set this machine compares against is untouched — the lavapipe set only
  compares under Mesa, which is CI's job, so the next `CI` run on this commit is the proof. No release
  workflow was dispatched: six baselines and this file touch no workflow, `apps/**`, a `Cargo.toml` or
  release code.
- **Files.** The two new and four repainted `crates/ui/tests/snapshots/lavapipe/*.png`; this file.
- **Not verified by hand**: nothing past the frames — no person has driven the RTL shell with a mouse.
- **Next.** The RTL row's two visible defects, in that order: mirror the Keywords heading/sort row and route
  `TagOrder::label()` through the catalogue (ar, ja, ko, th, vi, zh-TW rows — the audit in `i18n.rs` will
  demand them), then the RTL count anchor in `nav_row`. Both need a frame, so add or extend a headless
  `run_ui` test that asserts the two rects no longer intersect, and repaint the `rtl-library-*` scenes.
  Read this commit's `CI` run first — `gh run list --workflow CI --limit 1 --json databaseId,conclusion,headSha`
  — and adopt its `screenshots` frames if they differ. No release run is in flight.
## 2026-11-26 (hundred-and-fifteenth) — the Arabic shell is painted right-to-left, and the harness stops borrowing the machine's fonts

- **Why this came up.** The hundred-and-fourteenth entry's Next was exactly this. `crates/ui/tests/screenshots.rs`
  paints egui's defaults, so the RTL frame SL-L07 still listed could not be painted — the Settings picker's
  `العربية` was boxes in every committed baseline — and installing the app's own `fonts::stack()` there would
  make a frame depend on whichever `CRAFT_FONTS_DIR` the machine happened to have.
- **One comparable stack.** `fonts::stack_builtin()` and `install_builtin(ctx)` are egui's defaults plus the one
  vendored Arabic face and never a craft face; the append is one `append_face` helper shared with `stack()`.
  The harness's `context_setup` installs this stack, so every baseline is painted with the same faces on CI
  and on a developer's checkout.
- **The RTL scene.** `the_arabic_interface_paints_right_to_left_in_both_themes` sets the thread-local language
  to `ar`, paints the scoped library at 1440×900 in Charcoal and Paper, and resets to English;
  `rtl-library-charcoal` and `rtl-library-paper` are the two new baselines, both read. The sources sidebar sits
  at the right edge with `المصادر`, `كل الصور`, the scope rows and the folders right-aligned; the inspector and
  its tool rail are mirrored to the left; the bottom bar and the utility row are Arabic. The frames also show
  what the row calls the unrouted canvas chrome still in English — the top bar's words, `12 photos`, the sort
  value, the filter strip's chips and `1 shown` — which the row's open line now names outright.
- **The Settings baselines repaint, and that is the face.** The Language row's `العربية` now draws real glyphs,
  so the four `settings-{short,sync}-{charcoal,paper}` baselines moved (91511→90808, 96167→95450,
  235110→236760, 248314→249826 bytes). `settings-short-charcoal` was opened: `العربية` paints in Arabic while
  the other untranslated scripts remain boxes, which is the harness's documented limit rather than a defect.
- **Tests that fail when the behaviour is absent.** `the_harness_stack_is_the_vendored_face_and_never_the_checkout`
  requires exactly one face past egui's defaults, no craft face even in an embedded build, the Arabic face last
  in both families, and its glyphs inside a painted frame. `the_harness_stack_draws_the_arabic_frames_words`
  tests the setup itself, not the font table: if `context_setup` stopped installing the stack, the RTL frame
  would compare its own tofu against its own baseline and pass, so this is the assertion that makes the face
  more than a claim. The shell's RTL test carried a stale "CI has no Arabic face" comment; it now says what it
  really proves (geometry and words, on egui's default fonts).
- **Files.** `crates/ui/src/fonts.rs` (`append_face`, `stack_builtin`, `install_builtin`, one test);
  `crates/ui/tests/screenshots.rs` (`context_setup`, the two shoot call sites, the RTL scene, one test);
  `crates/ui/src/shell.rs` (a comment); the two new and four repainted baselines; `docs/ARCHITECTURE.md`
  (the harness's face rule, seventeen surfaces, fifty reference PNGs); `docs/UX.md`, `docs/FEATURES.md`,
  `docs/ROADMAP.md` (SL-L07 keeps its box for the unrouted surfaces and records the RTL frames as shipped).
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the seven `fonts` tests pass;
  `cargo xtask screenshots --update` painted 29 tests green and the frames above were opened; `cargo xtask ci`
  green (fmt, clippy, the workspace suite, layers, roots, assets, fonts, version, attribution, a11y, docs,
  templates, workflows), `cargo xtask snapshot` (5 pass) and `cargo xtask deny` (advisories, bans, licenses,
  sources) green. No release workflow was dispatched: this slice touches no workflow, `apps/**`, a
  `Cargo.toml` or release code.
- **Next.** This commit changes the committed reference set, so its `CI` `screenshots` run is expected red:
  read the run, `gh run download <id> -n screenshot-set`, read the frames and adopt the four `settings-*`
  plus the two `rtl-library-*` with `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt <dir>`.
  Then the unrouted canvas chrome the RTL frame put side by side: the canvas header's `{n} photos` count and
  sort value, the filter strip's labels and hovers and its `{n} shown` summary, and `New collection`. No
  release run is in flight.
## 2026-11-26 (hundred-and-fourteenth) — the Arabic interface ships the face it has always named, and `العربية` stops being a box

- **Why this came up.** The hundred-and-thirteenth entry's Next named SL-L07's last item, the RTL screenshots,
  and said they needed "a font build that embeds an Arabic face". Reading the code first (as the rule asks)
  found a product defect behind the test gap: `ar` is a shipped interface language and the picker offers it,
  but every Arabic glyph came from the *optional* craft-fonts checkout — `crates/ui/src/fonts.rs` embedded
  `CRAFT_FONTS` and nothing else, that table is empty whenever `CRAFT_FONTS_DIR` is unset (CI, and every fresh
  clone), and `install()` even returned early in that case. So an Arabic interface painted every word as a
  replacement box on the configuration the project treats as normal.
- **One face, vendored, last.** `assets/fonts/NotoSansArabic.ttf` (Noto Sans Arabic, variable, 844,676 bytes,
  OFL-1.1, licence text in `assets/fonts/OFL.txt`) is embedded as `fonts::BUILTIN_ARABIC`. `stack()` appends
  it to each family **after** the craft faces — and only when the craft set has no face tagged `Arab`, so a
  developer's checkout keeps its own face and no build carries two. `install()` no longer returns early: it
  always installs the stack and reports how many faces it added, which is `1` without a checkout now.
  `fonts::arabic()` answers whether this build can paint Arabic at all.
- **The test asks the fonts, not the table.** `the_arabic_interface_paints_without_the_checkout` installs the
  stack on a bare context and asks inside one painted frame (`Context::run_ui`, its `textures_delta` drained)
  whether the *installed* fonts have the glyphs of `العربية`, `الكل` and `٣ صور` — three strings from the
  catalogue's own words. Without the face the first assert fails; with a face that is present in the table
  but fails to parse it also fails. The same test pins that each default family still starts with the face
  it started with, so the Arabic face is a fallback and cannot take Latin text over.
- **No baseline moved, and that is measured, not assumed.** The screenshot harness paints egui's own defaults:
  `crates/ui/tests/screenshots.rs` never calls `fonts::install`, which is why the Settings row's other
  non-Latin names are boxes in the committed frames. Installing the real stack there would make every frame
  depend on whichever craft-fonts checkout the machine has, so it stays a separate decision — and it is the
  first thing the next tick needs, because an Arabic frame painted through the harness still has no face.
- **Files.** `assets/fonts/NotoSansArabic.ttf` and `assets/fonts/OFL.txt` (new); `crates/ui/src/fonts.rs`;
  `docs/ATTRIBUTION.md` (the one vendored face, its licence, and why it is not a build input); `docs/ROADMAP.md`
  (SL-L07 names what now ships and keeps the RTL baselines open); `docs/UX.md` (the picker's `العربية` prints
  Arabic words on every build); `fonts/embedded.txt` (the record says the built-in face is not a craft face).
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean and the six `fonts` tests
  pass. The new test **bites**: with the face not appended, it fails on the first string
  (`العربية has no glyphs in the installed stack`), then passes again when it is restored. `cargo xtask ci`
  green (fmt, clippy, the whole workspace test suite, layers, roots, assets, fonts, version, attribution,
  a11y, docs, templates, workflows), `cargo xtask snapshot` (five pass) and `cargo xtask deny`
  (advisories, bans, licenses, sources) green. No `cargo xtask screenshots` run: no baseline moved, because
  the harness does not install the stack (see above). No release workflow was dispatched: `crates/ui`
  source, one vendored asset, docs and the font record touch no workflow, `apps/**`, a `Cargo.toml` or
  release code.
- **Next.** The RTL baselines, and the first step is the harness's font story rather than the scene:
  `crates/ui/tests/screenshots.rs` paints egui's defaults, so give it the built-in face only
  (`sortlens_ui::fonts::BUILTIN_ARABIC`, never the machine's craft set) so a frame is machine-independent,
  then add an Arabic scene that switches the language and paints the shell right-to-left, and read the
  frame. Expect the four `settings-*` baselines to repaint with real Arabic glyphs in the Language row, and
  expect the lavapipe set to need one adoption from the runner's own paint — the same two-step the last
  three language slices used. Read this commit's CI run first (`gh run list --workflow CI --limit 1`) and
  adopt its `screenshots` frames with `SORTLENS_SNAPSHOT_SET=lavapipe` if the runner disagrees. No release
  run is in flight.
## 2026-11-26 (hundred-and-thirteenth) — the Vietnamese row's own paint goes into the lavapipe set

- **Why this came up.** The trunk was red on `b753c97` (`45634449220`) with exactly one failing job,
  `screenshots`; the hundred-and-twelfth entry's Next named that read first, and a red trunk is the tick's
  slice rather than a new roadmap row. The other seven jobs — `ci` on Windows, Ubuntu and macOS, `deny`,
  `snapshot` and both `mobile` legs — were green, so the red was the baseline set and nothing else.
- **One artifact, one command.** `gh run download 45634449220 -n screenshot-set` wrote four frames and
  nothing else — `lavapipe/settings-{short,sync}-{charcoal,paper}.new.png` — the four Settings scenes the
  failing run reported. `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt <dir>` copied them
  into `crates/ui/tests/snapshots/lavapipe/`. All four adopted files hash byte-identical to the artifact's
  frames (`f9449e2a…`, `4e116743…`, `f9fb6a5d…`, `98ba4512…`), so each committed baseline is the paint the
  comparing run reported and not a second paint from this machine.
- **Read, all four.** Each paints the Language row with seven entries — English, Arabic, Japanese, Korean,
  Thai, `Tiếng Việt` with its diacritics, and `zh-TW` — inside the dialog at 900 px, in both themes, with no
  clipping and no overlap (`CRAFT_FONTS_DIR` unset, so the runner paints the non-Latin names as boxes and
  `Tiếng Việt` as Latin text, exactly as this machine does). The two `sync` scenes still scroll their eleven
  sections with the row intact. Nothing else in any frame moved: the row grew into room it had.
- **What the diff is.** Four PNGs, `89857→91776`, `94686→96760`, `233910→235207` and `246882→248269` bytes:
  one more entry in the Language row on each scene.
- **Verified.** `cargo xtask ci` green (fmt, clippy, 213 workspace tests, layers, roots, assets, fonts,
  version, attribution, a11y, docs, templates, workflows), `cargo xtask snapshot` (5 pass) and
  `cargo xtask deny` green. No `cargo xtask screenshots` run was needed: no code changed and the default set
  this machine compares against is untouched — the lavapipe set only compares under Mesa, which is CI's job.
  No release workflow was dispatched: four baselines and this file touch no workflow, `apps/**`, a
  `Cargo.toml` or release code.
- **Files.** The four `crates/ui/tests/snapshots/lavapipe/settings-{short,sync}-{charcoal,paper}.png`; this file.
- **Next.** SL-L07's last open item: the RTL screenshots. They need a font build that embeds an Arabic face
  before an Arabic frame can be painted and compared (the picker already offers `ar` and the row already
  paints it as a box), so the slice is the face in `crates/ui/src/font_manifest.rs` plus a headless
  right-to-left `Context::run_ui` test and the frame it paints. Read this commit's CI run first —
  `gh run list --workflow CI --limit 1 --json databaseId,conclusion,headSha` — and adopt its `screenshots`
  frames with `SORTLENS_SNAPSHOT_SET=lavapipe` if it is red. No release run is in flight.
## 2026-11-26 (hundred-and-twelfth) — the sixth catalogue: `Tiếng Việt`, and the six languages the design names all ship

- **Why this came up.** The hundred-and-eleventh entry's Next named `vi.tsv`: the last of the six languages
  the design names, and the last open item of SL-L07's catalogue half (the RTL screenshots stay open).
- **One catalogue, the same keys in the same order.** `crates/ui/src/i18n/vi.tsv` was built by zipping
  `ko.tsv`'s keys with Vietnamese translations — 754 key rows, 11 `@plural` rows and the `@id` row
  (`Tiếng Việt`), 769 lines, so a key cannot be mistyped and the English column stays a catalogue's own
  text. `I18n::check_all()` parses every line with no bad row.
- **One plural form, not two.** The table asks Vietnamese for a single form, exactly as it does Korean and
  Thai; a two-form `@plural` row is a parse problem (`expected 1 nonempty plural forms`), which the keymap
  audit reported as eleven missing rows until each row was written as one form. Three rows also had to
  differ from English to count as rows: `Raw` -> `RAW`, `Video` -> `Đoạn phim`, and `{name} · {kind}` ->
  `{name} · loại {kind}` — a row that translates to itself is indistinguishable from a missing one.
- **Wiring, and no new face.** `CATALOGS` embeds the file and `LANGUAGES` offers `vi` between `th` and
  `zh-TW`; `format::STYLES` gains Vietnam's shapes (dot grouping, comma decimal, day-first slash date):
  `12.408`, `1,4 MB`, `03/10/2026 04:00`. `SHIPPED_LANGUAGE_SCRIPTS` gains `(vi, Latn)` -- `Latn` is
  already required, so `PHONE_SCRIPTS` and the phone build are unchanged and the embedded Latin face paints
  the diacritics.
- **Tests that fail when the behaviour is absent.** The catalogue audit's two loops now include `vi` (every
  routed `tl!`/`fmt` key, and the shortcut editor's 175 strings and eleven plurals), the picker test names
  `Tiếng Việt`, the direction test adds a Vietnamese left-to-right leg, `format`'s numbers and dates tests pin
  the three shapes above, and `the_language_switch_moves_the_sidebar_to_the_reading_start` paints `NGUỒN`
  and `Tất cả ảnh` after switching through the real Settings dialog.
- **Files.** `crates/ui/src/i18n/vi.tsv` (new); `crates/ui/src/i18n.rs` (`CATALOGS`, `LANGUAGES`, the two
  audits, the picker and direction tests, the module doc); `crates/ui/src/format.rs` (the `vi` style and its
  tests); `crates/ui/src/font_manifest.rs` (the shipped-language row); `crates/ui/src/shell.rs` (the headless
  language test's Vietnamese leg); the four `settings-{short,sync}-{charcoal,paper}.png` baselines repainted
  by `cargo xtask screenshots --update`; `docs/ROADMAP.md`, `docs/UX.md`, `docs/FEATURES.md`.
- **Verified.** `cargo xtask ci`, `cargo xtask snapshot` (five pass) and `cargo xtask deny` green, and
  `cargo xtask screenshots` green on the repainted baselines. The `settings-short-charcoal` frame was read:
  the Language row paints English plus five names, `Tiếng Việt` with its diacritics and no clipping or
  overlap, and nothing else in the dialog moved. No release workflow was dispatched: this slice touches no
  workflow, no `apps/` path, no `Cargo.toml` and no release code.
- **Next.** Read this commit's CI run (`gh run list --workflow CI --limit 1`) and adopt its `screenshots`
  frames with `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt <dir>` if the runner's paint
  differs from this machine's. That closes SL-L07's catalogue half; its last open item is the RTL
  screenshots, which need a font build that embeds an Arabic face so a right-to-left frame can be painted
  and compared. No release run is in flight.
## 2026-11-26 (hundred-and-eleventh) — the trunk goes green on the Thai row's own paint

- **Why this came up.** The trunk was red on `4912da8` (`45629163245`) with exactly one failing job,
  `screenshots`; the hundred-and-tenth entry's Next named that read first, and a red trunk is the tick's
  slice rather than a new roadmap row. The other seven jobs — `ci` on Windows, Ubuntu and macOS, `deny`,
  `snapshot` and both `mobile` legs — were green, so the red was the baseline set and nothing else.
- **One artifact, one command.** `gh run download 45629163245 -n screenshot-set` wrote four frames and
  nothing else — `lavapipe/settings-{short,sync}-{charcoal,paper}.new.png` — the four Settings scenes the
  failing run reported, so no scene was left behind and no frame that matched was handed over.
  `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt <dir>` copied them into
  `crates/ui/tests/snapshots/lavapipe/`. All four adopted files hash byte-identical to the artifact's frames
  (`035ce88f…`, `d56c4c83…`, `88658beb…`, `f39a2156…`), so each committed baseline is the paint the comparing
  run reported and not a second paint from this machine.
- **Read, all four.** Each paints the Language row with six entries — English plus the five non-Latin names —
  inside the dialog at 900 px, in both themes, with no clipping and no overlap; the two `sync` scenes still
  scroll their eleven sections with the row intact. Nothing else in any frame moved: the row grew into room it
  had. As the hundred-and-ninth entry established, the runner paints the non-Latin names as boxes
  (`CRAFT_FONTS_DIR` unset), so this set carries the same honest boxes this machine does.
- **What the diff is.** Four PNGs, `89386→89857`, `94097→94686`, `233440→233910` and `246335→246882` bytes:
  one more entry in the Language row on each scene.
- **Verified.** `cargo xtask ci` green (fmt, clippy, 212 workspace tests, layers, roots, assets, fonts, version,
  attribution, a11y, docs, templates, workflows), `cargo xtask snapshot` (5 pass) and `cargo xtask deny`
  (advisories, bans, licenses, sources) green. No `cargo xtask screenshots` run was needed: no code changed and
  the default set this machine compares against is untouched — the lavapipe set only compares under Mesa, which
  is CI's job. No release workflow was dispatched: four baselines and this file touch no workflow, `apps/**`, a
  `Cargo.toml` or release code.
- **Files.** The four `crates/ui/tests/snapshots/lavapipe/settings-{short,sync}-{charcoal,paper}.png`; this file.
- **Next.** `vi.tsv`: Vietnamese is Latin-script, so it needs no new face — the same 754 keys, 11 `@plural`
  counts and `@id` row as `th.tsv`, in the audit's order; then the `CATALOGS`/`LANGUAGES`/`STYLES` rows
  (Vietnamese groups with `.` and writes `12.408`, `1,4 MB`, `03/10/2026 04:00`), `("vi", "Latn")` in
  `SHIPPED_LANGUAGE_SCRIPTS` (`Latn` is already in `PHONE_SCRIPTS`), the picker/format/headless language
  tests, and the four Settings baselines in the default set via `--update`. Read this commit's CI run first —
  `gh run list --workflow CI --limit 1 --json databaseId,conclusion,headSha` — and adopt its `screenshots`
  frames with `SORTLENS_SNAPSHOT_SET=lavapipe` if it is red. No release run is in flight.
## 2026-11-26 (hundred-and-tenth) — the Thai catalogue ships, and the language row gains ไทย

- **Why this came up.** The hundred-and-ninth entry's Next named `th.tsv` first: the same 754 key rows and
  11 `@plural` counts as `ko.tsv`, in the audit's order, so every routed key already had an English source
  and four catalogues to answer it.
- **One catalogue, the same keys in the same order.** `crates/ui/src/i18n/th.tsv` was built by zipping
  `ko.tsv`'s keys with Thai translations — 754 key rows, 11 `@plural` rows and the `@id` row, whose name is
  `ไทย` — so the English column is the Korean catalogue's own text and the row count is asserted against it,
  a key that cannot be mistyped. `I18n::check_all()` parses all 769 lines with no bad row.
- **Two rows had to differ from English to be readable as translated.** The audit proves a row exists by
  asking whether the lookup differs from its key, so a byte-identical translation reads as missing:
  `1:1  Z` is now `ขนาดจริง  Z` (the actual-size label every other catalogue translates) and
  `{name} · {kind}` is `{name} — {kind}` (Korean's dash). The audit stays honest rather than special-casing
  a language.
- **The rest is one row each.** `CATALOGS` gains `("th", "", include_str!("i18n/th.tsv"))`, `LANGUAGES`
  offers `th` between `ko` and `zh-TW`, `format::STYLES` gives Thailand Latin digits, comma grouping, `.` as
  the decimal mark and a day-first slash date (`12,408`, `03/10/2026 04:00`), and
  `SHIPPED_LANGUAGE_SCRIPTS`/`PHONE_SCRIPTS` name `Thai`, so a phone build asks for a Thai face.
- **Tests.** `catalogs_load_and_routed_keys_have_rows` and the keymap-strings audit now loop over `th` too;
  `the_picker_names_every_language_and_directions_follow_the_script` expects `ไทย` in the row;
  `the_language_switch_moves_the_sidebar_to_the_reading_start` paints a Thai leg (`แหล่งที่มา`,
  `รูปภาพทั้งหมด`, no English and no Korean behind it, left to right); `numbers_follow_the_language` and
  `dates_follow_the_language_order` pin `12,408` and `03/10/2026 04:00`; the font-manifest fixture gains a
  Thai face (7→8 faces, `Thai` in the coverage problems list, `Noto Sans Thai` asserted in the phone set).
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the `i18n` and `language`
  filters pass (3 + 34 tests); `cargo xtask ci`, `snapshot` and `deny` green; `cargo xtask screenshots
  --update` moved exactly the four Settings baselines, and each was read: six entries — English plus the five
  non-Latin names — sit inside the dialog at 900 px in both themes with no clipping and no overlap (this
  machine paints the non-Latin names as boxes, exactly as before, because `CRAFT_FONTS_DIR` is unset), and
  nothing else in any frame moved. No release workflow was dispatched: `crates/ui` source, the catalogue and
  four baselines touch no workflow, `apps/**`, a `Cargo.toml` or release code.
- **Files.** `crates/ui/src/i18n/th.tsv` (new); `crates/ui/src/i18n.rs`; `crates/ui/src/format.rs`;
  `crates/ui/src/font_manifest.rs`; `crates/ui/src/shell.rs` (one test); the four
  `crates/ui/tests/snapshots/settings-{short,sync}-{charcoal,paper}.png`; `docs/ROADMAP.md`, `docs/UX.md`,
  `docs/FEATURES.md`.
- **Next.** `vi.tsv`: Vietnamese is Latin-script, so it needs no new face — the same 754 keys, 11 `@plural`
  counts and `@id` row as `th.tsv`, in the audit's order; then the `CATALOGS`/`LANGUAGES`/`STYLES` rows
  (Vietnamese groups with `.` and writes `12.408`, `1,4 MB`, `03/10/2026 04:00`), `("vi", "Latn")` in
  `SHIPPED_LANGUAGE_SCRIPTS` (`Latn` is already in `PHONE_SCRIPTS`), the picker/format/headless language
  tests, and the four Settings baselines in the default set via `--update`. Read this commit's CI run first —
  `gh run list --workflow CI --limit 1 --json databaseId,conclusion,headSha` — and adopt its `screenshots`
  frames with `SORTLENS_SNAPSHOT_SET` if it is red. No release run is in flight.
## 2026-11-26 (hundred-and-ninth) — the red trunk is adopted from its own paint, and the lavapipe set gains the fifth name

- **Why this came up.** The trunk was red on `0ece80e` (`45624521129`) with exactly one failing job, `screenshots`;
  the hundred-and-eighth entry's Next named that read first, and a red trunk is the tick's slice rather than a
  new roadmap row. The other eight jobs — `ci` on Windows, Ubuntu and macOS, `deny`, `snapshot` and both `mobile`
  legs — were green, so the red was the baseline set and nothing else.
- **One artifact, one command.** `gh run download 45624521129 -n screenshot-set` wrote four frames and nothing else —
  `lavapipe/settings-{short,sync}-{charcoal,paper}.new.png` — the four Settings scenes the failing run reported, so
  no scene was left behind and no frame that matched was handed over. `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask
  screenshots --adopt <dir>` copied them into `crates/ui/tests/snapshots/lavapipe/`. The adopted
  `settings-short-charcoal.png` hashes `1f15a028…632ee`, byte-identical to the artifact's frame, so the committed
  baseline is the paint the comparing run reported and not a second paint from this machine.
- **Read, all four.** Each paints the Language row with five entries — English plus the four names — inside the
  dialog at 900 px, in both themes, with no clipping and no overlap; the two `sync` scenes still scroll their
  eleven sections with the row intact. Nothing else in any frame moved: the row grew into room it had.
- **A claim corrected, not repeated.** The hundred-and-eighth entry said CI's lavapipe set "embeds craft-fonts and
  paints them"; this artifact proves the runner paints the four non-Latin names as boxes, exactly as this machine
  does. That sentence now carries the correction in place, and no other doc makes the claim.
- **What the diff is.** Four PNGs, `88938→89386`, `93541→94097`, `232992→233440` and `245784→246335` bytes: one more
  entry in the Language row on each scene.
- **Verified.** `cargo xtask ci` green (fmt, clippy, 212 workspace tests, layers, roots, assets, fonts, version,
  attribution, a11y, docs, templates, workflows), `cargo xtask snapshot` (5 pass) and `cargo xtask deny`
  (advisories, bans, licenses, sources) green. No `cargo xtask screenshots` run was needed: no code changed and the
  default set this machine compares against is untouched — the lavapipe set only compares under Mesa, which is CI's
  job. No release workflow was dispatched: four baselines and this file touch no workflow, `apps/**`, a `Cargo.toml`
  or release code.
- **Next.** `th.tsv`: the same 754 keys, 11 `@plural` counts and `@id` row as `ko.tsv`, in the audit's order; then the
  `CATALOGS`/`LANGUAGES`/`STYLES` rows, `("th", "Thai")` in `SHIPPED_LANGUAGE_SCRIPTS` with `Thai` in `PHONE_SCRIPTS`
  and `fonts::TRANSLATED_SCRIPTS`, the picker/format/headless language tests, the four Settings baselines in the
  default set via `--update`, and this commit's CI run read first — `gh run list --workflow CI --limit 1 --json
  databaseId,conclusion,headSha`; its `screenshots` leg is the red that gets adopted the same way. No release run is
  in flight.

## 2026-11-26 (hundred-and-eighth) — the Korean catalogue ships, and the language row gains 한국어

- **Why this came up.** The hundred-and-seventh entry's Next named `ko.tsv` first: the same 754 keys and
  11 `@plural` counts as `zh-TW.tsv`, in the audit's order, so every routed key already had an English
  source and three catalogues to answer it.
- **One catalogue, the same keys in the same order.** `crates/ui/src/i18n/ko.tsv` was built by zipping
  `zh-TW.tsv`'s keys with Korean translations — 754 key rows, 11 `@plural` rows and the `@id` row, whose
  name is `한국어`. The header is Korean's own and promises the same thing: a missing row is English,
  never an error. `I18n::check_all()` parses all 769 lines with no bad row.
- **Everything the picker needs.** `CATALOGS` gains the catalogue, `LANGUAGES` becomes
  `en, ar, ja, ko, zh-TW`, `format::STYLES` gains Korea's numbers and dates (comma grouping, `.` decimal,
  year-first `2026.10.03 04:00`), and `font_manifest::SHIPPED_LANGUAGE_SCRIPTS` gains `("ko", "Hang")`
  with `Hang` added to `PHONE_SCRIPTS` and `fonts::TRANSLATED_SCRIPTS`. A phone build now asks for the
  Hangul face a Korean interface needs, and `the_shipped_language_table_names_exactly_the_picker_s_languages`
  holds the table and the picker together.
- **Tests that fail when the behaviour is absent.** Both catalogue audits now loop `ko` (a routed key with
  no row, or the shortcut editor's 175 strings and 11 counts, fails); the picker test expects `한국어`
  between `日本語` and `繁體中文` and asserts Korean resolves left-to-right; the formatting tests pin
  `12,408` and `2026.10.03 04:00`; and the headless
  `the_language_switch_moves_the_sidebar_to_the_reading_start` gained a Korean leg that paints `소스` and
  `모든 사진`, finds no English behind them, and asserts the panels stay at the left edge. 34
  language-filtered tests, both audits and the format/font tests pass.
- **Screenshots.** `cargo xtask screenshots --update` regenerated exactly the four Settings baselines
  (`settings-short-{charcoal,paper}`, `settings-sync-{charcoal,paper}`); both frames were read: the
  Language row now carries English plus four names and does not overflow. This machine has no CJK or
  Hangul face embedded, so the four non-English names paint as boxes in the committed reference set —
  the same honest state `ar`/`ja`/`zh-TW` have always had here. *(Corrected in the hundred-and-ninth entry:
  the runner's lavapipe set paints them as boxes too, so the claim that CI embeds craft-fonts here was wrong.)*
- **Docs reconciled in the same commit.** ROADMAP's SL-L07 row gained the Korean paragraph and now names
  only `th`/`vi` as open; its M9 state line and Phase 4 history dropped `zh-TW`/`ko`; UX.md §11 gained the
  Korean paragraph and the same two-language remainder.
- **Verified.** `cargo fmt --all -- --check`, `cargo clippy -p sortlens-ui --all-targets -- -D warnings`,
  `cargo xtask snapshot` and `cargo xtask deny` green; `cargo xtask ci` stopped at its first step on
  formatting, which was fixed with `cargo fmt --all`; its rerun then ran 212 workspace tests and found
  one wrong expectation — this tick had listed `Hang` after `Hani` in the manifest test, and the
  coverage list follows `SHIPPED_LANGUAGE_SCRIPTS` order (`Hang` before `Hani`). Fixed; the eight
  font-manifest tests pass, and every remaining `ci` step (layers, roots, assets, fonts, version,
  attribution, a11y, docs, templates, workflows) was then run and is green. No release
  workflow was dispatched: the slice touches `crates/ui/**` and docs only, not a workflow, `apps/**`, a
  `Cargo.toml` or release code.
- **Next.** Read the CI run for this commit first; `screenshots` is expected red on the four Settings
  scenes, whose lavapipe baselines still carry four language names. Adopt its `screenshot-set` artifact
  with `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt <dir>` rather than fetching frames
  twice. Then `th.tsv` (script `Thai`; add it to `PHONE_SCRIPTS`/`SHIPPED_LANGUAGE_SCRIPTS` and the
  `format` table), then `vi`.

## 2026-11-26 (hundred-and-seventh) — a red screenshots run is fixed from its own paint, in one command

- **Why this came up.** The trunk was red on `ef1627b` (`45619063394`) with four lavapipe baselines
  behind the language row, and the fix is mechanical: the failing run uploads what it painted as
  `*.new.png`, so the set is repaired by adopting that paint. It was done by hand — download, four
  renames into `crates/ui/tests/snapshots/lavapipe/`, and a hash check to be sure the file being
  adopted was the paint the run reported — which is why the same four files were fetched twice.
  This tick had the artifact open and a byte-identical result to the fix commit when `5e3ff04`
  landed, so the ritual is now a command instead of a paragraph someone has to remember.
- **What ships.** `cargo xtask screenshots --adopt <dir>` takes the folder `gh run download` wrote and
  copies every `*.new.png` under it to `crates/ui/tests/snapshots/<set>/<name>.png`. It refuses
  three things rather than guessing: a folder that does not exist (named in the error), a folder with
  no frame in it (a run that matched uploads none), and `--update` in the same breath (`--update`
  rewrites the set from *this* machine's adapter, `--adopt` from the runner's). It also requires
  `SORTLENS_SNAPSHOT_SET`, so a runner's paint cannot be mistaken for the committed reference set,
  and it normalises that name with the harness's own rule (`Mesa LLVM 15.0` → `mesa-llvm-15-0`,
  `../..` → no set) so a frame lands in the folder the comparing run actually reads.
- **The rule is copied, and the copy is held to it.** The harness owns `set_dir` in
  `crates/ui/tests/screenshots.rs`, a test binary `xtask` cannot link, so `xtask`'s copy is asserted
  against the same cases; a set name that spelled a different folder would adopt frames the gate never
  reads, which is the one way this command could be worse than doing it by hand.
- **Tests that fail when the behaviour is absent.**
  `an_adopted_frame_lands_where_the_comparing_run_looks` (a frame keeps its scene's name and loses
  `.new`, a `.diff.png` is a report and not a baseline, an already-committed `.png` is not a
  hand-over), `a_set_name_is_a_directory_like_the_harness_spells_it`,
  `adopting_a_downloaded_folder_writes_the_sets_baselines` (a real folder with one frame, one diff
  image and one note: exactly one file lands, byte for byte, and the diff image does not), and
  `a_missing_download_folder_names_the_folder_it_was_given`. `cargo test -p xtask` is 53 tests.
- **Files.** `xtask/src/main.rs` (argument parsing, `adopt_screenshots`, `adopt_frames`,
  `adopted_baseline`, `set_dir`, `frames_under`, the `--adopt` line in `--help`, four tests);
  `docs/AUTONOMOUS_AGENT.md` (the gate step names the command); this entry.
- **Not shipped, named.** The command replaces the tool's `screenshots` subcommand only: nothing here
  changes which adapter a committed set belongs to, and the reference set is still painted by this
  machine. The Korean catalogue (`ko.tsv`) is untouched — see Next.
- **Two runs on the trunk, and one is in flight.** `45619063394` on `ef1627b` is red in
  `screenshots` only (the four Settings baselines). `5e3ff04` adopted exactly those four frames —
  this tick hashed its artifact `screenshot-set` against the committed files and they are identical,
  which is the proof that fix is complete and not a guess — and `45620147702` on `5e3ff04` was
  still painting when this tick read the list. That run is the next tick's first read, one
  `gh run view 45620147702 --json status,conclusion,jobs`.
- **Verified.** `cargo clippy -p xtask --all-targets -- -D warnings` and `cargo test -p xtask` (53
  pass, four new) clean; `cargo xtask ci`, `cargo xtask snapshot` and `cargo xtask deny` green. No
  baseline moved, so no `screenshots` run was needed and no image was read: this slice paints
  nothing. No release workflow was dispatched — `xtask/src/main.rs` is not
  `.github/workflows/**`, `xtask/src/workflow.rs`, `apps/**`, a `Cargo.toml` or release code.
- **Next.** The Korean catalogue (`ko.tsv`, script `Hang`): the same 754 keys and 11 `@plural`
  counts as `zh-TW.tsv` in the audit's order, then its `CATALOGS`/`LANGUAGES`/`STYLES`/
  `font_manifest` rows, then `th` and `vi`. Read `45620147702` first, and if it is red, adopt its
  frames with the command this tick shipped rather than fetching them twice.

## 2026-11-26 (hundred-and-sixth) — the Traditional Chinese catalogue ships, and the language row gains 繁體中文

- **Why this came up.** The hundred-and-fifth entry's Next named it first: `zh-TW` was the next of the
  four catalogues the SL-L07 row leaves open, and every routed key already had an English source string
  and a row in `ar.tsv`/`ja.tsv` to translate.
- **One catalogue, the same keys in the same order.** `crates/ui/src/i18n/zh-TW.tsv` came from zipping
  `ja.tsv`'s keys with Traditional Chinese translations: 754 key rows, 11 `@plural` rows and the `@id`
  row, 769 lines with its three comments. That order is the audit's own order, so the catalogue answers
  every routed key, the shortcut editor's 175 source strings and its eleven `@plural` counts at once —
  the tests below hold it to all three.
- **A row that translates to itself is debt, so one row was really translated.** `1:1  Z` would have been
  the same string in this language, and the audit counts a row whose translation equals the English as
  missing; the row now reads `實際大小  Z`, which is what a Taiwanese zoom control says. The other rows
  that stay Latin (`Z`, `S`, `C`, `F`) keep their keys and their catalogue rows.
- **Wiring.** `i18n.rs` gains the `CATALOGS` entry and offers `zh-TW` in `LANGUAGES`; the picker names it
  `繁體中文`, and since `I18n` normalises the stored code, `I18N.code()` answers `zh-tw` — the test asserts
  the normalised code it will persist, not the spelling the picker holds. The direction comes from the
  script: `Hani` resolves `Ltr`, so the shell keeps its left-to-right geometry.
- **Numbers and dates follow the language too.** `format.rs` gains its `STYLES` row — Latin digits, comma
  grouping, `.` decimal mark, year-first dates with slashes — so a count reads `12,408` and a date
  `2026/10/03 04:00`, and the style table's own test now holds `zh-TW` to the picker the same way it holds
  `en`, `ar` and `ja`.
- **The font tables name the script, not a face.** `font_manifest.rs`'s shipped-language table gains
  `("zh-TW", "Hani")`, so `PHONE_SCRIPTS` now asks the phone build for a Han face, and `fonts.rs` lists
  `Hani` among the translated scripts. The manifest test grew a Traditional Chinese face
  (`Noto Sans TC … | Hani`) and now asserts the phone filter keeps it — a build whose manifest lacks that
  face fails `select` by name rather than painting boxes silently.
- **Named as not shipped.** No Traditional Han face is committed here: `fonts/embedded.txt` still lists
  only the Japanese and Latin faces, and this development checkout has no craft-fonts directory, so the
  glyphs this turn painted were egui's own fallback (boxes for every CJK name, exactly as Arabic and
  Japanese already paint in the committed baselines). The catalogue is complete and the build now asks
  for the face; the face itself is a craft-fonts checkout, not this repository. Also still open: `ko`,
  `th`, `vi` and the RTL screenshots.
- **Tests that fail when the behaviour is absent.** `catalogs_load_and_routed_keys_have_rows` loops
  `["ar", "ja", "zh-TW"]` over every routed key, so deleting any translated row fails by name;
  `every_keymap_editor_string_is_in_both_catalogues` holds the shortcut editor's strings and counts to the
  same loop; `the_picker_names_every_language_and_directions_follow_the_script` asserts the fourth radio and
  its `zh-tw` code. `format::tests::numbers_follow_the_language` and `dates_follow_the_language_order` assert
  the Taiwan shapes, and `every_offered_language_has_one_style` refuses a language without a style.
  `font_manifest`'s phone test asserts `Noto Sans TC` survives the filter. In `shell.rs`,
  `the_language_switch_moves_the_sidebar_to_the_reading_start` gained a Traditional Chinese leg: after the
  Arabic frames it switches to `zh-TW` and paints the real shell, asserting `來源` and `所有照片` are painted,
  `SOURCES` and `المصادر` are not, and the direction is `Ltr` — so reverting the catalogue or the direction
  table fails there.
- **Visual verification.** The four Settings baselines moved and only the language row changed:
  `crates/ui/tests/snapshots/settings-short-{charcoal,paper}.png` and `settings-sync-{charcoal,paper}.png`,
  regenerated by `cargo xtask screenshots --update` and opened: the row now reads `English` and three
  boxes, one more than before, with the rest of the dialog, the sidebar and the photo byte-identical. The
  other 23 committed scenes did not move, which is the proof the English interface is untouched. The boxes
  are the missing Han/Arabic faces, not a painted word — the headless test above proves the geometry and
  the words; the glyphs need a font build.
- **Files.** `crates/ui/src/i18n/zh-TW.tsv` (new), `crates/ui/src/i18n.rs` (module prose, `CATALOGS`,
  `LANGUAGES`, three test loops, picker names), `crates/ui/src/format.rs` (style and two assertions),
  `crates/ui/src/font_manifest.rs` (shipped table, `PHONE_SCRIPTS`, test manifest and counts),
  `crates/ui/src/fonts.rs` (`TRANSLATED_SCRIPTS`), `crates/ui/src/shell.rs` (one test's zh-TW leg),
  `crates/ui/tests/snapshots/settings-*.png` (four baselines), `docs/ROADMAP.md` (the SL-L07 row),
  `docs/UX.md` (§7), this entry.
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the catalogue, format,
  font-manifest and shell tests pass; `cargo xtask ci` green through every gate (fmt --check, workspace
  clippy with `-D warnings`, the full test suite, layers, roots, assets, fonts, version, attribution, a11y,
  docs, templates and workflows) — its first invocation this turn exited 1 with its output lost in the
  pipe, so each gate was re-run on its own (all green) and the whole gate was then re-run to a log and
  exited 0; `cargo xtask snapshot` (5 tests), `cargo xtask deny` (advisories, bans, licenses, sources all
  ok) and `cargo xtask screenshots` (27 scenes, the four Settings baselines being the only diff, which is
  what `--update` wrote). There is no `cargo xtask wasm` in this workspace, so no wasm gate ran. No
  release dispatch: this slice touches catalogues, UI sources, tests, baselines and docs, none of which is
  a build or release surface. No release run in flight.
- **Next.** The Korean catalogue (`ko.tsv`, script `Hang`): the same 754 keys and 11 plurals as `zh-TW.tsv`,
  then `th` and `vi`; a Traditional Han (and an Arabic) face in the craft-fonts manifest is what turns the
  boxes in these baselines into glyphs, and the RTL screenshot set waits on that Arabic face. No release
  run in flight.

## 2026-11-26 (hundred-and-fifth) — the bottom bar's destinations speak the interface language

- **Why this came up.** The hundred-and-fourth entry's Next named it, and so did the ROADMAP's SL-L07 row:
  every view's own chrome was a catalogue row, but the nine names that switch between them still came from
  `View::label()`, so a Japanese window read `比較` in the canvas header and `Compare` on the tab beside it,
  and the narrow bar's menu was English over a translated screen.
- **One word, one source.** `View::label()` stays the English source of truth — it is also what the command
  palette searches and the reports name — and a new `view_word(view)` looks its catalogue row up, exactly
  as `family_word`/`source_word` do for the Suggestions tab. The tab row's four painted names, the collapsed
  menu's button and its nine rows, the `Views, now {view}` assistive name and the canvas header that repeats
  the current destination all read that one function, so the bar and the header cannot describe different
  places.
- **Measured from the words, not the window.** `bottom_bar` measures its tabs from the labels it is about to
  paint, so routing the names means the collapse threshold follows the language: `ソフトフォーカス` is wider than
  `Soft Focus`, and the row collapses at the width where the translated row genuinely stops fitting. That is
  the behaviour the bar was built for; no constant moved.
- **Seven rows already existed, two were added.** `Library`, `For You`, `Cull`, `Compare`, `Similar`,
  `Insights` and `Soft Focus` were already rows because each view's own title is painted from one. `Detail`
  and `Places` were not, so `ar.tsv`/`ja.tsv` each gained them (`تفاصيل`/詳細, `الأماكن`/場所), nine rows per language in total.
- **English is unchanged, which is why no baseline moved.** The tabs paint the same Latin strings in the same
  order at the same widths; the 27 committed scenes are byte-identical.
- **Named as not shipped.** The composed assistive sentence (`Views, now …`) keeps its English wrapper while
  the destination inside it is the catalogue's word, and the `… arrives in {milestone}` notice stays English
  by the notices' own call (and never shows: every destination opens today). The four remaining catalogues
  (zh-TW/ko/th/vi) and the RTL screenshots are still the SL-L07 row's open items, and the phone builds still
  need a device.
- **Tests that fail when the behaviour is absent.** `the_bottom_bar_destinations_speak_the_interface_language`
  first holds every `View::ALL` entry to a row in each catalogue, so a tenth destination cannot arrive
  English-only. It then paints the real `bottom_bar` at 1440×300 in English (all nine names, unchanged),
  at 1600 in Japanese and Arabic (each of the nine in that language, with the English name asserted absent),
  and at 820 in Japanese, where the collapsed tab is `ライブラリ` and a click on it opens a menu that still
  names all nine in Japanese. Finally it paints the whole shell at 1280×800 with the docks closed, so the
  only two places `詳細` can be are the canvas header and the bar: a new `word_rects` helper proves one sits
  in the top 150 points and one below 600, that no `Detail` is painted behind the Japanese frame, and that
  English still names the header. Revert the routing and the Japanese frame goes English — every assertion
  above fails.
- **Files.** `crates/ui/src/shell.rs` (`view_word`; `bottom_bar`, `view_tab`, `view_menu` and `canvas`'s
  title; `word_rects` and one test), `crates/ui/src/i18n/ar.tsv` and `ja.tsv` (2 rows each), `docs/ROADMAP.md`
  (the SL-L07 row, including the two stale `what stays English` clauses it had accumulated), `docs/UX.md`
  (§7), this entry.
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the new test, the catalogue
  audit and the existing collapse test pass. `cargo xtask ci` green through every gate (fmt --check after
  `cargo fmt --all`, clippy `--workspace --all-targets` with `-D warnings`, the full test suite, layers,
  roots, assets, version, attribution, a11y, docs, templates and workflows); `cargo xtask snapshot` (5 tests),
  `cargo xtask deny` (`advisories ok, bans ok, licenses ok, sources ok`) and `cargo xtask screenshots`
  (27 scenes, byte-identical because English has no catalogue). There is no `cargo xtask wasm` in this
  workspace, so no wasm gate ran. No release dispatch: this slice touches `crates/ui` source, catalogues,
  tests and docs, so no build or release surface changed. No release run in flight.
- **Next.** The four remaining catalogues (zh-TW/ko/th/vi), starting with zh-TW: the same 175-odd routed keys
  as `ar.tsv`/`ja.tsv`, added as `crates/ui/src/i18n/zh-TW.tsv` and offered in `LANGUAGES`; then the RTL
  screenshots, which need a font build with an Arabic face. No release run in flight.
## 2026-11-26 (hundred-and-fourth) — the For You view speaks the interface language, and its cards are worded from the crate's own pieces

- **Why this came up.** The hundred-and-third entry's Next named it: after Soft Focus, For You was the last
  view painting its own chrome in English — the title, the hint, the empty state, and the `memories.headline()`
  and `card.detail()` sentences, which were English composed inside `sortlens-memories`. So a Japanese window
  painted `For You` over a card headed `Never looked at` and counted `1 photo · since 2020`.
- **The crate hands back pieces, not sentences.** `Memories::headline_parts()` answers the card count and the
  kinds in display order, `Card::title_parts()` answers a calendar day (month, day) or the kind whose label is
  the heading, and `Card::detail_parts()` answers the shown count, the matched count when the card was cut down
  and the oldest capture year. `headline()` and `detail()` are now joined from those pieces, so the CLI, the
  `memories.list` JSON and the screen cannot describe different cards. `Card` gained a `day: Option<Date>` field
  so a screen can word "June 24" without parsing it back out of English.
- **What is routed.** 14 new rows in each of `ar.tsv` and `ja.tsv`: `For You`, the view's UTC hint, its empty
  state's two lines, the three kind words (`On this day`, `This week in past years`, `Never looked at`), the
  headline template `{cards} card(s): {kinds}`, the heading template `{month} {day}`, the detail templates
  `{shown} of {total} photos`, `{count} photo`/`{count} photos` and `{photos} · since {year}`, and the hover
  `{name} · {kind}`. The month and the day are the catalogues' own month rows already shipped for `Insights` plus
  `format::count`, so a Japanese card reads `6月24日` with `写真 1 枚 · 2020 年以降` and an Arabic one writes
  `١ صورة · منذ ٢٠٢٠`.
- **A year is a label, not a quantity.** `format::count(2020)` writes `2 020` in English — the grouping every
  three digits gets — so years go through a new `format::year`, which keeps four digits and the language's own
  digits (`2020`, `٢٠٢٠`) and never groups them.
- **English is unchanged, which is why no baseline moved.** A `tl!` with no row is the English text itself, and
  the headings and details are joined exactly as the crate joins them: `For You`, `1 card(s): Never looked at`,
  `Never looked at`, `1 photo · since 2020`. The 27 committed scenes are byte-identical.
- **Named as not shipped.** The bottom bar's destination names still come from `View::label()`, so the tab beside
  this translated title still reads `For You` in a Japanese window; the four remaining catalogues (zh-TW/ko/th/vi)
  and the RTL screenshots are still open in the SL-L07 row; each photograph's own file name is a value and stays
  as it is on disk, and the assistive names (`crate::a11y`) stay English, which the row does not count as painted
  chrome.
- **Tests that fail when the behaviour is absent.** `the_for_you_view_speaks_the_interface_language` paints the
  real `for_you_view` at 1000×760 over a library holding one photograph captured on 29 February 2020 — a date no
  ordinary year has, so the card's words do not depend on when the test runs — in English (the title, the whole
  hint, the card's heading and `1 photo · since 2020`, and the headline naming the card), Japanese (`あなたへ`,
  its hint, `まだ見ていない写真`, `写真 1 枚 · 2020 年以降`, and a headline containing `カード` and the kind, with
  the English chrome asserted absent) and Arabic (`لك`, its hint, `لم تُستعرض قط`, `١ صورة · منذ ٢٠٢٠`, and a
  headline naming `بطاقة`). It then paints the empty library in all three languages, so a first launch's
  `Nothing to resurface yet` and its explanation are the rows rather than a second literal. `fmt` isolates values,
  so the frame is read through the same isolate filter the other language tests use. In `sortlens-memories`,
  `the_english_is_joined_from_the_pieces_a_screen_words` rebuilds the heading, the detail and the headline from
  their pieces and holds them to the English the CLI prints, so a piece that stops being filled is caught there.
  The catalogue audit in `i18n.rs` reads the new `tl!`/`fmt` literals and fails if either catalogue loses a row.
- **Files.** `crates/memories/src/lib.rs` (`TitleParts`, `DetailParts`, `HeadlineParts`, `Card::day`,
  `title_parts`/`detail_parts`/`headline_parts`, one test); `crates/ui/src/shell.rs` (`for_you_view`,
  `memory_kind_word`/`month_key`/`memory_card_title`/`memory_card_detail`/`memory_headline`, one test);
  `crates/ui/src/format.rs` (`year`); `crates/ui/src/i18n/ar.tsv` and `ja.tsv` (14 rows each); `docs/ROADMAP.md`
  (the SL-L07 row); `docs/UX.md` (§5.11); this entry.
- **Verified.** Local: `cargo clippy` is part of `cargo xtask ci` below; the new UI test, the memories crate's 10
  tests and `catalogs_load_and_routed_keys_have_rows` pass. `cargo xtask ci` green through every gate (fmt --check
  after `cargo fmt --all`, clippy `--workspace --all-targets` with `-D warnings`, the full test suite, layers,
  roots, assets, version, attribution, a11y, docs, templates and workflows); `cargo xtask snapshot` (5 tests),
  `cargo xtask deny` (`advisories ok, bans ok, licenses ok, sources ok`) and `cargo xtask screenshots` (27
  scenes, `for_you_paints_in_both_themes` among them, byte-identical to the committed baselines because English
  has no catalogue and the pieces join in the same order). There is no `cargo xtask wasm` in this workspace, so no
  wasm gate ran. No release dispatch: this slice touches `crates/ui`, `crates/memories`, tests and docs, so no
  build or release surface changed.
- **Next.** The four remaining catalogues (zh-TW/ko/th/vi) and the RTL screenshots of the painted interface, which
  is what the SL-L07 row still lists; the bottom bar's destination labels (`View::label()`) are still an unrouted
  English key on every view. No release run in flight.
## 2026-11-26 (hundred-and-third) — the Soft Focus view speaks the interface language

- **Why this came up.** The hundred-and-second entry's Next named it: after Compare, Soft Focus was the last
  view painting its own chrome in English — the title and subtitle, `Measure sharpness` and its hover, the
  progress line, the histogram caption, the review's score words and the `Select N photo(s)` action.
- **What is routed.** 17 new rows in each of `ar.tsv` and `ja.tsv`; `Sharpness` and `{done} of {total}`
  already had rows from the Insights slice, so they are reused rather than duplicated. The title, `A relative
  cut, never a verdict`, the hint that says what a relative cut can claim, `Measuring`, `Measure sharpness`
  and its hover, `{n} photo(s) have no current score`, the `Nothing measured yet…` empty review, the caption
  `Softest {percent}% · scores {low} to {high}`, `Select {n} photo(s)` and its hover, `Nothing at the cut`,
  and the cut's own words — `Nothing is scored yet`, the "every photo scored the same" refusal, the tie
  sentence, the single-photo sentence and `{count} photos at or below the cut ({percent}%)`.
- **The cut's words live in `sharpness.rs`, not the shell.** `Review::cut_label` is arithmetic and prose in
  one place, so routing it there means the catalogue audit (`catalogs_load_and_routed_keys_have_rows`) now
  reads `sharpness.rs` as a third routed source beside `shell.rs` and `theme.rs`; a row that goes missing
  from a catalogue fails the audit rather than showing English. Its English assertions still hold unchanged.
- **The numbers are templates, and the digits are the language's.** A new `format::decimal` writes a value
  with the language's decimal mark and digits (`0.50`, `٠٫٥٠`), so the score range and the percentage are
  placeholders filled through `format_count`/`format::decimal` rather than `{:.2}` inside the shell.
  English is byte-identical: `Softest 10% · scores 0.50 to 4.50`, `1 photo at or below the cut`,
  `Select 1 photo(s)` — which is why the committed scenes did not move. An Arabic review reads
  `الأضعف ١٠% · الدرجات من ٠٫٥٠ إلى ٤٫٥٠` and `تحديد ١ صورة`, Japanese
  `最も柔らかい 10% · スコア 0.50〜4.50` and `1 枚の写真を選択`.
- **Named as not shipped.** The notices (`Measured … photo(s)`, `Sharpness pass stopped after …`,
  `Every photo already has a current score`, `Selected the … softest`) stay English — they belong to the
  notice class, the same call the Places, Detail and Compare entries recorded. The slider's assistive name
  (`Softest share of the library to review`) stays English because it is what assistive technology speaks,
  which the ROADMAP row does not count as painted chrome. The bottom bar's destination names still come from
  `View::label()` and are not routed yet, so the tab beside this translated title still reads `Soft Focus`
  in a Japanese window; that gap is recorded in the ROADMAP row and UX.md, not implied done.
- **Tests that fail when the behaviour is absent.** `the_soft_focus_view_speaks_the_interface_language`
  paints the real `soft_focus_view` at 1000×760 over five photographs in three states: nothing measured
  (the title, subtitle, `Sharpness`, `Measure sharpness` and the `5 photo(s) have no current score`
  line, plus the empty review's whole sentence, in English then Japanese then Arabic), five different
  scores (the caption, `1 photo at or below the cut` and `Select 1 photo(s)`; Arabic's own digits and
  `٫`, with the English action and caption asserted absent), and five identical scores (the refusal
  sentence in Arabic). `fmt` isolates a value, so the frame is read through the same isolate filter the
  other language tests use. `a_decimal_writes_the_languages_mark_and_digits` holds `format::decimal` to
  English's `{:.2}` shape and to Arabic's `٫`, and the catalogue audit reads all 17 new rows.
- **Files.** `crates/ui/src/shell.rs` (`soft_focus_view`, `sharpness_section`, one test),
  `crates/ui/src/sharpness.rs` (`cut_label`), `crates/ui/src/format.rs` (`decimal`, one test),
  `crates/ui/src/i18n.rs` (the audit's source list), `crates/ui/src/i18n/ar.tsv` and `ja.tsv` (17 rows
  each), `docs/ROADMAP.md` (the SL-L07 row), `docs/UX.md` (§5.8 and the i18n paragraph), this entry.
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the new test,
  `a_decimal_writes_the_languages_mark_and_digits` and `catalogs_load_and_routed_keys_have_rows` pass, and
  the sharpness module's nine tests are unchanged and green. `cargo xtask ci` green through every gate
  (fmt --check, clippy `--workspace --all-targets` with `-D warnings`, the full test suite, layers, roots,
  assets, version, attribution, a11y, docs, templates and workflows); `cargo xtask snapshot` (5 tests),
  `cargo xtask deny` (`advisories ok, bans ok, licenses ok, sources ok`) and `cargo xtask screenshots`
  (27 scenes, `soft_focus_paints_in_both_themes` among them, byte-identical to the committed baselines).
  There is no `cargo xtask wasm` in this workspace (removed in the sixty-first entry), so no wasm gate ran.
  No release dispatch: this slice touches `crates/ui` source, catalogues, tests and docs, so no build or
  release surface changed.
- **Next.** For You is the last view still painting English chrome — its title and hint, its empty state,
  and the `memories.headline()`/`card.detail()` sentences, which need pieces in `sortlens-engine` the way
  the Insights highlights strip got them. Then the four catalogues (zh-TW/ko/th/vi) and the RTL screenshots.
  The bottom bar's destination labels (`View::label()`) are still an unrouted English key. No release run
  in flight.

## 2026-11-26 (hundred-and-second) — the Compare view speaks the interface language

- **Why this came up.** The hundred-and-first entry's Next named Compare and Soft Focus as the two views still
  painting English. Compare's facts row — the one line under the two panes that says what actually differs — was
  composed in the shell out of English words: `stars 5 vs 3`, `label none vs none`, `pixels 3000×2000 vs
  2000×2000`, whatever Settings → Language said.
- **What is routed.** Seven new rows in each of `ar.tsv` and `ja.tsv`: the six fact names (`stars`, `label`,
  `exposure`, `length`, `size`, `pixels`), the join `{left} vs {right}` and the pixel pair
  `{left_width}×{left_height} vs {right_width}×{right_height}`. A photograph with no colour label reuses the
  existing `none` row, and the ratings go through `format_count`, so English still writes `5 vs 3`, Japanese
  writes `5 対 3`, and Arabic writes `٥ مقابل ٣` with its own digits.
- **The pair is a template, not a sentence.** `compare_pair` fills `{left} vs {right}` through
  `crate::i18n::fmt`, which isolates each side so a value cannot reorder the join around itself; the pixel pair is
  one template with four placeholders rather than two joined cells, because a language may put the × where it wants.
  Dimensions keep their raw digits (`3000×2000`), the way the Details tab's `Dimensions` row already prints them;
  the ratings, which that tab already writes in the language's digits, follow suit.
- **Named as not shipped.** Each side's value is still the engine's own word: a colour token (`red`), an exposure
  sentence, `format_bytes` and the `—` of a still. The notices that open a compare (`Pick a photo first`,
  `Nothing to compare with`, `Compare needs two photos…`) stay English — they belong to the notice class, not the
  view chrome, the call the Places entry recorded. Both are in the ROADMAP row and UX.md rather than implied done.
- **Tests that fail when the behaviour is absent.** `the_compare_view_speaks_the_interface_language` paints the real
  `detail_view` at 1000×760 over two photographs with the second pane set and `View::Compare`, and reads the facts
  row in English (`stars`, `0 vs 0`, `label`, `none vs none`, `pixels`, `0×0 vs 0×0`), then Japanese
  (`星`, `0 対 0`, `ラベル`, `なし 対 なし`, `ピクセル`, `0×0 対 0×0`) and Arabic (`النجوم`,
  `٠ مقابل ٠`, `التسمية`, `لا يوجد مقابل لا يوجد`, `البكسلات`, `0×0 مقابل 0×0`), with the English names and
  joins asserted absent from the last two. `fmt` isolates values, so the frame is read through the same isolate
  filter the Detail and Places tests use. `a_compare_names_what_actually_differs` still holds the English values
  (`5 vs 3`, `none vs none`, `3000×2000 vs 2000×2000`) and the "two copies say little" claim, so a template
  that stopped filling or a name that stopped being a row is caught. The catalogue audit reads the seven new
  `tl!`/`fmt` literals and fails if either catalogue loses a row.
- **Files.** `crates/ui/src/shell.rs` (`compare_facts`, `compare_pair`, one test), `crates/ui/src/i18n/ar.tsv`
  and `ja.tsv` (seven rows each), `docs/ROADMAP.md` (the SL-L07 row), `docs/UX.md` (§5.4, §5.6 and the i18n
  paragraph), this entry.
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the new test,
  `a_compare_names_what_actually_differs` and `catalogs_load_and_routed_keys_have_rows` pass. `cargo xtask ci`
  green through every gate (fmt --check, clippy `--workspace --all-targets` with `-D warnings`, the full test
  suite, layers, roots, assets, version, attribution, a11y, docs, templates and workflows); `cargo xtask snapshot`
  (5 tests), `cargo xtask deny` (`advisories ok, bans ok, licenses ok, sources ok`) and `cargo xtask screenshots`
  (27 scenes, `detail_and_compare_paint_in_both_themes` among them, byte-identical to the committed baselines —
  English has no catalogue, so a `tl!` with no row is its own key, and the routed facts row paints the same
  glyphs). There is no `cargo xtask wasm` in this workspace (removed in the sixty-first entry), so no wasm gate
  ran. No release dispatch: this slice touches `crates/ui` source, catalogues, tests and docs, so no build or
  release surface changed.
- **Next.** Soft Focus — its measure button, its score words, its histogram and its select action — then For You,
  then the four catalogues (zh-TW/ko/th/vi) and the RTL screenshots. No release run in flight.

## 2026-11-26 (hundred-and-first) — the Detail view and the Details tab speak the interface language

- **Why this came up.** The hundredth entry's Next named it: after Places, the Detail view still painted
  `Nothing selected`, `Previous`/`Next`, `Lookahead` and its keyboard hint in English, and the inspector's
  Details tab — the sidecar rows that give the view its point — painted `Kind`, `Camera`, `Rating` and every
  value beside them in English whatever Settings → Language said.
- **What is routed.** 43 new rows in each of `ar.tsv` and `ja.tsv`; `none` and `{position} of {total}`
  already had rows, so they are reused rather than duplicated. The empty state and its `Back to the
  grid` button, the `Open`/`Show in folder`/`Export…` labels with their hover sentences, `Close  Esc`, the
  `1:1  Z`/`Fit  Z` toggle, `Swap  S`, `Close compare  C`, `Compare  C`, `Previous`/`Next`,
  `Loading preview…`, `No preview to measure`, `Lookahead` and the keyboard hint; and on the Details tab every
  row name (`Kind`, `Folder`, `Size`, `Dimensions`, `Camera`, `Lens`, `Exposure`, `Captured`, `Rating`,
  `Flag`, `Label`, `Status`), its kind values (`Raw`/`Image`/`Video`), its flag values
  (`none`/`pick`/`reject`), its state values (`missing`/`present`), the empty hint and the rating hint.
- **The numbers are templates.** The position line is `{position} of {total}` filled with `format_count`, so
  Arabic writes `١ من ٢`; the rating is `{rating} of 5`, so a zero reads `٠` in Arabic. `flag_word` maps the
  core `Flag` to a catalogue word, which is why the inspector stops printing the op log's `pick` token in a
  Japanese interface.
- **Tests that fail when the behaviour is absent.** `the_detail_view_speaks_the_interface_language` paints the
  real `detail_view` at 1000×760 over two photographs whose EXIF names a camera: English first (the controls,
  the compare row, `1 of 2`, `Lookahead` and the keyboard hint, plus every Details row and `0 of 5`), then the
  empty selection (the three empty-state strings), then Japanese (`前へ`, `実寸  Z`, `2 中 1`, `評価`,
  `0 / 5`, `なし` and no `Previous`/`Kind`), then Arabic (`السابق`, `الحجم الأصلي  Z`, `١ من ٢`,
  `التقييم`, `٠ من 5`, `لا يوجد`). `fmt` isolates a value so it holds its direction, so the frame is read
  through the same isolate filter the Places test uses. The catalogue audit reads the new `tl!`/`fmt`
  literals and fails if either catalogue loses a row.
- **Files.** `crates/ui/src/shell.rs` (`detail_view`, `detail_actions`, `detail_names`, `details`,
  `flag_word`, one test), `crates/ui/src/i18n/ar.tsv` and `ja.tsv` (43 new rows each), `docs/ROADMAP.md`
  (the SL-L07 row's prose), `docs/UX.md` (§5.4 and the i18n paragraph), this entry.
- **Verified.** `cargo xtask ci` green through every gate (fmt --check, clippy `--workspace --all-targets`
  with `-D warnings`, the full test suite, layers, roots, assets, version, attribution, a11y, docs, templates
  and workflows); the new test and `catalogs_load_and_routed_keys_have_rows` pass; `cargo xtask snapshot`
  (5 tests), `cargo xtask deny` (`advisories ok, bans ok, licenses ok, sources ok`) and `cargo xtask
  screenshots` (27 scenes, `detail_and_compare_paint_in_both_themes` among them, byte-identical to the
  committed baselines — English has no catalogue, so a `tl!` with no row is its own key). There is no
  `cargo xtask wasm` in this workspace (removed in the sixty-first entry), so no wasm gate ran. No release
  dispatch: this slice touches `crates/ui` source, catalogues, tests and docs, so no build or release surface
  changed.
- **Next.** Compare and Soft Focus are the next two unrouted views, then For You, then the four catalogues
  (zh-TW/ko/th/vi) and the RTL screenshots. No release dispatch.

## 2026-11-26 (hundredth) — the Places view speaks the interface language

- **Why this came up.** The ninety-ninth entry's Next named it: Places was the last plainly English surface the
  ninety-sixth entry had listed — a Japanese owner's cull screen, Similar and Insights had all learned the catalogue
  while the screen that answers "where were these taken?" still painted English.
- **What is routed.** Nine keys in each of `ar.tsv` and `ja.tsv`: the empty state's title and its EXIF sentence,
  `zoom {zoom}`, the footer's `{located} photo(s) with a location · {dots} dot(s) · click a dot to select it`,
  `No places.txt — dots show their coordinates`, `Named from places.txt · {named} place(s) · dots within {km} km
  borrow a name`, the schematic-coastline note, the tile-folder instruction that carries the tile directory, and
  `The basemap cannot be used`.
- **The counts are templates, and so are the digits.** `located`, `dots`, `named`, the 30 km of the gazetteer line
  and the zoom level are all placeholders filled through `crate::i18n::fmt` with the shell's own `format_count`, so a
  Japanese footer reads `位置情報のある写真 2 枚 · 1 地点 · 地点をクリックして選択` and an Arabic one writes `٢ صورة بموقع
  · ١ نقطة`. The tile instruction keeps `{z}/{x}/{y}` as literal text (no argument of those names exists, so the
  filler leaves them alone) and puts only the directory in `{dir}`.
- **Named as not shipped.** The click notice (`places::selection_notice`, "Selected 2 photo(s) near Brighton…") is
  still English: it belongs to the notice class, not the view chrome, and the same is true of dozens of toasts. The
  basemap refusal's reason is `TileError::message()`, an engine sentence like the export spec and the job detail — it
  needs a piece of its own, not a lookup. The map's assistive name stays English, which the ROADMAP row does not count
  as painted chrome. All three are recorded in the ROADMAP row and UX.md rather than implied done.
- **Tests that fail when the behaviour is absent.** `the_places_view_speaks_the_interface_language` paints the real
  `places_view` headlessly twice: an empty library (the two empty-state lines and the basemap note in English,
  Japanese and Arabic) and a two-photograph geotagged library (the zoom label, both footer counts and the
  `places.txt` line in all three), stripping `fmt`'s isolates before comparing, so the Arabic frame proves its own
  digits. The catalogue audit reads all nine `tl!`/`fmt` literals and fails if either catalogue loses a row.
- **Files.** `crates/ui/src/shell.rs` (`places_view`, `basemap_status`, one test), `crates/ui/src/i18n/ar.tsv` and
  `ja.tsv` (nine rows each), `docs/ROADMAP.md` (the i18n row's prose), `docs/UX.md` (the Places section).
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the new test and the catalogue
  audit pass. `cargo xtask ci` green through every gate; `cargo xtask snapshot` (5 tests),
  `cargo xtask deny` (`advisories ok, bans ok, licenses ok, sources ok`) and `cargo xtask screenshots` (all 27
  committed scenes byte-identical to their baselines, which is the proof the English Places view did not move —
  English has no catalogue, so a `tl!` with no row is its own key). There is no `cargo xtask wasm` in this
  workspace (removed in the sixty-first entry when the wasm target went), so no wasm gate ran. No release
  dispatch: this slice touches `crates/ui` source, catalogues, tests and docs, no build or release surface.
- **Next.** The Detail view's chrome — `Nothing selected`, `Pick a photo in the grid, then press D.`, `Previous`/
  `Next`, `Lookahead`, the keyboard hint and the sidecar lines — then Compare, Soft Focus and For You, before the
  four catalogues (zh-TW/ko/th/vi) and the RTL screenshots.

## 2026-11-26 (ninety-ninth) — the Insights highlights name their months and seasons in your language

- **Why this came up.** The ninety-eighth entry's Next named it: the highlights strip above the
  figures and the `Busiest: … · mostly …` line were English composed inside `sortlens-insights`,
  so the month, the season, the labels and the `photo(s)` sentences were the last prose on the page
  that Settings → Language could not reach.
- **The piece, not the sentence.** `sortlens-insights` gained `Month` (twelve values, `number`,
  `from_number`, `en`), `Season` (four values, `months`, `en`), `HighlightKind` (the four things
  the strip can say, `en`) and `HighlightFact` (month + count + season, day + count, camera + count,
  undated + percent). `Highlight` is now `{ kind, fact }`; `label()`/`value()` are built from the
  pieces, so the engine's JSON and the screen word the same words by construction. `peak_season`
  now returns `Option<Season>`, `month_name` delegates to `Month::en`, and the engine re-exports
  `InsightMonth`/`InsightSeason`/`InsightWord`/`InsightFact` beside `InsightHighlight`. The MCP
  `insights.stats` payload keeps its `label`/`value` strings and its `peakSeason` is the English
  word `Season::en` — a shape change nobody's client has to notice.
- **The shell matches on the piece, never on the text.** `insight_highlight_word`,
  `insight_month_word`, `insight_season_word` and `insight_highlight_line` are exhaustive matches
  whose arms are `tl!`/`crate::i18n::fmt` literals; `highlights_strip` and `busiest_sentence` are
  small pure painters/worders, so the test drives the strip with synthetic facts instead of
  building a library. 27 new rows in each of `ar.tsv` and `ja.tsv`: the four labels, the twelve
  month names, the four seasons, and seven templates (month-with-season, month, day, camera,
  undated, `Busiest:` with and without a season).
- **The date follows the language too.** The busiest day used the engine's ISO `date_label`, which
  Arabic would have painted in Latin digits. `crates/ui/src/format.rs` gained `date`/`date_in`,
  and `date_time_in` now builds on `date_in` so a date and its date-time cannot disagree; Arabic
  reads `١٠/٠٢/٢٠٢٤`, Japanese `2024/02/10`.
- **Tests that fail when the behaviour is absent.** `every_month_and_season_answers_through_its_own_piece`
  in the insights crate: the twelve numbers round-trip through `from_number`, `month_name` equals
  `Month::en`, the seasons tile the year exactly once, and the four kind names are the strip's.
  `the_insights_highlights_speak_the_interface_language` in the shell: every English month, season
  and label equals the crate's own `en()` (retype a month in the shell and it fails), then a painted
  strip and a worded busiest line in English, Japanese and Arabic, with the English sentences
  asserted absent. `fmt` isolates values in right-to-left text, so the test reads the frame without
  the FSI/PDI marks; the Arabic undated row also proves the percent's digits are the reader's.
- **Named as not shipped.** The month chart's twelve axis ticks keep the English initials
  (`J F M A M J …`): a column is one character wide, so it is a tick rather than prose, and the
  localized month words live in the strip and the busiest line. Recorded in the ROADMAP row and
  UX.md rather than implied done.
- **Files.** `crates/insights/src/lib.rs` (the pieces, `highlights()`, two tests),
  `crates/engine/src/lib.rs` (re-exports, the JSON builder, one test), `crates/ui/src/format.rs`
  (`date`/`date_in`, one test), `crates/ui/src/shell.rs` (four worders, the strip, the busiest
  line, one test), `crates/ui/src/i18n/ar.tsv` and `ja.tsv` (27 rows each), `docs/ROADMAP.md`
  and `docs/UX.md` (the "still English" paragraph replaced with what shipped).
- **Verified.** `cargo clippy -p sortlens-insights -p sortlens-engine -p sortlens-ui --all-targets
  -- -D warnings` clean; the insights crate's 9 tests, the engine's insights tests and the UI's
  three insights tests pass; `cargo xtask ci`, `cargo xtask snapshot`, `cargo xtask deny`,
  `cargo xtask wasm` and `cargo xtask screenshots` each exit 0, the last painting all 27
  committed scenes against their baselines, which is the proof that the English strip and busiest
  line are byte-identical (English has no catalogue). No release
  dispatch: this slice touches crates, catalogues and docs, no build or release surface.
- **Next.** Places: `No photo has a location yet` and the `places.txt` line, the last plainly
  English surface the ninety-sixth entry named. Then a language sweep of the remaining views before
  the four catalogues (zh-TW/ko/th/vi) and the RTL screenshots.

## 2026-11-26 (ninety-eighth) — the Insights page counts, names and heads its columns in your language

- **Why this came up.** The ninety-seventh entry's Next named it: the Insights page was the next surface
  painting English whatever Settings → Language said, its column headings (`Ratings`, `Cameras`, `Timeline`)
  and its two empty states the plainest cases.
- **What is routed.** The `Recount` button and its hover sentence, the `Counting` progress label and the
  `{done} of {total}` template (an existing row, reused rather than duplicated), the seven figures
  (`Photos`, `On disk`, `Rated`, `Keyworded`, `Picks`, `Rejects`, `Missing` — five of them existing rows),
  the eight column headings and section names (`Ratings`, `Labels`, `Kinds`, `Cameras`, `Lenses`,
  `Keywords`, `Timeline`, `Months`), `Unrated`, the three kinds (`Images`, `Raw`, `Video`), and the two
  empty states `No capture dates yet` and `Nothing dated yet` — 19 new rows in each of `ar.tsv` and
  `ja.tsv`. The undated line is a `{n}` template filled with `format_count`, so Arabic writes its own digits.
- **The duplicate the catalogue refused.** `{done} of {total}` already had a row from the model-install
  progress, and the parser rejects a second row with the same source text ("defined twice … the first is
  kept"), so the audit test caught the copy before it could shadow a translation. The row is reused.
- **Tests that fail when the behaviour is absent.** `the_insights_screen_speaks_the_interface_language`
  paints the whole shell at 1200×800 on an empty library and reads the frame through `all_shapes`, because
  the empty month chart paints its word inside a layout shape: English first (all 22 labels, so a baseline that
  moved is named rather than inferred), then Japanese and Arabic with the English headings asserted absent.
  The catalogue audit now sees the 19 new literals and fails if either catalogue loses one.
- **Named as not shipped.** The highlights strip and the `Busiest: … · mostly …` line stay English: the month
  name, the season and the sentence itself are composed in `sortlens-insights`, so it needs the engine-piece
  treatment the similar modes got — not a shell literal. Recorded in the ROADMAP row and UX.md rather than
  implied done.
- **Files.** `crates/ui/src/shell.rs` (`insights_view`, one test), `crates/ui/src/i18n/ar.tsv` and `ja.tsv`
  (19 rows each), `docs/ROADMAP.md` (the i18n row's prose), `docs/UX.md` (the same reconciliation, including
  the stale "still English: the keymap editor" line — that editor was catalogued in the ninety-first entry).
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the new test and the catalogue
  audit pass; `cargo xtask ci` green — its first run stopped at `cargo fmt --check` on the new test's long
  Arabic assertion list, which `cargo fmt --all` fixed — and `cargo xtask snapshot`, `cargo xtask deny` and
  `cargo xtask screenshots` each exit 0, the last painting all 27 committed scenes against their baselines, which
  is the proof that the English page is unchanged (English has no catalogue, so a `tl!` with no row is its own
  key). No release dispatch: this slice touches `crates/ui` source, tests and docs, no build or release surface.
- **Next.** The insights highlights strip: give `sortlens-insights` pieces (a `Highlight` kind rather than an
  English label, a month and a season as values rather than words) and word it from those, so `Busiest month`,
  `January`, `Winter` and the `photo(s)` sentences read in the chosen language. Then Places (`No photo has a
  location yet`, the `places.txt` line), the last plainly English surface the ninety-sixth entry named.

## 2026-11-26 (ninety-seventh) — the similar view's mode picker names its modes from the similar crate's own pieces

- **Why this came up.** The ninety-sixth entry's Next named exactly this: `Structure`/`Colour`/`Either` and
  their three hint sentences were `sortlens_similar::Match::label`/`hint` — English built inside the crate —
  so the one English surface left on the Similar view was the one a person uses to *change* what it finds.
- **The piece, not the sentence.** `sortlens-similar` gained two enums, `MatchWord` (the picker's word) and
  `MatchSense` (what a mode lets through), each with `en()`; `Match::word()`/`sense()` are the mapping, and
  `label`/`hint` are now built from `word().en()`/`sense().en()` — one table, so the English the engine answers
  and the piece a screen translates are the same words by construction. The engine re-exports them as
  `SimilarWord`/`SimilarSense` beside `SimilarMatch`.
- **The shell matches on the piece, never on the text.** `similar_mode_word`/`similar_mode_hint` are exhaustive
  matches over those enums whose arms are `tl!` literals, used for the three buttons, their hovers and the line
  under the picker. `Colour` already had a row (the colour chip), so five keys are new — `Structure`, `Either`
  and the three hints — in each of `ar.tsv` and `ja.tsv`; the catalogue audit's duplicate check would have
  refused a second `Colour`.
- **Tests that fail when the behaviour is absent.** `every_mode_answers_through_its_own_piece` in the similar
  crate (label/hint equal their piece's English, the words and the senses are distinct); in the shell,
  `every_similar_mode_translates_the_engines_own_piece` compares the shell's English literal against
  `mode.label()`/`mode.hint()` for every mode — retype a sentence in the shell and it fails — and asserts the
  `ja`/`ar` rows really differ from English rather than falling back. `the_similarity_view_speaks_the_interface_language`
  gained the mode row: English paints `Structure`, `Colour`, `Either` and the default hint, Japanese paints
  `構図`/`どちらでも`/`同じ構図と同じ配色 — 安全な既定値です。` with no English mode word left, Arabic paints
  `البنية`/`أيّهما`.
- **The test's own shape.** Its assertions moved into two module helpers, `painted_texts` (the text a frame
  painted) and `assert_painted` (every expected string, naming the whole frame when one is missing), because
  the seventh assertion pushed the function past clippy's 30-point complexity budget — the same failure a
  growing language test will hit again, so the helper is the answer rather than an `allow`.
- **Files.** `crates/similar/src/lib.rs`, `crates/engine/src/lib.rs` (the re-export), `crates/ui/src/shell.rs`,
  `crates/ui/src/i18n/ar.tsv` and `ja.tsv` (five rows each), `docs/ROADMAP.md` (the i18n row's prose; the row
  stays unticked for the four missing languages and the RTL shots).
- **Verified.** `cargo clippy -p sortlens-similar -p sortlens-engine -p sortlens-ui --all-targets -- -D warnings`
  clean; the similar crate's 15 tests, the UI's three similarity tests and the catalogue audit pass. `cargo
  xtask ci` green through every gate — its first run stopped at `cargo fmt --check` on the new test's
  formatting, which `cargo fmt --all` fixed — and `cargo xtask snapshot`, `cargo xtask deny` and `cargo xtask
  screenshots` green, the last painting all 27 committed scenes byte-identical to their baselines, which is the
  proof that the English picker is unchanged (English has no catalogue, so a `tl!` with no row is its own key).
- **Next.** The Insights screen is the next surface (`Nothing has happened yet` and its column headers), then
  Places (`No photo has a location yet`, the `places.txt` line). Both are plain UI routing; no engine piece is
  needed for either.

In-flight: no release dispatch (this slice touches `crates/*` source, tests and docs — no build or release
surface). CI for this commit is the next tick's first read, one `gh run view`.

## 2026-11-26 (ninety-sixth) — the similarity view's chrome speaks the interface language, and its counts are templates

- **Why this came up.** The ninety-fifth entry's Next named it: after the cull screen, the Similar view still
  painted `Find similar photos`, `Match`, its empty state and every select-all hover in English, so an owner who
  chose Japanese judged photographs in Japanese and found similar ones in English.
- **What was routed.** 27 strings go through the catalogues; 23 needed new rows in each of `ar.tsv` and
  `ja.tsv`, because `Cancel`, `Clear`, `{n} photos` and `{done} of {total}` already had rows and are reused
  (the duplicate check caught the first attempt at the last one, which is the audit doing its job): the empty
  state's two lines, `Find similar photos`, `Hashing…`, the progress line, `Match`, the strictness and palette
  sliders with the palette's hover, the leftover and delete buttons with their hovers, `Clear`, the
  nothing-selected line, the group header (`{n} photos`, `spread {bits} bits`), `Select all but the best` and
  its hover, the three notices, `keeping {keep}`, and the thumbnail's `best` tag.
- **Counts are templates, not concatenations.** `{groups} groups · {bits} identical bits`, `Selected {n}
  leftover(s) across {groups} group(s)`, `{n} group(s) · nothing selected`, `{n} set aside` and `keeping
  {keep}` are filled through `i18n::fmt` with `format_count`, so a Japanese group reads `写真 3 枚` with
  `各グループの余りを選択 (2)`, and Arabic writes `٣ صور`.
- **Left English on purpose, and named.** The mode buttons and their hint lines are `sortlens_similar`'s own
  `Match::label`/`Match::hint` — engine sentences, the same class the export spec and the job detail are
  routed by piece rather than by lookup. Routing them needs an engine piece (a `Match` word enum) and is the
  next i18n slice, not this one. Nothing else on the view paints an English literal.
- **Tests.** `the_similarity_view_speaks_the_interface_language` paints the real `similar_view` headlessly at
  1100×780 over four scanned photographs: English first (the empty state's two lines, then — after the three
  copies are hashed — `Match`, `3 photos`, `Select all but the best`, `Select leftovers in every group (2)`),
  then Japanese (`似た写真を探す`, `一致基準`, `最良以外をすべて選択`, `各グループの余りを選択 (2)`, `写真 3
  枚`, and none of the English chrome), then Arabic (`ابحث عن صور مشابهة`, `المطابقة`, and the group count with
  `٣`). It fails the moment a route goes back to a bare literal.
- **Files.** `crates/ui/src/shell.rs` (`similar_view`, one test); `crates/ui/src/i18n/ar.tsv` and `ja.tsv` (23
  rows each); `docs/ROADMAP.md` — the i18n row's prose (the row stays unticked: four languages and RTL shots
  remain) and the `craft-ipc` row, whose stated reason ("there is no second process to talk to") the shipped
  `sortlens-server` made false; it now records the process the host actually spawns, the loopback HTTP plus
  capability token that is its channel, and the socket consumer that would still earn the crate a place.
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the new test and the catalogue
  audit pass, and the cull screen's language test still passes (this signature's neighbour). `cargo xtask ci`,
  `snapshot` and `deny` green, and `cargo xtask screenshots` painted all 27 committed scenes byte-identical to
  their baselines — which is the proof every English string is unchanged, since a `tl!` with no row is its own
  English key. No release workflow was dispatched: `crates/ui` source, tests and docs, no build or release
  surface.
- **Next.** The mode buttons and hints (`Structure`/`Colour`/`Either` and their three sentences) are the last
  English chrome on Similar, and they need a `Match`-side piece first. Then the Insights screen (`Nothing has
  happened yet`, its headers) and Places (`No photo has a location yet`, the `places.txt` line).

In-flight: no release dispatch (UI source, tests and docs only). CI for this commit is the next tick's first
read, one `gh run view`.

## 2026-11-26 (ninety-fifth) — the cull screen judges photographs in your language, and its buttons keep the keyboard's letters

- **Why this came up.** The ninety-fourth entry's item was the trunk (run **45586062477** on `ff169a7` is now green on
  all eight jobs, `screenshots` and both `mobile` legs among them), and after that it named the `craft-ipc` row's
  reconciliation — which is prose, and the tick rules forbid a docs-only commit after Phase 0. The ninety-second
  entry had called the i18n row finished apart from four languages and RTL shots. Reading `cull_view` said
  otherwise: the screen an owner judges three hundred photographs on painted `Cull`, `Summary`,
  `End session`, `Up next`, `Session complete`, `Pick`/`Reject`/`Defer`/`Clear`, `Apply rejects…`,
  `Move picks…`, `Export picks…`, `Back`, the position line, the session's counts, the empty state, the keyboard
  hint, the keyword field's hint and both keyword toasts in English whatever Settings → Language said. The cull
  *summary sheet* had been routed; the cull *screen* had not, so a Japanese cull session was an English session
  with Japanese chrome around it.
- **What was routed.** 30 rows in each of `ar.tsv` and `ja.tsv` (31 keys, one of them — `Clear` — already had a
  row, and the catalogue's duplicate check caught the second one at line 589, which is the audit doing its job):
  the empty state's sentence, its start button and the notice it raises; the header's title, position, `Summary`,
  `End session` and its hover; the four decision buttons; the anti-accident hover on `Apply rejects…`; the three
  apply/move/export actions and their hovers; `Up next` and `Session complete`; the keyword chip's remove hover,
  the field's hint, `Add`, the "already on this photo" notice and both tagging toasts.
- **The counts are templates, and the position is not English's word order.** `{position} of {total}` becomes
  `{total} 中 {position}` in Japanese and `{position} من {total}` in Arabic, and the digits are `format_count`'s, so
  the Arabic screen writes `١ من ٣` with Arabic-Indic digits. The session's line is
  `{picked} picked · {rejected} rejected · {deferred} deferred`, filled from the session's own counts.
- **The keys stay the keyboard's.** `Pick  P` becomes `採用  P`: the label travels, the letter does not, because
  `P` is the key you press. The keyboard-hint line is one row, so a language can reword the whole sequence.
- **Tests.** `the_cull_screen_speaks_the_interface_language` paints the real `cull_view` headlessly at 1000×700
  with three photographs: English first (the empty state's sentence and start button, then the running session's
  `Summary`, `End session`, `Up next`, `1 of 3` and `Pick  P`), then Japanese (`選別`, `概要`, `セッション終了`,
  `次の候補`, `3 中 1`, `採用  P`, and no English chrome left), then Arabic (`ملخّص`, and the position as `١ من ٣`).
  The catalogue audit `catalogs_load_and_routed_keys_have_rows` now demands an ar and a ja row for all 30 new
  keys and templates, so a route without a translation fails instead of quietly showing English.
- **Files.** `crates/ui/src/shell.rs` (`cull_view`, one test); `crates/ui/src/i18n/ar.tsv` and `ja.tsv` (30 rows
  each); `docs/ROADMAP.md` (the i18n row's prose — the row stays unticked: four languages and RTL shots remain).
- **Verified.** `cargo fmt --all` was the only thing the first `cargo xtask ci` found (the routing's formatting);
  after it `cargo xtask ci` is green, `cargo xtask snapshot` and `cargo xtask deny` green, and
  `cargo xtask screenshots` painted all 27 committed scenes against their baselines — byte-identical, which is the
  proof that every English string is unchanged: a `tl!` with no row is its own English key. No release workflow was
  dispatched: this slice touches `crates/ui` source, tests and docs, no build or release surface.
- **Next.** Still painted English on the main views: the Insights screen (`Nothing has happened yet`, its headers),
  the Similar view (`Match`, `Clear`, `No groups yet`, `Hash the library to find bursts and re-edits.`, the
  select-all hovers) and Places (`No photo has a location yet`, the `places.txt` line). The Similar view is the
  next slice — its chrome, one test in the same shape as this one. The `craft-ipc` row's reconciliation rides in
  that commit rather than its own: its sentence ("there is no second process to talk to") was made false by
  `sortlens-server`, and it must not be a docs-only commit.

In-flight: no release dispatch (no build or release surface changed). CI for this commit is the next tick's first
read, one `gh run view`.

## 2026-11-26 (ninety-fourth) — the two Places baselines were the old footer, and the ninety-second entry was missing its body

- **Why this came up.** `CI` on `main` was red at `b43ddfc` on the `screenshots` job only; `ci` (ubuntu, macos), `deny`,
  `snapshot` and both `mobile` legs were green, and Windows' leg was still running when this tick read it. The
  ninety-third entry's Next named the fault exactly: the MBTiles commit regenerated the reference set but not the
  `lavapipe` set, so `places-charcoal` and `places-paper` were still the pre-MBTiles footer.
- **The frames came from the run that painted them, not from a local render.** CI's `screenshot-set` artifact for run
  **45584865662** carries `lavapipe/places-charcoal.new.png` and `places-paper.new.png`, painted by the same Mesa
  adapter the comparison uses. Both were opened: the only change is the footer's last block, which now ends
  "Put {z}/{x}/{y} png tiles and an attribution.txt in tests/screenshots/Library/tiles — or a tiles.mbtiles archive
  with an `attribution` metadata row beside it — to replace the 1:110m outline. Nothing is downloaded." Every other
  pixel — the coastline, the three map dots, the counts, the bottom bar — is identical to the committed frame, so
  these are the MBTiles prose arriving in the baseline rather than a rendering drift being papered over.
- **Why not `cargo xtask screenshots` locally.** A run on this machine compares against `tests/snapshots/<name>.png`
  (the default set), not `lavapipe/`, because the set is selected by the adapter that painted it. The local gate
  cannot see this fault; the CI job is the only verifier for a `lavapipe` frame, which is why the change is two
  committed PNGs and a checkpoint line.
- **The ninety-second entry lost its body in `b43ddfc`.** That commit's edit replaced the MBTiles heading plus its
  whole body with the ninety-third heading, leaving an orphan heading above the new entry. The body is restored
  byte for byte from `6a810cd:CHECKPOINT.md` (verified identical by comparing the first 41 lines against that blob);
  the only surviving trace had been the ninety-third entry's own reference to what it said.
- **Files.** `crates/ui/tests/snapshots/lavapipe/places-charcoal.png`,
  `crates/ui/tests/snapshots/lavapipe/places-paper.png` (the two `.new.png` frames), `CHECKPOINT.md` (restored
  ninety-second body, this entry). No source, test or manifest changed.
- **Verified.** Both frames inspected at full size in both themes; the run they come from is green on `ci`, `deny`,
  `snapshot` and `mobile` and red only on `screenshots`. No local `cargo xtask ci` was spent: the diff is two PNGs
  and prose, so no Rust gate can move, and the previous commit's run already proves the tree compiles everywhere.
- **Next.** The pushed commit's `screenshots` job is the thing to read (`gh run list --workflow CI --limit 1`,
  then one `gh run view <id> --json status,conclusion,jobs`) to confirm it flips green with `mobile` still green.
  Then the loop is back on the plan: the `craft-ipc` row's reconciliation in `docs/ROADMAP.md` (it still says
  "there is no second process to talk to", which `sortlens-server` made false), then Phase 5's
  `docs/AUTONOMOUS_AGENT.md` playbook.

In-flight: no release dispatch (PNG baselines and docs only — no build or release surface). The CI run for this
commit is the first thing the next tick checks.

## 2026-11-26 (ninety-second) — an MBTiles basemap opens read-only, and its credit is not optional

- **Why this came up.** The ninety-first entry's Next said the i18n row had no buildable surface left and to take the
  next unstarted roadmap row instead. The unchecked rows were benchmarks part 2 (hardware), optional MBTiles (needs a
  SQLite reader) and mobile 3b (external toolchains). MBTiles is the one that needed code, not a machine or a
  translation pass, so this tick paid its stated price: a read-only SQLite reader, and nothing else.
- **The reader is `rusqlite` with `bundled`, MIT, one direct dependency added to `sortlens-geo`.** The roadmap deferred
  MBTiles because a SQLite reader felt large for one format; a hand-written file parser or a shell-out to `sqlite3`
  would each be worse than a maintained crate that compiles SQLite into the binary and assumes no system library.
  `docs/ATTRIBUTION.md` names it and `deny.toml`'s existing MIT entry already allows it.
- **`sortlens_geo::mbtiles::Mbtiles`** opens the archive `SQLITE_OPEN_READ_ONLY`, proves the header with
  `SELECT count(*) FROM sqlite_master` before anything else, and reads the credit from `metadata.attribution` or, when
  the archive has none, an `attribution.txt` beside it. The header proof matters: SQLite opens lazily, so without it a
  text file renamed to `.mbtiles` failed at the attribution query and was reported as a missing credit instead of as
  the wrong file.
- **`xyz_to_tms` is the whole correctness of the row.** MBTiles counts `tile_row` from the south; every other reader in
  this crate counts from the north. `2^z - 1 - y` is applied in one function and pinned by a test that stores row 13
  at zoom 4 and reads it back through the view's own y = 2; a row past the world is refused rather than wrapping.
- **`TileSet` is the basemap as the view sees it**: `Folder(Tileset)` or `Archive(Mbtiles)`, with the folder winning
  when both are present. It answers in bytes, so `places::TileCache` now keys on `(zoom, x, y)` and fetches through
  `TileSet::tile`, clearing when the source path changes — one library's tiles cannot be painted over another's.
  The cell needs `Hash`, which `TileCell` now derives.
- **Engine**: `tileset()` opens the catalog folder when `tiles/` or `tiles.mbtiles` exists, and returns `None` only
  when neither does; `mbtiles_path()` says where the archive would go. `tiles_dir()` still names the folder for the
  footer's instructions, which now mention both shapes.
- **Tests.** Four geo tests: the TMS flip and zoom range, a missing credit naming both the `metadata` row and the file
  beside it (then a blank file refused), a non-database and an empty archive each refused with their reason, and the
  row conversion's edges. One engine test builds a real archive with `rusqlite`, reads it through `engine.tileset()`,
  then proves a `tiles/` folder beside it wins and an archive with no credit is refused. The Places cache test now
  builds a `tiles/` and asserts one decode per cell, a remembered corrupt tile and a remembered hole; the shell's
  `places_reports_the_basemap_or_why_there_is_none` still paints every visible cell through the new byte path.
- **Files**: `crates/geo/src/mbtiles.rs` (new), `crates/geo/src/tiles.rs`, `crates/geo/src/lib.rs`,
  `crates/engine/src/lib.rs`, `crates/ui/src/places.rs`, `crates/ui/src/shell.rs`, `Cargo.toml`,
  `crates/geo/Cargo.toml`, `crates/engine/Cargo.toml`, `Cargo.lock`, `docs/ROADMAP.md` (SL-E06 row and the part 3a
  decision prose), `docs/UX.md` §5.9, `docs/ARCHITECTURE.md` §13, `docs/RELEASE.md`, `docs/ATTRIBUTION.md`, this entry.
- **Verified locally**: `cargo test -p sortlens-geo` (31), `cargo test -p sortlens-engine basemap` (1) and
  `cargo test -p sortlens-ui tile` (3) pass, and `cargo clippy -p sortlens-geo --all-targets -- -D warnings` is
  clean. `cargo xtask ci` stopped once, at clippy, on a nested `if` in `archive_zooms`; the fix and the `rusqlite`
  0.41 bump (0.30.1's bundled SQLite is CVE-2026-119375) came after it, so `snapshot`, `deny` and `screenshots` were
  re-run on the fixed tree and all pass (27 scenes). The full ci was not re-run in this tick.
- **Next**: the i18n row's remaining work is still a translation pass for zh-TW/ko/th/vi and an Arabic-capable font
  build for RTL screenshots, neither of which a tick can honestly invent; benchmarks part 2 needs hardware and mobile
  3b needs the Android NDK and Xcode. If none is available next tick, the honest slice is Phase 5 maintenance:
  read CI for this commit first and fix whatever it names, then the `craft-ipc` row's reconciliation (it says
  "there is no second process to talk to", which stopped being true when `sortlens-server` shipped).

In-flight: release run **45582509665** was dispatched for `86f8766` (this slice touches `Cargo.toml`, a packaging
surface). Its result and the push's CI run are the first things the next tick checks.

## 2026-11-26 (ninety-third) — a hole in the basemap is not a corrupted tile, and that is why the trunk was red

- **Why this came up.** `CI` on `main` was red at `6a810cd` on all three `ci` legs and on `screenshots`. The three
  `ci` legs failed on one test, `shell::tests::places_reports_the_basemap_or_why_there_is_none`, at
  `assert_eq!(state.places_tiles.failures(), 0, "every fixture tile was a real PNG")` with 980 failures — and it was
  red on this machine too, so the trunk had been failing since the MBTiles commit, whose entry said `deny`, `snapshot`
  and `screenshots` pass and never claimed `ci` did.
- **The bug is one word in the byte path.** When `TileCache` moved from `TileSet::tile_path` to `TileSet::tile`,
  `texture()` began counting every empty answer as a decode failure: `set.tile(...).and_then(load)` returns `None` both
  for "no such tile" and for "bytes that will not decode", and the counter took both. Before the move a missing tile
  short-circuited in the caller (`and_then(|path| ...)`) and was never counted at all, which is why the assertion passed
  then. A hole is a normal basemap — a partial tile set, or a viewport wider than its coverage; a failure is a corrupt
  file. `texture()` now counts a failure only when bytes were present and would not decode, and a hole is remembered as
  an entry (so it is not re-fetched every frame) while being neither a `decodes()` nor a `failures()`.
- **The fixture was also making a claim it could not keep.** It wrote the cells one 720×480 viewport asks for, then
  painted into `RawInput::default()`, whose screen is ten thousand pixels on a side: at zoom 5 that asks for the whole
  world (1024 cells), so 980 "failures" were tiles the fixture never wrote. The test now paints a fixed 1920×1080
  window and writes the whole zoom-5 world — 32×32 cells of `sortlens_icon::png_at(16)` — so every visible cell is a
  real PNG and the assertion is about decoding rather than about arithmetic the test cannot see. "One decode per
  visible tile" is now `decodes() == len()`, which is one decode per entry precisely because every visible cell is
  carried.
- **Tests.** `a_tile_is_decoded_once_and_a_bad_one_is_remembered_as_bad` grew two assertions: after asking for a cell
  the folder does not carry, `failures()` stays 1 and `decodes()` stays 2, so the old counting fails that test as well
  as the paint test. The paint test's failure message now prints the held and decoded counts — that is how the
  980-against-12 shape was read.
- **Files.** `crates/ui/src/places.rs` (the counting split, the `decodes`/`failures` docs, two assertions);
  `crates/ui/src/shell.rs` (the fixed window, the whole-world fixture, the stronger assertions); `docs/ROADMAP.md`
  (the part 3b `TileCache` sentence now says what each kind of absence means); this entry.
- **Verified.** `cargo test -p sortlens-ui --lib places` passes (15), and `cargo xtask ci`, `snapshot`, `deny` and
  `screenshots` all ran green on this tree, the ci exit code read from a file rather than a pipeline's. No baseline
  moved: the change alters what is counted, not what is painted — a hole was already painted as background.
- **Next.** `screenshots` is still red on `main` and is a different fault: the `lavapipe` set is committed and 47
  frames match, but `lavapipe/places-charcoal` and `lavapipe/places-paper` are the **old** footer (diff 1748 and 1729
  px) because the MBTiles commit regenerated only the reference set. Take run `45582577369`'s `screenshot-set`
  artifact (`gh run download 45582577369 -n screenshot-set`), read the two `places-*.new.png` frames and commit them
  under `crates/ui/tests/snapshots/lavapipe/`. `mobile` is still the NDK-clang job named in the seventy-fifth entry.
  No release workflow was dispatched: this slice touches `crates/ui` source, tests and docs only.

- **Why this came up.** The ninety-first entry's Next said the i18n row had no buildable surface left and to take the
  next unstarted roadmap row instead. The unchecked rows were benchmarks part 2 (hardware), optional MBTiles (needs a
  SQLite reader) and mobile 3b (external toolchains). MBTiles is the one that needed code, not a machine or a
  translation pass, so this tick paid its stated price: a read-only SQLite reader, and nothing else.
- **The reader is `rusqlite` with `bundled`, MIT, one direct dependency added to `sortlens-geo`.** The roadmap deferred
  MBTiles because a SQLite reader felt large for one format; a hand-written file parser or a shell-out to `sqlite3`
  would each be worse than a maintained crate that compiles SQLite into the binary and assumes no system library.
  `docs/ATTRIBUTION.md` names it and `deny.toml`'s existing MIT entry already allows it.
- **`sortlens_geo::mbtiles::Mbtiles`** opens the archive `SQLITE_OPEN_READ_ONLY`, proves the header with
  `SELECT count(*) FROM sqlite_master` before anything else, and reads the credit from `metadata.attribution` or, when
  the archive has none, an `attribution.txt` beside it. The header proof matters: SQLite opens lazily, so without it a
  text file renamed to `.mbtiles` failed at the attribution query and was reported as a missing credit instead of as
  the wrong file.
- **`xyz_to_tms` is the whole correctness of the row.** MBTiles counts `tile_row` from the south; every other reader in
  this crate counts from the north. `2^z - 1 - y` is applied in one function and pinned by a test that stores row 13
  at zoom 4 and reads it back through the view's own y = 2; a row past the world is refused rather than wrapping.
- **`TileSet` is the basemap as the view sees it**: `Folder(Tileset)` or `Archive(Mbtiles)`, with the folder winning
  when both are present. It answers in bytes, so `places::TileCache` now keys on `(zoom, x, y)` and fetches through
  `TileSet::tile`, clearing when the source path changes — one library's tiles cannot be painted over another's.
  The cell needs `Hash`, which `TileCell` now derives.
- **Engine**: `tileset()` opens the catalog folder when `tiles/` or `tiles.mbtiles` exists, and returns `None` only
  when neither does; `mbtiles_path()` says where the archive would go. `tiles_dir()` still names the folder for the
  footer's instructions, which now mention both shapes.
- **Tests.** Four geo tests: the TMS flip and zoom range, a missing credit naming both the `metadata` row and the file
  beside it (then a blank file refused), a non-database and an empty archive each refused with their reason, and the
  row conversion's edges. One engine test builds a real archive with `rusqlite`, reads it through `engine.tileset()`,
  then proves a `tiles/` folder beside it wins and an archive with no credit is refused. The Places cache test now
  builds a `tiles/` and asserts one decode per cell, a remembered corrupt tile and a remembered hole; the shell's
  `places_reports_the_basemap_or_why_there_is_none` still paints every visible cell through the new byte path.
- **Files**: `crates/geo/src/mbtiles.rs` (new), `crates/geo/src/tiles.rs`, `crates/geo/src/lib.rs`,
  `crates/engine/src/lib.rs`, `crates/ui/src/places.rs`, `crates/ui/src/shell.rs`, `Cargo.toml`,
  `crates/geo/Cargo.toml`, `crates/engine/Cargo.toml`, `Cargo.lock`, `docs/ROADMAP.md` (SL-E06 row and the part 3a
  decision prose), `docs/UX.md` §5.9, `docs/ARCHITECTURE.md` §13, `docs/RELEASE.md`, `docs/ATTRIBUTION.md`, this entry.
- **Verified locally**: `cargo test -p sortlens-geo` (31), `cargo test -p sortlens-engine basemap` (1) and
  `cargo test -p sortlens-ui tile` (3) pass, and `cargo clippy -p sortlens-geo --all-targets -- -D warnings` is
  clean. `cargo xtask ci` stopped once, at clippy, on a nested `if` in `archive_zooms`; the fix and the `rusqlite`
  0.41 bump (0.30.1's bundled SQLite is CVE-2026-119375) came after it, so `snapshot`, `deny` and `screenshots` were
  re-run on the fixed tree and all pass (27 scenes). The full ci was not re-run in this tick.
- **Next**: the i18n row's remaining work is still a translation pass for zh-TW/ko/th/vi and an Arabic-capable font
  build for RTL screenshots, neither of which a tick can honestly invent; benchmarks part 2 needs hardware and mobile
  3b needs the Android NDK and Xcode. If none is available next tick, the honest slice is Phase 5 maintenance:
  read CI for this commit first and fix whatever it names, then the `craft-ipc` row's reconciliation (it says
  "there is no second process to talk to", which stopped being true when `sortlens-server` shipped).

In-flight: release run **45582509665** was dispatched for `86f8766` (this slice touches `Cargo.toml`, a packaging
surface). Its result and the push's CI run are the first things the next tick checks.

## 2026-11-26 (ninety-first) — the shortcut editor speaks the interface language, because the crate's own string list became the audit

- **Why this came up.** The ninetieth entry's "Next" said `craft-keymap`'s editor chrome "lives in
  the dependency and stays English until it offers its own keys". It already offers them: the pinned
  rev exposes `ui::Text` (`ui`, `plural`, `command`, `segment`, `rtl`), `ui::STRINGS` (the 175
  strings the dialog looks up) and `ui::PLURALS` (11 counts), and `ShortcutsDialog::new` takes
  `&dyn Text`. Sortlens passed `&English`.
- **`i18n::CatalogText`** implements that trait over the same TSV catalogues: `ui`/`command`/`segment`
  look the English key up at paint time, `plural` goes through `I18N.trn` so Arabic reads its
  `@plural` forms, and `rtl` follows the chosen language. The dialog's title, its captions and its
  format labels already pass through `Text::ui`, so the whole chrome follows without a second
  options path.
- **Fixed rows are translated before the handoff.** `craft-keymap` paints a `FixedRow`'s group and
  label raw, and those are the overlay's own context heading and action text;
  `localized_fixed_row` looks both up as the options are built.
- **Notices are routed too**: "Shortcuts saved", "Imported shortcut set “{name}”", "Could not import
  the set: {error}" and the accept guard's double-press message are `tl!`/`fmt` keys.
- **Catalogues grew by 183 rows each.** 167 of the 175 crate strings were missing (8 were already
  routed by the overlay), all 11 `@plural` rows were missing, and this slice's five shell keys are
  new; `ja.tsv` and `ar.tsv` now hold 572 lines each. Arabic's plural rows carry all six forms for
  each pair, and every form keeps the pair's `{n}`. Presence is what the audit checks, not
  difference: `Control`, `Shift` and templates like `{chord} ({scope})` translate to themselves on
  purpose.
- **Tests.** `every_keymap_editor_string_is_in_both_catalogues` walks `STRINGS` and `PLURALS` and
  fails naming the language and the string when a row is absent — it first compared `I18N.t(...)`
  with the source, which reported every intentional identity as missing, so it now asks
  `I18N.get` for the row. `the_shortcut_editor_speaks_the_interface_language` opens the editor with
  Japanese chosen and asserts the title, the `Command` heading and `Import…` paint Japanese while
  the English button is gone from the frame. The source audit caught the new `fmt` key written with
  `\u{201c}` escapes — it reads source text, so the call now carries the curly quotes directly.
- **Files**: `crates/ui/src/i18n.rs`, `crates/ui/src/shell.rs`, `crates/ui/src/i18n/{ja,ar}.tsv`,
  `docs/ROADMAP.md` (SL-L07), `docs/ARCHITECTURE.md` §8.4, `docs/FEATURES.md` (SL-L07), this entry.
- **Verified locally**: `cargo test -p sortlens-ui i18n::` (3 pass),
  `cargo test -p sortlens-ui shortcut_editor` (2 pass, the paint test included), and the full gates —
  `cargo xtask ci`, `cargo xtask snapshot`, `cargo xtask deny`, `cargo xtask screenshots` — all green.
  The baselines are byte-for-byte unchanged: no scene paints a non-English editor, and
  `shortcut_editor_paints_in_both_themes` still passes.
- **Next**: the i18n row has no buildable surface left inside the shell. What remains is the four
  other languages (a translation pass) and the RTL screenshot baselines (a font build with an Arabic
  face). If neither is available next tick, take the next unstarted row instead of re-opening i18n:
  the roadmap's remaining unchecked rows are benchmarks part 2 (hardware), optional MBTiles (needs a
  SQLite reader) and mobile 3b (external toolchains).

In-flight: no release dispatch (only `crates/ui` and docs, no release surface). The push's CI run is
the first thing the next tick checks.

## 2026-11-26 (ninetieth) — the Suggestions panel names its sources and confidences in your language, from the AI crate's own piece

- **Why this came up.** The eighty-ninth entry's "Next" named Suggested keywords as the last routed
  surface on the row: the right-panel tab, the inspector's `Suggested` row and the whole-library
  dialog painted `detector`/`camera`/`pack`/`caption`, the family headings and every count as English
  literals under already-translated headers, so a Japanese screen could not read a proposal.
- **`sortlens-ai` hands back the line's pieces.** `Suggestion::line_kind` returns a `SuggestionLine`
  (the keyword, the `Source` and the confidence as a percentage), and `Suggestion::line` — the English
  the panel and the report print — is exactly that piece's `english()`, pinned for all four sources by
  `the_line_is_joined_from_its_piece`, so a translated row cannot name a source or score the English
  sentence does not.
- **The sources and families are looked up, not copied.** `source_word` and `family_word` read
  `Source::label` and `SuggestKind::label` as catalogue keys at paint time (the keymap table's rule):
  the four source words and `Objects`/`Scene`/`EXIF` now have Japanese and Arabic rows, and
  `every_source_word_and_family_heading_is_the_catalogues` reads every one out of both catalogues so a
  source the pipeline adds later cannot arrive with a word only English has.
- **The surfaces are routed end to end.** The suggestions tab's hint, empty state, explanation,
  `{proposals} for {photos}` count (each count worded by its own `{n}` template so English keeps its
  plural), the family headers, `Accept all` and its hover, every row's percent/source/partial count,
  the accept and dismiss toasts and hovers; the inspector's `Suggested` label and hover; and the
  whole-library dialog's title, progress line, `Stop`, `Suggest for the whole library`, `Clear`,
  explanation, empty state, `{n} photo(s) with something to suggest` line and per-row hover. 32 new
  rows in each catalogue (`Stop` and `{done} of {total}` already existed). Confidences go through
  `format_count`, so Arabic writes `٨٧%` where English writes `87%`.
- **Tests.** `a_proposals_percentage_and_provenance_speak_the_interface_language` pins the hover and the
  digits in all three languages; `the_suggestions_tab_speaks_the_interface_language` paints the real
  right-panel tab at 360×600 in English, Japanese and Arabic and fails if the English hint is left
  behind; the catalogue audit demanded rows for the new `tl!`/`fmt` keys and caught the three Arabic
  rows this slice first forgot.
- **Files.** `crates/ai/src/suggest.rs` (`SuggestionLine`, `line_kind`, one test),
  `crates/ui/src/shell.rs` (`family_word`, `source_word`, `suggestion_percent`, `suggestion_hover`,
  the tab, the inspector row and the dialog, two tests), `crates/ui/src/i18n/ja.tsv` and `ar.tsv`
  (32 rows each), `docs/ROADMAP.md` (Phase 4 item 2d), `docs/ARCHITECTURE.md` (8.4), `docs/UX.md` and
  `docs/FEATURES.md` (SL-L07).
- **Verified.** `cargo fmt --all`; `cargo xtask ci` green (the first run failed on `fmt --check` and was
  re-run after formatting); `snapshot`, `deny` and `screenshots` green (27 scenes against the committed
  baselines, none moved — no scene paints a non-English suggestions panel). No release workflow
  dispatched: this slice touches no workflow, `xtask`, `Cargo.toml` or release code.
- **Next.** The last open surface on the i18n row is `craft-keymap`'s editor chrome, which lives in the
  dependency and stays English until it offers its own keys; the remaining four languages and the RTL
  screenshots still wait for a translation pass and an Arabic-capable font build.

## 2026-11-26 (eighty-ninth) — the Activity tab's job rows are the engine's pieces, in the reader's digits

- **Why this came up.** The eighty-eighth entry's "Next" named the sentences the engine composes for
  the activity row's job lines: `Job::detail` built "12 files · 3 folders · 2 unreadable" inside
  `sortlens-engine`, and `JobKind::label`/`JobStatus::label` painted "Scan"/"done" as English
  words under a panel header that had already been translated, so no row of the tab could be.
- **The engine hands back a piece.** `Job::detail_kind` returns a `JobDetail` — `Progress`,
  `Counts { files, dirs }`, `Finished { files, dirs, unreadable }`, `Cancelled`, `Failed` —
  and `Job::detail`, the line the report and the JSON print, is exactly `detail_kind().english()`,
  pinned byte for byte for every shape by `the_detail_english_is_joined_from_its_piece`.
- **The tab is routed.** The shell's `job_detail` words the piece through `i18n::fmt`: the two
  counters (and the unreadable count) are placeholders filled by `format_count`, so Arabic writes
  Arabic-Indic digits and Japanese the same digits in its own sentence. The kind and the state are
  looked up by their English text (`JobKind::label`, `JobStatus::label` stay the source of truth,
  as the keymap's rows do), the empty hint `Nothing has run yet. Scans and exports appear here.` and
  the `Indexed`/`On disk`/`Missing` rows are `tl!` keys, and a failed job's error stays the
  platform's own words. 18 new rows in each catalogue.
- **Tests.** `the_detail_english_is_joined_from_its_piece` (engine) pins running, finished with and
  without an unreadable file, a progress line, cancelled and failed with and without an error;
  `the_activity_row_speaks_the_interface_language` opens a real scanned library and paints the real
  Activity tab — English (`Scan`, `done`, `1 files · 1 folders`), Japanese (`スキャン`,
  `完了`, `1 ファイル · 1 フォルダー`, and no English word behind any of them) and Arabic
  (`فحص`, `منتهٍ`, the detail in Arabic-Indic digits). The catalogue audit demanded both
  catalogues' 18 rows and passed.
- **Files.** `crates/engine/src/job.rs` (the `JobDetail` piece, `detail_kind`, one test),
  `crates/engine/src/lib.rs` (the re-export), `crates/ui/src/shell.rs` (`job_detail`, the routed
  row, one paint test), `crates/ui/src/i18n/ja.tsv` and `ar.tsv` (18 rows each),
  `docs/ROADMAP.md` (Phase 4 item 2d), `docs/ARCHITECTURE.md` (8.4), `docs/UX.md` and
  `docs/FEATURES.md` (SL-L07).
- **Verified.** `cargo fmt --all`; `cargo clippy -p sortlens-engine -p sortlens-ui --all-targets
  -- -D warnings` clean (exit 0); the engine's five job tests pass and the UI's activity-row and
  catalogue-audit tests pass; `cargo xtask ci`, `snapshot`, `deny`, `wasm` and `screenshots`
  green (27 scenes against the committed baselines, none moved — no scene paints the Activity tab
  in a language other than English). No release workflow dispatched: this slice touches no workflow,
  `xtask`, `Cargo.toml` or release code.
- **Next.** Suggested keywords is the last routed surface on this row: it needs `sortlens-ai`'s
  own suggestion wording (the AI suggestion card's `detail()` is engine-composed English) and a
  piece per proposal shape before its panel can be translated; the `craft-keymap` editor's chrome
  stays English in the dependency until it offers keys.

## 2026-11-26 (eighty-eighth) — the shortcut overlay speaks the interface language, and a new binding cannot arrive untranslated

- **Why this came up.** The eighty-seventh entry's "Next" named Keyboard shortcuts: the overlay's
  title, search hint, count lines and every row of `crates/ui/src/keymap.rs`'s table were English
  literals, so a Japanese screen opened a catalogued dialog and found an English table inside it.
- **The table is looked up, not copied.** `Binding::localized_action`, `Context::localized_label`
  and `Context::localized_note` read their English text as the catalogue key at paint time, and
  `Binding::matches` searches the localized words as well as the English ones, so the search field
  works in the interface language. The English stays the source of truth — the editor's catalog and
  the fixed rows keep it, because `craft-keymap`'s own dialog is English — so the overlay cannot
  drift from the table it documents.
- **The overlay is routed end to end.** The title, `Search keys or actions`, `{n} bindings`,
  `{bound} of {total} bound` and `Nothing is bound to that.` are `tl!`/`fmt` keys with both
  counts as placeholders; the five headings, their five notes and all forty-four action rows are
  catalogue rows. 57 new rows in each catalogue (`Library` reuses the sidebar's own row).
- **Tests.** `every_shortcut_row_is_in_both_catalogues` walks `BINDINGS` and `Context::ALL` against
  both catalogues and fails on a row with no translation — the guard the source-reading audit cannot
  provide, because these keys are looked up at runtime. `the_shortcuts_dialog_speaks_the_interface_language`
  paints the real dialog: English (`Keyboard shortcuts`, `Flag as picked`), Japanese
  (`キーボードショートカット`, `ピックとしてマーク`, no English title, row or heading behind it)
  and Arabic (`اختصارات لوحة المفاتيح`, `وضع علامة «مختارة»`).
- **Files.** `crates/ui/src/keymap.rs` (the three accessors, the search), `crates/ui/src/shell.rs`
  (the routed overlay, two tests), `crates/ui/src/i18n/ja.tsv` and `ar.tsv` (57 rows each),
  `docs/ROADMAP.md` (Phase 4 item 2d), `docs/ARCHITECTURE.md` (8.4), `docs/UX.md` and
  `docs/FEATURES.md` (SL-L07).
- **Verified.** `cargo fmt --all`; `cargo clippy -p sortlens-ui --all-targets -- -D warnings`
  clean (exit 0); the shortcut filter's five tests pass, the two new ones among them; `cargo xtask ci`,
  `snapshot`, `deny` and `screenshots` green (exit 0 each; 27 scenes painted against the committed
  baselines with none moved, because the only scene that paints the overlay — `shortcuts-*`, searched
  to `◀` — is English and its English rows are unchanged byte for byte). No release workflow
  dispatched: this slice touches no workflow, `xtask`, `Cargo.toml` or release code.
- **Next.** Suggested keywords still needs `sortlens-ai`'s own suggestion wording and a piece per
  proposal shape before its panel can be translated; the `craft-keymap` editor's chrome stays
  English in the dependency until it offers keys; the sentences the engine composes for the support
  report and the activity row's job lines are the same shape as the change viewer's pieces.

## 2026-11-26 (eighty-seventh) — every colour word is the catalogue's, from the chip to the status line

- **Why this came up.** The eighty-sixth entry's "Next" named the sidebar's colour chips and the label
  menu: they painted `Red`…`Purple` literally, and `colour_word` already existed one panel away for the
  change viewer, so a Japanese screen said 最近の変更 above a filter chip that still said `Red`.
- **One piece, one helper.** `colour_word` is now the only place a label is named; `colour_in_sentence`
  lowercases it for English prose and leaves Japanese and Arabic exactly as the translator wrote them.
  Three wording helpers — `folder_mark_notice`, `collection_colour_notice` and `label_notice` — word the
  status lines from the same piece with `{colour}` and `{n}` placeholders, so a toast cannot name a colour
  differently from the menu that set it and a count is written in the reader's digits.
- **Every surface is routed.** The filter chip's word and its spoken name (and the hover
  `Only photos with this colour label — an empty ring means every colour`), both sidebar `Colour` menus
  (`Colour`, `None`, then the five words), the inspector's Label row and Tags tab (`No colour label.`),
  Library insights' label bars, and the recolour or folder-mark status lines. Eighteen rows in each
  catalogue; English is unchanged to the byte, except the folder notice's stray trailing space, now gone.
- **Tests.** `every_colour_word_is_the_catalogues` pins the English wording byte for byte and the Japanese
  and Arabic words in the menus and all three sentence helpers; the catalogue audit demanded both
  catalogues' eighteen rows and passed first try. `the_colour_chip_speaks_the_interface_language` paints
  the real filter strip with a colour chosen: English `Red`, then Japanese `赤` with no `Red` behind it,
  then Arabic `أحمر`.
- **Files.** `crates/ui/src/shell.rs` (the helpers, ten routed surfaces, two tests),
  `crates/ui/src/i18n/ja.tsv` and `ar.tsv` (18 rows each), `docs/ROADMAP.md` (Phase 4 item 2d),
  `docs/ARCHITECTURE.md` (8.4), `docs/FEATURES.md` (SL-L07) and `docs/UX.md` (5.2).
- **Verified.** `cargo fmt --all`; `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean;
  `cargo xtask ci` green; the UI crate's 193 tests pass and `snapshot`/`deny` are green (exit 0);
  `wasm` green and `screenshots` green (27 scenes, no baseline moved, because no scene paints the chip
  in a language other than English). No release workflow dispatched: this slice touches no workflow,
  `xtask`, `Cargo.toml` or release code.
- **Next.** Keyboard shortcuts still paints the keymap table's labels literally (the colour rows at
  `(Key::Num6, ColorLabel::Red)` and their siblings), so that dialog is next; Suggested keywords needs
  `sortlens-ai`'s own suggestion wording and a piece per proposal shape before its panel can be translated.

## 2026-11-26 (eighty-sixth) — the Batch applied receipt is worded from the batch's own pieces

- **Why this came up.** The eighty-fifth entry's "Next" named Batch applied first: its headline and
  detail were English sentences built inside `sortlens-cull` and painted under a translated title,
  so no word of the dialog could be translated until those sentences were pieces.
- **The receipt hands back the pieces.** `Receipt::headline_kind` returns a `ReceiptHeadline`
  (`Handled { photos }`) and `Receipt::detail_parts` a `Vec<DetailPart>` (`Destination`, `Picks`,
  `Failed`), with `Destination` `Trash`, `KeptOnDisk` or `Folder(String)`; `headline()` and
  `detail()` are now those pieces' `english()` joined with ` · `, so the record the report prints
  and the dialog cannot describe different batches. English is unchanged to the byte, pinned for
  the moved, tag-only and trash shapes with a pick and a failure.
- **The dialog is routed end to end.** The title, `Put them back`, `Done` and the restored toast are
  `tl!` keys; the headline is two templates (`{n} photo handled`, `{n} photos handled`) filled by
  `format_count`, the destination line is `tl!` per shape with `Moved to {folder}` a template, the
  pick and failure counts are templates, and so are the batch's own line (`Batch {batch} · {n}
  file(s) kept on disk`) and `{n} file(s) could not be handled` — 12 new rows in each catalogue,
  with `{n} failed` and `Recently Deleted` reused rather than duplicated.
- **Tests.** `the_receipt_english_is_joined_from_its_pieces` (cull crate) pins the piece, the
  English and the parts for all three destinations;
  `the_batch_applied_receipt_speaks_the_interface_language` opens a real scanned library, stages one
  pick and one reject, applies TagOnly and paints the dialog: English first (`Batch applied`, `1
  photo handled`, the exact detail line, `Put them back`), then Japanese (`バッチを適用しました`,
  `完了`, the headline from the catalogue) and Arabic (`تم تطبيق الدفعة`, the headline and detail
  in Arabic-Indic digits). The catalogue audit demanded both catalogues' 12 rows and passed.
- **Files.** `crates/cull/src/trash.rs` (the three pieces, `headline_kind`, `destination`,
  `detail_parts`, one test), `crates/engine/src/lib.rs` (the re-exports), `crates/ui/src/shell.rs`
  (`receipt_headline`, `receipt_destination`, `receipt_detail`, the routed dialog, one paint test),
  `crates/ui/src/i18n/ja.tsv` and `ar.tsv` (12 rows each), `docs/ROADMAP.md` (Phase 4 item 2d),
  `docs/ARCHITECTURE.md` (8.4), `docs/UX.md` and `docs/FEATURES.md` (SL-L07).
- **Verified.** `cargo fmt --all`; `cargo clippy -p sortlens-cull -p sortlens-engine -p sortlens-ui
  --all-targets -- -D warnings` clean; `cargo xtask ci` green (its UI test step includes the
  catalogue audit); `snapshot`, `deny` and `screenshots` green (27 scenes, no baseline moved,
  because no scene paints the receipt). No release workflow dispatched: this slice touches no
  workflow, `xtask`, `Cargo.toml` or release code.
- **Next.** The eighty-fifth entry's remaining surfaces: Suggested keywords needs `sortlens-ai`'s own
  suggestion wording and a piece per proposal shape before its panel can be translated (the AI
  suggestion card's `detail()` is engine-composed English), and Keyboard shortcuts still paints the
  keymap table's labels literally. Also open from the eighty-fifth entry: the sidebar's colour chips
  and the label menu paint `Red`…`Purple` literally and should share `colour_word`.

## 2026-11-26 (eighty-fifth) — the change viewer words the log's own pieces, and stops printing the op's code name

- **Why this came up.** The eighty-fourth entry's "Next" named the change viewer first: its rows were
  the log's own `Op::summary` English beside the raw op name, and the sentence could not be translated
  without the piece underneath it.
- **The log hands back a piece.** `sortlens-catalog`'s `op::Description` carries what the sentence
  needs — counts, paths, file names, collection names, ids, a folder mark's pin and colour, the
  colour label itself — and `Op::description()` builds it while `Op::summary()` is now exactly
  `description().english()`. A group reads as its first real change (`GroupedMore { first, others }`
  one level up), a provenance-only group reads as its own single piece, and an empty group as
  `GroupedNothing`. `history::Entry` carries the piece next to the English `summary` the report and
  the command line print, so a translated viewer and the English report cannot describe different
  work.
- **The viewer is routed end to end.** `history_line` words every variant through `i18n::fmt`; the
  counts go through `format_count` inside the suite's `{n} photo(s)` wording (the cull sheet's and
  Library health's voice), the two keyword counts through the same, and the colour inside a
  sentence through `colour_word` (`Red`…`Purple`, five new rows). The title, both hints, `Export
  report…` with its hover, the footer and `no date` are `tl!` keys. The row's second line is now the
  time alone: `tagged`, `rate` and their siblings stay in the log, the report and the wire, where
  they are record fields, instead of being printed untranslated under every sentence.
- **Tests.** `the_summary_is_joined_from_the_description_piece` pins eleven op/piece/English triples
  byte for byte and the three group shapes; `recent_changes_come_back_newest_first_and_dated` now
  also asserts the entry carries `Keyworded { photos: 1 }` and `Rated { photos: 2 }`;
  `the_change_viewer_speaks_the_interface_language` opens a real scanned library, rates one photo and
  paints the dialog: English (`Recent changes`, `Rated 1 photo(s)`, `Export report…`), Japanese
  (`最近の変更`, the rated row worded from the piece, no English title or row behind it), Arabic
  (`التغييرات الأخيرة`, the row in Arabic-Indic digits, no English row behind it). The catalogue
  audit demanded both catalogues' new rows and passed first try.
- **Files.** `crates/catalog/src/op.rs` (the `Description` piece, `description()`, `summary()` joined
  from it, one test), `crates/catalog/src/history.rs` (the field, the fill, one extended test),
  `crates/engine/src/lib.rs` (the re-export), `crates/ui/src/shell.rs` (`history_line`,
  `history_photos`, `colour_word`, the routed dialog, one paint test), `crates/ui/src/i18n/ja.tsv`
  and `ar.tsv` (51 rows each), `docs/ROADMAP.md` (Phase 4 item 2d) and `docs/ARCHITECTURE.md` (8.4).
- **Verified.** `cargo fmt --all`; `cargo test -p sortlens-catalog` (107 passed); the new paint test
  passes; `cargo clippy -p sortlens-ui -p sortlens-catalog -p sortlens-engine --all-targets --
  -D warnings` clean (exit 0). `cargo xtask ci` stopped once, at its UI test step, on the catalogue
  audit naming one routed row with no Arabic row (`{n} keyword(s)`) — the one row this tick added
  after the gate started; both catalogues now carry it and the audit passes (189 UI tests, the only
  failure above was that row). `snapshot`, `deny` and `screenshots` were then run; no baseline moved,
  because no screenshot scene paints the change viewer. No release workflow dispatched: this slice
  touches no workflow, `xtask`, `Cargo.toml` or release code.
- **Next.** Batch applied, Suggested keywords and Keyboard shortcuts — the three surfaces the
  ROADMAP row still lists as English. Batch applied's receipt already has pieces (the export
  precedent); Suggested keywords needs `sortlens-ai`'s own suggestion wording and a piece per
  proposal shape before its panel can be translated. Also open, found here: colour label names are
  routed only inside the change viewer (`colour_word`); the sidebar's colour chips and the label
  menu still paint `Red`…`Purple` literally and should share that helper.
## 2026-11-26 (eighty-fourth) — the quit guard warns in your language, from the engine's own account of what is in flight

- **Why this came up.** The eighty-third entry's "Next" listed Recent changes, Quit Sortlens?, Batch
  applied, Suggested keywords and Keyboard shortcuts. Recent changes needs the log's twenty-five
  `Op::summary` sentences turned into pieces before a word of it can be translated; the quit guard's
  reasons are three engine-owned sentences, so this tick finished the guard outright instead of
  starting that one.
- **The engine hands back a piece.** `UnsavedReason` is `Scanning`, `StagedCull { staged }` or
  `Hashing`, and `Engine::unsaved_reasons` builds the list. `Engine::unsaved` — the sentences the
  support report and the command line print — is now that list mapped through `english()`, so the
  guard and the report cannot describe different work; `has_unsaved` reads the pieces too.
- **The guard is routed end to end.** `unsaved_line` words each piece: the scan and the similarity
  pass through `tl!`, the staged line through `i18n::fmt` with `{staged}` filled by `format_count`,
  so Arabic writes `١`. The title, `Keep working`, `Quit anyway` and its hover are `tl!` keys (7 rows
  in each catalogue).
- **Tests.** `the_quit_guard_sentences_are_joined_from_their_pieces` pins the three English
  sentences byte for byte; `the_quit_guard_names_work_in_flight` now asserts the list *is* the piece
  (`Scanning` during a scan, `StagedCull { staged: 1 }` after one pick) and that `unsaved()` equals
  the joined pieces. `the_quit_guard_speaks_the_interface_language` opens a real scanned library,
  stages one pick and paints the guard: English first (the title, the exact staged sentence, both
  buttons), then Japanese (`Sortlens を終了しますか？`, `作業を続ける`, the reason in kana with no
  `staged` left behind), then Arabic (`إنهاء Sortlens؟`, `الإنهاء على أي حال`, and `١` inside the
  Arabic sentence). The catalogue audit reads the new `tl!`/`fmt` keys and demands both rows.
- **Files.** `crates/engine/src/lib.rs` (the `UnsavedReason` piece, `unsaved_reasons`, one new test
  and one extended), `crates/ui/src/shell.rs` (`unsaved_line`, the routed guard, one paint test),
  `crates/ui/src/i18n/ja.tsv` and `ar.tsv` (7 rows each), `docs/ROADMAP.md` (M9 and Phase 4 item 2d),
  `docs/ARCHITECTURE.md` (8.4).
- **Verified.** `cargo clippy -p sortlens-engine -p sortlens-ui --all-targets -- -D warnings` clean;
  the two engine tests and the UI paint test pass; `cargo xtask ci`, `snapshot`, `deny` and
  `screenshots` are green (each exit 0) after `cargo fmt --all`, and no baseline moved. No release
  workflow dispatched:
  this slice touches no workflow, `xtask`, `Cargo.toml` or release code.
- **Next.** Recent changes — its rows are the log's own `Op::summary` English, so it needs the same
  pieces-before-labels treatment (a description enum per op, with `summary()` joined from it) before
  a word of that dialog can be translated; then Batch applied, Suggested keywords and Keyboard
  shortcuts.
## 2026-11-26 (eighty-third) — the cull sheet is worded from the session's own piece, and every count is a placeholder a language places

- **Why this came up.** The eighty-second entry's "Next" listed the unrouted surfaces, and the cull
  summary is the one whose only engine sentence is a single line — so it could be finished outright
  rather than started, the way Library health needed two ticks (eighty-first and eighty-second).
- **The session hands back a piece now.** `sortlens_cull::Summary` grew
  `headline_kind() -> Headline` (`Nothing`, `AllJudged { total }`, `Partial { judged, total }`), and
  `headline()` is now `headline_kind().english()` — English unchanged to the byte, which
  `the_headline_is_joined_from_its_piece` pins for the empty, all-judged and partly-judged cases.
  `sortlens-engine` re-exports it as `CullHeadline`, so the shell names the piece without depending
  on the cull crate. This is the same shape `sortlens-catalog`'s audit `Headline` gave Library health:
  a translated sheet cannot join an English sentence with its own labels without contradicting the
  counts beneath it.
- **The sheet is routed end to end.** `cull_headline` words the piece (`Judged all {total} photos`,
  `{judged} of {total} judged`) and `cull_foot` words
  `{changed} photo(s) will change on Apply; {deferred} deferral(s) are never written.`, both through
  `i18n::fmt` with every count a placeholder, so Arabic writes its own digits. The title, the four
  rows (`Deferred`, `Not judged`; `Picks`/`Rejects` already had rows), `Apply…`, `Keep culling`,
  `Jump to next`, `Discard`, both hover texts and the `Cull discarded` toast are `tl!` keys.
- **Tests.** `the_headline_is_joined_from_its_piece` (cull crate) checks the piece and the sentence
  agree for all three shapes. `the_cull_summary_speaks_the_interface_language` opens a real scanned
  library, stages one pick and one reject so the sheet reads `2 of 3 judged`, and paints the dialog:
  English first (`Cull summary`, the headline, `Not judged`, the exact foot sentence), then Japanese
  (`選別のまとめ`, `3 枚中 2 枚を判定`, `未判定`, `適用…`, `選別を続ける`, with no English title or row
  behind them), then Arabic (`ملخص الفرز`, `تجاهل`, and both counts in Arabic-Indic digits with no
  English left in the foot line). The catalogue audit
  (`catalogs_load_and_routed_keys_have_rows`) reads the new `tl!`/`fmt` keys and demands a row in both
  languages.
- **Files.** `crates/cull/src/lib.rs` (the `Headline` piece, `headline_kind`, one test),
  `crates/engine/src/lib.rs` (the re-export), `crates/ui/src/shell.rs` (`cull_headline`, `cull_foot`,
  the routed sheet and one paint test), `crates/ui/src/i18n/ja.tsv` and `ar.tsv` (14 rows each),
  `docs/ROADMAP.md` (Phase 4 item 2d) and `docs/ARCHITECTURE.md` (8.4).
- **Verified.** `cargo fmt --all` — `cargo xtask ci` first stopped at its opening `cargo fmt --all -- --check` on
  the hand-wrapped helper signatures and the new test's `std::fs::copy` call; `cargo fmt --all` fixed
  exactly those and the re-run is green (clippy, workspace tests, layers, roots, assets, fonts,
  version, attribution, a11y, docs, templates, workflows). `cargo xtask snapshot`, `deny`, `wasm` and
  `screenshots` green (27 scenes, no baseline moved, which is the proof English is unchanged); the
  i18n paint test and the catalogue audit pass. No release workflow dispatched: this slice touches no
  workflow, `xtask`, `Cargo.toml` or release code.
- **Next.** Recent changes — its rows are the log's own English sentences, so it needs the same
  pieces-before-labels treatment the audit and this sheet got; then Quit Sortlens? (its reasons are
  `Engine::unsaved` sentences), Batch applied, Suggested keywords and Keyboard shortcuts.
## 2026-11-26 (eighty-second) — every word in Library health is a translator's, and the toast is a sentence the catalogue words

- **Why this came up.** The eighty-first entry left the dialog half-routed and named the exact half: the
  health check's own counts, its `format!` toast, the changed-on-disk line, the root rows, the metadata
  retry and the remove-missing row. Those were shell-only strings — no engine piece was missing — so this
  tick could finish the dialog rather than start another.
- **What is routed now.** `health_stats` words the `{present} of {checked} files present · {missing}
  missing` line; `health_checked_notice` the `Checked … · … missing · … changed` toast; `health_changed_line`,
  `health_refreshed_notice`, `health_relocated_notice`, `health_root_line` (`{n} photo(s) under {root}`,
  with the folder a placeholder so a right-to-left path cannot reorder its sentence), `health_retry_line`,
  `health_retried_notice` (nothing recovered, a full read, or a partial one), `health_reread_notice`,
  `health_exifless_hover` and `health_removed_notice` the rest. Every count is a placeholder filled by
  `i18n::fmt` with `format_count`, so Arabic writes its own digits; the buttons, hover texts and the
  remove-missing helper line are `tl!` keys. English is byte for byte what it was, which is why no
  baseline moved.
- **The toast is worded when it is raised, not kept.** `health_checked_notice` is built when the check
  finishes, the same rule the delete toasts follow (seventy-ninth entry): a toast is momentary, unlike the
  export receipt or `AuditView`, which are kept as pieces because they stay on screen across a language
  switch. The dialog itself repaints its routed labels on a switch; the last toast does not follow it.
- **Thirty-one keys, both catalogues.** ja and ar rows for every new source; the placeholders keep their
  names (`{n}`, `{checked}`, `{root}`, `{remaining}`), so `fmt` fills each language's own order.
- **Tests.** `every_word_in_the_library_health_dialog_is_a_catalogued_one` pins the thirteen English
  sentences character for character. `the_library_health_dialog_speaks_japanese` asserts the Japanese
  button/keys and the translated count lines with no English left (`Checked`, `camera`).
  `the_library_health_dialog_speaks_arabic_with_its_own_digits` asserts Arabic-Indic digits in the stats,
  check, retried and removed lines. The existing paint test
  `the_library_health_audit_speaks_the_interface_language` now also sees `ライブラリを確認` and
  `まだ確認していません。` on the painted dialog and no `Check library` behind them, and the catalogue audit
  (`catalogs_load_and_routed_keys_have_rows`) reads the new `tl!`/`fmt` keys and demands a row in both
  languages — it passed first try, so the rows and the sources agree. The three wording tests were one
  test until clippy refused it at 32/30 cognitive complexity; splitting it by language is also a better
  failure message.
- **Files.** `crates/ui/src/shell.rs` (eleven wording helpers, the dialog's routed body, three tests and
  two paint assertions), `crates/ui/src/i18n/ja.tsv` and `ar.tsv` (31 rows each), `docs/ROADMAP.md`
  (Phase 4 item 2d), `docs/ARCHITECTURE.md` (8.4).
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the `health` filter's
  four tests and the catalogue audit pass; `cargo xtask snapshot`, `deny` and `screenshots` green (no
  baseline moved, which is the proof English is unchanged). `cargo xtask ci` first stopped at its last
  step, `cargo fmt --all -- --check`, on three hand-wrapped lines `health_root_line`, the root row's
  `ui.label` and two asserts; `cargo fmt --all` fixed exactly those, and the re-run is green. No release
  workflow dispatched: this slice touches no workflow, `xtask`, `Cargo.toml` or release code.
- **Next.** Recent changes, the next unrouted surface, then Quit Sortlens? (its reasons are
  `Engine::unsaved` sentences, so it needs the same pieces-before-labels treatment Library health got),
  Cull summary, Batch applied, Suggested keywords and Keyboard shortcuts.

## 2026-11-26 (eighty-first) — the library-health audit speaks your language, from the catalog's own pieces

- **Why this came up.** The eightieth entry's "Next" named Library health, and named the reason it had
  to start in the engine rather than the shell: its audit headline and detail were English sentences
  built inside `sortlens-catalog`, so translating the labels around them would have left English
  under a translated title — the trap the export entry called out.
- **The catalogue hands back pieces now.** `sortlens_catalog::integrity::Audit` grew
  `headline_kind() -> Headline` (`Clean { photos, seq }`, `Recoverable { photos, replayed }`,
  `NeedsAttention`) and `parts() -> Vec<AuditPart>` (`Snapshot(Option<u64>)`, `Replayed`, `Skipped`,
  `LogBytes`), and its own `headline()`/`detail()` are joined from exactly those pieces — English
  unchanged to the byte, which is what `the_headline_and_detail_are_joined_from_their_pieces` pins for
  the clean, recoverable, snapshot-less and needs-attention cases. `sortlens-engine` re-exports both as
  `AuditHeadline`/`AuditPart`, so the shell names them without depending on the catalog crate.
- **What the dialog says now.** `health_dialog` paints `tl!("Library health")` and the
  `Catalog files` section, and the audit line is worded piece by piece through `i18n::fmt` with the
  counts grouped by `format_count` — so Arabic writes its own headline and its detail's numbers
  (`٣`, `٣١`, `٢٠٤٨`) in Arabic-Indic digits. The last audit is kept as pieces in the new
  `AuditView` rather than as the English sentence it used to be, so the section repaints when
  Settings → Language moves; the toast remains worded when it is raised. Sixteen keys, both catalogues.
- **Tests.** `the_headline_and_detail_are_joined_from_their_pieces` (catalog) fails if a piece and its
  English diverge. `the_library_health_audit_speaks_the_interface_language` paints the real dialog in
  English, Japanese and Arabic with a fabricated audit and asserts the English words, the Japanese
  title/section/button with no `Library health` behind them, the Japanese headline and detail, the
  Arabic title/section/button, the Arabic-Indic digits in both audit lines, and that painting closes
  nothing. The catalogue audit covers all sixteen new rows.
- **Named as not shipped.** This is the dialog's *catalog half*. Still English inside Library health:
  the health check's own stats line and its `Checked … · … missing · … changed` toast, the changed-on-disk
  line and Refresh, root repointing (`Point at…` and its sample note), the metadata retry section and
  `Remove missing from library`. Then Recent changes, Quit Sortlens? (its reasons are `Engine::unsaved`
  sentences), Cull summary, Batch applied, Suggested keywords and Keyboard shortcuts. The four remaining
  catalogues (zh-TW, ko, th, vi) and RTL screenshots still wait on a font build with an Arabic face.
- **Files.** `crates/catalog/src/integrity.rs` (the two piece enums, the joining, one test),
  `crates/engine/src/lib.rs` (the re-export), `crates/ui/src/shell.rs` (`AuditView`, four wording
  helpers, the dialog's catalog section, one test), `crates/ui/src/i18n/ja.tsv` and `ar.tsv` (16 rows
  each), `docs/ROADMAP.md` (Phase 4 item 2d), `docs/ARCHITECTURE.md` (8.4).
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; `cargo xtask ci`,
  `snapshot`, `deny`, `wasm` and `screenshots` run in this tick. No release workflow dispatched: this
  slice touches no workflow, `xtask`, `Cargo.toml` or release code.
- **Next.** Finish the dialog in the shell alone, where no new engine pieces are needed: the health
  check's stats line and its toast as `fmt` templates, the changed-on-disk line and Refresh, the root
  rows (`{count} photo(s) under {root}`, `Point at…`), the metadata retry lines and `Remove missing`.
  Then Recent changes.

## 2026-11-26 (eightieth) — the rename and About dialogs speak your language, and every word in them is one a translator can reach

- **Why this came up.** The seventy-ninth entry's "Next" named *Rename collection* the smallest
  unrouted dialog, with Library health behind it. It and About are the two dialogs whose every word
  belongs to the shell — a version string and a registry count, no engine sentence anywhere — so
  routing them is a slice that can be finished rather than half-done.
- **What the shell says now.** `rename_dialog` paints `rename_title()` and `tl!("Save")"/`tl!("Cancel")`,
  and its toast is `renamed_notice(name)` — `Renamed to {name}`, with the name as a placeholder so
  `fmt` isolates it and a right-to-left collection name cannot reorder the sentence around itself.
  `about_dialog` paints `tl!("About Sortlens")` and the one-sentence description through `tl!`, and
  its two facts became templates: `Version {version}` and `{commands} commands, {ready} ready today.`
  The counts go through `format_count`, so the digits are the reader's own. English is byte for byte
  what it was, which is why no baseline moved.
- **Seven keys, both catalogues.** ja and ar rows for the seven sources; "Sortlens" stays the
  product's own name in both, because a product's name is not a word to translate.
- **Tests.** `the_rename_and_about_dialogs_speak_the_interface_language` paints both dialogs in one
  frame in English, Japanese and Arabic. It asserts English character for character ("Renamed to
  Summer", "47 commands, 12 ready today."), the Japanese title and buttons ("コレクション名を変更",
  "保存", "キャンセル"), the Japanese toast leaving no "Renamed" behind, both Japanese About lines
  ("バージョン …", "コマンド 47 件、うち 12 件が利用可能"), the Arabic title and the command line's
  Arabic-Indic digits (٤٧, ١٢), and that painting closes nothing: the rename target and About are both
  still open afterwards. The catalogue audit was the gate that mattered — it reads the new `tl!`
  keys and `fmt` templates out of `shell.rs` and demands a row in both languages.
- **Named as not shipped.** Library health is next but not this slice: its audit headline and detail
  are `Engine::audit` sentences, so routing the labels alone would leave English under a translated
  title — the trap the export entry called out, and it needs the engine's own parts first. Behind it:
  Recent changes, Quit Sortlens? (its reasons are `Engine::unsaved` sentences), Cull summary, Batch
  applied, Suggested keywords and Keyboard shortcuts. After the dialogs: the four remaining
  catalogues (zh-TW, ko, th, vi) and RTL screenshots, still waiting on a font build with an Arabic face.
- **Files.** `crates/ui/src/shell.rs` (two dialogs, four helpers, one test),
  `crates/ui/src/i18n/ja.tsv` and `ar.tsv` (7 rows each), `docs/ROADMAP.md` (Phase 4 item 2d),
  `docs/ARCHITECTURE.md` (8.4).
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean, `cargo xtask ci`
  green, `snapshot`, `deny` and `screenshots` green. No release workflow dispatched: this slice
  touches no workflow, `xtask`, `Cargo.toml` or release code.
- **Next.** Library health, starting in the engine rather than the shell: give `Engine::audit` a
  headline and detail that are pieces (`AuditPart`), word them through the catalogues, then route the
  dialog's own labels. Then Quit Sortlens?, whose reasons out of `Engine::unsaved` need the same
  treatment.

## 2026-11-26 (seventy-ninth) — the screens that delete files ask, warn and count in your language

- **Why this came up.** The seventy-eighth entry's "Next" named the delete confirmation as the
  worst unrouted surface: its count and its changed-on-disk warning were English on the screen that
  destroys files, beside mode radios that were already catalogued.
- **Three confirmations are routed now.** The similar view's delete, the cull batch's Apply and the
  rejects-bin emptying word their counts as `fmt` templates — `{n} photo(s)`, `{rejects} rejects ·
  {picks} picks`, `{batches} batch(es), {files} file(s) will lose their restore record.` — so a
  language is not forced through English's "(s)". The warning about files changed on disk is one
  key (`changed_on_disk_warning`) shared by both delete dialogs, so the two screens cannot drift
  apart. A toast is worded when it is raised (`delete_notice`, `apply_notice`,
  `forgotten_notice`): a toast is momentary, unlike the export receipt that is kept as pieces
  because it stays on screen across a language switch. The cull foot's staged sentence became one
  template with `{n}` and `{mode}`, so the mode word lands where a translation's grammar wants it
  instead of being glued onto an English sentence. English is byte for byte what it was, which is
  why the screenshot baselines did not move.
- **The catalogue audit grew teeth.** It read only `tl!` literals; it now reads the
  `crate::i18n::fmt(` templates too, deduplicated, so a template with no row fails the test
  instead of quietly painting English. Writing that exposed a trap: a routed key written with a
  backslash line continuation is read to its closing quote, continuation and indentation included,
  so the empty-bin sentence was rewritten as one literal — a key no translator could type is a key
  no catalogue can hold.
- **Tests.** `the_delete_confirmations_speak_the_interface_language` paints all three dialogs in one
  frame in English, Japanese and Arabic: it asserts the English words character for character, the
  Japanese count ("写真 3 枚"), the translated warning and mode ("ゴミ箱へ移動"), the Japanese
  toast and staged line leaving no English word behind, and Arabic counts in Arabic-Indic digits
  ("حُذفت ٢ صورة", "نُسي ٢ دفعة تغطي ٧ ملفًا") with no "Deleted" left. It also asserts that
  opening the dialogs deletes nothing. The audit test covers all eighteen new keys — it failed
  first on the Japanese `Forgot …` row (written "の記録" where the test reads "復元記録"), and the
  row was the thing fixed.
- **Files.** `crates/ui/src/shell.rs` (the five wording helpers, three dialogs, the cull foot, one
  test), `crates/ui/src/i18n.rs` (the audit reads `fmt` templates), `crates/ui/src/i18n/ja.tsv` and
  `ar.tsv` (18 rows each), `docs/ROADMAP.md` (M9 and Phase 4 item 2d), `docs/FEATURES.md`,
  `docs/ARCHITECTURE.md` (8.4).
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean, `cargo xtask ci`
  green, `cargo xtask snapshot` and `deny` green, and `cargo xtask screenshots` green — the English
  dialogs are the same pixels, so every baseline matched. No release workflow dispatched: this
  slice touches no workflow, `xtask`, `Cargo.toml` or release code.
- **Next.** The remaining dialogs are the next unrouted surface, and *Rename collection* is the
  smallest of them; then Library health, Recent changes, Quit Sortlens?, Cull summary, Batch
  applied, Suggested keywords, Keyboard shortcuts and About. After the dialogs: the four remaining
  catalogues (zh-TW, ko, th, vi) and RTL screenshots, which still wait on a font build with an
  Arabic face.

## 2026-11-26 (seventy-eighth) — the export dialog tells you what it will write in your language, because a spec is now its own pieces

- **Why this came up.** The i18n row's open list named "the unrouted surfaces", and the export dialog
  was the next one a cull reaches: its count line printed `Spec::summary()`, an English sentence built
  inside `sortlens-export`, so translating the dialog's labels would have left the one line that says
  what will be written in English, and an export receipt is the same shape.
- **The exporter hands back pieces now.** `Spec::parts()` yields `SpecPart::Format`, `Quality`,
  `LongestEdge`, `NoMetadata` and `Lossless` in reading order; `Report::parts()` yields
  `ReportPart::Written`, `Skipped`, `Bytes` and `Failed`. `summary()` and `headline()` join those
  pieces, so the English is what it always said (pinned by
  `a_description_is_the_pieces_it_is_written_from` and `a_receipt_is_the_pieces_it_is_written_from`,
  which also assert the sentence equals its own pieces). The numbers stay numbers, which is what lets
  the dialog group them the way the language does ("2 048 px" in English, "2,048 px" in Japanese)
  while the engine's log line keeps its plain "2048".
- **What the shell does with them.** `export_format_name`, `export_collision_name`,
  `export_spec_line` and `export_receipt_line` word each piece through the catalogues; the dialog's
  30 new keys are in `ar.tsv` and `ja.tsv`, and the catalogue audit demands both. A format's own
  name (`JPEG`, `PNG`) is not a word to translate. `UiState::export_receipt` is now an
  `ExportReceipt` (written, skipped, failed, bytes, dest) rather than a finished string, so a receipt
  already on screen repaints when Settings → Language changes; the host builds it with
  `ExportReceipt::from_report`.
- **Tests.** `the_export_dialog_and_its_receipt_speak_the_interface_language` paints the dialog in
  English, Japanese and Arabic, asserts the Japanese spec line ("JPEG · 品質 82 · 長辺 2,048 px ·
  メタデータなし") and that no English word survives, that `JPEG` is not translated, and that the
  Arabic receipt writes its count in Arabic-Indic digits; English is asserted to be the words it
  always was. The audit covers the new rows because it reads `tl!` keys straight out of `shell.rs`.
- **Files.** `crates/export/src/lib.rs` (parts, two tests), `crates/engine/src/lib.rs` (the
  re-exports), `crates/ui/src/shell.rs` (`ExportReceipt`, the four wording helpers, the dialog, one
  test), `crates/ui/src/i18n/ar.tsv` and `ja.tsv` (30 rows each), `apps/sortlens/src/main.rs`
  (receipt built from the report), `docs/ROADMAP.md`, `docs/FEATURES.md`, `docs/UX.md`,
  `docs/ARCHITECTURE.md` — the same reconciliation.
- **Verified.** `cargo xtask ci` green (its first run stopped at `cargo fmt --all -- --check`; the
  workspace was formatted and the gate re-run), `cargo xtask snapshot`, `deny` and
  `screenshots` green — all 27 scenes painted against the committed baselines, so the English
  dialog the scenes contain did not move. No `wasm` gate exists in this workspace. No release
  workflow was dispatched: the slice changes no packaging input (no workflow, `xtask`, `Cargo.toml`
  or release code), and `cargo xtask ci` builds the app crate the release would package.
- **Next.** The remaining dialogs are the next unrouted surface, and the delete confirmation is the
  worst of them: its count and the mode words (`DeleteMode`) are English on the screen that destroys
  files. After that: the four remaining catalogues (zh-TW, ko, th, vi) and RTL screenshots, which
  still wait on a font build with an Arabic face.

## 2026-11-25 (seventy-seventh) — the lavapipe set is the whole shell again: forty-seven scenes, nothing missing

- **Why this came up.** `CI`'s `screenshots` job had been red for six commits on four paper scenes, and the
  seventy-sixth tick made a red run hand over *every* mismatch instead of the first one. Its run for `04736e9`
  (`45546143839`) therefore uploaded exactly four frames: `compare-paper`, `detail-paper`,
  `library-justified-paper` and `library-square-paper`. The commit after that (`9b75260`) installed the first
  two of them; this tick installed the other two, which is the whole of the gap.
- **The frames are the job's own.** `gh run download 45546143839 -n screenshot-set -D tmp-shot3` gave four
  `lavapipe/*.new.png`, and the two still missing were copied with `.new` stripped. That the set is now
  byte-exact is not a guess: the artifact's `detail-paper.new.png` and the already-committed
  `detail-paper.png` — installed from the *previous* run — hash identically (`DEEFEF39…`), so the job's
  renderer is deterministic run to run and the two new files are exactly what the next `screenshots` job
  will paint.
- **Why those two lagged.** Each paper scene is the second half of a pair inside one test: `compare` after
  `detail` in `detail_and_compare_paint_in_both_themes`, `library-square` after `library-justified` in
  `library_grid_paints_both_layouts_in_both_themes`. Before the seventy-sixth tick a mismatch panicked its
  test, so no artifact could ever name the scene after the first. The pre-refactor artifact of run
  `45544180229` is the proof: twenty frames, `library-square-charcoal.new.png` among them and no paper
  sibling at all.
- **Files.** `crates/ui/tests/snapshots/lavapipe/compare-paper.png` (new),
  `crates/ui/tests/snapshots/lavapipe/library-square-paper.png` (new), `CHECKPOINT.md`. That is
  forty-seven PNGs, the number `docs/ARCHITECTURE.md` §12 already names as every scene `screenshots.rs`
  paints.
- **Verified.** The artifact is the renderer's own output and the overlap hashes byte-identical to the
  committed scene, so nothing here is repainted by hand. The gate agrees: `cargo xtask ci` exits `1` on the
  tree without these two files — the `screenshots` test (`--all-features`, so the gate does paint) panics on
  the two missing scenes — and exits `0` with them, which is the failing-when-broken test this slice owes.
  `cargo xtask snapshot` and `cargo xtask deny` green; no `wasm` (no `core`/`ui` source).
- **Next.** The trunk first: the run for this commit turns the `screenshots` job green if the set is
  complete, and the `ci` job that was in flight for `9b75260` (`45546324509`) can be read with one
  `gh run view` to confirm nothing else was red. After that the only open roadmap row is SL-L07's
  remainder — the zh-TW/ko/th/vi catalogues (data + `LANGUAGES` + `font_manifest`'s script table + a
  `format.rs` style row each), the unrouted surfaces, and the RTL screenshots that wait on a font build
  with an Arabic face — which is the next slice.

## 2026-11-24 (seventy-sixth) — a red screenshot run hands over every scene it painted, not just the first failure

- **Why this came up.** `CI` on `main` is red in `screenshots`, and the run at `40e838a` (`45543501424`) is the
  evidence. Its `screenshot-set` artifact holds twenty `.new.png` frames, and the committed `lavapipe` set still
  lacks four paper scenes — `detail-paper`, `compare-paper`, `library-justified-paper`, `library-square-paper`.
  The artifact could not have held them. Each pair lives in one test (`detail_and_compare_paint_in_both_themes`,
  `library_grid_paints_both_layouts_in_both_themes`) and `shoot` panicked on the first mismatch, so every scene
  after it was never painted at all. Six red runs and two baseline commits later the set is still four short, and
  the artifact that is supposed to complete a set was structurally unable to name more than the first failure of
  each test. That is the defect the four missing files are a symptom of.
- **A mismatch no longer ends the test.** `shoot_at`/`shoot_at_lenient` record the scene's baseline name and the
  error in a per-thread `Painted { scenes, mismatches }` instead of `.expect()`-ing, so the next scene of the same
  test still paints and still writes its `.new.png`. Each of the 23 painting tests ends with
  `assert_every_scene_matched()`, which drains the record and fails with every scene that differed and
  `N of this test's M scenes` — the count is what lets the message say a scene matched. The record is
  thread-local because libtest gives every test its own thread, and taking it empties it so nothing leaks between
  tests.
- **The check cannot be forgotten.** Three new tests, one of which reads `include_str!("screenshots.rs")`:
  `every_test_that_paints_a_scene_checks_its_whole_report` names any test that paints and never calls the check.
  It was proved to bite by deleting `cull`'s call — it answered `["cull_paints_in_both_themes"]` — and then the
  call was restored. `a_report_names_every_scene_that_differed` pins the counting and the emptying;
  `a_mismatch_fails_the_test_once_every_scene_has_painted` pins the panic.
- **Files.** `crates/ui/tests/screenshots.rs` (`Painted`, `RECORD`, `record`, `take_report`,
  `assert_every_scene_matched`, the three `shoot*` call sites, one call at the end of each painting test, the
  three tests); `docs/ARCHITECTURE.md` §12 (what a red run hands over; the stale "Thirty-one PNGs" corrected to
  forty-seven); `docs/ROADMAP.md` (Phase 5 item 3's artifact sentence now says *every* frame, and why).
- **Verified.** `cargo xtask screenshots` green: 27 tests, 50 s, no baseline moved and no `.new` written — the
  refactor paints exactly what it painted before. `cargo xtask ci` (one `cargo fmt --all` first), `snapshot` and
  `deny` for this commit. Nothing here is a UI change, so no new `Context::run_ui` test was owed. `cargo xtask
  mobile` still needs the NDK and Xcode, as documented.
- **Next.** The trunk first, and it is one command: read the `CI` run for this commit with one
  `gh run view <id> --json status,conclusion,jobs`. Its `screenshots` job will be red on the four missing paper
  scenes — and now paints all four, which is the point of this commit. `gh run download <id> -n screenshot-set -D
  tmp-shot`, move `detail-paper.new.png`, `compare-paper.new.png`, `library-justified-paper.new.png` and
  `library-square-paper.new.png` to `crates/ui/tests/snapshots/lavapipe/` with `.new` stripped, read them, and
  commit: that is the tick that turns the job green. After that the only open roadmap row is SL-L07's remainder
  (zh-TW/ko/th/vi catalogues, the unrouted surfaces, and RTL screenshots which wait on an Arabic-capable font
  build).
## 2026-11-25 (seventy-second) — the shell writes numbers and dates the reader's language (SL-L08)

- **Why this came up.** The last row left open in M9 by the sixty-eighth tick's scaffolding was
  the arithmetic: every count in the shell went through `sortlens_core::format_count` (thin spaces,
  Latin digits), every file size through `format_bytes` (`1.4 MB`), and the detail panel's date
  through `Photo::captured_label` (`2026-11-21 14:32`) — one universal shape regardless of whether
  the interface was English, Japanese or Arabic. An Arabic interface that mirrors its rows into
  the reading start but writes `12,408` and `21/11/2026` in Latin digits is translated, not localised.
- **One table, held to the picker.** `crates/ui/src/format.rs` is the whole slice: a `Style` per
  offered language (digit zero, grouping mark, decimal mark, date field order and separator), the
  three functions the shell calls (`count`, `size`, `date_time`), and explicit `_in(style, …)`
  forms so tests never switch the thread-local language. The table is English (Latin, `1 234 567`
  with the narrow no-break space, `.` decimal, `2026-11-21 14:32`), Japanese (Latin, `12,408`,
  `2026/10/03 04:00`) and Arabic (Arabic-Indic `١٢٬٤٠٨`, `٫` decimal, day first
  `٠٣/١٠/٢٠٢٦ ٠٤:٠٠`). The formatting region is the interface language on purpose: the operating
  system's regional settings would make a committed scene read differently on two machines.
- **English is byte-identical.** The English style reproduces what the app painted before, and a
  test compares it against `format_count`/`format_epoch` across the counts and the epoch both
  modules were tested on — so routing the shell through the new module moved no committed scene.
  The shell changes are three one-line wires, not 30 call-site edits: `format_count` (the single
  funnel every sidebar, figure, notice and status count already used) now calls `format::count`;
  the detail panel's Size row calls `format::size`; its Captured row formats the epoch through
  `format::date_time` instead of `captured_label`.
- **The layers it needed.** `sortlens_meta::CivilTime` and `civil_time(seconds)` take an epoch apart
  (`format_epoch` is now written in terms of it) so an interface can order the fields itself
  instead of parsing the ISO sentence back apart; `sortlens_core::byte_measure(bytes)` returns the
  scaled value, unit and decimal digits `format_bytes` writes, so `format::size` can put those
  digits in the language's set and mark without duplicating the unit thresholds. `crates/ui` now
  depends on `sortlens-meta` for the date parts.
- **Files.** `crates/ui/src/format.rs` (new); `crates/ui/src/lib.rs` (`mod format`);
  `crates/ui/src/shell.rs` (the three wires, the language test's digit assertion);
  `crates/meta/src/lib.rs` (`CivilTime`, `civil_time`); `crates/core/src/units.rs` (`byte_measure`);
  `crates/core/src/lib.rs` (the re-export); `crates/ui/Cargo.toml`; `docs/ROADMAP.md` (SL-L08 row
  ticked with the shipped shape); `docs/FEATURES.md` (SL-L08); `docs/UX.md` (the i18n bullet);
  `docs/ARCHITECTURE.md` §8.4 (the formatting region and the table test; the stale "all 19
  baselines" sentence corrected).
- **Tests that fail when the behaviour is absent** (six new): each offered language has exactly
  one style and every style belongs to an offered language; English reproduces `format_count`/
  `format_epoch`; grouping, digits and decimal marks per language; date field order per language
  including a pre-1970 epoch; an unknown code falls back to English. Headless in the existing
  Settings → Language test: after clicking العربية, the sidebar's count galley contains `٠`
  (`U+0660`), which fails when the count funnel goes back to Latin digits.
- **Verified.** `cargo xtask ci` green (the first run failed only `cargo fmt --check`, reformatted
  and rerun); `cargo xtask snapshot`, `deny`, `wasm` and `screenshots` run for this commit.
- **Next.** The trunk first: CI run `45542266937` was still running for `cec3f94` when this tick
  began, so read it with one `gh run view 45542266937 --json status,conclusion,jobs` before
  anything else. Then the only open roadmap row is SL-L07's remainder: zh-TW/ko/th/vi catalogues
  (data + `LANGUAGES` + `font_manifest`'s script table + a style row), the unrouted surfaces, and
  RTL screenshots, which still wait on a font build with an Arabic face. If that run is red, fixing
  it is the tick's slice instead.
## 2026-11-25 (seventy-first) — a frame the app draws has the window's three buttons (Phase 3, 3B.6 (i))

- **Why this came up.** The shell has been able to run under its own frame since `craft-window`'s
  `Frame::Integrated` shipped (macOS draws the bar as the title bar, with `title_inset` clearing the traffic
  lights), but the bar never drew the window's own buttons. Under an integrated frame on Windows and Linux that
  left a window with no minimize, no maximize and no close on the bar — the frame was the app's and the
  buttons were nobody's.
- **The frame decides; the shell draws.** `UiState::captions` is set once by the host from the frame policy
  (`matches!(decision.frame, Frame::Integrated(_))`) and never guessed by the bar. When it is true the bar
  paints minimize, maximize/restore and close at its far end: the row is laid out right-to-left, so the
  buttons are its last children and nothing the shell adds afterwards can push them off. When it is false the
  bar paints nothing, because the system title bar already drew them.
- **Each button is a glyph with a name.** A rule, a frame (or the two-frame restore glyph — read from
  `viewport().maximized` each frame, so a system maximize moves it), and a cross. `caption_button` allocates
  `CAPTION_W` square, tints hover with `t.hover` — close with `t.reject`, its glyph taking `t.bg` so the
  cross stays legible on the red — and labels each through `a11y::label_widget` and `on_hover_text`.
- **The bar asks, the host commands.** `Action::MinimizeWindow`, `ToggleZoom` and `CloseWindow` are what the
  bar raises; `apps/sortlens` sends `ViewportCommand::Minimized(true)`, `Maximized(!maximized)` and `Close`.
  Close is deliberately not `Quit`: `Quit` means the unsaved-changes guard was already answered, and this
  button has not asked — so the window's own close handling runs the guard, exactly as the system button does.
- **Files.** `crates/ui/src/shell.rs` (`Action`'s three arms, `UiState::captions`, `Caption`,
  `caption_buttons`, `caption_button`, `caption_glyph`, the bar's right-to-left block, the test);
  `apps/sortlens/src/main.rs` (`captions` from the frame decision, passed into `App::new`, the three viewport
  commands); `docs/ROADMAP.md` (3B.6 (i)); `docs/UX.md` §3; `docs/ARCHITECTURE.md` §8.1.
- **Tests that fail when the behaviour is absent.** One headless `Context::run_ui` test at 1280×800:
  `the_window_buttons_appear_with_the_app_s_own_frame_and_ask_the_host` walks the painted shapes for diagonal
  strokes at the bar's right edge (none where the OS draws them, the close cross's two strokes with the app's
  frame), reads each button's AccessKit label and asserts the order `Minimize`, `Maximize`, `Close`, then
  presses close's centre and asserts `Action::CloseWindow` was raised and `Action::Quit` was not.
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; `cargo test -p sortlens-ui
  --lib the_window_buttons` passes. `cargo xtask ci`, `snapshot`, `deny` and `screenshots` were run this tick
  for the commit; the caption glyphs paint only when `captions` is true and no committed scene sets it, so no
  baseline moves.
- **Still red on `main`, and these are the next two slices.** The `CI` workflow run `45536458391` (sha
  `85da2b4`) failed two jobs, both for reasons that predate this entry: `screenshots` (13 of 23 scenes differ
  on Mesa's software Vulkan — six commits of red the loop never looked at, because it only watched `release`)
  and `mobile (ubuntu-latest, android)` (`cc-rs` cannot find `aarch64-linux-android-clang`; the NDK is
  installed but no `CC_aarch64_linux_android`/`AR_aarch64_linux_android` points at it). The caption slice does
  not touch either.
- **Next.** Take the red trunk first, as the loop's rules say. `screenshots`: the baselines are painted on
  this machine's GPU and compared on a runner with none, so no tolerance can honestly bridge them — the fix is
  a per-rasteriser baseline set (a distinct directory for the lavapipe render) plus the CI job uploading the
  `.new` images it paints so the next tick commits them. Then `mobile`'s android leg: give `cargo xtask
  mobile` the NDK's `CC`/`AR` for the target, the same way the iOS leg inherits Xcode's clang.

## 2026-11-24 (seventieth) — the phone's font set is trimmed to the languages it ships (Phase 4 item 3b)

- **Why this came up.** The sixty-ninth entry left the roadmap's 3b row with three open items — a device
  build, the iOS Photos library, and "trimming the embedded font set for a phone". The first two wait for a
  Mac and an NDK; the third was buildable here, and a phone binary that embeds the whole 48 MiB craft-fonts
  manifest to paint three languages was paying for families it could never draw (the manifest's Hangul face,
  for a build that ships English, Arabic and Japanese).
- **One parser answers for two readers.** `crates/ui/src/font_manifest.rs` is a dependency-free module
  `crates/ui/build.rs` includes at build time (`#[path]`) and `xtask` includes for its font report, so a
  build and `cargo xtask fonts` cannot disagree about what a manifest line means or which faces survive. It
  parses craft-fonts' `family | style | file | scripts | …` lines, selects the faces a build embeds, and
  validates that a set covers the scripts the interface draws.
- **The filter is `CRAFT_FONTS_SCRIPTS`.** Comma- or space-separated ISO 15924 codes; unset means the whole
  manifest, as the desktop embeds today. `cargo xtask mobile` passes exactly `PHONE_SCRIPTS` — `Latn`,
  `Zsym`, `Arab`, `Jpan`, the interface's own two plus every shipped language's — when `CRAFT_FONTS_DIR` is
  set, through a new `run_env` helper (edition 2024 makes `set_var` unsafe). `build.rs` also writes
  `SELECTED_SCRIPTS` into the generated table, and `fonts::selected_scripts()` reports it, so a release can
  say what it carries without re-deriving it.
- **Refusals before bytes, not after.** A requested script no manifest face covers is refused by name (a
  typo such as `Hng` never embeds a half-set); `cargo xtask fonts` refuses a set — filtered or not — that
  misses a script the interface ships, which strengthens the old "covers no Latin" check to the shipped
  languages too. The phone script table is held to its definition by a test (required scripts ∪ shipped
  languages), and another test holds `SHIPPED_LANGUAGE_SCRIPTS` to the picker's own `LANGUAGES`, so adding
  a language without its script fails the build, not the phone.
- **Verified end to end against a fixture checkout**: a fake craft-fonts clone with six faces (Japanese,
  Arabic, Hangul, two symbol faces) — `cargo xtask fonts` reported `5 of 6 face(s) in 4 family(ies) after
  the CRAFT_FONTS_SCRIPTS filter`, the unfiltered report kept all six, and the typo filter was refused by
  name; the filtered `cargo build -p sortlens-ui` generated a table with the five kept faces and no
  `NanumGothic`. On this checkout (no fonts checkout, CI's configuration) nothing changes: `xtask fonts`
  still reports "not embedded" and the app still runs on egui's own faces.
- **Files.** `crates/ui/src/font_manifest.rs` (new); `crates/ui/build.rs` (the filter, `SELECTED_SCRIPTS`,
  the parser shared); `crates/ui/src/fonts.rs` (`selected_scripts`, `REQUIRED_SCRIPTS` re-exported from the
  module, two new tests); `crates/ui/src/lib.rs`; `xtask/src/main.rs` (the module include, `run_env`,
  `mobile` passing the filter, `fonts` validating coverage); `docs/ROADMAP.md` (3b shipped bullet, the font
  row, M10's stale "craft-update still open" sentence fixed — it shipped as item 2c on 2026-11-24);
  `docs/ARCHITECTURE.md` §8.3, `docs/ATTRIBUTION.md` (the trim and the strengthened refusal),
  `docs/RELEASE.md`, `docs/UX.md` (what 3b still lacks is now two items, not three).
- **Tests that fail when the behaviour is absent** (11 new: nine in `font_manifest` — parse, no-filter
  keeps everything, the phone filter drops the Hangul face, the typo refusal, the no-face refusal, the
  filter value's grammar, the coverage validation both ways, and the phone script set's definition — plus
  two in `fonts`: the shipped-language table matches the picker, and a filtered build names every script
  the interface ships). `cargo xtask fonts` refuses a set that misses a script the interface ships, proved
  on the fixture.
- **Verified.** `cargo test -p sortlens-ui --lib` (173), `-p xtask` (42, the module's tests run there too).
  `cargo xtask ci` green through every gate (two fmt reformats and one clippy `manual_contains` + a
  never-actually-loops loop fixed along the way). `cargo xtask snapshot` green, no golden moved.
  `cargo xtask screenshots` green, 23 scenes, no baseline moved — the phone's paint is untouched; only what
  a phone *build* embeds changed. `cargo xtask deny` green. `cargo xtask mobile` still needs the NDK and
  Xcode, as documented.
- **Next.** The iOS Photos library (iOS still reads the Files-visible Documents folder) and a device
  build; both wait for a machine with the NDK, Xcode and the iOS Photos framework. That is everything
  buildable left in the owner's Phase 0 → Phase 5 plan — the remaining 3b "Still open" items are the
  device verification itself, the APK/AAB and `.ipa` workflow legs (whose signing halves are the same
  Apple-identity blocker the M10 table already names), and the post-1.0 backlog rows.

## 2026-11-24 (sixty-ninth) — the phone asks whether the desktop is there before a code is spent (SL-J07 from the app)

- **Why this came up.** The sixty-eighth entry ended on the one thing the pairing flow still asked
  a person to do blind: type the six-digit code into an address that may answer nothing, and learn
  from the pairing's failure alone that the address was wrong — after spending the code, which
  expires, on a mistake a `GET /health` would have named in one sentence.
- **The test runs on the address alone.** Settings → Send to desktop gains **Test connection**, placed
  between the port field and the code field, because it is what the code is for. The typed address and
  port — nothing else — run it: `Client::health()`, no token, on its own `sortlens-check` thread, the
  same pattern the pairing already uses, so no frame blocks. The sentence it earns names the address it
  tested, which need not be the paired one (`The desktop at 192.168.1.10:52114 answered`, or `No answer
  from …` with the reason), and it is painted in the section and said as a notice. Nothing is stored
  either way — the pairing's own answer stays the only truth about the address — and a successful
  pairing clears the sentence. While it waits the section says `Testing host:port…` and the button
  stands down; a pairing or a send run in flight answers a test with nothing rather than piling threads
  on the same server; Forget clears it with the rest.
- **Files.** `apps/sortlens-mobile/src/send.rs` (`Check`, `DesktopLink::test`/`poll_test`/`checking`,
  the surface's `checking`/`check`); `apps/sortlens-mobile/src/lib.rs` (`Action::SendTest` arm, the
  poll); `crates/ui/src/shell.rs` (`SendSurface::checking`/`check`, `Action::SendTest`, the section's
  button and its two sentences); `crates/ui/src/i18n/{ar,ja}.tsv` (`Test connection`);
  `apps/sortlens/src/main.rs` (the desktop host's honest answer to a phone-only action);
  `docs/ROADMAP.md`, `docs/UX.md`, `docs/ARCHITECTURE.md`, `docs/FEATURES.md` (SL-J07), `docs/RELEASE.md`.
- **Tests that fail when the behaviour is absent** (4 new: three in the phone's link, the shell's send
  test extended). Link, over real TCP: a fake `/health` server answers and the sentence says so while
  `config()` stays empty and `paired` stays false; a refused port earns `No answer from …` with its
  reason, and stores nothing; a pairing in flight means a second test is never spawned (and the pairing
  still lands). Shell at 390 points: the test runs on the typed address and port with the code field
  empty, hands over both trimmed; an addressless test answers nothing; a test in flight paints
  `Testing 192.168.1.10:52114…` and is not re-run; the answered sentence is painted.
- **Verified.** `cargo test -p sortlens-mobile` (20 passed), `-p sortlens-ui --lib` (163). `cargo xtask
  ci` green through every gate (the desktop host's `Action` match needed the new arm — a phone-only
  action is answered, never dropped — and clippy's fmt pass reformatted the test). `cargo xtask
  snapshot` green, no golden moved. `cargo xtask screenshots` green with 23 scenes and no baseline
  moved (the section paints only in the phone's unpaired Settings, which no committed baseline holds).
  `cargo xtask deny` green. `cargo xtask mobile` still needs the NDK and Xcode.
- **Reconciled.** ROADMAP 3b's shipped bullet names the connection test and its tests; UX §2's Send
  rules name the Test connection step and what it stores; ARCHITECTURE §10 says the surface runs
  `GET /health` before a code is spent; FEATURES SL-J07 moved the settings screen out of "open with the
  client"; RELEASE's phone bullet says the address can be tested first.
- **Next.** The iOS Photos library (iOS still reads the Files-visible Documents folder) and a device
  build; both wait for a machine with the NDK, Xcode and the iOS Photos framework. The link itself is
  now measured, paired and tested — nothing on it is blind any more.

## 2026-11-24 (sixty-eighth) — the phone sends its photographs (SL-J04/J05 from the app)

- **Why this came up.** The sixty-seventh entry ended on the one thing the phone's own screen still
  could not do: the app could index a camera roll and paint a phone's canvas, but nothing on it could
  hand a photograph to the desktop. The wire was already there (`sortlens_remote::client` and its
  loopback harness), the desktop had its Sync screen, and the roadmap's "Still open" named exactly
  "a send-to-desktop action over the resumable upload". A phone that indexes a camera roll but cannot get
  a photograph off it is still not a remote.
- **The queue is one thread the interface polls.** `sortlens_remote::send::SendQueue` spawns one worker
  (`sortlens-send`) that health-checks once, reads each file, uploads it with `Client::upload`, and publishes
  a `SendOutcome`: sent, kept and removed counts, per-file failures, cancelled, and the one connection error
  that ends a run instead of failing every remaining item after its own timeout. `report()` is what a frame
  paints; `cancel()` asks it to stop between photographs, leaving a half-sent chunk as a server-side partial
  the next run resumes; `take_outcome()` hands the result over once. A thread that cannot start finishes the
  run at once with the reason, rather than a queue that claims to be running forever with nothing behind it.
- **The ack's remove is named, not guessed.** `SendOutcome::to_remove` lists the device ids whose ack said
  `LocalCopy::Remove`. The queue deliberately does not delete: it holds a path, not the library that can audit
  a file. On the phone the ids go to `Engine::delete_photos(..., Trash)` — the same audited path the desktop
  applies a cull with — which re-checks each file's size and mtime, refuses the batch if one moved, flags the
  photographs rejects, keeps the undo batch, and moves them to the phone's trash. "Remove" therefore means
  what the checkbox says, and an unreachable desktop or a refused batch leaves the file where it is.
- **The link keeps the address its token belongs to.** `apps/sortlens-mobile/src/send.rs`: `DesktopLink` holds
  the address, token, recorded device and the run. `pair` runs the handshake on its own thread; `poll_pair`
  peeks at the answer slot and takes the attempt only once an answer is there — the first frame after the tap
  usually polls before the worker has connected, and an eager `take` drops the handle and loses the
  answer forever. A successful answer writes the typed host and port together with the token, so the settings
  document can never hold a token pointing at `:0`; a failed one leaves the previous pairing untouched.
  Forget cancels a run in flight.
- **The phone's and the desktop's sections never share a screen.** `send_section` had been painted above the
  desktop's Sync & devices section on a phone, whose host answers every server action with "not available on
  the phone yet". Sync is now desktop-only and Send is phone-only. Forget is disabled while a run is in
  flight, because dropping the token mid-run would leave the worker sending under a pairing the phone no
  longer keeps. The hint says what Remove actually does, and the sentence a finished run earns says what the
  ack cleared (`… 1 to remove`), with the app appending what it could carry out (`; removed 1 from this
  phone`, or the reason it could not).
- **Files.** `crates/remote/src/send.rs` (new); `apps/sortlens-mobile/src/send.rs` (new), `Cargo.toml`
  (`sortlens-cull`), `src/lib.rs`, `src/prefs.rs`; `crates/ui/src/shell.rs`, `crates/ui/src/lib.rs`,
  `i18n/{ar,ja}.tsv`; `docs/ROADMAP.md`, `docs/UX.md`, `docs/ARCHITECTURE.md`, `docs/FEATURES.md`,
  `docs/RELEASE.md`.
- **Tests that fail when the behaviour is absent** (14 new: five in `sortlens_remote::send`, seven in the
  phone's link, one in the phone's settings, one in the shell). Remote, over a real-TCP fake upload server:
  both keep and remove land with `to_remove == [2]`, a missing file fails alone, progress counts items as
  they finish, cancel stops between
  items, and an unreachable desktop ends the run with one reason. Phone: a successful `/pair` through a real
  socket keeps the address with the token (the old poll loses the attempt); a closed connection stores
  nothing; the ack's remove choice reaches the library through `delete_photos` while a keep-only run touches
  nothing; the items come from the catalog, missing files skipped; an older settings file opens unpaired.
  Shell at 390 points: Pair is disabled until all three fields are full, Pair and Send hand over the typed
  values and the selection with the toggle's answer, Send refuses an empty selection, Stop hands over a
  cancel, Forget answers nothing during a run, and each width paints only its own section — the phone no
  Sync & devices, the desktop no Send to desktop.
- **Verified.** `cargo test -p sortlens-remote` (36 passed), `-p sortlens-mobile` (17), `-p sortlens-ui --lib`
  (163). `cargo xtask ci` green through every gate; the first run failed clippy on a `needless_borrow` in a
  test and on the desktop host's non-exhaustive `Action` match (both fixed here), and the shell test then
  caught its own assertion matching the hint's "Sync & devices" wording, so it now compares section headings
  exactly. `cargo xtask snapshot` green. `cargo xtask screenshots` green with 23 scenes and no baseline
  moved. `cargo xtask deny` green. `cargo xtask mobile` still needs the NDK and Xcode, which this checkout
  does not have.
- **Reconciled.** ROADMAP 3b gained the shipped bullet and its "Still open" no longer names send-to-desktop.
  UX §2 has the Send rules and no longer lists it among the lacks. ARCHITECTURE §10 says the phone drives
  `sortlens_remote::send` and carries removals through the audited delete path. FEATURES SL-J02 dropped
  "uploading from the app" from Still open; SL-J04 and SL-J05 name the app half. RELEASE says the phone can
  use the remote half and describes the phone's canvas as its own.
- **Next.** The iOS Photos library (iOS still reads the Files-visible Documents folder) and a device build; on
  the link itself, a connection test (`/health`) before the code is typed.
## 2026-11-24 (sixty-seventh) — the canvas is the phone's (SL-J10, Phase 4 item 3b)

- **Why this came up.** The sixty-sixth entry gave the phone a frame — docks put away, a wordless bar — and
  named what was still the desktop's: the canvas header, the filter strip and the bottom bar's tray. Measured
  at 390×780 with a populated library, three numbers said how far off it was: the far-edge rail painted four
  glyphs at x 364–378 beside a *closed* inspector, so a tenth of the screen went to a tool the bar's own
  inspector glyph offers; the filter strip wrapped into three rows because its reserved count column
  ("6 shown", the same number the header already prints) kept the chips' room down to 297 points; and the
  bottom bar's pill ran to x 402 — the last flag button half off the edge and untappable.
- **The rail stands down.** `right_tools` is painted on a phone only while the inspector's sheet is open: the
  closed panel no longer pays its 30 points, and the open sheet keeps the rail as its tab switcher — which
  also settles the earlier "close-then-reopen" note in the roadmap: the tap test drives a click on the rail
  beside the open sheet and switches Details → Keywords without closing it. `left_sheet_width` stops
  subtracting `TAB_STRIP_W`, so the sidebar sheet is the width of the screen (the rail belongs to the
  inspector, and the two docks never share a screen).
- **The strip is two rows.** On a phone the strip keeps no count (the header's is the same number on the same
  screen), its chips sit 4 points apart, and its card margin is 6: at 390 the eight chips plus `Clear` wrap
  into two rows. The canvas's own gutter narrows from 14 to 10 points. The strip's card now ends at y 150 —
  the chrome before the first photograph, against ~190 before.
- **The tray fits.** The pill's buttons get 2-point padding and 3-point gaps on a phone, the keyword count
  steps aside (the inspector's Tags tab is one tap away), and the density slider yields to the header's own
  glyph — the same control, so the bar cannot disagree with the header. Everything the tray paints is inside
  the screen.
- **Files.** `crates/ui/src/shell.rs`: the `right_tools` call condition in `show`, `left_sheet_width`, the
  canvas gutter, the strip's narrow branch (no count, tighter chips and margins), the pill's narrow branch
  (spacing, padding, no keyword count), and the bottom bar's slider condition.
  `crates/ui/tests/snapshots/phone-charcoal.png` and `phone-sidebar-charcoal.png` regenerated; no other
  baseline moved.
- **Tests that fail when the behaviour is absent** (3 new, 162 in the crate): a populated 390-point canvas
  proves the rail's column empty, the strip's card at its full width ending by y 152 with exactly two chip
  rows and no second count, every bottom-bar picture and text inside the screen (the old pill fails by
  thirty points), and the header intact — name, count, sort combo, both grid glyphs; a 900-point window
  keeps its rail (one glyph per tool), its strip summary and its pill; the rail beside the open sheet
  switches the inspector's tab. The strip-summary sweep now mirrors the shell: absent under 600 points,
  measured and collision-free above it.
- **Verified.** `cargo xtask ci` green through every gate. `cargo xtask snapshot` green, no golden moved.
  `cargo xtask deny` green. `cargo xtask screenshots` green with 23 scenes: the two phone baselines
  regenerated and read — three photo columns where there were two, a two-row strip, the tray inside the
  screen, the sidebar sheet the width of the screen — and the other 21 byte-identical, the check that no
  width from 900 up moved. There is no `cargo xtask wasm` in this workspace; the portability gate is
  `cargo xtask mobile`, still for a machine with the NDK and Xcode.
- **Reconciled.** ROADMAP 3b gained the shipped bullet and its "Still open" now names only what a phone
  still lacks (device build, font trim, iOS Photos, send-to-desktop, packaging). UX §2 has the phone-canvas
  rules. ARCHITECTURE §10 says the canvas is the phone's too. RELEASE's phone bullet no longer says the
  phone paints the desktop's layout. FEATURES SL-J02 moved the canvas out of "still open".
- **Next.** Send-to-desktop (SL-J04/J05 from the app): a share-or-send action in the phone app that queues
  the device's photographs to the paired desktop over the resumable `/upload` with keep-or-remove, surfaced
  in the app's own UI.

## 2026-11-24 (sixty-sixth) — the phone gets its own frame (SL-J10, Phase 4 item 3b)

- **Why this came up.** Yesterday's entry ended on the honest gap: the phone app paints the desktop shell, and
  the roadmap said the shell "demanded 880 px". The number was in three documents and in no constant. Measured,
  a 390-point window painted a 264-point sidebar, the 30-point rail and a 296-point inspector, so the canvas —
  the grid, the reason the app exists — was given a negative width, while the bar ran eight glyphs and a
  240-point field off the right edge. A phone saw the desktop squeezed and nothing to look at.
- **The measurement.** `NARROW_BELOW = 600.0` in `crates/ui/src/shell.rs`, read once per frame before anything
  paints into `UiState::narrow`, so the bar, the docks and the canvas answer the same number on the same frame.
  600 is wider than every phone in portrait (390–440 points) and narrower than every window the committed
  baselines record (900 and up) — which is why none of them moved.
- **What a phone frame is.** As a window becomes that narrow the docks are put away, so a window that opens on a
  phone opens on the grid; afterwards a person's taps win over the measurement (`toggle_left`, `toggle_right`,
  `open_right`). The bar keeps the mark and gives up its two words; the search field takes the room the five
  remaining glyphs leave, floors at 140 points and is left out below that; its hint stops naming `Ctrl+K`,
  because a phone has none; the keyboard overlay and the appearance popover step aside (one is about a keyboard,
  both live in Settings); the read-only warning becomes the same sentence in a glyph's hover. A dock that opens
  is a sheet the width of the screen, the canvas is not painted behind it, and the two docks are one-at-a-time
  by construction — a sheet needs no close button because the bar's glyph is the way back.
- **Files.** `crates/ui/src/shell.rs`: `NARROW_BELOW`, `SEARCH_FIELD_NARROW`, `UiState::narrow` with the three
  dock helpers, the narrow branches of `top_bar`/`left_panel`/`right_panel`, `left_sheet_width`, and
  `search_field` taking the room it may use. `crates/ui/tests/screenshots.rs`: the two phone scenes.
  `crates/ui/tests/snapshots/phone-charcoal.png` and `phone-sidebar-charcoal.png`: the new baselines.
- **Tests that fail when the behaviour is absent** (3 new, 159 in the crate): a 390-point frame opens with both
  docks put away, no "Sortlens"/"Local library" words in the bar, the Welcome card painted — and every glyph and
  text rect of the top bar inside the screen, which the old bar failed by roughly 300 points; each dock opens and
  closes from its own glyph, one at a time, the sidebar painting its sections and the inspector its Details tab;
  a 900-point window keeps both docks and both words.
- **Verified.** `cargo xtask ci` green through every gate (fmt, clippy, workspace tests, layers, roots, assets,
  fonts, version, attribution, a11y, docs, templates, workflows). `cargo xtask snapshot` green with no golden
  moved. `cargo xtask deny` green (advisories, bans, licences, sources). `cargo xtask screenshots` green with 23
  scenes: the two phone scenes are new, and the 21 earlier baselines are byte-identical, which is the check that
  nothing at 900 points or wider moved. The two new images were read: the phone frame is a bar with a field and
  five glyphs over the grid, and the sidebar sheet covers the screen with the rail still beside it. There is no
  `cargo xtask wasm` in this workspace — the string does not occur anywhere under `xtask` — so the wasm gate the
  plan names has no task here; the portability gate for this change is `cargo xtask mobile`, still for a machine
  with the NDK and Xcode.
- **Reconciled.** ROADMAP 3b gained the shipped bullet, and its "Still open" now names what a phone still lacks
  (the canvas header, filter strip and bottom-bar tray at that width; switching the inspector's tab while its
  sheet is open; the font trim; the iOS Photos library; send-to-desktop; packaging) instead of claiming the shell
  is "still the desktop's, 880 px minimum". UX §2 has the phone-width rules. ARCHITECTURE §10 says the shell
  answers a phone's width itself, so the phone app inherits it. RELEASE's phone bullet says the frame is there and
  the canvas inside it is not. FEATURES SL-J02 moved the phone layout out of "still open" and kept the canvas.
- **Next.** The canvas at a phone's width: its header (name, count, sort combo, grid glyphs), the filter strip
  (two rows of chips, about 200 points before the first photograph at 390) and the bottom bar's control tray. A
  frame that shows a grid but spends its first quarter on desktop chrome is half a phone layout.

## 2026-11-24 (sixty-fifth) — the phone remembers what you chose (SL-J10, Phase 4 item 3b)

- **Why this came up.** The phone's `App` was handed a `data_dir` and spent it on the catalog and the
  thumbnail cache only. Every launch opened Charcoal at the shell's default density, and the Appearance
  menu's taps went nowhere: `Action::SetDensity`, `SetGrid` and `SetMotion` shared an empty arm with
  the actions a phone genuinely cannot do. A phone app that forgets the theme you picked is not a phone app.
- **The read.** The document already exists on the desktop (`apps/sortlens/src/settings.rs`): two sections,
  `[appearance]` with the four ids and `[library]` with the order, and `craft-settings` owns the hard
  parts — atomic save, backup, damaged-file recovery, unknown-key preservation. None of that is
  desktop-shaped; only the *location* is. `Location::Auto` wants a platform config folder a phone does not
  have, so the phone names its own directory and inherits every rule: `config.toml` in the app's private
  data directory, beside `Library/` and `Cache/`, never inside them.
- **The document.** `apps/sortlens-mobile/src/prefs.rs`: `PhonePrefs` — the desktop's two sections cut to
  the five choices the phone's shell can make (theme, density, layout, motion, order), with the same ids and
  the same defaults. `open` is `SettingsStore::open_at(data_dir)`; `notice` repeats the desktop's
  sentence only when the load report has something to say; `apply` reads each id the way the desktop reads
  it, an unknown one landing on its default; `record` is the one place an `Action` becomes a stored id,
  kept out of `App` so the mapping is testable without a window.
- **The application.** `App` owns the store: it opens before the first frame, so that frame is already the
  saved theme and order; every appearance or order action is written before the frame shows it; `tick` fires
  the debounced save each frame and schedules a repaint for its deadline (the desktop's own rule, and the
  reason an OS that ends the process mid-session loses nothing already tapped); a clean exit flushes.
  `SetDensity`, `SetGrid` and `SetMotion` now change the shell instead of being ignored.
- **Tests that fail when the behaviour is absent** (7 new, 10 in the crate): the defaults are the desktop's
  own and land on the shell's own defaults; every id the shell offers — 5 themes, 3 densities, 2 layouts, 2
  motions and 7 orders — round-trips through the real file one variant at a time; a test drives
  `Action::SetDensity(Compact)`, `SetGrid(Square)`, `SetMotion(Reduced)`, `SetSort(Newest)` and
  `SetTheme(Paper)` through `record`, flushes, reopens and reads the shell back, while `Action::Notice`
  and `Action::PickFolder` must write nothing (a missing arm fails); an unknown id is not a damaged file; an
  `[experimental]` key written by a later build survives a save this build makes; a damaged file is moved
  aside and `notice` says so while a clean file says nothing; the file sits outside the library directory.
- **Verified.** `cargo test -p sortlens-mobile` is 10 passed. `cargo xtask ci` green through every gate
  (fmt, clippy, workspace tests, layers, roots, assets, fonts, version, attribution, a11y, docs, templates,
  workflows); `cargo xtask snapshot` green with no golden moved; `cargo xtask deny` green. No paint
  changed, so no baseline was regenerated and `cargo xtask screenshots` was not needed. The new dependency is
  the only part that could have broken the phone targets this checkout cannot link:
  `cargo check -p craft-settings --target aarch64-linux-android` and `--target aarch64-apple-ios` both
  finish clean (dirs has an explicit path for each, and the phone passes an explicit directory anyway), so the
  settings crate is portable; `cargo xtask mobile` itself is still for a machine with the NDK and Xcode.
- **Reconciled.** ROADMAP 3b moved settings persistence out of "Still open" into a shipped bullet and left the
  rest of the row open; ARCHITECTURE §10 says where the phone's document lives and which rules it keeps;
  RELEASE's "No remote" bullet was stale — the server, pairing and the phone app shipped yesterday — and now
  says what exists and that nothing is packaged for a phone; its "prototype-grade layout / Phase 3" bullet,
  which still called Phase 3 ahead, now names the one layout gap that is real: the phone paints the desktop
  shell.
- **Next.** Phase 4 item 3b's remainder, in order of value here: a phone layout (the shell still demands
  880 px, which no phone has), a send-to-desktop action over the resumable upload (the client is built and
  tested; the phone needs a paired address and a button), trimming the embedded font set for a phone binary,
  the iOS Photos library, and an APK/AAB plus an `.ipa` in the release workflow.

## 2026-11-24 (sixty-fourth) — the shell taught the wrong key, and every dialog opened in the corner

- **Why this came up.** The sweep stayed on the top bar and then opened what it had not seen live. The
  search field's hint said `Search photos or ⌘K` on Windows, where the key is Ctrl+K; under a 10× crop the
  ⌘ was painted over the K rather than before it, because the glyph is egui's own and its advance does not
  match the sample's. The palette opened cleanly (grouped rows, filtering, the shortcut column) — and
  About opened half off the canvas: its rounded corner cut at the screen edge, its title bar under the top
  bar, the paragraph's first letter clipped. Settings was in the same corner, title bar under the bar.
- **The read.** egui stacks a window it has never seen at the available area's top-left, and nothing in the
  shell had ever said otherwise: the Alerts popover is the only window with a position of its own, and it
  uses `anchor(RIGHT_TOP)` deliberately. The About box was simply the first dialog the audit opened, so it
  was the one that showed the corner to be wrong.
- **The paint.** The hint's key now comes from `palette_hint()`: `Ctrl+K` off a Mac, `⌘K` on one, so the
  words teach the press the platform actually runs and Windows never draws the ⌘ over its letter. The
  field keeps measuring itself from the hint and gives the words up in a stated order (key, then `or …`,
  then `photos`) — on Windows that costs a few points, which is why the field is 3×700 wide in the scenes
  and every baseline moved by the same 105 pixels. Every dialog now goes through `dialog_window()`, which
  anchors it `CENTER_CENTER`: the window measures itself and is centred exactly, at 1440×900 and at
  900×620 alike, and stays centred. The cost is that a dialog cannot be dragged aside; for a surface that
  closes on Escape or its own button that is no loss, and it is stated in `docs/UX.md`.
- **One more, on the next screen.** Insights draws its bars with the count right-aligned at the row's
  end, which is *inside* the track: a full bar — all images, a rating everyone shares, `Images 7` —
  carried its number on the accent colour in `text_faint`, the worst pairing the palette has. The track
  now stops 26 points short and the number keeps that room, so every bar row (ratings, labels, kinds,
  the breakdowns) reads its count against the background.
- **Tests.** `cargo test -p sortlens-ui --lib` is 156 passed; `the_search_hint_shrinks_instead_of_being_cut`
  now expects the Windows spelling in the roomy case. `cargo xtask screenshots` is 21 scenes regenerated
  and green — the twenty-one were all touched by the field's width, the four dialog scenes (`shortcuts`,
  `settings-short`, `settings-sync`, `shortcut-editor`) now paint centred, and `insights-*` shows every
  count clear of its bar. `cargo xtask ci` is green, and `cargo xtask wasm` was green too, hours before the
  browser page and its task were removed.
  One test changed shape: `clicking_the_detail_photograph_toggles_its_zoom` now waits for the full-size
  decode, up to ten seconds, instead of counting three frames. On a loaded box the decode lost that
  race and the view was still painting its thumbnail, so the gate would have called a slow machine a
  defect. The wait is bounded and the assertion is unchanged.
- **Verified live, after the rebuild.** At 1280×800 the field reads `Search photos or Ctrl+K` whole, with
  no overlap; Ctrl+K opens the palette under the field, `exp` narrows it to the two Export rows and Enter
  runs the first; About opens centred with its whole paragraph, and the palette and About are both clear
  of the dark scroll bars that the corner placement had hidden them under.

## 2026-11-24 (sixty-third) — the chrome that only holds at one size: a toast with nowhere to sit, a footer cut by the bar at every window, and a compare's names running under its buttons

- **Why this came up.** The live sweep that produced the last checkpoint kept reading 1280×800, then 900×620, and
  found three more places where the layout was exact rather than measured: a transient message painted
  through the annotation pill, the rows under the Detail photograph overflowed the bottom bar at every
  size — including the 1440×900 baselines — and a compare's two file names plus its "2 of 7" ran under
  the button cluster, which painted the position as a stray "2" between the names.
- **The read.** (1) At 1280 the bottom bar is full end to end — tabs 0–707, the annotation pill 726–1102,
  the right cluster 1104–1270 — so a message had about 54 points of its own. (2) The footer rows (facts,
  Previous/Next, keywords, suggestions, histogram with lookahead, the key hint) were budgeted at a
  guessed 118 points; the real rows are always taller, so the photograph took their room and the bar cut
  the hint off. (3) `Sides` with `truncate` bounds left-to-right, not piece by piece: the first name spent
  the whole row on itself and pushed its siblings past the block, still painted because the clip belongs
  to the parent.
- **The paint.** The notice is now an `Area` centred above the bar (`Order::Middle`, no click taken, one
  line, the full text when it fits), which is also clear of the Details dock a corner toast would sit on.
  The detail view measures its footer after the rows paint (`UiState::detail_footer`, the same one-frame
  lag `settings_height` already pays) and lets the photograph take exactly what is left; when that
  measurement changes it asks for a repaint, because the app goes idle after a view switch and would
  otherwise keep the fallback layout until the next input. A short window (under 700 points) drops the two
  decorative rows — suggested keywords and the histogram strip — so the hint row still lands above the bar;
  the test is the window, not the measured footer, which would flip the layout back and forth as the
  measurement followed the decision. The header sets each name's share before laying it out, with the
  arrow, the position and the item gaps taken out of the budget first, and degrades in a stated order:
  below 720 points the conveniences the palette also offers (Open, Show in folder, Export…) go, below 420
  the zoom toggle and Swap go, and below that the names take a row of their own with the buttons under
  them.
- **Tests.** `cargo test -p sortlens-ui --lib` is 155 passed. New claims: the notice floats above the bar
  rather than through the selection count; the detail view keeps its last row above the bar at 1440×900,
  1280×800 and 1100×700 (the old 118-point budget panics "the view paints no key hint at all"); and
  `a_compares_names_stop_where_its_buttons_start` walks 1440, 1280, 1100, 980 and 900, asserting that every
  text left of the button cluster ends before it starts, that the conveniences appear at 1440 and are gone
  below, that the position survives at 1280, and that under 1100 the names get a row of their own wider
  than a letter. `cargo xtask screenshots` is 21 scenes, green; the four detail/compare baselines were
  regenerated and now show the whole footer. `cargo xtask ci` and `cargo xtask wasm` are green.
- **Verified live, after the rebuild.** At 1280×800 a rating notice appears centred above the bar clear of
  the pill; Detail shows its names, `Compare C`, `1:1 Z`, `Close Esc` and the whole footer; Compare shows
  `IMG_0004.png ↔ IMG_0005.png 1 of 7` with no overlap. At 900×620 the bar collapses to its destination
  menu (nine items, caret included), Compare puts the names on their own row above two buttons, and the
  compact footer ends with the key hint above the bar instead of under it.

## 2026-11-24 (sixty-second) — the sweep found what a static baseline hides: bars you could not see, insights columns that did not fit, a Settings dialog taller than the window, and Esc that only sometimes closed it

- **Why this came up.** The suite has baselines for every view, but a baseline proves a frame paints, not
  that it can be read or reached. This pass opened the live app over the QA library at the size a laptop
  actually gives it (1280×800) and read each view and dialog as a person would: For You, Insights,
  Soft Focus, Compare, Places, the Settings and Recently Deleted dialogs, and the keyboard.
- **The read.** Three defects a screenshot at 1440×900 could not state. (1) The Insights bars were
  `#3A311E` on `#151515` — 1.4:1, effectively invisible: `columns()` filled with `t.selection`, the
  selected-row tint, instead of the data colour. (2) The For You year labels collided once the filter row
  reached four digits, because every column was a fixed 16 points wide. (3) Settings is ~1265 points
  tall, so at 800 its tail — Updates, Sync & devices, the keyword packs — ran past the bottom of the
  window with no scrollbar and no way to reach it. Plus two smaller ones: the receipt dialog stayed on
  screen after its own close, and Recently Deleted's empty state said "Nothing has been applied yet".
- **The paint.** Charts grew `bar_fill`: the accent at full strength for the highlighted column and an
  85/15 accent-on-panel blend otherwise, so every preset clears 3:1 against the surface (Silver was the
  tight case at 0.75 before the blend and clears it after); `share_row` uses the full accent, which clears
  3:1 on every preset's `thumb_bg`. Columns widen to their label (`column_width` measures the galley), and
  the label colour follows the highlight (`t.text` / `t.text_faint`). Insights lays its three columns out
  with one `horizontal_wrapped` row instead of a fixed 3× layout that overflowed the canvas at 1280.
  Settings now measures its own body on the frame that paints it (`UiState::settings_height`, read from
  the scroll area's `content_size` and not from the scrolled `ui.min_rect()`, which reports the viewport
  and would flip the ceiling on and off every frame) and, past a 96-point screen ceiling, caps the window
  and scrolls the body with the bar always visible. A screen tall enough for the whole dialog keeps the
  auto-sized window, so the 1700-point Settings baseline is untouched. Escape dismisses the front-most
  window through `dismiss_top_window`, one press one window, in the reverse of the paint order.
- **Tests.** `cargo test -p sortlens-ui --lib` is 152 passed, including new claims: the four-digit label
  widens its column instead of painting over its neighbour, chart bars hold 3:1 against their surface on
  all five presets, the Insights columns fit between the docks at any width the window can have, Escape
  unwinds the dialogs one press at a time, and the settings section tests (AI, Updates, Sync, language)
  read their sections on a screen tall enough to hold them — the short case is the new baseline.
  `a_short_window_scrolls_the_settings_dialog_in_both_themes` shoots Settings at 900×620 in both themes
  (lenient, for the language row's own scripts); `cargo xtask screenshots` is 21 scenes, green, and only
  the two new `settings-short-*` files plus the four chart baselines changed.
- **Verified live, after the rebuild.** At 1280×800 Settings opens capped, the wheel scrolls it to
  Updates → Sync & devices → Keyword packs with the bar in sight, Esc closes it, and Recently Deleted
  reads "Nothing has been deleted yet." with its cull-batch note.

## 2026-11-24 (sixty-first) — the wasm shims are gone: no target, no gate, no dead branch

- **Why.** With no wasm target installed, no task that builds one and a written non-goal, every
  `cfg(target_arch = "wasm32")` branch and every doc comment about "the wasm client" was dead weight
  that read as an invitation. A reader should not have to work out whether a browser path still runs.
- **What changed.** The stubs, the paired `cfg(not(...))` attributes and the browser-only comments are
  gone from core, catalog, cull, meta, thumbs, ai, engine, ui and remote; three manifests'
  `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` sections became ordinary `[dependencies]`
  (the same crates, the same builds). The layer check now states the exception it always had: the
  engine takes `craft-update` with default features — no `egui` — for the update check.
- **Verified.** `cargo xtask ci`, `snapshot` and `deny` green; no source file contains `wasm`.

## 2026-11-24 (sixtieth) — the intake the agent wrote stays; the browser page it also wrote does not

- **Why.** Phase 5's intake and playbook landed on `main` while this branch was removing the web
  application and adding the phone app. Both slips are wanted; the browser page is not.
- **Kept whole.** The issue forms and PR template, `docs/AUTONOMOUS_AGENT.md`, `cargo xtask intake`,
  `cargo xtask workflows`, `xtask/src/yaml.rs`, both workflows and their tests. The one change the
  merge needed: the fifth gate is `mobile`, not the removed wasm task — in `xtask/src/workflow.rs`,
  `ci.yml`, the PR template, the playbook and the Phase 5 roadmap row.
- **Dropped.** The page's upload surface and the files it lived in, the browser transport, the host's
  page route and its flag, and the harness test that read them. The one-shot upload regression the
  page found stays, with its test, because the defect was the server's.
- **Verified.** `cargo xtask ci`, `snapshot`, `deny` and `workflows` after the merge, on this checkout.

## 2026-11-24 (fifty-ninth) — Phase 5 closed: the playbook is enforced, the intake reads a stranger's report without obeying it, and the fifth gate runs on every PR

- **Why this came up.** Items 2 and 3 of Phase 5 were the last unchecked rows in the completion
  plan. They ship as one slice because the repository forbids docs-only commits after Phase 0: a
  playbook with no workflow is prose nobody checks, and a workflow with no written contract is a
  gate nobody can read. Every rule below exists so that the promise and its enforcement land
  together. The parent rows that had been left unchecked while their children shipped (M8 item 3,
  M10 item 2) were reconciled in the same commit, because otherwise Phases 0–5 could never read as
  closed.

- **The playbook** (`docs/AUTONOMOUS_AGENT.md`, 145 lines) is the narrow operating contract for an
  agent that reads issues and opens pull requests unattended. It draws the boundary first: a report
  is untrusted data, never instructions — no command, install, link or file is obeyed because an
  issue asks, and the reason the intake passes the body through the environment is written down as
  the same rule. Only an `agent-ready` issue is picked up; `triaged` is a statement about attention
  (the forms apply it themselves, the intake removes it when structure is missing), not pickup
  permission, and a `needs-info` issue is never implemented. The never-list is explicit: never push
  to `main`, never auto-merge or self-approve, never bypass `cargo xtask deny` or another gate,
  never read or pass a signing or release secret, never publish, force-push or spend. The
  repro-first loop is ordered and finite — read the evidence, reproduce and write the failing test
  that is the proof it bites, minimal fix, reconcile the documents in the same commit, run the
  gates, open a **Draft pull request**, stop. The improvement queue is bounded (`sortlens-perf`, then
  `enhancement-approved`, then unchecked roadmap rows, one slice per PR, empty queue means stop),
  and the stop rules say when trying again would be noise: a red tree not understood, an external
  secret, or two runs shipping nothing.

- **The intake checker** (`xtask/src/intake.rs`, +178 lines): `cargo xtask intake` reads
  `SORTLENS_ISSUE_BODY` and `SORTLENS_ISSUE_LABELS` from the environment, decides from the labels
  which form the report came through (a report that is neither a bug nor a feature is not judged),
  and reads the required headings **from the form itself** rather than from a second copy that
  would drift. Each missing heading and each empty section is named; `_No response_` counts as
  empty, and `Diagnostics` must still contain `sortlens`, so a sentence is not a bundle. Exit 1
  prints the comment body the workflow posts. Six new tests cover the reading of headings from the
  form, a complete report, a missing section, an empty section, a diagnostics sentence, and an
  unlabelled issue.

- **The workflow gate** (`xtask/src/workflow.rs`, new, 368 lines): `cargo xtask workflows`, wired
  into `cargo xtask ci`, reads both workflows with the same small YAML-subset reader `templates`
  uses. It refuses a `ci.yml` with no `pull_request` trigger, a missing one of the five gate
  commands (`ci`, `snapshot`, `deny`, `mobile`, `screenshots`), a `continue-on-error` step, or an `if`
  that would leave pull requests out. It refuses an intake that does not run on opened/edited/labeled
  issues, that uses `pull_request_target`, that reaches a `secrets.` value, that cannot write issues,
  that does not stay off pull request events, that does not pass both environment variables, or that
  lost the comment / `--remove-label triaged` / `--add-label needs-info` bookkeeping. And it holds
  the playbook to ten load-bearing phrases, so a rule can be rewritten but not removed. Ten tests,
  each proved by mutating a fixture (a removed gate, a weakened step, a skipped job, a lost
  trigger, a secret, a dropped label command, a deleted rule) until the checker bites.

- **`ci.yml` was rewritten in block style** (the reader refuses flow lists, so `branches: [main]`
  and the `os:` flow list became block lists) and gained the fifth job: `screenshots` on ubuntu with
  Mesa's software Vulkan (`mesa-vulkan-drivers`, `WGPU_BACKEND=vulkan`, `LIBGL_ALWAYS_SOFTWARE=1`),
  so the 33 painted baselines are a real pull-request gate rather than a local hope. The existing
  jobs, cache keys and the per-platform conditions are unchanged.

- **`agent-intake.yml`** triggers on issues only, holds `contents: read` and `issues: write` and
  nothing else, has no path to a repository secret, and refuses pull request events. It runs
  `cargo xtask intake` and posts the returned body as a comment; an incomplete report loses
  `triaged` and gains `needs-info`, and a complete edit swaps them back. If the checker fails
  before writing anything (a build failure), the job exits red without touching the issue, so a
  broken toolchain cannot mislabel a stranger's report. The five label names the playbook and the
  forms use (`triaged`, `needs-info`, `agent-ready`, `sortlens-perf`, `enhancement-approved`) were
  created on the repository with `gh label create --force`; the workflow recreates the two it swaps
  on every run, so a fresh repository or a renamed label cannot silently drop the bookkeeping.

- **Verification.** `. scripts/env.sh && cargo xtask workflows` → `xtask workflows: ok`.
  `cargo test -p xtask` → 34 passed. A body containing only `### Sortlens version` produced exit 1
  naming the empty section and six missing ones; a bare enhancement body named its three missing
  feature fields. Both workflow files were parsed independently with PyYAML, which agrees with the
  project reader on keys, triggers and jobs (PyYAML reads `on` as `True`, which is the YAML 1.1
  quirk, not a difference in the file). `cargo xtask ci`, `snapshot` and `deny` green below.
  Not verified: the workflows have never run on GitHub — this commit is their first run — and the
  ubuntu `screenshots` job is expected to spell chords in ASCII like Windows, which cannot be
  observed from either local checkout (macOS paints `⌘Shift+R`; the Mac gate remains red for the
  reason in the fifty-sixth entry).

- **Documents reconciled in the same commit.** `README.md` (status prose no longer says core
  completion is in progress; the document table names the playbook), `docs/PLAN.md` §12 (the five
  phases done; what remains is the external table, the visible backlog and the two M9 rows),
  `docs/ROADMAP.md` (Phase 5 items 2 and 3 ticked with shipped notes; M8 item 3 and M10 item 2
  ticked with the signing blocker left in its own table; a status-log row), `CONTRIBUTING.md` (the
  `ci` line and `screenshots`), `docs/TOOLING.md` (the intake paragraph extended to the workflows
  and the playbook).

- **The plan is closed.** Every row under "Completion phases" is ticked except the native phone app
  (Phase 4 row 3b), which is tracked and open until a machine with the NDK and Xcode builds it. What remains is named
  where it was already named and nothing buildable was moved: the M10 "Deliberately absent" table
  (Apple identity, installer toolchains, a screen-reader session, a chosen palette, the owner's
  tag), the post-1.0 backlog, and the two M9 rows (more i18n catalogues with RTL screenshots,
  localised numbers and dates). The scheduled continuation task is left disabled and the run ends
  with the completion sentence.
## 2026-11-24 (fifty-eighth) — Phase 5 item 1: the GitHub intake is written, and `cargo xtask templates` refuses what GitHub would refuse

- **What shipped.** The three intake files Phase 5 item 19 named, and nothing else moved: the two issue
  forms GitHub renders, the pull-request checklist a contributor reads, and a gate that keeps both
  honest. `.github/` had workflows only; it now has `ISSUE_TEMPLATE/` and `PULL_REQUEST_TEMPLATE.md`.
- **The bug form** (`.github/ISSUE_TEMPLATE/bug_report.yml`): labels `bug` and `triaged`, the title
  prefix `[Bug]: `, a markdown intro that says why the bundle is safe to attach, and seven required
  fields — version, OS (a seven-option dropdown, Apple silicon through to "another platform"), OS
  version and display setup, numbered repro steps, expected, actual, and the `sortlens-cli diagnostics`
  textarea — plus one optional "anything else". The diagnostics field is the one the gate checks by
  name: GitHub can render a bug form without it, and a bug report without the bundle is a round trip.
- **The feature form** (`.github/ISSUE_TEMPLATE/feature_request.yml`): labels `enhancement` and
  `triaged`, title prefix `[Feature]: `, the command-first framing in the intro, required problem,
  proposal and area (ten options that follow the crates and surfaces), two "before opening"
  checkboxes, optional workarounds and extras.
- **The pull-request template** (`.github/PULL_REQUEST_TEMPLATE.md`): the five gates as checkboxes —
  `ci`, `snapshot`, `deny`, `mobile`, and `screenshots` with when it applies — the failing-then-passing
  test prompt (`Test:` and "it fails on `HEAD~` because"), `Closes #`, the scan/catalog/thumbs
  `cargo xtask perf` before-and-after table, and the `crates/ui` before-and-after screenshot table.
- **The gate** (`cargo xtask templates`, in `ci`): `xtask/src/yaml.rs` reads exactly the subset the
  forms use — block mappings, block sequences, plain and quoted scalars, `|` blocks — and refuses flow
  collections, folded scalars, anchors, tags, duplicate keys and tab indentation *with the line that
  caused them*; `xtask/src/intake.rs` then checks the shape GitHub renders: the keys a form carries,
  the six element types, an id on every field and no id on a markdown note, unique lowercase ids, a
  label wherever a person types or picks, options (and their own allow-list) for dropdowns and
  checkboxes, `validations` with only `required`, at least one required field, that the bug form's
  diagnostics field is a required textarea whose description names `sortlens-cli diagnostics`, and the
  GitHub length limits on `name` and `description`. The pull-request template is checked for the
  markers a reviewer needs, because a checklist that lost its `deny` box is a promise that already
  drifted. Both run from the repository root derived from `CARGO_MANIFEST_DIR`, so the same code works
  from `cargo xtask` and from `cargo test`.
- **Against a real parser.** The two shipped forms were also read once with PyYAML as an independent
  check, and it agreed on the top-level keys and the field ids — the reader is a subset, not a
  different dialect. It is still the shape of the file that is checked here, not GitHub's server-side
  validation; the entry says so where the code does.
- **Tests.** 18 in `xtask` (10 intake, 8 reader), all in `cargo test --workspace`. The reader's
  refusals are pinned: tab indent, flow list, `: ` in a plain value, duplicate key, indented document,
  and a block scalar that keeps its lines. The intake's are pinned too: a missing required id, a form
  with no required field, a duplicate id, an unrenderable element type, a misspelled attribute
  (`lable:`), a dropdown with no options, a bug form whose diagnostics description never names the
  command, and a pull-request template missing its `deny` checkbox. Breaking the shipped form for real
  — `id: os_version` → `id: os-version` — made `cargo xtask templates` fail naming that file and that
  id, and pass again once restored.
- **Visual verification: none needed.** No `crates/ui` file changed, so no baseline moved and
  `cargo xtask screenshots` was not run; `cargo xtask mobile` was not either, because no phone-facing
  crate changed.
- **Also this run:** `cargo-deny` was not installed in this worktree's fresh `.toolchain` (deny
  reported it and stopped), so it was installed with the documented `cargo install cargo-deny --locked`
  and the gate then passed.
- **Verified on this macOS checkout:** `. scripts/env.sh && cargo xtask ci` (fmt, clippy `-D warnings`,
  the whole workspace's tests including the 18 above, layers, roots, assets, fonts, version,
  attribution, a11y, docs and the new templates check), `cargo xtask snapshot` (no golden moved), and
  `cargo xtask deny` (advisories, bans, licences, sources all ok).
- **Also reconciled in this commit:** ROADMAP's Phase 5 item 1 ticked with what shipped; README's
  `cargo xtask ci` line, CONTRIBUTING's gate list and `docs/TOOLING.md`'s "checked, not trusted"
  section now name the intake gate.
- **Next**: Phase 5 item 2 — `docs/AUTONOMOUS_AGENT.md`: issue text is untrusted data; only
  `agent-ready`/`triaged` issues; never push to `main`, never auto-merge, never bypass `deny`, never
  touch signing secrets; the repro-first loop that ends in a Draft PR; and the bounded improvement
  queue (perf regressions, `enhancement-approved`, unchecked roadmap rows).
## 2026-11-24 (fifty-seventh) — phones get a native Rust app (SL-J10), and no web app is part of the product

- **Why.** The vendor's own product ships iOS and Android apps. The owner asked for a downloadable phone app
  built from the same Rust as the desktop, and for the web application to be removed.
- **Removed.** The web application and everything that built, served or gated it: its crate, its transport, its
  build task and gate, the server's page route, and the browser steps in both workflows. Nothing of it remains.
  **No web or browser application is part of this product and none may be added** — the non-goal is written
  down in `docs/PLAN.md`, `docs/ARCHITECTURE.md` §10 and the Phase 4 row of `docs/ROADMAP.md`. Kept because a
  phone uses them: `Engine::open_memory`, `library.snapshot`, `/thumb` and `sortlens_remote::client`.
- **Added.** `apps/sortlens-mobile`: the desktop host without the desktop — the same `sortlens-engine` and
  `sortlens-ui` in an eframe `App`, the library under the app's private directory, the camera roll as its
  source, `android_main` and `sortlens_ios_main` as the only unsafe code in the application. `cargo xtask
  mobile [android|ios]`, a `mobile` CI job per platform, the phone targets in `rust-toolchain.toml` and
  `scripts/bootstrap.sh`. D8 in `docs/PLAN.md` is reversed and says so.
- **Docs.** Every page now says the same thing: no web or browser application is part of the product
  (`docs/PLAN.md` §4 non-goals, `docs/ARCHITECTURE.md` §10, the Phase 4 row of `docs/ROADMAP.md`). The gate
  lists say `mobile` instead of the removed task, the five-face web allowlist and the two browser-facing
  crate pins are gone, and the font, attribution and architecture pages no longer describe a web build.
- **Verified here.** `cargo check -p sortlens-mobile` and the crate's tests on the host; `layers`, `roots`,
  `assets`, `version`, `attribution`, `a11y` and `docs`. **Not verified:** the Android and iOS targets. Both
  stop in `ring` and `tract-linalg`'s build scripts for want of the NDK and the iPhoneOS SDK (this Mac has
  command line tools only, and 3 GB free). The first machine with both runs `cargo xtask mobile`.
- **Next for the phone.** A phone layout (the shell is still desktop-shaped), settings persistence, the font set,
  send-to-desktop over the resumable upload, the iOS Photos library, packaging in the release workflow.

## 2026-11-24 (fifty-sixth) — the Mac reads the table the way the table means it: ⌘ is the command key, and a build without a symbols face says the glyphs as words

- **Why this came up.** The whole suite had only ever run on Windows. On this macOS checkout six
  `sortlens-ui` tests failed before any of this run's work: every shortcut was bound to the physical
  Control key (the table spells `Ctrl+O`, and `craft_keymap`'s `effective` folds ctrl into cmd only *off*
  macOS), so the ⌘O the help lists did nothing; and the shell painted `⌃`/`⇧`/`⏎` as keycaps while
  egui's bundled faces carry `⌘` and no other modifier glyph, so the audits tripped over tofu. A tree
  cannot be committed red, so the repair ships as a commit of its own.
- **The read.** `keymap::on_platform` is now the single translation at the ui's edge: on macOS a chord
  with `ctrl` and no `cmd` becomes `cmd` (⌘), which is what the table's `Ctrl+` means everywhere
  (`Ctrl+O` is ⌘O on a Mac and Ctrl+O elsewhere). Applied to the table's text (`chord_of`), to a
  rebind parsed as text (`keymap::parse_binding`) and to matching (`binding_runs`), so a table row,
  a user's `Ctrl+Alt+O` and the chord the dialog paints are one press — one function, not a copy in
  each reader.
- **The paint.** `keymap::spell` asks `fonts::symbols()`: with a symbols face embedded (craft
  fonts present) the keycaps are the platform's own (`⌘⇧R`); without one the glyphs egui's bundled
  faces cannot draw are spelled as the table's words (`⌘Shift+R`, `⌘Enter`), with ⌘ kept because it is
  the one modifier glyph the bundled emoji face does have. The arrows keep their triangles.
- **Tests.** Six tests that encoded Windows now read the platform: `dispatch_reads_the_keymap_not_the_handwritten_table`
  (⌘ opens), `painted_keys_follow_the_rebinds` (compares against `spell`, not a literal), the three
  glyph audits (they pass because nothing unpaintable is painted) and the shell's
  `the_shortcut_editor_opens_rebinds_and_saves` (compares against the catalog's own display, not the
  literal `Ctrl+Shift+R`). New `the_platform_spells_the_primary_modifier_its_own_way` pins the rule per
  platform: `Ctrl+Shift+R` on Windows/Linux, `⌘⇧R` with a symbols face on a Mac, `⌘Shift+R` without,
  and the plain keys and triangles unchanged. `cargo test -p sortlens-ui --lib` is 148 passed.
- **Still red on a Mac, and it predates this run:** `cargo xtask screenshots`. Two scenes
  (`command-palette`, `places`) differ by the chord wording, and
  `shortcut_editor_paints_in_both_themes` panics because `craft-keymap`'s own dialog paints the
  platform's `⇧`/`⌃` and the screenshot harness installs only egui's bundled faces — a face it
  needs is a craft-fonts face, and the harness never calls `fonts::install`. The gate is therefore a
  Windows/Linux gate today; making it a Mac gate means the harness embedding the symbol faces and a
  separate baseline set, which is a UI slice of its own (recorded in ROADMAP's Phase 4 item 1 row).
- **Verified:** `. scripts/env.sh && cargo xtask ci` (fmt, clippy `-D warnings`, the whole workspace's
  tests, assets, icons, fonts, docs, a11y, layers, roots, version, attribution), `cargo xtask snapshot`,
  and `cargo xtask deny` are green on this macOS checkout.

## 2026-11-24 (fifty-fourth) — Phase 4 item 3, thumbnails over the wire: the server's PNG route and the client's fetch

- **What shipped:** the piece a remote device cannot do without — it has no file access, so the
  pixels have to come over the wire. `GET /thumb/<photo-id>/<edge>` on `sortlens-server`: the photo is
  looked up by its stable id, decoded through the desktop grid's own `sortlens-thumbs::decode_file` path
  (so HEIC/RAW/JPEG behave exactly as they do locally), and answered as the export path's lossless PNG
  (`craft_image_codecs`, the same encoder `sortlens-export` uses — no second encoder, no second format).
  Under the paired token's `read` scope, like every other read.
- The edge is bounded, and the bounds are the reasoning: **16** (below it a device is upscaling a few
  pixels over a grid cell, which it can do itself from a bigger answer) to `sortlens_thumbs::DETAIL` —
  **1024** (above it a request means decoding a primary image to serve a grid cell). Out of range answers
  400 naming the range; a photo the catalog does not hold answers 404; a photo whose file cannot be read
  answers 404 too — a page paints a hole, not a lie, and no failure path answers 500.
- `sortlens_remote::client::Client::thumbnail(PhotoId, edge)` fetches it: `GET /thumb/<id>/<edge>` with
  the bearer token, PNG bytes returned untouched (they start with the PNG magic or the answer was a JSON
  refusal, so nothing is re-wrapped), and an unpaired client refuses without talking. A phone
  cannot invent a second format — this is the method it calls.
- Server plumbing: `http::Response::bytes(status, type, body)` beside `json` (the hand-written HTTP
  answer was JSON-only), the route in `handle`'s dispatch and `/`'s route list, `--help` updated; the
  server gains `sortlens-thumbs` and `craft-image-codecs` — both already in the tree, attributed, and
  behind the same licence review as the desktop and export paths.
- Tests: two against the real server process in `apps/sortlens-server/tests/client.rs` —
  `a_paired_device_reads_thumbnails_over_the_wire` (a PNG that decodes to the red fixture's own pixels
  with no upscaling past the 48×32 file, edge bounds / unknown photo / missing file each refused with the
  reason named, the file deleted under the answer so the 404 is real) and `a_token_without_read_cannot_pull_the_snapshot`
  extended to prove the same narrow token cannot read pixels either; three unit tests in
  `crates/remote/src/client.rs` (the request shape and token header, PNG bytes untouched, unpaired
  refuses without talking and a JSON refusal is a typed error naming status and reason).
- 809 tests; ci, deny, snapshot green. No UI change, so no screenshot baselines move.
- **Not verified by hand**: PNG size for a large photograph over a slow link.

## 2026-11-24 (fifty-third) — Phase 4 item 3, the client half: the device protocol, the snapshot on the wire, the engine in memory

- **What shipped:** the paired device's side of the protocol, in one place beside the server's.
  `crates/remote/src/client.rs` is what a paired phone speaks:
  `Client` over a `Transport` trait — native `TcpTransport` on `std::net`, because the protocol is
  the crate's, not the transport's. `health` (the SL-J07 connection test, no token), `pair` (the code is spent, the
  token is kept, `Client::paired` reopens one that a previous session saved), `call` for every
  allow-listed command, `snapshot` pulling the new `library.snapshot` registry command, and
  `upload` (SL-J04's client half): one request at or under the chunk size, `Content-Range`
  chunks above it, a `409`'s `received` count moving the cursor instead of failing the
  transfer, the completing chunk carrying `X-Content-Sha256`, and the id minted when the item
  is queued (`UploadItem::new`) so a retry after the app was backgrounded resumes the same
  file. The keep-or-remove toggle (SL-J05) rides on the item too: the ack answers with
  `LocalCopy::Keep`/`Remove`, decided client-side on the server's ack — which is what makes it
  survive the resume, because it lives on the item, not on the connection.
- The wire gained one command: `library.snapshot` (registry spec, `Engine::run` arm, the
  allow-list under `read`, an MCP example) returning the exact JSON `Engine::open_memory`
  opens. The engine itself gained the memory path one layer up: `open_memory` (no directory,
  the disk-side sessions — rejects bin, move receipts, cull file — simply absent, which is the
  truth for a fresh client), `is_memory`, `snapshot_json` as a public read.
- **Tests that fail when the behaviour is broken:** `sortlens-remote`'s unit tests against a
  scripted transport (the chunk plan covers the file once contiguously and an exact fit is one
  range, a 409 is an instruction not a failure and its `received` is where the next chunk
  starts, a 409 naming no progress fails instead of looping, nothing reaches the wire unpaired,
  pairing stores token and device, the one-shot carries the digest and no span, and the ack
  maps the toggle both ways); the engine's memory tests (a memory engine holding the library
  and operating while touching no file, the snapshot command serving exactly what the memory
  engine opens with the stale-revision guard holding in memory, a fresh client starting empty
  and round-tripping); three loopback tests in `apps/sortlens-server/tests/client.rs` against
  the real server process (pair → hold the library in memory from the snapshot → annotate
  through the wire with the write visible in a re-pulled snapshot; a token without `read`
  refused the snapshot by name; an upload that dies after the first chunk and resumes to a
  byte-verified file in the album with the partial gone and the keep-or-remove decision in the
  ack both ways — via a `DropAfter` transport wrapper, so the fault is the connection, not the
  server).
- **Files:** `crates/remote/src/client.rs` (new, ~700 lines with tests), `crates/remote/src/
  {lib,allowlist}.rs`, `crates/engine/src/{lib,command}.rs` (`open_memory`, `is_memory`,
  `snapshot_json`, the `library.snapshot` spec and run arm, three tests),
  `crates/mcp/src/examples.rs` (the `library.snapshot` row the arguments test demanded),
  `apps/sortlens-server/tests/client.rs` (new), `docs/ROADMAP.md`, `docs/FEATURES.md` (SL-J02, SL-J05), `docs/ARCHITECTURE.md` §10,
  `CHECKPOINT.md`.
- **Visual verification:** none needed — `crates/ui` is untouched and no painted surface
  changed; the slice is protocol, engine and app glue.
- Gates: `ci`, `snapshot` (no golden moved — the MCP examples are not a published output),
  `deny` (no new dependency: the client reuses serde_json and the suite's own SHA-256 already
  linked through `sortlens-ai`) all green.

## 2026-11-24 (fifty-second) — Phase 4 item 3, the in-memory catalog path

- **What shipped:** `Catalog::open_memory` in `sortlens-catalog` — a catalog that lives only in
  memory, because a remote device has no library directory to open. It receives the snapshot JSON
  the server holds (or nothing, for a fresh client), applies operations in memory with the same
  undo, redo, batch and stale-revision rules as the disk catalog, and never touches the
  filesystem: no directory, no log file, no lock. `read_only` is always false there — there is no
  other process to contend with and no lock to lose — and the periodic snapshot cadence is free,
  since there is nowhere to write. Two new surfaces make the round trip possible:
  `Catalog::snapshot_json` serialises the exact state `snapshot()` writes on disk (shared code,
  so the two cannot drift), which the client will hand to its own storage or back to the server,
  and a newer snapshot schema is *refused* rather than opened read-only — the disk path can warn
  and sit read-only beside the library files, but a client that cannot read the format must
  not pretend to hold the library. `is_memory` names the mode for the engine layer that follows.
- **Tests that fail when the behaviour is broken:** `a_disk_snapshot_opens_in_memory_and_operating_on_it_touches_no_file`
  (a disk catalog's snapshot JSON opens in memory answering the same questions — roots, ratings,
  tags, sequence — the tag undo replays from the snapshot's history, a commit in memory advances
  the sequence and lands the rating, and the temp directory's file count is unchanged through all
  of it); `a_fresh_memory_catalog_round_trips_through_its_snapshot_json` (an empty client syncs,
  rates past the `SNAPSHOT_EVERY` cadence with `snapshots_written` still zero, and a reopen from
  its own JSON restores the photos, the flag, the saved revision and an undo step);
  `a_broken_or_newer_snapshot_is_refused_in_memory` (unreadable JSON and a schema-past-this-build
  snapshot both err, the latter naming "newer than"); `the_stale_revision_guard_holds_in_memory`
  (`commit_expect` refuses a stale revision and the stale write never lands).
- **Files:** `crates/catalog/src/store.rs` (the `memory` flag, `log: Option<File>`, `open_memory`,
  `snapshot_json`, `is_memory`, the no-op persistence paths, four tests), `docs/ROADMAP.md`
  (Phase 4 item 3 row), `docs/ARCHITECTURE.md` §10, `CHECKPOINT.md`.
- **Visual verification:** none needed — no UI code changed; `crates/ui` untouched.
- Gates: `ci`, `snapshot`, `deny` (no new dependency — serde_json was already in the crate) all green.
- **Next**: the phone client itself (SL-J02), reusing `sortlens-ui` over the
  server's `/api` and `/upload` protocol, with an engine-side in-memory path if the engine needs
  one beyond the catalog's (check what `Engine::open` demands of the filesystem before writing
  it), and the keep-or-remove-after-upload ack (SL-J05) as the client's half. Phase 4 is then one
  row from done; after that, Phase 5 item 1 (the GitHub issue/PR templates) per the ROADMAP
  order. The i18n row keeps its open half (zh-TW/ko/th/vi, unrouted surfaces, RTL screenshots
  needing an Arabic font build).

## 2026-11-24 (fifty-first) — Phase 4 item 3, the desktop's own start/stop surface

- **What shipped:** the owner's window on the server process, in Settings → Sync & devices. Start
  spawns the `sortlens-server` binary that sits beside the app (`apps/sortlens/src/sync.rs`
  locates it beside `current_exe`, so `cargo run` and a packaged bundle both work) against the
  open library — loopback, ephemeral port — and the process's stdout is followed through the new
  `sortlens-remote::supervise`: `listening http://host:port` and `pairing NNNNNN` are parsed into
  a `ServerState`, the code a reprint replaces the expired one rather than growing a history, and
  an exit is visible on the next repaint. Stop kills the child and waits, so the library's write
  lock is free when the notice lands; dropping the supervisor kills it too, so a window that
  closes without tidying up cannot leave a server holding the lock.
- The shell's half (`crates/ui`): `UiState::server` is a `ServerSurface { running, port, code,
  devices }` the host feeds every frame the Settings dialog is open (default: the stopped state,
  so a headless frame paints an honest "not serving"), and the section paints the port, the live
  code, the device rows (name, scopes as ids, issue date through the same civil-date arithmetic
  the change report uses) and three new `Action`s — `ServerStart`, `ServerStop`,
  `ServerRevoke(token)`. Revoke is disabled while the server runs, with the reason on hover: a
  running server holds its own token store and would resurrect the row on its next write-through.
  The host refuses it too, so the rule survives a click on a stale frame; a stopped server revokes
  the exact row by exact token through `TokenStore::revoke`.
- Serving beyond loopback stays the CLI's `--bind lan`; the desktop's own surface does not offer a
  checkbox for it (SL-J09's default is the safe one).
- **Tests that fail when the behaviour is broken:** `crates/ui`'s
  `the_sync_section_starts_stops_and_revokes_only_when_stopped` (Start emits the action; the
  running surface names port and code; a click on the disabled Revoke answers nothing; once
  stopped the same button hands over the exact token) — the disabled button is the point: a
  revoke the running server could resurrect is not a revoke. `apps/sortlens`'s
  `the_device_list_is_the_token_file_and_revoke_needs_a_stopped_server` (the list is the token
  file, mapped; revocation rewrites it; an unknown token says so) and
  `revoke_while_the_server_runs_is_refused_by_name`. `crates/remote`'s
  `the_startup_lines_parse_and_nothing_else_does` and
  `a_reprinted_code_replaces_the_old_one` (a stub child, so the state-folding is proved without a
  real server). And in the loopback harness,
  `the_desktop_supervisor_starts_a_real_server_and_reads_the_token_file`: the supervisor spawns
  the real server binary, the port and code arrive from its own stdout, the desktop pairs through
  that port, and the row it would show is read back from the token file after the stop.
- **Files:** `crates/remote/src/supervise.rs` (new), `crates/remote/src/{lib,tokens}.rs` (the
  module and a public `TokenStore::all`), `crates/ui/src/shell.rs` (the section, the surface
  types, the three actions), `crates/ui/src/i18n/{ar,ja}.tsv` (four routed rows each),
  `crates/ui/src/lib.rs` (the exports), `crates/ui/tests/screenshots.rs` (the lenient harness and
  the scene), `apps/sortlens/src/sync.rs` (new), `apps/sortlens/src/main.rs`,
  `apps/sortlens/Cargo.toml`, `apps/sortlens-server/tests/loopback.rs`, `docs/ROADMAP.md`,
  `docs/FEATURES.md` (SL-J03), `docs/ARCHITECTURE.md` §10.
- **Visual verification:** two new baselines, `settings-sync-charcoal` and `settings-sync-paper`,
  painted at 900×1700 because the Settings dialog is longer than one screen and a baseline that
  clips its subject proves nothing. Read: the port, the code, the hint and the device row are all
  in frame, Revoke visibly disabled. The harness runs with `allow_missing_glyphs` for this scene
  only: the language picker names every language in itself, and CI's font build has no Arabic or
  Japanese face — the documented gap, not a silent defect.
- Gates: `ci`, `snapshot`, `deny` (no new dependency — the supervisor is std-only) (crates/ui changed) all green; `screenshots` green with the two new baselines.
- **Next**: Phase 4 item 3's remaining half — the phone client (SL-J02) reusing
  `sortlens-ui` over the same `/api` and `/upload` protocol. Its first slice is the in-memory
  catalog path in `sortlens-catalog` (snapshot JSON in, ops applied, no directory, no log, no
  lock), because a remote engine has no directory to open; the transport and the keep-or-remove
  ack (SL-J05) follow. Then Phase 4 is one row from done; after that, Phase 5 item 1 (the GitHub
  issue/PR templates) per the ROADMAP order. The i18n row keeps its open half (zh-TW/ko/th/vi,
  unrouted surfaces, RTL screenshots needing an Arabic font build).

## 2026-11-24 (fiftieth) — Phase 4 item 3, server half: `sortlens-server`, the loopback harness

- **What shipped:** the library is served to a paired device. `crates/remote` stops being a scaffold:
  `tokens.rs` (the four scopes — `read`, `upload`, `annotate`, `cull` — as a `Permissions` set, a
  `TokenStore` persisted as `<library>/remote-tokens.json` with atomic write-through on every change,
  re-issue replacing the row and revoke reporting the truth), `pairing.rs` (a six-digit code with a
  ten-minute life and a five-wrong-attempts lockout that survives even the right code; the code and the
  64-hex token both draw from the suite's own SHA-256 over a local entropy mix — no crypto dependency),
  and `allowlist.rs` (the table naming every registry id the wire accepts with the scope it needs,
  audited against `Registry::v0()` at startup by `allowlist::audit`, plus the two-sided test: every row
  must be a ready command, and every ready command must be either on the table or named in an exclusion
  list with its reason). `apps/sortlens-server` is the process: a hand-written minimal HTTP/1.1 on
  `std::net` (`http.rs` — Content-Length bodies only, `Transfer-Encoding: chunked` refused with a
  reason, `Connection: close`, zero new dependencies), `api.rs` (routes `/health`, `/pair`, `/api`,
  `/upload`, `/upload/<id>`; `/api` checks bearer token → allow-list scope → `Engine::run`, and a
  method the wire does not accept answers 403 without revealing whether it exists), and `upload.rs`
  (resumable chunks appending to `<album>/.uploads/<id>.part`, so the file length is the received
  count and a restart loses nothing; a wrong-position chunk answers 409 with `received`; the completing
  chunk is hashed against `X-Content-Sha256` *before* it is appended, so a digest mismatch leaves the
  partial exactly where it was; the published name never overwrites — it takes ` (n)` beside the stem;
  names are leaf-only and folder-safe). The server binds loopback by default and every interface only
  under `--bind lan` (SL-J09), prints its port and pairing code on stdout, and pumps the engine's
  finished scan work between connections.
- **The two-process loopback harness** (`apps/sortlens-server/tests/loopback.rs`) is the proof the
  old M9 table said was missing: five tests, each spawning the real server binary
  (`CARGO_BIN_EXE_sortlens-server`) against a real library the test scans through the engine first and
  then releases. `a_paired_phone_reads_annotates_and_is_scoped` pairs, reads two scanned photographs,
  writes a 5-star rating through `/api`, is refused `/api` without a token and with a nonsense one,
  is refused `app.quit` with a good one, and reads the rating back from disk after the server process
  is killed; `the_pairing_code_locks_out_after_five_wrong_attempts` spends the code;
  `a_narrow_token_cannot_cull_and_the_wide_one_can` proves `--allow read,annotate` grants exactly
  that and `cull.start` answers 403 naming the missing scope;
  `an_upload_resumes_after_a_dropped_connection_and_verifies_the_digest` uploads 300 KB in three
  chunks, drops the connection mid-transfer, asks `GET /upload/<id>` where to continue, is refused a
  wrong digest (the partial stays), completes with the right one, and finds `IMG_0001 (1).jpg` beside
  the first when the same name uploads again; `a_server_restart_keeps_the_tokens_and_the_pairing_scope`
  pairs, kills the server, restarts it, and the same token still works.
- **Files:** `crates/remote/src/{lib,tokens,pairing,allowlist}.rs` (rewritten from the M0 scaffold),
  `crates/remote/Cargo.toml` (`sortlens-engine`, `sortlens-ai`, `serde`, `serde_json`),
  `apps/sortlens-server/src/{main,http,api,upload}.rs`, `apps/sortlens-server/Cargo.toml`,
  `apps/sortlens-server/tests/loopback.rs` (new), `docs/ROADMAP.md` (the Phase 4 item 3 row split into
  the shipped server half and the open client half; the M9 state prose and the "Re-opened from M9"
  paragraph updated), `docs/FEATURES.md` (SL-J01/J03/J04/J06/J07/J08/J09 annotated),
  `docs/ARCHITECTURE.md` (§10 rewritten to describe what runs; the layer diagram's remote line).
- **Tests that fail when the behaviour is broken:** 11 in `crates/remote` (the allow-list audit, the
  two-sided registry test, the lockout and TTL arithmetic, token generation, the store's persistence,
  re-issue and revoke, scope parsing) and 3 unit + 5 loopback in `sortlens-server` (name sanitising,
  range parsing, id alphabet; the five harness tests above). The lockout test fails against a gate
  that forgets its attempts — the first version of the handler redeemed on a clone of the gate, and
  the harness caught it before it shipped; the digest test fails against a server that appends the
  final chunk before verifying it, because the resumed re-send answers 409 instead of 200.
- **Visual verification:** n/a — no UI surface changed.
- Gates: `ci` green (fmt, clippy `--all-targets -D warnings`, workspace tests, layers/roots/assets/
  fonts/version/attribution/a11y/docs), `snapshot` green (no CLI verb touched), `deny` green (no new
  dependency). Nothing under `crates/core`, `crates/ui` or a phone-facing crate
  changed. `screenshots` not required: no `crates/ui` change.
- **Next**: Phase 4 item 3's remaining half — the phone client (SL-J02) reusing
  `sortlens-ui` over the same `/api` and `/upload` protocol, plus the desktop's own start/stop surface
  for the server. Then Phase 4 is one row from done; after that, Phase 5 item 1 (the GitHub issue/PR
  templates) per the ROADMAP order. The i18n row keeps its open half (zh-TW/ko/th/vi, unrouted
  surfaces, RTL screenshots needing an Arabic font build).

## 2026-11-24 (forty-ninth) — Phase 4 item 2d: the `craft-i18n` scaffolding (SL-L07)

- **What shipped:** the interface's strings begin to load through the suite's i18n. `crates/ui/src/i18n.rs`
  builds one `craft_i18n::I18n` from the embedded Arabic and Japanese catalogues (`crates/ui/src/i18n/ar.tsv`,
  `ja.tsv` — the first two of the six languages the design names), keeps a thread-local current language,
  and hands call sites `tl!`/`fmt`/`tn`. The English text is the lookup key, a missing row is the English
  itself, and a bad row is logged once as a warning naming the line — partial coverage shows English
  rather than breaking anything. The first batch routed: the sidebar's six sources, five section titles
  and the `Library` header, the foot's four registry labels (`Check Library`, `Recently Deleted`,
  `Recent Changes`, `Keyboard Shortcuts`), the right panel's four tab labels, both delete modes, and the
  whole Settings dialog (its window title, every section header, the three buttons, the sidecar checkbox,
  the cull-batch header and the closing note).
- **The language is a setting:** Settings → Language offers every language named in itself (English,
  العربية, 日本語, from each catalogue's `@id language.name` row); switching is instant (the shell applies
  the direction every frame) and asks the host with the new `Action::SetLanguage(code)`; the app persists
  `Appearance.language` and restores it at launch, an unknown code reading as English.
- **RTL, the direction plumbing SL-L07 stands on:** `i18n::apply_direction` resolves the interface
  direction from the chosen language through `craft-ui`'s `direction::resolve` (the `CRAFT_UI_DIRECTION`
  override intact, for checking a mirrored layout before a right-to-left catalogue exists) and applies it
  to the context every frame — egui's own chrome mirrors through the fork's `Style::right_to_left`. The
  sidebar's panel sits at the reading start, the tool strip and right panel at the reading end;
  `nav_row` paints its glyph, label and count from the reading start (the colour dot with it), and
  section titles align to the reading start. The `with_layout` wrap is behind an RTL check on purpose:
  at `Align::Min` it rounds the label's origin differently from a bare `ui.label`, and the 19 screenshot
  baselines are the proof that a left-to-right interface paints byte for byte what it painted before.
- **Tests that fail when the behaviour is broken:** the catalogue audit
  `catalogs_load_and_routed_keys_have_rows` (zero parse problems; the routed keys read out of
  `shell.rs`'s own `tl!` calls, more than 20 of them, and every one has a row in both Arabic and
  Japanese — a routed key without a row fails naming the key); `the_picker_names_every_language_and_directions_follow_the_script`
  (the picker is exactly English/العربية/日本語, each named in itself; Arabic resolves right to left,
  Japanese left to right); the app's `the_language_setting_round_trips_and_an_unknown_one_is_english`
  (every offered code round-trips through the TOML and is one the interface offers, `tlh` reads as
  English, and a pre-scaffold settings file still opens); and the headless
  `the_language_switch_moves_the_sidebar_to_the_reading_start` — the radio clicked through the real
  Settings widget, `SetLanguage("ar")` asked of the host, the sources header measured in the right half
  and the All Photos row reading Arabic, then switched back to English through the same dialog and the
  sidebar measured back at the left edge. Headless on purpose: CI's fonts have no Arabic face, so this
  proves the geometry, not the glyphs.
- **Visual verification:** all 19 screenshot baselines regenerated byte-identical. The first run moved
  one sidebar section header by a rounding fraction and the diff caught it (472 pixels in one 50×20
  box) — that is the audit working, and the reason the wrap is conditional.
- Files: `crates/ui/src/i18n.rs` (new), `crates/ui/src/i18n/ar.tsv` (new), `crates/ui/src/i18n/ja.tsv`
  (new), `crates/ui/src/lib.rs` (the module), `crates/ui/Cargo.toml` (`craft-i18n`, `log`),
  `crates/ui/src/shell.rs` (the routes, the panels, `nav_row`, `Action::SetLanguage`, the Settings
  language row, the headless test), `apps/sortlens/src/settings.rs` (`Appearance.language`, round-trip
  and old-file tests), `apps/sortlens/src/main.rs` (restore at launch, persist on the action),
  `docs/ROADMAP.md` (2d ticked with the proof named; the M9 row and state prose updated to say what is
  still open), `docs/FEATURES.md` (the SL-L07 row), `docs/UX.md` (the i18n bullet),
  `docs/ARCHITECTURE.md` (new §8.4 Interface language, keymap renumbered to 8.5).
- Gates: `ci`, `snapshot`, `deny`, `screenshots` (19/19 byte-identical)
  all green.
- **Next**: Phase 4 item 3 — `sortlens-server` (loopback/LAN bind, pairing code → scoped device token,
  command-registry allow-list, resumable uploads) and the phone client reusing `sortlens-ui`, proved by a
  two-process loopback integration harness in CI (SL-J01..J09). The i18n work that remains lives in M9's
  row: zh-TW/ko/th/vi, the unrouted surfaces, and RTL screenshots of the painted interface, which need a
  font build with an Arabic face.

## 2026-11-24 (forty-eighth) — Phase 4 item 2c: the opt-in update check (SL-I10)

- **What shipped:** `craft-update` behind an explicit opt-in — the second of the two owner-approved
  network calls, consent-gated in exactly the shape `model.install` set. `sortlens-engine`'s new
  `update` module (desktop-only, gated like the model downloader) reads the latest published release of `Topzic/sortlens-rust` through `craft-update`'s
  parser — drafts and error bodies refused, every text capped, asset links held to the app's own
  repository — and compares it against the running version with `craft-update`'s semver rules,
  including the local-build rule, so a build from source is never offered the release it already
  contains. Three registry commands: `update.status` (version, consent, repo, whether a check is
  running, the last answer), `update.check` (refuses without consent, naming the setting, the way
  the downloader does; refuses again while one is running), and `update.consent`. The check runs on
  a worker thread as an `Update` activity job; the answer is kept for `update.status` and the
  Settings row, so one that lands after the dialog closes is not thrown away.
- **Every face wired:** the app persists the consent as `updates.check_consent` and restores it at
  launch like the model downloader's; Settings → Updates paints the consent switch, the `Check now`
  button only behind it, `Checking…` while the worker runs and the last answer's one-line headline;
  the CLI answers with `update` (status), `update-consent on|off` and `update-check` (a progress
  verb, like `model-install`); the MCP examples name `update.check`, `update.consent` and
  `update.status`. Nothing downloads or installs automatically — an update still means building from
  source; the self-update path stays in M10's absence table with the installer work it needs.
- **Tests that fail when the behaviour is broken:** in the module, the parse tests (a newer stable
  release offered and summarised, an older one not, a prerelease skipped for a stable running build,
  a local build only ever offered a higher release number, garbage and a rate limit named as errors,
  an empty answer read as up to date rather than as a failure); in the engine, the refusal without
  consent naming `Settings → Updates`, the consent flip through its command, `update.status`'s shape
  before and after a check, the full worker path over a loopback feed in GitHub's own shape (the job
  finishing Done, the answer kept, the headline read back), a failing feed failing the job and
  keeping no answer, a second check refusing while one runs, and the endpoint/fetcher seam that
  keeps the network out of the tests (`craft-update`'s real client is HTTPS-only, so `update.check`
  under test swaps in the loopback fetcher, never the consent gate). In the shell, the headless
  `the_updates_settings_section_gates_the_check_behind_consent`: the switch clicked through the real
  widget flips the engine and asks the host to persist it, `Check now` exists only behind consent,
  and the test never presses it — pressing it would be the real network call.
- **Visual verification:** no baseline moved. No settings scene exists (the dialog is covered by the
  headless tests), and the section is desktop-only, so `cargo xtask screenshots` ran green over all
  33 baselines byte-identical — the proof the Settings change touches no other surface.
- Files: `crates/engine/src/update.rs` (new), `crates/engine/src/lib.rs` (module, fields,
  `update_consent`/`set_update_consent`/`check_for_updates[_with]`/`start_update_check`/
  `poll_update_check`, the three command handlers, the tests), `crates/engine/src/command.rs` (the
  three registry rows), `crates/engine/src/job.rs` (`JobKind::UpdateCheck`),
  `crates/engine/Cargo.toml` (`craft-update` without the `egui` feature, so the CLI build stays
  clear of egui), `crates/ui/src/shell.rs` (`Action::SetUpdateConsent`, `updates_section`, the
  `run_ui` test), `apps/sortlens/src/settings.rs` (`Updates.check_consent`, defaults, round-trip and
  old-file tests), `apps/sortlens/src/main.rs` (restore at launch, persist on the action),
  `apps/sortlens-cli/src/main.rs` (the three verbs, help, schema row), `crates/mcp/src/examples.rs`,
  `docs/ROADMAP.md` (2c ticked with the proof named; M10 prose reconciled), `docs/FEATURES.md` (the
  SL-I10 row), `docs/UX.md` (the Updates dialog row), `docs/RELEASE.md` (the "no update check"
  claim replaced), `docs/ARCHITECTURE.md` (§13 updates).
- Gates: `ci`, `snapshot` (the two CLI goldens that name the verb list were regenerated with
  `--update` and read), `deny`, `screenshots` all green.
- **Next**: Phase 4 item 2d — `craft-i18n` scaffolding (SL-L07): the `craft-i18n` crate is already a
  pinned workspace dependency; wire the catalog so the interface's strings load through it, with the
  RTL behaviour of SL-L07's design note and the font stack already handling the symbols, then item 3
  `sortlens-server` + the phone client with the two-process loopback harness.

## 2026-11-24 (forty-seventh) — Phase 4 item 2b: the High-Contrast preset (SL-L09)

- **What shipped:** the fifth theme preset — `ThemeKind::HighContrast`, persisted as
  `theme = "high-contrast"` through the same `id()`/`from_id()` path as the other four, so no migration
  is needed and an unknown id still falls back to Charcoal. Pure black canvas and thumb wells, pure white
  text, a yellow accent (`#ffd23f`) that holds 4.5:1 against black with room to spare, muted text at AA
  (`#cccccc`), a grey `#6b6b6b` hairline so a border reads as a boundary rather than as text, 2-point
  corner radius, and a new `Tokens::stroke_w` metric that doubles every chrome stroke — window frames,
  widget outlines, the selection ring and the press ring — while the four colour presets keep their
  1-point hairlines.
- **The one token, not five branches.** `stroke_w` is the metric `apply()` uses for every `Stroke::new`
  it installs; no preset carries its own stroke-writing code, and a sixth preset would be a new constant,
  as the file's doc comment already promised. The preset appears everywhere presets are named because
  every menu iterates `ThemeKind::ALL` — the Appearance chip's cycle, its right-click list, and the
  Settings radios — with no new drawing code.
- **Tests that fail when the behaviour is broken:** `high_contrast_is_black_white_yellow_and_doubled`
  (WCAG luminance ratios, not eyeballing: text at AAA ≥ 7:1, muted at ≥ 4.5:1, accent and accent-foreground
  at ≥ 3/4.5:1, the accent actually yellow, all five presets' `stroke_w` correct, and `apply` really
  installs the 2-point strokes and the token fills); the five preset-wide tests (`ids_round_trip`,
  `light_presets_have_dark_text`, `the_widget_palette_is_the_token_set_and_apply_installs_it`,
  `accents_are_visible_against_the_background`, `badge_and_flag_marks_hold_three_to_one_against_a_bad_cell`)
  now run over all five — the badge test bit immediately: the first reject colour's white glyph measured
  2.78:1, fixed to `#ff5252` before anything was committed.
  `the_settings_dialog_offers_high_contrast_and_clicking_it_asks_the_host` drives the real radio widget
  headlessly and asserts `Action::SetTheme(HighContrast)` reaches the host while the shell waits (the host
  applies and persists it). The persistence layer is covered by `every_theme_round_trips_and_a_future_one_falls_back`
  in the app's settings — the round-trip test the grid and density had and theme did not.
- **Visual verification:** two new baselines, `library-justified-high-contrast` and
  `welcome-high-contrast` (33 now), both read: black canvas, white words, yellow stars and selections,
  the chip/badge marks clearly visible, borders that read as boundaries; the other 31 regenerated
  byte-identical. `cargo xtask screenshots` green; the other presets' baselines did not move, which is
  the proof the `stroke_w` change is exactly zero for them.
- Files: `crates/ui/src/theme.rs` (the variant, `Tokens::HIGH_CONTRAST`, `stroke_w`, the test),
  `crates/ui/src/shell.rs` (the Settings-dialog `run_ui` test), `crates/ui/src/lib.rs` and
  `crates/ui/src/icons.rs` (doc counts), `crates/ui/tests/screenshots.rs` (`kind_id` arm, the new
  baseline test, the THEMES comment), `apps/sortlens/src/settings.rs` (the theme round-trip test),
  `docs/ROADMAP.md` (2b shipped with the proof named; M10 state and absent-table prose reconciled; the
  item 1 baseline count marked historical), `docs/UX.md` (the five-preset table, the contrast bullet
  now naming a shipped preset, the screenshot-suite prose), `docs/ARCHITECTURE.md` (§8.2 five presets).
- Gates: `ci`, `snapshot`, `deny`, `screenshots` all green.
- **Next**: Phase 4 item 2c — `craft-update` behind an explicit opt-in (SL-I10): a consent-gated update
  check in Settings, the same shape as `model.install`'s gate (user-initiated network only), with the
  engine or app host holding the consent flag and no call without it. Then 2d `craft-i18n` scaffolding
  (SL-L07), then item 3 `sortlens-server` + the phone client with the two-process loopback harness.

## 2026-11-24 (forty-sixth) — Phase 4 item 2a: the packaging icons are renders of the one drawing

- **What shipped:** `assets/icon.ico` and `assets/icon.icns`, generated — never dropped in — from the same
  geometry in `crates/icon`, with `cargo xtask icons` as the check-and-rewrite front door. This was the
  first clause of ROADMAP Phase 4 item 2; the item is now split (2a done, 2b High-Contrast, 2c craft-update,
  2d craft-i18n still open).
- **One drawing, four renderings.** `sortlens-icon::render(size)` is the resolution-independent render
  (the geometry was already in fractions of the tile, so every size is a fresh raster, not a resample —
  a curve stays a curve at 16 px), and `png_at(size)` is its PNG. `ico()` packs 16/32/48/64/128/256 px
  entries into the little-endian ICONDIR (PNG entries throughout, the 0-as-256 edge byte honoured, planes
  1 and 32 bpp declared), and `icns()` packs `ic11`/`ic12`/`ic07`/`ic08`/`ic09` — the 32/64 @2x list-view
  slots, the 128 and 256 Finder icons and the 512 Get Info slot — into the big-endian container whose
  header length is the file length. No new dependency, and the same stored-deflate PNG writer as the
  window icon, so the bytes are deterministic on every machine (the crate's stated trade: size for
  reproducibility — 359 KB and 1.4 MB committed).
- **The front door lives in `xtask`, as the row asked.** `cargo xtask icons` generates both files in
  memory and compares them to the committed bytes, failing with the rewrite command on a mismatch;
  `cargo xtask icons --update` rewrites them (the `snapshot --update` pattern). The suite still gates
  them: `the_committed_packaging_icons_are_the_drawing` compares byte for byte and shares the SVG's
  `SORTLENS_UPDATE_ICON=1` rewrite path, so `cargo xtask ci` refuses a stale packaging file without
  running xtask twice.
- **Tests that fail when the behaviour is broken:** `the_ico_carries_one_png_per_desktop_size` parses the
  ICONDIR back field by field, walks the offsets the directory declared, decodes every embedded PNG
  through `sortlens_thumbs` and asserts the 40%-of-tile pixel is the front print at all six sizes (the
  centre pixel was tried first and is honestly wrong at 16 px — the lens edge reaches it, so the test
  samples a pixel that is the front print at every size); `the_icns_carries_the_mac_slots` walks the
  chunk chain, asserts each length stays inside the header's total and decodes each slot to its named
  square; `the_same_bytes_every_time` grew `render(64)`, `ico()` and `icns()` determinism. The committed
  test failed on a missing file before generation, proving it bites.
- Files: `crates/icon/src/lib.rs` (`render`, `png_at`, `ico`, `icns`, four tests), `xtask/Cargo.toml`
  (`sortlens-icon`), `xtask/src/main.rs` (the `icons` task + help line), `assets/icon.ico`,
  `assets/icon.icns`, `docs/ROADMAP.md` (Phase 4 item 2a shipped and item split; M10 state, icon row and
  absent-table prose reconciled), `docs/RELEASE.md` (the icon section now describes what ships),
  `docs/ARCHITECTURE.md` (§13 icon line and the layer-diagram row), `docs/ATTRIBUTION.md` (the two new
  asset rows).
- Gates: `ci` green (the 7 `sortlens-icon` tests among them), `snapshot`, `deny`, `icons` all green;
  `screenshots` not run — no `crates/core`/`crates/ui` or UI code changed.
- **Next**: Phase 4 item 2b — the High-Contrast preset (SL-L09), then 2c the opt-in `craft-update`
  (SL-I10), 2d the `craft-i18n` scaffold (SL-L07), then item 3 the server and phone loopback harness.

## 2026-11-24 (forty-fifth) — Phase 4 item 1: the keymap is a machine now, not just a table

- **What shipped (SL-I09):** the `craft-keymap` editor in Settings → Shortcuts — rebinding, conflict
  detection, named sets, and craft-keymap JSON import/export — with dispatch genuinely reading it. The
  crate was already pinned in `[workspace.dependencies]` and linked by nothing; it is linked by `sortlens-ui`
  and `sortlens` now, which also closes the last unlinked Phase 4 pin.
- **The runtime half lives beside the table it documents.** `crates/ui/src/keymap.rs` keeps `BINDINGS` as
  the readable, registry-cross-checked table it was, and adds the machine built from it: `catalog()` turns
  every single-chord command row into a `craft_keymap::Bindable` under `Scope::app()` (19 commands; the two
  flag rows are one registry command as two parameterised refs, `photo.flag {"flag":"pick"}` and `"reject"`,
  so a rebind of one leaves the other alone), `fixed_rows()` presents the unrebindable rows — the ranges
  `0–5`/`6–9`, the mouse rows, `Esc`, the arrows — read-only in the editor, with a chord set only where it
  does not sit on a catalog default. `build(overrides)` is the keymap the shell runs on `Platform::current()`.
- **Dispatch is the keymap's now.** `plain_pressed(keymap, input, command)` is the runtime twin of the old
  `key_pressed` calls (plain keys, repeats included, nothing consumed, the command modifier still gating);
  `consume(keymap, input, command)` reads *exactly* through craft's layout and removes the press events that
  ran — deliberately not `consume_shortcut`, whose "extra Shift and Alt are ignored" rule would have left a
  rebound `Ctrl+Alt+O` still answering plain `Ctrl+O`. Every command-bound key check in the shell was
  rerouted: `?`, `Ctrl+O`, `Ctrl+Shift+R`, `Ctrl+Shift+N`, `Ctrl+,`, `Ctrl+Q`, `Ctrl+Z`, `Ctrl+Shift+Z`,
  `Ctrl+Enter`, `I`/`M`/`S`, `D`, `C`, `P`/`X`, `K`, `T` (library and cull), and the cull `◀`. The hardcoded
  rows (`U`, `G`, `Esc`, `Space`, `Ctrl+A`, `Ctrl+K`, the zooms, the detail arrows) stay hardcoded and are
  exactly the editor's fixed rows — a rebind the shell would never read does not exist. The redo-before-undo
  consume-order note is gone: exact matching separates `Ctrl+Shift+Z` from `Ctrl+Z` by itself.
- **The help cannot go stale.** The palette and the `?` overlay paint `painted()`/`row_keys()` — the current
  bindings, triangles for arrows, joined ` / ` the way the registry spells two — falling back to the
  registry's declared shortcut for commands outside the catalog. `Shift+P` still flags (the plain-key
  semantics are unchanged for defaults); a rebind carrying Shift reads exactly.
- **The editor itself.** Settings → Shortcuts opens craft-keymap's dialog over the app's catalog: recording
  with the conflict bar (Ask/Steal/Keep Both), named sets (a built-in `Default`; a first save becomes the
  user's `Custom` set and goes active), craft-keymap JSON import and export through the host's file dialogs,
  per-set revert, undo/redo inside the dialog, and double-press recording refused through an `accept` rule —
  the dispatch does not read doubles, so the editor must not record them. The dialog owns the keyboard while
  it is open: `keyboard()` steps aside and ⌘K waits. `Outcome::Ok` hands `Profiles` to the host
  (`Action::SaveShortcuts`), which persists them; `Import`/`Export` come back as actions the host answers
  with `rfd`, the imported text landing in `pending_shortcut_import` for the next frame. Overrides live in
  `Prefs::shortcuts` as craft-keymap `Profiles` (the `overrides_list` array form), so a deleted set stays
  deleted after saving and a file written before the section existed reads as the factory bindings.
- **Tests that fail when the behaviour is broken:** in `keymap.rs`, `the_catalog_is_the_rebindable_rows_of
  _the_table` (every catalog command ready in the registry, every single-chord command row present, the
  ranges and `Click` refused), `the_fixed_rows_are_the_rest_of_the_table_and_clash_with_no_catalog_default`,
  `a_rebinding_moves_the_bindings` (and the second key on one command is additive),
  `two_commands_on_one_chord_are_conflicts`, `dispatch_reads_the_keymap_not_the_handwritten_table` (headless
  frames: `Ctrl+O` opens, the rebind moves the chord, `⌘P` is not a flag, `Q` becomes similar after a rebind),
  and `painted_keys_follow_the_rebinds`. In the shell, `a_rebound_key_moves_the_action` (Q flags as picked,
  P stops answering, X untouched) and `the_shortcut_editor_opens_rebinds_and_saves` (the editor paints the
  catalog under the table's own sections, owns the keyboard, OK hands the host the override and Q still
  flags after the save). In the app, `shortcut_sets_round_trip_and_an_older_file_reads_the_defaults`.
- **Visual verification**: new `shortcut-editor-charcoal`/`shortcut-editor-paper` baselines (31 now), both
  read at native scale — the dialog follows the theme, the table is grouped Anywhere/Library/Cull session
  with the factory chords in keycaps, the set row carries New/Rename/Delete/Save/Revert/Export/Import, and
  Cancel/OK sit at the foot. The other 29 baselines regenerated byte-identical: with factory bindings the
  runtime text equals what the registry declared, so no surface moved. One honest rough edge, noted not
  hidden: the editor spells `cull.back`'s arrow "Left" (craft's own display), while the overlay, the palette
  and the table spell the triangle `◀` — the editor's spelling is craft's, ours is the triangles.
- Files: `crates/ui/src/keymap.rs` (the runtime half + six tests), `crates/ui/src/shell.rs` (runtime state
  in `UiState`, the editor dialog and its options, the Settings section, the key reroutes, the palette and
  overlay painting, two shell tests), `crates/ui/Cargo.toml` (`craft-keymap`, `serde_json`),
  `crates/ui/tests/screenshots.rs` (the new scene), `apps/sortlens/Cargo.toml` + `src/settings.rs`
  (`Prefs::shortcuts` + round-trip test) + `src/main.rs` (startup load, the three host actions),
  `docs/ROADMAP.md` (M9 keymap row and Phase 4 item 1 ticked, M9 state sentence reconciled),
  `docs/RELEASE.md` (the "no keymap editor" claim replaced by what shipped), `docs/UX.md` (§8 Shortcuts
  row), `docs/ARCHITECTURE.md` (§8.4 cull defaults corrected to the shipped keys), `docs/ATTRIBUTION.md`
  (the pin is linked).
- Gates: `ci` green (140 `sortlens-ui` unit tests, all crates), `snapshot`, `deny`, `screenshots` (18 scenes) all green.
- **Next**: Phase 4 item 2 — `.icns`/`.ico` generation from `crates/icon` inside `xtask` (ROADMAP Phase 4
  item 2's first clause), then the High-Contrast preset (SL-L09), the opt-in `craft-update` (SL-I10) and
  `craft-i18n` scaffolding (SL-L07).
## 2026-11-23 (forty-fourth) — Phase 3B.6 part 9: the arrows the faces lack are triangles now

- **The defect was measured before it was fixed, and the measurement moved the slice.** A probe against the default
  font stack (`FontDefinitions::default()` of the pinned egui fork) found: the proportional chain (Ubuntu-Light →
  egui-icons → NotoEmoji-Regular → emoji-icon-font) has neither `←` (U+2190) nor `→` (U+2192), and also lacks `⇧`,
  `⌃`, `✕`, `✓`, `⌥` and `↵`; the monospace chain carries Hack, which has all of them; and the triangles `◀`/`▶`
  render in *both* chains (NotoEmoji in proportional, Hack in monospace). So the checkpoint's premise was half
  right: the keymap overlay's keycaps paint in `FontId::monospace` and had been drawing the arrows fine — the
  proportional surfaces the same strings reach were the broken ones: the palette's shortcut column (the registry's
  `cull.back` row would draw a box if it were ever listed), and the cull start screen's hint line, which painted
  `← back` today.
- **One spelling everywhere, chosen for both faces.** Every painted arrow is now `◀`/`▶`: the keymap's four
  `keys` strings (`▶ / Space`, `◀`, `◀`, `▶ / ◀`), the registry's `cull.back` `shortcut` — which with it changes
  the CLI golden `24 shortcuts.txt` — and the cull hint. The same audit retired the other proportional tofu the
  shell painted: the keymap's Shift row and the strip's count hint say `Shift-click` (⇧ is not in the proportional
  face), the keyword menu marks active rows with a strong `•` instead of `✓`, and the strip's clear chip is `×`
  (U+00D7, which Ubuntu-Light has) instead of `✕`. `fonts.rs`'s doc claimed the keymap draws `⌃`, `✕`, `◀` and
  `✓` — no code drew `⌃` and the defaults lack `✕`/`✓`; it now names what the UI really paints (`◀`/`▶`, `↔`,
  `⌘`) and why `Zsym` stays required.
- **Two guards, both failing on the old spelling.** `keymap::every_painted_keycap_and_action_exists_in_the_face_that_paints_it`
  asserts every binding's `keys` renders in the monospace face and every action *and every ready registry
  `shortcut`* in the proportional one — verified to fail when the registry's `◀` is reverted to `←`. And
  `shell::the_shortcuts_dialog_paints_only_glyphs_the_bundled_faces_have` paints the shortcuts overlay headlessly
  with an empty query (every binding) and a `◀` needle, asserting no painted text lacks a glyph.
- **Visual verification**: new `shortcuts-charcoal`/`shortcuts-paper` baselines, scene narrowed to the triangles
  themselves (`shortcut_query = "◀"`, `3 of 43 bound`) so the keycap column's glyphs are the subject; both read at
  native scale and zoomed — Detail's `◀`, Cull's `◀` and Compare's `▶ / ◀` are real glyphs pointing the right ways
  in both presets, and the search field's own `◀` renders. The other 29 baselines regenerated byte-identical (no
  other scene paints these glyphs — the strip hint and keyword menu need a selection or an active filter).
- Files: `crates/ui/src/keymap.rs` (four keycap strings, the Shift reword, the module doc's new note, the new
  face-coverage test), `crates/engine/src/command.rs` (`cull.back`), `crates/ui/src/shell.rs` (cull hint, strip
  hint, keyword menu marker, strip chip, the overlay audit test), `crates/ui/src/fonts.rs` (reconciled doc),
  `crates/ui/tests/screenshots.rs` (new scene), `apps/sortlens-cli/tests/golden/24 shortcuts.txt` (regenerated via
  `cargo xtask snapshot --update` and diff-read), and the reconciled `docs/UX.md` (§5.4 steps, §4.3 Shift-click)
  + `docs/ROADMAP.md` (3B.6 (g) ticked).
- Gates: `ci` green (131 `sortlens-ui` unit tests), `snapshot`, `deny`,
  `screenshots` (17 scenes) all green.
- **Next**: Phase 4 item 16, the `craft-keymap` editor in Settings → Shortcuts (ROADMAP Phase 4 item 1, SL-I09:
  rebinding, conflict detection, import/export). One caveat for whoever runs it: the peer agent working issue #1
  (Places map not loading) is building its fix in a separate worktree off `HEAD` and will rebase after this push;
  if its work has landed first, rebase this branch before starting and re-read the Places baselines. Also noted for
  a later slice, found while auditing: `⌥-drag pan` (shell.rs doc) is not a painted string, but `0–5`-style key
  names in `hints` and the compare footer should get the same audit when Phase 4 item 1 rebuilds the keymap editor.

## 2026-11-23 (forty-third) — Phase 3B.6 part 8: the cull foot is claimed before the photograph

- **The defect was an allowance, not a measurement.** `cull_view` sized its canvas as
  `available_height - strip_height(84) - action_height(46) - 16`, where 84 and 46 accounted for the up-next strip and
  the decision row — and nothing accounted for the keyword band, the field that adds a keyword, the live line or the
  staged sentence below them. At 1440×900 the field was painted at y 846–861 with the bottom bar at 852: its glyphs
  were cut in half, and `Photo 1 of 12` under it was not painted at all. The screenshot baseline showed it plainly
  and the defect had been recorded in 3B.6 since the suite's first run.
- **The fix inverts the order and removes the guess.** The block under the photograph is now
  `egui::Panel::bottom("cull_foot")` with `.exact_size(cull_foot_height(…))`, claimed *before* the canvas; the canvas
  is then allocated out of `ui.available_height()`, so there is no window height at which the two can overlap — the
  photograph gives way, never the controls. `cull_foot_height` is arithmetic over rows of fixed height (the up-next
  strip 56, a `interact_size.y` row for the decisions, the keyword band, a row for the field, a line for the spoken
  status); only the band and the staged sentence are conditional, and both conditions — does the photo on screen carry
  keywords, is anything staged — are known *before* the block is laid out. The live line's height is a galley
  measured from the font it is painted with, so a taller face cannot push the field out. This is the same discipline
  as `sidebar_foot_height`, and it needs no extra frame: an auto-sized panel is the thing that hangs below its own
  clip on the first frame.
- **The keyword band scrolls instead of growing, which is also the better behaviour.** Two chip rows, in a
  `ScrollArea::vertical().min_scrolled_height(0.0)` — the fork's scroll area has a 64-point floor of its own, taller
  than the band, and a band that ignored the height it was given pushed the field out of the panel on the first
  attempt. The warrant is not only the height budget: an AI pass can leave sixty keywords on one frame, and a band
  that grew with its content would resize the photograph every time the queue moved. Culling wants a still canvas.
- **Two live lines became one, and that is a real fix rather than a saving.** `state.announce` was never cleared, so
  after the first decision the announcement sat under the queue position for the rest of the session — saying the
  same kind of thing twice and costing the photograph a row. The foot now paints `state.announce.take()` when there is
  one and the queue position otherwise: the announcement is spoken in the frame it is made, and the position returns
  behind it. Deciding happens after the strip has painted and before the band, so the queue never advances underneath
  a row that is still being laid out.
- **The test is the arithmetic's audit, and it fails on the old layout.** `the_cull_foot_stays_above_the_bottom_bar_at_nine_hundred_points`
  paints the real shell at 1440×900 with a live session and two keywords on screen, then: (1) every cull text whose
  rect starts in the column must be *visible* — its rect intersected with the clip rect it carries — and end above the
  bottom bar, with the live line and the `sea`/`portrait` chips and the `Add` button all on screen (on the old layout
  the live line was below the fold and the field's own glyphs were clipped); (2) the photograph still has room to
  itself (the canvas rect, found as the largest rectangle filled with the thumbnail well's colour, is over 300 points
  tall), so a "fix" that reserved the whole column for controls fails; (3) sixty keywords paint the canvas at exactly
  the height two did, the last chip is *not* visible, and a wheel over the band brings it into view across the frames
  smooth scrolling eases through — so the band is a scroll and not a clip. Non-vacuity was checked by shrinking the
  panel by 40 points: the test then fails on the live line, exactly where the old code failed.
- **Visual verification**: `cull-charcoal` and `cull-paper` regenerated and read — `Up next` with its four cells, the
  decisions and the four export/apply buttons, the `sea ×` chip, `Add keyword to this …` with its `Add` button and
  `Photo 1 of 12` are all inside the column with clear space above the bottom bar, and the photograph is ~40 points
  shorter, which is the room the field had been hiding in. `places-charcoal`/`-paper` were re-painted in the same run
  for a reason that is not this slice: they had been committed with the status line one point off what this machine
  paints (a rounding difference, visible as a 1-point vertical shift of the credit line plus scattered subpixel AA)
  and `places-charcoal` was already failing `cargo xtask screenshots` on `HEAD` before this change. All other baselines
  are byte-identical.
- Files: `crates/ui/src/shell.rs` (`cull_view`'s foot panel, `cull_foot_height`, `cull_chip_band`, `CULL_UP_NEXT`,
  `CULL_CHIP_ROWS`, the keyword band's scroll area, one merged live line, one new test),
  `crates/ui/tests/snapshots/cull-{charcoal,paper}.png` and `places-{charcoal,paper}.png` (regenerated), and the
  reconciled `docs/UX.md` §5.5 + `docs/ROADMAP.md` (3B.6 (f) ticked; (g) left open in its own row rather than folded
  into the ticked one).
- Gates: `ci`, `snapshot`, `deny` and `screenshots` (16 scenes) all green; 129
  `sortlens-ui` unit tests, the new one among them.
- **Next**: 3B.6 (g), `←`/`→` as text in the keymap overlay's keycaps and the registry's `shortcut` strings, where no
  bundled face has the glyph — a fix that has to touch `crates/ui/src/keymap.rs`, `crates/engine/src/command.rs` and
  the CLI goldens together. Phase 4 items 16–18 after it.

## 2026-11-23 (forty-second) — Phase 3B.6 part 7: the canvas header keeps its name whole

- **The defect was a layout that could not refuse.** The header was one `ui.horizontal` with the title and the live
  count on the left and a `Layout::right_to_left` child for the controls. In egui a `horizontal` hands its last
  child whatever the earlier ones left over, so when that number ran out the right-to-left row was laid out from a
  cursor already past the right edge. At 900×620 with both docks open the canvas is ~310 points wide and the row
  needs ~390: the title was painted at 287–335 *underneath* the sort combo at 283.5–348.5, and the count underneath
  the scope chip. Reproduced exactly, on the old code, by the new sweep test before the fix.
- **The fix is the fork's own `Sides` row, and it measures rather than hopes.** The row is now
  `egui::containers::Sides::new().spacing(6.0).shrink_left().truncate()`: the controls are laid out and measured
  first, the left block's max rect is the remainder, and the words give way instead of the controls. An overlap is
  no longer a width-specific slip waiting to happen — there is no width at which the left block may enter the
  controls' rect.
- **The degradation is stated and measured, in that order.** The count's galley is measured against the room left
  beside the name and the count steps aside when it will not fit (it is repeated by the filter strip's own summary
  and spoken by the live line); the two chips are offered only above `HEADER_CHIP_ROOM` (430 points) because the
  scope is also cleared from the sidebar and the active-filter count is also on the top bar's Filter chip, while the
  sort combo and the two grid glyphs exist nowhere else on that line. Measured over the sweep: with a fresh
  `UiState` per width the count appears from a 942-point window and the chips from 1059, so at 900 the header is
  exactly the view's name, the sort and the two grid glyphs. A header with room to spare still shows all four — the
  thirteen 1440×900 baselines are byte-identical, which is the evidence.
- **The test is a sweep, and it fails on the old layout.** `the_canvas_header_title_never_runs_under_the_sort_combo_at_any_width`
  paints the real shell at **every window width from 900 to 1300 points in one-point steps**, each from a fresh
  `UiState` (a reused one carries the panel sizes the previous frame left behind, which would make the sweep depend
  on how many frames came before it). Each step asserts: the sort combo is painted; the title is painted, is *not*
  truncated, lies inside its own clip rect, and intersects neither the combo nor either chip; and the count, when
  painted, intersects nothing. The canvas is identified as the clip the sort combo was painted in, because the
  sidebar paints its own `Library` row on the same line. Old code: `at 900 points the title at [[287.0 72.0] -
  [335.0 87.0]] is painted over Added order at [[283.5 73.5] - [348.5 82.5]]`. Non-vacuity is asserted in both
  directions: the count and the chips must be absent at 900 and present at 1300, so a header that dropped them
  everywhere fails as loudly as one that kept them in a row they do not fit.
- **Visual verification**: `narrow-canvas-charcoal` and `narrow-canvas-paper` added at 900×620 with both docks open,
  scoped to the library folder, and read: `Library` whole at the left, the density and layout glyphs and the
  `Added order` combo beside it with clear space, no count and no chips, and the breadcrumb, strip and grid below
  them unchanged. Every other baseline was regenerated under `--update` and none was rewritten — the change is
  invisible at 1440 by construction, and the new pair is where it is legible.
- Files: `crates/ui/src/shell.rs` (`canvas`'s `Sides` row, `HEADER_CHIP_ROOM`, `HEADER_COUNT_GAP`, one new sweep
  test), `crates/ui/tests/screenshots.rs` (the new scene), `crates/ui/tests/snapshots/narrow-canvas-{charcoal,paper}.png`
  (new), and the reconciled `docs/UX.md` §5.1 + `docs/ROADMAP.md` (3B.6 (e) ticked, (f)–(g) left open in their own
  row). `docs/UX.md` §5.1's diagram was stale in a second way — it showed the count at the right edge and no chips —
  so it now shows the shipped row and states the degradation order.
- Gates: `ci`, `snapshot`, `deny` and `screenshots` (16 scenes) all green; 128
  `sortlens-ui` unit tests, the new sweep among them.
- **Next**: 3B.6 (f), the cull keyword row clipping under the bottom bar at 900 px, then (g), the `←`/`→` text in
  the keymap overlay and the registry's `shortcut` strings; Phase 4 items 16–18 after them.

## 2026-11-23 (forty-first) — Phase 3B.6 part 6: the strip's count gets a column instead of a chip's lap

- **The defect was in the layout, not in the drawing.** `filter_strip` ended its `horizontal_wrapped` row with a
  `Layout::right_to_left` child holding `N shown`. A wrapped layout hands its last child the *row's* width, so that
  label right-aligned itself against the card's edge from wherever the wrap cursor happened to be — and when the
  row's last chip already reached into the label's own width, the two were painted on top of each other. That is
  what the `short-sidebar-*` baselines showed: `Clear` at 873–935 and `12 shown` starting at 905.
- **Reproduced at a width the test controls.** The full-shell scene at 1280 was not a reliable reproducer, because
  a chip's 58-point button either fits on the row or wraps to the next one; the collision only happens in the narrow
  band where it *just* fits. So the test drives the real `filter_strip` through `Context::run_ui` at **every width
  from 320 to 760 points in one-point steps** and asserts the invariant at each: no painted text in the strip
  intersects the count, and the count lies inside the clip it was painted with (a fix that hid it would fail). On
  the old code it fails at 326 points with `2 shown at [[276.8 51.5] - [316.8 60.5]] is painted over ⚠️ at
  [[265.0 52.5] - [278.0 63.5]]` — the `Missing` chip's own glyph, not merely a word. The sweep is not vacuous: at
  least one width must have the chips on more than one row (grouped in 10-point bands, because a font's own 1-point
  centre differences are not rows) and at least one must put the count's centre inside the chips' band.
- **The fix reserves the column before anything wraps.** The count's galley is measured with
  `layout_no_wrap(…, FontId::proportional(11.0), t.text_faint)`, the strip's max width is reduced by that plus an
  8-point gap, and the chips wrap inside the remainder; the label is painted into the right of the card at the
  chips block's vertical centre. Because the space is taken away *before* wrapping, there is no width at which the
  two can meet, and the chips' own wrapping is unaffected on a wide window (every 1440×900 baseline diff is exactly
  the count's 47×10 rect). Two edges are deliberate: `set_min_width` restores the card's full width, because a
  narrower content rect would otherwise shrink the card to its widest row; and a single chip wider than the column
  (a long collection name) is clipped at the column boundary with `shrink_clip_rect` rather than painted across the
  count. Below `STRIP_CHIP_ROOM` (180 points of chip room) the count steps aside and the chips keep the full width:
  a chip is a control, the count is a statement.
- **Visual verification**: all 29 baselines regenerated and read. `short-sidebar-charcoal`/`-paper` (1280×620) are
  the defect's own scene: `Clear` now wraps to a second row at the left, `12 shown` sits in its column with clear
  space around it, and the strip is one row taller (the grid moves down a row). Every other scene's diff bbox is
  only the count's own rect — measured with `ImageChops` per file, not assumed; `filters-*` and `no-matches-*`
  already had two rows and only the count's vertical position moved.
- Files: `crates/ui/src/shell.rs` (`filter_strip`'s reserved column, `STRIP_CHIP_ROOM`, one new sweep test), all 29
  `crates/ui/tests/snapshots/*.png` (regenerated), and the reconciled `docs/UX.md` §5.2 + `docs/ROADMAP.md`
  (3B.6 (h) ticked, (e) named as the next slice).
- Gates: `ci` (fmt had to be re-run first), `snapshot`, `deny` and `screenshots`;
  127 `sortlens-ui` unit tests, the new one among them.
- **Next**: 3B.6 (e), the canvas header's title under the right-aligned sort combo at 900 px with both docks open,
  then (f) and (g); Phase 4 items 16–18 after them.

## 2026-11-23 (fortieth) — Phase 3B.6 part 5: the sidebar scrolls above its foot instead of over it

- **The defect, reproduced before it was fixed.** At 1280×620 the left panel holds six scopes, two subfolders,
  `FOLDERS`, twelve keyword rows and `COLLECTIONS`; the foot claims the last ~133 points, and egui happily laid
  the sections out past the hairline, inside the panel's clip rect but *over* the four utility rows. The new test
  failed on exactly that before the fix: `keyword kw04 is visible at [[45.0 446.5] - [73.0 455.5]], inside the
  foot's band ([[0.0 451.0] - [264.0 580.0]])`.
- **The fix is a clip, not a guess.** Everything below the `Library` header now lives in one
  `egui::ScrollArea::vertical()` with `.id_salt("sidebar_sections")` and `.auto_shrink([false, false])`, so the
  viewport is exactly the room the foot left. The foot is still laid out *first* (its `Panel::bottom` claim and
  its exact height arithmetic are unchanged), which is what makes the viewport's bottom edge the hairline; a row
  that does not fit is now scrolled rather than drawn across the utilities. `auto_shrink` keeps the bar absent
  while the sections fit, so nothing changes on a tall window.
- **Two halves of the claim, because hiding is not fixing.** The test paints the whole window at 1280×620 over a
  library tagged with twelve keywords, and then: (1) **nothing from the sections is visible in the foot's band** —
  "visible" is each painted text rect intersected with the clip rect its shape carries, so a row the scroll area
  pushed past the viewport does not count while a row drawn over the utilities does; the four foot labels must all
  be *readable* there, so a fix that clipped the footer too would fail the same test; and (2) **the sections
  scroll** — a `MouseWheel` event over the sidebar brings a keyword that started below the fold into view, and
  `Recently Deleted` does not move a point. The test refuses to pass vacuously: it asserts that fewer keyword rows
  are visible than exist, so growing the window instead of fixing the layout says so out loud.
- **Visual verification**: `short-sidebar-charcoal` and `short-sidebar-paper` added at 1280×620 and read — the
  sections are cut cleanly at the hairline, the four utility rows sit on the bottom edge with clear space, and no
  section text is drawn across them. `filters-charcoal`/`-paper` and `no-matches-charcoal`/`-paper` (1200×800)
  were regenerated and read: their only change is that `+ New collection` — which used to sit *on* the hairline — is
  now clipped above it. Every 1440×900 baseline is byte-identical, which is the evidence the tall window is
  untouched. The five `places-*` `.diff.png`/`.old.png` leaves from an earlier failed comparison were removed.
- **One defect found while reading the new baselines, not fixed here**: at 1280 points with both docks open the
  filter strip's right-aligned `N shown` summary is painted *under* the `Clear` chip, so the two overlap. It is
  recorded as ROADMAP 3B.6 (h) and is the next slice; it is visible in the new `short-sidebar-*` baselines on
  purpose, because hiding it by choosing a flattering window width would make the baseline a claim rather than
  evidence.
- Files: `crates/ui/src/shell.rs` (`left_panel`'s scroll area, one new test), `crates/ui/tests/screenshots.rs`
  (the new scene), `crates/ui/tests/snapshots/short-sidebar-{charcoal,paper}.png` (new),
  `crates/ui/tests/snapshots/{filters,no-matches}-{charcoal,paper}.png` (regenerated), and the reconciled
  `docs/UX.md` §4.4 + `docs/ROADMAP.md` (3B.6 (d) ticked, (e)–(g) left open in their own row, (h) opened).
- Gates: `ci`, `snapshot`, `deny` and `screenshots` all green; 127 `sortlens-ui` unit
  tests including the new one, and the workspace suite has no failures.
- **Next**: 3B.6 (h), the strip's `N shown` summary colliding with `Clear` at 1280, then (e), (f), (g) in that
  order; Phase 4 items 16–18 after them.

## 2026-11-23 (thirty-ninth) — Phase 3B.6 part 4: Places has a coast before you ask for one

- **The outline ships, and it is the one the project may ship.** `sortlens_geo::coastline` embeds
  `assets/coastline.txt` with `include_str!`: the Natural Earth **1:110m coastline**, which is public domain, so the
  earlier "no bundled basemap" decision (ROADMAP part 3a) was not overturned but narrowed — satellite and street
  tiles still need data the project cannot ship, while a world outline in the public domain does not. 134 shorelines,
  5,128 points, rounded to 0.001° and one polyline per line, with the source, the transform and the licence in the
  file's own header. 77 KB of text, inspectable in a diff, and no fetch at run time.
- **The antimeridian splits a line at parse time.** The source happens to keep no walk across ±180°, but a parser that
  joined one would paint a stripe through every other continent; two consecutive points more than 180° apart now end
  the run, and the test feeds exactly that walk and asserts two shores and no segment wider than 100 px at zoom 2.
- **Antarctica is clamped, and only Antarctica.** Six source points sit below −85.05°, where Mercator diverges.
  Clamping them to the limit draws the continent's edge along the projection's own border; the alternative is a
  missing continent. A *photograph* beyond the limit is still refused rather than clamped (that decision is unchanged
  — a coordinate is a claim about a photograph, a coastline is a drawing of land), and the test walks every projected
  point and asserts it is inside the world square.
- **Projection happens once, at parse time.** `screen_lines` scales zoom-0 world pixels by `2^zoom` and subtracts the
  viewport origin, so five thousand tangents and logarithms are not computed per frame; and it returns only the runs
  of consecutive segments whose box touches the viewport, so the caller gets geometry it can hand straight to a
  painter instead of a world-sized polyline for egui to clip. An impossible viewport (zero width, non-finite origin)
  returns nothing instead of panicking in a paint path.
- **Refusals are counted, not swallowed.** `Coastline::skipped()` counts a malformed token, a coordinate off the globe,
  and a line left with one point. The shipped file asserts **zero** — that is the test that would catch a truncated
  `include_str!`, a lon,lat export, or a rounding pass that wrote `NaN`.
- **Places opens fitted, which was the reported defect.** `coastline::bounds_centre` is the middle of the photographs'
  projected box, not the mean of their latitudes: Brighton and Lima average 19°N, 1,400 km north of the box's middle,
  and the old mean-based centre pushed the northern dot towards the edge of its own fit. `coastline::fit_zoom` picks
  the deepest zoom at which the whole spread fits inside the viewport with 28 px clear on every side, never deeper
  than the caller's ceiling (a tile folder's top zoom) or `MAX_FIT_ZOOM` (10, because the outline is 1:110m and a
  deeper view would be a staircase). `UiState::places_zoom_chosen` is the flag that separates the view's fit from the
  person's choice: the fit runs every frame until `+`/`−` is pressed, after which the zoom is theirs and a library
  that grows does not move their map.
- **The view draws one basemap, never two.** A usable `tiles/` folder is painted as before and the shipped outline is
  skipped; with no folder the outline is stroked in the new `Tokens::map_coast` (one value per preset, geography
  rather than chrome) under the dots, and the credit line swaps to `Coastline: Natural Earth 1:110m (public domain)`.
  The "no basemap is installed" copy is gone because that state no longer exists: it now says the coastline is
  schematic and names the `tiles/` folder that would replace it.
- **Tests that fail when the behaviour is absent** (verified by two mutations, both reverted): hardcoding `fit_zoom`
  to 5 fails `fitting_the_view_puts_the_photographs_on_screen_where_a_fixed_zoom_loses_them` (which first asserts the
  old zoom really did lose both dots) and the new shell test on its first assertion; skipping the coastline paint
  fails `places_reports_the_basemap_or_why_there_is_none` on "a library with no tile folder still gets the shipped
  coastline" and the new shell test on "the shipped outline is the basemap when no tiles are installed". The stale
  full-output `paint` closure in the basemap test now returns its output so the same test can assert both halves —
  outline present without tiles, absent with them.
- **The new `run_ui` test** paints Places at 1200×800 over a library in Brighton and Sydney: the view chose a zoom
  below 5, exactly two filled dots (radius `dot_radius(1)`) are painted and both centres are inside the map rectangle
  (a dot's ring is a second circle, which is why the count filters on the fill), the coastline runs are strokes in
  `map_coast` with at least one longer than four points and every point finite, a manually chosen zoom 9 survives the
  next frame, and a library with no location paints neither outline nor dot.
- **Visual verification**: `places-charcoal` and `places-paper` regenerated with `cargo xtask screenshots --update`
  and read — both show the coastlines under the Brighton and Lima dots, the new credit in the footer, and the schematic
  notice naming the tiles folder; the scene now pins zoom 3 *and* `places_zoom_chosen`, because a baseline must not
  move when the fitting arithmetic is touched. No other baseline moved.
- Files: `assets/coastline.txt` (new), `crates/geo/src/coastline.rs` (new, 7 tests), `crates/geo/src/lib.rs`,
  `crates/geo/src/tiles.rs`, `crates/ui/src/theme.rs` (`map_coast`), `crates/ui/src/shell.rs` (the fit, the coastline
  pass, the credit, the status copy, `places_zoom_chosen`, `coastline_runs`, one new test, one extended),
  `crates/ui/tests/screenshots.rs`, and the reconciled `docs/{ROADMAP,UX,ARCHITECTURE,ATTRIBUTION,RELEASE}.md` +
  `README.md`. ROADMAP's 3B.6 row is split again: (c) ticked, (d)–(g) still open in their own row, nothing moved into
  an absent table.
- 714 tests (seven geo, one shell new, one shell extended); ci, snapshot, deny, screenshots, a11y, docs,
  layers, roots, assets, attribution green.
- **Not verified by hand**: the outline at a zoom where its 0.001° steps are visible as staircases (the arithmetic
  says zoom 8+ and the Paper baseline is the deepest read), a library whose photographs ring the globe at ±180°
  (the fit would choose zoom 0 and the world square would sit inside the viewport with sea beside it), and whether a
  two-line `Basemap` footer plus the `places.txt` line crowds the map at 620 px of height — that is defect (d)'s
  neighbourhood, not a claim made here. **Also visible in the baseline and left as it is**: at 1:110m a coastal city's
  dot can sit a few pixels offshore (Lima's does), because the outline is a world map's coast and not a harbour chart —
  which is why the credit says 1:110m rather than claiming a map.

## 2026-11-23 (thirty-eighth) — Phase 3B.6 part 3: a library filtered to nothing says so, instead of greeting you

- **The empty canvas is decided by two counts, not one.** `canvas` already carried the filtered count, and the
  empty screen derived its stage from `engine.stats().total` — so the question "is there a library" and the question
  "is there anything to draw" were answered by the same number, and a filter that hid all twelve photographs
  reached the Welcome card: *Welcome to Sortlens · Point it at a folder…* with `Add Folder…` as the primary action,
  the one action that could not help. The new `canvas_empty(roots, scanning, library, shown, dismissed)` returns a
  four-way `Empty` — `Add`, `Indexing`, `NoMatches { library }`, `Quiet` — so the wizard is gated on the catalog and
  the no-matches screen on the filtered count. `Welcome` is gone as a name because the canvas is no longer decided
  by the wizard alone.
- **`Not now` was a button that did nothing.** `empty_library` read `welcome_stage(...).unwrap_or(Welcome::Add)`, so
  the `None` a dismissed wizard returned was turned straight back into the wizard it had just refused. Dismissal now
  wraps the two wizard stages only; `NoMatches` ignores it, because a screen that explains why the grid is blank is
  not an interruption to be dismissed. Both halves are pinned by the stage table, and the old test that asserted
  `welcome_stage(0, false, 0, true) == None` was asserting the function's intent while the caller overrode it.
- **The screen itself**: "Nothing matches these filters" with the library's own count in words ("12 photographs in
  this library, and the current filters exclude every one of them") and one `Clear filters` button in the same shape
  as the Welcome card's, so the two empty states are one family. The button clears the search text *and* every
  filter dimension and forgets the folder scope — exactly what the header's `N active` badge already counted, and
  the reason a search-only zero-result also gets this screen rather than a mystery. The count is never the filtered
  zero: the sentence is about what the filters cost, so it has to name what they cost.
- **Tests that fail when the behaviour is absent** (one rewritten stage table + one new `run_ui` test; verified by
  changing `Empty::NoMatches { library }` back to `Empty::Add`, which fails the new test on "the canvas says why it
  is empty instead of greeting the person"): the table covers the empty catalog, an empty root, a running scan,
  photographs shown, dismissal over both wizard stages, and a filtered-to-zero library both dismissed and not. The
  window test scans two real JPEGs, narrows with `min_rating = 5`, asserts the card and the "2 photographs in this
  library" sentence are painted and *"Welcome to Sortlens" is not*, then presses the button with a real pointer
  event and asserts every dimension cleared and both photographs are back; it then narrows with search text alone
  and asserts the same screen offers the same way back. The other half — an actually-empty catalog — asserts the
  Welcome card and that no filter screen is painted over it.
- **Visual verification**: a new `no-matches-charcoal`/`-paper` baseline pair at 1200×800 with both docks open,
  generated by `cargo xtask screenshots --update` and read. Both show the strip above saying `Videos · 0 shown` and
  `2 active` (the folder scope plus the kind filter), the card, the library's 12, and the button — and neither shows
  any part of the Welcome card. No existing baseline moved, which is the check that the change is confined to the
  canvas with nothing to draw.
- 706 tests; ci, snapshot, deny, screenshots (14 scenes, 31 PNGs) all green. `docs/ROADMAP.md` gained the
  3B.6 (b') row and its stale "Eighteen PNGs" count, `docs/UX.md` §5.12 now says which two empty states exist and
  which count decides each, and `docs/ARCHITECTURE.md`'s screenshot paragraph lists the sixteen surfaces.
- **Not verified by hand**: the card at the minimum window size, where the two-line sentence and the button still
  fit (the rationale is arithmetic, not a picture), and the copy's tone in the Paper preset beyond the baseline.

## 2026-11-23 (thirty-seventh) — Phase 3B.6 part 2: the strip asks in pictures, and only when there is something to narrow

- **The strip is not painted on an empty library, and "empty" is the catalog, not the filter.** Over a first-run
  library it was nine controls asking questions about photographs that do not exist yet, sitting on top of the
  Welcome card whose whole job is to say where photographs come from. The gate is `engine.stats().total`, never the
  filtered count `canvas` already has: a filter that hides every photo must keep the strip, because clearing it is
  what the strip is for. The rule lives at the call site, so the Filter toggle in the top bar can still be on when
  there is nothing to show — and the moment a photo is indexed the strip appears at that same toggle state.
- **The rating chip answers in stars, not in `3★ and up`.** One star glyph per star of the minimum, tinted `accent`,
  in a themed chip; at rest it is the old faint star plus the word "Rating", because a chip with no picture and no
  word would be a mystery. The typed `★` is gone from the strip — a character the bundled faces render at different
  widths and heights and not at all in some — and so is the numeral, because the count was the answer all along.
  The cycle (0→1→…→5→0) is unchanged: the pictures changed, the behaviour did not.
- **The colour chip paints the colour.** A filled 4.5-point dot in the chosen label's own token colour, an outlined
  ring for "any colour", and the colour's name beside it (a dot alone is not an answer for a colourblind reader).
  The `droplet` glyph that stood in for every colour is out of the strip.
- **The flag chip wears the state's own glyph** — `pick`, `reject` or `flag` — through `flag_filter_icon`, a pure
  mapping with its own assertions, so the four states are pictures before they are words. The keyword menu gained
  the set's `chevron`, because it is a menu and now looks like one.
- **The row wraps instead of overflowing.** Nine chips plus a five-star minimum are wider than the canvas at 1200
  points with both docks open, which pushed `Clear` — the one control that undoes the strip — past the canvas edge.
  `horizontal_wrapped` turns that into a second line. This is a defect the new test found rather than one the note
  named: the assertion "the strip stays when a filter hides everything" failed because `Clear` was never painted at
  all, and the frame's own text dump showed the row ending at `Missing`. A five-star chip has the same width as the
  old `5★ and up` label, so this predates the slice; it is fixed here because a filter bar that cannot be cleared is
  worse than a taller one.
- **Tests that fail when the behaviour is absent** (3 new, 1 updated; verified by removing the gate, which fails
  `the_filter_strip_needs_a_library_to_narrow` on its first assertion): the strip absent over the Welcome card and
  back after one indexed photo, still present — and still counting — when the filter answers nothing; painted alone,
  the rating chip at three stars paints exactly three 12-point glyphs in a row and no `★` or numeral, the colour
  chip's circle is `t.label(Green)` filled and the unset one is an outline; clicks on the painted star and on the
  swatch drive `min_rating`/`label_filter` (a `Button` atom really is the button). Two new test helpers flatten
  `Shape::Vec` (`all_shapes`, `painted_pictures`, `painted_circles`), because a picture inside a button is nested and
  a top-level scan reads as "nothing painted" — the trap the existing `label_at` already avoided for text.
- **Visual verification**: a new `filters-charcoal`/`filters-paper` baseline pair at 1200×800 with three stars and a
  green label — the only baselines that show the strip *asking* something. Both read: three accent stars in the
  chip, a green swatch beside the word "Green", and `Missing | Clear` wrapped onto a second line with `2 shown`
  right-aligned under the first row. All 26 existing baselines regenerated; the Welcome pair lost the strip (the
  fix) and every other scene's only change is the strip's own glyphs. The narrow 900×620 baseline still fits on one
  row, so wrapping costs nothing where it is not needed.
- Files: `crates/ui/src/shell.rs` (the gate, `stars_atom`, `rating_chip`, `colour_chip`, `flag_filter_icon`, the
  chevron, `horizontal_wrapped`, the test helpers and four tests), `crates/ui/tests/screenshots.rs` (the filter
  scene), `crates/ui/tests/snapshots/*` (28 regenerated, 2 new), and the reconciled `docs/{ROADMAP,UX,ARCHITECTURE}.md`
  — the ROADMAP row is split so (a) and (b) are ticked and (c)–(g) stay open in one row, with no work moved into an
  absent table.
- 705 tests (3 new); ci, snapshot, deny, screenshots, a11y, docs, layers, roots, attribution green.
- **Not fixed, and observed**: with a filter that hides every photo the canvas shows the *Welcome* card ("Add
  Folder…") under the strip, because `empty_library` derives its stage from the filtered count. The strip is now
  right (it stays) but the copy under it is not — that is the "nothing matches these filters" empty state, and it is
  the next thing in this row's neighbourhood rather than a claim made here. Also unverified by hand: the wrapped
  row at a width where only one chip fits per line (the chip row would become a column of chips), and whether a
  five-star minimum plus a wrapped `Clear` reads as one control group at 1200 points.

## 2026-11-23 (thirty-sixth) — Phase 3B.6 part 1: the Welcome promises start with pictures

- **The three first-run promises carry the icon set's glyphs, not a typed bullet.** `folder` (nothing is uploaded,
  renamed or moved), `history` (one log you can step back) and `reject` (culling stages the decision — the same
  glyph the sidebar's Rejects row wears, so the vocabulary is the app's and not a new one). The `·` it replaced was
  typed text at 12 points: one speck whose height and weight differ per bundled face, which is exactly the kind of
  mark that reads as a typo rather than as a list.
- **A fixed 16-point glyph column**, `WELCOME_GLYPH_COLUMN`, holding a 14-point glyph centred at `left + 8`. The
  three rows use `ui.horizontal_top`, so the glyph sits on its title's line rather than floating between the title
  and the detail line, and every detail line starts at the same x — the same rule `nav_row`'s glyph column follows
  in the sidebar. The glyph is tinted `t.accent`, the colour the typed bullet had, so the rows still read as one
  accent-marked list in both presets.
- **One test, exact rather than a floor.** `the_welcome_rows_start_with_the_icon_sets_glyphs` paints `welcome_add`
  alone in a `Context::run_ui` (installing the SVG loader itself, because it does not go through `show`), then
  counts the textured rectangles: exactly three, each 14×14, all at one x, increasing down the page, and each one
  left of and on the same line as its own title (`label_at`). It also asserts no item containing `·` is painted —
  the regression the slice exists to prevent. Verified failing with the paint removed: `left: 0, right: 3`.
- Files: `crates/ui/src/shell.rs` (the rows, the constant, the test), the regenerated
  `crates/ui/tests/snapshots/{welcome-charcoal,welcome-paper}.png`, and the reconciled `docs/{ROADMAP,UX}.md`.
- **Visual verification**: `cargo xtask screenshots --update`; only the two Welcome baselines changed (every other
  baseline byte-identical, so the slice touched nothing else), and both were read. The charcoal and paper frames
  show the three accent glyphs in one column, each beside its title, with the `·` gone and the copy unmoved.
- 702 tests; ci, snapshot, deny, screenshots, a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: the glyph column at `pixels_per_point` 2 and the rows with a detail line that wraps
  (both details are one line at 420 points today, and a wrapped one would leave the glyph beside the title only).
  Both are named here rather than claimed fixed.

## 2026-11-23 (thirty-fifth) — Phase 3B.5: the fourth question, and a rail that never goes away

- **`RightTab::ALL` is four.** Details, Tags, Suggestions, Activity — Suggestions between Tags and Activity, which is
  the order `docs/UX.md` §6 has claimed since the panel was designed and which the enum now enforces because the rail
  paints in `RightTab::ALL` order. The panel's own tab row is gone: it was four labelled buttons that ate the top of a
  250–420 px column to say what a glyph now says.
- **The far-edge rail.** `right_tools` is a 30-point `Panel::right` called *before* `right_panel`, so it owns the
  window's edge and the body sits inside it. It paints whether `state.right_open` is true or not — that is what makes
  it a rail and not a tab bar — every entry is `Button::image` with `Role::Tab` and the tab's name as its accessible
  name and tooltip, and the active one carries the selection fill. Switching tools keeps the body open; clicking the
  *active* tool closes it. That second behaviour is the one design decision here: the pattern's whole promise is "the
  panel never disappears when you switch tools", and a rail with no way to close the body would hold 340 points of a
  900-point window forever. The panel header now prints the active tool's name in words, because the rail is glyphs.
- **The Suggestions tab** answers for the whole selection, not one photo: `suggestions_for_all` is merged into one row
  per keyword with the best confidence any selected photo offered and the number of photographs it applies to. Rows are
  grouped into three families — Objects (`Detector`), Scene (`Pack`/`Caption`), EXIF (`ExifHint`) — because "the camera
  said so" and "a model guessed" are different kinds of claim, and one family list would ask a person to trust them
  equally. Each row carries `+ keyword`, its confidence, its source, and `×`; each family carries `Accept all`. The
  count prints only when the proposal is *partial* (`1×` of a three-photo selection) — `3×` on every row is noise.
- **Accepting a kind is one ⌘Z.** `write_tags` replaced the body of `set_tag_from` and now takes a list of keywords, so
  `accept_suggestions(ids, tags)` is the same write as typing one keyword, N times in one `Op::Group` — the words first,
  then their `Ai` provenance. `set_tag_from` keeps its exact behaviour (`last_tag` set whenever `add`, even when nothing
  changed; sidecars synced unconditionally) by delegating and doing those two things itself; the positive path returns
  `(photos changed, keywords added)` so the notice can say how many keywords, not how many rows.
- **Dismissal is stored, because a proposal is derived.** `Photo.dismissed_suggestions: BTreeSet<String>` with
  `#[serde(default, skip_serializing_if)]`, and `Op::SuggestionDismissed { changes: Vec<(PhotoId, Vec<String>)> }`
  carrying each photo's complete new set — the same read-then-replace shape as `Op::Tagged`, so the inverse is the
  previous set and one undo returns the proposal. `catalog::dismiss_suggestions` records only where something is
  actually new (dismissing twice is `Ok(0)` and spends no undo level); `suggestions_for` filters dismissed candidates
  *before* `merge` applies the eight-slot cap, so a refused proposal cannot occupy a live one's place. The revision is
  deliberately not bumped and nothing reaches a sidecar — `docs/BEHAVIOUR.md` §4 says both, next to provenance, for the
  same reason: another application must not inherit our opinion about what to offer.
- **Two doors onto it, not one.** `suggest.dismiss` is a `Ready` registry command (`{ids, tag}` → `dismissed`), with an
  MCP example and a `sortlens-cli dismiss <ids> <tag>` verb (`--json` schema included), because a decision that lives
  in the catalog and is undoable is a library change and the registry is where library changes are described. The CLI's
  non-JSON form prints how many photographs newly stopped being offered it.
- **A new glyph: `wand`.** Suggestions are a proposal, and reusing `sparkle` (memories) or `tag` (keywords) would have
  made the rail say something it does not mean. 40 glyphs now; the enum, the catalog, the folder and the rasterise
  tests all walk the new one.
- Files: `crates/catalog/src/{photo,op,store}.rs`, `crates/engine/src/{lib,command}.rs`, `crates/mcp/src/examples.rs`,
  `apps/sortlens-cli/src/main.rs` (+ two regenerated goldens), `crates/ui/src/{shell,icons}.rs`,
  `assets/icons/wand.svg`, `crates/ui/tests/screenshots.rs`, `crates/testkit/src/lib.rs` (`Camera` +
  `jpeg_with_camera`), and the reconciled `docs/{ROADMAP,UX,ARCHITECTURE,BEHAVIOUR,PACKS,CLI}.md`.
- Tests: catalog (dismiss → reopen → dismissed, one undo for the batch, redo, `Ok(0)` on a repeat, refusals for an
  empty keyword and an unknown photo, and a snapshot that never mentions the field when nothing was dismissed); engine
  (a dismissed proposal is not offered while the rest are, the cap counts live proposals only, accept-two-kinds on
  three photos is six keywords in one ⌘Z that undo removes wholesale, dismissal is one op and undo returns it); UI
  (`Context::run_ui`: the rail holds four glyphs, clicking the third opens Suggestions *with the panel still open*,
  switching to Tags keeps it open, clicking the active tool closes the body while the rail stays, and nothing asks the
  host for anything; the Suggestions tab paints `Scene` for a pack, `+ jpeg` on click tags all three selected photos
  with `TagSource::Ai`, one undo empties them, `×` dismisses for all three and the engine stops proposing it).
- **Visual verification**: `cargo xtask screenshots` — two new baselines (`suggestions-charcoal`/`-paper`) and all 25
  existing ones regenerated and read. The lab book was checked against the fixture: the fixture gained a keyword pack
  (`Coast`: `shoreline`/`golden hour` for `Trip`, `studio light` for `Studio`) and two frames with real camera EXIF
  (`Nikon Z6` at ISO 6400/24 mm, `Fujifilm X-T5` at 56 mm), because a fixture JPEG carries no EXIF and one family would
  not have tested a grouping. The baseline now shows Scene 3 + EXIF 5, `1×` on the two Trip-only keywords, `2×` on
  `studio light`, and the detail view's "Suggested" row carrying `+ fujifilm x-t5 + studio light + normal`.
- 701 tests; ci, deny, snapshot, screenshots, a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: the rail at a touch target's real size on a 2× display (14-point glyphs in 24-point rows is
  a mouse target, and a 30-point rail is thinner than the 44-point rule would like), and whether `Accept all` beside a
  long family name crowds the `×` column at the panel's 250-point minimum. Both are named here rather than claimed
  fixed.

## 2026-11-23 (thirty-fourth) — Phase 3B.4: nine destinations, and a bar that says what you are looking at

- **Detail, Compare and Soft Focus are destinations now, not a mode flag and a section.** `View::ALL` is nine:
  Library, For You, Cull, Detail, Compare, Places, Similar, Soft Focus, Insights. `state.detail_open` is gone; the
  question every keyboard guard used to ask is `View::is_detail()`, so the grid's shortcuts and the detail view's
  shortcuts cannot both answer the same press. `state.compare` is `Some` exactly while `View::Compare` is open —
  `activate_view` clears it on every other destination and `toggle_compare` closes the pane rather than the
  photograph — because a second pane that outlived the view drawing it was invisible state nobody could see.
  `open_detail` is the state-only path a grid double-click uses; `activate_view` is the one path for the tabs, the
  palette and the keyboard.
- **Soft Focus left the foot of Insights.** `soft_focus_view` polls the sharpness pass (the view that asked for it
  owns the report), states what a relative cut can claim, and reuses `sharpness_section` unchanged. Insights no
  longer scrolls to a review it was not about.
- **The bar itself.** Labels are measured (`tab_width` sums the real galleys), and `tabs_collapse` turns the row
  into the current destination plus a menu when the tabs and the controls beside them do not fit — the same nine
  items, on the icon set's own new `chevron` glyph, because no bundled face has `▾` (a caret character painted
  tofu in the 900 px detail baseline before this). The centre is an annotation pill: five stars (clicking the lit
  one clears it), Pick/Unflag/Reject, the primary's keyword count and "N selected". The stars and flags call the
  same `rate_selection`/`flag_selection` the `0–5` and `P`/`X`/`U` keys call, so the two cannot drift. The right
  side carries the three-step thumbnail slider in the grid (same `state.density` the canvas-header chip cycles,
  snapping through `Density::index`/`from_index` so there is no fourth step), Fit/1:1 in Detail and Compare plus
  click-zoom on the photograph itself, `?`, and a reduced-motion indicator. The old one-line library summary left
  the bar: the canvas header already says the count and the selection.
- **Every destination is a command.** `view.compare` and `view.softFocus` are new `Ready` rows with `ShowView`
  effects, and `view.places` was still `Planned("M5")` — the map shipped slices ago, so a working view was hidden
  from the palette and from MCP. `M` now opens the map (the keymap overlay lists it, and the CLI's shortcut golden
  grew by that one line), the palette is 24 rows, and Compare is reached through `toggle_compare` in the window
  because only the shell knows which second photograph to show. Examples added for the three new MCP tools.
- Files: `crates/ui/src/shell.rs` (View, `activate_view`/`toggle_compare`/`open_detail`, `soft_focus_view`,
  `bottom_bar` + `view_tab`/`view_menu`/`annotation_pill`/`rate_selection`/`flag_selection`), `crates/ui/src/theme.rs`
  (`Density::index`/`from_index`), `crates/ui/src/icons.rs` + `assets/icons/{compare,focus,chevron}.svg` (39 glyphs),
  `crates/ui/src/keymap.rs`, `crates/engine/src/command.rs` (3 rows), `crates/engine/src/lib.rs` (3 effect arms,
  `view.insights` now names itself)`, `crates/mcp/src/{examples,lib}.rs`, `apps/sortlens/src/main.rs`,
  `crates/ui/tests/screenshots.rs`, and the reconciled `docs/UX.md` §5.6/§5.8/§7 + `docs/ARCHITECTURE.md` §8.1.
- Tests: the pill driven by real clicks on its painted glyphs (rate both photos, the lit star clears, Pick and
  Reject both land, a keyword count joins the pill); `tabs_collapse`'s threshold plus a wide row that names
  Soft Focus and a narrow one that does not, whose menu still holds it; the peer-view invariant and Compare's
  spoken refusal with nothing selected; click-zoom on the largest textured rect of the detail canvas; an engine
  audit that every view destination is `Ready` and sets its `ShowView` effect; the size control's snapping. The
  registry-vs-message milestone test now reads its milestone from the registry instead of hard-coding `M2`.
- 696 tests; ci, deny, snapshot, screenshots (23 baselines), a11y, docs, layers, roots, attribution green.
- **Visual verification**: 20 baselines regenerated and read, plus three new ones: `soft-focus-*` (the peer view
  with stored scores, both themes) and `narrow-charcoal` (900 px with the docks closed) showing the collapsed
  row: the current destination, the chevron, and the pill and controls in their places. The bottom bar is in
  every baseline: nine tabs, the pill, the slider and `?` at 1440 px. The screenshot harness's accessibility
  pass found the sharpness slider unnamed and that is fixed (`a11y::label_widget` with the question it asks).
  Two older defects were seen while painting the 900 px scene and are recorded in `Next` under 3B.6 rather than
  hidden by the new baseline: at 620 px of height the sidebar's foot overlaps its own list, and at 900 px with
  both docks open the canvas header's title runs under the sort combo. Still not verified by hand: the slider's
  feel at the width where the row is about to collapse, and whether a keyword count crowds the pill at 900 px.

## 2026-11-23 (thirty-third) — Phase 3 item 12 (3B.3): the sidebar asks six questions, and the utilities move to its foot

- **`SOURCES` is six rows, one question each.** `Source::ALL` gained `Missing`, so the panel now lists
  All Photos, Recently Added, Picks, Rated, Missing and Rejects — the six scopes `docs/UX.md` §4.1 has asked
  for since the wireframe, of which five existed. Each row's count is computed by `source_count`, which builds
  the filter through `source_filter`: the same function the tests call, so a row's number and the grid that row
  opens are one question asked twice rather than two questions that look alike. `UiState` gained `missing_only`
  (the question, like `recently_added`), `apply_source` sets it, `source_selected` compares it, and
  `active_filter_count`/`has_filters`/`clear_filters` know about it.
- **Missing needed a real filter, and it is a narrowing.** `sortlens_catalog::Filter` gained
  `missing: Option<bool>`: `Some(true)` is only files that are gone, `Some(false)` only files that are there,
  `None` leaves the old behaviour. It outranks `include_missing`, which only widens the view — "everything,
  missing included" and "only what is missing" cannot both be satisfied, and the narrower question is the one
  asked deliberately. `missing` is part of the filter's JSON surface too, so `library.page` accepts
  `{"filter": {"missing": true}}` and the MCP example now says so.
- **The four dialog rows left the middle of the sidebar.** `SIDEBAR_FOOT` is
  `[Deleted, Health, History, Shortcuts]` as `PaletteCommand`s, and `sidebar_foot` paints them in an
  `egui::Panel::bottom` laid out *before* the sections so it claims the bottom edge and nothing can push it
  off. Every row's label and right-hand column come from the command registry (`Recently Deleted` + the bin
  count, `Check Library`, `Recent Changes`, `Keyboard Shortcuts` + `?`), and a click goes through the same
  `PaletteCommand::run` the `⌘K` palette uses — so the row that opens the bin and the palette row that opens
  the bin are one handler, and a fifth row can only be added by naming a command that already exists. The
  words therefore changed with the registry: "Library health" is now "Check Library", "Changes" is "Recent
  Changes" and "Shortcuts" is "Keyboard Shortcuts", which is the point rather than a regression.
- **The strip is sized exactly, and that was a real defect.** The first version let the panel measure its own
  content, which a panel only learns *after* laying it out: on the first frame the four rows hung below the
  panel's own clip rect and the strip flickered into place a frame late. `sidebar_foot_height` now states the
  height from `NAV_ROW_H` (hoisted out of `nav_row`), the item spacing, a 9-point hairline band and the frame's
  margins, and the panel is given it with `exact_size`. The hairline is painted rather than `ui.separator()`:
  that widget's height is a style lookup, and a point nobody counted is a point that clips a row.
- **Not verified by hand**: how the strip behaves at the narrow end of the panel's 210–380 px range with
  "Keyboard Shortcuts" as its longest row (it truncates from the measured width, so it should), and the foot
  over a keyword list long enough to reach it — the sections do not scroll yet, and clipping is unchanged in
  kind, only raised by the strip's height.
- **Named, not hidden**: `docs/UX.md` §4.1 used to imply a per-row "filtered" marker and a `Rejects (bin)`
  predicate that does not exist (the bin is a dialog, and the Rejects row is the *flag*); §4.1 now says what
  ships and names the marker as not shipped, and §4.4 describes the foot and names the storage summary,
  branding block and collapsible/reorderable sections as not shipped. No new verbs: every foot row was
  already a `Ready` registry command.
- **Tests that fail when the behaviour is absent (3 new, 690 workspace total).**
  `only_missing_narrows_while_include_missing_widens` (catalog) pins both directions and the precedence over
  the widening flag; `the_missing_scope_counts_the_files_that_are_gone_and_only_those` removes a real file,
  runs `check_health`, and asserts the Missing row's count, the grid it opens, the badge and that `Clear` leaves
  the scope — removing `missing: source.missing()` from `source_filter` fails it; and
  `the_sidebar_foot_pins_the_utilities_to_the_bottom` paints the whole window at 1280×800 and requires the
  foot's four glyphs to be the last four in the column, nothing else in the band, the sections to end at least
  60 points above it, and *every* sidebar glyph to be inside the clip rect it was painted into — which is what
  the auto-sized version failed. The chrome icon-count test's sidebar floor is now six sources + utilities.
- **Visual verification: all 20 existing baselines regenerated with `--update`, then read, then re-run without
  it (9/9 tests matched).** Two are new: `missing-charcoal` and `missing-paper` — one frame's file removed and
  the health check run, `apply_source(Missing)`, so the sidebar's Missing row counts 1, the filter bar's chip is
  lit, and the grid holds the record of a file nobody can open. The library baselines were read in both presets
  to confirm the six rows, the hairline and the four foot rows with the registry's words.

## 2026-11-23 (thirty-second) — Phase 3 item 11 (3B.2): one field with two modes, and the bar loses its words

- **`⌘K` is the same field, not a second search box.** `UiState` grew `palette_open`, `palette_query`,
  `palette_index`, `palette_focus`, `focus_search` and `palette_anchor`. The top bar draws the photo field or
  the command field in the same 240-point slot depending on the mode, so the palette's rows hang under the
  control the person is typing in. `Esc` leaves command mode and asks the photo field for focus, and the photo
  query is untouched throughout (the test types "beach", opens ⌘K, runs a command, escapes, and finds "beach"
  still there).
- **The palette cannot list a command it cannot run.** `PaletteCommand` is an enum of 21 `Ready` registry
  ids; `run` matches every variant, and the rows paint the registry's `label`, `Group::label` and `shortcut`
  rather than strings of their own. `palette_rows` resolves them through `Registry::v0()` and filters on
  label, id and group name. `↑/↓` wrap, `Enter` runs the highlighted row, a row that cannot run (no selection,
  nothing staged, no folder to rescan) closes the palette and puts a sentence in the notice line rather than
  pretending. The keys are consumed at the top of `show`, before the bar lays out: a `TextEdit` takes Enter
  and the caret keys for itself, and the first ⌘K wiring failed exactly that way until the consumption moved.
- **The bar is glyphs with names.** App mark (the `sortlens-icon` SVG registered once per context — the same
  geometry as the window icon), then Settings, Help (`app.shortcuts`), Appearance, Activity, Export, Import,
  Filter and the right-panel toggle; the search field stays the one piece of chrome that is made for typing.
  Export is `add_enabled` on the selection and says "Select a photo to export" when it cannot run. Filter is a
  toggle whose badge is `UiState::active_filter_count`, painted onto the button's own rect by a new
  `paint_badge` the bell now shares, so a count never reflows the bar.
- **Appearance is one control for two settings.** Left click cycles the theme; right click lists all four
  presets and a Reduced-motion checkbox, so the two value chips that were here are gone rather than moved.
  Density and layout are two glyphs beside the sort in the canvas header — the same question ("what does the
  grid show") answered in one place.
- **The filter strip can be put away.** `filters_open` (default true) gates `filter_strip`, and a hidden strip
  leaves no gap. Hidden-on-empty-library is 3B.6's job and is not claimed here.
- **Two glyphs joined the set**, both project-owned stroke SVGs in the established conventions: `funnel`
  (Filter) and `export` (a tray with a rising arrow), 36 total. `Icon::ALL`, `CATALOG` and the folder test stay
  in step; `docs/ATTRIBUTION.md`'s `assets/icons/*.svg` row already covers them. `Ctrl+K` joined the keymap
  overlay (`crates/ui/src/keymap.rs`) so the shortcut is discoverable there as well as in the bar.
- **Tests that fail when the behaviour is absent (5 new, 687 workspace total).** `every_palette_command_is_a_ready_registry_command`
  walks the enum against `Registry::v0()` in both directions (unknown, planned and duplicated ids all fail)
  and pins the query rules; `the_palette_commands_run_or_explain_why_not` drives `run` for the refusal and
  success paths and checks the `Action` a command asks the host for; `command_k_opens_the_palette_and_enter_runs_the_highlighted_command`
  drives real `RawInput` events through `show` (⌘K, text, Enter, ⌘K, Esc) and asserts the state each step;
  `the_filter_chip_toggles_the_strip_and_wears_the_active_count` clicks the painted badge in isolation and
  then shows the whole window with the strip on and off; and `the_top_bar_and_palette_paint_only_glyphs_the_bundled_faces_have`
  audits every string the bar and the palette paint against the bundled faces — it passes for `⌘K`, the `·`
  separators and every registry label, and it caught the missing `icons::install` in its own first run as
  32 egui error placeholders. The chrome icon-count test now asserts 8 pictures in the top-bar band (was 3).
- **Visual verification: all 20 baselines regenerated with `--update`, then read, then re-run without it
  (8/8 tests matched).** Two are new: `palette-charcoal` and `palette-paper`, showing the field in command
  mode with the rows grouped by registry group and the declared shortcuts right-aligned. The library
  baselines were read in both presets to confirm the mark, the eight glyphs, the `⌘K` hint, the Filter badge
  and the density/layout pair in the canvas header. `apps/sortlens` was not run by hand; the screenshots come
  from the same `show` call.
- **Not verified by hand**: the palette on a narrow window (it is `max(field width, 430)` wide and hangs left
  of a 240-point field; below roughly 700 px it would reach the sidebar), and the app mark at 16 px on a
  non-Retina display, where the tile's internal prints are a pixel or two each.
- **Named, not hidden**: `docs/UX.md` §3 previously gave this bar a `view.appearance`, `view.filterBar`,
  `library.filter` and `app.libraryMenu` command; none of those ids exist in the registry, and the table now
  says which controls are shell state and marks the back/forward and library-menu rows as not shipped. The
  registry gained no new verbs this slice — every palette row already existed and was `Ready`.

## 2026-11-23 (thirty-first) — Phase 3 item 10 (3B.1): the palette is one mapping, and the chrome has pictures

- **`Tokens::palette()` is the mapping and `apply` is its only consumer.** `craft_ui::theme::WidgetPalette`
  (`noninteractive: panel`, `inactive: card`, `hovered: hover`, `active: active`, `open: card`) is now what
  `apply` installs through `WidgetPalette::apply_to`, so a suite widget reading `Visuals` and a Sortlens widget
  reading `Tokens` agree about a hover by construction instead of by two lists kept in step. Two documented
  exceptions survive: the pressed state wears the accent ring, and the borders stay `t.border` on every state.
  One visible consequence is *intended*: `weak_bg_fill` now equals the strong fill per state, so a resting
  button is a card rather than something between card and hover. `ThemeKind::egui_theme()` names which half of
  `Style` a preset paints under, instead of `apply` and its test spelling it twice.
- **The icon set exists as files, not as a font.** 34 project-owned 24-unit stroke SVGs under `assets/icons/`,
  embedded with `include_bytes!` and named by `Icon::ALL`. `crate::icons::install` puts `egui_extras`' SVG
  loader on the context once and registers every glyph's bytes up front, so the first frame a glyph appears on
  is the frame it is painted — not one frame later, which is what an unregistered `bytes://` URI looks like.
  `craft_ui::icons::SvgIconSet` prepares each drawing (white `currentColor`, 1.75 stroke) so the tint is the
  theme colour and not a second palette; the loader rasterises at exactly the slot size, which is what keeps a
  14-point glyph sharp at 2× rather than a blurred bitmap.
- **New workspace pin: `egui_extras`** (the same egui fork rev, `default-features = false`, `features = ["svg"]`
  — no HTTP, no file loader, no mime sniffing). `deny` passes its licence tree unchanged (`resvg`/`usvg` are
  MPL-2.0, already allowed for `option-ext`), and `attribution` gained its row plus a provenance row for `assets/icons/*.svg`.
- **Wired into all five surfaces, additively where the word carries a value.** Sidebar: every source row, the
  four utility rows, folder rows, keyword rows and collection rows now start with a glyph, in a fixed 16-point
  column so the text edges stay aligned; the `+` add-folder button became `folder-plus`. Filter strip: rating,
  flags, colour, kind, source, edited, missing and clear chips each carry their glyph, painted in the chip's own
  colour so an active chip does not have a light icon on a light pill. Bottom bar: every destination is a glyph
  beside its label. Right panel: the three tabs. Top bar: Settings became `sliders` (a gear at 13 px reads as a
  sun), the right-panel toggle became `panel` and now *selects* when the panel is open, the bell became a bell
  with an unread count painted into its own rect, and Add Folder is `folder-plus` plus its words. The value
  labels — theme, density, layout, motion — deliberately stayed words: they print a current value, and a glyph
  cannot say "Comfortable". Replacing those is 3B.2's Appearance menu, not this slice.
- **An icon-only button keeps its words.** `icons::button` labels the response through
  `a11y::label_widget` with the same string it uses as the tooltip: a screen reader must not be handed a file
  stem, and two different answers to "what is this button" is worse than one.
- **Tests that fail when the behaviour is absent** (6 new, 682 total): the enum ↔ catalog ↔ folder name check in
  both directions (so a glyph added to one list only is caught); a conventions test (`currentColor` present,
  `stroke-width="2"` present, 24-unit viewBox, no `rgb(...)`, so every glyph is tintable); a rasterisation test
  that asks the real loader — through the same `install` the shell calls — for all 34 glyphs at 14 and 32 px and
  requires both ink and empty pixels (verified by removing the `install` call from `show`, which failed it); an
  accessible-name test on an icon-only button through AccessKit; the palette test that compares what `apply`
  actually stored in `Visuals` against `Tokens::palette()` for all four presets; and a shell test that counts the
  textured rectangles painted inside the top bar, bottom bar and sidebar bands (3, 6 and 11 — exactly the
  designed counts, asserted with `assert_eq!` rather than a floor, and it failed when `install` was removed).
- **Visual verification: all 18 baselines regenerated with `--update`, then read, then re-run without it (7/7
  matched).** The top-right of the library baseline was magnified to check the new Settings glyph (the first
  gear drew a sun, the second a mush of three sliders — the shipped one is two rows with visible knobs), the
  sidebar and filter strip were magnified in Charcoal and Paper to confirm the glyphs read at 14 px and that the
  light presets get dark icons (`text_muted` tint, not white), and the detail baseline was read whole in Paper.
- **Not verified by hand**: `apps/sortlens` in a real window (no change in this commit's rendering path, but the
  glyphs are only seen through the harness), and how the sidebar's fixed 16-point icon column behaves at the
  narrow end of its 210–380 px resize range with a long folder name — `truncate` still measures from the reduced
  width, but only the picture would settle it.
- **Still open in 3B**: the `⌘K` palette and Appearance menu, the sidebar's utility strip, the bottom bar's
  annotation pill and peer views, the right panel's Suggestions tab and tool strip, the Welcome bullets and
  filter chevrons, and the three visual defects 3A recorded. Item 10 closed only what it named.

## 2026-11-23 (thirtieth) — Phase 3 item 9 (3A): the screenshot harness, its baselines, and what they found

- **`cargo xtask screenshots` exists and paints the real shell.** `crates/ui/tests/screenshots.rs` runs
  `sortlens_ui::show` inside an `eframe::App` — the same entry `apps/sortlens` uses — through
  `egui_kittest` at the suite's pinned egui revision, at a fixed 1440×900 with `pixels_per_point = 1.0`.
  Eight surfaces × the Charcoal and Paper presets = **18 committed PNGs** under
  `crates/ui/tests/snapshots/`: Empty Welcome, Library Justified, Library Square, Detail, Compare, Cull,
  Similar, Places and Insights (Detail and Compare share one test, as do the two grid layouts). The
  diffing is `egui_kittest`'s own — per-pixel tolerance in the new root `kittest.toml` (`threshold = 0.6`,
  `max_failed_pixels = 0`) — so a mismatch writes `<name>.new.png` and `<name>.diff.png` beside the
  baseline, reports the failing-pixel sweep, and fails. `cargo xtask screenshots --update` sets
  `UPDATE_SNAPSHOTS=1` and is a person's decision; the images are the review.
- **The harness is opt-in and the CI gate stays headless.** It lives behind `sortlens-ui`'s `screenshots`
  feature (`dep:egui_kittest`, `dep:eframe`), so `cargo xtask ci` neither compiles nor needs a GPU; the
  file is `#![cfg(feature = "screenshots")]`. `cargo xtask screenshots` runs it with `--test-threads=1`
  because each test creates a wgpu device, and six devices at once is a way to fail for a reason that has
  nothing to do with the UI. `egui_kittest` is declared once in `[workspace.dependencies]` like every other
  dependency and attributed in `docs/ATTRIBUTION.md`; `deny` (all-features) passes its licence tree
  (`kittest`, `dify`, `open`).
- **Every settled surface also passes `egui_kittest`'s accessibility check, and it found a real defect on
  the first run**: the canvas header's sort `ComboBox` had no accessible name — read aloud it said "Added",
  the current value, and nothing about what the control changes. Fixed with the crate's own
  `a11y::label_widget(ui, &combo.response, Role::ComboBox, "Sort order", false)`, so it is labelled the
  same way the other custom controls are. `cargo xtask a11y` could not see this one: the widget is egui's,
  not a hand-painted clickable.
- **The second finding was tofu**: the detail footer painted `← Previous` / `Next →` and `← → walk`, and
  none of the four bundled faces has U+2190/U+2192, so a zero-config build (the documented default, with
  no `craft-fonts` checkout) drew boxes. The buttons now read "Previous"/"Next" and the hint reads
  "left/right walk"; no behaviour changed. The overlay's keycap labels in `crates/ui/src/keymap.rs` and the
  registry's `shortcut` strings in `crates/engine/src/command.rs` still say `←`/`→` — they are cross-checked
  against each other and against the CLI goldens, so they are the next slice's work, not a half-fix here.
- **Three more visual defects are recorded, not hidden** (ROADMAP 3B.6 now names them): Places opens at
  zoom 5, so a library spread across two continents centres between them and paints no dots at all — the
  baseline zooms to 3 to show the dots, and choosing an opening zoom that fits is 3B.6's job; the cull
  view's keyword row clips under the bottom bar at 900 px; and the `1 active` + `Clear` pair still doubles
  up in the filter strip. A baseline that showed a blank map as "the Places view" would have been a
  placeholder presented as working, so the scene says what it changed and why.
- **The fixture library is real and now carries real metadata.** `sortlens-testkit` gained
  `Fixture::at(path)` (a caller-named, cleared folder — `new` keeps its process id for parallel tests) and
  `Pattern` (`Solid` or a deterministically phased `Gradient`) with `Fixture::jpeg_with_pattern`, which
  writes the JPEG's pixel dimensions into an XMP packet (where the suite's EXIF writer does not record
  them) and GPS into EXIF. Solid colours were not enough: the similarity pass reads structure, so twelve
  solid frames were one identical group — and the justified grid had uniform cells because a fixture with
  no dimension tags indexes as 0 × 0. The screenshot library is now twelve gradients with mixed aspect
  ratios, five EXIF locations, a duplicate pair (IMG_0009/IMG_0010, byte-identical), fixed ratings, two
  picks, a reject, two keywords and a collection — scanned once, then moved between scenes so every
  baseline paints the same ids. The folder is a *relative* path (`tests/.screenshots`, gitignored) because
  the Places status line prints the library's `tiles/` folder: an absolute temporary path with a process id
  in it made the baseline unreproducible, which is exactly what the second run caught.
- **Tests that fail when the behaviour is absent**: the seven screenshot tests (a changed token fails with
  a named diff — verified by temporarily recolouring Charcoal's accent, which failed 891 pixels, then
  reverted); `sortlens-testkit`'s gradient-stability test (same phase, same bytes; different phase,
  different file) and its dimension/GPS round-trip through `craft_metadata::extract`; and the existing
  headless `Context::run_ui` detail-view test was **extended** to audit every string the view paints against
  the bundled faces' glyph coverage, failing with the offending text — verified by reintroducing
  `← Previous`, which it named, then reverting. 676 workspace tests (+2 in testkit; the 7 screenshot tests
  run only with the feature). ci, snapshot, deny and `cargo xtask screenshots` green; a second
  screenshot run without `--update` matched all 18 baselines.
- **Visual verification**: the 18 baselines were opened and read, not just generated. They show the shell
  at 1440×900 in both presets, the two grid layouts with real aspect ratios, the detail and compare
  canvases, a settled cull session, two similar groups, the Places dots with no basemap, and the Insights
  figures. `apps/sortlens` was not run by hand; the screenshots come from the same `show` call, and the
  harness's only difference from the window is the 0 title-bar inset (documented in the test: a harness has
  no traffic lights to clear).

## 2026-11-23 (twenty-eighth) — Phase 2 item 3b: pack keywords matched by meaning (SL-F03, the CLIP side)

- **The gate first, as promised: the real pins were downloaded, digest-verified and run once this
  run.** All three `models-v1` files land under their digest names and pass `Model::verify_file`; the
  real detector loads (800×512, 100 queries) and runs; the real CLIP ViT-B/32 image + text pair loads
  and answers 512-d embeddings. The shipped tokenizer was validated against the real text model: the
  cosine ordering of five prompts came out exactly right (dog↔cat 0.946 > dog↔harbour 0.934 >
  dog↔financial report 0.893 > dog↔circuit diagram 0.850, and the two unrelated prompts are each
  other's furthest pair). A real cat photo (scikit-image's public-domain chelsea.png) through the real
  image embedder ranks “a photo of a cat” **first** (0.281) among the prompts, with unrelated
  photographic prompts at 0.19–0.24, and the real detector answers `cat · 1.00` on the same photo. The
  live check was a temporary example, deleted before commit; the tree's own tests run synthetic models
  only.
- **That live gate caught a real bug the square test models had hidden**: `Detector::input()` answered
  (height, width) while `Image::from_rgba` takes (width, height) — every synthetic detector is square,
  so the tests passed while the real 512×800 detector would have been silently transposed at every
  call. `input()` now answers (width, height) and the docstring says the order matches `from_rgba` on
  purpose.
- **`crates/ai/src/tokenize.rs` is the CLIP BPE tokenizer.** The vocabulary is the upstream
  `bpe_simple_vocab_16e6.txt.gz`, shipped byte-for-byte (`crates/ai/src/bpe_simple_vocab_16e6.txt.gz`,
  1,356,917 bytes, SHA-256 recorded in ATTRIBUTION.md) and decompressed here by `flate2` — desktop
  only, like the inference runtime, and still no network anywhere (`no_network` still passes, source
  scan included). The tokeniser reproduces the published table exactly: byte tokens in vocabulary
  order (the printable runs first, not byte order — the order bug the known-ids test caught), `</w>`
  attached to the last byte, 48,894 merges, BOS/EOS 49406/49407, zero-padded to the model's token
  count. The ids the tests pin (`a photo of a dog` → 49406·320·1125·539·320·1929·49407…) were derived
  from the published vocabulary, not from this implementation's output.
- **`crates/ai/src/text_cache.rs` caches each pack's keyword embeddings.** A pack file's SHA-256 and
  the text model's digest name the cache file
  (`<library>/models/text-cache/<digest>/<pack hash>.json`): a changed pack or a different installed
  embedder is a different file by construction, nothing is ever invalidated. The file is written
  atomically (temp + rename), read back only when its recorded model digest and pack hash match and
  every keyword's embedding is finite and the same length, and completed keyword-by-keyword — a pack
  that gained a keyword embeds only the new one. `load_only` serves a pack whose embeddings are fully
  cached without any model: the pack side is a derived index, not a second model.
- **The engine proposes pack keywords by meaning.** `Engine::embedder_session` (the text embedder +
  tokenizer) and `Engine::image_embedder_session` (the vision side) load once per session with the
  detector's rules; installing, removing or swapping the manifest resets all three session caches —
  including the detector, which never actually reset on install/remove before (the twenty-sixth
  entry's claim is now true). `pack_keyword_embeddings` fills the cache once per pass; per photo, the
  224 px preview is embedded once and every cached keyword is a cosine away. A keyword is proposed
  when its cosine clears `EMBED_MIN_COSINE` (0.25, set from the live numbers above: the correct match
  scored 0.281, unrelated photographic prompts 0.19–0.24), the cosine is the score, and the proposals
  go through the same `suggest::merge` as the matcher's hits — deduped against existing tags, capped
  at eight, ordered by confidence. The pack matcher's own rules still run beside it; a keyword both
  sources propose is one suggestion at the better score.
- **Tests that fail when the behaviour is absent** (20 new): the tokenizer's known ids, exact
  sequence shape for any token count, truncation that keeps EOS, case/whitespace insensitivity, byte
  fallback for untrained words, the published byte-encoder table at its edges; the text cache's
  build-and-write, serve-without-embedding, per-pack and per-model keying, completion, tamper refusal,
  refusal-propagation and per-model clearing; a lookup text embedder in `sortlens-testkit::onnx`
  (Gather + ReduceSum over real token ids) answered and pinned in `sortlens-ai`; and two engine
  tests — a synthetic pair installed under a swapped manifest, a pack whose matcher never fires, both
  fixture photos proposing “dog” (never “cat”) with the cosine as the score and the cache file on
  disk, the single-photo path answering from the same cache — and, after removing the text model, the
  fully cached pack still matching while a never-cached pack stays silent.
- ROADMAP's Phase 2 item 3 (both halves) ticked in the same commit; ATTRIBUTION.md attributes the
  vocabulary file and `flate2`; RELEASE.md's model bullet now says packs are matched by meaning;
  ARCHITECTURE.md §9 says where the cache lives and how the gate was chosen. UX.md needed no change —
  the panel's sources are unchanged.
- 665 tests (20 new); ci, snapshot, deny green. No UI change, so no `run_ui` or screenshot work
  applies; the screenshot harness does not exist yet (Phase 3 item 9).

## 2026-11-23 (twenty-seventh) — Phase 2 item 3a: the tract-onnx inference runtime and the detector path (SL-F03)

- **`crates/ai/src/infer.rs` is the runtime; it stays offline and desktop-only.** tract-onnx 0.23.10 —
  the exact build every pin was verified against — runs in `sortlens-ai` itself; the crate's `no_network` test scans every source file including this one and
  still passes, because there is nothing to find. Loading is a contract, not an assumption:
  `Session::load` refuses a file whose input/output shapes do not match what `docs/ATTRIBUTION.md`
  records (wrong rank, wrong batch, wrong channel count, a symbolic shape where the pinned models are
  static), so a wrong file fails with words instead of a strange photo.
- **The detector answers in the shapes the rest of the crate already understands.** `Detector::load`
  disambiguates the YOLOS outputs by shape (1×Q×4 is boxes, 1×Q×C is logits), builds the COCO-91 label
  map (`crates/ai/src/coco91.txt`, `include_str!`-ed and attributed in ATTRIBUTION.md), and `run`
  softmaxes each query, drops the unnamed slots (`N/A` classes and the EOS slot past the map), converts
  normalised cxcywh to corner boxes and hands everything to the same `detect::suppress` every other
  detector would face. `Image::from_rgba` stretches a decoded preview to the model's input size
  (nearest-neighbour, on purpose) and applies the ImageNet/CLIP mean/std normalisation. The CLIP pair
  loads and answers through the same contract-checked runtime: `ImageEmbedder` (1×3×edge×edge → 1×512,
  with a cosine that answers 0 rather than a NaN for a zero vector) and `TextEmbedder` (1×77 int64 →
  1×512) are tested and waiting for the pack-embedding slice.
- **Tests run on synthetic ONNX models, assembled from the same protobuf types the loader parses.**
  `sortlens-testkit::onnx` builds tiny valid models with the pinned models' exact I/O signatures whose
  outputs are constants the test supplies — no 166 MB pin in any test, no network, and the plumbing
  (loader contract, softmax, EOS drop, box conversion, NMS, cosine) is what is pinned. 10 new tests in
  `sortlens-ai`, 4 in `sortlens-testkit`.
- **The engine runs the detector over 384 px previews in the suggestion pass.**
  `Engine::begin_batch_suggest` loads the pinned detector once per session (`Engine::detector_session`,
  cached, reset by install/remove and by the test manifest swap), moves it to the existing suggestion
  worker, and decodes a `PREVIEW_EDGE` (384 px) preview per photo through `sortlens-thumbs`; what the
  model sees becomes `Candidate`s with `Source::Detector` and the box attached, merged with the EXIF
  hints and the packs by `suggest::merge`. A photo whose file is missing or undecodable simply has
  nothing to say — the pass continues. The single-photo panel (`Engine::suggestions`) and the
  selection panel (`suggestions_for_all`) answer from the same session cache. Without the model
  installed, the pass proposes exactly what it proposed before — the detector is an addition, not a
  dependency (a test pins that). Nothing is written by the pass; accepting a proposal remains the only
  path to a keyword.
- **Engine tests that fail when the behaviour is absent** (2 new, all offline): a synthetic detector
  installed in the store, the pass polled to its end, both fixture photos carrying "person" + "bicycle"
  with the box converted to corners and `Source::Detector`, the single-photo path answering from the
  same cache — and the no-model pass unchanged.
- **One standalone fix landed first** (2c0ce95): the reveal test asserted `a.jpg` in the command on
  every desktop, but Linux's `photo.reveal` deliberately opens the containing folder (`xdg-open` cannot
  select a file), which ubuntu-latest CI caught on the cross-platform pass of 769c461. Each platform
  now asserts its own documented contract.
- ROADMAP's Phase 2 item 3 was split into 3a (runtime + detector path, ticked today) and 3b (per-pack
  text embeddings cached at install, image embeddings, both feeding `suggest::merge`); RELEASE.md's
  model bullet now says the detector runs and what is still open; ARCHITECTURE.md §9 says where the
  inference lives; ATTRIBUTION.md says the label map ships now.
- 645 tests (16 new: 10 in `sortlens-ai`, 4 in `sortlens-testkit`, 2 in the engine); ci, snapshot, deny green. No UI change, so no
  `run_ui` or screenshot work applies; the screenshot harness does not exist yet (Phase 3 item 9).
- **Not verified against the real weights**: loading and running the pinned 13/89/64 MB files was done
  once when the pins were recorded, before this runtime existed; the suite covers the plumbing over
  synthetic models with identical I/O contracts, and the contract check refuses anything shaped
  differently. A live run against the downloaded pins is the next slice's first gate.

## 2026-11-21 — M0 complete (6a4983e)

23-crate workspace, shell, themes, settings, command registry, xtask CI, licences, CI on 3 OSes.

## 2026-11-21 — M1: index and browse

Shipped:
- `sortlens-scan`: bounded walker (skip lists, no symlinks, depth cap), media classification,
  content key from size + first/last 64 KiB. 5 tests.
- `sortlens-catalog`: `Photo`, `Filter`/`Sort`/`Page`, and a catalog of 19 tests: checksummed op
  log, atomic snapshots, replay with a dropped torn tail, undo, rescan that keeps ids and
  annotations, missing/restored files, stats, paging.
- `sortlens-engine`: catalog ownership, worker-thread scan with `poll()`, typed rate/flag/label/
  undo/remove_root, JSON `run()` for 25 commands. 9 tests.
- `sortlens-ui`: virtualised grid (visible rows only), selection, keyboard rating (0–5), flags
  (P/X/U), sidebar roots with counts and context menu, details/tags/activity, real status bar.
- `sortlens-cli`: add, rescan, roots, list, stats, rate, flag, undo, report. Verified end to end:
  index a folder, rate, flag, reopen in a second process, state intact.

Verified: `cargo xtask ci` green (fmt, clippy -D warnings, 42 tests, layer/lint/provenance rules);
CLI smoke test round-trips through disk.

Known gaps (deliberate, next milestones): no thumbnails yet (cells show kind + name), metadata
not read (width/camera/captured stay empty), filters in the bar are still disabled, no cull view.

## 2026-11-21 — M2: previews and metadata

Shipped:
- `sortlens-meta`: EXIF through `craft-metadata` into one optional-field struct (dimensions,
  capture time with UTC-offset handling, camera, lens, ISO, aperture, shutter, focal), plus
  civil-date conversion both ways. Read is bounded to 8 MiB per file. 6 tests.
- `sortlens-thumbs`: JPEG decode (8-bit, RGB/grey), box-filter downscale that never upscales,
  content-addressed disk cache with atomic writes and torn-file tolerance, and a two-worker
  `Service` with in-flight de-duplication and remembered failures. 7 tests, including real
  JPEG encode→decode round trips.
- `sortlens-ui`: texture cache (cover-fit, 900-texture cap), thumbnail requests only for visible
  rows, richer details panel (dimensions, camera, lens, exposure, capture date).
- scan/catalog: metadata travels with the file and is stored on the record; rescan refreshes it.
- CLI `list` shows dimensions.

Verified: `cargo xtask ci` green (55 tests); CLI round trip on a real 320×200 JPEG reads
`320x200` from EXIF; thumbnails decode from that file in tests.

Known gaps: RAW/HEIC/PNG/TIFF thumbnails, video posters, priority scheduling, activity stack,
non-JPEG previews fall back to the kind glyph.

## 2026-11-21 — M2: progress and cancellation

- scan: `ScanControl` with shared atomic counters and a cancel flag; `ScanReport.cancelled`
- engine: `scan_progress()`, `cancel_scan()`; a cancelled walk is never synced, because a partial
  walk would mark every unseen file as missing
- ui: live "Indexing · N files · M folders" in the bottom bar and sidebar, progress rows plus a
  Cancel button in Activity, tidy cancelled notice
- 57 tests, CI green

## 2026-11-21 — M2: PNG, TIFF, WebP, BMP, GIF thumbnails

- thumbs: `decode_image` dispatches through `craft-image-codecs` (PNG, TIFF, WebP, GIF, BMP, QOI,
  PPM and 8-bit JPEG), falls back to the JPEG path, and carries alpha through the resize
- 3 new tests: PNG round trip, TIFF round trip with downscale, alpha preserved
- 60 tests; `cargo xtask ci` green

Still missing: RAW embedded previews (`craft-raw`), video posters, HEIC.

## 2026-11-21 — M2: raw previews + shared media vocabulary

- core: `media` module owns the extension tables (raw/image/video/sidecar), `kind_from_extension`,
  `is_raw_path`; the scanner now delegates instead of keeping a second copy
- thumbs: `decode_raw_preview` pulls the largest embedded preview with `craft-raw` and decodes it
  through the ordinary pipeline; `decode_file` prefers it for raw extensions and falls back to the
  plain decoder for mislabelled files
- tests: raw-without-preview, mislabelled .cr2 that is really a JPEG, extension tables
- 65 tests; CI green

Not verified against a real camera raw: no sample file in the repo. The dispatch and fallback are
covered; the preview extraction itself is `craft-raw`'s code path.

## 2026-11-21 — M2: activity stack

- engine/job.rs: `Job` with kind, label, status (running/done/cancelled/failed), counters, error,
  start/finish times and a one-line detail; kept to the last 50
- engine: every scan creates a job, finishes as Done / Cancelled / Failed, and `jobs()` fills the
  running scan's counters from the scan control; `library.jobs` JSON command for scripts
- ui: Activity tab lists jobs newest first with status colour, elapsed time, detail, per-job error,
  and Cancel for the running one; library totals and warnings sit below
- tests: job lifecycle, missing folder fails the job, jobs JSON, plus unit tests for the row states
- 72 tests, CI green

## 2026-11-21 — M3: keywords

- catalog: `Photo.tags` (sorted, deduplicated), `Op::Tagged` replacing a photo's whole tag list so
  undo is a plain inverse, `Filter.tag`, `tag_vocabulary()` by count, `Stats.tagged`
- engine: `set_tag(ids, keyword, add)` normalises (trim, strip `#`, lowercase, ≤64 chars), skips
  no-op changes, returns how many photos changed; `photo.keyword` command is Ready;
  `tag_vocabulary()` for the sidebar
- ui: Keywords sidebar section with counts and click-to-filter, keyword chip in the filter strip,
  Tags tab with removable chips, an add field, and "used elsewhere" suggestions; `K` opens and
  focuses it
- cli: `tag`, `untag`, `keywords`, `list --tag`
- tests: tag sorting/dedup/undo, rescan keeps tags, filter and vocabulary agree, normalisation,
  command round trip, undo of a batch tag
- 77 tests, CI green; CLI smoke test across separate processes confirms the tags persist

XMP sidecar writing is still pending: keywords live in the op log for now, so they survive restarts
but are invisible to other apps.

## 2026-11-21 — M3: XMP sidecars

- `sortlens-sidecar`: `Sidecar { rating, flag, label, keywords }`, `path_for` (same stem, `.xmp`),
  `read`, `merge`, `write`. Merging edits the first `rdf:Description` in place: attributes are
  replaced element-value-wise, the `dc:subject` bag is swapped, everything else (crop settings,
  creator tool, unknown namespaces) is left byte-for-byte alone. Namespaces are declared on demand.
  Writes are temp-file + rename and skipped when nothing would change.
- engine: `sync_sidecars` runs after rate/flag/label/keyword changes, reports a `Sidecars` job with
  the number written and per-file failures, and never aborts the batch. `write_sidecars` toggles it.
- ui: Settings → Library checkbox, plus a notice; the toggle reaches the app as `Action::SetSidecars`.
- tests: fresh round trip, foreign file keeps `crs:Exposure2012`/`xmp:CreatorTool`, self-closing
  description gains children, clearing a label/flag removes it, element-form rating, entity
  escaping, atomic + idempotent write, missing sidecar, path mapping, engine writes a sidecar,
  switch-off, read-only folder reports a failure but keeps the rating
- 90 tests, CI green; CLI smoke test produced an `xmllint`-parseable sidecar

Not implemented yet: sidecars are not read back into the catalog at scan time, and failed writes
are retried on the next change rather than by a queue.

## 2026-11-21 — M3: sidecars are read at scan time

- core: `Review { rating, flag, label, tags }` is the shared review record, with `is_empty()`
- scan: each accepted file reads its sidecar (`sortlens_sidecar::read`) into `ScannedFile.review`;
  an unreadable sidecar is ignored rather than failing the scan, and an empty review is dropped
- catalog: `Seed.review` is applied by `Photo::adopt_review` when the photo is first indexed
- engine: passes the review through, so a folder annotated elsewhere arrives already reviewed
- tests: sidecar beside a photo becomes its review, photo without one stays blank, a new photo
  takes the sidecar's state, an existing photo keeps the catalog's state
- 94 tests, CI green; CLI smoke test imported a rating-4 / pick / keyword sidecar

Only *new* photos adopt sidecar state: a photo the catalog already knows keeps its review until
external-change detection lands, so a stale sidecar cannot quietly undo a cull.

## 2026-11-21 — M4: the cull state machine

- `sortlens-cull`: `Decision { Unjudged, Pick, Reject, Deferred }` and `Session` — queue, cursor,
  staged decisions, original flags. `stage` reports `Changed`/`Unchanged`/`Deferred`; a decision that
  matches the photo's original flag is not staged at all, and `Deferred` never touches a flag.
  `changed()` lists exactly what Apply would act on; `summary()` feeds the action bar.
- engine: `begin_cull`, `cull_decide`, `cull_stage`, `cull_back`, `cull_clear`, `cull_summary`, and
  the `cull.start`/`cull.decide`/`cull.back`/`cull.clear`/`cull.summary` commands (all Ready).
  `cull.apply` and `cull.undo` stay Planned("M4") because nothing moves yet.
- tests: the transition table as a partition, advance/back, re-applying the same value writes
  nothing, deferring preserves flags, clearing a flag is a change, rejects staged in queue order,
  completion, jump/next-unjudged, plus engine staging that leaves the catalog untouched
- 110 tests, CI green

Nothing is written until Apply, so a discarded session leaves the library exactly as it was.

## 2026-11-21 — M4: the cull view

- ui: `View::Cull` with a start screen (photo count from the current filters, "Start culling N
  photos"), a canvas that letterboxes the current photo at its own aspect, the decision badge,
  an "up next" strip of the following four photos (click to jump), a progress bar, live
  pick/reject/defer counts, and the P/X/U/Space/←/C keys
- `C` starts a session from the filters even with nothing selected; a session is a fixed queue, so
  changing filters afterwards cannot silently change what is being culled
- in a session P/X/U stage decisions instead of writing flags; outside one they keep their old
  meaning. Apply stays disabled and names its milestone: nothing moves yet
- `fit_rect` and its fallback are unit tested
- 112 tests, CI green

Not built: the summary sheet with an applied-batch receipt, gesture culling, zoom/pan and session
persistence. The cull view is real; Apply is still explicitly disabled.

## 2026-11-21 — M4: cull sessions survive a crash

- `sortlens-cull`: `Snapshot { version, queue, cursor, decisions, original }` with `snapshot()` and
  `restore()`; a future or empty snapshot is refused rather than half-read, a cursor past the end is
  clamped, and `retain()` drops photos that no longer exist while staying on the same photo
- engine: the session is written to `<library>/cull-session.json` after every change (temp file +
  rename), removed when the session ends, restored on `Engine::open` with photos the catalog no
  longer has pruned out. A write failure is a warning, never an error: culling continues
- ui: `C` opens a restored session instead of replacing it with a new one
- tests: snapshot round trip, version refusal, cursor clamping, retention, plus engine tests that
  reopen the library mid-session (cursor and reject survive), that ending a session deletes the
  file, and that a session whose photos are gone is discarded
- 118 tests, CI green

## 2026-11-21 — M4: staged deletes and Apply

- `sortlens-cull::trash`: `Mode { Trash, Move(folder), TagOnly }`, `Candidate`, `Batch`, `audit`
  (inside its root, still on disk, size and mtime unchanged, no symlink escape, destination
  writable and not inside a watched root), `apply` (re-checks each file immediately before touching
  it, per-item results), `undo` for moved files, and `apply_checked` which refuses the whole batch
  when the audit fails — partial application is not an outcome
- catalog: `Op::Group` applies in order and undoes in reverse, so an applied batch is one undo step
- engine: `cull_apply(mode)` audits, disposes, then commits flags and missing marks as one group,
  syncs sidecars for the touched photos, records an `Applied N photo(s)` job, clears the session,
  and restores the session untouched when the audit refuses. `cull.apply` is Ready with
  `{ mode: trash|tag|move, target }`
- ui: "Apply rejects…" opens a confirmation with pick/reject counts and a delete-mode choice
  (Trash or Tag only); Settings carries the same choice
- tests: trash audit refusals (missing, changed, outside root, destination inside a root), move
  preserves the relative path and undo restores it, tag-only moves nothing, undo of a trash batch
  says to restore it by hand, grouped undo in the catalog, engine apply for tag-only and move,
  and a changed file refusing the whole batch with the session kept
- 130 tests, CI green (`trash` is a desktop-only dependency)

Not wired: undoing a batch restores catalog state only — moved files are put back by
`sortlens_cull::trash::undo`, which the UI does not call yet, and trashed files stay in the system
Trash. `Mode::Trash` has no automated test because it would write to the real Trash.

## 2026-11-21 — M4: the rejects bin

- `sortlens-cull::trash`: `DeletedBatch { id, applied_at, mode, items, previous_flags, failed }`
  with serde and a `summary()`; `Applied`/`Location` are serialisable so a batch can be replayed
- engine: applied batches are appended to `<library>/deleted.json` (atomic, reloaded on open, ids
  monotonic), `deleted_batches()` lists them, `restore_batch(id)` moves files back through
  `trash::undo`, clears the missing marks and restores the flags in one `Op::Group`, then drops the
  batch. A file that cannot come back refuses the restore instead of leaving the catalog lying.
  Commands `library.deleted` and `library.restoreBatch`
- ui: "Recently Deleted" sidebar row with a count, and a dialog listing batches newest first with
  summary, age, mode and a Restore button; it names the Trash limitation in place
- tests: batch JSON round trip, apply → bin → reopen → restore restoring file, missing state and
  flag, empty-bin error for an unknown id
- 132 tests, CI green

Not built: emptying the bin (permanent delete from Sortlens) and restoring trashed files, which
stays a Finder/Explorer action.

## 2026-11-21 — M5: library health

- catalog: `Op::Relocate` (new path, clears missing; undo restores the old path *and* the missing
  state as a small ordered group), `missing_photos()`, and `remove_missing()` which purges as one
  group so a single undo brings the whole purge back
- engine: `check_health()` stats every file and commits one group of missing flags (report + badges
  agree, one undo step); `relocate_missing()` rescans each root and matches missing photos by content
  key under the same root; `remove_missing()`; commands `library.health`, `library.relocate`,
  `library.removeMissing`
- ui: "Library health" sidebar row opening a dialog with the last result and the three actions, each
  labelled with what it does
- cli: `check` and `relocate`
- tests: relocate clearing missing + undo restoring both path and state, purge as one undoable group,
  engine health check detecting a moved file, relocate finding it by key, and remove-missing undo
- 136 tests, CI green; CLI smoke test moved a file, saw `1 of 2 missing`, relocated it and
  read back the new relative path

## 2026-11-21 — M5: collections

- catalog: `Collection { id, name, photos, created }` with a set of members; ops `AddCollection`,
  `RemoveCollection`, `RenameCollection` and `Collect` (inverse swaps added/removed, so undo is
  exact); collections and the id counter live in the snapshot with `serde(default)` so older
  snapshots still open; `Filter.collection` resolved by the catalog, so membership composes with
  every other chip
- engine: `collections`, `create_collection`, `rename_collection`, `delete_collection`,
  `add_to_collection`, `remove_from_collection`; commands `collection.new`/`addTo`/`removeFrom`/
  `rename`/`delete` are Ready
- ui: Collections sidebar section with counts and click-to-filter, right-click rename/delete,
  "+ New collection", a rename dialog, a filter chip, and a per-photo "Add to Collection" context
  menu in the grid
- tests: create/fill/filter/rename/delete with undo, membership surviving a snapshot and restart,
  a collection needing a name, and the whole flow through the engine commands
- 141 tests, CI green

Not built: collection favourites, colours, per-collection search/sort, and reordering.

## 2026-11-21 — M5: subfolder drill-down

- catalog: `folders(root, prefix)` lists the immediate subfolders of a scope with recursive photo
  counts, skipping missing photos
- engine: `subfolders(root, prefix)` and the `library.subfolders` command
- ui: the Folders section shows the chosen root's subfolders with counts, clicking one descends a
  level and re-selecting it goes back up; a breadcrumb row (`Root › day1 › sub`) sits above the
  filter strip with clickable segments and "Up one level"; choosing another root clears the scope
- cli: `folders [--root R] [--in PREFIX]`
- tests: one-level listing with recursive counts and folder filtering in the catalog, the engine
  command, and `parent_prefix`
- 144 tests, CI green; CLI smoke test listed `day1/` (2), `day2/` (1) and `day1/sub/` (1)

## 2026-11-21 — M3: sort keys

- catalog: `Sort::ALL`, `id()`, `label()` and `parse()` (unknown ids fall back to added order)
- ui: the header's static "Added order" label became a combo box listing added order, name, newest,
  oldest, largest and highest rated; the grid and the cull queue both use the chosen order, so a
  session starts in the order the user was looking at
- app: the order is saved to `library.sort` and restored on launch (`Action::SetSort`)
- tests: ids round trip and are unique, unknown ids fall back, settings default is "added"
- 146 tests, CI green

Per-view orders (a separate order for collections or folders) are not built: one order applies to
the whole library view.

## 2026-11-21 — M3: kind and edited filters

- sidecar: `has_edits(text)` recognises develop settings by their namespaces (`crs:`, `xmpMM:History`,
  `xmp:History`, `darktable:`, `photoshop:History`), so a sidecar with only stars or keywords is not
  an edit, and `read_text` gives callers the whole document
- scan: reads the sidecar once per file and fills both the review and a new `edited` flag
- catalog: `Photo.edited`, `Seed.edited`, `Filter.kind` and `Filter.edited`
- ui: the filter strip cycles "All kinds → Photos → RAW → Videos" and "Edited → Edited only →
  Unedited", and both clear with the rest
- tests: edit markers recognised and our own sidecar not, scan marking an edited photo from a `crs:`
  sidecar and not from a rating, catalog kind/edited counts
- 149 tests, CI green; CLI smoke test indexed a photo with a `crs:` sidecar

"Edited" means the sidecar carries someone's develop settings — detection happens at index time, so
a photo edited after it was scanned shows as edited only after a rescan.

## 2026-11-21 — M4: the detail view

- engine: `neighbour(filter, sort, id, delta)` walks the same order the grid shows and stops at the
  ends instead of wrapping; `position` gives the one-based counter
- ui: `state.detail_open` swaps the grid for a single-photo canvas — name, counter, Fit/1:1 toggle,
  a 2000px preview with the area as a clip rect for 1:1, a facts line (camera, exposure, size), the
  keyword chips, and Previous/Next buttons. Double-click or `D` opens it; `Esc`/`D` closes it;
  `←`/`→`/Space walk the view, `Z` toggles zoom, and P/X/U plus 1–5 keep working inside it
- tests: navigation walking the view order and clamping at both ends, `position` counting
- 150 tests, CI green
- `view.detail` is now a Ready command

Not built: the lookahead strip and the histogram in the detail view.

## 2026-11-21 — M4: histogram and lookahead

- thumbs: `Histogram` (64 luminance bins, Rec. 601 weights) with `from_thumbnail`, `total`, `peak`,
  and `histogram_file` which decodes a small copy and measures it
- ui: `Thumbnails::histogram(photo)` caches one histogram per content key (decoded at 256 px on
  demand, never by the grid); the detail view draws it as 64 bars scaled to the tallest bin and adds
  a lookahead strip of ±1/±2 neighbours that can be clicked
- tests: every pixel counted with black and white in the end bins, a mid grey landing in the middle
- 152 tests, CI green

Housekeeping: `[build] incremental = false` in `.cargo/config.toml` after the disk filled up —
incremental artefacts were 3.5 GB per build on a machine with almost no free space.

## 2026-11-21 — M3: multi-keyword filters and an active badge

- catalog: `Filter.tags: Vec<String>` with `TagMode { Any, All }` (default Any) replaces the single
  `tag`; `TagMode::flipped()` and `label()` for the toggle
- ui: sidebar keywords toggle membership instead of replacing the filter, the strip shows one chip
  per active keyword plus a "match any/all" toggle when more than one is active, and the header shows
  an "N active" badge that clears every filter when clicked (`active_filter_count` counts dimensions,
  not keywords)
- cli: `list --tag` maps onto the new vector
- tests: ANY versus ALL matching with a photo carrying one and both keywords, unknown keyword
  matching nothing, `flipped`, and the badge counting two keywords as one dimension while clearing
  resets it
- 154 tests, CI green; CLI smoke test listed both photos for `beach` and one for `sunset`

## 2026-11-21 — M3: keyword picker and ordering

- catalog: `TagOrder { Count, Name }` with `flipped()`/`label()`; `tag_vocabulary_in(order)` and the
  existing `tag_vocabulary()` (count order) delegating to it
- engine: `tag_vocabulary_in(order)`
- ui: a "+ keyword" menu in the filter strip with a search box and a ✓ on active keywords, and a
  small toggle in the Keywords header switching the list between most used and A–Z
- tests: ordering by use and by name agreeing on counts, `flipped`
- 155 tests, CI green

## 2026-11-21 — M6: insights view

- insights: `Insights::compute` over the catalog's photos in one pass — totals (`Stats`), 6 rating
  buckets, 5 label buckets, 3 kind buckets, cameras/lenses/keywords folded to the top 8 and capture
  years with a real leap-year calendar walk (epoch zero counts as undated, not 1970); `rated_share`,
  `tagged_share`, `peak_year`, `peak(rows)`
- engine: `insights()` and `view.insights` (Ready, key `I`) returning the whole view as JSON
- ui: `View::Insights` with headline figures, proportional bars for ratings/labels/kinds, breakdown
  lists for cameras/lenses/keywords and a timeline column; the nav tab is no longer "planned"
- cli: `insights` prints the same numbers
- tests: totals reconcile, buckets sum to the library size, breakdown ordering and the top-8 fold,
  leap-day bucketing, an empty library having no shares, and the JSON face matching the typed view
- 161 tests, CI green; CLI smoke test over a two-photo library

## 2026-11-21 — M6: perceptual hashing and duplicate groups

- hash: `Hash { difference, average }` from a 9×8 box-filtered luma grid (every source pixel counted
  once, so the result does not depend on sample placement); `distance` takes the smaller of the two
  distances and ignores a zero average hash, which is the no-structure signal that would otherwise
  make every flat image match every other; `hash_file` decodes a small copy, using RAW embedded
  previews like the grid
- similar: `group` chains near-duplicates with union-find (largest group first, best member first
  inside), `around` for find-similar-to, best pick by pixels then stars then bytes then id
- engine: `begin_similar`/`poll_similar`/`hash_progress`/`cancel_similar` on a worker thread that
  reuses hashes cached by content key, one `Hash` job in the activity stack, `similar_to`; commands
  `similar.start` and `similar.groups`, `view.similar` now Ready
- ui: `View::Similar` with a start button, progress bar and cancel, a strictness slider, and one card
  per group showing thumbnails with the best member marked
- tests: gradient sets all 64 difference bits, a brightened copy stays within 8 bits, stripes one way
  and the other are far apart, identical files hash together, degenerate averages do not match,
  union-find chaining, stable grouping across input order, and an engine pass that groups two
  identical JPEGs while a third stays alone
- 177 tests, CI green

## 2026-11-21 — M6: metric index and colour fingerprint

Planned as "LSH banding"; the plan changed while building it. Banded LSH cannot stay exact at a
radius of ten bits without so many bands that every bucket holds a large fraction of the library, so
the candidate search is a BK-tree instead: exact under Hamming distance, visiting only nodes whose
edge range can still reach the radius. A brute-force cross-check test pins the index to the naive
answer.

- hash: `Colour` (3×3 mean RGB over the same box filter as the hashes) with a mean-absolute
  `distance` 0–255, and `analyse_file` so structure and colour come from one decode
- similar: `group_with(members, threshold, colour)` shares a BK-tree candidate search and vetoes a
  pair when both have fingerprints further apart than the tolerance; `matches` is the single rule
  `group`, `group_with` and `around` all use; `Member.colour`
- engine: the hash cache stores `(Hash, Colour)`, the pass uses `analyse_file`, `poll_similar_with`
  takes a colour tolerance, UI polls it every frame so a finished pass reports its group count
- ui: separate structure and colour sliders in the Similar view
- tests: the index matching brute force on 64 hashes, a colour mismatch refusing a structural match,
  the no-fingerprint case still grouping, 3×3 colour grid arithmetic
- 181 tests, CI green

## 2026-11-21 — M4: cull summary and apply receipt

- cull: `Summary::headline()` ("3 of 12 judged" / "Judged all 12 photos"); `trash::Receipt` built
  from a finished `DeletedBatch` with `headline`, `detail` ("Moved to …", tag-only spelling), `kept`
  and `is_restorable`
- engine: `receipt()`/`clear_receipt()` set by every Apply (mode, rejects disposed of, picks
  written, files kept, failures, batch id), `cull_complete()`, `cull_next_unjudged()`
- ui: a summary sheet that opens itself the moment every photo is judged and can be reopened from
  the cull header — counts, "N photo(s) will change on Apply", Apply…/Keep culling/Jump to next/
  Discard; and a receipt window that opens after Apply and offers "Put them back" (restore the batch)
  or "Recently Deleted", with dismissal clearing the receipt
- tests: a session reporting incomplete until the last decision, then complete; a tag-only batch
  whose receipt counts one disposed file, one pick, one kept file, is restorable, names its batch and
  is forgotten on dismissal
- 182 tests, CI green

## 2026-11-21 — M4: gesture culling

- cull: `gesture::classify(delta)` — a drag counts only past 84 points and only when the vertical
  travel dominates by 1.4×, so a sideways scrub never flags a photo; up is keep, down is reject.
  `progress` clamps toward the threshold for the overlay, `Gesture::decision`/`label` for staging
- ui: the cull canvas senses drags, tints and labels the photo while the finger is down (with a hint
  when the drag is too short), stages the decision on release and clears the drag state
- tests: short drags doing nothing, direction choosing the decision, sideways and diagonal drags
  ignored, progress clamped to 1
- 186 tests, CI green

## 2026-11-21 — M4: zoom and pan on the cull canvas

- cull: `viewport::View` (zoom 1–8, offset in points) with `zoom_by`, `wheel` (1.12× per notch),
  `pan`, `size`, `reset`; every change re-clamps the offset to `(zoomed − canvas)/2` per axis so the
  picture can never be dragged off the canvas, and zooming back to fit recentres
- ui: the canvas senses the wheel while hovered, ⌥/⌘-drag pans (a plain drag still decides), `Z`
  toggles fit/2×, the drawn rect is offset and clipped to the canvas at 1:1 when zoomed, and a
  "2.0× · Z to fit" badge appears; the interaction was extracted into `canvas_interaction` and
  `fitted_size` to keep the view under the complexity budget
- tests: clamps at both ends, recentring on zoom-out, pan stopping at the edge, no pan while fitted,
  offsets shrinking with the slack, wheel steps and a zero wheel being ignored
- 192 tests, CI green

## 2026-11-21 — M4: screen-reader announcements

egui exposes an AccessKit tree but has no "say this now" call, so the announcement is a visible
status line marked as an AccessKit polite live region: assistive technology reads it when the text
changes and never interrupts. The line is on screen too, because a status only a screen reader can
check is a second invisible UI.

- ui/a11y: `live_line` (label + `Live::Polite` + label text on the node), `cull_status`
  ("Photo 3 of 12, picked", "No photos to cull"), `library_status` ("240 photos, 3 selected"),
  `decision_sentence` ("Rejected, 4 photos left") and `decision_word`
- ui/shell: the cull view always shows the queue position and current decision as a live line, the
  library header announces how many photos are in view and selected, and every decision path —
  keyboard, buttons — sets `state.announce` so the result is read once
- tests: status sentences for each decision and an empty queue, pluralisation of "photo/photos left",
  zero/one/many counts, and every decision having a word
- 196 tests, CI green

## 2026-11-21 — M4: quit guard

- engine: `unsaved()` returns the specific reasons work is in flight — a running scan (files found so
  far unsaved), staged cull decisions (with the count, and the note that the session is kept), a
  running similarity pass — and `has_unsaved()` for the guard
- ui: a "Quit Sortlens?" dialog listing those reasons with "Keep working" and "Quit anyway"; a plain
  Action::Quit from the menu still closes immediately
- app: a close request with work in flight sends `ViewportCommand::CancelClose` and opens the dialog;
  confirming sets `confirmed_close` so the guard cannot loop on the way out, and a clean quit still
  closes the engine and flushes settings
- tests: a quiet library having nothing to lose, a running scan counting, an empty session counting
  for nothing, one staged decision producing exactly one reason, and discarding clearing it
- 197 tests, CI green

## 2026-11-21 — M5: collection colours and favourites

- catalog: `Collection.colour: Option<ColorLabel>` and `favourite: bool` with `serde(default)` so
  records written before this load unchanged; ops `SetCollectionColour` and `SetCollectionFavourite`
  whose inverses restore the previous value; `collections()` now sorts favourites first, then by
  name, so pinning one does not disturb the rest
- engine: `set_collection_colour`/`set_collection_favourite` and the `collection.colour` (name or
  `none`, unknown names rejected) and `collection.favourite` commands
- ui: the collections section moved into `collections_section` (it was pushing the sidebar past the
  complexity budget); favourites show a star, colours a dot in the theme's label palette
  (`Tokens::label`), and the context menu gains favourite/unfavourite and a Colour submenu
- tests: a new collection having neither, an old JSON record loading with defaults, colour and
  favourite undoing exactly, favourites sorting first, and the commands round-tripping with an
  unknown colour rejected and the old colour left in place
- 201 tests, CI green

## 2026-11-21 — M5: per-collection sort memory

- catalog: `Collection.sort: Option<String>` (serde default), op `SetCollectionSort` with an exact
  inverse; `set_collection_sort` skips a no-op and stores the id so a later `Sort::parse` reads it
  back, with unknown ids falling back to the natural order
- engine: `set_collection_sort` and the `collection.sort` command
- ui: opening a collection adopts its remembered sort, changing the sort while a collection is open
  writes it back, and `library_sort` keeps the library's own order so leaving the collection
  (`clear_filters`) restores it; the app seeds `library_sort` from saved settings
- tests: a collection starting with no sort, the sort round-tripping through the op log and undoing,
  an unknown sort parsing to natural, the command's JSON face, and the UI restoring the library sort
  when a collection closes
- 205 tests, CI green

## 2026-11-21 — M5: collection-scoped search

- catalog: `Collection.search: Option<String>` (serde default), op `SetCollectionSearch` with an
  exact inverse; `set_collection_search` trims, treats whitespace as "forget it" and skips a no-op
- engine: `set_collection_search` and the `collection.search` command, which returns the stored value
  (null when cleared)
- ui: the search box hint reads "Search in <collection>" while one is open; selecting a collection
  restores its stored search and sort, `remember_search` flushes the box when it loses focus or Enter
  is pressed (one op per edit session rather than per keystroke), and `library_search` keeps the
  library's own query so leaving the collection restores it — `clear_filters` does the same
- tests: trimming, a whitespace search clearing the record, undo restoring it, the command's JSON
  face, and the UI restoring the library search when a collection closes
- 208 tests, CI green

## 2026-11-21 — M5: first-run wizard

- ui: `Welcome { Add, Indexing, Ready }` decided by `welcome_stage(roots, scanning, total, dismissed)`
  — derived every frame so it cannot get stuck, and a dismissed wizard stays dismissed. The Add step
  explains that files stay put, changes are undoable and rejects wait for Apply, then offers
  "Add Folder…" or "Not now"; the Indexing step shows live file/folder counts, a progress bar and the
  photos found so far
- ui: the first scan that finds anything raises a one-time card — "Indexed N photos in <folder>" —
  with "Start culling" (switches to the cull view) and "Dismiss"
- tests: the stage for no roots, an empty root, a scan in progress (which wins over an existing
  library), a library with photos (no wizard), and dismissal overriding everything
- 209 tests, CI green

## 2026-11-21 — M5: keyword rename and merge

- catalog: `rename_tag(from, to)` rewrites every photo carrying the keyword in a single `Tagged` op,
  so a rename of two thousand photos undoes in one step; renaming onto a name already in use merges
  (the target is not duplicated), empty names and no-op renames are refused, and no photos means no op
- engine: `rename_tag` normalises both names first (so `BEACH` and `#beach` match), syncs sidecars
  for the affected photos, records a Sidecar job, and the `tag.rename` command reports the count
- ui: the Keywords sidebar row has "Rename or merge…", opening a dialog that warns when the target
  exists ("the two will be merged"), renames on Enter, and drops the renamed keyword from the filter
- cli: `rename <old> <new>`
- tests: rewrite across photos, merge without duplication, two undos restoring in reverse, refusals,
  an unknown keyword changing nothing, and the engine's normalisation plus merge
- 211 tests, CI green; CLI smoke test renamed two photos

Found while smoke-testing: **one-level undo does not survive a restart** — the inverse lives in
memory, so a fresh process reports "nothing to undo". That is a real gap, now listed as the next task
rather than glossed over.

## 2026-11-21 — M7: undo survives a restart

- catalog: log records gain an optional `inverse` field (serde default, omitted when absent), so a
  replayed record restores one undo level; `undo()` logs its own inverse too, which makes an undo
  itself reversible from a fresh process; the snapshot carries the last inverse because closing
  snapshots and empties the log — that was the actual hole the first attempt hit
- engine: nothing new; `edit.undo` was already wired
- cleaned up: the stale duplicate `cull.undo` command (Planned("M4"), same Ctrl+Z as `edit.undo`) and
  its planned-message entry are gone, so one command owns undo
- tests: rating undone after a reopen with records replayed, a snapshot rollover (empty log) keeping
  one level, a record written without an inverse still replaying, and the CLI undoing a tag in a
  process that never saw the change
- 214 tests, CI green

## 2026-11-21 — M7: empty the rejects bin

Semantics first, because "empty the bin" is ambiguous: Sortlens only ever forgets the *restore
record*. Files already moved sit in the system Trash, files that were only flagged stay where they
are, and nothing on disk is deleted from here. That is what the confirmation says.

- engine: `forget_batch(id)` (errors on an unknown id) and `forget_all_batches()` returning batch and
  file counts; both persist, neither touches the catalog or the files
- engine: `library.forgetBatch` and `library.clearDeleted` commands
- ui: each bin row gains "Forget" (with a hover that says the files stay put) and the dialog gains
  "Empty bin…", which opens a confirmation naming the batches and files, spelling out that nothing on
  disk is deleted and the step cannot be undone
- tests: two tag-only batches, one forgotten leaving the other, the rest cleared with counts, both
  files still on disk afterwards, the photos still in the library, an unknown id refused, and
  emptying an empty bin being harmless
- 215 tests, CI green

## 2026-11-21 — M7: backup and restore

- catalog: new `backup` module. A backup is a JSON document carrying the catalog's own text files
  (`snapshot.json`, `ops.log`) with a CRC32 each, so it reads in `less`, needs no compression
  dependency and refuses a truncated or hand-edited file. `create`/`encode`/`read`/`restore`; `read`
  verifies every checksum *and* the schema version before anything is written, and `restore` writes
  through a `.restore` temporary then renames
- catalog: `Catalog::flush()` so a backup can be taken from the live state without snapshotting
- engine: `export_backup(path)` (flushes first, atomic replace, records an Export job) and
  `import_backup(path, dest)`; commands `library.backup` / `library.restoreBackup`
- Deliberate limit, stated in the UI and the module docs: photos are never copied. A backup holds the
  index and the review state, not pixels — and a restore always targets a *separate* folder, because
  writing an older archive over the open library would silently undo work
- ui: Settings gains a Backup section ("Back up library…" and "Restore into a folder…"), app wires
  both to `rfd` dialogs and names the folder the restored library went to
- cli: `backup <file>` and `restore <file> <folder>`
- tests: catalog round trip through encode/read/restore with the rating intact, a tampered file
  refused by checksum, a newer schema refused by name, unexpected files and empty archives refused,
  an empty library refused, a snapshot-only library restored and reopened; engine round trip into a
  separate library with rating and keyword intact and the open library untouched; a damaged archive
  refused with nothing written
- 223 tests, CI green

## 2026-11-21 — M7: volume remap

The case this exists for: a shoot arrives on a card, is indexed, then the card is renamed or the
folder moves. Every relative path still holds, so the root can be repointed instead of relocating
photos one at a time.

- catalog: `Op::RemapRoot { from, to }` — rewrites the root list and every member's root in one op,
  with `RemapRoot { from: to, to: from }` as the exact inverse, so a mistake costs one undo. It
  refuses an unindexed `from` and a `to` that is already indexed
- engine: `remap_root(from, to)` refuses a path that is not a folder, refuses a root with no photos,
  then checks a **spread** sample of up to `REMAP_SAMPLE = 12` photos (`sample_indices` takes every
  n-th, not the first twelve, so a sample covers the whole root) for presence *and* size. A wrong
  folder fails with "<n> of <m> sampled photo(s) were not found" instead of quietly marking the
  library missing on the next scan
- engine: `library.remapRoot` command
- ui: the Library health dialog lists each root with its photo count and a "Point at…" button, since
  a moved volume is exactly what that dialog is opened for
- cli: `remap <old> <new>`
- tests: a renamed folder remapped, roots repointed, health reporting zero missing, and one undo
  returning to the old root; a wrong folder and a right-name-wrong-size file both refused with the
  root unchanged; an unindexed root refused; the sample covering 12 of 30 and everything of 3
- 225 tests, CI green. Verified by hand over the CLI, including the refusal path.

## 2026-11-21 — M7: integrity check and compaction

- catalog: new `integrity` module. `audit(dir)` answers "can this library be read back exactly?" by
  copying the two catalog files to a scratch directory and opening them there — the same code a
  normal open runs, with no side effects on the library. It reports the snapshot's seq, records
  replayed, records already covered, log bytes, photos, collections, whether the log tail was torn,
  and any warnings from the open
- catalog: `open()` now reports `torn_tail` and `skipped` in `OpenReport` instead of leaving them in
  prose warnings; `log_bytes()` and `compact()` (snapshot then truncate, returning bytes reclaimed)
- engine: `audit()`, `compact()`, commands `library.audit` / `library.compact`. Compaction **refuses**
  when the audit found anything worse than a torn tail, because rewriting the log from a damaged
  snapshot would cement the damage — found by hand: compacting a half-written snapshot produced a
  clean-looking, empty library
- ui: Library health gains a Catalog files section ("Check integrity", "Compact") that reports the
  headline, the detail and the bytes reclaimed
- cli: `audit` and `compact`
- tests: fresh library clean, records replayed counted, a truncated log tail reported as recoverable
  and then repaired by compaction (clean again, the readable prefix kept), an empty folder refused,
  compaction reporting reclaimed bytes, the engine round trip, and compaction refusing a library whose
  snapshot could not be read
- 231 tests, CI green. Verified by hand: audit on a clean library, a damaged snapshot
  reported as "Needs attention" with the reason, and compaction refusing it.

## 2026-11-21 — M7: change viewer and support report

- catalog: log records now carry `at` (unix seconds, omitted when unknown), so a change list can say
  when. `Op::summary()` gives every op a sentence and `Op::photo_count()` a count — a new op cannot
  arrive mute, which a test enforces
- catalog: new `history` module: `recent(dir, limit)` reads the log back newest-first, and
  `export(dir, limit, version)` writes a plain-text report (a hand-written civil-date conversion, so
  no date dependency for one line of output)
- **Behaviour change:** `Catalog::close()` now flushes instead of snapshotting. Snapshotting on every
  quit emptied the log on every quit, which would have made "recent changes" permanently empty — found
  by trying the CLI. Snapshots stay every 256 ops and on explicit compaction; `docs/BEHAVIOUR.md`
  records the trade
- engine: `history(limit)`, `export_history(path)`, commands `library.history` / `library.exportHistory`
- ui: "Recent changes" dialog (seq, sentence, age, op kind) reachable from the sidebar and a "Export
  report…" button; the report states that folder and file names appear and photo contents never do
- cli: `history [--limit N]`
- tests: newest-first ordering with summaries and dates, an empty log reading as no changes, the
  report naming every change with a UTC time and saying what it holds, dates rendering without a date
  library (including 1969 and a leap-year boundary), every op having a capitalised sentence, the
  engine round trip, plus the backup/integrity tests updated for the new close semantics
- 237 tests, CI green. Verified by hand: history across three CLI processes and an undo
  still working afterwards.

## 2026-11-21 — M7: one writer per library

- catalog: opening takes an OS file lock on `<library>/library.lock` (`File::try_lock`, stable in
  1.89, no dependency and no `unsafe`). A second process opens **read-only**: it replays the log and
  reads everything, but `commit()` refuses with "this library is open read-only because another
  window is using it". The log is opened for reading in that mode, so an append cannot slip through
- catalog: `Catalog::read_only()`, `OpenReport.read_only`, `LOCK_NAME`. The lock is an OS lock, so a
  crashed window releases it on exit — there is no stale-lock timeout to get wrong
- engine: `read_only()`, `library.readOnly` command, `begin_scan` refuses, and the cull-session and
  rejects-bin side files are not written in read-only mode (writing them would lose them silently)
- ui: the title bar shows "Read-only · another window has this library" with a hover explaining what
  to do, so the state is visible rather than discovered by a failed keystroke
- Fixed a real bug found by the first test run: field initialisation order meant `lock.take()` ran
  before the log was opened, so the writer opened its own log read-only and every append failed with
  EBADF. The flag is now read before the struct is built
- tests: catalog-level — second open is read-only, reads fine, refuses a commit, and the lock is free
  after the first closes; engine-level — a second window refuses rating and rescanning while the
  writer is unaffected, and a third window can write once the writer is gone; the existing undo-reopen
  test now drops its first catalog, which is the new rule
- 239 tests, CI green. Verified by hand with an OS lock held by another process: writes
  refused with the read-only message, reads fine, writes working again after the holder exits.

## 2026-11-21 — M7: retry unreadable metadata

The gap: `sortlens_meta::read` returned defaults on *any* failure, so "this file could not be read"
was indistinguishable from "this photo has no EXIF", and nothing ever tried again.

- meta: `ReadOutcome { Parsed, Unreadable }` and `read_outcome()`; `read()` stays as the forgiving
  wrapper. A missing file is retryable, an empty result is final
- scan: `ScannedFile.meta_unreadable`, so the flag survives the walk
- catalog: `Photo.retry` (serde default), `Seed.meta_unreadable`, `Op::Retry { changes }` with an exact
  inverse, `retrying_photos()`, `exifless_photos()`, `mark_retry()`. A rescan clears the flag when the
  file reads again
- engine: `retry_metadata(all)` returns `(updated, remaining)`, `retry_count()`, `exifless_count()`,
  command `library.retryMetadata` (takes `{"all": true}`)
- The `all` scope is the honest second half: after the app learns to read a format it could not parse
  before, "re-read every photo with no camera and no date" is the action a photographer actually
  wants, and it is a plain sequence of undoable `AddPhoto` updates
- ui: Library health shows the queue ("N photo(s) could not be read when indexed") with "Retry
  metadata" and "Re-read all without EXIF" — the second names its count and is disabled at zero.
  cli: `retry [--all]`
- Deliberate scope note: a file the *scanner* cannot read at all is still refused at scan time and
  named in the scan report (it has no identity yet, so there is nothing to queue). The queue covers
  reads that fail after the identity hash — a flaky volume, a file replaced mid-walk
- tests: meta distinguishes retryable from empty; catalog queues on sync, clears with an undoable op,
  and treats a re-queue as a no-op; engine reports `updated`/`remaining` with a file removed mid-retry
- 242 tests, CI green. Verified by hand: `retry`, `retry --all` updating two photos, and undo
  reversing it.

## 2026-11-21 — M7: files changed on disk

- engine: `check_health` now compares size **and** modification time against the index instead of only
  testing existence. `Health.modified` counts them and `Health.modified_ids` names them, so a refresh
  can act on exactly what the check found. A size difference is definitive; a time difference is only
  trusted beyond `MODIFIED_TOLERANCE = 2` seconds, so a filesystem with whole-second stamps cannot
  invent changes
- engine: `refresh_changed()` re-hashes (`sortlens_scan::content_key`) and re-extracts metadata for
  those photos only, storing one undoable `AddPhoto` update each — a rescan, narrowed to what moved
- engine: commands `library.changed` (counts and ids) and `library.refreshChanged`
- ui: the Library health dialog reports "N file(s) changed on disk since they were indexed" with a
  "Refresh changed" button that clears the count. cli: `changed` (listing the relative paths) and
  `refresh`
- `Health` lost its `Copy` derive to carry the id list; every other field is still plain counts
- tests: a quiet library reports zero changed (a stat alone must not invent work), replacing one file
  reports exactly that one, refreshing settles it to zero, and the refresh undoes like any other update
- 243 tests, CI green. Verified by hand: `changed` reporting one file by name, `refresh`,
  and `changed` quiet again
- Noted honestly: one `cargo xtask ci` run failed inside `sortlens-engine` before the message could be
  captured; three subsequent workspace runs, three engine-only runs and the next CI run were all green,
  so it is recorded as an unexplained flake rather than dismissed

## 2026-11-21 — M7: move picks

- engine: new `moves` module — `MoveItem { old, new, sidecar }`, `MoveReceipt { id, dest, at, items }`,
  `moved_record` (only the place changes: size, key and every review field carry over), `put_back`,
  `take_sidecar`, `prepare_dest` (absolute paths only)
- engine: `move_picks(dest, ids)` reuses `sortlens_cull::trash` for the filesystem half, so the batch
  is audited as a whole and a cross-volume move is copied, size-verified and only then removed. The
  catalog then follows the files in one `Op::Group` of `RemovePhoto`/`AddPhoto` pairs plus `AddRoot`,
  so the move is a single undo step
- engine: `undo_move(id)` puts files back **and** restores the records from the receipt, which is
  independent of the one-level undo — a move can be reversed tomorrow. Receipts persist in
  `moves.json`, alongside the rejects bin
- Fixed while testing by hand: the XMP sidecar was left behind, orphaning the review state it holds.
  It now travels with the photo and back again, and the receipt remembers whether it did
- commands `edit.movePicks`, `edit.undoMove`, `edit.moves` under Organise; ui: a "Move picks…" button
  beside Apply rejects (enabled by the pick count) and a "Moved" dialog with "Put them back"; cli:
  `move <dest> <ids…>`, `moves`, `unmove <id>`
- tests: the moved record keeps every fact but its place; putting a photo back creates the folders;
  a relative destination is refused; engine round trip with rating and keyword intact, the
  destination becoming an indexed root, a receipt surviving a lookup, the undo restoring file and
  record, an empty selection and a relative destination refused, and the sidecar check
- 247 tests, CI green. Verified by hand over the CLI including the sidecar travelling both
  ways

## 2026-11-21 — M7: side-by-side compare

- ui: `UiState.compare: Option<PhotoId>` and `toggle_compare` — opens a compare against the next photo
  in the current order (or the previous one when it is the last), and closes it on a second press
- ui: the detail canvas splits into two panes drawn at the **smaller** of the two fit scales, so the
  photo that needs less room is letterboxed instead of being blown up: the same detail at the same
  size in both panes is what makes a compare honest. `Z` still switches both panes to a shared 1:1
- ui: a difference line under the panes (stars, label, exposure when it differs, file size when it
  differs, pixel dimensions), `C` toggles the compare, `S` swaps which pane is left, and left/right
  walk the pair together so the two photos stay neighbours
- `C` in the detail view now means compare; the global cull shortcut is guarded against
  `detail_open`, so one key cannot mean two things in the same view
- A photo that disappears from the library closes the compare rather than leaving an empty pane
- tests: `compare_facts` reporting stars, pixels and size differences while staying quiet about
  identical frames; `toggle_compare` closing an open compare, and saying "Pick a photo first" rather
  than doing nothing when there is no selection (found by the test itself — the first version was
  silent)
- 249 tests, CI green

## 2026-11-21 — M7: colour-only duplicate matches

- hash: `Colour::mean()` — overall brightness, which is what makes the colour search exact below
- similar: `Match { Families, Colour, Either }` with `id`/`label`/`hint`/`parse`, `palette_matches`,
  and `group_in(members, mode, threshold, colour)`; `group_with` stays as the Families shorthand so
  existing callers keep their meaning
- similar: colour candidates come from a **brightness-sorted sliding window**. The distance is the
  mean of the absolute differences, so it can never be smaller than the difference of the means: two
  palettes whose brightness differs by more than the tolerance cannot match, and stopping the scan
  there is exact rather than a heuristic. No second index to keep in sync, and the window is small at
  any sane tolerance
- engine: `poll_similar_in(threshold, colour, mode)` and `regroup(...)`, which reuses the cached
  fingerprints so changing the mode costs no decoding; `similar.groups` takes `mode` and `colour`
- ui: a Match row in the Similar view — Structure / Colour / Either with the explanation on hover —
  plus a palette slider that only shows for Colour. Changing either regroups immediately
- tests: colour-only finds a same-palette pair whose hashes are opposite, refuses identical layout
  with a different palette, refuses a pair where one side has no fingerprint, and a chain of
  one-step palettes proves the window walks far enough to link them (and that a tighter tolerance
  than the step breaks every link); mode names round trip; engine-level, two flat colours never
  group and regrouping leaves the hash cache untouched
- 255 tests, CI green

## 2026-11-21 — M7: insights progress, highlights, seasonality

- insights: `Accumulator` — the same folding code behind both `Insights::compute` and the engine's
  background pass, so a progress bar can never disagree with the numbers it is progressing towards
- insights: `by_month: [usize; 12]`, `busiest_day: Option<Day>`, `peak_month()`, `peak_season()`
  (northern reading of winter/spring/summer/autumn, stated in the docs) and `highlights()` — up to four
  facts the charts cannot say: busiest month with its share and dominant season, the single busiest day,
  the most-used camera, and how much of the library has no capture date at all
- insights: `civil()` now returns year, month, day-of-month **and** the day's first second — the first
  version returned the day number in seconds where a day-of-month belonged, which the highlights test
  caught ("2024-02-1707523200")
- engine: `begin_insights()` (worker over a cloned photo list, progress every `INSIGHTS_CHUNK = 512`),
  `insights_progress()`, `poll_insights()`, a `JobKind::Insights` row in the activity stack, commands
  `insights.start` / `insights.progress`; `insights()` returns the finished pass when there is one and
  computes on demand otherwise; `view.insights` gains `byMonth`, `peakMonth`, `peakSeason`, `busiestDay`
  and `highlights`
- ui: the Insights view polls the pass, shows a progress bar with "done of total" and a Recount button,
  puts the highlights on one wrapped line and draws a twelve-bar month strip with the busiest month in
  the accent colour. cli: `insights` prints months and highlights
- tests: months and seasons from real timestamps (two February frames, a winter peak, leap-day and
  1970 handling), highlights including the busiest day, and the accumulator agreeing field for field
  with the one-shot pass; engine-level, the pass lands with the right count, reports no progress
  afterwards and agrees with `view.insights`
- 259 tests, CI green

## 2026-11-21 — M7: the keyword registry

- catalog: `aliases: BTreeMap<String, String>` in the snapshot and in memory, `Op::Aliased { changes }`
  with an exact inverse (a `Group` of per-alias inverses), `resolve_tag` (chains resolve all the way,
  with a step cap so a mistaken cycle cannot hang), `set_alias` (refuses a self-alias, no-ops when
  nothing changes), `aliases_of`, `orphaned_aliases`
- catalog: `rename_tag` now records `from -> to` as an alias **in the same op as the retag**, so one
  undo still reverses a rename — the first version committed two ops and a test that has been green
  since M5 caught it immediately
- catalog: new `registry` module — `Entry { name, count, aliases }`, `Registry { entries, orphans }`,
  `merged()`, `headline()` ("3 keyword(s) · 1 renamed into another · 1 alias(es) pointing nowhere")
- engine: `set_tag` resolves through the registry first, so typing `STRAND` after a rename lands on
  `beach`; `resolve_tag`, `registry_aliases_of`, `tag_registry`, `set_alias`; commands `tag.registry`
  and `tag.alias`
- ui: a keyword row with aliases says so on hover ("also written as strand"), which is where a
  photographer would otherwise wonder why two keywords became one
- cli: `registry` (counts, swallowed names, orphans) and `alias <name> [canonical]`; `tag`/`untag` print
  the keyword actually used and name the alias ("untagged beach (STRAND is an alias)")
- 263 tests, CI green. Verified by hand: rename, retype the old name, registry showing
  `2 beach <- strand`, and the CLI naming the alias

## 2026-11-21 — M7: the shortcut overlay and a wired keymap

- new `crates/ui/src/keymap.rs`: the keymap as a table — `Binding { keys, action, context, command }`
  over five contexts (Anywhere, Library, Detail, Cull, Compare), `search()`, `in_context()`, and a
  header that says how many bindings the overlay lists
- the overlay is `?` (or the sidebar "Shortcuts" row): a search field that takes focus on open, then
  each context with a one-line note and keycap-aligned rows
- **the table is checked against the command registry in both directions** — a binding that names a
  command must agree with that command's declared shortcut and be ready, and every ready command with
  a shortcut must appear in the table. It found four real drifts on its first run:
  `library.importSidecars` does not exist (the id is `library.import`), `library.import` is still M1 so
  Ctrl+Shift+I is not a live shortcut, `photo.flag` is declared `P / X` not `P`, and `cull.back` was
  written `Left` where the overlay writes `←`
- **the registry was claiming shortcuts the shell never handled.** `Ctrl+O`, `Ctrl+Shift+R`,
  `Ctrl+Shift+N`, `Ctrl+Z`, `Ctrl+,`, `Ctrl+Q`, `Ctrl+Enter` and the `I`/`S`/`D`/`C` view keys are now
  wired in `command_keyboard`, which consumes the press so the plain-key handlers behind it do not
  also fire; the view letters are refused while the detail view or a cull session owns them, so `S`
  still swaps a compare
- colour labels get their keys: `6–9` for red, yellow, green, blue (pressing again clears), and `0`
  stays "clear the rating" — `ColorLabel::KEYED` documents why purple is left off the row. The
  registry's `6–0` was wrong and is now `6–9`
- cli: `shortcuts` prints the bound commands (17 today) from the `app.shortcuts` command, so the
  wiring is checkable without the GUI
- 268 tests (5 keymap tests), CI green

## 2026-11-21 — M7: memories, and capture dates that can be corrected

- new crate `sortlens-memories` (23 → 24 workspace crates): `Memories::build(candidates, today,
  limit)` produces three deterministic cards — **On this day** (same month and day in earlier years,
  29 February falling back to the 28th so the card does not vanish three years out of four), **This
  week in past years** (a seven-day window compared by month and day, so a window across new year
  wraps instead of going empty), and **Never looked at** (no stars, no flag, at least a year old,
  oldest first). Cards are ordered best-first and say what they left out ("3 of 10 photos · since
  2019"). Dates are read as UTC and that trade is written down rather than papered over.
- engine: `memories(today)`, command `memories.list` (with an optional `today` so a card can be
  reproduced), and **`photo.setCaptured`** — a photo whose EXIF date is wrong could otherwise never
  take part in "on this day", so the date is now editable as an ordinary op (undoable, logged,
  written to the sidecar)
- catalog: `Op::Captured` with an exact inverse, and **`Op::is_noop()`**: `commit` now drops an op
  whose inverse changed nothing instead of logging it and spending the single undo level on it. The
  capture-date test caught this — undoing after a no-op commit had done nothing at all
- ui: a **For You** destination (bottom bar, 6th view) drawing each card as a strip of thumbnails;
  clicking one opens the detail view
- cli: `memories [--today YYYY-MM-DD]` and `date <id> <YYYY-MM-DD|->`; `--today` validated against
  real month lengths so `2021-02-30` is refused
- docs: `docs/BEHAVIOUR.md` gains a Memories section — derived not stored, UTC, the leap-day and
  new-year rules, deliberate overlap between cards, and why a judged photo leaves "never looked at"
- 278 tests (8 in `sortlens-memories`), CI green. Verified by hand: two dates set, three
  cards for 2024-06-24, and the change log showing both date writes

## 2026-11-21 — M7: a chart component

- new `crates/ui/src/charts.rs`: `Column { label, value, highlight }`, `columns()` (a column chart
  with a baseline, labels, hover tooltips and a highlight colour), `share_row()` (the proportional
  row the insights breakdowns use), `empty()` (a faint box that says "nothing dated yet" instead of a
  layout that silently lost its chart), and the arithmetic underneath as pure functions
- three rules, each with a test: **a non-zero value never rounds below one pixel** (an invisible bar
  reads as "nothing here"), **the tallest bar fills the height exactly** (integer-first arithmetic so
  a bar cannot stop short of its own axis), and **`share_of` refuses zero** rather than dividing by an
  empty library. `nice_max` gives round axis maxima (1/2/5 × powers of ten)
- heights are **fixed pixels**, never a fraction of the window: a chart that grows with the window
  makes the same library look different at different sizes and makes screenshots incomparable
- insights now draws its months through the shared chart (initials under the columns, busiest month
  in the accent) and its rating/label/kind/timeline rows through `share_row`, which also gained the
  tooltip the old local copy lacked — one drawing routine instead of two
- For You draws a small year chart per card, so a card that says "since 2019" shows whether the
  photos are spread across those years or all come from one
- `sortlens_memories::civil_year` added for the card chart, rather than duplicating date maths in UI
- the chart paints in a **headless egui frame** in a test (`Context::run_ui` with no window, texture
  deltas cleared), so bad geometry or an empty series fails in CI rather than in front of a user
- 285 tests (6 in `charts`), CI green

## 2026-11-21 — M7: show a file to the desktop

- new `crates/core/src/reveal.rs`: the two OS actions as **pure command construction** —
  `Desktop { MacOs, Windows, Linux }` with `current()`, `reveal()` (`open -R` / `explorer /select,` /
  `xdg-open <folder>`), `open_with_default()` (`open` / `cmd /c start ""` / `xdg-open`) and
  `reveal_selects()` so a caller can tell the truth about Linux. Arguments are always a list, so a
  file called `$(rm -rf ~).jpg` is one argument and never a command; six tests pin the exact argv,
  including the empty `start` title argument Windows needs
- **the split is the design**: the engine builds the command, the host runs it. `photo.reveal` and
  `photo.open` return `{program, args, line, ran: false}` — a library held in memory has no
  process table, so a command that claimed to have run would be a lie. `Engine::reveal_target` and
  `path_of` do the resolution and refuse a file that is no longer on disk with "rescan or relocate"
- app: `Action::Reveal` / `Action::OpenExternal` spawn the built command and report the line (or the
  desktop's inability to select a file on Linux) in the status bar
- ui: "Show in folder" and "Open" in the grid's context menu and in the detail header, next to Close
- cli: `reveal <id>` and `open <id>` execute the same command
- docs: `docs/BEHAVIOUR.md` gains "Showing a file to the desktop" — who builds, who runs, why
  arguments are a list, the Linux folder deviation, and that nothing here mutates the catalog
- 292 tests, CI green. Verified end to end: the CLI ran `open -R /tmp/sl-rev/ph/a.jpg` (Finder
  opened), and a file deleted behind the app's back was refused with the rescan instruction

## 2026-11-21 — M7: the MCP server

- `crates/mcp/src/lib.rs` is no longer a scaffold: newline-delimited JSON-RPC over stdio —
  `initialize`, `notifications/initialized`, `ping`, `tools/list`, `tools/call`. `Mcp::handle(line)`
  takes one line and returns one line (or `None` for a notification), so the whole protocol is
  testable without a pipe, and `Mcp::serve(reader, writer)` is the stdio loop
- **a tool is a command id**: `tools/list` is the command registry filtered to `Ready`, `tools/call`
  is `Engine::run`. No second list of agent actions, no per-tool schema to keep in sync, and a
  command still marked `Planned` stays out of the list — a tool that can only refuse is noise
- a refusal is a **tool result** (`isError: true` with the engine's own message), not a JSON-RPC
  error; JSON-RPC errors are reserved for malformed requests (`-32600`) and unknown methods
  (`-32601`). Notifications are never answered, because an extra message corrupts the stream
- `sortlens-cli --library DIR mcp` serves it. Verified end to end by piping four lines into the real
  binary: `initialize` answered the protocol version, the `notifications/initialized` line produced
  no output, `photo.rate` rated two photos, `library.stats` reported them, and `history` afterwards
  showed `#5 rate Rated 2 photos` — the agent's write lands in the same op log the user's undo reads
- six tests, including an agent culling a real three-photo library through the protocol
- 298 tests, CI green. **Not verified by hand**: a real MCP client (Claude Desktop et al)
  connecting to the binary; the protocol was exercised with piped JSON only
- docs: `docs/ARCHITECTURE.md` gains an MCP section (tool = command id, refusal is a result, why
  notifications go unanswered, and that writes go through the ordinary op log)

## 2026-11-21 — M7: arguments an agent can copy, proven by running them

- new `crates/mcp/src/examples.rs`: a worked argument set and a one-line note per command (55
  examples) plus `NEEDS_CALLER` for the ones whose arguments only the caller can supply — a folder to
  index, an archive to restore, a destination to move into (12 of those). `tools/list` carries both:
  the note goes into the description an agent reads before choosing, and `arguments.example` carries
  the exact JSON.
- **the examples are executed, not remembered.** `every_published_example_actually_runs` builds a
  fixture library, walks the table in order through `tools/call`, and fails the build on any
  `isError`. That is the property that matters: a published name is one the engine accepts, so a
  guess cannot drift into the docs.
- it immediately paid for itself, finding four kinds of real problem:
  1. **my own wrong names** — `collection.*` takes `id`, not `collection_id`; `cull.apply` takes
     `mode: "tag"`, not `"tag_only"`; `library.subfolders` requires `root`.
  2. **state dependencies** — `library.restoreBatch`, `library.forgetBatch` and `edit.undoMove` need
     a batch or a receipt, so the test creates one and reads the id back rather than assuming `1`
     (assuming it let a stale id pass on the first attempt).
  3. **an example ordering bug** — `cull.back` un-decides the staged photo, so `cull.apply` after it
     said "nothing is staged yet"; `cull.back` is now last, with a comment saying why.
  4. **two commands that were `Ready` with no handler at all**: `view.detail` and `view.similar`
     returned "command has no handler". Both now answer, via a new `Effect::ShowView(&'static str)`
     that the app turns into a view change — the engine names the view and the host switches it.
- the coverage test also refuses to let a ready command go undocumented, which is why the list is
  complete rather than partial.
- 302 tests, CI green. Verified against the real binary: `tools/list` returns 66 tools, all
  66 with argument guidance. **Not verified by hand**: a real MCP client; the protocol is exercised
  with piped JSON and the examples with direct calls.

## 2026-11-21 — M7: the registry and `Engine::run` now audit each other

- new test `every_ready_command_has_a_handler_and_every_handler_is_a_command` in `command.rs`: it
  reads the engine's own source (`include_str!("lib.rs")`), isolates `Engine::run`, collects its arms
  and compares both ways. Reading source in a test is blunt, but it is the only check that catches a
  half-connected command, and it found both bugs that had already shipped
- **fixed `library.page`**: `run` handled it while the registry had no entry, so it was invisible to
  the palette and to MCP. It is now `library.page` · "Read a Page" in `Group::Library`, with an MCP
  example that the examples test executes
- **fixed a quieter bug next to it**: `library.page` hard-coded `Sort::Natural`, so a caller could
  page but never order by rating or date even though every other paging path can. It now takes
  `sort` and reports the order it used; a test rates the last photo by name and asserts it leads a
  rating-sorted page
- cli: `page [--offset N] [--limit N] [--sort KEY]` prints one page as JSON, so a script can walk the
  library without the app
- 304 tests, CI green. Verified by hand: `page --sort rating` returned the five-star photo
  first. **Not verified by hand**: the palette entry for "Read a Page" (the UI does not call it; the
  grid uses `Engine::page` directly)

## 2026-11-21 — M0 debt: craft-fonts, optional by design

- `crates/ui/build.rs` reads `CRAFT_FONTS_DIR` and writes a `CRAFT_FONTS: &[CraftFont]` table, with
  **unset as a supported configuration** — CI has no checkout, so the empty table is the configuration
  under test. A native build embeds all 21 faces.
- `crates/ui/src/fonts.rs`: `install()` registers them and **appends** each face to `Proportional` and
  `Monospace` after egui's own, never prepended — a fallback that changes Latin text is a bug.
  `families()`, `embedded()`, and `coverage()` over `REQUIRED_SCRIPTS = ["Latn", "Zsym"]`, because the
  keymap draws `⌃ ✕ ◀ ✓` as text; `TRANSLATED_SCRIPTS` lists what translations will need.
- the app installs them in `App::new` before the first frame and logs the face count, so a build with
  fonts says so once instead of looking identical to one without.
- `cargo xtask fonts` (now part of `xtask ci`): unset prints how to enable them and passes; a set
  checkout reports face/family counts and script coverage, and a manifest with no Latin face fails.
  `fonts/embedded.txt` records what shipped, so a release can diff it.
- **verified with the real checkout**, cloned to gitignored `.fonts/craft-fonts`:
  `CRAFT_FONTS_DIR=… CRAFT_FONTS_REQUIRED=1 cargo test -p sortlens-ui -p sortlens` passes (30 ui
  tests, fonts installed), `cargo build -p sortlens` links, and `cargo xtask fonts` reports
  "21 face(s) in 20 family(ies) … Latn covered · Zsym covered"
- 307 tests, CI green in both configurations (with and without `CRAFT_FONTS_DIR`)
- **not verified by hand**: how the text actually looks with the craft faces — no screenshot, and the
  font stack is asserted structurally (defaults kept, first face unchanged, fallbacks appended), not
  visually. The app's `TIM` typography also still uses egui's default sizes.

## 2026-11-21 — M0 debt: a supply-chain gate that actually bites

- `deny.toml`: an allow-list, not a snapshot of what happens to be in the tree. Allow-listed: 0BSD,
  Apache-2.0 (and with LLVM-exception), BSD-2/3-Clause, BSL-1.0, IJG, ISC, MIT, MPL-2.0, OFL-1.1,
  Unicode-3.0, Ubuntu-font (per-crate exception for `epaint_default_fonts`), Unlicense, Zlib, with a
  `unicode-ident` exception for its Unicode-3.0 tables. GPL/AGPL/LGPL/CC-BY-NC fail by omission.
- `[sources]` allows exactly three git origins — `storytold/craft-libs`, `storytold/egui`,
  `storytold/winit` — so a new git dependency is a decision, not a diff nobody reads. (The first
  version listed `emilk/egui`; the check caught the fork URL, which is the point.)
- `[bans]`: `wildcards = "deny"` with `allow-wildcard-paths = true`. That needed every workspace
  member to say `publish = false` (25 `Cargo.toml`s, inherited from `[workspace.package]`), because
  cargo-deny treats a publishable crate's path dependencies as wildcards — the honest answer is that
  nothing here is published to crates.io; the repository is the distribution.
- **`BSL-1.0` and `Ubuntu-font-1.0` are deliberately absent from the allow list** even though crates
  in the graph carry them: `ryu` is "Apache-2.0 OR BSL-1.0" so BSL is never *chosen*, and the font is
  covered by the exception. Listing them produced "licence not encountered" noise, which trains
  people to ignore the output.
- `cargo xtask deny`: creates no pass when cargo-deny is missing — it fails and says
  `cargo install cargo-deny --locked`, because a check that silently does nothing is worse than none.
- CI: its own `deny` job on ubuntu, so a licence or advisory regression is named as that instead of
  hiding inside a three-OS test log.
- **the gate was proved to bite**: with MIT removed from the allow-list the licences check FAILS, and
  with `storytold/winit` removed from `[sources]` the sources check FAILS. Both reverted.
- `deny.toml`, `xtask deny`, README, TOOLING and CONTRIBUTING (including "a dependency change is a
  licence change") record the policy where a contributor will read it.

## 2026-11-21 — M0 debt: golden outputs, and a testkit to feed them

- `sortlens-testkit` is real instead of a scaffold: `Fixture` writes solid-colour JPEGs through the
  **real encoder** (so a fixture photo decodes, hashes and has a resolution), writes sidecars under
  the `docs/BEHAVIOUR.md` §4 naming rule, and removes its folder on drop. The same bytes every run is
  the property the golden job depends on, and `the_same_fixture_writes_the_same_bytes_twice` pins it.
  The sidecar helper's first version wrote `a.jpg.xmp`; the golden run showed no keyword imported,
  which is how the convention (`a.xmp`) got enforced in the helper instead of in a comment.
- `apps/sortlens-cli/tests/golden.rs` records **30 invocations of the real binary**
  (`CARGO_BIN_EXE_sortlens-cli`) against a fixture library: add ×2, roots, stats, list, keywords,
  folders, check, changed, rate, tag, rename, registry ×2, alias, untag-through-the-alias, date ×2,
  memories, insights, page, undo, history, shortcuts, move, moves, unmove, audit, compact, and the
  catalog's own `snapshot.json`.
- only three things are normalised — the fixture path, `in <elapsed>s`, and `"modified":<mtime>` — so
  a renamed field or a reworded summary is a visible diff. `page`'s JSON is checked verbatim
  otherwise, keys and content hashes included.
- both directions are checked: a missing golden fails, and so does a golden whose step no longer
  exists, because otherwise a deleted step leaves a claim passing forever.
- `cargo xtask snapshot` is **read-only**; `--update` sets `SORTLENS_UPDATE_GOLDEN=1` and prints a
  reminder to read `git diff`. A check that can update its own expectation checks nothing.
- CI: its own `snapshot` job on ubuntu. The fixture bytes and normalised output are
  machine-independent, so a second OS would only add noise.
- **proved to bite, twice**: editing one digit in `04 stats.txt` fails with the expected/actual diff,
  and dropping a `99 orphan.txt` into the folder fails with "no step records this golden any more".
  Both were reverted. The suite is deterministic across repeated runs (three consecutive read-only
  runs green).
- CONTRIBUTING says what to do when it fails: read the diff, `--update`, and say in the commit body
  what the output now says. Regenerating to get green is how a golden stops meaning anything.

## 2026-11-21 — the reconciliation pass, and what it caught

- **ROADMAP now says what is there.** 26 rows were marked unshipped while the code shipped them
  (M7's delete modes, backup/restore, remap, audit/compaction, read-only mode; M4's gestures, zoom,
  lookahead, apply receipt, quit guard, announcements; M5's collection colours/sort/search, move,
  wizard, tag rename/merge; M6's compare and insights worker; M1's folder tree and add/remove/rescan).
  Each was checked against the registry or the crate before ticking. The duplicate rows are gone: the
  catalog row that repeated a ticked one, and the cancelling/scheduler pair that appeared twice each.
- **the rows that stayed unchecked now name what is missing** instead of describing a whole feature:
  no density control, no multi-select or marquee, no justified rows, no `PathKey` or volume probe, no
  magic-byte sniffing, no `tagFromPrev`, no 20-step undo, no folder-level colours, no `--json` for the
  CLI verbs, no stale-revision rejection.
- **two real durability bugs, found by writing the two missing corruption tests.** `snapshot()`
  truncated the log after writing a snapshot, so a snapshot that was later damaged (half-written by a
  crash) left ops with nothing to act on: opening the library failed with `NoSuchPhoto` and the whole
  catalog was gone. Now `snapshot()` keeps the log — a snapshot is only an accelerator — and
  `compact()` is the single thing that empties it, after reading the snapshot back and checking its
  sequence, failing with "the snapshot did not read back; the log is kept". A record that will not
  replay stops the replay and is reported instead of aborting the open.
- tests added: `a_flipped_checksum_costs_one_record_not_the_library` (the good records before it
  survive, and the damaged record is the rating), `a_partial_snapshot_falls_back_to_the_log`,
  `compaction_is_what_empties_the_log`; `a_snapshot_rollover_keeps_one_undo_level` was rewritten to
  assert the *new* contract (the log keeps its records, `skipped` counts what the snapshot covers).
  The old version asserted the log was empty, which is what let the bug through.
- docs/BEHAVIOUR and ARCHITECTURE describe the snapshot/compaction rule as built; ROADMAP's stale
  "State:" lines for M0/M1/M2 are corrected; PLAN §12 is now "where delivery stands" instead of an M0
  to-do list, and §5.2 says it is the plan as written rather than a status report.
- README's badge and status block say M0–M7, list what works, and have a "what is not built yet"
  section so the absences are stated rather than implied.

## 2026-11-21 — M8 (half): export

- new crate `sortlens-export` (25 workspace crates). Planning is separate from writing, so every
  destination and every collision is decided before a byte moves: `plan()` takes the folder's existing
  names and returns jobs with a `skip` reason, which is what lets the UI say "0 written, 1 skipped"
  instead of starting a bar that never fills.
- **Format::Original** is a byte copy that keeps the source's extension case (`IMG_0001.CR3` stays
  `.CR3`); **Format::Jpeg** decodes, optionally clamps the *long* edge, re-encodes at a validated
  quality, and carries only APP1 (EXIF) and APP13 (IPTC) forward — forwarding every application
  segment would carry another program's editing state. PNG/TIFF/WebP are **not** offered: the
  workspace has one encoder, and a format chip that cannot write its format is a lie.
- collision policy is `Skip` / `Rename` (`name-2.jpg`, up to `RENAME_LIMIT = 1000` then reported) /
  `Overwrite`; two same-named photos inside one export collide with each other, not just with the folder.
- engine: `begin_export` / `export_progress` / `cancel_export` / `poll_export` / `last_export`, a
  worker thread and `JobKind::Export` in the activity stack. A photo that is no longer on disk is
  named and the export refuses. **No op is logged** — an export changes nothing about the library, so
  `docs/BEHAVIOUR.md` says why it cannot be undone: there is nothing to undo.
- commands `edit.export` and `edit.cancelExport` (+ MCP examples, which are executed by
  `every_published_example_actually_runs`); `all_photos()` became public for whole-library operations,
  with the doc comment that had drifted onto `key_for` put back.
- CLI `export <dest> [ids…] [--format] [--max] [--quality] [--collision] [--no-metadata]`, waiting for
  the worker and printing the receipt.
- UI: "Export picks…" in the cull bar, "Export…" in the grid context menu and the detail header, and an
  `export_dialog` with format chips, edge/quality, metadata, three collision words, progress, Stop, and
  a receipt that repeats that the originals were not touched.
- **the golden job caught a real bug the first time the verb was recorded**: `--max 24` was read as
  photo 24 (`no such photo: 24`), because the id list was `filter_map(parse)` over every argument. The
  CLI now walks arguments and skips flag values. New goldens: `30 export`, `31 export again` (JPEG,
  `--max 24`, `--collision rename`), and the `%PHOTOS%` placeholder that lets a step name an absolute
  path without the golden depending on where the fixture landed.
- **not verified by hand**: the dialog in a real window (no screenshot, same as every UI commit here);
  the headless `Context::run_ui` test paints it with nothing selected, with a full JPEG spec and with a
  receipt, and the engine tests exercise the worker against real JPEGs — but nobody has clicked
  "Choose folder…" in front of a person.
- 336 tests, `cargo xtask ci` green.

## 2026-11-21 — M8 (second half, part 1): the model store, and the promise it cannot reach the net

- `crates/ai` is no longer a scaffold. Three modules, no inference and no downloader — both stated in
  the crate docs so nobody assumes the other half exists.
- `sha256`: its own implementation rather than a dependency, checked against every FIPS vector
  (empty, `abc`, the 56-byte two-block case, the 64-byte aligned case, 1000 bytes), cross-checked
  against Python's `hashlib` while writing it, plus hex round-trip and a one-bit-change test.
- `manifest`: `Model { id, label, kind, licence, sha256, bytes, url, input_edge }`, `ModelKind`
  (Detector/Embedder/Captioner), `Manifest::parse` that refuses a newer schema and treats a missing
  digest, a zero size or a blank licence as errors. `verify()` checks **size first** (free) then the
  digest, and distinguishes a truncated file from a wrong one because the fixes differ.
- `store`: files are named `<sha256>.<kind>`, so a model cannot be silently replaced and two models
  cannot collide. `install` verifies in memory, writes `.incoming`, **reads it back and verifies
  again** (a short write is what a full disk looks like), then renames. `remove` refuses to delete a
  file it cannot identify — that file may be someone's download in progress.
- the shipped manifest is **empty**, on purpose: no model has been chosen, pinned and licensed for
  redistribution, so there is nothing to describe and no command or menu item was added. A settings
  row that lists nothing is better than a "Suggest tags" button that cannot work.
- `no_network.rs`: a source scan for socket APIs and a dependency scan for HTTP clients, proved to
  bite by planting `std::net::TcpStream` in `lib.rs` — it failed with the file name and the symbol, and
  was reverted. The file also carries a test that the scan *can* match real code, because a check that
  cannot fail is not a check.
- ROADMAP: the manifest/store row (SL-F01) and the no-network row (SL-F10) are ticked, with the
  downloader (SL-F02) called out as unbuilt and unlisted; ARCHITECTURE's ai section describes what
  exists and keeps the download plan clearly in the future tense.
- 362 tests, `cargo xtask ci`, `deny` and `snapshot` green.

## 2026-11-21 — M8 (second half, part 2): the arithmetic a detector needs, tested without a detector

- `crates/ai/src/detect.rs`: `Box` in **normalised** coordinates (so a 320 px and a 640 px copy of the
  same photo mean the same thing), `iou`, and `suppress` — NMS **per class**. Per class is the part
  that matters: a person in front of a car overlaps it almost completely, and suppressing across
  classes would delete one of them. A zero-area box cannot suppress anything, so a broken detector
  emitting a thousand empty boxes cannot erase the one real one — that case is a test.
- ties are broken by score, then class, then position, so a shuffled input produces the identical
  output (pinned by `equal_scores_still_produce_the_same_order_every_run` and the reversed-input test
  in `suggest`). NaN and sub-`MIN_SCORE` guesses are dropped before anything is compared.
- `LabelMap::parse` accepts `index name`, `index: name`, bare names and `#` comments, and **refuses a
  hole** in the indices — a map with a hole mislabels everything after it.
- `crates/ai/src/suggest.rs`: `merge` is where a guess becomes a proposal. A keyword the photo already
  has is never suggested again; two sources proposing the same tag become one at the higher score; a
  camera hint wins a tie against a guess because it is not guessing; tags shorter than three characters
  are dropped ("a", "of" are what a captioner says when it has nothing to say); the list is capped at
  `DEFAULT_PER_PHOTO = 8`, ordered by confidence then name.
- **nothing here writes to the catalog.** Accepting a suggestion is the only path from a proposal to a
  keyword, and that half (panel, `source = Ai` filter, the tag write) is not built — ROADMAP's two rows
  are split into "part 1" (ticked) and "part 2" (not) rather than left ambiguous.
- `rustc`/clippy pushed two small corrections worth keeping: a negated float comparison became
  `matches!(area().partial_cmp(&0.0), Some(Greater))` because that is what actually catches a NaN, and
  a box built from corners is compared with a tolerance (0.6 − 0.2 is not exactly 0.4).
- 385 tests, `cargo xtask ci`, `deny` and `snapshot` green. No new dependency: this is still a
  crate that cannot reach the network.

## 2026-11-21 — M8 (second half, part 3): the hints that need no model

- **why not the model**: pinning a real detector needs weights to hash, and no model file exists in
  this environment and none can be fetched here — inventing a digest for a file nobody has would be
  the exact lie this project keeps refusing. `tract` was therefore not added either: a session with no
  weights is untestable shape, and the checkpoint's own fallback was this.
- `crates/ai/src/hints.rs`: camera and lens names become keywords; focal length becomes `wide`
  (≤ 35 mm), `normal` (≤ 70 mm) or `telephoto`; ISO ≥ 3200 is `low light`; 1/15 s or slower is
  `long exposure`. Each threshold is a named constant with the reason written beside it, and every hint
  carries `Source::ExifHint` so the panel can say the camera said it, not a guess.
- the rules refuse nonsense rather than guessing: a focal length of 0, negative or NaN produces no
  band; a shutter of 0, negative, NaN or infinite produces no hint; a lens name that only repeats the
  camera (different case, same words) is not suggested twice. Output order is fixed, so a scan order
  cannot reshuffle the panel.
- **a real bug caught by a test written for the happy path**: the long-exposure rule compared
  `shutter <= 1/15`, which is backwards — seconds *increase* as the shutter slows, so a 4-second
  exposure matched nothing while a 1/1000 s one did. The constant is now
  `LONG_EXPOSURE_AT_OR_ABOVE` with the direction spelled out in its doc comment, and the boundary test
  asserts 1/15 s counts while 1/1000 s does not.
- hints feed `suggest::merge` unchanged — a test runs a camera hint through the merger against a photo
  that already has that camera keyword, and gets only `low light` back.
- ROADMAP's SL-F05 row is ticked; 395 tests, `cargo xtask ci` and `deny` green.

## 2026-11-21 — M8 (second half, part 4): the suggestion pipeline reaches the app

- engine: `Engine::suggestions` (one photo), `suggestions_for_all` (a selection) and
  `accept_suggestion`. The mapping from `Photo` to `ai::Hints` is a free function, `suggestions_for`,
  so it can be tested against a constructed photo — a fixture JPEG carries no EXIF, and a test that
  needed a camera in the file would be testing the scanner instead.
- commands `suggest.list` (ids → each photo's offers, with tag, score and source) and `suggest.accept`
  (id + tag). **Accepting goes through `Engine::set_tag`**, so it is an ordinary `Tagged` op: it is in
  the log, it is in the history, and `undo` takes it back exactly like a keyword typed by hand — tested,
  including that accepting twice reports `tagged: false` rather than claiming work it did not do, and
  that an empty tag or an unknown id is an error rather than a silent no-op.
- CLI `suggest <id>` prints score, source and tag; `accept <id> <keyword>` writes it.
- UI: a "Suggested" row in the detail view, one button per offer, hover saying the score and whether the
  camera or a model said it. The row is drawn by `suggestions_row`, which takes the offers rather than
  the engine, so the headless test paints it with real suggestions and headless egui proves it draws.
- MCP examples for both commands — executed, not remembered, by the existing test.
- **the golden test earned its keep again**: while extracting the CLI arm I sliced out the `keywords`
  arm by accident, and `06 keywords` failed immediately with `unknown command: keywords` before the
  commit existed. Restored, and three new goldens (`30 suggest`, `31/32 accept`, renumbered exports and
  snapshot after them) now pin the new surface. Renamed goldens were deleted first, so the orphan check
  never had to be loosened.
- ROADMAP: SL-F06 part 2 ticked. A new row records what is *not* done: `source = Ai` in the keyword
  filter needs a per-tag provenance in the catalog, which is a schema change, so the panel shows
  provenance in its own row only.
- **not verified by hand**: how the row looks in a window. No click was simulated headlessly — the
  button's action is covered by the engine test and the app's handler by the compiler.
- 400 tests; `cargo xtask ci`, `deny` and `snapshot` green.

## 2026-11-21 — M8 (second half, part 5): keyword packs are data

- `ai::packs`: a pack is JSON — a name, a licence, and rules whose `when` conditions (folder, camera,
  lens, minimum ISO, focal band) are **ANDed**. The ones that would misbehave quietly are refused: a rule
  with no conditions (one typo would tag the whole library), a blank name/licence/keyword, a pack with no
  keywords, a band that is not `wide`/`normal`/`telephoto`, and — via `deny_unknown_fields` — a
  misspelled condition, which is otherwise a condition that silently never matches.
- "No value on the photo" is a non-match, not a wildcard: a photo with no ISO does not match `min_iso`,
  and one with no focal length does not match a band. Pinned by tests.
- `Catalog::set_pack`: the switch is an op (`SetPack`), so it persists across restarts, appears in the
  history, and undoes; switching to the state it is already in changes nothing and does not spend the
  undo level. Tested across two reopens, with the log being what carries it — the first version of that
  test asserted a snapshot had carried it, which is not what happens (snapshots roll over every 256
  records), so the assertion now says "one record in the log".
- `Engine::packs` lists every file in `<library>/packs/`, **including the broken ones**, each with its
  problem: a mistyped pack someone was relying on must say so rather than vanish. Enabled packs feed
  `suggestions` through the same merger, so a pack keyword is a proposal like any other.
- `packs.list`/`packs.set` commands, MCP examples (executed), `packs`/`pack <name> on|off` in the CLI,
  a golden step, and Settings ▸ Keyword packs with a switch per pack and the licence beside it.
- `docs/PACKS.md` documents the format, the table of what is refused and why, and what a pack cannot do.
  `craft-flags` was deliberately not used: a boolean in the op log already persists and undoes, and a
  flag crate would add a second place for the truth to live — recorded on the ROADMAP row.
- **the golden test caught the same mistake twice**: slicing the CLI arms by index deleted the
  `keywords` arm again, and `06 keywords` failed with `unknown command: keywords` before either commit
  existed. Restored both times. The lesson is in the diff, not just in this file: the CLI match is
  long, and a text slice that ends at the next arm is not a safe edit.
- **not verified by hand**: the Settings rows in a window; no checkbox was clicked headlessly.
- 413 tests; `ci` (exit 0), `deny`, `snapshot` green.

## 2026-11-21 — M8 (second half, part 6): the batch pass, and M8 called done honestly

- `Engine::begin_batch_suggest` (+ `batch_progress`, `cancel_batch`, `poll_batch`, `batch_results`,
  `clear_batch`): the same merge as the detail view, run over the whole library on a worker with the
  progress/cancel/poll shape the other jobs use. `JobKind::Suggest` labels it "Suggestions".
- **it writes nothing**, and that is the tested promise: a test snapshots every photo's tags and the
  change log length, runs a pass over a library where a pack matches all three fixtures, asserts three
  results and then asserts the tags and the log are byte-identical. Accepting one of the proposals
  afterwards is what puts a keyword on the photo, through the ordinary tag op.
- commands `suggest.start`/`progress`/`cancel`/`results`/`clear`; the registry audit refused to pass
  until `progress` and `clear` were named there too, so no command can be unreachable from a menu; MCP
  argument notes and examples for all five (executed by the existing test); `batchsuggest` in the CLI
  with a golden step; a "Suggested keywords" panel opened from Settings, one button per proposal, no
  apply-all on purpose.
- `show()` had grown past the complexity gate, so every dialog moved into one `dialogs()` function — a
  mechanical change, but it is why this commit also touches unrelated-looking lines.
- **M8 is now called done honestly**: the three rows that need something this repository does not ship
  (a `tract` session and embeddings need weights; `source = Ai` needs tag provenance in the schema;
  PNG/TIFF/WebP need an encoder) moved from unchecked boxes into a "Deliberately absent from M8" table
  with the reason and what would unblock each. The demo line now describes something a person can
  actually do today.
- **not verified by hand**: the new panel in a window; the headless test paints it idle and mid-pass, and
  no click was simulated. 416 tests, ci (exit 0), deny, snapshot green.

## 2026-11-21 — M9 (part 1): the CLI answers in JSON, and refuses when it cannot

- `--json` prints the engine's own answer for 16 read verbs (roots, keywords, stats, list, folders, check,
  moves, changed, history, audit, memories, shortcuts, registry, packs, suggest, jobs), anywhere on the
  command line including before the verb.
- a verb without a schema **refuses and lists the ones that have one**. That is the difference between a
  script failing loudly and a script parsing prose; the refusal is a golden, so the message is contract.
- `library.roots` and `tag.keywords` added (registry + MCP note + executed example), tested to answer an
  empty list rather than an error for an empty library.
- **the new goldens found a real defect in the harness**: it normalised `"modified":<digits>` but
  pretty-printed JSON writes `"modified": <digits>`, so those goldens would have failed on the next
  machine. Fixed for both spellings; the first attempt used a trimmed length as a byte index and panicked,
  which the test caught immediately. Proved to bite by renaming a key in the roots golden.
- docs/TOOLING.md has a "Scripting the CLI" section with the verb list and the refusal text; ROADMAP
  ticks SL-K03/K06 and leaves one row naming what is still missing.
- 417 tests; ci (exit 0), deny, snapshot green. **Not verified by hand**: any of this in a real
  terminal — the goldens run the binary, which is the closest this environment gets.

## 2026-11-21 — M9 (part 2): the diagnostics bundle, with the leak test the golden would have hidden

- `Engine::diagnostics` (`crates/engine/src/diagnostics.rs`): plain text with versions, counts, warnings,
  the read-only health check, packs, the last 25 changes and the roots by placeholder.
- `diagnostics::redact` replaces every absolute path — `<root-N>`, `<library>`, `<home>` — sorted
  longest-first so a root under the library is not half-replaced. Root-relative paths stay.
- `diagnostics [--out FILE]` in the CLI; `app.diagnostics` in the registry with an MCP note and an
  executed example.
- **the test that mattered**: the golden normaliser replaces the library and fixture paths, so a *leak*
  would be invisible in a golden diff. A separate test runs the binary, reads stdout before normalisation
  and fails if either path appears. Without it the promise would have been untested by construction.
- the golden itself would have failed on the ubuntu job (`os = macos`); os/arch and version lines are
  recorded as `<host>`/`<version>`. First attempt inserted the normaliser before the tail was appended,
  which silently undid it — visible only because the golden was re-recorded and still said `macos`.
- M9's remote rows (server client, remote album edits) moved to a "Deliberately absent from M9"
  table: they need a second device and a network peer, and an untested auth path is worse than none.
- 422 tests; ci (exit 0), deny, snapshot green. **Not verified by hand**: the report as pasted into
  a bug tracker.

## 2026-11-21 — M10 (part 1): one version, and a release that says what it does not have

- `cargo xtask version` (in `ci`): the root `[workspace.package] version` is the only place a number is
  written. It fails when a member declares its own version or stops inheriting the workspace one, and when
  `sortlens_core::VERSION` stops coming from `CARGO_PKG_VERSION`. **Proved to bite** by hardcoding
  `version = "0.0.9"` in `crates/ai/Cargo.toml` (the first attempt at that broke `rust-version` instead,
  which is why the second attempt replaced the exact line) and restoring it.
- `sortlens-cli --version` / `-V` and `--help`: answered **before** a library is opened, so a version
  question cannot create one. A test runs the binary with `SORTLENS_LIBRARY_DIR` pointing at a folder that
  does not exist and asserts it still does not exist afterwards; a golden pins the text.
- `.github/workflows/release.yml`: verify → build `sortlens` and `sortlens-cli` for macOS/Linux/Windows →
  compute checksums **in one place from the uploaded files** → attach them. **Never run** — there is no
  tag, and the file's own comment says so. Per-runner checksums were removed while writing it: a hash taken
  on the same runner that produced the file is how a release ends up shipping a checksum of something else.
- `docs/RELEASE.md`: the single-version rule, the artefacts, the five steps to cut a release (green ci,
  bump once, check the absent-by-design tables, tag, read the generated page), the checksum format, and
  what this version does not have — no models, no remote, no localisation, no HEIC/video, no PNG/TIFF/WebP.
- `--version` golden and the ROADMAP rows ticked; signing, installers, a11y, performance budgets and the
  final release notes stay open and are named as needing things this environment lacks.
- 423 tests; ci (exit 0), deny, snapshot green. **Not verified by hand**: the workflow itself — it
  has never had a tag to run on.

## 2026-11-21 — M10 (part 2): attribution that is checked, not asserted

- **the finding**: 25 of the 40 external crates in `[workspace.dependencies]` are not linked by any
  manifest — including `log`, `chrono`, `rayon`, and fifteen `craft-*` crates. The old document implied a
  dependency set that does not exist. Rather than delete the pins (they are promises about versions for
  work that is coming), the document now separates **Linked today** from **Declared, not linked yet**, and
  says plainly that nothing in the second table is compiled into the app.
- `cargo xtask attribution` (now in `ci`) checks the document against the manifests in both directions:
  a linked crate must be attributed; a declared-but-unlinked crate must appear in its own table; a table
  entry for a dependency that no longer exists is a problem; every family in `fonts/embedded.txt` must be
  attributed; and the "No model weights, corpora or datasets ship with this project" sentence must still be
  there. It handles both manifest spellings — `crate = { workspace = true }` and `crate.workspace = true` —
  which the first version did not, and it reported `thiserror` as unlinked for that reason.
- **proved to bite twice**: a fake `serde_yaml` in `[workspace.dependencies]` fails with its name, and
  deleting the `NanumGothic` row fails naming the family. Both restored.
- the document itself was rewritten from the facts: the three pinned git origins, per-crate licences from
  `cargo metadata`, craft-fonts as a **build input rather than a dependency**, the four embedded families
  (all OFL-1.1) and why a symbols face is required, the fixtures being generated rather than stored, and
  the model-weights statement.
- 423 tests; ci (exit 0), deny, snapshot green. **Not verified by hand**: nothing here is visual.

## 2026-11-21 — M10 (part 3): every painted control has a name, and a check that keeps it that way

- **the gap**: the grid, the detail preview, the lookahead strip, cull thumbnails, similar clusters, memory
  cards and sidebar rows are painted rather than built from widgets, so none of them had a name. A control
  that can be clicked and cannot be described does not exist for anyone using a screen reader.
- `a11y::label_widget` now registers a `WidgetInfo` (role, label, selected) for each of them, and
  `a11y::describe_photo` says the file name first, then stars, flag, missing state, colour label and
  keywords — the order that decides a cull. Sidebar rows say their label and count.
- `cargo xtask a11y` (in `ci`) walks `crates/ui/src/shell.rs` function by function and fails when a
  function makes something clickable without naming it. **Proved to bite** by deleting the `nav_row`
  registration, which failed naming that function, then restoring it.
- **what this deliberately does not claim**: the emitted accessibility tree is not asserted, because this
  egui version has no public way to read nodes back. `Context::has_accesskit_node` reports `true` even for a
  plain unnamed allocation (measured, not assumed) and is cleared at the end of the frame, so a test built
  on it would have been theatre. The names are tested as pure functions, the coverage by the source walk,
  and the missing half — focus order, traps, a real client — is a separate ROADMAP row naming VoiceOver,
  Orca and NVDA as what would unblock it.
- `epaint` deprecation caught by clippy: `WidgetType` is now `Role`; the rename is applied.
- 425 tests; ci (exit 0), deny, snapshot and `a11y` green. **Not verified by hand**: how any of it
  sounds or reads with an actual screen reader.

## 2026-11-21 — M10 (part 4): measurements, and the snapshot defect they found

- **`apps/sortlens-perf`** + `cargo xtask perf`: a release-mode developer binary that builds its own
  fixtures (1200×800 JPEGs with a gradient and deterministic noise, because a flat colour decodes for free
  and would produce a thumbnail rate for the easiest possible file) and prints one table — scan, index,
  library open, cold decode, warm cache hit, the two-worker service the grid actually uses, the suggestion
  pass with and without EXIF, and headless grid frames with the first frame reported separately. Outside
  `ci` on purpose: a timing that fails on a loaded box gets ignored.
- **`docs/PERF.md`** records two runs on Apple M4 / 16 GB / macOS 27.0.1: two thousand photos (scan
  191 µs/file, index 190 µs/photo, cold thumb 9.5 ms, warm 16 µs, service 200 thumbs/s, frame 0.28 ms
  median, first frame 4.3 ms) and ten thousand (scan 823 µs/file, index 1.1 ms/photo, frame 1.40 ms),
  plus a table of the budgets that are **not** measured here — 100k scroll p99, GPU frames, RSS, mobile,
  HEIC/RAW — each naming what would unblock it.
- **The defect the numbers exposed**: indexing ten thousand photos took 78 s, 7.8 ms each against 0.19 ms
  at two thousand. Two bugs in the snapshot policy: `commit` snapshotted every 256 ops and a snapshot
  rewrites *every* photo (quadratic import), and `snapshot()` never reset `unlogged`, so once a library
  passed 256 commits *every edit* rewrote the whole library. Fix: a sync is a batch taking one snapshot at
  its end, and the snapshot resets the counter; single edits keep the 256 cadence. Ten thousand photos now
  index in 11 s at 1.1 ms each — the same per-photo cost as two thousand, so the curve is linear (7×).
- Tests assert the work, not a duration: `a_large_sync_takes_one_snapshot_instead_of_one_every_256_ops`
  (one snapshot, nothing unlogged, log still replays on reopen) and
  `single_edits_still_snapshot_every_256_commits`. Two audit tests and one golden changed because a fresh
  sync now leaves a snapshot behind — the golden shows `snapshot at seq 7` where it used to say `none`,
  which is the behaviour change made visible rather than hidden.
- 427 tests; ci (exit 0), deny, snapshot, a11y green. **Not verified by hand**: nothing in this
  commit touches the GUI beyond a headless frame; the frame numbers are layout and shape building, not a
  display.

## 2026-11-21 — M10 (part 5): the icon that existed and nobody loaded, and the table that names the gaps

- **the finding**: `assets/icon.svg` had been in the repo since M0 and **nothing loaded it** — the window
  was created with no icon at all. An asset that looks finished in a file listing and is absent everywhere
  a user meets the app.
- `crates/icon` is now the one place the mark is defined, in code: `rgba()` renders it (4×4 supersampled,
  512 px, rounded tile, three stacked prints, red lens on the front), `svg()` renders the same geometry for
  `assets/icon.svg`, and `png()` writes an RGBA PNG through a hand-rolled stored-deflate writer plus CRC32
  and Adler32 — no dependency, and byte-identical on every machine, which is what a generated asset needs.
- `apps/sortlens/build.rs` renders the pixels into `OUT_DIR` and `main` sets `with_icon`, so no binary blob
  is committed and there is no second copy to disagree with the first.
- **Proved to bite**: the SVG is held to the drawing by
  `the_committed_svg_is_the_drawing`; appending a line to the file failed it, and `SORTLENS_UPDATE_ICON=1`
  rewrote it (that path is the documented way to change the mark). `the_window_icon_decodes_to_the_mark`
  decodes the PNG with `sortlens-thumbs` (a decoder that was not written for it) and checks the transparent
  corner, the front print's colour and the lens; `the_icon_is_square_and_rounded` checks the shape;
  `the_window_icon_is_the_generated_mark` in the app checks what the build script actually produced.
- **M10 closed honestly**: "Deliberately absent from M10" table added (signed/notarised macOS, MSI and
  AppImage/Flatpak, `.icns`/`.ico` and the brand pass, theme import, high contrast and reduced motion,
  `craft-update`, release notes and the 1.0 tag) — each with the reason and the unblocker, and the "State:"
  line rewritten to match. The rows those replace were moved into it rather than left as empty boxes.
- Files: `crates/icon/`, `apps/sortlens/build.rs`, `apps/sortlens/src/main.rs`, `assets/icon.svg`,
  `docs/ROADMAP.md`, `docs/ARCHITECTURE.md`, `docs/ATTRIBUTION.md`, `docs/RELEASE.md`.
- 432 tests; ci (exit 0), deny, snapshot green. **Not verified by hand**: how the icon looks in a
  real window on each platform (macOS dock, Windows taskbar); checked only as pixels and as a decoded PNG,
  plus a rendered preview of the SVG at 200/64/32 px.

## 2026-11-21 — M10 (part 6): the documents checked instead of trusted

- **the drift**: README's badge said `M0–M7 shipped` and its prose said "M0–M8 partially shipped … M9 and
  M10 are unstarted" while M10 had shipped; `docs/PLAN.md` §12 still listed M8, M9 and M10 as remaining
  work; `crates/memories` had never been added to the layer diagram; `apps/sortlens-perf` was unknown to it;
  `docs/PERF.md` existed and the README's document table never mentioned it.
- **`cargo xtask docs`** (in `ci`) now checks the facts that have one right answer: the shipped milestone
  range is **computed from `docs/ROADMAP.md`** (the highest milestone with a ticked row) and required in
  `README.md` twice — prose and the percent-encoded status badge, which is how it silently kept saying
  M0–M7 — and in `docs/PLAN.md` §12; every `crates/*` package name and every `apps/*` directory must be
  named in the layer diagram; every `docs/*.md` must be reachable from the README. **Proved to bite** three
  ways: an edited badge, a deleted `sortlens-memories` line, and each reported its own file.
- README rewritten where it was wrong: status, the document table (`docs/PERF.md` added), the build
  commands (`ci` now also runs version, attribution, a11y and docs; `perf` added), and "What is not built
  yet" — export and AI suggestions were listed as missing after they shipped. The comparison table's
  Updates row now says `craft-update` is declared and unlinked rather than claiming signed artifacts.
- PLAN §12 and §5.2 rewritten to the M0–M10 reality; `docs/FEATURES.md` gained a note that its `M` column
  is a plan, not a status; `docs/TOOLING.md` documents the new gate; ROADMAP's status log has the entry.
- Duplicated ROADMAP rows **deleted** as the rule requires: the five M10 rows that only said "moved to the
  table below", the duplicate `craft-update` row, and the accessibility part 2 row, which is now a row in
  the absent-by-design table (with why a headless test there would assert nothing).
- 432 tests; ci (exit 0), snapshot, deny green. **Not verified by hand**: nothing here is GUI work.

## 2026-11-21 — M1/M2 rows: real sources, and a grid that says why a cell is empty

- **Sources, part 1 (SL-A14)**: the sidebar now offers **Picks**, **Rated** and **Rejects** beside All Photos.
  Each row counts what it would show (Picks reading "0" while the library holds thousands is the row working),
  and each **replaces** the question instead of nudging it — picking Picks with three stars and a keyword
  active used to leave them on, so the row silently meant "picks among the ones you starred and tagged".
  The folder scope survives: that is where you are, not what you asked. The highlighted row is derived from
  the filter, so a filter chip asking the same question highlights the same row; a stored flag would let the
  two disagree. `Source` lives in `crates/ui/src/shell.rs` with `apply_source`/`source_selected`, tested as
  pure logic, plus a headless paint of the real sidebar over a two-photo library with counts that differ.
- **Placeholder + badges (SL-M10, SL-A12 part 1)**: a cell that has no pixels now says which kind of nothing
  it is — `?` for a file that is not on disk, `!` for one that could not be decoded, each with a hover that
  explains it, and a missing file outranking the decode failure it caused. `cell_badge` is pure and tested
  against all four combinations; the cell paints for a missing photo without requesting a decode.
- Rows split rather than ticked whole: **Recently Added** needs an `added` timestamp per photo (a schema
  migration, and the file's mtime would be "recently modified" — a different question), and cached-thumb
  behaviour for offline libraries needs staleness marking. Both say so in their rows.
- Files: `crates/ui/src/shell.rs`, `crates/ui/Cargo.toml` (dev-dependency on the fixture kit),
  `crates/ui/src/lib.rs`, `docs/ROADMAP.md`. 436 tests; ci (exit 0), snapshot green.
  **Not verified by hand**: the badge at small cell sizes, and whether "Rejects" is a source people want
  next to Picks — it is in the list because rejects are still in the library until you delete them.

## 2026-11-21 — M1 row: the header decides when the name is wrong

- **`sortlens_core::media::sniff`** reads the first 16 bytes and names the family: JPEG, PNG, TIFF (+ every
  raw built on it), HEIF/AVIF, WebP, and ISO base-media brands — including `crx ` for CR3 and the video
  brands. An **unrecognised** brand comes back `Unknown` rather than "video": guessing would drop podcasts
  and `.m4a` sidecars into the grid.
- **`resolve_kind`** corrects only a contradiction: a JPEG preview named `.cr2` is a still, an MP4 named
  `.jpg` is a video, a still named `.mov` is a still. Ambiguity keeps the extension's answer — every raw is
  also a TIFF, so a `.tif` stays an image instead of turning the whole scanned tree into raws.
- The scan counts what it corrected (`Skipped::mislabelled` → `ScanFinished::mislabelled`), the window's
  notice says "N routed by content, not name", and the CLI prints the same line: a correction nobody is
  told about is a silent rewrite of what their tree means.
- **Golden proof**: the fixture gained `Photos/Trip/preview.cr2` holding JPEG bytes, and `01 add photos` now
  reads "5 added … 1 routed by content, not name" — the correction and the report are pinned as published
  text. Fourteen goldens moved by exactly the counts one extra photo implies; the diff was read.
- Files: `crates/core/src/media.rs`, `crates/scan/src/lib.rs`, `crates/engine/src/lib.rs`,
  `crates/ui/src/shell.rs`, `apps/sortlens-cli/src/main.rs` (+ `report_scan`, after clippy's cognitive
  complexity gate caught `run()` at 31/30), `apps/sortlens-cli/tests/golden.rs` and the goldens.
- 440 tests; ci (exit 0), snapshot green. **Not verified by hand**: a real camera raw whose header is
  a TIFF with an embedded JPEG (the `cr2` case is synthetic JPEG bytes), and a real CR3.

## 2026-11-21 — M1 row: know the volume, so a missing drive is one answer

- **`sortlens_core::volume`**: `parse_mounts` reads a mount table (skips comments, resolves the four
  documented escapes `\040 \134 \011 \012` — as **octal**, which the first attempt got wrong and the
  escape test caught), `volume_of` picks the longest mount-point prefix so a nested volume or a bind mount
  behaves, and the platform convention is explicit: Linux uses the table, macOS the `/Volumes/<name>` rule
  (no `/proc`, and `statfs` would need `libc`), Windows the drive letter with a UNC path marked removable.
- **`Engine::root_volumes()`** answers per root, not per photo: the whole point is that an unplugged drive
  holding ten thousand photos is one missing volume. Each entry carries `mount`, `removable` and
  `mounted: Option<bool>`, where `None` means "this platform could not tell".
- **The sidebar** marks such a root "12 · away" with a hover naming the mount point, and treats `None` as
  present — a missing answer must never be rendered as a missing drive.
- Tests: seven on the parser/conventions (including a path that merely *starts* like a removable mount
  falling back to `/`, and `/Volumes` itself not being a volume), one on the engine, and one on the sidebar
  decision that pins `Some(false)`→away, `Some(true)`/`None`→quiet, plus a headless paint with a real root.
- **Split, not ticked whole**: `PathKey` moved to its own row — it needs Unicode normalisation tables and a
  per-volume case-sensitivity probe, and an ASCII lowercase "fix" would silently merge `Ångström.jpg` with
  `ångström.jpg` on a case-sensitive disk.
- Files: `crates/core/src/volume.rs` (new), `crates/core/src/lib.rs`, `crates/engine/src/lib.rs`,
  `crates/ui/src/shell.rs`, `docs/ROADMAP.md`. 449 tests; ci (exit 0), snapshot, deny green.
  **Not verified by hand**: a real unplugged drive on macOS (the "away" path is exercised only through the
  decision function here), and a Windows machine.

## 2026-11-21 — M1 row: density, and a settings field that finally does something

- **the finding**: `appearance.density` had existed in `config.toml` since M0 with a comment saying
  "comfortable or compact", and **nothing read it** — the grid hard-coded `min_cell = 132.0`. A persisted
  setting nothing consumes is the same lie as a button that looks enabled and does nothing.
- **`Density`** (`crates/ui/src/theme.rs`): Compact / Comfortable / Roomy, each naming a minimum cell
  (104/132/184), with `id()`, `label()`, `from_id()` (an unknown id reads as the default so a settings file
  from a later build cannot stop this one opening) and `next()` for the chip's cycle.
- **`grid_layout(available, gap, min_cell)`** is the whole of density, extracted and pure: it divides the
  window so the last cell in a row reaches the right edge, and floors a cell at 64 pt — below that the
  caption and stars stop being legible, so a narrow window gets one column that does not shrink rather than
  five that are unreadable.
- The chip sits beside the theme (both are "how the library looks") and the choice persists through
  `Action::SetDensity` → `appearance.density`, applied at startup from the store.
- Tests: layout (more columns when dense, exact fit at each step, the 64 pt floor and one-column minimum),
  the step cycle and id round trip, a settings round trip for all three ids **plus an unknown id falling
  back**, and a headless paint of the real grid at all three densities over a three-photo library.
- Files: `crates/ui/src/theme.rs`, `crates/ui/src/shell.rs`, `crates/ui/src/lib.rs`,
  `apps/sortlens/src/main.rs`, `apps/sortlens/src/settings.rs`, `docs/ROADMAP.md`. 454 tests; ci (exit 0),
  snapshot green. **Not verified by hand**: how the three steps feel on a real display — the numbers
  are a judgement call, and the only way to judge them is to scroll a shoot.

## 2026-11-21 — M1 row: a selection that can hold more than one photo

- **`Selection`** (`crates/ui/src/shell.rs`) keeps three things apart: the **primary** (what the inspector
  shows), the **ticks** (what a batch acts on) and the **anchor** (where a ⇧-click measures from). They cannot
  be one field — removing the primary has to move the inspector to a photo that is still ticked, and a
  ⇧-click measured from the primary would walk instead of extending.
- Gestures: **click** replaces, **⌘-click** toggles, **⇧-click** takes the run from the anchor *in the order
  the grid shows them* (a range in id order would select something nobody is looking at), **⌘A** selects
  everything in view (capped at `MAX_RANGE` = 20 000 so a click cannot stall on a huge library) and **Esc**
  clears. A ⇧-click with no anchor in view falls back to a plain click rather than selecting half the library.
- Batch keys: rating, flag and label now act on `selection.ids()` and the notice says the count — "Rated 4★
  on 12 photos" — instead of pretending one photo changed. Label toggle clears only when the primary already
  carries the colour, so pressing the same key twice over a mixed selection sets rather than clears.
- The selection is pruned against the library once per frame, so a delete or a rescan cannot leave the
  inspector showing a photo that no longer exists; `library_status` now speaks the selected count instead of
  a 0/1 flag.
- `keymap.rs` documents Click / ⌘-click / ⇧-click and Ctrl+A, so the shortcuts dialog is not silent about the
  three gestures it now has.
- Tests: the gesture state machine (add, remove, primary follows survivors, range forwards and backwards
  from a stable anchor, anchor-less range falls back), select-all + prune, a real engine batch write over
  three photos, and a headless paint with a three-photo selection that asks for nothing.
- 458 tests; ci (exit 0), snapshot green. **Not verified by hand**: the gestures with a real mouse —
  modifier-click in egui behaves, but only a person can say whether ⇧-click over a wrapped row feels right.
- **Split, not ticked whole**: marquee is its own row. It needs cell rectangles collected during paint and a
  drag that does not fight the scroll area's own, which is a piece of work rather than a footnote.

## 2026-11-21 — M1 row: marquee, without stealing the grid's clicks

- **the hazard**: a drag rectangle over a grid is easy to add badly — if the drag is allocated on top of the
  cells, clicking a photo stops working. So the drag only *starts* on the background, decided against the
  cell rects painted on the previous frame: the pointer may cross cells this frame that it had not reached
  when the press happened, and the press is the moment that decides which gesture you are in.
- `marquee()` paints the band, selects the cells it touches (a graze counts), notices the count only when the
  set actually changes — a notice every frame would spam the live region sixty times a second — and clears
  its own state on release without clearing the selection it made.
- `cells_in()` is the hit test as a pure function: intersects, sorted by id so a log reads the same twice.
  Tested on a 2×2 of cells, including the edge graze that the first version of the test got wrong (it grazed
  105, and cell two starts at 110 — the test was fixed, not the code).
- `Selection::set_many` is what a marquee and a future "select these" command share; an empty band clears,
  because dragging over nothing means nothing selected.
- Two headless tests: the pure hit test, and the gesture driven with real pointer events — press, move,
  release — asserting the band paints while the pointer is down and the selection survives the release.
  `keymap.rs` documents the drag so the shortcut sheet is not silent about it.
- 460 tests; ci (exit 0), snapshot green. **Not verified by hand**: how the band feels while dragging a
  real grid, and whether the 0.35-alpha fill reads on the light themes.

## 2026-11-21 — M1 row: the justified grid, and both layouts through one gesture handler

- **The pack** (`justified_rows`, pure): each row grows until the next photo would push it below the target
  height, then takes whichever of the two candidate heights is closer to the target — stopping at the first
  overshoot left rows 40 points taller than asked for whenever the library's average shape did not divide
  evenly, which the first version of the test caught as a failure of my *expectation*, and the second version
  caught as a real flaw in the packer. A short last row keeps the target height and sits left-aligned instead
  of stretching two photographs across the window; a photo wider than the window gets a row of its own; an
  empty library has no rows.
- **The plan** (`JustifiedPlan`) is cached on (width, filter, sort, count, target) and rebuilt only when one
  changes: the pack is one pass over every filtered aspect ratio, paged from the catalog in 2 048-photo
  chunks, and doing that per frame would make a big library a slideshow. Aspect ratios come from
  `aspect_of` — the function the compare view already used, now clamped so a panorama cannot produce a
  zero-height row — with 3:2 for photos whose dimensions never loaded.
- **Painting** windows by *row*, not by a fixed height: `show_viewport`, one `add_space` to jump past
  everything above the viewport instead of a loop over skipped rows, then only the rows that intersect it.
- **One gesture handler** (`cell_gestures`) for both layouts: click / ⌘-click / ⇧-click, double-click and the
  context menu are about photos, not pixels, and two grids that disagreed about ⌘-click would be one grid too
  many. `photo_cell_sized` gained `Fit::Cover` (square, crops) vs `Fit::Contain` (justified, already the
  photo's own shape, so cropping would discard the part of the frame the row height was computed from).
- **Density keeps its meaning**: in justified mode the 104/132/184 target becomes the row height, so
  "compact" still means more photographs at once. `G` and a chip toggle the layout; the choice persists as
  `appearance.grid`; the doc's claim that justified is the default is now true (`GridMode::default()`).
- `apps/sortlens-perf` now measures **both** layouts and labels them: one unlabelled `grid frame` line would
  have changed meaning the day the default flipped. Square 0.31 ms, justified 0.14 ms median at 2 000 photos —
  justified is cheaper per frame because the pack is cached, which `docs/PERF.md` now says out loud.
- Docs reconciled in the same commit: ROADMAP SL-B01/B02 ticked with what each layout actually does, and
  `docs/UX.md` §5.3 rewritten — it had been describing a density **slider** (90–480 px, `-`/`=` stepping)
  that was never built, while the shipped control is a three-step chip.
- 465 tests; ci (exit 0), snapshot, docs green. **Not verified by hand**: how the row heights feel while
  scrolling a real library, and whether `Contain` leaves visible letterboxing for any real camera ratio.

## 2026-11-21 — M2 row: priority and a bound on the decode queue

- **Priority is a value in the request**, not a queue per priority: `Urgency::{OnScreen, Nearby, Background}`
  served most urgent first and oldest first within a class. Workers now pull from one shared queue under a
  condvar instead of a channel — a channel hands out what it was given in the order it was given, so priority
  ordering was impossible without this change, and that is why the row had never been ticked.
- **Backpressure is a bound**: `QUEUE_CAPACITY` = 4 096 waiters. A request arriving at a full queue evicts the
  least urgent waiter only if it is more urgent itself; otherwise it is refused and counted. The victim is the
  *newest* of the least urgent class, because the oldest has waited longest and is closest to being decoded.
  An evicted key is released from the in-flight set, so a later attempt for the same thumbnail is not
  mistaken for a duplicate that is already on its way — the bug this design would otherwise have had.
- Callers: the grid asks at `OnScreen` (unchanged signature), the detail view prefetches the next photo in the
  sorted order at `Nearby`, and library-wide passes belong at `Background`. `Thumbnails::queue()` exposes
  waiters and refusals; `apps/sortlens-perf` prints the refusals so the bound is visible in a measurement
  rather than only in a doc.
- Tests: the queue on its own (urgency ordering across and within classes, eviction of the newest least
  urgent, refusal and its counter), the service at the bound (a refused request can be asked for again once
  the queue drains), and a headless 400-photo grid against an **eight**-slot queue that must refuse.
  **Proved to bite**: raising that test's bound to 4 096 made it fail with "63 requests waiting, over the
  eight slot bound", and lowering it back made it pass.
- `docs/ARCHITECTURE.md` §5 rewritten from the plan to what the code does, including why the channel went.
- 469 tests; ci (exit 0), snapshot green. **Not verified by hand**: whether the priority actually feels
  different while scrolling a real library — the ordering is unit-tested on the queue, but only a person can
  say whether a stale scroll still shows decoded cells sooner than it did.

## 2026-11-21 — M2 row: the bell, and notices that outlive their toast

- **The gap**: notices were a four-second toast and nothing else. Anything that happened while you were
  looking at another part of the window — a scan finishing, a move failing — was gone before you looked back,
  and a failure read exactly like a success.
- **`Notice { text, kind, at, read }`** in a bounded `VecDeque` (50, newest first) beside the toast, so the
  bell is a history of the same events rather than a second system that could disagree with the first.
  `NoticeKind::{Info, Problem}`; `notice()` is info, `notice_problem()` raises a problem.
- **The bell** appears only when it has something to say (a permanent "0" is chrome), shows a count of
  **unread** notices, and goes red when any unread one is a problem. Opening the popover marks them read;
  reading is not clearing, so "Clear" is separate. Rows carry an age said the way a person would say it.
- **The toast uses the same kind**, taken from the history entry raised with it, so a failure is red in both
  places and the two cannot drift.
- **32 call sites** that toasted a bare `error.to_string()` now raise a problem: a failure that scrolls past
  after four seconds has not been reported, and the badge should say so.
- Tests: the count and the red state, reading versus clearing, trimming past `NOTICE_HISTORY` without
  corrupting the count, the age table, and a headless paint of the bell (asking for nothing, reading nothing)
  and of the popover. `docs/UX.md`'s component table now describes the bell instead of promising a job list it
  never was.
- 473 tests; ci (exit 0), snapshot green. **Not verified by hand**: where the popover lands against a
  real top bar at a real window size, and whether "just now" is the right threshold rather than "now".

## 2026-11-21 — M2 row: the MCP server stops hand-rolling its protocol

- **Envelopes are `craft-jsonrpc`.** It validates the `jsonrpc` version and the id, tells requests from
  notifications, answers a batch as a batch, and refuses a message over a byte bound **before** parsing it —
  the bound protects the allocation, so it cannot be a check on the parsed result. Its `read_line` replaced our
  `input.lines()`, so an eight-megabyte line is refused without being allocated, the rest of that frame is
  discarded by a loop that never holds it, and the session survives rather than dying on one bad frame.
- **The session is `craft-mcp`.** Version negotiation (newest `2025-03-26`, and `2024-11-05` echoed back to a
  client that asks for it, which is what "supported versions" has to mean), the `initialize`/`initialized`
  handshake, the tools envelope and the text result. Our hand-written `initialize` — with a hard-coded
  `2024-11-05` and a session state that was really just "has anyone said hello" — is gone. Tool *routing* stays
  here, straight from the command registry.
- **Two behaviours changed, deliberately, because the protocol says so**: input that is not JSON is `-32700
  Parse error` rather than `-32600`, and `tools/call` with non-object arguments is `-32602` rather than an empty
  call. Both are named in the test that asserts them and in the ROADMAP row, not quietly changed.
- **`craft-ipc` is documented as absent, not ticked.** There is no second process: the window owns the engine
  and the write lock, and the CLI opens the library itself, which is what makes that lock meaningful. The
  ROADMAP row now says what would justify a socket (a running-window consumer: a menu-bar helper, a browser
  extension, two windows on one library) and what it would need first (a server mode, authentication on a
  multi-user machine, and an answer for "the app is not running").
- Tests: negotiation (an older version echoed, an unknown one answered with the newest), the handshake
  (`is_initialized` after the notification, and no reply to it), a batch answered as a batch with the
  notification skipped and an over-bound batch refused whole, an oversized frame refused with a message that
  names the bound while the *next* frame is answered, and non-object arguments refused. 16 tests in the crate.
- 479 tests; ci (exit 0), deny, snapshot, attribution green. **Not verified by hand**: against a real MCP
  client — the handshake is exercised in-process, but no client has ever connected to this server.

## 2026-11-21 — M2 row: HEIC, routed by header and measured from its container

- **The decoder is the suite's**: `craft-heif` (pure Rust `heic-rs`, every call inside `catch_unwind`, colour
  converted to match libheif instead of assuming BT.709). It decodes the primary image and prefers the ~320 ×
  240 thumbnail an iPhone already stores when that is big enough for the request, which is the difference
  between a grid filling and a grid waiting.
- **16-bit samples are divided by 257, not shifted by 8.** A shift maps everything in 65535..=65280 to 255 and
  throws away the top of a 10-bit HDR photo's range; 257 maps 65535 to 255 and 0 to 0 exactly.
- **Routing is by header** (`sniff` → `Magic::Heif`), so an iPhone photo renamed `.jpg` decodes, and a `.heic`
  that is really a JPEG goes to the JPEG decoder. Proved to bite: disabling the branch makes the renamed case
  fail with "not a JPEG stream".
- **Sizes come from the container** in the scan path (`ispe`/`irot`/`clap` via `craft_heif::probe`), because the
  container is what the decoder will use: a portrait iPhone photo is 64 × 96 in the justified grid and 1.4 MP
  in insights rather than 0 × 0 with a 3:2 guess. `heif_size` returns `None` for every non-HEIF file, and the
  container's answer wins outright rather than being merged per axis with a stale EXIF tag.
- **No binary fixture was committed**: the tests build synthetic HEIF files with the decoder's own writer — a
  grey ramp that must still be a ramp after the container parse, the HEVC stage, colour conversion and resize;
  a rotated file whose displayed size must swap axes; the same bytes under a `.jpg` name; and a truncated file
  refused with a reason rather than a panic or a blank cell.
- **A flaky test was fixed, not retried**: the priority test that expected the queue to be full assumed a worker
  had not already taken the first request. The admission decision (push, eviction, in-flight bookkeeping) is now
  a pure `admit()` and is tested without threads; the timing-dependent service test is gone.
- 482 tests; ci (exit 0), deny, docs, attribution green. **Not verified by hand**: a real iPhone HEIC —
  the decoder is exercised on synthetic files it was designed against, and performance is unmeasured (the perf
  fixtures are JPEG, and `docs/ROADMAP.md` says so).

## 2026-11-21 — M2 row: a video's length, rate and codec, from its container

- **Metadata first, pixels later, and said so**: the row is split. Part 1 (this commit) reads `moov` through
  `craft-media`'s isobmff demuxer during the scan — length from the video track's own timescale, frame rate as
  **sample count over duration** (29.97 reads 29.97, a variable-rate clip reads its average), size from the
  track or the sample entry, codec from `CodecId::name`. Part 2 (the poster) is a ROADMAP row with its blocker
  named: `craft-video`'s decoder **plus** a YUV→RGB conversion honouring `ColorTags`, which nothing in the suite
  provides because a wrong matrix is a colour bug that looks like a decode bug.
- `Photo` gained `duration_ms`, `fps`, `video_codec` (all `#[serde(default)]`, so an older log still replays),
  `PhotoMeta` carries them through `apply_meta`, and `format_duration` lives once in the catalog so the cell,
  the inspector and the screen reader cannot disagree: a duration pill on video cells, "1:12" in the spoken
  description after the cull facts, and a length row in the compare facts.
- **Boundaries it refuses to fudge**: a file that is not a container, or a container whose video track declares
  no length, answers `None` rather than a zero-length clip; MKV/AVI are not probed because their container
  features are off, and reporting them with no length would be worse than reporting nothing.
- **Fixture without a blob**: the test muxes a real MP4 with the suite's own writer — 24 samples of 25 ticks at
  600 Hz is one second at 24 fps — and asserts duration, size, codec and rate, then truncates the same bytes and
  asserts the probe refuses half a file, and points it at a JPEG and asserts it is not a video.
- Goldens changed by exactly the three new JSON keys plus the log byte counts; the diff was read before the
  update, not accepted blind.
- 484 tests; ci (exit 0), deny, docs, snapshot, attribution green. **Not verified by hand**: a real camera
  clip — the fixture is muxed by the same suite, so it exercises the demuxer, not a phone's peculiar `moov`.

## 2026-11-21 — M2 row: blur-up from a tier that already exists

- **The finding first**: a "blur tier" would not be cheaper. `craft-jpeg` reconstructs at full resolution and our
  `resize` reduces afterwards, so decoding a 32-pixel tier costs exactly what the 128-pixel grid tier costs —
  paying twice for the same work to show a softer picture. So the blur-up paints a tier that is **already
  resident**: the detail view, the compare panes and the cull stage ask `sharp_or_blur`, get the 128 grid tier
  scaled while the 1600–2000 decode runs, and it sharpens in place when that lands.
- `blur_tier` is pure: exact size, else the largest below, else the smallest above (painting a bigger picture
  into a smaller hole is sharp, merely wasteful), never another photo's tier, and **`None` when nothing is
  resident** — an empty rectangle is not a blur, so an undecoded cell keeps its honest placeholder rather than a
  smear that pretends to be the photograph.
- Tests: the tier table over every case; an end-to-end run where a real JPEG is decoded by the real service at
  128 and `sharp_or_blur(1600)` returns that texture and its tier instead of waiting; and a headless paint of the
  detail view before its decode lands (which caught a `mut` clippy would have failed on).
- One test lesson: the first version of the end-to-end test asserted "nothing resident yet" on the same
  thumbnail set it then waited on, making the later assertion a race against a worker. The empty-map case now
  uses its own instance, so nothing depends on timing.
- `docs/UX.md` said "skeleton shimmer"; the code has never shimmered, and now the row says what it actually does.
- 487 tests; ci (exit 0), deny, snapshot, docs green. **Not verified by hand**: whether a 128 tier scaled to
  a 1600-pixel pane reads as "soft photograph" or as "broken" on a real screen — that is a judgement only a
  person looking at it can make.

## 2026-11-22 — M3 row: apply the last keyword (SL-D05 part 2)

- `photo.tagFromPrev`: type a keyword once, then one key per photo. In the registry (`T`, Organise), routed
  through the engine, applied from the UI key handler.
- **What "last" means, decided rather than assumed**: the last keyword this session **added**, one of them,
  in memory only. Not persisted — a keyword applied last week is not "the last keyword", and silently pasting
  somebody else's older tags onto a photo is a wrong write rather than a convenience. Removing a keyword does
  not become "the last" one either (`engine.last_tag()` is set on adds only, and a test asserts the removal
  leaves it in place).
- It goes through `set_tag`, so a keyword renamed since still resolves to its current name, a photo that
  already carries it is not written twice (second press → 0 changes), and the whole batch is one undoable op.
- With nothing remembered the engine refuses with a sentence rather than a silent no-op, and the UI raises it
  as a *problem* notice; the success notice names the keyword, because "Tagged 5 photos" leaves you guessing
  what you just wrote.
- **Two gates bit, both correctly.** The published golden `24 shortcuts.txt` changed by exactly the one new
  binding (18 bound commands, `T  Organise  Apply Last Keyword`) — read before committing. And
  `no_ready_command_is_left_without_an_answer_about_its_arguments` failed for the new tool until its MCP
  argument schema existed, which is the gate doing its job; the example now states that there is no `tag`
  argument, which is the point of the command.
- Tests: engine (refusal with nothing remembered, add/remove ordering, batch apply over 3 photos, idempotent
  second press), UI (real library, real `Event::Key` for `T` through the real handler: nothing written and the
  reason in the notice, then the keyword landing on every selected photo with the notice naming it), MCP
  example. 489 tests; ci (exit 0), deny, snapshot, docs green.
- **Not verified by hand**: the notice's tone on screen, and whether `T` collides with any OS-level shortcut on
  a real keyboard layout (the keymap gate only proves the registry's internal uniqueness).

## 2026-11-22 — M2 row: stale cached thumbnails, and the wake-up nobody rang

- **The row**: a thumbnail decoded before the drive went away is still in the cache and was still drawn at full
  strength, so an offline library looked current. Missing photos' cached pixels are now painted at
  `CACHED_ALPHA = 110/255` — visibly present, visibly not live — in the grid cell, the detail view, the compare
  panes and the cull stage (a cached full-size preview of a missing file is still a cached preview). The `?`
  badge's hover now distinguishes the two offline states: "showing the cached thumbnail from before it went
  away" versus the plain missing-file text. Nothing is marked when nothing is cached — a grey rectangle is not
  stale, it is absent.
- **The bug this found is bigger than the row.** The M2 priority scheduler replaced a channel with a `Condvar`
  queue and never notified it. Workers parked as soon as the queue emptied, so every thumbnail request after the
  first quiet moment — which in the real app is most of them — waited for the life of the process. The tests all
  missed it because a worker that had not yet reached its first `wait` finds the entry by itself, so requesting
  immediately after startup worked; the first test that slept before requesting failed with
  `queued 1, dropped 0` and a four-second silence. `request_with` now calls `notify_one` on an accepted push,
  and `a_request_after_the_workers_have_parked_is_still_decoded` sleeps 250 ms before asking — **proved to
  bite** by removing the notify: it then fails, waiting out its ten-second deadline.
- Tests: the tint/hint table, the wake-up regression, and an end-to-end paint (decode a thumbnail from a real
  JPEG, delete the file, rescan, paint) asserting the image mesh's vertex alpha falls 255 → 110 while the
  default view — which hides missing files — paints nothing at all. 492 tests; ci (exit 0), deny,
  snapshot, docs green.
- **Not verified by hand**: whether 110 is the right fade on a real screen, and whether the real app's first
  paint after a long idle now recovers — the regression test proves the mechanism, not the app's timing.

## 2026-11-22 (second) — a refusal that named the wrong fact

- The previous commit's `ci` came back 1 on the same tree that then returned 0, so this run hunted the flake
  instead of starting new work. **Found it**: `applying_move_moves_the_file_and_a_changed_file_refuses_the_batch`
  restores the bytes after an edit but not the timestamp, so whether the batch was refused depended on whether
  the rewrite landed inside a new clock second. The test now restores the mtime with `File::set_times`.
- Chasing it found a real defect: the safety audit checks size **and** mtime, but both answered
  `Refusal::Changed`, which prints byte counts — so an edited file of the same length was refused with
  "changed since it was indexed (1 → 1 bytes)". A refusal that reads as a bug in the app is worse than no
  refusal. New `Refusal::Modified { path, indexed, on_disk }` with its own message, and
  `a_file_whose_timestamp_moved_is_refused_and_says_so` pins it and asserts the message never dresses a
  timestamp change up as a size change.
- 493 tests; ten consecutive runs of the previously flaky test with `--test-threads=8`, and three full-suite
  runs with `--test-threads=16`, all green; ci, deny, snapshot, docs green.
- **Not verified by hand**: nothing new; the fade from the previous commit is still un-eyeballed.

## 2026-11-22 (third) — M9 row: `--json` for the write verbs, with a written contract

- The CLI's JSON table is now a table of **schemas** — verb, engine command, read/write, and a one-sentence
  promise for the object — with thirteen mutating verbs added (`rate`, `flag`, `label`, `tag`, `untag`, `date`,
  `rename`, `alias`, `accept`, `pack`, `undo`, `retry`, `remap`). `--json schemas` prints the table, so a script
  discovers the contract instead of reading source.
- `docs/CLI.md` states the promise (additive only within a major version; absent ≠ null; counts are photos
  unless noted; a refusal is a non-zero exit with stderr, never an error object that looks like a result;
  progress verbs like `export` deliberately have no schema because a job with a cancel is not one object). The
  README links it, and `cargo xtask docs` enforces that reachability.
- **The contract found a lie, which is the reason to write one.** The new goldens showed `rate 1,2 4` answering
  `{"changed": 2}` for photos that already had four stars: `rate`, `flag`, `label` and `setCaptured` counted the
  *request*. They now compare against the catalog first, so a repeat answers `0`. `a_repeat_write_reports_no_change`
  pins it, and the goldens show `2` then `0`.
- Gate: `every_write_verb_has_a_json_schema_and_a_stated_contract` — every mutating verb must answer, every note
  must be a sentence, and the table must mark writes as writes. Proved to bite by renaming `pack` in the table.
- A self-inflicted mess worth recording: replacing a block of the golden step list by index silently swallowed
  four existing steps (`keywords as JSON`, the old `no schema says so`, `diagnostics`, `version`). The golden
  harness said so — "no step records this golden any more" — which is exactly why it tracks orphans; the steps
  were restored, and the four goldens are unchanged.
- 495 tests; ci (exit 0), deny, snapshot, docs green. **Not verified by hand**: nothing new on screen.

## 2026-11-22 (fourth) — M4 row: the decision overlay, in two phases, with an off switch

- **SL-C04 done**: the cull decision overlay is painted inside the image's own bounds (clipped to the image
  rect, not the canvas — a label outside the photograph reads as window chrome), and it now has the two phases
  the design asked for. `overlay_phase(elapsed_ms, motion)` is pure: the label fades in over 90 ms with the
  photograph still in place, then the photograph slides out over 140 ms while the next one is already behind it.
- The overlay keeps the **judged** photo's id, because `cull_decide` has already advanced the session — sliding
  out "the photo that just left" is only possible if you remember it. It clears itself when the slide ends and
  requests a repaint while running, so it cannot freeze at the phase whatever input happened to cause.
- `Motion::{Full, Reduced}` is a real persisted setting (`appearance.motion`, unknown value → full), a chip in
  the top bar next to density and layout (it is the setting you turn on *while* the animation is bothering
  you), and the phase function honours it: reduced draws the label at once, cuts the slide to zero, and still
  holds the label for its 90 ms because the confirmation is information rather than decoration.
- Docs corrected while here: `docs/UX.md` claimed reduced motion "respects OS setting + an in-app toggle" —
  only the toggle is true, and the OS half is now named in M10's deliberately-absent table with its unblocker
  (egui exposes no cross-platform reduce-motion signal). The same table's reduced-motion cell was rewritten
  rather than left saying the toggle does not exist.
- Tests: the phase table over every case (monotonic, both settings, overshoot-free), the settings round-trip
  with an unknown value and a settings file predating the key, and a headless cull view where a decision paints
  an overlay over the stage and then drops it. 498 tests; ci (exit 0), deny, snapshot, docs green.
- **Not verified by hand**: the timings. 90 ms and 140 ms are reasoned from "one glance" and "reads as
  movement", not measured against an eye, and the fade-plus-slide on a real display is exactly the kind of
  thing this harness cannot judge.

## 2026-11-22 (fifth) — M5 row part 1: ticking collections, and batch changes that undo as one

- The sidebar's collection list can now be worked on in bulk: click ticks, ⌘-click toggles, ⇧-click takes the
  run in **sidebar order**, a ticked row shows a dot distinct from the filter highlight, and a line appears with
  the count and ✕ when more than one is ticked. The context menu acts on the whole ticked set when the row is
  part of it and on that row alone otherwise — right-clicking an unrelated row must not recolour four others.
  Rename and delete stay single in the multi case (renaming four rows to one name merges them); a batch delete
  is offered explicitly with the count.
- Ticking lives in its own `CollectionSelection` (anchor + ticks, no primary) rather than reusing the photo
  `Selection`: the ids are different kinds of thing, and a set that could be passed where photos were expected
  is a bug waiting for a refactor. It is pruned each frame against the engine, so a delete — or an undo —
  cannot leave a ghost ticked.
- **The engine grew batch methods rather than a loop in the UI.** `set_collections_colour`,
  `set_collections_favourite` and `delete_collections` each commit one `Op::Group`, so five recoloured
  collections undo in one ⌘Z instead of five; they count only what changed, like every other write; and the
  registry commands now accept `collections: [1,2]` as well as a single `id`.
- Tests: the gesture table (replace/toggle/range/widen/narrow/backwards/no anchor/prune/clear), the engine
  batch (one undo restores all three; a *second* undo reaches the underlying collection creation, which is how
  we know the group went in as one entry, and a batch delete leaves the photos alone), and a headless sidebar
  paint with two ticked.
- The ROADMAP row is split honestly: this is part 1; **folders** having favourites/colours/search/sort is
  part 2 and is now its own row with its reason (the folder section is a projection of roots and subfolders, so
  each of the four needs somewhere to live) instead of an empty box.
- 501 tests; ci (exit 0), deny, snapshot, docs green. **Not verified by hand**: how the tick dot reads
  next to the colour dot at real size.

## 2026-11-22 (sixth) — SL-A11 part 1: `PathKey`, and folders that remember a pin and a colour

- **`PathKey` exists**, and the row that blocked it was blocking on a dependency decision that had already been
  made elsewhere in the tree: `unicode-normalization` was a transitive dependency, so the tables were present.
  It is now a declared, attributed direct dependency and `crates/core/src/pathkey.rs` normalises NFC/NFD and
  trailing separators unconditionally, and resolves **case by asking the disk**: per component, a
  case-insensitive lookup in the parent, adopting the on-disk spelling only when exactly one entry matches. Two
  matches means the volume can hold both, so nothing is folded and `Ångström.jpg`/`ångström.jpg` stay distinct —
  the silent merge the row was written to prevent. No per-volume probe: the disk answers directly.
- Tests hold the key to whatever volume the test runs on (`case_is_resolved_by_the_disk_and_never_folded` asks
  the volume whether it distinguishes case, then asserts *that* answer), rather than to a guess about the
  platform. NFD/NFC collapse, separators collapse, every component is resolved, absent paths keep what was typed.
- **Folders in the sidebar now carry a mark**: star + colour dot, set from the row's context menu, stored as
  `Op::SetFolderMark` which records the *whole* mark so one ⌘Z clears pin and colour together, and an empty mark
  removes the entry both when applied and when replayed. New `Op`, `crates/catalog/src/folder.rs`, catalog
  accessors, engine methods (`set_folder_mark` computes the key itself so a UI and a CLI cannot make two marks
  for one folder), registry commands `folder.mark`/`folder.marks`, MCP argument schemas, and two `--json` schemas.
- `left_panel`'s closure crossed the cognitive-complexity gate, so the folder list became `folders_section` —
  splitting because the gate said so, not silencing it.
- Attribution gate bit as designed: adding `unicode-normalization` failed with "linked but docs/ATTRIBUTION.md
  does not attribute it" before the row was added.
- 508 tests; ci (exit 0), deny, snapshot, docs green. **Not verified by hand**: how the star plus the
  colour dot read together on a real sidebar row, and whether resolving case per component is fast enough on a
  network volume — it is one `read_dir` per component, per mark change, not per frame.

## 2026-11-22 (seventh) — folders part 3: a folder remembers how you were browsing it

- The folder mark gained `sort` and `search`, which finishes the M5 folders row. **The decision the last run
  deferred**: a sort belongs to the folder when a folder is the scope, and to the library otherwise. A folder
  with nothing remembered follows the library's sort rather than snapping back to Natural — losing the sort you
  chose a moment ago is the more annoying failure, and Natural is not a sensible default for a library of RAWs.
- Search follows the collections rule exactly (committed on lost focus or Enter, never per keystroke — that
  comment was already there) and comes back into the search box on open rather than filtering behind your back.
- `folder_scope(mark, library_sort) -> (Sort, String)` is the rule as a pure function and the row's click runs
  exactly it, so the test asserts the code path rather than restating the values. Its first version *did*
  restate them; the extraction is what made the test worth having.
- `FolderMark` is no longer `Copy` (two `String`s), which the compiler then chased through the catalog, engine
  and UI: every `.copied()` became `.cloned()` and every partial update became `..mark.clone()`. Worth naming
  because it is the kind of change that silently loses a field if one of the struct literals had been rewritten
  by hand — `..mark.clone()` is what makes adding the fifth field safe next time.
- `folder.mark` gained `sort`/`search` (omitted keeps, null clears, unknown sort id refused), `folder.marks`
  reports both, and the CLI's `mark` verb passes `--sort`/`--search` through.
- 509 tests; ci (exit 0), deny, snapshot, docs green. **Not verified by hand**: whether restoring the
  search into the box on folder open reads as helpful or intrusive — it is the kind of judgement only use makes.

## 2026-11-22 (eighth) — `sortlens-quality` part 1: a sharpness score that can be trusted and re-derived

- `crates/quality` stopped being a scaffold. The score is the variance of the 4-neighbour Laplacian of the luma
  channel over the squared mean luma, measured on a box-filtered grid capped at 256 px on its long edge. Each
  clause is a decision with a reason in the crate docs: Laplacian not edge energy (a steep ramp is not detail),
  variance not a mean (one sharp edge must not carry a flat frame), divided by mean² (exposure invariance), at a
  fixed analysis edge (a photograph is as sharp as the photograph, not as the copy you hold).
- **`Score { value, version }` with `SCORE_VERSION` and one `needs_rescore` predicate.** A library half-measured
  by two formulas would sort wrongly with nothing looking wrong, so the version is checked rather than trusted,
  and a non-finite value counts as needing a re-measure.
- **A near-black frame is refused, not scored — and the proof is the finding.** Removing `MIN_MEAN_LUMA` makes a
  1–2 level noise frame score **1.78, above a sharp bar pattern's 1.41**: without the floor every night shot
  tops the sharpness list. "Cannot tell" is not "in focus".
- **Layer discipline caught by the docs, not by a gate.** The first version depended on `sortlens-thumbs` to take
  a `Thumbnail`; ARCHITECTURE says `quality` is pure math at L0 with no IO, and `xtask layers` passed anyway.
  The API now takes RGBA (or luma) planes and `thumbs` is a dev-dependency, so the runtime rule holds and the
  end-to-end test still measures real decoded pixels.
- **A stated limit, because the alternative is an overclaim:** the fixed edge makes copies related by a whole
  number ratio measure alike (proved, ratio < 1.05); a fractional reduction is averaged differently by the box
  filter and can shift the score. Correcting that means measuring at several edges — precision a ranking does not
  need. The first version of that test asserted 1024 vs 480 (a fractional ratio) and failed at 1.66×, which is
  how the limit was found.
- Twelve tests: blur separation and ordering, a flat frame scoring exactly zero, exposure invariance (twice — one
  hand-built, one fixed-seed property over 24 random mid-tone frames), size invariance plus the mechanism test
  that native-size measurement *does* differ, the darkness floor on both sides, degenerate buffers, versioning,
  JSON round-trip and a real JPEG decode. Two invariants were proved to bite by removing the normalisation and the
  floor, then restored.
- No property-testing dependency exists in the workspace and adding one for two invariants would cost a package
  and its attribution, so the property runs use a fixed-seed generator — reproducible on failure, which is the
  part a property test is actually for.
- 521 tests; ci (exit 0), deny, snapshot, docs, layers, roots, attribution green.
- **Not verified by hand**: none of it is reachable from the UI yet — the scan job is part 2, so there is no
  screen to look at and no threshold to judge. The threshold deliberately does not live in this crate.

## 2026-11-22 (ninth) — blur scan part 2: scores stored, as one undoable pass

- `Op::Scores { changes }` carries a whole measurement batch, so a 5 000-photo pass is **one log entry and one ⌘Z**,
  and its inverse is the previous scores — a re-measure at a new `SCORE_VERSION` undoes back to the old numbers.
- **The score lives on the `Photo` record, and the test found why that matters.** A scan that replaces a record
  because the bytes changed used to clone the previous one and keep every annotation; with `sharpness` on the record
  that meant a changed file kept a score about pixels that were gone. Stars survive a re-encode; a sharpness score
  does not, because it is a statement about pixels, so the update path now clears it. The first version of the test
  asserted exactly this and failed, which is the point of writing it before believing the design.
- `Engine::begin_sharpness` queues only what `sortlens_quality::needs_rescore` says needs it (never measured, another
  version, or not a number), decodes at `LARGE` on a worker, and `poll_sharpness` reports
  `{ measured, unjudged, remaining }`. **`unjudged` is first-class**: an undecodable file and a near-black frame are
  counted and left unscored, never stored as a number. `at_least_one_measurement_survived_the_undo_above` and
  `a_file_that_cannot_be_decoded_is_counted_rather_than_scored` pin the two halves of that.
- A cancelled pass still commits what it measured, because each score is true on its own — unlike a cancelled scan,
  which commits nothing because a half-scanned folder describes a disk that is not there. Recorded in the struct doc.
- `photo.sharpness` / `photo.sharpnessProgress` in the registry, MCP examples, and a CLI `sharpen` pass (goldens 50/51:
  six scores stored, then "every photo has a current score").
- **`sharpen` deliberately has no `--json` schema, like `export`, and the gate now proves it refuses.** The first
  version *did* have a schema, and the golden exposed the bug: `--json` routes to a single `engine.run` call, so it
  answered `{"queued":6}` and stored nothing. A pass has no single object to return, so it is a named progress verb,
  and `every_write_verb_has_a_json_schema_and_a_stated_contract` now requires each named progress verb to refuse.
- The complexity gate fired on `Catalog::apply` at 31/30 and the arm became `apply_scores` — split because the gate
  said so, not silenced.
- 529 tests; ci (exit 0), deny, snapshot, docs, layers, roots, attribution green.
- **Not verified by hand, and not reachable from the window yet**: the UI does not start the pass or show a score.
  That is part 3, and there is nothing to look at until it exists.

## 2026-11-22 (tenth) — blur scan part 3: the review surface, and a cut that says what it can claim

- **M6's blur row is now closed.** The section lives in Insights — the analysis view that already has the charts —
  with a "Measure sharpness" button (shown with how many photos have no current score), a histogram, a slider for
  how much of the library to review, and a button that ticks the selection and returns to the library.
- `crates/ui/src/sharpness.rs` is the arithmetic with **no egui at all**: `review(scores, percentile) -> Review`
  returns the histogram buckets, the cut, the ids at or below it, the tie count and `selected_bands`. Deciding which
  photos get acted on belongs in a test, not only in a painter.
- **The cut is a value at a percentile, not a rank.** Taking "the lowest N" would pick an arbitrary subset of a tie —
  the same photograph selected or not depending on its id. A value cut can select more than the percentage asked
  for, so `ties` is returned and the count is shown: a bigger selection honestly described beats a smaller one
  chosen at random.
- **Identical scores select nothing** (`separable: false`): a relative cut on a flat library would select every
  photo, and selecting everything is an action, not a review. The label says a relative cut cannot separate them.
- **The band the cut sits in is computed once.** The first version of the painting used a helper that returned
  `true` — a placeholder that would have highlighted the whole histogram. It was deleted and replaced by
  `Review::selected_bands`, with a test that `band_selected(n - 1)` is true and `band_selected(n)` is false, so the
  painter and the button cannot disagree about where the cut is.
- The pass is polled in the view that asked for it (`insights_view`), and the report becomes a notice that names how
  many could not be judged — an unmeasured frame never reads as a measured one.
- `Selection::select_many` caps at `MAX_RANGE`, the same cap a range-select has: a computed selection of a hundred
  thousand ids is a batch waiting to run on a library nobody meant to change.
- The slider is session state, not a persisted setting: remembering yesterday's cut would make today's review appear
  to make a claim it never made. Written into `docs/UX.md` §5.8, which previously described a sortable grid that
  does not exist.
- 538 tests; ci (exit 0), deny, snapshot, a11y, docs, layers green. ROADMAP's stale "blur ladder and cluster
  determinism wait for their features" row was split: the blur ladder is checked, cluster determinism waits for geo.
- **Not verified by hand, and named as such in ROADMAP**: whether 10% is a sensible default, and whether the
  histogram is legible at real score ranges. Both need a library of actual photographs, not fixtures.

## 2026-11-22 (eleventh) — similar leftovers: a selection that is what it says, and one delete executor

- **A real bug in shipped code**: "Select all but the best" cleared the selection, selected **one** id, and announced
  "{n} set aside". A button that names a set and delivers a different one is exactly the quiet lie a batch delete is
  dangerous with. It now selects `group.others()` through `Selection::select_many`, and the group header names the
  keeper so the set is chosen with it in view.
- A batch bar: "Select leftovers in every group (N)" and "Delete the N selected…", the second opening a confirmation
  with the count and the three modes.
- **The row needed a delete path that did not exist outside a cull session, and the answer was a refactor, not a
  second path.** `Engine::apply_candidates` now holds the audit, apply, batch record, flags and job; `apply_session`
  keeps only the session bookkeeping; `delete_photos(ids, mode)` builds a candidate set and calls the same function.
  The alternative — a delete that quietly skipped the audit — is the failure this codebase exists to avoid.
- `photo.delete` in the registry, `delete_mode_from` shared with `cull.apply`, CLI `delete <ids> --mode … --target F`
  with a `--json` schema, MCP example. The golden uses `--mode tag` so a test never touches the real Trash.
- **A test-only constraint worth naming**: `Mode::Trash` is not exercised anywhere, because it hands the file to the
  operating system's trash and a test that does that damages the machine it runs on. My first attempt did exactly
  that and took **240 seconds** before failing (the macOS trash path under a temp volume). The batch/audit/undo
  machinery is covered through `Mode::Move`, and the gap is written into ROADMAP rather than papered over.
- The audit also refused my first test for a *second* reason worth remembering: a rejects folder inside a watched
  root is refused outright, because rejects there would reappear on the next scan. The test now puts it outside.
- 542 tests; ci (exit 0), deny, snapshot, a11y, docs, layers, attribution green.

## 2026-11-22 (twelfth) — `sortlens-geo` part 1: projection, deterministic clusters, an offline gazetteer

- `crates/geo` stopped being a scaffold and holds three things, all pure: `LatLon` (the only way to hold a coordinate,
  refusing non-finite numbers, latitudes off the globe and longitudes outside ±180), Web Mercator `project`/
  `unproject`, a 64-pixel grid `cluster`, and a `Gazetteer` parsed from a tab-separated file the user supplies.
- **Mercator has no pole, so a polar photograph is refused, not clamped.** Clamping would draw it at the top edge as
  if it were taken at 85°N — a wrong claim about a real photograph rather than a missing dot. `MERCATOR_LIMIT` is the
  tile scheme's own latitude, and every returned point is inside the world square so callers never check.
- **Clustering determinism is now a fact rather than an aspiration**: cells are sorted by count then cell, and a test
  feeds the same forty photos in two orders and asserts the same dots in the same order. That closes the ROADMAP row
  which had been waiting for the feature.
- The gazetteer ships **no data and makes no network call**: it parses `name<TAB>lat<TAB>lon[<TAB>country]`, counts
  malformed lines instead of dropping them silently, indexes by one-degree cell, and breaks ties by name so file order
  cannot decide the answer.
- Two test bugs of my own, both worth the note: a coordinate at 89°N is *valid* and *unmappable* (the test asserted the
  wrong one), and a "near Brighton" lookup 54 km away with a 30 km radius correctly returned nothing — the test was
  wrong, not the code.
- Found while checking: **`sortlens-mcp`'s example harness broke on my new `photo.delete` example**, which really
  deleted the photos the later examples needed (and after 112 s, because `mode: trash` goes through the OS trash).
  The example is now `mode: "tag"`, moves nothing, and says in a comment why — the same hazard as the golden step.
- **The blocking dependency for part 2 is named, not discovered later**: `sortlens-meta` parses no GPS at all and
  `Photo` has no coordinate, so there is nothing to cluster until EXIF GPS is read and stored. The next run is
  therefore data first — parse GPS with its hemisphere reference, put `lat`/`lon` on the record, prove it survives a
  snapshot and a rescan — and only then paint.
- 553 tests; ci (exit 0), deny, snapshot, docs, layers green.

## 2026-11-22 (thirteenth) — Places part 2: GPS in the catalog, and a view that admits what it lacks

- `sortlens-meta` reads the camera's GPS into a `sortlens_geo::LatLon`; `Photo` carries it, so a scan fills it in and a
  re-read replaces it (a successive read with no GPS clears it rather than keeping a stale dot). End-to-end test uses
  a JPEG whose EXIF APP1 block is written by the same library that reads it — JPEG → EXIF → DMS → coordinate, with the
  southern hemisphere's sign intact. `Testkit::jpeg_with_location` makes that fixture available everywhere.
- **The decision worth keeping**: a device with no fix writes 0, 0, and 0/0 is a real coordinate in the Gulf of
  Guinea. Treating it as real would put every pre-lock frame in one dot in the Atlantic — a data disaster rather than
  a missing tag — so an exact 0, 0 is "no location". Cost of being wrong: a photo genuinely taken at Null Island is
  left off the map. Cost of the other choice: the map lies about hundreds. Stated in `location_of`'s doc.
- `LatLon` now deserialises **through its constructor** (`#[serde(try_from)]`), so a corrupt snapshot is a load error
  instead of a photograph drawn in the wrong ocean.
- `Engine::geotagged`/`located`/`places(zoom)` and a Places view that reports counts and the biggest clusters, and
  says in the view that the coastline is next. **No dots on a blank rectangle**: a field of dots with no coastline
  looks like a map and tells you nothing about where anything is.
- **Two stale nav gates deleted**: `Cull` still said "M4" and `Places` "M5" in the bottom bar, so the nav closed
  working views behind an out-of-date label. `View::milestone` now returns `None` for everything, `planned_view` is
  gone with its last caller, and a partly built view says so inside itself where the person looking will read it.
- A grid-clustering limitation is now a decision in the record, not a surprise: two photos a few hundred metres apart
  that straddle a cell edge are two dots. My first engine test asserted one dot and failed — the code was right.
- 560 tests; ci (exit 0), deny, snapshot, a11y, docs, layers, attribution green.

## 2026-11-22 (fourteenth) — Places part 3a: the basemap decision, and a credit that cannot be left out

- **Decision, made rather than deferred again**: the basemap is a **folder of tiles the person supplies** —
  `{z}/{x}/{y}.png` beside the catalog, with `attribution.txt` in the folder. Not a bundled vector basemap (map data the
  project does not have, and a licence it cannot clean-room), not MBTiles (a SQLite reader in the tree for one
  format). The layout is exactly what unzipping any tile archive gives, so the tiles somebody may legally use are the
  tiles that work.
- `sortlens_geo::tiles::Tileset::open` **refuses a folder with no credit**, naming the file to write and saying why:
  an unlabelled map implies the application made it. The refusal is at the type level — no state exists in which the
  view holds a usable tileset whose attribution is unknown — which is stronger than a comment asking the view to
  remember. A folder with a credit and no tiles is refused too.
- `tile_path` checks presence once per request (`None` for a hole) so a partial set paints around its gaps instead of
  failing an open every repaint. `zoom_range`/`has_zoom`/`zooms` read the directory names; `tile_count` answers "is
  this set partial".
- **The zoom bound came from a failing test**: I asserted a directory named `255` is not a zoom level and was wrong —
  `255` parses as a `u8`. `MAX_TILE_ZOOM = 30` now refuses it, because a view offering zoom 255 offers a billion
  billion tiles that are not there. The test stayed as written; the code moved.
- `Engine::tileset()`/`tiles_dir()` — the folder is **part of the library**, not a global setting, so two libraries
  can have different basemaps and moving a library brings its map. The Places view now reports the range, the tile
  count and the credit, or the sentence for whichever state the library is in, or the refusal.
- 564 tests; ci, deny, snapshot, a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: any real tile set, and whether the "no basemap" copy reads well at the view's width. The
  painted words are **not** asserted — egui returns paint shapes, not laid-out text — so the smoke test proves the
  view paints in all three states and that the engine reports the right state; the credit guarantee is enforced by
  `Tileset::open`, one layer down, where it can actually bite.

## 2026-11-22 (fifteenth) — Places part 3b: the tiles under the dots, and a dot that selects

- `sortlens_geo`: `viewport_origin`, `screen_position`, `visible_tiles`, `tile_rect` — the viewport maths as pure
  functions, so the UI holds no projection of its own. **The world does not wrap**: a repeated antimeridian would draw
  the same photographs twice with nothing to say which was which, so columns clamp to the world square.
- `crates/ui/src/places.rs`: `dot_radius` (square root, so area tracks the count), `dot_at` (**largest** dot within
  reach wins — where a big dot overhangs a small one the person is aiming at the big one), `clamp_zoom` (past the top
  of a set the tiles stop and the dots keep going), and `TileCache` (decode once; a failed tile is remembered as failed
  so one corrupt file is not re-decoded every frame; bounded at 512).
- **Two real bugs, both caught by tests written to expect correct behaviour:**
  1. The view skipped clusters it could not place and then indexed the cluster list **by position**, so one off-screen
     cluster shifted every dot onto its neighbour's photographs. `Dot` now carries its cluster index.
  2. The hit test compared the pointer in screen coordinates against dot positions **relative to the map's corner** —
     every click missed by the panel margin. `clicking_a_dot_selects_its_photos` drives real pointer events through
     `run_ui`, finds the map by the rectangle the view fills, presses and releases, and asserts the members are
     selected; without finding them it would have looked like it worked.
- The map centres on the **mean of the library's own photos**. Clicking a dot selects its members and says where they
  are; clicking the sea selects nothing and clears nothing. The credit is printed over the map every frame, and the map
  is a named `Role::Canvas` region — `cargo xtask a11y` demanded that, which is the gate earning its keep.
- `docs/UX.md` §5.9 rewritten: it described an offline *vector* basemap, thumbnail fans and a `geoBounds` library
  filter, none of which exist. The absent ones are now named with reasons instead of being described as if built.
- 572 tests; ci, deny, snapshot, a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: how it looks. The tests prove the tiles decode once, dots carry the right clusters, the
  click resolves and the credit is the folder's own; they cannot say the coastline sits in the right place, because
  there is no tile set in the repository to compare against. That needs a real archive and eyes.

## 2026-11-22 (sixteenth) — Places part 4: dots borrow a name from a gazetteer the person supplies

- `Gazetteer::load` reads a file with the size checked **before** the bytes (`MAX_GAZETTEER_BYTES` = 64 MB, enough for
  a four-million-place world list, small enough that pointing the app at a disk image is refused); `GazetteerError`
  carries a sentence naming the file. **A file with no places is not an error** — `is_empty`/`skipped` report it, and
  a line of tabs is blank rather than malformed. My test asserted two skipped lines where the code counted one; the
  code was right and the test changed.
- `Engine::places_file`/`load_gazetteer` hand back the path and a loader and deliberately do **not** cache: the thing
  that must never happen is a view parsing tens of megabytes to draw a frame.
- `places::Names` is the cache — size plus modification time as the stamp, re-read only when it changes. **A deleted
  file stops naming anything** (a place list outliving its file puts names on a map whose authority was deleted) and
  **a missing file is not a fault** (most libraries have none; a red line for that teaches people to ignore red lines).
  Found while wiring the footer: without that distinction the view reported "there is no gazetteer at …" in red for a
  library that simply had no places file.
- `LABEL_KM` = 30, `place_label` returns `None` rather than guessing, and `selection_notice` is one function with two
  forms so the named and unnamed cases cannot drift: "Selected 42 photo(s) near Brighton (50.82N 0.14W)" and
  "Selected 1 photo(s) at 25.00N 40.00W". Both state the coordinate, so a name is never mistaken for the file's
  resolution.
- A named dot prints its name beneath it; the footer says whether names are in use or why they are not.
- Deduplicated ROADMAP: the `sortlens-geo` part-1 block appeared twice, and the second copy was followed by a stale
  "part 2: the map view" bullet claiming GPS was not parsed — it has been since `f8f6a34`. Both removed.
- 576 tests; ci, deny, snapshot, a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: how a dense gazetteer looks — overlapping labels are the file's density problem and there
  is no declutter pass (named as absent in UX.md) — and how a real GeoNames export performs on load.

## 2026-11-22 (seventeenth) — Registry delete-everywhere: `tag.delete`, wired end to end

- `sortlens-catalog`: `delete_tag` — the keyword comes off every photo that carries it, and the whole
  rename trail dies with it (aliases whose chain resolves to the keyword are forgotten in the same
  undoable `Group`), so re-typing the old name cannot resurrect what was deleted. Naming an alias
  deletes the keyword it stands for, the way `set_alias` resolves names. Deleting a keyword nobody
  carries and no alias points at is **refused**, not silently "deleted" — a typo should not report
  success. Two catalog tests: the batch+one-undo case and the alias-chain case.
- `sortlens-engine`: `Engine::delete_tag` syncs the sidecars of every touched photo (so a deletion is
  not resurrected by the next file read) and records a `#tag deleted everywhere` job; `tag.delete` is
  in the command registry and the MCP examples, normalised like every other tag verb.
- `sortlens-ui`: sidebar keyword rows offer "Delete everywhere"; the handler (`delete_keyword_everywhere`,
  driven by the test, not a copy of it) prunes a keyword filter pointing at the deleted keyword and
  names the keyword and the count in the notice. Headless `run_ui` test paints the sidebar over a
  scanned library, deletes through the real handler, asserts the vocabulary is empty and one undo
  restores it.
- Collection delete already had its surface (`collection.delete` context menu, batch variant, tests) —
  verified, not rebuilt.
- **Windows environment fixes, needed to make the tree buildable at all here:** the project toolchain
  was missing, so 1.95.0 was installed into `.toolchain` and `scripts/env.sh` made platform-aware
  (Homebrew path only where it exists, project cargo bin first). Two checkout bugs that break the
  CI on Windows only: `core.autocrlf` checked the byte-compared goldens and `assets/icon.svg` out
  with CRLF — pinned LF for both in `.gitattributes`. And a real bug the macOS runs could not see:
  `PathKey` split paths on `/` only, so every `Path::join`-built Windows path (`\`) was resolved as
  one whole component and never case-resolved by the disk. It now splits on the platform separator
  (`std::path::is_separator`) and spells every key with `/`; a new test holds a joined path and a
  hand-spelled one to the same key.
- 580 tests; ci, snapshot, a11y, docs, layers, roots, attribution, deny green.
- **Not verified by hand**: the menu item on screen (a right-click menu cannot be driven headlessly
  here; the handler itself is tested) and the look of the hover text.

## 2026-11-22 (eighteenth) — Twenty steps of undo and redo, kept across a restart

- `sortlens-catalog`: the undo stack is bounded at `UNDO_DEPTH` = 20 and a `redo` stack walks the
  undone steps forward. A new commit abandons the redo branch — stepping forward again would
  replay a past that no longer matches what was just written. `redo()` is a real operation: it
  applies, logs with its inverse, and puts the inverse back on the undo stack, so a redo is
  itself undoable.
- **The history survives a restart, not just the last step**: the snapshot now carries
  `undo_history` (oldest first) next to the old single-level `undo` field, which is still written
  so an older build reads the snapshot it knows; a snapshot with only `undo` opens with that one
  step (a test rewinds the file to prove it). Log records replaying after the snapshot append
  their inverses to the stack, so a reopen keeps twenty steps, not one. Redo as a stack is
  per-session; after a restart the same steps stay reachable by undoing the undo entries the log
  holds in order — the "never a one-way door" property, now across the whole depth.
- `sortlens-engine`: `Engine::redo`, `edit.redo` in the registry and dispatch, and an MCP example.
  The example is published *before* `edit.undo` on purpose: the published examples run in order
  against one library, and the undo below it clears a decision the later examples need cleared —
  redoing it back handed `library.restoreBatch` a photo that was already rejected, and
  `every_published_example_actually_runs` caught it.
- `sortlens-ui`: `Ctrl+Shift+Z` in the keymap and the shell, notices "Redid the last undone
  change" / "Nothing to redo". Two orderings matter and only the headless test could catch them:
  egui's `consume_shortcut` matches with extra Shift and Alt **ignored**, so the shell must
  consume Ctrl+Shift+Z before Ctrl+Z — and the tuple's destructure is positional, so renaming the
  consumption order without renaming the bindings silently swaps the two. The test drives real key
  events through `run_ui` over a scanned library: undo steps the rating back, redo steps it
  forward, a second redo says so.
- ROADMAP M3's undo row ticked in the same commit, rewritten to say what is true now. README and
  PLAN §12 already reconciled (the 2026-11-21 pass); `cargo xtask docs` gates it.
- 586 tests; ci, deny, snapshot (two goldens updated: the shortcut list gained a row, the
  snapshot golden gained `undo_history`), a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: the keypress on a real keyboard (the key events are synthetic, though
  routed through the shell's own input handling) and how twenty-step notices read over a long
  session.

## 2026-11-22 (nineteenth) — Inline tagging in the cull stage, `T` re-applies there, and a deadlocked shell

- **SL-C11/C12, ROADMAP M4's last open row — M4 is now fully checked.** The cull stage gained an inline
  keyword strip under the action bar: chips for the keywords the photo on screen carries (click takes one
  off through `set_tag(.., false)`, one undoable op) and a field that adds one with Enter. Keywording now
  happens while the same photo is on screen instead of a round trip through the right panel. While the
  field is focused it owns the keys — `keyboard()` stands down for any focused widget — so P and T type
  letters, not decisions.
- `T` in a cull session re-applies the keyword you last added to the photo on screen (the grid's one-key
  repeat, aimed at the stage's own target), with the notice naming the keyword: "Tagged “backlit”" /
  "Already tagged “backlit”". Keymap gained a Context::Cull row for it; the empty-stage hint line mentions it.
- **A real latent deadlock the headless test exposed:** the top-bar search called `search.has_focus()`
  *inside* `ui.input(...)`. In this egui fork nested context access deadlocks the lock the closure runs
  under — meaning any Enter keypress anywhere in the app (the details-panel keyword field, the search box
  itself) could freeze the shell. `has_focus` is now read outside the closure; the search still commits
  when Enter is pressed while the box is focused. Found because the new test drives a real Enter for the
  inline field and hung for 10 s.
- Test details worth keeping: `egui::Button` in the fork has no `.id()`, so the chip is located the way a
  person sees it — by the text it paints ("backlit" bare; the toast repeats the keyword in quotes and the
  sidebar lists it too, both distinguished). And `keyboard()` runs after the view paints, so the strip
  catches up one frame after the key: the chip is asserted on the following frame.
- ROADMAP M4's inline-tagging row ticked in the same commit with a truthful description; UX.md §5.5 now
  names the strip and says `T` targets the photo on screen in cull.
- 587 tests; ci green.
- **Not verified by hand**: typing a long keyword over several photos on a real keyboard, and how the strip
  wraps when a photo carries many keywords (horizontal_wrapped lays it out; nobody has looked at it).

## 2026-11-22 (twentieth) — Phase 0: re-scope blockers, reconcile the docs (no feature code)

- **Course correction from the owner** (`docs/sortlens_agent_completion_directive.md`): "M0–M10 shipped" was
  wrong while core features sat in "Deliberately absent" tables. This entry is Phase 0 only; no Phase 1–5 code.
- `docs/ROADMAP.md`: new **Completion phases** section with Phases 0–5 as checkbox rows (Phase 0 ticked, the
  rest open). The M8 and M9 "Deliberately absent" tables became "Re-opened" pointers; M10's table lost the
  `.icns`/`.ico`, `craft-update` and high-contrast rows (now Phase 4) and keeps only what needs an identity, a
  toolchain, a screen reader or a decision. The two stale M1/M2/M3 rows point at their Phase 1 items. Status log
  gained a Phase 0 row and the old "shipped" row no longer says shipped.
- README badge/status and PLAN §12 reworded ("M0–M10 built, core completion in progress"); `cargo xtask docs`
  still computes `M0–M10` from the tracker, so the range string stays, the claim around it changed. The directive
  is linked from the README document table.
- `docs/RELEASE.md`: the "does not have" list rewritten from the facts — HEIC thumbnails and the tile-folder map
  exist, and PNG/TIFF/WebP *thumbnails* exist; what is missing is video posters, PNG export, models and the
  downloader, a bundled basemap, the remote, i18n and the keymap editor, installers, and the UI pass — each with
  its phase.
- **Dependency audit** (21 declared-but-unlinked pins, not 25: `craft-heif`, `craft-media`, `craft-jsonrpc` and
  `craft-mcp` had been linked since, and their attribution rows were still in the "not linked" table — moved to
  "Linked today"; the `craft-mcp`/`craft-jsonrpc` "hand-written" reasons were stale). Pruned from
  `[workspace.dependencies]` as having no consumer in Phases 1–5: `craft-flags`, `craft-fonts-build`,
  `craft-menu`, `craft-quit`, `craft-dialogs`, `craft-gainmap`, `craft-sniff`, `craft-zip`, `craft-crs`,
  `craft-webp`, `craft-ipc`, `chrono`, `log`. Kept, each tied to a phase in `docs/ATTRIBUTION.md`'s new
  "Linked by" column: `craft-video` (P1), `craft-atomic` and `rayon` (P2), `craft-keymap`, `craft-update`,
  `craft-i18n`, `craft-web-api`, `craft-web-host` (P4). `Cargo.lock` is unchanged (unlinked pins are not locked).
- Owner sign-offs recorded in ROADMAP: opt-in network only for `model.install` and `craft-update` (`crates/ai`
  stays offline), real permissively licensed weights and map data attributed in `docs/ATTRIBUTION.md`, licence
  `MIT OR Apache-2.0`.
- **Not verified by hand**: nothing visual changed.

## 2026-11-22 (twenty-first) — Phase 1 item 1: video poster frames in grid, detail and cull

- **SL-B11 part 2, ROADMAP M2's poster row ticked.** A video cell now shows its first frame, not a "VID"
  glyph. `sortlens-thumbs::poster::decode_video_poster` demuxes the clip straight from disk
  (`craft_media::FileReader` — a poster never reads a 4 GB clip whole), picks the first sync sample of the
  first video track (falling back to sample 0 for a track that marks none), and decodes it with
  `craft-video`'s H.264 (configured from `avcC`) or HEVC (from `hvcC`). Because a decoder releases the
  first picture only when a later access unit forces it out of its reorder buffer, the loop feeds at most
  eight samples before a flush; the bound keeps a broken GOP from decoding half a clip for one cell.
- **Colour conversion honours the stream's own colour tags** through `craft-color` (new workspace dep, same
  suite rev): matrix BT.709/601/2020 (SD-sized fallback when the stream is unlabelled), limited/full range,
  transfer (PQ and HLG decode to linear and re-encode to sRGB), and primaries gamut-mapped into sRGB for
  BT.2020 and P3D65 — same D65 white throughout, so one 3×3 each and no chromatic adaptation. The `tkhd`
  quarter-turn is applied after the resize, so a portrait phone clip shows portrait, matching the
  catalog's width and height, which already describe the displayed frame.
- **Routing is by header, not by name**, in `decode_file` — sniffed from the first 64 bytes before anything
  is read whole — so a clip renamed `.jpg` still gets its poster, a still named `.mp4` is still a still, and
  a raw (CR3 wears an ISO brand too) is still routed by name first. The poster flows through the ordinary
  `Service` → `Cache` path, so grid, detail view and cull stage all show it with no UI change, and the
  failure set falls back to the glyph pill for undecodable codecs (ProRes and friends refuse with a reason).
- Tests are **round-trips through the suite's own encoders and muxer** — no video blob committed: an H.264
  clip (Baseline, length-prefixed packets) whose first frame is a left-dark/right-light grey ramp and whose
  later frames are flat mid-grey, so a poster of the wrong frame reads flat and fails; the same through the
  HEVC encoder with a real `hvcC`; both routed through the ordinary `decode_file`; the rename cases; a
  truncated container and a non-container refused with reasons; and pure rotation tests (a 2×1 turned
  clockwise becomes 1×2 with the left pixel on top).
- `craft-video` moved from `docs/ATTRIBUTION.md`'s "Declared, not linked" table to "Linked today"
  (Phase 1's only unlinked dependency is now linked); `craft-color` joined it with its own row. ROADMAP's
  Phase 1 item 1 ticked in the same commit; RELEASE.md and README dropped "video poster frames" from the
  not-built lists. No UI crate changed, so no screenshot pass was needed.
- 593 tests; ci, snapshot, deny green.
- **Not verified by hand**: a real camera clip (phone HDR and a drone's) in the app; the fixtures are the
  suite's own encoder, which is honest about decode and colour but not about a manufacturer's bitstream
  quirks.

## 2026-11-23 (twenty-second) — Phase 1 item 2: stale-revision rejection, and the M3 test row closed

- **SL-D11, the optimistic-concurrency half.** Every `Photo` now carries `rev: u64` — the number of real
  annotation changes it has taken (rating, flag, label, keywords, capture time), bumped only when a value
  actually changes, so re-setting the same stars is not a revision. It travels in the log, the snapshot
  and every `AddPhoto`/`RemovePhoto` op, so a reopen reproduces the same numbers and undo raises it
  honestly (an undo also changes the record).
- **`Catalog::commit_expect(op, expected)`** refuses a write made against an older revision with
  `sortlens-core::Error::StaleRevision { expected, current }`, naming both numbers so a caller can
  re-read and retry. `Catalog::commit` stays the ordinary single-writer path; the guard is opt-in per
  write.
- **Every engine command response now carries `rev`** — the catalog sequence number the response
  describes — and **any command can pass `"expect": n`**, which is checked after the registry lookup
  and before anything runs: a stale caller is refused before it touches a single photo. This is the
  contract `docs/ARCHITECTURE.md` §4 already promised (`"rev": n` in every envelope); the code now
  keeps it, and the MCP examples file documents the `expect`/`rev` round trip for agents.
- **The M3 test row is closed.** Stale-revision tests: the catalog guard (stale refused with both
  revisions, current commits, nothing written on refusal), the per-photo `rev` (rises on real changes,
  not on no-ops, rises on undo, survives a reopen), and the engine round trip (`rev` on reads, `expect`
  accepted at the current revision and refused at a stale one, the newer stars not overwritten).
  Badge-contrast test: WCAG 2 relative-luminance contrast in `sortlens-ui::theme`, judging every
  preset's cell badges (offline dot, unreadable dot, white glyph on each) and flag dots at 3:1 against
  `thumb_bg` — the worst background a badge is ever painted over, because a badge exists exactly when
  there are no pixels. **It found a real defect**: Silver and Paper's `text_faint` (the offline badge
  dot) sat at 2.25 and 2.67 to 1 against their own thumb wells; both presets are darker now
  (Silver `0x969189` → `0x6f6a63`, Paper `0x8e8e88` → `0x6b655e`), which also lifts the faint text they
  carry elsewhere.
- ROADMAP's Phase 1 item 2 and both M3 rows ticked in the same commit; FEATURES.md's SL-D11 design note
  now says shipped. No layout or drawing logic changed, only two token values, so the headless run_ui
  suite and the pure token test cover it; the screenshot harness does not exist yet (Phase 3 item 9).
- 599 tests (6 new); ci, snapshot, deny green.

## 2026-11-23 (twenty-third) — Phase 1 item 3: the Recently Added source, honestly timed (SL-A14 part 2)

- **`Photo::added: Option<i64>`** — the instant Sortlens recorded a file entering the catalog, set by the
  scan the moment a new photo id is born and kept through every later update (a rescan that finds changed
  bytes re-indexes the file but does not reset its add instant, because the record did not re-enter the
  catalog). It travels in the log, the snapshot and every `AddPhoto` op, so a reopen reproduces the same
  number. **Never approximated from `mtime`** — that would answer "recently modified", a different and
  more confusing question.
- **The migration is the absence, faithfully carried.** The snapshot schema moved to 2; a schema-1 photo
  decodes with `added: None` and keeps it — no instant is invented, not even from the mtime sitting right
  there on the record. A `None` photo cannot answer the Recently Added question: the filter and the sort
  exclude it, and a resave writes schema 2 with the field still honestly absent. Tested against a
  hand-written schema-1 snapshot.
- **Catalog**: `Filter::added_after` (only photos recorded into the catalog at or after an instant) and
  `Sort::RecentlyAdded` (`recently-added`), newest record first, unknown sinks last, ties to the later id.
- **Engine**: `library.page` answers the question from the command layer (`filter: {"added_after": n}`,
  `sort: "recently-added"`), and the `added` value rides every photo envelope; the MCP examples document
  the call and the no-mtime guarantee.
- **Sidebar**: the `Recently Added` row joins Sources with its own count (the last 30 days, computed from
  the clock at draw time, not cached), clicking it replaces the whole question like every source does and
  asks it newest-record-first, and leaving it returns the user's sort. The window is stored as the
  *question* (`recently_added: bool`), so the cutoff is always the current 30 days even in a library left
  open past midnight.
- ROADMAP's Phase 1 item 3 and the M1 "Sources, part 2" row ticked in the same commit. UX.md §Sources
  already described the predicate ("indexed in the last 30 days") and now matches the shipped code.
- 604 tests (5 new); ci, snapshot, deny green. Screenshot harness does not exist yet (Phase 3A);
  no layout or drawing logic changed, only a new sidebar row painted through the existing `nav_row`, so
  the headless `run_ui` suite covers it.

## 2026-11-23 (twenty-fourth) — Phase 1 item 4: PNG export (SL-B24)

- **`Format::Png`** joins `sortlens-export`, so `Format::ALL` is `[Original, Jpeg, Png]` and the format id,
  CLI `--format png` and the dialog chip all exist. The export decodes through the ordinary pipeline
  (`sortlens_thumbs::decode_file`, falling back to `decode_image`), clamps the longest edge when asked, and
  re-encodes with `craft-image-codecs::encode` as PNG — the suite's own licence-approved encoder, so
  `cargo xtask deny` stays clean. **Lossless means lossless**: a source PNG exported to PNG reproduces
  every RGBA byte exactly, alpha included, and the test pins that byte-for-byte rather than “close
  enough”. A JPEG source exports fully opaque, and the output is verified to start with the PNG magic.
- **`Spec::summary()` no longer lies about PNG**: a PNG export says `PNG · lossless` and its longest edge,
  and never quotes a JPEG quality number. `quality` and `metadata` stay in the spec (a JPEG still uses
  them) but are inert for PNG; the dialog's PNG branch shows only the edge control and a hint that there
  is nothing lossy to pick, and the “Keep EXIF and IPTC” checkbox is JPEG-only.
- UI: the export dialog draws all three chips from `Format::ALL`; the PNG branch is covered by the
  headless `run_ui` test (the dialog does not rewrite the format, and the summary line says lossless).
  Engine: a worker export asked for PNG writes `.png` files that decode back as PNG — round-tripped
  through `begin_export`/`poll_export` like the JPEG face. CLI: `--format png` parses and the help text
  lists it.
- ROADMAP's Phase 1 item 4 and the M8 export row ticked in the same commit; the “Re-opened from M8”
  paragraph now lists three remaining Phase 2 rows; RELEASE.md's export limit names Original/JPEG/PNG;
  UX.md's export row marks quality and metadata as JPEG-only.
- 610 tests (6 new); ci, snapshot, deny green. Screenshot harness does not exist yet (Phase 3 item
  9); the change is one dialog branch and one new chip painted through the existing `chip`/`hint` rows,
  so the headless `run_ui` suite covers it.

## 2026-11-23 (twenty-fifth) — Phase 2 item 1: real weights, pinned by digest

- **`Manifest::shipped()` is no longer empty.** Three real, permissively licensed ONNX files are pinned:
  `yolos-tiny` — YOLOS-tiny detector, Apache-2.0, fp16 export, 13,251,235 bytes, input 1×3×512×800,
  answering `logits` [1,100,92] and `pred_boxes` [1,100,4]; and the CLIP ViT-B/32 int8 pair — MIT,
  `clip-vit-b32-vision` (89,023,662 bytes, 1×3×224×224 → `image_embeds` [1,512]) and
  `clip-vit-b32-text` (64,390,561 bytes, `input_ids` [1,77] int64 → `text_embeds` [1,512]). The files
  live in the GitHub release `models-v1` on this repository and are fetched only by the opt-in
  `model.install` command once it exists (Phase 2 item 2); nothing ships inside the app.
- **The pins are verified, not believed.** Candidate exports were rejected until each pinned file was
  proven to load *and run* under `tract-onnx` 0.23.10: the quantized CLIP pair and YOLOS-tiny answer
  real inputs with all-finite values; the 2019 zoo tiny-yolov3 (TF-style Squeeze), zoo ssd-mobilenet
  (unimplemented Loop), yolos as published and the transformers.js exports as published (symbolic
  `batch_size`/`num_channels` dims that tract cannot type) were all found and rejected — hence the
  pinned derivatives. YOLO-family/RT-DETR models under AGPL (YOLOv5/v8) and non-permissive licences
  (MobileCLIP's Apple ML licence) were excluded by the owner's sign-off.
- **The transformation is documented, not hidden.** Each pinned file is a derivative of an upstream
  export (Xenova's): the exporter's symbolic input shapes concretized (batch 1) and shape-inference
  annotations stripped, which changes no weights; upstream digests were checked against the
  publishers' recorded hashes before deriving, and GitHub records the same SHA-256 for each release
  asset. ATTRIBUTION.md's "Data and models" section now states all of this, keeps the sentence that
  no weights ship with the app, and records the I/O contracts (including that the COCO-91 label map
  and the CLIP BPE vocabulary ship with the inference slice).
- **The store and the manifest keep each other honest.** New tests: the shipped manifest satisfies
  its own `parse`, every shipped licence is one the owner approved (MIT/Apache-2.0), the pin is one
  detector plus one embedder pair with distinct digests, every pin is real-world sized (>10 MB — a
  fake few-byte pin would verify garbage perfectly), and every shipped id appears in ATTRIBUTION.md.
  `xtask attribution` learned the difference between the model-pins table and a dependency table, and
  now checks every release URL named in `manifest.rs` against ATTRIBUTION.md instead.
- ROADMAP's Phase 2 item 1 and the M8 manifest row ticked/named in the same commit; RELEASE.md's
  "no bundled models" bullet now says pins exist and what is still open (downloader, inference).
- 614 tests (4 new); ci, snapshot, deny green. No UI change, so no run_ui or screenshot work
  applies; the screenshot harness does not exist yet (Phase 3 item 9).

## 2026-11-23 (twenty-sixth) — Phase 2 item 2: the consent-gated model downloader (SL-F02)

- **`crates/engine/src/models.rs` is the only thing in the repository that fetches a model.** It was placed
  deliberately outside `sortlens-ai` — whose `no_network` test still holds, untouched — and deliberately
  desktop-only, with two new target-gated
  dependencies (`ureq` over rustls for HTTPS with a `Range` header; `craft-atomic` for the Windows-safe
  rename and the fsync before it).
- **Resumable, and the resume is tested.** The bytes stream into `<library>/models/<digest>.<kind>.part` in
  64 KiB chunks; a download that stops — cancelled between chunks, or the network dropped mid-body — leaves
  the part exactly where it got to. The next attempt sends `Range: bytes=<len>-` and appends. Three
  dishonest servers are answered by code, not luck: one that ignores the range (a fresh 200 body triggers a
  restart, never a splice), one that truncates (the part is kept, the error says "running it again resumes"),
  and one that sends more than the manifest promised (the part is removed — it can never be a resume point).
- **The digest is the contract, enforced twice.** A finished part is verified against the manifest
  (streaming, 64 KiB at a time — a 166 MB model is never held in RAM) before it is renamed into place under
  its digest name, and the renamed file is read back and verified again. A file that is not the model never
  becomes a model, and it never sits where a reader would find one.
- **`sortlens-ai` gains two pure, offline additions**: an incremental SHA-256 (`Hasher`, pinned equal to the
  one-shot over every padding boundary 55/56/64/65/…) and `Model::verify_file`, the streaming check that
  `Store::status` now uses — so a Settings row no longer reads a model file into memory to answer
  "is it installed".
- **Consent is a state, checked before anything moves.** `Engine::models_consent` starts `false` every
  session; the app persists the switch in Settings → AI (`prefs.ai.download_consent`) and restores it at
  launch. `model.install` refuses without it from every face — UI, CLI, MCP — with a refusal that names the
  setting. The engine answers the model verbs from `self.model_manifest` (the shipped pins; a test swaps in
  a loopback manifest, because no test can serve bytes under a digest it does not know).
- **The surfaces**: Settings → AI lists every pinned model with licence · kind · size and its verified
  state; the Install buttons exist only behind the consent switch; a download draws its progress and a Stop
  that keeps the partial file. Activity gained a `Model` job whose detail is the byte count while running
  and the install receipt when done. CLI: `models`, `model-consent on|off`, `model-install <id>` (a
  progress verb like `sharpen`), `model-remove <id>`, plus a `models` JSON schema. MCP: a `model.status`
  example and executable examples for `model.cancel`/`model.remove`; `model.install` and `model.consent`
  carry notes (a proven example cannot download).
- **Tests that fail when the behaviour is absent** (8 module tests + 5 engine tests, all over loopback
  servers — no network): the verified install under its digest name; interrupted-then-resumed with the
  Range request asserted on the wire; the 200-restart; wrong bytes refused with the part removed; a
  mid-copy cancel keeping the part byte-for-byte and resuming from it; a stale too-long part not trusted;
  a manifest without a URL refused before any fetch; the worker polled to an install; consent refusal
  naming Settings; install/remove/progress and the row cache following the store; the UI gate driven by a
  real pointer click on the checkbox in the headless `run_ui` harness (Install absent without consent,
  present after, gone again when consent is withdrawn; `Action::SetModelConsent` pushed both ways).
- `cargo deny` gained one licence decision, written in `deny.toml` where it belongs: webpki-roots' bundled
  TLS root list is CDLA-Permissive-2.0 — permissive community *data*, no code under it. ATTRIBUTION.md
  moved `ureq` and `craft-atomic` to "Linked today".
- ROADMAP's Phase 2 item 2 and the M8 manifest row ticked in the same commit; RELEASE.md's model bullet now
  says the downloader exists and what is still open (inference); UX.md's dialog table and ARCHITECTURE.md
  §9 + the store layout gained the downloader and the `.part` file.
- 633 tests (19 new); ci, snapshot (two goldens updated: the schema list gained `models`), deny green.
  CLI verified by hand against a fresh library: `models` lists three pins, `model-install` without consent
  refuses naming Settings, `model-remove` reports nothing to reclaim.
- **Not verified against the real release**: a live download of the pinned 13/89/64 MB files from the
  `models-v1` GitHub release (this run keeps the tree offline by choice; the loopback suite covers the
  protocol, and the digest pins make any wrong byte a refusal, not a usable model).

## 2026-11-23 (twenty-ninth) — every keyword says who wrote it (SL-D12, Phase 2 item 4)

- `sortlens_core::TagSource`: `Manual`, `Ai`, `Xmp`, and `Unrecorded`. The fourth is the honest one: a library
  written before sources were kept has keywords nobody recorded an origin for, and migrating those to `Manual`
  would invent a decision. It is **derived** — a tag with no entry in the map — never stored, so no snapshot
  ever claims it. `parse`/`id`/`label`/`explanation` and a serde round-trip test.
- `catalog::Photo.tag_sources: BTreeMap<String, TagSource>` with `#[serde(default, skip_serializing_if)]`, so a
  snapshot from an older build replays and rewrites **byte-identically** (asserted in a test that reads the
  snapshot, checks no `tag_sources` key was invented, reopens, and asks the filter for `Manual` — it answers 0).
  `adopt_review` marks a sidecar's keywords `Xmp`; `tag_source`/`carries_source`/`prune_tag_sources` are the
  read side.
- New additive `Op::TagSourced`, committed in the same `Op::Group` as the `Op::Tagged` it describes. That choice
  is what keeps one ⌘Z for a tag *and* its receipt without breaking the 2-tuple `tagged` records already on disk.
  Its `photo_count` is **0** and a group's summary skips it, because a keyword typed by hand is one change, not
  two — without that, every ordinary tag read "Keyworded 1 photo (+1 more change(s))" and a 40-photo keyword
  counted as 80. `Op::name` for a group holding one real change now returns that change's name, so the change
  list says `tagged`, not `group`.
- Pruning runs **after the whole op**, not inside the apply arm: a group sets the words and then their provenance,
  and an undo replays both in reverse, so mid-op pruning dropped a source that the next step was about to restore
  (found by the undo half of the rename test, not by reading the code).
- Spelling out where each source comes from: `Engine::set_tag_from` (hand-typed is `Manual`), `accept_suggestion`
  (`Ai`), sidecar adoption at import (`Xmp`). `rename_tag` carries a source to the new name and `delete_tag` takes
  it away, both inside the group they already committed — a renamed AI keyword is still AI, and a deleted word
  leaves nothing claiming it.
- The **catalog revision is deliberately not bumped** by provenance: `rev` is what refuses a stale sidecar
  writer, and provenance never reaches a sidecar. `docs/BEHAVIOUR.md` §4 now says that, because §1's "every
  annotation change increments the revision" is otherwise a contradiction.
- Surfaces: `Filter::tag_source` ("some keyword came from there", so one accepted suggestion among ten typed
  words counts), the filter strip's cycling **Source** chip with its own tooltip per source, the keyword chip
  printing `cat · AI` instead of hiding the fact in a hover, `sortlens-cli list --source` / `page --source` with a
  refused typo listing the real names, `sortlens-cli`'s MCP `library.page` note, and `library.page`'s filter JSON.
- Tests: core enum round-trip; catalog (partial-group undo, rename carries, delete clears, ghost map pruned,
  legacy snapshot → `Unrecorded` and *not* `Manual`, sidecar → `Xmp`, filter semantics); engine (Manual vs AI
  through `library.page`, one undo for word plus origin, rename/delete); UI (`Context::run_ui` clicks the Source
  chip twice — Source → Manual → AI — asserting the query actually narrowed and the chip relabelled, then finds
  `guessed · AI` painted on the keyword chip); goldens regenerated and read: the only visible changes are the
  new `tag_sources` field, the extra ops in the log, and `group · Flagged` becoming `flag · Flagged`.
- 674 tests; ci, deny, snapshot, a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: how the Source chip sits next to the other seven in the strip at a narrow width, and
  whether `cat · AI` crowds a long keyword. There is still no `cargo xtask screenshots` (Phase 3 item 9, which is
  explicitly to be done before the rest of the visual work), so the visual evidence is the headless paint.

## 2026-11-24 (seventy-first) — the release builds the phone's APK, and the workflow file is a checked file at last

- **Why this came up.** The seventieth entry closed with "everything buildable left is done", but its own
  reasoning had folded two different blockers into one: the `.ipa` leg waits on the Apple identity, while
  the **APK leg needs no external secret at all** — a sideloaded Android APK is signed with a self-signed
  keystore, and the build runs on a runner that already carries the NDK. The roadmap's 3b still had it open,
  so it was the one buildable item left in the owner's Phase 0 → Phase 5 plan.
- **Shipped.** `.github/workflows/release.yml` gained an `android` job: `cargo apk build --release` from
  the crate's own `package.metadata.android` (the manifest is the specification cargo-apk reads), the SDK
  pieces the metadata names installed by `sdkmanager` (`platforms;android-34`, `build-tools;34.0.0`,
  `ndk;26.3.11579264`) rather than trusted from the runner image, then `zipalign` + `apksigner` with a
  keystore generated for the run and a signature verified before upload. The APK rides the same one-runner
  checksum step and release attachment as every other artefact. Two latent defects in the never-run
  workflow were fixed in the same commit: `verify` had no Linux windowing headers (the `cargo xtask ci`
  it runs needs them on Ubuntu, as `ci.yml`'s own job does) and it used a `dtolnay/rust-toolchain@1.95.0`
  ref where the checked `ci.yml` uses `@master` with the toolchain as an input; the release body — which
  still said "no bundled models, no remote client, no localisation yet", three stale claims — was rewritten
  to say what ships.
- **Checked, not just written.** `cargo xtask workflows` now reads `release.yml` the way it reads
  `ci.yml` and `agent-intake.yml`: a release with no `cargo apk build` step, a `publish` not gated on a
  tag (a manual run from `main` must never create a release — that gate is new too, which is why a
  dispatch can be used to exercise the pipeline), checksums that leave the APK out, and an attachment
  that leaves it out are each refused with their file named. The subset reader refuses flow collections,
  which caught `needs: [build, android]` and made the `on.push.tags` list a block list. Tests: the shipped
  file passes, and a fixture release loses the APK build, the tag gate, the attachment and the checksum
  coverage one at a time, each naming its own refusal.
- **Reconciled.** ROADMAP 3b moved the APK out of "Still open" into a shipped bullet and split the rest
  by its real unblocker (device verification and the iOS Photos library need the NDK/Xcode machine; the
  `.ipa` needs the Apple identity; an AAB needs Play tooling; an in-place-update key is a release secret
  the owner can add); RELEASE.md's phone bullet says what a release run now attaches and what still waits.
- **Verification status.** `cargo test -p xtask workflow` (15, including the four new ones) and the
  checker's read of the shipped file are green here; the full gates below. The one thing this checkout
  cannot do is run the workflow itself — that is what a manual dispatch after the push is for, and the
  result is read with `gh run` rather than assumed.

## 2026-11-24 (seventy-second) — the APK leg compiles: on a phone, the trash is the library's own folder

- **Why this came up.** The release workflow's first real run (a manual dispatch after the seventy-first
  push, run 45513471896) failed exactly where a `cargo check` could never see it: linking. The `trash`
  crate does not compile for `aarch64-linux-android` — its platform module has no freedesktop-less
  answer — and the phone's delete path asks for `Mode::Trash`.
- **Shipped.** `Mode::Trash` on Android and iOS now moves the file into the library's own
  `.sortlens/trash` (the name `crates/scan` refuses to descend into, so a rejected photograph never
  reappears in the grid), through the same `move_into` the `Move` mode uses, after the same `audit_one`
  re-check — a file whose bytes or mtime moved since it was indexed is refused here too, even when the
  function is called outside `apply`. The receipt still says `Trash`, and `undo` refuses a trash receipt
  on every platform, so the desktop's semantics are unchanged and nothing destroyable is destroyed: the
  file sits in the folder for a person to recover by hand. The dependency is target-gated to the
  platforms the crate speaks (`windows`/`macos`/`linux-not-android`); the workspace pin stays.
- **Tests.** `the_phone_trash_moves_into_the_librarys_own_skipped_folder` runs on the host on every
  platform (a phone path nobody can test would be the placeholder the plan forbids): the move preserves
  the relative path, the bytes read back, a changed-size file is refused and stays where it was, and the
  undo report refuses the receipt while leaving the file alone. `the_phone_trash_folder_is_skipped_by_
  the_scan` in `crates/engine` is the one place both sides of the cross-crate contract run: a library
  containing `.sortlens/trash` scans clean, the folder refused (hidden on Unix, a system dir on
  Windows), never descended into.
- **Reconciled.** ROADMAP 3b gained the shipped bullet and re-split "Still open" by its real unblockers;
  `docs/BEHAVIOUR.md` §3 says what the trash mode does on a phone.
- **Next.** Re-dispatch the release workflow; `verify` and the desktop matrix were already past `android`
  when it failed, so the APK job is the only leg unproven.

## 2026-11-25 (seventy-third) — the release signs the APK it builds and uploads what it packages

- **Why this came up.** The seventy-second entry fixed the compile the first dispatch run (45513471896)
  stopped at, and left the APK leg "unproven" with a note to re-dispatch. The re-dispatch (45519830389)
  came back with two failures that no local gate could see, and they were different in kind: the APK
  build reached the last step and stopped there, while the three desktop jobs passed and attached
  nothing at all.
- **A release build of the APK refuses to sign without a key.** `cargo apk build --release` aligns the
  APK and signs it with `apksigner` itself, and stops with `Configure a release keystore via
  `[package.metadata.android.signing.release]`` when it has none — which is the state a public repo is
  in on purpose. cargo-apk 0.10 reads `CARGO_APK_RELEASE_KEYSTORE` and its `_PASSWORD` before the
  manifest (checked in the crate's own `src/apk.rs`), so the android job now generates a keystore for
  the run, names it to `cargo apk`, and lets cargo-apk align and sign. The old `zipalign` + `apksigner`
  pair is gone: it ran after a build that never produced an APK, and re-aligning a signed APK is the
  one way to lose a signature that was already correct.
- **The three desktop jobs uploaded nothing and passed.** `path: dist/*.tar.gz* dist/*.zip*` was read by
  `upload-artifact` as one newline-separated list — so as one glob with a space in it, which matches no
  file. The step warned, the action default (`if-no-files-found: warn`) kept the job green, and the run
  ended with zero artifacts: a `publish` on a tag would have had nothing to checksum or attach. Both
  upload steps now carry one glob per line.
- **The checker could not have caught either.** `cargo xtask workflows` now refuses an APK build whose
  step is not given `CARGO_APK_RELEASE_KEYSTORE=`, a release with no `keytool -genkeypair` and no
  `apksigner verify`, an `upload-artifact` whose `path` line holds more than one glob, and a `publish`
  that never downloads the built artefacts — four rules, four fixtures, plus the shipped file.
- **Verified on the run itself** (45527734155, commit `09cd112`): every job green, `publish` skipped as
  designed on a dispatch; the log says `Verifying alignment of .../target/release/apk/sortlens-mobile.apk
  (4)`, `Signing ... with keystore /home/runner/work/_temp/sortlens.keystore`, and `apksigner verify
  --print-certs` printed `Signer #1 certificate DN: CN=Sortlens` with SHA-256 `61be0349ac75…`. Four
  artifacts exist where there were none: `android-apk` 71.3 MiB, `x86_64-unknown-linux-gnu` 138.3 MiB,
  `x86_64-pc-windows-msvc` 33.9 MiB, `aarch64-apple-darwin` 35.7 MiB.
- **Files.** `.github/workflows/release.yml` (the keystore step, the two variables, the verify step, one
  glob per line, a header that says why); `xtask/src/workflow.rs` (four rules, the module doc, the
  fixture and four tests); `docs/RELEASE.md` (what a release run signs with and why the archive step
  uploads one glob per line); `docs/ROADMAP.md` (the 3b bullet rewritten around the two bugs and the
  verified run).
- **Tests.** `cargo test -p xtask workflow` (16 pass): `a_release_that_puts_two_globs_on_one_upload_line_
  `is_refused`, `a_release_that_builds_the_apk_without_a_keystore_is_refused`,
  `a_publish_that_downloads_nothing_is_refused`, and the three older release tests still refuse an
  unattached APK, an ungated `publish` and a release with no `cargo apk build`. `cargo xtask ci` green
  through every gate, `cargo xtask workflows` included.
- **Next.** The `publish` job has still never run: it needs a `v*` tag, and a release is a decision a
  person makes. Everything above it — `verify`, the four builds, the signing, the artifact upload — is
  now proven by a real run. Phase 4 item 3b keeps its named unblockers: a device build, the iOS Photos
  library, an `.ipa` (Apple identity), an AAB (Play tooling), and a release key the owner can add so one
  APK updates another in place.

## 2026-11-26 (seventy-fourth) — every verb that answers one object answers `--json`, and the flags it reads are the flags it honours

- **Why this came up.** The roadmap row said `page` prints JSON and "there is no `--json` for the other
  verbs". That was half stale — forty schemas already existed — and half a live bug, and the bug was the
  expensive half: the JSON form of `list` put `--tag` at the top level of the request, and
  `library.page` reads only `args["filter"]`, so `list --json --tag dunes` answered **the whole
  library**. `--source` was not read at all on that path. The plain form built a real `Filter` and
  filtered; the JSON form silently answered a different question than the one asked. The plain `page`
  arm dropped `--tag` the same way (its filter carried only `tag_source`).
- **One mapper, so the two spellings cannot drift.** The `list`/`page` arm of `json_verb` builds the
  same `Filter` the plain `list` does — `tags`, `tag_source`, offset/limit/sort — and the plain
  `page` arm now just calls the mapper. An unknown source is refused with the real ones rather than
  treated as "no filter": the class of bug a filter must not have. Golden steps `45`–`49` pin the
  behaviour: `tag 1 dunes` → `list --json --tag dunes` is 1 photo, `--source manual` is 1,
  `--source unrecorded` is 0, and `--source guessed` is a refusal naming `manual, ai, xmp, unrecorded`.
- **The eleven verbs that had no schema now have one**, each the engine answer it already produced:
  `page`, `insights`, `report`, `refresh`, `compact`, `backup`, `restore`, `move`, `unmove`,
  `model-consent` and `update-consent`. `report` already printed its object as JSON without the flag;
  it now answers it with the flag too. The two consent switches gained `changed` like every other write,
  which exposed a truth worth stating in the golden: the CLI keeps consent for the run only, so a fresh
  process setting `off` (the default) reports `changed: false` — the engine test pins the in-session
  repeat that reports `false` the second time.
- **Four verbs were reachable only through `--json`** — `label`, `mark`, `marks`, `jobs` — and now
  print prose like the rest. `move`'s schema note now says the destination is absolute (the engine
  refuses a relative one, and the existing golden `25` pins that refusal; the new golden uses an absolute
  path and follows it with `unmove`, so the move/unmove pair is exercised end to end for the first time).
- **The usage text was wrong, not just thin.** Its `--json` paragraph sat *between* `backup` and
  `restore`, three verbs were missing, and it named none of the newer commands; it is rewritten, and its
  final paragraph now says the JSON form takes the same flags as the plain form and lists the passes that
  refuse.
- **Tests.** Twenty-five new golden steps (`45`–`69`) cover every new schema, the keyword-filtered page,
  the unknown-source refusal, a pass that still refuses, a no-op consent switch, and the move/unmove pair.
  The contract test now names 23 one-object writes (up from 15), eight passes and `diagnostics` as verbs
  that must *refuse* `--json`, so "no schema" is a decision with a test rather than an omission. The
  support report's `os`/`arch` strings are normalised like the diagnostics bundle's, so the new golden is
  portable; JSON-escaped Windows paths are normalised too. A new contract test reads the help text a person
  sees and requires every schema verb to appear in it — it failed on `schemas` itself before the row was
  exempted, which is the proof it bites.
- **Reconciled.** `docs/CLI.md` (verb lists, the pass list, a "the same flags" paragraph, the test
  section), `docs/TOOLING.md` (the "pin five schemas" claim and a refusal example that quoted a verb list
  from before forty schemas existed), `docs/ROADMAP.md` (the row ticked with what it really was; the
  "pin five of the schemas" parenthetical corrected).
- **Verified.** Clippy `-D warnings` on the two changed crates; the engine's five consent-filtered tests;
  the goldens regenerated and **read**, which is how the `--source ai` step was found to be named "a
  source nothing has" while the fixture did have an AI tag (it now asks for `unrecorded`), and how the
  relative `move` destination was found to be refused rather than moved. `cargo xtask ci`, `snapshot`
  and `deny` green (the CI exit code read from a log file, not from a pipeline's); no screenshot change,
  since nothing painted changed. Because the slice touches `apps/**`, the release workflow was dispatched
  once on this commit — run `45536428707` — and is checked next tick, not watched.
- **Next.** Check run `45536428707` with one `gh run view`. Then `docs/ROADMAP.md`'s last buildable honest
  row: **SL-L08, localised numbers and dates** —
  counts, file sizes and dates are formatted with English rules whatever the picker says. The four other
  catalogues and the RTL screenshots still wait for a translation pass and an Arabic font build.

## 2026-11-26 (seventy-fifth) — the screenshots gate compares against a set named for the adapter, and a red run hands over what it painted

- **Why this came up.** `CI` on `main` was red, and had been for six commits: the loop read release runs
  and never this one, so the one red it could not see was the one it could have fixed. Run
  `45540115487` on `b1e7578` named two jobs — `screenshots` and `mobile` — and this tick took
  `screenshots`, because the work for it was already in the tree (an uncommitted `ci.yml` and
  `crates/ui/tests/screenshots.rs`, finished here rather than left as a second tick's inheritance).
- **The screenshots job was comparing one adapter's paint against another's.** The committed baselines
  are painted by the reference rasteriser; the runner has no GPU, so wgpu finds Mesa's software Vulkan
  (lavapipe), which paints a few pixels differently. The old gate answered that with a byte-for-byte
  comparison across two different painters, which can only be red. `SORTLENS_SNAPSHOT_SET` now names a
  baseline **set**: with a set named, every scene compares against `tests/snapshots/<set>/<name>.png`.
  The tolerance does not move — both are compared byte for byte — so a set is either painted by that
  adapter or absent, and the job cannot go green by being forgiven.
- **The set name becomes a directory, so it is sanitised.** It arrives from a workflow written by hand
  and lands under `tests/snapshots/`, so `set_dir` lower-cases it and folds every run of anything that
  is not a letter or a digit into one dash: `Mesa LLVM 15.0` is one folder, and `../..` is no folder at
  all instead of an escape from the baselines. A blank or unset value is the committed set, which is
  what a developer's own machine uses.
- **A red run now leaves its paint behind.** `if-no-files-found: ignore` guards an upload of
  `crates/ui/tests/snapshots/**/*.new.png` as the `screenshot-set` artifact, `if: always()`. A set
  nobody has committed cannot compare, and the frames it failed on are exactly the frames that set
  needs: `gh run download <id>` gets them, a person reads them, and they are committed under the set's
  directory. Without that step a red `screenshots` job had nothing to fix it with.
- **Verified.** `cargo xtask workflows` accepts the edited `ci.yml` (`xtask workflows: ok`) — the
  checker already refuses a gate that cannot fail, and an `if` this one is not. `cargo xtask ci` green.
  The new test `a_baseline_set_scopes_every_scene_to_its_own_directory` passes and would fail if
  `scoped_name` stopped prefixing the set: it pins the empty, blank, plain, mixed-case and escape
  spellings. No baseline was regenerated and none is read differently — the reference set is unchanged,
  which is the point of a set.
- **Files.** `crates/ui/tests/screenshots.rs` (`SORTLENS_SNAPSHOT_SET`, `baseline_name`, `scoped_name`,
  `set_dir`, one test, the two `try_snapshot` call sites); `.github/workflows/ci.yml` (the set's value and
  the artifact upload); `docs/ARCHITECTURE.md` (what a set is and how one gets committed);
  `docs/ROADMAP.md` (the Mac row's "separate baseline set" — that mechanism exists now, the Mac set and
  the embedded symbol faces do not; and the gate's own row says how it compares and what it uploads).
- **Next.** The `lavapipe` set is not committed yet: this commit is what paints it. Read the `CI` run
  for it, `gh run download <id> -n screenshot-set`, read the frames and commit them under
  `crates/ui/tests/snapshots/lavapipe/`. **The other red job is still red** and is the tick after:
  `mobile` (android, `aarch64-linux-android`) stops in `ring`'s build script with `failed to find tool
  "aarch64-linux-android-clang"` — the job's comment says the Ubuntu runner carries the Android NDK, and
  its log says nothing on PATH answers to that name, so the job has to install and name the NDK's clang
  (or state honestly that the target is unverified here) rather than assume it.

## 2026-11-26 (seventy-sixth) — a first launch speaks the interface language, and its count sentence is not forced through English's plural

- **Why this came up.** The i18n row's open list said "the unrouted surfaces", and the screens a new
  owner meets first were exactly that: the Welcome card, the indexing screen and the "Nothing matches
  these filters" screen painted English words whatever Settings → Language said, so a Japanese or
  Arabic interface introduced itself in English.
- **What was routed.** Every word `welcome_add`, `welcome_indexing` and `empty_no_matches` paint is now
  `tl!`-keyed, with rows in `ar.tsv` and `ja.tsv`: the title, the promise line, the three promise
  titles and their details, `Add Folder…`/`Not now`, the format line, `Indexing…`, the scan progress
  line, the progress bar's own word, `Found so far`/`Nothing yet`, the empty-filter headline,
  `Clear filters` and its hover text — 21 keys.
- **The count sentences are one template, not a `format!`.** "N photograph(s) in this library…" used to
  pick its noun in English (`if library == 1`), a decision a language cannot override: Arabic and
  Japanese write the same noun for every count. The sentence is now a template with `{n}` filled by
  `i18n::fmt`, so the count is written in the language's own digits, and the two English sentences stay
  two keys (`1 photograph`, `3 photographs`) for the languages that do inflect. The scan line is the
  same shape with `{files}` and `{folders}`.
- **English is unchanged, which is why no baseline moved.** A `tl!` with no row is the English text
  itself, and English has no catalogue, so the card paints byte for byte what it painted before; the
  test asserts the whole English count sentence, not a word of it, rather than trusting the claim.
- **Tests.** `the_welcome_card_and_the_empty_filter_screen_speak_the_interface_language` paints both
  screens headlessly at 1100×780: English first (the entire count sentence, exactly as it was built),
  then Japanese (`Sortlens へようこそ`, and no English left on the card), then Arabic, where the
  three-photograph sentence must contain `٣`. It fails the moment a string goes back to a bare literal.
  The catalogue audit in `i18n.rs`, which reads `shell.rs` for `tl!` keys, now demands an ar and a ja
  row for all 21, so a routed string without a translation fails instead of quietly showing English.
- **Files.** `crates/ui/src/shell.rs` (`welcome_add`, `welcome_indexing`, `empty_no_matches`, one test);
  `crates/ui/src/i18n/ar.tsv` and `ja.tsv` (21 rows each); `docs/UX.md`, `docs/ARCHITECTURE.md`,
  `docs/FEATURES.md`, `docs/ROADMAP.md` — the same reconciliation: what is routed, and what is still
  English by omission (the dialogs, and the sentences the engine composes for a receipt or a spec
  summary).
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the new test and the
  `language` filter (nine tests, the picker's names among them) pass; `cargo xtask ci`, `snapshot` and
  `deny` green — exit codes written to a file and read from it, since the first `ci` run's log was lost
  to a redirect that PowerShell wrote as UTF-16 — and `cargo xtask screenshots` painted all 27 scenes
  against the committed baselines, `empty_welcome` among them, so "English is unchanged" is a
  comparison rather than a claim. No release workflow was dispatched: no build or release surface
  changed (this is `crates/ui` source, tests and docs).
- **Next.** The export dialog is the next unrouted surface, and it needs one piece of engine work first:
  its count line prints `Spec::summary()`, an English sentence built inside `sortlens-export`, so the
  dialog can only be fully translated once the UI can compose that line from the spec's own fields (or
  the engine hands back parts rather than a sentence). The export receipt is the same shape. No release
  workflow was dispatched: this slice touches no build or release surface (`crates/ui` source, tests and
  docs only). The `CI` run for the baseline commit before this one, `45547733709` on `974f038`, had every
  job green but `screenshots` still painting when this tick read it — that run is the next tick's first
  read, one `gh run view`, because it is the proof the lavapipe set is whole.

## 2026-11-26 (seventy-seventh) — the rejects bin speaks the interface language, and a window's first frame is a measurement

- **Why this came up.** The dialogs have been joining the catalogues one slice at a time, and the
  rejects bin — the screen holding the only record of what a cull batch did — was still painted entirely
  in English words: its title, its empty state and all three of its buttons. The catalogue already
  carried a `Recently Deleted` row for the sidebar's foot row, but the dialog printed a bare literal, so
  the same word was two things: a translated menu row and an untranslatable title.
- **What was routed.** Eleven keys: the title, `Nothing has been deleted yet.`, the sentence about a cull
  batch being listed there, `Restore`, `Forget`, `Empty bin…`, the three hover sentences, and both
  toasts (`Restored {n} photo(s)`, `Forgot a batch of {n} file(s)`) as `i18n::fmt` templates with the
  count in the language's digits. The title reuses the row the sidebar already had rather than adding a
  second translation of the same words for the same surface.
- **What stays English, and says so.** A batch row's `Batch::summary()` and its mode are
  `sortlens-engine`'s own sentences — the same words the report prints — and the shell does not
  re-word them. `Moved` and `Rename keyword` remain the two dialog titles that are still literals;
  both are named in UX.md rather than implied closed.
- **Rows.** Eleven rows in each of the six catalogues (`ar`, `ja`, `ko`, `th`, `vi`, `zh-TW`), appended
  after the rating-filter block. The first attempt appended a `Recently Deleted` row the catalogues
  already had; the audit's duplicate-row check caught it on the first run, which is the check doing the
  job it was added for.
- **Tests.** `the_recently_deleted_dialog_speaks_the_interface_language` paints the dialog at
  1100×780 in English (title, empty state and the cull-batch sentence, character for character), then
  Japanese (the title is the catalogue's own `最近削除` and no English is left on the screen), then
  Arabic (`المحذوفة حديثًا` and `لم يُحذف شيء بعد.`), and asserts the dialog is not closed by being
  painted. `catalogs_load_and_routed_keys_have_rows` now demands all eleven rows in all six languages.
  The paint needed a lesson the other window tests had not needed: an anchored `egui::Window` measures
  itself on its first frame and paints from the second, so the harness runs a warm frame and reads the
  frame after it — reading the first frame alone is an empty shape list, not a missing translation.
- **Files.** `crates/ui/src/shell.rs` (`deleted_dialog`, one test);
  `crates/ui/src/i18n/{ar,ja,ko,th,vi,zh-TW}.tsv` (eleven rows each); `docs/UX.md`, `docs/ARCHITECTURE.md`,
  `docs/FEATURES.md` — the routed surface and the two titles it leaves English.
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the whole
  `sortlens-ui` lib suite green (219 tests, the new one among them); `cargo xtask ci` stopped at
  `cargo fmt --check` on one `fmt` call it wanted joined, which is fixed and re-proved with
  `cargo fmt --all -- --check`; `cargo xtask snapshot` green. **Local `cargo xtask deny` cannot run on
  this checkout and neither can it be fixed from here:** cargo-deny clones its advisory database onto
  `/Volumes/Expansion` (exFAT), macOS writes a 4096-byte `._*` AppleDouble beside every file, and the
  parser rejects the first one as invalid UTF-8. Deleting them and re-cloning reproduces them; CI's
  `deny` job on Linux is the proof for this commit. English is
  byte-identical, so no screenshot baseline moves, and `cargo xtask screenshots` was not spent on a
  set this slice cannot change: it adds no English word and no layout. No release workflow was
  dispatched: this is `crates/ui` source, catalogues, tests and docs.
- **Next.** The top bar's own words are the last English on the canvas: `Local library`, the read-only
  sentence and its hover, then `New collection` in the sidebar, then the `Moved` and `Rename keyword`
  dialog titles (their bodies are routed already). The `CI` run for `62fb266` (`45667132463`) was still
  in progress when this tick read it; the run before it, `45664188618` on `5685d9b`, was green and is
  what says the lavapipe set is whole. No release run is in flight.

## 2026-11-26 (seventy-eighth) — the trunk's screenshot gate goes green by adopting the frames the failing run itself uploaded

- **Why this came up.** `CI` on `main` had been red since `25412ce` (rivers, lakes and borders on the Places map) — four runs, because the loop had been reading release runs and never this one. The run on `9485f66` (`45757388716`) named four jobs: `ci` on all three legs, and `screenshots`. The `ci` legs were one `cargo fmt` diff in `crates/ui/src/shell.rs` (`shown_centre`, a call the formatter wanted split), and `44e3e40` carries that fix: its three `ci` legs are green. What was left red was `screenshots`.
- **Why it was red, exactly.** `25412ce` repainted the reference `places-charcoal`/`places-paper` baselines but not their `lavapipe/` twins, and `44e3e40` repainted the reference `rtl-library-*` frames for the routed header chips and could not paint the lavapipe set either — this machine has no Mesa adapter, so that paint exists only on the runner. The run on HEAD (`45759603628`, job `139706240234`) failed on exactly those four frames and uploaded them as its `screenshot-set` artifact, which is the hand-over the seventy-fifth entry built the gate to make.
- **The ritual is a command, not four renames.** `gh run download 45759603628 -n screenshot-set -D .ci-shots`, then `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt .ci-shots` — the `--adopt` path already in `xtask`, which refuses to run without a set name (so one machine's paint cannot land in the committed defaults) and copies each `<scene>.new.png` to `<set>/<scene>.png`. Four frames adopted.
- **Read, not just copied.** The two Places frames carry the rivers, lakes and borders `25412ce` added and the `Map: Natural Earth (public domain)` credit line; the two RTL frames are the same Arabic library as their reference frames — the same chips, the same `١٢ صور`, the same right-hand sidebar — differing only in text rasterisation, which is the whole reason a set exists. The comparison stays byte for byte: a set is either painted by that adapter or absent, so this commit records a paint; it does not loosen a test.
- **Verified.** `cargo xtask ci` green (exit code read from a log file), `cargo xtask snapshot` green, `cargo xtask deny` green. `cargo test -p sortlens-ui --features screenshots --test screenshots places_paints_in_both_themes` compares the *reference* set against this machine's paint and differs by 615 and 712 pixels; the diff image was read, and the difference is the `⌘K` hint's glyph and the credit line's text antialiasing, not layout, so the reference frames stay as the reference machine painted them while the lavapipe set is the runner's own paint. No release workflow was dispatched: four PNGs and this entry, no build or release surface.
- **Next.** Read this commit's `CI` run with one `gh run view` — it is the first run since `8123392` that can be green on all five gates if the adopted paint is whole. Then SL-L07's last English: the top bar's `Local library` and the read-only sentence with its hover, then the `Moved` and `Rename keyword` dialog titles (`+ New collection` is routed now, so the sidebar half of that row is done).

## 2026-11-26 (seventy-ninth) — the top bar says the library's name in the interface's language, and the keyword editor renames in it

- **Why this came up.** The RTL frames still painted English canvas chrome, and the queue named the top
  bar's `Local library` line and its two read-only sentences as the next row. All three were literals at
  the call site, so an Arabic shell never painted `مكتبة محلية` and still said
  "Read-only · another window has this library" in English over an Arabic library.
- **What was routed.** Three rows in the bar: `Local library`, `Read-only · another window has this
  library` (one row behind two painters — the wide bar's 10.5-point line and the narrow bar's glyph
  name), and the hover that explains the lock,
  `Close the other Sortlens window to edit here. Everything stays readable meanwhile.`
- **The keyword editor came with them**, because it is the same omission one level down: `Rename
  keyword`, `#{tag} appears on every photo it was added to.`, `That keyword already exists; the two
  will be merged.`, `Rename`/`Cancel` (rows it shared with the rename-collection dialog but never read)
  and both toasts — `Renamed #{tag} on {n} photo(s)` and `#{tag} is not used by any photo` — as `fmt`
  templates, the tag travelling as an isolated placeholder and the count in the language's digits.
- **What stays English, and says so.** The `Moved` receipt: its title, the sentence under it, its
  `and {n} more` line and its two buttons are still literals at the call site (the button rows exist for
  other dialogs, so that is a call-site route rather than new rows). UX.md and ROADMAP.md now name it as
  the canvas's remaining English instead of the top bar. The receipt's own body would need a real move to
  paint, which is why it was not folded into this slice's test.
- **Rows.** Eight rows in each of the six catalogues (`ar`, `ja`, `ko`, `th`, `vi`, `zh-TW`), appended
  after the filter-strip block. The catalogue audit's duplicate-row check passed on the first run.
- **Tests.** `the_top_bar_names_the_library_in_the_interface_language` paints the whole shell at
  1440×900: English shows `Local library`; Japanese shows `ローカルライブラリ` and no `Local library`;
  Arabic shows `مكتبة محلية`. `the_keyword_editor_speaks_the_interface_language` paints the dialog with
  `run_ui` (a warm frame first, then the frame that paints) with a `cat` tag armed: English pins the
  title, the tag sentence, `Rename` and `Cancel` character for character, Japanese pins
  `キーワード名を変更`, the sentence and both buttons and requires no English left, Arabic requires the
  placeholder inside its own sentence, and both languages assert the paint did not commit the rename.
  `catalogs_load_and_routed_keys_have_rows` now demands the eight new keys in all six catalogues.
- **Files.** `crates/ui/src/shell.rs` (the bar, `tag_edit_dialog`, two tests);
  `crates/ui/src/i18n/{ar,ja,ko,th,vi,zh-TW}.tsv` (eight rows each); `docs/UX.md`, `docs/ROADMAP.md`.
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; the two new tests and
  the catalogue audit green; `cargo fmt --all` left the tree formatted; `cargo xtask ci` exit code 0
  read from `$LASTEXITCODE` with the output in a log file (the pipe's own exit 1 is PowerShell treating
  cargo's stderr as an error; the log's last lines are the gate's `ok`s), `cargo xtask snapshot` green,
  `cargo xtask deny` green (`advisories ok, bans ok, licenses ok, sources ok`). English is
  byte-identical and no layout moved, so no baseline can change — `cargo xtask screenshots` was not spent
  on a set this slice cannot move. `cargo xtask mobile` cannot run here (no Android NDK, no Xcode) and
  this slice touches no phone code. No release workflow was dispatched: `crates/ui` source, catalogues,
  tests and docs.
- **Next.** The `Moved` receipt is the last English on the canvas: title, the "Ratings, labels and
  keywords travelled with the files." sentence, `and {n} more`, `Put them back` and `Done`. Its paint
  test needs a receipt, and a receipt needs one indexed photo and one real move, which is the fixture the
  next tick builds. Then the search field's `Search photos or Ctrl+K` hint and the bottom bar's
  `No selection`.

## 2026-11-26 (eightieth) — the Arabic frames repaint for the bar that learned its own language, and the reference set's rot is named

- **Why it was red.** `CI` on `d9df9d3` (run `45763073289`, job `139717404901`) failed
  `the_arabic_interface_paints_right_to_left_in_both_themes`: `lavapipe/rtl-library-{charcoal,paper}`
  differed by 182 and 183 pixels. The seventy-ninth entry routed `Local library` and skipped the
  screenshot gate on the claim that English is byte-identical — true, and irrelevant: the two RTL frames
  paint Arabic, and the bar's `Local library` is `مكتبة محلية` now. The diff image put every changed
  pixel inside that one word; everything else in the 1440×900 frame was identical.
- **Both sets, not just the red one.** The reference `rtl-library-*` frames were stale for the same
  reason, and `CI` never compares them. The harness repainted them on this machine: `cargo test -p
  sortlens-ui --features screenshots --test screenshots the_arabic_interface_paints_right_to_left_in_both_themes`
  wrote `.new.png` files that differed from the reference by 191 and 182 pixels, again only in the
  top-bar word; they were adopted and the same test then passed. The lavapipe pair came from the failing
  run's own `screenshot-set` artifact through the seventy-fifth entry's ritual: `gh run download
  45763073289 -n screenshot-set -D .ci-shots`, then `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask
  screenshots --adopt .ci-shots` (`adopted 2 frame(s)`). All four frames were read before they were
  committed, and the comparison stays byte for byte.
- **The committed reference set is stale, and this tick names it rather than repainting it silently.**
  `cargo xtask screenshots` on this machine after the fix passes the two rtl scenes and fails 23 others,
  and not by antialiasing: `filters-charcoal`'s committed frame predates the sidebar's QUICK ACTIONS
  block, and its FOLDERS/COLLECTIONS rows sit at heights the app left behind. `CI` only ever compares
  the lavapipe set, so the reference set rots unseen, and the last two UI commits did not run the local
  gate. Repainting 23 frames is a slice of its own — every frame has to be read — so it is the next
  tick's first job, and the rtl scenes, the only reference frames this machine's paint authored, are the
  only ones this commit moves.
- **Verified.** The rtl test green against the adopted reference pair; `cargo xtask ci`, `cargo xtask
  snapshot` and `cargo xtask deny` green. `cargo xtask screenshots` is red only on the 23 stale reference
  frames described above, which this commit does not touch — its lavapipe half is the runner's job on
  the next read. `cargo xtask mobile` cannot run here (no NDK, no Xcode); this slice changes no phone
  code. No release workflow was dispatched: four PNGs and this entry, no build or release surface.
- **Next.** Repaint the committed reference set and read every frame (the explicit plan is in the
  section below), then SL-L07's `Moved` receipt. No release run is in flight.

## 2026-11-26 (eighty-first) — the reference set is repainted at last, and the red trunk's two RTL frames

- **Why it was red.** `CI` on `ce56574` (run `45767097177`) concluded red in one job: `screenshots`. Its
  `screenshot-set` artifact held exactly two frames, `lavapipe/rtl-library-{charcoal,paper}.new.png` — the
  eightieth entry's fix was right, and the sidebar's new QUICK ACTIONS block and the search hint are now
  Arabic in that frame, so the lavapipe RTL baselines that predate them no longer match. Nothing else in
  that set moved, which is the useful half of the read: `CI` compares only the lavapipe set.
- **The committed reference set was stale by 47 of its 49 frames, and this commit repaints them.**
  `cargo test -p sortlens-ui --features screenshots --test screenshots -- --test-threads=1` wrote a
  `.new.png` beside every one of them; only the two RTL frames — adopted from the runner last tick —
  matched, which is why the failure count was 23 tests and not 29. Each pair was diffed to its
  changed-pixel bounding box before adoption, and every box is structural: the sidebar's QUICK ACTIONS
  block (x 0–300 in all 28 shell scenes), the Insights summary card (245k pixels, the largest delta),
  the Places pin labels, the shortcut editor's rows, and a 13,767-pixel band at x 700–1099, y 850–899
  that is the bottom bar's annotation pill — the same count in charcoal and paper to the pixel.
  Antialiasing noise from a different rasteriser is neither localized to one block nor identical in
  both themes, so these are the app's own paint and not this machine's.
- **Read before adopting.** Three full pairs: `library-justified-charcoal` (QUICK ACTIONS: Add Folder…,
  Start Cull, Export Picks…, Insights), `insights-charcoal` (the Summary card's ruler and percentages),
  and the runner's own `lavapipe/rtl-library-charcoal`, where the sidebar, the bar's count and the
  filter strip all speak Arabic and no English is left in them. The remaining 44 were read as their
  changed-region crops, each one inside the block its diff box named. The `.new.png` files were then
  promoted in place, so the six scenes that already matched keep their bytes.
- **Verified.** `cargo xtask screenshots` green afterwards: `29 passed; 0 failed; 1 ignored` (before
  adoption the same run was `6 passed; 23 failed`). `cargo xtask ci`, `cargo xtask snapshot` and
  `cargo xtask deny` green. `cargo xtask mobile` cannot run here (no Android NDK, no Xcode) and this
  commit touches no phone code. No release workflow was dispatched: 49 PNGs and this entry, no build or
  release surface.
- **Named defect — the next tick's first slice.** With QUICK ACTIONS in place at 1440×900, the sidebar's
  scroll clip ends mid-row: the keyword row under `sea` is cut through its text above the pinned foot.
  The fortieth entry's test asserts every painted text rect intersects its own clip rect, and a half row
  still does — the clip is legal and wrong. The fix is to clip whole rows only, and to extend that test
  so a row is either fully visible or not painted at all.
- **Next.** That sidebar clip, then SL-L07's `Moved` receipt (title, the sentence under it,
  `and {n} more`, `Put them back`, `Done`) with a paint test built on one indexed photo and one real move, then
  the search field's `Search photos or Ctrl+K` hint and the bottom bar's `No selection`. Even with the
  reference set current, `screenshots` on GitHub remains the only half this machine cannot prove — it
  compares the lavapipe set — so the next commit's run is read first, as always. No release run is in
  flight.

## 2026-11-26 (eighty-second) — the sidebar's clip ends on a whole row, at both edges and at any height

- **Why it came up.** The eighty-first entry named it as the next tick's first slice: the sections' scroll
  clip ends wherever the pinned foot leaves room, which is not a row boundary, so at 1440×900 the keyword row
  under `sea` was painted as the top half of its own text. The fortieth entry's sweep could not see it — it asked
  whether a painted text *intersects* its clip, and half a row does.
- **Shipped.** `whole_row(ui, height)` asks the question exactly: is the row at the cursor wholly inside
  `ui.clip_rect()`? `nav_row` and `nav_heading` ask it before they paint. A row that does not fit claims its
  space and paints nothing, so the sections keep their full extent and a wheel still reaches the rows under the
  fold; a skipped row is allocated with `Sense::hover()` instead of `Sense::click()`, because a sliver of a row
  is a target nobody aimed at. The rule is checked against the clip rather than a precomputed grid, so it holds
  at every panel height and at the top edge too, after a scroll.
- **Tests.** `the_sidebar_paints_whole_rows_and_never_half_of_one` paints the baselines' library (four photos in
  two folders, three keywords, a collection) through the whole shell at 1440×900 and asserts every painted
  sidebar text is *contained* in the clip it carries — at rest and after a settled wheel event. Two things keep
  it honest: the same library is painted at 1440×1600 and the short frame must paint fewer texts (so the rows
  really are being hidden), and the scrolled frame must paint a row the resting frame did not. It fails on the
  old paint both ways: at rest `COLLECTIONS` straddles the clip, after the scroll `QUICK ACTIONS` does.
- **Visual verification.** 40 reference frames repainted (all 29 scenes, both themes, plus high contrast). Before
  adoption every pair's changed-pixel box was computed, not eyeballed: each one is a single row's band in the
  sections' column — `18,706–244,709` at 1440×900 (42–87 pixels: the tops of the skipped row's glyphs),
  `10,598–252,609` at 1200×800, `18,417–246,427` at 900×620, and `1195,706–1419,709` in the RTL pair, mirrored
  with the sidebar. No frame changed outside that band. `short-sidebar-charcoal` and
  `library-justified-charcoal` were then read at 3×: the sections end on `Rated 8` and on `sea 2` respectively,
  the hairline and the four utility rows below them, and no cut glyphs. `cargo xtask screenshots`: 29 passed,
  0 failed, 1 ignored.
- **Docs.** ROADMAP 3B.6 (d) gained the revisit in the same commit, and UX.md §4.4 now says the clip ends between
  rows rather than through one.
- **Gates.** `cargo xtask ci` (the first run stopped at `cargo fmt --check`, fixed with `cargo fmt --all` and the
  full gate re-run), `cargo xtask snapshot`, `cargo xtask deny` green. No `cargo xtask wasm` exists in this
  workspace; `cargo xtask mobile` cannot run here (no NDK, no Xcode) and no phone code changed. No release
  workflow was dispatched: this is UI source and PNGs, no build or release surface.
- **Next.** Read this commit's `CI` run first. The committed `lavapipe/` set is now stale for the same 40
  scenes — this machine painted the reference set, and only the runner can paint the other — so the
  `screenshots` job is expected to hand over its `screenshot-set` artifact, which is adopted with
  `SORTLENS_SNAPSHOT_SET=lavapipe cargo xtask screenshots --adopt .ci-shots` after each frame's changed box is
  computed the same way. Then SL-L07's `Moved` receipt, the search field's `Search photos or Ctrl+K` hint and
  the bottom bar's `No selection`.

## 2026-11-26 (eighty-third) — the host's last notices speak the interface language, from the suggestion pass to remove-root

- **Why it came up.** The hundred-and-thirty-first entry left a named remainder on SL-L07's row: every
  notice `apps/sortlens/src/main.rs` builds that no shell dialog paints, and the two `rfd` titles still
  hard-coded in English. The shell's catalogue audit reads `crates/ui` only, so nothing would have caught
  these staying English in all six languages.
- **Shipped.** The suggestion pass (`Nothing to suggest: the library is empty`, `Looking at {n} photo(s)…`,
  stopping, cleared, and the finished `{n} photo(s) have something to suggest`), the pack and tag toasts,
  undo-move, the change report's title, its `Text` filter and its `Wrote {lines} line(s) to {folder}`
  receipt, remap's title and `Repointed {n} photo(s) to {folder}`, `Rescanning {folder}…`, both consent
  lines for models and updates, `Removed {n} photos from {folder}`, `Opened the containing folder…` and
  `Could not run {program}: {error}`, and the phone-only fallback all go through `i18n::tr`/`i18n::fmt`,
  with counts through `i18n::count`.
- **Reused, not re-worded.** The pack toasts route the change report's own rows (`Switched on/off the
  “{name}” keyword pack`) and the tag toast the shell's `Tagged “{tag}”`, so a toast and the report word the
  same event identically and a translator sees one row. Those three rows already existed; this tick adds 21.
- **Catalogues.** 21 new rows in each of ar/ja/ko/th/vi/zh-TW, each translation carrying the English
  template's placeholder names unchanged, so `I18n::check_all` stays empty.
- **Tests.** `the_hosts_suggest_and_repair_notices_read_the_interface_language` pins the English wording
  (`Repointed 3 photo(s) to sea`) and then requires Arabic to carry Arabic-Indic digits and no English noun
  for `Looking at`, `Put … back` and `Removed … from`, with both consent lines, `Text` and remap's title
  differing from English. `the_hosts_words_have_a_row_in_every_catalogue` grew to 46 listed routes; it
  already demanded a row for every routed key in all six catalogues, so a route added without one fails.
- **Visual verification.** No baseline paints a host notice or either native dialog, so none was expected to
  move, and none did: `cargo xtask screenshots` finished `29 passed; 0 failed; 1 ignored` against the
  committed set, so no PNG in this commit changed.
- **Gates.** `cargo xtask ci` green, `cargo xtask snapshot` green (`every_published_output_is_unchanged`),
  `cargo xtask deny` green (`advisories ok, bans ok, licenses ok, sources ok`) and `cargo xtask screenshots`
  as above. `cargo xtask wasm` does not exist in this workspace; `cargo xtask mobile` cannot run here (no
  Android NDK, no Xcode) and no phone code changed. No release workflow was dispatched: host source,
  catalogue rows and docs, no build or release surface.
- **What this leaves on SL-L07.** `apps/sortlens/src/sync.rs` still returns four English sentences as
  notices (`The server is already running`, `Stop the server before revoking a device`, `That device is not
  paired any more`, `Could not find this app: {error}`), and the engine's own error and `Op` description
  sentences stay the engine's English by design — the shell re-words what it paints, but a host notice that
  forwards `error.to_string()` shows those words. The four sync rows are the next tick's first slice; the
  engine's sentences are a data-layer question, not a catalogue gap, and the phone app is Phase 4 item 3.
- **Next.** Read this commit's `CI` run first (windows-latest is the slow leg; `screenshots` is the one that
  can disagree with this machine). Then the four `sync.rs` sentences through `i18n::fmt`/`i18n::tr`, added
  to the host's listing the same way. No release run is in flight.

## 2026-11-26 (eighty-fourth) — the server supervision's refusals speak the interface language

- **Why it came up.** The eighty-third entry left the last English the host builds itself: `sync.rs`'s
  refusals and missing-binary sentences, which the shell paints verbatim through `UiState::notice`
  (`Action::ServerStart` and `Action::ServerRevoke` hand it the `Err` untouched), so nothing in the
  shell's catalogue audit could ever see them.
- **Shipped.** `Sync::start`'s `The server is already running`, `Sync::revoke`'s `Stop the server before
  revoking a device` and `That device is not paired any more`, `locate`'s `Could not find this app:
  {error}`, `{path} was not found — build it with the rest of the workspace` and `the app has no
  containing directory`, and `format_day`'s `unknown` date all go through `i18n::tr`/`i18n::fmt`. The
  first of the two missing-binary rows was lower case in the code (`could not find this app:`); it now
  starts with a capital like every other notice, which is what the row's key says.
- **The audit now reads both host files.** `routed_keys()` in `main.rs` reads `main.rs` and `sync.rs`
  through `include_str!` and collects the routes from both, so a route added in either file with no
  catalogue row fails `the_hosts_words_have_a_row_in_every_catalogue` instead of painting English. The
  listing grew from 46 to 53 keys — the seven above — and all six catalogues carry a row for each.
- **Tests.** `the_device_list_is_the_token_file_and_revoke_needs_a_stopped_server` and
  `revoke_while_the_server_runs_is_refused_by_name` keep their English assertions and now also pin the
  refusals in Arabic: `That device is not paired any more`, `Stop the server before revoking a device`
  and a second `start`'s `The server is already running` must each differ from the English and carry
  Arabic letters. A literal left in `sync.rs` fails both, and `the_hosts_words_have_a_row_in_every_catalogue`
  fails if a catalogue falls behind. The running-server test now proves `start` twice is named rather than
  silently ignored, which no test asked before.
- **Visual verification.** None expected and none taken: no baseline paints a host notice or a device
  list's date, and this slice changes no shell source. `cargo xtask screenshots` was not run for that
  reason — the previous tick ran it against identical host-only changes and every frame matched its bytes.
- **Gates.** `cargo xtask ci` stopped at `cargo fmt --check` (a long `map_err` chain and two trailing
  newlines); `cargo fmt --all` was applied and the full gate re-run per the eighty-second entry's
  precedent. `cargo xtask snapshot` and `cargo xtask deny` green. There is no `cargo xtask wasm` in this
  workspace; `cargo xtask mobile` cannot run here (no Android NDK, no Xcode) and no phone code changed.
  No release workflow was dispatched: host source, catalogue rows and docs, no build or release surface.
- **What remains on SL-L07.** The engine's own error and `Op` description sentences stay the engine's
  English by design, because the shell re-words every one it paints. The phone's canvas chrome named in
  `FEATURES.md` is the row's open part; it is Phase 4 item 3's surface, not a catalogue gap.
- **Next.** Read this commit's `CI` run first (`screenshots` is the job that can disagree with this
  machine, and it compares the lavapipe set). Then the canvas chrome `FEATURES.md` names as SL-L07's
  open part — the areas the shell still paints from engine strings rather than catalogue rows — one
  surface per tick, each with a paint test. No release run is in flight.

## 2026-11-26 (hundred-and-thirty-third) — the rejects bin's rows are worded from the batch, and a mode id stops reaching the screen

- **Why it came up.** The hundred-and-thirty-second entry's Next named the shell surfaces that still
  paint engine strings. The `CI` run for `984b126` (`45784147625`) was in progress at the tick's first
  step and the five runs before it were green (`45782233764` and back), so this is a roadmap slice
  rather than a repair.
- **What was English.** `deleted_dialog` painted `DeletedBatch::summary()` — `2 photos · kept on disk ·
  1 failed` — over `format!("{age} · {mode}")`, where `mode` is the stored id (`trash`, `tag`,
  `move:Rejects`) and `age_label` was four English unit strings. A translated window therefore read the
  engine's English under a translated title, above a value the interface never names anywhere else.
- **What is routed.** `bin_row` joins both lines from the batch's own pieces: `{n} photo`/`{n} photos`
  through `i18n::fmt` over `i18n::count`, ` · {n} failed`, and the age through the routed `age_label`.
  The destination comes from `Receipt::of(batch, 0).destination()` — the same `Destination` the Batch
  applied receipt words — so `kept on disk` is one row shared by both, and `Moved to the system Trash`
  and `Moved to {folder}` are the receipt's existing rows. Five new rows per catalogue (30):
  `kept on disk`, `just now`, `{n} min ago`, `{n} h ago`, `{n} days ago`.
- **The age label is shared.** `age_label` paints the change viewer's timestamps as well as the bin's,
  so the viewer's ages became catalogue rows in the same edit and an Arabic viewer writes them in
  Arabic-Indic digits.
- **Tests.** `a_bin_row_speaks_the_interface_language` indexes two photographs, rejects both, applies a
  real tag-only batch and paints the dialog through `Context::run_ui` in English, Japanese and Arabic.
  English must read `2 photos` over `just now · kept on disk` and never `tag`; Japanese must read
  `写真 2 枚` and `たった今 · ディスクに保持` with no `photo` or `kept on disk` left behind; Arabic must
  carry `٢` and `بقي على القرص`. `age_label(now - 3_600)` and `(now - 7_200)` pin `1 h ago` and
  `2 時間前`, which the old literals could not be. `catalogs_load_and_routed_keys_have_rows` fails if
  any of the five rows is missing from a catalogue.
- **Visual verification.** `cargo xtask screenshots` exit 0 — 29 passed, 1 ignored, no baseline moved,
  which is the expected answer: no committed frame opens the bin with a batch in it (the bin's frame
  only ever paints its empty state) and no frame paints the change viewer's age.
- **Gates.** `cargo xtask ci` exit 0 on the first run (fmt, workspace clippy, the workspace suite
  including the catalogue audit and the new paint test, `layers`, `roots`, `assets`, `fonts`,
  `version`, `attribution`, `a11y`, `docs`, `templates`, `workflows`), then
  `cargo xtask snapshot`, `cargo xtask deny` and `cargo xtask screenshots` all exit 0. There is no
  `cargo xtask wasm` in this workspace; `cargo xtask mobile` cannot run here (no Android NDK, no
  Xcode) and no phone code changed. No release workflow was dispatched: shell source, catalogue rows
  and docs, no build or release surface.
- **Files.** `crates/ui/src/shell.rs` (`age_label`, the new `bin_row`, the dialog and the test),
  `crates/ui/src/i18n/{ar,ja,ko,th,vi,zh-TW}.tsv` (five rows each), `docs/UX.md`,
  `docs/ARCHITECTURE.md`, `docs/ROADMAP.md` and this file.
- **Next.** Read this commit's `CI` run first (`screenshots` is the job that can disagree with this
  machine). Then SL-L07's remaining open part as `FEATURES.md` states it: the phone host's own notices
  in `apps/sortlens-mobile` — `lib.rs`, `prefs.rs` and `send.rs` still build every status line in
  English, and the desktop's source-reading audit cannot see them — one surface per tick, each with a
  paint test. No release run is in flight.

## Next

Phase 0 is done. The queue below is the order work runs in, one vertical slice per commit, each with a test
that fails when the behaviour is absent, its ROADMAP row ticked and a CHECKPOINT entry in the same commit.
Gates every commit: `. scripts/env.sh && cargo xtask ci`, `snapshot`, `deny`, and `mobile` when `crates/core`,
`crates/ui` or phone-facing crates change (`mobile` needs the Android NDK and Xcode, so a machine without them
says so rather than skipping it silently). The full text is in `docs/ROADMAP.md` "Completion phases".

**Next tick (after the hundred-and-thirty-third entry).** Read that commit's `CI` run with one
`gh run view` — `screenshots` is the job that matters. Then SL-L07's remaining open part as
`docs/FEATURES.md` states it: the phone host's own notices in `apps/sortlens-mobile` (`lib.rs`,
`prefs.rs`, `send.rs`), which the desktop's source-reading audit cannot reach — one surface per tick,
each with its own source-reading test and catalogue rows. No release run is in flight.

**Phase 1 — core engine and media gaps**
1. **Video poster frames** (SL-B11 part 2) — **done 2026-11-22**, see the twenty-first entry.
2. Stale-revision rejection (SL-D11) + the stale-revision and badge-contrast tests that close M3 —
   **done 2026-11-23**, see the twenty-second entry.
3. `Recently Added` (SL-A14 part 2): `added` timestamp, snapshot migration, catalog/engine/sidebar —
   **done 2026-11-23**, see the twenty-third entry.
4. PNG export (SL-B24): licence-approved encoder, export dialog and CLI — **done 2026-11-23**, see the
   twenty-fourth entry.

**Phase 2 — real local AI tagging**
5. Pin real ONNX models (detector + CLIP/MobileCLIP) in `Manifest::shipped()` and `ATTRIBUTION.md` —
   **done 2026-11-23**, see the twenty-fifth entry.
6. `model.install`/`model.remove` consent-gated resumable downloader in `sortlens-engine`; Settings → AI —
   **done 2026-11-23**, see the twenty-sixth entry.
7. `tract-onnx` inference (SL-F03): the runtime, the detector path over 384 px previews, NMS +
   `suggest::merge` — **done 2026-11-23**, see the twenty-seventh entry. Part 2 (3b) — per-pack text
   embeddings cached at install (CLIP BPE tokenisation) and image embeddings, matched through
   `suggest::merge` — **done 2026-11-23**, see the twenty-eighth entry; the real downloaded pins were
   loaded and run once as that slice's first gate.
8. Tag provenance (SL-D12) with snapshot migration; filter by source in UI and CLI —
   **done 2026-11-23**, see the twenty-ninth entry.

**Phase 2 is complete.** Phase 3 starts here, in this order:

9. **`egui_kittest` screenshot harness + `cargo xtask screenshots`**, Charcoal and Paper baselines for Empty
   Welcome, Library (Justified and Square), Cull, Detail, Compare, Similar, Places and Insights — **done
   2026-11-23**, see the thirtieth entry. 18 baselines; `egui_kittest` does exist at the fork revision, so no
   fallback rig was needed. It found the unnamed sort combo and the missing arrow glyphs (both fixed) and
   recorded three visual defects under 3B.6.

**Phase 3 — UI/UX overhaul**
9. `egui_kittest` screenshot harness, `cargo xtask screenshots`, Charcoal and Paper baselines (do this before 10–15)
   — **done 2026-11-23**, see the thirtieth entry.
10. `WidgetPalette` mapping + monochrome SVG icon set — **done 2026-11-23**, see the thirty-first entry:
    34 glyphs in `assets/icons/` (36 after 3B.2), `crates/ui/src/icons.rs`, wired into all five surfaces, and
    `Tokens::palette()` is the one widget-state mapping. The four value chips in the top bar (theme, density,
    layout, motion) are still words on purpose: they print a value, and 3B.2's Appearance menu is where they move.
11. **Top bar declutter and the `⌘K` command palette — done 2026-11-23**, see the thirty-second entry:
    21 palette rows, every one a `Ready` registry command; the field switches mode in place; the bar's eight
    glyphs; the Filter badge and toggle; Appearance as one menu; density/layout in the canvas header; 36 glyphs;
    20 baselines including the two new palette scenes. Named as not shipped there: the app-mark library menu
    and view history (the UX table used to imply both).
12. **Sidebar scopes and the utility strip (3B.3) — done 2026-11-23**, see the thirty-third entry: six
    scopes with live counts (Missing is new, backed by `Filter::missing`), and the four dialog rows moved to an
    exactly-sized foot whose words and handlers are the palette's. Named as not shipped there: the per-row
    "filtered" marker and the footer's storage summary.
13. **The bottom bar (3B.4) — done 2026-11-23**, see the thirty-fourth entry: nine destinations with a measured
    narrow-window collapse, the annotation pill, the size slider and zoom/`?` controls, and three new `Ready`
    view commands.
14. **The right panel (3B.5) — done 2026-11-23**, see the thirty-fifth entry: `RightTab::ALL` is four with
    Suggestions between Tags and Activity, `right_tools` is the 30-point far-edge rail that is painted whether the
    body is open or not, the tab row is gone and a header names the active tool. Proposals are grouped into
    Objects / Scene / EXIF for the whole selection, with confidence, partial-selection counts, `Accept all` per
    family and `×` per row; accepting a family is one ⌘Z (`Engine::accept_suggestions` over one `Op::Group`) and
    dismissal is a stored, undoable `Op::SuggestionDismissed` with `suggest.dismiss` in the registry, an MCP
    example and a CLI verb. Two new baselines plus all 25 regenerated and read. Named as not shipped there: the
    rail's 14-point glyphs in 24-point rows are a mouse target, not a touch one, and `Accept all` beside a long
    family name is unverified at the panel's 250-point minimum.
15. **The visual defects (3B.6), in this order.** (a) the Welcome bullets are `•` text where they should be the
    icon set's own glyphs — **done 2026-11-23**, see the thirty-sixth entry: `folder`/`history`/`reject` in one
    16-point column, an exact-count `run_ui` test, two baselines regenerated and read. (b) the strip on an empty
    library, and its flat-text controls — **done 2026-11-23**, see the thirty-seventh entry: hidden while the
    catalog is empty (a filter that hides everything keeps it), stars as glyphs, the label's own colour as a
    swatch, the state's own flag, a chevron on the keyword menu, and a wrapping row that found `Clear` was
    unreachable at 1200 px with both docks open. (b') the canvas under a filter that hides every photograph —
    **done 2026-11-23**, see the thirty-eighth entry: `canvas_empty` splits the catalog's total from the filtered
    count, so a narrowed-to-zero library paints *Nothing matches these filters*, the library's own count and one
    `Clear filters` button, an actually-empty catalog still paints the Welcome card, and `Not now` finally stays
    dismissed. Two baselines at 1200 px read. (c) Places ships the offline coastline basemap and falls back to the
    user's `tiles/` and `places.txt`, and opens fitted instead of at zoom 5 — **done 2026-11-23**, see the
    thirty-ninth entry: the Natural Earth 1:110m outline embedded in `sortlens-geo` with its credit on screen, split
    at the antimeridian and projected once, `bounds_centre`/`fit_zoom` replacing the fixed zoom and the mean centre,
    a `tiles/` folder still winning, a chosen zoom never overwritten, seven geo tests, two shell tests and both Places
    baselines read. (d) the sidebar's pinned foot overlapped the keyword and collection rows above it at ~620 px
    — **done 2026-11-23**, see the fortieth entry: the sections live in one `ScrollArea` sized to the room the foot
    left, a test intersects every painted text rect with its own clip rect so "visible" means visible, a wheel event
    proves the rows under the fold are reachable while the foot stays put, and `short-sidebar-charcoal`/`-paper` were
    added and read. (h) the filter strip's `N shown` summary was painted under the `Clear` chip at 1280 px with
    both docks open — **done 2026-11-23**, see the forty-first entry: the count is measured first and given a
    reserved column on the right, the chips wrap in the room that is actually theirs, and a test sweeps the strip
    from 320 to 760 points a step at a time asserting no painted text ever touches the count (it fails at 326 on
    the old layout); all 29 baselines regenerated and read. (e) the canvas header's title ran under the
    right-aligned sort combo at 900 px with both docks open — **done 2026-11-23**, see the forty-second entry: one
    `Sides::shrink_left` row that measures the controls first, a count that steps aside when it will not fit, chips
    offered only above 430 points, a 401-width sweep that fails at 900 on the old layout, and two new
    `narrow-canvas-*` baselines at 900×620 read. (f) the cull keyword row clipped under the bottom bar at 900 px —
    **done 2026-11-23**, see the forty-third entry: the block under the photograph is a `Panel::bottom` with an exact
    height claimed before the canvas, `cull_foot_height` is arithmetic over fixed-height rows with a measured live
    line, the keyword band is two chip rows that scrolls so sixty keywords cannot resize the photograph, the two live
    lines became one, and a test at 1440×900 asserts every cull text is visible inside the column and above the bar.
    **(g) is done** — see the forty-fourth entry: the arrows are triangles in both faces, measured, tested twice
    and read in new baselines. (g) was *observed*, so it is "the baseline is clean and the geometry test says so"
    and it is.

**Phase 4 — platform, keymap, remote**
16. `craft-keymap` editor in Settings → Shortcuts — **done 2026-11-24**, see the forty-fifth entry.
17. Platform finishing, in this order — **2a `.icns`/`.ico` via `cargo xtask icons` done 2026-11-24**, see
    the forty-sixth entry; **2b the High-Contrast preset (SL-L09) done 2026-11-24**, see the forty-seventh
    entry; **2c the opt-in `craft-update` (SL-I10) done 2026-11-24**, see the forty-eighth entry; **2d the
    `craft-i18n` scaffolding (SL-L07) done 2026-11-24**, see the forty-ninth entry.
18. `sortlens-server` + the phone client with a two-process loopback integration harness in CI — the
    server half, the harness and the desktop's start/stop surface are **done 2026-11-24** (fiftieth and
    fifty-first entries); the in-memory catalog path a paired device needs is **done 2026-11-24**
    (fifty-second entry); the device-side protocol, `library.snapshot` on the wire and the client's
    keep-or-remove ack are **done 2026-11-24** (fifty-third entry); the server's thumbnail route and
    the client's `thumbnail` fetch are **done 2026-11-24** (fifty-fourth entry); the native
    `apps/sortlens-mobile` was added the same day (fifty-seventh entry). Its settings persistence is **done 2026-11-24** (sixty-fifth entry). The phone's frame — a window under 600 points put away both docks, wordless bar, one full-width sheet at a time — is **done 2026-11-24** (sixty-sixth entry). The canvas at that width — the far-edge rail standing down, a two-row filter strip, a tray whose every control is on screen — is **done 2026-11-24** (sixty-seventh entry). The send half, the pairing's connection test and the phone's own screens shipped the same day (sixty-eighth and sixty-ninth entries), and the phone's embedded font set is trimmed to the shipped languages' scripts (seventieth entry). The release workflow now builds and attaches the Android APK (seventy-first entry), and the phone trash that leg exposed is a library-folder move (seventy-second entry). What remains of the row is the iOS Photos library, a device build, the `.ipa` (Apple identity) and an AAB — all named with their unblockers in the 3b row itself.

**Phase 5 — autonomous GitHub maintenance**
19. Issue/PR templates — **done 2026-11-24**, see the fifty-eighth entry.
20. `docs/AUTONOMOUS_AGENT.md` — **done 2026-11-24**, see the fifty-ninth entry.
21. `agent-intake.yml` and a PR-gating `ci.yml` (`ci`, `snapshot`, `deny`, `mobile`, `screenshots`) —
    **done 2026-11-24**, see the fifty-ninth entry. **Phase 5 is complete.**

Stop condition: every row under "Completion phases" ticked. Never move a buildable feature into a
"Deliberately absent" table to close a milestone.

Note for the next run: this checkout builds on Windows (x86_64, `x86_64-pc-windows-msvc`) with its own
toolchain in `.toolchain` — `. scripts/env.sh && cargo xtask ci` works as documented, and so do
`snapshot`, `deny` and `screenshots` (23 scenes, green as of the sixty-seventh entry; the two phone scenes at
390×780 are in that count). There is no `cargo xtask wasm` in this workspace, whatever a plan may ask for. `cargo xtask
mobile` cannot run here: there is no Android NDK and no Xcode, so both phone targets stay *unverified on a
device* rather than silently skipped — that is why the roadmap's 3b still names it as open. On a Mac the
screenshots are a separate known problem: a Mac without a craft-faces symbols font paints `⌘Shift+R`
instead of the ASCII baseline and makes `craft-keymap`'s editor scene panic on `⇧` (fifty-sixth entry), so
making the shots a Mac gate means the harness embedding the symbol faces and a second baseline set.
