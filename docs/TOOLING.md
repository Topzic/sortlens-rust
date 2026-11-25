# Tooling audit — what we build on

The user asked for a deep analysis of the tools available. This is that audit: what each one
gives us, what it costs us, what we must do to use it correctly, and what we should not use.

Verdict in one line: **ArtCraft's stack is a near-perfect fit for Sortlens.** It already ships
the photo decode/metadata layer, the UI shell, the theme and font infrastructure, the settings
and keymap crates, the automation surface and the packaging model. Sortlens needs to supply the
library/culling domain (which does not exist there) and the ArtCraft shell around it.

---

## 1. getartcraft.com — the distribution and product model

Observed (site, 2026):

- "ArtCraft builds open source tools, free to use." Launcher-first: **Download Launcher**, the
  launcher installs/updates the individual apps.
- Craft Apps: PhotoCraft, VectorCraft, FilmCraft, LightCraft, PdfCraft, EffectCraft, AniCraft,
  DesignCraft, FigCraft, TextureCraft, MaterialCraft, SoundCraft, WordCraft, DeckCraft,
  CADCraft, GridCraft, Craft Anything.
- Per-app pages with a one-line promise (LightCraft: "Your photos. Your pixels. Your machine."),
  a screenshot, and platform matrix: **macOS · Windows · Linux · free and open source**.
- Community: Discord, wiki, tutorials, news, FAQ, press kit, support.
- Separate commercial product, ArtCraft Studio, for AI image/video — explicitly *not* part of
  the free suite.

What this means for Sortlens:

| Implication | Action |
| --- | --- |
| Apps are installed and updated by the launcher | Implement `craft-update`'s app descriptor and a launcher entry; do not ship a bespoke updater |
| Positioning is "capable, local, free, no subscription" | Our README and About must make the same three claims Sortlens makes: local, no account, no telemetry |
| Every app has a one-line promise and a screenshot | Marketing copy is part of the deliverable, not an afterthought |
| Suites share shortcuts, menus and look | Consistency beats invention; use suite chrome and only diverge where culling needs it |
| A culling app is missing from the suite | Sortlens fills a real gap (LightCraft edits; nothing in the suite is built for 5,000-frame culls) |
| Licence expectations are MIT/Apache-2.0 | Matches `craft-libs`; keep every dependency on the allowed list |

**Naming.** The user owns Sortlens, so the name and feature names can carry over. If the app
should sit inside the ArtCraft brand family, `SortCraft` is the natural sibling label and
`sortlens` stays the repository/binary name. Decision: keep **Sortlens** as the product name,
crates prefixed `sortlens-`, reverse-DNS id `app.sortlens.desktop`.

---

## 2. storytold/craft-libs — the foundation

A ~120-crate Rust workspace (`edition 2024`, `rust-version 1.95`, `egui 0.37`, MIT OR
Apache-2.0, `unsafe` forbidden, `copyright`-clean with per-crate READMEs and clean-room notes).
Downstream apps pin **one `rev`** of `main` for every craft crate they use, and move that rev in
its own PR. That convention matters: mixed revs of craft crates produce duplicate `egui` types
and incomprehensible trait errors.

### 2.1 What we use, and why

