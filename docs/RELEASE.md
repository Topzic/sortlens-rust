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
   `docs/ROADMAP.md` — its "Deliberately absent" tables are what the release notes repeat, because a user
   deciding whether to install this needs the absences as much as the features.
4. `git tag vX.Y.Z && git push --tags`. The release workflow runs the same checks, builds the two
   binaries on macOS, Linux and Windows, and attaches them with checksums.
5. Read the generated release page before announcing it: a workflow that attached a stale binary would
   look identical to one that attached the right one.

## What this version does not have

Named here because a release is where "later" turns into "not in it":

- **No bundled models.** Tagging suggestions come from the photo's own EXIF and from keyword packs you
  write or install. The model store, the manifest and the refusal to run without a digest are built and
  tested; no weights are pinned, so there is no "download model" button and no menu item for one.
- **No remote.** There is no server and no phone client; the design is in `docs/ARCHITECTURE.md` §11.
- **No localisation.** The interface is English; the font stack and the keymap already handle the symbols
  it draws.
- **No video or HEIC thumbnails**, no PNG/TIFF/WebP export, no map.

## Checksums

Every artefact is published with its SHA-256 in a `SHA256SUMS` file next to it, in the format
`sha256sum -c` reads:

```sh
shasum -a 256 sortlens-cli-0.1.0-aarch64-apple-darwin.tar.gz
```

The hash is computed from the file that is uploaded, not from a build artefact that was renamed on the
way, because a checksum of something else is worse than no checksum.
