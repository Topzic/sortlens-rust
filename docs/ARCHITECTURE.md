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
L5  sortlens-ui (egui)   sortlens-mcp
    apps: sortlens (desktop), sortlens-cli, sortlens-perf (measurements)
L4  sortlens-engine      commands, session, activity, jobs, filters, export, filesystem ops
L3  sortlens-catalog     op log + snapshot + counters + undo/audit
    sortlens-cull        session state machine
    sortlens-insights    dashboard, review progress, charts
    sortlens-memories    deterministic "memories" from capture dates and places
    sortlens-similar     hashing + grouping
    sortlens-remote      pairing, capability tokens, the command-registry allow-list
L2  sortlens-thumbs      tiered preview cache + job scheduler
    sortlens-scan        walker, identity, delta, offline probes
L1  sortlens-meta        craft-metadata → neutral PhotoMeta
    sortlens-media       format routing, embedded previews, video posters
    sortlens-sidecar     XMP read/merge/write
    sortlens-geo         Web Mercator, clusters, offline gazetteer, shipped land + coastline, tile folder (+ credits)
    sortlens-ai          manifest, digest-verified store, NMS + label map, EXIF hints, suggestion merge
L0  sortlens-core        ids, time, units, bytes, versioned errors
    sortlens-hash        dHash/pHash, Hamming, LSH
    sortlens-quality     sharpness score (unitless, versioned); exposure and clipping follow
    sortlens-export      export planning, collision policy, copy and re-encode
    sortlens-testkit     synthetic libraries, fixtures, assertions
    sortlens-icon        the application mark, drawn in code for the window, the SVG and the .ico/.icns files
```

Dependency rule: a crate may depend on strictly lower layers plus `craft-*` crates.
`sortlens-engine` is the only crate that mutates the catalog; `sortlens-ui` is the only one
that depends on egui.

### Why these crates, not fewer

- `hash` and `quality` are pure math with no IO: they must be unit-testable with property
  tests and reusable by the CLI without dragging in the catalog. Neither holds a decoder —
  `quality` takes RGBA planes or a luma plane and hands back a score, so the engine keeps the
  one decode path it already has, and a test that wants real pixels depends on `thumbs` as a
  dev-dependency rather than making the measure depend on it at runtime.
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
| cli, mcp, server, mobile | 3,000 | thin adapters |
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
    <sha256>/<kind>       # downloaded model files, digest-named, consent-gated
    <sha256>.<kind>.part  # an interrupted download; the next install resumes it
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
- Snapshots as built: `snapshot.json` is rewritten every `SNAPSHOT_EVERY = 256` single edits, and a
  scan takes one snapshot at the end of the whole sync rather than one every 256 files — a snapshot
  rewrites every photo, so the naive cadence made importing quadratic (78 s for ten thousand photos,
  11 s after the fix; the counter of ops since the last snapshot is reset by the snapshot, without
  which every edit past 256 rewrote the library). **The log is kept** either way. A snapshot only makes opening fast; the log is what makes the snapshot recoverable, so a
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

The CLI's `--json` is a **table** of verb to command id, not a guess: a verb that is not in the table
refuses and names the ones that are, so a script can never mistake prose for JSON. The table lives beside
the CLI so the answer to "which verbs have a schema" is one list, and the golden files pin five of the
answers plus the refusal.

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
- **Backpressure** is a bounded queue per pool. The thumbnail queue holds at most
  `QUEUE_CAPACITY` = 4 096 requests — several screens of headroom over what a grid frame asks for. A request
  that arrives at a full queue is refused *unless* it is more urgent than the least urgent waiter, which it
  then evicts; the least urgent waiter is the newest of that class, because the oldest has waited longest and
  is closest to being decoded. A refusal is not an error: the caller asks again when the photo is on screen.
  `Service::dropped()` counts refusals and `Service::queued()` counts waiters, so the bound is observable
  rather than assumed.
- **Priority is a value in the request, not a separate queue per priority** (separate queues starve):
  `Urgency::{OnScreen, Nearby, Background}`, served most-urgent-first and, within a class, oldest-first.
  The grid asks at `OnScreen`, the detail view prefetches the next photo at `Nearby`, and library-wide passes
  at `Background`. Workers pull from one shared queue under a condvar rather than from a channel, because a
  channel hands out its contents in arrival order and a queue read at pop time is what makes priority
  possible at all.

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

// What is built today: a photo is keyed by its root and relative path, and its identity is the content key.
// `sortlens_core::pathkey::PathKey` is the *folder* identity used for settings that belong to a folder (pin,
// colour): NFC-normalised, trailing separators dropped, and each component resolved to the disk's own spelling
// when the disk answers unambiguously. A photo-level PathKey (volume serial + relative path, so a photo survives
// a drive being remounted elsewhere) is the remaining half of SL-A11.

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
    pub tags: Vec<String>,            // sorted, lowercase
    pub tag_sources: Map<String, TagSource>, // manual | ai | xmp, absent = not recorded
    pub deleted: Option<Deleted>,     // at, mode, restore handle
    pub preview: PreviewState,        // Missing | Micro | Grid | Full | Failed(reason)
    pub analysis: AnalysisState,      // blur score, hashes, ai, each with version
}
```

Annotations are explicit fields, not a bag, so batch ops and counters are O(n) and typed.
Tags are the one many-to-many relation, held in the tag index (§3.4).

