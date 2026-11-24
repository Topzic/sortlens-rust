## What this changes

<!-- One paragraph. The commit body explains why; this says what a reviewer is looking at. -->

## Linked issue

Closes #

<!-- One issue, one pull request. A change with no issue behind it is a change nobody agreed to;
     say what it is for above instead of leaving this blank. -->

## The test that fails without this change

- Test:
- It fails on `HEAD~` because:

<!-- A test that passes before the change and after it has not seen the change. If the behaviour
     is exercised only by hand, say what you pressed and what changed on screen. -->

## Gates

- [ ] `cargo xtask ci` — fmt, clippy `-D warnings`, the whole workspace's tests, and the layer, header, asset, font, version, attribution, a11y, document and intake checks
- [ ] `cargo xtask snapshot` — the recorded CLI output is unchanged, or `--update` and a sentence below saying what it now says
- [ ] `cargo xtask deny` — licences, advisories, bans, sources
- [ ] `cargo xtask mobile` — the phone targets still check for Android and iOS
- [ ] `cargo xtask screenshots` — only when `crates/ui` (or a crate it paints) changed; the baselines were read, not regenerated to make the run green

<!-- CI runs exactly these tasks. A box ticked without the command having run is the one thing a
     reviewer cannot check for you. -->

## Performance

<!-- Required when the change touches scan, catalog or thumbnails (thumbs): run `cargo xtask perf`
     on the same machine before and after, and paste both numbers. A regression needs a sentence
     saying what it buys, in the PR, not in a follow-up issue. -->

| Area | Before | After |
| --- | --- | --- |
| scan | | |
| catalog | | |
| thumbs | | |

## Screenshots

<!-- Required when `crates/ui` changed: a before and an after of the same scene at the same size,
     because a baseline diff is the review for a painted surface. Delete this section otherwise. -->

| Before | After |
| --- | --- |
| | |
