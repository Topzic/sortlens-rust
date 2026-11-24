# Performance: what was measured, on what, and what was not

Every number here came from `cargo xtask perf`, run on the machine named below, in a release build. Nothing
is estimated, and nothing is copied from a plan. Run the same command on your own machine and you should get
numbers in the same shape, though not the same digits: fixtures are regenerated locally, the JPEG decoder is
scalar, and disc cache decides how fast a first scan goes.

```
hardware  Apple M4, 16 GB, macOS 27.0.1, 10 logical cores
toolchain rust 1.95.0, edition 2024, aarch64-linux-android and aarch64-apple-ios installed
command   cargo xtask perf --photos 2000 --thumbs 200 --frames 120
          cargo xtask perf --photos 10000 --thumbs 20 --frames 40
build     release (the binary prints a warning when it is a debug build, because those timings lie)
```

The fixtures are generated, not photographed: `1200×800` JPEGs at quality 85 with a gradient and
deterministic noise, so the decoder has coefficients to reconstruct and a Huffman stream to walk. A flat
colour would have compressed to nothing and produced a thumbnail rate for the easiest possible file. They
carry no EXIF, which is why "suggestions, as indexed" offers nothing and a second pass with camera, lens,
ISO and focal length set in memory is measured separately.

## Two thousand photos

| measurement | total | per item | rate |
|---|---:|---:|---:|
| scan (walk + stat) | 382 ms | 191 µs/file | 5 235 files/s |
| index (scan + EXIF + catalog op) | 379 ms | 190 µs/photo | 5 275 photos/s |
| open library (snapshot + log) | 6.2 ms | 3.1 µs/photo | 321 507 opens/s |
| thumbnail decode, cold, 128 px | 1 896 ms | 9.5 ms/thumb | 105 thumbs/s |
| thumbnail read, warm (cache hit) | 3.2 ms | 16 µs/thumb | 62 293 thumbs/s |
| thumbnail service, 2 workers | 1 000 ms | 5.0 ms/thumb | 200 thumbs/s |
| requests refused at the 4 096 bound | 0 of 200 | — | — |
| suggestions, with EXIF | 754 µs | 0.4 µs/photo | 2 653 252 photos/s |
| grid frame, 1440×900, headless, square | 0.28 ms median | p95 0.33 ms | 3 545 frames/s |
| grid frame, 1440×900, headless, justified | 0.14 ms median | p95 0.16 ms | 6 928 frames/s |
| first frame (fonts, first layout) | 4.3 ms | — | — |

Justified is cheaper per frame because its row pack is cached and only rebuilt when the width, filter, sort,
count or target height changes — the number measures painting at a steady state, not the pack. The pack itself
is one pass over the filtered library's aspect ratios (paged from the catalog), and its cost shows up on the
frame that changes one of those inputs, not in this median.

## Ten thousand photos

| measurement | total | per item | rate |
|---|---:|---:|---:|
| scan (walk + stat) | 8.2 s | 823 µs/file | 1 215 files/s |
| index (scan + EXIF + catalog op) | 11.2 s | 1.1 ms/photo | 889 photos/s |
| open library (snapshot + log) | 30 ms | 3.0 µs/photo | 332 306 opens/s |
| grid frame, 1440×900, headless, square | 1.40 ms median | p95 1.76 ms | 716 frames/s |

Scan drops from 5 200 to 1 200 files/s between two and ten thousand files because the walk also reads each
file to derive its content key: five gigabytes of JPEGs stop fitting in the page cache, and the number
becomes a disk number. That is worth knowing and not worth optimising yet — a scan is a background job with
a progress bar, and it is bounded by I/O rather than by anything this project controls.

## The bug the measurement found

The first ten-thousand-photo run took **78 seconds** to index, 7.8 ms per photo against 0.19 ms at two
thousand. That is not a slope, it is a different shape, so it was worth chasing rather than documenting.

Two defects, both in the snapshot policy:

1. `commit` took a snapshot every 256 operations, and a snapshot rewrites **every** photo in the library.
   Ten thousand files therefore meant 39 rewrites of a growing file — quadratic work for linear input.
2. `snapshot()` never reset the counter of operations since the last snapshot. Once a library crossed 256
   commits, `unlogged` stayed above the threshold forever, so *every single edit* rewrote the whole library.
   On a large library that turns an ordinary rating change into a full-file write.

The fix keeps the guarantee and drops the cost: a sync is a batch that takes one snapshot at its end (a
crash mid-import replays the log, which is what the log is for), the counter is reset by the snapshot, and
single interactive edits keep the 256-op cadence. Tests assert the work rather than a duration, so they
cannot pass by accident on a fast machine: `a_large_sync_takes_one_snapshot_instead_of_one_every_256_ops`
and `single_edits_still_snapshot_every_256_commits`.

After the fix, ten thousand photos index in 11.2 s at 1.1 ms per photo — the same per-photo cost as two
thousand, so the curve is linear. That is a 7× improvement on the same machine, same fixtures, same command.

## What these numbers are not

| budget that matters | why it is not here |
|---|---|
| 100 000-photo scroll at 60 Hz | Not measured. Generating and indexing a hundred thousand fixtures takes half an hour on this machine, and the grid is virtualised — it paints the visible cells, not the library — so a frame cost measured at ten thousand is mostly a statement about the viewport, not the library. What would unblock it: a real library to test against, which this project deliberately does not ship. |
| GPU frame time | Not measured, and not measurable here. Frames are `Context::run_ui` without a GPU and without texture uploads, so they are layout and shape building only. There is no vsync in this harness and no 60 Hz budget to check against. What would unblock it: running the app on a machine with a display and reading a real frame timer. |
| Memory, RSS per thousand photos | Not measured. The harness reports resident textures, not process memory, because a headless context does not allocate what a window does. What would unblock it: a long single-process run of the real app, which is not something this loop can drive. |
| Mobile, or any device that is not this laptop | Not measured. The phone targets are checked in `cargo xtask mobile`, but a build compiling is not a build running at speed. |
| HEIC, RAW, video thumbnails | No decoder ships for the first two (`docs/ATTRIBUTION.md` lists what is linked), so there is nothing to time. |
| Cold start of the window, first paint on screen | Not measured: the harness has no window. The first headless frame costs 4.3 ms and installs fonts, which is a floor, not the answer. |

## How to reproduce, and how to read a difference

```sh
. scripts/env.sh
cargo xtask perf --photos 2000 --thumbs 200 --frames 120
```

The table is printed for a person to read, not asserted: `perf` is deliberately not part of
`cargo xtask ci`, because a timing that fails on a loaded CI box teaches people to ignore failures, and a
timing that passes there says nothing about a library on a laptop. If a number moves by more than a few
percent, run it twice before believing it — fixture generation itself is not measured, and the first run
after a build pays for cold caches.
