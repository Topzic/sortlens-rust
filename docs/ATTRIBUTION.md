# Attribution and licences

Every third-party dependency and asset in this repository, with the licence it is used under. Two
checks keep this file honest, and both run in `cargo xtask ci`:

- `cargo xtask attribution` fails when a crate that a manifest links is missing from the "Linked today"
  table, when a crate is declared in `[workspace.dependencies]` but not listed as "Declared, not linked
  yet", when a table names something that is no longer a dependency at all, when an embedded font family
  is not attributed, or when the sentence about model weights is removed.
- `cargo xtask assets` fails when a file in `assets/` has no provenance entry below.

## Policy

Allowed: 0BSD, MIT, Apache-2.0 (with or without the LLVM exception), BSD-2-Clause, BSD-3-Clause, BSL-1.0,
IJG, ISC, MPL-2.0, OFL-1.1, Unicode-3.0, Unlicense, Zlib. Not allowed: GPL, AGPL, LGPL when statically
linked, SSPL, Commons Clause, "source available" licences, and anything without a licence. `deny.toml`
states the list and `cargo deny check` enforces all four categories (licences, advisories, bans, sources)
in CI; the allow-list there is the authority, and this table explains it rather than duplicating it.

Code copied from another application is never allowed, whatever its licence. Sortlens is written from
behaviour, not from source: `docs/TOOLING.md` records what was studied and what was deliberately not read.

## Where dependencies come from

Most crates come from crates.io. The Craft suite crates and the two GUI forks come from three pinned git
origins — `storytold/craft-libs`, `storytold/egui` and `storytold/winit` — all at one revision, recorded in
the workspace `Cargo.toml`. `deny.toml` allows exactly those three origins, so a fork or a patch branch
cannot appear without someone editing the policy on purpose.

## Linked today

These are the crates a manifest actually asks for and the build links.

| Crate | Licence | Used for |
|---|---|---|
| `craft-ui` | MIT OR Apache-2.0 | Suite chrome, docking, tokens, widgets |
| `craft-window` | MIT OR Apache-2.0 | Window policy, captions, title-bar insets |
| `craft-settings` | MIT OR Apache-2.0 | The settings document and its debounced store |
| `craft-metadata` | MIT OR Apache-2.0 | EXIF and XMP extraction |
| `craft-tiff` | MIT OR Apache-2.0 | TIFF and DNG decoding |
| `craft-jpeg` | MIT OR Apache-2.0 | JPEG decode and baseline encode |
| `craft-image-codecs` | MIT OR Apache-2.0 | PNG and other formats for previews |
| `craft-raw` | MIT OR Apache-2.0 | RAW containers and their embedded previews |
| `craft-heif` | MIT OR Apache-2.0 | HEIC decoding for thumbnails and container sizes during the scan |
| `craft-media` | MIT OR Apache-2.0 | The isobmff demuxer behind a video's length, frame rate, codec and poster frames |
| `craft-video` | MIT OR Apache-2.0 | The H.264/HEVC decoders behind a video's poster frame (first sync sample) |
| `craft-color` | MIT OR Apache-2.0 | The poster conversion's arithmetic: matrix, range, transfer and gamut |
| `craft-jsonrpc` | MIT OR Apache-2.0 | JSON-RPC framing for the MCP server |
| `craft-mcp` | MIT OR Apache-2.0 | MCP protocol types for `sortlens-mcp` |
| `egui` | MIT OR Apache-2.0 | Immediate-mode GUI (storytold fork) |
| `egui_extras` | MIT OR Apache-2.0 | The SVG image loader behind the shell's icon set; `svg` feature only, no HTTP or file loader is linked |
| `egui_kittest` | MIT OR Apache-2.0 | The screenshot harness (`cargo xtask screenshots`): the same fork's headless wgpu renderer, AccessKit queries and image snapshot diffing; `sortlens-ui` tests only, never linked into the app |
| `eframe` | MIT OR Apache-2.0 | The application shell and its renderer |
| `serde` | MIT OR Apache-2.0 | Op-log, snapshot and settings serialisation |
| `serde_json` | MIT OR Apache-2.0 | JSON records, the CLI's `--json`, MCP |
| `thiserror` | MIT OR Apache-2.0 | Error types |
| `unicode-normalization` | MIT OR Apache-2.0 | Unicode tables for `PathKey`: macOS writes filenames in NFD, a person types NFC, and one folder must be one key |
| `crc32fast` | MIT OR Apache-2.0 | One checksum per op-log record |
| `trash` | MIT | Moving rejects to the operating system's trash |
| `craft-atomic` | MIT OR Apache-2.0 | Crash-safe writes and Windows-safe renames: the model download lands by verified rename |
| `ureq` | MIT OR Apache-2.0 | The opt-in model downloader's HTTP(S) client, TLS via rustls; engine only, desktop only |
| `tract-onnx` | MIT OR Apache-2.0 | Pure-Rust ONNX inference for the pinned models; `sortlens-ai` only, desktop only, offline |
| `flate2` | MIT OR Apache-2.0 | Decompressing the shipped CLIP BPE vocabulary in `crates/ai/src/tokenize.rs`; desktop only, offline |
| `prost` | MIT OR Apache-2.0 | ONNX protobuf assembly for the synthetic models the inference tests run; `sortlens-testkit` and test code only |
| `rusqlite` | MIT | Read-only access to an optional MBTiles basemap (`bundled` SQLite, so no system library is assumed); `sortlens-geo` only, never written to |

