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

## Next

M7: side-by-side compare of two photos, with the same viewport for both.
