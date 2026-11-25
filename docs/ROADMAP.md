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

- [x] `rust-toolchain.toml` (1.95.0, phone targets), `.cargo/config.toml` alias; egui + winit forks patched in `Cargo.toml`
- [x] Workspace: 19 library crates + 4 app crates; `[lints] workspace = true`; crate-root denies (A)
- [x] `xtask`: `ci`, `fmt`, `clippy`, `test`, `layers`, `roots`, `assets`, `mobile` (E)
- [x] `cargo deny` allow-list in `deny.toml` (licences, advisories, bans, sources), `cargo xtask deny`, its own CI job; `Cargo.lock` committed (E)
- [x] App shell: top bar, bottom bar, left/right panels, canvas (B)
- [x] `craft-window` title-bar policy; macOS traffic-light inset (B)
- [x] Fonts: `crates/ui/build.rs` embeds craft-fonts when `CRAFT_FONTS_DIR` is set (optional by design),
  `fonts::install` appends them as fallbacks after egui's own faces, `fonts::coverage` checks `Latn`/`Zsym`,
  and `cargo xtask fonts` reports the set (B) — egui's own faces remain the zero-config default. **The
  `CRAFT_FONTS_SCRIPTS` trim shipped 2026-11-24:** a filtered build (the phone's) embeds only the faces the
  shipped languages need, selected by `crates/ui/src/font_manifest.rs`, the module the report shares — see
  Phase 4 item 3b.
- [x] Four theme presets + contrast tests (B) — widget states set on `Visuals` through the `WidgetPalette` mapping in `Tokens::palette`
- [x] `craft-settings` v1 type, damaged-file notice, unknown-key preservation (A)
- [x] Command registry skeleton; `Engine::run` returns typed errors (A)
- [x] Commands: `app.about`, `app.settings`, `app.quit`, `app.supportReport`; 17 planned commands carry their milestone (A,B)
- [x] `app.shortcuts` — the keymap table, the searchable overlay, and every command shortcut wired (A,B)
- [x] Empty-library state (B)
- [x] CI: 3 OSes + phone-target check + snapshot job (E) — `cargo xtask snapshot` diffs 30 recorded CLI outputs in its own job

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
- [x] Sources, part 2 — **Recently Added**: an `added` instant on `Photo`, recorded by the scan when a
  file first enters the catalog and never approximated from the file's `mtime`; the snapshot schema moved
  to 2, and a schema-1 photo migrates with no instant rather than an invented one, so it honestly stays
  out of the source instead of standing in with a modified time it did not choose. The source is the last
  30 days in the sidebar with its own count, asked newest-record-first via `Sort::RecentlyAdded`, and the
  question is callable from the engine (`library.page` with `filter.added_after`) and documented in the
  MCP examples (A,B) — SL-A14 part 2
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
- [x] Video poster frame — **part 2**, shipped 2026-11-22 (C) — SL-B11 part 2
  - `sortlens-thumbs::poster` reads the first sync sample of the first video track straight from disk
    (`craft_media::FileReader`, never a whole-file read) and decodes it with `craft-video`'s H.264
    (`avcC`) or HEVC (`hvcC`) decoder, feeding at most eight access units before a flush, because the
    decoder's reorder buffer releases the first picture only when a later one arrives.
  - The conversion honours the frame's **colour tags**: matrix (BT.709/601/2020, SD fallback for an
    unlabelled stream), limited/full range, transfer (through `craft-color`, PQ and HLG included), and
    primaries gamut-mapped into sRGB (BT.2020, P3D65; same D65 white, so one 3×3 each). The `tkhd`
    quarter-turn is applied, so a portrait phone clip shows portrait, matching the catalog's size.
  - Routed **by header** in `decode_file` before anything is read whole — a clip renamed `.jpg` still
    gets its poster and a still named `.mp4` is still a still — so grid cells, the detail view and the
    cull stage all show posters through the ordinary thumbnail service and cache. Until it lands a video
    cell showed its type, its length and its filename.
  - Tested with clips **encoded and muxed by the suite's own crates** — an H.264 clip whose first frame
    carries a grey ramp and whose later frames are flat grey (so a wrong-frame poster reads flat), an
    HEVC clip, the rename cases, a truncated container and a non-container — plus rotation unit tests.
    No video blob is committed.
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
- [ ] `craft-ipc` is still unused, but the reason it gave is now false and is corrected here: the app
  **does** spawn a second process. The host starts `sortlens-server` beside itself (supervised by
  `crates/remote/src/supervise.rs`), passes it the library and a capability token, and speaks to it over a
  loopback HTTP binding. The crate would add a second local transport with its own authentication and its own
  answer for "the app is not running" beside the one the phone protocol already needs. **Unblocker**: a consumer
  that must reach a *running* window through a local socket rather than the server (a menu-bar helper, a browser
  extension, two windows on one library) — and it would follow the server's capability-token model rather than
  open an unauthenticated pipe.
- [x] `sortlens-cli add|rescan|list|stats|rate|flag|undo|report` (E) — SL-K03
- [x] `--json` for every verb that answers one object (E) — **done 2026-11-26**, see the seventy-fourth
  entry. `page` printed JSON and the MCP server returned it, but a script could not ask for the same answer from
  `insights`, `report`, `jobs`, `mark`/`marks`, `refresh`, `compact`, `backup`, `restore`, `move`,
  `unmove` or the two consent switches; those eleven answer `--json` now, and `model-consent`/`update-consent`
  report `changed` like every other write.
  - The row was half stale and half a bug: 40 schemas already existed, and the verbs above were the remainder —
    except that `list --json --tag KW` and `list --json --source SRC` were *accepted and dropped*, because the
    JSON path put the keyword in a key the engine does not read. A script asking the filtered question got the
    whole library; the plain form filtered, the JSON form did not. Both forms now build the same `Filter`, an
    unknown source is refused with the list of real ones, and golden steps `46`–`49` fail if it regresses.
  - `label`, `mark`, `marks` and `jobs` had no plain form at all — reachable only through `--json` — and now
    print prose like every other verb. The usage text was rewritten: its `--json` paragraph sat *between*
    `backup` and `restore`, and it named neither those four verbs nor `page`/`insights`/`model-*`/`update*`.
  - The passes that deliberately keep no schema — `add`, `rescan`, `batchsuggest`, `sharpen`, `export`,
    `model-install`, `model-remove`, `update-check` — and `diagnostics`, whose answer is a redacted text
    bundle, are named in the contract test and required to refuse; "no schema" is a decision with a test, not
    an omission.
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
- [x] Registry create/rename/merge/alias with a searchable picker, and deleting a keyword everywhere (A) — SL-D06
  - `tag.delete` in the registry and the MCP examples; `Engine::delete_tag` runs the catalog change, syncs
    the sidecars of every photo that carried the keyword, and records a `#tag deleted everywhere` job.
  - `Catalog::delete_tag` is one undoable `Group`: a `Tagged` op (every photo, tags minus the keyword) and
    an `Aliased` op that forgets the whole rename trail, so re-typing the old name cannot resurrect the
    keyword. Naming an alias deletes the keyword it stands for, as the rest of the app resolves names.
  - Deleting a keyword nobody carries and no alias points at is refused, not silently "deleted".
  - Sidebar keyword rows offer "Delete everywhere"; the notice names the keyword and the count, and a
    keyword filter pointing at the deleted keyword does not linger. Headless test drives the real handler
    over a scanned library and undoes it; catalog and engine tests cover refusal and the alias chain.
- [x] `sortlens-sidecar`: XMP read/merge/write, unknown fields preserved, atomic (A) — SL-D08/D09
- [x] Sidecar review imported when a photo is first indexed (A) — SL-D09
- [x] External-change detection for already-known photos + retry for unreadable metadata (A) — SL-D11
- [x] Stale-revision rejection (`Error::StaleRevision`) (A) — SL-D11 — shipped 2026-11-23: every photo
  carries a `rev` that rises once per real annotation change (and not on a no-op), and travels in the
  log and snapshot; `Catalog::commit_expect` refuses a write made against an older revision, and every
  engine command response now carries `rev` with any command able to pass `expect` — a stale caller is
  refused before anything is touched. ARCHITECTURE.md's `rev` envelope paragraph was the spec; the code
  now keeps it
- [x] Rating/flag/label controls in the detail header, the inspector panel and the grid context menu (B) — SL-D01..D03
- [x] Filter bar: rating, flags, labels, keyword, collection, kind, edited, missing, clear (B) — SL-B07..B13
- [x] Keyword ANY/ALL, multi-keyword filtering, active-filter badge (B)
- [x] Sort keys: added/name/newest/oldest/largest/rating, persisted (B) — SL-B06
- [x] Detail view: large canvas, counter, fit/1:1, arrow navigation, facts line (B) — SL-B18..B20
- [x] Detail lookahead strip + 64-bin luminance histogram (B) — SL-B21
- [x] Shortcut overlay + keymap wiring (B) — SL-B23: `crates/ui/src/keymap.rs` is the table, checked against the command registry in both directions
- [x] One persisted, reversible undo level + the change viewer; the history is now 20 steps of undo and redo (A) — SL-D10/H04
  - `Catalog` keeps up to `UNDO_DEPTH` (20) inverses, oldest first; a 21st step is dropped from the bottom, not
    silently unbounded. `redo` walks the same steps forward, and a new change abandons the redo branch — stepping
    forward again would replay a past that no longer matches what was just written.
  - The history travels through a restart: the snapshot now carries `undo_history` (oldest first, still writing the
    old single-level `undo` field for an older reader), and the log replay after it appends the rest, so a reopen
    keeps twenty steps. Redo as a stack is per-session; after a restart the same steps are reachable by undoing the
    undo entries, which the log holds in order — the pre-existing "an undo is never a one-way door" behaviour.
  - `edit.redo` is in the command registry and the MCP examples; `Ctrl+Shift+Z` is in the keymap and the shell.
    egui's shortcut matching ignores extra Shift and Alt, so the shell consumes Ctrl+Shift+Z before Ctrl+Z — the
    headless test drives both through real key events and catches the undo branch firing for redo's press when the
    order is wrong. An older snapshot (single `undo` field) still opens with its one step.
- [x] `photo.reveal` / `photo.open`: argv built in `sortlens-core::reveal` (pure, per-desktop, tested), executed by the app and the CLI (A) — SL-B22
- [x] XMP round-trip and filter-algebra tests are in; stale-revision and badge-contrast tests are in too
  (closed by Phase 1 item 2): the stale-revision tests cover the catalog guard, the per-photo `rev`, its
  survival of a reopen, and the engine's `expect`/`rev` round trip; the badge-contrast test judges every
  preset's cell badges and flag dots at 3:1 (WCAG graphics) against `thumb_bg` — which is what found the
  real defect: Silver and Paper's faint grey sat at 2.25 and 2.67 to 1 against their own thumb wells, and
  both are darker now

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
- [x] Folders in the sidebar: remembered sort and search (SL-A05..A10 part 3)
  - The mark gained `sort` and `search`, so a folder carries how it was being browsed. **The decision the row
    asked for**: a sort belongs to the folder when a folder is the scope, and to the library otherwise — a
    folder with nothing remembered follows the library's sort rather than snapping back to Natural, because
    losing the sort you chose a moment ago is the more annoying failure. Search is remembered on the same rule
    as collections (committed when the box is left or Enter is pressed, never per keystroke) and comes back into
    the search box rather than filtering behind your back, so it is visible and one click from being cleared.
  - `folder_scope(mark, library_sort) -> (Sort, String)` is the rule as a pure function, and the row's click
    runs exactly it. `folder.mark` gained `sort` and `search` (omitted keeps, null clears), and `folder.marks`
    reports both.
  - Tested: the rule (marked/unmarked/sort-only), the mark round-tripping with all four fields, a mark written by
    an older build with only the first two fields still parsing, and the mark surviving one undo. **Not verified
    by hand**: whether restoring the search into the box on folder open feels helpful or intrusive.
- [x] Collections: create/rename/delete, add/remove, sidebar, filter (A,B) — SL-G01..G03
- [x] Collection favourites, colours, per-collection search and sort (B) — SL-G04
- [x] Batch move with sidecars travelling, receipts and undo (A) — SL-G07
- [x] Persistent undo across restarts: the inverse travels in the log record and in the snapshot (A) — SL-A13
- [x] Inline tagging + re-apply last tags in cull (B) — SL-C11/C12
  - The cull stage gained an inline keyword strip: chips for the keywords the photo on screen carries
    (click takes one off, one undoable op) and a field that adds one with Enter, so keywording happens
    while the same photo is on screen instead of a round trip through the sidebar. The field owns the
    keys while it is focused — `keyboard()` stands down for any focused widget, so P and T type letters,
    not decisions. `T` re-applies the keyword you last added to the photo on screen, the same one-key
    repeat the grid has, with the notice naming the keyword. Fixed on the way: the top-bar search
    evaluated `has_focus()` inside `ui.input`, which deadlocks the egui fork's lock whenever Enter was
    pressed anywhere — the inline field's Enter exposed it.
  - Tested: a headless `run_ui` session that keywords the photo on screen from the real field, decides
    it, presses T on the next, and removes the chip through a real click on its painted rect.
- [x] Library doctor: check, find-moved-by-content-key, remove-missing, all undoable (A,B) — SL-H07/H08/A13
- [x] First-run wizard: welcome → add a folder → indexing (B) — SL-I12; the remote test waits for M9
- [x] Tag rename/merge UI, plus aliases in the keyword registry (B) — SL-D06

**Demo:** three folders → two collections; move 200 files with hash verification and undo;
remap a moved drive; run the wizard on a clean machine.

---

## M6 — Analyse · 2.5 w · tracks D,B

- [x] `sortlens-quality`: sharpness score + versioning + tests (D) — SL-E01 **part 1**
  - The score is the variance of the 4-neighbour Laplacian of the luma channel over the squared mean luma,
    measured on a box-filtered grid no larger than 256 px on its long edge. Laplacian rather than edge energy
    (a steep ramp is not detail; a change of direction is), variance rather than a mean (one sharp edge in a flat
    frame must not carry the score), divided by mean² (the Laplacian scales with brightness, so a correct exposure
    and one a stop down must score alike), at a fixed analysis edge (a photograph is as sharp as the photograph,
    not as the copy of it you happen to hold). The number is unitless: it orders frames, it is not a percentage.
  - `Score { value, version }` with `SCORE_VERSION`, and `needs_rescore(stored)` as the single place that decides
    whether a stored score is comparable — none stored, an older version, or a non-finite value all mean
    "measure it again". A library half-measured by two formulas would sort wrongly with nothing looking wrong.
  - **A near-black frame is refused (`MIN_MEAN_LUMA`), not scored.** Proved by removing the guard: a 1-2 level
    noise frame then scored 1.78, *above* a sharp bar pattern's 1.41, so without the floor every night shot would
    top the sharpness list. "Cannot tell" is not "in focus".
  - Twelve tests, including two fixed-seed property runs (no property-testing dependency exists in the workspace,
    and adding one for two invariants would cost a package and its attribution). Both invariants were proved to
    bite by removing the normalisation and the darkness floor.
  - **Stated limit**: the fixed edge makes copies related by a whole-number ratio measure alike; a fractional
    reduction is averaged differently by the box filter and can shift the score somewhat. Correcting that means
    measuring at several edges, which is precision this ranking does not need.
