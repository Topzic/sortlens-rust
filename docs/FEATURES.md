# Sortlens — Feature Deep Analysis

Purpose: a complete, buildable inventory of what Sortlens does today, so the Rust
recreation can reach parity deliberately instead of by accident.

> The `M` column is the milestone a feature was *planned* for. It is not a status: some planned rows were
> delivered later than their letter, and some were deliberately left out. `docs/ROADMAP.md` is the
> tracker, `CHECKPOINT.md` is the record of what actually shipped, and `cargo xtask docs` keeps the
> README and PLAN's headline range in step with the tracker.

## Method and evidence

Sortlens (Topzic/Sortlens, v0.11.0, proprietary, Windows + macOS + iOS) is published
as a packaged app. Its repository contains a README, screenshots and release notes —
no source. This analysis therefore comes from public behaviour only:

| Tag | Source | Trust |
| --- | --- | --- |
| **[D]** | README, release notes, release bodies | Documented by the vendor |
| **[S]** | Screenshots in `docs/screenshots/` | Observed UI, current-ish build |
| **[I]** | Inferred from [D]/[S] (measurements, ordering, defaults) | Our reading; verify in M0 |
| **[?]** | Not stated anywhere | Unknown; we decide our own behaviour |

No part of this plan depends on decompiling, disassembling or operating the proprietary
binary. That matches ArtCraft clean-room rules (`standards/clean-room.md`, AGENTS.md §23):
a behaviour specification is written first, code is written from the specification, and
no vendor asset, string or code is copied. Sortlens' own screenshots are not shipped as
assets; only used as evidence here.

The existing implementation is Python (FastAPI-style backend) + React/TypeScript frontend
inside a `pywebview` window, with a bundled Python runtime, venv repair at startup,
dynamic backend port discovery written to a `.port` file, PyInstaller packaging, Inno Setup
on Windows, and a PowerShell-based self-updater. **None of that is recreated.** It exists
only because of the webview stack; the Rust app has no backend process, no ports, no venv,
no WebView and no script-based updater.

## Product thesis

A local-first culling and library tool for photographers returning from a shoot with
1,000–10,000 frames and for libraries of 50k–200k images. The job to be done, in order:

1. **Ingest fast** — point at folders, index, see thumbnails, nothing uploaded.
2. **Cull fast** — one decision per frame with keyboard or gesture, staged and reversible.
3. **Label** — stars, colour labels, pick/reject flags, tags, written to XMP so other
   tools see them.
4. **Find** — search, filters, smart collections, similarity, soft-focus, place.
5. **Understand** — what the library contains, what was shot when, what is still unreviewed.
6. **Never lose anything** — staged deletes, trash by default, undo, audit log, backups,
   volume remapping.

Everything else (AI suggestions, map, memories, remote phone access) is a multiplier on
those six, never a prerequisite.

## Coverage summary

| Domain | Features | Phase | Notes |
| --- | ---: | --- | --- |
| A. Library sources, folders, volumes | 20 | M1–M5 | Identity + offline handling is the hard part |
| B. Browse, grid, lightbox, batch | 24 | M1–M4 | Virtualisation and preview pipeline drive this |
| C. Cull (rapid review) | 17 | M4 | The signature feature; state machine + staging |
| D. Ratings, labels, flags, tags | 13 | M3 | XMP sidecar is the interop contract |
| E. Analysis: blur, similarity, map, insights, memories | 14 | M6 | All local, all incremental |
| F. Local AI suggestions | 11 | M8 | Optional, downloaded, CPU, gated |
| G. Collections and albums | 10 | M5 | Static vs smart vs on-disk |
| H. Safety, data, recovery | 14 | M3, M7 | Non-negotiable; shipped before deletes exist |
| I. Settings, appearance, platform | 16 | M0–M10 | ArtCraft chrome, not the blue webview look |
| J. Profiles and remote access | 9 | M9 | Optional; phone app + local server |
| K. Automation (CLI/control/MCP) | 8 | M2–M9 | Every UI action is a command from day one |
| L. Accessibility and i18n | 9 | continuous | Required by ArtCraft standards |
| M. Performance and scale | 11 | continuous | Budgeted and CI-checked |
| **Total** | **176** | | 70 P0, 56 P1, 39 P2 (11 performance budgets carry no priority) |

Priority key: **P0** = v1.0 blocker (without it the app has no reason to exist),
**P1** = expected in v1.x (parity gap users would notice), **P2** = later/optional
(differentiator or long tail).

---

## A. Library sources, folders, volumes