**Tag provenance** (SL-D12) is the one annotation that is a map rather than a field, because it is a
fact about a word rather than about the photo: `tag_sources` holds an entry for each keyword whose
origin was recorded, and a keyword with no entry is `TagSource::Unrecorded` — derived, never stored,
so a library written before sources existed says "nobody wrote it down" instead of inventing "typed
by hand". It rides in the log as its own `Op::TagSourced` committed in the same `Op::Group` as the
`Op::Tagged` it describes: one ⌘Z restores the word and its origin together, and the 2-tuple `tagged`
records already on disk still decode. `rev` is deliberately not bumped by it — `rev` is what a stale
sidecar writer is refused against, and provenance is never written to a sidecar. Renaming a keyword
carries its source to the new name; deleting one takes the source away.

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
│ sources   │ filter bar (collapsible, wraps, badge count;        │ Details           │
│           │             hidden while the catalog is empty)      │                   │
│ folders   │                                                     │ Tags              │
│ collections│              view canvas                          │ Suggestions       │
│           │      library · for you · cull · detail · compare ·  │ Activity          │
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
- The bar is the title bar when the frame is the app's (`craft-window`'s `Frame::Integrated`), so it draws
  the window's minimize, maximize/restore and close glyphs at its far end; under a system title bar the OS
  draws them and the shell draws none. The bar asks rather than commands: `Action::MinimizeWindow`,
  `ToggleZoom` and `CloseWindow` go to the host, which sends the viewport commands. Close is the system close
  request, not `Quit`, so the unsaved-changes guard runs first.
- Empty states are first-class: every view has a title, one sentence, and 1–2 primary buttons
  that run real commands.
- Destructive dialogs are in-app (themeable, testable, screenshot-able), not native message
  boxes. File pickers are native via `craft-dialogs`.

### 8.2 Theme tokens

One `Tokens` struct (colours + metrics) with five presets — **Charcoal**, **Midnight**,
**Silver**, **Paper** and the accessibility preset **High Contrast** (pure black/white/yellow,
doubled chrome strokes). Mapping to egui via
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
  a `CRAFT_FONTS: &[CraftFont]` table. It also reads `CRAFT_FONTS_SCRIPTS` — a script filter for builds
  that must not carry faces the interface cannot paint: the phone's trim. The parser and selection
  rules live in `crates/ui/src/font_manifest.rs`, a module `cargo xtask fonts` includes too, so a
  build and its report answer to one implementation; `cargo xtask mobile` passes the phone's script
  set (`Latn`, `Zsym`, `Arab`, `Jpan`) when a checkout is being embedded from.
- `fonts::install(ctx)` registers them and **appends** each face to `Proportional` and `Monospace`,
  after egui's own, so a fallback can never take over Latin text.
- `fonts::coverage()` checks the scripts the interface itself draws (`Latn`, `Zsym`); the translated
  scripts are listed separately, ready for when translations land.
- `cargo xtask fonts` reports the face and family count from the checkout and refuses a manifest with
  no Latin face. `fonts/embedded.txt` records what the last release shipped.

### 8.4 Interface language (i18n)

`crates/ui/src/i18n.rs` routes the interface's strings through `craft-i18n`: the English text is
the lookup key, a missing row is the English itself, and the catalogue a translation has not
reached yet never breaks the interface. The scaffold (SL-L07, Phase 4 item 2d) embeds Arabic and
Japanese rows for the strings routed so far — the sidebar's sources and sections, the foot's
registry labels and the Settings dialog — and offers the picker (Settings → Language) that names
every language in itself. `Appearance.language` persists the code and an unknown code reads as
English.

The direction plumbing is `craft-ui`'s: `i18n::apply_direction` resolves the interface direction
from the chosen language (overridable with `CRAFT_UI_DIRECTION` for checking a mirrored layout
before a right-to-left catalogue exists) and installs it on the context every frame — egui's own
chrome mirrors through the fork's `Style::right_to_left`, the sidebar sits at the reading start
and the tool strip and right panel at the reading end, and `nav_row` paints its glyph, label and
count from the reading start. A left-to-right interface paints byte for byte what it painted
before the scaffold existed — the screenshot baselines are the proof.

Strings reach the screen through `crates/ui/src/i18n.rs`, whose English text is the catalogue key
(the row list is in `crates/ui/src/i18n/{ar,ja}.tsv`). The Welcome card, the indexing screen and the
empty-filter screen are routed through it: a sentence with a count in it is one template with `{n}`
filled by `i18n::fmt`, so the count is written in the language's digits and a language that does
not inflect the noun is not forced through English's plural.

The command palette's own chrome is routed too: `Type a command…` is the field's hint and its
accessible name, `No command matches` and the `Up/Down choose · Enter run · Esc photo search` legend
are rows, and `a11y::name_field` names the field with the same sentence its hint paints — a hint is
painted, not announced, so without it a screen reader met an unnamed box. The key caps
(`Up/Down`, `Enter`, `Esc`, `⌘K`/`Ctrl+K`) are the platform's names for the keys and stay as written
in every language.

The export dialog and its receipt are routed the same way, but their numbers come from
`sortlens-export`'s `Spec::parts` and `Report::parts` — the very pieces the engine's English
`summary` and `headline` are joined from — and each piece is worded through the catalogues. A
translated dialog therefore cannot describe a spec the export will not write, and the receipt is kept
as those pieces until it is dismissed rather than as a sentence frozen in the language it was set in.

The three confirmations that destroy files are routed too: the similar view's delete, the cull
batch's Apply and the rejects-bin emptying word their counts through `i18n::fmt` templates (so the
noun is not forced through English's plural), their modes through `tl!`, and the warning about files
changed on disk with the same keys everywhere it appears. A toast is worded when it is raised — it is
momentary, unlike the receipt — while the dialog itself repaints on a language switch. The catalogue
audit reads both routes, `tl!` literals and `fmt` templates, so a template a translator has no row for
is a failing test rather than a sentence that quietly stays English.

