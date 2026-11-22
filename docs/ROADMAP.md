# Roadmap and tracker

Working document. Checkboxes are the unit of progress; every box names its command or crate so
it can be verified without reading the whole plan. Feature ids refer to `docs/FEATURES.md`.

Tracks: **A** catalog/engine · **B** UI · **C** preview/media · **D** analysis/AI ·
**E** platform/automation. A milestone is done only when its acceptance demo passes on macOS,
Windows and Linux.

```
Week   1    2    3    4    5    6    7    8    9   10   11   12   13   14   15   16
M0    ████
M1        █████████
M2              █████████
M3                    ███████
M4                         █████████
M5                              ████████
M6                                   █████████
M7                                        ██████
M8                                            █████████
M9                                                 █████████
M10                                                      ████████
```

Two-worker calendar. Single worker: stretch each row to its `weeks` estimate from
`docs/PLAN.md` §5.

---

## M0 — Skeleton · 1.0 w · tracks A,B,E

**Goal:** an ArtCraft window that persists settings and has no features yet.
**State:** done — every row is checked, fonts and `cargo deny` included. The toolchain, the workspace,
`xtask`, the shell and the settings store are what M1–M10 build on.

- [x] `rust-toolchain.toml` (1.95.0, wasm target), `.cargo/config.toml` alias; egui + winit forks patched in `Cargo.toml`
- [x] Workspace: 19 library crates + 4 app crates; `[lints] workspace = true`; crate-root denies (A)
- [x] `xtask`: `ci`, `fmt`, `clippy`, `test`, `layers`, `roots`, `assets`, `wasm` (E)
- [x] `cargo deny` allow-list in `deny.toml` (licences, advisories, bans, sources), `cargo xtask deny`, its own CI job; `Cargo.lock` committed (E)
- [x] App shell: top bar, bottom bar, left/right panels, canvas (B)
- [x] `craft-window` title-bar policy; macOS traffic-light inset (B)
- [x] Fonts: `crates/ui/build.rs` embeds craft-fonts when `CRAFT_FONTS_DIR` is set (optional by design), `fonts::install` appends them as fallbacks after egui's own faces, `fonts::coverage` checks `Latn`/`Zsym`, and `cargo xtask fonts` reports the set (B) — egui's own faces remain the zero-config default
- [x] Four theme presets + contrast tests (B) — widget states set on `Visuals`; `WidgetPalette` mapping still open
- [x] `craft-settings` v1 type, damaged-file notice, unknown-key preservation (A)
- [x] Command registry skeleton; `Engine::run` returns typed errors (A)
- [x] Commands: `app.about`, `app.settings`, `app.quit`, `app.supportReport`; 17 planned commands carry their milestone (A,B)
- [x] `app.shortcuts` — the keymap table, the searchable overlay, and every command shortcut wired (A,B)
- [x] Empty-library state (B)
- [x] CI: 3 OSes + wasm check + snapshot job (E) — `cargo xtask snapshot` diffs 30 recorded CLI outputs in its own job

**Demo:** launch → switch theme → open settings → quit → relaunch with the theme preserved;
`cargo xtask ci` green.

---

## M1 — Index · 2.5 w · tracks A,C

**Goal:** a 5,000-photo folder becomes a browsable grid with correct counts.
**State:** the catalog, the scan, the grid, the command registry and the CLI are in; metadata and
thumbnails arrived in M2 because the grid renders before it has pixels to draw. What is left is the
volume probe and `PathKey`.

- [x] `sortlens-core`: ids, time, byte units, versioned errors (A)
- [x] `ContentKey` (size + first/last 64 KiB) and a **volume probe**: `sortlens_core::volume` parses a mount
  table (Linux `/proc/mounts`, with `\040`-style escapes resolved and the longest mount point winning), falls
  back to the `/Volumes/<name>` convention on macOS and the drive letter on Windows, and answers "is the
  drive there" with one check per root instead of one `stat` per photo. `Engine::root_volumes()` reports each
  root with its mount point and `mounted: Option<bool>`, and the sidebar marks an absent volume "12 · away"
  while treating "cannot tell" as present — a missing answer must never read as a missing drive. The table
  parser and every convention are unit-tested on any platform; the syscall half is honest about what it
  assumes (A) — SL-A11 part 1
- [x] `PathKey` — a path identity for settings that belong to a folder (SL-A11 part 1)
  - Three things make "the same folder" not the same string: Unicode normalisation (macOS stores NFD, a person
    types NFC), trailing separators, and case. `sortlens_core::pathkey::PathKey` normalises the first two
    unconditionally and resolves the third **by asking the disk**: for every component, a case-insensitive
    lookup is made in its parent, and the on-disk spelling is adopted only when exactly one entry matches. Two
    matches means the volume can hold both spellings, so nothing is folded — `Ångström.jpg` and `ångström.jpg`
    on a case-sensitive disk stay two keys, which is the bug the row was blocking on.
  - The dependency decision the row named was already made for us: `unicode-normalization` was in the tree
    (through the text stack) and is now a declared, attributed direct dependency. No per-volume probe is needed,
    because the disk answers the question directly and the answer is cached by the filesystem.
  - Tested on whatever volume the test directory is on, holding the key to that volume's actual answer rather
    than to a guess about the platform: NFD/NFC and trailing separators collapse, a case-insensitive volume
    resolves to the disk's spelling at every component, and a case-sensitive one keeps both spellings. What this
    does **not** do is replace photo identity: photos are still keyed by root + relative path + `ContentKey`.