| ID | Feature | Evidence | Behaviour | Rust approach | Pri | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| SL-A01 | Add folder as library source | [D] | Sidebar ➕ adds any folder; images stay in place | `folder.add` command → walker → index | P0 | M1 |
| SL-A02 | Multiple source folders | [D] | Unlimited roots, browsable together or one at a time | `Folder{root_id, rel_path}`; `library.source` filter | P0 | M1 |
| SL-A03 | Subfolder-first drill-down | [D] v0.11 | Opening a folder shows subfolders, then photos | Virtual `FolderRow` nodes from the rel-path tree | P0 | M5 |
| SL-A04 | Folder counts | [S] | Counts beside each folder, live after every op | Maintained counters in catalog, not per-frame SQL | P0 | M1 |
| SL-A05 | Folder favourites | [D] | Pin folders, shown in a Favourites group | `favorite` flag + sidebar section | P1 | M5 |
| SL-A06 | Folder colour accents | [D]/[S] | Colour dot on the row and its icon | `label` on folders, theme palette | P1 | M5 |
| SL-A07 | Folder search | [D] | Filter the folder list by name | In-memory lowercase match over the tree | P1 | M5 |
| SL-A08 | Folder sort (name / count / date) | [D] | Sort control in the section header | `folder.sort` per section, persisted | P1 | M5 |
| SL-A09 | Multi-select folders incl. range select | [D] | Ctrl+Click two folders selects everything between | Anchor + index range over the visible row list | P1 | M5 |
| SL-A10 | Select all / deselect all folders | [D] v0.3.5 | Header actions | `selection.set` | P1 | M5 |
| SL-A11 | External device awareness | [D] | USB/SD/mounted phone detected; library keeps its photos | Volume identity: serial/label + mount probe | P0 | M1 |
| SL-A12 | Offline indicators | [D] | Badge on the folder and thumbnails when the source is gone | `offline` state from volume probe; cached previews still shown | P0 | M2 |
| SL-A13 | Missing photos source | [D] | Dedicated source listing photos whose file cannot be found | Periodic sweep (not per frame), cancellable | P0 | M5 |
| SL-A14 | Recently Added / Picks / Rated / All sources | [S] | Built-in sources in the sidebar | Derived views over the catalog | P0 | M1 |
| SL-A15 | Rejects bin ("Recycling Bin") | [D] v0.11 | Rejected photos collect in a bin; restore or delete | `deleted_at` + `reject_reason`; restore clears flag | P0 | M7 |
| SL-A16 | Recently Deleted | [S]/[I] | Sidebar source for deleted items | Same table, different predicate | P1 | M7 |
| SL-A17 | Rescan / incremental scan | [D] | Rescan from the sidebar; only changed files are re-read | `(volume, file_index, size, mtime)` identity; delta walk | P0 | M1 |
| SL-A18 | Scan progress + cancel | [D] | Toast/activity row with progress; safe to cancel | Activity stack entry with token + atomic commit batches | P0 | M2 |
| SL-A19 | Remove folder without deleting photos | [D] v0.6.2 | Detach source, files untouched | `folder.remove` (catalog only) | P0 | M1 |
| SL-A20 | Remap a moved folder / drive | [D] | Point a missing root at its new location | `app.remap {folder,new_root}` rewrites rel paths under one transaction | P0 | M7 |

**Design note — identity.** Path-based identity breaks on rename, move and remount; name-based
breaks on duplicates. Use a three-tier key: `(volume_serial, volume_relative_path)` as the
primary identity, `(size, mtime, first 64 KiB hash)` as a corroborating fingerprint, and a
content hash only for the expensive dedupe path. A move inside a volume is then a rename,
which is detectable in the delta walk, and a remount on another machine is a remap, not a
re-import.

---

## B. Browse, grid, lightbox, batch

| ID | Feature | Evidence | Behaviour | Rust approach | Pri | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| SL-B01 | Photo grid (justified rows) | [S] | Rows of equal height, natural aspect | Row packer over cached aspect ratios | P0 | M1 |
| SL-B02 | Square grid | [S] | Uniform cells, same data | Second layout strategy, same cell painter | P0 | M1 |
| SL-B03 | List/detail view | [S] | Compact rows with metadata columns | `egui_extras::TableBuilder`-style virtual rows | P2 | M6 |
| SL-B04 | Density / thumbnail size slider | [S] | Continuous size, `-`/`=` steps | Token-driven size, re-flow without reload | P0 | M1 |
| SL-B05 | Filename search | [S] | Centred top-bar field, live | Trigram or lowercase-needle over an index | P0 | M3 |
| SL-B06 | Sort by name/size/date/rating/dimensions/EXIF date | [D] | Asc/desc | Sort keys precomputed per view, stable tiebreak by id | P0 | M3 |
| SL-B07 | Rating filter with operator | [S] | `≥ / = / ≤` + 0–5 stars | `RatingOp` in filter struct | P1 | M3 |
| SL-B08 | Flag filter (multi-select) | [D]/[S] | Pick / Unflagged / Reject, any-of | Bitset in filter | P1 | M3 |
| SL-B09 | Colour-label filter (multi-select) | [D]/[S] | Any-of five labels | Bitset in filter | P1 | M3 |
| SL-B10 | Tag filter with ANY / ALL | [D] | Comma-separated tags, matching mode | Normalise + dedupe, then set intersection/subset | P0 | M3 |
| SL-B11 | Kind filter (photo / raw / video / merged) | [D] v0.8 | Media-kind chips | `MediaKind` enum, video included | P1 | M3 |
| SL-B12 | Metadata pickers (camera, lens, tag) | [S] | Dropdown lists from the library's own values | Distinct-value cache, invalidated by op log | P1 | M3 |
| SL-B13 | Filter bar toggle with active-count badge | [S] | Icon + badge in the top bar | `view.filterBar`, count from filter chips | P1 | M3 |
| SL-B14 | Filter presets | [I] (LightCraft parity) | Save/apply/delete named filters | Serialisable filter struct + presets list | P2 | M6 |
| SL-B15 | Selection: click, ctrl-click, shift-click, ctrl+A, ctrl+D | [D] | Anchor + range + toggle | Selection {anchor, ids, active}, all commands take id sets | P0 | M1 |
| SL-B16 | Batch toolbar | [D] | Appears with ≥2 selected: delete, move, tag, rate, label, export | Contextual action row bound to `selection` | P0 | M4 |
| SL-B17 | Context menus (grid, folder, collection, photo) | [D] | Right-click operations incl. reveal, open, move, remove from collection | One menu builder per entity, ids registered for automation | P0 | M3 |
| SL-B18 | Lightbox | [D] | Full-resolution view with EXIF panel | `View::Detail`; decode at screen size, upgrade to full on zoom | P0 | M3 |
| SL-B19 | Lightbox position counter | [D] v0.7 | "42 / 1,203" | Cursor index into the active ordered view | P1 | M3 |
| SL-B20 | Seamless pagination / pre-fetch | [D] v0.7 | Next page loads before the boundary | Windowed source with a look-ahead reader | P0 | M2 |
| SL-B21 | EXIF panel | [D] | Camera, lens, ISO, aperture, shutter, dimensions, GPS, capture time | `craft-metadata` neutral model → grouped rows | P0 | M3 |
| SL-B22 | Reveal in file manager / open in editor | [D] | `R` reveals, `E` opens in the configured editor | `craft-dialogs` + platform reveal; per-OS open-with | P1 | M3 |
| SL-B23 | Keyboard navigation and shortcut overlay | [D] | Arrows, 0–5, 6–9, P/U/X, Delete, Enter, Esc, `?` | `craft-keymap` catalog + overlay panel | P0 | M2 |
| SL-B24 | Export selection (original / JPEG / PNG) | [D] v0.8 | Converted export with size/quality choices | `export` job on a worker; metadata policy explicit | P2 | M8 |

