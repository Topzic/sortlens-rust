# Attribution and licences

Every third-party dependency and asset in this repository is listed here with the licence it is
used under. `cargo xtask assets` fails when a file in `assets/` has no entry.

## Policy

Allowed: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, MPL-2.0, CC0-1.0, Unlicense.
Not allowed: GPL, AGPL, LGPL (statically linked), SSPL, Commons Clause, "source available"
licences, and anything without a licence file. `cargo deny check licenses` enforces the list once
`deny.toml` is wired in CI (M1).

Code copied from another application is never allowed, whatever its licence. Sortlens is written
from behaviour, not from source: see `docs/TOOLING.md` for the clean-room notes.

## Build and runtime dependencies

| Component | Licence | Use |
|---|---|---|
| craft-libs (`craft-ui`, `craft-window`, `craft-settings`, `craft-metadata`, …) | MIT OR Apache-2.0 | Suite chrome, docking, settings, decoders, metadata |
| egui / eframe (storytold fork) | MIT OR Apache-2.0 | Immediate-mode UI and windowing |
| winit (storytold fork) | Apache-2.0 OR MIT | Window creation |
| wgpu | MIT OR Apache-2.0 | GPU renderer behind eframe |
| serde, serde_json | MIT OR Apache-2.0 | Op-log and settings serialisation |
| thiserror | MIT OR Apache-2.0 | Error types |
| toml / toml_edit | MIT OR Apache-2.0 | Settings documents |
| chrono | MIT OR Apache-2.0 | Capture dates and timestamps |
| crc32fast | MIT OR Apache-2.0 | Op-log record checksums |

Exact versions are pinned in the workspace `Cargo.toml`; craft crates share one git revision.

## Assets

| File | Origin | Licence |
|---|---|---|
| `assets/icon.svg` | Created for this project; project-owned | MIT OR Apache-2.0 |

No fonts, icons, sample photos or model weights are vendored yet. When they are:

- fonts ship through `craft-fonts` and keep their own licences (OFL families only),
- icons must be drawn for this project or come from an allowed icon set with its attribution,
- sample photographs must be project-owned or CC0, never from the original application.

## Product naming

"Sortlens" is the working name of this recreation and is not claimed as a trademark. If the
project is published under a different name, this file and the app id (`app.sortlens.desktop`)
change together.
