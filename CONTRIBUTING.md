# Contributing to Sortlens

Sortlens is a local-first photo culling tool. The thing it must never do is lose a photograph, so
the contribution rules are stricter around files than around pixels.

By contributing you agree that your work is licensed under **MIT OR Apache-2.0**, the licence of
this repository (see `LICENSE-MIT` and `LICENSE-APACHE`).

## Ground rules

1. **One issue, one pull request.** Small PRs get reviewed in a day; large ones rot.
2. **Claim before you start.** Comment on the issue so two people do not build the same thing.
3. **Clean room.** Do not copy code, assets, strings or file formats from the original Sortlens
   application or any other product. Behaviour is fair to study; source is not. If a decision
   came from observing the vendor app, say so in the PR.
4. **No new dependency** without a licence check (`MIT`, `Apache-2.0`, `BSD`, `ISC`, `Zlib`,
   `MPL-2.0`, `CC0` only) and one sentence in the PR about why the standard library or an existing
   crate is not enough.
5. **Green before review.** `cargo xtask ci` must pass. CI runs exactly that task on three
   operating systems.

## Architecture rules

These are enforced by `cargo xtask layers` and `cargo xtask roots`, not by review comments.

- Layers go down, never up: `core` → `meta`/`media`/`catalog` → `engine` → `ui` → apps.
- Only `crates/ui` may depend on `egui`, `eframe` or a `craft-*` UI crate.
- No app crate may depend on another app crate.
- Every crate root denies `unsafe_code`, `unwrap_used`, `expect_used` and `panic`. Tests are
  exempt via a module-level `#[allow(...)]`.
- Everything the user can do is a command in `crates/engine/src/command.rs` first, then a menu
  item, a shortcut, a CLI subcommand or an MCP tool. Never a bespoke code path.
- Nothing below `crates/ui` knows that a window exists.

## Things that need a design note first

Open an issue before writing code if you are changing:

- the op-log format, the snapshot format, or anything already written to disk,
- what Apply does to files, or the order of the safety checks in `docs/BEHAVIOUR.md`,
- the meaning of a rating, flag, label or sidecar field,
- a public command id or its arguments (they are a compatibility surface for scripts and agents).

`docs/BEHAVIOUR.md` is normative. A PR that contradicts it is a bug, or a proposal to change it.

## Working on it

```sh
. scripts/env.sh          # project-local toolchain, if you used scripts/bootstrap.sh
cargo run -p sortlens     # the application
cargo xtask ci            # fmt, clippy, tests, layer rules, lint headers, provenance
cargo xtask wasm          # wasm-safe crates
```

Commit messages: `<area>: <imperative summary>` — for example `catalog: keep ratings across a
rescan` or `ui: fix filter bar height at narrow widths`. The body explains why, not what.

## Reporting a security problem

Open a private advisory through the repository's **Security** tab rather than a public issue,
especially for anything that could delete, move or overwrite a file outside a watched folder.