**Design note — the view is a query, not a list.** Every grid is `(filter, sort, scope)` evaluated
against the catalog with a stable cursor. That makes "the lightbox prefetches the next page",
"undo restores the selection", "a swipe session resumes at photo N" and "an agent asks for the
same view" the same mechanism instead of four.

---

## C. Cull (rapid review)

The signature feature. Sortlens calls it Swipe; the behaviour matters more than the name.

| ID | Feature | Evidence | Behaviour | Rust approach | Pri | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| SL-C01 | Full-screen cull view | [S] | One photo centred, next-up strip on the right, action bar below | `View::Cull`; canvas + strip + bar | P0 | M4 |
| SL-C02 | Keyboard decisions | [D] | Keep `→`/`K`, Reject `←`/`X`, Skip `↓`/`S`, Undo `⌫`/`U`, Favourite `F`, Zoom `1`–`4` | Command ids `cull.keep` etc., all key-bindable | P0 | M4 |
| SL-C03 | Gesture decisions | [D] | Swipe right/left/down/up | Pointer drag → threshold + velocity → same command | P1 | M4 |
| SL-C04 | Decision overlays | [D] v0.6 | KEEP / REJECT / SKIP / STAR drawn over the image, then slide | Two-phase animation: label, then transform | P1 | M4 |
| SL-C05 | Prefetch next 3 images | [D] | Next-up strip and the next full frames are decoded ahead | Priority queue + generation tokens; 3–6 lookahead | P0 | M4 |
| SL-C06 | Up-next strip with filenames | [S] | 3 upcoming thumbnails | Windowed strip, same source as the canvas | P1 | M4 |
| SL-C07 | Resume session exactly | [D] | Reopening a folder continues where it stopped | `CullSession{cursor, decisions}` persisted per scope | P0 | M4 |
| SL-C08 | Order by filename or capture date | [D] | Two orderings, remembered | `order` in session, sort key in the view | P1 | M4 |
| SL-C09 | Staged decisions | [D] | Nothing is deleted until Apply | Decisions in the session, not the file system | P0 | M4 |
| SL-C10 | Apply dialog with impact | [D] | Count, total size, file list; mode choice | Pre-flight stat pass, then an `apply` job | P0 | M4 |
| SL-C11 | Inline tagging during cull | [D] | Tags and suggestions available without leaving the view | Right inspector panel, same commands | P1 | M5 |
| SL-C12 | Re-apply last tags | [D] v0.7 | One click carries the previous photo's tags | `photo.tag {from: prev}` semantics | P2 | M5 |
| SL-C13 | Compare (2-up) | [I] | Side-by-side with synced zoom | Two canvases sharing a zoom/pan state | P2 | M8 |
| SL-C14 | Auto-advance after rate/flag | [I] | Next photo after a decision | Session cursor advance policy | P2 | M8 |
| SL-C15 | Full-screen and lights-out | [I] | Chrome fades away | Canvas-only mode, veil opacity in tokens | P2 | M8 |
| SL-C16 | Traffic-light confidence (burst grouping) | [?] | Not in Sortlens; ArtCraft-style addition | Group near-identical frames, cull one per burst | P2 | M9 |
| SL-C17 | Session summary | [?] | Kept / rejected / skipped, time, rate | Computed from the session | P2 | M8 |

**State machine (normative for our build).**

```
Idle ──start(scope,order)──▶ Ready ──decide(keep|reject|skip)──▶ Decided
                              ▲                                     │
                              └──────── undo ◀───────────────────────┘
Ready ──apply(mode)──▶ Applying ──ok/err──▶ Applied | Failed
Applied ──undo──▶ Restored (if the mode was recoverable)
```

Rules that are easy to get wrong and therefore go in the spec and the tests:

1. `reject` never touches the file system. It sets `flag = Reject` in the session and marks the
   photo for the bin on apply. Undo clears the flag.
2. `skip` is not `unflag`: the photo keeps any existing flag and only moves the cursor.
3. Decisions are idempotent per photo; deciding twice replaces the decision and keeps one
   undo step.
4. Apply is a single transaction per mode: either every photo is moved/trashed, or the batch
   reports exactly which succeeded and which failed, with per-item reasons.
5. The cursor is durable: crash and relaunch resumes at the same photo with the same decisions.
6. A photo that disappears (external drive unplugged) pauses the session with a reason rather
   than skipping silently.
