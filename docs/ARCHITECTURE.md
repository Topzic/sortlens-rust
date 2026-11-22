# Sortlens (Rust) — Architecture

Goal: the same feature set as Sortlens, rebuilt as a native, pure-Rust, ArtCraft-style
application, with a clean-room behaviour spec as the contract and `craft-libs` as the
shared foundation.

This document is the build map. It is intentionally opinionated; every choice names the
alternative it beat and the reason, so a later reader can revisit it.

## 1. Principles

1. **One process, no sidecar.** No Python, no localhost HTTP between UI and engine, no
   port discovery file. The UI calls the engine in-process.
2. **UI is a thin projection.** Panels change state only through `Engine::run(command, params)`.
   Consequence: CLI, control channel and MCP get every feature for free, and the UI cannot
   invent a mutation path that the automation layer cannot replay.
3. **Layered, acyclic, egui only at the top.** Nothing below L5 depends on egui, eframe, winit
   or rfd. Enforced by `cargo xtask layers`.
4. **Never lose data.** Append-only op log, atomic snapshots, staged deletes, undo, audit.
   Every destructive command has a pre-flight, a record and a reversal.
5. **Never block the frame.** Decode, hash, write, infer and scan happen on workers; the UI
   thread only takes completed results and applies them under a revision check.
6. **Budgeted performance.** Numbers in `FEATURES.md` §M become benchmarks in CI; a regression
   fails the build, not the review.
7. **Hostile input everywhere.** Libraries, sidecars, model files and import archives are
   untrusted. Bounded reads, no panics, no `unsafe`.
8. **ArtCraft identity.** Tokens, fonts, docking, menus, dialogs, keymap, settings and
   update all come from the suite crates, so Sortlens looks and behaves like a sibling app.

## 2. Layers and crates

```
L5  sortlens-ui (egui)   sortlens-mcp   apps: sortlens (desktop), -cli, -web, -server
L4  sortlens-engine      commands, session, activity, jobs, filters, export, filesystem ops
L3  sortlens-catalog     op log + snapshot + counters + undo/audit
    sortlens-cull        session state machine
    sortlens-insights    dashboard + memories
    sortlens-similar     hashing + grouping
    sortlens-remote      optional server API (feature-gated)
L2  sortlens-thumbs      tiered preview cache + job scheduler
    sortlens-scan        walker, identity, delta, offline probes
L1  sortlens-meta        craft-metadata → neutral PhotoMeta
    sortlens-media       format routing, embedded previews, video posters
    sortlens-sidecar     XMP read/merge/write
    sortlens-geo         gazetteer, clustering, projection
    sortlens-ai          manifest, digest-verified store, NMS + label map, EXIF hints, suggestion merge
L0  sortlens-core        ids, time, units, bytes, versioned errors
    sortlens-hash        dHash/pHash, Hamming, LSH
    sortlens-quality     sharpness, exposure, clipping metrics
    sortlens-export      export planning, collision policy, copy and re-encode
    sortlens-testkit     synthetic libraries, fixtures, assertions
```

Dependency rule: a crate may depend on strictly lower layers plus `craft-*` crates.
`sortlens-engine` is the only crate that mutates the catalog; `sortlens-ui` is the only one
that depends on egui.

### Why these crates, not fewer

- `hash` and `quality` are pure math with no IO: they must be unit-testable with property
  tests and reusable by the CLI without dragging in the catalog.
- `sidecar` is separated because XMP merge semantics (keep unknown fields, detect external
  edits, atomic replace) are the most subtle correctness surface in the product.
- `cull` is separated because the state machine has its own transition table and its own
  tests, and because it must be runnable headlessly (an agent can cull).
- `insights` is separated because it is a read-only projection; it must never be able to
  mutate the library, which the type system can then help enforce.

### Estimated size at v1.0

| Crate | LOC (est.) | Notes |
| --- | ---: | --- |
| core, hash, quality | 2,000 | small, dense tests |
| meta, media, sidecar, geo | 6,000 | mostly mapping + bounds |
| scan, thumbs | 4,500 | scheduler and cache dominate |
| catalog | 7,000 | ops, snapshot, migrations, counters |
| cull, similar, insights | 3,500 | algorithms + state machines |
| ai | 3,000 | model store, tract glue, NMS, packs |
| engine | 5,000 | ~150 commands, jobs, safety rails |
| ui | 18,000 | the largest surface, by nature |
| cli, mcp, server, web | 3,000 | thin adapters |
| **Total** | **~52,000** | excludes generated tables and `xtask` |

For reference, LightCraft's UI crate alone is larger than this entire estimate; 52k lines is
a realistic target for a focused culling tool, not a photo editor.

