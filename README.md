<div align="center">
  <img src="assets/icon.svg" width="72" height="72" alt="">
  <h1>Sortlens</h1>
  <p><strong>Cull thousands of photos. Keep everything local. Lose nothing.</strong></p>
  <p>
    <img alt="Rust" src="https://img.shields.io/badge/pure-Rust-f2a516?style=flat-square">
    <img alt="ArtCraft" src="https://img.shields.io/badge/ArtCraft-Craft%20App-8a5800?style=flat-square">
    <img alt="License" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-8a5800?style=flat-square">
    <img alt="Status" src="https://img.shields.io/badge/status-M0%E2%80%93M10%20built%2C%20phases%201%E2%80%935%20open-8a5800?style=flat-square">
  </p>
</div>

Sortlens is a local-first photo culling and library tool for photographers who come back from a
shoot with thousands of frames. Rate, flag, tag, group and cull at speed; every decision is
staged, reversible and written where other tools can read it. No account, no cloud, no
telemetry, no subscription.

This repository is the **Rust recreation** of Sortlens (v0.11.0), built as a sibling of the
ArtCraft Craft Apps: native on macOS, Windows and Linux, in the same visual and interaction
language as LightCraft, PhotoCraft and the rest of the suite, and drivable end to end by
scripts and AI agents.

> **Status: M0–M10 built, core completion in progress.** The milestones are built, but several core
> features are still open and tracked as unchecked rows under "Completion phases" in `docs/ROADMAP.md`
> (video poster frames, PNG export, real local AI tagging, an out-of-the-box map, the Adobe/ArtCraft layout
> pass, the keymap editor, the remote client). A working local photo library: scanning and indexing, a virtualised grid,
> filters and search, keywords with an alias registry, XMP sidecars, the cull machine with gestures and
> zoom, collections, duplicate and colour matching, insights, memories, detail and side-by-side compare,
> backup/restore, audit and compaction, one-writer locking, move with undo, a change viewer, export to
> originals or JPEG, keyword suggestions from the file's own EXIF and from keyword packs, a whole-library
> suggestion pass that writes nothing, `--json` on every read verb, a diagnostics bundle that redacts your
> paths, and an MCP server — all driven by one command registry that the window, the CLI and an agent share.
>
> What is **not** here is named rather than hidden, in three places: `docs/ROADMAP.md` tracks every open
> core feature as an unchecked row in "Completion phases" and keeps a "Deliberately absent" table only for
> work that needs something outside the repository (signing identities, a screen reader, a release feed),
> `docs/PERF.md` lists the budgets this machine cannot measure, and every
> disabled control in the app says which milestone would bring it.

## Read the plan

| Document | What it answers |
| --- | --- |
| [`docs/PLAN.md`](docs/PLAN.md) | Mission, goals, non-goals, locked decisions, milestones, quality gates, risks, release plan |
| [`docs/FEATURES.md`](docs/FEATURES.md) | All 176 features with evidence (vendor docs, screenshots, inference), priority, phase and Rust approach |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Crates and layers, op-log storage, command registry, threading, safety rails, testing, distribution |
| [`docs/UX.md`](docs/UX.md) | The five surfaces, every screen, information architecture, keyboard map, themes, accessibility |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Checkbox tracker per milestone, acceptance demos, backlog |
| Fonts | egui's own faces by default; craft-fonts fallbacks when `CRAFT_FONTS_DIR` is set (see below) |
| [`docs/TOOLING.md`](docs/TOOLING.md) | Deep audit: getartcraft.com, craft-libs, craft-fonts, craft-extensions, rustup, and the Sortlens repo |
| [`docs/BEHAVIOUR.md`](docs/BEHAVIOUR.md) | Normative contracts: cull states, delete modes, XMP, scan identity, previews, crash safety |
| [`docs/PACKS.md`](docs/PACKS.md) | The keyword pack format: what a rule may match, what is refused, and why |
| [`docs/RELEASE.md`](docs/RELEASE.md) | How a release is cut, what the artefacts are, and what this version deliberately lacks |
| [`docs/CLI.md`](docs/CLI.md) | The `--json` contract: what a script may rely on, what may change, and every verb that has a schema |
| [`docs/sortlens_agent_completion_directive.md`](docs/sortlens_agent_completion_directive.md) | The owner's course-correction: sign-offs, and the five completion phases the roadmap now tracks |
| [`docs/PERF.md`](docs/PERF.md) | Measured numbers with the machine and command that produced them, the defect they exposed, and the budgets that cannot be measured here |