7. Rate/favourite during cull is an annotation (durable immediately), not a staged decision.

---

## D. Ratings, labels, flags, tags

| ID | Feature | Evidence | Behaviour | Rust approach | Pri | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| SL-D01 | Star rating 0–5 | [D] | Independent annotation, batch-applicable | `rating: u8` in catalog, op `photo.rate` | P0 | M3 |
| SL-D02 | Colour labels (red/yellow/green/blue/purple) | [D] | Five labels, batch-applicable | `label: Option<ColorLabel>`, theme palette with contrast tests | P0 | M3 |
| SL-D03 | Flags (pick / unflagged / reject) | [D] | Tri-state, independent of rating | `flag: Flag` | P0 | M3 |
| SL-D04 | Manual tags with autocomplete | [D] | Add/remove tags, suggestions from history | Interned tag names, prefix index | P0 | M3 |
| SL-D05 | Batch tagging | [D] | Apply tag set to selection | One op with an id set | P0 | M3 |
| SL-D06 | Tag rename / merge / delete | [D] v0.6.2 | Registry management | Ops that rewrite `PhotoTag` rows transactionally | P1 | M5 |
| SL-D07 | Tag filter ANY / ALL | [D] | Comma-separated, normalised | Set ops in the query | P0 | M3 |
| SL-D08 | XMP sidecar write-back | [D] | Tags written to standard sidecars; other tools read them | `craft-metadata` XMP writer, atomic replace, `.xmp` naming rules | P0 | M3 |
| SL-D09 | Read XMP on import | [D] | Existing ratings/labels/tags imported | XMP read path in the scan, sidecar wins over empty catalog | P1 | M3 |
| SL-D10 | Annotation undo (20 + steps) | [D] | Undo/redo across annotations and file ops | Op log with inverse ops, bounded stack | P0 | M3 |
| SL-D11 | External-change conflict handling | [D] v0.8 fixes | No tag loss under races | Single-writer catalog; per-photo revision; sidecar merge rules | P0 | M3 |
| SL-D12 | Tag source and confidence | [I] (AI) | Manual vs AI-suggested vs XMP | `PhotoTag{source, confidence}` | P1 | M8 |
| SL-D13 | Keywords panel with hierarchy | [I] (ArtCraft parity) | Flat in Sortlens; optional tree | `Tag.parent` optional, flat by default | P2 | M9 |

**Design note — why the race bugs existed.** Sortlens' v0.6.2/v0.8 notes mention "frontend async
race conditions while adding or removing tags" and "stale tag fetches overwriting newer tag
changes". A UI that PATCHes per-tag over HTTP has no ordering guarantee. Our fix is structural:
the UI sends one command with a full intent, the engine applies it on a single writer thread,
and every response carries the catalog revision so a stale render can be dropped rather than
written back. Tags are never patched field-by-field from the view. This is shipped, not just
designed: `Photo.rev` rises once per real annotation change, `Catalog::commit_expect` refuses a
write made against an older revision, every command response carries `rev`, and any command can
pass `expect` to be refused instead of overwriting newer work (SL-D11).

**Sidecar policy.** Write only when the user has opted in (default on for tags/rating/label,
off for cull decisions), write only changed fields, keep unknown fields verbatim, use a
temp-file + rename, and record the sidecar's mtime so an external edit is detected and merged
rather than clobbered. A photo whose sidecar cannot be written is reported in the activity
stack, not silently ignored.

---

## E. Analysis: soft focus, similarity, places, insights, memories

| ID | Feature | Evidence | Behaviour | Rust approach | Pri | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| SL-E01 | Soft-focus (blur) scan | [D] | Laplacian-variance scan, adjustable threshold, review list, batch scan | 1024 px luma → Laplacian variance, normalised by contrast; score cached | P0 | M6 |
| SL-E02 | Blur review with threshold | [D] | Slider changes the flagged set without rescanning | Score stored per photo; filter is a comparison | P1 | M6 |
| SL-E03 | Similar-shots finder | [D] | Perceptual hash groups, adjustable threshold, best pick, bulk delete | 64-bit dHash + 64-bit pHash, BK-tree/banded LSH, best-pick score | P0 | M6 |
| SL-E04 | Duplicate group UI | [D] | Groups with representative, select-all-dupes | Group model, per-group selection helpers | P1 | M6 |
| SL-E05 | Find similar to this photo | [I] (LightCraft parity) | Reference photo → ranked matches | Hamming distance over hashes; embeddings later | P2 | M8 |
| SL-E06 | Places map with clusters | [S] | Dark map, photo clusters, geotagged count, zoom | GPS → Web Mercator; offline basemap + optional tiles | P1 | M6 |
| SL-E07 | Reverse geocoding (place names) | [I] | "Brighton", "Trent Lakes" as place memories | Offline gazetteer the user supplies (tab-separated `name/lat/lon[country]`, 1° cell index, 30 km match); ships no data, downloads nothing | P2 | M6 |
| SL-E08 | Dashboard overview | [S] | Total images, library size, reviewed, reviewed % | Counters maintained by the catalog | P0 | M6 |
| SL-E09 | Review-progress breakdown | [S] | Kept / rejected / skipped with percentages and bar | Counter groups, recomputed on op apply | P0 | M6 |
| SL-E10 | Highlights (first year, busiest month/day, top weekday) | [S] | Four highlight cards | Group-by over capture-date histogram | P1 | M6 |
| SL-E11 | Seasonality chart | [S] | Month × count chart | Fixed-height custom painter chart | P2 | M6 |
| SL-E12 | Memories ("For You") | [S] | On This Day, This Week in past years, Never Looked At; place memories still planned | Deterministic date cards in `sortlens-memories`; UTC reading documented | P1 | M7 |
| SL-E13 | Per-folder / per-collection stats | [I] | Scope the dashboard | Same aggregator with a scope | P2 | M6 |
| SL-E14 | Histogram (RGB/luma) | [I] (ArtCraft parity) | Inspector histogram for the active photo | Downsampled luma/RGB bins from the preview | P2 | M9 |