The rejects bin's rows are routed the same way, from the applied batch's own pieces: `bin_row` joins a
row from `DeletedBatch`'s count and `failed`, its `applied_at` through the shared `age_label`, and the
`Destination` `sortlens-cull` decodes from the stored `mode` — the piece the Batch applied receipt is
worded from — so the two surfaces cannot describe one batch differently and no mode id reaches the
screen. `a_bin_row_speaks_the_interface_language` applies a real tag-only batch and paints the dialog in
English, Japanese and Arabic.

The rename and About dialogs are routed end to end: titles and buttons through `tl!`, the rename
toast and both About lines (`Version {version}`, `{commands} commands, {ready} ready today.`) through
`i18n::fmt`, because a language puts a version number where its own grammar wants it. They are the two
dialogs whose every word is the shell's own — no engine sentence inside — so a missing row there is
English on an otherwise translated screen, which is what `the_rename_and_about_dialogs_speak_the_interface_language`
paints in all three languages.

Library health's catalog half is routed from the catalog's own pieces: `sortlens-catalog`'s `Audit`
returns a `Headline` (`Clean`, `Recoverable`, `NeedsAttention`, each with its counts) and a
`Vec<AuditPart>` (`Snapshot`, `Replayed`, `Skipped`, `LogBytes`), and its own English `headline()`
and `detail()` are joined from exactly those pieces, so the dialog cannot describe an audit the audit
did not find. The shell words each piece through `i18n::fmt` and groups the counts with
`format_count`, and keeps the pieces in `AuditView` rather than the English sentence, so the section
repaints when Settings → Language moves — the same rule the export receipt follows. The rest of the
dialog is routed the same way: the health check's count line and its toast, the changed-on-disk line and
`Refresh changed`, the root rows (`{n} photo(s) under {root}`, `Point at…`) and the metadata retry and
remove-missing lines are `tl!` keys or `fmt` templates, so every count in them is a placeholder a
language places — and a language whose digits are not ASCII writes them (`health_stats` and its
siblings are the wording helpers the tests read).

The cull summary is routed from `sortlens-cull`'s own pieces: `Summary::headline_kind` returns a
`Headline` (`Nothing`, `AllJudged { total }`, `Partial { judged, total }`) and the session's own
`headline()` is joined from that piece, so the sheet's sentence and the counts beneath it cannot
disagree. The shell words it with `format_count`, and the sheet's four rows, its Apply, Keep
culling, Jump to next and Discard buttons with their hovers, the `{changed} … {deferred} …` foot
line and the discarded toast are `tl!`/`fmt` keys — `cull_headline` and `cull_foot` are the
wording helpers the paint test reads.

The quit guard is routed from the engine's own pieces: `Engine::unsaved_reasons` returns an
`UnsavedReason` (`Scanning`, `StagedCull { staged }`, `Hashing`) and `Engine::unsaved` — the
sentences the support report prints — is joined from exactly those pieces, so a translated guard
cannot warn about different work than the report describes. The shell words each piece through
`tl!`/`i18n::fmt` and the staged count through `format_count`; the title, both buttons and the
hover are catalogued, and `the_quit_guard_speaks_the_interface_language` paints the dialog in all
three languages.

The rejects bin is routed the same way: its title, its empty state, `Restore`, `Forget`,
`Empty bin…` and the three hovers are `tl!` keys, and its two toasts are `i18n::fmt` templates with
the counts as placeholders, so a restored or forgotten batch is announced in the chosen language.
The row above the buttons is the exception it names: `Batch::summary()` and the batch's mode are
`sortlens-engine`'s own sentences, the same words the report prints, and
`the_recently_deleted_dialog_speaks_the_interface_language` paints the empty state in English,
Japanese and Arabic.

The change viewer is routed from the log's own pieces: `Op::description` returns a
`sortlens_catalog::op::Description` (the counts, paths and names travel with it) and `Op::summary`
— the English the report and the command line print — is that piece joined by `Description::english`,
so a translated viewer and the English report cannot describe different work. `history::Entry`
carries the piece, and the shell words each variant through `i18n::fmt`, with a photo count filled
by `format_count` inside the suite's `{n} photo(s)` wording, so a language places the count and
Arabic writes its own digits. The row's second line is the time alone: the op's own name
(`tagged`, `rate`) stays in the log, the report and the wire rather than being printed untranslated,
and `the_change_viewer_speaks_the_interface_language` paints the dialog in all three languages.
The Batch applied receipt is routed from `sortlens-cull`'s own pieces: `Receipt::headline_kind`
returns a `ReceiptHeadline` (`Handled { photos }`) and `Receipt::detail_parts` a `Vec<DetailPart>`
(`Destination`, `Picks`, `Failed`), with `Receipt::headline` and `Receipt::detail` — the sentences
the record and the report print — joined from exactly those pieces. The shell words the destination
(`Moved to the system Trash`, `Left on disk and flagged as rejected`, `Moved to {folder}`), the
pick count and the failure count through `tl!`/`i18n::fmt`, with every count a `format_count`
placeholder; `receipt_headline` and `receipt_detail` are the wording helpers, and
`the_batch_applied_receipt_speaks_the_interface_language` paints the dialog in all three
languages.