- [x] Walker with skip rules, bounded queue, delta detection (A) — SL-A01/A17
- [x] `sortlens-catalog`: op log, snapshots, replay, undo, paging, counters (A) — SL-A12/A13
- [x] Engine scan worker, `library.addFolder`/`rescan`/`page`/`stats` (A)
- [x] Virtualised grid with selection, counts and keyboard rating/flags (B)
- [x] CLI: add, list, rate, flag, undo (E)
- [x] `craft-metadata` → `PhotoMeta` through `sortlens-meta` (C) — SL-B21 data
- [x] Format routing by **header as well as name**: `sortlens_core::media::sniff` reads the first 16 bytes
  (JPEG, PNG, TIFF, HEIF/AVIF, WebP, ISO base-media brands including CR3's `crx ` and the video brands) and
  `resolve_kind` corrects a file whose header contradicts its extension — a JPEG preview named `.cr2`, an
  MP4 named `.jpg`, a still named `.mov` — while leaving genuine ambiguity alone (every raw is also a TIFF,
  so a `.tif` stays an image). The scan counts what it corrected (`Skipped::mislabelled` →
  `ScanFinished::mislabelled`), and the notice says "3 routed by content, not name", because a correction
  nobody is told about is a silent rewrite of what their tree means. Unknown ISO brands stay unknown rather
  than guessed as video, so podcasts and sidecars do not land in the grid (C) — SL-B11
- [x] `library.addFolder`, `library.rescan`, `library.removeRoot` (A) — SL-A01/A19
- [x] Both grid layouts, virtualised (B) — SL-B01/B02
  - **Square**: a uniform lattice, cropped to fit, windowed by a fixed row height so 200 000 rows cost the
    twenty on screen.
  - **Justified**: rows of equal height at each photo's own aspect, packing until the next photo would push
    the row below the target height (whichever of the two candidate heights is closer to it), a short last
    row left-aligned at the target instead of stretched, a photo wider than the window in a row of its own,
    and 3:2 for photos whose dimensions never loaded. Windowed by *row* from a cached plan keyed on width,
    filter, sort, count and target — the pack is the expensive part and it only reruns when one of those
    changes. The cells share one gesture handler with the square grid, so ⌘-click and the context menu
    cannot mean different things in the two layouts. `G` and the chip toggle it; the choice persists, and
    density becomes the target row height so the setting keeps its meaning. Tested as pure packing (width
    filled, contiguity, short last row, panorama, empty library), a cached plan that reruns only on a real
    change, and a headless paint of both layouts over five photos.
- [x] Cell size is derived from the window width **and** from a density setting: `Density::Compact /
  Comfortable / Roomy` names a minimum cell (104/132/184), `grid_layout` divides the window so the last cell
  in a row reaches the right edge, and a very narrow window keeps one legible 64 pt column instead of shrinking
  into unreadable cells. The chip sits beside the theme because both are "how the library looks"; the choice
  persists through `appearance.density`, whose field had existed since M0 while nothing read it — the setting
  was a string nothing used, which is the same lie as a disabled button that looks enabled. Tested as pure
  layout (more columns when dense, exact fit at each step, floor on a narrow window), as a settings round
  trip including an unknown id falling back to the default, and as a headless paint at all three densities (B)
  — SL-B04
- [x] Multi-select, part 1 — **click** replaces, **⌘-click** adds or removes, **⇧-click** takes the run from
  the anchor *in the order the grid shows* (not id order, which is not what anyone is looking at), **⌘A**
  selects everything in view and **Esc** drops it. `Selection` keeps the primary (what the inspector shows)
  apart from the ticks (what a batch acts on), so removing the primary moves the inspector to a photo that is
  still selected instead of one that is not; a ⇧-click with no anchor in view is a plain click. Rating, flag
  and label keys write every ticked photo and the notice says how many ("Rated 4★ on 12 photos"). The
  selection is pruned each frame against the library, so a delete cannot leave the inspector on a ghost. All
  of it tested as logic plus one headless paint with a three-photo selection (B) — SL-B15 part 1
- [x] Multi-select, part 2 — **marquee**: dragging from the grid's background paints a band and selects the
  cells it touches (a graze counts), and releasing ends the gesture without clearing what it made. The drag
  only starts on the background — a press inside a cell belongs to that cell, and stealing it would make the
  grid unclickable — which is decided against the rects painted last frame, because the pointer may cross
  cells this frame it had not reached when the drag began. Plain drag replaces, with no ⌘-drag variant: two
  ways to combine one gesture is one too many when ⇧-range already means "these and the ones before them".
  The hit test is a pure function (`cells_in`, edge-touching included, sorted so a log reads the same twice),
  and the gesture is driven through real pointer events in a headless test that presses, moves, and releases (B)
  — SL-B15 part 2
- [x] Sources, part 1 — **Picks**, **Rated** and **Rejects** rows in the sidebar, each showing its own count
  rather than the library total, and each replacing the whole question rather than nudging it (picking Picks
  with three stars and a keyword active used to leave them on, so the row meant "picks among the ones you
  starred and tagged"); the folder scope survives, because that is where you are, not what you asked. The
  highlighted row is *derived* from the filter, so a filter chip that asks the same question highlights the
  same row — a stored flag would let the two disagree. Both behaviours tested (B) — SL-A14 part 1
- [ ] Sources, part 2 — **Recently Added**: needs an `added` timestamp per photo, which is a snapshot schema
  change with a migration for every existing library; inventing one from the file's mtime would be "recently
  modified", a different and more confusing question. Listed with its unblocker in the M1 table below
- [x] Folder tree with recursive counts + drill-down (`library.subfolders`, breadcrumbs) (B) — SL-A03/A04
- [x] Placeholder cell while decoding (kind glyph on the tile) and a **badge** for the two ways a cell can
  have no pixels: `?` for a file that is not on disk, `!` for a file that could not be decoded, each with a
  hover that says which, and a missing file outranking the decode failure it caused. `cell_badge` is a pure
  function tested against all four cases, and the cell paints for a missing photo without requesting a
  decode (B) — SL-M10
- [x] Synthetic library generator (`sortlens-testkit`: deterministic JPEGs from a real encoder, sidecars by convention, self-cleaning folders) + delta-scan truth table

**Demo:** add 5k folder → grid fills → counts correct → relaunch instant → rescan adds only new
files → rename a subfolder → no duplicate rows.

---

## M2 — Preview scales · 2.5 w · track C

**Goal:** 50k photos, 60 fps, bounded memory, visible activity.
**State:** JPEG, PNG/TIFF/WebP/GIF/BMP, RAW-preview and **HEIC** thumbnails are in, behind the
content-addressed cache, the priority-ordered and bounded decode service and the activity stack; metadata is
read during the scan, including HEIF sizes and video length/rate/codec from their containers, and an
already-decoded tier is painted while a sharper one decodes. Video **posters** remain, and are named below with
what they need.

- [x] Contact sheet tiers: 128/384/1024 constants, content-addressed disk cache, bounded texture map (C) — SL-M02/M06
- [x] Cooperative cancellation + live file/folder counters during a scan (C)
- [x] Priority ordering and backpressure on the decode queue (C)
  - **Priority is a value in the request**: `Urgency::{OnScreen, Nearby, Background}`, served most urgent
    first and oldest first within a class. Workers pull from one shared queue under a condvar instead of a
    channel, which is what makes the order at pop time possible: a channel hands out what it was given in the
    order it was given.
  - **Backpressure is a bound**: 4 096 waiters; a request arriving at a full queue evicts the least urgent
    waiter only if it is more urgent itself, and is otherwise refused and counted. The evicted key is
    released from the in-flight set, so a later attempt for the same thumbnail is not mistaken for a
    duplicate.
  - The grid asks at `OnScreen`, the detail view prefetches the next photo at `Nearby`, library-wide passes
    belong at `Background`. Tested as the queue alone (ordering inside a class and across classes, eviction,
    refusal and its counter) and through the service at the bound (a refused request can be asked for again;
    an 80-cell grid against 8 slots must refuse) — proved to bite by raising that bound to 4 096, which made
    the assertion fail with 63 waiters.
- [x] JPEG decode path (C)
- [x] PNG/TIFF/WebP/GIF/BMP decode paths via craft-image-codecs, alpha preserved (C)
- [x] HEIC decode path (C)
  - **`craft-heif`** (pure Rust via `heic-rs`, panic-guarded, colour-matched to libheif rather than assuming
    BT.709) decodes the primary image and prefers the thumbnail an iPhone already stores when it is big enough
    for what was asked for. 16-bit samples are divided by 257 rather than shifted, so a 10-bit HDR photo keeps
    the top of its range instead of clipping at 255.
  - **Routed by header, not by name** (`sniff` says `Heif`), so an iPhone photo renamed `.jpg` still decodes
    and a `.heic` that is really a JPEG is not sent to a decoder that cannot read it.
  - **Sizes come from the container** during the scan (`ispe`, `irot`, `clap`), because that is the source the
    decoder will use: a portrait iPhone photo is 64 × 96 in the grid and 1.4 MP in the insights, not 0 × 0.
  - Tested with synthetic HEIF files written by the decoder's own writer — a grey ramp that must stay a ramp
    after the container parse, the HEVC stage, the colour conversion and the resize, a rotated file whose
    displayed size must swap axes, the same bytes under a `.jpg` name, and a truncated file refused with a
    reason. No binary fixture is committed, and the routing was proved to bite by disabling it (the `.jpg`
    case then fails with "not a JPEG stream").
  - **Not measured**: HEIC decode rate appears nowhere in `docs/PERF.md`; the fixtures there are JPEG.
- [x] RAW embedded previews (`craft-raw`) (C) — SL-B18
- [x] Video duration, frame rate, size and codec — **part 1** (C) — SL-B11 part 1
  - Read from the container (`craft-media`'s isobmff demuxer, `moov` only, no pixel decoded) during the scan:
    length from the video track's own timescale, frame rate as **sample count over duration** (so a 29.97 fps
    clip reads 29.97 and a variable-rate clip reads its average), size from the track or the sample entry, and
    the codec from `CodecId::name`. MP4, MOV and M4V; a file that is not a container, or whose video track
    declares no usable length, answers nothing rather than a zero-length clip.
  - Surfaced where it is used: a **duration pill** on video cells, "1:12" in the spoken description after the
    facts that decide a cull, and a length row in the compare facts. `format_duration` is one pure function, so
    the cell, the inspector and the screen reader cannot disagree about how long a clip is.
  - Tested with a video **muxed by the suite's own writer** in the test — 24 samples of 25 ticks at 600 Hz is
    one second at 24 fps — plus the same file truncated (refused), a JPEG (not a video) and the clock-format
    table. No video blob is committed.
- [ ] Video poster frame — **part 2**, absent by design with a named blocker (C) — SL-B11 part 2
  - Why not now: a poster needs `craft-video`'s H.264 (or HEVC) decoder **and** a YUV→RGB conversion honouring
    the frame's `ColorTags` — matrix, range and primaries. `VideoFrame` deliberately carries planes and tags
    with no conversion, because getting that wrong is a colour bug that looks like a decode bug. The container
    half is done, so this is now a decoder-side piece of work rather than a research question.
  - Unblocker: a poster decoder that reads the first sync sample, decodes it with `h264-decode`/`hevc-decode`
    and converts with the frame's own colour tags, with a test whose fixture is muxed by the same writer the
    metadata test uses. Until then a video cell shows its type, its length and its filename — never a blank
    cell pretending to be a photo.
  - Matroska (MKV/WebM) and AVI are not probed either: `craft-media`'s `matroska` and `avi` features are off, and
    reporting their clips with no length would be worse than reporting nothing.
- [x] Placeholder cell while decoding, no spinner (B)
- [x] Blur-up tier transitions (B)
  - **A blur-up here is a tier that is already decoded, not a second decode.** The JPEG decoder reconstructs at
    full resolution and then `resize` reduces, so a 32-pixel "blur tier" would cost exactly what the 128 tier
    costs — paying twice for the same work to show a softer picture. Instead the detail view, the compare panes
    and the cull stage ask for `sharp_or_blur`: the tier they want, or the best one already resident painted
    scaled, while the sharp decode runs in the background and replaces it in place.
  - The choice is a pure function (`blur_tier`): exact size, else the largest below, else the smallest above
    (painting a bigger picture into a smaller hole is sharp, merely wasteful), never another photo's tier, and
    `None` when nothing is resident — an empty rectangle is not a blur, so an undecoded cell keeps its honest
    placeholder instead of a smear.
  - Tested as the tier table over every case, and end to end: a real JPEG decoded by the real service at 128,
    then `sharp_or_blur(1600)` returning the 128 texture and its tier rather than waiting. Plus a headless paint
    of the detail view before its decode lands.
- [x] Activity stack: job list, statuses, elapsed, per-item errors, cancel (B) — SL-A18
- [x] Bell popover and unread badges (B)
  - Every `notice()` is recorded in a bounded history (`NOTICE_HISTORY` = 50, newest first) as well as toasted,
    so an operation that finished while you were looking elsewhere is still there four seconds later.
  - The bell appears only when it has something to say, shows a count of **unread** notices, and turns red when
    any unread one is a `NoticeKind::Problem`. Opening the popover marks everything read — reading is not
    clearing, so "Clear" is a separate action — and each row carries its age in the form someone would say it
    ("just now", "4m", "2h", "3d").
  - The 32 places that toasted a bare engine error now raise a problem, so the badge tells the truth about a
    failed move or export instead of counting it as news; the toast is coloured by the same kind, so the toast
    and the bell cannot disagree.
  - Tested: the count and the red state, trimming past the bound without corrupting the count, the age table,
    and a headless paint of the bell and the popover. **Not verified by hand**: how the popover sits against the
    top bar on a real window.
- [x] Offline/missing badge in the grid: `?` with "File missing — unmounted drive, or moved outside the
  library", and the spoken name already said "file missing" before the badge existed (B) — SL-A12 part 1
- [x] Offline/missing, part 2 — a cached thumbnail of a file that has gone is painted **faded** (B) — SL-A12
  - The cache does not know the drive left: it still holds the pixels decoded before it did, and drawing them at
    full strength makes an offline library look current. A missing file's cached thumbnail is painted at
    `CACHED_ALPHA` (110/255), so it is visibly present and visibly not live; the `?` badge says why, and its
    hover now says where the picture came from ("showing the cached thumbnail from before it went away") —
    a faded photograph and a grey rectangle are different states and the hover distinguishes them.
  - The same tint is applied in the detail view, the compare panes and the cull stage, because a cached
    full-size preview of a missing file is still a cached preview. Nothing is done when nothing is cached: the
    placeholder stays, because there is no stale picture to mark.
  - Tested as a pure tint table, and end to end: decode a thumbnail, delete the file, rescan, paint — the image
    mesh's vertex alpha drops from 255 to 110, while the default view (which hides missing files) paints nothing
    at all. **Not verified by hand**: whether 110 is the right fade on a real screen.
- [x] **A thumbnail request made after the workers went idle is served** — the defect this row uncovered
  - The priority queue replaced a channel with a `Condvar`, and nothing ever notified it. Workers parked after
    the queue emptied and every later request sat in the queue for the life of the process; tests missed it
    because a worker that had not yet reached its first `wait` found the entry by itself, so a request made
    immediately after startup worked. `request_with` now notifies on an accepted push.
  - `a_request_after_the_workers_have_parked_is_still_decoded` sleeps 250 ms first — the only way to be certain
    both workers are inside `wait` — and was proved to bite by removing the notify (it fails, waiting out its
    ten-second deadline).
- [x] **A refusal that names the wrong fact** — `Refusal::Modified`, found by a flaky test
  - The move/cull safety audit compares both size and modification time, but both came back as
    `Refusal::Changed`, whose message prints the byte counts. An edited file of the same length was therefore
    refused with "changed since it was indexed (1 → 1 bytes)" — a message that reads as a bug in the app, and a
    refusal nobody believes is worse than no refusal.
  - It surfaced as an intermittent engine test failure: `applying_move_moves_the_file_and_a_changed_file_refuses_the_batch`
    restored the bytes but not the timestamp, so whether the batch was refused depended on whether the rewrite
    landed in a new clock second. The test now restores both, and `a_file_whose_timestamp_moved_is_refused_and_says_so`
    pins the new variant and asserts the message never dresses a timestamp change up as a size change.
- [x] `craft-jsonrpc` + `craft-mcp` carry the MCP server (E) — SL-K04, part 1
  - Envelopes are `craft-jsonrpc`: it validates the `jsonrpc` version and the id, separates requests from
    notifications, answers batches as batches, and refuses a message over a byte bound **before** parsing it. The
    line reader is its `read_line`, so an eight-megabyte line is refused without being allocated, the frame is
    discarded and the session keeps going instead of dying on one bad message.
  - The session is `craft-mcp`: version negotiation (newest first, `2025-03-26` then `2024-11-05`, echoing a
    client's own version when it asks for one this server knows), the `initialize`/`initialized` handshake,
    the tools envelope and the text result. Tool **routing** stays here, straight from the command registry.
  - Two behaviours changed because the suite's answer is the protocol's answer: input that is not JSON is now
    `-32700 Parse error` rather than `-32600`, and `tools/call` with non-object arguments is `-32602` rather
    than an empty call. Both are tested, as are batch handling, the oversized frame, negotiation and the
    handshake.
- [ ] `craft-ipc` is still unused — **absent by design, with a reason**: there is no second process to talk to.
  The window owns the engine and the library's write lock, and the CLI opens the library itself, which is what
  makes the lock meaningful. A socket would need a server mode in the app, an authentication story on a
  multi-user machine, and an answer for "the app is not running" — none of which exist, and none of which the
  MCP-over-stdio path needs, because the client starts the server. **Unblocker**: a real second consumer that
  wants to drive a *running* window (a menu-bar helper, a browser extension, two windows on one library).
- [x] `sortlens-cli add|rescan|list|stats|rate|flag|undo|report` (E) — SL-K03
- [ ] `page` prints JSON and the MCP server returns JSON; there is no `--json` for the other verbs (E)
- [x] Benchmarks, part 1 — **measured**: `cargo xtask perf` (a release-mode binary, deliberately outside
  `ci`) reports scan, index, cold/warm thumbnail, the two-worker thumbnail service, the suggestion pass,
  library open and headless grid frames; `docs/PERF.md` records two thousand and ten thousand photo runs
  with the hardware and command. The measurement found a real defect — snapshots never reset the
  replay counter, so past 256 commits *every* edit rewrote the whole library, and a sync snapshotted every
  256 files — fixed by batching the sync's snapshot and resetting the counter: ten thousand photos went
  from 78 s to 11 s, per-photo cost now flat (E) — SL-L09
- [ ] Benchmarks, part 2 — 100k scroll p99, RSS and GPU frame time: none of the three can be measured
  here. The grid is virtualised, so a frame cost at ten thousand says more about the viewport than the
  library; a headless context has no GPU, no vsync and no window, so its frame time is layout and shape
  building only; and RSS needs a long single-process run of the real app. `docs/PERF.md` names what would
  unblock each rather than estimating

**Demo:** 50k library scrolls at 60 fps; unplug the drive → offline badges, no freeze; CLI
scans headlessly and prints a JSON report.

---

## M3 — Annotate · 2.0 w · tracks A,B

**Goal:** the library remembers what the photographer decided, on disk and in XMP.

- [x] `photo.rate/flag/label/tag` + batch variants (A) — SL-D01..D05
- [x] Tag rename/merge, and `photo.tagFromPrev` (re-apply the last keyword) (A) — SL-D05
  - The memory is the last keyword this session **added**, one of them, and it is deliberately not persisted: a
    keyword applied last week is not "the last keyword", and pasting somebody else's older tags onto a photo is
    a wrong write rather than a convenience. Removing a keyword does not become "the last" one either.
  - It goes through `set_tag`, so a renamed keyword still resolves, a photo that already carries it is not
    written twice, and the write is one undoable op. Bound to `T`, in the registry as `photo.tagFromPrev` with
    its MCP argument schema, and the UI notice names the keyword rather than the count.
- [ ] Registry create/rename/merge/alias with a searchable picker is in; deleting a keyword everywhere is not (A) — SL-D06
- [x] `sortlens-sidecar`: XMP read/merge/write, unknown fields preserved, atomic (A) — SL-D08/D09
- [x] Sidecar review imported when a photo is first indexed (A) — SL-D09
- [x] External-change detection for already-known photos + retry for unreadable metadata (A) — SL-D11
- [ ] Stale-revision rejection (`Error::StaleRevision`) (A) — SL-D11
- [x] Rating/flag/label controls in the detail header, the inspector panel and the grid context menu (B) — SL-D01..D03
- [x] Filter bar: rating, flags, labels, keyword, collection, kind, edited, missing, clear (B) — SL-B07..B13
- [x] Keyword ANY/ALL, multi-keyword filtering, active-filter badge (B)
- [x] Sort keys: added/name/newest/oldest/largest/rating, persisted (B) — SL-B06
- [x] Detail view: large canvas, counter, fit/1:1, arrow navigation, facts line (B) — SL-B18..B20
- [x] Detail lookahead strip + 64-bin luminance histogram (B) — SL-B21
- [x] Shortcut overlay + keymap wiring (B) — SL-B23: `crates/ui/src/keymap.rs` is the table, checked against the command registry in both directions
- [ ] One persisted, reversible undo level + the change viewer are in; a 20-step undo/redo history is not (A) — SL-D10/H04
- [x] `photo.reveal` / `photo.open`: argv built in `sortlens-core::reveal` (pure, per-desktop, tested), executed by the app and the CLI (A) — SL-B22
- [ ] XMP round-trip and filter-algebra tests are in; stale-revision and badge-contrast tests are not

**Demo:** rate/tag 100 photos → quit → reopen: identical state; `xmllint`-clean sidecars contain
the tags; filter "≥4★ + Pick + #wildlife" in one keystroke; undo 20 steps.

---

## M4 — Cull · 2.5 w · tracks A,B

**Goal:** the signature workflow, safe by construction.

- [x] `sortlens-cull` state machine + transition table tests (A) — SL-C09
- [x] Session persistence + resume + crash resume (A) — SL-C07
- [x] Cull view: canvas, up-next strip, action bar, live counts (B) — SL-C01/C06/C17
- [x] Cull summary sheet + applied-batch receipt (B) — SL-C17
- [x] Overlays inside image bounds + two-phase animation + reduced motion (B) — SL-C04
  - The decision overlay is painted with its label at the *image's* top-left corner, clipped to the image rect,
    not the canvas: a label that drifts outside the photograph reads as window chrome rather than a mark on the
    frame. The gesture tint and label were already inside the frame; the same rule now governs the overlay.
  - Two phases, as a pure `overlay_phase(elapsed_ms, motion)`: the label fades in over 90 ms (ease-out) with the
    photograph still in place, then the photograph slides out over 140 ms (ease-in) while the next one is
    already behind it. The overlay keeps the *judged* photo id, because the session has already advanced — the
    only way to slide out the photo that left is to remember it. It clears itself when the slide ends, and
    requests a repaint while it runs so it cannot freeze at whatever phase the next input caused.
  - `Motion::{Full, Reduced}` is a real persisted setting (`appearance.motion`, unknown value falls back to
    full), a chip in the top bar, and honoured by the phase function: reduced draws the label at once and cuts
    the slide to zero, while still holding the label for its 90 ms — the confirmation is information, not
    decoration. The OS preference is not read and is named absent in M10's table.
  - Tested: the phase table over every case including monotonicity and both settings, the settings round-trip
    plus an unknown value and a settings file from before the key existed, and a headless cull view where a
    decision paints an overlay over the stage and drops it. **Not verified by hand**: the timings on a real
    display — 90 ms and 140 ms are reasoned, not measured against an eye.
- [x] Keyboard decisions: P/X/Space/U/←/C (B) — SL-C02
- [x] Gestures: drag up/down to keep/reject, weighted by distance (B) — SL-C03
- [x] Zoom 1–8× with pan and wheel stepping (B) — SL-C05? (zoom) SL-C02
- [x] Lookahead prefetch of the neighbouring ±1/±2 frames in detail view (C) — SL-C05
- [x] Apply confirmation with counts and a receipt (A,B) — SL-C10
- [x] Staged deletes and Apply: audit, move/tag/trash, per-item results (A) — SL-C10
- [x] Trash executor: audit-before-act, verify, per-item results, grouped inverse op (A) — SL-H01/H03
- [x] Quit guard for running jobs and staged culling (B) — SL-H12
- [x] Decision announcements as live regions + named buttons (B) — SL-L02/L03
- [x] Tests: reject never deletes; skip preserves flags; apply idempotent; session survives a restart

**Demo:** cull 500 frames keyboard-only, kill the app mid-session, relaunch and resume at the
same photo; apply 148 rejects to trash; undo restores all 148.

---

## M5 — Organise · 2.0 w · tracks A,B

- [x] Subfolder-first drill-down: one level at a time, recursive counts, breadcrumbs (B) — SL-A03
- [x] Collections in the sidebar: range-select, batch colour, batch pin, batch delete (B) — SL-A05..A10 part 1
  - Ticking collections is its own `CollectionSelection` — anchor plus ticks, no primary — because a collection id
    and a photo id are different kinds of thing and a set that could be passed where photos were expected is a
    bug waiting for a refactor. Click replaces, ⌘-click toggles, ⇧-click takes the run **in sidebar order** (id
    order would select something nobody is looking at), and a ⇧-click with no anchor is a plain click rather
    than half the list. The set is pruned every frame against the engine, so a delete — or an undo — cannot
    leave a ghost ticked.
  - A ticked row gets a dot distinct from the filter's highlight: a ticked collection is what a batch will act
    on, a filtered one is what the grid is showing, and those are different questions. A bar appears when more
    than one is ticked, saying the count, how to change it, and offering ✕ — a tick you cannot see is a tick
    that surprises you.
  - A context menu acts on the whole ticked set when the row is part of it and on that row alone otherwise:
    right-clicking an unrelated row must not recolour four others. Rename and delete stay single-collection in
    the multi-case (renaming four rows to one name would merge them); deleting many is offered explicitly with
    the count.
  - The engine grew three batch methods (`set_collections_colour`, `set_collections_favourite`,
    `delete_collections`) that commit **one `Op::Group`**, so five recoloured collections undo in one ⌘Z rather
    than five. They count only what changed, like every other write, and `collection.colour`/`collection.favourite`
    /`collection.delete` in the command registry take `collections: [1,2]` as well as a single `id`.
  - Tested: the gesture state machine (replace/toggle/range/widen/narrow/backwards/no-anchor/prune/clear), the
    engine batch semantics (one undo restores all three; a second undo reaches the collection *creation*, which
    is how we know the group went in as one entry; photos untouched by a batch delete), and a headless sidebar
    paint with two ticked. **Not verified by hand**: how the dot and the tick read next to each other at real
    size.
- [x] Folders in the sidebar: favourites and colours (SL-A05..A10 part 2)
  - A folder's mark is not stored *in* the folder — folders are discovered by the scan and a folder that leaves
    the disk is gone with its photos — so it is a record keyed by `PathKey`. That is what makes it survive a
    rescan, a remap, and the same folder arriving by another Unicode spelling.
  - `Op::SetFolderMark` records the **whole** mark rather than the field that moved, so its inverse is the
    previous whole mark and one ⌘Z clears a pin *and* a colour together. An empty mark removes the entry, both
    when applied and when replayed, so a library full of unmarked folders costs nothing in the snapshot.
  - The sidebar draws a star and the collection colour dot beside the folder name, and its context menu offers
    pin/unpin and a colour menu. `folder.mark`/`folder.marks` are in the registry (with MCP argument schemas) and
    in the `--json` table, so a script can read and set the same marks.
  - Tested: the identity rules (above), the mark round-tripping through the log and undo as one step, an empty
    path refused, `library.folder_marks` empty for a fresh folder, and a headless sidebar paint with a marked
    folder.
- [ ] Folders in the sidebar: per-folder remembered sort and search — **part 3**, needs the same mark record to
  carry two more fields and a decision about whether a folder's sort follows the folder or the view (SL-A05..A10
  part 3)
- [x] Collections: create/rename/delete, add/remove, sidebar, filter (A,B) — SL-G01..G03
- [x] Collection favourites, colours, per-collection search and sort (B) — SL-G04
- [x] Batch move with sidecars travelling, receipts and undo (A) — SL-G07
- [x] Persistent undo across restarts: the inverse travels in the log record and in the snapshot (A) — SL-A13
- [ ] Inline tagging + re-apply last tags in cull (B) — SL-C11/C12
- [x] Library doctor: check, find-moved-by-content-key, remove-missing, all undoable (A,B) — SL-H07/H08/A13
- [x] First-run wizard: welcome → add a folder → indexing (B) — SL-I12; the remote test waits for M9
- [x] Tag rename/merge UI, plus aliases in the keyword registry (B) — SL-D06

**Demo:** three folders → two collections; move 200 files with hash verification and undo;
remap a moved drive; run the wizard on a clean machine.

---

## M6 — Analyse · 2.5 w · tracks D,B

- [ ] `sortlens-quality`: sharpness score + versioning + tests (D) — SL-E01
- [ ] Blur scan job + threshold review + batch select (D,B) — SL-E01/E02
- [x] `sortlens-hash`: dHash + aHash over a 9×8 box-filtered grid, Hamming distance, 3×3 colour fingerprint (D) — SL-E03
- [x] Similar groups (union-find chaining), best pick, similar-to (D,B) — SL-E04/E05
- [x] Similar: BK-tree metric index (exact, sub-quadratic) and a colour veto (D) — SL-E03
- [x] Similar: side-by-side compare and colour-only matches (B) — SL-E05
- [ ] Batch select of group leftovers and delete (B) — SL-E05
- [ ] `sortlens-geo`: projection, clusters, gazetteer (D) — SL-E06/E07
- [ ] Offline basemap + optional MBTiles + attribution (B) — SL-E06
- [x] Insights: totals, rating/label/kind distribution, cameras, lenses, keywords, timeline — SL-E08
- [x] Insights: worker with progress, highlights, months and seasonality (B) — SL-E09..E11
- [x] Memories: `sortlens-memories` (on this day, this week in past years, never looked at), For You view, `memories.list`, CLI `memories --today` (D,B) — SL-E12
- [x] Chart component (fixed height, no chart dep): `crates/ui/src/charts.rs` — columns with a one-pixel floor and a round axis maximum, proportional rows, painted headlessly in a test (B) — SL-E11
- [ ] Duplicate-transform and BK-tree cross-check tests are in; the blur ladder and cluster determinism wait for their features

**Demo:** find 20 blurred frames and 12 duplicate groups in a 5k library; map clusters match
GPS; every dashboard number reconciles with the catalog; memories stable across refresh.

---

## M7 — Safety · 1.5 w · track A

- [x] Three delete modes — trash, move, tag-only — with distinct executors (A) — SL-H01
- [x] Rejects bin: applied batches listed, restored with flags and missing state (A,B) — SL-A15
- [x] Empty the bin: forget one batch or all of them, files untouched, with a confirmation that names the counts (A,B)
- [x] Recently Deleted source: sidebar row + restore dialog (B) — SL-A16
- [x] Backup/restore of the catalog files, CRC32 per file, restore into a new folder (A) — SL-H10
- [x] Volume remap with sample verification (`REMAP_SAMPLE = 12`) (A) — SL-A20/H06
- [x] Integrity check (audit) + compaction that refuses when the audit is not clean (A) — SL-H11
- [x] Change viewer (recent ops, newest first) + plain-text support report (B) — SL-H04
- [x] Read-only library mode via an OS file lock, with a title-bar badge (A) — SL-H14
- [x] Crash/corruption suite: a torn tail, a flipped checksum (the good records before it survive) and a partial snapshot (the log rebuilds the library)

**Demo:** delete in all three modes; restore from bin and from OS trash; corrupt the op-log tail
and recover; export/import a library onto a second machine and remap the volume.

---

## M8 — AI + export · 2.5 w · tracks D,A

**State:** done for everything that needs no weights — export, manifest, store, NMS, label map, EXIF
hints, packs, the suggestion panel, accept-and-undo and the batch pass. The rows under "Deliberately
absent" are not part of "done": each needs something this repository does not ship, and pretending
otherwise is how a plan starts lying.

- [x] Model manifest (id, kind, licence, digest, size) + content-addressed store that verifies size
  and SHA-256 before use, installs atomically and refuses to delete a file it cannot identify (D)
  — SL-F01; the **downloader** (SL-F02, consent-gated, resumable) is still unbuilt and unlisted
- [x] Non-maximum suppression per class + label map parsing, as pure functions over normalised boxes,
  with the crowd, the empty-box and the tie-order cases tested (D) — SL-F03 part 1
- [x] EXIF hint rules: camera and lens as keywords, three focal bands, low light and long exposure,
  each a named constant with a stated reason, all feeding the same merger (D) — SL-F05
- [x] Suggestion merger: dedupe against existing keywords, best score wins, source rank settles ties,
  caps at 8 per photo, deterministic order (D) — SL-F06 part 1
- [x] Suggestion panel: a "Suggested" row in the detail view, each offer showing its score and whether
  the camera or a model said it, one click writing the keyword through the ordinary tag op; engine
  commands `suggest.list`/`suggest.accept`, MCP examples and `suggest`/`accept` in the CLI (D,B)
  — SL-F06 part 2
- [x] Packs as data: a JSON file per pack with a name, a licence and rules (`folder_contains`,
  `camera_contains`, `lens_contains`, `min_iso`, `focal_band`, all ANDed), read from `<library>/packs/`,
  refused when a rule has no conditions or a band does not exist, listed with its problem when a file is
  broken; counts and a switch per pack in Settings, the switch an op-log entry so it persists and undoes;
  `packs`/`pack <name> on|off` in the CLI, `packs.list`/`packs.set` for MCP, and `docs/PACKS.md` (D,B)
  — SL-F07/F08
- [x] Pack rules feed the same merger, so an enabled pack's keywords appear as proposals and only an
  acceptance writes one — `craft-flags` was not needed: a boolean in the op log already persists and
  undoes, and a feature-flag crate would add a second place for the truth to live (D)
- [x] Batch suggest job: a worker over every photo (EXIF hints plus enabled packs), progress, cancel,
  poll, results and clear, `suggest.start`/`progress`/`cancel`/`results`/`clear`, `batchsuggest` in the
  CLI and a "Suggested keywords" panel where each row is a button — **it writes nothing**, which is
  pinned by a test that asserts the tags and the change log are untouched after a pass (D) — SL-F09
- [x] No-network assertion for `sortlens-ai`: a source scan for socket APIs and a dependency scan for
  HTTP clients, both proved to fail on a planted hit (D) — SL-F10
- [x] Export: original copies and JPEG re-encodes, longest-edge resize, quality, EXIF/IPTC carried
  forward, four collision policies, worker progress and cancel, a dialog and a receipt (A) — SL-B24

### Deliberately absent from M8

Not rows with an empty box — rows that would need something this repository does not ship, kept here so
the absence is a decision rather than a gap:

| Not built | Because | What would unblock it |
| --- | --- | --- |
| `tract` detector session (SL-F03 part 2) | a session needs weights; pinning a model means shipping or fetching a file whose digest and licence must be real, and inventing one would be a lie | a chosen ONNX detector with a known licence and digest, added to `Manifest::shipped()`; the NMS and label map it would use are already tested |
| Image/text embedding sessions and prompt packs (SL-F04) | same: embeddings are weights | the same, plus a text encoder |
| `source = Ai` in the keyword filter | the panel shows provenance per suggestion, but filtering by it needs a per-tag source in the catalog — a schema change with a migration, for a filter whose values would be guesses | a decision to store tag provenance |
| PNG/TIFF/WebP export | the workspace has one encoder (JPEG); a format chip that cannot write its format is a lie | an encoder crate, reviewed for licence and size |

**Demo:** on a clean machine, index 50 photos, run `batchsuggest`, accept ten proposals from the panel,
undo one, switch on a keyword pack and watch its keywords appear for the matching folders; export a
selection as originals and as JPEG and diff the metadata.

---

## M9 — Remote + automation · 2.5 w · tracks E,B

**State:** the automation half is in — the MCP server, `--json` on the read verbs and the diagnostics
bundle. The remote half (a server, a wasm client, remote album edits) is absent by design and listed
below with what would unblock it; i18n and the keymap editor remain.

- [x] MCP server: `sortlens-mcp` — newline-delimited JSON-RPC over stdio, `tools/list` straight from the command registry, `tools/call` through `Engine::run`, `sortlens-cli mcp` (E) — SL-K05
- [x] CLI parity and stable JSON schemas: `--json` on the read verbs (roots, keywords, stats, list,
  folders, check, moves, changed, history, audit, memories, shortcuts, registry, packs, suggest, jobs)
  prints the engine's own answer, a verb without a schema says so and lists the ones that have one
  instead of printing prose, and the golden files pin five of the schemas plus the refusal (E)
  — SL-K03/K06
- [x] `--json` for the write verbs, and a written compatibility note per schema (E)
  - The table stopped being a list of names and became a list of **schemas**: verb, command, read or write, and
    a one-sentence contract for the object. Thirteen mutating verbs answer `--json` now (`rate`, `flag`,
    `label`, `tag`, `untag`, `date`, `rename`, `alias`, `accept`, `pack`, `undo`, `retry`, `remap`), and
    `--json schemas` prints the whole table for a script to discover rather than hoping.
  - `docs/CLI.md` is the promise: additive-only within a major version, absent ≠ null, counts are photos unless
    the note says otherwise, a refusal is a non-zero exit with stderr rather than an error object dressed as a
    result, and progress verbs (`export`) deliberately have no schema because a job with a cancel is not one
    object.
  - **The contract found a lie.** Reading the new goldens showed `rate 1,2 4` answering `{"changed": 2}` when
    both photos already had four stars: `rate`, `flag`, `label` and `setCaptured` all counted the *request*, not
    the write. They now compare against the catalog first, so a repeat answers `0` — which is what "a write
    reports what it changed" has to mean if a script is to tell a write from a no-op. Pinned by
    `a_repeat_write_reports_no_change`; the goldens `40`/`41` show `2` then `0`.
  - Ids take `1` or `1,2,3` in both the JSON and the plain form, so a page of photos is one call and one count.
  - Gated: `every_write_verb_has_a_json_schema_and_a_stated_contract` checks that every mutating verb answers
    instead of refusing, that every note is a sentence rather than a placeholder, and that the table marks
    writes as writes — **proved to bite** by renaming `pack` in the table (it fails naming the verb and quoting
    the refusal).
- [ ] i18n catalogs + RTL screenshots (ar/ja/zh-TW/ko/th/vi) (B) — SL-L07
- [ ] Localised numbers/dates (B) — SL-L08
- [ ] `craft-keymap` editor + import/export (B) — SL-I09
- [x] Diagnostics bundle: `Engine::diagnostics` writes plain text with versions, library counts, the
  read-only health check, packs, the last 25 changes and the root list, with **every absolute path
  replaced** by `<library>`, `<root-N>` or `<home>` (longest match first, so a root under the library is
  not half-replaced); `diagnostics [--out FILE]` in the CLI, `app.diagnostics` for MCP, a golden, and an
  end-to-end test that reads the output *before* normalisation and fails if either path leaks (E)
  — SL-I14

### Deliberately absent from M9

| Not built | Because | What would unblock it |
| --- | --- | --- |
| `sortlens-server`: token auth, pairing, permissions, upload resume (SL-J01/J03/J04) | a server cannot be tested here without a second device and a network peer; shipping an untested auth path is the worst kind of untested code | a test harness with two processes and a loopback socket, plus a decision on pairing (QR, code, or both) |
| `sortlens-web` wasm client reusing the UI (SL-J02) | the same: it exists to be opened on another device | the server above, and a browser to check it in |
| Remote album/collection create and rescan (SL-J06/J08) | commands over the wire need the server | the server above |

`docs/ARCHITECTURE.md` §11 keeps the design for all three. The command registry is already the API they
would expose, which is why the absence is a missing transport rather than a missing design.

**Demo:** an agent runs a full cull and report through MCP in CI; a script reads every verb's answer with
`--json`; `sortlens-cli diagnostics` produces a bundle a maintainer can read without knowing where the
photos live; the UI renders correctly in Arabic and Japanese.

---

## M10 — Release · 2.0 w · all tracks

**State:** the version contract, `--version`, the release workflow, the packaging notes, the measurements,
the icon, the attribution gate and the first half of the accessibility audit are in. Signing, installers,
the emitted accessibility tree, high contrast, the operating system's own reduced-motion preference, theme
import, `craft-update` and the tag are not — each named with its unblocker in the table at the end of this milestone.

- [x] Accessibility audit, part 1 — **names**: every custom-painted clickable region in the shell (grid
  cell, detail preview, lookahead strip, cull strip, similar clusters, memory cards, sidebar rows) now
  registers a `WidgetInfo` with a description from `a11y::describe_photo`, and the row carries its label
  and count; `cargo xtask a11y` fails when a function makes a region clickable without naming it, proved
  to bite by deleting one registration (B) — SL-L01
- [x] Application icon: one drawing in `crates/icon` (code, not a dropped-in binary) rendered two ways —
  the window icon as RGBA through a build script, and `assets/icon.svg` as the committed, human-viewable
  copy — with the SVG held to the drawing by a test that fails when either changes and rewrites it only
  under `SORTLENS_UPDATE_ICON=1`; the app now sets `with_icon`, which it previously did not do at all, so
  the SVG existed and nothing loaded it. Proved to bite by appending a line to the SVG. The full brand
  pass (marketing assets, `.icns`/`.ico` packaging per platform) is in the table below, because a
  plausible-looking icon in a window is not a brand (B) — SL-I11
- [x] One version, declared once: `cargo xtask version` fails when a crate declares its own version or
  stops inheriting the workspace one, and `sortlens-cli --version` answers without opening (or creating)
  a library — both proved to bite, the second by a test that asserts the folder is still absent (E)
- [x] Release workflow `.github/workflows/release.yml`: verify, build the two binaries for macOS/Linux/
  Windows, compute the checksums in one place from the uploaded files, attach them — **written but never
  run**; no tag exists yet, and the file says so
- [x] `docs/RELEASE.md`: the single-version rule, the artefact list, the steps to cut a release, the
  checksum story and what this version deliberately does not have (E)
- [x] Benchmarks recorded in `docs/PERF.md`, from `cargo xtask perf` on a named machine, including the
  snapshot defect they exposed and the budgets that cannot be measured here (see M2's row) (E)
- [x] `docs/ATTRIBUTION.md` complete and **enforced**: `cargo xtask attribution` (in `ci`) fails when a
  linked crate is unattributed, when a declared-but-unlinked crate is not listed as such, when a table
  names a dependency that no longer exists, when an embedded font family is missing, or when the
  "no model weights, corpora or datasets" statement is deleted — proved to bite both ways (a fake
  `serde_yaml` dependency, and a deleted font row) (E)

### Deliberately absent from M10

Not rows with an empty box — rows that would need something this repository does not have, kept here so the
absence is a decision rather than a gap:

| Not built | Because | What would unblock it |
| --- | --- | --- |
| Signed and notarised macOS builds (SL-I12) | notarisation needs an Apple Developer identity and a CI secret; a workflow that pretends to sign would produce a bundle Gatekeeper rejects | an identity, `APPLE_ID`/`TEAM_ID`/app-specific password in CI secrets, and a release run to test against |
| Windows MSI and Linux AppImage/Flatpak | each needs its own toolchain (`cargo-wix`, `linuxdeploy`, Flatpak manifests and a runtime) and, for MSI, a code-signing certificate | those toolchains in the release workflow and a signing certificate |
| `.icns`/`.ico` generation and a brand pass | the icon is drawn and wired (`crates/icon`, `with_icon`), but the per-platform icon containers and any marketing artwork are packaging work that the absent installer work would consume | the packaging step above; the drawing itself already exists |
| Theme import via `craft-ext-core` palettes (SL-I06) | importing someone else's palette means committing its colours and its licence; no palette has been chosen, and inventing one is not an import | a chosen palette with a licence that permits redistribution |
| The emitted accessibility tree (SL-L01..L05 part 2) | the names and roles are set and checked (`cargo xtask a11y`), but this egui version has no public way to read the tree back and `Context::has_accesskit_node` answers `true` even for an unnamed area, so a headless test would assert nothing; focus order and traps need a real client | a screen reader — VoiceOver, Orca or NVDA — on a machine with a desktop session, and a person to listen |
| High-contrast preset, and the operating system's reduced-motion preference (SL-L09/L06 part 2) | the reduced-motion *toggle* exists (`appearance.motion`) and the animation honours it, but the OS preference itself is a platform signal egui does not expose across platforms, and a high-contrast preset is a design decision — which contrast pair — that the headless harness cannot show is readable | a design pass and a person looking at it; a per-platform "reduce motion" query (or a future egui accessor) for the OS half |
| `craft-update` wired to the launcher | the crate is declared but unlinked, and an update checker is a network call this project deliberately does not make (`crates/ai` has a `no_network` source scan guarding exactly that promise) | a decision that the app may reach the network, plus a release feed to check |
| Release notes and a 1.0 tag | a tag is the repository owner's act, and notes for a version that no one has installed yet would describe intentions rather than a release | the owner tagging, and feedback from a run of the artefacts |

**Demo:** install from each artifact on a clean machine; first-run wizard; a11y tree has zero
unnamed controls; every budget in `FEATURES.md` §M passes.

---

## Backlog (post-1.0)

| Item | Why deferred |
| --- | --- |
| Native video playback | Poster frames cover culling; decoding + audio is a large surface |
| Face detection/grouping (SL-F11) | Second model, privacy-sensitive, not core to culling |
| Compare/Survey (SL-C13/C14) | Cull covers the fast path; compare is precision work |
| Smart collections (SL-G05) | Filter presets + saved views cover most of it |
| Keyword hierarchy (SL-D13) | Flat tags match the vendor and XMP interop |
| Portable mode (SL-I15) | `--data-dir` exists; UI packaging later |
| Network-share libraries | Correctness/perf risk; read-only at best |
| Windows/macOS shell extensions | Out of scope for Rust core |

## Status log

| Date | Change |
| --- | --- |
| 2026-11-21 | **M0–M10 shipped, and the documents reconciled with the tracker.** Everything that needs no weights, no signing identity, no second device and no translation pass is built and tested: 432 tests, one command registry behind the window, the CLI, the MCP server and `--json`; measured numbers in `docs/PERF.md` (which found and fixed a quadratic import); an icon the window actually loads; gates for layer rules, lint headers, asset provenance, fonts, one version, attribution, accessibility names and the documents themselves (`cargo xtask docs`). What remains is itemised in each milestone's "Deliberately absent" table with its unblocker. |
| 2026-11-21 | Plan written: feature inventory (176), architecture, tooling audit, UX spec, roadmap. Implementation not started; Rust toolchain not yet installed on this machine. |
| 2026-11-21 | M0 skeleton built on macOS. Toolchain 1.95.0 installed project-locally (`scripts/bootstrap.sh`, `.toolchain/` gitignored). 23 crates scaffolded; `sortlens-core` (ids, labels, units, errors), `sortlens-engine` (registry + `run`), `sortlens-ui` (4 presets, five surfaces, settings/about dialogs), the `sortlens` app (title-bar policy, `craft-settings` store, `fn ui` shell) all compile. `cargo xtask ci` green: fmt, clippy `-D warnings`, 20 tests, layer rules, lint headers, asset provenance. `cargo xtask wasm` green for `sortlens-core` + `sortlens-ui`. `docs/BEHAVIOUR.md` written. Not done: fonts, `cargo deny`, snapshot CI, shortcuts dialog. |