**Algorithms we will specify (so tests can assert relations, not magic numbers).**

- *Sharpness*: convert a ≤1024 px preview to linear luma, 3×3 Laplacian, variance; normalise by
  the luma variance of the same patch (R = Var(∇²I) / (Var(I) + ε)) so low-contrast subjects
  are not punished. Threshold is a percentile of the current scope plus an absolute floor.
  Store `score: f32` and `version: u8` so an algorithm change can re-score in the background.
- *Similarity*: dHash (9×8 grayscale, horizontal gradient) and pHash (32×32 DCT, top-left 8×8)
  combined; group when both distances are under a user threshold (default tuned on our corpus);
  additionally require aspect ratio within 2% and capture time within a window when deciding
  *burst* vs *duplicate*.
- *Best pick*: lexicographic score of (sharpness percentile, resolution, rating, pick flag,
  smaller file size on tie). Never delete automatically, ever.
- *Memories*: a provider returns `(title, subtitle, photos, kind)`; "On This Day" buckets by
  month-day across years, "Highlights" by 4★+/picks in a rolling window, "Place" by geocoded
  cluster with ≥N photos. Each provider is deterministic given the catalog revision, so the
  UI and tests agree.

---

## F. Local AI suggestions

| ID | Feature | Evidence | Behaviour | Rust approach | Pri | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| SL-F01 | Shared local model store | [D]/[S] | "Installed · 580 MB · Remove" | Content-addressed model dir + sha256 + size + licence | P1 | M8 |
| SL-F02 | On-demand download with consent | [D] | Explicit download, removable, offline afterwards | `model.install` job, resumable, pinned URL + digest | P1 | M8 |
| SL-F03 | Object detection suggestions | [D] | People, animals, vehicles, birds, trees… | YOLO-family ONNX via `tract`, NMS in Rust, label map | P1 | M8 |
| SL-F04 | Embedding / zero-shot scene classification | [D] | Domain vocabulary packs, prompt scoring | CLIP-family text+image towers; precomputed text embeddings | P1 | M8 |
| SL-F05 | EXIF-derived hints | [D] | Camera, lens, GPS context, time of day | Rule table over metadata, no model needed | P1 | M8 |
| SL-F06 | Unified suggestion panel | [D] | All sources merged, per-tag confidence, accept/reject | Merge + dedupe + score; never auto-apply | P1 | M8 |
| SL-F07 | Suggestion packs with counts | [S] | Photography 62, Wildlife & Nature 110, Food & Cuisine 64, Scene, Event | Pack = JSON term list + prompt templates + thresholds | P1 | M8 |
| SL-F08 | Enable/disable packs and detectors | [D] | Toggles in Settings → AI | `craft-flags` + settings; off by default | P1 | M8 |
| SL-F09 | Batch suggest for a selection | [D] | Background job with progress/cancel | Activity entry, bounded queue, low-priority thread pool | P2 | M8 |
| SL-F10 | Privacy guarantee | [D] | No upload, CPU only, no telemetry | No network code in the inference path; audited by a test | P0 | M8 |
| SL-F11 | Face detection / grouping | [I] (LightCraft parity) | Later; requires a second model | Deferred, separate flag | P2 | M10 |

**Design constraints.** Inference must never block a frame: worker threads, bounded queue,
results applied on the UI thread with a revision check. An AI tag is a *suggestion* until the
user accepts it; accepted tags carry `source = Ai` so they can be filtered or cleared later.
Model downloads are the only network operation in the product besides update checks and
optional map tiles, and each is separately disclosable in Settings.

Text embedding cost is why packs exist: at model-install time, embed every prompt in every
enabled pack once and cache the vectors next to the model; per photo only the image tower runs.
That is the difference between a 200 ms and a 2 s per-photo cost.

---

## G. Collections and albums

| ID | Feature | Evidence | Behaviour | Rust approach | Pri | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| SL-G01 | Create / rename / delete collection | [D] | Unlimited named collections | `collection.*` ops | P0 | M5 |
| SL-G02 | Add / remove photos from any folder | [D] | Context menu + lightbox + drag | `collection.add/remove` with id sets | P0 | M5 |
| SL-G03 | Collection favourites and colours | [D] v0.3.9 | Pin and tint, like folders | Same fields as folders | P1 | M5 |
| SL-G04 | Collection search and sort | [D] | Name/count/date | Shared list controls | P1 | M5 |
| SL-G05 | Smart collections from filters | [I] (filter presets) | Live collection defined by a filter | `kind: Smart{filter}` re-evaluated on revision | P2 | M6 |
| SL-G06 | On-disk albums | [D] v0.11 (iPhone) | An album is a real folder that the phone can upload into | `Album{folder_id}`; create/move files | P2 | M9 |
| SL-G07 | Bulk move to folder | [D] | Move selection, confirm, progress, undo | `photo.move {ids,dest}` staged + verified | P0 | M5 |
| SL-G08 | Drag photos onto a collection | [I] (LightCraft parity) | Drop to add | DnD with a payload of ids, drop target registered | P2 | M8 |
| SL-G09 | Collection cover / representative | [?] | First photo, or user-picked | `cover_id` field | P2 | M9 |
| SL-G10 | Export collection as folder | [I] | Copy or convert into a destination | Export job with collision policy | P2 | M9 |

