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
| `eframe` | MIT OR Apache-2.0 | The application shell and its renderer |
| `serde` | MIT OR Apache-2.0 | Op-log, snapshot and settings serialisation |
| `serde_json` | MIT OR Apache-2.0 | JSON records, the CLI's `--json`, MCP |
| `thiserror` | MIT OR Apache-2.0 | Error types |
| `unicode-normalization` | MIT OR Apache-2.0 | Unicode tables for `PathKey`: macOS writes filenames in NFD, a person types NFC, and one folder must be one key |
| `crc32fast` | MIT OR Apache-2.0 | One checksum per op-log record |
| `trash` | MIT | Moving rejects to the operating system's trash |

`craft-tiff`, `craft-jpeg`, `craft-image-codecs` and `craft-raw` sit below the UI layer on purpose: they
decode pixels and know nothing about a window, so the CLI and the wasm build can use them too.

## Declared, not linked yet

`[workspace.dependencies]` pins these so that every crate that eventually needs one uses the same version
and the same git revision. No manifest links them today, so **nothing here is compiled into the app** —
this table is a promise about versions, not a claim about features. Each row names the roadmap phase that
links it; a pin with no phase was pruned in Phase 0 (the reconciliation audit) rather than kept as a
promise nobody intends to keep.

| Crate | Licence | Why it is declared | Linked by |
|---|---|---|---|
| `craft-atomic` | MIT OR Apache-2.0 | Atomic file replacement for the resumable model downloader | Phase 2 |
| `rayon` | MIT OR Apache-2.0 | The low-priority compute pool for local inference | Phase 2 |
| `craft-keymap` | MIT OR Apache-2.0 | The rebinding editor, conflict detection and import/export | Phase 4 |
| `craft-update` | MIT OR Apache-2.0 | Opt-in update checks (a user-approved network call) | Phase 4 |
| `craft-i18n` | MIT OR Apache-2.0 | Localisation scaffolding | Phase 4 |
| `craft-web-api` | MIT OR Apache-2.0 | The remote client | Phase 4 |
| `craft-web-host` | MIT OR Apache-2.0 | The remote host | Phase 4 |

## Fonts

`craft-fonts` is a **build input, not a dependency**: `crates/ui/build.rs` reads a clone at `CRAFT_FONTS_DIR`
and embeds what it finds, so a build with no clone is a supported configuration and simply uses egui's own
faces. Nothing from that repository is vendored here, and `.fonts/` is gitignored.

When a build embeds craft faces, they are appended to each family (never prepended — a fallback that
changes Latin text is a bug). The web build keeps a five-face allowlist because the full manifest is about
48 MiB. `fonts/embedded.txt` records what the last such build carried, and `cargo xtask fonts` reports the
face set and refuses one that covers no Latin text.

Faces that appear in that record, all under the SIL Open Font License 1.1, with the licence text travelling
in `craft-fonts` itself:

| Family | Face | Licence |
|---|---|---|
| BIZ UDPGothic | Regular and Bold | OFL-1.1 |
| NanumGothic | Regular | OFL-1.1 |
| Noto Sans Symbols | Regular | OFL-1.1 |
| Noto Sans Symbols 2 | Regular | OFL-1.1 |

The keymap draws `⌃ ✕ ◀ ✓` as text, which is why a symbols face is required rather than optional. A build
with no craft faces ships egui's bundled fonts instead, which are also permissively licensed.

## Data and models

**No model weights, corpora or datasets ship with this project.** The AI half is built and tested — a
digest-verified model store, non-maximum suppression, label maps, EXIF hint rules, keyword packs — but no
model is pinned, so there is nothing to attribute and no download that could be attributed later without
editing this section. Keyword packs are files the user writes or installs; each pack carries its own
`licence` field, which the app shows beside it.

The test fixtures are generated at run time by `sortlens-testkit` from solid colours through a real JPEG
encoder: no photographs are stored, so no photograph's licence needs recording.

## Assets

| File | Origin | Licence |
|---|---|---|
| `assets/icon.svg` | Drawn for this project in `crates/icon/src/lib.rs`; the file is a rendering of that geometry, and a test fails when the two disagree | MIT OR Apache-2.0 |

The window icon is not committed: `apps/sortlens/build.rs` renders the same drawing to RGBA in `OUT_DIR`,
so there is no binary blob in the repository and no second copy that could disagree with the first.

## Product naming

"Sortlens" is the working name of this recreation and is not claimed as a trademark. If the project is
published under a different name, this file and the app id (`app.sortlens.desktop`) change together.