The Moved receipt is routed from `sortlens-engine`'s own pieces too, but it has no kind enum to hand
over: `MoveReceipt` carries its count (`items.len()`) and its destination, so
`move_receipt_headline` words those through `{n} photo moved to {folder}`/`{n} photos moved to
{folder}` with `format_count`, while the engine's English `headline()` — the sentence its other
callers print — stays as it was. The title, the sentence under it, `and {n} more` and both buttons
are `tl!`/`i18n::fmt` keys, and `the_move_receipt_speaks_the_interface_language` makes one real
move of one indexed photo and paints the dialog in English, Japanese and Arabic.

The notices panel takes the same seam: its window title, hover, empty sentence and `Clear` action
are `tl!` rows, `bell_button` words the unread count it speaks through `Alerts, {n} unread`, and
`notice_age` fills `{n}s`/`{n}m`/`{n}h`/`{n}d` (or `just now`) with `i18n::count` — the same digits
`paint_badge` now writes for the bell and the Filter chip. `the_notices_panel_speaks_the_interface_language`
paints the popover and the bell in English, Arabic and Japanese and requires the Arabic age and
badge to carry `١` and no English title or `Clear` behind them.

The scan-finished notice is the same seam over a `ScanFinished`: `indexed_lead` words one photo and
several through two templates (`Indexed {n} photo in {folder}` / `Indexed {n} photos in {folder}`),
`scan_notice` appends `{n} missing`, `{n} routed by content, not name` and `{n} unreadable` with the
` · ` junction outside the rows, and `cancelled_notice` words the stopped walk. The first-scan banner
takes its lead from the same `indexed_lead` and its `Start culling {n} photos` from the row the cull
screen's button paints. `the_indexing_notices_speak_the_interface_language` pins the English sentence
with all three suffixes, the Arabic and Japanese fills, and then runs a real first scan under Arabic
through `show`, requiring both the banner and the notice to be the catalogue's sentence.

The rating receipt takes the same seam over the click's stars and count: `rating_notice` words
`Cleared rating` / `Cleared the rating on {n} photos` and `Rated {stars}★` / `Rated {stars}★ on {n}`
through `format_count`, so neither the stars nor the count can arrive in Latin digits inside an
Arabic sentence, and `the_rating_notices_speak_the_interface_language` rates a real two-photo
selection under Arabic and requires the toast and the bell's painted history to agree with the
catalogue.

The flag receipt takes it over the click's flag and count: `flag_notice` words `Picked`, `Rejected`
and `Unflagged` and their three several-photo templates, with the count through `format_count`, so an
Arabic flag reads `٢ صورة مختارة` and `the_flag_notices_speak_the_interface_language` flags a real
two-photo selection under Arabic and requires the toast and the bell's painted history to agree.

`apps/sortlens` paints words of its own — the status line after a pick, a move or an export, and
the titles of the native `rfd` dialogs — where no `tl!` site in `crates/ui` can reach them. The
host calls the same lookup through `i18n::tr` (a literal) and `i18n::fmt` (a template with
`i18n::count` for regional digits), and the move notice asks `shell::move_receipt_headline`, so the
status line and the receipt dialog cannot word the same move differently. Because the catalogue
audit in `crates/ui` reads only its own crate's sources, the host carries its own listing:
`the_hosts_words_have_a_row_in_every_catalogue` reads `main.rs` back, collects every `i18n::tr`/
`i18n::fmt` key and requires a row for each in all six catalogues, and
`the_hosts_move_notice_reads_the_interface_language` pins the English, the Arabic count and noun,
and the Arabic dialog title. The library-lifecycle notices take the same seam — settings, library
open, backup and restore (titles, filters and toasts), the shortcut set's import and export, the
sync server's start, stop and revoke lines, and both sidecar toggles — and
`the_hosts_library_notices_read_the_interface_language` pins the English count and the Arabic
failure frame and restored toast with its Arabic-Indic digit.

The phone host, `apps/sortlens-mobile`, takes the same seam for its own words: the settings repair
line in `prefs.rs`, and in `lib.rs` the library-open failure, both folder lines, the removed-photos
notice, the suggestion pass's start and finished lines, both tag toasts, undo-move, the pairing,
testing, forget, send and cancel lines, the phone-only fallback and both pairing answers. Counts go
through `i18n::count`, so an Arabic phone writes its own digits inside a template it also words. The
phone's listing reads `lib.rs` and `prefs.rs` (`the_hosts_words_have_a_row_in_every_catalogue`), and
`the_hosts_notices_read_the_interface_language` pins the English frame and the Arabic template. The
`send.rs` sentences the shell paints as the run's summary and the removal coda are still the crate's
English; the engine's own `error.to_string()` is appended unchanged there and here.

A colour label is one worded piece: `colour_word` reads the `Red`…`Purple` rows, and every
surface that paints one goes through it — the filter chip's word and its spoken name, the folder
and collection `Colour` menus, the inspector's Label row and Tags tab, Library insights' label
bars, and the status lines a recolour or a folder mark raises (`folder_mark_notice`,
`collection_colour_notice`, `label_notice`), whose `{colour}` and `{n}` are placeholders so a
count is written in the reader's digits and a sentence cannot word the colour differently from the
menu that set it. English writes the word lowercase inside a sentence and unchanged as a menu item;
a language without case is untouched either way.

The `?` overlay is catalogued the same way: every row of `crates/ui/src/keymap.rs`'s table paints
through `Binding::localized_action`, and every section through `Context::localized_label` and
`Context::localized_note`, which look the English text up at paint time; the title, the search hint,
both count lines and the empty-search line are `tl!`/`fmt` keys. `every_shortcut_row_is_in_both_catalogues`
reads the whole table and fails on a binding with no Japanese or Arabic row, so a new shortcut is a
translation debt the suite names. `the_shortcuts_dialog_speaks_the_interface_language` paints the
overlay in all three languages.