`craft-tiff`, `craft-jpeg`, `craft-image-codecs` and `craft-raw` sit below the UI layer on purpose: they
decode pixels and know nothing about a window, so the CLI and the phone app can use them too.

## Declared, not linked yet

`[workspace.dependencies]` pins these so that every crate that eventually needs one uses the same version
and the same git revision. No manifest links them today, so **nothing here is compiled into the app** —
this table is a promise about versions, not a claim about features. Each row names the roadmap phase that
links it; a pin with no phase was pruned in Phase 0 (the reconciliation audit) rather than kept as a
promise nobody intends to keep.

| Crate | Licence | Why it is declared | Linked by |
|---|---|---|---|
| `rayon` | MIT OR Apache-2.0 | The low-priority compute pool for local inference | Phase 2 |
| `craft-keymap` | MIT OR Apache-2.0 | The rebinding editor, conflict detection and import/export | Phase 4 — **linked 2026-11-24**: `sortlens-ui` (the editor and dispatch) and `sortlens` (the persisted sets) |
| `craft-update` | MIT OR Apache-2.0 | Opt-in update checks (a user-approved network call) | Phase 4 |
| `craft-i18n` | MIT OR Apache-2.0 | Localisation scaffolding | Phase 4 |

## Fonts

`craft-fonts` is a **build input, not a dependency**: `crates/ui/build.rs` reads a clone at `CRAFT_FONTS_DIR`
and embeds what it finds, so a build with no clone is a supported configuration and simply uses egui's own
faces. Nothing from that repository is vendored here, and `.fonts/` is gitignored.

When a build embeds craft faces, they are appended to each family (never prepended — a fallback that
changes Latin text is a bug). The full manifest is about 48 MiB. `fonts/embedded.txt` records what the
last such build carried, and `cargo xtask fonts` reports the face set and refuses one that misses a
script the interface ships (its own two, or one a shipped translation is written in). A build may also
be *trimmed*: `CRAFT_FONTS_SCRIPTS` (a comma-separated script list) embeds only the faces the named
scripts need — the phone's build sets it to `Latn,Zsym,Arab,Jpan` through `cargo xtask mobile`, so a
phone binary does not carry families no shipped language uses, and a script the manifest has no face
for is refused by name before anything is embedded.

Faces that appear in that record, all under the SIL Open Font License 1.1, with the licence text travelling
in `craft-fonts` itself:

| Family | Face | Licence |
|---|---|---|
| BIZ UDPGothic | Regular and Bold | OFL-1.1 |
| NanumGothic | Regular | OFL-1.1 |
| Noto Sans Symbols | Regular | OFL-1.1 |
| Noto Sans Symbols 2 | Regular | OFL-1.1 |

The keymap draws `⌃ ⇧ ⌘ ✕ ◀ ✓` as text, which is why a symbols face matters: with one the keycaps are
the platform's own, and without one the shell spells what egui's bundled fonts lack as words
(`Ctrl+Shift+R`, `⌘Shift+R`) rather than painting boxes. A build with no craft faces ships egui's
bundled fonts instead, which are also permissively licensed. One corner is still not covered on a
Mac without craft faces: `craft-keymap`'s own editor dialog paints the platform's keycaps and so needs
that face, which is why the screenshot gate is a desktop-Linux/Windows gate today.

## Data and models

**No model weights, corpora or datasets are stored in this repository or bundled into the
application.** What ships is a *pin*: `sortlens-ai::manifest::Manifest::shipped()` names three
permissively licensed model files with their byte sizes and SHA-256 digests, and the opt-in
`model.install` command (a user-initiated network call, Phase 2) fetches them from a GitHub release
on this repository. A downloaded file is refused unless its size and digest match the pin, and a
test keeps this table and the manifest pinned against each other.

The pinned files are derivatives of upstream weights: the exporter's symbolic input shapes were
concretized (batch 1) and its shape-inference annotations stripped, because `tract-onnx` cannot type
those exports as published. Each derived file was verified to load and run under `tract-onnx` 0.23.10
before its digest was pinned. The transformations change no weights, and the upstream licences (MIT,
Apache-2.0) permit the derivative. The upstream files themselves were verified by digest against the
publishers' recorded hashes before the derivative was built.

