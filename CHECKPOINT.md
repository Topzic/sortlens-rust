# Checkpoint

## 2026-11-21 — M0 complete (6a4983e)

23-crate workspace, shell, themes, settings, command registry, xtask CI, licences, CI on 3 OSes.

## 2026-11-21 — M1: index and browse

Shipped:
- `sortlens-scan`: bounded walker (skip lists, no symlinks, depth cap), media classification,
  content key from size + first/last 64 KiB. 5 tests.
- `sortlens-catalog`: `Photo`, `Filter`/`Sort`/`Page`, and a catalog of 19 tests: checksummed op
  log, atomic snapshots, replay with a dropped torn tail, undo, rescan that keeps ids and
  annotations, missing/restored files, stats, paging.
- `sortlens-engine`: catalog ownership, worker-thread scan with `poll()`, typed rate/flag/label/
  undo/remove_root, JSON `run()` for 25 commands. 9 tests.
- `sortlens-ui`: virtualised grid (visible rows only), selection, keyboard rating (0–5), flags
  (P/X/U), sidebar roots with counts and context menu, details/tags/activity, real status bar.
- `sortlens-cli`: add, rescan, roots, list, stats, rate, flag, undo, report. Verified end to end:
  index a folder, rate, flag, reopen in a second process, state intact.

Verified: `cargo xtask ci` green (fmt, clippy -D warnings, 42 tests, layer/lint/provenance rules);
`cargo xtask wasm` green; CLI smoke test round-trips through disk.

Known gaps (deliberate, next milestones): no thumbnails yet (cells show kind + name), metadata
not read (width/camera/captured stay empty), filters in the bar are still disabled, no cull view.

## Next

M2 — previews: `sortlens-meta` + `craft-metadata`/`craft-jpeg`/`craft-tiff`, tiered thumbnail
cache, texture upload, priority scheduler, scan progress with counts.
