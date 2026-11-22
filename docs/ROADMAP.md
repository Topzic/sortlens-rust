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
- [ ] `ContentKey` (size + first/last 64 KiB) is in; a volume probe and `PathKey` are not (A) — SL-A11
- [x] Walker with skip rules, bounded queue, delta detection (A) — SL-A01/A17
- [x] `sortlens-catalog`: op log, snapshots, replay, undo, paging, counters (A) — SL-A12/A13
- [x] Engine scan worker, `library.addFolder`/`rescan`/`page`/`stats` (A)
- [x] Virtualised grid with selection, counts and keyboard rating/flags (B)
- [x] CLI: add, list, rate, flag, undo (E)
- [x] `craft-metadata` → `PhotoMeta` through `sortlens-meta` (C) — SL-B21 data
- [ ] Format routing by extension is in (`sortlens-core::media`); magic-byte sniffing is not (C) — SL-B11
- [x] `library.addFolder`, `library.rescan`, `library.removeRoot` (A) — SL-A01/A19
- [ ] Square grid is virtualised (a window of cells over 200k rows); justified rows are not built (B) — SL-B01/B02
- [ ] Cell size is derived from the window width; there is no density control (B) — SL-B04
- [ ] One selected photo plus the cull follow-selection; no multi-select, marquee or ⇧-range (B) — SL-B15
- [ ] Sources: only All Photos and Recently Deleted; Recently Added, Picks and Rated rows are not built (B) — SL-A14
- [x] Folder tree with recursive counts + drill-down (`library.subfolders`, breadcrumbs) (B) — SL-A03/A04
- [ ] Skeleton thumbs + failure badge (B) — SL-M10
- [x] Synthetic library generator (`sortlens-testkit`: deterministic JPEGs from a real encoder, sidecars by convention, self-cleaning folders) + delta-scan truth table

**Demo:** add 5k folder → grid fills → counts correct → relaunch instant → rescan adds only new
files → rename a subfolder → no duplicate rows.

---

## M2 — Preview scales · 2.5 w · track C

**Goal:** 50k photos, 60 fps, bounded memory, visible activity.
**State:** JPEG and RAW-preview thumbnails, the content-addressed cache, the two-worker decode
service and the activity stack are in; metadata is read during the scan. HEIC, video, priority
scheduling and blur-up remain.

- [x] Contact sheet tiers: 128/384/1024 constants, content-addressed disk cache, bounded texture map (C) — SL-M02/M06
- [x] Cooperative cancellation + live file/folder counters during a scan (C)
- [ ] One worker thread with a job queue is in; priority ordering and backpressure are not (C)
- [x] JPEG decode path (C)
- [x] PNG/TIFF/WebP/GIF/BMP decode paths via craft-image-codecs, alpha preserved (C)
- [ ] HEIC decode path (C)
- [x] RAW embedded previews (`craft-raw`) (C) — SL-B18
- [ ] Video poster + duration/fps/codec (C) — SL-B11
- [x] Placeholder cell while decoding, no spinner (B)
- [ ] Blur-up tier transitions (B)
- [x] Activity stack: job list, statuses, elapsed, per-item errors, cancel (B) — SL-A18
- [ ] Bell popover and unread badges (B)
- [ ] Offline/missing badges + cached-thumb behaviour (A,B) — SL-A12
- [ ] `craft-ipc` + `craft-jsonrpc` are not used: agent access is the MCP server over stdio instead (E) — SL-K04
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
- [ ] Tag rename/merge is in; `photo.tagFromPrev` (re-apply the last tags) is not (A) — SL-D05
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
- [ ] Overlays inside image bounds + two-phase animation + reduced motion (B) — SL-C04
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
- [ ] Collections have favourites, colours, search and sort; folders have none of these, and range-select/select-all is missing (B) — SL-A05..A10
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
- [ ] `--json` for the remaining verbs, and a written compatibility note per schema — the write verbs
  need a defined result shape before they can be called from a script safely (E)
- [ ] i18n catalogs + RTL screenshots (ar/ja/zh-TW/ko/th/vi) (B) — SL-L07
- [ ] Localised numbers/dates (B) — SL-L08
- [ ] `craft-keymap` editor + import/export (B) — SL-I09
- [ ] `craft-update` + in-app release notes (E) — SL-I10
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
the emitted accessibility tree, high contrast, reduced motion, theme import, `craft-update` and the tag are
not — each named with its unblocker in the table at the end of this milestone.

