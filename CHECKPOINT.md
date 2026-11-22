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

## 2026-11-21 — M3: keyword picker and ordering

- catalog: `TagOrder { Count, Name }` with `flipped()`/`label()`; `tag_vocabulary_in(order)` and the
  existing `tag_vocabulary()` (count order) delegating to it
- engine: `tag_vocabulary_in(order)`
- ui: a "+ keyword" menu in the filter strip with a search box and a ✓ on active keywords, and a
  small toggle in the Keywords header switching the list between most used and A–Z
- tests: ordering by use and by name agreeing on counts, `flipped`
- 155 tests, CI and wasm green

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
- 161 tests, CI and wasm green; CLI smoke test over a two-photo library

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
- 177 tests, CI and wasm green

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
- 181 tests, CI and wasm green

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
- 182 tests, CI and wasm green

## 2026-11-21 — M4: gesture culling

- cull: `gesture::classify(delta)` — a drag counts only past 84 points and only when the vertical
  travel dominates by 1.4×, so a sideways scrub never flags a photo; up is keep, down is reject.
  `progress` clamps toward the threshold for the overlay, `Gesture::decision`/`label` for staging
- ui: the cull canvas senses drags, tints and labels the photo while the finger is down (with a hint
  when the drag is too short), stages the decision on release and clears the drag state
- tests: short drags doing nothing, direction choosing the decision, sideways and diagonal drags
  ignored, progress clamped to 1
- 186 tests, CI and wasm green

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
- 192 tests, CI and wasm green

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
- 196 tests, CI and wasm green

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
- 197 tests, CI and wasm green

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
- 201 tests, CI and wasm green

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
- 205 tests, CI and wasm green

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
- 208 tests, CI and wasm green

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
- 209 tests, CI and wasm green

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
- 211 tests, CI and wasm green; CLI smoke test renamed two photos

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
- 214 tests, CI and wasm green

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
- 215 tests, CI and wasm green

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
- 223 tests, CI and wasm green

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
- 225 tests, CI and wasm green. Verified by hand over the CLI, including the refusal path.

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
- 231 tests, CI and wasm green. Verified by hand: audit on a clean library, a damaged snapshot
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
- 237 tests, CI and wasm green. Verified by hand: history across three CLI processes and an undo
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
- 239 tests, CI and wasm green. Verified by hand with an OS lock held by another process: writes
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
- 242 tests, CI and wasm green. Verified by hand: `retry`, `retry --all` updating two photos, and undo
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
- 243 tests, CI and wasm green. Verified by hand: `changed` reporting one file by name, `refresh`,
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
- 247 tests, CI and wasm green. Verified by hand over the CLI including the sidecar travelling both
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
- 249 tests, CI and wasm green

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
- 255 tests, CI and wasm green

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
- 259 tests, CI and wasm green

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
- 263 tests, CI and wasm green. Verified by hand: rename, retype the old name, registry showing
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
- 268 tests (5 keymap tests), CI and wasm green

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
- 278 tests (8 in `sortlens-memories`), CI and wasm green. Verified by hand: two dates set, three
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
- 285 tests (6 in `charts`), CI and wasm green

## 2026-11-21 — M7: show a file to the desktop

- new `crates/core/src/reveal.rs`: the two OS actions as **pure command construction** —
  `Desktop { MacOs, Windows, Linux }` with `current()`, `reveal()` (`open -R` / `explorer /select,` /
  `xdg-open <folder>`), `open_with_default()` (`open` / `cmd /c start ""` / `xdg-open`) and
  `reveal_selects()` so a caller can tell the truth about Linux. Arguments are always a list, so a
  file called `$(rm -rf ~).jpg` is one argument and never a command; six tests pin the exact argv,
  including the empty `start` title argument Windows needs
- **the split is the design**: the engine builds the command, the host runs it. `photo.reveal` and
  `photo.open` return `{program, args, line, ran: false}` — a library opened by the wasm client has no
  process table, so a command that claimed to have run would be a lie. `Engine::reveal_target` and
  `path_of` do the resolution and refuse a file that is no longer on disk with "rescan or relocate"