| Crate | What it gives Sortlens | Phase | Notes |
| --- | --- | --- | --- |
| `craft-metadata` | EXIF/IPTC/XMP/GPX read+write, maker notes, one `Metadata` model, per-container embedding (JPEG, PNG, WebP, TIFF-based, RAF, CR3) | M1–M3 | The single most valuable crate for us. Replaces a whole metadata subsystem |
| `craft-tiff` | TIFF/BigTIFF IFD reader/writer, SubIFDs, maker-note IFDs, strip/tile layout | M1 | Foundation of RAW metadata; bounded on hostile input |
| `craft-raw` | Camera RAW containers → neutral raw + embedded previews; one feature per format (NEF, CR2, CR3, ARW, RAF, ORF, RW2, DNG, PEF, SRW, 3FR…) | M2 | We use the *embedded preview* path for culling; demosaic stays in LightCraft |
| `craft-jpeg` | Clean-room 8/12-bit JPEG decode, baseline encode, frame probe | M2 | Fast path for grid thumbs |
| `craft-heif` | HEIC/HEIF still decode → 16-bit RGB, orientation, alpha, EXIF/XMP, thumbnails | M2 | Optional feature; never panics |
| `craft-image-codecs` | Shared image codec surface used by LightCraft | M2 | Umbrella for JPEG/PNG/WebP/GIF/TGA/etc. |
| `craft-webp`, `craft-gif`, `craft-tga`, `craft-tiff` | Additional raster paths | M2 | Needed for the format table in the README |
| `craft-video` | H.264/HEVC/VP8/VP9/AV1 decode + encode, MPEG-2, ProRes, DNx | M2, M9 | Poster frames in M2; playback behind a flag later |
| `craft-media` | MP4/MOV, Matroska/WebM, MXF, AVI, MPEG-TS demux; ISO BMFF box reader | M2 | Duration/fps/codec for the details panel |
| `craft-audio` | WAV/AIFF/FLAC/AAC/Opus/MP3 decode + encode | M9 | Only for video playback |
| `craft-gainmap` | HDR gain maps (ISO 21496-1 + `hdrgm`) | P2 | If HDR photo previews are in scope |
| `craft-crs` | Coordinate reference systems / projections | M6 | Map projection correctness without hand-rolled math |
| `craft-svg`, `craft-svg-paths` | SVG parse/render helpers | M0 | Only if icons need runtime SVG beyond `craft-ui::icons` |
| `craft-ui` | Icon catalogs, icon buttons, tabs, splits, resize handles, toolkit fitting, docking, `WidgetPalette`, font stack, localized number fields, screenshot/`kittest` helpers | M0+ | The UI foundation. Read `docs/ui-docking.md`, `docs/ui-panes.md`, `docs/ui-components-audit.md`, `docs/theme-infrastructure.md` |
| `craft-window` | Title-bar policy per desktop, integrated macOS traffic lights, window restore, multi-window list | M0 | Gives the ArtCraft window behaviour for free |
| `craft-settings` | `config.toml`, atomic saves, backups, damaged-file handling, schema migrations | M0 | One settings type per app; we supply defaults + migrations |
| `craft-atomic` | Atomic file replace primitives | M1 | Snapshots, XMP writes, settings |
| `craft-keymap` | Chords, scopes, conflicts, overrides, import of other apps' shortcut sets, editor model + shared dialog | M4, M9 | Our command catalog feeds it |
| `craft-menu` | macOS menu bar model + platform backend | M4 | Keeps native menus and shortcuts in sync |
| `craft-dialogs` | Non-blocking native file/message dialogs | M1 | Avoids the AppKit re-entrancy crash the vendor's webview never had to face |
| `craft-quit` | Quit policy/confirmation | M4 | "Jobs running / unsaved" guard |
| `craft-update` | Update checks, release notes UI, suite app metadata | M9 | Replaces Inno Setup + PowerShell updater |
| `craft-flags` | Suite feature flags (`AI-MCP`, `AI-MODELS`), off by default | M0, M8 | Perfect for gating AI and remote |
| `craft-i18n` | Embedded TSV catalogs, fallback, plurals, language names, system locale | M9 | English keys in code |
| `craft-fonts-build` + `craft-fonts` | Embed or serve the suite's OFL faces, subsetting, coverage checks | M0 | `CRAFT_FONTS_DIR`; `CRAFT_FONTS_REQUIRED=1` in releases |
| `craft-ext-core` / `craft-ext-store` | Theme/plug-in package contract, palette with WCAG invariants, bounded unpacking, atomic install, per-app enable | M10 | Status is *provisional*; themes first, code later |
| `craft-ipc` | Per-user local sockets, token auth, instance discovery, launch-on-demand, frame transfer | M4, M9 | Control channel and launcher hand-off |
| `craft-jsonrpc` | JSON-RPC message parsing/batch dispatch | M4 | Pairs with `craft-ipc` and `craft-mcp` |
| `craft-mcp` | MCP 2025-03-26 server init + tool result helpers | M9 | We own tools, transport and roots |
| `craft-ai-core` | AI connection/model/capability contract, `SecretRef`, no I/O | M8 (optional) | Only if we ever add BYO cloud AI; local inference does not need it |
| `craft-sniff` / `craft-file-formats` | Format sniffing helpers | M1 | Double-check extension with magic bytes |
| `craft-zip` | Bounded ZIP read/write | M7 | Backup archives, imports |
| `craft-time` | Rational ticks/SMPTE time | – | Not needed (photo/video timestamps go through `chrono`) |
| `craft-codepage` | Legacy code pages | P2 | Filename/EXIF text edge cases |
| `craft-mime` | MIME + charset handling | P2 | Imported sidecar/HTML metadata |
| `craft-testkit` / `plate-test` | Test helpers, plate/image comparison | M2+ | Useful for thumbnail visual checks |