---

## H. Safety, data and recovery

This domain ships **before** any destructive feature. The rule: no code path can delete a file
whose approval flow, undo record and audit entry do not already exist.

| ID | Feature | Evidence | Behaviour | Rust approach | Pri | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| SL-H01 | Delete modes | [D] | OS Trash (default) / rejected folder / permanent | `DeleteMode` in settings; each mode has its own executor | P0 | M7 |
| SL-H02 | Confirmation with impact | [D] | File count, total size, explicit list | Pre-flight dialog built from the same selection as the job | P0 | M4 |
| SL-H03 | Undo stack (≥20) | [D] | Undo across annotations and file ops | Op log inverses; file ops carry restore handles | P0 | M7 |
| SL-H04 | Audit log | [D] | Every destructive op logged to disk | Append-only JSONL next to the catalog, fsync on commit | P0 | M7 |
| SL-H05 | Library data export/import | [D] | JSON backup, moved between machines | Versioned catalog export; import validates then merges | P0 | M7 |
| SL-H06 | Volume remap after migration | [D] | Reattach a moved library | Rewrite root mapping + verify a sample of files | P0 | M7 |
| SL-H07 | Library health panel | [D] | Missing/moved/unreadable, per-folder status, rescan | Health report struct with per-folder probes | P0 | M5 |
| SL-H08 | Remove all missing images | [D] v0.3.2 | Bulk purge of dead entries | Batch op with counts + audit | P1 | M5 |
| SL-H09 | Crash recovery | [I] | Kill mid-write → consistent library | CRC'd append-only ops, atomic snapshot, replay on open | P0 | M3 |
| SL-H10 | Backup / restore | [D] | Explicit backup path and restore | Catalog snapshot + settings archive, verified | P1 | M7 |
| SL-H11 | Compact / integrity check | [I] (LightCraft "Optimize Library") | Reclaim space, verify hashes | Snapshot rewrite + op-log truncation + verify pass | P2 | M8 |
| SL-H12 | Quit guard | [I] | Warn on running jobs / unsaved changes | `craft-quit` prompt from engine state | P1 | M4 |
| SL-H13 | Permission and IO errors surfaced | [D] | Read-only volume, locked file | Every job returns per-item results; UI shows a report | P0 | M4 |
| SL-H14 | Read-only library mode | [?] | Open a library from a network share without writing | `--read-only`; all mutating commands refuse with a reason | P2 | M9 |

---

## I. Settings, appearance, platform

| ID | Feature | Evidence | Behaviour | Rust approach | Pri | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| SL-I01 | Sectioned settings dialog | [S] | Accounts, Review, Appearance, AI & Tagging, Library Health, Library Data, Updates | `craft-ui` sectioned dialog + shell navigation | P0 | M4 |
| SL-I02 | Review settings | [D] | Deletion mode, sidecar handling, prefetch, preview size, scan batch | Typed settings struct, validated on load | P0 | M4 |
| SL-I03 | External editor path | [D] | Open with a chosen app; per-OS | Settings + platform opener, validated on save | P1 | M5 |
| SL-I04 | Themes | [D] | Light / Dark / Dark+ | Four ArtCraft presets: Charcoal, Midnight, Silver, Paper | P0 | M0 |
| SL-I05 | Accent + high contrast | [I] (ArtCraft) | Accent choice, contrast-checked | Token palette + WCAG tests | P1 | M2 |
| SL-I06 | Theme packages | [I] (craft-extensions) | Install a theme from a file/store | `craft-ext-core` palette → tokens, data-only | P2 | M10 |
| SL-I07 | Language + RTL | [I] (ArtCraft) | UI language, Arabic RTL | `craft-i18n` catalogs + `craft-ui::fonts` stack | P1 | M9 |
| SL-I08 | UI scale | [I] | 0.8–2.0 | egui zoom factor, persisted | P1 | M2 |
| SL-I09 | Shortcuts editor | [I] (craft-keymap) | Rebind, conflict detection, import other apps' sets | `craft-keymap` data + editor UI | P1 | M9 |
| SL-I10 | Update check + release notes | [D] | Consent-gated check with release notes headline | `craft-update` parser + semver, engine `update.check`/`update.consent`/`update.status` | P1 | M9 |
| SL-I11 | Credits / attribution | [I] (ArtCraft) | Licences for models, fonts, data | `docs/ATTRIBUTION.md` + in-app credits | P0 | M9 |
| SL-I12 | First-run setup | [D] v0.10 | Choose library, add a folder, optional remote test | Wizard as commands, skippable, resumable | P1 | M5 |
| SL-I13 | Library profiles ("accounts") | [D] v0.10 | Each person has their own library | `profile` dir with its own catalog + settings; no auth server | P1 | M9 |
| SL-I14 | Diagnostics bundle | [I] | Log folder, copy diagnostics | Bounded log files + a zip export | P2 | M9 |
| SL-I15 | Portable mode | [?] | Data beside the executable | `--data-dir` / env override | P2 | M10 |
| SL-I16 | No telemetry / no account | [D] | Zero tracking | Nothing to opt out of; asserted by a dependency audit | P0 | M0 |

---

## J. Profiles and remote access

Sortlens' phone story is: a Mac hosts the library, the iPhone connects over Tailscale, lists
folders/collections, creates albums, uploads photos, and can keep or remove the local copy.