- [x] Accessibility audit, part 1 — **names**: every custom-painted clickable region in the shell (grid
  cell, detail preview, lookahead strip, cull strip, similar clusters, memory cards, sidebar rows) now
  registers a `WidgetInfo` with a description from `a11y::describe_photo`, and the row carries its label
  and count; `cargo xtask a11y` fails when a function makes a region clickable without naming it, proved
  to bite by deleting one registration (B) — SL-L01
- [ ] Accessibility audit, part 2 — focus order, focus-trap checks and a real tree walk: this egui version
  has no public way to read accessibility nodes back, so the emitted tree cannot be asserted here; that
  half needs a platform accessibility client (VoiceOver, Orca, NVDA) on a machine with a desktop session
- [ ] High-contrast preset + reduced-motion toggle (B) — SL-L09/L06 — moved to the table below: not
  attempted, and not because it is hard
- [ ] Theme import via `craft-ext-core` palette (B) — SL-I06
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
- [ ] Packaging: macOS signed/notarised, Windows MSI, Linux AppImage/Flatpak (E) — moved to the table
  below: it needs signing identities this repository does not have
- [x] Benchmarks recorded in `docs/PERF.md`, from `cargo xtask perf` on a named machine, including the
  snapshot defect they exposed and the budgets that cannot be measured here (see M2's row) (E)
- [ ] `craft-update` wired to the launcher (E) — moved to the table below
- [x] `docs/ATTRIBUTION.md` complete and **enforced**: `cargo xtask attribution` (in `ci`) fails when a
  linked crate is unattributed, when a declared-but-unlinked crate is not listed as such, when a table
  names a dependency that no longer exists, when an embedded font family is missing, or when the
  "no model weights, corpora or datasets" statement is deleted — proved to bite both ways (a fake
  `serde_yaml` dependency, and a deleted font row) (E)
- [ ] Release notes + 1.0 tag (E) — moved to the table below: a tag is a decision for the person who
  owns the repository, not something a loop should push

### Deliberately absent from M10

Not rows with an empty box — rows that would need something this repository does not have, kept here so the
absence is a decision rather than a gap:

| Not built | Because | What would unblock it |
| --- | --- | --- |
| Signed and notarised macOS builds (SL-I12) | notarisation needs an Apple Developer identity and a CI secret; a workflow that pretends to sign would produce a bundle Gatekeeper rejects | an identity, `APPLE_ID`/`TEAM_ID`/app-specific password in CI secrets, and a release run to test against |
| Windows MSI and Linux AppImage/Flatpak | each needs its own toolchain (`cargo-wix`, `linuxdeploy`, Flatpak manifests and a runtime) and, for MSI, a code-signing certificate | those toolchains in the release workflow and a signing certificate |
| `.icns`/`.ico` generation and a brand pass | the icon is drawn and wired (`crates/icon`, `with_icon`), but the per-platform icon containers and any marketing artwork are packaging work that the absent installer work would consume | the packaging step above; the drawing itself already exists |
| Theme import via `craft-ext-core` palettes (SL-I06) | importing someone else's palette means committing its colours and its licence; no palette has been chosen, and inventing one is not an import | a chosen palette with a licence that permits redistribution |
| High-contrast preset and reduced-motion toggle (SL-L09/L06) | both are real design decisions — which contrast pair, which motion to drop — and the headless harness cannot show whether the result is readable or comfortable | a design pass and a person looking at it |
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
| 2026-11-21 | Plan written: feature inventory (176), architecture, tooling audit, UX spec, roadmap. Implementation not started; Rust toolchain not yet installed on this machine. |
| 2026-11-21 | M0 skeleton built on macOS. Toolchain 1.95.0 installed project-locally (`scripts/bootstrap.sh`, `.toolchain/` gitignored). 23 crates scaffolded; `sortlens-core` (ids, labels, units, errors), `sortlens-engine` (registry + `run`), `sortlens-ui` (4 presets, five surfaces, settings/about dialogs), the `sortlens` app (title-bar policy, `craft-settings` store, `fn ui` shell) all compile. `cargo xtask ci` green: fmt, clippy `-D warnings`, 20 tests, layer rules, lint headers, asset provenance. `cargo xtask wasm` green for `sortlens-core` + `sortlens-ui`. `docs/BEHAVIOUR.md` written. Not done: fonts, `cargo deny`, snapshot CI, shortcuts dialog. |