The editor dialog behind Settings → Shortcuts speaks the same catalogues through
`i18n::CatalogText`, an implementation of `craft-keymap`'s `Text` trait: the crate's own strings
(`craft_keymap::ui::STRINGS`), its `@plural` counts, the command labels and the menu paths are
looked up at paint time, and the read-only fixed rows are translated before they are handed over
because the dialog paints those raw. `every_keymap_editor_string_is_in_both_catalogues` walks the
crate's lists and demands an Arabic and a Japanese row for every one — presence rather than
difference, because key names and order-only templates translate to themselves on purpose — and
`the_shortcut_editor_speaks_the_interface_language` paints the dialog with Japanese chosen. The
import and saved toasts are `fmt`/`tl!` keys like every other notice.

The Activity tab is routed from `sortlens-engine`'s own piece: `Job::detail_kind` returns a
`JobDetail` — `Progress`, `Counts { files, dirs }`, `Finished { files, dirs, unreadable }`,
`Cancelled`, `Failed` — and `Job::detail`, the line the report and the JSON print, is that piece's
`english()`. The shell's `job_detail` words it through `i18n::fmt` with both counters (and the
unreadable count) as `format_count` placeholders, so Arabic writes its own digits; the job's kind
(`JobKind::label`) and state (`JobStatus::label`) are looked up by their English text, the empty
hint and the stats rows are `tl!` keys, and a failed job's error stays the error the platform
reported. English is byte for byte what the row painted before it took a piece, and
`the_activity_row_speaks_the_interface_language` paints the tab in all three languages.

The Suggestions panel is routed from `sortlens-ai`'s own pieces: `Suggestion::line_kind` returns a
`SuggestionLine` (the keyword, the `Source`, and the confidence as a percentage) and
`Suggestion::line` — the English the panel and the report print — is that piece's `english()`, so a
translated row cannot name a different source or confidence. `source_word` and `family_word` look the
four source labels (`detector`, `camera`, `pack`, `caption`) and the three family headings up at paint
time, `suggestion_percent` writes the confidence through `format_count`, and the right-panel tab, the
Suggested row in the inspector and the whole-library `Suggested keywords` dialog word their counts as
`fmt` templates. `every_source_word_and_family_heading_is_the_catalogues` reads every one of them out
of the catalogues, so a source the pipeline adds later cannot arrive with a word only English has, and
`the_suggestions_tab_speaks_the_interface_language` paints the tab in all three languages.

`crates/ui/src/format.rs` (SL-L08) writes the numbers and dates the shell paints in the interface
language's own convention: an explicit style per offered language fixes the digit set, the grouping
and decimal marks and the date's field order, and a test holds the table to `i18n::LANGUAGES` so a
new language needs its numbers and dates decided. The formatting region is the interface language
rather than the operating system's, so a committed scene reads the same on every machine; English
is the shape the app has always painted, byte for byte.

### 8.5 Keymap and menus

`craft-keymap` owns chords, scopes, conflicts, overrides and the editor. Our contribution is
the catalog: ids, labels, menu paths, defaults — the rebindable rows of `crates/ui/src/keymap.rs`'s
table, which is also what the editor, the palette and the `?` overlay read.

The table spells the primary modifier `Ctrl+`, and that is the platform's command key, not the
physical Control: ⌘ on macOS, Ctrl everywhere else (where `craft_keymap` folds the two for matching
anyway). The translation is one function at the ui's edge, so a table row, a rebind typed as
`Ctrl+Alt+O` and the chord the dialog paints all mean the same press — before it, a Mac build bound
Control, so none of the shortcuts the help listed worked. Painting follows the faces: with a symbols
face embedded the keycaps are the platform's own (`⌘⇧R`), and without one the shell spells what egui's
bundled fonts cannot draw as the table's words (`⌘Shift+R`), because `⌘` is the one modifier glyph they
carry and `⌃`/`⇧`/`⏎` are boxes in front of every shortcut.

Defaults for culling (the shipped ones, not a plan):

| Action | Key |
| --- | --- |
| Keep / Reject / Skip / Undecide | `P` / `X` / `Space` / `U` |
| Step back one photo | `◀` (`cull.back`, rebindable) |
| Apply the last keyword | `T` (`photo.tagFromPrev`) |
| Apply the staged rejections | `Ctrl+Enter` (`cull.apply`) |
| Zoom to fit or 2× | `Z` · pan while zoomed `Alt+drag` |
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
      ├── downloader (engine, consent-gated, resumable, Range + digest rename)
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
  visible to the inference path. The downloader exists (`crates/engine/src/models.rs`); the inference
  runtime lives in `crates/ai/src/infer.rs` — tract-onnx, desktop-only, the same pure-Rust store of
  guesses the arithmetic below describes.
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
- `diagnostics::redact` is a sorted, longest-first replacement of absolute paths, and
  `Engine::diagnostics` builds the bundle around it; nothing in the bundle is a decision, so it writes no
  op and the health check it runs is read-only.
- `ai::packs` reads a pack from JSON: a name, a licence, and rules whose `when` conditions are ANDed
  over the photo's folder, camera, lens, ISO and focal band. A rule with no conditions is refused,
  because one typo would otherwise tag the whole library; an unknown `when` field is refused rather than
  ignored. `Engine::packs` lists every file in `<library>/packs/` including the broken ones, and
  `Catalog::set_pack` records the switch as an op, so it persists and undoes like anything else.