### 2.2 What we deliberately do not use

| Crate | Reason |
| --- | --- |
| `craft-develop`, `craft-pipeline` | That is LightCraft's editing engine. Sortlens culls; it does not develop. Adding it would double the dependency surface and blur the product |
| `craft-pdf`, `craft-pptx`, `craft-docx`, … | Out of scope |
| LightCraft's own app crates (`lightcraft-catalog`, `lightcraft-engine`, …) | AGENTS.md §9: apps must not depend on another app's crates. Read them for design precedent, do not link them |
| `craft-sqlite` | Wire-format helpers only; we are not implementing a SQL database |
| `vendor/*` (krilla, hayro, vello) | PDF/vector machinery, irrelevant |

### 2.3 Integration shape

```toml
# workspace Cargo.toml
[workspace]
resolver = "3"
members = ["crates/*", "apps/*", "xtask"]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT OR Apache-2.0"
rust-version = "1.95"
repository = "https://github.com/Topzic/sortlens-rust"

# One rev for every craft crate. Move it in its own PR. (rev omitted here; pick the merge on main)
[workspace.dependencies]
craft-ui        = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>", features = ["serde"] }
craft-window    = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>", features = ["egui"] }
craft-settings  = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-metadata  = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-tiff      = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-jpeg      = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-raw       = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-video     = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-media     = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>", default-features = false }
craft-keymap    = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>", features = ["ui"] }
craft-menu      = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-dialogs   = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-update    = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>", features = ["egui"] }
craft-flags     = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-i18n      = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-fonts-build = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-mcp       = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-ipc       = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-jsonrpc   = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-sniff     = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
craft-zip       = { git = "https://github.com/storytold/craft-libs", rev = "<PIN>" }
```

```toml
# .cargo/config.toml — the suite patches the egui family to a reviewed fork.
[patch.crates-io]
egui   = { git = "https://github.com/storytold/egui.git", rev = "<EGUI_PIN>" }
eframe = { git = "https://github.com/storytold/egui.git", rev = "<EGUI_PIN>" }
# …ecolor, emath, epaint, egui-winit, egui-wgpu, egui_extras, egui_kittest, winit
```

Both pins are copied from the `Cargo.toml` of a sibling app (LightCraft is the closest) at the
moment we start, so the whole suite is on identical versions. A `pnpm`-style lock for Rust does
not exist; `Cargo.lock` is committed and reviewed when the pin moves.

### 2.4 Risks in craft-libs

| Risk | Mitigation |
| --- | --- |
| Rev pin drifts from the rest of the suite | One pin, moved in an isolated PR, CI re-runs the full suite |
| `craft-ui` docking API is young and has non-accessible hit targets (the README says so for resize corners) | We wrap it: our own minimum 24 px resize targets and keyboard alternatives; visual regression tests |
| `craft-extensions` is explicitly provisional ("API will change", "no app ships it yet") | Ship themes as our own presets in v1; treat package install as an M10 feature behind a flag |
| Many crates are unpublished (`rev`-only) | Vendor nothing; pin rev; `cargo xtask` prints the resolved revs so a build is reproducible |
| `craft-ai` is a spec, not an implementation | Do local inference ourselves (`tract`); use `craft-ai-core` types only if a cloud path is added |
| Large feature matrix in `craft-video`/`craft-raw` can bloat build time | Enable only the formats in the support table; `cargo xtask` reports binary size and build time |