| ID | Feature | Evidence | Behaviour | Rust approach | Pri | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| SL-J01 | Local network server (opt-in) | [D] | Serve the library to a paired device | Server shipped 2026-11-24 (Phase 4 item 3): loopback by default, `--bind lan` explicit, `/health`, `/pair`, `/api`, `/upload`, `/thumb` | P2 | M9 |
| SL-J02 | Phone client | [D] | Browse, cull, albums, upload | The native phone app (`apps/sortlens-mobile`, SL-J10) is the same engine and shell on Android and iOS. Done: the device protocol (pair, `/api`, resumable `/upload`, SL-J05's ack) in `sortlens_remote::client`, proven against the real server in the loopback harness; thumbnails over the wire; `Engine::open_memory`; the app crate with its Android and iOS entry points; the shell's phone-width layout (under 600 points: one full-width dock sheet at a time, a wordless bar, a field that keeps the room the glyphs leave) and its canvas at that width (the far-edge rail stands down while the inspector is closed and is the sheet's tab switcher when open, a two-row filter strip without a second count, a tray whose every control is on screen); settings persistence through `craft-settings`; and Settings → Send to desktop, which pairs with the code the desktop prints, queues the chosen photographs over the resumable upload, names the photograph it is on, stops the run, and offers keep-or-remove per send. Still open: the iOS Photos library and a device build (needs the NDK and Xcode) | P2 | M9 |
| SL-J03 | Pairing + tokens | [D] | Connection test, saved address | Shipped 2026-11-24 (Phase 4 item 3): six-digit code (10 min, 5 wrong attempts lock out) → scoped bearer token in `<library>/remote-tokens.json`; the desktop's Settings → Sync & devices (2026-11-24) spawns the server, shows the live port and code, lists the paired devices from the token file and revokes by exact token once the server is stopped | P2 | M9 |
| SL-J04 | Upload into an album | [D] v0.11 | Phone → album folder on the host | Shipped 2026-11-24 (Phase 4 item 3): `Content-Range` chunks on disk, resume from the received count, size+SHA-256 verified before the final chunk lands, ` (n)` collisions. The app half (2026-11-24): Settings → Send to desktop queues the selection through `sortlens_remote::send::SendQueue`, one worker thread the UI polls for progress and can stop between photographs | P2 | M9 |
| SL-J05 | Keep or remove after upload | [D] | Toggle, including background refresh | Shipped 2026-11-24 (Phase 4 item 3, client half): `UploadItem.remove_after_upload` chosen when the item is queued, applied on the server's ack as `LocalCopy::Keep`/`Remove` — the decision rides on the item, so a transfer that died mid-flight and resumed still lands with the right answer (proved in the loopback harness). The app half (2026-11-24): a send run lists the ids the ack cleared in `SendOutcome::to_remove`, and the phone carries them out through `Engine::delete_photos(..., Trash)` — audited first, so a file whose bytes/size/mtime moved since the scan is refused rather than deleted; Stop leaves earlier confirmations intact and unconfirmed items untouched | P2 | M9 |
| SL-J06 | Remote album/collection create | [D] v0.11 | Create real folders and collections remotely | The `collection.*` commands are on the shipped allow-list under `annotate` (Phase 4 item 3, 2026-11-24); folder-on-disk creation is the album folder uploads land in | P2 | M9 |
| SL-J07 | Connection test | [D] v0.10 | In setup and settings | `GET /health` (no token) plus the auth probes shipped 2026-11-24 (Phase 4 item 3); the phone's Send section runs it as Test connection (2026-11-24) — the typed address and port alone, on its own thread, before any code is spent, saying which address answered or why none did | P2 | M9 |
| SL-J08 | Remote rescan | [I] | Host rescans on demand | `library.rescan` on the shipped allow-list under `annotate` (Phase 4 item 3, 2026-11-24) | P2 | M9 |
| SL-J09 | No cloud, VPN-friendly | [D] | Works over an existing VPN | Loopback default, `--bind lan` explicit, allow-list over the registry — shipped 2026-11-24 (Phase 4 item 3) | P2 | M9 |
| SL-J10 | Phone app | [I] | Install Sortlens on a phone or tablet | `apps/sortlens-mobile`: the same engine and shell as the desktop, native on Android and iOS (see SL-J02) | P2 | M9 |

Note: Sortlens integrates a specific VPN product. We implement "reachable over the network
the user already has" (LAN, or any VPN/overlay they run) and never bundle or configure a
third-party service.

---

## K. Automation (CLI, control channel, MCP)

ArtCraft rule: every panel action must be a registered command, so an agent can do everything
a person can, and every feature must work without the GUI.

| ID | Feature | Evidence | Behaviour | Rust approach | Pri | Phase |
| --- | --- | --- | --- | --- | --- | --- |
| SL-K01 | Command registry | [I] (ArtCraft) | One catalog drives menus, shortcuts, automation | `CommandSpec{id, params, result, menu, scope}` | P0 | M1 |
| SL-K02 | Engine as single entry point | [I] | UI never mutates state directly | `Engine::run(id, params)` with typed errors | P0 | M1 |
| SL-K03 | Headless CLI | [I] | scan, cull-report, dedupe, export, snapshot | `sortlens-cli` with JSON output | P1 | M2 |
| SL-K04 | Control channel | [I] (LightCraft) | Local socket/port for tests and scripts | `craft-ipc` + `craft-jsonrpc` | P1 | M4 |
| SL-K05 | MCP server | [I] | Agents drive the app | `craft-mcp`, read/write root enforcement | P2 | M9 |
| SL-K06 | Stable ids and JSON schemas | [I] | Scripts can rely on output | Versioned result schemas, documented | P1 | M4 |
| SL-K07 | Screenshot + accessibility-tree capture | [I] (ArtCraft) | Visual regression and agent verification | Offscreen view + AccessKit dump | P1 | M2 |
| SL-K08 | Safety roots | [I] | Automation cannot touch files outside allowed roots | Validated paths on every file command | P0 | M9 |

---

## L. Accessibility and internationalisation

ArtCraft standards make these release-blocking, not polish.

| ID | Feature | Requirement | Pri | Phase |
| --- | --- | --- | --- | --- |
| SL-L01 | Keyboard-only operation | Every command reachable; no traps; focus follows the visible control | P0 | continuous |
| SL-L02 | Screen-reader roles | Grid cells, thumbnails, cull buttons, sliders expose role/name/state | P0 | continuous |
| SL-L03 | Announcements | Decisions and selection changes announced through a live region | P1 | M6 |
| SL-L04 | Contrast | Text 4.5:1, controls and selection rings 3:1, in every theme | P0 | M2 |
| SL-L05 | Colour is not the only signal | Flags/labels carry icon or text, not just hue | P0 | M3 |
| SL-L06 | Reduced motion | Swipe/zoom animations honour the preference | P1 | M6 |
| SL-L07 | Translations | English keys, TSV catalogs, CJK/Arabic/Thai/Vietnamese coverage. Scaffolding shipped 2026-11-24 (Phase 4 item 2d): ar/ja catalogues, the language picker, and the RTL direction plumbing; the Welcome card, the indexing screen, the empty-filter screen, the export dialog and receipt, the three delete confirmations, rename, About, Library health, the cull summary, the quit guard, the change viewer and the Batch applied receipt are catalogued (2026-11-26), the remaining four languages, Suggested keywords, Keyboard shortcuts, the engine-composed job and support sentences, and RTL screenshots are open | P1 | M9 |
| SL-L08 | Localised numbers and dates | Region digits, decimal mark, date order. Shipped 2026-11-25 (Phase 4 item 2e): the shell's counts, sizes and dates follow the interface language — Arabic-Indic digits and day-first dates in Arabic, comma grouping and slash dates in Japanese, English unchanged | P2 | M9 |
| SL-L09 | High-contrast theme | A fifth preset with maximum separation | P2 | M9 |

---

## M. Performance and scale

| ID | Budget | Target | How it is checked |
| --- | --- | --- | --- |
| SL-M01 | Interactive with 100k photos | < 1.5 s cold start from snapshot | benchmark on synthetic library |
| SL-M02 | Grid scroll | 60 fps at 100k, 384 px thumbs | frame-time test in CI |
| SL-M03 | Thumbnail cold generation | ≥ 200 photos/s on 8 cores | bench |
| SL-M04 | Cull key → next photo | < 100 ms when prefetched | instrumented test |
| SL-M05 | No UI frame over budget | p99 < 8 ms UI thread work | frame trace in debug overlay |
| SL-M06 | Memory with 50k library | < 1.5 GB RSS, preview cache bounded | allocation counter test |
| SL-M07 | Scan 10k files | < 30 s metadata-only on SSD | bench |
| SL-M08 | Similarity over 100k | hashing < 2 s incremental; grouping < 500 ms | bench |
| SL-M09 | Apply 1,000 rejects | < 5 s to trash, progress visible, cancellable | integration test |
| SL-M10 | Startup never blocks on IO | library opens, then previews arrive | manual + trace |
| SL-M11 | Graceful on slow/removable media | no freeze, timeouts, offline state | fault-injection test |

---

## Behaviours we deliberately do **not** recreate

| Sortlens mechanism | Why not | What we do instead |
| --- | --- | --- |
| Python backend + WebView + local HTTP port | Not needed in-process; source of port discovery and venv-repair bugs | Native Rust engine in the same process |
| `.port` file and dynamic port selection | Same | Direct function calls |
| Inno Setup + PowerShell self-updater | Fragile, platform-specific | `craft-update` with signed artifacts |
| React/Tailwind blue-dark chrome | ArtCraft identity required | egui + ArtCraft tokens, docking, menus |
| Vendored Python AI runtime | Huge, opaque | `tract`/candle with pinned model digests |
| iOS native app | In scope | `apps/sortlens-mobile` (SL-J10): the same Rust engine and UI |
| Vendor screenshots and icons | Clean-room rule | Original icon set and our own screenshots |

## Open questions to resolve before M4

1. **RAW tone rendering in cull.** Do we render a developed preview (slow, pretty) or use the
   embedded JPEG (fast, camera look)? Proposal: embedded preview by default, "develop" render
   on demand — it matches culling needs and keeps RAW decodes off the hot path.
2. **Video playback.** Poster frames are cheap; playback needs a decoder + audio. Proposal:
   poster + metadata in v1, decode-and-present playback behind a flag, system player as fallback.
3. **Sidecar default.** On for rating/label/tags, off for cull decisions (proposal).
4. **Deletion default.** OS trash (matches vendor default), with "rejected folder" as the
   selected alternative for people culling on external media.
5. **Map data.** Offline vector world by default (privacy), optional user-supplied MBTiles,
   optional online tiles behind an explicit, separately-flagged opt-in.
6. **AI backend licence.** `tract` (MIT/Apache, pure Rust) first; allow an ONNX Runtime feature
   only if a benchmark proves it necessary, documented as a departure from "pure Rust".