- The packs also speak to the CLIP pair once both embedders are installed: `crates/ai/src/tokenize.rs`
  turns each pack keyword into a 77-token CLIP BPE sequence (the vocabulary is the upstream merges file,
  shipped byte-for-byte and decompressed here), the text embedder turns it into a 512-d vector, and
  `crates/ai/src/text_cache.rs` keeps those vectors in `<library>/models/text-cache/<text model digest>/
<pack content hash>.json` — keyed by the model's digest and the pack file's hash, so a changed pack or a
  different installed model is a different cache file by construction, nothing is ever invalidated. The
  engine embeds a 224 px preview per photo with the image embedder and proposes every cached keyword
  whose cosine clears `EMBED_MIN_COSINE` (0.25, set from the real pins' live numbers: the correct match
  scored 0.281, unrelated photographic prompts 0.19–0.24), through the same merge as the matcher's hits;
  the cosine *is* the score, so the panel stays honest about how much the model agreed. A fully cached
  pack is still served while its text model is away; a pack that was never embedded is not, and the pass
  goes on without it.
- `Engine::suggestions` maps a `Photo` to `ai::Hints`, adds the enabled packs' candidates for that photo,
  merges, and returns proposals; `suggest.list`, `suggest.accept` and `suggest.dismiss` expose it, and the
  CLI has `suggest <id>` / `accept <id> <tag>` / `dismiss <ids> <tag>`. Accepting goes through
  `Engine::set_tag`, so a suggestion becomes an ordinary `Tagged` op — in the log, in the history, and
  undone by the same `undo` as a keyword typed by hand. `accept_suggestions` writes a whole selection's
  keywords and their provenance in one `Op::Group`, so "accept every proposal of this kind" is one ⌘Z.
  Dismissing is stored rather than derived: `Op::SuggestionDismissed` replaces a photo's
  `dismissed_suggestions` set, `suggestions_for` filters those candidates out *before* the cap applies, and
  a unit test asserts the dismissal survives a reopen and a single undo returns the proposal. The detail
  view draws the list as a "Suggested" row whose buttons name their score and source; the right panel's
  Suggestions tab draws the same proposals for the whole selection, grouped by family.
- `Engine::begin_batch_suggest` runs the same merge over the whole library on a worker, with progress,
  cancel, poll, results and clear, and **writes nothing**: the panel offers rows and each row is a
  button. A test asserts the tags and the change log are identical before and after a pass, because a
  batch that tagged by itself would be a bulk edit nobody reviewed.
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

## 10. Remote and phones

- `apps/sortlens-mobile` (SL-J10): the phone and tablet app, a native Rust library — a `cdylib` for the
  Android activity, a `staticlib` for the iOS app, an `rlib` for tests. It is the desktop host with the
  desktop taken out: the same `sortlens-engine` and `sortlens-ui` driven by an eframe `App`, minus the title
  bar, file dialogs, reveal-in-folder and the sync server. The photographs are on the device, so the phone
  opens its own catalog under the app's private directory and indexes the camera roll (`places`); the
  two entry points (`android_main`, `sortlens_ios_main`) are the only unsafe code in the application and
  sit in one module. `cargo xtask mobile` checks both targets, which keeps the shared crates portable.
  The choices its shell offers live in `config.toml` in that same private directory, read and written
  through `craft-settings` (`apps/sortlens-mobile/src/prefs.rs`): a debounced save, a damaged file moved
  aside with a notice, and an unknown id reading as its default — the desktop's exact rules. The shell
  answers a phone's width by itself, in `crates/ui`, so the phone app inherits it: under 600 points the
  two docks become full-width sheets, one at a time, the bar keeps the mark, one search field and five
  glyphs, and the canvas is the phone's too — the far-edge rail stands down while the inspector is closed
  and tab-switches beside its open sheet, the strip wraps into two rows without a second count, and the
  bottom bar's tray keeps every control on screen (docs/UX.md §2). Not yet done: trimming the embedded font
  set for a phone binary, and the iOS Photos library (until then iOS reads the Files-visible Documents
  folder).
- `Catalog::open_memory`, `Engine::open_memory` and `library.snapshot` (shipped 2026-11-24): a catalog
  opened from snapshot JSON alone — no directory, log or lock — with the same undo, redo and
  stale-revision rules as the disk catalog. The device protocol lives in `sortlens_remote::client`: pairing,
  `/api` calls, the resumable `/upload` with the keep-or-remove decision applied on the ack, and
  `thumbnail` fetching `GET /thumb/<id>/<edge>`. A phone uses it to send what it shoots to the
  desktop's library, through `sortlens_remote::send`: a run owns one worker thread, the shell polls its
  progress each frame and stops it between items, and the finished run hands the ids the ack cleared to
  `Engine::delete_photos(..., Trash)` — the desktop's audited delete path, so a file the catalog no longer
  matches is refused rather than removed. Before a code is spent the same surface can run the connection test
  (`GET /health`, no token, SL-J07) against the typed address on its own thread, saying which address answered
  or why none did and storing nothing either way. The app's state around it is `apps/sortlens-mobile/src/send.rs`
  (the token, the address it belongs to, the pairing attempt, the connection test and the run) and the surface
  is Settings → Send to desktop in `crates/ui`.