- [x] Blur scan **part 2 of 3**: the score is stored, one undoable op per pass
  - `Op::Scores { changes }` carries the batch, so measuring a 5 000-photo library is **one log entry and one ⌘Z**,
    and the inverse is the previous scores — a re-measure at a new `SCORE_VERSION` undoes back to the old numbers.
  - The score lives on the `Photo` record, not in a side map, because it is a statement about *this file*: a scan
    that replaces the record because the bytes changed clears it, which the test found (the update path cloned the
    old record and kept every annotation, so a changed file kept a score about pixels that were gone — annotations
    like stars survive a re-encode, a sharpness score does not).
  - `Engine::begin_sharpness` queues only the photos `needs_rescore` says need it, decodes at `LARGE` on a worker,
    and `poll_sharpness` reports `{ measured, unjudged, remaining }`. **`unjudged` is a first-class count**: a file
    this build cannot decode and a frame too dark to judge are counted and left unscored, never stored as a number.
    A cancelled pass still commits what it measured, because each score is true on its own — unlike a cancelled
    scan, which commits nothing because a half-scanned folder describes a disk that is not there.
  - `photo.sharpness` / `photo.sharpnessProgress` in the registry, MCP examples, and a CLI `sharpen` pass. Goldens
    pin that the first run stores six scores and the second says "every photo has a current score".
  - **`sharpen` deliberately has no `--json` schema**, like `export`: a job with progress and a cancel is not one
    object. The schema gate now names the progress verbs and requires each to *refuse* `--json`, so the absence is
    a decision the harness checks rather than a hole.
- [x] Blur scan **part 3**: threshold review and batch select in the window
  - `crates/ui/src/sharpness.rs` holds the arithmetic and no egui: `review(scores, percentile) -> Review` with the
    histogram buckets, the cut, the ids at or below it, the tie count and `selected_bands`. A question that decides
    which photos get acted on belongs in a test, not only in a painter.
  - **The cut is a value at a percentile, not a rank.** Taking "the lowest N" would pick an arbitrary subset of a
    tie — the same photograph selected or not depending on its id. A value cut can select more than asked for, so
    `ties` and the count are shown; that is a bigger selection honestly described rather than a smaller one chosen
    at random.
  - **Identical scores select nothing and say why** (`separable: false`): a relative cut on a flat library would
    select every photo, and selecting everything is an action, not a review.
  - The section lives in Insights — the analysis view that already has the charts — with the measurement button
    (count of photos without a current score), the histogram (bands at or below the cut highlighted via
    `selected_bands`, drawn by the existing chart component), the slider, and a button that ticks the photos via
    `Selection::select_many` and returns to the library. The pass is polled in the view that asked for it, and the
    result is a notice that names how many could not be judged.
  - Tested: eight arithmetic tests in `sharpness.rs` (percentile, ties, flat library, empty, clamping, non-finite
    scores, band/cut agreement, histogram summing) and one headless paint over a real five-photo library that walks
    unscored → scored → all-equal and asserts the paint and the review agree. **Not hand-verified**: whether 10% is a
    sensible default, and whether the histogram is legible at real score ranges — both need a library of photographs.

- [x] `sortlens-hash`: dHash + aHash over a 9×8 box-filtered grid, Hamming distance, 3×3 colour fingerprint (D) — SL-E03
- [x] Similar groups (union-find chaining), best pick, similar-to (D,B) — SL-E04/E05
- [x] Similar: BK-tree metric index (exact, sub-quadratic) and a colour veto (D) — SL-E03
- [x] Similar: side-by-side compare and colour-only matches (B) — SL-E05
- [x] Batch select of group leftovers, and delete them (B) — SL-E05
  - "Select all but the best" selected **exactly one photo while announcing "N set aside"** — a quieter promise than
    the button makes, and the kind of lie that makes a batch delete dangerous. It now takes `group.others()` through
    `Selection::select_many`, and the group header names the photo it is keeping.
  - A batch bar adds "Select leftovers in every group (N)", and a "Delete the N selected…" action that opens a
    confirmation naming the count and the three modes (trash, rejects folder, tag only).
  - **There is one delete executor, and this row refactored rather than added one.** `Engine::apply_candidates` now
    holds the audit, the apply, the batch record and the flags; `apply_session` keeps the session bookkeeping and
    `delete_photos(ids, mode)` builds a set and calls the same function. Two ways to delete would be two places for
    the safety check to be wrong.
  - `photo.delete` is in the registry (so the window, MCP and the CLI all reach it), with `delete_mode_from` shared
    with `cull.apply`; CLI `delete <ids> --mode trash|tag|move [--target F]` and a `--json` schema. The golden step
    uses `--mode tag`, because a golden must not drop a file into the machine's real Trash.
  - Tested: three engine tests (one batch and one undo, a refused batch deletes nothing at all, an empty set is an
    error), a headless similarity test that the selection is every leftover and not just the first, and the golden
    JSON. **`Mode::Trash` is not exercised by a test** — it hands the file to the operating system's trash, and a
    test that moves a real file into the real user's Trash is a test that damages the machine it runs on; the batch,
    audit and undo machinery is covered through `Mode::Move`.
- [x] `sortlens-geo` **part 1**: projection, clusters and gazetteer — SL-E06/E07
  - `LatLon` is the only way to hold a coordinate, so nothing downstream has to ask whether it is real; the
    constructor refuses non-finite numbers, latitudes off the globe and longitudes outside ±180 rather than folding
    them onto it.
  - **Web Mercator, because tiles are Mercator** — one place for the maths, and a projection that agrees with every
    offline basemap and MBTiles archive. `project`/`unproject` round-trip to within a millionth of a degree at zooms
    0–18, checked at Greenwich, the antimeridian and the northern limit.
  - **Mercator has no pole, so a polar photograph is refused, not clamped.** Clamping would draw it at the top edge
    as if it had been taken at 85°N — a wrong statement about a real photograph instead of a missing one. Every
    returned point is inside the world square, so callers never check.
  - **Clustering is a 64-pixel grid and it is deterministic**: cells are sorted by count then cell, and a test feeds
    the same forty photos in two orders and asserts the same dots in the same order. This closes the ROADMAP row
    that was waiting for the feature ("cluster determinism waits for its feature").
  - The gazetteer is a **tab-separated file the user supplies**, parsed with comments and malformed lines counted
    rather than dropped silently; lookup is a one-degree cell index, nearest wins within a radius and ties break by
    name so file order cannot decide. This crate ships no place data and makes no network call, which is why the
    data's attribution can stay where its licence can be read.
  - Eleven tests, including the great-circle distance (London–Paris ~344 km rather than 3.5 "degrees") and the
    refusal cases. **Not verified by hand**: nothing draws yet — see the dependency below.
- [x] Places **part 2**: GPS in the catalog, and a view that tells the truth about what it has
  - `sortlens-meta` now reads `GPSLatitude`/`GPSLongitude` into a `sortlens_geo::LatLon`, and `Photo` carries it, so a
    scan fills it in and a re-read replaces it. Tested end to end with a JPEG whose EXIF APP1 block is written by the
    same library that reads it: JPEG → EXIF → degrees/minutes/seconds → a coordinate, southern-hemisphere sign intact.
  - **A device with no fix writes 0, 0, and that is a real coordinate.** Taking it at face value would put every
    photo taken before the receiver locked in one dot in the Gulf of Guinea, which reads as a data disaster rather
    than a missing tag, so an exact 0, 0 is treated as "no location". The cost of being wrong is that a photograph
    genuinely taken at Null Island is left off the map; the other choice lies about hundreds.
  - **A stored coordinate cannot be an impossible one**: `LatLon` deserialises through its constructor, so a corrupt
    snapshot is a load error rather than a photograph drawn in the wrong ocean.
  - `Engine::geotagged`, `located` and `places(zoom)`; the Places view reports the counts, the biggest clusters as
    latitude/longitude rows, and says in the view that the coastline is the next piece of work. **No dots on a blank
    rectangle** — a field of dots with no coastline looks like a map and tells you nothing about where anything is.
  - **Two stale nav gates removed**: `Cull` still claimed "M4" and `Places` "M5" in the bottom bar, closing working
    views behind a label that had gone out of date. A view that is partly built now says so inside itself.
  - A grid's known limitation is pinned as a decision rather than a surprise: two photos a few hundred metres apart
    that fall either side of a cell edge are two dots (`two_photos_either_side_of_a_cell_edge_are_two_dots`). The
    alternative — chaining neighbours until everything within a radius is one dot — is not deterministic from frame to
    frame, and a dot that jumps when a photo is added is worse than a dot occasionally in two pieces.
  - `Testkit::jpeg_with_location` writes a GPS block into any fixture, which is what makes the whole path testable.
  - **Not verified by hand**: how the counts look at real library scale, and whether `zoom 6` is the right summary.
- [x] Places **part 3a**: the basemap decision, made and implemented — **a folder of tiles the person supplies**
  - **The decision**: `{z}/{x}/{y}.png` in a `tiles/` folder beside the catalog, with an `attribution.txt` beside it.
    Chosen over a bundled *real* basemap — satellite or street tiles need map data the project cannot ship free of
    charge or licensing. MBTiles was deferred here for one reason: it needs a SQLite reader in the dependency tree
    for one format, and a folder covers the same ground without one. That reading shipped later (see the unchecked
    row below). The layout is what falls out of unzipping any tile archive, so the tiles somebody can legally use are
    the tiles that work. The one world outline this project *can* ship is the public-domain Natural Earth 1:110m
    coastline, which arrived later as the starter basemap: 3B.6 (c). A tile folder still replaces it, and a
    `tiles.mbtiles` archive does too when there is no folder.
  - **The folder is part of the library, not a global setting**: two libraries can have different basemaps, and moving
    a library brings its map with it.
  - **`sortlens_geo::tiles::Tileset::open` refuses a folder without a credit**, with a message naming the file to
    write and saying why. An unlabelled map implies the application made it; a map without its licence line is not
    shippable, so the refusal is at the type level — no state exists in which the view holds a usable tileset whose
    attribution is unknown. A folder with a credit and no tiles is refused too: it would promise a map and paint
    nothing.
  - Tile presence is **checked once per request, not discovered per frame**: `tile_path` answers `None` for a hole, so
    the view paints around it instead of failing an open on every repaint.
  - The zoom range is read from the directory names, **bounded at 30**: `255` parses as a `u8` and would be accepted
    as a zoom level, letting a view offer a billion billion tiles that are not there. A test asserted 255 was refused
    and failed, which is how the missing bound was found.
  - `TileError::message()` for every refusal, `Engine::tileset()`/`tiles_dir()`, and the Places view showing the
    range, the tile count and the credit — or the sentence explaining which state the library is in.
  - **Not verified by hand**: any real tile set, and whether the "no basemap" copy reads well at the view's width. The
    painted words are not asserted — egui hands back shapes, not text — so what the test proves is that the view
    paints in all three states and that the engine reports the right state; the credit guarantee is enforced one layer
    down, where `Tileset::open` refuses to exist without one.
- [x] Places **part 3b**: drawing it — the tiles under the dots, and a dot you can click
  - `sortlens_geo` gained the viewport maths as pure functions: `viewport_origin`, `screen_position`,
    `visible_tiles`, `tile_rect`. **The world does not wrap**: a map that repeated the antimeridian would draw the same
    photographs twice with nothing to say which was which, so tile columns clamp to the world square and a viewport
    wider than the world shows empty space beside it.
  - `crates/ui/src/places.rs` holds the parts worth testing without a window: `dot_radius` (square-root growth, so
    area tracks the count and a dot never swamps the coastline), `dot_at` (the **largest** dot within reach wins, not
    the nearest, because where a big dot overhangs a small one the person is aiming at the big one), `clamp_zoom`
    (zooming past the top of a set clamps: the tiles stop, the dots keep going), and `TileCache` — decoded once,
    bounded at 512 entries, with two kinds of absence kept apart: a corrupt tile is counted as a failure, a cell the
    set does not carry is a **hole** that is remembered (so it is not looked up every frame) but is neither a decode
    nor a failure. Conflating them made a partial map look corrupted — it is what turned `CI` red on 2026-11-26.
  - **Two real bugs, both found by tests I wrote expecting the correct behaviour:**
    1. The view built its dot list by skipping clusters it could not place and then indexed the cluster list by
       *position*, so one off-screen cluster shifted every dot onto its neighbour's photographs. `Dot` now carries its
       cluster index; `a_dot_points_at_its_own_cluster_even_when_another_is_off_screen` pins it.
    2. The hit test compared the pointer, in screen coordinates, against dot positions relative to the map's own
       corner — every click missed by the panel margin. Found by `clicking_a_dot_selects_its_photos`, which drives
       real pointer events through `run_ui` and finds the map by the rectangle the view fills, so it does not have to
       guess where the view put anything.
  - The map is centred on the **mean of the library's own photos**, not on a place name or an atlas corner: opening
    Places should show where these photographs are. Clicking a dot selects its members and says where they are;
    clicking the sea selects nothing and clears nothing. The zoom controls sit over the map corner, disabled at the
    ends of the set.
  - The credit is printed **over the map, every frame it is drawn**, and the map is a named `Role::Canvas` region for
    assistive technology (`cargo xtask a11y` demanded it, which is the gate doing its job).
  - A missing or undecodable tile is painted as background, never an error toast: a partial tile set is a normal
    thing, and a notice per gap would bury the map in noise.
  - **Not verified by hand**: how it looks. The tests prove the tiles decode once, the dots carry the right clusters,
    the click resolves, and the credit is the folder's own; they cannot say whether the coastline sits in the right
    place, because there is no tile set in the repository to compare against — that needs a real archive and eyes.

- [x] `sortlens-hash`: dHash + aHash over a 9×8 box-filtered grid, Hamming distance, 3×3 colour fingerprint (D) — SL-E03
- [x] Similar groups (union-find chaining), best pick, similar-to (D,B) — SL-E04/E05
- [x] Similar: BK-tree metric index (exact, sub-quadratic) and a colour veto (D) — SL-E03
- [x] Similar: side-by-side compare and colour-only matches (B) — SL-E05
- [x] Batch select of group leftovers, and delete them (B) — SL-E05
  - "Select all but the best" selected **exactly one photo while announcing "N set aside"** — a quieter promise than
    the button makes, and the kind of lie that makes a batch delete dangerous. It now takes `group.others()` through
    `Selection::select_many`, and the group header names the photo it is keeping.
  - A batch bar adds "Select leftovers in every group (N)", and a "Delete the N selected…" action that opens a
    confirmation naming the count and the three modes (trash, rejects folder, tag only).
  - **There is one delete executor, and this row refactored rather than added one.** `Engine::apply_candidates` now
    holds the audit, the apply, the batch record and the flags; `apply_session` keeps the session bookkeeping and
    `delete_photos(ids, mode)` builds a set and calls the same function. Two ways to delete would be two places for
    the safety check to be wrong.
  - `photo.delete` is in the registry (so the window, MCP and the CLI all reach it), with `delete_mode_from` shared
    with `cull.apply`; CLI `delete <ids> --mode trash|tag|move [--target F]` and a `--json` schema. The golden step
    uses `--mode tag`, because a golden must not drop a file into the machine's real Trash.
  - Tested: three engine tests (one batch and one undo, a refused batch deletes nothing at all, an empty set is an
    error), a headless similarity test that the selection is every leftover and not just the first, and the golden
    JSON. **`Mode::Trash` is not exercised by a test** — it hands the file to the operating system's trash, and a
    test that moves a real file into the real user's Trash is a test that damages the machine it runs on; the batch,
    audit and undo machinery is covered through `Mode::Move`.