---

## 3. storytold/craft-fonts — typography

- One directory per family with the licence, plus machine-readable manifests:
  `fonts/manifest.txt` (default), `latin-manifest.txt` (opt-in presentation faces),
  `simplified-chinese-manifest.txt` (native-only).
- Default payload covers Japanese, Korean, Simplified Chinese, Arabic, Thai, Georgian, Hebrew,
  Devanagari, Bengali, Gurmukhi, Gujarati, Oriya, Tamil, Telugu, Kannada, Malayalam, symbols
  and monochrome emoji.
- `crates/craft-fonts` + `craft-fonts-build`:
  - `Embed::new().subset_to(catalogs)` cuts CJK faces to the characters our catalogs use
    (~3 MB for five languages instead of tens of MB);
  - `CRAFT_FONTS_DIR` points at a checkout; `CRAFT_FONTS_REQUIRED=1` makes a release fail
    without fonts.
- `craft_ui::fonts::UiFonts` assembles the fallback chain: app face → UI-language script →
  other shipped scripts → system fonts → emoji.
- `fonts::assert_covers` is the guard that prevents "boxes instead of Thai filenames".

Our plan: Inter Regular/SemiBold as the app face (as LightCraft does), craft-fonts for every
other script, system-font provider enabled on desktop, a coverage test per
shipped language, and the credits list generated from the font manifest into
`docs/ATTRIBUTION.md`.

Why this matters for a culling app specifically: photo filenames are user data, and they are
frequently Arabic, Hebrew, Thai, Japanese or Chinese. A culling tool that renders `?????.JPG`
is broken, not unpolished.

---

## 4. storytold/craft-extensions — themes, plug-ins, store

Status: **provisional and early**; no app ships it; six architecture decisions are settled but
signatures, catalogue client, revocation and the native host are not built.

Two crates today:

- `craft-ext-core` (pure): package manifest, ids, paths, trust tiers, theme palette
  with WCAG contrast validation, catalogue index schema. Denies panics; 27 deliberate breakages
  are caught by a mutation script.
- `craft-ext-store` (native): bounded unpacking, atomic install, enable-per-app, read side.
  Never touches the network.

Design rules worth copying into our own extension story:

1. A package never states its own trust level; the host assigns it from provenance.
2. Themes carry no code.
3. Everything read from a package is hostile input: sizes capped before parsing, ids checked by
   character class, archive unpacking limited by count/size/ratio, no path escapes.
4. Install once, enable per app; a running app notices an install without restart.
5. Ranking without telemetry (GitHub stars fetched centrally, no download tracking).

Plan: v1 ships built-in themes plus a "theme file" import using `craft-ext-core`'s palette →
our tokens (data only, no code); the full store wait until the upstream API stops being
provisional. Any plug-in that could run code is out of scope until the permission families and
sandbox runtime land upstream.

---

## 5. rustup and the toolchain

The user supplied the standard installer:
`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`.

What we actually need on each development machine and in CI:

```sh
# 1. Toolchain, pinned by rust-toolchain.toml (craft-libs requires 1.95+)
rustup toolchain install 1.95.0 --profile minimal \
  --component rustfmt clippy \
  --target aarch64-linux-android aarch64-apple-ios
rustup default 1.95.0

# 2. Cargo tools used by the repo (all optional for building the app)
cargo install --locked cargo-nextest cargo-deny cargo-mutants cargo-binstall
# optional: sccache for faster rebuilds of the decode crates

# 3. Platform prerequisites
# macOS:   xcode-select --install     (linker, SDK, codesign)
# Windows: MSVC build tools + Windows SDK
# Android: the NDK (ANDROID_NDK_HOME) — `ring` and `tract-linalg` compile C and assembly
# iOS:     full Xcode (the iPhoneOS SDK), not just the command line tools
# Linux:   build-essential pkg-config libxkbcommon-dev libwayland-dev \
#          libx11-dev libxrandr-dev libxi-dev libxcursor-dev libegl1-mesa-dev
```

`rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.95.0"
components = ["rustfmt", "clippy"]
targets = ["aarch64-linux-android", "aarch64-apple-ios"]
profile = "minimal"
```

Notes and gotchas:

- **Do not use `stable`.** `craft-libs` is `edition 2024` with `rust-version 1.95`; a CI runner
  with an older stable fails on syntax before it fails on a real problem.
- **The phone targets from day one.** The phone app reuses the engine and UI; adding the targets
  later tends to surface non-portable code (threads, file paths, `std::time`) late and cheaply
  only if the check runs in CI from M0.
- **`rustup` installs into `$HOME`,** not the project. On a sandboxed machine, run the installer
  yourself (or via your normal shell) before the first `cargo` command; the agent's project-scoped
  tool permissions may refuse writes outside the workspace.
- **Sandbox/CI caching:** `~/.cargo/registry`, `~/.cargo/git`, `target/`, and the craft-libs git
  checkout dominate build time. Cache `~/.cargo/git` keyed on the pinned rev.
- **`cargo-deny`** enforces `deny.toml` via `cargo xtask deny`, in its own CI job. Allow-listed:
  0BSD, Apache-2.0 (and with LLVM-exception), BSD-2/3-Clause, BSL-1.0, IJG, ISC, MIT, MPL-2.0,
  OFL-1.1, Unicode-3.0, Unlicense, Zlib, plus per-crate exceptions for `unicode-ident` and
  `epaint_default_fonts`. Denied by omission: GPL, AGPL, LGPL, CC-BY-NC. Advisories, yanked crates,
  wildcard version requirements and unknown git sources fail the same job. Install with
  `cargo install cargo-deny --locked`; the task says so rather than passing silently.
- **The documents are checked, not trusted**: `cargo xtask docs` (in `ci`) computes the shipped milestone
  range from `docs/ROADMAP.md` and requires it in both `README.md` (prose and status badge) and
  `docs/PLAN.md` §12, requires every crate and app in the workspace to be named in the layer diagram, and
  requires every `docs/*.md` to be reachable from the README. It checks facts that have one right answer,
  not how they are phrased.
- **The GitHub intake is checked with the code**: `cargo xtask templates` (in `ci`) reads the two issue
  forms through a deliberately small YAML reader — block mappings, block sequences, scalars and `|`
  blocks; flow collections, folded scalars, anchors and tab indentation are refused with their line —
  and then checks the shape GitHub renders: the keys an issue form carries, the element types it knows,
  an id on every field, a label on everything a person fills in, options where a dropdown or checkbox
  list needs them, at least one required field, and that the bug form asks for
  `sortlens-cli diagnostics`. The pull-request template is checked for the markers a reviewer needs:
  the linked issue, the failing-then-passing test, every gate as a checkbox, the scan/catalog/thumbs
  `cargo xtask perf` table and the `crates/ui` before-and-after. It checks facts, not phrasing.
- **The workflows and the autonomous-agent contract are checked too**: `cargo xtask workflows` (in `ci`)
  reads `.github/workflows/ci.yml`, `.github/workflows/agent-intake.yml` and `docs/AUTONOMOUS_AGENT.md`.
  It fails when a gate command disappears from `ci.yml`, when a gate gets `continue-on-error` or an `if`
  that leaves pull requests out, when the intake stops running on filed issues, reaches a repository
  secret, loses its issue-only guard or its comment-and-relabel step, and when the playbook loses one of
  the rules it is the only copy of. `cargo xtask intake` is the checker the intake workflow runs: given
  `SORTLENS_ISSUE_BODY` and `SORTLENS_ISSUE_LABELS`, it reads the required headings from the form itself
  and names each one that is missing or empty. The workflow files are written in the block style that
  reader accepts — a flow list such as `branches: [main]` is refused, because a check that cannot read
  its own workflow is not a check.