- `apps/sortlens-server` (shipped 2026-11-24, Phase 4 item 3): a token-authenticated JSON API on a
  hand-written minimal HTTP/1.1 over `std::net` — no HTTP dependency to audit. Binds loopback by
  default and every interface only under `--bind lan`; a six-digit pairing code (ten minutes, five
  wrong attempts lock it out) becomes a scoped device token stored as
  `<library>/remote-tokens.json` with per-token permissions (`read`, `upload`, `annotate`, `cull`).
  Routes: `GET /health` (the connection test, no token), `POST /pair`, `POST /api`
  (`{method, params}` → the registry's answer), `POST /upload`, `GET /upload/<id>`, and
  `GET /thumb/<photo-id>/<edge>` (the export path's lossless PNG, decoded through the desktop
  grid's `sortlens-thumbs` path, bounded to 16–1024 px, under `read`).
- The desktop's own surface (shipped 2026-11-24, Phase 4 item 3): Settings → Sync & devices spawns
  the server binary beside the app and follows its stdout through `sortlens-remote::supervise`
  (the startup-line contract is parsed once, beside the protocol), shows the live port and pairing
  code, lists the paired devices from the token file, and revokes by exact token — refused while
  the process runs, because a running server holds its own token store and would write the row
  straight back. Serving beyond loopback is the CLI's `--bind lan`, not a desktop checkbox.
- The remote API is the command registry with a restricted allow-list, not a second API. `crates/remote`'s
  table names every id the wire accepts with the scope it needs, is audited against `Registry::v0()` at
  startup, and refuses a host-effect command (`app.quit`, `photo.reveal`, the model/update network
  calls, library surgery) without leaking whether it exists. A phone culling session is the same
  `cull.*` commands the window runs.
- Uploads are resumable, verified by size + SHA-256 (the suite's own), and land in a configured album
  folder. Chunks append to `<album>/.uploads/<id>.part`, so the file length is the received count and a
  restart loses nothing; the completing chunk is hashed before it is appended, so a digest mismatch
  leaves the partial exactly where it was; a published name never overwrites. A one-shot upload, one
  with no partial yet, is verified against the body alone — the first-contact case once refused every
  one-shot with a 500 because it opened a partial file that did not exist.
- Proven by the two-process loopback harness in `apps/sortlens-server/tests/loopback.rs`: a real server
  process against a real scanned library, pairing through `/pair`, reads and writes through `/api`, an
  upload that dies mid-transfer and resumes, and a token that survives a server restart.
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
What exists today: `Fixture` writes JPEGs through the real encoder (so a fixture photo decodes,
hashes and has a resolution) — solid colour, or a deterministically phased gradient carrying XMP
pixel dimensions and optional EXIF GPS, because the similarity pass reads structure and a library
of solid colours is one enormous group. It writes sidecars under the `docs/BEHAVIOUR.md` §4 naming
rule, names its own folder (`Fixture::at`) when a test needs a stable path, and removes the folder
on drop. The same bytes every run is the property that makes the golden and screenshot jobs below
possible.

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

**Screenshot baselines** (`crates/ui/tests/screenshots.rs`, run by `cargo xtask screenshots`): the real
shell — `sortlens_ui::show` inside an `eframe::App`, the same entry the window uses — painted at a fixed
1440×900 with `pixels_per_point = 1.0` across seventeen surfaces (Empty Welcome, Library in both
layouts, the empty-filter canvas, Cull, Detail, Compare, Similar, Places, Insights, Suggestions, the palette,
Soft Focus, Missing, the active filter strip, the Arabic interface right-to-left in both themes, and a
900×620 narrow window that keeps one theme because it is about geometry) in the Charcoal and Paper presets,
and the 1200×800 filter baselines that share the strip's wrap. The harness installs the vendored Arabic face
and never the machine's craft-fonts checkout (`fonts::stack_builtin`), so a baseline is the same frame on a
developer's build and on CI. Fifty PNGs are committed under `crates/ui/tests/snapshots/` and diffed by
`egui_kittest`, with the per-pixel tolerance
in `kittest.toml`; a mismatch writes the painted image and a diff beside the baseline and fails the run.
A test does not stop at its first mismatch: `shoot` records the scene's name and error in a per-thread
report instead of panicking, and each test ends with `assert_every_scene_matched()`, which fails with every
scene that differed and says how many of the test's scenes matched — and a contract test reads the file it
lives in and refuses a painting test that never calls it. That is what makes a red run hand over all of its
paint: a panic mid-test used to skip every later scene of that test, so a red artifact held twenty of the
twenty-four frames the set needed and the other four took two more red runs to arrive.
The library behind them is a real fixture: fixed ratings, flags, keywords, a collection, EXIF
dimensions, two locations and a duplicate pair, scanned once. The harness is a `sortlens-ui` feature
(`screenshots`) that no app links, because it needs a GPU adapter and `cargo xtask ci` stays headless;
tests run one at a time, and each settled surface also passes `egui_kittest`'s accessibility check —
which is how the unnamed sort dropdown was found. `--update` rewrites the baselines and is a person's
decision; the image diff is the review. Baselines are verified on macOS (Metal), which is where the
suite's own project treats them as authoritative.

A painter that differs is a **baseline set**, not a looser test. `SORTLENS_SNAPSHOT_SET` names one —
the runner sets it to `lavapipe` — and every scene then reads
`tests/snapshots/<set>/<name>.png` instead of `tests/snapshots/<name>.png`; the set is still compared
byte for byte, so a set is either painted by that adapter or absent. The name is folded to a plain
directory (lower case, one dash per run of anything else), so a hand-written `Mesa LLVM 15.0` is one
folder and `../..` is none. CI uploads a failed run's `.new.png` paint as the `screenshot-set`
artifact, which is how a set that nobody has committed yet gets committed: download, read the frames,
add them under the set's directory.
`every_scene_is_painted_in_both_committed_sets` reads both directories and refuses a scene that
only one of them holds, so a new surface cannot be painted into one set and left as a missing
baseline on the other; a change to a word also repaints both sets — the reference one with `--update`
on a developer's machine, the `lavapipe` one by adopting the runner's uploaded frames — because the
same text laid out by two rasterisers is still two committed frames.

CI (`xtask`, mirroring LightCraft):
`cargo xtask ci --quick` (fmt, clippy -D warnings, changed-package tests, layers, roots,
assets), `cargo xtask ci` (full + benches smoke), plus `cargo xtask mobile` per platform, plus a Windows leg and a
snapshot-update job. Sonar or `cargo-mutants` optional for the catalog crate.

## 13. Distribution

- Desktop: `eframe` + `wgpu` (Metal/Vulkan/DX12), signed and notarised macOS app bundle,
  Windows MSI/Inno-free installer (WiX or `cargo-bundle`), Linux AppImage + Flatpak.
- Updates: `craft-update`'s release parsing and semver rules behind the engine's consent-gated
  `update.check` (SL-I10, opt-in); the answer — tag, version, notes headline — shows in Settings →
  Updates and `update.status`. Nothing downloads or installs automatically; self-update stays out
  until the installer work in M10's absence table exists.
- Phones: an APK/AAB from `cargo-apk` and an iOS app linking the `sortlens-mobile` staticlib; no network at runtime.
- Geography: `assets/land.txt` and `assets/coastline.txt` — the Natural Earth 1:110m land polygons and
  coastline (public domain), embedded in `sortlens-geo` with `include_str!`. The land is triangulated once with
  `earcutr` and painted as one filled mesh under the Places pins; the coast is stroked over it. Both carry their
  credit on screen. A `tiles/` folder or a `tiles.mbtiles` archive (read-only SQLite, credit in `metadata` or
  beside it) the person supplies replaces them; nothing is fetched at run time. `walkers` is built with its default
  features off and is given a disk-backed `Tiles` adapter; no HTTP tile source is ever constructed, so the map never
  opens a socket even though the crate's client code is linked.
- Icons: our own SVG set (40 monochrome glyphs in `assets/icons/`, prepared by
  `craft_ui::icons::SvgIconSet` and rasterised by `egui_extras`' SVG loader — the one image loader this
  app links), one brand icon in `assets/` whose geometry in `sortlens-icon` is rendered three ways — the
  window RGBA, the committed SVG, and the committed `assets/icon.ico` / `assets/icon.icns` packaging
  files that `cargo xtask icons` checks (`--update` rewrites) and the `sortlens-icon` tests hold to the
  drawing byte for byte.
- Manifest: `ai.storyteller`-style reverse-DNS id per ArtCraft convention, app name
  `Sortlens`, `SortlensCraft` internal crate prefix `sortlens_`.

## 14. Decisions and alternatives considered

| Decision | Chosen | Rejected | Why |
| --- | --- | --- | --- |
| Storage | Op log + snapshot | SQLite, redb, sled | Undo/audit/replay for free; deterministic tests |
| UI toolkit | egui 0.37 (storytold fork) | GTK, Qt, Slint, Tauri | ArtCraft suite consistency; one codebase for desktop and phone |
| Image decode | craft-libs decoders (`craft-jpeg`, `craft-image-codecs`, `craft-raw`, `craft-heif`) | `image` crate | Control, purity, bounded hostile input, RAW/HEIC |
| HEIC | `craft-heif` (pure Rust `heic-rs`, panic-guarded, libheif colour) | libheif (C) | No C dependency; 16-bit samples so a 10-bit photo is not clipped |
| AI runtime | `tract` (pure Rust ONNX) | `ort` (C++), Burn+Candle | Licence/portability; revisit only on a benchmark |
| Map | `walkers` gestures + offline Natural Earth land, opt-in tiles | Online tiles by default; custom pan/zoom | Privacy promise is the product; one proven gesture implementation |
| Similarity | dHash + pHash | CNN embeddings only | Works before any model download, milliseconds per 100k |
| Thumbnail format | JPEG | WebP/AVIF | Encoder maturity and speed; WebP optional later |
| Phones | Native Rust app (`sortlens-mobile`) + optional Rust server | React Native, Swift/Kotlin UIs | The vendor ships native phone apps; the Rust engine and UI are reused, no handwritten UI code |
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
| M9 Remote/automation | CLI, MCP, server, phone app, i18n, keymap | mcp, remote, ui, cli |
| M10 Release | a11y audit, themes, updates, packaging | all |

## MCP

`sortlens-mcp` speaks the model-context protocol over newline-delimited JSON on stdio: one message
per line, `initialize`, `tools/list`, `tools/call`. Run it with `sortlens-cli --library DIR mcp`.

- **The protocol plumbing is the suite's.** `craft-jsonrpc` validates envelopes, separates requests from
  notifications, answers batches and enforces a per-message byte bound *before* parsing, so an eight-megabyte
  line is refused without being allocated. `craft-mcp` owns the session: version negotiation, the
  `initialize`/`initialized` handshake, the tools envelope and the text result. This crate owns the transport
  and the method table — the division both crates state for themselves — which is why a protocol detail is
  fixed once in the suite rather than twice here.
- **`craft-ipc` is deliberately unused.** There is no second process: the window owns the engine and the
  library's write lock, and the CLI opens the library itself. A socket needs a server mode, authentication on a
  multi-user machine and an answer for "the app is not running"; `docs/ROADMAP.md` names the opener that would
  justify it.

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