- [x] Places **part 4**: dots borrow a name from a gazetteer the person supplies — SL-E07
  - `Gazetteer::load` reads the file with the size checked **before** the bytes are: `MAX_GAZETTEER_BYTES` is 64 MB,
    enough for a four-million-place world list and small enough that pointing the app at a disk image is refused
    rather than read into memory. `GazetteerError::{Missing, TooLarge, Unreadable}` each carry a sentence naming the
    file and what to do.
  - **A file with no places is not an error.** `is_empty`/`skipped` report it, because a gazetteer with three
    malformed lines is still a gazetteer, and a blank line of tabs is blank rather than broken — a test asserted
    otherwise and was wrong.
  - `Engine::places_file`/`load_gazetteer`: `places.txt` beside the catalog, the same shape as the tile folder. The
    engine hands back the path and a loader; it deliberately does not cache, because the thing that must not happen is
    a view parsing tens of megabytes to draw a frame.
  - `places::Names` is the cache: size plus modification time as the stamp, re-read only when that changes.
    **A file that disappears stops naming anything** rather than keeping the last names it had — a place list that
    outlives its file puts names on a map whose authority the person deleted — and a missing file is *not* reported as
    a fault, because most libraries have none and a red line for that teaches people to ignore red lines.
  - `places::LABEL_KM` (30 km) decides how much slack a match gets; `place_label` returns `None` rather than a guess;
    `selection_notice` has two forms — "Selected 42 photo(s) near Brighton (50.82N 0.14W)" and "Selected 1 photo(s) at
    25.00N 40.00W" — written as one function so the named and unnamed forms cannot drift, and both say the coordinate
    so a name is never mistaken for a finer resolution than the file has.
  - A dot with a name prints it below the dot; the footer states whether names are in use (`Named from places.txt ·
    N place(s)`) or why they are not.
  - Tests: the file reader (good, missing, too large via a sparse file, empty), the naming rule over named/unnamed and
    both hemispheres, the cache's reload/stamp/disappearance/unreadable behaviour, and a headless run over two real
    libraries. **Not verified by hand**: how a dense gazetteer looks — overlapping labels are the file's density
    problem and there is no declutter pass yet — and how a real GeoNames export performs on load.
- [x] Offline basemap + attribution (B) — SL-E06, part 3a above: a tile folder with a mandatory credit line
- [x] Optional MBTiles reading (B) — SL-E06 — **shipped 2026-11-26**: `tiles.mbtiles` beside the catalog is read through a read-only SQLite connection (`rusqlite`, bundled, MIT), with `tile_row` converted from TMS to the XYZ row every other reader here uses. The credit comes from `metadata.attribution`, or an `attribution.txt` beside the archive; an archive with neither is refused with the same message rule as a folder. A `tiles/` folder still wins when both exist, and the archive is opened per lookup so nothing is held open between frames.
- [x] Insights: totals, rating/label/kind distribution, cameras, lenses, keywords, timeline — SL-E08
- [x] Insights: worker with progress, highlights, months and seasonality (B) — SL-E09..E11
- [x] Memories: `sortlens-memories` (on this day, this week in past years, never looked at), For You view, `memories.list`, CLI `memories --today` (D,B) — SL-E12
- [x] Chart component (fixed height, no chart dep): `crates/ui/src/charts.rs` — columns with a one-pixel floor and a round axis maximum, proportional rows, painted headlessly in a test (B) — SL-E11
- [x] Duplicate-transform and BK-tree cross-check tests are in, and the blur ladder is checked: `crates/quality`'s
  `a_blurred_frame_scores_far_below_the_sharp_one_it_came_from` blurs a known pattern and asserts both the separation
  and the ordering, so "more blur scores lower" is pinned rather than assumed
- [x] Map cluster determinism: `sortlens-geo::cluster` is a sorted grid, and the test that reverses the input photo order and asserts identical clusters is where it is pinned

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

**State:** the no-weights half is built — export (Original, JPEG and PNG), manifest, store, NMS, label map, EXIF
hints, packs, the suggestion panel, accept-and-undo and the batch pass. **M8 is not closed**: the real local
inference rows (detector, embeddings, downloader) and tag provenance are in **Phase 2** below, where the
detector, the embeddings and the consent-gated downloader have shipped and provenance closed on 2026-11-23.
The owner has signed off on pinning permissively licensed weights and on a user-initiated
download, so the earlier "needs weights we cannot ship" blocker no longer applies.

- [x] Model manifest (id, kind, licence, digest, size) + content-addressed store that verifies size
  and SHA-256 before use, installs atomically and refuses to delete a file it cannot identify (D)
  — SL-F01; real weights (YOLOS-tiny, CLIP ViT-B/32 pair) pinned in `Manifest::shipped()` 2026-11-23,
  Phase 2 item 1; the **downloader** (SL-F02) shipped 2026-11-23, Phase 2 item 2 below
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
  forward, four collision policies, worker progress and cancel, a dialog and a receipt (A) — SL-B24;
  **PNG export** (lossless, alpha carried, licence-approved encoder through `craft-image-codecs`) shipped
  2026-11-23 — Phase 1 item 4

### Re-opened from M8

The three rows this section used to hide — the `tract` detector session (SL-F03 part 2), the image/text
embedding sessions and prompt packs (SL-F04) and `source = Ai` filtering via per-tag provenance (SL-D12) — are
now unchecked rows in **Phase 2**. PNG export (SL-B24) is closed: it shipped as Phase 1 item 4 on 2026-11-23,
and `source = Ai` filtering (SL-D12) is closed too: it shipped as Phase 2 item 4 on 2026-11-23.

**Demo:** on a clean machine, index 50 photos, run `batchsuggest`, accept ten proposals from the panel,
undo one, switch on a keyword pack and watch its keywords appear for the matching folders; export a
selection as originals and as JPEG and diff the metadata.

---

## M9 — Remote + automation · 2.5 w · tracks E,B

**State:** the automation half is in — the MCP server, `--json` on the read verbs and the diagnostics
bundle. The remote server shipped 2026-11-24 (pairing, scoped tokens, the allow-listed command API,
resumable uploads — see the Phase 4 item 3 row), the desktop's own server start/stop shipped the same
day, and the device protocol shipped 2026-11-24 with it (`sortlens_remote::client` +
`Engine::open_memory`, SL-J02's engine and protocol halves — see the Phase 4 item 3 row) while the
native phone app (`apps/sortlens-mobile`) stays open in Phase 4, as is the rest of i18n (the scaffolding shipped 2026-11-24 with
Arabic, Japanese and the direction plumbing — see the Phase 4 row — while zh-TW/ko/th/vi, the unrouted
surfaces and the RTL screenshots stay open); the keymap editor shipped 2026-11-24 (see the Phase 4 row),
and none of M9 is closed until the remaining rows are ticked.

- [x] MCP server: `sortlens-mcp` — newline-delimited JSON-RPC over stdio, `tools/list` straight from the command registry, `tools/call` through `Engine::run`, `sortlens-cli mcp` (E) — SL-K05
- [x] CLI parity and stable JSON schemas: `--json` on the read verbs (roots, keywords, stats, list,
  folders, check, moves, changed, history, audit, memories, shortcuts, registry, packs, suggest, jobs)
  prints the engine's own answer, a verb without a schema says so and lists the ones that have one
  instead of printing prose, and the golden files pin every schema and its answer plus the refusal (E)
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
- [ ] i18n catalogs + RTL screenshots (ar/ja/zh-TW/ko/th/vi) (B) — SL-L07 — the scaffolding is in:
  embedded ar/ja catalogues, Settings → Language, the reading-start/end panels and the mirrored rows
  (see the Phase 4 item 2d row). The Welcome card, the indexing screen and the empty-filter screen
  went through the catalogues on 2026-11-26 (seventy-sixth entry), so a first launch and a filter
  that hides everything read in the chosen language; the export dialog and its receipt followed
  later that day (seventy-eighth entry) by describing a spec from `sortlens-export`'s own
  `Spec::parts`/`Report::parts` instead of its English sentence, so a Japanese dialog writes
  "品質 82" about the same spec an English receipt reports on. The three screens that destroy files
  followed (seventy-ninth entry): the similar-view delete, the cull batch's Apply and the
  rejects-bin emptying word their counts, their modes, their changed-on-disk warning and their
  toasts through the catalogues, and the catalogue audit now reads `fmt` templates as well as `tl!`
  literals, so a template with no row fails the test instead of showing English. The quit guard
  followed (eighty-fourth entry), worded from `sortlens-engine`'s own `UnsavedReason` pieces rather
  than its English sentences, so the guard and the support report cannot describe different work.
  The remaining dialogs went through too — moves, changes, suggestions, shortcuts and About — and the
  shortcut editor's own chrome is the last surface (ninety-first entry): `CatalogText` implements
  `craft-keymap`'s `Text` trait, so the crate's 175 dialog strings, its eleven `@plural` counts and
  its read-only fixed rows paint from the same catalogues, and the crate's own `STRINGS`/`PLURALS`
  lists are the audit that keeps a new crate string from arriving in English only.
  The cull screen's own chrome followed (ninety-fifth entry): the header's position line, the
  pick/reject/defer/clear buttons, `Summary`, `End session`, `Up next`, `Session complete`, `Apply
  rejects…`, `Move picks…`, `Export picks…`, `Back`, the session's counts line, the empty state's
  sentence and start button, the keyboard hint, the keyword field's hint and both keyword toasts are
  catalogue rows, and the position and count sentences are templates — so a Japanese cull session
  reads `3 中 1` with `採用  P`, and an Arabic one writes `١ من ٣`. What is still English on that screen
  is what assistive technology speaks (`crate::a11y`), which the row does not count as painted chrome.
  The Similar view's chrome followed (ninety-sixth entry): `Find similar photos`, `Hashing…`, the progress
  line, `Match`, the strictness and palette sliders, the leftover/delete/clear row, both empty-state lines, the
  group headers and every select-all hover are catalogue rows, and their counts are templates — so a Japanese
  group reads `写真 3 枚` with `各グループの余りを選択 (2)`, and an Arabic one writes `٣ صور`.
  Its mode picker followed the next day (ninety-seventh entry): `Structure`/`Colour`/`Either` and their three
  hint sentences come from `sortlens-similar`'s own `MatchWord`/`MatchSense` pieces — the enum its English
  `label`/`hint` are built from — so a language names the mode without the shell retyping the sentence, and
  `every_similar_mode_translates_the_engines_own_piece` fails if a literal drifts from the crate's.
  The Insights page followed (ninety-eighth entry): its `Recount` button and its hover, the `Counting`
  line and the `{done} of {total}` template, the seven figures, the nine column headings, the three
  kinds, `Unrated` and both empty states are catalogue rows, and the undated count is a template — so a
  Japanese page reads `評価`, `タイムライン` and `日付のある写真はまだありません` where it read English.
  The highlights strip followed (ninety-ninth entry): `sortlens-insights` now answers a
  `HighlightKind`, a `Month` and a `Season` as values beside the counts, so the strip's labels, the
  month and season words and the `photo(s)` sentences are catalogue rows and the
  `Busiest: … · mostly …` line is worded from the same pieces — nothing on that page is English by
  omission. The busiest day's date now goes through the shell's `format::date`, so Arabic writes
  `١٠/٠٢/٢٠٢٤`.
  The Places view followed (hundredth entry): its empty state's two lines, the `zoom` label, the footer
  counts, both `places.txt` lines, the basemap note and the tile-folder instruction are catalogue rows, and
  the counts are templates — so a Japanese map reads `位置情報のある写真 3 枚 · 1 地点` and an Arabic one
  writes `٣ صورة بموقع`. What stays English on that view is the click notice and the tile refusal's own
  sentence (`TileError::message()`, an engine word with no piece yet), plus the map's assistive name, which
  the row does not count as painted chrome.
  The Detail view followed (hundred-and-first entry): its `Nothing selected` state and `Back to the grid`
  button, the `Previous`/`Next` controls, `Lookahead`, the keyboard hint and the zoom/compare row
  (`Fit  Z`, `1:1  Z`, `Swap  S`, `Close compare  C`, `Compare  C`) are catalogue rows, and its position
  line (`{position} of {total}`) and the Details tab's rating (`{rating} of 5`) are templates, so an Arabic
  photograph reads `١ من ٢`. The inspector's Details tab is catalogued end to end: every row name
  (`Kind`, `Folder`, `Size`, `Dimensions`, `Camera`, `Lens`, `Exposure`, `Captured`, `Rating`, `Flag`,
  `Label`, `Status`), its kind values (`Raw`/`Image`/`Video`), its flag values (`none`/`pick`/`reject`),
  its state values (`missing`/`present`) and its empty-state hint are rows, so a Japanese inspector reads
  `評価` where it read `Rating`. What stays English on the view is the facts line (a camera, an exposure
  and a size are values) and its assistive names, which the row does not count as painted chrome.
  The Compare view followed (hundred-and-second entry): its facts row — the six fact names (`stars`,
  `label`, `exposure`, `length`, `size`, `pixels`), the join (`{left} vs {right}`) and the pixel pair
  (`{left_width}×{left_height} vs {right_width}×{right_height}`) — is catalogue rows and templates, and
  the ratings are `format_count` digits, so an Arabic compare reads `٥ مقابل ٣` where it read `5 vs 3`.
  The Soft Focus view followed (hundred-and-third entry): its title and subtitle, the hint that says what a
  relative cut can claim, `Sharpness`, `Measuring`, the `{done} of {total}` progress line, `Measure sharpness`
  and its hover, the `{n} photo(s) have no current score` line, `Nothing measured yet…`, the histogram caption
  `Softest {percent}% · scores {low} to {high}`, the review's own words (`Nothing is scored yet`, the tie and
  cut sentences), `Select {n} photo(s)` and its hover are catalogue rows and templates, and the counts, the
  percentage and the two scores go through `format::count`/`format::decimal`, so an Arabic review reads
  `الأضعف ١٠% · الدرجات من ٠٫٥٠ إلى ٤٫٥٠` and `تحديد ١ صورة`. What stays English on that view is its notices
  (`Measured … photo(s)`, `Selected the … softest`) and the slider's assistive name, which the row does not
  count as painted chrome, plus the bottom bar's destination names, which still come from `View::label()`.
  The For You view followed (hundred-and-fourth entry): its title, its hint and its empty state are rows,
  and the `N card(s): …` headline, a card's heading and its detail line are worded from `sortlens-memories`'
  own pieces (`Memories::headline_parts`, `Card::title_parts`, `Card::detail_parts`) rather than its English
  sentence — so the `June 24` heading is a month row plus the language's digits (`6月24日`), the counts are
  `{shown} of {total} photos` filled through `format::count`, and `since {year}` is its own clause. An Arabic
  card writes `١ صورة · منذ ٢٠٢٠` where it wrote English. What stays English on that view is each photograph's
  own file name, which is a value, and the bottom bar's destination names above.
  What stays English there is each side's own value (a colour token, the engine's exposure sentence,
  `format_bytes`) and the notices that open a compare.
  Open: the other four languages (zh-TW/ko/th/vi), which wait for a translation pass, and RTL
  screenshots, which need a font build with an Arabic face