- **Measurements, not budgets, in `perf`**: `cargo xtask perf` runs `apps/sortlens-perf` in release mode
  and prints one table — scan, index, cold and warm thumbnails, the worker service, the suggestion pass,
  library open, headless grid frames. It is deliberately not in `ci`: a timing that fails on a loaded box
  gets ignored, and a timing that passes there says nothing about a library on a laptop. Numbers from a
  run, with the machine's name next to them, live in `docs/PERF.md`.
- **Golden outputs for published text, `egui_kittest` for painted pixels**: `cargo xtask snapshot`
  diffs `apps/sortlens-cli/tests/golden/*.txt` (read-only in CI, `--update` for an intended change), because
  the recorded artifact there is the tool's published output, not an internal value, and a plain text file
  in the tree is reviewable in a diff. The one snapshot crate is `egui_kittest`, behind
  `cargo xtask screenshots`: a painted frame is not a value a text golden can carry, so 18 PNG baselines of
  the real shell are committed and the image diff is the review (`docs/ARCHITECTURE.md` §12). `insta` is
  still not used — there is no internal value snapshot to record.
- **`cargo-nextest`** for the suite's test profile; `cargo test` remains supported.
- **`cargo-mutants`** on `sortlens-catalog` and `sortlens-cull` only, nightly job.
- **No `sccache` in release builds** (LTO + one codegen unit); it helps dev loops only.

---

## 6. Topzic/Sortlens — what the analysed repository actually contains

| Observation | Consequence |
| --- | --- |
| Public repo has README + `docs/screenshots/` only; no source | We cannot and must not copy code. Behaviour spec + clean-room reimplementation is the only path, and it is also the fastest |
| 33 tagged releases with detailed notes (v0.1 → v0.11) | A free, high-quality changelog of real bug classes: stale tag fetches, FK errors on apply, folder counts drifting, port discovery, updater failures. Each maps to a design decision in `ARCHITECTURE.md` |
| Version history lives on Releases, not a CHANGELOG file | Don't reproduce vendor release text; write our own notes from our own commits |
| Installer bundles VC++ redist + WebView2; macOS build unsigned | Our build has no runtime prerequisites and is signed/notarised |
| Library DB is a single `~/.sortlens/sortlens.db`; audit log at `~/.sortlens/audit.log` | We keep the same *promises* (one portable library folder, an audit log) with a different, replayable format |
| Screenshots show a blue/dark Tailwind-ish chrome, full-height sidebar, dense filter row | We keep the information architecture (sources/tools/folders/collections + filter bar) but restyle completely with ArtCraft tokens |
| Supported formats table (JPEG, PNG, WebP, TIFF, HEIC, NEF, CR2/CR3, ARW, RAF, ORF, RW2, DNG) | `craft-raw` + `craft-heif` cover this; encode it as a feature matrix test |
| "AI runs locally, CPU only, no GPU required", CLIP model ~150 MB in README vs 580 MB installed in the screenshot | Model sizes change; the UI must read the real size from the model manifest, never hardcode |
| iOS and Android apps exist in the vendor's product (not in the public repo) | In scope as `apps/sortlens-mobile`: the Rust engine and egui shell on `winit`'s Android and iOS backends; nothing of the vendor's code is used |
| Proprietary licence | Confirm with the owner (the user) before publishing a reimplementation; the repo's own README should state the licence of *this* code (MIT OR Apache-2.0 unless told otherwise) |

---

## 7. Dependency policy

**In:** `serde`, `serde_json`, `thiserror`, `log`, `chrono`, `rayon`, `bytemuck`, `crc32fast`,
`roaring`, `tract-onnx` (AI, optional), `jpeg-encoder`, `miniz_oxide`, `rfd` (only if
`craft-dialogs` needs a fallback), `web-time`,
`egui`/`eframe`/`egui_kittest`/`egui_extras` (patched to the suite fork).

**Out:** any GPL/AGPL/LGPL crate, any CC-BY-NC model or data, `image` (we use craft decoders),
`ort`/ONNX Runtime unless a benchmark justifies an explicit, documented exception, `opencv`,
`libraw` bindings, Tauri/Electron/webview, `sqlx`/`diesel` (no SQL database), any crate whose
licence is not on the suite's allowed list.