| Model | Kind | Licence | Provenance | Bytes | SHA-256 | Pinned file |
|---|---|---|---|---|---|---|
| `yolos-tiny` | Detector | Apache-2.0 | [hustvl/yolos-tiny](https://huggingface.co/hustvl/yolos-tiny), fp16 ONNX export by [Xenova/yolos-tiny](https://huggingface.co/Xenova/yolos-tiny) | 13,251,235 | `f4d8c3ec0ad9a573d32c0c6646498baf8ea3d89e478a62f617bf6735cb4384fc` | [`yolos-tiny-fp16-static.onnx`](https://github.com/Topzic/sortlens-rust/releases/download/models-v1/yolos-tiny-fp16-static.onnx) |
| `clip-vit-b32-vision` | Image embedder | MIT | [openai/clip-vit-base-patch32](https://huggingface.co/openai/clip-vit-base-patch32) (MIT), int8 ONNX export by [Xenova/clip-vit-base-patch32](https://huggingface.co/Xenova/clip-vit-base-patch32) | 89,023,662 | `cd5bb3cba859097c0c8c1b5bb34f365d6e6583b47956eaa2626024da2acd8050` | [`clip-vit-b32-vision-int8-static.onnx`](https://github.com/Topzic/sortlens-rust/releases/download/models-v1/clip-vit-b32-vision-int8-static.onnx) |
| `clip-vit-b32-text` | Text embedder | MIT | [openai/clip-vit-base-patch32](https://huggingface.co/openai/clip-vit-base-patch32) (MIT), int8 ONNX export by [Xenova/clip-vit-base-patch32](https://huggingface.co/Xenova/clip-vit-base-patch32) | 64,390,561 | `8fba28dbcca353679e8d444e8f5114ba299c709b84b7e289b7698002bbbadc43` | [`clip-vit-b32-text-int8-static.onnx`](https://github.com/Topzic/sortlens-rust/releases/download/models-v1/clip-vit-b32-text-int8-static.onnx) |

I/O contracts, for the inference slice to code against: the detector takes `pixel_values`
1×3×512×800 fp32 (image-net mean/std normalisation) and answers `logits` 1×100×92 (100 queries,
COCO-91 classes plus the EOS slot, so the label map ships with that slice) and `pred_boxes` 1×100×4
(cxcywh, normalised); the vision embedder takes 1×3×224×224 fp32 and answers `image_embeds` 1×512;
the text embedder takes `input_ids` 1×77 int64 (CLIP BPE tokenisation, padded to 77) and answers
`text_embeds` 1×512 — the pair shares one embedding space, so pack phrases and photos compare
directly. The label map does ship: `crates/ai/src/coco91.txt`, compiled into the inference slice by
`include_str!` and attributed here. The BPE vocabulary the text side needs ships with the
pack-embedding slice, exactly as upstream publishes it:

| File | Origin | Licence | Bytes | SHA-256 |
|---|---|---|---|---|
| `crates/ai/src/bpe_simple_vocab_16e6.txt.gz` | [openai/CLIP](https://github.com/openai/CLIP/blob/main/clip/bpe_simple_vocab_16e6.txt.gz) (file [bpe_simple_vocab_16e6.txt.gz](https://github.com/openai/CLIP/raw/main/clip/bpe_simple_vocab_16e6.txt.gz)), byte-for-byte; `crates/ai/src/tokenize.rs` reads the first 48,894 merges, the slice CLIP itself trains on | MIT | 1,356,917 | `924691ac288e54409236115652ad4aa250f48203de50a9e4722a6ecd48d6804a` |

Keyword packs are files the user writes or installs; each pack carries its own `licence` field, which
the app shows beside it.

The test fixtures are generated at run time by `sortlens-testkit` from solid colours through a real JPEG
encoder: no photographs are stored, so no photograph's licence needs recording.

## Assets

| File | Origin | Licence |
|---|---|---|
| `assets/icon.svg` | Drawn for this project in `crates/icon/src/lib.rs`; the file is a rendering of that geometry, and a test fails when the two disagree | MIT OR Apache-2.0 |
| `assets/icon.ico`, `assets/icon.icns` | Generated by `sortlens-icon::ico()`/`icns()` from the same drawing — PNG entries packed per format; `cargo xtask icons --update` rewrites them, and tests hold the committed bytes to the drawing | MIT OR Apache-2.0 |
| `assets/icons/*.svg` | Drawn for this project as the shell's monochrome icon set: 24-unit stroke drawings (`currentColor`, 2-unit stroke) that `craft_ui::icons` tints with the active theme; a test fails when a glyph has a hard-coded colour, and another when the set and `crate::icons::Icon` disagree | MIT OR Apache-2.0 |
| `assets/coastline.txt` | **Natural Earth 1:110m coastline**, the `ne_110m_coastline.geojson` layer from [Natural Earth](https://www.naturalearthdata.com/) (nvkelso/natural-earth-vector). Derived for this project by rounding every coordinate to three decimals and writing one polyline per line as `lat,lon` pairs; the file's own header records the source and the transform, and `sortlens_geo::coastline` embeds it. The credit line `Coastline: Natural Earth 1:110m (public domain)` is printed wherever the outline is drawn | Public domain (no permission needed; credit given) |

The window icon is not committed: `apps/sortlens/build.rs` renders the same drawing to RGBA in `OUT_DIR`,
so there is no binary blob in the repository and no second copy that could disagree with the first.

## Product naming

"Sortlens" is the working name of this recreation and is not claimed as a trademark. If the project is
published under a different name, this file and the app id (`app.sortlens.desktop`) change together.