- [x] Localised numbers/dates (B) — SL-L08 — **done 2026-11-25** (Phase 4 item 2e): `crates/ui/src/format.rs`
  writes the shell's counts, file sizes and dates the way the interface language writes them — English
  keeps its Latin digits, thin-space grouping and ISO date exactly, Japanese groups with commas and
  writes `2026/10/03 04:00`, Arabic writes Arabic-Indic digits with `٬` grouping and `٫` decimal, and
  dates day-first (`٠٣/١٠/٢٠٢٦ ٠٤:٠٠`). The style table is held to the picker by a test, so a language
  added without its numbers and dates fails rather than painting English shapes. The formatting region
  is the interface language, not the operating system: another machine's regional settings cannot move
  a committed scene.
- [x] `craft-keymap` editor + import/export (B) — SL-I09 — **done 2026-11-24**, see Phase 4 item 1: the
  rebindable catalog is the single-chord command rows of `crates/ui/src/keymap.rs`'s own table (the two
  flag rows are one command with two parameterised refs), Settings → Shortcuts opens craft-keymap's dialog
  over it, overrides live in settings as named sets, and the palette and the `?` overlay paint the current
  chords, so a rebind cannot leave the help lying
- [x] Diagnostics bundle: `Engine::diagnostics` writes plain text with versions, library counts, the
  read-only health check, packs, the last 25 changes and the root list, with **every absolute path
  replaced** by `<library>`, `<root-N>` or `<home>` (longest match first, so a root under the library is
  not half-replaced); `diagnostics [--out FILE]` in the CLI, `app.diagnostics` for MCP, a golden, and an
  end-to-end test that reads the output *before* normalisation and fails if either path leaks (E)
  — SL-I14

### Re-opened from M9

`sortlens-server` (token auth, pairing, upload resume — SL-J01/J03/J04) and remote collection create and
rescan (SL-J06/J08) shipped 2026-11-24, proved by the two-process loopback integration harness the old
table's blocker asked for; the device protocol (SL-J02's engine and protocol halves plus
SL-J05's client-side ack) shipped 2026-11-24, leaving the native phone app as the remaining half of
Phase 4 item 3.
`docs/ARCHITECTURE.md` §10 keeps the design; the command registry is already the API.

**Demo:** an agent runs a full cull and report through MCP in CI; a script reads every verb's answer with
`--json`; `sortlens-cli diagnostics` produces a bundle a maintainer can read without knowing where the
photos live; the UI renders correctly in Arabic and Japanese.

---

## M10 — Release · 2.0 w · all tracks

**State:** the version contract, `--version`, the release workflow, the packaging notes, the measurements,
the icon, the attribution gate and the first half of the accessibility audit are in. Signing, installers,
the emitted accessibility tree, the operating system's own reduced-motion preference, theme import and the tag
are not — each named with its unblocker in the table at the end of this milestone. The `.icns`/`.ico` files
shipped in Phase 4 item 2a (`cargo xtask icons`), the High-Contrast preset in item 2b, and the opt-in
`craft-update` toggle in item 2c.

- [x] Accessibility audit, part 1 — **names**: every custom-painted clickable region in the shell (grid
  cell, detail preview, lookahead strip, cull strip, similar clusters, memory cards, sidebar rows) now
  registers a `WidgetInfo` with a description from `a11y::describe_photo`, and the row carries its label
  and count; `cargo xtask a11y` fails when a function makes a region clickable without naming it, proved
  to bite by deleting one registration (B) — SL-L01
- [x] Application icon: one drawing in `crates/icon` (code, not a dropped-in binary) rendered two ways —
  the window icon as RGBA through a build script, and `assets/icon.svg` as the committed, human-viewable
  copy — with the SVG held to the drawing by a test that fails when either changes and rewrites it only
  under `SORTLENS_UPDATE_ICON=1`; the app now sets `with_icon`, which it previously did not do at all, so
  the SVG existed and nothing loaded it. Proved to bite by appending a line to the SVG. The packaging
  files (`.icns`/`.ico`) have since shipped through `cargo xtask icons` (Phase 4 item 2a); the rest of
  the brand pass (marketing assets) and the installer work that consumes them remain in the table
  below, because a plausible-looking icon in a window is not a brand (B) — SL-I11
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

