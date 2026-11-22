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
**State:** built and green on macOS; the four unchecked rows are the remainder.

- [x] `rust-toolchain.toml` (1.95.0, wasm target), `.cargo/config.toml` alias; egui + winit forks patched in `Cargo.toml`
- [x] Workspace: 19 library crates + 4 app crates; `[lints] workspace = true`; crate-root denies (A)
- [x] `xtask`: `ci`, `fmt`, `clippy`, `test`, `layers`, `roots`, `assets`, `wasm` (E)
- [ ] `cargo deny` allow-list; `Cargo.lock` committed (E) — policy is written in `docs/ATTRIBUTION.md`, tool not wired
- [x] App shell: top bar, bottom bar, left/right panels, canvas (B)
- [x] `craft-window` title-bar policy; macOS traffic-light inset (B)
- [ ] `craft_ui` fonts (`Inter` + craft-fonts) + `assert_covers` for `en` (B) — egui defaults are in use
- [x] Four theme presets + contrast tests (B) — widget states set on `Visuals`; `WidgetPalette` mapping still open
- [x] `craft-settings` v1 type, damaged-file notice, unknown-key preservation (A)
- [x] Command registry skeleton; `Engine::run` returns typed errors (A)
- [x] Commands: `app.about`, `app.settings`, `app.quit`, `app.supportReport`; 17 planned commands carry their milestone (A,B)
- [ ] `app.shortcuts` dialog and `view.theme` as a command (A,B)
- [x] Empty-library state (B)
- [ ] CI: 3 OSes + wasm check + snapshot job (E) — 3 OSes and wasm are in; no snapshot job

**Demo:** launch → switch theme → open settings → quit → relaunch with the theme preserved;
`cargo xtask ci` green.

---

## M1 — Index · 2.5 w · tracks A,C

**Goal:** a 5,000-photo folder becomes a browsable grid with correct counts.
**State:** catalog, scan, grid, commands and CLI are in (commit after M0); metadata and
thumbnails move to M2 because the grid renders before it has pixels to draw.

- [x] `sortlens-core`: ids, time, byte units, versioned errors (A)
- [ ] Volume probe + `PathKey`/`ContentKey` identity (A) — SL-A11
- [x] Walker with skip rules, bounded queue, delta detection (A) — SL-A01/A17
- [x] `sortlens-catalog`: op log, snapshots, replay, undo, paging, counters (A) — SL-A12/A13
- [x] Engine scan worker, `library.addFolder`/`rescan`/`page`/`stats` (A)
- [x] Virtualised grid with selection, counts and keyboard rating/flags (B)
- [x] CLI: add, list, rate, flag, undo (E)
- [ ] `craft-metadata` + `craft-tiff` → `PhotoMeta` (C) — SL-B21 data
- [ ] Format routing + magic sniff (C) — SL-B11
- [ ] Catalog: ops, snapshot, replay, counters (A) — SL-H09
- [ ] `library.addFolder/removeFolder/rescan/source` (A) — SL-A01/A19
- [ ] Justified + square grid, virtualised (B) — SL-B01/B02
- [ ] Density slider (B) — SL-B04
- [ ] Selection model + marquee + ⌘A/⌘D (B) — SL-B15
- [ ] Sources: All / Recently Added / Picks / Rated (B) — SL-A14
- [ ] Folder tree with counts (B) — SL-A03/A04
- [ ] Skeleton thumbs + failure badge (B) — SL-M10
- [ ] Synthetic library generator + delta-scan truth table (testkit)

**Demo:** add 5k folder → grid fills → counts correct → relaunch instant → rescan adds only new
files → rename a subfolder → no duplicate rows.

---

## M2 — Preview scales · 2.5 w · track C

**Goal:** 50k photos, 60 fps, bounded memory, visible activity.
**State:** JPEG thumbnails, the disk cache and the two-worker decode service are in; metadata is
read during the scan. RAW, HEIC, video, priority scheduling and the activity stack remain.

- [x] Contact sheet tiers: 128/384/1024 constants, content-addressed disk cache, bounded texture map (C) — SL-M02/M06
- [x] Cooperative cancellation + live file/folder counters during a scan (C)
- [ ] Priority scheduler + backpressure (C)
- [x] Cooperative cancellation + live file/folder counters during a scan (C)
- [ ] Priority scheduler + backpressure (C)
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
- [ ] `craft-ipc` + `craft-jsonrpc` control channel (E) — SL-K04
- [x] `sortlens-cli add|rescan|list|stats|rate|flag|undo|report` (E) — SL-K03
- [ ] JSON output mode for the CLI (E)
- [ ] Benchmarks: cold thumb rate, 100k scroll p99, RSS (E)

