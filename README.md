<div align="center">
  <img src="assets/icon.svg" width="72" height="72" alt="">
  <h1>Sortlens</h1>
  <p><strong>Cull thousands of photos. Keep everything local. Lose nothing.</strong></p>
  <p>
    <img alt="Rust" src="https://img.shields.io/badge/pure-Rust-f2a516?style=flat-square">
    <img alt="ArtCraft" src="https://img.shields.io/badge/ArtCraft-Craft%20App-8a5800?style=flat-square">
    <img alt="License" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-8a5800?style=flat-square">
    <img alt="Status" src="https://img.shields.io/badge/status-M0%20scaffold-f2a516?style=flat-square">
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

> **Status: M0 scaffold.** The workspace, the shell, the theme presets, the settings store and the
> command table build and run; the library itself (indexing, grid, culling) starts in M1. The plan
> and feature analysis are in `docs/`.

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
- **Never lose anything**: OS trash by default, an audit log, undo across annotations and file
  operations, crash-safe storage, backups and volume remapping.
- **Optionally** suggest tags with CPU-only local models you download yourself, and reach the
  library from a phone through a paired, local-network client.

## Why a rewrite rather than a port

The current Sortlens is Python + React inside a WebView: a localhost HTTP hop per action, a
bundled Python runtime, dynamic port discovery, a PowerShell updater and a UI that patches
individual fields over the wire. This recreation keeps the behaviour and drops the runtime:

| | Sortlens (current) | Sortlens (Rust) |
| --- | --- | --- |
| Runtime | Python service + WebView + local HTTP | One native process, in-process engine |
| Storage | Mutable SQLite database | Append-only op log + snapshots (undo and audit for free) |
| UI | React/Tailwind dark blue chrome | egui + ArtCraft shell: tokens, docking, fonts, native menus |
| Updates | Inno Setup + PowerShell script | `craft-update` with signed artifacts |
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
. scripts/env.sh          # project-local CARGO_HOME and RUSTUP_HOME
cargo run -p sortlens     # the shell: four themes, settings, command table
cargo xtask ci            # fmt, clippy, tests, layer rules, lint headers, asset provenance
cargo xtask layers        # only the architecture rules
```

The window opens on the empty library. Themes cycle from the top-bar chip or Settings; the choice
persists in `config.toml` through `craft-settings`. Commands that are not built yet are disabled and
say which milestone brings them.

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

Published dependencies keep their own licences. If you redistribute a build, include those notices;
`cargo deny check licenses` (M1) enforces the allow-list in CI.

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