Rows that need something this repository genuinely does not have (an identity, a toolchain, a screen reader, a
person's decision). Anything buildable lives in "Completion phases" below instead. The packaging icons
(`.icns`/`.ico`) shipped in Phase 4 item 2a; the opt-in update check shipped in item 2c.

| Not built | Because | What would unblock it |
| --- | --- | --- |
| Signed and notarised macOS builds (SL-I12) | notarisation needs an Apple Developer identity and a CI secret; a workflow that pretends to sign would produce a bundle Gatekeeper rejects | an identity, `APPLE_ID`/`TEAM_ID`/app-specific password in CI secrets, and a release run to test against |
| Windows MSI and Linux AppImage/Flatpak | each needs its own toolchain (`cargo-wix`, `linuxdeploy`, Flatpak manifests and a runtime) and, for MSI, a code-signing certificate | those toolchains in the release workflow and a signing certificate |
| Theme import via `craft-ext-core` palettes (SL-I06) | importing someone else's palette means committing its colours and its licence; no palette has been chosen, and inventing one is not an import | a chosen palette with a licence that permits redistribution |
| The emitted accessibility tree (SL-L01..L05 part 2) | the names and roles are set and checked (`cargo xtask a11y`), but this egui version has no public way to read the tree back and `Context::has_accesskit_node` answers `true` even for an unnamed area, so a headless test would assert nothing; focus order and traps need a real client | a screen reader — VoiceOver, Orca or NVDA — on a machine with a desktop session, and a person to listen |
| The operating system's reduced-motion preference (SL-L06 part 2) | the reduced-motion *toggle* exists (`appearance.motion`) and the animation honours it, but the OS preference itself is a platform signal egui does not expose across platforms | a per-platform "reduce motion" query, or a future egui accessor |
| Release notes and a 1.0 tag | a tag is the repository owner's act, and notes for a version that no one has installed yet would describe intentions rather than a release | the owner tagging, and feedback from a run of the artefacts |

**Demo:** install from each artifact on a clean machine; first-run wizard; a11y tree has zero
unnamed controls; every budget in `FEATURES.md` §M passes.

---

## Completion phases

Re-opened by the owner's course correction (`docs/sortlens_agent_completion_directive.md`). **M0–M10 are not
"done" while any row below is open.** One vertical slice per commit; every slice lands with a test that fails
when the behaviour is absent, ticks its row here, and updates `CHECKPOINT.md` in the same commit. Gates per
commit: `cargo xtask ci`, `snapshot`, `deny`, and `mobile` when `crates/core`, `crates/ui` or phone-facing
crates change.

Owner sign-offs (Phase 0): user-initiated, opt-in network calls for `model.install` and `craft-update` only —
`crates/ai` stays offline with its `no_network` test; real permissively licensed (MIT / Apache-2.0 / public
domain) model weights and offline map data, or an opt-in downloader for them, attributed in
`docs/ATTRIBUTION.md`; licence `MIT OR Apache-2.0`.

### Phase 0 — Re-scope blockers and fix stale docs

- [x] Re-open the deferred core features as tracked rows (this section) and stop the M8/M9/M10 "Deliberately
  absent" tables hiding buildable work
- [x] `docs/RELEASE.md` no longer claims there are no HEIC thumbnails or no map; it lists the real gaps with their phase
- [x] Workspace dependency audit: 13 unlinked pins with no planned consumer pruned from `Cargo.toml`
  (`craft-flags`, `craft-fonts-build`, `craft-menu`, `craft-quit`, `craft-dialogs`, `craft-gainmap`,
  `craft-sniff`, `craft-zip`, `craft-crs`, `craft-webp`, `craft-ipc`, `chrono`, `log`); six kept with a named
  phase (`craft-video` P1, `craft-atomic` and `rayon` P2, `craft-keymap`, `craft-update`, `craft-i18n`
  P4); four linked crates (`craft-heif`, `craft-media`, `craft-jsonrpc`,
  `craft-mcp`) moved to the "Linked today" table, where they belonged

### Phase 1 — Core engine and media gaps (M1–M3 / M8)

- [x] 1. Video poster frames (SL-B11 part 2): first sync sample via `craft-video` (`h264-decode`/`hevc-decode`),
  `ColorTags`-aware YUV→RGB, cached through `sortlens-thumbs`, drawn in grid cells, detail and the cull stage;
  muxed-fixture round-trip test (C) — shipped 2026-11-22, see the M2 row above
- [x] 2. Stale-revision rejection (SL-D11): `Error::StaleRevision` on conflicting mutations in
  `sortlens-catalog`/`sortlens-engine`, plus the stale-revision and badge-contrast tests that close M3's test row (A)
  — shipped 2026-11-23, see the M3 rows above
- [x] 3. `Recently Added` (SL-A14 part 2): an `added` timestamp on `Photo` with a backwards-compatible snapshot
  migration (never approximated from `mtime`), last 30 days, in catalog, engine and sidebar (A,B)
  — shipped 2026-11-23, see the M1 "Sources, part 2" row above
- [x] 4. PNG export (SL-B24): a licence-approved encoder (`cargo xtask deny`) in `sortlens-export`, offered in the
  export dialog and the CLI (A)
  — shipped 2026-11-23, see the M8 row above

### Phase 2 — Real local AI tagging (M8)

- [x] 1. Pin real models: a CPU-friendly ONNX object detector (YOLO-family or RT-DETR) and a CLIP/MobileCLIP
  image + text pair, with real URLs, byte sizes, SHA-256 digests and licences in `Manifest::shipped()` and
  `docs/ATTRIBUTION.md` (D) — shipped 2026-11-23: YOLOS-tiny (Apache-2.0, 1×3×512×800, 100 queries × 92
  classes + boxes) and the CLIP ViT-B/32 int8 image + text pair (MIT, 224 px / 77 ids, 512-d embeddings);
  concretized and shape-stripped derivatives of the Xenova ONNX exports, each verified to load and run
  under `tract-onnx` 0.23.10 before pinning, hosted in the `models-v1` release
- [x] 2. Consent-gated downloader (SL-F02): `model.install`/`model.remove` in `sortlens-engine` (never in
  `sortlens-ai`), resumable, progress and cancel in Activity, SHA-256 via `sortlens-ai::store`, status in Settings → AI (D,A)
  — shipped 2026-11-23: `crates/engine/src/models.rs` streams into `<library>/models/<digest>.<kind>.part`,
  resumes with `Range: bytes=<len>-` (a server that ignores the range is answered by a restart), verifies the
  finished part against the manifest before renaming it under its digest name, and reads the renamed file back
  once more; `sortlens-ai` gains only incremental SHA-256 and the streaming file check, and its `no_network`
  test still holds (the fetcher — `ureq` over rustls — lives in the engine, desktop-only). Consent is an engine state the app persists in Settings → AI (`ai.download_consent`), and the
  `model.install` command refuses without it from every face: UI, CLI (`models`, `model-consent`,
  `model-install`, `model-remove`) and MCP. Progress and cancel sit in the Activity stack as a `Model` job.
- [x] 3. Pure-Rust inference with `tract-onnx` (SL-F03/F04) — both halves shipped 2026-11-23; see 3a and 3b:
  - [x] 3a. The runtime and the detector path: `crates/ai/src/infer.rs` (desktop-only, offline;
    contract-checked loading, softmax over the YOLOS logits, the COCO-91 label map shipped with the
    slice, NMS through `detect::suppress`), wired into the engine's suggestion pass and the
    single-photo panel over 384 px previews, merged with hints and packs by `suggest::merge` (D)
    — shipped 2026-11-23, see the twenty-seventh entry; the CLIP image + text pair is loaded and
    contract-tested in the same runtime, and the synthetic-model test fixtures live in
    `sortlens-testkit::onnx`
  - [x] 3b. Per-pack text embeddings cached at install (CLIP BPE tokenisation of each pack's
    keywords) and image embeddings on the low-priority pool, matched against the packs and fed
    through `suggest::merge` (D)
    — shipped 2026-11-23, see the twenty-eighth entry: `crates/ai/src/tokenize.rs` runs the shipped
    upstream CLIP BPE vocabulary (`crates/ai/src/bpe_simple_vocab_16e6.txt.gz`, attributed in
    ATTRIBUTION.md); `crates/ai/src/text_cache.rs` keys each pack's keyword embeddings by the text
    model's digest and the pack file's content hash, and serves a fully cached pack without its
    text model; the engine's suggestion pass embeds a 224 px preview and proposes the pack
    keywords whose cached embedding clears `EMBED_MIN_COSINE` (0.25, set from the real pins' live
    numbers), through the same merge as every other source; the whole path was run once against
    the real downloaded pins (digests verified, tokeniser validated by the real text model's
    cosine ordering, a real cat photo ranking “a photo of a cat” first and the real detector
    answering `cat · 1.00` on it)
- [x] 4. Tag provenance (SL-D12): per-tag source `Manual`/`Ai`/`Xmp` in the catalog with a backwards-compatible
  snapshot migration, filterable in the UI and CLI (A,B) — shipped 2026-11-23, see the twenty-ninth entry:
  `sortlens_core::TagSource` (`manual`, `ai`, `xmp`, plus `unrecorded` for keywords that were in a library before
  sources were kept — the honest answer, not a guess of `manual`); `Photo::tag_sources` with
  `#[serde(default, skip_serializing_if)]`, so a snapshot written by an older build replays and rewrites exactly
  as it was, with no invented entries; a new additive `Op::TagSourced` committed in the same `Group` as the
  `Op::Tagged` it describes, so one ⌘Z takes the word and its origin together and the pre-existing 2-tuple op
  records still decode; `Engine::set_tag_from` (hand-typed is `Manual`, `accept_suggestion` is `Ai`, a sidecar
  review adopted at import is `Xmp`), with rename carrying the source to the new name and delete taking it away;
  `Filter::tag_source` answering "some keyword came from there" through `library.page`, the filter strip's
  Source chip in the window, `sortlens-cli list/page --source`, and the keyword chip printing its source
  (`cat · AI`) instead of hiding it in a tooltip

### Phase 3 — Adobe/ArtCraft UI/UX overhaul and a visual verification harness (M4–M6 surfaces)

- [x] 3A. `egui_kittest` screenshot suite behind `cargo xtask screenshots`: Empty Welcome, Library (Justified and
  Square), Cull, Detail, Compare, Similar, Places, Insights, each in Charcoal and Paper, with committed or
  cleanly emitted baselines (B) — shipped 2026-11-23: `crates/ui/tests/screenshots.rs` paints the real shell
  inside an `eframe::App` at 1440×900 (ppp 1.0) through the same egui fork, and 18 baselines lived under
  `crates/ui/tests/snapshots/` at that time (33 after the later Phase 3 slices), diffed by `egui_kittest` with the tolerance in `kittest.toml`. The harness is
  a `sortlens-ui` feature no app links (it needs a GPU; `cargo xtask ci` stays headless) and `--update` is the
  rewrite path. It found and this commit fixed two defects: the sort dropdown had no accessible name (the
  suite's AccessKit check fails an unnamed control) and the detail footer painted `←`/`→`, which no bundled
  font has, so a zero-config build showed tofu (the footer now says "left/right"). It also recorded three
  open visual defects named in 3B.6 below — all closed since, (a) through (h), the last of them (g) adding a
  narrowed `shortcuts-*` scene of its own
- [x] 3B.1 `Tokens` mapped onto `craft_ui::theme::WidgetPalette`; monochrome SVG icons across top bar, sidebar,
  filter strip, bottom bar and right panel (B) — shipped 2026-11-23: `Tokens::palette` is the one mapping and
  `apply` installs it through `WidgetPalette::apply_to` (the accent ring on the pressed state is the one
documented exception), tested per preset against what `apply` actually put in `Visuals`. The icon set is 34
project-owned SVGs in `assets/icons/`, embedded and tinted through `craft_ui::icons` + `egui_extras`' SVG
loader (a new workspace pin; `deny` and `attribution` all green). Wired into all five surfaces: the
sidebar's source/folder/keyword/collection rows, the filter strip's chips, every bottom-bar view tab, the
right panel's three tabs, and the top bar's Settings, panel toggle and Add Folder buttons. Four tests hold it
(`Icon::ALL` ↔ catalog ↔ folder, `currentColor` and stroke conventions, every glyph rasterised through the
real loader at 14 and 32 px, and an icon-only button's accessible name), plus a shell test that counts the
textured rectangles in the top bar, bottom bar and sidebar. All 18 baselines regenerated and read
- [x] 3B.2 Top bar: app mark, `Search photos or ⌘K`, a dual-mode command palette (commands with shortcuts, and
  photos), text buttons replaced by named, tooltipped icon buttons (Filter + badge, Import, Export, Activity, Help,
  Appearance, Settings); layout and density move to the canvas header; theme and motion behind one Appearance menu
  (B) — shipped 2026-11-23: the mark is `sortlens-icon`'s own geometry, the field switches to command mode on `⌘K`
  with rows grouped by the registry, `↑/↓`/`Enter`/`Esc` wired, and every row a `Ready` registry command (21 of
  them; a test resolves each and refuses a row that is planned, doubled or unknown). The Filter chip toggles the
  strip and paints the active count; Export opens the export dialog for the selection; Appearance cycles the theme
  and lists every preset plus reduced motion on right click; Help opens the shortcut overlay; density and layout
  are two glyphs beside the sort in the canvas header. Two glyphs joined the set for it (`funnel`, `export`, 36
  total), `Ctrl+K` is in the keymap overlay, and the new `palette-charcoal`/`palette-paper` baselines are its
  visual proof. Still not shipped, and now named in `docs/UX.md` §3 rather than implied: the app-mark library
  menu and view history.
- [x] 3B.3 Sidebar: `SOURCES` holds only photo scopes with icons and live counts; Library health, Changes and
  Shortcuts move to a utility strip (B) — shipped 2026-11-23: `SOURCES` is the six scopes a photograph can be
  *in* (`All Photos`, `Recently Added`, `Picks`, `Rated`, `Missing`, `Rejects`), each with a glyph and a live
  count computed by the question its own row would ask (`source_filter`/`source_count`, shared with the tests,
  so a row's number and the grid it opens cannot disagree). `Missing` is new and needed one real filter: the
  catalog gained `Filter::missing: Option<bool>` — a *narrowing* that outranks `include_missing`, because
  "everything, missing included" and "only what is missing" are different questions and the narrower one was
  asked deliberately. The four dialog rows (`Recently Deleted`, `Check Library`, `Recent Changes`, `Keyboard
  Shortcuts`) left the middle of the sidebar for a hairline-separated strip pinned to its foot, and each one
  *is* a `PaletteCommand`: the words and the shortcut come from the command registry and the click goes through
  the same `run` the `⌘K` palette uses, so a fifth row can only be added by naming a command that exists. The
  strip is given an exact height (`sidebar_foot_height`) rather than measured from its content, because a panel
  learns its content's height only after laying it out — the auto-sized version hung below its own clip rect on
  the first frame. The filter bar's `Missing` chip now reads active when either the chip or the scope is on, and
  clearing it leaves both. Tests: the scope's count over a library with a removed file, the catalog's
  narrowing-vs-widening precedence, and a geometric check that the foot's four glyphs are the last four in the
  column, nothing else is down there, and every sidebar glyph is inside the clip rect it was painted into. Two
  new baselines (`missing-charcoal`/`missing-paper`) show the scope on; all 20 existing ones were regenerated
  and read. Not shipped here, and named in `docs/UX.md` §4.1/§4.4: the per-row "filtered" marker, and the
  footer's storage summary and branding.
- [x] 3B.4 Bottom bar: icons on every view tab; `Detail`, `Compare` and `Soft Focus` as peer views with narrow-window
  collapse; centre annotation pill (5-star rating, Pick/Unflag/Reject, selection count); right-side density/size,
  zoom and `?` controls (B) — shipped 2026-11-23, see the thirty-fourth entry: `View::ALL` is nine destinations
  (Library, For You, Cull, Detail, Compare, Places, Similar, Soft Focus, Insights) and `View::is_detail` is the
  one answer to "is a photograph on screen rather than the grid" (`detail_open` is gone). `state.compare` is
  `Some` exactly while `View::Compare` is open, maintained by `activate_view`/`toggle_compare`, so a second pane
  cannot outlive the view drawing it. The row is measured from its labels and collapses to the current
  destination plus a menu on the icon set's own `chevron` (the bundled faces have no `▾`); the centre pill is
  five stars (the lit one clears), Pick/Unflag/Reject, the primary's keyword count and "N selected", calling the
  same `rate_selection`/`flag_selection` the number and letter keys call; the right side carries the three-step
  thumbnail slider in the grid, Fit/1:1 plus click-zoom in Detail/Compare, `?` and the reduced-motion indicator.
  `view.compare`, `view.softFocus` and `view.places` are `Ready` registry commands with `ShowView` effects (the
  Places row had still said `Planned("M5")`; `M` now opens the map), and the palette grew to 24 rows. Tests: the
  pill driven by real clicks on its painted glyphs, the collapse threshold plus the menu holding every
  destination, the peer-view invariant and Compare's refusal, click-zoom, a registry/effect audit, and the size
  control's snapping. Three new baselines (`soft-focus-charcoal`/`-paper`, and `narrow-charcoal` at 900 px for
  the collapsed row) and all 20 existing ones regenerated and read; the screenshot accessibility pass found the
  sharpness slider unnamed, which is fixed
- [x] 3B.5 Right panel: a `Suggestions` tab beside Details, Tags and Activity, and the far-edge iconic tool strip (B)
  — shipped 2026-11-23, see the thirty-fifth entry: `RightTab::ALL` is four (Details, Tags, Suggestions, Activity)
  and `right_tools` is a 30-point rail on the far right, painted whether the body is open or not, so switching
  tools never hides the panel and clicking the active tool is the one way to close it. The panel's own tab row is
  gone; a header names the active tool in words. The Suggestions tab groups proposals into three families
  (Objects / Scene / EXIF), prints each one's confidence, source and — when it does not cover the whole selection
  — how many photographs it applies to, and offers `+ keyword` per row plus `Accept all` per family. Accepting and
  dismissing are one undo for the whole selection: `Engine::accept_suggestions` writes every keyword and its `Ai`
  provenance in one `Op::Group`, and dismissal is a new `Op::SuggestionDismissed` on `Photo::dismissed_suggestions`,
  a real record with `suggest.dismiss` in the registry, an MCP example and a `dismiss` CLI verb, because a
  dismissal kept only on screen would last until the panel was reopened. The specimen that proves a dismissal
  outlives the panel: the catalog test reopens the library, undoes, redoes, and asserts a library that never
  dismissed anything writes no such field. Two new baselines (`suggestions-charcoal`/`-paper`) and all 25 existing
  ones regenerated and read; the fixture gained a keyword pack and two frames with real camera EXIF, because a
  fixture JPEG is silent and one family would not test a grouping
- [x] 3B.6 (a) Welcome bullets as proper icons — **shipped 2026-11-23 (part 1)**: the three promises start with
  `folder`, `history` and `reject` in one 16-point column (the typed `·` was a per-face speck), a shell test asserts
  exactly three 14-point textured glyphs in that column with each title beside its own glyph, and
  `welcome-charcoal`/`welcome-paper` were regenerated and read
- [x] 3B.6 (b) The filter strip's own controls — **shipped 2026-11-23 (part 2)**: the strip is not painted on an
  empty library (the catalog's total, not the filtered count, so a filter that hides every photo keeps the strip
  that clears it); the rating chip states its minimum in lit star glyphs instead of `3★ and up`, the colour chip
  paints the label's own swatch instead of a droplet, the flag chip wears the state's own glyph, and the keyword
  menu carries a chevron; the row wraps instead of pushing `Clear` past the canvas edge. Two `run_ui` tests and a
  `filters-charcoal`/`-paper` baseline pair at 1200 px, regenerated and read
- [x] 3B.6 (b') The canvas under a filter that hides every photograph — **shipped 2026-11-23 (part 3)**: the
  empty canvas is decided by a four-way `canvas_empty` whose gate splits the catalog's total from the filtered
  count, so a narrowed-to-nothing library paints "Nothing matches these filters", the library's own count and one
  `Clear filters` button (which clears the search text and forgets the folder scope, exactly what the header's
  `N active` badge counts) instead of "Welcome to Sortlens / Add Folder…". The same call fixed `Not now`: the old
  `unwrap_or(Welcome::Add)` painted the wizard again the frame after it was dismissed. A `run_ui` test drives both
  halves with a real press (a zero-result filter shows the card and its button restores the grid; an actually-empty
  catalog still paints the Welcome card and never claims a filter), a search-only query is covered too, and a
  `no-matches-charcoal`/`-paper` baseline pair at 1200 px was regenerated and read
- [x] 3B.6 (c) Places has geography out of the box — **shipped 2026-11-23 (part 4)**: `sortlens_geo::coastline`
  embeds `assets/coastline.txt`, the Natural Earth 1:110m coastline (public domain): 134 shorelines and 5,128
  points, rounded to 0.001° and split at the antimeridian, so a first-run library is a map rather than dots on a
  blank rectangle, with the credit line printed every frame the outline is drawn. `Coastline::parse` refuses and
  counts what it cannot use, and clamps only the six Antarctic points below Mercator's limit — a photograph beyond
  it is still refused, because a coastline is a drawing of land while a coordinate is a claim about a photograph.
  `screen_lines` projects once at zoom 0 and returns only the runs of consecutive segments that touch the viewport. A
  `tiles/` folder still wins, as does a `tiles.mbtiles` archive when no folder exists, and either replaces the outline
  rather than being drawn over it. `coastline::bounds_centre` and
  `coastline::fit_zoom` replace the fixed opening zoom: Places centres on the **middle of the photographs' spread**
  (the mean of Brighton and Lima is 1,400 km north of it, which pushed the northern dot towards the edge of its own
  fit) and opens at the deepest zoom that still shows every located photograph, which is the reported "opens at zoom
  5 with both clusters off screen". Seven geo tests and two shell tests (one new, one extended): the shipped file is a whole world with zero
  refusals and every projected point inside the world square; a walk across the antimeridian is two shores rather
  than a world-wide stripe; refusals are counted; only on-screen segments are returned; the fit shows Brighton and
  Sydney where zoom 5 showed neither; a chosen zoom is never overwritten; and the view paints two dots inside the
  map with the outline beneath them, and paints neither for the person's own tile folder or archive nor for a library
  with no location. `places-charcoal`/`-paper` regenerated and read
- [x] 3B.6 (d) The sidebar's sections scroll above the pinned foot — **shipped 2026-11-23 (part 5)**: at 620 px of
  height six scopes, two subfolders and twelve keyword rows are taller than the panel, and the rows used to keep
  laying themselves out past the hairline and paint over the four utility rows. Everything below the `Library`
  header now lives in one vertical `ScrollArea` sized to the room the foot left (`auto_shrink([false, false])`, so
  the bar appears only when the sections really need it), which makes the boundary a clip rather than luck and
  leaves the rows under the fold reachable by scrolling instead of lost. A headless test paints 1280×620 with
  twelve keywords: no keyword row is *visible* inside the foot's band (visibility is a rect intersected with its
  clip rect, so a clipped row does not count), the foot's own four labels are all readable there, the test refuses
  to pass if the content happens to fit, and a wheel event brings a below-the-fold keyword up while
  `Recently Deleted` does not move a point. `short-sidebar-charcoal`/`-paper` at 1280×620 added; `filters-*` and
  `no-matches-*` regenerated and read — their only change is that the `+ New collection` row, which used to sit on
  the hairline, is now clipped above it. `docs/UX.md` §4.4 updated: the sections scroll, they are still not
  collapsible or reorderable
- [x] 3B.6 (e) The canvas header keeps its name whole on a narrow canvas — **shipped 2026-11-23 (part 7)**: the
  header was a plain `horizontal`, so the right block was handed the width the left block left over and, when that
  number ran small, was laid out from a cursor already past the right edge — at 900 px with both docks open the
  title was painted underneath the sort combo (287..335 under 283.5..348.5) and the count underneath the scope
  chip. It is now one `egui::containers::Sides` row with `shrink_left()`: the controls are measured first and the
  words take the room that is actually left over, so an overlap is not a width-specific slip waiting to happen.
  The row degrades by measurement rather than by luck — the count is measured against the room beside the name and
  steps aside when it will not fit (it is repeated in the strip and spoken in the live line), and the two chips
  are offered only above `HEADER_CHIP_ROOM` points, because the scope is also cleared in the sidebar and the
  active-filter count is also on the top bar's Filter chip, while the sort combo and the two grid glyphs exist
  nowhere else on that line. A headless test sweeps 900→1300 px a step at a time over a scoped library: no header
  text ever intersects the combo or either chip (it fails at 900 on the old layout, on the reported overlap), the
  title is never truncated, is inside its own clip, and the degradation is asserted in both directions — count and
  chips absent at 900 and present at 1300, so a header that dropped them everywhere fails as loudly as one that
  kept them in a row they do not fit. `narrow-canvas-charcoal`/`-paper` at 900×620 with both docks open added and
  read; every other baseline is unchanged, because at 1440 the same header still shows all four
- [x] 3B.6 (f) The cull foot no longer clips under the bottom bar — **shipped 2026-11-23 (part 8)**: the canvas
  was sized from the window with a guessed allowance (`strip_height` + `action_height` + 16) for what followed it,
  so at 1440×900 the keyword field — the control that adds a keyword while the same photograph is being judged —
  was painted under the bottom bar, partially clipped, and the live line below it was not painted at all. The block
  under the photograph is now a `Panel::bottom("cull_foot")` of an **exact** height claimed *before* the canvas, and
  the canvas is allocated out of what is left, so no window height can make the two overlap. `cull_foot_height` is
  arithmetic over fixed-height rows (the up-next strip, the four decisions, the keyword band, the field, the live
  line) with only the band and the staged sentence conditional, and the condition — does the photo on screen carry
  keywords, is anything staged — is known before the block is laid out; the live line's height is a galley measured
  from the face it is painted with. The keyword band is two chip rows and scrolls (`min_scrolled_height(0.0)`,
  because the scroll area's own 64-point floor is taller than the band), so an AI pass that leaves sixty keywords on
  a frame cannot resize the photograph as the queue moves. The two live lines became one: the announcement replaces
  the queue position for the frame it is spoken and is then consumed, rather than sitting under the status line
  forever and costing the photograph another row. Tests: `the_cull_foot_stays_above_the_bottom_bar_at_nine_hundred_points`
  paints the real shell at 1440×900 and asserts every cull text is visible *inside* the column (rect ∩ its own clip)
  and ends above the bottom bar, that the live line and the keyword field are on screen, that the canvas is still over
  300 points tall, and that sixty keywords paint the canvas at exactly the height two did while the last chip is
  reachable only after a wheel over the band. It fails on the old arithmetic (verified by shrinking the panel by 40
  points: the live line falls out of the column). `cull-charcoal`/`cull-paper` regenerated and read — the full strip,
  the decisions, the chip band, the field and `Photo 1 of 12` are all above the bar; the photograph is ~40 points
  smaller, which is the room the field was hiding in
- [x] 3B.6 (g) The keycaps and shortcut strings no longer spell arrows the bundled faces cannot
  paint — **shipped 2026-11-23 (part 9)**: measured, not assumed — in a default build (no craft-fonts)
  the proportional face's chain (Ubuntu-Light → egui-icons → NotoEmoji → emoji-icon) has neither
  `←` (U+2190) nor `→` (U+2192), while the monospace face (Hack) has both and the triangles `◀`/`▶`
  render in *both* chains; the keycaps had been drawing fine in monospace all along, but the
  proportional surfaces the same strings reach — the palette's shortcut column if `cull.back` is ever
  listed there, and the cull start screen's hint line, which painted `← back` today — did not. Every
  painted spelling is now the triangles: the four keymap rows, the registry's `cull.back` shortcut
  (and with it the CLI golden `24 shortcuts.txt`), and the cull hint; the same audit retired the
  proportional `⇧`/`✓`/`✕` the keymap's Shift row, the strip's count hint, the keyword menu and the
  strip's clear chip painted (Shift-click, a strong `•` marker, `×`). `fonts.rs`'s comment claimed the
  keymap draws `⌃`, `✕`, `◀` and `✓` — no code drew `⌃` and the defaults lack `✕`/`✓`; it now names
  what the UI really paints (`◀`/`▶`, `↔`, `⌘`). Two guards: a string-level test asserting every
  keycap renders in the monospace face and every action and registry shortcut in the proportional
  one, and a headless audit painting the shortcuts dialog with an empty query and a `◀` needle; the
  string test fails on the old spelling (`←` has no proportional glyph). New `shortcuts-charcoal`/
  `-paper` baselines regenerated with all others and read — the keycap column, the triangles and the
  `0–5`/`6–9` ranges are all real glyphs in both presets
- [x] 3B.6 (h) The filter strip's `N shown` summary no longer lands on a chip — **shipped 2026-11-23 (part 6)**:
  the count was the wrapped row's last child, and a wrapped layout hands its last child the row's whole width, so a
  right-aligned label there was laid out from the wrap cursor and drawn underneath `Clear` (visible in the
  `short-sidebar-*` baselines). The count is now measured first and given a reserved column on the right; the chips
  wrap in the width that is actually theirs, the card keeps the strip's full width, a chip wider than its column is
  clipped at that boundary rather than painted over the count, and below 180 points of chip room the count steps
  aside instead of taking a control's room. A headless test sweeps the strip from 320 to 760 points a step at a time
  and asserts no painted text ever intersects the count (it fails at 326 points on the old layout, over the `Missing`
  glyph), with the count inside its clip and two widths proving the sweep is not vacuous. All 29 baselines
  regenerated: every 1440×900 scene changes only the count's own rect, `filters-*`/`no-matches-*` only its position in
  the two-row strip, and `short-sidebar-*` now wraps `Clear` onto a second row (one strip row taller), read in both
  themes
- [x] 3B.6 (i) A frame the app draws has the window's three buttons — **shipped 2026-11-24**: under an
  integrated frame the top bar *is* the title bar, so it draws minimize, maximize/restore and close at the
  bar's right edge (right-to-left layout, so nothing the shell adds can push them off); under a system title
  bar it draws none, because two sets of buttons are two answers to one question. Each is a glyph with an
  AccessKit label, hover tints close with the reject colour and takes the background's ink for the glyph,
  and the maximize glyph becomes the two-frame restore glyph while the window is maximized (read from
  `viewport().maximized`). The bar only asks: `Action::MinimizeWindow`/`ToggleZoom`/`CloseWindow` go to the
  host, which sends the viewport commands; close sends the system close request rather than `Quit`, so the
  unsaved-changes guard still runs before the window goes. Held by
  `the_window_buttons_appear_with_the_app_s_own_frame_and_ask_the_host` (no cross where the OS draws them,
  painted close with the app's frame, the three labels in order, and a press on close asking the window —
  and not asking for a quit).

### Phase 4 — Platform, keymap and remote polish (M9–M10)

- [x] 1. `craft-keymap` editor (SL-I09): rebinding, conflict detection, import/export in Settings → Shortcuts
  (B) — **done 2026-11-24**. `crates/ui/src/keymap.rs` now has a runtime half: `catalog()` turns every
  single-chord command row of the table into a `craft_keymap::Bindable` (`photo.flag` as two parameterised
  refs, pick and reject), `fixed_rows()` shows the unrebindable rows (ranges, mouse rows, `Esc`, the arrows)
  read-only, `plain_pressed`/`consume` are what the shell's key handlers read, and `painted()`/`row_keys()`
  are what the palette and the `?` overlay paint — a rebind moves dispatch the same frame it is recorded.
  Settings → Shortcuts opens craft-keymap's dialog with the catalog, rebinding with the conflict bar, named
  sets (a built-in `Default` plus the user's), craft-keymap JSON import and export through the host's file
  dialogs, and double-press recording refused (the dispatch does not read it). Overrides live in
  `Prefs::shortcuts` as craft-keymap `Profiles`, so a deleted set stays deleted after saving. Gated by:
  `the_catalog_is_the_rebindable_rows_of_the_table`,
  `the_fixed_rows_are_the_rest_of_the_table_and_clash_with_no_catalog_default`,
  `a_rebinding_moves_the_bindings`, `two_commands_on_one_chord_are_conflicts`,
  `dispatch_reads_the_keymap_not_the_handwritten_table`, `painted_keys_follow_the_rebinds`, the shell's
  `a_rebound_key_moves_the_action` and `the_shortcut_editor_opens_rebinds_and_saves` (Q flags as picked
  after OK and the host receives the override), the settings round trip
  `shortcut_sets_round_trip_and_an_older_file_reads_the_defaults`, and the `shortcut-editor-charcoal`/
  `-paper` baselines (31 at the time; 33 after item 2b), all read.
- **The platform read of the table shipped 2026-11-24** (a repair: the suite had only run on Windows, and
  on macOS six tests failed). The table's `Ctrl+` is the platform's command key — `keymap::on_platform`
  folds `ctrl` into `cmd` on macOS, once, for the table's own chords, a rebind parsed as text
  (`parse_binding`) and matching (`binding_runs`), so the Mac build finally answers the ⌘ chords the
  help lists; before, every shortcut was the physical Control key. Painting follows the faces:
  `spell` keeps the platform's keycaps (`⌘⇧R`) when a symbols face is embedded and the table's words
  (`⌘Shift+R`, `⌘Enter`) when it is not, because egui's bundled faces carry `⌘` and none of
  `⌃`/`⇧`/`⏎`. Proved by `the_platform_spells_the_primary_modifier_its_own_way` and the six
  formerly-Windows-only tests, 148 in the crate. **Still open on a Mac without craft faces:** the
  screenshot gate — `craft-keymap`'s own editor dialog paints the platform's `⇧`/`⌃` and the harness
  installs only egui's faces, so `shortcut_editor_paints_in_both_themes` panics and two scenes' chord
  wording differs. The second half of that is now a mechanism: `SORTLENS_SNAPSHOT_SET` scopes every
  scene to `tests/snapshots/<set>/` and the gate still compares byte for byte (2026-11-26, seventy-fifth
  entry), so a Mac set is a directory to commit rather than a code change. Making the shots a Mac gate
  still means the harness embedding the symbol faces and that set.
- [x] 2. Packaging and accessibility, in this order — 2a–2d shipped 2026-11-24; the signed and notarised
  bundles that would consume the icons are not this row and stay blocked in "Deliberately absent from M10":
  - [x] 2a. `.icns`/`.ico` generated from `crates/icon` by `cargo xtask icons` (B)
    — shipped 2026-11-24: `sortlens-icon::ico()` packs PNG entries at 16/32/48/64/128/256 px into the
    little-endian ICONDIR layout (0-as-256 honoured), `icns()` packs PNG chunks `ic11`/`ic12`/`ic07`/
    `ic08`/`ic09` (32→512) into the big-endian container, both rendered fresh from the one geometry
    (`render(size)`) rather than resampled, and the committed `assets/icon.ico` and `assets/icon.icns`
    are held to the drawing by tests byte for byte — the same `SORTLENS_UPDATE_ICON=1` rewrite path the
    SVG has, and `cargo xtask icons` (check; `--update` to rewrite) is the front door in `xtask` that
    the ROADMAP row asked for. Proved by `the_ico_carries_one_png_per_desktop_size` (parses the
    directory back, decodes every embedded PNG, checks offsets) and `the_icns_carries_the_mac_slots`
    (walks the chunks, checks the container length) plus `the_committed_packaging_icons_are_the_drawing`.
    Still open, unchanged: signed/notarised bundles that would consume these files (the M10 table).
  - [x] 2b. A High-Contrast preset (SL-L09)
    — shipped 2026-11-24: `ThemeKind::HighContrast` (`theme = "high-contrast"` in `config.toml`, round-tripped
    through the same `id()`/`from_id()` path as the other four, so a preset switch needs no migration) with
    `Tokens::HIGH_CONTRAST` — pure black canvas and thumb wells, pure white text, a yellow accent that holds
    4.5:1 against black with room to spare, muted text at AA, and a new `Tokens::stroke_w` token that doubles
    every chrome stroke (window frames, widget outlines, the selection ring, the press ring) while the four
    colour presets keep their hairlines. Proved by `high_contrast_is_black_white_yellow_and_doubled` (WCAG
    luminance ratios, the accent is yellow, `apply` really installs the 2-point strokes and the token fills),
    the existing preset-wide tests now running over five presets, `the_settings_dialog_offers_high_contrast_and_clicking_it_asks_the_host`
    (the radio clicked through the real widget returns `SetTheme(HighContrast)` and every preset label paints),
    and two new baselines `library-justified-high-contrast` and `welcome-high-contrast` (33 now), read: black
    canvas, white words, yellow marks, borders that read as boundaries. It appears everywhere presets are
    named because every menu iterates `ThemeKind::ALL` — the Appearance chip's right-click, its cycle, and
    Settings — with no new drawing code.
  - [x] 2c. `craft-update` behind an explicit opt-in (SL-I10) — **done 2026-11-24**: the consent-gated
    update check in `sortlens-engine`'s `update` module, desktop-only and shaped like `model.install`:
    `update.status`, `update.check` and `update.consent` in the registry (never in `crates/ai`), the
    consent stored per session in the engine and persisted by the app as `updates.check_consent`,
    `update.check` refusing without it from every face — UI (Settings → Updates, `Check now` only
    behind the switch), CLI (`update`, `update-consent`, `update-check`) and MCP examples. The check
    runs on a worker as an `Update` activity job; the answer (the latest release read through
    `craft-update`'s capped parser, held to `Topzic/sortlens-rust`'s own download links, and compared
    with `craft-update`'s semver rules including the local-build rule) is kept for `update.status` and
    the Settings row, so an answer landing after the dialog closes is not thrown away. Proved by
    `update_check_refuses_without_consent_and_names_the_setting`, `update_consent_flips_through_its_command`,
    `update_status_reports_version_consent_and_last_check`, `update_check_runs_on_a_worker_and_keeps_its_answer`
    (a loopback feed in GitHub's shape, the worker path end-to-end), `a_failing_feed_fails_the_job_and_keeps_no_answer`,
    `a_second_check_refuses_while_one_is_running`, the module's own parse tests (prerelease skipped,
    local build never downgraded, rate limit named, empty answer read as up to date), and the headless
    `the_updates_settings_section_gates_the_check_behind_consent` (the checkbox clicked through the real
    widget, `Check now` appearing and disappearing with the consent). No baseline moved: no settings scene
    exists, and the section is desktop-only.
  - [x] 2d. `craft-i18n` scaffolding (SL-L07) — **done 2026-11-24**: `crates/ui/src/i18n.rs` builds the
    suite's `I18n` once from the embedded Arabic and Japanese catalogues (`crates/ui/src/i18n/ar.tsv`,
    `ja.tsv`) and routes the first batch of strings through it — the sidebar's six sources and five
    sections, the foot's four registry labels, the right-panel tab labels, the delete modes and the
    Settings dialog — a missing row is the English text, so partial coverage shows English rather
    than breaking. Settings → Language offers each language named in itself; the choice persists as
    `Appearance.language` and is restored at launch. Direction: `i18n::apply_direction` resolves it
    from the language (overridable with `CRAFT_UI_DIRECTION`) and applies it every frame; the sidebar
    sits at the reading start, the tool strip and right panel at the reading end, `nav_row` mirrors
    its glyph/label/count, and section titles align to the reading start. Proved by the catalogue
    audit (`catalogs_load_and_routed_keys_have_rows` reads the routed keys out of the sources and
    fails on a key with no row in either language), `the_picker_names_every_language_and_directions_follow_the_script`,
    the settings round-trip `the_language_setting_round_trips_and_an_unknown_one_is_english`, and the
    headless `the_language_switch_moves_the_sidebar_to_the_reading_start` (the radio clicked through
    the real Settings widget, `SetLanguage` asked of the host, the sources header measured in the
    right half in Arabic and the left half again in English). All 19 screenshot baselines regenerated
    byte-identical — the proof a left-to-right interface paints exactly what it painted before. Not
    shipped here, still open under the M9 row: the other four languages the design names (zh-TW, ko,
    th, vi), the remaining unrouted surfaces, and RTL screenshots of the painted interface, which
    need an Arabic-capable font build (CI's has none). The export dialog and its receipt joined the
    catalogues on 2026-11-26 (seventy-eighth entry): `sortlens-export` hands the shell
    `Spec::parts` and `Report::parts`, the pieces its English `summary`/`headline` are joined
    from, so a translated dialog cannot describe numbers the export will not write, and
    `the_export_dialog_and_its_receipt_speak_the_interface_language` paints the dialog in all three
    languages. The delete confirmations joined them the same day (seventy-ninth entry): the
    similar-view delete, the cull batch's Apply and the rejects-bin emptying word their counts,
    modes, their changed-on-disk warning and their toasts through the catalogues, and
    `the_delete_confirmations_speak_the_interface_language` paints all three in English, Japanese
    and Arabic. The catalogue audit there reads `tl!` literals *and* `fmt` templates, so a template
    with no row is a named debt rather than a sentence that silently stays English. The rename and
    About dialogs joined them the same day (eightieth entry): their titles and buttons, the rename
    toast and both About lines are catalogued — the version number and the command counts as `fmt`
    templates, so each language places them — and
    `the_rename_and_about_dialogs_speak_the_interface_language` paints them in all three languages.
    Library health's catalog half joined them the same day (eighty-first entry): `sortlens-catalog`'s
    `Audit` now hands the shell a `Headline` and its `AuditPart`s — the pieces its English
    `headline()`/`detail()` are joined from, pinned by `the_headline_and_detail_are_joined_from_their_pieces`
    — and the shell words them, keeping them in `AuditView` so the section repaints on a language
    switch; `the_library_health_audit_speaks_the_interface_language` paints it in all three languages
    with the audit's numbers in the reader's digits. The rest of the dialog joined them in the next tick
    (eighty-second entry): the health check's own count line and its toast, the changed-on-disk line and
    `Refresh changed`, the root rows (`{n} photo(s) under {root}`, `Point at…`), the metadata retry
    lines and `Remove missing from library` are `tl!`/`fmt` keys with every count a placeholder, so
    Arabic writes its own digits there too; `every_word_in_the_library_health_dialog_is_a_catalogued_one`,
    `the_library_health_dialog_speaks_japanese` and `the_library_health_dialog_speaks_arabic_with_its_own_digits`
    are the wording tests, and the audit test now also sees `Check library` and `Not checked yet.` painted
    in Japanese. The cull summary joined them in the next tick (eighty-third entry): `sortlens-cull`'s
    `Summary::headline_kind` returns a `Headline` (`Nothing`, `AllJudged`, `Partial`) and its own
    `headline()` is joined from that piece, pinned by `the_headline_is_joined_from_its_piece`; the
    sheet's title, four rows, Apply/Keep culling/Jump to next/Discard buttons and their hovers, the
    `{changed} … {deferred} …` foot line and the discarded toast are `tl!`/`fmt` keys with both counts
    as placeholders, and `the_cull_summary_speaks_the_interface_language` paints the sheet in all
    three languages. The quit guard joined them in the next tick (eighty-fourth entry):
    `Engine::unsaved_reasons` returns an `UnsavedReason` (`Scanning`, `StagedCull { staged }`,
    `Hashing`) and `Engine::unsaved` — the sentences the support report prints — is joined from
    exactly those pieces, pinned by `the_quit_guard_sentences_are_joined_from_their_pieces`; the
    title, both buttons, the hover and the three reasons are `tl!`/`fmt` keys with the staged count a
    placeholder, and `the_quit_guard_speaks_the_interface_language` paints the guard in all three
    languages. The change viewer joined them in the next tick (eighty-fifth entry): `Op::description`
    returns a `Description` piece (counts, paths and names travel with it) and `Op::summary` is that
    piece's `english()`, `history::Entry` carries it, every variant is a `fmt` template whose count is
    the language's own digits through `format_count` in the suite's `{n} photo(s)` wording, the title, both
    hints, the export button and its hover and the footer are `tl!` keys, and
    `the_change_viewer_speaks_the_interface_language` paints the rows in all three languages; the op's
    raw name left the row for the report and the wire, where it is a record field rather than a word a
    reader has to parse. The Batch applied receipt joined them in the next tick (eighty-sixth
    entry): `sortlens-cull`'s `Receipt::headline_kind` returns a `ReceiptHeadline` and
    `Receipt::detail_parts` a `Vec<DetailPart>` (`Destination`, `Picks`, `Failed`), with
    `headline()`/`detail()` joined from those pieces, pinned by
    `the_receipt_english_is_joined_from_its_pieces`; the title, the two count lines, the destination
    line, the pick and failure counts, `Put them back`, `Done` and the restored toast are `tl!`/`fmt`
    keys with every count a placeholder, and `the_batch_applied_receipt_speaks_the_interface_language`
    paints it in all three languages. The colour words joined them in the next tick (eighty-seventh
    entry): the filter chip's word and its spoken name, the folder and collection `Colour` menus, the
    inspector's Label row and Tags tab, Library insights' label bars and the recolour/folder-mark status
    lines all read `colour_word`, and the status templates' `{colour}`/`{n}` placeholders keep a count
    in the reader's digits; `every_colour_word_is_the_catalogues` pins the English byte for byte and the
    Japanese and Arabic words, and `the_colour_chip_speaks_the_interface_language` paints the chip in
    all three. The shortcut overlay joined them in the next tick (eighty-eighth entry): the keymap
    table's forty-four action rows, its five context headings and their notes are looked up by
    their English text through `Binding::localized_action` / `Context::localized_label`, so a new
    row cannot ship untranslated — `every_shortcut_row_is_in_both_catalogues` fails if one does —
    and the title, search hint, both count lines and the empty-search line are `tl!`/`fmt` keys
    with the counts as placeholders; `the_shortcuts_dialog_speaks_the_interface_language` paints
    the overlay in all three. The activity row's job lines joined them in the next tick
    (eighty-ninth entry): `Job::detail_kind` returns a `JobDetail` (`Progress`, `Counts`,
    `Finished` with its unreadable count, `Cancelled`, `Failed`) and `Job::detail` is that
    piece's `english()`; the shell's `job_detail` words it through `i18n::fmt` with both counters
    and the unreadable count as `format_count` placeholders, the kind and state are looked up by
    their English text, and the empty hint and the Indexed/On disk/Missing rows are `tl!` keys, so
    `the_activity_row_speaks_the_interface_language` paints the tab in all three. The Suggestions
    panel joined them in the next tick (ninetieth entry): `sortlens-ai`'s `Suggestion::line_kind`
    returns a `SuggestionLine` (keyword, `Source`, percentage) and `line()` is that piece's
    `english()`, pinned by `the_line_is_joined_from_its_piece`; the four source words and the three
    family headings are looked up by their English text, the confidence is written through
    `format_count`, and the right-panel tab, the inspector's Suggested row and the whole-library
    dialog word their counts as `fmt` templates —
    `every_source_word_and_family_heading_is_the_catalogues` and
    `the_suggestions_tab_speaks_the_interface_language` are the guards. Still open on this row: the
    craft-keymap editor's own English chrome.
  (B,E)
- [x] 3. `sortlens-server` (loopback/LAN bind, pairing code → scoped device token, command-registry allow-list,
  resumable uploads) and the device protocol (SL-J01..J09, SL-J05), proved by a two-process loopback
  integration harness in CI (E). The client that consumes it is the native phone app, row 3b, still open.
  - **The server half shipped 2026-11-24.** `crates/remote` holds the three shared pieces: the scoped
    device tokens (`Permission::Read/Upload/Annotate/Cull`, persisted as
    `<library>/remote-tokens.json`, re-issue replaces the row, revoke is real), the pairing gate (a
    six-digit code, ten minutes, five wrong attempts and it locks out; `--code` pins one), and the
    allow-list — every registry id the wire accepts with the scope it needs, checked at startup against
    `Registry::v0()` by `allowlist::audit`, with a test that also refuses to let a ready command vanish
    silently between the table and an exclusion list naming why. `apps/sortlens-server` is the process:
    a hand-written minimal HTTP/1.1 on `std::net` (Content-Length bodies only, `Connection: close`,
    no new dependencies), binding loopback by default and `0.0.0.0` only under `--bind lan` (SL-J09);
    routes `GET /health` (the connection test, no token), `POST /pair`, `POST /api` (`{method, params}`
    through `Engine::run` behind the allow-list and the token's scopes — a method the wire does not
    accept answers 403 without leaking whether it exists), the resumable upload (SL-J04):
    `Content-Range` chunks append to `<album>/.uploads/<id>.part` so the file length *is* the received
    count and a restart loses nothing, a chunk that does not start where the file ends answers 409 with
    `received`, the completing chunk is hashed against `X-Content-Sha256` *before* it is appended so a
    digest mismatch leaves the partial exactly where it was, and the published name never overwrites —
    it takes ` (n)` beside the stem — and `GET /thumb/<photo-id>/<edge>` (shipped 2026-11-24): one
    photograph's thumbnail as the export path's own lossless PNG, decoded through the same
    `sortlens-thumbs` path the desktop grid uses, bounded to 16–1024 px (a smaller edge would be
    upscaled nonsense; a larger one means decoding a primary image to serve a grid cell) under the
    token's `read` scope, with a photo whose file cannot be read answering 404 — a page shows a hole,
    not a lie. Proved by the loopback harness `apps/sortlens-server/tests/loopback.rs`:
    five tests, each a real server process (`CARGO_BIN_EXE_sortlens-server`) against a real scanned
    library — pair/read/annotate plus the scope refusals and the write read back from disk after the
    server exits, the five-wrong-codes lockout, `--allow read,annotate` being refused `cull.start`, an
    upload that dies mid-transfer, resumes from `GET /upload/<id>`, refuses a wrong digest and still
    completes, then collides into `IMG_0001 (1).jpg`, and a restart keeping the token valid.
  - **The desktop's start/stop surface shipped 2026-11-24.** Settings → Sync & devices is the
    owner's window on the process: Start spawns the `sortlens-server` binary that sits beside the
    app against the open library (loopback, ephemeral port), reads the startup lines through
    `sortlens-remote::supervise` — the same line contract the loopback harness reads, so a
    reprinted pairing code replaces the expired one instead of growing a history — and Stop kills
    the process and waits, so the library's write lock is free when the notice lands. The paired
    devices are read from the token file while the dialog is open, each row naming its scopes and
    issue date, and Revoke is disabled while the server runs (a running server holds its own token
    store and would resurrect the row on its next write-through); the refusal is also enforced in
    the host, not just greyed out. Revoking by exact token lands only after a stop. Proved by the
    headless test (Start emits `Action::ServerStart`; a running surface paints port and code and a
    Revoke that answers nothing; stopped, the same button hands over the exact token), the
    supervisor's line tests, and a new harness test that drives the real server through the
    supervisor and reads the paired row back from the token file.
  - **The in-memory catalog path shipped 2026-11-24.** `Catalog::open_memory` in `sortlens-catalog`
    opens the state from snapshot JSON alone — no directory, no log, no lock — with the same undo,
    redo and stale-revision rules as the disk catalog, and `Catalog::snapshot_json` hands the state
    back for the client to persist. A newer snapshot schema is refused rather than opened
    read-only: a client that cannot read the format must not pretend to hold the library. Proved
    by four tests in `store.rs`: a disk snapshot opening in memory and answering the same
    questions while operating on it touches no file, a fresh client round-tripping through its own
    snapshot JSON with the undo history intact and the snapshot cadence costing nothing, broken or
    newer-schema JSON refused, and `commit_expect` refusing a stale revision in memory.
  - **The client protocol and in-memory engine shipped 2026-11-24.** The device side of the same
    protocol now lives beside the server's: `crates/remote/src/client.rs` is what a paired phone
    speaks — `Client` over a `Transport` (native `TcpTransport`), `health` (SL-J07), `pair` storing the token, `call`
    for every allow-listed command, `snapshot` pulling `library.snapshot`, and `upload`
    (SL-J04's client half): one request under the chunk size, `Content-Range` chunks above it,
    a `409`'s `received` count moving the cursor instead of failing, the completing chunk
    carrying `X-Content-Sha256`, and the keep-or-remove toggle (SL-J05) riding on the queued
    item — the ack answers with `LocalCopy::Keep/Remove`, so the decision survives the resume
    because it lives on the item, not the connection. `library.snapshot` joined the registry
    and the allow-list under `read`, and `Engine::open_memory` mirrors `Catalog::open_memory`
    one layer up (no directory, the disk-side sessions simply absent).     Proved by `sortlens-remote`'s unit tests (chunk plan covers the file once contiguously, a
    409 is an instruction not a failure, a 409 naming no progress fails instead of looping,
    nothing is sent unpaired, pairing stores token and device), three new loopback tests in
    `apps/sortlens-server/tests/client.rs` against the real server process (pair → hold the
    library in memory from the snapshot → annotate through the wire with the write visible
    from a re-pulled snapshot; a token without `read` refused the snapshot; an upload that
    dies after the first chunk and resumes to a byte-verified file in the album, the partial
    gone, with the keep-or-remove decision in the ack both ways), the engine's own memory tests
    (a memory engine operating while touching no file, the snapshot command serving exactly
    what the memory engine opens, the stale-revision guard holding in memory, a fresh client
    round-tripping).
  - **The thumbnails over the wire shipped 2026-11-24.** `GET /thumb/<photo-id>/<edge>` on the
    server decodes through the desktop grid's own `sortlens-thumbs` path and answers the export
    path's lossless PNG, bounded to 16–1024 px under the token's `read` scope, a photo whose file
    cannot be read answering 404; `sortlens_remote::client::Client::thumbnail` fetches it, so a
    phone does not invent a second format. Proved by two client-harness
    tests against the real server process (a paired device reads a PNG that decodes to the
    fixture's own pixels without upscaling, edge bounds and unknown photo and missing file each
    named in the refusal; a token without `read` refused the same as the snapshot) and unit tests
    on the client (the request is `GET /thumb/<id>/<edge>` with the bearer token, PNG bytes return
    untouched, unpaired refuses without talking).
  - **Phones get a native app, not a web page.** The vendor's own product ships iOS and Android apps, so the
    phone client is `apps/sortlens-mobile` (3b below). No web or browser application is built, and none is planned.
- [ ] 3b. **`apps/sortlens-mobile` (SL-J10)** — the native Android and iOS app over the desktop's engine and shell.
  - **Shipped 2026-11-24:** the crate (`cdylib` + `staticlib` + `rlib`), the shared `App`, the Android
    (`android_main`) and iOS (`sortlens_ios_main`) entry points, where the library and the camera roll live,
    `cargo xtask mobile`, and a CI job per platform. Verified: the crate builds and its tests pass on the host.
  - **Shipped 2026-11-24 (settings):** the choices the phone's shell offers are kept in the suite's own
    `craft-settings` document — `config.toml` in the app's private directory, beside the library and never
    inside it — so the theme, density, layout, motion and order chosen on the phone reopen with it. The file
    goes through the same store the desktop uses (`apps/sortlens-mobile/src/prefs.rs`): it saves on a
    debounce, because an OS that ends the process mid-session must not lose a choice already tapped; a damaged
    file is moved aside with the "Settings could not be read in full" notice; an id from a later build reads
    as its default; and keys the phone does not understand survive its saves, so a file shared with the
    desktop keeps the desktop's settings. Tests: every id round-trips one variant at a time through the real
    file, the action→id mapping is driven through `Action` itself (a missing arm fails), and the settings sit
    outside the catalog.
  - **Shipped 2026-11-24 (phone width):** a window narrower than 600 points is a phone, decided once per frame
    before anything paints (`NARROW_BELOW` in `crates/ui/src/shell.rs`). The two docks are put away so the grid
    gets the screen instead of being squeezed to nothing; the bar drops the wordmark and "Local library" and
    keeps the mark, the search field sized to the room its glyphs leave (with the palette's chord dropped from
    the hint, and the field itself dropped below 140 points) and five glyphs — library sheet, inspector,
    export, notices, settings — because the keyboard overlay and the appearance popover are about keys a phone
    does not have; the read-only warning becomes the same sentence in a glyph's hover. Opening either dock makes
    it a sheet the width of the screen, draws no canvas behind it, and closes the other dock by construction
    (`toggle_left`/`toggle_right`/`open_right`). Visual proof: two new baselines, `phone-charcoal` and
    `phone-sidebar-charcoal`, at 390×780, painted by the same harness as every other scene, and no earlier
    baseline moved — the check that 900 points and up are unchanged. Tests: a phone frame opens on the grid with
    every bar control on screen, each dock opens and closes from its own glyph one at a time, and a 900-point
    window keeps both docks and its wordmark.
  - **Shipped 2026-11-24 (phone canvas):** the grid is now the point of the screen. The far-edge rail stands
    down while the inspector is put away — a tenth of a 390-point screen spent on a tool the bar's own inspector
    glyph offers — and keeps its post beside the open sheet, where it is the tab switcher: a tap changes the
    tool without closing and reopening the sheet (the earlier "close-then-reopen" note was wrong; the test
    drives the tap). The canvas's gutter narrows to 10 points. The filter strip keeps no count on a phone — the
    canvas header's count is the same number on the same screen — and its chips sit closer, so eight chips plus
    `Clear` wrap into two rows instead of three; the strip's bottom edge is at ~150 points, where the desktop
    layout spent ~190 before the first photograph. The bottom bar's tray is the phone's: the pill's buttons sit
    tighter, the keyword count steps aside (the inspector's Tags tab is one tap away), the density slider
    yields to the header's own glyph, and every control it paints is on the screen — the desktop pill's last
    flag button ran thirty points off the edge. Tests: a populated 390-point canvas proves the rail's column
    empty, the strip's card at its full width and two chip rows, the tray unclipped and the header intact
    (name, count, sort, both grid glyphs); a 900-point window keeps its rail, its strip summary and its pill;
    the sweep that checks the count never lands on a chip now mirrors the shell (absent under 600, measured
    above it). Both phone baselines regenerated and read: three photo columns, a two-row strip, the tray inside
    the screen, the sidebar sheet the width of the screen.
  - **Shipped 2026-11-24 (send from the app):** Settings → Send to desktop is the phone's half of the
    protocol. Unpaired, it is the address the desktop's Sync screen shows, the port, the connection test —
    `GET /health`, no token (SL-J07), run on the typed address and port alone before any code is spent, on its
    own thread, saying which address answered or why none did and storing nothing either way — and the
    six-digit code; paired, it names the desktop and offers Forget, a progress line naming
    the photograph it is on, Stop, and Send the selection with a keep-or-remove switch. The queue
    (`sortlens_remote::send`) owns one worker thread, so no frame blocks; the shell polls its report and takes
    the finished run's outcome once. The switch rides on each item: after the ack, the app hands the cleared
    ids to `Engine::delete_photos(..., Trash)` — the same audited path the desktop applies a cull with — so an
    unreachable desktop removes nothing, Stop removes only what already landed, and a file whose bytes moved
    since the scan is refused rather than deleted. The desktop's Sync & devices section is not painted on the
    phone (its host cannot serve) and this one is not painted on the desktop. Tests: a real-TCP fake upload
    server proves both keep and remove, a missing file fails alone, cancel stops between items and an
    unreachable desktop ends the run with one reason; the link pairs through a fake `/pair` server and keeps
    the address with the token, and its connection test answers through a fake `/health` server before any
    code is typed, storing nothing either way and standing down while a pairing runs; the shell drives the
    connection test, Pair, Send, Stop and Forget at phone width, and proves each
    width paints only its own section.
  - **Shipped 2026-11-24 (the phone's trimmed font set):** a phone build that embeds craft-fonts
    embeds only the faces the shipped interface languages need. `CRAFT_FONTS_SCRIPTS` — a comma- or
    space-separated list of ISO 15924 codes — is read by `crates/ui/build.rs`, whose parser and
    selection rules live in `crates/ui/src/font_manifest.rs`, a module `cargo xtask fonts` includes
    too, so a build and its report cannot disagree about what a manifest line means or which faces
    survive. `cargo xtask mobile` passes exactly `PHONE_SCRIPTS` (`Latn`, `Zsym`, `Arab`, `Jpan` —
    the interface's own two plus every shipped language's, a table a test holds to the picker) so a
    phone binary leaves out whole families it could never paint (the manifest's Hangul face, for a
    build that ships English, Arabic and Japanese) while a desktop build leaves the variable unset
    and embeds the whole manifest. A requested script no face covers is refused by name (a typo is
    refused before anything is embedded), and both the build report and `cargo xtask fonts` refuse a
    set that misses a script the interface ships. Verified end to end against a fixture checkout:
    the filtered build's generated table carries the five faces the filter keeps and not the
    Hangul one. Tests: manifest parsing, selection, the typo refusal, the coverage validation, the
    phone script set's definition, and a filtered build naming every shipped script.
  - **The APK in the release workflow shipped and is proven by a real run 2026-11-25.** A release run (a
    `v*` tag, or a manual dispatch that stops before `publish`) builds `sortlens-mobile` into an APK on
    the `aarch64-linux-android` target with `cargo apk build --release` — the crate's own
    `package.metadata.android` is the specification cargo-apk reads, so a manifest change and the package
    cannot disagree. `cargo apk` aligns and signs it with a per-run keystore, named to it through
    `CARGO_APK_RELEASE_KEYSTORE`/`_PASSWORD` (a release build refuses to sign without them), and the
    workflow then proves the signature with `apksigner verify` before attaching the file beside the
    desktop archives. A sideloaded APK needs no signing identity; a key that lets one release update
    another in place is a release secret the owner can add. Two failures the first dispatches found are
    now rules `cargo xtask workflows` enforces with tests: a build step with no keystore variable (the
    APK job stopped at `cargo apk`'s last step), and an `upload-artifact` `path` holding two globs on one
    line (the action splits `path` on newlines, so all three desktop jobs uploaded nothing, warned, and
    passed — a tag run would have attached nothing). Run 45527734155 (commit `09cd112`) is green through
    every job, `publish` skipped as designed, the log shows `Signing ... with keystore` and
    `Signer #1 certificate DN: CN=Sortlens` (SHA-256 `61be0349…`), and four artifacts exist where there
    were none: `android-apk` 71.3 MiB and one archive per desktop target. The checker also refuses a
    `publish` that never downloads what it attaches (the same `publish` step order the bug crossed), a
    release with no `apksigner verify`, and a flow list — which caught `needs: [build, android]`, and
    fixed a `verify` job that had never run and was missing the Linux windowing headers `cargo xtask ci`
    needs on Ubuntu. The `publish` job itself still waits for a tag, which is a person's decision.
  - **The APK leg compiles: the phone's trash is a folder move (2026-11-24, the same day).** The first
    release-workflow run caught what a `cargo check` never links into view: the `trash` crate does not
    compile for `aarch64-linux-android` (no freedesktop trash there). `Mode::Trash` on Android and iOS
    now moves the file into the library's own `.sortlens/trash` — the name `crates/scan` skips — with
    the same re-check (`audit_one`), the same size verification and the same receipt; `undo` refuses a
    trash receipt on every platform, so nothing a mistake could recover is destroyed, and the file sits
    in the folder for a person to bring back. The dependency is target-gated to the platforms the crate
    speaks (`trash` 5.2 out of the workspace table), `move_to_phone_trash` is compiled and tested on the
    host everywhere (move, relative path, size-refusal, receipt), and `crates/engine`'s scan test holds
    the cross-crate contract: a library containing `.sortlens/trash` scans clean with the folder
    refused, not descended into. `docs/BEHAVIOUR.md` §3 says what the trash mode does on a phone.
  - **Still open:** running the targets on a device (a machine with the Android NDK and Xcode; CI's
    runners check both targets compile); the iOS Photos library; an `.ipa` in the release workflow (the
    Apple identity the M10 table names); an AAB (Play-distribution tooling, for a Play listing); and a
    release key that lets one APK update another in place (a release secret the owner can add).

### Phase 5 — Autonomous GitHub maintenance setup

- [x] 1. `.github/ISSUE_TEMPLATE/bug_report.yml` (OS, version, repro, expected vs actual, `sortlens-cli diagnostics`),
  `feature_request.yml`, and a PR template requiring the linked issue, the failing-then-passing test, the gate
  confirmations, a `perf` delta for scan/catalog/thumbs and before/after screenshots for `crates/ui` (E) —
  **done 2026-11-24**, see the fifty-eighth CHECKPOINT entry: seven bug fields (version, OS, OS version and
  display, repro, expected, actual, diagnostics) and six feature fields, both with a markdown intro, the
  `bug`/`enhancement` and `triaged` labels, and every reporter field labelled and required where it must be;
  the PR template carries the five gates as checkboxes, the failing-then-passing test prompt, the
  scan/catalog/thumbs `cargo xtask perf` table and the `crates/ui` before-and-after. `cargo xtask templates`,
  in `ci`, reads both forms through a small YAML-subset reader and refuses the shape GitHub would reject,
  naming the file and line; it also checks the PR template still names every gate.
- [x] 2. `docs/AUTONOMOUS_AGENT.md`: issue text is untrusted data; only `agent-ready`/`triaged` issues; never push to
  `main`, auto-merge, bypass `deny` or touch signing secrets; the repro-first loop ending in a Draft PR; the bounded
  improvement queue (perf regressions, `enhancement-approved`, unchecked roadmap rows) (E) — **done 2026-11-24**, see
  the fifty-ninth CHECKPOINT entry: the playbook draws the boundary (a report is data, never instructions, and the
  intake hands the body to the checker through the environment for that reason), makes `agent-ready` the only pickup
  signal with `triaged` a statement about attention rather than permission, and names the never-list — never push to
  `main`, auto-merge, bypass `deny`, or read a signing or release secret. The repro-first loop runs read the evidence
  → failing test → minimal fix → documents reconciled in the same commit → the gates → **Draft pull request** and
  stop; the improvement queue is `sortlens-perf`, then `enhancement-approved`, then unchecked roadmap rows, and an
  empty queue means stop. The document is not prose nobody checks: `cargo xtask workflows` (in `ci`) fails when it
  loses one of ten load-bearing rules, matched by phrase rather than wording.
- [x] 3. `.github/workflows/agent-intake.yml` and a `ci.yml` that runs `ci`, `snapshot`, `deny`, `mobile` and
  `screenshots` on every PR (E) — **done 2026-11-24**, see the fifty-ninth entry: `agent-intake.yml` triggers on
  opened/edited/labeled issues, holds `contents: read` and `issues: write` and nothing else, cannot run on a pull
  request, never reaches a repository secret, and passes the body and labels to `cargo xtask intake` through the
  environment (never interpolated into the script). A report missing a required heading gets a comment naming each
  one, loses `triaged` and gains `needs-info`; a later complete edit swaps them back. `ci.yml` now runs the fifth
  gate, `screenshots`, on Mesa's software Vulkan so the baselines are checked headlessly — through
  `SORTLENS_SNAPSHOT_SET=lavapipe`, because that software adapter's paint is not the reference machine's, and
  a failed run uploads every frame it painted as the `screenshot-set` artifact — every scene of a test that
  differed, not just the first, because a mismatch is recorded and checked once the test has painted them all —
  so the set it is missing can be downloaded and committed — and `cargo xtask workflows` refuses a missing gate, a `continue-on-error` step, or
  an `if` that would leave pull requests out.

---

## Backlog (post-1.0)

| Item | Why deferred |
| --- | --- |
| Native video playback | Poster frames cover culling; decoding + audio is a large surface |
| Face detection/grouping (SL-F11) | Second model, privacy-sensitive, not core to culling |
| Survey (SL-C14) | Compare is now a peer view, so what is deferred is the 2–9 tile survey specifically: culling is the fast path, compare the precision one, and a survey would be a third way of looking at the same decision |
| Smart collections (SL-G05) | Filter presets + saved views cover most of it |
| Keyword hierarchy (SL-D13) | Flat tags match the vendor and XMP interop |
| Portable mode (SL-I15) | `--data-dir` exists; UI packaging later |
| Network-share libraries | Correctness/perf risk; read-only at best |
| Windows/macOS shell extensions | Out of scope for Rust core |

## Status log

| Date | Change |
| --- | --- |
| 2026-11-24 | **Phase 5 complete — the owner's completion plan is closed.** `docs/AUTONOMOUS_AGENT.md` and the workflows that enforce it shipped: `agent-intake.yml` validates a filed report's structure and corrects its labels, `ci.yml` gained the `screenshots` gate, and `cargo xtask workflows` holds both files and the playbook's ten rules. Phases 0–5 are all ticked; what remains is the M10 "Deliberately absent" table (Apple identity, installer toolchains, a screen reader, a chosen palette, the owner's tag) and the visible post-1.0 backlog and M9 rows — named, not hidden. |
| 2026-11-22 | **Phase 0 of the completion directive.** "M0–M10 shipped" withdrawn: the buildable features that had been moved into "Deliberately absent" tables are re-opened as Phases 1–5 above. `docs/RELEASE.md` and `docs/ATTRIBUTION.md` corrected, 13 unused dependency pins pruned. Owner sign-offs recorded: opt-in network for model download and update checks, real permissively licensed weights and map data, `MIT OR Apache-2.0`. |
| 2026-11-21 | **M0–M10 built, and the documents reconciled with the tracker** (the word "shipped" was withdrawn in Phase 0). Everything that needs no weights, no signing identity, no second device and no translation pass is built and tested: 432 tests, one command registry behind the window, the CLI, the MCP server and `--json`; measured numbers in `docs/PERF.md` (which found and fixed a quadratic import); an icon the window actually loads; gates for layer rules, lint headers, asset provenance, fonts, one version, attribution, accessibility names and the documents themselves (`cargo xtask docs`). What remains is itemised in each milestone's "Deliberately absent" table with its unblocker. |
| 2026-11-21 | Plan written: feature inventory (176), architecture, tooling audit, UX spec, roadmap. Implementation not started; Rust toolchain not yet installed on this machine. |
| 2026-11-21 | M0 skeleton built on macOS. Toolchain 1.95.0 installed project-locally (`scripts/bootstrap.sh`, `.toolchain/` gitignored). 23 crates scaffolded; `sortlens-core` (ids, labels, units, errors), `sortlens-engine` (registry + `run`), `sortlens-ui` (4 presets, five surfaces, settings/about dialogs), the `sortlens` app (title-bar policy, `craft-settings` store, `fn ui` shell) all compile. `cargo xtask ci` green: fmt, clippy `-D warnings`, 20 tests, layer rules, lint headers, asset provenance. `docs/BEHAVIOUR.md` written. Not done: fonts, `cargo deny`, snapshot CI, shortcuts dialog. |
