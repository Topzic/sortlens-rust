# Checkpoint

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
`cargo xtask wasm` green; CLI smoke test round-trips through disk.

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
- 60 tests; `cargo xtask ci` and `cargo xtask wasm` green (the dispatcher builds for wasm)

Still missing: RAW embedded previews (`craft-raw`), video posters, HEIC.

## 2026-11-21 — M2: raw previews + shared media vocabulary

- core: `media` module owns the extension tables (raw/image/video/sidecar), `kind_from_extension`,
  `is_raw_path`; the scanner now delegates instead of keeping a second copy
- thumbs: `decode_raw_preview` pulls the largest embedded preview with `craft-raw` and decodes it
  through the ordinary pipeline; `decode_file` prefers it for raw extensions and falls back to the
  plain decoder for mislabelled files
- tests: raw-without-preview, mislabelled .cr2 that is really a JPEG, extension tables
- 65 tests; CI and wasm green

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
- 72 tests, CI and wasm green

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
- 77 tests, CI and wasm green; CLI smoke test across separate processes confirms the tags persist

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
- 90 tests, CI and wasm green; CLI smoke test produced an `xmllint`-parseable sidecar

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
- 94 tests, CI and wasm green; CLI smoke test imported a rating-4 / pick / keyword sidecar

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
- 110 tests, CI and wasm green

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
- 112 tests, CI and wasm green

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
- 118 tests, CI and wasm green

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
- 130 tests, CI and wasm green (`trash` is a desktop-only dependency; the browser build keeps
  tag-only)

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
- 132 tests, CI and wasm green

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
- 136 tests, CI and wasm green; CLI smoke test moved a file, saw `1 of 2 missing`, relocated it and
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
- 141 tests, CI and wasm green

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
- 144 tests, CI and wasm green; CLI smoke test listed `day1/` (2), `day2/` (1) and `day1/sub/` (1)

## 2026-11-21 — M3: sort keys

- catalog: `Sort::ALL`, `id()`, `label()` and `parse()` (unknown ids fall back to added order)
- ui: the header's static "Added order" label became a combo box listing added order, name, newest,
  oldest, largest and highest rated; the grid and the cull queue both use the chosen order, so a
  session starts in the order the user was looking at
- app: the order is saved to `library.sort` and restored on launch (`Action::SetSort`)
- tests: ids round trip and are unique, unknown ids fall back, settings default is "added"
- 146 tests, CI and wasm green

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
- 149 tests, CI and wasm green; CLI smoke test indexed a photo with a `crs:` sidecar

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
- 150 tests, CI and wasm green
- `view.detail` is now a Ready command

Not built: the lookahead strip and the histogram in the detail view.

## 2026-11-21 — M4: histogram and lookahead

- thumbs: `Histogram` (64 luminance bins, Rec. 601 weights) with `from_thumbnail`, `total`, `peak`,
  and `histogram_file` which decodes a small copy and measures it
- ui: `Thumbnails::histogram(photo)` caches one histogram per content key (decoded at 256 px on
  demand, never by the grid); the detail view draws it as 64 bars scaled to the tallest bin and adds
  a lookahead strip of ±1/±2 neighbours that can be clicked
- tests: every pixel counted with black and white in the end bins, a mid grey landing in the middle
- 152 tests, CI and wasm green

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
- 154 tests, CI and wasm green; CLI smoke test listed both photos for `beach` and one for `sunset`

## Next

Filter pickers and sort of the keyword list: choosing a keyword from a menu rather than the sidebar,
and ordering keywords by name or count.