**Demo:** 50k library scrolls at 60 fps; unplug the drive → offline badges, no freeze; CLI
scans headlessly and prints a JSON report.

---

## M3 — Annotate · 2.0 w · tracks A,B

**Goal:** the library remembers what the photographer decided, on disk and in XMP.

- [x] `photo.rate/flag/label/tag` + batch variants (A) — SL-D01..D05
- [ ] `photo.tagFromPrev` and tag rename/merge (A) — SL-D05
- [ ] Tag registry: create/rename/merge/delete, autocomplete index (A) — SL-D06
- [x] `sortlens-sidecar`: XMP read/merge/write, unknown fields preserved, atomic (A) — SL-D08/D09
- [x] Sidecar review imported when a photo is first indexed (A) — SL-D09
- [ ] External-change detection for already-known photos + retry queue (A) — SL-D11
- [ ] Stale-revision rejection (`Error::StaleRevision`) (A) — SL-D11
- [ ] Rating/flag/label pill + inspector + context menus (B) — SL-D01..D03
- [x] Filter bar: rating, flags, labels, keyword, collection, kind, edited, missing, clear (B) — SL-B07..B13
- [x] Keyword ANY/ALL, multi-keyword filtering, active-filter badge (B)
- [x] Sort keys: added/name/newest/oldest/largest/rating, persisted (B) — SL-B06
- [x] Detail view: large canvas, counter, fit/1:1, arrow navigation, facts line (B) — SL-B18..B20
- [x] Detail lookahead strip + 64-bin luminance histogram (B) — SL-B21
- [ ] Shortcut overlay + full keymap wiring (B) — SL-B23
- [ ] Undo/redo (20+) + audit log + viewer skeleton (A) — SL-D10/H04
- [ ] `photo.reveal/open` (A) — SL-B22
- [ ] Tests: XMP golden, filter algebra, stale revision, contrast of badges

**Demo:** rate/tag 100 photos → quit → reopen: identical state; `xmllint`-clean sidecars contain
the tags; filter "≥4★ + Pick + #wildlife" in one keystroke; undo 20 steps.

---

## M4 — Cull · 2.5 w · tracks A,B

**Goal:** the signature workflow, safe by construction.

- [x] `sortlens-cull` state machine + transition table tests (A) — SL-C09
- [x] Session persistence + resume + crash resume (A) — SL-C07
- [x] Cull view: canvas, up-next strip, action bar, live counts (B) — SL-C01/C06/C17
- [ ] Cull summary sheet + applied-batch receipt (B) — SL-C17
- [ ] Overlays inside image bounds + two-phase animation + reduced motion (B) — SL-C04
- [x] Keyboard decisions: P/X/Space/U/←/C (B) — SL-C02
- [ ] Gestures (drag up/down) (B) — SL-C03
- [ ] Zoom levels + pan (B) — SL-C05? (zoom) SL-C02
- [ ] Prefetch 3 ahead with priority (C) — SL-C05
- [ ] Apply confirmation with counts (A,B) — SL-C10; size/list still to come
- [x] Staged deletes and Apply: audit, move/tag/trash, per-item results (A) — SL-C10
- [x] Trash executor: audit-before-act, verify, per-item results, grouped inverse op (A) — SL-H01/H03
- [ ] Quit guard for running jobs/unsaved sidecars (B) — SL-H12
- [ ] Accessibility: decision announcements + button names (B) — SL-L02/L03
- [ ] Tests: reject never deletes; skip preserves flags; apply idempotent; crash resume

**Demo:** cull 500 frames keyboard-only, kill the app mid-session, relaunch and resume at the
same photo; apply 148 rejects to trash; undo restores all 148.

---

## M5 — Organise · 2.0 w · tracks A,B