## 3. Persistence

### 3.1 Library layout

```
<library>/
  library.toml            # id, schema, created, app version (human-readable, small)
  catalog/
    snapshot-000042.bin   # serialised state at op 42 (CRC + schema + app version)
    ops-000043.jsonl      # append-only operations, one JSON object per line, CRC per line
  previews/
    <xx>/<shard>/<hash>-<size>.jpg
  models/
    <sha256>/…            # downloaded model files + manifest
  sidecars/               # optional: our own metadata cache when XMP is off
  logs/
  audit.jsonl             # destructive operations only, append-only, never compacted
```

A "library" is a directory the user chooses. The default lives under the platform data dir
(`~/Library/Application Support/Sortlens` on macOS, `%LOCALAPPDATA%\Sortlens` on Windows,
`$XDG_DATA_HOME/sortlens` on Linux) via `craft-settings` folder rules. `--data-dir` overrides
it (portable mode).

### 3.2 Why an op log instead of a relational database

Sortlens uses SQLite (`~/.sortlens/sortlens.db`). That is a good choice for a PHP/Python
backend issuing ad-hoc queries; it is a poor fit here:

| Need | SQLite | Op log + snapshot |
| --- | --- | --- |
| Crash safety | WAL, well understood | ATP: append + CRC, replay |
| Undo (20 steps) | Reverse-engineer each write | Inverse op stored at write time |
| Audit log | Second table, can drift | The log *is* the audit trail |
| Portability between machines | Copy file, version-locked | Export/import is just the log + snapshot |
| Concurrency | One writer, locking | One writer thread, no locks |
| 100k-photo scans | Fine | Fine (batched ops, snapshot compaction) |
| Ad-hoc queries | Native | Precomputed indices + in-memory views |

The last row is the honest trade-off: we lose SQL. We gain determinism, replayable tests,
free undo, and an audit trail that cannot disagree with the data. Filters over 100k photos
are evaluated over in-memory columnar structures (see §3.4), which is faster than the query
round-trip the vendor's UI pays per keystroke.

If a future feature genuinely needs relational queries, add a read-only derived index
(rebuildable from the log) rather than making it the source of truth.

### 3.3 Operation format

```jsonc
{"seq":42,"ts":"2026-11-21T14:03:11.220Z","actor":"ui","schema":1,
 "op":"photo.rate","ids":[101,102],"rating":4,"rev":17,"crc":3735928559}
```

