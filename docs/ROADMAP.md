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
- [ ] Benchmarks: cold thumb rate, 100k scroll p99, RSS (E)

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

- [x] Model manifest (id, kind, licence, digest, size) + content-addressed store that verifies size
  and SHA-256 before use, installs atomically and refuses to delete a file it cannot identify (D)
  — SL-F01; the **downloader** (SL-F02, consent-gated, resumable) is still unbuilt and unlisted
- [ ] `tract` detector session + NMS + label map (D) — SL-F03
- [ ] Image/text embedding sessions + precomputed pack prompts (D) — SL-F04
- [ ] EXIF hint rules (D) — SL-F05
- [ ] Suggestion merger + caps + UI panel + accept → tag (D,B) — SL-F06
- [ ] Packs as data + counts in settings + toggles (`craft-flags`) (D,B) — SL-F07/F08
- [ ] Batch suggest job (D) — SL-F09
- [x] No-network assertion for `sortlens-ai`: a source scan for socket APIs and a dependency scan for
  HTTP clients, both proved to fail on a planted hit (D) — SL-F10
- [x] Export: original copies and JPEG re-encodes, longest-edge resize, quality, EXIF/IPTC carried
  forward, four collision policies, worker progress and cancel, a dialog and a receipt (A) — SL-B24
- [ ] Export PNG/TIFF/WebP — needs an encoder this workspace does not have yet; the dialog offers only
  the two formats it can actually write

**Demo:** on a clean machine, download the model, suggest tags for 50 photos offline, accept
10, filter by `source = Ai`; export a selection three ways and diff metadata.

---

## M9 — Remote + automation · 2.5 w · tracks E,B

- [x] MCP server: `sortlens-mcp` — newline-delimited JSON-RPC over stdio, `tools/list` straight from the command registry, `tools/call` through `Engine::run`, `sortlens-cli mcp` (E) — SL-K05
- [ ] CLI parity + stable JSON schemas (E) — SL-K03/K06
- [ ] `sortlens-server`: token auth, pairing, permissions, upload resume (E) — SL-J01/J03/J04
- [ ] `sortlens-web` wasm client reusing the UI (B,E) — SL-J02
- [ ] Remote album/collection create + rescan (E) — SL-J06/J08
- [ ] i18n catalogs + RTL screenshots (ar/ja/zh-TW/ko/th/vi) (B) — SL-L07
- [ ] Localised numbers/dates (B) — SL-L08
- [ ] `craft-keymap` editor + import/export (B) — SL-I09
- [ ] `craft-update` + in-app release notes (E) — SL-I10
- [ ] Diagnostics bundle (E) — SL-I14

**Demo:** cull 20 photos from a phone over LAN; an agent runs a full cull + report via MCP in
CI; the UI renders correctly in Arabic and Japanese.

---

## M10 — Release · 2.0 w · all tracks

- [ ] Accessibility audit: focus order, roles, names, announcements, no traps (B) — SL-L01..L05
- [ ] High-contrast preset + reduced-motion toggle (B) — SL-L09/L06
- [ ] Theme import via `craft-ext-core` palette (B) — SL-I06
- [ ] Icon set final pass + brand assets (B) — SL-I11
- [ ] Packaging: macOS signed/notarised, Windows MSI, Linux AppImage/Flatpak (E)
- [ ] `craft-update` wired to the launcher (E)
- [ ] Benchmarks recorded in `docs/PERF.md`; budgets green (E)
- [ ] `docs/ATTRIBUTION.md` complete (fonts, models, data, corpora) (E)
- [ ] Release notes + 1.0 tag (E)

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
