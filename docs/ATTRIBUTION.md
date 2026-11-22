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
| `egui` | MIT OR Apache-2.0 | Immediate-mode GUI (storytold fork) |
| `eframe` | MIT OR Apache-2.0 | The application shell and its renderer |
| `serde` | MIT OR Apache-2.0 | Op-log, snapshot and settings serialisation |
| `serde_json` | MIT OR Apache-2.0 | JSON records, the CLI's `--json`, MCP |
| `thiserror` | MIT OR Apache-2.0 | Error types |
| `crc32fast` | MIT OR Apache-2.0 | One checksum per op-log record |
| `trash` | MIT | Moving rejects to the operating system's trash |

`craft-tiff`, `craft-jpeg`, `craft-image-codecs` and `craft-raw` sit below the UI layer on purpose: they
decode pixels and know nothing about a window, so the CLI and the wasm build can use them too.

## Declared, not linked yet

`[workspace.dependencies]` pins these so that every crate that eventually needs one uses the same version
and the same git revision. No manifest links them today, so **nothing here is compiled into the app** —
this table is a promise about versions, not a claim about features.

| Crate | Licence | Why it is declared |
|---|---|---|
| `craft-atomic` | MIT OR Apache-2.0 | Atomic file replacement for the snapshot rollover |
| `craft-flags` | MIT OR Apache-2.0 | Reserved; keyword-pack switches went into the op log instead |
| `craft-i18n` | MIT OR Apache-2.0 | Translations, which are not started |
| `craft-fonts-build` | MIT OR Apache-2.0 | Reserved; `crates/ui/build.rs` reads `craft-fonts` directly and dependency-free |
| `craft-dialogs` | MIT OR Apache-2.0 | Reserved; the app uses `rfd` for folder pickers today |
| `craft-menu` | MIT OR Apache-2.0 | Reserved; menus are drawn by `craft-ui` |
| `craft-keymap` | MIT OR Apache-2.0 | Reserved for the rebinding editor |
| `craft-quit` | MIT OR Apache-2.0 | Reserved; the quit guard is written by hand |
| `craft-update` | MIT OR Apache-2.0 | In-app updates, which need a release feed |
| `craft-heif` | MIT OR Apache-2.0 | HEIC decoding, which is not built |
| `craft-webp` | MIT OR Apache-2.0 | WebP decoding and export, neither built |
| `craft-gainmap` | MIT OR Apache-2.0 | HDR gain maps, not built |
| `craft-video` | MIT OR Apache-2.0 | Video posters, not built |
| `craft-media` | MIT OR Apache-2.0 | Reserved; `sortlens-core::media` covers the extension table |
| `craft-crs` | MIT OR Apache-2.0 | Develop-setting detection for "edited" state |
| `craft-sniff` | MIT OR Apache-2.0 | Magic-byte sniffing, which is not built |
| `craft-zip` | MIT OR Apache-2.0 | A zip-based diagnostics bundle; the bundle is plain text |
| `craft-ipc` | MIT OR Apache-2.0 | Reserved; agent access is MCP over stdio instead |
| `craft-jsonrpc` | MIT OR Apache-2.0 | Reserved; `sortlens-mcp` speaks the protocol itself |
| `craft-mcp` | MIT OR Apache-2.0 | Reserved; `sortlens-mcp` is hand-written against the command registry |
| `craft-web-api` | MIT OR Apache-2.0 | The remote client, which is absent by design |
| `craft-web-host` | MIT OR Apache-2.0 | The remote host, which is absent by design |
| `log` | MIT OR Apache-2.0 | Declared for a log sink; the app reports through jobs and the change log |
| `chrono` | MIT OR Apache-2.0 | Declared for date formatting; capture dates are Unix seconds today |
| `rayon` | MIT OR Apache-2.0 | Declared for parallel scans; the scanner uses its own worker pool |

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
| `assets/icon.svg` | Created for this project | MIT OR Apache-2.0 |

## Product naming

"Sortlens" is the working name of this recreation and is not claimed as a trademark. If the project is
published under a different name, this file and the app id (`app.sortlens.desktop`) change together.