- `seq` strictly increasing; a gap or CRC mismatch stops replay at the last good line and
  reports the damaged tail (never silently truncates the user's data).
- `actor` ∈ `ui | cli | agent | remote | migration`.
- `rev` is the catalog revision the command was computed against. A command whose `rev` is
  older than the current revision for a *conflicting* field is rejected with
  `Error::StaleRevision` rather than applied — this is the structural fix for the vendor's
  tag-race bugs.
- Batching: one op per user intent (e.g. "tag 300 photos") rather than 300 ops.
- Snapshots as built: `snapshot.json` is rewritten every `SNAPSHOT_EVERY = 256` ops and **the log is
  kept**. A snapshot only makes opening fast; the log is what makes the snapshot recoverable, so a
  snapshot that is later damaged or half-written costs nothing — replay rebuilds the library and the
  audit says so. `compact()` is the only thing that empties the log, and it reads the snapshot back
  and checks its sequence first: truncating a log whose snapshot did not survive is how a library
  disappears. The planned `snapshot-N.bin` numbering is not needed while there is one snapshot and a
  log beside it.
- A record that fails its checksum, or that will not apply (an op left with nothing to act on by a
  damaged snapshot), stops the replay there and is reported. Recovering everything before it beats
  refusing to open the library.

Snapshot is versioned (`schema`, `app_version`), CRC'd, written to `snapshot.tmp` then
renamed. A snapshot that fails to load is moved aside (`snapshot.broken-<ts>.bin`) and the
op log is replayed from the previous good snapshot.

### 3.4 In-memory model

```rust
pub struct Catalog {
    photos: SlotMap<PhotoId, Photo>,
    folders: SlotMap<FolderId, Folder>,
    tags: SlotMap<TagId, Tag>,
    collections: SlotMap<CollectionId, Collection>,
    by_path: HashMap<PathKey, PhotoId>,      // (volume, rel_path) → photo
    by_content: HashMap<ContentKey, PhotoId>,// (size, mtime, head hash) → photo
    by_tag: HashMap<TagId, RoaringBitmap>,   // roaring bitmaps, cheap set algebra
    ratings: Vec<u8>,                        // index-aligned columns for scans
    flags: Vec<Flag>,
    labels: Vec<Option<ColorLabel>>,
    capture: Vec<Option<i64>>,
    sharpness: Vec<f32>,
    rev: u64,
}
```

Columnar side-vectors make filter evaluation a tight loop over 100k elements (<1 ms on a
modern CPU) while `SlotMap` keeps stable ids and O(1) lookup. Tag sets use roaring bitmaps,
so `ANY`/`ALL` are `union`/`intersection` instead of row scans. This is why the filter bar can
respond on every keystroke without a query layer.

`roaring` is a small, permissively licensed dependency; if we want zero non-craft deps it can
be replaced by sorted `Vec<u32>` + galloping intersection, which is fast enough below ~50k
photos. Decision recorded, default: use roaring.

### 3.5 Migrations

`schema` in `library.toml` + every op. Migration is a function
`fn(from: u32, snapshot: &mut Value) -> Result<(), MigrateError>` chain, run on open, with a
backup copy of the library written first (`catalog/backup-pre-vN/`). Migrations are never
destructive-only: a downgrade must at least open read-only.

## 4. The command registry

Every user-visible action is a command. This is the spine of the app.

```rust
pub struct CommandSpec {
    pub id: &'static str,              // "photo.rate"
    pub params: Schema,                // JSON schema, used by CLI/MCP help + validation
    pub result: Schema,
    pub menu: &'static str,            // "Photo > Rate" ("" = not in a menu)
    pub default_key: Option<&'static str>,
    pub scope: Scope,                  // Global | Library | Cull | Detail
    pub mutates: bool,
}
```

- The UI calls `engine.run(id, params)`; it never calls catalog methods directly.
- The keymap, menu bar, command palette, MCP tool list and CLI subcommands are all generated
  from this catalog, so they cannot drift apart.
- `params` is validated before dispatch (`serde` + schema), so a malformed agent call returns
  a typed error instead of a partial mutation.
- UI-only commands (`view.*`, `panel.*`) live in the UI crate but are still registered, so
  "switch to cull view, filter to 4★, screenshot" is one script.

### 4.1 Command families (about 150 at v1.0)

| Family | Examples |
| --- | --- |
| `library.*` | `addFolder`, `removeFolder`, `rescan`, `source`, `sort`, `filter`, `clearFilter`, `health`, `compact`, `backup`, `restore`, `remap` |
| `photo.*` | `rate`, `flag`, `label`, `tag`, `tagFromPrev`, `delete`, `move`, `reveal`, `open`, `revealInFolders` |
| `selection.*` | `set`, `add`, `remove`, `toggle`, `all`, `none`, `range` |
| `cull.*` | `start`, `keep`, `reject`, `skip`, `undo`, `favorite`, `zoom`, `apply`, `session` |
| `analysis.*` | `blur.run`, `blur.apply`, `similar.run`, `similar.findTo`, `ai.suggest`, `ai.accept` |
| `collection.*` | `create`, `rename`, `delete`, `add`, `remove`, `favorite`, `color`, `cover` |
| `tag.*` | `create`, `rename`, `merge`, `delete`, `color` |
| `insights.*` | `stats`, `memories`, `refresh` |
| `view.*` | `library`, `cull`, `detail`, `compare`, `places`, `similar`, `softFocus`, `density`, `theme`, `filterBar` |
| `panel.*` | `details`, `tags`, `suggestions`, `activity`, `close` |
| `app.*` | `settings`, `shortcuts`, `export`, `quit`, `about`, `updates`, `diagnostics` |

Result envelopes always include `{"rev": n}` so callers can detect staleness.

## 5. Threading and jobs

```
UI thread ──run(cmd)──▶ Engine ──┬─ Catalog (single writer, short critical sections)
                                 ├─ JobQueue ──▶ IO pool (scan, delete, move, sidecar)
                                 ├─ JobQueue ──▶ decode pool (thumbs, lightbox, posters)
                                 ├─ JobQueue ──▶ compute pool (hash, sharpness, AI)
                                 └─ Activity registry (progress, cancel, errors)
        ◀──results + rev─── (frame boundary, applied under revision check)
```

Rules:

- **Write commands complete synchronously** when they are pure catalog mutations (rate, tag,
  decide). They append an op, update memory, bump `rev`, return. That keeps undo and tests
  simple.
- **Long work is a job**: scan, thumbnail batch, trash 1,000 files, dedupe index, AI batch,
  export, model download. Jobs are visible in the activity stack, cancellable, and report
  per-item results.
- **Cancellation is cooperative and generation-based**: each job carries a `JobId` and an
  `AtomicBool`; results from a cancelled or superseded job are dropped by comparing the job's
  generation with the current one. No channel is closed to stop work.
- **Backpressure** is a bounded queue per pool (e.g. 2× threads). The grid requests only what
  is visible + one screen of lookahead, so a fast scroll cannot enqueue 100k decodes.
- **Priority**: visible viewport > cull lookahead > lightbox neighbours > background analysis.
  Priority is a value in the job, not a separate queue per priority (avoids starvation).

### 5.1 Preview cache

| Tier | Long edge | Use |
| --- | --- | --- |
| micro | 128 px | cull strip, map cluster thumbs, sidebar |
| grid | 384 px (or `2×` density) | browse grid |
| loupe | 1024 px | detail view at fit |
| full | screen-sized | 1:1 zoom, EXIF-free inspection |

- Decode at the *target* size, never full-res-then-downscale (except when the decoder cannot
  scale: then decode once and resize, but still cap by `max_texture_side`).
- Disk cache keyed by `(content_key, tier, algorithm_version)`; sharded two levels; LRU
  eviction by atime with a byte budget (default 4 GB, configurable).
- Memory cache is an LRU of textures bounded by a byte budget (default 512 MB) and by
  `max_texture_side`; oversized images are tiled rather than uploaded whole.
- A cache miss shows the previous tier (blur-up) instead of a spinner. Spinners are a last
  resort.
- Format: JPEG (fast, universal) via `jpeg-encoder`, with WebP as an optional alternate if
  `craft-webp` proves its encoder. Never PNG for photos.

### 5.2 Scan algorithm

1. Enumerate roots, probe volumes (serial, label, mount state).
2. Walk with a bounded queue; skip known non-media directories (`.git`, `node_modules`,
   package caches) and hidden files unless configured otherwise.
3. For each file: cheap sniff by extension + magic; stat; look up `by_path`.
4. If present and `(size, mtime)` unchanged → skip entirely (the common case on rescan).
5. If present and changed → re-read metadata, invalidate derived data (thumbs, hash, scores).
6. If absent → create a `Pending` photo (id allocated, no preview yet) and enqueue metadata
   read; thumbnails follow at low priority so the grid fills fast.
7. Commit in batches of ~500 ops, one snapshot-worthy transaction per batch.
8. After the walk: mark photos under the root that were not seen as `missing` (not deleted),
   and report the count in the activity row.

Time budget: metadata-only walk of 10k files on an SSD under 30 s; thumbnail generation is
decoupled so the grid becomes usable in seconds.

## 6. Data model

```rust
/// Stable across renames, moves and remounts within a volume.
pub struct VolumeKey { serial: u64, label: String, kind: VolumeKind }
pub struct PathKey { volume: VolumeKey, rel: String }        // rel is '/' normalized

pub struct Photo {
    pub id: PhotoId,
    pub path: PathKey,
    pub content: ContentKey,        // size + mtime + head hash (cheap identity)
    pub kind: MediaKind,            // Image | Raw | Video
    pub ext: Ext,
    pub dims: Option<Dim>,
    pub orientation: u8,
    pub capture: Option<CaptureTime>, // EXIF original, else file mtime, with source flag
    pub camera: Option<Camera>,       // make/model/serial
    pub lens: Option<String>,
    pub exposure: Option<Exposure>,   // iso, shutter, aperture, focal, ev, flash
    pub gps: Option<Gps>,
    pub place: Option<Place>,         // cached reverse geocode
    pub video: Option<VideoInfo>,     // duration, fps, codec, audio
    pub rating: u8,                   // 0..=5
    pub flag: Flag,                   // None | Pick | Reject
    pub label: Option<ColorLabel>,
    pub deleted: Option<Deleted>,     // at, mode, restore handle
    pub preview: PreviewState,        // Missing | Micro | Grid | Full | Failed(reason)
    pub analysis: AnalysisState,      // blur score, hashes, ai, each with version
}
```

Annotations are explicit fields, not a bag, so batch ops and counters are O(n) and typed.
Tags are the one many-to-many relation, held in the tag index (§3.4).

`Deleted { at, mode, original: PathKey, restore: Option<PathBuf> }` powers the rejects bin,
undo and the audit log from one record.

## 7. Safety rails for file operations

```rust
pub enum DeleteMode { Trash, RejectedFolder(PathBuf), Permanent }
```

Every mutating file command passes through one executor with these steps:

1. **Resolve** each id to a live path; drop entries that are already missing (reported).
2. **Preflight**: count, total bytes, free space for move/copy, write permission, collision
   check; produce a `Plan` the user sees.
3. **Record**: append the intent to the audit log before touching anything
   (`intent`, then per-item `done`/`failed`).
4. **Execute** with per-item results; never abort the batch on one failure.
5. **Verify**: for trash, confirm the file is gone; for move, confirm destination exists with
   the same size + hash; for rejected-folder mode, never overwrite.
6. **Update** the catalog in one op, with the inverse op built from the plan (so undo can
   restore from trash or move back).
7. **Report**: toast with counts, details in the activity row, failures copyable.

Hard prohibitions, checked in code and in tests: never delete a directory, never follow a
symlink out of a library root, never operate on a path that failed canonicalisation, never
delete when the preflight plan and the executed count disagrees.

## 8. UI architecture

### 8.1 Shell

```
┌─ top bar (integrated titlebar on macOS via craft-window) ───────────────────────────┐
│ mark · library name │ ← → │      search        │ filter▾ │ import export bell help ⚙│
├───────────┬─────────────────────────────────────────────────────┬───────────────────┤
│ left      │ contextual header: title · count · sort · density   │ right inspector   │
│ sidebar   ├─────────────────────────────────────────────────────┤ (docked, tabs)    │
│ sources   │ filter bar (collapsible, badge count)               │ Details           │
│ folders   │                                                     │ Tags              │
│ collections│              view canvas                          │ Suggestions       │
│           │      library · cull · detail · compare ·            │ Activity          │
│           │      places · similar · soft focus · insights       │                   │
├───────────┴─────────────────────────────────────────────────────┴───────────────────┤
│ bottom bar: view switcher · ★/flag pill · selection · thumb size · zoom · ⌨ help    │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

- Docking: `craft_ui::docking` for left/right panels with a protected canvas node (the photo
  viewer cannot be tabbed into), saved workspace per library, reset command.
- Right side also has an iconic tool strip (ArtCraft pattern) to swap inspectors without
  losing the dock position.
- Status: bottom-centre toast HUD for confirmations; activity bell opens the job list with
  progress bars, cancel, and error detail.
- Empty states are first-class: every view has a title, one sentence, and 1–2 primary buttons
  that run real commands.
- Destructive dialogs are in-app (themeable, testable, screenshot-able), not native message
  boxes. File pickers are native via `craft-dialogs`.

### 8.2 Theme tokens

One `Tokens` struct (colours + metrics) with four presets — **Charcoal**, **Midnight**,
**Silver**, **Paper** — plus accent and high-contrast variants. Mapping to egui via
`craft_ui::theme::WidgetPalette` (the shared five-state operation), with app-specific
exceptions applied afterwards. Label colours are theme functions (`label_color`), with
contrast tests: text 4.5:1, control/selection 3:1, in every preset.

Metrics: top bar 42, bottom bar 48, panel 270, strip 48, filmstrip 142, row 44, thumb 90–480.
These are numbers we can tune, but they live in one place so a "compact" mode is a second
token set, not a sweep through the UI.

### 8.3 Fonts

`craft-fonts` + `craft_fonts_build` with `CRAFT_FONTS_DIR`, exactly like LightCraft:

1. app face (Inter Regular/SemiBold, mono) first;
2. craft-fonts faces for the UI language's script;
3. craft-fonts faces for every shipped script;
4. installed system fonts (optional feature);
5. emoji/symbol faces.

Implemented as `crates/ui/build.rs` + `crates/ui/src/fonts.rs`:

- `build.rs` reads `CRAFT_FONTS_DIR` (unset is a supported configuration, and what CI uses) and writes
  a `CRAFT_FONTS: &[CraftFont]` table. A wasm build keeps an explicit five-face allowlist.
- `fonts::install(ctx)` registers them and **appends** each face to `Proportional` and `Monospace`,
  after egui's own, so a fallback can never take over Latin text.
- `fonts::coverage()` checks the scripts the interface itself draws (`Latn`, `Zsym`); the translated
  scripts are listed separately, ready for when translations land.
- `cargo xtask fonts` reports the face and family count from the checkout and refuses a manifest with
  no Latin face. `fonts/embedded.txt` records what the last release shipped.
- The web build embeds faces rather than serving them for now; `Embed::serve` stays the plan if the
  wasm bundle grows past its budget.

### 8.4 Keymap and menus

`craft-keymap` owns chords, scopes, conflicts, overrides and the editor. Our contribution is
the catalog: ids, labels, menu paths, defaults. Defaults for culling:

| Action | Key |
| --- | --- |
| Keep / Reject / Skip | `→` `K` / `←` `X` / `↓` `S` |
| Undo decision | `⌫` `U` |
| Favourite | `F` |
| Zoom 1:1, 2:1, 3:1, 4:1 | `1` `2` `3` `4` |
| Stars 0–5 | `0`–`5` |
| Labels | `6`–`9` |
| Pick / Unflag / Reject | `P` / `U` / `X` |
| Lightbox / Detail | `Enter` |
| Reveal / Edit | `R` / `E` |
| Filters | `\` |
| Shortcut overlay | `?` |
| Command palette | `⌘K` / `Ctrl+K` |
| Full screen / Lights out | `F11` / `L` |

Menus: native macOS menu bar via `craft-menu`; in-window menubar on Windows/Linux (same model,
so shortcuts and enablement agree).

## 9. AI subsystem

```
model store (sha256, size, licence, packs)   ← explicit download, no auto-fetch
      │
      ├── object detector (YOLO-family, tract)  → labels + boxes + confidence
      ├── image embedder (CLIP-family, tract)   → 512-d vector
      ├── text embedder (same model)            → cached per pack prompt at install
      └── EXIF rules                            → hints (no model)
                    │
             suggestion merger (score, dedupe, cap)
                    │
             Suggestions panel → accept → photo.tag {source: Ai}
```

- Everything runs on the compute pool at low priority; a photo's suggestion is computed from
  its *grid* preview (384 px is enough), not the full image.
- Packs are data: `{id, name, description, terms: [{label, prompts: [...]}], threshold}`.
  Adding a pack is a JSON file, not a code change.
- Accepting a suggestion is the only way an AI tag enters the catalog, so the library never
  fills with machine guesses the user did not ask for.
- `sortlens-ai` has no network code, and `crates/ai/src/no_network.rs` checks that rather than
  trusting review: a source scan for socket APIs and a dependency scan for HTTP clients, both proved
  to fail on a planted hit. The plan is that downloads live in `sortlens-engine`'s `model.install`
  job — the single audited place that fetches bytes — and verify the digest before the file becomes
  visible to the inference path. Neither the job nor the downloader exists yet.
- As built, the arithmetic: `detect::Box` in **normalised** coordinates (so a 320 px and a 640 px copy
  of the same photo produce the same boxes), `Box::iou`, and `suppress` — non-maximum suppression
  **per class**, because a person standing in front of a car overlaps it almost completely and
  suppressing across classes would delete one of them. Boxes below `MIN_SCORE`, with a NaN score or
  with no area are dropped before comparison: a broken detector emitting a thousand empty boxes must
  not erase the one real one. Ties are broken by class, then position, so the same input in any order
  produces the same output.
- `detect::LabelMap` parses `index name` or `index: name` with `#` comments and **refuses a hole** in the
  indices: a map with a hole mislabels everything after it, and guessing is worse than refusing.
- `hints::hints` is the half that needs no model at all: the camera and lens names become keywords,
  the focal length becomes one of three bands (`wide` ≤ 35 mm, `normal` ≤ 70 mm, `telephoto` above),
  `low light` at ISO ≥ 3200 and `long exposure` at 1/15 s or slower. Each rule is a named constant with
  a stated reason, a nonsense value (negative or NaN) produces nothing rather than a wrong hint, a lens
  name that only repeats the camera is not suggested twice, and the output order is fixed so a scan
  order cannot reshuffle the panel. These are proposals like any other: the panel merges them, and only
  accepting one writes a keyword.
- `Engine::suggestions` maps a `Photo` to `ai::Hints`, merges, and returns proposals; `suggest.list` and
  `suggest.accept` expose it, and the CLI has `suggest <id>` / `accept <id> <tag>`. Accepting goes
  through `Engine::set_tag`, so a suggestion becomes an ordinary `Tagged` op — in the log, in the
  history, and undone by the same `undo` as a keyword typed by hand. The detail view draws the list as a
  "Suggested" row whose buttons name their score and source.
- `suggest::merge` is where a guess becomes a proposal: a keyword the photo already has is never
  suggested again, two sources proposing the same tag become one at the higher score, a camera hint
  outranks a guess on a tie, tags shorter than three characters are dropped, and the list is capped at
  `DEFAULT_PER_PHOTO = 8` and ordered by confidence then name. Nothing here writes to the catalog —
  accepting a suggestion is the only path, and that half is not built.
- As built: `manifest::Manifest` (id, kind, licence, SHA-256, size, optional input edge) parsed from
  JSON with a newer schema refused and a missing digest or licence treated as an error;
  `store::Store` names each file after its digest, verifies size then digest before anything is
  written, writes through `.incoming` and re-verifies the bytes that landed before renaming; a file
  that fails verification is reported and **left alone**, because it may be someone's download in
  progress, and `remove()` refuses to delete what it cannot identify. The shipped manifest is empty:
  no model has been chosen, pinned and licensed for redistribution, so there is nothing to describe.

## 10. Remote and web

- `apps/sortlens-web`: the same `sortlens-ui` compiled to wasm32, minus native file access.
- `apps/sortlens-server` (feature `remote`): serves the wasm bundle plus a token-authenticated
  JSON API (`craft-web-host`, `craft-web-api`). Binds loopback/LAN only; pairing code →
  device token; per-token permissions (`read`, `upload`, `annotate`, `cull`).
- The remote API is the command registry with a restricted allow-list, not a second API. A
  phone culling session is the same `cull.*` commands with a remote actor and a device id in
  the audit log.
- Uploads are resumable, verified by size + hash, and land in a configured album folder.
- No cloud service, no account server, no telemetry. If a user wants to reach their desktop
  from outside, they use whatever VPN/overlay they already run; we document the requirement
  and never configure a third-party product.

## 11. Errors, logging, diagnostics

- One error type per crate (`thiserror`), with a `kind` that survives into the UI:
  `NotOnline`, `Permission`, `NotFound`, `StaleRevision`, `ModelMissing`, `Unsupported`,
  `Cancelled`, `Io`.
- User-facing text is a translation key + parameters; the raw error goes to the log.
- Logs: bounded rolling files (5 × 2 MiB) under `logs/`, level from settings; no file paths
  hashed away (they are the user's own data and they need them for support), but no EXIF
  dumps.
- Diagnostics bundle: version, OS, GPU, library schema, counts, last 200 log lines, settings
  with secrets (remote tokens) redacted.
- A panic hook writes a crash file and shows a themed dialog; non-test code is still
  `unwrap`-free by lint.

## 12. Testing and CI

| Layer | Test type | Example assertion |
| --- | --- | --- |
| core/hash/quality | unit + proptest | Hamming is symmetric; a rotated image keeps dHash distance under threshold |
| catalog | proptest + model tests | apply(op) then undo(op) restores the exact prior state and `rev` |
| catalog | crash tests | truncated last line replays to the previous good op |
| sidecar | golden + hostile | unknown XMP fields survive a write; a 10 MB packet is rejected |
| scan | fixture tree | rename detection: one move produces zero new photos |
| thumbs | perf + correctness | tier sizes monotone; no upscale; cache hit returns identical bytes |
| cull | state machine | reject never deletes; skip preserves flags; resume restores cursor |
| similar | corpus | known duplicate pairs grouped; unrelated pairs not |
| ai | model-gated | suggestion set for a fixture photo contains an expected label |
| ui | egui_kittest snapshots | layout in ar/ja/zh-TW/ko/th/vi; focus order; contrast |
| e2e | control channel | script a full cull session and assert the catalog + audit log |
| perf | benches | budgets from `FEATURES.md` §M |

Fixture strategy: a synthetic library generator (`sortlens-testkit`) writes JPEG/TIFF/PNG
files with controlled EXIF, GPS, blur, near-duplicate transforms and known capture dates.
What exists today: `Fixture` writes solid-colour JPEGs through the real encoder (so a fixture
photo decodes, hashes and has a resolution), sidecars under the `docs/BEHAVIOUR.md` §4 naming
rule, and removes its folder on drop. The same bytes every run is the property that makes the
golden job below possible.

**Golden outputs** (`apps/sortlens-cli/tests/golden`, run by `cargo xtask snapshot`): 30 recorded
invocations of the real binary against a fixture library — `add`, `stats`, `list`, `keywords`,
`folders`, `insights`, `page`, `registry`, `memories`, a move and its undo, `audit`, `compact`, and
the `snapshot.json` of the catalog itself. Only three things are normalised (the fixture path, the
scan's elapsed time, a file's mtime), so a renamed JSON field or a reworded summary is a diff. The
job is read-only in CI: `--update` exists for a person making an intended change, and the diff is
the review. Both directions are checked, so a golden whose step was deleted fails rather than
passing forever.
That covers 95% of behaviour without shipping anyone's photos, and it makes performance
tests deterministic. Public-domain photographs (NASA, Library of Congress) are used only for
visual checks, with provenance in `docs/ATTRIBUTION.md`.

CI (`xtask`, mirroring LightCraft):
`cargo xtask ci --quick` (fmt, clippy -D warnings, changed-package tests, layers, roots,
assets), `cargo xtask ci` (full + benches smoke + wasm check), plus a Windows leg and a
snapshot-update job. Sonar or `cargo-mutants` optional for the catalog crate.

## 13. Distribution

- Desktop: `eframe` + `wgpu` (Metal/Vulkan/DX12), signed and notarised macOS app bundle,
  Windows MSI/Inno-free installer (WiX or `cargo-bundle`), Linux AppImage + Flatpak.
- Updates: `craft-update` with the suite app descriptor, release notes rendered in-app.
- Web: wasm bundle served by the optional server; no CDN dependencies.
- Icons: our own SVG set (monochrome, `craft_ui::icons::SvgIconSet`), one brand icon in
  `assets/`, macOS `.icns` / Windows `.ico` generated by `xtask`.
- Manifest: `ai.storyteller`-style reverse-DNS id per ArtCraft convention, app name
  `Sortlens`, `SortlensCraft` internal crate prefix `sortlens_`.

## 14. Decisions and alternatives considered

| Decision | Chosen | Rejected | Why |
| --- | --- | --- | --- |
| Storage | Op log + snapshot | SQLite, redb, sled | Undo/audit/replay for free; deterministic tests |
| UI toolkit | egui 0.37 (storytold fork) | GTK, Qt, Slint, Tauri | ArtCraft suite consistency; one codebase to wasm |
| Image decode | craft-libs decoders | `image` crate | Control, purity, bounded hostile input, RAW/HEIC |
| AI runtime | `tract` (pure Rust ONNX) | `ort` (C++), Burn+Candle | Licence/portability; revisit only on a benchmark |
| Map | Offline vector world, opt-in tiles | Online tiles by default | Privacy promise is the product |
| Similarity | dHash + pHash | CNN embeddings only | Works before any model download, milliseconds per 100k |
| Thumbnail format | JPEG | WebP/AVIF | Encoder maturity and speed; WebP optional later |
| Remote | Rust wasm client + Rust server | React native app, Electron | Rule: no handwritten JS; one UI |
| Deletion | Trash default, staged | Direct delete | Safety is a feature, not a preference |
| Library identity | Volume serial + rel path | Absolute path | Removable media and moves are the normal case |

## 15. Milestone-to-crate map

| Milestone | Ships | Crates touched |
| --- | --- | --- |
| M0 Skeleton | window, chrome, tokens, fonts, settings, CI | core, ui, xtask |
| M1 Index | scan, catalog, grid, sources | core, scan, catalog, thumbs, engine, ui |
| M2 Preview scales | cache, prefetch, virtualization, activity | thumbs, engine, ui |
| M3 Annotate | rate/flag/label/tags, XMP, filter bar, lightbox | meta, sidecar, catalog, engine, ui |
| M4 Cull | cull view, staging, apply, undo, quit guard | cull, catalog, engine, ui |
| M5 Organize | folders, collections, health, setup wizard | catalog, scan, engine, ui |
| M6 Analyze | blur, similarity, map, dashboard, memories | quality, hash, geo, similar, insights, ui |
| M7 Safety | delete modes, bin, audit, backup/remap | catalog, engine, ui |
| M8 AI + export | model store, inference, packs, export | ai, engine, ui |
| M9 Remote/automation | CLI, MCP, server, wasm client, i18n, keymap | mcp, remote, ui, cli |
| M10 Release | a11y audit, themes, updates, packaging | all |

## MCP

`sortlens-mcp` speaks the model-context protocol over newline-delimited JSON on stdio: one message
per line, `initialize`, `tools/list`, `tools/call`. Run it with `sortlens-cli --library DIR mcp`.

- **A tool is a command id.** `tools/list` is the command registry filtered to `Ready`; `tools/call`
  is `Engine::run`. There is no second list of agent actions, so an agent can reach exactly what the
  app can reach — and a command still marked `Planned` refuses with the milestone that will build it.
- **Every tool publishes arguments an agent can copy.** `crates/mcp/src/examples.rs` holds a worked
  argument set per command plus a note for the ones whose arguments only the caller can supply (a
  folder to index, a file to back up). The examples are **executed against a fixture library in a
  test**, so a published name is a name the engine really accepts; a wrong guess fails the build
  instead of costing an agent a round trip. The coverage test refuses to let a ready command go
  undocumented, which is what caught `view.detail` and `view.similar` being `Ready` with no handler.
- **A refusal is a tool result, not a protocol error.** `isError: true` with the engine's own message
  ("ids are required", "this library is open read-only") lets the agent read the reason and adjust;
  JSON-RPC errors are reserved for malformed requests and unknown methods.
- **Notifications are never answered**, because a client that receives an extra message where it
  expected none fails to parse the stream.
- The write path is the ordinary op log: an agent's rating shows up in `library.history` and is
  undone by the same `edit.undo` the user presses.