Every new dependency needs: licence checked, size/build-time impact measured, a one-line reason
in `Cargo.toml`, and (if it parses untrusted bytes) a hostile-input test.

---

## 8. Tooling checklist for Milestone 0

- [ ] `rust-toolchain.toml` pinned to 1.95.0 + phone targets
- [ ] Workspace skeleton with the 19 library crates and 4 app crates from `ARCHITECTURE.md` §2; `[lints] workspace = true`
- [ ] `#![forbid(unsafe_code)]` and the panic-free lint set in every crate root
- [ ] Craft rev pinned in `Cargo.toml`, egui fork pinned in `.cargo/config.toml`, `Cargo.lock` committed
- [ ] `xtask` with `ci --quick`, `ci`, `layers`, `roots`, `assets`, `fonts`, `mobile`, `bench`
- [ ] Window opens with ArtCraft chrome, 4 themes, Inter + craft-fonts, integrated macOS titlebar
- [ ] `craft-settings` wired with a versioned settings type and a damaged-file test
- [ ] Empty-library state with working commands: add folder, settings, shortcuts, about
- [ ] `docs/ATTRIBUTION.md` generated from the font manifest + crate licences
- [ ] CI: fmt, clippy `-D warnings`, tests, phone-target check, Windows leg, snapshot job
- [ ] A one-photo smoke path: add folder → index → thumbnail → detail view

## Scripting the CLI

Every read verb answers the same question in JSON that it answers in prose when `--json` is given, and
the flag may sit anywhere on the line:

```sh
sortlens-cli stats --json
sortlens-cli --json list --limit 50 --sort date
```

The shape is the schema: the keys are the contract, and the golden files under
`apps/sortlens-cli/tests/golden/` pin every verb that has one plus the refusal below, so a rename is a
visible diff in review rather than a surprise in someone's script. A verb without a schema refuses instead
of printing prose into a JSON stream, and names every verb that does have one (the full list is always
`sortlens-cli --json schemas`):

```text
sortlens-cli: add has no JSON schema yet; --json works for: roots, keywords, stats, list, page, …
```

A verb's JSON form reads the flags its plain form reads — `list --json --tag coast` is the filtered page,
not the whole library. MCP is the same registry over stdio, so anything an agent can do there, `--json`
can do from a shell — and `batchsuggest` deliberately has no schema, because it is a pass with progress
and a cancel whose proposals are read from `suggest`, the same way `export` refuses and reports through
`jobs`.

## Reporting a bug

`sortlens-cli diagnostics` prints a report — or writes it with `--out FILE` — meant to be attached to a
bug report. It carries the versions, the library counts, the result of the read-only health check, the
installed keyword packs, the last 25 changes and the number of roots.

Absolute paths are replaced before anything is printed or written: each indexed root becomes `<root-N>`,
the library folder `<library>`, and the home folder `<home>`. Longest match first, so a root inside the
library is not half-replaced. Root-relative paths stay, because `Trips/Japan/DSC_0001.jpg` is what a
maintainer needs and it says nothing about the machine.

There is a test for the promise rather than only for the format: it runs the binary, reads the output
before the golden normaliser touches it, and fails if either the library or the fixture path appears.

## Checks that describe the repository

`cargo xtask version` reports the one version the workspace declares and fails when a member carries its
own number. `cargo xtask attribution` checks `docs/ATTRIBUTION.md` against reality in both directions: a
crate a manifest links must be attributed, a crate the workspace declares but no manifest links must be
listed as such, and a table entry for a dependency that no longer exists is a problem too. It also refuses
a missing embedded font family and a deleted statement about model weights.

`cargo xtask a11y` walks `crates/ui/src/shell.rs` function by function and fails when a function makes a
region clickable (`Sense::click()`) without registering a name for it.

All three run inside `cargo xtask ci`, so a dependency that arrives without its licence being read cannot
be merged, a document that has drifted from the manifests fails in the same run, and a painted control
without a name fails before anyone can ship it.