- [x] Subfolder-first drill-down: one level at a time, recursive counts, breadcrumbs (B) — SL-A03
- [ ] Folder favourites/colours/search/sort/range-select/select-all (B) — SL-A05..A10
- [x] Collections: create/rename/delete, add/remove, sidebar, filter (A,B) — SL-G01..G03
- [ ] Collection favourites, colours, per-collection search and sort (B) — SL-G04
- [ ] Batch move with verification + undo (A) — SL-G07
- [x] Persistent undo across restarts: the inverse travels in the log record and in the snapshot (A) — SL-A13
- [ ] Inline tagging + re-apply last tags in cull (B) — SL-C11/C12
- [x] Library doctor: check, find-moved-by-content-key, remove-missing, all undoable (A,B) — SL-H07/H08/A13
- [ ] First-run wizard (library → folder → remote test) (B) — SL-I12
- [ ] Tag rename/merge UI (B) — SL-D06

**Demo:** three folders → two collections; move 200 files with hash verification and undo;
remap a moved drive; run the wizard on a clean machine.

---

## M6 — Analyse · 2.5 w · tracks D,B

- [ ] `sortlens-quality`: sharpness score + versioning + tests (D) — SL-E01
- [ ] Blur scan job + threshold review + batch select (D,B) — SL-E01/E02
- [x] `sortlens-hash`: dHash + aHash over a 9×8 box-filtered grid, Hamming distance, 3×3 colour fingerprint (D) — SL-E03
- [x] Similar groups (union-find chaining), best pick, similar-to (D,B) — SL-E04/E05
- [x] Similar: BK-tree metric index (exact, sub-quadratic) and a colour veto (D) — SL-E03
- [ ] Similar: side-by-side compare and colour-only matches (B) — SL-E05
- [ ] Batch select of group leftovers and delete (B) — SL-E05
- [ ] `sortlens-geo`: projection, clusters, gazetteer (D) — SL-E06/E07
- [ ] Offline basemap + optional MBTiles + attribution (B) — SL-E06
- [x] Insights: totals, rating/label/kind distribution, cameras, lenses, keywords, timeline — SL-E08
- [ ] Insights: progress, highlights, seasonality (B) — SL-E09..E11
- [ ] Memories providers + card feed (D,B) — SL-E12
- [ ] Chart component (fixed height, no chart dep) (B) — SL-E11
- [ ] Tests: blur ladder, duplicate transforms, cluster determinism

**Demo:** find 20 blurred frames and 12 duplicate groups in a 5k library; map clusters match
GPS; every dashboard number reconciles with the catalog; memories stable across refresh.

---

## M7 — Safety · 1.5 w · track A

- [ ] Three delete modes + distinct executors (A) — SL-H01
- [x] Rejects bin: applied batches listed, restored with flags and missing state (A,B) — SL-A15
- [x] Empty the bin: forget one batch or all of them, files untouched, with a confirmation that names the counts (A,B)
- [x] Recently Deleted source: sidebar row + restore dialog (B) — SL-A16
- [ ] Backup/restore (catalog + settings, verified) (A) — SL-H10
- [ ] Volume remap with sample verification (A) — SL-A20/H06
- [ ] Integrity check + compaction (A) — SL-H11
- [x] Change viewer (recent ops, newest first) + plain-text support report (B) — SL-H04
- [ ] Read-only library mode (A) — SL-H14
- [ ] Crash/corruption suite expanded (truncation, CRC flip, partial snapshot)

**Demo:** delete in all three modes; restore from bin and from OS trash; corrupt the op-log tail
and recover; export/import a library onto a second machine and remap the volume.

---

## M8 — AI + export · 2.5 w · tracks D,A

- [ ] Model manifest + content-addressed store + digest verify (D) — SL-F01/F02
- [ ] `tract` detector session + NMS + label map (D) — SL-F03
- [ ] Image/text embedding sessions + precomputed pack prompts (D) — SL-F04
- [ ] EXIF hint rules (D) — SL-F05
- [ ] Suggestion merger + caps + UI panel + accept → tag (D,B) — SL-F06
- [ ] Packs as data + counts in settings + toggles (`craft-flags`) (D,B) — SL-F07/F08
- [ ] Batch suggest job (D) — SL-F09
- [ ] No-network assertion test for `sortlens-ai` (D) — SL-F10
- [ ] Export: original/JPEG/PNG, resize, quality, metadata, collisions, progress (A) — SL-B24

**Demo:** on a clean machine, download the model, suggest tags for 50 photos offline, accept
10, filter by `source = Ai`; export a selection three ways and diff metadata.

---

## M9 — Remote + automation · 2.5 w · tracks E,B

- [ ] MCP server: tool list from the registry, read/write roots (E) — SL-K05
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
