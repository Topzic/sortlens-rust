# Releasing Sortlens

One version, declared once. `[workspace.package] version` in the root `Cargo.toml` is the only place a
number is written; every member inherits it with `version.workspace = true`, and
`sortlens_core::VERSION` is `env!("CARGO_PKG_VERSION")`, so the About window, `sortlens-cli --version`
and the release tag cannot disagree. `cargo xtask version` fails when a crate declares its own version or
stops inheriting the workspace one, and it runs as part of `cargo xtask ci`.

## What a release contains

| Artefact | Built from | Notes |
| --- | --- | --- |
| `sortlens` | `apps/sortlens` | the window |
| `sortlens-cli` | `apps/sortlens-cli` | everything the window does, plus `mcp` and `diagnostics` |
| `docs/` | the repository | the plan, the behaviour contract, the pack format |

There is no installer yet. The binaries are what a release attaches, with a SHA-256 next to each one, so
someone can check the file they downloaded is the file that was built. Signed and notarised macOS builds,
a Windows MSI and a Linux AppImage are M10 work that needs signing identities this repository does not
have — see the roadmap rather than assuming they exist.

## Cutting one

1. `cargo xtask ci` must be green on `main`, and so must the `snapshot` and `deny` jobs. A release from a
   red tree is a release nobody can bisect.
2. Bump the version in the root `Cargo.toml` only, then `cargo xtask version` to prove nothing else
   carries a number.
3. Update `CHANGELOG`-style prose in `CHECKPOINT.md` if the release covers more than one commit, and check
   `docs/ROADMAP.md` — its unchecked rows and the section above are what the release notes repeat, because
   a user deciding whether to install this needs the absences as much as the features.
4. `git tag vX.Y.Z && git push --tags`. The release workflow runs the same checks, builds the two
   binaries on macOS, Linux and Windows, and attaches them with checksums.
5. Read the generated release page before announcing it: a workflow that attached a stale binary would
   look identical to one that attached the right one.

## What this version does not have