## What it will do

- **Index** any number of folders, in place, with removable-media awareness and offline states.
- **Cull** by keyboard or gesture, one decision per frame, with prefetched full-screen previews,
  resumable sessions and staged rejects that only touch disk when you apply them.
- **Label** with five stars, five colour labels, pick/reject flags and tags, written to XMP
  sidecars so other tools see the same thing.
- **Find** with search, a composable filter bar, smart scopes, similarity grouping and
  soft-focus detection — all computed locally.
- **Understand** the library with a dashboard, review progress, capture-date highlights, place
  clusters and deterministic "memories".
- **Attributed**: every dependency, font face and asset is listed in `docs/ATTRIBUTION.md`, and a check fails when that document and the manifests disagree.
- **Never lose anything**: OS trash by default, an audit log, undo across annotations and file
  operations, crash-safe storage, backups and volume remapping.
- **Script** it: every read verb answers in JSON with `--json`, the CLI and the MCP server speak the
  same command registry, and the schemas are pinned by golden files.
- **Report** a bug without leaking your folders: `sortlens-cli diagnostics` prints a bundle whose absolute
  paths are replaced by placeholders before anything is written.
- **Optionally** suggest tags with CPU-only local models you download yourself (no model is pinned yet,
  which the app says plainly), and reach the library from a phone through a paired, local-network
  client (not built — see the roadmap's absent-by-design table).

## Why a rewrite rather than a port

The current Sortlens is Python + React inside a WebView: a localhost HTTP hop per action, a
bundled Python runtime, dynamic port discovery, a PowerShell updater and a UI that patches
individual fields over the wire. This recreation keeps the behaviour and drops the runtime:

| | Sortlens (current) | Sortlens (Rust) |
| --- | --- | --- |
| Runtime | Python service + WebView + local HTTP | One native process, in-process engine |
| Storage | Mutable SQLite database | Append-only op log + snapshots (undo and audit for free) |
| UI | React/Tailwind dark blue chrome | egui + ArtCraft shell: tokens, docking, fonts, native menus |
| Updates | Inno Setup + PowerShell script | not wired yet: `craft-update` is declared and unlinked, because an update check is a network call this version does not make |
| Automation | Local HTTP endpoints | Command registry + CLI + control channel + MCP |
| AI | Vendored Python runtimes | Pinned ONNX models run by a pure-Rust engine (`tract`) |

## Build on

`craft-libs` (decoders, metadata, XMP, UI components, docking, fonts, settings, keymap, menus,
dialogs, update, MCP, web host), `craft-fonts`, `craft-extensions`, and the ArtCraft engineering
standards: layered crates, command-first UI, theme tokens, panic-free non-test code, bounded
parsing of hostile input, and a clean-room rule that forbids copying another product's code or
assets.

## Getting started

The toolchain is pinned in `rust-toolchain.toml`. On a machine without Rust, `scripts/bootstrap.sh`
installs rustup with Homebrew and puts the toolchain in `./.toolchain` so nothing user-wide
changes; the same folder is what this repository's `scripts/env.sh` expects.

```sh
. scripts/env.sh              # project-local CARGO_HOME and RUSTUP_HOME
cargo run -p sortlens         # the window
cargo run -p sortlens-cli -- --library ./lib stats
cargo xtask ci                # fmt, clippy, tests, layers, roots, assets, fonts, version, attribution, a11y, docs
cargo xtask snapshot          # diff the recorded CLI output
cargo xtask layers            # only the architecture rules
cargo xtask perf              # measure scan, index, thumbnails, suggestions, frames (release build)
```

The window opens on the first-run wizard: add a folder and it indexes while the grid fills in.
Themes cycle from the top-bar chip or Settings, and the choice persists in `config.toml` through
`craft-settings`. Commands that are not built yet are disabled and say which milestone brings them.

The CLI speaks the same engine, so anything the window does can be scripted:

```sh
sortlens-cli add ~/Pictures/2024      # index a folder
sortlens-cli rate 12 5                # five stars
sortlens-cli tag 12 harbour           # keyword
sortlens-cli cull …                   # the same command ids the window uses
sortlens-cli mcp                       # expose the registry to an agent over stdio
```

`sortlens-cli --help` lists the verbs; `sortlens-cli shortcuts` lists the keyboard map, and the same
table is in the app under `?`.

### What is not built yet

Named so that nothing here reads as finished: video poster frames, blur-up, an out-of-the-box map,
blur scoring, local model inference (the owner has approved pinning permissively licensed weights and a
consent-gated downloader), PNG export, the remote/companion server, localisation and right-to-left layout,
shortcut rebinding, high contrast and reduced motion, theme import, per-platform installers and signing,
and a wired update checker. Each is an unchecked row under "Completion phases" in `docs/ROADMAP.md` (or, when
it needs something this repository lacks, a row in a "Deliberately absent" table there), and is disabled in
the UI rather than faked. (HEIC thumbnails and the tile-folder map have shipped.)

`docs/TOOLING.md` §5 lists the platform prerequisites and the optional cargo tools.

## Licence

Dual-licensed under either of

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option. Contributions are accepted under the same terms; see [CONTRIBUTING.md](CONTRIBUTING.md).
The Apache option includes an explicit patent grant, which is the reason to offer both rather than MIT
alone.

Sortlens is an independent, clean-room recreation. The original application's name, binaries and
screenshots remain the property of their owner. This repository contains no vendor code, no code
derived from decompilation, and no vendor assets; `docs/ATTRIBUTION.md` lists every dependency and
every bundled file with its licence.

Published dependencies keep their own licences. If you redistribute a build, include those notices.

`deny.toml` is the allow-list, and `cargo xtask deny` (its own CI job) enforces it: permissive
licences only — MIT, Apache-2.0 (with the LLVM exception), BSD-2/3, ISC, Zlib, 0BSD, BSL-1.0 via
Apache, MPL-2.0, OFL-1.1, Unicode-3.0, the IJG and Ubuntu font licences, Unlicense — with no GPL,
AGPL, LGPL or CC-BY-NC. It also denies vulnerable and yanked crates, refuses wildcard version
requirements and any git source that is not one of the three pinned here. Nothing in the workspace is
published to crates.io (`publish = false`), which is why internal path dependencies carry no version.

## Fonts

The interface runs on egui's own faces unless a craft-fonts checkout is named at build time. That is
deliberate: the fonts live in a separate repository, and a Cargo dependency on it would make every
build — offline, in a fork, on a mirror — fetch it.

```sh
git clone https://github.com/storytold/craft-fonts .fonts/craft-fonts
CRAFT_FONTS_DIR=$PWD/.fonts/craft-fonts cargo build -p sortlens
CRAFT_FONTS_DIR=$PWD/.fonts/craft-fonts cargo xtask fonts     # what is embedded
```

Native builds embed all 21 faces of the craft-fonts manifest (Japanese, Korean, Simplified Chinese,
Hebrew, Arabic, Thai, Georgian, nine Indian scripts and two symbol faces). A wasm build keeps five —
`BIZ UDPGothic Regular/Bold`, `NanumGothic Regular`, `Noto Sans Symbols` and `Noto Sans Symbols 2` —
because the whole manifest is about 48 MiB. `fonts/embedded.txt` records what ships so a release can
diff it. Release builds set `CRAFT_FONTS_REQUIRED=1` so a missing checkout fails instead of silently
shipping without them.

Craft faces are **appended** to each family, never prepended: a fallback that changes Latin text is a
bug. `crates/ui/src/fonts.rs` names the scripts the interface itself needs — `Latn` for the text and
`Zsym` for the `⌃ ✕ ◀ ✓` keycaps — and `coverage()` says which of them the embedded table covers.