- app: `Action::Reveal` / `Action::OpenExternal` spawn the built command and report the line (or the
  desktop's inability to select a file on Linux) in the status bar
- ui: "Show in folder" and "Open" in the grid's context menu and in the detail header, next to Close
- cli: `reveal <id>` and `open <id>` execute the same command
- docs: `docs/BEHAVIOUR.md` gains "Showing a file to the desktop" — who builds, who runs, why
  arguments are a list, the Linux folder deviation, and that nothing here mutates the catalog
- 292 tests, CI and wasm green. Verified end to end: the CLI ran `open -R /tmp/sl-rev/ph/a.jpg` (Finder
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
- 298 tests, CI and wasm green. **Not verified by hand**: a real MCP client (Claude Desktop et al)
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
- 302 tests, CI and wasm green. Verified against the real binary: `tools/list` returns 66 tools, all
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
- 304 tests, CI and wasm green. Verified by hand: `page --sort rating` returned the five-star photo
  first. **Not verified by hand**: the palette entry for "Read a Page" (the UI does not call it; the
  grid uses `Engine::page` directly)

## 2026-11-21 — M0 debt: craft-fonts, optional by design

- `crates/ui/build.rs` reads `CRAFT_FONTS_DIR` and writes a `CRAFT_FONTS: &[CraftFont]` table, with
  **unset as a supported configuration** — CI has no checkout, so the empty table is the configuration
  under test. A wasm build keeps an explicit five-face allowlist (BIZ UDPGothic Regular/Bold,
  NanumGothic, Noto Sans Symbols 1 and 2) because the whole manifest is ~48 MiB; a native build embeds
  all 21 faces.
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
- 307 tests, CI and wasm green in both configurations (with and without `CRAFT_FONTS_DIR`)
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
- 336 tests, `cargo xtask ci` green, `cargo xtask wasm` green.

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
- 362 tests, `cargo xtask ci`, `wasm`, `deny` and `snapshot` green.

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
- 385 tests, `cargo xtask ci`, `wasm`, `deny` and `snapshot` green. No new dependency: this is still a
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
- ROADMAP's SL-F05 row is ticked; 395 tests, `cargo xtask ci`, `wasm` and `deny` green.

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
- 400 tests; `cargo xtask ci`, `wasm`, `deny` and `snapshot` green.

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
- 413 tests; `ci` (exit 0), `wasm`, `deny`, `snapshot` green.

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
  no click was simulated. 416 tests, ci (exit 0), wasm, deny, snapshot green.

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
- 417 tests; ci (exit 0), wasm, deny, snapshot green. **Not verified by hand**: any of this in a real
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
- M9's remote rows (server, wasm client, remote album edits) moved to a "Deliberately absent from M9"
  table: they need a second device and a network peer, and an untested auth path is worse than none.
- 422 tests; ci (exit 0), wasm, deny, snapshot green. **Not verified by hand**: the report as pasted into
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
- 423 tests; ci (exit 0), wasm, deny, snapshot green. **Not verified by hand**: the workflow itself — it
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
- 423 tests; ci (exit 0), wasm, deny, snapshot green. **Not verified by hand**: nothing here is visual.

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
- 425 tests; ci (exit 0), wasm, deny, snapshot and `a11y` green. **Not verified by hand**: how any of it
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
- 427 tests; ci (exit 0), wasm, deny, snapshot, a11y green. **Not verified by hand**: nothing in this
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
- 432 tests; ci (exit 0), wasm, deny, snapshot green. **Not verified by hand**: how the icon looks in a
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
  `crates/ui/src/lib.rs`, `docs/ROADMAP.md`. 436 tests; ci (exit 0), wasm, snapshot green.
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
- 440 tests; ci (exit 0), wasm, snapshot green. **Not verified by hand**: a real camera raw whose header is
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
  `crates/ui/src/shell.rs`, `docs/ROADMAP.md`. 449 tests; ci (exit 0), wasm, snapshot, deny green.
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
  wasm, snapshot green. **Not verified by hand**: how the three steps feel on a real display — the numbers
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
- 458 tests; ci (exit 0), wasm, snapshot green. **Not verified by hand**: the gestures with a real mouse —
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
- 460 tests; ci (exit 0), wasm, snapshot green. **Not verified by hand**: how the band feels while dragging a
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
- 465 tests; ci (exit 0), wasm, snapshot, docs green. **Not verified by hand**: how the row heights feel while
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
- 469 tests; ci (exit 0), wasm, snapshot green. **Not verified by hand**: whether the priority actually feels
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
- 473 tests; ci (exit 0), wasm, snapshot green. **Not verified by hand**: where the popover lands against a
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
- 479 tests; ci (exit 0), wasm, deny, snapshot, attribution green. **Not verified by hand**: against a real MCP
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
- 482 tests; ci (exit 0), wasm, deny, docs, attribution green. **Not verified by hand**: a real iPhone HEIC —
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
  features are off, and reporting them with no length would be worse than reporting nothing. On wasm there is no
  filesystem to open, so a clip has no length there — named in the code, not silently zero.
- **Fixture without a blob**: the test muxes a real MP4 with the suite's own writer — 24 samples of 25 ticks at
  600 Hz is one second at 24 fps — and asserts duration, size, codec and rate, then truncates the same bytes and
  asserts the probe refuses half a file, and points it at a JPEG and asserts it is not a video.
- Goldens changed by exactly the three new JSON keys plus the log byte counts; the diff was read before the
  update, not accepted blind.
- 484 tests; ci (exit 0), wasm, deny, docs, snapshot, attribution green. **Not verified by hand**: a real camera
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
- 487 tests; ci (exit 0), wasm, deny, snapshot, docs green. **Not verified by hand**: whether a 128 tier scaled to
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
  example. 489 tests; ci (exit 0), wasm, deny, snapshot, docs green.
- **Not verified by hand**: the notice's tone on screen, and whether `T` collides with any OS-level shortcut on
  a real keyboard layout (the keymap gate only proves the registry's internal uniqueness).

## Next

M3–M7 pass, next unticked row that needs no absent prerequisite. Good candidates, in the order I would take
them: `Offline/missing part 2` (cached-thumb behaviour when a drive leaves — needs a decision about what a cell
shows, not a missing dependency) and `--json` write verbs (M9). Video posters stay named absent; the map, AI
inference, i18n and the keymap editor rows each carry their own blocker.