Named here because a release is where "later" turns into "not in it". These are **tracked work, not
decisions**: each line points at the roadmap phase that closes it (`docs/ROADMAP.md`, "Completion
phases"), and the list shrinks as those rows are ticked.

- **Models are pinned, downloaded on consent, and the models can now be asked questions.** Tagging
  suggestions come from the photo's own EXIF, from keyword packs you write or install, from what the
  pinned YOLOS-tiny detector sees in a 384 px preview of each photo, and — when the pinned CLIP
  ViT-B/32 pair is installed — from which pack keywords the photo resembles by meaning: each pack's
  keywords are embedded once (CLIP BPE tokenisation, the shipped upstream vocabulary) and cached in
  the model store, a 224 px preview is embedded per photo, and every cosine above 0.25 is proposed
  through the same merger as every other source. All of it runs on the CPU by pure-Rust `tract-onnx`
  inference with no network. The model store, the manifest and the refusal to run without a digest are
  built and tested, real permissively licensed weights are pinned by digest (a YOLOS-tiny detector and
  a CLIP ViT-B/32 image + text pair, attributed in `docs/ATTRIBUTION.md`), and Settings → AI can
  install them: the consent-gated `model.install` downloader streams, resumes an interrupted download
  and refuses anything whose size or SHA-256 disagrees with the pin (an explicit, user-initiated
  network call — `crates/ai` itself stays offline). **Every keyword says where it came from**: typed by hand
  (`Manual`), proposed by a model or a pack and accepted (`AI`), read from the file's own sidecar (`XMP`), or
  `Not recorded` for a library written before sources were kept, which is deliberately not guessed at. The
  source is printed on the keyword chip, rides in the same undo step as the word, and is a filter in the window
  and in the CLI (`list --source ai`).
- **Export is Original, JPEG and PNG.** TIFF and WebP export are not planned.
- **The map is schematic until you supply tiles.** Places ships the public-domain Natural Earth 1:110m coastline,
  so there is geography with no setup, and it opens fitted to your photographs. A real basemap is a `tiles/`
  folder you supply, and names are a `places.txt` you supply; nothing is downloaded.
- **The remote half exists and the phone can use it; nothing is packaged for a phone yet.** A
  token-authenticated server (`apps/sortlens-server`), a pairing code, scoped device tokens and resumable
  uploads ship, and Settings → Sync & devices spawns and follows that server. `apps/sortlens-mobile` is the
  native Rust app over the same engine and shell — it indexes the device's photographs, keeps its own settings
  in `config.toml` beside the library, and `cargo xtask mobile` checks both targets. Its Settings → Send to
  desktop pairs with the code the desktop prints — and can test the address first, `GET /health`, no code
  spent — and sends the chosen photographs over the resumable upload
  with keep-or-remove; Remove carries the ack's decision out through the library's audited delete path to the
  phone's trash, and a file the catalog no longer matches is refused, not deleted. The layout is a phone's:
  one dock sheet at a time, a wordless bar, a two-row filter strip and a tray that stays on the screen. No
  APK, AAB or `.ipa` is published yet, and a device build waits for a machine with the NDK and Xcode. See
  §10 of `docs/ARCHITECTURE.md`.
- **No localisation.** The interface is English (Phase 4); the font stack and the keymap already handle the
  symbols the interface draws. Shortcuts *are* rebindable now: Settings → Shortcuts opens the `craft-keymap`
  editor (rebinding, conflict detection, named sets, craft-keymap JSON import and export), and the help and
  the palette paint the current chords.
- **No installer, no signed builds, no automatic update.** See M10 in the roadmap. What exists is
  the opt-in check: Settings → Updates or `sortlens-cli update-check` asks the project's GitHub
  releases once, with the consent switch off until you turn it on, and nothing is downloaded or
  installed automatically — an update means fetching the source and building, as today.
- **A phone-sized layout.** The Adobe/ArtCraft-style icon-first chrome in `docs/UX.md` shipped for the
  desktop (Phase 3), and a window under 600 points is now laid out as a phone: the docks become one
  full-width sheet at a time, the bar drops its words and keeps five glyphs, and the search field keeps
  the room they leave. The canvas inside that frame is the phone's too — the far-edge rail stands down while
  the inspector is closed and tab-switches beside its open sheet, the filter strip wraps into two rows
  without a second count, and the bottom bar's tray keeps every control on screen. The font set a phone build embeds is trimmed the same
way: `cargo xtask mobile` passes `CRAFT_FONTS_SCRIPTS` (the interface's two scripts plus every shipped
language's), so a phone binary embeds only the faces it could paint. What 3b still lacks is
  the device build and the iOS Photos library.

What *is* in it that older notes denied: HEIC thumbnails (`craft-heif`), PNG, TIFF, WebP, BMP and GIF
thumbnails, RAW previews, the shipped coastline and the tile-folder map.

## Checksums

Every artefact is published with its SHA-256 in a `SHA256SUMS` file next to it, in the format
`sha256sum -c` reads:

```sh
shasum -a 256 sortlens-cli-0.1.0-aarch64-apple-darwin.tar.gz
```

The hash is computed from the file that is uploaded, not from a build artefact that was renamed on the
way, because a checksum of something else is worse than no checksum.

## The icon

The mark is drawn in `crates/icon`: `rgba()` is rendered by `apps/sortlens/build.rs` into the window icon and
`svg()` is the committed `assets/icon.svg`. A test holds the file to the drawing (`SORTLENS_UPDATE_ICON=1`
rewrites it when the mark changes on purpose). The packaging files are generated from the same drawing too:
`assets/icon.ico` carries PNG entries at 16–256 px for Windows and `assets/icon.icns` carries the 32–512 px
PNG slots macOS reads, both checked byte-for-byte by the `sortlens-icon` tests and regenerable with
`cargo xtask icons --update`. What still does not exist is the installer and bundle work that would consume
them, with signing — those remain in `docs/ROADMAP.md`'s M10 absence table, each with its unblocker.
