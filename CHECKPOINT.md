# Checkpoint

## 2026-11-24 (seventy-sixth) — a red screenshot run hands over every scene it painted, not just the first failure

- **Why this came up.** `CI` on `main` is red in `screenshots`, and the run at `40e838a` (`45543501424`) is the
  evidence. Its `screenshot-set` artifact holds twenty `.new.png` frames, and the committed `lavapipe` set still
  lacks four paper scenes — `detail-paper`, `compare-paper`, `library-justified-paper`, `library-square-paper`.
  The artifact could not have held them. Each pair lives in one test (`detail_and_compare_paint_in_both_themes`,
  `library_grid_paints_both_layouts_in_both_themes`) and `shoot` panicked on the first mismatch, so every scene
  after it was never painted at all. Six red runs and two baseline commits later the set is still four short, and
  the artifact that is supposed to complete a set was structurally unable to name more than the first failure of
  each test. That is the defect the four missing files are a symptom of.
- **A mismatch no longer ends the test.** `shoot_at`/`shoot_at_lenient` record the scene's baseline name and the
  error in a per-thread `Painted { scenes, mismatches }` instead of `.expect()`-ing, so the next scene of the same
  test still paints and still writes its `.new.png`. Each of the 23 painting tests ends with
  `assert_every_scene_matched()`, which drains the record and fails with every scene that differed and
  `N of this test's M scenes` — the count is what lets the message say a scene matched. The record is
  thread-local because libtest gives every test its own thread, and taking it empties it so nothing leaks between
  tests.
- **The check cannot be forgotten.** Three new tests, one of which reads `include_str!("screenshots.rs")`:
  `every_test_that_paints_a_scene_checks_its_whole_report` names any test that paints and never calls the check.
  It was proved to bite by deleting `cull`'s call — it answered `["cull_paints_in_both_themes"]` — and then the
  call was restored. `a_report_names_every_scene_that_differed` pins the counting and the emptying;
  `a_mismatch_fails_the_test_once_every_scene_has_painted` pins the panic.
- **Files.** `crates/ui/tests/screenshots.rs` (`Painted`, `RECORD`, `record`, `take_report`,
  `assert_every_scene_matched`, the three `shoot*` call sites, one call at the end of each painting test, the
  three tests); `docs/ARCHITECTURE.md` §12 (what a red run hands over; the stale "Thirty-one PNGs" corrected to
  forty-seven); `docs/ROADMAP.md` (Phase 5 item 3's artifact sentence now says *every* frame, and why).
- **Verified.** `cargo xtask screenshots` green: 27 tests, 50 s, no baseline moved and no `.new` written — the
  refactor paints exactly what it painted before. `cargo xtask ci` (one `cargo fmt --all` first), `snapshot` and
  `deny` for this commit. Nothing here is a UI change, so no new `Context::run_ui` test was owed. `cargo xtask
  mobile` still needs the NDK and Xcode, as documented.
- **Next.** The trunk first, and it is one command: read the `CI` run for this commit with one
  `gh run view <id> --json status,conclusion,jobs`. Its `screenshots` job will be red on the four missing paper
  scenes — and now paints all four, which is the point of this commit. `gh run download <id> -n screenshot-set -D
  tmp-shot`, move `detail-paper.new.png`, `compare-paper.new.png`, `library-justified-paper.new.png` and
  `library-square-paper.new.png` to `crates/ui/tests/snapshots/lavapipe/` with `.new` stripped, read them, and
  commit: that is the tick that turns the job green. After that the only open roadmap row is SL-L07's remainder
  (zh-TW/ko/th/vi catalogues, the unrouted surfaces, and RTL screenshots which wait on an Arabic-capable font
  build).
## 2026-11-25 (seventy-second) — the shell writes numbers and dates the reader's language (SL-L08)

- **Why this came up.** The last row left open in M9 by the sixty-eighth tick's scaffolding was
  the arithmetic: every count in the shell went through `sortlens_core::format_count` (thin spaces,
  Latin digits), every file size through `format_bytes` (`1.4 MB`), and the detail panel's date
  through `Photo::captured_label` (`2026-11-21 14:32`) — one universal shape regardless of whether
  the interface was English, Japanese or Arabic. An Arabic interface that mirrors its rows into
  the reading start but writes `12,408` and `21/11/2026` in Latin digits is translated, not localised.
- **One table, held to the picker.** `crates/ui/src/format.rs` is the whole slice: a `Style` per
  offered language (digit zero, grouping mark, decimal mark, date field order and separator), the
  three functions the shell calls (`count`, `size`, `date_time`), and explicit `_in(style, …)`
  forms so tests never switch the thread-local language. The table is English (Latin, `1 234 567`
  with the narrow no-break space, `.` decimal, `2026-11-21 14:32`), Japanese (Latin, `12,408`,
  `2026/10/03 04:00`) and Arabic (Arabic-Indic `١٢٬٤٠٨`, `٫` decimal, day first
  `٠٣/١٠/٢٠٢٦ ٠٤:٠٠`). The formatting region is the interface language on purpose: the operating
  system's regional settings would make a committed scene read differently on two machines.
- **English is byte-identical.** The English style reproduces what the app painted before, and a
  test compares it against `format_count`/`format_epoch` across the counts and the epoch both
  modules were tested on — so routing the shell through the new module moved no committed scene.
  The shell changes are three one-line wires, not 30 call-site edits: `format_count` (the single
  funnel every sidebar, figure, notice and status count already used) now calls `format::count`;
  the detail panel's Size row calls `format::size`; its Captured row formats the epoch through
  `format::date_time` instead of `captured_label`.
- **The layers it needed.** `sortlens_meta::CivilTime` and `civil_time(seconds)` take an epoch apart
  (`format_epoch` is now written in terms of it) so an interface can order the fields itself
  instead of parsing the ISO sentence back apart; `sortlens_core::byte_measure(bytes)` returns the
  scaled value, unit and decimal digits `format_bytes` writes, so `format::size` can put those
  digits in the language's set and mark without duplicating the unit thresholds. `crates/ui` now
  depends on `sortlens-meta` for the date parts.
- **Files.** `crates/ui/src/format.rs` (new); `crates/ui/src/lib.rs` (`mod format`);
  `crates/ui/src/shell.rs` (the three wires, the language test's digit assertion);
  `crates/meta/src/lib.rs` (`CivilTime`, `civil_time`); `crates/core/src/units.rs` (`byte_measure`);
  `crates/core/src/lib.rs` (the re-export); `crates/ui/Cargo.toml`; `docs/ROADMAP.md` (SL-L08 row
  ticked with the shipped shape); `docs/FEATURES.md` (SL-L08); `docs/UX.md` (the i18n bullet);
  `docs/ARCHITECTURE.md` §8.4 (the formatting region and the table test; the stale "all 19
  baselines" sentence corrected).
- **Tests that fail when the behaviour is absent** (six new): each offered language has exactly
  one style and every style belongs to an offered language; English reproduces `format_count`/
  `format_epoch`; grouping, digits and decimal marks per language; date field order per language
  including a pre-1970 epoch; an unknown code falls back to English. Headless in the existing
  Settings → Language test: after clicking العربية, the sidebar's count galley contains `٠`
  (`U+0660`), which fails when the count funnel goes back to Latin digits.
- **Verified.** `cargo xtask ci` green (the first run failed only `cargo fmt --check`, reformatted
  and rerun); `cargo xtask snapshot`, `deny`, `wasm` and `screenshots` run for this commit.
- **Next.** The trunk first: CI run `45542266937` was still running for `cec3f94` when this tick
  began, so read it with one `gh run view 45542266937 --json status,conclusion,jobs` before
  anything else. Then the only open roadmap row is SL-L07's remainder: zh-TW/ko/th/vi catalogues
  (data + `LANGUAGES` + `font_manifest`'s script table + a style row), the unrouted surfaces, and
  RTL screenshots, which still wait on a font build with an Arabic face. If that run is red, fixing
  it is the tick's slice instead.
## 2026-11-25 (seventy-first) — a frame the app draws has the window's three buttons (Phase 3, 3B.6 (i))

- **Why this came up.** The shell has been able to run under its own frame since `craft-window`'s
  `Frame::Integrated` shipped (macOS draws the bar as the title bar, with `title_inset` clearing the traffic
  lights), but the bar never drew the window's own buttons. Under an integrated frame on Windows and Linux that
  left a window with no minimize, no maximize and no close on the bar — the frame was the app's and the
  buttons were nobody's.
- **The frame decides; the shell draws.** `UiState::captions` is set once by the host from the frame policy
  (`matches!(decision.frame, Frame::Integrated(_))`) and never guessed by the bar. When it is true the bar
  paints minimize, maximize/restore and close at its far end: the row is laid out right-to-left, so the
  buttons are its last children and nothing the shell adds afterwards can push them off. When it is false the
  bar paints nothing, because the system title bar already drew them.
- **Each button is a glyph with a name.** A rule, a frame (or the two-frame restore glyph — read from
  `viewport().maximized` each frame, so a system maximize moves it), and a cross. `caption_button` allocates
  `CAPTION_W` square, tints hover with `t.hover` — close with `t.reject`, its glyph taking `t.bg` so the
  cross stays legible on the red — and labels each through `a11y::label_widget` and `on_hover_text`.
- **The bar asks, the host commands.** `Action::MinimizeWindow`, `ToggleZoom` and `CloseWindow` are what the
  bar raises; `apps/sortlens` sends `ViewportCommand::Minimized(true)`, `Maximized(!maximized)` and `Close`.
  Close is deliberately not `Quit`: `Quit` means the unsaved-changes guard was already answered, and this
  button has not asked — so the window's own close handling runs the guard, exactly as the system button does.
- **Files.** `crates/ui/src/shell.rs` (`Action`'s three arms, `UiState::captions`, `Caption`,
  `caption_buttons`, `caption_button`, `caption_glyph`, the bar's right-to-left block, the test);
  `apps/sortlens/src/main.rs` (`captions` from the frame decision, passed into `App::new`, the three viewport
  commands); `docs/ROADMAP.md` (3B.6 (i)); `docs/UX.md` §3; `docs/ARCHITECTURE.md` §8.1.
- **Tests that fail when the behaviour is absent.** One headless `Context::run_ui` test at 1280×800:
  `the_window_buttons_appear_with_the_app_s_own_frame_and_ask_the_host` walks the painted shapes for diagonal
  strokes at the bar's right edge (none where the OS draws them, the close cross's two strokes with the app's
  frame), reads each button's AccessKit label and asserts the order `Minimize`, `Maximize`, `Close`, then
  presses close's centre and asserts `Action::CloseWindow` was raised and `Action::Quit` was not.
- **Verified.** `cargo clippy -p sortlens-ui --all-targets -- -D warnings` clean; `cargo test -p sortlens-ui
  --lib the_window_buttons` passes. `cargo xtask ci`, `snapshot`, `deny` and `screenshots` were run this tick
  for the commit; the caption glyphs paint only when `captions` is true and no committed scene sets it, so no
  baseline moves.
- **Still red on `main`, and these are the next two slices.** The `CI` workflow run `45536458391` (sha
  `85da2b4`) failed two jobs, both for reasons that predate this entry: `screenshots` (13 of 23 scenes differ
  on Mesa's software Vulkan — six commits of red the loop never looked at, because it only watched `release`)
  and `mobile (ubuntu-latest, android)` (`cc-rs` cannot find `aarch64-linux-android-clang`; the NDK is
  installed but no `CC_aarch64_linux_android`/`AR_aarch64_linux_android` points at it). The caption slice does
  not touch either.
- **Next.** Take the red trunk first, as the loop's rules say. `screenshots`: the baselines are painted on
  this machine's GPU and compared on a runner with none, so no tolerance can honestly bridge them — the fix is
  a per-rasteriser baseline set (a distinct directory for the lavapipe render) plus the CI job uploading the
  `.new` images it paints so the next tick commits them. Then `mobile`'s android leg: give `cargo xtask
  mobile` the NDK's `CC`/`AR` for the target, the same way the iOS leg inherits Xcode's clang.

## 2026-11-24 (seventieth) — the phone's font set is trimmed to the languages it ships (Phase 4 item 3b)

- **Why this came up.** The sixty-ninth entry left the roadmap's 3b row with three open items — a device
  build, the iOS Photos library, and "trimming the embedded font set for a phone". The first two wait for a
  Mac and an NDK; the third was buildable here, and a phone binary that embeds the whole 48 MiB craft-fonts
  manifest to paint three languages was paying for families it could never draw (the manifest's Hangul face,
  for a build that ships English, Arabic and Japanese).
- **One parser answers for two readers.** `crates/ui/src/font_manifest.rs` is a dependency-free module
  `crates/ui/build.rs` includes at build time (`#[path]`) and `xtask` includes for its font report, so a
  build and `cargo xtask fonts` cannot disagree about what a manifest line means or which faces survive. It
  parses craft-fonts' `family | style | file | scripts | …` lines, selects the faces a build embeds, and
  validates that a set covers the scripts the interface draws.
- **The filter is `CRAFT_FONTS_SCRIPTS`.** Comma- or space-separated ISO 15924 codes; unset means the whole
  manifest, as the desktop embeds today. `cargo xtask mobile` passes exactly `PHONE_SCRIPTS` — `Latn`,
  `Zsym`, `Arab`, `Jpan`, the interface's own two plus every shipped language's — when `CRAFT_FONTS_DIR` is
  set, through a new `run_env` helper (edition 2024 makes `set_var` unsafe). `build.rs` also writes
  `SELECTED_SCRIPTS` into the generated table, and `fonts::selected_scripts()` reports it, so a release can
  say what it carries without re-deriving it.
- **Refusals before bytes, not after.** A requested script no manifest face covers is refused by name (a
  typo such as `Hng` never embeds a half-set); `cargo xtask fonts` refuses a set — filtered or not — that
  misses a script the interface ships, which strengthens the old "covers no Latin" check to the shipped
  languages too. The phone script table is held to its definition by a test (required scripts ∪ shipped
  languages), and another test holds `SHIPPED_LANGUAGE_SCRIPTS` to the picker's own `LANGUAGES`, so adding
  a language without its script fails the build, not the phone.
- **Verified end to end against a fixture checkout**: a fake craft-fonts clone with six faces (Japanese,
  Arabic, Hangul, two symbol faces) — `cargo xtask fonts` reported `5 of 6 face(s) in 4 family(ies) after
  the CRAFT_FONTS_SCRIPTS filter`, the unfiltered report kept all six, and the typo filter was refused by
  name; the filtered `cargo build -p sortlens-ui` generated a table with the five kept faces and no
  `NanumGothic`. On this checkout (no fonts checkout, CI's configuration) nothing changes: `xtask fonts`
  still reports "not embedded" and the app still runs on egui's own faces.
- **Files.** `crates/ui/src/font_manifest.rs` (new); `crates/ui/build.rs` (the filter, `SELECTED_SCRIPTS`,
  the parser shared); `crates/ui/src/fonts.rs` (`selected_scripts`, `REQUIRED_SCRIPTS` re-exported from the
  module, two new tests); `crates/ui/src/lib.rs`; `xtask/src/main.rs` (the module include, `run_env`,
  `mobile` passing the filter, `fonts` validating coverage); `docs/ROADMAP.md` (3b shipped bullet, the font
  row, M10's stale "craft-update still open" sentence fixed — it shipped as item 2c on 2026-11-24);
  `docs/ARCHITECTURE.md` §8.3, `docs/ATTRIBUTION.md` (the trim and the strengthened refusal),
  `docs/RELEASE.md`, `docs/UX.md` (what 3b still lacks is now two items, not three).
- **Tests that fail when the behaviour is absent** (11 new: nine in `font_manifest` — parse, no-filter
  keeps everything, the phone filter drops the Hangul face, the typo refusal, the no-face refusal, the
  filter value's grammar, the coverage validation both ways, and the phone script set's definition — plus
  two in `fonts`: the shipped-language table matches the picker, and a filtered build names every script
  the interface ships). `cargo xtask fonts` refuses a set that misses a script the interface ships, proved
  on the fixture.
- **Verified.** `cargo test -p sortlens-ui --lib` (173), `-p xtask` (42, the module's tests run there too).
  `cargo xtask ci` green through every gate (two fmt reformats and one clippy `manual_contains` + a
  never-actually-loops loop fixed along the way). `cargo xtask snapshot` green, no golden moved.
  `cargo xtask screenshots` green, 23 scenes, no baseline moved — the phone's paint is untouched; only what
  a phone *build* embeds changed. `cargo xtask deny` green. `cargo xtask mobile` still needs the NDK and
  Xcode, as documented.
- **Next.** The iOS Photos library (iOS still reads the Files-visible Documents folder) and a device
  build; both wait for a machine with the NDK, Xcode and the iOS Photos framework. That is everything
  buildable left in the owner's Phase 0 → Phase 5 plan — the remaining 3b "Still open" items are the
  device verification itself, the APK/AAB and `.ipa` workflow legs (whose signing halves are the same
  Apple-identity blocker the M10 table already names), and the post-1.0 backlog rows.

## 2026-11-24 (sixty-ninth) — the phone asks whether the desktop is there before a code is spent (SL-J07 from the app)

- **Why this came up.** The sixty-eighth entry ended on the one thing the pairing flow still asked
  a person to do blind: type the six-digit code into an address that may answer nothing, and learn
  from the pairing's failure alone that the address was wrong — after spending the code, which
  expires, on a mistake a `GET /health` would have named in one sentence.
- **The test runs on the address alone.** Settings → Send to desktop gains **Test connection**, placed
  between the port field and the code field, because it is what the code is for. The typed address and
  port — nothing else — run it: `Client::health()`, no token, on its own `sortlens-check` thread, the
  same pattern the pairing already uses, so no frame blocks. The sentence it earns names the address it
  tested, which need not be the paired one (`The desktop at 192.168.1.10:52114 answered`, or `No answer
  from …` with the reason), and it is painted in the section and said as a notice. Nothing is stored
  either way — the pairing's own answer stays the only truth about the address — and a successful
  pairing clears the sentence. While it waits the section says `Testing host:port…` and the button
  stands down; a pairing or a send run in flight answers a test with nothing rather than piling threads
  on the same server; Forget clears it with the rest.
- **Files.** `apps/sortlens-mobile/src/send.rs` (`Check`, `DesktopLink::test`/`poll_test`/`checking`,
  the surface's `checking`/`check`); `apps/sortlens-mobile/src/lib.rs` (`Action::SendTest` arm, the
  poll); `crates/ui/src/shell.rs` (`SendSurface::checking`/`check`, `Action::SendTest`, the section's
  button and its two sentences); `crates/ui/src/i18n/{ar,ja}.tsv` (`Test connection`);
  `apps/sortlens/src/main.rs` (the desktop host's honest answer to a phone-only action);
  `docs/ROADMAP.md`, `docs/UX.md`, `docs/ARCHITECTURE.md`, `docs/FEATURES.md` (SL-J07), `docs/RELEASE.md`.
- **Tests that fail when the behaviour is absent** (4 new: three in the phone's link, the shell's send
  test extended). Link, over real TCP: a fake `/health` server answers and the sentence says so while
  `config()` stays empty and `paired` stays false; a refused port earns `No answer from …` with its
  reason, and stores nothing; a pairing in flight means a second test is never spawned (and the pairing
  still lands). Shell at 390 points: the test runs on the typed address and port with the code field
  empty, hands over both trimmed; an addressless test answers nothing; a test in flight paints
  `Testing 192.168.1.10:52114…` and is not re-run; the answered sentence is painted.
- **Verified.** `cargo test -p sortlens-mobile` (20 passed), `-p sortlens-ui --lib` (163). `cargo xtask
  ci` green through every gate (the desktop host's `Action` match needed the new arm — a phone-only
  action is answered, never dropped — and clippy's fmt pass reformatted the test). `cargo xtask
  snapshot` green, no golden moved. `cargo xtask screenshots` green with 23 scenes and no baseline
  moved (the section paints only in the phone's unpaired Settings, which no committed baseline holds).
  `cargo xtask deny` green. `cargo xtask mobile` still needs the NDK and Xcode.
- **Reconciled.** ROADMAP 3b's shipped bullet names the connection test and its tests; UX §2's Send
  rules name the Test connection step and what it stores; ARCHITECTURE §10 says the surface runs
  `GET /health` before a code is spent; FEATURES SL-J07 moved the settings screen out of "open with the
  client"; RELEASE's phone bullet says the address can be tested first.
- **Next.** The iOS Photos library (iOS still reads the Files-visible Documents folder) and a device
  build; both wait for a machine with the NDK, Xcode and the iOS Photos framework. The link itself is
  now measured, paired and tested — nothing on it is blind any more.

## 2026-11-24 (sixty-eighth) — the phone sends its photographs (SL-J04/J05 from the app)

- **Why this came up.** The sixty-seventh entry ended on the one thing the phone's own screen still
  could not do: the app could index a camera roll and paint a phone's canvas, but nothing on it could
  hand a photograph to the desktop. The wire was already there (`sortlens_remote::client` and its
  loopback harness), the desktop had its Sync screen, and the roadmap's "Still open" named exactly
  "a send-to-desktop action over the resumable upload". A phone that indexes a camera roll but cannot get
  a photograph off it is still not a remote.
- **The queue is one thread the interface polls.** `sortlens_remote::send::SendQueue` spawns one worker
  (`sortlens-send`) that health-checks once, reads each file, uploads it with `Client::upload`, and publishes
  a `SendOutcome`: sent, kept and removed counts, per-file failures, cancelled, and the one connection error
  that ends a run instead of failing every remaining item after its own timeout. `report()` is what a frame
  paints; `cancel()` asks it to stop between photographs, leaving a half-sent chunk as a server-side partial
  the next run resumes; `take_outcome()` hands the result over once. A thread that cannot start finishes the
  run at once with the reason, rather than a queue that claims to be running forever with nothing behind it.
- **The ack's remove is named, not guessed.** `SendOutcome::to_remove` lists the device ids whose ack said
  `LocalCopy::Remove`. The queue deliberately does not delete: it holds a path, not the library that can audit
  a file. On the phone the ids go to `Engine::delete_photos(..., Trash)` — the same audited path the desktop
  applies a cull with — which re-checks each file's size and mtime, refuses the batch if one moved, flags the
  photographs rejects, keeps the undo batch, and moves them to the phone's trash. "Remove" therefore means
  what the checkbox says, and an unreachable desktop or a refused batch leaves the file where it is.
- **The link keeps the address its token belongs to.** `apps/sortlens-mobile/src/send.rs`: `DesktopLink` holds
  the address, token, recorded device and the run. `pair` runs the handshake on its own thread; `poll_pair`
  peeks at the answer slot and takes the attempt only once an answer is there — the first frame after the tap
  usually polls before the worker has connected, and an eager `take` drops the handle and loses the
  answer forever. A successful answer writes the typed host and port together with the token, so the settings
  document can never hold a token pointing at `:0`; a failed one leaves the previous pairing untouched.
  Forget cancels a run in flight.
- **The phone's and the desktop's sections never share a screen.** `send_section` had been painted above the
  desktop's Sync & devices section on a phone, whose host answers every server action with "not available on
  the phone yet". Sync is now desktop-only and Send is phone-only. Forget is disabled while a run is in
  flight, because dropping the token mid-run would leave the worker sending under a pairing the phone no
  longer keeps. The hint says what Remove actually does, and the sentence a finished run earns says what the
  ack cleared (`… 1 to remove`), with the app appending what it could carry out (`; removed 1 from this
  phone`, or the reason it could not).
- **Files.** `crates/remote/src/send.rs` (new); `apps/sortlens-mobile/src/send.rs` (new), `Cargo.toml`
  (`sortlens-cull`), `src/lib.rs`, `src/prefs.rs`; `crates/ui/src/shell.rs`, `crates/ui/src/lib.rs`,
  `i18n/{ar,ja}.tsv`; `docs/ROADMAP.md`, `docs/UX.md`, `docs/ARCHITECTURE.md`, `docs/FEATURES.md`,
  `docs/RELEASE.md`.
- **Tests that fail when the behaviour is absent** (14 new: five in `sortlens_remote::send`, seven in the
  phone's link, one in the phone's settings, one in the shell). Remote, over a real-TCP fake upload server:
  both keep and remove land with `to_remove == [2]`, a missing file fails alone, progress counts items as
  they finish, cancel stops between
  items, and an unreachable desktop ends the run with one reason. Phone: a successful `/pair` through a real
  socket keeps the address with the token (the old poll loses the attempt); a closed connection stores
  nothing; the ack's remove choice reaches the library through `delete_photos` while a keep-only run touches
  nothing; the items come from the catalog, missing files skipped; an older settings file opens unpaired.
  Shell at 390 points: Pair is disabled until all three fields are full, Pair and Send hand over the typed
  values and the selection with the toggle's answer, Send refuses an empty selection, Stop hands over a
  cancel, Forget answers nothing during a run, and each width paints only its own section — the phone no
  Sync & devices, the desktop no Send to desktop.
- **Verified.** `cargo test -p sortlens-remote` (36 passed), `-p sortlens-mobile` (17), `-p sortlens-ui --lib`
  (163). `cargo xtask ci` green through every gate; the first run failed clippy on a `needless_borrow` in a
  test and on the desktop host's non-exhaustive `Action` match (both fixed here), and the shell test then
  caught its own assertion matching the hint's "Sync & devices" wording, so it now compares section headings
  exactly. `cargo xtask snapshot` green. `cargo xtask screenshots` green with 23 scenes and no baseline
  moved. `cargo xtask deny` green. `cargo xtask mobile` still needs the NDK and Xcode, which this checkout
  does not have.
- **Reconciled.** ROADMAP 3b gained the shipped bullet and its "Still open" no longer names send-to-desktop.
  UX §2 has the Send rules and no longer lists it among the lacks. ARCHITECTURE §10 says the phone drives
  `sortlens_remote::send` and carries removals through the audited delete path. FEATURES SL-J02 dropped
  "uploading from the app" from Still open; SL-J04 and SL-J05 name the app half. RELEASE says the phone can
  use the remote half and describes the phone's canvas as its own.
- **Next.** The iOS Photos library (iOS still reads the Files-visible Documents folder) and a device build; on
  the link itself, a connection test (`/health`) before the code is typed.
## 2026-11-24 (sixty-seventh) — the canvas is the phone's (SL-J10, Phase 4 item 3b)

- **Why this came up.** The sixty-sixth entry gave the phone a frame — docks put away, a wordless bar — and
  named what was still the desktop's: the canvas header, the filter strip and the bottom bar's tray. Measured
  at 390×780 with a populated library, three numbers said how far off it was: the far-edge rail painted four
  glyphs at x 364–378 beside a *closed* inspector, so a tenth of the screen went to a tool the bar's own
  inspector glyph offers; the filter strip wrapped into three rows because its reserved count column
  ("6 shown", the same number the header already prints) kept the chips' room down to 297 points; and the
  bottom bar's pill ran to x 402 — the last flag button half off the edge and untappable.
- **The rail stands down.** `right_tools` is painted on a phone only while the inspector's sheet is open: the
  closed panel no longer pays its 30 points, and the open sheet keeps the rail as its tab switcher — which
  also settles the earlier "close-then-reopen" note in the roadmap: the tap test drives a click on the rail
  beside the open sheet and switches Details → Keywords without closing it. `left_sheet_width` stops
  subtracting `TAB_STRIP_W`, so the sidebar sheet is the width of the screen (the rail belongs to the
  inspector, and the two docks never share a screen).
- **The strip is two rows.** On a phone the strip keeps no count (the header's is the same number on the same
  screen), its chips sit 4 points apart, and its card margin is 6: at 390 the eight chips plus `Clear` wrap
  into two rows. The canvas's own gutter narrows from 14 to 10 points. The strip's card now ends at y 150 —
  the chrome before the first photograph, against ~190 before.
- **The tray fits.** The pill's buttons get 2-point padding and 3-point gaps on a phone, the keyword count
  steps aside (the inspector's Tags tab is one tap away), and the density slider yields to the header's own
  glyph — the same control, so the bar cannot disagree with the header. Everything the tray paints is inside
  the screen.
- **Files.** `crates/ui/src/shell.rs`: the `right_tools` call condition in `show`, `left_sheet_width`, the
  canvas gutter, the strip's narrow branch (no count, tighter chips and margins), the pill's narrow branch
  (spacing, padding, no keyword count), and the bottom bar's slider condition.
  `crates/ui/tests/snapshots/phone-charcoal.png` and `phone-sidebar-charcoal.png` regenerated; no other
  baseline moved.
- **Tests that fail when the behaviour is absent** (3 new, 162 in the crate): a populated 390-point canvas
  proves the rail's column empty, the strip's card at its full width ending by y 152 with exactly two chip
  rows and no second count, every bottom-bar picture and text inside the screen (the old pill fails by
  thirty points), and the header intact — name, count, sort combo, both grid glyphs; a 900-point window
  keeps its rail (one glyph per tool), its strip summary and its pill; the rail beside the open sheet
  switches the inspector's tab. The strip-summary sweep now mirrors the shell: absent under 600 points,
  measured and collision-free above it.
- **Verified.** `cargo xtask ci` green through every gate. `cargo xtask snapshot` green, no golden moved.
  `cargo xtask deny` green. `cargo xtask screenshots` green with 23 scenes: the two phone baselines
  regenerated and read — three photo columns where there were two, a two-row strip, the tray inside the
  screen, the sidebar sheet the width of the screen — and the other 21 byte-identical, the check that no
  width from 900 up moved. There is no `cargo xtask wasm` in this workspace; the portability gate is
  `cargo xtask mobile`, still for a machine with the NDK and Xcode.
- **Reconciled.** ROADMAP 3b gained the shipped bullet and its "Still open" now names only what a phone
  still lacks (device build, font trim, iOS Photos, send-to-desktop, packaging). UX §2 has the phone-canvas
  rules. ARCHITECTURE §10 says the canvas is the phone's too. RELEASE's phone bullet no longer says the
  phone paints the desktop's layout. FEATURES SL-J02 moved the canvas out of "still open".
- **Next.** Send-to-desktop (SL-J04/J05 from the app): a share-or-send action in the phone app that queues
  the device's photographs to the paired desktop over the resumable `/upload` with keep-or-remove, surfaced
  in the app's own UI.

## 2026-11-24 (sixty-sixth) — the phone gets its own frame (SL-J10, Phase 4 item 3b)

- **Why this came up.** Yesterday's entry ended on the honest gap: the phone app paints the desktop shell, and
  the roadmap said the shell "demanded 880 px". The number was in three documents and in no constant. Measured,
  a 390-point window painted a 264-point sidebar, the 30-point rail and a 296-point inspector, so the canvas —
  the grid, the reason the app exists — was given a negative width, while the bar ran eight glyphs and a
  240-point field off the right edge. A phone saw the desktop squeezed and nothing to look at.
- **The measurement.** `NARROW_BELOW = 600.0` in `crates/ui/src/shell.rs`, read once per frame before anything
  paints into `UiState::narrow`, so the bar, the docks and the canvas answer the same number on the same frame.
  600 is wider than every phone in portrait (390–440 points) and narrower than every window the committed
  baselines record (900 and up) — which is why none of them moved.
- **What a phone frame is.** As a window becomes that narrow the docks are put away, so a window that opens on a
  phone opens on the grid; afterwards a person's taps win over the measurement (`toggle_left`, `toggle_right`,
  `open_right`). The bar keeps the mark and gives up its two words; the search field takes the room the five
  remaining glyphs leave, floors at 140 points and is left out below that; its hint stops naming `Ctrl+K`,
  because a phone has none; the keyboard overlay and the appearance popover step aside (one is about a keyboard,
  both live in Settings); the read-only warning becomes the same sentence in a glyph's hover. A dock that opens
  is a sheet the width of the screen, the canvas is not painted behind it, and the two docks are one-at-a-time
  by construction — a sheet needs no close button because the bar's glyph is the way back.
- **Files.** `crates/ui/src/shell.rs`: `NARROW_BELOW`, `SEARCH_FIELD_NARROW`, `UiState::narrow` with the three
  dock helpers, the narrow branches of `top_bar`/`left_panel`/`right_panel`, `left_sheet_width`, and
  `search_field` taking the room it may use. `crates/ui/tests/screenshots.rs`: the two phone scenes.
  `crates/ui/tests/snapshots/phone-charcoal.png` and `phone-sidebar-charcoal.png`: the new baselines.
- **Tests that fail when the behaviour is absent** (3 new, 159 in the crate): a 390-point frame opens with both
  docks put away, no "Sortlens"/"Local library" words in the bar, the Welcome card painted — and every glyph and
  text rect of the top bar inside the screen, which the old bar failed by roughly 300 points; each dock opens and
  closes from its own glyph, one at a time, the sidebar painting its sections and the inspector its Details tab;
  a 900-point window keeps both docks and both words.
- **Verified.** `cargo xtask ci` green through every gate (fmt, clippy, workspace tests, layers, roots, assets,
  fonts, version, attribution, a11y, docs, templates, workflows). `cargo xtask snapshot` green with no golden
  moved. `cargo xtask deny` green (advisories, bans, licences, sources). `cargo xtask screenshots` green with 23
  scenes: the two phone scenes are new, and the 21 earlier baselines are byte-identical, which is the check that
  nothing at 900 points or wider moved. The two new images were read: the phone frame is a bar with a field and
  five glyphs over the grid, and the sidebar sheet covers the screen with the rail still beside it. There is no
  `cargo xtask wasm` in this workspace — the string does not occur anywhere under `xtask` — so the wasm gate the
  plan names has no task here; the portability gate for this change is `cargo xtask mobile`, still for a machine
  with the NDK and Xcode.
- **Reconciled.** ROADMAP 3b gained the shipped bullet, and its "Still open" now names what a phone still lacks
  (the canvas header, filter strip and bottom-bar tray at that width; switching the inspector's tab while its
  sheet is open; the font trim; the iOS Photos library; send-to-desktop; packaging) instead of claiming the shell
  is "still the desktop's, 880 px minimum". UX §2 has the phone-width rules. ARCHITECTURE §10 says the shell
  answers a phone's width itself, so the phone app inherits it. RELEASE's phone bullet says the frame is there and
  the canvas inside it is not. FEATURES SL-J02 moved the phone layout out of "still open" and kept the canvas.
- **Next.** The canvas at a phone's width: its header (name, count, sort combo, grid glyphs), the filter strip
  (two rows of chips, about 200 points before the first photograph at 390) and the bottom bar's control tray. A
  frame that shows a grid but spends its first quarter on desktop chrome is half a phone layout.

## 2026-11-24 (sixty-fifth) — the phone remembers what you chose (SL-J10, Phase 4 item 3b)

- **Why this came up.** The phone's `App` was handed a `data_dir` and spent it on the catalog and the
  thumbnail cache only. Every launch opened Charcoal at the shell's default density, and the Appearance
  menu's taps went nowhere: `Action::SetDensity`, `SetGrid` and `SetMotion` shared an empty arm with
  the actions a phone genuinely cannot do. A phone app that forgets the theme you picked is not a phone app.
- **The read.** The document already exists on the desktop (`apps/sortlens/src/settings.rs`): two sections,
  `[appearance]` with the four ids and `[library]` with the order, and `craft-settings` owns the hard
  parts — atomic save, backup, damaged-file recovery, unknown-key preservation. None of that is
  desktop-shaped; only the *location* is. `Location::Auto` wants a platform config folder a phone does not
  have, so the phone names its own directory and inherits every rule: `config.toml` in the app's private
  data directory, beside `Library/` and `Cache/`, never inside them.
- **The document.** `apps/sortlens-mobile/src/prefs.rs`: `PhonePrefs` — the desktop's two sections cut to
  the five choices the phone's shell can make (theme, density, layout, motion, order), with the same ids and
  the same defaults. `open` is `SettingsStore::open_at(data_dir)`; `notice` repeats the desktop's
  sentence only when the load report has something to say; `apply` reads each id the way the desktop reads
  it, an unknown one landing on its default; `record` is the one place an `Action` becomes a stored id,
  kept out of `App` so the mapping is testable without a window.
- **The application.** `App` owns the store: it opens before the first frame, so that frame is already the
  saved theme and order; every appearance or order action is written before the frame shows it; `tick` fires
  the debounced save each frame and schedules a repaint for its deadline (the desktop's own rule, and the
  reason an OS that ends the process mid-session loses nothing already tapped); a clean exit flushes.
  `SetDensity`, `SetGrid` and `SetMotion` now change the shell instead of being ignored.
- **Tests that fail when the behaviour is absent** (7 new, 10 in the crate): the defaults are the desktop's
  own and land on the shell's own defaults; every id the shell offers — 5 themes, 3 densities, 2 layouts, 2
  motions and 7 orders — round-trips through the real file one variant at a time; a test drives
  `Action::SetDensity(Compact)`, `SetGrid(Square)`, `SetMotion(Reduced)`, `SetSort(Newest)` and
  `SetTheme(Paper)` through `record`, flushes, reopens and reads the shell back, while `Action::Notice`
  and `Action::PickFolder` must write nothing (a missing arm fails); an unknown id is not a damaged file; an
  `[experimental]` key written by a later build survives a save this build makes; a damaged file is moved
  aside and `notice` says so while a clean file says nothing; the file sits outside the library directory.
- **Verified.** `cargo test -p sortlens-mobile` is 10 passed. `cargo xtask ci` green through every gate
  (fmt, clippy, workspace tests, layers, roots, assets, fonts, version, attribution, a11y, docs, templates,
  workflows); `cargo xtask snapshot` green with no golden moved; `cargo xtask deny` green. No paint
  changed, so no baseline was regenerated and `cargo xtask screenshots` was not needed. The new dependency is
  the only part that could have broken the phone targets this checkout cannot link:
  `cargo check -p craft-settings --target aarch64-linux-android` and `--target aarch64-apple-ios` both
  finish clean (dirs has an explicit path for each, and the phone passes an explicit directory anyway), so the
  settings crate is portable; `cargo xtask mobile` itself is still for a machine with the NDK and Xcode.
- **Reconciled.** ROADMAP 3b moved settings persistence out of "Still open" into a shipped bullet and left the
  rest of the row open; ARCHITECTURE §10 says where the phone's document lives and which rules it keeps;
  RELEASE's "No remote" bullet was stale — the server, pairing and the phone app shipped yesterday — and now
  says what exists and that nothing is packaged for a phone; its "prototype-grade layout / Phase 3" bullet,
  which still called Phase 3 ahead, now names the one layout gap that is real: the phone paints the desktop
  shell.
- **Next.** Phase 4 item 3b's remainder, in order of value here: a phone layout (the shell still demands
  880 px, which no phone has), a send-to-desktop action over the resumable upload (the client is built and
  tested; the phone needs a paired address and a button), trimming the embedded font set for a phone binary,
  the iOS Photos library, and an APK/AAB plus an `.ipa` in the release workflow.

## 2026-11-24 (sixty-fourth) — the shell taught the wrong key, and every dialog opened in the corner

- **Why this came up.** The sweep stayed on the top bar and then opened what it had not seen live. The
  search field's hint said `Search photos or ⌘K` on Windows, where the key is Ctrl+K; under a 10× crop the
  ⌘ was painted over the K rather than before it, because the glyph is egui's own and its advance does not
  match the sample's. The palette opened cleanly (grouped rows, filtering, the shortcut column) — and
  About opened half off the canvas: its rounded corner cut at the screen edge, its title bar under the top
  bar, the paragraph's first letter clipped. Settings was in the same corner, title bar under the bar.
- **The read.** egui stacks a window it has never seen at the available area's top-left, and nothing in the
  shell had ever said otherwise: the Alerts popover is the only window with a position of its own, and it
  uses `anchor(RIGHT_TOP)` deliberately. The About box was simply the first dialog the audit opened, so it
  was the one that showed the corner to be wrong.
- **The paint.** The hint's key now comes from `palette_hint()`: `Ctrl+K` off a Mac, `⌘K` on one, so the
  words teach the press the platform actually runs and Windows never draws the ⌘ over its letter. The
  field keeps measuring itself from the hint and gives the words up in a stated order (key, then `or …`,
  then `photos`) — on Windows that costs a few points, which is why the field is 3×700 wide in the scenes
  and every baseline moved by the same 105 pixels. Every dialog now goes through `dialog_window()`, which
  anchors it `CENTER_CENTER`: the window measures itself and is centred exactly, at 1440×900 and at
  900×620 alike, and stays centred. The cost is that a dialog cannot be dragged aside; for a surface that
  closes on Escape or its own button that is no loss, and it is stated in `docs/UX.md`.
- **One more, on the next screen.** Insights draws its bars with the count right-aligned at the row's
  end, which is *inside* the track: a full bar — all images, a rating everyone shares, `Images 7` —
  carried its number on the accent colour in `text_faint`, the worst pairing the palette has. The track
  now stops 26 points short and the number keeps that room, so every bar row (ratings, labels, kinds,
  the breakdowns) reads its count against the background.
- **Tests.** `cargo test -p sortlens-ui --lib` is 156 passed; `the_search_hint_shrinks_instead_of_being_cut`
  now expects the Windows spelling in the roomy case. `cargo xtask screenshots` is 21 scenes regenerated
  and green — the twenty-one were all touched by the field's width, the four dialog scenes (`shortcuts`,
  `settings-short`, `settings-sync`, `shortcut-editor`) now paint centred, and `insights-*` shows every
  count clear of its bar. `cargo xtask ci` is green, and `cargo xtask wasm` was green too, hours before the
  browser page and its task were removed.
  One test changed shape: `clicking_the_detail_photograph_toggles_its_zoom` now waits for the full-size
  decode, up to ten seconds, instead of counting three frames. On a loaded box the decode lost that
  race and the view was still painting its thumbnail, so the gate would have called a slow machine a
  defect. The wait is bounded and the assertion is unchanged.
- **Verified live, after the rebuild.** At 1280×800 the field reads `Search photos or Ctrl+K` whole, with
  no overlap; Ctrl+K opens the palette under the field, `exp` narrows it to the two Export rows and Enter
  runs the first; About opens centred with its whole paragraph, and the palette and About are both clear
  of the dark scroll bars that the corner placement had hidden them under.

## 2026-11-24 (sixty-third) — the chrome that only holds at one size: a toast with nowhere to sit, a footer cut by the bar at every window, and a compare's names running under its buttons

- **Why this came up.** The live sweep that produced the last checkpoint kept reading 1280×800, then 900×620, and
  found three more places where the layout was exact rather than measured: a transient message painted
  through the annotation pill, the rows under the Detail photograph overflowed the bottom bar at every
  size — including the 1440×900 baselines — and a compare's two file names plus its "2 of 7" ran under
  the button cluster, which painted the position as a stray "2" between the names.
- **The read.** (1) At 1280 the bottom bar is full end to end — tabs 0–707, the annotation pill 726–1102,
  the right cluster 1104–1270 — so a message had about 54 points of its own. (2) The footer rows (facts,
  Previous/Next, keywords, suggestions, histogram with lookahead, the key hint) were budgeted at a
  guessed 118 points; the real rows are always taller, so the photograph took their room and the bar cut
  the hint off. (3) `Sides` with `truncate` bounds left-to-right, not piece by piece: the first name spent
  the whole row on itself and pushed its siblings past the block, still painted because the clip belongs
  to the parent.
- **The paint.** The notice is now an `Area` centred above the bar (`Order::Middle`, no click taken, one
  line, the full text when it fits), which is also clear of the Details dock a corner toast would sit on.
  The detail view measures its footer after the rows paint (`UiState::detail_footer`, the same one-frame
  lag `settings_height` already pays) and lets the photograph take exactly what is left; when that
  measurement changes it asks for a repaint, because the app goes idle after a view switch and would
  otherwise keep the fallback layout until the next input. A short window (under 700 points) drops the two
  decorative rows — suggested keywords and the histogram strip — so the hint row still lands above the bar;
  the test is the window, not the measured footer, which would flip the layout back and forth as the
  measurement followed the decision. The header sets each name's share before laying it out, with the
  arrow, the position and the item gaps taken out of the budget first, and degrades in a stated order:
  below 720 points the conveniences the palette also offers (Open, Show in folder, Export…) go, below 420
  the zoom toggle and Swap go, and below that the names take a row of their own with the buttons under
  them.
- **Tests.** `cargo test -p sortlens-ui --lib` is 155 passed. New claims: the notice floats above the bar
  rather than through the selection count; the detail view keeps its last row above the bar at 1440×900,
  1280×800 and 1100×700 (the old 118-point budget panics "the view paints no key hint at all"); and
  `a_compares_names_stop_where_its_buttons_start` walks 1440, 1280, 1100, 980 and 900, asserting that every
  text left of the button cluster ends before it starts, that the conveniences appear at 1440 and are gone
  below, that the position survives at 1280, and that under 1100 the names get a row of their own wider
  than a letter. `cargo xtask screenshots` is 21 scenes, green; the four detail/compare baselines were
  regenerated and now show the whole footer. `cargo xtask ci` and `cargo xtask wasm` are green.
- **Verified live, after the rebuild.** At 1280×800 a rating notice appears centred above the bar clear of
  the pill; Detail shows its names, `Compare C`, `1:1 Z`, `Close Esc` and the whole footer; Compare shows
  `IMG_0004.png ↔ IMG_0005.png 1 of 7` with no overlap. At 900×620 the bar collapses to its destination
  menu (nine items, caret included), Compare puts the names on their own row above two buttons, and the
  compact footer ends with the key hint above the bar instead of under it.

## 2026-11-24 (sixty-second) — the sweep found what a static baseline hides: bars you could not see, insights columns that did not fit, a Settings dialog taller than the window, and Esc that only sometimes closed it

- **Why this came up.** The suite has baselines for every view, but a baseline proves a frame paints, not
  that it can be read or reached. This pass opened the live app over the QA library at the size a laptop
  actually gives it (1280×800) and read each view and dialog as a person would: For You, Insights,
  Soft Focus, Compare, Places, the Settings and Recently Deleted dialogs, and the keyboard.
- **The read.** Three defects a screenshot at 1440×900 could not state. (1) The Insights bars were
  `#3A311E` on `#151515` — 1.4:1, effectively invisible: `columns()` filled with `t.selection`, the
  selected-row tint, instead of the data colour. (2) The For You year labels collided once the filter row
  reached four digits, because every column was a fixed 16 points wide. (3) Settings is ~1265 points
  tall, so at 800 its tail — Updates, Sync & devices, the keyword packs — ran past the bottom of the
  window with no scrollbar and no way to reach it. Plus two smaller ones: the receipt dialog stayed on
  screen after its own close, and Recently Deleted's empty state said "Nothing has been applied yet".
- **The paint.** Charts grew `bar_fill`: the accent at full strength for the highlighted column and an
  85/15 accent-on-panel blend otherwise, so every preset clears 3:1 against the surface (Silver was the
  tight case at 0.75 before the blend and clears it after); `share_row` uses the full accent, which clears
  3:1 on every preset's `thumb_bg`. Columns widen to their label (`column_width` measures the galley), and
  the label colour follows the highlight (`t.text` / `t.text_faint`). Insights lays its three columns out
  with one `horizontal_wrapped` row instead of a fixed 3× layout that overflowed the canvas at 1280.
  Settings now measures its own body on the frame that paints it (`UiState::settings_height`, read from
  the scroll area's `content_size` and not from the scrolled `ui.min_rect()`, which reports the viewport
  and would flip the ceiling on and off every frame) and, past a 96-point screen ceiling, caps the window
  and scrolls the body with the bar always visible. A screen tall enough for the whole dialog keeps the
  auto-sized window, so the 1700-point Settings baseline is untouched. Escape dismisses the front-most
  window through `dismiss_top_window`, one press one window, in the reverse of the paint order.
- **Tests.** `cargo test -p sortlens-ui --lib` is 152 passed, including new claims: the four-digit label
  widens its column instead of painting over its neighbour, chart bars hold 3:1 against their surface on
  all five presets, the Insights columns fit between the docks at any width the window can have, Escape
  unwinds the dialogs one press at a time, and the settings section tests (AI, Updates, Sync, language)
  read their sections on a screen tall enough to hold them — the short case is the new baseline.
  `a_short_window_scrolls_the_settings_dialog_in_both_themes` shoots Settings at 900×620 in both themes
  (lenient, for the language row's own scripts); `cargo xtask screenshots` is 21 scenes, green, and only
  the two new `settings-short-*` files plus the four chart baselines changed.
- **Verified live, after the rebuild.** At 1280×800 Settings opens capped, the wheel scrolls it to
  Updates → Sync & devices → Keyword packs with the bar in sight, Esc closes it, and Recently Deleted
  reads "Nothing has been deleted yet." with its cull-batch note.

## 2026-11-24 (sixty-first) — the wasm shims are gone: no target, no gate, no dead branch

- **Why.** With no wasm target installed, no task that builds one and a written non-goal, every
  `cfg(target_arch = "wasm32")` branch and every doc comment about "the wasm client" was dead weight
  that read as an invitation. A reader should not have to work out whether a browser path still runs.
- **What changed.** The stubs, the paired `cfg(not(...))` attributes and the browser-only comments are
  gone from core, catalog, cull, meta, thumbs, ai, engine, ui and remote; three manifests'
  `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` sections became ordinary `[dependencies]`
  (the same crates, the same builds). The layer check now states the exception it always had: the
  engine takes `craft-update` with default features — no `egui` — for the update check.
- **Verified.** `cargo xtask ci`, `snapshot` and `deny` green; no source file contains `wasm`.

## 2026-11-24 (sixtieth) — the intake the agent wrote stays; the browser page it also wrote does not

- **Why.** Phase 5's intake and playbook landed on `main` while this branch was removing the web
  application and adding the phone app. Both slips are wanted; the browser page is not.
- **Kept whole.** The issue forms and PR template, `docs/AUTONOMOUS_AGENT.md`, `cargo xtask intake`,
  `cargo xtask workflows`, `xtask/src/yaml.rs`, both workflows and their tests. The one change the
  merge needed: the fifth gate is `mobile`, not the removed wasm task — in `xtask/src/workflow.rs`,
  `ci.yml`, the PR template, the playbook and the Phase 5 roadmap row.
- **Dropped.** The page's upload surface and the files it lived in, the browser transport, the host's
  page route and its flag, and the harness test that read them. The one-shot upload regression the
  page found stays, with its test, because the defect was the server's.
- **Verified.** `cargo xtask ci`, `snapshot`, `deny` and `workflows` after the merge, on this checkout.

## 2026-11-24 (fifty-ninth) — Phase 5 closed: the playbook is enforced, the intake reads a stranger's report without obeying it, and the fifth gate runs on every PR

- **Why this came up.** Items 2 and 3 of Phase 5 were the last unchecked rows in the completion
  plan. They ship as one slice because the repository forbids docs-only commits after Phase 0: a
  playbook with no workflow is prose nobody checks, and a workflow with no written contract is a
  gate nobody can read. Every rule below exists so that the promise and its enforcement land
  together. The parent rows that had been left unchecked while their children shipped (M8 item 3,
  M10 item 2) were reconciled in the same commit, because otherwise Phases 0–5 could never read as
  closed.

- **The playbook** (`docs/AUTONOMOUS_AGENT.md`, 145 lines) is the narrow operating contract for an
  agent that reads issues and opens pull requests unattended. It draws the boundary first: a report
  is untrusted data, never instructions — no command, install, link or file is obeyed because an
  issue asks, and the reason the intake passes the body through the environment is written down as
  the same rule. Only an `agent-ready` issue is picked up; `triaged` is a statement about attention
  (the forms apply it themselves, the intake removes it when structure is missing), not pickup
  permission, and a `needs-info` issue is never implemented. The never-list is explicit: never push
  to `main`, never auto-merge or self-approve, never bypass `cargo xtask deny` or another gate,
  never read or pass a signing or release secret, never publish, force-push or spend. The
  repro-first loop is ordered and finite — read the evidence, reproduce and write the failing test
  that is the proof it bites, minimal fix, reconcile the documents in the same commit, run the
  gates, open a **Draft pull request**, stop. The improvement queue is bounded (`sortlens-perf`, then
  `enhancement-approved`, then unchecked roadmap rows, one slice per PR, empty queue means stop),
  and the stop rules say when trying again would be noise: a red tree not understood, an external
  secret, or two runs shipping nothing.

- **The intake checker** (`xtask/src/intake.rs`, +178 lines): `cargo xtask intake` reads
  `SORTLENS_ISSUE_BODY` and `SORTLENS_ISSUE_LABELS` from the environment, decides from the labels
  which form the report came through (a report that is neither a bug nor a feature is not judged),
  and reads the required headings **from the form itself** rather than from a second copy that
  would drift. Each missing heading and each empty section is named; `_No response_` counts as
  empty, and `Diagnostics` must still contain `sortlens`, so a sentence is not a bundle. Exit 1
  prints the comment body the workflow posts. Six new tests cover the reading of headings from the
  form, a complete report, a missing section, an empty section, a diagnostics sentence, and an
  unlabelled issue.

- **The workflow gate** (`xtask/src/workflow.rs`, new, 368 lines): `cargo xtask workflows`, wired
  into `cargo xtask ci`, reads both workflows with the same small YAML-subset reader `templates`
  uses. It refuses a `ci.yml` with no `pull_request` trigger, a missing one of the five gate
  commands (`ci`, `snapshot`, `deny`, `mobile`, `screenshots`), a `continue-on-error` step, or an `if`
  that would leave pull requests out. It refuses an intake that does not run on opened/edited/labeled
  issues, that uses `pull_request_target`, that reaches a `secrets.` value, that cannot write issues,
  that does not stay off pull request events, that does not pass both environment variables, or that
  lost the comment / `--remove-label triaged` / `--add-label needs-info` bookkeeping. And it holds
  the playbook to ten load-bearing phrases, so a rule can be rewritten but not removed. Ten tests,
  each proved by mutating a fixture (a removed gate, a weakened step, a skipped job, a lost
  trigger, a secret, a dropped label command, a deleted rule) until the checker bites.

- **`ci.yml` was rewritten in block style** (the reader refuses flow lists, so `branches: [main]`
  and the `os:` flow list became block lists) and gained the fifth job: `screenshots` on ubuntu with
  Mesa's software Vulkan (`mesa-vulkan-drivers`, `WGPU_BACKEND=vulkan`, `LIBGL_ALWAYS_SOFTWARE=1`),
  so the 33 painted baselines are a real pull-request gate rather than a local hope. The existing
  jobs, cache keys and the per-platform conditions are unchanged.

- **`agent-intake.yml`** triggers on issues only, holds `contents: read` and `issues: write` and
  nothing else, has no path to a repository secret, and refuses pull request events. It runs
  `cargo xtask intake` and posts the returned body as a comment; an incomplete report loses
  `triaged` and gains `needs-info`, and a complete edit swaps them back. If the checker fails
  before writing anything (a build failure), the job exits red without touching the issue, so a
  broken toolchain cannot mislabel a stranger's report. The five label names the playbook and the
  forms use (`triaged`, `needs-info`, `agent-ready`, `sortlens-perf`, `enhancement-approved`) were
  created on the repository with `gh label create --force`; the workflow recreates the two it swaps
  on every run, so a fresh repository or a renamed label cannot silently drop the bookkeeping.

- **Verification.** `. scripts/env.sh && cargo xtask workflows` → `xtask workflows: ok`.
  `cargo test -p xtask` → 34 passed. A body containing only `### Sortlens version` produced exit 1
  naming the empty section and six missing ones; a bare enhancement body named its three missing
  feature fields. Both workflow files were parsed independently with PyYAML, which agrees with the
  project reader on keys, triggers and jobs (PyYAML reads `on` as `True`, which is the YAML 1.1
  quirk, not a difference in the file). `cargo xtask ci`, `snapshot` and `deny` green below.
  Not verified: the workflows have never run on GitHub — this commit is their first run — and the
  ubuntu `screenshots` job is expected to spell chords in ASCII like Windows, which cannot be
  observed from either local checkout (macOS paints `⌘Shift+R`; the Mac gate remains red for the
  reason in the fifty-sixth entry).

- **Documents reconciled in the same commit.** `README.md` (status prose no longer says core
  completion is in progress; the document table names the playbook), `docs/PLAN.md` §12 (the five
  phases done; what remains is the external table, the visible backlog and the two M9 rows),
  `docs/ROADMAP.md` (Phase 5 items 2 and 3 ticked with shipped notes; M8 item 3 and M10 item 2
  ticked with the signing blocker left in its own table; a status-log row), `CONTRIBUTING.md` (the
  `ci` line and `screenshots`), `docs/TOOLING.md` (the intake paragraph extended to the workflows
  and the playbook).

- **The plan is closed.** Every row under "Completion phases" is ticked except the native phone app
  (Phase 4 row 3b), which is tracked and open until a machine with the NDK and Xcode builds it. What remains is named
  where it was already named and nothing buildable was moved: the M10 "Deliberately absent" table
  (Apple identity, installer toolchains, a screen-reader session, a chosen palette, the owner's
  tag), the post-1.0 backlog, and the two M9 rows (more i18n catalogues with RTL screenshots,
  localised numbers and dates). The scheduled continuation task is left disabled and the run ends
  with the completion sentence.
## 2026-11-24 (fifty-eighth) — Phase 5 item 1: the GitHub intake is written, and `cargo xtask templates` refuses what GitHub would refuse

- **What shipped.** The three intake files Phase 5 item 19 named, and nothing else moved: the two issue
  forms GitHub renders, the pull-request checklist a contributor reads, and a gate that keeps both
  honest. `.github/` had workflows only; it now has `ISSUE_TEMPLATE/` and `PULL_REQUEST_TEMPLATE.md`.
- **The bug form** (`.github/ISSUE_TEMPLATE/bug_report.yml`): labels `bug` and `triaged`, the title
  prefix `[Bug]: `, a markdown intro that says why the bundle is safe to attach, and seven required
  fields — version, OS (a seven-option dropdown, Apple silicon through to "another platform"), OS
  version and display setup, numbered repro steps, expected, actual, and the `sortlens-cli diagnostics`
  textarea — plus one optional "anything else". The diagnostics field is the one the gate checks by
  name: GitHub can render a bug form without it, and a bug report without the bundle is a round trip.
- **The feature form** (`.github/ISSUE_TEMPLATE/feature_request.yml`): labels `enhancement` and
  `triaged`, title prefix `[Feature]: `, the command-first framing in the intro, required problem,
  proposal and area (ten options that follow the crates and surfaces), two "before opening"
  checkboxes, optional workarounds and extras.
- **The pull-request template** (`.github/PULL_REQUEST_TEMPLATE.md`): the five gates as checkboxes —
  `ci`, `snapshot`, `deny`, `mobile`, and `screenshots` with when it applies — the failing-then-passing
  test prompt (`Test:` and "it fails on `HEAD~` because"), `Closes #`, the scan/catalog/thumbs
  `cargo xtask perf` before-and-after table, and the `crates/ui` before-and-after screenshot table.
- **The gate** (`cargo xtask templates`, in `ci`): `xtask/src/yaml.rs` reads exactly the subset the
  forms use — block mappings, block sequences, plain and quoted scalars, `|` blocks — and refuses flow
  collections, folded scalars, anchors, tags, duplicate keys and tab indentation *with the line that
  caused them*; `xtask/src/intake.rs` then checks the shape GitHub renders: the keys a form carries,
  the six element types, an id on every field and no id on a markdown note, unique lowercase ids, a
  label wherever a person types or picks, options (and their own allow-list) for dropdowns and
  checkboxes, `validations` with only `required`, at least one required field, that the bug form's
  diagnostics field is a required textarea whose description names `sortlens-cli diagnostics`, and the
  GitHub length limits on `name` and `description`. The pull-request template is checked for the
  markers a reviewer needs, because a checklist that lost its `deny` box is a promise that already
  drifted. Both run from the repository root derived from `CARGO_MANIFEST_DIR`, so the same code works
  from `cargo xtask` and from `cargo test`.
- **Against a real parser.** The two shipped forms were also read once with PyYAML as an independent
  check, and it agreed on the top-level keys and the field ids — the reader is a subset, not a
  different dialect. It is still the shape of the file that is checked here, not GitHub's server-side
  validation; the entry says so where the code does.
- **Tests.** 18 in `xtask` (10 intake, 8 reader), all in `cargo test --workspace`. The reader's
  refusals are pinned: tab indent, flow list, `: ` in a plain value, duplicate key, indented document,
  and a block scalar that keeps its lines. The intake's are pinned too: a missing required id, a form
  with no required field, a duplicate id, an unrenderable element type, a misspelled attribute
  (`lable:`), a dropdown with no options, a bug form whose diagnostics description never names the
  command, and a pull-request template missing its `deny` checkbox. Breaking the shipped form for real
  — `id: os_version` → `id: os-version` — made `cargo xtask templates` fail naming that file and that
  id, and pass again once restored.
- **Visual verification: none needed.** No `crates/ui` file changed, so no baseline moved and
  `cargo xtask screenshots` was not run; `cargo xtask mobile` was not either, because no phone-facing
  crate changed.
- **Also this run:** `cargo-deny` was not installed in this worktree's fresh `.toolchain` (deny
  reported it and stopped), so it was installed with the documented `cargo install cargo-deny --locked`
  and the gate then passed.
- **Verified on this macOS checkout:** `. scripts/env.sh && cargo xtask ci` (fmt, clippy `-D warnings`,
  the whole workspace's tests including the 18 above, layers, roots, assets, fonts, version,
  attribution, a11y, docs and the new templates check), `cargo xtask snapshot` (no golden moved), and
  `cargo xtask deny` (advisories, bans, licences, sources all ok).
- **Also reconciled in this commit:** ROADMAP's Phase 5 item 1 ticked with what shipped; README's
  `cargo xtask ci` line, CONTRIBUTING's gate list and `docs/TOOLING.md`'s "checked, not trusted"
  section now name the intake gate.
- **Next**: Phase 5 item 2 — `docs/AUTONOMOUS_AGENT.md`: issue text is untrusted data; only
  `agent-ready`/`triaged` issues; never push to `main`, never auto-merge, never bypass `deny`, never
  touch signing secrets; the repro-first loop that ends in a Draft PR; and the bounded improvement
  queue (perf regressions, `enhancement-approved`, unchecked roadmap rows).
## 2026-11-24 (fifty-seventh) — phones get a native Rust app (SL-J10), and no web app is part of the product

- **Why.** The vendor's own product ships iOS and Android apps. The owner asked for a downloadable phone app
  built from the same Rust as the desktop, and for the web application to be removed.
- **Removed.** The web application and everything that built, served or gated it: its crate, its transport, its
  build task and gate, the server's page route, and the browser steps in both workflows. Nothing of it remains.
  **No web or browser application is part of this product and none may be added** — the non-goal is written
  down in `docs/PLAN.md`, `docs/ARCHITECTURE.md` §10 and the Phase 4 row of `docs/ROADMAP.md`. Kept because a
  phone uses them: `Engine::open_memory`, `library.snapshot`, `/thumb` and `sortlens_remote::client`.
- **Added.** `apps/sortlens-mobile`: the desktop host without the desktop — the same `sortlens-engine` and
  `sortlens-ui` in an eframe `App`, the library under the app's private directory, the camera roll as its
  source, `android_main` and `sortlens_ios_main` as the only unsafe code in the application. `cargo xtask
  mobile [android|ios]`, a `mobile` CI job per platform, the phone targets in `rust-toolchain.toml` and
  `scripts/bootstrap.sh`. D8 in `docs/PLAN.md` is reversed and says so.
- **Docs.** Every page now says the same thing: no web or browser application is part of the product
  (`docs/PLAN.md` §4 non-goals, `docs/ARCHITECTURE.md` §10, the Phase 4 row of `docs/ROADMAP.md`). The gate
  lists say `mobile` instead of the removed task, the five-face web allowlist and the two browser-facing
  crate pins are gone, and the font, attribution and architecture pages no longer describe a web build.
- **Verified here.** `cargo check -p sortlens-mobile` and the crate's tests on the host; `layers`, `roots`,
  `assets`, `version`, `attribution`, `a11y` and `docs`. **Not verified:** the Android and iOS targets. Both
  stop in `ring` and `tract-linalg`'s build scripts for want of the NDK and the iPhoneOS SDK (this Mac has
  command line tools only, and 3 GB free). The first machine with both runs `cargo xtask mobile`.
- **Next for the phone.** A phone layout (the shell is still desktop-shaped), settings persistence, the font set,
  send-to-desktop over the resumable upload, the iOS Photos library, packaging in the release workflow.

## 2026-11-24 (fifty-sixth) — the Mac reads the table the way the table means it: ⌘ is the command key, and a build without a symbols face says the glyphs as words

- **Why this came up.** The whole suite had only ever run on Windows. On this macOS checkout six
  `sortlens-ui` tests failed before any of this run's work: every shortcut was bound to the physical
  Control key (the table spells `Ctrl+O`, and `craft_keymap`'s `effective` folds ctrl into cmd only *off*
  macOS), so the ⌘O the help lists did nothing; and the shell painted `⌃`/`⇧`/`⏎` as keycaps while
  egui's bundled faces carry `⌘` and no other modifier glyph, so the audits tripped over tofu. A tree
  cannot be committed red, so the repair ships as a commit of its own.
- **The read.** `keymap::on_platform` is now the single translation at the ui's edge: on macOS a chord
  with `ctrl` and no `cmd` becomes `cmd` (⌘), which is what the table's `Ctrl+` means everywhere
  (`Ctrl+O` is ⌘O on a Mac and Ctrl+O elsewhere). Applied to the table's text (`chord_of`), to a
  rebind parsed as text (`keymap::parse_binding`) and to matching (`binding_runs`), so a table row,
  a user's `Ctrl+Alt+O` and the chord the dialog paints are one press — one function, not a copy in
  each reader.
- **The paint.** `keymap::spell` asks `fonts::symbols()`: with a symbols face embedded (craft
  fonts present) the keycaps are the platform's own (`⌘⇧R`); without one the glyphs egui's bundled
  faces cannot draw are spelled as the table's words (`⌘Shift+R`, `⌘Enter`), with ⌘ kept because it is
  the one modifier glyph the bundled emoji face does have. The arrows keep their triangles.
- **Tests.** Six tests that encoded Windows now read the platform: `dispatch_reads_the_keymap_not_the_handwritten_table`
  (⌘ opens), `painted_keys_follow_the_rebinds` (compares against `spell`, not a literal), the three
  glyph audits (they pass because nothing unpaintable is painted) and the shell's
  `the_shortcut_editor_opens_rebinds_and_saves` (compares against the catalog's own display, not the
  literal `Ctrl+Shift+R`). New `the_platform_spells_the_primary_modifier_its_own_way` pins the rule per
  platform: `Ctrl+Shift+R` on Windows/Linux, `⌘⇧R` with a symbols face on a Mac, `⌘Shift+R` without,
  and the plain keys and triangles unchanged. `cargo test -p sortlens-ui --lib` is 148 passed.
- **Still red on a Mac, and it predates this run:** `cargo xtask screenshots`. Two scenes
  (`command-palette`, `places`) differ by the chord wording, and
  `shortcut_editor_paints_in_both_themes` panics because `craft-keymap`'s own dialog paints the
  platform's `⇧`/`⌃` and the screenshot harness installs only egui's bundled faces — a face it
  needs is a craft-fonts face, and the harness never calls `fonts::install`. The gate is therefore a
  Windows/Linux gate today; making it a Mac gate means the harness embedding the symbol faces and a
  separate baseline set, which is a UI slice of its own (recorded in ROADMAP's Phase 4 item 1 row).
- **Verified:** `. scripts/env.sh && cargo xtask ci` (fmt, clippy `-D warnings`, the whole workspace's
  tests, assets, icons, fonts, docs, a11y, layers, roots, version, attribution), `cargo xtask snapshot`,
  and `cargo xtask deny` are green on this macOS checkout.

## 2026-11-24 (fifty-fourth) — Phase 4 item 3, thumbnails over the wire: the server's PNG route and the client's fetch

- **What shipped:** the piece a remote device cannot do without — it has no file access, so the
  pixels have to come over the wire. `GET /thumb/<photo-id>/<edge>` on `sortlens-server`: the photo is
  looked up by its stable id, decoded through the desktop grid's own `sortlens-thumbs::decode_file` path
  (so HEIC/RAW/JPEG behave exactly as they do locally), and answered as the export path's lossless PNG
  (`craft_image_codecs`, the same encoder `sortlens-export` uses — no second encoder, no second format).
  Under the paired token's `read` scope, like every other read.
- The edge is bounded, and the bounds are the reasoning: **16** (below it a device is upscaling a few
  pixels over a grid cell, which it can do itself from a bigger answer) to `sortlens_thumbs::DETAIL` —
  **1024** (above it a request means decoding a primary image to serve a grid cell). Out of range answers
  400 naming the range; a photo the catalog does not hold answers 404; a photo whose file cannot be read
  answers 404 too — a page paints a hole, not a lie, and no failure path answers 500.
- `sortlens_remote::client::Client::thumbnail(PhotoId, edge)` fetches it: `GET /thumb/<id>/<edge>` with
  the bearer token, PNG bytes returned untouched (they start with the PNG magic or the answer was a JSON
  refusal, so nothing is re-wrapped), and an unpaired client refuses without talking. A phone
  cannot invent a second format — this is the method it calls.
- Server plumbing: `http::Response::bytes(status, type, body)` beside `json` (the hand-written HTTP
  answer was JSON-only), the route in `handle`'s dispatch and `/`'s route list, `--help` updated; the
  server gains `sortlens-thumbs` and `craft-image-codecs` — both already in the tree, attributed, and
  behind the same licence review as the desktop and export paths.
- Tests: two against the real server process in `apps/sortlens-server/tests/client.rs` —
  `a_paired_device_reads_thumbnails_over_the_wire` (a PNG that decodes to the red fixture's own pixels
  with no upscaling past the 48×32 file, edge bounds / unknown photo / missing file each refused with the
  reason named, the file deleted under the answer so the 404 is real) and `a_token_without_read_cannot_pull_the_snapshot`
  extended to prove the same narrow token cannot read pixels either; three unit tests in
  `crates/remote/src/client.rs` (the request shape and token header, PNG bytes untouched, unpaired
  refuses without talking and a JSON refusal is a typed error naming status and reason).
- 809 tests; ci, deny, snapshot green. No UI change, so no screenshot baselines move.
- **Not verified by hand**: PNG size for a large photograph over a slow link.

## 2026-11-24 (fifty-third) — Phase 4 item 3, the client half: the device protocol, the snapshot on the wire, the engine in memory

- **What shipped:** the paired device's side of the protocol, in one place beside the server's.
  `crates/remote/src/client.rs` is what a paired phone speaks:
  `Client` over a `Transport` trait — native `TcpTransport` on `std::net`, because the protocol is
  the crate's, not the transport's. `health` (the SL-J07 connection test, no token), `pair` (the code is spent, the
  token is kept, `Client::paired` reopens one that a previous session saved), `call` for every
  allow-listed command, `snapshot` pulling the new `library.snapshot` registry command, and
  `upload` (SL-J04's client half): one request at or under the chunk size, `Content-Range`
  chunks above it, a `409`'s `received` count moving the cursor instead of failing the
  transfer, the completing chunk carrying `X-Content-Sha256`, and the id minted when the item
  is queued (`UploadItem::new`) so a retry after the app was backgrounded resumes the same
  file. The keep-or-remove toggle (SL-J05) rides on the item too: the ack answers with
  `LocalCopy::Keep`/`Remove`, decided client-side on the server's ack — which is what makes it
  survive the resume, because it lives on the item, not on the connection.
- The wire gained one command: `library.snapshot` (registry spec, `Engine::run` arm, the
  allow-list under `read`, an MCP example) returning the exact JSON `Engine::open_memory`
  opens. The engine itself gained the memory path one layer up: `open_memory` (no directory,
  the disk-side sessions — rejects bin, move receipts, cull file — simply absent, which is the
  truth for a fresh client), `is_memory`, `snapshot_json` as a public read.
- **Tests that fail when the behaviour is broken:** `sortlens-remote`'s unit tests against a
  scripted transport (the chunk plan covers the file once contiguously and an exact fit is one
  range, a 409 is an instruction not a failure and its `received` is where the next chunk
  starts, a 409 naming no progress fails instead of looping, nothing reaches the wire unpaired,
  pairing stores token and device, the one-shot carries the digest and no span, and the ack
  maps the toggle both ways); the engine's memory tests (a memory engine holding the library
  and operating while touching no file, the snapshot command serving exactly what the memory
  engine opens with the stale-revision guard holding in memory, a fresh client starting empty
  and round-tripping); three loopback tests in `apps/sortlens-server/tests/client.rs` against
  the real server process (pair → hold the library in memory from the snapshot → annotate
  through the wire with the write visible in a re-pulled snapshot; a token without `read`
  refused the snapshot by name; an upload that dies after the first chunk and resumes to a
  byte-verified file in the album with the partial gone and the keep-or-remove decision in the
  ack both ways — via a `DropAfter` transport wrapper, so the fault is the connection, not the
  server).
- **Files:** `crates/remote/src/client.rs` (new, ~700 lines with tests), `crates/remote/src/
  {lib,allowlist}.rs`, `crates/engine/src/{lib,command}.rs` (`open_memory`, `is_memory`,
  `snapshot_json`, the `library.snapshot` spec and run arm, three tests),
  `crates/mcp/src/examples.rs` (the `library.snapshot` row the arguments test demanded),
  `apps/sortlens-server/tests/client.rs` (new), `docs/ROADMAP.md`, `docs/FEATURES.md` (SL-J02, SL-J05), `docs/ARCHITECTURE.md` §10,
  `CHECKPOINT.md`.
- **Visual verification:** none needed — `crates/ui` is untouched and no painted surface
  changed; the slice is protocol, engine and app glue.
- Gates: `ci`, `snapshot` (no golden moved — the MCP examples are not a published output),
  `deny` (no new dependency: the client reuses serde_json and the suite's own SHA-256 already
  linked through `sortlens-ai`) all green.

## 2026-11-24 (fifty-second) — Phase 4 item 3, the in-memory catalog path

- **What shipped:** `Catalog::open_memory` in `sortlens-catalog` — a catalog that lives only in
  memory, because a remote device has no library directory to open. It receives the snapshot JSON
  the server holds (or nothing, for a fresh client), applies operations in memory with the same
  undo, redo, batch and stale-revision rules as the disk catalog, and never touches the
  filesystem: no directory, no log file, no lock. `read_only` is always false there — there is no
  other process to contend with and no lock to lose — and the periodic snapshot cadence is free,
  since there is nowhere to write. Two new surfaces make the round trip possible:
  `Catalog::snapshot_json` serialises the exact state `snapshot()` writes on disk (shared code,
  so the two cannot drift), which the client will hand to its own storage or back to the server,
  and a newer snapshot schema is *refused* rather than opened read-only — the disk path can warn
  and sit read-only beside the library files, but a client that cannot read the format must
  not pretend to hold the library. `is_memory` names the mode for the engine layer that follows.
- **Tests that fail when the behaviour is broken:** `a_disk_snapshot_opens_in_memory_and_operating_on_it_touches_no_file`
  (a disk catalog's snapshot JSON opens in memory answering the same questions — roots, ratings,
  tags, sequence — the tag undo replays from the snapshot's history, a commit in memory advances
  the sequence and lands the rating, and the temp directory's file count is unchanged through all
  of it); `a_fresh_memory_catalog_round_trips_through_its_snapshot_json` (an empty client syncs,
  rates past the `SNAPSHOT_EVERY` cadence with `snapshots_written` still zero, and a reopen from
  its own JSON restores the photos, the flag, the saved revision and an undo step);
  `a_broken_or_newer_snapshot_is_refused_in_memory` (unreadable JSON and a schema-past-this-build
  snapshot both err, the latter naming "newer than"); `the_stale_revision_guard_holds_in_memory`
  (`commit_expect` refuses a stale revision and the stale write never lands).
- **Files:** `crates/catalog/src/store.rs` (the `memory` flag, `log: Option<File>`, `open_memory`,
  `snapshot_json`, `is_memory`, the no-op persistence paths, four tests), `docs/ROADMAP.md`
  (Phase 4 item 3 row), `docs/ARCHITECTURE.md` §10, `CHECKPOINT.md`.
- **Visual verification:** none needed — no UI code changed; `crates/ui` untouched.
- Gates: `ci`, `snapshot`, `deny` (no new dependency — serde_json was already in the crate) all green.
- **Next**: the phone client itself (SL-J02), reusing `sortlens-ui` over the
  server's `/api` and `/upload` protocol, with an engine-side in-memory path if the engine needs
  one beyond the catalog's (check what `Engine::open` demands of the filesystem before writing
  it), and the keep-or-remove-after-upload ack (SL-J05) as the client's half. Phase 4 is then one
  row from done; after that, Phase 5 item 1 (the GitHub issue/PR templates) per the ROADMAP
  order. The i18n row keeps its open half (zh-TW/ko/th/vi, unrouted surfaces, RTL screenshots
  needing an Arabic font build).

## 2026-11-24 (fifty-first) — Phase 4 item 3, the desktop's own start/stop surface

- **What shipped:** the owner's window on the server process, in Settings → Sync & devices. Start
  spawns the `sortlens-server` binary that sits beside the app (`apps/sortlens/src/sync.rs`
  locates it beside `current_exe`, so `cargo run` and a packaged bundle both work) against the
  open library — loopback, ephemeral port — and the process's stdout is followed through the new
  `sortlens-remote::supervise`: `listening http://host:port` and `pairing NNNNNN` are parsed into
  a `ServerState`, the code a reprint replaces the expired one rather than growing a history, and
  an exit is visible on the next repaint. Stop kills the child and waits, so the library's write
  lock is free when the notice lands; dropping the supervisor kills it too, so a window that
  closes without tidying up cannot leave a server holding the lock.
- The shell's half (`crates/ui`): `UiState::server` is a `ServerSurface { running, port, code,
  devices }` the host feeds every frame the Settings dialog is open (default: the stopped state,
  so a headless frame paints an honest "not serving"), and the section paints the port, the live
  code, the device rows (name, scopes as ids, issue date through the same civil-date arithmetic
  the change report uses) and three new `Action`s — `ServerStart`, `ServerStop`,
  `ServerRevoke(token)`. Revoke is disabled while the server runs, with the reason on hover: a
  running server holds its own token store and would resurrect the row on its next write-through.
  The host refuses it too, so the rule survives a click on a stale frame; a stopped server revokes
  the exact row by exact token through `TokenStore::revoke`.
- Serving beyond loopback stays the CLI's `--bind lan`; the desktop's own surface does not offer a
  checkbox for it (SL-J09's default is the safe one).
- **Tests that fail when the behaviour is broken:** `crates/ui`'s
  `the_sync_section_starts_stops_and_revokes_only_when_stopped` (Start emits the action; the
  running surface names port and code; a click on the disabled Revoke answers nothing; once
  stopped the same button hands over the exact token) — the disabled button is the point: a
  revoke the running server could resurrect is not a revoke. `apps/sortlens`'s
  `the_device_list_is_the_token_file_and_revoke_needs_a_stopped_server` (the list is the token
  file, mapped; revocation rewrites it; an unknown token says so) and
  `revoke_while_the_server_runs_is_refused_by_name`. `crates/remote`'s
  `the_startup_lines_parse_and_nothing_else_does` and
  `a_reprinted_code_replaces_the_old_one` (a stub child, so the state-folding is proved without a
  real server). And in the loopback harness,
  `the_desktop_supervisor_starts_a_real_server_and_reads_the_token_file`: the supervisor spawns
  the real server binary, the port and code arrive from its own stdout, the desktop pairs through
  that port, and the row it would show is read back from the token file after the stop.
- **Files:** `crates/remote/src/supervise.rs` (new), `crates/remote/src/{lib,tokens}.rs` (the
  module and a public `TokenStore::all`), `crates/ui/src/shell.rs` (the section, the surface
  types, the three actions), `crates/ui/src/i18n/{ar,ja}.tsv` (four routed rows each),
  `crates/ui/src/lib.rs` (the exports), `crates/ui/tests/screenshots.rs` (the lenient harness and
  the scene), `apps/sortlens/src/sync.rs` (new), `apps/sortlens/src/main.rs`,
  `apps/sortlens/Cargo.toml`, `apps/sortlens-server/tests/loopback.rs`, `docs/ROADMAP.md`,
  `docs/FEATURES.md` (SL-J03), `docs/ARCHITECTURE.md` §10.
- **Visual verification:** two new baselines, `settings-sync-charcoal` and `settings-sync-paper`,
  painted at 900×1700 because the Settings dialog is longer than one screen and a baseline that
  clips its subject proves nothing. Read: the port, the code, the hint and the device row are all
  in frame, Revoke visibly disabled. The harness runs with `allow_missing_glyphs` for this scene
  only: the language picker names every language in itself, and CI's font build has no Arabic or
  Japanese face — the documented gap, not a silent defect.
- Gates: `ci`, `snapshot`, `deny` (no new dependency — the supervisor is std-only) (crates/ui changed) all green; `screenshots` green with the two new baselines.
- **Next**: Phase 4 item 3's remaining half — the phone client (SL-J02) reusing
  `sortlens-ui` over the same `/api` and `/upload` protocol. Its first slice is the in-memory
  catalog path in `sortlens-catalog` (snapshot JSON in, ops applied, no directory, no log, no
  lock), because a remote engine has no directory to open; the transport and the keep-or-remove
  ack (SL-J05) follow. Then Phase 4 is one row from done; after that, Phase 5 item 1 (the GitHub
  issue/PR templates) per the ROADMAP order. The i18n row keeps its open half (zh-TW/ko/th/vi,
  unrouted surfaces, RTL screenshots needing an Arabic font build).

## 2026-11-24 (fiftieth) — Phase 4 item 3, server half: `sortlens-server`, the loopback harness

- **What shipped:** the library is served to a paired device. `crates/remote` stops being a scaffold:
  `tokens.rs` (the four scopes — `read`, `upload`, `annotate`, `cull` — as a `Permissions` set, a
  `TokenStore` persisted as `<library>/remote-tokens.json` with atomic write-through on every change,
  re-issue replacing the row and revoke reporting the truth), `pairing.rs` (a six-digit code with a
  ten-minute life and a five-wrong-attempts lockout that survives even the right code; the code and the
  64-hex token both draw from the suite's own SHA-256 over a local entropy mix — no crypto dependency),
  and `allowlist.rs` (the table naming every registry id the wire accepts with the scope it needs,
  audited against `Registry::v0()` at startup by `allowlist::audit`, plus the two-sided test: every row
  must be a ready command, and every ready command must be either on the table or named in an exclusion
  list with its reason). `apps/sortlens-server` is the process: a hand-written minimal HTTP/1.1 on
  `std::net` (`http.rs` — Content-Length bodies only, `Transfer-Encoding: chunked` refused with a
  reason, `Connection: close`, zero new dependencies), `api.rs` (routes `/health`, `/pair`, `/api`,
  `/upload`, `/upload/<id>`; `/api` checks bearer token → allow-list scope → `Engine::run`, and a
  method the wire does not accept answers 403 without revealing whether it exists), and `upload.rs`
  (resumable chunks appending to `<album>/.uploads/<id>.part`, so the file length is the received
  count and a restart loses nothing; a wrong-position chunk answers 409 with `received`; the completing
  chunk is hashed against `X-Content-Sha256` *before* it is appended, so a digest mismatch leaves the
  partial exactly where it was; the published name never overwrites — it takes ` (n)` beside the stem;
  names are leaf-only and folder-safe). The server binds loopback by default and every interface only
  under `--bind lan` (SL-J09), prints its port and pairing code on stdout, and pumps the engine's
  finished scan work between connections.
- **The two-process loopback harness** (`apps/sortlens-server/tests/loopback.rs`) is the proof the
  old M9 table said was missing: five tests, each spawning the real server binary
  (`CARGO_BIN_EXE_sortlens-server`) against a real library the test scans through the engine first and
  then releases. `a_paired_phone_reads_annotates_and_is_scoped` pairs, reads two scanned photographs,
  writes a 5-star rating through `/api`, is refused `/api` without a token and with a nonsense one,
  is refused `app.quit` with a good one, and reads the rating back from disk after the server process
  is killed; `the_pairing_code_locks_out_after_five_wrong_attempts` spends the code;
  `a_narrow_token_cannot_cull_and_the_wide_one_can` proves `--allow read,annotate` grants exactly
  that and `cull.start` answers 403 naming the missing scope;
  `an_upload_resumes_after_a_dropped_connection_and_verifies_the_digest` uploads 300 KB in three
  chunks, drops the connection mid-transfer, asks `GET /upload/<id>` where to continue, is refused a
  wrong digest (the partial stays), completes with the right one, and finds `IMG_0001 (1).jpg` beside
  the first when the same name uploads again; `a_server_restart_keeps_the_tokens_and_the_pairing_scope`
  pairs, kills the server, restarts it, and the same token still works.
- **Files:** `crates/remote/src/{lib,tokens,pairing,allowlist}.rs` (rewritten from the M0 scaffold),
  `crates/remote/Cargo.toml` (`sortlens-engine`, `sortlens-ai`, `serde`, `serde_json`),
  `apps/sortlens-server/src/{main,http,api,upload}.rs`, `apps/sortlens-server/Cargo.toml`,
  `apps/sortlens-server/tests/loopback.rs` (new), `docs/ROADMAP.md` (the Phase 4 item 3 row split into
  the shipped server half and the open client half; the M9 state prose and the "Re-opened from M9"
  paragraph updated), `docs/FEATURES.md` (SL-J01/J03/J04/J06/J07/J08/J09 annotated),
  `docs/ARCHITECTURE.md` (§10 rewritten to describe what runs; the layer diagram's remote line).
- **Tests that fail when the behaviour is broken:** 11 in `crates/remote` (the allow-list audit, the
  two-sided registry test, the lockout and TTL arithmetic, token generation, the store's persistence,
  re-issue and revoke, scope parsing) and 3 unit + 5 loopback in `sortlens-server` (name sanitising,
  range parsing, id alphabet; the five harness tests above). The lockout test fails against a gate
  that forgets its attempts — the first version of the handler redeemed on a clone of the gate, and
  the harness caught it before it shipped; the digest test fails against a server that appends the
  final chunk before verifying it, because the resumed re-send answers 409 instead of 200.
- **Visual verification:** n/a — no UI surface changed.
- Gates: `ci` green (fmt, clippy `--all-targets -D warnings`, workspace tests, layers/roots/assets/
  fonts/version/attribution/a11y/docs), `snapshot` green (no CLI verb touched), `deny` green (no new
  dependency). Nothing under `crates/core`, `crates/ui` or a phone-facing crate
  changed. `screenshots` not required: no `crates/ui` change.
- **Next**: Phase 4 item 3's remaining half — the phone client (SL-J02) reusing
  `sortlens-ui` over the same `/api` and `/upload` protocol, plus the desktop's own start/stop surface
  for the server. Then Phase 4 is one row from done; after that, Phase 5 item 1 (the GitHub issue/PR
  templates) per the ROADMAP order. The i18n row keeps its open half (zh-TW/ko/th/vi, unrouted
  surfaces, RTL screenshots needing an Arabic font build).

## 2026-11-24 (forty-ninth) — Phase 4 item 2d: the `craft-i18n` scaffolding (SL-L07)

- **What shipped:** the interface's strings begin to load through the suite's i18n. `crates/ui/src/i18n.rs`
  builds one `craft_i18n::I18n` from the embedded Arabic and Japanese catalogues (`crates/ui/src/i18n/ar.tsv`,
  `ja.tsv` — the first two of the six languages the design names), keeps a thread-local current language,
  and hands call sites `tl!`/`fmt`/`tn`. The English text is the lookup key, a missing row is the English
  itself, and a bad row is logged once as a warning naming the line — partial coverage shows English
  rather than breaking anything. The first batch routed: the sidebar's six sources, five section titles
  and the `Library` header, the foot's four registry labels (`Check Library`, `Recently Deleted`,
  `Recent Changes`, `Keyboard Shortcuts`), the right panel's four tab labels, both delete modes, and the
  whole Settings dialog (its window title, every section header, the three buttons, the sidecar checkbox,
  the cull-batch header and the closing note).
- **The language is a setting:** Settings → Language offers every language named in itself (English,
  العربية, 日本語, from each catalogue's `@id language.name` row); switching is instant (the shell applies
  the direction every frame) and asks the host with the new `Action::SetLanguage(code)`; the app persists
  `Appearance.language` and restores it at launch, an unknown code reading as English.
- **RTL, the direction plumbing SL-L07 stands on:** `i18n::apply_direction` resolves the interface
  direction from the chosen language through `craft-ui`'s `direction::resolve` (the `CRAFT_UI_DIRECTION`
  override intact, for checking a mirrored layout before a right-to-left catalogue exists) and applies it
  to the context every frame — egui's own chrome mirrors through the fork's `Style::right_to_left`. The
  sidebar's panel sits at the reading start, the tool strip and right panel at the reading end;
  `nav_row` paints its glyph, label and count from the reading start (the colour dot with it), and
  section titles align to the reading start. The `with_layout` wrap is behind an RTL check on purpose:
  at `Align::Min` it rounds the label's origin differently from a bare `ui.label`, and the 19 screenshot
  baselines are the proof that a left-to-right interface paints byte for byte what it painted before.
- **Tests that fail when the behaviour is broken:** the catalogue audit
  `catalogs_load_and_routed_keys_have_rows` (zero parse problems; the routed keys read out of
  `shell.rs`'s own `tl!` calls, more than 20 of them, and every one has a row in both Arabic and
  Japanese — a routed key without a row fails naming the key); `the_picker_names_every_language_and_directions_follow_the_script`
  (the picker is exactly English/العربية/日本語, each named in itself; Arabic resolves right to left,
  Japanese left to right); the app's `the_language_setting_round_trips_and_an_unknown_one_is_english`
  (every offered code round-trips through the TOML and is one the interface offers, `tlh` reads as
  English, and a pre-scaffold settings file still opens); and the headless
  `the_language_switch_moves_the_sidebar_to_the_reading_start` — the radio clicked through the real
  Settings widget, `SetLanguage("ar")` asked of the host, the sources header measured in the right half
  and the All Photos row reading Arabic, then switched back to English through the same dialog and the
  sidebar measured back at the left edge. Headless on purpose: CI's fonts have no Arabic face, so this
  proves the geometry, not the glyphs.
- **Visual verification:** all 19 screenshot baselines regenerated byte-identical. The first run moved
  one sidebar section header by a rounding fraction and the diff caught it (472 pixels in one 50×20
  box) — that is the audit working, and the reason the wrap is conditional.
- Files: `crates/ui/src/i18n.rs` (new), `crates/ui/src/i18n/ar.tsv` (new), `crates/ui/src/i18n/ja.tsv`
  (new), `crates/ui/src/lib.rs` (the module), `crates/ui/Cargo.toml` (`craft-i18n`, `log`),
  `crates/ui/src/shell.rs` (the routes, the panels, `nav_row`, `Action::SetLanguage`, the Settings
  language row, the headless test), `apps/sortlens/src/settings.rs` (`Appearance.language`, round-trip
  and old-file tests), `apps/sortlens/src/main.rs` (restore at launch, persist on the action),
  `docs/ROADMAP.md` (2d ticked with the proof named; the M9 row and state prose updated to say what is
  still open), `docs/FEATURES.md` (the SL-L07 row), `docs/UX.md` (the i18n bullet),
  `docs/ARCHITECTURE.md` (new §8.4 Interface language, keymap renumbered to 8.5).
- Gates: `ci`, `snapshot`, `deny`, `screenshots` (19/19 byte-identical)
  all green.
- **Next**: Phase 4 item 3 — `sortlens-server` (loopback/LAN bind, pairing code → scoped device token,
  command-registry allow-list, resumable uploads) and the phone client reusing `sortlens-ui`, proved by a
  two-process loopback integration harness in CI (SL-J01..J09). The i18n work that remains lives in M9's
  row: zh-TW/ko/th/vi, the unrouted surfaces, and RTL screenshots of the painted interface, which need a
  font build with an Arabic face.

## 2026-11-24 (forty-eighth) — Phase 4 item 2c: the opt-in update check (SL-I10)

- **What shipped:** `craft-update` behind an explicit opt-in — the second of the two owner-approved
  network calls, consent-gated in exactly the shape `model.install` set. `sortlens-engine`'s new
  `update` module (desktop-only, gated like the model downloader) reads the latest published release of `Topzic/sortlens-rust` through `craft-update`'s
  parser — drafts and error bodies refused, every text capped, asset links held to the app's own
  repository — and compares it against the running version with `craft-update`'s semver rules,
  including the local-build rule, so a build from source is never offered the release it already
  contains. Three registry commands: `update.status` (version, consent, repo, whether a check is
  running, the last answer), `update.check` (refuses without consent, naming the setting, the way
  the downloader does; refuses again while one is running), and `update.consent`. The check runs on
  a worker thread as an `Update` activity job; the answer is kept for `update.status` and the
  Settings row, so one that lands after the dialog closes is not thrown away.
- **Every face wired:** the app persists the consent as `updates.check_consent` and restores it at
  launch like the model downloader's; Settings → Updates paints the consent switch, the `Check now`
  button only behind it, `Checking…` while the worker runs and the last answer's one-line headline;
  the CLI answers with `update` (status), `update-consent on|off` and `update-check` (a progress
  verb, like `model-install`); the MCP examples name `update.check`, `update.consent` and
  `update.status`. Nothing downloads or installs automatically — an update still means building from
  source; the self-update path stays in M10's absence table with the installer work it needs.
- **Tests that fail when the behaviour is broken:** in the module, the parse tests (a newer stable
  release offered and summarised, an older one not, a prerelease skipped for a stable running build,
  a local build only ever offered a higher release number, garbage and a rate limit named as errors,
  an empty answer read as up to date rather than as a failure); in the engine, the refusal without
  consent naming `Settings → Updates`, the consent flip through its command, `update.status`'s shape
  before and after a check, the full worker path over a loopback feed in GitHub's own shape (the job
  finishing Done, the answer kept, the headline read back), a failing feed failing the job and
  keeping no answer, a second check refusing while one runs, and the endpoint/fetcher seam that
  keeps the network out of the tests (`craft-update`'s real client is HTTPS-only, so `update.check`
  under test swaps in the loopback fetcher, never the consent gate). In the shell, the headless
  `the_updates_settings_section_gates_the_check_behind_consent`: the switch clicked through the real
  widget flips the engine and asks the host to persist it, `Check now` exists only behind consent,
  and the test never presses it — pressing it would be the real network call.
- **Visual verification:** no baseline moved. No settings scene exists (the dialog is covered by the
  headless tests), and the section is desktop-only, so `cargo xtask screenshots` ran green over all
  33 baselines byte-identical — the proof the Settings change touches no other surface.
- Files: `crates/engine/src/update.rs` (new), `crates/engine/src/lib.rs` (module, fields,
  `update_consent`/`set_update_consent`/`check_for_updates[_with]`/`start_update_check`/
  `poll_update_check`, the three command handlers, the tests), `crates/engine/src/command.rs` (the
  three registry rows), `crates/engine/src/job.rs` (`JobKind::UpdateCheck`),
  `crates/engine/Cargo.toml` (`craft-update` without the `egui` feature, so the CLI build stays
  clear of egui), `crates/ui/src/shell.rs` (`Action::SetUpdateConsent`, `updates_section`, the
  `run_ui` test), `apps/sortlens/src/settings.rs` (`Updates.check_consent`, defaults, round-trip and
  old-file tests), `apps/sortlens/src/main.rs` (restore at launch, persist on the action),
  `apps/sortlens-cli/src/main.rs` (the three verbs, help, schema row), `crates/mcp/src/examples.rs`,
  `docs/ROADMAP.md` (2c ticked with the proof named; M10 prose reconciled), `docs/FEATURES.md` (the
  SL-I10 row), `docs/UX.md` (the Updates dialog row), `docs/RELEASE.md` (the "no update check"
  claim replaced), `docs/ARCHITECTURE.md` (§13 updates).
- Gates: `ci`, `snapshot` (the two CLI goldens that name the verb list were regenerated with
  `--update` and read), `deny`, `screenshots` all green.
- **Next**: Phase 4 item 2d — `craft-i18n` scaffolding (SL-L07): the `craft-i18n` crate is already a
  pinned workspace dependency; wire the catalog so the interface's strings load through it, with the
  RTL behaviour of SL-L07's design note and the font stack already handling the symbols, then item 3
  `sortlens-server` + the phone client with the two-process loopback harness.

## 2026-11-24 (forty-seventh) — Phase 4 item 2b: the High-Contrast preset (SL-L09)

- **What shipped:** the fifth theme preset — `ThemeKind::HighContrast`, persisted as
  `theme = "high-contrast"` through the same `id()`/`from_id()` path as the other four, so no migration
  is needed and an unknown id still falls back to Charcoal. Pure black canvas and thumb wells, pure white
  text, a yellow accent (`#ffd23f`) that holds 4.5:1 against black with room to spare, muted text at AA
  (`#cccccc`), a grey `#6b6b6b` hairline so a border reads as a boundary rather than as text, 2-point
  corner radius, and a new `Tokens::stroke_w` metric that doubles every chrome stroke — window frames,
  widget outlines, the selection ring and the press ring — while the four colour presets keep their
  1-point hairlines.
- **The one token, not five branches.** `stroke_w` is the metric `apply()` uses for every `Stroke::new`
  it installs; no preset carries its own stroke-writing code, and a sixth preset would be a new constant,
  as the file's doc comment already promised. The preset appears everywhere presets are named because
  every menu iterates `ThemeKind::ALL` — the Appearance chip's cycle, its right-click list, and the
  Settings radios — with no new drawing code.
- **Tests that fail when the behaviour is broken:** `high_contrast_is_black_white_yellow_and_doubled`
  (WCAG luminance ratios, not eyeballing: text at AAA ≥ 7:1, muted at ≥ 4.5:1, accent and accent-foreground
  at ≥ 3/4.5:1, the accent actually yellow, all five presets' `stroke_w` correct, and `apply` really
  installs the 2-point strokes and the token fills); the five preset-wide tests (`ids_round_trip`,
  `light_presets_have_dark_text`, `the_widget_palette_is_the_token_set_and_apply_installs_it`,
  `accents_are_visible_against_the_background`, `badge_and_flag_marks_hold_three_to_one_against_a_bad_cell`)
  now run over all five — the badge test bit immediately: the first reject colour's white glyph measured
  2.78:1, fixed to `#ff5252` before anything was committed.
  `the_settings_dialog_offers_high_contrast_and_clicking_it_asks_the_host` drives the real radio widget
  headlessly and asserts `Action::SetTheme(HighContrast)` reaches the host while the shell waits (the host
  applies and persists it). The persistence layer is covered by `every_theme_round_trips_and_a_future_one_falls_back`
  in the app's settings — the round-trip test the grid and density had and theme did not.
- **Visual verification:** two new baselines, `library-justified-high-contrast` and
  `welcome-high-contrast` (33 now), both read: black canvas, white words, yellow stars and selections,
  the chip/badge marks clearly visible, borders that read as boundaries; the other 31 regenerated
  byte-identical. `cargo xtask screenshots` green; the other presets' baselines did not move, which is
  the proof the `stroke_w` change is exactly zero for them.
- Files: `crates/ui/src/theme.rs` (the variant, `Tokens::HIGH_CONTRAST`, `stroke_w`, the test),
  `crates/ui/src/shell.rs` (the Settings-dialog `run_ui` test), `crates/ui/src/lib.rs` and
  `crates/ui/src/icons.rs` (doc counts), `crates/ui/tests/screenshots.rs` (`kind_id` arm, the new
  baseline test, the THEMES comment), `apps/sortlens/src/settings.rs` (the theme round-trip test),
  `docs/ROADMAP.md` (2b shipped with the proof named; M10 state and absent-table prose reconciled; the
  item 1 baseline count marked historical), `docs/UX.md` (the five-preset table, the contrast bullet
  now naming a shipped preset, the screenshot-suite prose), `docs/ARCHITECTURE.md` (§8.2 five presets).
- Gates: `ci`, `snapshot`, `deny`, `screenshots` all green.
- **Next**: Phase 4 item 2c — `craft-update` behind an explicit opt-in (SL-I10): a consent-gated update
  check in Settings, the same shape as `model.install`'s gate (user-initiated network only), with the
  engine or app host holding the consent flag and no call without it. Then 2d `craft-i18n` scaffolding
  (SL-L07), then item 3 `sortlens-server` + the phone client with the two-process loopback harness.

## 2026-11-24 (forty-sixth) — Phase 4 item 2a: the packaging icons are renders of the one drawing

- **What shipped:** `assets/icon.ico` and `assets/icon.icns`, generated — never dropped in — from the same
  geometry in `crates/icon`, with `cargo xtask icons` as the check-and-rewrite front door. This was the
  first clause of ROADMAP Phase 4 item 2; the item is now split (2a done, 2b High-Contrast, 2c craft-update,
  2d craft-i18n still open).
- **One drawing, four renderings.** `sortlens-icon::render(size)` is the resolution-independent render
  (the geometry was already in fractions of the tile, so every size is a fresh raster, not a resample —
  a curve stays a curve at 16 px), and `png_at(size)` is its PNG. `ico()` packs 16/32/48/64/128/256 px
  entries into the little-endian ICONDIR (PNG entries throughout, the 0-as-256 edge byte honoured, planes
  1 and 32 bpp declared), and `icns()` packs `ic11`/`ic12`/`ic07`/`ic08`/`ic09` — the 32/64 @2x list-view
  slots, the 128 and 256 Finder icons and the 512 Get Info slot — into the big-endian container whose
  header length is the file length. No new dependency, and the same stored-deflate PNG writer as the
  window icon, so the bytes are deterministic on every machine (the crate's stated trade: size for
  reproducibility — 359 KB and 1.4 MB committed).
- **The front door lives in `xtask`, as the row asked.** `cargo xtask icons` generates both files in
  memory and compares them to the committed bytes, failing with the rewrite command on a mismatch;
  `cargo xtask icons --update` rewrites them (the `snapshot --update` pattern). The suite still gates
  them: `the_committed_packaging_icons_are_the_drawing` compares byte for byte and shares the SVG's
  `SORTLENS_UPDATE_ICON=1` rewrite path, so `cargo xtask ci` refuses a stale packaging file without
  running xtask twice.
- **Tests that fail when the behaviour is broken:** `the_ico_carries_one_png_per_desktop_size` parses the
  ICONDIR back field by field, walks the offsets the directory declared, decodes every embedded PNG
  through `sortlens_thumbs` and asserts the 40%-of-tile pixel is the front print at all six sizes (the
  centre pixel was tried first and is honestly wrong at 16 px — the lens edge reaches it, so the test
  samples a pixel that is the front print at every size); `the_icns_carries_the_mac_slots` walks the
  chunk chain, asserts each length stays inside the header's total and decodes each slot to its named
  square; `the_same_bytes_every_time` grew `render(64)`, `ico()` and `icns()` determinism. The committed
  test failed on a missing file before generation, proving it bites.
- Files: `crates/icon/src/lib.rs` (`render`, `png_at`, `ico`, `icns`, four tests), `xtask/Cargo.toml`
  (`sortlens-icon`), `xtask/src/main.rs` (the `icons` task + help line), `assets/icon.ico`,
  `assets/icon.icns`, `docs/ROADMAP.md` (Phase 4 item 2a shipped and item split; M10 state, icon row and
  absent-table prose reconciled), `docs/RELEASE.md` (the icon section now describes what ships),
  `docs/ARCHITECTURE.md` (§13 icon line and the layer-diagram row), `docs/ATTRIBUTION.md` (the two new
  asset rows).
- Gates: `ci` green (the 7 `sortlens-icon` tests among them), `snapshot`, `deny`, `icons` all green;
  `screenshots` not run — no `crates/core`/`crates/ui` or UI code changed.
- **Next**: Phase 4 item 2b — the High-Contrast preset (SL-L09), then 2c the opt-in `craft-update`
  (SL-I10), 2d the `craft-i18n` scaffold (SL-L07), then item 3 the server and phone loopback harness.

## 2026-11-24 (forty-fifth) — Phase 4 item 1: the keymap is a machine now, not just a table

- **What shipped (SL-I09):** the `craft-keymap` editor in Settings → Shortcuts — rebinding, conflict
  detection, named sets, and craft-keymap JSON import/export — with dispatch genuinely reading it. The
  crate was already pinned in `[workspace.dependencies]` and linked by nothing; it is linked by `sortlens-ui`
  and `sortlens` now, which also closes the last unlinked Phase 4 pin.
- **The runtime half lives beside the table it documents.** `crates/ui/src/keymap.rs` keeps `BINDINGS` as
  the readable, registry-cross-checked table it was, and adds the machine built from it: `catalog()` turns
  every single-chord command row into a `craft_keymap::Bindable` under `Scope::app()` (19 commands; the two
  flag rows are one registry command as two parameterised refs, `photo.flag {"flag":"pick"}` and `"reject"`,
  so a rebind of one leaves the other alone), `fixed_rows()` presents the unrebindable rows — the ranges
  `0–5`/`6–9`, the mouse rows, `Esc`, the arrows — read-only in the editor, with a chord set only where it
  does not sit on a catalog default. `build(overrides)` is the keymap the shell runs on `Platform::current()`.
- **Dispatch is the keymap's now.** `plain_pressed(keymap, input, command)` is the runtime twin of the old
  `key_pressed` calls (plain keys, repeats included, nothing consumed, the command modifier still gating);
  `consume(keymap, input, command)` reads *exactly* through craft's layout and removes the press events that
  ran — deliberately not `consume_shortcut`, whose "extra Shift and Alt are ignored" rule would have left a
  rebound `Ctrl+Alt+O` still answering plain `Ctrl+O`. Every command-bound key check in the shell was
  rerouted: `?`, `Ctrl+O`, `Ctrl+Shift+R`, `Ctrl+Shift+N`, `Ctrl+,`, `Ctrl+Q`, `Ctrl+Z`, `Ctrl+Shift+Z`,
  `Ctrl+Enter`, `I`/`M`/`S`, `D`, `C`, `P`/`X`, `K`, `T` (library and cull), and the cull `◀`. The hardcoded
  rows (`U`, `G`, `Esc`, `Space`, `Ctrl+A`, `Ctrl+K`, the zooms, the detail arrows) stay hardcoded and are
  exactly the editor's fixed rows — a rebind the shell would never read does not exist. The redo-before-undo
  consume-order note is gone: exact matching separates `Ctrl+Shift+Z` from `Ctrl+Z` by itself.
- **The help cannot go stale.** The palette and the `?` overlay paint `painted()`/`row_keys()` — the current
  bindings, triangles for arrows, joined ` / ` the way the registry spells two — falling back to the
  registry's declared shortcut for commands outside the catalog. `Shift+P` still flags (the plain-key
  semantics are unchanged for defaults); a rebind carrying Shift reads exactly.
- **The editor itself.** Settings → Shortcuts opens craft-keymap's dialog over the app's catalog: recording
  with the conflict bar (Ask/Steal/Keep Both), named sets (a built-in `Default`; a first save becomes the
  user's `Custom` set and goes active), craft-keymap JSON import and export through the host's file dialogs,
  per-set revert, undo/redo inside the dialog, and double-press recording refused through an `accept` rule —
  the dispatch does not read doubles, so the editor must not record them. The dialog owns the keyboard while
  it is open: `keyboard()` steps aside and ⌘K waits. `Outcome::Ok` hands `Profiles` to the host
  (`Action::SaveShortcuts`), which persists them; `Import`/`Export` come back as actions the host answers
  with `rfd`, the imported text landing in `pending_shortcut_import` for the next frame. Overrides live in
  `Prefs::shortcuts` as craft-keymap `Profiles` (the `overrides_list` array form), so a deleted set stays
  deleted after saving and a file written before the section existed reads as the factory bindings.
- **Tests that fail when the behaviour is broken:** in `keymap.rs`, `the_catalog_is_the_rebindable_rows_of
  _the_table` (every catalog command ready in the registry, every single-chord command row present, the
  ranges and `Click` refused), `the_fixed_rows_are_the_rest_of_the_table_and_clash_with_no_catalog_default`,
  `a_rebinding_moves_the_bindings` (and the second key on one command is additive),
  `two_commands_on_one_chord_are_conflicts`, `dispatch_reads_the_keymap_not_the_handwritten_table` (headless
  frames: `Ctrl+O` opens, the rebind moves the chord, `⌘P` is not a flag, `Q` becomes similar after a rebind),
  and `painted_keys_follow_the_rebinds`. In the shell, `a_rebound_key_moves_the_action` (Q flags as picked,
  P stops answering, X untouched) and `the_shortcut_editor_opens_rebinds_and_saves` (the editor paints the
  catalog under the table's own sections, owns the keyboard, OK hands the host the override and Q still
  flags after the save). In the app, `shortcut_sets_round_trip_and_an_older_file_reads_the_defaults`.
- **Visual verification**: new `shortcut-editor-charcoal`/`shortcut-editor-paper` baselines (31 now), both
  read at native scale — the dialog follows the theme, the table is grouped Anywhere/Library/Cull session
  with the factory chords in keycaps, the set row carries New/Rename/Delete/Save/Revert/Export/Import, and
  Cancel/OK sit at the foot. The other 29 baselines regenerated byte-identical: with factory bindings the
  runtime text equals what the registry declared, so no surface moved. One honest rough edge, noted not
  hidden: the editor spells `cull.back`'s arrow "Left" (craft's own display), while the overlay, the palette
  and the table spell the triangle `◀` — the editor's spelling is craft's, ours is the triangles.
- Files: `crates/ui/src/keymap.rs` (the runtime half + six tests), `crates/ui/src/shell.rs` (runtime state
  in `UiState`, the editor dialog and its options, the Settings section, the key reroutes, the palette and
  overlay painting, two shell tests), `crates/ui/Cargo.toml` (`craft-keymap`, `serde_json`),
  `crates/ui/tests/screenshots.rs` (the new scene), `apps/sortlens/Cargo.toml` + `src/settings.rs`
  (`Prefs::shortcuts` + round-trip test) + `src/main.rs` (startup load, the three host actions),
  `docs/ROADMAP.md` (M9 keymap row and Phase 4 item 1 ticked, M9 state sentence reconciled),
  `docs/RELEASE.md` (the "no keymap editor" claim replaced by what shipped), `docs/UX.md` (§8 Shortcuts
  row), `docs/ARCHITECTURE.md` (§8.4 cull defaults corrected to the shipped keys), `docs/ATTRIBUTION.md`
  (the pin is linked).
- Gates: `ci` green (140 `sortlens-ui` unit tests, all crates), `snapshot`, `deny`, `screenshots` (18 scenes) all green.
- **Next**: Phase 4 item 2 — `.icns`/`.ico` generation from `crates/icon` inside `xtask` (ROADMAP Phase 4
  item 2's first clause), then the High-Contrast preset (SL-L09), the opt-in `craft-update` (SL-I10) and
  `craft-i18n` scaffolding (SL-L07).
## 2026-11-23 (forty-fourth) — Phase 3B.6 part 9: the arrows the faces lack are triangles now

- **The defect was measured before it was fixed, and the measurement moved the slice.** A probe against the default
  font stack (`FontDefinitions::default()` of the pinned egui fork) found: the proportional chain (Ubuntu-Light →
  egui-icons → NotoEmoji-Regular → emoji-icon-font) has neither `←` (U+2190) nor `→` (U+2192), and also lacks `⇧`,
  `⌃`, `✕`, `✓`, `⌥` and `↵`; the monospace chain carries Hack, which has all of them; and the triangles `◀`/`▶`
  render in *both* chains (NotoEmoji in proportional, Hack in monospace). So the checkpoint's premise was half
  right: the keymap overlay's keycaps paint in `FontId::monospace` and had been drawing the arrows fine — the
  proportional surfaces the same strings reach were the broken ones: the palette's shortcut column (the registry's
  `cull.back` row would draw a box if it were ever listed), and the cull start screen's hint line, which painted
  `← back` today.
- **One spelling everywhere, chosen for both faces.** Every painted arrow is now `◀`/`▶`: the keymap's four
  `keys` strings (`▶ / Space`, `◀`, `◀`, `▶ / ◀`), the registry's `cull.back` `shortcut` — which with it changes
  the CLI golden `24 shortcuts.txt` — and the cull hint. The same audit retired the other proportional tofu the
  shell painted: the keymap's Shift row and the strip's count hint say `Shift-click` (⇧ is not in the proportional
  face), the keyword menu marks active rows with a strong `•` instead of `✓`, and the strip's clear chip is `×`
  (U+00D7, which Ubuntu-Light has) instead of `✕`. `fonts.rs`'s doc claimed the keymap draws `⌃`, `✕`, `◀` and
  `✓` — no code drew `⌃` and the defaults lack `✕`/`✓`; it now names what the UI really paints (`◀`/`▶`, `↔`,
  `⌘`) and why `Zsym` stays required.
- **Two guards, both failing on the old spelling.** `keymap::every_painted_keycap_and_action_exists_in_the_face_that_paints_it`
  asserts every binding's `keys` renders in the monospace face and every action *and every ready registry
  `shortcut`* in the proportional one — verified to fail when the registry's `◀` is reverted to `←`. And
  `shell::the_shortcuts_dialog_paints_only_glyphs_the_bundled_faces_have` paints the shortcuts overlay headlessly
  with an empty query (every binding) and a `◀` needle, asserting no painted text lacks a glyph.
- **Visual verification**: new `shortcuts-charcoal`/`shortcuts-paper` baselines, scene narrowed to the triangles
  themselves (`shortcut_query = "◀"`, `3 of 43 bound`) so the keycap column's glyphs are the subject; both read at
  native scale and zoomed — Detail's `◀`, Cull's `◀` and Compare's `▶ / ◀` are real glyphs pointing the right ways
  in both presets, and the search field's own `◀` renders. The other 29 baselines regenerated byte-identical (no
  other scene paints these glyphs — the strip hint and keyword menu need a selection or an active filter).
- Files: `crates/ui/src/keymap.rs` (four keycap strings, the Shift reword, the module doc's new note, the new
  face-coverage test), `crates/engine/src/command.rs` (`cull.back`), `crates/ui/src/shell.rs` (cull hint, strip
  hint, keyword menu marker, strip chip, the overlay audit test), `crates/ui/src/fonts.rs` (reconciled doc),
  `crates/ui/tests/screenshots.rs` (new scene), `apps/sortlens-cli/tests/golden/24 shortcuts.txt` (regenerated via
  `cargo xtask snapshot --update` and diff-read), and the reconciled `docs/UX.md` (§5.4 steps, §4.3 Shift-click)
  + `docs/ROADMAP.md` (3B.6 (g) ticked).
- Gates: `ci` green (131 `sortlens-ui` unit tests), `snapshot`, `deny`,
  `screenshots` (17 scenes) all green.
- **Next**: Phase 4 item 16, the `craft-keymap` editor in Settings → Shortcuts (ROADMAP Phase 4 item 1, SL-I09:
  rebinding, conflict detection, import/export). One caveat for whoever runs it: the peer agent working issue #1
  (Places map not loading) is building its fix in a separate worktree off `HEAD` and will rebase after this push;
  if its work has landed first, rebase this branch before starting and re-read the Places baselines. Also noted for
  a later slice, found while auditing: `⌥-drag pan` (shell.rs doc) is not a painted string, but `0–5`-style key
  names in `hints` and the compare footer should get the same audit when Phase 4 item 1 rebuilds the keymap editor.

## 2026-11-23 (forty-third) — Phase 3B.6 part 8: the cull foot is claimed before the photograph

- **The defect was an allowance, not a measurement.** `cull_view` sized its canvas as
  `available_height - strip_height(84) - action_height(46) - 16`, where 84 and 46 accounted for the up-next strip and
  the decision row — and nothing accounted for the keyword band, the field that adds a keyword, the live line or the
  staged sentence below them. At 1440×900 the field was painted at y 846–861 with the bottom bar at 852: its glyphs
  were cut in half, and `Photo 1 of 12` under it was not painted at all. The screenshot baseline showed it plainly
  and the defect had been recorded in 3B.6 since the suite's first run.
- **The fix inverts the order and removes the guess.** The block under the photograph is now
  `egui::Panel::bottom("cull_foot")` with `.exact_size(cull_foot_height(…))`, claimed *before* the canvas; the canvas
  is then allocated out of `ui.available_height()`, so there is no window height at which the two can overlap — the
  photograph gives way, never the controls. `cull_foot_height` is arithmetic over rows of fixed height (the up-next
  strip 56, a `interact_size.y` row for the decisions, the keyword band, a row for the field, a line for the spoken
  status); only the band and the staged sentence are conditional, and both conditions — does the photo on screen carry
  keywords, is anything staged — are known *before* the block is laid out. The live line's height is a galley
  measured from the font it is painted with, so a taller face cannot push the field out. This is the same discipline
  as `sidebar_foot_height`, and it needs no extra frame: an auto-sized panel is the thing that hangs below its own
  clip on the first frame.
- **The keyword band scrolls instead of growing, which is also the better behaviour.** Two chip rows, in a
  `ScrollArea::vertical().min_scrolled_height(0.0)` — the fork's scroll area has a 64-point floor of its own, taller
  than the band, and a band that ignored the height it was given pushed the field out of the panel on the first
  attempt. The warrant is not only the height budget: an AI pass can leave sixty keywords on one frame, and a band
  that grew with its content would resize the photograph every time the queue moved. Culling wants a still canvas.
- **Two live lines became one, and that is a real fix rather than a saving.** `state.announce` was never cleared, so
  after the first decision the announcement sat under the queue position for the rest of the session — saying the
  same kind of thing twice and costing the photograph a row. The foot now paints `state.announce.take()` when there is
  one and the queue position otherwise: the announcement is spoken in the frame it is made, and the position returns
  behind it. Deciding happens after the strip has painted and before the band, so the queue never advances underneath
  a row that is still being laid out.
- **The test is the arithmetic's audit, and it fails on the old layout.** `the_cull_foot_stays_above_the_bottom_bar_at_nine_hundred_points`
  paints the real shell at 1440×900 with a live session and two keywords on screen, then: (1) every cull text whose
  rect starts in the column must be *visible* — its rect intersected with the clip rect it carries — and end above the
  bottom bar, with the live line and the `sea`/`portrait` chips and the `Add` button all on screen (on the old layout
  the live line was below the fold and the field's own glyphs were clipped); (2) the photograph still has room to
  itself (the canvas rect, found as the largest rectangle filled with the thumbnail well's colour, is over 300 points
  tall), so a "fix" that reserved the whole column for controls fails; (3) sixty keywords paint the canvas at exactly
  the height two did, the last chip is *not* visible, and a wheel over the band brings it into view across the frames
  smooth scrolling eases through — so the band is a scroll and not a clip. Non-vacuity was checked by shrinking the
  panel by 40 points: the test then fails on the live line, exactly where the old code failed.
- **Visual verification**: `cull-charcoal` and `cull-paper` regenerated and read — `Up next` with its four cells, the
  decisions and the four export/apply buttons, the `sea ×` chip, `Add keyword to this …` with its `Add` button and
  `Photo 1 of 12` are all inside the column with clear space above the bottom bar, and the photograph is ~40 points
  shorter, which is the room the field had been hiding in. `places-charcoal`/`-paper` were re-painted in the same run
  for a reason that is not this slice: they had been committed with the status line one point off what this machine
  paints (a rounding difference, visible as a 1-point vertical shift of the credit line plus scattered subpixel AA)
  and `places-charcoal` was already failing `cargo xtask screenshots` on `HEAD` before this change. All other baselines
  are byte-identical.
- Files: `crates/ui/src/shell.rs` (`cull_view`'s foot panel, `cull_foot_height`, `cull_chip_band`, `CULL_UP_NEXT`,
  `CULL_CHIP_ROWS`, the keyword band's scroll area, one merged live line, one new test),
  `crates/ui/tests/snapshots/cull-{charcoal,paper}.png` and `places-{charcoal,paper}.png` (regenerated), and the
  reconciled `docs/UX.md` §5.5 + `docs/ROADMAP.md` (3B.6 (f) ticked; (g) left open in its own row rather than folded
  into the ticked one).
- Gates: `ci`, `snapshot`, `deny` and `screenshots` (16 scenes) all green; 129
  `sortlens-ui` unit tests, the new one among them.
- **Next**: 3B.6 (g), `←`/`→` as text in the keymap overlay's keycaps and the registry's `shortcut` strings, where no
  bundled face has the glyph — a fix that has to touch `crates/ui/src/keymap.rs`, `crates/engine/src/command.rs` and
  the CLI goldens together. Phase 4 items 16–18 after it.

## 2026-11-23 (forty-second) — Phase 3B.6 part 7: the canvas header keeps its name whole

- **The defect was a layout that could not refuse.** The header was one `ui.horizontal` with the title and the live
  count on the left and a `Layout::right_to_left` child for the controls. In egui a `horizontal` hands its last
  child whatever the earlier ones left over, so when that number ran out the right-to-left row was laid out from a
  cursor already past the right edge. At 900×620 with both docks open the canvas is ~310 points wide and the row
  needs ~390: the title was painted at 287–335 *underneath* the sort combo at 283.5–348.5, and the count underneath
  the scope chip. Reproduced exactly, on the old code, by the new sweep test before the fix.
- **The fix is the fork's own `Sides` row, and it measures rather than hopes.** The row is now
  `egui::containers::Sides::new().spacing(6.0).shrink_left().truncate()`: the controls are laid out and measured
  first, the left block's max rect is the remainder, and the words give way instead of the controls. An overlap is
  no longer a width-specific slip waiting to happen — there is no width at which the left block may enter the
  controls' rect.
- **The degradation is stated and measured, in that order.** The count's galley is measured against the room left
  beside the name and the count steps aside when it will not fit (it is repeated by the filter strip's own summary
  and spoken by the live line); the two chips are offered only above `HEADER_CHIP_ROOM` (430 points) because the
  scope is also cleared from the sidebar and the active-filter count is also on the top bar's Filter chip, while the
  sort combo and the two grid glyphs exist nowhere else on that line. Measured over the sweep: with a fresh
  `UiState` per width the count appears from a 942-point window and the chips from 1059, so at 900 the header is
  exactly the view's name, the sort and the two grid glyphs. A header with room to spare still shows all four — the
  thirteen 1440×900 baselines are byte-identical, which is the evidence.
- **The test is a sweep, and it fails on the old layout.** `the_canvas_header_title_never_runs_under_the_sort_combo_at_any_width`
  paints the real shell at **every window width from 900 to 1300 points in one-point steps**, each from a fresh
  `UiState` (a reused one carries the panel sizes the previous frame left behind, which would make the sweep depend
  on how many frames came before it). Each step asserts: the sort combo is painted; the title is painted, is *not*
  truncated, lies inside its own clip rect, and intersects neither the combo nor either chip; and the count, when
  painted, intersects nothing. The canvas is identified as the clip the sort combo was painted in, because the
  sidebar paints its own `Library` row on the same line. Old code: `at 900 points the title at [[287.0 72.0] -
  [335.0 87.0]] is painted over Added order at [[283.5 73.5] - [348.5 82.5]]`. Non-vacuity is asserted in both
  directions: the count and the chips must be absent at 900 and present at 1300, so a header that dropped them
  everywhere fails as loudly as one that kept them in a row they do not fit.
- **Visual verification**: `narrow-canvas-charcoal` and `narrow-canvas-paper` added at 900×620 with both docks open,
  scoped to the library folder, and read: `Library` whole at the left, the density and layout glyphs and the
  `Added order` combo beside it with clear space, no count and no chips, and the breadcrumb, strip and grid below
  them unchanged. Every other baseline was regenerated under `--update` and none was rewritten — the change is
  invisible at 1440 by construction, and the new pair is where it is legible.
- Files: `crates/ui/src/shell.rs` (`canvas`'s `Sides` row, `HEADER_CHIP_ROOM`, `HEADER_COUNT_GAP`, one new sweep
  test), `crates/ui/tests/screenshots.rs` (the new scene), `crates/ui/tests/snapshots/narrow-canvas-{charcoal,paper}.png`
  (new), and the reconciled `docs/UX.md` §5.1 + `docs/ROADMAP.md` (3B.6 (e) ticked, (f)–(g) left open in their own
  row). `docs/UX.md` §5.1's diagram was stale in a second way — it showed the count at the right edge and no chips —
  so it now shows the shipped row and states the degradation order.
- Gates: `ci`, `snapshot`, `deny` and `screenshots` (16 scenes) all green; 128
  `sortlens-ui` unit tests, the new sweep among them.
- **Next**: 3B.6 (f), the cull keyword row clipping under the bottom bar at 900 px, then (g), the `←`/`→` text in
  the keymap overlay and the registry's `shortcut` strings; Phase 4 items 16–18 after them.

## 2026-11-23 (forty-first) — Phase 3B.6 part 6: the strip's count gets a column instead of a chip's lap

- **The defect was in the layout, not in the drawing.** `filter_strip` ended its `horizontal_wrapped` row with a
  `Layout::right_to_left` child holding `N shown`. A wrapped layout hands its last child the *row's* width, so that
  label right-aligned itself against the card's edge from wherever the wrap cursor happened to be — and when the
  row's last chip already reached into the label's own width, the two were painted on top of each other. That is
  what the `short-sidebar-*` baselines showed: `Clear` at 873–935 and `12 shown` starting at 905.
- **Reproduced at a width the test controls.** The full-shell scene at 1280 was not a reliable reproducer, because
  a chip's 58-point button either fits on the row or wraps to the next one; the collision only happens in the narrow
  band where it *just* fits. So the test drives the real `filter_strip` through `Context::run_ui` at **every width
  from 320 to 760 points in one-point steps** and asserts the invariant at each: no painted text in the strip
  intersects the count, and the count lies inside the clip it was painted with (a fix that hid it would fail). On
  the old code it fails at 326 points with `2 shown at [[276.8 51.5] - [316.8 60.5]] is painted over ⚠️ at
  [[265.0 52.5] - [278.0 63.5]]` — the `Missing` chip's own glyph, not merely a word. The sweep is not vacuous: at
  least one width must have the chips on more than one row (grouped in 10-point bands, because a font's own 1-point
  centre differences are not rows) and at least one must put the count's centre inside the chips' band.
- **The fix reserves the column before anything wraps.** The count's galley is measured with
  `layout_no_wrap(…, FontId::proportional(11.0), t.text_faint)`, the strip's max width is reduced by that plus an
  8-point gap, and the chips wrap inside the remainder; the label is painted into the right of the card at the
  chips block's vertical centre. Because the space is taken away *before* wrapping, there is no width at which the
  two can meet, and the chips' own wrapping is unaffected on a wide window (every 1440×900 baseline diff is exactly
  the count's 47×10 rect). Two edges are deliberate: `set_min_width` restores the card's full width, because a
  narrower content rect would otherwise shrink the card to its widest row; and a single chip wider than the column
  (a long collection name) is clipped at the column boundary with `shrink_clip_rect` rather than painted across the
  count. Below `STRIP_CHIP_ROOM` (180 points of chip room) the count steps aside and the chips keep the full width:
  a chip is a control, the count is a statement.
- **Visual verification**: all 29 baselines regenerated and read. `short-sidebar-charcoal`/`-paper` (1280×620) are
  the defect's own scene: `Clear` now wraps to a second row at the left, `12 shown` sits in its column with clear
  space around it, and the strip is one row taller (the grid moves down a row). Every other scene's diff bbox is
  only the count's own rect — measured with `ImageChops` per file, not assumed; `filters-*` and `no-matches-*`
  already had two rows and only the count's vertical position moved.
- Files: `crates/ui/src/shell.rs` (`filter_strip`'s reserved column, `STRIP_CHIP_ROOM`, one new sweep test), all 29
  `crates/ui/tests/snapshots/*.png` (regenerated), and the reconciled `docs/UX.md` §5.2 + `docs/ROADMAP.md`
  (3B.6 (h) ticked, (e) named as the next slice).
- Gates: `ci` (fmt had to be re-run first), `snapshot`, `deny` and `screenshots`;
  127 `sortlens-ui` unit tests, the new one among them.
- **Next**: 3B.6 (e), the canvas header's title under the right-aligned sort combo at 900 px with both docks open,
  then (f) and (g); Phase 4 items 16–18 after them.

## 2026-11-23 (fortieth) — Phase 3B.6 part 5: the sidebar scrolls above its foot instead of over it

- **The defect, reproduced before it was fixed.** At 1280×620 the left panel holds six scopes, two subfolders,
  `FOLDERS`, twelve keyword rows and `COLLECTIONS`; the foot claims the last ~133 points, and egui happily laid
  the sections out past the hairline, inside the panel's clip rect but *over* the four utility rows. The new test
  failed on exactly that before the fix: `keyword kw04 is visible at [[45.0 446.5] - [73.0 455.5]], inside the
  foot's band ([[0.0 451.0] - [264.0 580.0]])`.
- **The fix is a clip, not a guess.** Everything below the `Library` header now lives in one
  `egui::ScrollArea::vertical()` with `.id_salt("sidebar_sections")` and `.auto_shrink([false, false])`, so the
  viewport is exactly the room the foot left. The foot is still laid out *first* (its `Panel::bottom` claim and
  its exact height arithmetic are unchanged), which is what makes the viewport's bottom edge the hairline; a row
  that does not fit is now scrolled rather than drawn across the utilities. `auto_shrink` keeps the bar absent
  while the sections fit, so nothing changes on a tall window.
- **Two halves of the claim, because hiding is not fixing.** The test paints the whole window at 1280×620 over a
  library tagged with twelve keywords, and then: (1) **nothing from the sections is visible in the foot's band** —
  "visible" is each painted text rect intersected with the clip rect its shape carries, so a row the scroll area
  pushed past the viewport does not count while a row drawn over the utilities does; the four foot labels must all
  be *readable* there, so a fix that clipped the footer too would fail the same test; and (2) **the sections
  scroll** — a `MouseWheel` event over the sidebar brings a keyword that started below the fold into view, and
  `Recently Deleted` does not move a point. The test refuses to pass vacuously: it asserts that fewer keyword rows
  are visible than exist, so growing the window instead of fixing the layout says so out loud.
- **Visual verification**: `short-sidebar-charcoal` and `short-sidebar-paper` added at 1280×620 and read — the
  sections are cut cleanly at the hairline, the four utility rows sit on the bottom edge with clear space, and no
  section text is drawn across them. `filters-charcoal`/`-paper` and `no-matches-charcoal`/`-paper` (1200×800)
  were regenerated and read: their only change is that `+ New collection` — which used to sit *on* the hairline — is
  now clipped above it. Every 1440×900 baseline is byte-identical, which is the evidence the tall window is
  untouched. The five `places-*` `.diff.png`/`.old.png` leaves from an earlier failed comparison were removed.
- **One defect found while reading the new baselines, not fixed here**: at 1280 points with both docks open the
  filter strip's right-aligned `N shown` summary is painted *under* the `Clear` chip, so the two overlap. It is
  recorded as ROADMAP 3B.6 (h) and is the next slice; it is visible in the new `short-sidebar-*` baselines on
  purpose, because hiding it by choosing a flattering window width would make the baseline a claim rather than
  evidence.
- Files: `crates/ui/src/shell.rs` (`left_panel`'s scroll area, one new test), `crates/ui/tests/screenshots.rs`
  (the new scene), `crates/ui/tests/snapshots/short-sidebar-{charcoal,paper}.png` (new),
  `crates/ui/tests/snapshots/{filters,no-matches}-{charcoal,paper}.png` (regenerated), and the reconciled
  `docs/UX.md` §4.4 + `docs/ROADMAP.md` (3B.6 (d) ticked, (e)–(g) left open in their own row, (h) opened).
- Gates: `ci`, `snapshot`, `deny` and `screenshots` all green; 127 `sortlens-ui` unit
  tests including the new one, and the workspace suite has no failures.
- **Next**: 3B.6 (h), the strip's `N shown` summary colliding with `Clear` at 1280, then (e), (f), (g) in that
  order; Phase 4 items 16–18 after them.

## 2026-11-23 (thirty-ninth) — Phase 3B.6 part 4: Places has a coast before you ask for one

- **The outline ships, and it is the one the project may ship.** `sortlens_geo::coastline` embeds
  `assets/coastline.txt` with `include_str!`: the Natural Earth **1:110m coastline**, which is public domain, so the
  earlier "no bundled basemap" decision (ROADMAP part 3a) was not overturned but narrowed — satellite and street
  tiles still need data the project cannot ship, while a world outline in the public domain does not. 134 shorelines,
  5,128 points, rounded to 0.001° and one polyline per line, with the source, the transform and the licence in the
  file's own header. 77 KB of text, inspectable in a diff, and no fetch at run time.
- **The antimeridian splits a line at parse time.** The source happens to keep no walk across ±180°, but a parser that
  joined one would paint a stripe through every other continent; two consecutive points more than 180° apart now end
  the run, and the test feeds exactly that walk and asserts two shores and no segment wider than 100 px at zoom 2.
- **Antarctica is clamped, and only Antarctica.** Six source points sit below −85.05°, where Mercator diverges.
  Clamping them to the limit draws the continent's edge along the projection's own border; the alternative is a
  missing continent. A *photograph* beyond the limit is still refused rather than clamped (that decision is unchanged
  — a coordinate is a claim about a photograph, a coastline is a drawing of land), and the test walks every projected
  point and asserts it is inside the world square.
- **Projection happens once, at parse time.** `screen_lines` scales zoom-0 world pixels by `2^zoom` and subtracts the
  viewport origin, so five thousand tangents and logarithms are not computed per frame; and it returns only the runs
  of consecutive segments whose box touches the viewport, so the caller gets geometry it can hand straight to a
  painter instead of a world-sized polyline for egui to clip. An impossible viewport (zero width, non-finite origin)
  returns nothing instead of panicking in a paint path.
- **Refusals are counted, not swallowed.** `Coastline::skipped()` counts a malformed token, a coordinate off the globe,
  and a line left with one point. The shipped file asserts **zero** — that is the test that would catch a truncated
  `include_str!`, a lon,lat export, or a rounding pass that wrote `NaN`.
- **Places opens fitted, which was the reported defect.** `coastline::bounds_centre` is the middle of the photographs'
  projected box, not the mean of their latitudes: Brighton and Lima average 19°N, 1,400 km north of the box's middle,
  and the old mean-based centre pushed the northern dot towards the edge of its own fit. `coastline::fit_zoom` picks
  the deepest zoom at which the whole spread fits inside the viewport with 28 px clear on every side, never deeper
  than the caller's ceiling (a tile folder's top zoom) or `MAX_FIT_ZOOM` (10, because the outline is 1:110m and a
  deeper view would be a staircase). `UiState::places_zoom_chosen` is the flag that separates the view's fit from the
  person's choice: the fit runs every frame until `+`/`−` is pressed, after which the zoom is theirs and a library
  that grows does not move their map.
- **The view draws one basemap, never two.** A usable `tiles/` folder is painted as before and the shipped outline is
  skipped; with no folder the outline is stroked in the new `Tokens::map_coast` (one value per preset, geography
  rather than chrome) under the dots, and the credit line swaps to `Coastline: Natural Earth 1:110m (public domain)`.
  The "no basemap is installed" copy is gone because that state no longer exists: it now says the coastline is
  schematic and names the `tiles/` folder that would replace it.
- **Tests that fail when the behaviour is absent** (verified by two mutations, both reverted): hardcoding `fit_zoom`
  to 5 fails `fitting_the_view_puts_the_photographs_on_screen_where_a_fixed_zoom_loses_them` (which first asserts the
  old zoom really did lose both dots) and the new shell test on its first assertion; skipping the coastline paint
  fails `places_reports_the_basemap_or_why_there_is_none` on "a library with no tile folder still gets the shipped
  coastline" and the new shell test on "the shipped outline is the basemap when no tiles are installed". The stale
  full-output `paint` closure in the basemap test now returns its output so the same test can assert both halves —
  outline present without tiles, absent with them.
- **The new `run_ui` test** paints Places at 1200×800 over a library in Brighton and Sydney: the view chose a zoom
  below 5, exactly two filled dots (radius `dot_radius(1)`) are painted and both centres are inside the map rectangle
  (a dot's ring is a second circle, which is why the count filters on the fill), the coastline runs are strokes in
  `map_coast` with at least one longer than four points and every point finite, a manually chosen zoom 9 survives the
  next frame, and a library with no location paints neither outline nor dot.
- **Visual verification**: `places-charcoal` and `places-paper` regenerated with `cargo xtask screenshots --update`
  and read — both show the coastlines under the Brighton and Lima dots, the new credit in the footer, and the schematic
  notice naming the tiles folder; the scene now pins zoom 3 *and* `places_zoom_chosen`, because a baseline must not
  move when the fitting arithmetic is touched. No other baseline moved.
- Files: `assets/coastline.txt` (new), `crates/geo/src/coastline.rs` (new, 7 tests), `crates/geo/src/lib.rs`,
  `crates/geo/src/tiles.rs`, `crates/ui/src/theme.rs` (`map_coast`), `crates/ui/src/shell.rs` (the fit, the coastline
  pass, the credit, the status copy, `places_zoom_chosen`, `coastline_runs`, one new test, one extended),
  `crates/ui/tests/screenshots.rs`, and the reconciled `docs/{ROADMAP,UX,ARCHITECTURE,ATTRIBUTION,RELEASE}.md` +
  `README.md`. ROADMAP's 3B.6 row is split again: (c) ticked, (d)–(g) still open in their own row, nothing moved into
  an absent table.
- 714 tests (seven geo, one shell new, one shell extended); ci, snapshot, deny, screenshots, a11y, docs,
  layers, roots, assets, attribution green.
- **Not verified by hand**: the outline at a zoom where its 0.001° steps are visible as staircases (the arithmetic
  says zoom 8+ and the Paper baseline is the deepest read), a library whose photographs ring the globe at ±180°
  (the fit would choose zoom 0 and the world square would sit inside the viewport with sea beside it), and whether a
  two-line `Basemap` footer plus the `places.txt` line crowds the map at 620 px of height — that is defect (d)'s
  neighbourhood, not a claim made here. **Also visible in the baseline and left as it is**: at 1:110m a coastal city's
  dot can sit a few pixels offshore (Lima's does), because the outline is a world map's coast and not a harbour chart —
  which is why the credit says 1:110m rather than claiming a map.

## 2026-11-23 (thirty-eighth) — Phase 3B.6 part 3: a library filtered to nothing says so, instead of greeting you

- **The empty canvas is decided by two counts, not one.** `canvas` already carried the filtered count, and the
  empty screen derived its stage from `engine.stats().total` — so the question "is there a library" and the question
  "is there anything to draw" were answered by the same number, and a filter that hid all twelve photographs
  reached the Welcome card: *Welcome to Sortlens · Point it at a folder…* with `Add Folder…` as the primary action,
  the one action that could not help. The new `canvas_empty(roots, scanning, library, shown, dismissed)` returns a
  four-way `Empty` — `Add`, `Indexing`, `NoMatches { library }`, `Quiet` — so the wizard is gated on the catalog and
  the no-matches screen on the filtered count. `Welcome` is gone as a name because the canvas is no longer decided
  by the wizard alone.
- **`Not now` was a button that did nothing.** `empty_library` read `welcome_stage(...).unwrap_or(Welcome::Add)`, so
  the `None` a dismissed wizard returned was turned straight back into the wizard it had just refused. Dismissal now
  wraps the two wizard stages only; `NoMatches` ignores it, because a screen that explains why the grid is blank is
  not an interruption to be dismissed. Both halves are pinned by the stage table, and the old test that asserted
  `welcome_stage(0, false, 0, true) == None` was asserting the function's intent while the caller overrode it.
- **The screen itself**: "Nothing matches these filters" with the library's own count in words ("12 photographs in
  this library, and the current filters exclude every one of them") and one `Clear filters` button in the same shape
  as the Welcome card's, so the two empty states are one family. The button clears the search text *and* every
  filter dimension and forgets the folder scope — exactly what the header's `N active` badge already counted, and
  the reason a search-only zero-result also gets this screen rather than a mystery. The count is never the filtered
  zero: the sentence is about what the filters cost, so it has to name what they cost.
- **Tests that fail when the behaviour is absent** (one rewritten stage table + one new `run_ui` test; verified by
  changing `Empty::NoMatches { library }` back to `Empty::Add`, which fails the new test on "the canvas says why it
  is empty instead of greeting the person"): the table covers the empty catalog, an empty root, a running scan,
  photographs shown, dismissal over both wizard stages, and a filtered-to-zero library both dismissed and not. The
  window test scans two real JPEGs, narrows with `min_rating = 5`, asserts the card and the "2 photographs in this
  library" sentence are painted and *"Welcome to Sortlens" is not*, then presses the button with a real pointer
  event and asserts every dimension cleared and both photographs are back; it then narrows with search text alone
  and asserts the same screen offers the same way back. The other half — an actually-empty catalog — asserts the
  Welcome card and that no filter screen is painted over it.
- **Visual verification**: a new `no-matches-charcoal`/`-paper` baseline pair at 1200×800 with both docks open,
  generated by `cargo xtask screenshots --update` and read. Both show the strip above saying `Videos · 0 shown` and
  `2 active` (the folder scope plus the kind filter), the card, the library's 12, and the button — and neither shows
  any part of the Welcome card. No existing baseline moved, which is the check that the change is confined to the
  canvas with nothing to draw.
- 706 tests; ci, snapshot, deny, screenshots (14 scenes, 31 PNGs) all green. `docs/ROADMAP.md` gained the
  3B.6 (b') row and its stale "Eighteen PNGs" count, `docs/UX.md` §5.12 now says which two empty states exist and
  which count decides each, and `docs/ARCHITECTURE.md`'s screenshot paragraph lists the sixteen surfaces.
- **Not verified by hand**: the card at the minimum window size, where the two-line sentence and the button still
  fit (the rationale is arithmetic, not a picture), and the copy's tone in the Paper preset beyond the baseline.

## 2026-11-23 (thirty-seventh) — Phase 3B.6 part 2: the strip asks in pictures, and only when there is something to narrow

- **The strip is not painted on an empty library, and "empty" is the catalog, not the filter.** Over a first-run
  library it was nine controls asking questions about photographs that do not exist yet, sitting on top of the
  Welcome card whose whole job is to say where photographs come from. The gate is `engine.stats().total`, never the
  filtered count `canvas` already has: a filter that hides every photo must keep the strip, because clearing it is
  what the strip is for. The rule lives at the call site, so the Filter toggle in the top bar can still be on when
  there is nothing to show — and the moment a photo is indexed the strip appears at that same toggle state.
- **The rating chip answers in stars, not in `3★ and up`.** One star glyph per star of the minimum, tinted `accent`,
  in a themed chip; at rest it is the old faint star plus the word "Rating", because a chip with no picture and no
  word would be a mystery. The typed `★` is gone from the strip — a character the bundled faces render at different
  widths and heights and not at all in some — and so is the numeral, because the count was the answer all along.
  The cycle (0→1→…→5→0) is unchanged: the pictures changed, the behaviour did not.
- **The colour chip paints the colour.** A filled 4.5-point dot in the chosen label's own token colour, an outlined
  ring for "any colour", and the colour's name beside it (a dot alone is not an answer for a colourblind reader).
  The `droplet` glyph that stood in for every colour is out of the strip.
- **The flag chip wears the state's own glyph** — `pick`, `reject` or `flag` — through `flag_filter_icon`, a pure
  mapping with its own assertions, so the four states are pictures before they are words. The keyword menu gained
  the set's `chevron`, because it is a menu and now looks like one.
- **The row wraps instead of overflowing.** Nine chips plus a five-star minimum are wider than the canvas at 1200
  points with both docks open, which pushed `Clear` — the one control that undoes the strip — past the canvas edge.
  `horizontal_wrapped` turns that into a second line. This is a defect the new test found rather than one the note
  named: the assertion "the strip stays when a filter hides everything" failed because `Clear` was never painted at
  all, and the frame's own text dump showed the row ending at `Missing`. A five-star chip has the same width as the
  old `5★ and up` label, so this predates the slice; it is fixed here because a filter bar that cannot be cleared is
  worse than a taller one.
- **Tests that fail when the behaviour is absent** (3 new, 1 updated; verified by removing the gate, which fails
  `the_filter_strip_needs_a_library_to_narrow` on its first assertion): the strip absent over the Welcome card and
  back after one indexed photo, still present — and still counting — when the filter answers nothing; painted alone,
  the rating chip at three stars paints exactly three 12-point glyphs in a row and no `★` or numeral, the colour
  chip's circle is `t.label(Green)` filled and the unset one is an outline; clicks on the painted star and on the
  swatch drive `min_rating`/`label_filter` (a `Button` atom really is the button). Two new test helpers flatten
  `Shape::Vec` (`all_shapes`, `painted_pictures`, `painted_circles`), because a picture inside a button is nested and
  a top-level scan reads as "nothing painted" — the trap the existing `label_at` already avoided for text.
- **Visual verification**: a new `filters-charcoal`/`filters-paper` baseline pair at 1200×800 with three stars and a
  green label — the only baselines that show the strip *asking* something. Both read: three accent stars in the
  chip, a green swatch beside the word "Green", and `Missing | Clear` wrapped onto a second line with `2 shown`
  right-aligned under the first row. All 26 existing baselines regenerated; the Welcome pair lost the strip (the
  fix) and every other scene's only change is the strip's own glyphs. The narrow 900×620 baseline still fits on one
  row, so wrapping costs nothing where it is not needed.
- Files: `crates/ui/src/shell.rs` (the gate, `stars_atom`, `rating_chip`, `colour_chip`, `flag_filter_icon`, the
  chevron, `horizontal_wrapped`, the test helpers and four tests), `crates/ui/tests/screenshots.rs` (the filter
  scene), `crates/ui/tests/snapshots/*` (28 regenerated, 2 new), and the reconciled `docs/{ROADMAP,UX,ARCHITECTURE}.md`
  — the ROADMAP row is split so (a) and (b) are ticked and (c)–(g) stay open in one row, with no work moved into an
  absent table.
- 705 tests (3 new); ci, snapshot, deny, screenshots, a11y, docs, layers, roots, attribution green.
- **Not fixed, and observed**: with a filter that hides every photo the canvas shows the *Welcome* card ("Add
  Folder…") under the strip, because `empty_library` derives its stage from the filtered count. The strip is now
  right (it stays) but the copy under it is not — that is the "nothing matches these filters" empty state, and it is
  the next thing in this row's neighbourhood rather than a claim made here. Also unverified by hand: the wrapped
  row at a width where only one chip fits per line (the chip row would become a column of chips), and whether a
  five-star minimum plus a wrapped `Clear` reads as one control group at 1200 points.

## 2026-11-23 (thirty-sixth) — Phase 3B.6 part 1: the Welcome promises start with pictures

- **The three first-run promises carry the icon set's glyphs, not a typed bullet.** `folder` (nothing is uploaded,
  renamed or moved), `history` (one log you can step back) and `reject` (culling stages the decision — the same
  glyph the sidebar's Rejects row wears, so the vocabulary is the app's and not a new one). The `·` it replaced was
  typed text at 12 points: one speck whose height and weight differ per bundled face, which is exactly the kind of
  mark that reads as a typo rather than as a list.
- **A fixed 16-point glyph column**, `WELCOME_GLYPH_COLUMN`, holding a 14-point glyph centred at `left + 8`. The
  three rows use `ui.horizontal_top`, so the glyph sits on its title's line rather than floating between the title
  and the detail line, and every detail line starts at the same x — the same rule `nav_row`'s glyph column follows
  in the sidebar. The glyph is tinted `t.accent`, the colour the typed bullet had, so the rows still read as one
  accent-marked list in both presets.
- **One test, exact rather than a floor.** `the_welcome_rows_start_with_the_icon_sets_glyphs` paints `welcome_add`
  alone in a `Context::run_ui` (installing the SVG loader itself, because it does not go through `show`), then
  counts the textured rectangles: exactly three, each 14×14, all at one x, increasing down the page, and each one
  left of and on the same line as its own title (`label_at`). It also asserts no item containing `·` is painted —
  the regression the slice exists to prevent. Verified failing with the paint removed: `left: 0, right: 3`.
- Files: `crates/ui/src/shell.rs` (the rows, the constant, the test), the regenerated
  `crates/ui/tests/snapshots/{welcome-charcoal,welcome-paper}.png`, and the reconciled `docs/{ROADMAP,UX}.md`.
- **Visual verification**: `cargo xtask screenshots --update`; only the two Welcome baselines changed (every other
  baseline byte-identical, so the slice touched nothing else), and both were read. The charcoal and paper frames
  show the three accent glyphs in one column, each beside its title, with the `·` gone and the copy unmoved.
- 702 tests; ci, snapshot, deny, screenshots, a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: the glyph column at `pixels_per_point` 2 and the rows with a detail line that wraps
  (both details are one line at 420 points today, and a wrapped one would leave the glyph beside the title only).
  Both are named here rather than claimed fixed.

## 2026-11-23 (thirty-fifth) — Phase 3B.5: the fourth question, and a rail that never goes away

- **`RightTab::ALL` is four.** Details, Tags, Suggestions, Activity — Suggestions between Tags and Activity, which is
  the order `docs/UX.md` §6 has claimed since the panel was designed and which the enum now enforces because the rail
  paints in `RightTab::ALL` order. The panel's own tab row is gone: it was four labelled buttons that ate the top of a
  250–420 px column to say what a glyph now says.
- **The far-edge rail.** `right_tools` is a 30-point `Panel::right` called *before* `right_panel`, so it owns the
  window's edge and the body sits inside it. It paints whether `state.right_open` is true or not — that is what makes
  it a rail and not a tab bar — every entry is `Button::image` with `Role::Tab` and the tab's name as its accessible
  name and tooltip, and the active one carries the selection fill. Switching tools keeps the body open; clicking the
  *active* tool closes it. That second behaviour is the one design decision here: the pattern's whole promise is "the
  panel never disappears when you switch tools", and a rail with no way to close the body would hold 340 points of a
  900-point window forever. The panel header now prints the active tool's name in words, because the rail is glyphs.
- **The Suggestions tab** answers for the whole selection, not one photo: `suggestions_for_all` is merged into one row
  per keyword with the best confidence any selected photo offered and the number of photographs it applies to. Rows are
  grouped into three families — Objects (`Detector`), Scene (`Pack`/`Caption`), EXIF (`ExifHint`) — because "the camera
  said so" and "a model guessed" are different kinds of claim, and one family list would ask a person to trust them
  equally. Each row carries `+ keyword`, its confidence, its source, and `×`; each family carries `Accept all`. The
  count prints only when the proposal is *partial* (`1×` of a three-photo selection) — `3×` on every row is noise.
- **Accepting a kind is one ⌘Z.** `write_tags` replaced the body of `set_tag_from` and now takes a list of keywords, so
  `accept_suggestions(ids, tags)` is the same write as typing one keyword, N times in one `Op::Group` — the words first,
  then their `Ai` provenance. `set_tag_from` keeps its exact behaviour (`last_tag` set whenever `add`, even when nothing
  changed; sidecars synced unconditionally) by delegating and doing those two things itself; the positive path returns
  `(photos changed, keywords added)` so the notice can say how many keywords, not how many rows.
- **Dismissal is stored, because a proposal is derived.** `Photo.dismissed_suggestions: BTreeSet<String>` with
  `#[serde(default, skip_serializing_if)]`, and `Op::SuggestionDismissed { changes: Vec<(PhotoId, Vec<String>)> }`
  carrying each photo's complete new set — the same read-then-replace shape as `Op::Tagged`, so the inverse is the
  previous set and one undo returns the proposal. `catalog::dismiss_suggestions` records only where something is
  actually new (dismissing twice is `Ok(0)` and spends no undo level); `suggestions_for` filters dismissed candidates
  *before* `merge` applies the eight-slot cap, so a refused proposal cannot occupy a live one's place. The revision is
  deliberately not bumped and nothing reaches a sidecar — `docs/BEHAVIOUR.md` §4 says both, next to provenance, for the
  same reason: another application must not inherit our opinion about what to offer.
- **Two doors onto it, not one.** `suggest.dismiss` is a `Ready` registry command (`{ids, tag}` → `dismissed`), with an
  MCP example and a `sortlens-cli dismiss <ids> <tag>` verb (`--json` schema included), because a decision that lives
  in the catalog and is undoable is a library change and the registry is where library changes are described. The CLI's
  non-JSON form prints how many photographs newly stopped being offered it.
- **A new glyph: `wand`.** Suggestions are a proposal, and reusing `sparkle` (memories) or `tag` (keywords) would have
  made the rail say something it does not mean. 40 glyphs now; the enum, the catalog, the folder and the rasterise
  tests all walk the new one.
- Files: `crates/catalog/src/{photo,op,store}.rs`, `crates/engine/src/{lib,command}.rs`, `crates/mcp/src/examples.rs`,
  `apps/sortlens-cli/src/main.rs` (+ two regenerated goldens), `crates/ui/src/{shell,icons}.rs`,
  `assets/icons/wand.svg`, `crates/ui/tests/screenshots.rs`, `crates/testkit/src/lib.rs` (`Camera` +
  `jpeg_with_camera`), and the reconciled `docs/{ROADMAP,UX,ARCHITECTURE,BEHAVIOUR,PACKS,CLI}.md`.
- Tests: catalog (dismiss → reopen → dismissed, one undo for the batch, redo, `Ok(0)` on a repeat, refusals for an
  empty keyword and an unknown photo, and a snapshot that never mentions the field when nothing was dismissed); engine
  (a dismissed proposal is not offered while the rest are, the cap counts live proposals only, accept-two-kinds on
  three photos is six keywords in one ⌘Z that undo removes wholesale, dismissal is one op and undo returns it); UI
  (`Context::run_ui`: the rail holds four glyphs, clicking the third opens Suggestions *with the panel still open*,
  switching to Tags keeps it open, clicking the active tool closes the body while the rail stays, and nothing asks the
  host for anything; the Suggestions tab paints `Scene` for a pack, `+ jpeg` on click tags all three selected photos
  with `TagSource::Ai`, one undo empties them, `×` dismisses for all three and the engine stops proposing it).
- **Visual verification**: `cargo xtask screenshots` — two new baselines (`suggestions-charcoal`/`-paper`) and all 25
  existing ones regenerated and read. The lab book was checked against the fixture: the fixture gained a keyword pack
  (`Coast`: `shoreline`/`golden hour` for `Trip`, `studio light` for `Studio`) and two frames with real camera EXIF
  (`Nikon Z6` at ISO 6400/24 mm, `Fujifilm X-T5` at 56 mm), because a fixture JPEG carries no EXIF and one family would
  not have tested a grouping. The baseline now shows Scene 3 + EXIF 5, `1×` on the two Trip-only keywords, `2×` on
  `studio light`, and the detail view's "Suggested" row carrying `+ fujifilm x-t5 + studio light + normal`.
- 701 tests; ci, deny, snapshot, screenshots, a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: the rail at a touch target's real size on a 2× display (14-point glyphs in 24-point rows is
  a mouse target, and a 30-point rail is thinner than the 44-point rule would like), and whether `Accept all` beside a
  long family name crowds the `×` column at the panel's 250-point minimum. Both are named here rather than claimed
  fixed.

## 2026-11-23 (thirty-fourth) — Phase 3B.4: nine destinations, and a bar that says what you are looking at

- **Detail, Compare and Soft Focus are destinations now, not a mode flag and a section.** `View::ALL` is nine:
  Library, For You, Cull, Detail, Compare, Places, Similar, Soft Focus, Insights. `state.detail_open` is gone; the
  question every keyboard guard used to ask is `View::is_detail()`, so the grid's shortcuts and the detail view's
  shortcuts cannot both answer the same press. `state.compare` is `Some` exactly while `View::Compare` is open —
  `activate_view` clears it on every other destination and `toggle_compare` closes the pane rather than the
  photograph — because a second pane that outlived the view drawing it was invisible state nobody could see.
  `open_detail` is the state-only path a grid double-click uses; `activate_view` is the one path for the tabs, the
  palette and the keyboard.
- **Soft Focus left the foot of Insights.** `soft_focus_view` polls the sharpness pass (the view that asked for it
  owns the report), states what a relative cut can claim, and reuses `sharpness_section` unchanged. Insights no
  longer scrolls to a review it was not about.
- **The bar itself.** Labels are measured (`tab_width` sums the real galleys), and `tabs_collapse` turns the row
  into the current destination plus a menu when the tabs and the controls beside them do not fit — the same nine
  items, on the icon set's own new `chevron` glyph, because no bundled face has `▾` (a caret character painted
  tofu in the 900 px detail baseline before this). The centre is an annotation pill: five stars (clicking the lit
  one clears it), Pick/Unflag/Reject, the primary's keyword count and "N selected". The stars and flags call the
  same `rate_selection`/`flag_selection` the `0–5` and `P`/`X`/`U` keys call, so the two cannot drift. The right
  side carries the three-step thumbnail slider in the grid (same `state.density` the canvas-header chip cycles,
  snapping through `Density::index`/`from_index` so there is no fourth step), Fit/1:1 in Detail and Compare plus
  click-zoom on the photograph itself, `?`, and a reduced-motion indicator. The old one-line library summary left
  the bar: the canvas header already says the count and the selection.
- **Every destination is a command.** `view.compare` and `view.softFocus` are new `Ready` rows with `ShowView`
  effects, and `view.places` was still `Planned("M5")` — the map shipped slices ago, so a working view was hidden
  from the palette and from MCP. `M` now opens the map (the keymap overlay lists it, and the CLI's shortcut golden
  grew by that one line), the palette is 24 rows, and Compare is reached through `toggle_compare` in the window
  because only the shell knows which second photograph to show. Examples added for the three new MCP tools.
- Files: `crates/ui/src/shell.rs` (View, `activate_view`/`toggle_compare`/`open_detail`, `soft_focus_view`,
  `bottom_bar` + `view_tab`/`view_menu`/`annotation_pill`/`rate_selection`/`flag_selection`), `crates/ui/src/theme.rs`
  (`Density::index`/`from_index`), `crates/ui/src/icons.rs` + `assets/icons/{compare,focus,chevron}.svg` (39 glyphs),
  `crates/ui/src/keymap.rs`, `crates/engine/src/command.rs` (3 rows), `crates/engine/src/lib.rs` (3 effect arms,
  `view.insights` now names itself)`, `crates/mcp/src/{examples,lib}.rs`, `apps/sortlens/src/main.rs`,
  `crates/ui/tests/screenshots.rs`, and the reconciled `docs/UX.md` §5.6/§5.8/§7 + `docs/ARCHITECTURE.md` §8.1.
- Tests: the pill driven by real clicks on its painted glyphs (rate both photos, the lit star clears, Pick and
  Reject both land, a keyword count joins the pill); `tabs_collapse`'s threshold plus a wide row that names
  Soft Focus and a narrow one that does not, whose menu still holds it; the peer-view invariant and Compare's
  spoken refusal with nothing selected; click-zoom on the largest textured rect of the detail canvas; an engine
  audit that every view destination is `Ready` and sets its `ShowView` effect; the size control's snapping. The
  registry-vs-message milestone test now reads its milestone from the registry instead of hard-coding `M2`.
- 696 tests; ci, deny, snapshot, screenshots (23 baselines), a11y, docs, layers, roots, attribution green.
- **Visual verification**: 20 baselines regenerated and read, plus three new ones: `soft-focus-*` (the peer view
  with stored scores, both themes) and `narrow-charcoal` (900 px with the docks closed) showing the collapsed
  row: the current destination, the chevron, and the pill and controls in their places. The bottom bar is in
  every baseline: nine tabs, the pill, the slider and `?` at 1440 px. The screenshot harness's accessibility
  pass found the sharpness slider unnamed and that is fixed (`a11y::label_widget` with the question it asks).
  Two older defects were seen while painting the 900 px scene and are recorded in `Next` under 3B.6 rather than
  hidden by the new baseline: at 620 px of height the sidebar's foot overlaps its own list, and at 900 px with
  both docks open the canvas header's title runs under the sort combo. Still not verified by hand: the slider's
  feel at the width where the row is about to collapse, and whether a keyword count crowds the pill at 900 px.

## 2026-11-23 (thirty-third) — Phase 3 item 12 (3B.3): the sidebar asks six questions, and the utilities move to its foot

- **`SOURCES` is six rows, one question each.** `Source::ALL` gained `Missing`, so the panel now lists
  All Photos, Recently Added, Picks, Rated, Missing and Rejects — the six scopes `docs/UX.md` §4.1 has asked
  for since the wireframe, of which five existed. Each row's count is computed by `source_count`, which builds
  the filter through `source_filter`: the same function the tests call, so a row's number and the grid that row
  opens are one question asked twice rather than two questions that look alike. `UiState` gained `missing_only`
  (the question, like `recently_added`), `apply_source` sets it, `source_selected` compares it, and
  `active_filter_count`/`has_filters`/`clear_filters` know about it.
- **Missing needed a real filter, and it is a narrowing.** `sortlens_catalog::Filter` gained
  `missing: Option<bool>`: `Some(true)` is only files that are gone, `Some(false)` only files that are there,
  `None` leaves the old behaviour. It outranks `include_missing`, which only widens the view — "everything,
  missing included" and "only what is missing" cannot both be satisfied, and the narrower question is the one
  asked deliberately. `missing` is part of the filter's JSON surface too, so `library.page` accepts
  `{"filter": {"missing": true}}` and the MCP example now says so.
- **The four dialog rows left the middle of the sidebar.** `SIDEBAR_FOOT` is
  `[Deleted, Health, History, Shortcuts]` as `PaletteCommand`s, and `sidebar_foot` paints them in an
  `egui::Panel::bottom` laid out *before* the sections so it claims the bottom edge and nothing can push it
  off. Every row's label and right-hand column come from the command registry (`Recently Deleted` + the bin
  count, `Check Library`, `Recent Changes`, `Keyboard Shortcuts` + `?`), and a click goes through the same
  `PaletteCommand::run` the `⌘K` palette uses — so the row that opens the bin and the palette row that opens
  the bin are one handler, and a fifth row can only be added by naming a command that already exists. The
  words therefore changed with the registry: "Library health" is now "Check Library", "Changes" is "Recent
  Changes" and "Shortcuts" is "Keyboard Shortcuts", which is the point rather than a regression.
- **The strip is sized exactly, and that was a real defect.** The first version let the panel measure its own
  content, which a panel only learns *after* laying it out: on the first frame the four rows hung below the
  panel's own clip rect and the strip flickered into place a frame late. `sidebar_foot_height` now states the
  height from `NAV_ROW_H` (hoisted out of `nav_row`), the item spacing, a 9-point hairline band and the frame's
  margins, and the panel is given it with `exact_size`. The hairline is painted rather than `ui.separator()`:
  that widget's height is a style lookup, and a point nobody counted is a point that clips a row.
- **Not verified by hand**: how the strip behaves at the narrow end of the panel's 210–380 px range with
  "Keyboard Shortcuts" as its longest row (it truncates from the measured width, so it should), and the foot
  over a keyword list long enough to reach it — the sections do not scroll yet, and clipping is unchanged in
  kind, only raised by the strip's height.
- **Named, not hidden**: `docs/UX.md` §4.1 used to imply a per-row "filtered" marker and a `Rejects (bin)`
  predicate that does not exist (the bin is a dialog, and the Rejects row is the *flag*); §4.1 now says what
  ships and names the marker as not shipped, and §4.4 describes the foot and names the storage summary,
  branding block and collapsible/reorderable sections as not shipped. No new verbs: every foot row was
  already a `Ready` registry command.
- **Tests that fail when the behaviour is absent (3 new, 690 workspace total).**
  `only_missing_narrows_while_include_missing_widens` (catalog) pins both directions and the precedence over
  the widening flag; `the_missing_scope_counts_the_files_that_are_gone_and_only_those` removes a real file,
  runs `check_health`, and asserts the Missing row's count, the grid it opens, the badge and that `Clear` leaves
  the scope — removing `missing: source.missing()` from `source_filter` fails it; and
  `the_sidebar_foot_pins_the_utilities_to_the_bottom` paints the whole window at 1280×800 and requires the
  foot's four glyphs to be the last four in the column, nothing else in the band, the sections to end at least
  60 points above it, and *every* sidebar glyph to be inside the clip rect it was painted into — which is what
  the auto-sized version failed. The chrome icon-count test's sidebar floor is now six sources + utilities.
- **Visual verification: all 20 existing baselines regenerated with `--update`, then read, then re-run without
  it (9/9 tests matched).** Two are new: `missing-charcoal` and `missing-paper` — one frame's file removed and
  the health check run, `apply_source(Missing)`, so the sidebar's Missing row counts 1, the filter bar's chip is
  lit, and the grid holds the record of a file nobody can open. The library baselines were read in both presets
  to confirm the six rows, the hairline and the four foot rows with the registry's words.

## 2026-11-23 (thirty-second) — Phase 3 item 11 (3B.2): one field with two modes, and the bar loses its words

- **`⌘K` is the same field, not a second search box.** `UiState` grew `palette_open`, `palette_query`,
  `palette_index`, `palette_focus`, `focus_search` and `palette_anchor`. The top bar draws the photo field or
  the command field in the same 240-point slot depending on the mode, so the palette's rows hang under the
  control the person is typing in. `Esc` leaves command mode and asks the photo field for focus, and the photo
  query is untouched throughout (the test types "beach", opens ⌘K, runs a command, escapes, and finds "beach"
  still there).
- **The palette cannot list a command it cannot run.** `PaletteCommand` is an enum of 21 `Ready` registry
  ids; `run` matches every variant, and the rows paint the registry's `label`, `Group::label` and `shortcut`
  rather than strings of their own. `palette_rows` resolves them through `Registry::v0()` and filters on
  label, id and group name. `↑/↓` wrap, `Enter` runs the highlighted row, a row that cannot run (no selection,
  nothing staged, no folder to rescan) closes the palette and puts a sentence in the notice line rather than
  pretending. The keys are consumed at the top of `show`, before the bar lays out: a `TextEdit` takes Enter
  and the caret keys for itself, and the first ⌘K wiring failed exactly that way until the consumption moved.
- **The bar is glyphs with names.** App mark (the `sortlens-icon` SVG registered once per context — the same
  geometry as the window icon), then Settings, Help (`app.shortcuts`), Appearance, Activity, Export, Import,
  Filter and the right-panel toggle; the search field stays the one piece of chrome that is made for typing.
  Export is `add_enabled` on the selection and says "Select a photo to export" when it cannot run. Filter is a
  toggle whose badge is `UiState::active_filter_count`, painted onto the button's own rect by a new
  `paint_badge` the bell now shares, so a count never reflows the bar.
- **Appearance is one control for two settings.** Left click cycles the theme; right click lists all four
  presets and a Reduced-motion checkbox, so the two value chips that were here are gone rather than moved.
  Density and layout are two glyphs beside the sort in the canvas header — the same question ("what does the
  grid show") answered in one place.
- **The filter strip can be put away.** `filters_open` (default true) gates `filter_strip`, and a hidden strip
  leaves no gap. Hidden-on-empty-library is 3B.6's job and is not claimed here.
- **Two glyphs joined the set**, both project-owned stroke SVGs in the established conventions: `funnel`
  (Filter) and `export` (a tray with a rising arrow), 36 total. `Icon::ALL`, `CATALOG` and the folder test stay
  in step; `docs/ATTRIBUTION.md`'s `assets/icons/*.svg` row already covers them. `Ctrl+K` joined the keymap
  overlay (`crates/ui/src/keymap.rs`) so the shortcut is discoverable there as well as in the bar.
- **Tests that fail when the behaviour is absent (5 new, 687 workspace total).** `every_palette_command_is_a_ready_registry_command`
  walks the enum against `Registry::v0()` in both directions (unknown, planned and duplicated ids all fail)
  and pins the query rules; `the_palette_commands_run_or_explain_why_not` drives `run` for the refusal and
  success paths and checks the `Action` a command asks the host for; `command_k_opens_the_palette_and_enter_runs_the_highlighted_command`
  drives real `RawInput` events through `show` (⌘K, text, Enter, ⌘K, Esc) and asserts the state each step;
  `the_filter_chip_toggles_the_strip_and_wears_the_active_count` clicks the painted badge in isolation and
  then shows the whole window with the strip on and off; and `the_top_bar_and_palette_paint_only_glyphs_the_bundled_faces_have`
  audits every string the bar and the palette paint against the bundled faces — it passes for `⌘K`, the `·`
  separators and every registry label, and it caught the missing `icons::install` in its own first run as
  32 egui error placeholders. The chrome icon-count test now asserts 8 pictures in the top-bar band (was 3).
- **Visual verification: all 20 baselines regenerated with `--update`, then read, then re-run without it
  (8/8 tests matched).** Two are new: `palette-charcoal` and `palette-paper`, showing the field in command
  mode with the rows grouped by registry group and the declared shortcuts right-aligned. The library
  baselines were read in both presets to confirm the mark, the eight glyphs, the `⌘K` hint, the Filter badge
  and the density/layout pair in the canvas header. `apps/sortlens` was not run by hand; the screenshots come
  from the same `show` call.
- **Not verified by hand**: the palette on a narrow window (it is `max(field width, 430)` wide and hangs left
  of a 240-point field; below roughly 700 px it would reach the sidebar), and the app mark at 16 px on a
  non-Retina display, where the tile's internal prints are a pixel or two each.
- **Named, not hidden**: `docs/UX.md` §3 previously gave this bar a `view.appearance`, `view.filterBar`,
  `library.filter` and `app.libraryMenu` command; none of those ids exist in the registry, and the table now
  says which controls are shell state and marks the back/forward and library-menu rows as not shipped. The
  registry gained no new verbs this slice — every palette row already existed and was `Ready`.

## 2026-11-23 (thirty-first) — Phase 3 item 10 (3B.1): the palette is one mapping, and the chrome has pictures

- **`Tokens::palette()` is the mapping and `apply` is its only consumer.** `craft_ui::theme::WidgetPalette`
  (`noninteractive: panel`, `inactive: card`, `hovered: hover`, `active: active`, `open: card`) is now what
  `apply` installs through `WidgetPalette::apply_to`, so a suite widget reading `Visuals` and a Sortlens widget
  reading `Tokens` agree about a hover by construction instead of by two lists kept in step. Two documented
  exceptions survive: the pressed state wears the accent ring, and the borders stay `t.border` on every state.
  One visible consequence is *intended*: `weak_bg_fill` now equals the strong fill per state, so a resting
  button is a card rather than something between card and hover. `ThemeKind::egui_theme()` names which half of
  `Style` a preset paints under, instead of `apply` and its test spelling it twice.
- **The icon set exists as files, not as a font.** 34 project-owned 24-unit stroke SVGs under `assets/icons/`,
  embedded with `include_bytes!` and named by `Icon::ALL`. `crate::icons::install` puts `egui_extras`' SVG
  loader on the context once and registers every glyph's bytes up front, so the first frame a glyph appears on
  is the frame it is painted — not one frame later, which is what an unregistered `bytes://` URI looks like.
  `craft_ui::icons::SvgIconSet` prepares each drawing (white `currentColor`, 1.75 stroke) so the tint is the
  theme colour and not a second palette; the loader rasterises at exactly the slot size, which is what keeps a
  14-point glyph sharp at 2× rather than a blurred bitmap.
- **New workspace pin: `egui_extras`** (the same egui fork rev, `default-features = false`, `features = ["svg"]`
  — no HTTP, no file loader, no mime sniffing). `deny` passes its licence tree unchanged (`resvg`/`usvg` are
  MPL-2.0, already allowed for `option-ext`), and `attribution` gained its row plus a provenance row for `assets/icons/*.svg`.
- **Wired into all five surfaces, additively where the word carries a value.** Sidebar: every source row, the
  four utility rows, folder rows, keyword rows and collection rows now start with a glyph, in a fixed 16-point
  column so the text edges stay aligned; the `+` add-folder button became `folder-plus`. Filter strip: rating,
  flags, colour, kind, source, edited, missing and clear chips each carry their glyph, painted in the chip's own
  colour so an active chip does not have a light icon on a light pill. Bottom bar: every destination is a glyph
  beside its label. Right panel: the three tabs. Top bar: Settings became `sliders` (a gear at 13 px reads as a
  sun), the right-panel toggle became `panel` and now *selects* when the panel is open, the bell became a bell
  with an unread count painted into its own rect, and Add Folder is `folder-plus` plus its words. The value
  labels — theme, density, layout, motion — deliberately stayed words: they print a current value, and a glyph
  cannot say "Comfortable". Replacing those is 3B.2's Appearance menu, not this slice.
- **An icon-only button keeps its words.** `icons::button` labels the response through
  `a11y::label_widget` with the same string it uses as the tooltip: a screen reader must not be handed a file
  stem, and two different answers to "what is this button" is worse than one.
- **Tests that fail when the behaviour is absent** (6 new, 682 total): the enum ↔ catalog ↔ folder name check in
  both directions (so a glyph added to one list only is caught); a conventions test (`currentColor` present,
  `stroke-width="2"` present, 24-unit viewBox, no `rgb(...)`, so every glyph is tintable); a rasterisation test
  that asks the real loader — through the same `install` the shell calls — for all 34 glyphs at 14 and 32 px and
  requires both ink and empty pixels (verified by removing the `install` call from `show`, which failed it); an
  accessible-name test on an icon-only button through AccessKit; the palette test that compares what `apply`
  actually stored in `Visuals` against `Tokens::palette()` for all four presets; and a shell test that counts the
  textured rectangles painted inside the top bar, bottom bar and sidebar bands (3, 6 and 11 — exactly the
  designed counts, asserted with `assert_eq!` rather than a floor, and it failed when `install` was removed).
- **Visual verification: all 18 baselines regenerated with `--update`, then read, then re-run without it (7/7
  matched).** The top-right of the library baseline was magnified to check the new Settings glyph (the first
  gear drew a sun, the second a mush of three sliders — the shipped one is two rows with visible knobs), the
  sidebar and filter strip were magnified in Charcoal and Paper to confirm the glyphs read at 14 px and that the
  light presets get dark icons (`text_muted` tint, not white), and the detail baseline was read whole in Paper.
- **Not verified by hand**: `apps/sortlens` in a real window (no change in this commit's rendering path, but the
  glyphs are only seen through the harness), and how the sidebar's fixed 16-point icon column behaves at the
  narrow end of its 210–380 px resize range with a long folder name — `truncate` still measures from the reduced
  width, but only the picture would settle it.
- **Still open in 3B**: the `⌘K` palette and Appearance menu, the sidebar's utility strip, the bottom bar's
  annotation pill and peer views, the right panel's Suggestions tab and tool strip, the Welcome bullets and
  filter chevrons, and the three visual defects 3A recorded. Item 10 closed only what it named.

## 2026-11-23 (thirtieth) — Phase 3 item 9 (3A): the screenshot harness, its baselines, and what they found

- **`cargo xtask screenshots` exists and paints the real shell.** `crates/ui/tests/screenshots.rs` runs
  `sortlens_ui::show` inside an `eframe::App` — the same entry `apps/sortlens` uses — through
  `egui_kittest` at the suite's pinned egui revision, at a fixed 1440×900 with `pixels_per_point = 1.0`.
  Eight surfaces × the Charcoal and Paper presets = **18 committed PNGs** under
  `crates/ui/tests/snapshots/`: Empty Welcome, Library Justified, Library Square, Detail, Compare, Cull,
  Similar, Places and Insights (Detail and Compare share one test, as do the two grid layouts). The
  diffing is `egui_kittest`'s own — per-pixel tolerance in the new root `kittest.toml` (`threshold = 0.6`,
  `max_failed_pixels = 0`) — so a mismatch writes `<name>.new.png` and `<name>.diff.png` beside the
  baseline, reports the failing-pixel sweep, and fails. `cargo xtask screenshots --update` sets
  `UPDATE_SNAPSHOTS=1` and is a person's decision; the images are the review.
- **The harness is opt-in and the CI gate stays headless.** It lives behind `sortlens-ui`'s `screenshots`
  feature (`dep:egui_kittest`, `dep:eframe`), so `cargo xtask ci` neither compiles nor needs a GPU; the
  file is `#![cfg(feature = "screenshots")]`. `cargo xtask screenshots` runs it with `--test-threads=1`
  because each test creates a wgpu device, and six devices at once is a way to fail for a reason that has
  nothing to do with the UI. `egui_kittest` is declared once in `[workspace.dependencies]` like every other
  dependency and attributed in `docs/ATTRIBUTION.md`; `deny` (all-features) passes its licence tree
  (`kittest`, `dify`, `open`).
- **Every settled surface also passes `egui_kittest`'s accessibility check, and it found a real defect on
  the first run**: the canvas header's sort `ComboBox` had no accessible name — read aloud it said "Added",
  the current value, and nothing about what the control changes. Fixed with the crate's own
  `a11y::label_widget(ui, &combo.response, Role::ComboBox, "Sort order", false)`, so it is labelled the
  same way the other custom controls are. `cargo xtask a11y` could not see this one: the widget is egui's,
  not a hand-painted clickable.
- **The second finding was tofu**: the detail footer painted `← Previous` / `Next →` and `← → walk`, and
  none of the four bundled faces has U+2190/U+2192, so a zero-config build (the documented default, with
  no `craft-fonts` checkout) drew boxes. The buttons now read "Previous"/"Next" and the hint reads
  "left/right walk"; no behaviour changed. The overlay's keycap labels in `crates/ui/src/keymap.rs` and the
  registry's `shortcut` strings in `crates/engine/src/command.rs` still say `←`/`→` — they are cross-checked
  against each other and against the CLI goldens, so they are the next slice's work, not a half-fix here.
- **Three more visual defects are recorded, not hidden** (ROADMAP 3B.6 now names them): Places opens at
  zoom 5, so a library spread across two continents centres between them and paints no dots at all — the
  baseline zooms to 3 to show the dots, and choosing an opening zoom that fits is 3B.6's job; the cull
  view's keyword row clips under the bottom bar at 900 px; and the `1 active` + `Clear` pair still doubles
  up in the filter strip. A baseline that showed a blank map as "the Places view" would have been a
  placeholder presented as working, so the scene says what it changed and why.
- **The fixture library is real and now carries real metadata.** `sortlens-testkit` gained
  `Fixture::at(path)` (a caller-named, cleared folder — `new` keeps its process id for parallel tests) and
  `Pattern` (`Solid` or a deterministically phased `Gradient`) with `Fixture::jpeg_with_pattern`, which
  writes the JPEG's pixel dimensions into an XMP packet (where the suite's EXIF writer does not record
  them) and GPS into EXIF. Solid colours were not enough: the similarity pass reads structure, so twelve
  solid frames were one identical group — and the justified grid had uniform cells because a fixture with
  no dimension tags indexes as 0 × 0. The screenshot library is now twelve gradients with mixed aspect
  ratios, five EXIF locations, a duplicate pair (IMG_0009/IMG_0010, byte-identical), fixed ratings, two
  picks, a reject, two keywords and a collection — scanned once, then moved between scenes so every
  baseline paints the same ids. The folder is a *relative* path (`tests/.screenshots`, gitignored) because
  the Places status line prints the library's `tiles/` folder: an absolute temporary path with a process id
  in it made the baseline unreproducible, which is exactly what the second run caught.
- **Tests that fail when the behaviour is absent**: the seven screenshot tests (a changed token fails with
  a named diff — verified by temporarily recolouring Charcoal's accent, which failed 891 pixels, then
  reverted); `sortlens-testkit`'s gradient-stability test (same phase, same bytes; different phase,
  different file) and its dimension/GPS round-trip through `craft_metadata::extract`; and the existing
  headless `Context::run_ui` detail-view test was **extended** to audit every string the view paints against
  the bundled faces' glyph coverage, failing with the offending text — verified by reintroducing
  `← Previous`, which it named, then reverting. 676 workspace tests (+2 in testkit; the 7 screenshot tests
  run only with the feature). ci, snapshot, deny and `cargo xtask screenshots` green; a second
  screenshot run without `--update` matched all 18 baselines.
- **Visual verification**: the 18 baselines were opened and read, not just generated. They show the shell
  at 1440×900 in both presets, the two grid layouts with real aspect ratios, the detail and compare
  canvases, a settled cull session, two similar groups, the Places dots with no basemap, and the Insights
  figures. `apps/sortlens` was not run by hand; the screenshots come from the same `show` call, and the
  harness's only difference from the window is the 0 title-bar inset (documented in the test: a harness has
  no traffic lights to clear).

## 2026-11-23 (twenty-eighth) — Phase 2 item 3b: pack keywords matched by meaning (SL-F03, the CLIP side)

- **The gate first, as promised: the real pins were downloaded, digest-verified and run once this
  run.** All three `models-v1` files land under their digest names and pass `Model::verify_file`; the
  real detector loads (800×512, 100 queries) and runs; the real CLIP ViT-B/32 image + text pair loads
  and answers 512-d embeddings. The shipped tokenizer was validated against the real text model: the
  cosine ordering of five prompts came out exactly right (dog↔cat 0.946 > dog↔harbour 0.934 >
  dog↔financial report 0.893 > dog↔circuit diagram 0.850, and the two unrelated prompts are each
  other's furthest pair). A real cat photo (scikit-image's public-domain chelsea.png) through the real
  image embedder ranks “a photo of a cat” **first** (0.281) among the prompts, with unrelated
  photographic prompts at 0.19–0.24, and the real detector answers `cat · 1.00` on the same photo. The
  live check was a temporary example, deleted before commit; the tree's own tests run synthetic models
  only.
- **That live gate caught a real bug the square test models had hidden**: `Detector::input()` answered
  (height, width) while `Image::from_rgba` takes (width, height) — every synthetic detector is square,
  so the tests passed while the real 512×800 detector would have been silently transposed at every
  call. `input()` now answers (width, height) and the docstring says the order matches `from_rgba` on
  purpose.
- **`crates/ai/src/tokenize.rs` is the CLIP BPE tokenizer.** The vocabulary is the upstream
  `bpe_simple_vocab_16e6.txt.gz`, shipped byte-for-byte (`crates/ai/src/bpe_simple_vocab_16e6.txt.gz`,
  1,356,917 bytes, SHA-256 recorded in ATTRIBUTION.md) and decompressed here by `flate2` — desktop
  only, like the inference runtime, and still no network anywhere (`no_network` still passes, source
  scan included). The tokeniser reproduces the published table exactly: byte tokens in vocabulary
  order (the printable runs first, not byte order — the order bug the known-ids test caught), `</w>`
  attached to the last byte, 48,894 merges, BOS/EOS 49406/49407, zero-padded to the model's token
  count. The ids the tests pin (`a photo of a dog` → 49406·320·1125·539·320·1929·49407…) were derived
  from the published vocabulary, not from this implementation's output.
- **`crates/ai/src/text_cache.rs` caches each pack's keyword embeddings.** A pack file's SHA-256 and
  the text model's digest name the cache file
  (`<library>/models/text-cache/<digest>/<pack hash>.json`): a changed pack or a different installed
  embedder is a different file by construction, nothing is ever invalidated. The file is written
  atomically (temp + rename), read back only when its recorded model digest and pack hash match and
  every keyword's embedding is finite and the same length, and completed keyword-by-keyword — a pack
  that gained a keyword embeds only the new one. `load_only` serves a pack whose embeddings are fully
  cached without any model: the pack side is a derived index, not a second model.
- **The engine proposes pack keywords by meaning.** `Engine::embedder_session` (the text embedder +
  tokenizer) and `Engine::image_embedder_session` (the vision side) load once per session with the
  detector's rules; installing, removing or swapping the manifest resets all three session caches —
  including the detector, which never actually reset on install/remove before (the twenty-sixth
  entry's claim is now true). `pack_keyword_embeddings` fills the cache once per pass; per photo, the
  224 px preview is embedded once and every cached keyword is a cosine away. A keyword is proposed
  when its cosine clears `EMBED_MIN_COSINE` (0.25, set from the live numbers above: the correct match
  scored 0.281, unrelated photographic prompts 0.19–0.24), the cosine is the score, and the proposals
  go through the same `suggest::merge` as the matcher's hits — deduped against existing tags, capped
  at eight, ordered by confidence. The pack matcher's own rules still run beside it; a keyword both
  sources propose is one suggestion at the better score.
- **Tests that fail when the behaviour is absent** (20 new): the tokenizer's known ids, exact
  sequence shape for any token count, truncation that keeps EOS, case/whitespace insensitivity, byte
  fallback for untrained words, the published byte-encoder table at its edges; the text cache's
  build-and-write, serve-without-embedding, per-pack and per-model keying, completion, tamper refusal,
  refusal-propagation and per-model clearing; a lookup text embedder in `sortlens-testkit::onnx`
  (Gather + ReduceSum over real token ids) answered and pinned in `sortlens-ai`; and two engine
  tests — a synthetic pair installed under a swapped manifest, a pack whose matcher never fires, both
  fixture photos proposing “dog” (never “cat”) with the cosine as the score and the cache file on
  disk, the single-photo path answering from the same cache — and, after removing the text model, the
  fully cached pack still matching while a never-cached pack stays silent.
- ROADMAP's Phase 2 item 3 (both halves) ticked in the same commit; ATTRIBUTION.md attributes the
  vocabulary file and `flate2`; RELEASE.md's model bullet now says packs are matched by meaning;
  ARCHITECTURE.md §9 says where the cache lives and how the gate was chosen. UX.md needed no change —
  the panel's sources are unchanged.
- 665 tests (20 new); ci, snapshot, deny green. No UI change, so no `run_ui` or screenshot work
  applies; the screenshot harness does not exist yet (Phase 3 item 9).

## 2026-11-23 (twenty-seventh) — Phase 2 item 3a: the tract-onnx inference runtime and the detector path (SL-F03)

- **`crates/ai/src/infer.rs` is the runtime; it stays offline and desktop-only.** tract-onnx 0.23.10 —
  the exact build every pin was verified against — runs in `sortlens-ai` itself; the crate's `no_network` test scans every source file including this one and
  still passes, because there is nothing to find. Loading is a contract, not an assumption:
  `Session::load` refuses a file whose input/output shapes do not match what `docs/ATTRIBUTION.md`
  records (wrong rank, wrong batch, wrong channel count, a symbolic shape where the pinned models are
  static), so a wrong file fails with words instead of a strange photo.
- **The detector answers in the shapes the rest of the crate already understands.** `Detector::load`
  disambiguates the YOLOS outputs by shape (1×Q×4 is boxes, 1×Q×C is logits), builds the COCO-91 label
  map (`crates/ai/src/coco91.txt`, `include_str!`-ed and attributed in ATTRIBUTION.md), and `run`
  softmaxes each query, drops the unnamed slots (`N/A` classes and the EOS slot past the map), converts
  normalised cxcywh to corner boxes and hands everything to the same `detect::suppress` every other
  detector would face. `Image::from_rgba` stretches a decoded preview to the model's input size
  (nearest-neighbour, on purpose) and applies the ImageNet/CLIP mean/std normalisation. The CLIP pair
  loads and answers through the same contract-checked runtime: `ImageEmbedder` (1×3×edge×edge → 1×512,
  with a cosine that answers 0 rather than a NaN for a zero vector) and `TextEmbedder` (1×77 int64 →
  1×512) are tested and waiting for the pack-embedding slice.
- **Tests run on synthetic ONNX models, assembled from the same protobuf types the loader parses.**
  `sortlens-testkit::onnx` builds tiny valid models with the pinned models' exact I/O signatures whose
  outputs are constants the test supplies — no 166 MB pin in any test, no network, and the plumbing
  (loader contract, softmax, EOS drop, box conversion, NMS, cosine) is what is pinned. 10 new tests in
  `sortlens-ai`, 4 in `sortlens-testkit`.
- **The engine runs the detector over 384 px previews in the suggestion pass.**
  `Engine::begin_batch_suggest` loads the pinned detector once per session (`Engine::detector_session`,
  cached, reset by install/remove and by the test manifest swap), moves it to the existing suggestion
  worker, and decodes a `PREVIEW_EDGE` (384 px) preview per photo through `sortlens-thumbs`; what the
  model sees becomes `Candidate`s with `Source::Detector` and the box attached, merged with the EXIF
  hints and the packs by `suggest::merge`. A photo whose file is missing or undecodable simply has
  nothing to say — the pass continues. The single-photo panel (`Engine::suggestions`) and the
  selection panel (`suggestions_for_all`) answer from the same session cache. Without the model
  installed, the pass proposes exactly what it proposed before — the detector is an addition, not a
  dependency (a test pins that). Nothing is written by the pass; accepting a proposal remains the only
  path to a keyword.
- **Engine tests that fail when the behaviour is absent** (2 new, all offline): a synthetic detector
  installed in the store, the pass polled to its end, both fixture photos carrying "person" + "bicycle"
  with the box converted to corners and `Source::Detector`, the single-photo path answering from the
  same cache — and the no-model pass unchanged.
- **One standalone fix landed first** (2c0ce95): the reveal test asserted `a.jpg` in the command on
  every desktop, but Linux's `photo.reveal` deliberately opens the containing folder (`xdg-open` cannot
  select a file), which ubuntu-latest CI caught on the cross-platform pass of 769c461. Each platform
  now asserts its own documented contract.
- ROADMAP's Phase 2 item 3 was split into 3a (runtime + detector path, ticked today) and 3b (per-pack
  text embeddings cached at install, image embeddings, both feeding `suggest::merge`); RELEASE.md's
  model bullet now says the detector runs and what is still open; ARCHITECTURE.md §9 says where the
  inference lives; ATTRIBUTION.md says the label map ships now.
- 645 tests (16 new: 10 in `sortlens-ai`, 4 in `sortlens-testkit`, 2 in the engine); ci, snapshot, deny green. No UI change, so no
  `run_ui` or screenshot work applies; the screenshot harness does not exist yet (Phase 3 item 9).
- **Not verified against the real weights**: loading and running the pinned 13/89/64 MB files was done
  once when the pins were recorded, before this runtime existed; the suite covers the plumbing over
  synthetic models with identical I/O contracts, and the contract check refuses anything shaped
  differently. A live run against the downloaded pins is the next slice's first gate.

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
CLI smoke test round-trips through disk.

Known gaps (deliberate, next milestones): no thumbnails yet (cells show kind + name), metadata
not read (width/camera/captured stay empty), filters in the bar are still disabled, no cull view.

## 2026-11-21 — M2: previews and metadata

Shipped:
- `sortlens-meta`: EXIF through `craft-metadata` into one optional-field struct (dimensions,
  capture time with UTC-offset handling, camera, lens, ISO, aperture, shutter, focal), plus
  civil-date conversion both ways. Read is bounded to 8 MiB per file. 6 tests.
- `sortlens-thumbs`: JPEG decode (8-bit, RGB/grey), box-filter downscale that never upscales,
  content-addressed disk cache with atomic writes and torn-file tolerance, and a two-worker
  `Service` with in-flight de-duplication and remembered failures. 7 tests, including real
  JPEG encode→decode round trips.
- `sortlens-ui`: texture cache (cover-fit, 900-texture cap), thumbnail requests only for visible
  rows, richer details panel (dimensions, camera, lens, exposure, capture date).
- scan/catalog: metadata travels with the file and is stored on the record; rescan refreshes it.
- CLI `list` shows dimensions.

Verified: `cargo xtask ci` green (55 tests); CLI round trip on a real 320×200 JPEG reads
`320x200` from EXIF; thumbnails decode from that file in tests.

Known gaps: RAW/HEIC/PNG/TIFF thumbnails, video posters, priority scheduling, activity stack,
non-JPEG previews fall back to the kind glyph.

## 2026-11-21 — M2: progress and cancellation

- scan: `ScanControl` with shared atomic counters and a cancel flag; `ScanReport.cancelled`
- engine: `scan_progress()`, `cancel_scan()`; a cancelled walk is never synced, because a partial
  walk would mark every unseen file as missing
- ui: live "Indexing · N files · M folders" in the bottom bar and sidebar, progress rows plus a
  Cancel button in Activity, tidy cancelled notice
- 57 tests, CI green

## 2026-11-21 — M2: PNG, TIFF, WebP, BMP, GIF thumbnails

- thumbs: `decode_image` dispatches through `craft-image-codecs` (PNG, TIFF, WebP, GIF, BMP, QOI,
  PPM and 8-bit JPEG), falls back to the JPEG path, and carries alpha through the resize
- 3 new tests: PNG round trip, TIFF round trip with downscale, alpha preserved
- 60 tests; `cargo xtask ci` green

Still missing: RAW embedded previews (`craft-raw`), video posters, HEIC.

## 2026-11-21 — M2: raw previews + shared media vocabulary

- core: `media` module owns the extension tables (raw/image/video/sidecar), `kind_from_extension`,
  `is_raw_path`; the scanner now delegates instead of keeping a second copy
- thumbs: `decode_raw_preview` pulls the largest embedded preview with `craft-raw` and decodes it
  through the ordinary pipeline; `decode_file` prefers it for raw extensions and falls back to the
  plain decoder for mislabelled files
- tests: raw-without-preview, mislabelled .cr2 that is really a JPEG, extension tables
- 65 tests; CI green

Not verified against a real camera raw: no sample file in the repo. The dispatch and fallback are
covered; the preview extraction itself is `craft-raw`'s code path.

## 2026-11-21 — M2: activity stack

- engine/job.rs: `Job` with kind, label, status (running/done/cancelled/failed), counters, error,
  start/finish times and a one-line detail; kept to the last 50
- engine: every scan creates a job, finishes as Done / Cancelled / Failed, and `jobs()` fills the
  running scan's counters from the scan control; `library.jobs` JSON command for scripts
- ui: Activity tab lists jobs newest first with status colour, elapsed time, detail, per-job error,
  and Cancel for the running one; library totals and warnings sit below
- tests: job lifecycle, missing folder fails the job, jobs JSON, plus unit tests for the row states
- 72 tests, CI green

## 2026-11-21 — M3: keywords

- catalog: `Photo.tags` (sorted, deduplicated), `Op::Tagged` replacing a photo's whole tag list so
  undo is a plain inverse, `Filter.tag`, `tag_vocabulary()` by count, `Stats.tagged`
- engine: `set_tag(ids, keyword, add)` normalises (trim, strip `#`, lowercase, ≤64 chars), skips
  no-op changes, returns how many photos changed; `photo.keyword` command is Ready;
  `tag_vocabulary()` for the sidebar
- ui: Keywords sidebar section with counts and click-to-filter, keyword chip in the filter strip,
  Tags tab with removable chips, an add field, and "used elsewhere" suggestions; `K` opens and
  focuses it
- cli: `tag`, `untag`, `keywords`, `list --tag`
- tests: tag sorting/dedup/undo, rescan keeps tags, filter and vocabulary agree, normalisation,
  command round trip, undo of a batch tag
- 77 tests, CI green; CLI smoke test across separate processes confirms the tags persist

XMP sidecar writing is still pending: keywords live in the op log for now, so they survive restarts
but are invisible to other apps.

## 2026-11-21 — M3: XMP sidecars

- `sortlens-sidecar`: `Sidecar { rating, flag, label, keywords }`, `path_for` (same stem, `.xmp`),
  `read`, `merge`, `write`. Merging edits the first `rdf:Description` in place: attributes are
  replaced element-value-wise, the `dc:subject` bag is swapped, everything else (crop settings,
  creator tool, unknown namespaces) is left byte-for-byte alone. Namespaces are declared on demand.
  Writes are temp-file + rename and skipped when nothing would change.
- engine: `sync_sidecars` runs after rate/flag/label/keyword changes, reports a `Sidecars` job with
  the number written and per-file failures, and never aborts the batch. `write_sidecars` toggles it.
- ui: Settings → Library checkbox, plus a notice; the toggle reaches the app as `Action::SetSidecars`.
- tests: fresh round trip, foreign file keeps `crs:Exposure2012`/`xmp:CreatorTool`, self-closing
  description gains children, clearing a label/flag removes it, element-form rating, entity
  escaping, atomic + idempotent write, missing sidecar, path mapping, engine writes a sidecar,
  switch-off, read-only folder reports a failure but keeps the rating
- 90 tests, CI green; CLI smoke test produced an `xmllint`-parseable sidecar

Not implemented yet: sidecars are not read back into the catalog at scan time, and failed writes
are retried on the next change rather than by a queue.

## 2026-11-21 — M3: sidecars are read at scan time

- core: `Review { rating, flag, label, tags }` is the shared review record, with `is_empty()`
- scan: each accepted file reads its sidecar (`sortlens_sidecar::read`) into `ScannedFile.review`;
  an unreadable sidecar is ignored rather than failing the scan, and an empty review is dropped
- catalog: `Seed.review` is applied by `Photo::adopt_review` when the photo is first indexed
- engine: passes the review through, so a folder annotated elsewhere arrives already reviewed
- tests: sidecar beside a photo becomes its review, photo without one stays blank, a new photo
  takes the sidecar's state, an existing photo keeps the catalog's state
- 94 tests, CI green; CLI smoke test imported a rating-4 / pick / keyword sidecar

Only *new* photos adopt sidecar state: a photo the catalog already knows keeps its review until
external-change detection lands, so a stale sidecar cannot quietly undo a cull.

## 2026-11-21 — M4: the cull state machine

- `sortlens-cull`: `Decision { Unjudged, Pick, Reject, Deferred }` and `Session` — queue, cursor,
  staged decisions, original flags. `stage` reports `Changed`/`Unchanged`/`Deferred`; a decision that
  matches the photo's original flag is not staged at all, and `Deferred` never touches a flag.
  `changed()` lists exactly what Apply would act on; `summary()` feeds the action bar.
- engine: `begin_cull`, `cull_decide`, `cull_stage`, `cull_back`, `cull_clear`, `cull_summary`, and
  the `cull.start`/`cull.decide`/`cull.back`/`cull.clear`/`cull.summary` commands (all Ready).
  `cull.apply` and `cull.undo` stay Planned("M4") because nothing moves yet.
- tests: the transition table as a partition, advance/back, re-applying the same value writes
  nothing, deferring preserves flags, clearing a flag is a change, rejects staged in queue order,
  completion, jump/next-unjudged, plus engine staging that leaves the catalog untouched
- 110 tests, CI green

Nothing is written until Apply, so a discarded session leaves the library exactly as it was.

## 2026-11-21 — M4: the cull view

- ui: `View::Cull` with a start screen (photo count from the current filters, "Start culling N
  photos"), a canvas that letterboxes the current photo at its own aspect, the decision badge,
  an "up next" strip of the following four photos (click to jump), a progress bar, live
  pick/reject/defer counts, and the P/X/U/Space/←/C keys
- `C` starts a session from the filters even with nothing selected; a session is a fixed queue, so
  changing filters afterwards cannot silently change what is being culled
- in a session P/X/U stage decisions instead of writing flags; outside one they keep their old
  meaning. Apply stays disabled and names its milestone: nothing moves yet
- `fit_rect` and its fallback are unit tested
- 112 tests, CI green

Not built: the summary sheet with an applied-batch receipt, gesture culling, zoom/pan and session
persistence. The cull view is real; Apply is still explicitly disabled.

## 2026-11-21 — M4: cull sessions survive a crash

- `sortlens-cull`: `Snapshot { version, queue, cursor, decisions, original }` with `snapshot()` and
  `restore()`; a future or empty snapshot is refused rather than half-read, a cursor past the end is
  clamped, and `retain()` drops photos that no longer exist while staying on the same photo
- engine: the session is written to `<library>/cull-session.json` after every change (temp file +
  rename), removed when the session ends, restored on `Engine::open` with photos the catalog no
  longer has pruned out. A write failure is a warning, never an error: culling continues
- ui: `C` opens a restored session instead of replacing it with a new one
- tests: snapshot round trip, version refusal, cursor clamping, retention, plus engine tests that
  reopen the library mid-session (cursor and reject survive), that ending a session deletes the
  file, and that a session whose photos are gone is discarded
- 118 tests, CI green

## 2026-11-21 — M4: staged deletes and Apply

- `sortlens-cull::trash`: `Mode { Trash, Move(folder), TagOnly }`, `Candidate`, `Batch`, `audit`
  (inside its root, still on disk, size and mtime unchanged, no symlink escape, destination
  writable and not inside a watched root), `apply` (re-checks each file immediately before touching
  it, per-item results), `undo` for moved files, and `apply_checked` which refuses the whole batch
  when the audit fails — partial application is not an outcome
- catalog: `Op::Group` applies in order and undoes in reverse, so an applied batch is one undo step
- engine: `cull_apply(mode)` audits, disposes, then commits flags and missing marks as one group,
  syncs sidecars for the touched photos, records an `Applied N photo(s)` job, clears the session,
  and restores the session untouched when the audit refuses. `cull.apply` is Ready with
  `{ mode: trash|tag|move, target }`
- ui: "Apply rejects…" opens a confirmation with pick/reject counts and a delete-mode choice
  (Trash or Tag only); Settings carries the same choice
- tests: trash audit refusals (missing, changed, outside root, destination inside a root), move
  preserves the relative path and undo restores it, tag-only moves nothing, undo of a trash batch
  says to restore it by hand, grouped undo in the catalog, engine apply for tag-only and move,
  and a changed file refusing the whole batch with the session kept
- 130 tests, CI green (`trash` is a desktop-only dependency)

Not wired: undoing a batch restores catalog state only — moved files are put back by
`sortlens_cull::trash::undo`, which the UI does not call yet, and trashed files stay in the system
Trash. `Mode::Trash` has no automated test because it would write to the real Trash.

## 2026-11-21 — M4: the rejects bin

- `sortlens-cull::trash`: `DeletedBatch { id, applied_at, mode, items, previous_flags, failed }`
  with serde and a `summary()`; `Applied`/`Location` are serialisable so a batch can be replayed
- engine: applied batches are appended to `<library>/deleted.json` (atomic, reloaded on open, ids
  monotonic), `deleted_batches()` lists them, `restore_batch(id)` moves files back through
  `trash::undo`, clears the missing marks and restores the flags in one `Op::Group`, then drops the
  batch. A file that cannot come back refuses the restore instead of leaving the catalog lying.
  Commands `library.deleted` and `library.restoreBatch`
- ui: "Recently Deleted" sidebar row with a count, and a dialog listing batches newest first with
  summary, age, mode and a Restore button; it names the Trash limitation in place
- tests: batch JSON round trip, apply → bin → reopen → restore restoring file, missing state and
  flag, empty-bin error for an unknown id
- 132 tests, CI green

Not built: emptying the bin (permanent delete from Sortlens) and restoring trashed files, which
stays a Finder/Explorer action.

## 2026-11-21 — M5: library health

- catalog: `Op::Relocate` (new path, clears missing; undo restores the old path *and* the missing
  state as a small ordered group), `missing_photos()`, and `remove_missing()` which purges as one
  group so a single undo brings the whole purge back
- engine: `check_health()` stats every file and commits one group of missing flags (report + badges
  agree, one undo step); `relocate_missing()` rescans each root and matches missing photos by content
  key under the same root; `remove_missing()`; commands `library.health`, `library.relocate`,
  `library.removeMissing`
- ui: "Library health" sidebar row opening a dialog with the last result and the three actions, each
  labelled with what it does
- cli: `check` and `relocate`
- tests: relocate clearing missing + undo restoring both path and state, purge as one undoable group,
  engine health check detecting a moved file, relocate finding it by key, and remove-missing undo
- 136 tests, CI green; CLI smoke test moved a file, saw `1 of 2 missing`, relocated it and
  read back the new relative path

## 2026-11-21 — M5: collections

- catalog: `Collection { id, name, photos, created }` with a set of members; ops `AddCollection`,
  `RemoveCollection`, `RenameCollection` and `Collect` (inverse swaps added/removed, so undo is
  exact); collections and the id counter live in the snapshot with `serde(default)` so older
  snapshots still open; `Filter.collection` resolved by the catalog, so membership composes with
  every other chip
- engine: `collections`, `create_collection`, `rename_collection`, `delete_collection`,
  `add_to_collection`, `remove_from_collection`; commands `collection.new`/`addTo`/`removeFrom`/
  `rename`/`delete` are Ready
- ui: Collections sidebar section with counts and click-to-filter, right-click rename/delete,
  "+ New collection", a rename dialog, a filter chip, and a per-photo "Add to Collection" context
  menu in the grid
- tests: create/fill/filter/rename/delete with undo, membership surviving a snapshot and restart,
  a collection needing a name, and the whole flow through the engine commands
- 141 tests, CI green

Not built: collection favourites, colours, per-collection search/sort, and reordering.

## 2026-11-21 — M5: subfolder drill-down

- catalog: `folders(root, prefix)` lists the immediate subfolders of a scope with recursive photo
  counts, skipping missing photos
- engine: `subfolders(root, prefix)` and the `library.subfolders` command
- ui: the Folders section shows the chosen root's subfolders with counts, clicking one descends a
  level and re-selecting it goes back up; a breadcrumb row (`Root › day1 › sub`) sits above the
  filter strip with clickable segments and "Up one level"; choosing another root clears the scope
- cli: `folders [--root R] [--in PREFIX]`
- tests: one-level listing with recursive counts and folder filtering in the catalog, the engine
  command, and `parent_prefix`
- 144 tests, CI green; CLI smoke test listed `day1/` (2), `day2/` (1) and `day1/sub/` (1)

## 2026-11-21 — M3: sort keys

- catalog: `Sort::ALL`, `id()`, `label()` and `parse()` (unknown ids fall back to added order)
- ui: the header's static "Added order" label became a combo box listing added order, name, newest,
  oldest, largest and highest rated; the grid and the cull queue both use the chosen order, so a
  session starts in the order the user was looking at
- app: the order is saved to `library.sort` and restored on launch (`Action::SetSort`)
- tests: ids round trip and are unique, unknown ids fall back, settings default is "added"
- 146 tests, CI green

Per-view orders (a separate order for collections or folders) are not built: one order applies to
the whole library view.

## 2026-11-21 — M3: kind and edited filters

- sidecar: `has_edits(text)` recognises develop settings by their namespaces (`crs:`, `xmpMM:History`,
  `xmp:History`, `darktable:`, `photoshop:History`), so a sidecar with only stars or keywords is not
  an edit, and `read_text` gives callers the whole document
- scan: reads the sidecar once per file and fills both the review and a new `edited` flag
- catalog: `Photo.edited`, `Seed.edited`, `Filter.kind` and `Filter.edited`
- ui: the filter strip cycles "All kinds → Photos → RAW → Videos" and "Edited → Edited only →
  Unedited", and both clear with the rest
- tests: edit markers recognised and our own sidecar not, scan marking an edited photo from a `crs:`
  sidecar and not from a rating, catalog kind/edited counts
- 149 tests, CI green; CLI smoke test indexed a photo with a `crs:` sidecar

"Edited" means the sidecar carries someone's develop settings — detection happens at index time, so
a photo edited after it was scanned shows as edited only after a rescan.

## 2026-11-21 — M4: the detail view

- engine: `neighbour(filter, sort, id, delta)` walks the same order the grid shows and stops at the
  ends instead of wrapping; `position` gives the one-based counter
- ui: `state.detail_open` swaps the grid for a single-photo canvas — name, counter, Fit/1:1 toggle,
  a 2000px preview with the area as a clip rect for 1:1, a facts line (camera, exposure, size), the
  keyword chips, and Previous/Next buttons. Double-click or `D` opens it; `Esc`/`D` closes it;
  `←`/`→`/Space walk the view, `Z` toggles zoom, and P/X/U plus 1–5 keep working inside it
- tests: navigation walking the view order and clamping at both ends, `position` counting
- 150 tests, CI green
- `view.detail` is now a Ready command

Not built: the lookahead strip and the histogram in the detail view.

## 2026-11-21 — M4: histogram and lookahead

- thumbs: `Histogram` (64 luminance bins, Rec. 601 weights) with `from_thumbnail`, `total`, `peak`,
  and `histogram_file` which decodes a small copy and measures it
- ui: `Thumbnails::histogram(photo)` caches one histogram per content key (decoded at 256 px on
  demand, never by the grid); the detail view draws it as 64 bars scaled to the tallest bin and adds
  a lookahead strip of ±1/±2 neighbours that can be clicked
- tests: every pixel counted with black and white in the end bins, a mid grey landing in the middle
- 152 tests, CI green

Housekeeping: `[build] incremental = false` in `.cargo/config.toml` after the disk filled up —
incremental artefacts were 3.5 GB per build on a machine with almost no free space.

## 2026-11-21 — M3: multi-keyword filters and an active badge

- catalog: `Filter.tags: Vec<String>` with `TagMode { Any, All }` (default Any) replaces the single
  `tag`; `TagMode::flipped()` and `label()` for the toggle
- ui: sidebar keywords toggle membership instead of replacing the filter, the strip shows one chip
  per active keyword plus a "match any/all" toggle when more than one is active, and the header shows
  an "N active" badge that clears every filter when clicked (`active_filter_count` counts dimensions,
  not keywords)
- cli: `list --tag` maps onto the new vector
- tests: ANY versus ALL matching with a photo carrying one and both keywords, unknown keyword
  matching nothing, `flipped`, and the badge counting two keywords as one dimension while clearing
  resets it
- 154 tests, CI green; CLI smoke test listed both photos for `beach` and one for `sunset`

## 2026-11-21 — M3: keyword picker and ordering

- catalog: `TagOrder { Count, Name }` with `flipped()`/`label()`; `tag_vocabulary_in(order)` and the
  existing `tag_vocabulary()` (count order) delegating to it
- engine: `tag_vocabulary_in(order)`
- ui: a "+ keyword" menu in the filter strip with a search box and a ✓ on active keywords, and a
  small toggle in the Keywords header switching the list between most used and A–Z
- tests: ordering by use and by name agreeing on counts, `flipped`
- 155 tests, CI green

## 2026-11-21 — M6: insights view

- insights: `Insights::compute` over the catalog's photos in one pass — totals (`Stats`), 6 rating
  buckets, 5 label buckets, 3 kind buckets, cameras/lenses/keywords folded to the top 8 and capture
  years with a real leap-year calendar walk (epoch zero counts as undated, not 1970); `rated_share`,
  `tagged_share`, `peak_year`, `peak(rows)`
- engine: `insights()` and `view.insights` (Ready, key `I`) returning the whole view as JSON
- ui: `View::Insights` with headline figures, proportional bars for ratings/labels/kinds, breakdown
  lists for cameras/lenses/keywords and a timeline column; the nav tab is no longer "planned"
- cli: `insights` prints the same numbers
- tests: totals reconcile, buckets sum to the library size, breakdown ordering and the top-8 fold,
  leap-day bucketing, an empty library having no shares, and the JSON face matching the typed view
- 161 tests, CI green; CLI smoke test over a two-photo library

## 2026-11-21 — M6: perceptual hashing and duplicate groups

- hash: `Hash { difference, average }` from a 9×8 box-filtered luma grid (every source pixel counted
  once, so the result does not depend on sample placement); `distance` takes the smaller of the two
  distances and ignores a zero average hash, which is the no-structure signal that would otherwise
  make every flat image match every other; `hash_file` decodes a small copy, using RAW embedded
  previews like the grid
- similar: `group` chains near-duplicates with union-find (largest group first, best member first
  inside), `around` for find-similar-to, best pick by pixels then stars then bytes then id
- engine: `begin_similar`/`poll_similar`/`hash_progress`/`cancel_similar` on a worker thread that
  reuses hashes cached by content key, one `Hash` job in the activity stack, `similar_to`; commands
  `similar.start` and `similar.groups`, `view.similar` now Ready
- ui: `View::Similar` with a start button, progress bar and cancel, a strictness slider, and one card
  per group showing thumbnails with the best member marked
- tests: gradient sets all 64 difference bits, a brightened copy stays within 8 bits, stripes one way
  and the other are far apart, identical files hash together, degenerate averages do not match,
  union-find chaining, stable grouping across input order, and an engine pass that groups two
  identical JPEGs while a third stays alone
- 177 tests, CI green

## 2026-11-21 — M6: metric index and colour fingerprint

Planned as "LSH banding"; the plan changed while building it. Banded LSH cannot stay exact at a
radius of ten bits without so many bands that every bucket holds a large fraction of the library, so
the candidate search is a BK-tree instead: exact under Hamming distance, visiting only nodes whose
edge range can still reach the radius. A brute-force cross-check test pins the index to the naive
answer.

- hash: `Colour` (3×3 mean RGB over the same box filter as the hashes) with a mean-absolute
  `distance` 0–255, and `analyse_file` so structure and colour come from one decode
- similar: `group_with(members, threshold, colour)` shares a BK-tree candidate search and vetoes a
  pair when both have fingerprints further apart than the tolerance; `matches` is the single rule
  `group`, `group_with` and `around` all use; `Member.colour`
- engine: the hash cache stores `(Hash, Colour)`, the pass uses `analyse_file`, `poll_similar_with`
  takes a colour tolerance, UI polls it every frame so a finished pass reports its group count
- ui: separate structure and colour sliders in the Similar view
- tests: the index matching brute force on 64 hashes, a colour mismatch refusing a structural match,
  the no-fingerprint case still grouping, 3×3 colour grid arithmetic
- 181 tests, CI green

## 2026-11-21 — M4: cull summary and apply receipt

- cull: `Summary::headline()` ("3 of 12 judged" / "Judged all 12 photos"); `trash::Receipt` built
  from a finished `DeletedBatch` with `headline`, `detail` ("Moved to …", tag-only spelling), `kept`
  and `is_restorable`
- engine: `receipt()`/`clear_receipt()` set by every Apply (mode, rejects disposed of, picks
  written, files kept, failures, batch id), `cull_complete()`, `cull_next_unjudged()`
- ui: a summary sheet that opens itself the moment every photo is judged and can be reopened from
  the cull header — counts, "N photo(s) will change on Apply", Apply…/Keep culling/Jump to next/
  Discard; and a receipt window that opens after Apply and offers "Put them back" (restore the batch)
  or "Recently Deleted", with dismissal clearing the receipt
- tests: a session reporting incomplete until the last decision, then complete; a tag-only batch
  whose receipt counts one disposed file, one pick, one kept file, is restorable, names its batch and
  is forgotten on dismissal
- 182 tests, CI green

## 2026-11-21 — M4: gesture culling

- cull: `gesture::classify(delta)` — a drag counts only past 84 points and only when the vertical
  travel dominates by 1.4×, so a sideways scrub never flags a photo; up is keep, down is reject.
  `progress` clamps toward the threshold for the overlay, `Gesture::decision`/`label` for staging
- ui: the cull canvas senses drags, tints and labels the photo while the finger is down (with a hint
  when the drag is too short), stages the decision on release and clears the drag state
- tests: short drags doing nothing, direction choosing the decision, sideways and diagonal drags
  ignored, progress clamped to 1
- 186 tests, CI green

## 2026-11-21 — M4: zoom and pan on the cull canvas

- cull: `viewport::View` (zoom 1–8, offset in points) with `zoom_by`, `wheel` (1.12× per notch),
  `pan`, `size`, `reset`; every change re-clamps the offset to `(zoomed − canvas)/2` per axis so the
  picture can never be dragged off the canvas, and zooming back to fit recentres
- ui: the canvas senses the wheel while hovered, ⌥/⌘-drag pans (a plain drag still decides), `Z`
  toggles fit/2×, the drawn rect is offset and clipped to the canvas at 1:1 when zoomed, and a
  "2.0× · Z to fit" badge appears; the interaction was extracted into `canvas_interaction` and
  `fitted_size` to keep the view under the complexity budget
- tests: clamps at both ends, recentring on zoom-out, pan stopping at the edge, no pan while fitted,
  offsets shrinking with the slack, wheel steps and a zero wheel being ignored
- 192 tests, CI green

## 2026-11-21 — M4: screen-reader announcements

egui exposes an AccessKit tree but has no "say this now" call, so the announcement is a visible
status line marked as an AccessKit polite live region: assistive technology reads it when the text
changes and never interrupts. The line is on screen too, because a status only a screen reader can
check is a second invisible UI.

- ui/a11y: `live_line` (label + `Live::Polite` + label text on the node), `cull_status`
  ("Photo 3 of 12, picked", "No photos to cull"), `library_status` ("240 photos, 3 selected"),
  `decision_sentence` ("Rejected, 4 photos left") and `decision_word`
- ui/shell: the cull view always shows the queue position and current decision as a live line, the
  library header announces how many photos are in view and selected, and every decision path —
  keyboard, buttons — sets `state.announce` so the result is read once
- tests: status sentences for each decision and an empty queue, pluralisation of "photo/photos left",
  zero/one/many counts, and every decision having a word
- 196 tests, CI green

## 2026-11-21 — M4: quit guard

- engine: `unsaved()` returns the specific reasons work is in flight — a running scan (files found so
  far unsaved), staged cull decisions (with the count, and the note that the session is kept), a
  running similarity pass — and `has_unsaved()` for the guard
- ui: a "Quit Sortlens?" dialog listing those reasons with "Keep working" and "Quit anyway"; a plain
  Action::Quit from the menu still closes immediately
- app: a close request with work in flight sends `ViewportCommand::CancelClose` and opens the dialog;
  confirming sets `confirmed_close` so the guard cannot loop on the way out, and a clean quit still
  closes the engine and flushes settings
- tests: a quiet library having nothing to lose, a running scan counting, an empty session counting
  for nothing, one staged decision producing exactly one reason, and discarding clearing it
- 197 tests, CI green

## 2026-11-21 — M5: collection colours and favourites

- catalog: `Collection.colour: Option<ColorLabel>` and `favourite: bool` with `serde(default)` so
  records written before this load unchanged; ops `SetCollectionColour` and `SetCollectionFavourite`
  whose inverses restore the previous value; `collections()` now sorts favourites first, then by
  name, so pinning one does not disturb the rest
- engine: `set_collection_colour`/`set_collection_favourite` and the `collection.colour` (name or
  `none`, unknown names rejected) and `collection.favourite` commands
- ui: the collections section moved into `collections_section` (it was pushing the sidebar past the
  complexity budget); favourites show a star, colours a dot in the theme's label palette
  (`Tokens::label`), and the context menu gains favourite/unfavourite and a Colour submenu
- tests: a new collection having neither, an old JSON record loading with defaults, colour and
  favourite undoing exactly, favourites sorting first, and the commands round-tripping with an
  unknown colour rejected and the old colour left in place
- 201 tests, CI green

## 2026-11-21 — M5: per-collection sort memory

- catalog: `Collection.sort: Option<String>` (serde default), op `SetCollectionSort` with an exact
  inverse; `set_collection_sort` skips a no-op and stores the id so a later `Sort::parse` reads it
  back, with unknown ids falling back to the natural order
- engine: `set_collection_sort` and the `collection.sort` command
- ui: opening a collection adopts its remembered sort, changing the sort while a collection is open
  writes it back, and `library_sort` keeps the library's own order so leaving the collection
  (`clear_filters`) restores it; the app seeds `library_sort` from saved settings
- tests: a collection starting with no sort, the sort round-tripping through the op log and undoing,
  an unknown sort parsing to natural, the command's JSON face, and the UI restoring the library sort
  when a collection closes
- 205 tests, CI green

## 2026-11-21 — M5: collection-scoped search

- catalog: `Collection.search: Option<String>` (serde default), op `SetCollectionSearch` with an
  exact inverse; `set_collection_search` trims, treats whitespace as "forget it" and skips a no-op
- engine: `set_collection_search` and the `collection.search` command, which returns the stored value
  (null when cleared)
- ui: the search box hint reads "Search in <collection>" while one is open; selecting a collection
  restores its stored search and sort, `remember_search` flushes the box when it loses focus or Enter
  is pressed (one op per edit session rather than per keystroke), and `library_search` keeps the
  library's own query so leaving the collection restores it — `clear_filters` does the same
- tests: trimming, a whitespace search clearing the record, undo restoring it, the command's JSON
  face, and the UI restoring the library search when a collection closes
- 208 tests, CI green

## 2026-11-21 — M5: first-run wizard

- ui: `Welcome { Add, Indexing, Ready }` decided by `welcome_stage(roots, scanning, total, dismissed)`
  — derived every frame so it cannot get stuck, and a dismissed wizard stays dismissed. The Add step
  explains that files stay put, changes are undoable and rejects wait for Apply, then offers
  "Add Folder…" or "Not now"; the Indexing step shows live file/folder counts, a progress bar and the
  photos found so far
- ui: the first scan that finds anything raises a one-time card — "Indexed N photos in <folder>" —
  with "Start culling" (switches to the cull view) and "Dismiss"
- tests: the stage for no roots, an empty root, a scan in progress (which wins over an existing
  library), a library with photos (no wizard), and dismissal overriding everything
- 209 tests, CI green

## 2026-11-21 — M5: keyword rename and merge

- catalog: `rename_tag(from, to)` rewrites every photo carrying the keyword in a single `Tagged` op,
  so a rename of two thousand photos undoes in one step; renaming onto a name already in use merges
  (the target is not duplicated), empty names and no-op renames are refused, and no photos means no op
- engine: `rename_tag` normalises both names first (so `BEACH` and `#beach` match), syncs sidecars
  for the affected photos, records a Sidecar job, and the `tag.rename` command reports the count
- ui: the Keywords sidebar row has "Rename or merge…", opening a dialog that warns when the target
  exists ("the two will be merged"), renames on Enter, and drops the renamed keyword from the filter
- cli: `rename <old> <new>`
- tests: rewrite across photos, merge without duplication, two undos restoring in reverse, refusals,
  an unknown keyword changing nothing, and the engine's normalisation plus merge
- 211 tests, CI green; CLI smoke test renamed two photos

Found while smoke-testing: **one-level undo does not survive a restart** — the inverse lives in
memory, so a fresh process reports "nothing to undo". That is a real gap, now listed as the next task
rather than glossed over.

## 2026-11-21 — M7: undo survives a restart

- catalog: log records gain an optional `inverse` field (serde default, omitted when absent), so a
  replayed record restores one undo level; `undo()` logs its own inverse too, which makes an undo
  itself reversible from a fresh process; the snapshot carries the last inverse because closing
  snapshots and empties the log — that was the actual hole the first attempt hit
- engine: nothing new; `edit.undo` was already wired
- cleaned up: the stale duplicate `cull.undo` command (Planned("M4"), same Ctrl+Z as `edit.undo`) and
  its planned-message entry are gone, so one command owns undo
- tests: rating undone after a reopen with records replayed, a snapshot rollover (empty log) keeping
  one level, a record written without an inverse still replaying, and the CLI undoing a tag in a
  process that never saw the change
- 214 tests, CI green

## 2026-11-21 — M7: empty the rejects bin

Semantics first, because "empty the bin" is ambiguous: Sortlens only ever forgets the *restore
record*. Files already moved sit in the system Trash, files that were only flagged stay where they
are, and nothing on disk is deleted from here. That is what the confirmation says.

- engine: `forget_batch(id)` (errors on an unknown id) and `forget_all_batches()` returning batch and
  file counts; both persist, neither touches the catalog or the files
- engine: `library.forgetBatch` and `library.clearDeleted` commands
- ui: each bin row gains "Forget" (with a hover that says the files stay put) and the dialog gains
  "Empty bin…", which opens a confirmation naming the batches and files, spelling out that nothing on
  disk is deleted and the step cannot be undone
- tests: two tag-only batches, one forgotten leaving the other, the rest cleared with counts, both
  files still on disk afterwards, the photos still in the library, an unknown id refused, and
  emptying an empty bin being harmless
- 215 tests, CI green

## 2026-11-21 — M7: backup and restore

- catalog: new `backup` module. A backup is a JSON document carrying the catalog's own text files
  (`snapshot.json`, `ops.log`) with a CRC32 each, so it reads in `less`, needs no compression
  dependency and refuses a truncated or hand-edited file. `create`/`encode`/`read`/`restore`; `read`
  verifies every checksum *and* the schema version before anything is written, and `restore` writes
  through a `.restore` temporary then renames
- catalog: `Catalog::flush()` so a backup can be taken from the live state without snapshotting
- engine: `export_backup(path)` (flushes first, atomic replace, records an Export job) and
  `import_backup(path, dest)`; commands `library.backup` / `library.restoreBackup`
- Deliberate limit, stated in the UI and the module docs: photos are never copied. A backup holds the
  index and the review state, not pixels — and a restore always targets a *separate* folder, because
  writing an older archive over the open library would silently undo work
- ui: Settings gains a Backup section ("Back up library…" and "Restore into a folder…"), app wires
  both to `rfd` dialogs and names the folder the restored library went to
- cli: `backup <file>` and `restore <file> <folder>`
- tests: catalog round trip through encode/read/restore with the rating intact, a tampered file
  refused by checksum, a newer schema refused by name, unexpected files and empty archives refused,
  an empty library refused, a snapshot-only library restored and reopened; engine round trip into a
  separate library with rating and keyword intact and the open library untouched; a damaged archive
  refused with nothing written
- 223 tests, CI green

## 2026-11-21 — M7: volume remap

The case this exists for: a shoot arrives on a card, is indexed, then the card is renamed or the
folder moves. Every relative path still holds, so the root can be repointed instead of relocating
photos one at a time.

- catalog: `Op::RemapRoot { from, to }` — rewrites the root list and every member's root in one op,
  with `RemapRoot { from: to, to: from }` as the exact inverse, so a mistake costs one undo. It
  refuses an unindexed `from` and a `to` that is already indexed
- engine: `remap_root(from, to)` refuses a path that is not a folder, refuses a root with no photos,
  then checks a **spread** sample of up to `REMAP_SAMPLE = 12` photos (`sample_indices` takes every
  n-th, not the first twelve, so a sample covers the whole root) for presence *and* size. A wrong
  folder fails with "<n> of <m> sampled photo(s) were not found" instead of quietly marking the
  library missing on the next scan
- engine: `library.remapRoot` command
- ui: the Library health dialog lists each root with its photo count and a "Point at…" button, since
  a moved volume is exactly what that dialog is opened for
- cli: `remap <old> <new>`
- tests: a renamed folder remapped, roots repointed, health reporting zero missing, and one undo
  returning to the old root; a wrong folder and a right-name-wrong-size file both refused with the
  root unchanged; an unindexed root refused; the sample covering 12 of 30 and everything of 3
- 225 tests, CI green. Verified by hand over the CLI, including the refusal path.

## 2026-11-21 — M7: integrity check and compaction

- catalog: new `integrity` module. `audit(dir)` answers "can this library be read back exactly?" by
  copying the two catalog files to a scratch directory and opening them there — the same code a
  normal open runs, with no side effects on the library. It reports the snapshot's seq, records
  replayed, records already covered, log bytes, photos, collections, whether the log tail was torn,
  and any warnings from the open
- catalog: `open()` now reports `torn_tail` and `skipped` in `OpenReport` instead of leaving them in
  prose warnings; `log_bytes()` and `compact()` (snapshot then truncate, returning bytes reclaimed)
- engine: `audit()`, `compact()`, commands `library.audit` / `library.compact`. Compaction **refuses**
  when the audit found anything worse than a torn tail, because rewriting the log from a damaged
  snapshot would cement the damage — found by hand: compacting a half-written snapshot produced a
  clean-looking, empty library
- ui: Library health gains a Catalog files section ("Check integrity", "Compact") that reports the
  headline, the detail and the bytes reclaimed
- cli: `audit` and `compact`
- tests: fresh library clean, records replayed counted, a truncated log tail reported as recoverable
  and then repaired by compaction (clean again, the readable prefix kept), an empty folder refused,
  compaction reporting reclaimed bytes, the engine round trip, and compaction refusing a library whose
  snapshot could not be read
- 231 tests, CI green. Verified by hand: audit on a clean library, a damaged snapshot
  reported as "Needs attention" with the reason, and compaction refusing it.

## 2026-11-21 — M7: change viewer and support report

- catalog: log records now carry `at` (unix seconds, omitted when unknown), so a change list can say
  when. `Op::summary()` gives every op a sentence and `Op::photo_count()` a count — a new op cannot
  arrive mute, which a test enforces
- catalog: new `history` module: `recent(dir, limit)` reads the log back newest-first, and
  `export(dir, limit, version)` writes a plain-text report (a hand-written civil-date conversion, so
  no date dependency for one line of output)
- **Behaviour change:** `Catalog::close()` now flushes instead of snapshotting. Snapshotting on every
  quit emptied the log on every quit, which would have made "recent changes" permanently empty — found
  by trying the CLI. Snapshots stay every 256 ops and on explicit compaction; `docs/BEHAVIOUR.md`
  records the trade
- engine: `history(limit)`, `export_history(path)`, commands `library.history` / `library.exportHistory`
- ui: "Recent changes" dialog (seq, sentence, age, op kind) reachable from the sidebar and a "Export
  report…" button; the report states that folder and file names appear and photo contents never do
- cli: `history [--limit N]`
- tests: newest-first ordering with summaries and dates, an empty log reading as no changes, the
  report naming every change with a UTC time and saying what it holds, dates rendering without a date
  library (including 1969 and a leap-year boundary), every op having a capitalised sentence, the
  engine round trip, plus the backup/integrity tests updated for the new close semantics
- 237 tests, CI green. Verified by hand: history across three CLI processes and an undo
  still working afterwards.

## 2026-11-21 — M7: one writer per library

- catalog: opening takes an OS file lock on `<library>/library.lock` (`File::try_lock`, stable in
  1.89, no dependency and no `unsafe`). A second process opens **read-only**: it replays the log and
  reads everything, but `commit()` refuses with "this library is open read-only because another
  window is using it". The log is opened for reading in that mode, so an append cannot slip through
- catalog: `Catalog::read_only()`, `OpenReport.read_only`, `LOCK_NAME`. The lock is an OS lock, so a
  crashed window releases it on exit — there is no stale-lock timeout to get wrong
- engine: `read_only()`, `library.readOnly` command, `begin_scan` refuses, and the cull-session and
  rejects-bin side files are not written in read-only mode (writing them would lose them silently)
- ui: the title bar shows "Read-only · another window has this library" with a hover explaining what
  to do, so the state is visible rather than discovered by a failed keystroke
- Fixed a real bug found by the first test run: field initialisation order meant `lock.take()` ran
  before the log was opened, so the writer opened its own log read-only and every append failed with
  EBADF. The flag is now read before the struct is built
- tests: catalog-level — second open is read-only, reads fine, refuses a commit, and the lock is free
  after the first closes; engine-level — a second window refuses rating and rescanning while the
  writer is unaffected, and a third window can write once the writer is gone; the existing undo-reopen
  test now drops its first catalog, which is the new rule
- 239 tests, CI green. Verified by hand with an OS lock held by another process: writes
  refused with the read-only message, reads fine, writes working again after the holder exits.

## 2026-11-21 — M7: retry unreadable metadata

The gap: `sortlens_meta::read` returned defaults on *any* failure, so "this file could not be read"
was indistinguishable from "this photo has no EXIF", and nothing ever tried again.

- meta: `ReadOutcome { Parsed, Unreadable }` and `read_outcome()`; `read()` stays as the forgiving
  wrapper. A missing file is retryable, an empty result is final
- scan: `ScannedFile.meta_unreadable`, so the flag survives the walk
- catalog: `Photo.retry` (serde default), `Seed.meta_unreadable`, `Op::Retry { changes }` with an exact
  inverse, `retrying_photos()`, `exifless_photos()`, `mark_retry()`. A rescan clears the flag when the
  file reads again
- engine: `retry_metadata(all)` returns `(updated, remaining)`, `retry_count()`, `exifless_count()`,
  command `library.retryMetadata` (takes `{"all": true}`)
- The `all` scope is the honest second half: after the app learns to read a format it could not parse
  before, "re-read every photo with no camera and no date" is the action a photographer actually
  wants, and it is a plain sequence of undoable `AddPhoto` updates
- ui: Library health shows the queue ("N photo(s) could not be read when indexed") with "Retry
  metadata" and "Re-read all without EXIF" — the second names its count and is disabled at zero.
  cli: `retry [--all]`
- Deliberate scope note: a file the *scanner* cannot read at all is still refused at scan time and
  named in the scan report (it has no identity yet, so there is nothing to queue). The queue covers
  reads that fail after the identity hash — a flaky volume, a file replaced mid-walk
- tests: meta distinguishes retryable from empty; catalog queues on sync, clears with an undoable op,
  and treats a re-queue as a no-op; engine reports `updated`/`remaining` with a file removed mid-retry
- 242 tests, CI green. Verified by hand: `retry`, `retry --all` updating two photos, and undo
  reversing it.

## 2026-11-21 — M7: files changed on disk

- engine: `check_health` now compares size **and** modification time against the index instead of only
  testing existence. `Health.modified` counts them and `Health.modified_ids` names them, so a refresh
  can act on exactly what the check found. A size difference is definitive; a time difference is only
  trusted beyond `MODIFIED_TOLERANCE = 2` seconds, so a filesystem with whole-second stamps cannot
  invent changes
- engine: `refresh_changed()` re-hashes (`sortlens_scan::content_key`) and re-extracts metadata for
  those photos only, storing one undoable `AddPhoto` update each — a rescan, narrowed to what moved
- engine: commands `library.changed` (counts and ids) and `library.refreshChanged`
- ui: the Library health dialog reports "N file(s) changed on disk since they were indexed" with a
  "Refresh changed" button that clears the count. cli: `changed` (listing the relative paths) and
  `refresh`
- `Health` lost its `Copy` derive to carry the id list; every other field is still plain counts
- tests: a quiet library reports zero changed (a stat alone must not invent work), replacing one file
  reports exactly that one, refreshing settles it to zero, and the refresh undoes like any other update
- 243 tests, CI green. Verified by hand: `changed` reporting one file by name, `refresh`,
  and `changed` quiet again
- Noted honestly: one `cargo xtask ci` run failed inside `sortlens-engine` before the message could be
  captured; three subsequent workspace runs, three engine-only runs and the next CI run were all green,
  so it is recorded as an unexplained flake rather than dismissed

## 2026-11-21 — M7: move picks

- engine: new `moves` module — `MoveItem { old, new, sidecar }`, `MoveReceipt { id, dest, at, items }`,
  `moved_record` (only the place changes: size, key and every review field carry over), `put_back`,
  `take_sidecar`, `prepare_dest` (absolute paths only)
- engine: `move_picks(dest, ids)` reuses `sortlens_cull::trash` for the filesystem half, so the batch
  is audited as a whole and a cross-volume move is copied, size-verified and only then removed. The
  catalog then follows the files in one `Op::Group` of `RemovePhoto`/`AddPhoto` pairs plus `AddRoot`,
  so the move is a single undo step
- engine: `undo_move(id)` puts files back **and** restores the records from the receipt, which is
  independent of the one-level undo — a move can be reversed tomorrow. Receipts persist in
  `moves.json`, alongside the rejects bin
- Fixed while testing by hand: the XMP sidecar was left behind, orphaning the review state it holds.
  It now travels with the photo and back again, and the receipt remembers whether it did
- commands `edit.movePicks`, `edit.undoMove`, `edit.moves` under Organise; ui: a "Move picks…" button
  beside Apply rejects (enabled by the pick count) and a "Moved" dialog with "Put them back"; cli:
  `move <dest> <ids…>`, `moves`, `unmove <id>`
- tests: the moved record keeps every fact but its place; putting a photo back creates the folders;
  a relative destination is refused; engine round trip with rating and keyword intact, the
  destination becoming an indexed root, a receipt surviving a lookup, the undo restoring file and
  record, an empty selection and a relative destination refused, and the sidecar check
- 247 tests, CI green. Verified by hand over the CLI including the sidecar travelling both
  ways

## 2026-11-21 — M7: side-by-side compare

- ui: `UiState.compare: Option<PhotoId>` and `toggle_compare` — opens a compare against the next photo
  in the current order (or the previous one when it is the last), and closes it on a second press
- ui: the detail canvas splits into two panes drawn at the **smaller** of the two fit scales, so the
  photo that needs less room is letterboxed instead of being blown up: the same detail at the same
  size in both panes is what makes a compare honest. `Z` still switches both panes to a shared 1:1
- ui: a difference line under the panes (stars, label, exposure when it differs, file size when it
  differs, pixel dimensions), `C` toggles the compare, `S` swaps which pane is left, and left/right
  walk the pair together so the two photos stay neighbours
- `C` in the detail view now means compare; the global cull shortcut is guarded against
  `detail_open`, so one key cannot mean two things in the same view
- A photo that disappears from the library closes the compare rather than leaving an empty pane
- tests: `compare_facts` reporting stars, pixels and size differences while staying quiet about
  identical frames; `toggle_compare` closing an open compare, and saying "Pick a photo first" rather
  than doing nothing when there is no selection (found by the test itself — the first version was
  silent)
- 249 tests, CI green

## 2026-11-21 — M7: colour-only duplicate matches

- hash: `Colour::mean()` — overall brightness, which is what makes the colour search exact below
- similar: `Match { Families, Colour, Either }` with `id`/`label`/`hint`/`parse`, `palette_matches`,
  and `group_in(members, mode, threshold, colour)`; `group_with` stays as the Families shorthand so
  existing callers keep their meaning
- similar: colour candidates come from a **brightness-sorted sliding window**. The distance is the
  mean of the absolute differences, so it can never be smaller than the difference of the means: two
  palettes whose brightness differs by more than the tolerance cannot match, and stopping the scan
  there is exact rather than a heuristic. No second index to keep in sync, and the window is small at
  any sane tolerance
- engine: `poll_similar_in(threshold, colour, mode)` and `regroup(...)`, which reuses the cached
  fingerprints so changing the mode costs no decoding; `similar.groups` takes `mode` and `colour`
- ui: a Match row in the Similar view — Structure / Colour / Either with the explanation on hover —
  plus a palette slider that only shows for Colour. Changing either regroups immediately
- tests: colour-only finds a same-palette pair whose hashes are opposite, refuses identical layout
  with a different palette, refuses a pair where one side has no fingerprint, and a chain of
  one-step palettes proves the window walks far enough to link them (and that a tighter tolerance
  than the step breaks every link); mode names round trip; engine-level, two flat colours never
  group and regrouping leaves the hash cache untouched
- 255 tests, CI green

## 2026-11-21 — M7: insights progress, highlights, seasonality

- insights: `Accumulator` — the same folding code behind both `Insights::compute` and the engine's
  background pass, so a progress bar can never disagree with the numbers it is progressing towards
- insights: `by_month: [usize; 12]`, `busiest_day: Option<Day>`, `peak_month()`, `peak_season()`
  (northern reading of winter/spring/summer/autumn, stated in the docs) and `highlights()` — up to four
  facts the charts cannot say: busiest month with its share and dominant season, the single busiest day,
  the most-used camera, and how much of the library has no capture date at all
- insights: `civil()` now returns year, month, day-of-month **and** the day's first second — the first
  version returned the day number in seconds where a day-of-month belonged, which the highlights test
  caught ("2024-02-1707523200")
- engine: `begin_insights()` (worker over a cloned photo list, progress every `INSIGHTS_CHUNK = 512`),
  `insights_progress()`, `poll_insights()`, a `JobKind::Insights` row in the activity stack, commands
  `insights.start` / `insights.progress`; `insights()` returns the finished pass when there is one and
  computes on demand otherwise; `view.insights` gains `byMonth`, `peakMonth`, `peakSeason`, `busiestDay`
  and `highlights`
- ui: the Insights view polls the pass, shows a progress bar with "done of total" and a Recount button,
  puts the highlights on one wrapped line and draws a twelve-bar month strip with the busiest month in
  the accent colour. cli: `insights` prints months and highlights
- tests: months and seasons from real timestamps (two February frames, a winter peak, leap-day and
  1970 handling), highlights including the busiest day, and the accumulator agreeing field for field
  with the one-shot pass; engine-level, the pass lands with the right count, reports no progress
  afterwards and agrees with `view.insights`
- 259 tests, CI green

## 2026-11-21 — M7: the keyword registry

- catalog: `aliases: BTreeMap<String, String>` in the snapshot and in memory, `Op::Aliased { changes }`
  with an exact inverse (a `Group` of per-alias inverses), `resolve_tag` (chains resolve all the way,
  with a step cap so a mistaken cycle cannot hang), `set_alias` (refuses a self-alias, no-ops when
  nothing changes), `aliases_of`, `orphaned_aliases`
- catalog: `rename_tag` now records `from -> to` as an alias **in the same op as the retag**, so one
  undo still reverses a rename — the first version committed two ops and a test that has been green
  since M5 caught it immediately
- catalog: new `registry` module — `Entry { name, count, aliases }`, `Registry { entries, orphans }`,
  `merged()`, `headline()` ("3 keyword(s) · 1 renamed into another · 1 alias(es) pointing nowhere")
- engine: `set_tag` resolves through the registry first, so typing `STRAND` after a rename lands on
  `beach`; `resolve_tag`, `registry_aliases_of`, `tag_registry`, `set_alias`; commands `tag.registry`
  and `tag.alias`
- ui: a keyword row with aliases says so on hover ("also written as strand"), which is where a
  photographer would otherwise wonder why two keywords became one
- cli: `registry` (counts, swallowed names, orphans) and `alias <name> [canonical]`; `tag`/`untag` print
  the keyword actually used and name the alias ("untagged beach (STRAND is an alias)")
- 263 tests, CI green. Verified by hand: rename, retype the old name, registry showing
  `2 beach <- strand`, and the CLI naming the alias

## 2026-11-21 — M7: the shortcut overlay and a wired keymap

- new `crates/ui/src/keymap.rs`: the keymap as a table — `Binding { keys, action, context, command }`
  over five contexts (Anywhere, Library, Detail, Cull, Compare), `search()`, `in_context()`, and a
  header that says how many bindings the overlay lists
- the overlay is `?` (or the sidebar "Shortcuts" row): a search field that takes focus on open, then
  each context with a one-line note and keycap-aligned rows
- **the table is checked against the command registry in both directions** — a binding that names a
  command must agree with that command's declared shortcut and be ready, and every ready command with
  a shortcut must appear in the table. It found four real drifts on its first run:
  `library.importSidecars` does not exist (the id is `library.import`), `library.import` is still M1 so
  Ctrl+Shift+I is not a live shortcut, `photo.flag` is declared `P / X` not `P`, and `cull.back` was
  written `Left` where the overlay writes `←`
- **the registry was claiming shortcuts the shell never handled.** `Ctrl+O`, `Ctrl+Shift+R`,
  `Ctrl+Shift+N`, `Ctrl+Z`, `Ctrl+,`, `Ctrl+Q`, `Ctrl+Enter` and the `I`/`S`/`D`/`C` view keys are now
  wired in `command_keyboard`, which consumes the press so the plain-key handlers behind it do not
  also fire; the view letters are refused while the detail view or a cull session owns them, so `S`
  still swaps a compare
- colour labels get their keys: `6–9` for red, yellow, green, blue (pressing again clears), and `0`
  stays "clear the rating" — `ColorLabel::KEYED` documents why purple is left off the row. The
  registry's `6–0` was wrong and is now `6–9`
- cli: `shortcuts` prints the bound commands (17 today) from the `app.shortcuts` command, so the
  wiring is checkable without the GUI
- 268 tests (5 keymap tests), CI green

## 2026-11-21 — M7: memories, and capture dates that can be corrected

- new crate `sortlens-memories` (23 → 24 workspace crates): `Memories::build(candidates, today,
  limit)` produces three deterministic cards — **On this day** (same month and day in earlier years,
  29 February falling back to the 28th so the card does not vanish three years out of four), **This
  week in past years** (a seven-day window compared by month and day, so a window across new year
  wraps instead of going empty), and **Never looked at** (no stars, no flag, at least a year old,
  oldest first). Cards are ordered best-first and say what they left out ("3 of 10 photos · since
  2019"). Dates are read as UTC and that trade is written down rather than papered over.
- engine: `memories(today)`, command `memories.list` (with an optional `today` so a card can be
  reproduced), and **`photo.setCaptured`** — a photo whose EXIF date is wrong could otherwise never
  take part in "on this day", so the date is now editable as an ordinary op (undoable, logged,
  written to the sidecar)
- catalog: `Op::Captured` with an exact inverse, and **`Op::is_noop()`**: `commit` now drops an op
  whose inverse changed nothing instead of logging it and spending the single undo level on it. The
  capture-date test caught this — undoing after a no-op commit had done nothing at all
- ui: a **For You** destination (bottom bar, 6th view) drawing each card as a strip of thumbnails;
  clicking one opens the detail view
- cli: `memories [--today YYYY-MM-DD]` and `date <id> <YYYY-MM-DD|->`; `--today` validated against
  real month lengths so `2021-02-30` is refused
- docs: `docs/BEHAVIOUR.md` gains a Memories section — derived not stored, UTC, the leap-day and
  new-year rules, deliberate overlap between cards, and why a judged photo leaves "never looked at"
- 278 tests (8 in `sortlens-memories`), CI green. Verified by hand: two dates set, three
  cards for 2024-06-24, and the change log showing both date writes

## 2026-11-21 — M7: a chart component

- new `crates/ui/src/charts.rs`: `Column { label, value, highlight }`, `columns()` (a column chart
  with a baseline, labels, hover tooltips and a highlight colour), `share_row()` (the proportional
  row the insights breakdowns use), `empty()` (a faint box that says "nothing dated yet" instead of a
  layout that silently lost its chart), and the arithmetic underneath as pure functions
- three rules, each with a test: **a non-zero value never rounds below one pixel** (an invisible bar
  reads as "nothing here"), **the tallest bar fills the height exactly** (integer-first arithmetic so
  a bar cannot stop short of its own axis), and **`share_of` refuses zero** rather than dividing by an
  empty library. `nice_max` gives round axis maxima (1/2/5 × powers of ten)
- heights are **fixed pixels**, never a fraction of the window: a chart that grows with the window
  makes the same library look different at different sizes and makes screenshots incomparable
- insights now draws its months through the shared chart (initials under the columns, busiest month
  in the accent) and its rating/label/kind/timeline rows through `share_row`, which also gained the
  tooltip the old local copy lacked — one drawing routine instead of two
- For You draws a small year chart per card, so a card that says "since 2019" shows whether the
  photos are spread across those years or all come from one
- `sortlens_memories::civil_year` added for the card chart, rather than duplicating date maths in UI
- the chart paints in a **headless egui frame** in a test (`Context::run_ui` with no window, texture
  deltas cleared), so bad geometry or an empty series fails in CI rather than in front of a user
- 285 tests (6 in `charts`), CI green

## 2026-11-21 — M7: show a file to the desktop

- new `crates/core/src/reveal.rs`: the two OS actions as **pure command construction** —
  `Desktop { MacOs, Windows, Linux }` with `current()`, `reveal()` (`open -R` / `explorer /select,` /
  `xdg-open <folder>`), `open_with_default()` (`open` / `cmd /c start ""` / `xdg-open`) and
  `reveal_selects()` so a caller can tell the truth about Linux. Arguments are always a list, so a
  file called `$(rm -rf ~).jpg` is one argument and never a command; six tests pin the exact argv,
  including the empty `start` title argument Windows needs
- **the split is the design**: the engine builds the command, the host runs it. `photo.reveal` and
  `photo.open` return `{program, args, line, ran: false}` — a library held in memory has no
  process table, so a command that claimed to have run would be a lie. `Engine::reveal_target` and
  `path_of` do the resolution and refuse a file that is no longer on disk with "rescan or relocate"
- app: `Action::Reveal` / `Action::OpenExternal` spawn the built command and report the line (or the
  desktop's inability to select a file on Linux) in the status bar
- ui: "Show in folder" and "Open" in the grid's context menu and in the detail header, next to Close
- cli: `reveal <id>` and `open <id>` execute the same command
- docs: `docs/BEHAVIOUR.md` gains "Showing a file to the desktop" — who builds, who runs, why
  arguments are a list, the Linux folder deviation, and that nothing here mutates the catalog
- 292 tests, CI green. Verified end to end: the CLI ran `open -R /tmp/sl-rev/ph/a.jpg` (Finder
  opened), and a file deleted behind the app's back was refused with the rescan instruction

## 2026-11-21 — M7: the MCP server

- `crates/mcp/src/lib.rs` is no longer a scaffold: newline-delimited JSON-RPC over stdio —
  `initialize`, `notifications/initialized`, `ping`, `tools/list`, `tools/call`. `Mcp::handle(line)`
  takes one line and returns one line (or `None` for a notification), so the whole protocol is
  testable without a pipe, and `Mcp::serve(reader, writer)` is the stdio loop
- **a tool is a command id**: `tools/list` is the command registry filtered to `Ready`, `tools/call`
  is `Engine::run`. No second list of agent actions, no per-tool schema to keep in sync, and a
  command still marked `Planned` stays out of the list — a tool that can only refuse is noise
- a refusal is a **tool result** (`isError: true` with the engine's own message), not a JSON-RPC
  error; JSON-RPC errors are reserved for malformed requests (`-32600`) and unknown methods
  (`-32601`). Notifications are never answered, because an extra message corrupts the stream
- `sortlens-cli --library DIR mcp` serves it. Verified end to end by piping four lines into the real
  binary: `initialize` answered the protocol version, the `notifications/initialized` line produced
  no output, `photo.rate` rated two photos, `library.stats` reported them, and `history` afterwards
  showed `#5 rate Rated 2 photos` — the agent's write lands in the same op log the user's undo reads
- six tests, including an agent culling a real three-photo library through the protocol
- 298 tests, CI green. **Not verified by hand**: a real MCP client (Claude Desktop et al)
  connecting to the binary; the protocol was exercised with piped JSON only
- docs: `docs/ARCHITECTURE.md` gains an MCP section (tool = command id, refusal is a result, why
  notifications go unanswered, and that writes go through the ordinary op log)

## 2026-11-21 — M7: arguments an agent can copy, proven by running them

- new `crates/mcp/src/examples.rs`: a worked argument set and a one-line note per command (55
  examples) plus `NEEDS_CALLER` for the ones whose arguments only the caller can supply — a folder to
  index, an archive to restore, a destination to move into (12 of those). `tools/list` carries both:
  the note goes into the description an agent reads before choosing, and `arguments.example` carries
  the exact JSON.
- **the examples are executed, not remembered.** `every_published_example_actually_runs` builds a
  fixture library, walks the table in order through `tools/call`, and fails the build on any
  `isError`. That is the property that matters: a published name is one the engine accepts, so a
  guess cannot drift into the docs.
- it immediately paid for itself, finding four kinds of real problem:
  1. **my own wrong names** — `collection.*` takes `id`, not `collection_id`; `cull.apply` takes
     `mode: "tag"`, not `"tag_only"`; `library.subfolders` requires `root`.
  2. **state dependencies** — `library.restoreBatch`, `library.forgetBatch` and `edit.undoMove` need
     a batch or a receipt, so the test creates one and reads the id back rather than assuming `1`
     (assuming it let a stale id pass on the first attempt).
  3. **an example ordering bug** — `cull.back` un-decides the staged photo, so `cull.apply` after it
     said "nothing is staged yet"; `cull.back` is now last, with a comment saying why.
  4. **two commands that were `Ready` with no handler at all**: `view.detail` and `view.similar`
     returned "command has no handler". Both now answer, via a new `Effect::ShowView(&'static str)`
     that the app turns into a view change — the engine names the view and the host switches it.
- the coverage test also refuses to let a ready command go undocumented, which is why the list is
  complete rather than partial.
- 302 tests, CI green. Verified against the real binary: `tools/list` returns 66 tools, all
  66 with argument guidance. **Not verified by hand**: a real MCP client; the protocol is exercised
  with piped JSON and the examples with direct calls.

## 2026-11-21 — M7: the registry and `Engine::run` now audit each other

- new test `every_ready_command_has_a_handler_and_every_handler_is_a_command` in `command.rs`: it
  reads the engine's own source (`include_str!("lib.rs")`), isolates `Engine::run`, collects its arms
  and compares both ways. Reading source in a test is blunt, but it is the only check that catches a
  half-connected command, and it found both bugs that had already shipped
- **fixed `library.page`**: `run` handled it while the registry had no entry, so it was invisible to
  the palette and to MCP. It is now `library.page` · "Read a Page" in `Group::Library`, with an MCP
  example that the examples test executes
- **fixed a quieter bug next to it**: `library.page` hard-coded `Sort::Natural`, so a caller could
  page but never order by rating or date even though every other paging path can. It now takes
  `sort` and reports the order it used; a test rates the last photo by name and asserts it leads a
  rating-sorted page
- cli: `page [--offset N] [--limit N] [--sort KEY]` prints one page as JSON, so a script can walk the
  library without the app
- 304 tests, CI green. Verified by hand: `page --sort rating` returned the five-star photo
  first. **Not verified by hand**: the palette entry for "Read a Page" (the UI does not call it; the
  grid uses `Engine::page` directly)

## 2026-11-21 — M0 debt: craft-fonts, optional by design

- `crates/ui/build.rs` reads `CRAFT_FONTS_DIR` and writes a `CRAFT_FONTS: &[CraftFont]` table, with
  **unset as a supported configuration** — CI has no checkout, so the empty table is the configuration
  under test. A native build embeds all 21 faces.
- `crates/ui/src/fonts.rs`: `install()` registers them and **appends** each face to `Proportional` and
  `Monospace` after egui's own, never prepended — a fallback that changes Latin text is a bug.
  `families()`, `embedded()`, and `coverage()` over `REQUIRED_SCRIPTS = ["Latn", "Zsym"]`, because the
  keymap draws `⌃ ✕ ◀ ✓` as text; `TRANSLATED_SCRIPTS` lists what translations will need.
- the app installs them in `App::new` before the first frame and logs the face count, so a build with
  fonts says so once instead of looking identical to one without.
- `cargo xtask fonts` (now part of `xtask ci`): unset prints how to enable them and passes; a set
  checkout reports face/family counts and script coverage, and a manifest with no Latin face fails.
  `fonts/embedded.txt` records what shipped, so a release can diff it.
- **verified with the real checkout**, cloned to gitignored `.fonts/craft-fonts`:
  `CRAFT_FONTS_DIR=… CRAFT_FONTS_REQUIRED=1 cargo test -p sortlens-ui -p sortlens` passes (30 ui
  tests, fonts installed), `cargo build -p sortlens` links, and `cargo xtask fonts` reports
  "21 face(s) in 20 family(ies) … Latn covered · Zsym covered"
- 307 tests, CI green in both configurations (with and without `CRAFT_FONTS_DIR`)
- **not verified by hand**: how the text actually looks with the craft faces — no screenshot, and the
  font stack is asserted structurally (defaults kept, first face unchanged, fallbacks appended), not
  visually. The app's `TIM` typography also still uses egui's default sizes.

## 2026-11-21 — M0 debt: a supply-chain gate that actually bites

- `deny.toml`: an allow-list, not a snapshot of what happens to be in the tree. Allow-listed: 0BSD,
  Apache-2.0 (and with LLVM-exception), BSD-2/3-Clause, BSL-1.0, IJG, ISC, MIT, MPL-2.0, OFL-1.1,
  Unicode-3.0, Ubuntu-font (per-crate exception for `epaint_default_fonts`), Unlicense, Zlib, with a
  `unicode-ident` exception for its Unicode-3.0 tables. GPL/AGPL/LGPL/CC-BY-NC fail by omission.
- `[sources]` allows exactly three git origins — `storytold/craft-libs`, `storytold/egui`,
  `storytold/winit` — so a new git dependency is a decision, not a diff nobody reads. (The first
  version listed `emilk/egui`; the check caught the fork URL, which is the point.)
- `[bans]`: `wildcards = "deny"` with `allow-wildcard-paths = true`. That needed every workspace
  member to say `publish = false` (25 `Cargo.toml`s, inherited from `[workspace.package]`), because
  cargo-deny treats a publishable crate's path dependencies as wildcards — the honest answer is that
  nothing here is published to crates.io; the repository is the distribution.
- **`BSL-1.0` and `Ubuntu-font-1.0` are deliberately absent from the allow list** even though crates
  in the graph carry them: `ryu` is "Apache-2.0 OR BSL-1.0" so BSL is never *chosen*, and the font is
  covered by the exception. Listing them produced "licence not encountered" noise, which trains
  people to ignore the output.
- `cargo xtask deny`: creates no pass when cargo-deny is missing — it fails and says
  `cargo install cargo-deny --locked`, because a check that silently does nothing is worse than none.
- CI: its own `deny` job on ubuntu, so a licence or advisory regression is named as that instead of
  hiding inside a three-OS test log.
- **the gate was proved to bite**: with MIT removed from the allow-list the licences check FAILS, and
  with `storytold/winit` removed from `[sources]` the sources check FAILS. Both reverted.
- `deny.toml`, `xtask deny`, README, TOOLING and CONTRIBUTING (including "a dependency change is a
  licence change") record the policy where a contributor will read it.

## 2026-11-21 — M0 debt: golden outputs, and a testkit to feed them

- `sortlens-testkit` is real instead of a scaffold: `Fixture` writes solid-colour JPEGs through the
  **real encoder** (so a fixture photo decodes, hashes and has a resolution), writes sidecars under
  the `docs/BEHAVIOUR.md` §4 naming rule, and removes its folder on drop. The same bytes every run is
  the property the golden job depends on, and `the_same_fixture_writes_the_same_bytes_twice` pins it.
  The sidecar helper's first version wrote `a.jpg.xmp`; the golden run showed no keyword imported,
  which is how the convention (`a.xmp`) got enforced in the helper instead of in a comment.
- `apps/sortlens-cli/tests/golden.rs` records **30 invocations of the real binary**
  (`CARGO_BIN_EXE_sortlens-cli`) against a fixture library: add ×2, roots, stats, list, keywords,
  folders, check, changed, rate, tag, rename, registry ×2, alias, untag-through-the-alias, date ×2,
  memories, insights, page, undo, history, shortcuts, move, moves, unmove, audit, compact, and the
  catalog's own `snapshot.json`.
- only three things are normalised — the fixture path, `in <elapsed>s`, and `"modified":<mtime>` — so
  a renamed field or a reworded summary is a visible diff. `page`'s JSON is checked verbatim
  otherwise, keys and content hashes included.
- both directions are checked: a missing golden fails, and so does a golden whose step no longer
  exists, because otherwise a deleted step leaves a claim passing forever.
- `cargo xtask snapshot` is **read-only**; `--update` sets `SORTLENS_UPDATE_GOLDEN=1` and prints a
  reminder to read `git diff`. A check that can update its own expectation checks nothing.
- CI: its own `snapshot` job on ubuntu. The fixture bytes and normalised output are
  machine-independent, so a second OS would only add noise.
- **proved to bite, twice**: editing one digit in `04 stats.txt` fails with the expected/actual diff,
  and dropping a `99 orphan.txt` into the folder fails with "no step records this golden any more".
  Both were reverted. The suite is deterministic across repeated runs (three consecutive read-only
  runs green).
- CONTRIBUTING says what to do when it fails: read the diff, `--update`, and say in the commit body
  what the output now says. Regenerating to get green is how a golden stops meaning anything.

## 2026-11-21 — the reconciliation pass, and what it caught

- **ROADMAP now says what is there.** 26 rows were marked unshipped while the code shipped them
  (M7's delete modes, backup/restore, remap, audit/compaction, read-only mode; M4's gestures, zoom,
  lookahead, apply receipt, quit guard, announcements; M5's collection colours/sort/search, move,
  wizard, tag rename/merge; M6's compare and insights worker; M1's folder tree and add/remove/rescan).
  Each was checked against the registry or the crate before ticking. The duplicate rows are gone: the
  catalog row that repeated a ticked one, and the cancelling/scheduler pair that appeared twice each.
- **the rows that stayed unchecked now name what is missing** instead of describing a whole feature:
  no density control, no multi-select or marquee, no justified rows, no `PathKey` or volume probe, no
  magic-byte sniffing, no `tagFromPrev`, no 20-step undo, no folder-level colours, no `--json` for the
  CLI verbs, no stale-revision rejection.
- **two real durability bugs, found by writing the two missing corruption tests.** `snapshot()`
  truncated the log after writing a snapshot, so a snapshot that was later damaged (half-written by a
  crash) left ops with nothing to act on: opening the library failed with `NoSuchPhoto` and the whole
  catalog was gone. Now `snapshot()` keeps the log — a snapshot is only an accelerator — and
  `compact()` is the single thing that empties it, after reading the snapshot back and checking its
  sequence, failing with "the snapshot did not read back; the log is kept". A record that will not
  replay stops the replay and is reported instead of aborting the open.
- tests added: `a_flipped_checksum_costs_one_record_not_the_library` (the good records before it
  survive, and the damaged record is the rating), `a_partial_snapshot_falls_back_to_the_log`,
  `compaction_is_what_empties_the_log`; `a_snapshot_rollover_keeps_one_undo_level` was rewritten to
  assert the *new* contract (the log keeps its records, `skipped` counts what the snapshot covers).
  The old version asserted the log was empty, which is what let the bug through.
- docs/BEHAVIOUR and ARCHITECTURE describe the snapshot/compaction rule as built; ROADMAP's stale
  "State:" lines for M0/M1/M2 are corrected; PLAN §12 is now "where delivery stands" instead of an M0
  to-do list, and §5.2 says it is the plan as written rather than a status report.
- README's badge and status block say M0–M7, list what works, and have a "what is not built yet"
  section so the absences are stated rather than implied.

## 2026-11-21 — M8 (half): export

- new crate `sortlens-export` (25 workspace crates). Planning is separate from writing, so every
  destination and every collision is decided before a byte moves: `plan()` takes the folder's existing
  names and returns jobs with a `skip` reason, which is what lets the UI say "0 written, 1 skipped"
  instead of starting a bar that never fills.
- **Format::Original** is a byte copy that keeps the source's extension case (`IMG_0001.CR3` stays
  `.CR3`); **Format::Jpeg** decodes, optionally clamps the *long* edge, re-encodes at a validated
  quality, and carries only APP1 (EXIF) and APP13 (IPTC) forward — forwarding every application
  segment would carry another program's editing state. PNG/TIFF/WebP are **not** offered: the
  workspace has one encoder, and a format chip that cannot write its format is a lie.
- collision policy is `Skip` / `Rename` (`name-2.jpg`, up to `RENAME_LIMIT = 1000` then reported) /
  `Overwrite`; two same-named photos inside one export collide with each other, not just with the folder.
- engine: `begin_export` / `export_progress` / `cancel_export` / `poll_export` / `last_export`, a
  worker thread and `JobKind::Export` in the activity stack. A photo that is no longer on disk is
  named and the export refuses. **No op is logged** — an export changes nothing about the library, so
  `docs/BEHAVIOUR.md` says why it cannot be undone: there is nothing to undo.
- commands `edit.export` and `edit.cancelExport` (+ MCP examples, which are executed by
  `every_published_example_actually_runs`); `all_photos()` became public for whole-library operations,
  with the doc comment that had drifted onto `key_for` put back.
- CLI `export <dest> [ids…] [--format] [--max] [--quality] [--collision] [--no-metadata]`, waiting for
  the worker and printing the receipt.
- UI: "Export picks…" in the cull bar, "Export…" in the grid context menu and the detail header, and an
  `export_dialog` with format chips, edge/quality, metadata, three collision words, progress, Stop, and
  a receipt that repeats that the originals were not touched.
- **the golden job caught a real bug the first time the verb was recorded**: `--max 24` was read as
  photo 24 (`no such photo: 24`), because the id list was `filter_map(parse)` over every argument. The
  CLI now walks arguments and skips flag values. New goldens: `30 export`, `31 export again` (JPEG,
  `--max 24`, `--collision rename`), and the `%PHOTOS%` placeholder that lets a step name an absolute
  path without the golden depending on where the fixture landed.
- **not verified by hand**: the dialog in a real window (no screenshot, same as every UI commit here);
  the headless `Context::run_ui` test paints it with nothing selected, with a full JPEG spec and with a
  receipt, and the engine tests exercise the worker against real JPEGs — but nobody has clicked
  "Choose folder…" in front of a person.
- 336 tests, `cargo xtask ci` green.

## 2026-11-21 — M8 (second half, part 1): the model store, and the promise it cannot reach the net

- `crates/ai` is no longer a scaffold. Three modules, no inference and no downloader — both stated in
  the crate docs so nobody assumes the other half exists.
- `sha256`: its own implementation rather than a dependency, checked against every FIPS vector
  (empty, `abc`, the 56-byte two-block case, the 64-byte aligned case, 1000 bytes), cross-checked
  against Python's `hashlib` while writing it, plus hex round-trip and a one-bit-change test.
- `manifest`: `Model { id, label, kind, licence, sha256, bytes, url, input_edge }`, `ModelKind`
  (Detector/Embedder/Captioner), `Manifest::parse` that refuses a newer schema and treats a missing
  digest, a zero size or a blank licence as errors. `verify()` checks **size first** (free) then the
  digest, and distinguishes a truncated file from a wrong one because the fixes differ.
- `store`: files are named `<sha256>.<kind>`, so a model cannot be silently replaced and two models
  cannot collide. `install` verifies in memory, writes `.incoming`, **reads it back and verifies
  again** (a short write is what a full disk looks like), then renames. `remove` refuses to delete a
  file it cannot identify — that file may be someone's download in progress.
- the shipped manifest is **empty**, on purpose: no model has been chosen, pinned and licensed for
  redistribution, so there is nothing to describe and no command or menu item was added. A settings
  row that lists nothing is better than a "Suggest tags" button that cannot work.
- `no_network.rs`: a source scan for socket APIs and a dependency scan for HTTP clients, proved to
  bite by planting `std::net::TcpStream` in `lib.rs` — it failed with the file name and the symbol, and
  was reverted. The file also carries a test that the scan *can* match real code, because a check that
  cannot fail is not a check.
- ROADMAP: the manifest/store row (SL-F01) and the no-network row (SL-F10) are ticked, with the
  downloader (SL-F02) called out as unbuilt and unlisted; ARCHITECTURE's ai section describes what
  exists and keeps the download plan clearly in the future tense.
- 362 tests, `cargo xtask ci`, `deny` and `snapshot` green.

## 2026-11-21 — M8 (second half, part 2): the arithmetic a detector needs, tested without a detector

- `crates/ai/src/detect.rs`: `Box` in **normalised** coordinates (so a 320 px and a 640 px copy of the
  same photo mean the same thing), `iou`, and `suppress` — NMS **per class**. Per class is the part
  that matters: a person in front of a car overlaps it almost completely, and suppressing across
  classes would delete one of them. A zero-area box cannot suppress anything, so a broken detector
  emitting a thousand empty boxes cannot erase the one real one — that case is a test.
- ties are broken by score, then class, then position, so a shuffled input produces the identical
  output (pinned by `equal_scores_still_produce_the_same_order_every_run` and the reversed-input test
  in `suggest`). NaN and sub-`MIN_SCORE` guesses are dropped before anything is compared.
- `LabelMap::parse` accepts `index name`, `index: name`, bare names and `#` comments, and **refuses a
  hole** in the indices — a map with a hole mislabels everything after it.
- `crates/ai/src/suggest.rs`: `merge` is where a guess becomes a proposal. A keyword the photo already
  has is never suggested again; two sources proposing the same tag become one at the higher score; a
  camera hint wins a tie against a guess because it is not guessing; tags shorter than three characters
  are dropped ("a", "of" are what a captioner says when it has nothing to say); the list is capped at
  `DEFAULT_PER_PHOTO = 8`, ordered by confidence then name.
- **nothing here writes to the catalog.** Accepting a suggestion is the only path from a proposal to a
  keyword, and that half (panel, `source = Ai` filter, the tag write) is not built — ROADMAP's two rows
  are split into "part 1" (ticked) and "part 2" (not) rather than left ambiguous.
- `rustc`/clippy pushed two small corrections worth keeping: a negated float comparison became
  `matches!(area().partial_cmp(&0.0), Some(Greater))` because that is what actually catches a NaN, and
  a box built from corners is compared with a tolerance (0.6 − 0.2 is not exactly 0.4).
- 385 tests, `cargo xtask ci`, `deny` and `snapshot` green. No new dependency: this is still a
  crate that cannot reach the network.

## 2026-11-21 — M8 (second half, part 3): the hints that need no model

- **why not the model**: pinning a real detector needs weights to hash, and no model file exists in
  this environment and none can be fetched here — inventing a digest for a file nobody has would be
  the exact lie this project keeps refusing. `tract` was therefore not added either: a session with no
  weights is untestable shape, and the checkpoint's own fallback was this.
- `crates/ai/src/hints.rs`: camera and lens names become keywords; focal length becomes `wide`
  (≤ 35 mm), `normal` (≤ 70 mm) or `telephoto`; ISO ≥ 3200 is `low light`; 1/15 s or slower is
  `long exposure`. Each threshold is a named constant with the reason written beside it, and every hint
  carries `Source::ExifHint` so the panel can say the camera said it, not a guess.
- the rules refuse nonsense rather than guessing: a focal length of 0, negative or NaN produces no
  band; a shutter of 0, negative, NaN or infinite produces no hint; a lens name that only repeats the
  camera (different case, same words) is not suggested twice. Output order is fixed, so a scan order
  cannot reshuffle the panel.
- **a real bug caught by a test written for the happy path**: the long-exposure rule compared
  `shutter <= 1/15`, which is backwards — seconds *increase* as the shutter slows, so a 4-second
  exposure matched nothing while a 1/1000 s one did. The constant is now
  `LONG_EXPOSURE_AT_OR_ABOVE` with the direction spelled out in its doc comment, and the boundary test
  asserts 1/15 s counts while 1/1000 s does not.
- hints feed `suggest::merge` unchanged — a test runs a camera hint through the merger against a photo
  that already has that camera keyword, and gets only `low light` back.
- ROADMAP's SL-F05 row is ticked; 395 tests, `cargo xtask ci` and `deny` green.

## 2026-11-21 — M8 (second half, part 4): the suggestion pipeline reaches the app

- engine: `Engine::suggestions` (one photo), `suggestions_for_all` (a selection) and
  `accept_suggestion`. The mapping from `Photo` to `ai::Hints` is a free function, `suggestions_for`,
  so it can be tested against a constructed photo — a fixture JPEG carries no EXIF, and a test that
  needed a camera in the file would be testing the scanner instead.
- commands `suggest.list` (ids → each photo's offers, with tag, score and source) and `suggest.accept`
  (id + tag). **Accepting goes through `Engine::set_tag`**, so it is an ordinary `Tagged` op: it is in
  the log, it is in the history, and `undo` takes it back exactly like a keyword typed by hand — tested,
  including that accepting twice reports `tagged: false` rather than claiming work it did not do, and
  that an empty tag or an unknown id is an error rather than a silent no-op.
- CLI `suggest <id>` prints score, source and tag; `accept <id> <keyword>` writes it.
- UI: a "Suggested" row in the detail view, one button per offer, hover saying the score and whether the
  camera or a model said it. The row is drawn by `suggestions_row`, which takes the offers rather than
  the engine, so the headless test paints it with real suggestions and headless egui proves it draws.
- MCP examples for both commands — executed, not remembered, by the existing test.
- **the golden test earned its keep again**: while extracting the CLI arm I sliced out the `keywords`
  arm by accident, and `06 keywords` failed immediately with `unknown command: keywords` before the
  commit existed. Restored, and three new goldens (`30 suggest`, `31/32 accept`, renumbered exports and
  snapshot after them) now pin the new surface. Renamed goldens were deleted first, so the orphan check
  never had to be loosened.
- ROADMAP: SL-F06 part 2 ticked. A new row records what is *not* done: `source = Ai` in the keyword
  filter needs a per-tag provenance in the catalog, which is a schema change, so the panel shows
  provenance in its own row only.
- **not verified by hand**: how the row looks in a window. No click was simulated headlessly — the
  button's action is covered by the engine test and the app's handler by the compiler.
- 400 tests; `cargo xtask ci`, `deny` and `snapshot` green.

## 2026-11-21 — M8 (second half, part 5): keyword packs are data

- `ai::packs`: a pack is JSON — a name, a licence, and rules whose `when` conditions (folder, camera,
  lens, minimum ISO, focal band) are **ANDed**. The ones that would misbehave quietly are refused: a rule
  with no conditions (one typo would tag the whole library), a blank name/licence/keyword, a pack with no
  keywords, a band that is not `wide`/`normal`/`telephoto`, and — via `deny_unknown_fields` — a
  misspelled condition, which is otherwise a condition that silently never matches.
- "No value on the photo" is a non-match, not a wildcard: a photo with no ISO does not match `min_iso`,
  and one with no focal length does not match a band. Pinned by tests.
- `Catalog::set_pack`: the switch is an op (`SetPack`), so it persists across restarts, appears in the
  history, and undoes; switching to the state it is already in changes nothing and does not spend the
  undo level. Tested across two reopens, with the log being what carries it — the first version of that
  test asserted a snapshot had carried it, which is not what happens (snapshots roll over every 256
  records), so the assertion now says "one record in the log".
- `Engine::packs` lists every file in `<library>/packs/`, **including the broken ones**, each with its
  problem: a mistyped pack someone was relying on must say so rather than vanish. Enabled packs feed
  `suggestions` through the same merger, so a pack keyword is a proposal like any other.
- `packs.list`/`packs.set` commands, MCP examples (executed), `packs`/`pack <name> on|off` in the CLI,
  a golden step, and Settings ▸ Keyword packs with a switch per pack and the licence beside it.
- `docs/PACKS.md` documents the format, the table of what is refused and why, and what a pack cannot do.
  `craft-flags` was deliberately not used: a boolean in the op log already persists and undoes, and a
  flag crate would add a second place for the truth to live — recorded on the ROADMAP row.
- **the golden test caught the same mistake twice**: slicing the CLI arms by index deleted the
  `keywords` arm again, and `06 keywords` failed with `unknown command: keywords` before either commit
  existed. Restored both times. The lesson is in the diff, not just in this file: the CLI match is
  long, and a text slice that ends at the next arm is not a safe edit.
- **not verified by hand**: the Settings rows in a window; no checkbox was clicked headlessly.
- 413 tests; `ci` (exit 0), `deny`, `snapshot` green.

## 2026-11-21 — M8 (second half, part 6): the batch pass, and M8 called done honestly

- `Engine::begin_batch_suggest` (+ `batch_progress`, `cancel_batch`, `poll_batch`, `batch_results`,
  `clear_batch`): the same merge as the detail view, run over the whole library on a worker with the
  progress/cancel/poll shape the other jobs use. `JobKind::Suggest` labels it "Suggestions".
- **it writes nothing**, and that is the tested promise: a test snapshots every photo's tags and the
  change log length, runs a pass over a library where a pack matches all three fixtures, asserts three
  results and then asserts the tags and the log are byte-identical. Accepting one of the proposals
  afterwards is what puts a keyword on the photo, through the ordinary tag op.
- commands `suggest.start`/`progress`/`cancel`/`results`/`clear`; the registry audit refused to pass
  until `progress` and `clear` were named there too, so no command can be unreachable from a menu; MCP
  argument notes and examples for all five (executed by the existing test); `batchsuggest` in the CLI
  with a golden step; a "Suggested keywords" panel opened from Settings, one button per proposal, no
  apply-all on purpose.
- `show()` had grown past the complexity gate, so every dialog moved into one `dialogs()` function — a
  mechanical change, but it is why this commit also touches unrelated-looking lines.
- **M8 is now called done honestly**: the three rows that need something this repository does not ship
  (a `tract` session and embeddings need weights; `source = Ai` needs tag provenance in the schema;
  PNG/TIFF/WebP need an encoder) moved from unchecked boxes into a "Deliberately absent from M8" table
  with the reason and what would unblock each. The demo line now describes something a person can
  actually do today.
- **not verified by hand**: the new panel in a window; the headless test paints it idle and mid-pass, and
  no click was simulated. 416 tests, ci (exit 0), deny, snapshot green.

## 2026-11-21 — M9 (part 1): the CLI answers in JSON, and refuses when it cannot

- `--json` prints the engine's own answer for 16 read verbs (roots, keywords, stats, list, folders, check,
  moves, changed, history, audit, memories, shortcuts, registry, packs, suggest, jobs), anywhere on the
  command line including before the verb.
- a verb without a schema **refuses and lists the ones that have one**. That is the difference between a
  script failing loudly and a script parsing prose; the refusal is a golden, so the message is contract.
- `library.roots` and `tag.keywords` added (registry + MCP note + executed example), tested to answer an
  empty list rather than an error for an empty library.
- **the new goldens found a real defect in the harness**: it normalised `"modified":<digits>` but
  pretty-printed JSON writes `"modified": <digits>`, so those goldens would have failed on the next
  machine. Fixed for both spellings; the first attempt used a trimmed length as a byte index and panicked,
  which the test caught immediately. Proved to bite by renaming a key in the roots golden.
- docs/TOOLING.md has a "Scripting the CLI" section with the verb list and the refusal text; ROADMAP
  ticks SL-K03/K06 and leaves one row naming what is still missing.
- 417 tests; ci (exit 0), deny, snapshot green. **Not verified by hand**: any of this in a real
  terminal — the goldens run the binary, which is the closest this environment gets.

## 2026-11-21 — M9 (part 2): the diagnostics bundle, with the leak test the golden would have hidden

- `Engine::diagnostics` (`crates/engine/src/diagnostics.rs`): plain text with versions, counts, warnings,
  the read-only health check, packs, the last 25 changes and the roots by placeholder.
- `diagnostics::redact` replaces every absolute path — `<root-N>`, `<library>`, `<home>` — sorted
  longest-first so a root under the library is not half-replaced. Root-relative paths stay.
- `diagnostics [--out FILE]` in the CLI; `app.diagnostics` in the registry with an MCP note and an
  executed example.
- **the test that mattered**: the golden normaliser replaces the library and fixture paths, so a *leak*
  would be invisible in a golden diff. A separate test runs the binary, reads stdout before normalisation
  and fails if either path appears. Without it the promise would have been untested by construction.
- the golden itself would have failed on the ubuntu job (`os = macos`); os/arch and version lines are
  recorded as `<host>`/`<version>`. First attempt inserted the normaliser before the tail was appended,
  which silently undid it — visible only because the golden was re-recorded and still said `macos`.
- M9's remote rows (server client, remote album edits) moved to a "Deliberately absent from M9"
  table: they need a second device and a network peer, and an untested auth path is worse than none.
- 422 tests; ci (exit 0), deny, snapshot green. **Not verified by hand**: the report as pasted into
  a bug tracker.

## 2026-11-21 — M10 (part 1): one version, and a release that says what it does not have

- `cargo xtask version` (in `ci`): the root `[workspace.package] version` is the only place a number is
  written. It fails when a member declares its own version or stops inheriting the workspace one, and when
  `sortlens_core::VERSION` stops coming from `CARGO_PKG_VERSION`. **Proved to bite** by hardcoding
  `version = "0.0.9"` in `crates/ai/Cargo.toml` (the first attempt at that broke `rust-version` instead,
  which is why the second attempt replaced the exact line) and restoring it.
- `sortlens-cli --version` / `-V` and `--help`: answered **before** a library is opened, so a version
  question cannot create one. A test runs the binary with `SORTLENS_LIBRARY_DIR` pointing at a folder that
  does not exist and asserts it still does not exist afterwards; a golden pins the text.
- `.github/workflows/release.yml`: verify → build `sortlens` and `sortlens-cli` for macOS/Linux/Windows →
  compute checksums **in one place from the uploaded files** → attach them. **Never run** — there is no
  tag, and the file's own comment says so. Per-runner checksums were removed while writing it: a hash taken
  on the same runner that produced the file is how a release ends up shipping a checksum of something else.
- `docs/RELEASE.md`: the single-version rule, the artefacts, the five steps to cut a release (green ci,
  bump once, check the absent-by-design tables, tag, read the generated page), the checksum format, and
  what this version does not have — no models, no remote, no localisation, no HEIC/video, no PNG/TIFF/WebP.
- `--version` golden and the ROADMAP rows ticked; signing, installers, a11y, performance budgets and the
  final release notes stay open and are named as needing things this environment lacks.
- 423 tests; ci (exit 0), deny, snapshot green. **Not verified by hand**: the workflow itself — it
  has never had a tag to run on.

## 2026-11-21 — M10 (part 2): attribution that is checked, not asserted

- **the finding**: 25 of the 40 external crates in `[workspace.dependencies]` are not linked by any
  manifest — including `log`, `chrono`, `rayon`, and fifteen `craft-*` crates. The old document implied a
  dependency set that does not exist. Rather than delete the pins (they are promises about versions for
  work that is coming), the document now separates **Linked today** from **Declared, not linked yet**, and
  says plainly that nothing in the second table is compiled into the app.
- `cargo xtask attribution` (now in `ci`) checks the document against the manifests in both directions:
  a linked crate must be attributed; a declared-but-unlinked crate must appear in its own table; a table
  entry for a dependency that no longer exists is a problem; every family in `fonts/embedded.txt` must be
  attributed; and the "No model weights, corpora or datasets ship with this project" sentence must still be
  there. It handles both manifest spellings — `crate = { workspace = true }` and `crate.workspace = true` —
  which the first version did not, and it reported `thiserror` as unlinked for that reason.
- **proved to bite twice**: a fake `serde_yaml` in `[workspace.dependencies]` fails with its name, and
  deleting the `NanumGothic` row fails naming the family. Both restored.
- the document itself was rewritten from the facts: the three pinned git origins, per-crate licences from
  `cargo metadata`, craft-fonts as a **build input rather than a dependency**, the four embedded families
  (all OFL-1.1) and why a symbols face is required, the fixtures being generated rather than stored, and
  the model-weights statement.
- 423 tests; ci (exit 0), deny, snapshot green. **Not verified by hand**: nothing here is visual.

## 2026-11-21 — M10 (part 3): every painted control has a name, and a check that keeps it that way

- **the gap**: the grid, the detail preview, the lookahead strip, cull thumbnails, similar clusters, memory
  cards and sidebar rows are painted rather than built from widgets, so none of them had a name. A control
  that can be clicked and cannot be described does not exist for anyone using a screen reader.
- `a11y::label_widget` now registers a `WidgetInfo` (role, label, selected) for each of them, and
  `a11y::describe_photo` says the file name first, then stars, flag, missing state, colour label and
  keywords — the order that decides a cull. Sidebar rows say their label and count.
- `cargo xtask a11y` (in `ci`) walks `crates/ui/src/shell.rs` function by function and fails when a
  function makes something clickable without naming it. **Proved to bite** by deleting the `nav_row`
  registration, which failed naming that function, then restoring it.
- **what this deliberately does not claim**: the emitted accessibility tree is not asserted, because this
  egui version has no public way to read nodes back. `Context::has_accesskit_node` reports `true` even for a
  plain unnamed allocation (measured, not assumed) and is cleared at the end of the frame, so a test built
  on it would have been theatre. The names are tested as pure functions, the coverage by the source walk,
  and the missing half — focus order, traps, a real client — is a separate ROADMAP row naming VoiceOver,
  Orca and NVDA as what would unblock it.
- `epaint` deprecation caught by clippy: `WidgetType` is now `Role`; the rename is applied.
- 425 tests; ci (exit 0), deny, snapshot and `a11y` green. **Not verified by hand**: how any of it
  sounds or reads with an actual screen reader.

## 2026-11-21 — M10 (part 4): measurements, and the snapshot defect they found

- **`apps/sortlens-perf`** + `cargo xtask perf`: a release-mode developer binary that builds its own
  fixtures (1200×800 JPEGs with a gradient and deterministic noise, because a flat colour decodes for free
  and would produce a thumbnail rate for the easiest possible file) and prints one table — scan, index,
  library open, cold decode, warm cache hit, the two-worker service the grid actually uses, the suggestion
  pass with and without EXIF, and headless grid frames with the first frame reported separately. Outside
  `ci` on purpose: a timing that fails on a loaded box gets ignored.
- **`docs/PERF.md`** records two runs on Apple M4 / 16 GB / macOS 27.0.1: two thousand photos (scan
  191 µs/file, index 190 µs/photo, cold thumb 9.5 ms, warm 16 µs, service 200 thumbs/s, frame 0.28 ms
  median, first frame 4.3 ms) and ten thousand (scan 823 µs/file, index 1.1 ms/photo, frame 1.40 ms),
  plus a table of the budgets that are **not** measured here — 100k scroll p99, GPU frames, RSS, mobile,
  HEIC/RAW — each naming what would unblock it.
- **The defect the numbers exposed**: indexing ten thousand photos took 78 s, 7.8 ms each against 0.19 ms
  at two thousand. Two bugs in the snapshot policy: `commit` snapshotted every 256 ops and a snapshot
  rewrites *every* photo (quadratic import), and `snapshot()` never reset `unlogged`, so once a library
  passed 256 commits *every edit* rewrote the whole library. Fix: a sync is a batch taking one snapshot at
  its end, and the snapshot resets the counter; single edits keep the 256 cadence. Ten thousand photos now
  index in 11 s at 1.1 ms each — the same per-photo cost as two thousand, so the curve is linear (7×).
- Tests assert the work, not a duration: `a_large_sync_takes_one_snapshot_instead_of_one_every_256_ops`
  (one snapshot, nothing unlogged, log still replays on reopen) and
  `single_edits_still_snapshot_every_256_commits`. Two audit tests and one golden changed because a fresh
  sync now leaves a snapshot behind — the golden shows `snapshot at seq 7` where it used to say `none`,
  which is the behaviour change made visible rather than hidden.
- 427 tests; ci (exit 0), deny, snapshot, a11y green. **Not verified by hand**: nothing in this
  commit touches the GUI beyond a headless frame; the frame numbers are layout and shape building, not a
  display.

## 2026-11-21 — M10 (part 5): the icon that existed and nobody loaded, and the table that names the gaps

- **the finding**: `assets/icon.svg` had been in the repo since M0 and **nothing loaded it** — the window
  was created with no icon at all. An asset that looks finished in a file listing and is absent everywhere
  a user meets the app.
- `crates/icon` is now the one place the mark is defined, in code: `rgba()` renders it (4×4 supersampled,
  512 px, rounded tile, three stacked prints, red lens on the front), `svg()` renders the same geometry for
  `assets/icon.svg`, and `png()` writes an RGBA PNG through a hand-rolled stored-deflate writer plus CRC32
  and Adler32 — no dependency, and byte-identical on every machine, which is what a generated asset needs.
- `apps/sortlens/build.rs` renders the pixels into `OUT_DIR` and `main` sets `with_icon`, so no binary blob
  is committed and there is no second copy to disagree with the first.
- **Proved to bite**: the SVG is held to the drawing by
  `the_committed_svg_is_the_drawing`; appending a line to the file failed it, and `SORTLENS_UPDATE_ICON=1`
  rewrote it (that path is the documented way to change the mark). `the_window_icon_decodes_to_the_mark`
  decodes the PNG with `sortlens-thumbs` (a decoder that was not written for it) and checks the transparent
  corner, the front print's colour and the lens; `the_icon_is_square_and_rounded` checks the shape;
  `the_window_icon_is_the_generated_mark` in the app checks what the build script actually produced.
- **M10 closed honestly**: "Deliberately absent from M10" table added (signed/notarised macOS, MSI and
  AppImage/Flatpak, `.icns`/`.ico` and the brand pass, theme import, high contrast and reduced motion,
  `craft-update`, release notes and the 1.0 tag) — each with the reason and the unblocker, and the "State:"
  line rewritten to match. The rows those replace were moved into it rather than left as empty boxes.
- Files: `crates/icon/`, `apps/sortlens/build.rs`, `apps/sortlens/src/main.rs`, `assets/icon.svg`,
  `docs/ROADMAP.md`, `docs/ARCHITECTURE.md`, `docs/ATTRIBUTION.md`, `docs/RELEASE.md`.
- 432 tests; ci (exit 0), deny, snapshot green. **Not verified by hand**: how the icon looks in a
  real window on each platform (macOS dock, Windows taskbar); checked only as pixels and as a decoded PNG,
  plus a rendered preview of the SVG at 200/64/32 px.

## 2026-11-21 — M10 (part 6): the documents checked instead of trusted

- **the drift**: README's badge said `M0–M7 shipped` and its prose said "M0–M8 partially shipped … M9 and
  M10 are unstarted" while M10 had shipped; `docs/PLAN.md` §12 still listed M8, M9 and M10 as remaining
  work; `crates/memories` had never been added to the layer diagram; `apps/sortlens-perf` was unknown to it;
  `docs/PERF.md` existed and the README's document table never mentioned it.
- **`cargo xtask docs`** (in `ci`) now checks the facts that have one right answer: the shipped milestone
  range is **computed from `docs/ROADMAP.md`** (the highest milestone with a ticked row) and required in
  `README.md` twice — prose and the percent-encoded status badge, which is how it silently kept saying
  M0–M7 — and in `docs/PLAN.md` §12; every `crates/*` package name and every `apps/*` directory must be
  named in the layer diagram; every `docs/*.md` must be reachable from the README. **Proved to bite** three
  ways: an edited badge, a deleted `sortlens-memories` line, and each reported its own file.
- README rewritten where it was wrong: status, the document table (`docs/PERF.md` added), the build
  commands (`ci` now also runs version, attribution, a11y and docs; `perf` added), and "What is not built
  yet" — export and AI suggestions were listed as missing after they shipped. The comparison table's
  Updates row now says `craft-update` is declared and unlinked rather than claiming signed artifacts.
- PLAN §12 and §5.2 rewritten to the M0–M10 reality; `docs/FEATURES.md` gained a note that its `M` column
  is a plan, not a status; `docs/TOOLING.md` documents the new gate; ROADMAP's status log has the entry.
- Duplicated ROADMAP rows **deleted** as the rule requires: the five M10 rows that only said "moved to the
  table below", the duplicate `craft-update` row, and the accessibility part 2 row, which is now a row in
  the absent-by-design table (with why a headless test there would assert nothing).
- 432 tests; ci (exit 0), snapshot, deny green. **Not verified by hand**: nothing here is GUI work.

## 2026-11-21 — M1/M2 rows: real sources, and a grid that says why a cell is empty

- **Sources, part 1 (SL-A14)**: the sidebar now offers **Picks**, **Rated** and **Rejects** beside All Photos.
  Each row counts what it would show (Picks reading "0" while the library holds thousands is the row working),
  and each **replaces** the question instead of nudging it — picking Picks with three stars and a keyword
  active used to leave them on, so the row silently meant "picks among the ones you starred and tagged".
  The folder scope survives: that is where you are, not what you asked. The highlighted row is derived from
  the filter, so a filter chip asking the same question highlights the same row; a stored flag would let the
  two disagree. `Source` lives in `crates/ui/src/shell.rs` with `apply_source`/`source_selected`, tested as
  pure logic, plus a headless paint of the real sidebar over a two-photo library with counts that differ.
- **Placeholder + badges (SL-M10, SL-A12 part 1)**: a cell that has no pixels now says which kind of nothing
  it is — `?` for a file that is not on disk, `!` for one that could not be decoded, each with a hover that
  explains it, and a missing file outranking the decode failure it caused. `cell_badge` is pure and tested
  against all four combinations; the cell paints for a missing photo without requesting a decode.
- Rows split rather than ticked whole: **Recently Added** needs an `added` timestamp per photo (a schema
  migration, and the file's mtime would be "recently modified" — a different question), and cached-thumb
  behaviour for offline libraries needs staleness marking. Both say so in their rows.
- Files: `crates/ui/src/shell.rs`, `crates/ui/Cargo.toml` (dev-dependency on the fixture kit),
  `crates/ui/src/lib.rs`, `docs/ROADMAP.md`. 436 tests; ci (exit 0), snapshot green.
  **Not verified by hand**: the badge at small cell sizes, and whether "Rejects" is a source people want
  next to Picks — it is in the list because rejects are still in the library until you delete them.

## 2026-11-21 — M1 row: the header decides when the name is wrong

- **`sortlens_core::media::sniff`** reads the first 16 bytes and names the family: JPEG, PNG, TIFF (+ every
  raw built on it), HEIF/AVIF, WebP, and ISO base-media brands — including `crx ` for CR3 and the video
  brands. An **unrecognised** brand comes back `Unknown` rather than "video": guessing would drop podcasts
  and `.m4a` sidecars into the grid.
- **`resolve_kind`** corrects only a contradiction: a JPEG preview named `.cr2` is a still, an MP4 named
  `.jpg` is a video, a still named `.mov` is a still. Ambiguity keeps the extension's answer — every raw is
  also a TIFF, so a `.tif` stays an image instead of turning the whole scanned tree into raws.
- The scan counts what it corrected (`Skipped::mislabelled` → `ScanFinished::mislabelled`), the window's
  notice says "N routed by content, not name", and the CLI prints the same line: a correction nobody is
  told about is a silent rewrite of what their tree means.
- **Golden proof**: the fixture gained `Photos/Trip/preview.cr2` holding JPEG bytes, and `01 add photos` now
  reads "5 added … 1 routed by content, not name" — the correction and the report are pinned as published
  text. Fourteen goldens moved by exactly the counts one extra photo implies; the diff was read.
- Files: `crates/core/src/media.rs`, `crates/scan/src/lib.rs`, `crates/engine/src/lib.rs`,
  `crates/ui/src/shell.rs`, `apps/sortlens-cli/src/main.rs` (+ `report_scan`, after clippy's cognitive
  complexity gate caught `run()` at 31/30), `apps/sortlens-cli/tests/golden.rs` and the goldens.
- 440 tests; ci (exit 0), snapshot green. **Not verified by hand**: a real camera raw whose header is
  a TIFF with an embedded JPEG (the `cr2` case is synthetic JPEG bytes), and a real CR3.

## 2026-11-21 — M1 row: know the volume, so a missing drive is one answer

- **`sortlens_core::volume`**: `parse_mounts` reads a mount table (skips comments, resolves the four
  documented escapes `\040 \134 \011 \012` — as **octal**, which the first attempt got wrong and the
  escape test caught), `volume_of` picks the longest mount-point prefix so a nested volume or a bind mount
  behaves, and the platform convention is explicit: Linux uses the table, macOS the `/Volumes/<name>` rule
  (no `/proc`, and `statfs` would need `libc`), Windows the drive letter with a UNC path marked removable.
- **`Engine::root_volumes()`** answers per root, not per photo: the whole point is that an unplugged drive
  holding ten thousand photos is one missing volume. Each entry carries `mount`, `removable` and
  `mounted: Option<bool>`, where `None` means "this platform could not tell".
- **The sidebar** marks such a root "12 · away" with a hover naming the mount point, and treats `None` as
  present — a missing answer must never be rendered as a missing drive.
- Tests: seven on the parser/conventions (including a path that merely *starts* like a removable mount
  falling back to `/`, and `/Volumes` itself not being a volume), one on the engine, and one on the sidebar
  decision that pins `Some(false)`→away, `Some(true)`/`None`→quiet, plus a headless paint with a real root.
- **Split, not ticked whole**: `PathKey` moved to its own row — it needs Unicode normalisation tables and a
  per-volume case-sensitivity probe, and an ASCII lowercase "fix" would silently merge `Ångström.jpg` with
  `ångström.jpg` on a case-sensitive disk.
- Files: `crates/core/src/volume.rs` (new), `crates/core/src/lib.rs`, `crates/engine/src/lib.rs`,
  `crates/ui/src/shell.rs`, `docs/ROADMAP.md`. 449 tests; ci (exit 0), snapshot, deny green.
  **Not verified by hand**: a real unplugged drive on macOS (the "away" path is exercised only through the
  decision function here), and a Windows machine.

## 2026-11-21 — M1 row: density, and a settings field that finally does something

- **the finding**: `appearance.density` had existed in `config.toml` since M0 with a comment saying
  "comfortable or compact", and **nothing read it** — the grid hard-coded `min_cell = 132.0`. A persisted
  setting nothing consumes is the same lie as a button that looks enabled and does nothing.
- **`Density`** (`crates/ui/src/theme.rs`): Compact / Comfortable / Roomy, each naming a minimum cell
  (104/132/184), with `id()`, `label()`, `from_id()` (an unknown id reads as the default so a settings file
  from a later build cannot stop this one opening) and `next()` for the chip's cycle.
- **`grid_layout(available, gap, min_cell)`** is the whole of density, extracted and pure: it divides the
  window so the last cell in a row reaches the right edge, and floors a cell at 64 pt — below that the
  caption and stars stop being legible, so a narrow window gets one column that does not shrink rather than
  five that are unreadable.
- The chip sits beside the theme (both are "how the library looks") and the choice persists through
  `Action::SetDensity` → `appearance.density`, applied at startup from the store.
- Tests: layout (more columns when dense, exact fit at each step, the 64 pt floor and one-column minimum),
  the step cycle and id round trip, a settings round trip for all three ids **plus an unknown id falling
  back**, and a headless paint of the real grid at all three densities over a three-photo library.
- Files: `crates/ui/src/theme.rs`, `crates/ui/src/shell.rs`, `crates/ui/src/lib.rs`,
  `apps/sortlens/src/main.rs`, `apps/sortlens/src/settings.rs`, `docs/ROADMAP.md`. 454 tests; ci (exit 0),
  snapshot green. **Not verified by hand**: how the three steps feel on a real display — the numbers
  are a judgement call, and the only way to judge them is to scroll a shoot.

## 2026-11-21 — M1 row: a selection that can hold more than one photo

- **`Selection`** (`crates/ui/src/shell.rs`) keeps three things apart: the **primary** (what the inspector
  shows), the **ticks** (what a batch acts on) and the **anchor** (where a ⇧-click measures from). They cannot
  be one field — removing the primary has to move the inspector to a photo that is still ticked, and a
  ⇧-click measured from the primary would walk instead of extending.
- Gestures: **click** replaces, **⌘-click** toggles, **⇧-click** takes the run from the anchor *in the order
  the grid shows them* (a range in id order would select something nobody is looking at), **⌘A** selects
  everything in view (capped at `MAX_RANGE` = 20 000 so a click cannot stall on a huge library) and **Esc**
  clears. A ⇧-click with no anchor in view falls back to a plain click rather than selecting half the library.
- Batch keys: rating, flag and label now act on `selection.ids()` and the notice says the count — "Rated 4★
  on 12 photos" — instead of pretending one photo changed. Label toggle clears only when the primary already
  carries the colour, so pressing the same key twice over a mixed selection sets rather than clears.
- The selection is pruned against the library once per frame, so a delete or a rescan cannot leave the
  inspector showing a photo that no longer exists; `library_status` now speaks the selected count instead of
  a 0/1 flag.
- `keymap.rs` documents Click / ⌘-click / ⇧-click and Ctrl+A, so the shortcuts dialog is not silent about the
  three gestures it now has.
- Tests: the gesture state machine (add, remove, primary follows survivors, range forwards and backwards
  from a stable anchor, anchor-less range falls back), select-all + prune, a real engine batch write over
  three photos, and a headless paint with a three-photo selection that asks for nothing.
- 458 tests; ci (exit 0), snapshot green. **Not verified by hand**: the gestures with a real mouse —
  modifier-click in egui behaves, but only a person can say whether ⇧-click over a wrapped row feels right.
- **Split, not ticked whole**: marquee is its own row. It needs cell rectangles collected during paint and a
  drag that does not fight the scroll area's own, which is a piece of work rather than a footnote.

## 2026-11-21 — M1 row: marquee, without stealing the grid's clicks

- **the hazard**: a drag rectangle over a grid is easy to add badly — if the drag is allocated on top of the
  cells, clicking a photo stops working. So the drag only *starts* on the background, decided against the
  cell rects painted on the previous frame: the pointer may cross cells this frame that it had not reached
  when the press happened, and the press is the moment that decides which gesture you are in.
- `marquee()` paints the band, selects the cells it touches (a graze counts), notices the count only when the
  set actually changes — a notice every frame would spam the live region sixty times a second — and clears
  its own state on release without clearing the selection it made.
- `cells_in()` is the hit test as a pure function: intersects, sorted by id so a log reads the same twice.
  Tested on a 2×2 of cells, including the edge graze that the first version of the test got wrong (it grazed
  105, and cell two starts at 110 — the test was fixed, not the code).
- `Selection::set_many` is what a marquee and a future "select these" command share; an empty band clears,
  because dragging over nothing means nothing selected.
- Two headless tests: the pure hit test, and the gesture driven with real pointer events — press, move,
  release — asserting the band paints while the pointer is down and the selection survives the release.
  `keymap.rs` documents the drag so the shortcut sheet is not silent about it.
- 460 tests; ci (exit 0), snapshot green. **Not verified by hand**: how the band feels while dragging a
  real grid, and whether the 0.35-alpha fill reads on the light themes.

## 2026-11-21 — M1 row: the justified grid, and both layouts through one gesture handler

- **The pack** (`justified_rows`, pure): each row grows until the next photo would push it below the target
  height, then takes whichever of the two candidate heights is closer to the target — stopping at the first
  overshoot left rows 40 points taller than asked for whenever the library's average shape did not divide
  evenly, which the first version of the test caught as a failure of my *expectation*, and the second version
  caught as a real flaw in the packer. A short last row keeps the target height and sits left-aligned instead
  of stretching two photographs across the window; a photo wider than the window gets a row of its own; an
  empty library has no rows.
- **The plan** (`JustifiedPlan`) is cached on (width, filter, sort, count, target) and rebuilt only when one
  changes: the pack is one pass over every filtered aspect ratio, paged from the catalog in 2 048-photo
  chunks, and doing that per frame would make a big library a slideshow. Aspect ratios come from
  `aspect_of` — the function the compare view already used, now clamped so a panorama cannot produce a
  zero-height row — with 3:2 for photos whose dimensions never loaded.
- **Painting** windows by *row*, not by a fixed height: `show_viewport`, one `add_space` to jump past
  everything above the viewport instead of a loop over skipped rows, then only the rows that intersect it.
- **One gesture handler** (`cell_gestures`) for both layouts: click / ⌘-click / ⇧-click, double-click and the
  context menu are about photos, not pixels, and two grids that disagreed about ⌘-click would be one grid too
  many. `photo_cell_sized` gained `Fit::Cover` (square, crops) vs `Fit::Contain` (justified, already the
  photo's own shape, so cropping would discard the part of the frame the row height was computed from).
- **Density keeps its meaning**: in justified mode the 104/132/184 target becomes the row height, so
  "compact" still means more photographs at once. `G` and a chip toggle the layout; the choice persists as
  `appearance.grid`; the doc's claim that justified is the default is now true (`GridMode::default()`).
- `apps/sortlens-perf` now measures **both** layouts and labels them: one unlabelled `grid frame` line would
  have changed meaning the day the default flipped. Square 0.31 ms, justified 0.14 ms median at 2 000 photos —
  justified is cheaper per frame because the pack is cached, which `docs/PERF.md` now says out loud.
- Docs reconciled in the same commit: ROADMAP SL-B01/B02 ticked with what each layout actually does, and
  `docs/UX.md` §5.3 rewritten — it had been describing a density **slider** (90–480 px, `-`/`=` stepping)
  that was never built, while the shipped control is a three-step chip.
- 465 tests; ci (exit 0), snapshot, docs green. **Not verified by hand**: how the row heights feel while
  scrolling a real library, and whether `Contain` leaves visible letterboxing for any real camera ratio.

## 2026-11-21 — M2 row: priority and a bound on the decode queue

- **Priority is a value in the request**, not a queue per priority: `Urgency::{OnScreen, Nearby, Background}`
  served most urgent first and oldest first within a class. Workers now pull from one shared queue under a
  condvar instead of a channel — a channel hands out what it was given in the order it was given, so priority
  ordering was impossible without this change, and that is why the row had never been ticked.
- **Backpressure is a bound**: `QUEUE_CAPACITY` = 4 096 waiters. A request arriving at a full queue evicts the
  least urgent waiter only if it is more urgent itself; otherwise it is refused and counted. The victim is the
  *newest* of the least urgent class, because the oldest has waited longest and is closest to being decoded.
  An evicted key is released from the in-flight set, so a later attempt for the same thumbnail is not
  mistaken for a duplicate that is already on its way — the bug this design would otherwise have had.
- Callers: the grid asks at `OnScreen` (unchanged signature), the detail view prefetches the next photo in the
  sorted order at `Nearby`, and library-wide passes belong at `Background`. `Thumbnails::queue()` exposes
  waiters and refusals; `apps/sortlens-perf` prints the refusals so the bound is visible in a measurement
  rather than only in a doc.
- Tests: the queue on its own (urgency ordering across and within classes, eviction of the newest least
  urgent, refusal and its counter), the service at the bound (a refused request can be asked for again once
  the queue drains), and a headless 400-photo grid against an **eight**-slot queue that must refuse.
  **Proved to bite**: raising that test's bound to 4 096 made it fail with "63 requests waiting, over the
  eight slot bound", and lowering it back made it pass.
- `docs/ARCHITECTURE.md` §5 rewritten from the plan to what the code does, including why the channel went.
- 469 tests; ci (exit 0), snapshot green. **Not verified by hand**: whether the priority actually feels
  different while scrolling a real library — the ordering is unit-tested on the queue, but only a person can
  say whether a stale scroll still shows decoded cells sooner than it did.

## 2026-11-21 — M2 row: the bell, and notices that outlive their toast

- **The gap**: notices were a four-second toast and nothing else. Anything that happened while you were
  looking at another part of the window — a scan finishing, a move failing — was gone before you looked back,
  and a failure read exactly like a success.
- **`Notice { text, kind, at, read }`** in a bounded `VecDeque` (50, newest first) beside the toast, so the
  bell is a history of the same events rather than a second system that could disagree with the first.
  `NoticeKind::{Info, Problem}`; `notice()` is info, `notice_problem()` raises a problem.
- **The bell** appears only when it has something to say (a permanent "0" is chrome), shows a count of
  **unread** notices, and goes red when any unread one is a problem. Opening the popover marks them read;
  reading is not clearing, so "Clear" is separate. Rows carry an age said the way a person would say it.
- **The toast uses the same kind**, taken from the history entry raised with it, so a failure is red in both
  places and the two cannot drift.
- **32 call sites** that toasted a bare `error.to_string()` now raise a problem: a failure that scrolls past
  after four seconds has not been reported, and the badge should say so.
- Tests: the count and the red state, reading versus clearing, trimming past `NOTICE_HISTORY` without
  corrupting the count, the age table, and a headless paint of the bell (asking for nothing, reading nothing)
  and of the popover. `docs/UX.md`'s component table now describes the bell instead of promising a job list it
  never was.
- 473 tests; ci (exit 0), snapshot green. **Not verified by hand**: where the popover lands against a
  real top bar at a real window size, and whether "just now" is the right threshold rather than "now".

## 2026-11-21 — M2 row: the MCP server stops hand-rolling its protocol

- **Envelopes are `craft-jsonrpc`.** It validates the `jsonrpc` version and the id, tells requests from
  notifications, answers a batch as a batch, and refuses a message over a byte bound **before** parsing it —
  the bound protects the allocation, so it cannot be a check on the parsed result. Its `read_line` replaced our
  `input.lines()`, so an eight-megabyte line is refused without being allocated, the rest of that frame is
  discarded by a loop that never holds it, and the session survives rather than dying on one bad frame.
- **The session is `craft-mcp`.** Version negotiation (newest `2025-03-26`, and `2024-11-05` echoed back to a
  client that asks for it, which is what "supported versions" has to mean), the `initialize`/`initialized`
  handshake, the tools envelope and the text result. Our hand-written `initialize` — with a hard-coded
  `2024-11-05` and a session state that was really just "has anyone said hello" — is gone. Tool *routing* stays
  here, straight from the command registry.
- **Two behaviours changed, deliberately, because the protocol says so**: input that is not JSON is `-32700
  Parse error` rather than `-32600`, and `tools/call` with non-object arguments is `-32602` rather than an empty
  call. Both are named in the test that asserts them and in the ROADMAP row, not quietly changed.
- **`craft-ipc` is documented as absent, not ticked.** There is no second process: the window owns the engine
  and the write lock, and the CLI opens the library itself, which is what makes that lock meaningful. The
  ROADMAP row now says what would justify a socket (a running-window consumer: a menu-bar helper, a browser
  extension, two windows on one library) and what it would need first (a server mode, authentication on a
  multi-user machine, and an answer for "the app is not running").
- Tests: negotiation (an older version echoed, an unknown one answered with the newest), the handshake
  (`is_initialized` after the notification, and no reply to it), a batch answered as a batch with the
  notification skipped and an over-bound batch refused whole, an oversized frame refused with a message that
  names the bound while the *next* frame is answered, and non-object arguments refused. 16 tests in the crate.
- 479 tests; ci (exit 0), deny, snapshot, attribution green. **Not verified by hand**: against a real MCP
  client — the handshake is exercised in-process, but no client has ever connected to this server.

## 2026-11-21 — M2 row: HEIC, routed by header and measured from its container

- **The decoder is the suite's**: `craft-heif` (pure Rust `heic-rs`, every call inside `catch_unwind`, colour
  converted to match libheif instead of assuming BT.709). It decodes the primary image and prefers the ~320 ×
  240 thumbnail an iPhone already stores when that is big enough for the request, which is the difference
  between a grid filling and a grid waiting.
- **16-bit samples are divided by 257, not shifted by 8.** A shift maps everything in 65535..=65280 to 255 and
  throws away the top of a 10-bit HDR photo's range; 257 maps 65535 to 255 and 0 to 0 exactly.
- **Routing is by header** (`sniff` → `Magic::Heif`), so an iPhone photo renamed `.jpg` decodes, and a `.heic`
  that is really a JPEG goes to the JPEG decoder. Proved to bite: disabling the branch makes the renamed case
  fail with "not a JPEG stream".
- **Sizes come from the container** in the scan path (`ispe`/`irot`/`clap` via `craft_heif::probe`), because the
  container is what the decoder will use: a portrait iPhone photo is 64 × 96 in the justified grid and 1.4 MP
  in insights rather than 0 × 0 with a 3:2 guess. `heif_size` returns `None` for every non-HEIF file, and the
  container's answer wins outright rather than being merged per axis with a stale EXIF tag.
- **No binary fixture was committed**: the tests build synthetic HEIF files with the decoder's own writer — a
  grey ramp that must still be a ramp after the container parse, the HEVC stage, colour conversion and resize;
  a rotated file whose displayed size must swap axes; the same bytes under a `.jpg` name; and a truncated file
  refused with a reason rather than a panic or a blank cell.
- **A flaky test was fixed, not retried**: the priority test that expected the queue to be full assumed a worker
  had not already taken the first request. The admission decision (push, eviction, in-flight bookkeeping) is now
  a pure `admit()` and is tested without threads; the timing-dependent service test is gone.
- 482 tests; ci (exit 0), deny, docs, attribution green. **Not verified by hand**: a real iPhone HEIC —
  the decoder is exercised on synthetic files it was designed against, and performance is unmeasured (the perf
  fixtures are JPEG, and `docs/ROADMAP.md` says so).

## 2026-11-21 — M2 row: a video's length, rate and codec, from its container

- **Metadata first, pixels later, and said so**: the row is split. Part 1 (this commit) reads `moov` through
  `craft-media`'s isobmff demuxer during the scan — length from the video track's own timescale, frame rate as
  **sample count over duration** (29.97 reads 29.97, a variable-rate clip reads its average), size from the
  track or the sample entry, codec from `CodecId::name`. Part 2 (the poster) is a ROADMAP row with its blocker
  named: `craft-video`'s decoder **plus** a YUV→RGB conversion honouring `ColorTags`, which nothing in the suite
  provides because a wrong matrix is a colour bug that looks like a decode bug.
- `Photo` gained `duration_ms`, `fps`, `video_codec` (all `#[serde(default)]`, so an older log still replays),
  `PhotoMeta` carries them through `apply_meta`, and `format_duration` lives once in the catalog so the cell,
  the inspector and the screen reader cannot disagree: a duration pill on video cells, "1:12" in the spoken
  description after the cull facts, and a length row in the compare facts.
- **Boundaries it refuses to fudge**: a file that is not a container, or a container whose video track declares
  no length, answers `None` rather than a zero-length clip; MKV/AVI are not probed because their container
  features are off, and reporting them with no length would be worse than reporting nothing.
- **Fixture without a blob**: the test muxes a real MP4 with the suite's own writer — 24 samples of 25 ticks at
  600 Hz is one second at 24 fps — and asserts duration, size, codec and rate, then truncates the same bytes and
  asserts the probe refuses half a file, and points it at a JPEG and asserts it is not a video.
- Goldens changed by exactly the three new JSON keys plus the log byte counts; the diff was read before the
  update, not accepted blind.
- 484 tests; ci (exit 0), deny, docs, snapshot, attribution green. **Not verified by hand**: a real camera
  clip — the fixture is muxed by the same suite, so it exercises the demuxer, not a phone's peculiar `moov`.

## 2026-11-21 — M2 row: blur-up from a tier that already exists

- **The finding first**: a "blur tier" would not be cheaper. `craft-jpeg` reconstructs at full resolution and our
  `resize` reduces afterwards, so decoding a 32-pixel tier costs exactly what the 128-pixel grid tier costs —
  paying twice for the same work to show a softer picture. So the blur-up paints a tier that is **already
  resident**: the detail view, the compare panes and the cull stage ask `sharp_or_blur`, get the 128 grid tier
  scaled while the 1600–2000 decode runs, and it sharpens in place when that lands.
- `blur_tier` is pure: exact size, else the largest below, else the smallest above (painting a bigger picture
  into a smaller hole is sharp, merely wasteful), never another photo's tier, and **`None` when nothing is
  resident** — an empty rectangle is not a blur, so an undecoded cell keeps its honest placeholder rather than a
  smear that pretends to be the photograph.
- Tests: the tier table over every case; an end-to-end run where a real JPEG is decoded by the real service at
  128 and `sharp_or_blur(1600)` returns that texture and its tier instead of waiting; and a headless paint of the
  detail view before its decode lands (which caught a `mut` clippy would have failed on).
- One test lesson: the first version of the end-to-end test asserted "nothing resident yet" on the same
  thumbnail set it then waited on, making the later assertion a race against a worker. The empty-map case now
  uses its own instance, so nothing depends on timing.
- `docs/UX.md` said "skeleton shimmer"; the code has never shimmered, and now the row says what it actually does.
- 487 tests; ci (exit 0), deny, snapshot, docs green. **Not verified by hand**: whether a 128 tier scaled to
  a 1600-pixel pane reads as "soft photograph" or as "broken" on a real screen — that is a judgement only a
  person looking at it can make.

## 2026-11-22 — M3 row: apply the last keyword (SL-D05 part 2)

- `photo.tagFromPrev`: type a keyword once, then one key per photo. In the registry (`T`, Organise), routed
  through the engine, applied from the UI key handler.
- **What "last" means, decided rather than assumed**: the last keyword this session **added**, one of them,
  in memory only. Not persisted — a keyword applied last week is not "the last keyword", and silently pasting
  somebody else's older tags onto a photo is a wrong write rather than a convenience. Removing a keyword does
  not become "the last" one either (`engine.last_tag()` is set on adds only, and a test asserts the removal
  leaves it in place).
- It goes through `set_tag`, so a keyword renamed since still resolves to its current name, a photo that
  already carries it is not written twice (second press → 0 changes), and the whole batch is one undoable op.
- With nothing remembered the engine refuses with a sentence rather than a silent no-op, and the UI raises it
  as a *problem* notice; the success notice names the keyword, because "Tagged 5 photos" leaves you guessing
  what you just wrote.
- **Two gates bit, both correctly.** The published golden `24 shortcuts.txt` changed by exactly the one new
  binding (18 bound commands, `T  Organise  Apply Last Keyword`) — read before committing. And
  `no_ready_command_is_left_without_an_answer_about_its_arguments` failed for the new tool until its MCP
  argument schema existed, which is the gate doing its job; the example now states that there is no `tag`
  argument, which is the point of the command.
- Tests: engine (refusal with nothing remembered, add/remove ordering, batch apply over 3 photos, idempotent
  second press), UI (real library, real `Event::Key` for `T` through the real handler: nothing written and the
  reason in the notice, then the keyword landing on every selected photo with the notice naming it), MCP
  example. 489 tests; ci (exit 0), deny, snapshot, docs green.
- **Not verified by hand**: the notice's tone on screen, and whether `T` collides with any OS-level shortcut on
  a real keyboard layout (the keymap gate only proves the registry's internal uniqueness).

## 2026-11-22 — M2 row: stale cached thumbnails, and the wake-up nobody rang

- **The row**: a thumbnail decoded before the drive went away is still in the cache and was still drawn at full
  strength, so an offline library looked current. Missing photos' cached pixels are now painted at
  `CACHED_ALPHA = 110/255` — visibly present, visibly not live — in the grid cell, the detail view, the compare
  panes and the cull stage (a cached full-size preview of a missing file is still a cached preview). The `?`
  badge's hover now distinguishes the two offline states: "showing the cached thumbnail from before it went
  away" versus the plain missing-file text. Nothing is marked when nothing is cached — a grey rectangle is not
  stale, it is absent.
- **The bug this found is bigger than the row.** The M2 priority scheduler replaced a channel with a `Condvar`
  queue and never notified it. Workers parked as soon as the queue emptied, so every thumbnail request after the
  first quiet moment — which in the real app is most of them — waited for the life of the process. The tests all
  missed it because a worker that had not yet reached its first `wait` finds the entry by itself, so requesting
  immediately after startup worked; the first test that slept before requesting failed with
  `queued 1, dropped 0` and a four-second silence. `request_with` now calls `notify_one` on an accepted push,
  and `a_request_after_the_workers_have_parked_is_still_decoded` sleeps 250 ms before asking — **proved to
  bite** by removing the notify: it then fails, waiting out its ten-second deadline.
- Tests: the tint/hint table, the wake-up regression, and an end-to-end paint (decode a thumbnail from a real
  JPEG, delete the file, rescan, paint) asserting the image mesh's vertex alpha falls 255 → 110 while the
  default view — which hides missing files — paints nothing at all. 492 tests; ci (exit 0), deny,
  snapshot, docs green.
- **Not verified by hand**: whether 110 is the right fade on a real screen, and whether the real app's first
  paint after a long idle now recovers — the regression test proves the mechanism, not the app's timing.

## 2026-11-22 (second) — a refusal that named the wrong fact

- The previous commit's `ci` came back 1 on the same tree that then returned 0, so this run hunted the flake
  instead of starting new work. **Found it**: `applying_move_moves_the_file_and_a_changed_file_refuses_the_batch`
  restores the bytes after an edit but not the timestamp, so whether the batch was refused depended on whether
  the rewrite landed inside a new clock second. The test now restores the mtime with `File::set_times`.
- Chasing it found a real defect: the safety audit checks size **and** mtime, but both answered
  `Refusal::Changed`, which prints byte counts — so an edited file of the same length was refused with
  "changed since it was indexed (1 → 1 bytes)". A refusal that reads as a bug in the app is worse than no
  refusal. New `Refusal::Modified { path, indexed, on_disk }` with its own message, and
  `a_file_whose_timestamp_moved_is_refused_and_says_so` pins it and asserts the message never dresses a
  timestamp change up as a size change.
- 493 tests; ten consecutive runs of the previously flaky test with `--test-threads=8`, and three full-suite
  runs with `--test-threads=16`, all green; ci, deny, snapshot, docs green.
- **Not verified by hand**: nothing new; the fade from the previous commit is still un-eyeballed.

## 2026-11-22 (third) — M9 row: `--json` for the write verbs, with a written contract

- The CLI's JSON table is now a table of **schemas** — verb, engine command, read/write, and a one-sentence
  promise for the object — with thirteen mutating verbs added (`rate`, `flag`, `label`, `tag`, `untag`, `date`,
  `rename`, `alias`, `accept`, `pack`, `undo`, `retry`, `remap`). `--json schemas` prints the table, so a script
  discovers the contract instead of reading source.
- `docs/CLI.md` states the promise (additive only within a major version; absent ≠ null; counts are photos
  unless noted; a refusal is a non-zero exit with stderr, never an error object that looks like a result;
  progress verbs like `export` deliberately have no schema because a job with a cancel is not one object). The
  README links it, and `cargo xtask docs` enforces that reachability.
- **The contract found a lie, which is the reason to write one.** The new goldens showed `rate 1,2 4` answering
  `{"changed": 2}` for photos that already had four stars: `rate`, `flag`, `label` and `setCaptured` counted the
  *request*. They now compare against the catalog first, so a repeat answers `0`. `a_repeat_write_reports_no_change`
  pins it, and the goldens show `2` then `0`.
- Gate: `every_write_verb_has_a_json_schema_and_a_stated_contract` — every mutating verb must answer, every note
  must be a sentence, and the table must mark writes as writes. Proved to bite by renaming `pack` in the table.
- A self-inflicted mess worth recording: replacing a block of the golden step list by index silently swallowed
  four existing steps (`keywords as JSON`, the old `no schema says so`, `diagnostics`, `version`). The golden
  harness said so — "no step records this golden any more" — which is exactly why it tracks orphans; the steps
  were restored, and the four goldens are unchanged.
- 495 tests; ci (exit 0), deny, snapshot, docs green. **Not verified by hand**: nothing new on screen.

## 2026-11-22 (fourth) — M4 row: the decision overlay, in two phases, with an off switch

- **SL-C04 done**: the cull decision overlay is painted inside the image's own bounds (clipped to the image
  rect, not the canvas — a label outside the photograph reads as window chrome), and it now has the two phases
  the design asked for. `overlay_phase(elapsed_ms, motion)` is pure: the label fades in over 90 ms with the
  photograph still in place, then the photograph slides out over 140 ms while the next one is already behind it.
- The overlay keeps the **judged** photo's id, because `cull_decide` has already advanced the session — sliding
  out "the photo that just left" is only possible if you remember it. It clears itself when the slide ends and
  requests a repaint while running, so it cannot freeze at the phase whatever input happened to cause.
- `Motion::{Full, Reduced}` is a real persisted setting (`appearance.motion`, unknown value → full), a chip in
  the top bar next to density and layout (it is the setting you turn on *while* the animation is bothering
  you), and the phase function honours it: reduced draws the label at once, cuts the slide to zero, and still
  holds the label for its 90 ms because the confirmation is information rather than decoration.
- Docs corrected while here: `docs/UX.md` claimed reduced motion "respects OS setting + an in-app toggle" —
  only the toggle is true, and the OS half is now named in M10's deliberately-absent table with its unblocker
  (egui exposes no cross-platform reduce-motion signal). The same table's reduced-motion cell was rewritten
  rather than left saying the toggle does not exist.
- Tests: the phase table over every case (monotonic, both settings, overshoot-free), the settings round-trip
  with an unknown value and a settings file predating the key, and a headless cull view where a decision paints
  an overlay over the stage and then drops it. 498 tests; ci (exit 0), deny, snapshot, docs green.
- **Not verified by hand**: the timings. 90 ms and 140 ms are reasoned from "one glance" and "reads as
  movement", not measured against an eye, and the fade-plus-slide on a real display is exactly the kind of
  thing this harness cannot judge.

## 2026-11-22 (fifth) — M5 row part 1: ticking collections, and batch changes that undo as one

- The sidebar's collection list can now be worked on in bulk: click ticks, ⌘-click toggles, ⇧-click takes the
  run in **sidebar order**, a ticked row shows a dot distinct from the filter highlight, and a line appears with
  the count and ✕ when more than one is ticked. The context menu acts on the whole ticked set when the row is
  part of it and on that row alone otherwise — right-clicking an unrelated row must not recolour four others.
  Rename and delete stay single in the multi case (renaming four rows to one name merges them); a batch delete
  is offered explicitly with the count.
- Ticking lives in its own `CollectionSelection` (anchor + ticks, no primary) rather than reusing the photo
  `Selection`: the ids are different kinds of thing, and a set that could be passed where photos were expected
  is a bug waiting for a refactor. It is pruned each frame against the engine, so a delete — or an undo —
  cannot leave a ghost ticked.
- **The engine grew batch methods rather than a loop in the UI.** `set_collections_colour`,
  `set_collections_favourite` and `delete_collections` each commit one `Op::Group`, so five recoloured
  collections undo in one ⌘Z instead of five; they count only what changed, like every other write; and the
  registry commands now accept `collections: [1,2]` as well as a single `id`.
- Tests: the gesture table (replace/toggle/range/widen/narrow/backwards/no anchor/prune/clear), the engine
  batch (one undo restores all three; a *second* undo reaches the underlying collection creation, which is how
  we know the group went in as one entry, and a batch delete leaves the photos alone), and a headless sidebar
  paint with two ticked.
- The ROADMAP row is split honestly: this is part 1; **folders** having favourites/colours/search/sort is
  part 2 and is now its own row with its reason (the folder section is a projection of roots and subfolders, so
  each of the four needs somewhere to live) instead of an empty box.
- 501 tests; ci (exit 0), deny, snapshot, docs green. **Not verified by hand**: how the tick dot reads
  next to the colour dot at real size.

## 2026-11-22 (sixth) — SL-A11 part 1: `PathKey`, and folders that remember a pin and a colour

- **`PathKey` exists**, and the row that blocked it was blocking on a dependency decision that had already been
  made elsewhere in the tree: `unicode-normalization` was a transitive dependency, so the tables were present.
  It is now a declared, attributed direct dependency and `crates/core/src/pathkey.rs` normalises NFC/NFD and
  trailing separators unconditionally, and resolves **case by asking the disk**: per component, a
  case-insensitive lookup in the parent, adopting the on-disk spelling only when exactly one entry matches. Two
  matches means the volume can hold both, so nothing is folded and `Ångström.jpg`/`ångström.jpg` stay distinct —
  the silent merge the row was written to prevent. No per-volume probe: the disk answers directly.
- Tests hold the key to whatever volume the test runs on (`case_is_resolved_by_the_disk_and_never_folded` asks
  the volume whether it distinguishes case, then asserts *that* answer), rather than to a guess about the
  platform. NFD/NFC collapse, separators collapse, every component is resolved, absent paths keep what was typed.
- **Folders in the sidebar now carry a mark**: star + colour dot, set from the row's context menu, stored as
  `Op::SetFolderMark` which records the *whole* mark so one ⌘Z clears pin and colour together, and an empty mark
  removes the entry both when applied and when replayed. New `Op`, `crates/catalog/src/folder.rs`, catalog
  accessors, engine methods (`set_folder_mark` computes the key itself so a UI and a CLI cannot make two marks
  for one folder), registry commands `folder.mark`/`folder.marks`, MCP argument schemas, and two `--json` schemas.
- `left_panel`'s closure crossed the cognitive-complexity gate, so the folder list became `folders_section` —
  splitting because the gate said so, not silencing it.
- Attribution gate bit as designed: adding `unicode-normalization` failed with "linked but docs/ATTRIBUTION.md
  does not attribute it" before the row was added.
- 508 tests; ci (exit 0), deny, snapshot, docs green. **Not verified by hand**: how the star plus the
  colour dot read together on a real sidebar row, and whether resolving case per component is fast enough on a
  network volume — it is one `read_dir` per component, per mark change, not per frame.

## 2026-11-22 (seventh) — folders part 3: a folder remembers how you were browsing it

- The folder mark gained `sort` and `search`, which finishes the M5 folders row. **The decision the last run
  deferred**: a sort belongs to the folder when a folder is the scope, and to the library otherwise. A folder
  with nothing remembered follows the library's sort rather than snapping back to Natural — losing the sort you
  chose a moment ago is the more annoying failure, and Natural is not a sensible default for a library of RAWs.
- Search follows the collections rule exactly (committed on lost focus or Enter, never per keystroke — that
  comment was already there) and comes back into the search box on open rather than filtering behind your back.
- `folder_scope(mark, library_sort) -> (Sort, String)` is the rule as a pure function and the row's click runs
  exactly it, so the test asserts the code path rather than restating the values. Its first version *did*
  restate them; the extraction is what made the test worth having.
- `FolderMark` is no longer `Copy` (two `String`s), which the compiler then chased through the catalog, engine
  and UI: every `.copied()` became `.cloned()` and every partial update became `..mark.clone()`. Worth naming
  because it is the kind of change that silently loses a field if one of the struct literals had been rewritten
  by hand — `..mark.clone()` is what makes adding the fifth field safe next time.
- `folder.mark` gained `sort`/`search` (omitted keeps, null clears, unknown sort id refused), `folder.marks`
  reports both, and the CLI's `mark` verb passes `--sort`/`--search` through.
- 509 tests; ci (exit 0), deny, snapshot, docs green. **Not verified by hand**: whether restoring the
  search into the box on folder open reads as helpful or intrusive — it is the kind of judgement only use makes.

## 2026-11-22 (eighth) — `sortlens-quality` part 1: a sharpness score that can be trusted and re-derived

- `crates/quality` stopped being a scaffold. The score is the variance of the 4-neighbour Laplacian of the luma
  channel over the squared mean luma, measured on a box-filtered grid capped at 256 px on its long edge. Each
  clause is a decision with a reason in the crate docs: Laplacian not edge energy (a steep ramp is not detail),
  variance not a mean (one sharp edge must not carry a flat frame), divided by mean² (exposure invariance), at a
  fixed analysis edge (a photograph is as sharp as the photograph, not as the copy you hold).
- **`Score { value, version }` with `SCORE_VERSION` and one `needs_rescore` predicate.** A library half-measured
  by two formulas would sort wrongly with nothing looking wrong, so the version is checked rather than trusted,
  and a non-finite value counts as needing a re-measure.
- **A near-black frame is refused, not scored — and the proof is the finding.** Removing `MIN_MEAN_LUMA` makes a
  1–2 level noise frame score **1.78, above a sharp bar pattern's 1.41**: without the floor every night shot
  tops the sharpness list. "Cannot tell" is not "in focus".
- **Layer discipline caught by the docs, not by a gate.** The first version depended on `sortlens-thumbs` to take
  a `Thumbnail`; ARCHITECTURE says `quality` is pure math at L0 with no IO, and `xtask layers` passed anyway.
  The API now takes RGBA (or luma) planes and `thumbs` is a dev-dependency, so the runtime rule holds and the
  end-to-end test still measures real decoded pixels.
- **A stated limit, because the alternative is an overclaim:** the fixed edge makes copies related by a whole
  number ratio measure alike (proved, ratio < 1.05); a fractional reduction is averaged differently by the box
  filter and can shift the score. Correcting that means measuring at several edges — precision a ranking does not
  need. The first version of that test asserted 1024 vs 480 (a fractional ratio) and failed at 1.66×, which is
  how the limit was found.
- Twelve tests: blur separation and ordering, a flat frame scoring exactly zero, exposure invariance (twice — one
  hand-built, one fixed-seed property over 24 random mid-tone frames), size invariance plus the mechanism test
  that native-size measurement *does* differ, the darkness floor on both sides, degenerate buffers, versioning,
  JSON round-trip and a real JPEG decode. Two invariants were proved to bite by removing the normalisation and the
  floor, then restored.
- No property-testing dependency exists in the workspace and adding one for two invariants would cost a package
  and its attribution, so the property runs use a fixed-seed generator — reproducible on failure, which is the
  part a property test is actually for.
- 521 tests; ci (exit 0), deny, snapshot, docs, layers, roots, attribution green.
- **Not verified by hand**: none of it is reachable from the UI yet — the scan job is part 2, so there is no
  screen to look at and no threshold to judge. The threshold deliberately does not live in this crate.

## 2026-11-22 (ninth) — blur scan part 2: scores stored, as one undoable pass

- `Op::Scores { changes }` carries a whole measurement batch, so a 5 000-photo pass is **one log entry and one ⌘Z**,
  and its inverse is the previous scores — a re-measure at a new `SCORE_VERSION` undoes back to the old numbers.
- **The score lives on the `Photo` record, and the test found why that matters.** A scan that replaces a record
  because the bytes changed used to clone the previous one and keep every annotation; with `sharpness` on the record
  that meant a changed file kept a score about pixels that were gone. Stars survive a re-encode; a sharpness score
  does not, because it is a statement about pixels, so the update path now clears it. The first version of the test
  asserted exactly this and failed, which is the point of writing it before believing the design.
- `Engine::begin_sharpness` queues only what `sortlens_quality::needs_rescore` says needs it (never measured, another
  version, or not a number), decodes at `LARGE` on a worker, and `poll_sharpness` reports
  `{ measured, unjudged, remaining }`. **`unjudged` is first-class**: an undecodable file and a near-black frame are
  counted and left unscored, never stored as a number. `at_least_one_measurement_survived_the_undo_above` and
  `a_file_that_cannot_be_decoded_is_counted_rather_than_scored` pin the two halves of that.
- A cancelled pass still commits what it measured, because each score is true on its own — unlike a cancelled scan,
  which commits nothing because a half-scanned folder describes a disk that is not there. Recorded in the struct doc.
- `photo.sharpness` / `photo.sharpnessProgress` in the registry, MCP examples, and a CLI `sharpen` pass (goldens 50/51:
  six scores stored, then "every photo has a current score").
- **`sharpen` deliberately has no `--json` schema, like `export`, and the gate now proves it refuses.** The first
  version *did* have a schema, and the golden exposed the bug: `--json` routes to a single `engine.run` call, so it
  answered `{"queued":6}` and stored nothing. A pass has no single object to return, so it is a named progress verb,
  and `every_write_verb_has_a_json_schema_and_a_stated_contract` now requires each named progress verb to refuse.
- The complexity gate fired on `Catalog::apply` at 31/30 and the arm became `apply_scores` — split because the gate
  said so, not silenced.
- 529 tests; ci (exit 0), deny, snapshot, docs, layers, roots, attribution green.
- **Not verified by hand, and not reachable from the window yet**: the UI does not start the pass or show a score.
  That is part 3, and there is nothing to look at until it exists.

## 2026-11-22 (tenth) — blur scan part 3: the review surface, and a cut that says what it can claim

- **M6's blur row is now closed.** The section lives in Insights — the analysis view that already has the charts —
  with a "Measure sharpness" button (shown with how many photos have no current score), a histogram, a slider for
  how much of the library to review, and a button that ticks the selection and returns to the library.
- `crates/ui/src/sharpness.rs` is the arithmetic with **no egui at all**: `review(scores, percentile) -> Review`
  returns the histogram buckets, the cut, the ids at or below it, the tie count and `selected_bands`. Deciding which
  photos get acted on belongs in a test, not only in a painter.
- **The cut is a value at a percentile, not a rank.** Taking "the lowest N" would pick an arbitrary subset of a tie —
  the same photograph selected or not depending on its id. A value cut can select more than the percentage asked
  for, so `ties` is returned and the count is shown: a bigger selection honestly described beats a smaller one
  chosen at random.
- **Identical scores select nothing** (`separable: false`): a relative cut on a flat library would select every
  photo, and selecting everything is an action, not a review. The label says a relative cut cannot separate them.
- **The band the cut sits in is computed once.** The first version of the painting used a helper that returned
  `true` — a placeholder that would have highlighted the whole histogram. It was deleted and replaced by
  `Review::selected_bands`, with a test that `band_selected(n - 1)` is true and `band_selected(n)` is false, so the
  painter and the button cannot disagree about where the cut is.
- The pass is polled in the view that asked for it (`insights_view`), and the report becomes a notice that names how
  many could not be judged — an unmeasured frame never reads as a measured one.
- `Selection::select_many` caps at `MAX_RANGE`, the same cap a range-select has: a computed selection of a hundred
  thousand ids is a batch waiting to run on a library nobody meant to change.
- The slider is session state, not a persisted setting: remembering yesterday's cut would make today's review appear
  to make a claim it never made. Written into `docs/UX.md` §5.8, which previously described a sortable grid that
  does not exist.
- 538 tests; ci (exit 0), deny, snapshot, a11y, docs, layers green. ROADMAP's stale "blur ladder and cluster
  determinism wait for their features" row was split: the blur ladder is checked, cluster determinism waits for geo.
- **Not verified by hand, and named as such in ROADMAP**: whether 10% is a sensible default, and whether the
  histogram is legible at real score ranges. Both need a library of actual photographs, not fixtures.

## 2026-11-22 (eleventh) — similar leftovers: a selection that is what it says, and one delete executor

- **A real bug in shipped code**: "Select all but the best" cleared the selection, selected **one** id, and announced
  "{n} set aside". A button that names a set and delivers a different one is exactly the quiet lie a batch delete is
  dangerous with. It now selects `group.others()` through `Selection::select_many`, and the group header names the
  keeper so the set is chosen with it in view.
- A batch bar: "Select leftovers in every group (N)" and "Delete the N selected…", the second opening a confirmation
  with the count and the three modes.
- **The row needed a delete path that did not exist outside a cull session, and the answer was a refactor, not a
  second path.** `Engine::apply_candidates` now holds the audit, apply, batch record, flags and job; `apply_session`
  keeps only the session bookkeeping; `delete_photos(ids, mode)` builds a candidate set and calls the same function.
  The alternative — a delete that quietly skipped the audit — is the failure this codebase exists to avoid.
- `photo.delete` in the registry, `delete_mode_from` shared with `cull.apply`, CLI `delete <ids> --mode … --target F`
  with a `--json` schema, MCP example. The golden uses `--mode tag` so a test never touches the real Trash.
- **A test-only constraint worth naming**: `Mode::Trash` is not exercised anywhere, because it hands the file to the
  operating system's trash and a test that does that damages the machine it runs on. My first attempt did exactly
  that and took **240 seconds** before failing (the macOS trash path under a temp volume). The batch/audit/undo
  machinery is covered through `Mode::Move`, and the gap is written into ROADMAP rather than papered over.
- The audit also refused my first test for a *second* reason worth remembering: a rejects folder inside a watched
  root is refused outright, because rejects there would reappear on the next scan. The test now puts it outside.
- 542 tests; ci (exit 0), deny, snapshot, a11y, docs, layers, attribution green.

## 2026-11-22 (twelfth) — `sortlens-geo` part 1: projection, deterministic clusters, an offline gazetteer

- `crates/geo` stopped being a scaffold and holds three things, all pure: `LatLon` (the only way to hold a coordinate,
  refusing non-finite numbers, latitudes off the globe and longitudes outside ±180), Web Mercator `project`/
  `unproject`, a 64-pixel grid `cluster`, and a `Gazetteer` parsed from a tab-separated file the user supplies.
- **Mercator has no pole, so a polar photograph is refused, not clamped.** Clamping would draw it at the top edge as
  if it were taken at 85°N — a wrong claim about a real photograph rather than a missing dot. `MERCATOR_LIMIT` is the
  tile scheme's own latitude, and every returned point is inside the world square so callers never check.
- **Clustering determinism is now a fact rather than an aspiration**: cells are sorted by count then cell, and a test
  feeds the same forty photos in two orders and asserts the same dots in the same order. That closes the ROADMAP row
  which had been waiting for the feature.
- The gazetteer ships **no data and makes no network call**: it parses `name<TAB>lat<TAB>lon[<TAB>country]`, counts
  malformed lines instead of dropping them silently, indexes by one-degree cell, and breaks ties by name so file order
  cannot decide the answer.
- Two test bugs of my own, both worth the note: a coordinate at 89°N is *valid* and *unmappable* (the test asserted the
  wrong one), and a "near Brighton" lookup 54 km away with a 30 km radius correctly returned nothing — the test was
  wrong, not the code.
- Found while checking: **`sortlens-mcp`'s example harness broke on my new `photo.delete` example**, which really
  deleted the photos the later examples needed (and after 112 s, because `mode: trash` goes through the OS trash).
  The example is now `mode: "tag"`, moves nothing, and says in a comment why — the same hazard as the golden step.
- **The blocking dependency for part 2 is named, not discovered later**: `sortlens-meta` parses no GPS at all and
  `Photo` has no coordinate, so there is nothing to cluster until EXIF GPS is read and stored. The next run is
  therefore data first — parse GPS with its hemisphere reference, put `lat`/`lon` on the record, prove it survives a
  snapshot and a rescan — and only then paint.
- 553 tests; ci (exit 0), deny, snapshot, docs, layers green.

## 2026-11-22 (thirteenth) — Places part 2: GPS in the catalog, and a view that admits what it lacks

- `sortlens-meta` reads the camera's GPS into a `sortlens_geo::LatLon`; `Photo` carries it, so a scan fills it in and a
  re-read replaces it (a successive read with no GPS clears it rather than keeping a stale dot). End-to-end test uses
  a JPEG whose EXIF APP1 block is written by the same library that reads it — JPEG → EXIF → DMS → coordinate, with the
  southern hemisphere's sign intact. `Testkit::jpeg_with_location` makes that fixture available everywhere.
- **The decision worth keeping**: a device with no fix writes 0, 0, and 0/0 is a real coordinate in the Gulf of
  Guinea. Treating it as real would put every pre-lock frame in one dot in the Atlantic — a data disaster rather than
  a missing tag — so an exact 0, 0 is "no location". Cost of being wrong: a photo genuinely taken at Null Island is
  left off the map. Cost of the other choice: the map lies about hundreds. Stated in `location_of`'s doc.
- `LatLon` now deserialises **through its constructor** (`#[serde(try_from)]`), so a corrupt snapshot is a load error
  instead of a photograph drawn in the wrong ocean.
- `Engine::geotagged`/`located`/`places(zoom)` and a Places view that reports counts and the biggest clusters, and
  says in the view that the coastline is next. **No dots on a blank rectangle**: a field of dots with no coastline
  looks like a map and tells you nothing about where anything is.
- **Two stale nav gates deleted**: `Cull` still said "M4" and `Places` "M5" in the bottom bar, so the nav closed
  working views behind an out-of-date label. `View::milestone` now returns `None` for everything, `planned_view` is
  gone with its last caller, and a partly built view says so inside itself where the person looking will read it.
- A grid-clustering limitation is now a decision in the record, not a surprise: two photos a few hundred metres apart
  that straddle a cell edge are two dots. My first engine test asserted one dot and failed — the code was right.
- 560 tests; ci (exit 0), deny, snapshot, a11y, docs, layers, attribution green.

## 2026-11-22 (fourteenth) — Places part 3a: the basemap decision, and a credit that cannot be left out

- **Decision, made rather than deferred again**: the basemap is a **folder of tiles the person supplies** —
  `{z}/{x}/{y}.png` beside the catalog, with `attribution.txt` in the folder. Not a bundled vector basemap (map data the
  project does not have, and a licence it cannot clean-room), not MBTiles (a SQLite reader in the tree for one
  format). The layout is exactly what unzipping any tile archive gives, so the tiles somebody may legally use are the
  tiles that work.
- `sortlens_geo::tiles::Tileset::open` **refuses a folder with no credit**, naming the file to write and saying why:
  an unlabelled map implies the application made it. The refusal is at the type level — no state exists in which the
  view holds a usable tileset whose attribution is unknown — which is stronger than a comment asking the view to
  remember. A folder with a credit and no tiles is refused too.
- `tile_path` checks presence once per request (`None` for a hole) so a partial set paints around its gaps instead of
  failing an open every repaint. `zoom_range`/`has_zoom`/`zooms` read the directory names; `tile_count` answers "is
  this set partial".
- **The zoom bound came from a failing test**: I asserted a directory named `255` is not a zoom level and was wrong —
  `255` parses as a `u8`. `MAX_TILE_ZOOM = 30` now refuses it, because a view offering zoom 255 offers a billion
  billion tiles that are not there. The test stayed as written; the code moved.
- `Engine::tileset()`/`tiles_dir()` — the folder is **part of the library**, not a global setting, so two libraries
  can have different basemaps and moving a library brings its map. The Places view now reports the range, the tile
  count and the credit, or the sentence for whichever state the library is in, or the refusal.
- 564 tests; ci, deny, snapshot, a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: any real tile set, and whether the "no basemap" copy reads well at the view's width. The
  painted words are **not** asserted — egui returns paint shapes, not laid-out text — so the smoke test proves the
  view paints in all three states and that the engine reports the right state; the credit guarantee is enforced by
  `Tileset::open`, one layer down, where it can actually bite.

## 2026-11-22 (fifteenth) — Places part 3b: the tiles under the dots, and a dot that selects

- `sortlens_geo`: `viewport_origin`, `screen_position`, `visible_tiles`, `tile_rect` — the viewport maths as pure
  functions, so the UI holds no projection of its own. **The world does not wrap**: a repeated antimeridian would draw
  the same photographs twice with nothing to say which was which, so columns clamp to the world square.
- `crates/ui/src/places.rs`: `dot_radius` (square root, so area tracks the count), `dot_at` (**largest** dot within
  reach wins — where a big dot overhangs a small one the person is aiming at the big one), `clamp_zoom` (past the top
  of a set the tiles stop and the dots keep going), and `TileCache` (decode once; a failed tile is remembered as failed
  so one corrupt file is not re-decoded every frame; bounded at 512).
- **Two real bugs, both caught by tests written to expect correct behaviour:**
  1. The view skipped clusters it could not place and then indexed the cluster list **by position**, so one off-screen
     cluster shifted every dot onto its neighbour's photographs. `Dot` now carries its cluster index.
  2. The hit test compared the pointer in screen coordinates against dot positions **relative to the map's corner** —
     every click missed by the panel margin. `clicking_a_dot_selects_its_photos` drives real pointer events through
     `run_ui`, finds the map by the rectangle the view fills, presses and releases, and asserts the members are
     selected; without finding them it would have looked like it worked.
- The map centres on the **mean of the library's own photos**. Clicking a dot selects its members and says where they
  are; clicking the sea selects nothing and clears nothing. The credit is printed over the map every frame, and the map
  is a named `Role::Canvas` region — `cargo xtask a11y` demanded that, which is the gate earning its keep.
- `docs/UX.md` §5.9 rewritten: it described an offline *vector* basemap, thumbnail fans and a `geoBounds` library
  filter, none of which exist. The absent ones are now named with reasons instead of being described as if built.
- 572 tests; ci, deny, snapshot, a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: how it looks. The tests prove the tiles decode once, dots carry the right clusters, the
  click resolves and the credit is the folder's own; they cannot say the coastline sits in the right place, because
  there is no tile set in the repository to compare against. That needs a real archive and eyes.

## 2026-11-22 (sixteenth) — Places part 4: dots borrow a name from a gazetteer the person supplies

- `Gazetteer::load` reads a file with the size checked **before** the bytes (`MAX_GAZETTEER_BYTES` = 64 MB, enough for
  a four-million-place world list, small enough that pointing the app at a disk image is refused); `GazetteerError`
  carries a sentence naming the file. **A file with no places is not an error** — `is_empty`/`skipped` report it, and
  a line of tabs is blank rather than malformed. My test asserted two skipped lines where the code counted one; the
  code was right and the test changed.
- `Engine::places_file`/`load_gazetteer` hand back the path and a loader and deliberately do **not** cache: the thing
  that must never happen is a view parsing tens of megabytes to draw a frame.
- `places::Names` is the cache — size plus modification time as the stamp, re-read only when it changes. **A deleted
  file stops naming anything** (a place list outliving its file puts names on a map whose authority was deleted) and
  **a missing file is not a fault** (most libraries have none; a red line for that teaches people to ignore red lines).
  Found while wiring the footer: without that distinction the view reported "there is no gazetteer at …" in red for a
  library that simply had no places file.
- `LABEL_KM` = 30, `place_label` returns `None` rather than guessing, and `selection_notice` is one function with two
  forms so the named and unnamed cases cannot drift: "Selected 42 photo(s) near Brighton (50.82N 0.14W)" and
  "Selected 1 photo(s) at 25.00N 40.00W". Both state the coordinate, so a name is never mistaken for the file's
  resolution.
- A named dot prints its name beneath it; the footer says whether names are in use or why they are not.
- Deduplicated ROADMAP: the `sortlens-geo` part-1 block appeared twice, and the second copy was followed by a stale
  "part 2: the map view" bullet claiming GPS was not parsed — it has been since `f8f6a34`. Both removed.
- 576 tests; ci, deny, snapshot, a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: how a dense gazetteer looks — overlapping labels are the file's density problem and there
  is no declutter pass (named as absent in UX.md) — and how a real GeoNames export performs on load.

## 2026-11-22 (seventeenth) — Registry delete-everywhere: `tag.delete`, wired end to end

- `sortlens-catalog`: `delete_tag` — the keyword comes off every photo that carries it, and the whole
  rename trail dies with it (aliases whose chain resolves to the keyword are forgotten in the same
  undoable `Group`), so re-typing the old name cannot resurrect what was deleted. Naming an alias
  deletes the keyword it stands for, the way `set_alias` resolves names. Deleting a keyword nobody
  carries and no alias points at is **refused**, not silently "deleted" — a typo should not report
  success. Two catalog tests: the batch+one-undo case and the alias-chain case.
- `sortlens-engine`: `Engine::delete_tag` syncs the sidecars of every touched photo (so a deletion is
  not resurrected by the next file read) and records a `#tag deleted everywhere` job; `tag.delete` is
  in the command registry and the MCP examples, normalised like every other tag verb.
- `sortlens-ui`: sidebar keyword rows offer "Delete everywhere"; the handler (`delete_keyword_everywhere`,
  driven by the test, not a copy of it) prunes a keyword filter pointing at the deleted keyword and
  names the keyword and the count in the notice. Headless `run_ui` test paints the sidebar over a
  scanned library, deletes through the real handler, asserts the vocabulary is empty and one undo
  restores it.
- Collection delete already had its surface (`collection.delete` context menu, batch variant, tests) —
  verified, not rebuilt.
- **Windows environment fixes, needed to make the tree buildable at all here:** the project toolchain
  was missing, so 1.95.0 was installed into `.toolchain` and `scripts/env.sh` made platform-aware
  (Homebrew path only where it exists, project cargo bin first). Two checkout bugs that break the
  CI on Windows only: `core.autocrlf` checked the byte-compared goldens and `assets/icon.svg` out
  with CRLF — pinned LF for both in `.gitattributes`. And a real bug the macOS runs could not see:
  `PathKey` split paths on `/` only, so every `Path::join`-built Windows path (`\`) was resolved as
  one whole component and never case-resolved by the disk. It now splits on the platform separator
  (`std::path::is_separator`) and spells every key with `/`; a new test holds a joined path and a
  hand-spelled one to the same key.
- 580 tests; ci, snapshot, a11y, docs, layers, roots, attribution, deny green.
- **Not verified by hand**: the menu item on screen (a right-click menu cannot be driven headlessly
  here; the handler itself is tested) and the look of the hover text.

## 2026-11-22 (eighteenth) — Twenty steps of undo and redo, kept across a restart

- `sortlens-catalog`: the undo stack is bounded at `UNDO_DEPTH` = 20 and a `redo` stack walks the
  undone steps forward. A new commit abandons the redo branch — stepping forward again would
  replay a past that no longer matches what was just written. `redo()` is a real operation: it
  applies, logs with its inverse, and puts the inverse back on the undo stack, so a redo is
  itself undoable.
- **The history survives a restart, not just the last step**: the snapshot now carries
  `undo_history` (oldest first) next to the old single-level `undo` field, which is still written
  so an older build reads the snapshot it knows; a snapshot with only `undo` opens with that one
  step (a test rewinds the file to prove it). Log records replaying after the snapshot append
  their inverses to the stack, so a reopen keeps twenty steps, not one. Redo as a stack is
  per-session; after a restart the same steps stay reachable by undoing the undo entries the log
  holds in order — the "never a one-way door" property, now across the whole depth.
- `sortlens-engine`: `Engine::redo`, `edit.redo` in the registry and dispatch, and an MCP example.
  The example is published *before* `edit.undo` on purpose: the published examples run in order
  against one library, and the undo below it clears a decision the later examples need cleared —
  redoing it back handed `library.restoreBatch` a photo that was already rejected, and
  `every_published_example_actually_runs` caught it.
- `sortlens-ui`: `Ctrl+Shift+Z` in the keymap and the shell, notices "Redid the last undone
  change" / "Nothing to redo". Two orderings matter and only the headless test could catch them:
  egui's `consume_shortcut` matches with extra Shift and Alt **ignored**, so the shell must
  consume Ctrl+Shift+Z before Ctrl+Z — and the tuple's destructure is positional, so renaming the
  consumption order without renaming the bindings silently swaps the two. The test drives real key
  events through `run_ui` over a scanned library: undo steps the rating back, redo steps it
  forward, a second redo says so.
- ROADMAP M3's undo row ticked in the same commit, rewritten to say what is true now. README and
  PLAN §12 already reconciled (the 2026-11-21 pass); `cargo xtask docs` gates it.
- 586 tests; ci, deny, snapshot (two goldens updated: the shortcut list gained a row, the
  snapshot golden gained `undo_history`), a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: the keypress on a real keyboard (the key events are synthetic, though
  routed through the shell's own input handling) and how twenty-step notices read over a long
  session.

## 2026-11-22 (nineteenth) — Inline tagging in the cull stage, `T` re-applies there, and a deadlocked shell

- **SL-C11/C12, ROADMAP M4's last open row — M4 is now fully checked.** The cull stage gained an inline
  keyword strip under the action bar: chips for the keywords the photo on screen carries (click takes one
  off through `set_tag(.., false)`, one undoable op) and a field that adds one with Enter. Keywording now
  happens while the same photo is on screen instead of a round trip through the right panel. While the
  field is focused it owns the keys — `keyboard()` stands down for any focused widget — so P and T type
  letters, not decisions.
- `T` in a cull session re-applies the keyword you last added to the photo on screen (the grid's one-key
  repeat, aimed at the stage's own target), with the notice naming the keyword: "Tagged “backlit”" /
  "Already tagged “backlit”". Keymap gained a Context::Cull row for it; the empty-stage hint line mentions it.
- **A real latent deadlock the headless test exposed:** the top-bar search called `search.has_focus()`
  *inside* `ui.input(...)`. In this egui fork nested context access deadlocks the lock the closure runs
  under — meaning any Enter keypress anywhere in the app (the details-panel keyword field, the search box
  itself) could freeze the shell. `has_focus` is now read outside the closure; the search still commits
  when Enter is pressed while the box is focused. Found because the new test drives a real Enter for the
  inline field and hung for 10 s.
- Test details worth keeping: `egui::Button` in the fork has no `.id()`, so the chip is located the way a
  person sees it — by the text it paints ("backlit" bare; the toast repeats the keyword in quotes and the
  sidebar lists it too, both distinguished). And `keyboard()` runs after the view paints, so the strip
  catches up one frame after the key: the chip is asserted on the following frame.
- ROADMAP M4's inline-tagging row ticked in the same commit with a truthful description; UX.md §5.5 now
  names the strip and says `T` targets the photo on screen in cull.
- 587 tests; ci green.
- **Not verified by hand**: typing a long keyword over several photos on a real keyboard, and how the strip
  wraps when a photo carries many keywords (horizontal_wrapped lays it out; nobody has looked at it).

## 2026-11-22 (twentieth) — Phase 0: re-scope blockers, reconcile the docs (no feature code)

- **Course correction from the owner** (`docs/sortlens_agent_completion_directive.md`): "M0–M10 shipped" was
  wrong while core features sat in "Deliberately absent" tables. This entry is Phase 0 only; no Phase 1–5 code.
- `docs/ROADMAP.md`: new **Completion phases** section with Phases 0–5 as checkbox rows (Phase 0 ticked, the
  rest open). The M8 and M9 "Deliberately absent" tables became "Re-opened" pointers; M10's table lost the
  `.icns`/`.ico`, `craft-update` and high-contrast rows (now Phase 4) and keeps only what needs an identity, a
  toolchain, a screen reader or a decision. The two stale M1/M2/M3 rows point at their Phase 1 items. Status log
  gained a Phase 0 row and the old "shipped" row no longer says shipped.
- README badge/status and PLAN §12 reworded ("M0–M10 built, core completion in progress"); `cargo xtask docs`
  still computes `M0–M10` from the tracker, so the range string stays, the claim around it changed. The directive
  is linked from the README document table.
- `docs/RELEASE.md`: the "does not have" list rewritten from the facts — HEIC thumbnails and the tile-folder map
  exist, and PNG/TIFF/WebP *thumbnails* exist; what is missing is video posters, PNG export, models and the
  downloader, a bundled basemap, the remote, i18n and the keymap editor, installers, and the UI pass — each with
  its phase.
- **Dependency audit** (21 declared-but-unlinked pins, not 25: `craft-heif`, `craft-media`, `craft-jsonrpc` and
  `craft-mcp` had been linked since, and their attribution rows were still in the "not linked" table — moved to
  "Linked today"; the `craft-mcp`/`craft-jsonrpc` "hand-written" reasons were stale). Pruned from
  `[workspace.dependencies]` as having no consumer in Phases 1–5: `craft-flags`, `craft-fonts-build`,
  `craft-menu`, `craft-quit`, `craft-dialogs`, `craft-gainmap`, `craft-sniff`, `craft-zip`, `craft-crs`,
  `craft-webp`, `craft-ipc`, `chrono`, `log`. Kept, each tied to a phase in `docs/ATTRIBUTION.md`'s new
  "Linked by" column: `craft-video` (P1), `craft-atomic` and `rayon` (P2), `craft-keymap`, `craft-update`,
  `craft-i18n`, `craft-web-api`, `craft-web-host` (P4). `Cargo.lock` is unchanged (unlinked pins are not locked).
- Owner sign-offs recorded in ROADMAP: opt-in network only for `model.install` and `craft-update` (`crates/ai`
  stays offline), real permissively licensed weights and map data attributed in `docs/ATTRIBUTION.md`, licence
  `MIT OR Apache-2.0`.
- **Not verified by hand**: nothing visual changed.

## 2026-11-22 (twenty-first) — Phase 1 item 1: video poster frames in grid, detail and cull

- **SL-B11 part 2, ROADMAP M2's poster row ticked.** A video cell now shows its first frame, not a "VID"
  glyph. `sortlens-thumbs::poster::decode_video_poster` demuxes the clip straight from disk
  (`craft_media::FileReader` — a poster never reads a 4 GB clip whole), picks the first sync sample of the
  first video track (falling back to sample 0 for a track that marks none), and decodes it with
  `craft-video`'s H.264 (configured from `avcC`) or HEVC (from `hvcC`). Because a decoder releases the
  first picture only when a later access unit forces it out of its reorder buffer, the loop feeds at most
  eight samples before a flush; the bound keeps a broken GOP from decoding half a clip for one cell.
- **Colour conversion honours the stream's own colour tags** through `craft-color` (new workspace dep, same
  suite rev): matrix BT.709/601/2020 (SD-sized fallback when the stream is unlabelled), limited/full range,
  transfer (PQ and HLG decode to linear and re-encode to sRGB), and primaries gamut-mapped into sRGB for
  BT.2020 and P3D65 — same D65 white throughout, so one 3×3 each and no chromatic adaptation. The `tkhd`
  quarter-turn is applied after the resize, so a portrait phone clip shows portrait, matching the
  catalog's width and height, which already describe the displayed frame.
- **Routing is by header, not by name**, in `decode_file` — sniffed from the first 64 bytes before anything
  is read whole — so a clip renamed `.jpg` still gets its poster, a still named `.mp4` is still a still, and
  a raw (CR3 wears an ISO brand too) is still routed by name first. The poster flows through the ordinary
  `Service` → `Cache` path, so grid, detail view and cull stage all show it with no UI change, and the
  failure set falls back to the glyph pill for undecodable codecs (ProRes and friends refuse with a reason).
- Tests are **round-trips through the suite's own encoders and muxer** — no video blob committed: an H.264
  clip (Baseline, length-prefixed packets) whose first frame is a left-dark/right-light grey ramp and whose
  later frames are flat mid-grey, so a poster of the wrong frame reads flat and fails; the same through the
  HEVC encoder with a real `hvcC`; both routed through the ordinary `decode_file`; the rename cases; a
  truncated container and a non-container refused with reasons; and pure rotation tests (a 2×1 turned
  clockwise becomes 1×2 with the left pixel on top).
- `craft-video` moved from `docs/ATTRIBUTION.md`'s "Declared, not linked" table to "Linked today"
  (Phase 1's only unlinked dependency is now linked); `craft-color` joined it with its own row. ROADMAP's
  Phase 1 item 1 ticked in the same commit; RELEASE.md and README dropped "video poster frames" from the
  not-built lists. No UI crate changed, so no screenshot pass was needed.
- 593 tests; ci, snapshot, deny green.
- **Not verified by hand**: a real camera clip (phone HDR and a drone's) in the app; the fixtures are the
  suite's own encoder, which is honest about decode and colour but not about a manufacturer's bitstream
  quirks.

## 2026-11-23 (twenty-second) — Phase 1 item 2: stale-revision rejection, and the M3 test row closed

- **SL-D11, the optimistic-concurrency half.** Every `Photo` now carries `rev: u64` — the number of real
  annotation changes it has taken (rating, flag, label, keywords, capture time), bumped only when a value
  actually changes, so re-setting the same stars is not a revision. It travels in the log, the snapshot
  and every `AddPhoto`/`RemovePhoto` op, so a reopen reproduces the same numbers and undo raises it
  honestly (an undo also changes the record).
- **`Catalog::commit_expect(op, expected)`** refuses a write made against an older revision with
  `sortlens-core::Error::StaleRevision { expected, current }`, naming both numbers so a caller can
  re-read and retry. `Catalog::commit` stays the ordinary single-writer path; the guard is opt-in per
  write.
- **Every engine command response now carries `rev`** — the catalog sequence number the response
  describes — and **any command can pass `"expect": n`**, which is checked after the registry lookup
  and before anything runs: a stale caller is refused before it touches a single photo. This is the
  contract `docs/ARCHITECTURE.md` §4 already promised (`"rev": n` in every envelope); the code now
  keeps it, and the MCP examples file documents the `expect`/`rev` round trip for agents.
- **The M3 test row is closed.** Stale-revision tests: the catalog guard (stale refused with both
  revisions, current commits, nothing written on refusal), the per-photo `rev` (rises on real changes,
  not on no-ops, rises on undo, survives a reopen), and the engine round trip (`rev` on reads, `expect`
  accepted at the current revision and refused at a stale one, the newer stars not overwritten).
  Badge-contrast test: WCAG 2 relative-luminance contrast in `sortlens-ui::theme`, judging every
  preset's cell badges (offline dot, unreadable dot, white glyph on each) and flag dots at 3:1 against
  `thumb_bg` — the worst background a badge is ever painted over, because a badge exists exactly when
  there are no pixels. **It found a real defect**: Silver and Paper's `text_faint` (the offline badge
  dot) sat at 2.25 and 2.67 to 1 against their own thumb wells; both presets are darker now
  (Silver `0x969189` → `0x6f6a63`, Paper `0x8e8e88` → `0x6b655e`), which also lifts the faint text they
  carry elsewhere.
- ROADMAP's Phase 1 item 2 and both M3 rows ticked in the same commit; FEATURES.md's SL-D11 design note
  now says shipped. No layout or drawing logic changed, only two token values, so the headless run_ui
  suite and the pure token test cover it; the screenshot harness does not exist yet (Phase 3 item 9).
- 599 tests (6 new); ci, snapshot, deny green.

## 2026-11-23 (twenty-third) — Phase 1 item 3: the Recently Added source, honestly timed (SL-A14 part 2)

- **`Photo::added: Option<i64>`** — the instant Sortlens recorded a file entering the catalog, set by the
  scan the moment a new photo id is born and kept through every later update (a rescan that finds changed
  bytes re-indexes the file but does not reset its add instant, because the record did not re-enter the
  catalog). It travels in the log, the snapshot and every `AddPhoto` op, so a reopen reproduces the same
  number. **Never approximated from `mtime`** — that would answer "recently modified", a different and
  more confusing question.
- **The migration is the absence, faithfully carried.** The snapshot schema moved to 2; a schema-1 photo
  decodes with `added: None` and keeps it — no instant is invented, not even from the mtime sitting right
  there on the record. A `None` photo cannot answer the Recently Added question: the filter and the sort
  exclude it, and a resave writes schema 2 with the field still honestly absent. Tested against a
  hand-written schema-1 snapshot.
- **Catalog**: `Filter::added_after` (only photos recorded into the catalog at or after an instant) and
  `Sort::RecentlyAdded` (`recently-added`), newest record first, unknown sinks last, ties to the later id.
- **Engine**: `library.page` answers the question from the command layer (`filter: {"added_after": n}`,
  `sort: "recently-added"`), and the `added` value rides every photo envelope; the MCP examples document
  the call and the no-mtime guarantee.
- **Sidebar**: the `Recently Added` row joins Sources with its own count (the last 30 days, computed from
  the clock at draw time, not cached), clicking it replaces the whole question like every source does and
  asks it newest-record-first, and leaving it returns the user's sort. The window is stored as the
  *question* (`recently_added: bool`), so the cutoff is always the current 30 days even in a library left
  open past midnight.
- ROADMAP's Phase 1 item 3 and the M1 "Sources, part 2" row ticked in the same commit. UX.md §Sources
  already described the predicate ("indexed in the last 30 days") and now matches the shipped code.
- 604 tests (5 new); ci, snapshot, deny green. Screenshot harness does not exist yet (Phase 3A);
  no layout or drawing logic changed, only a new sidebar row painted through the existing `nav_row`, so
  the headless `run_ui` suite covers it.

## 2026-11-23 (twenty-fourth) — Phase 1 item 4: PNG export (SL-B24)

- **`Format::Png`** joins `sortlens-export`, so `Format::ALL` is `[Original, Jpeg, Png]` and the format id,
  CLI `--format png` and the dialog chip all exist. The export decodes through the ordinary pipeline
  (`sortlens_thumbs::decode_file`, falling back to `decode_image`), clamps the longest edge when asked, and
  re-encodes with `craft-image-codecs::encode` as PNG — the suite's own licence-approved encoder, so
  `cargo xtask deny` stays clean. **Lossless means lossless**: a source PNG exported to PNG reproduces
  every RGBA byte exactly, alpha included, and the test pins that byte-for-byte rather than “close
  enough”. A JPEG source exports fully opaque, and the output is verified to start with the PNG magic.
- **`Spec::summary()` no longer lies about PNG**: a PNG export says `PNG · lossless` and its longest edge,
  and never quotes a JPEG quality number. `quality` and `metadata` stay in the spec (a JPEG still uses
  them) but are inert for PNG; the dialog's PNG branch shows only the edge control and a hint that there
  is nothing lossy to pick, and the “Keep EXIF and IPTC” checkbox is JPEG-only.
- UI: the export dialog draws all three chips from `Format::ALL`; the PNG branch is covered by the
  headless `run_ui` test (the dialog does not rewrite the format, and the summary line says lossless).
  Engine: a worker export asked for PNG writes `.png` files that decode back as PNG — round-tripped
  through `begin_export`/`poll_export` like the JPEG face. CLI: `--format png` parses and the help text
  lists it.
- ROADMAP's Phase 1 item 4 and the M8 export row ticked in the same commit; the “Re-opened from M8”
  paragraph now lists three remaining Phase 2 rows; RELEASE.md's export limit names Original/JPEG/PNG;
  UX.md's export row marks quality and metadata as JPEG-only.
- 610 tests (6 new); ci, snapshot, deny green. Screenshot harness does not exist yet (Phase 3 item
  9); the change is one dialog branch and one new chip painted through the existing `chip`/`hint` rows,
  so the headless `run_ui` suite covers it.

## 2026-11-23 (twenty-fifth) — Phase 2 item 1: real weights, pinned by digest

- **`Manifest::shipped()` is no longer empty.** Three real, permissively licensed ONNX files are pinned:
  `yolos-tiny` — YOLOS-tiny detector, Apache-2.0, fp16 export, 13,251,235 bytes, input 1×3×512×800,
  answering `logits` [1,100,92] and `pred_boxes` [1,100,4]; and the CLIP ViT-B/32 int8 pair — MIT,
  `clip-vit-b32-vision` (89,023,662 bytes, 1×3×224×224 → `image_embeds` [1,512]) and
  `clip-vit-b32-text` (64,390,561 bytes, `input_ids` [1,77] int64 → `text_embeds` [1,512]). The files
  live in the GitHub release `models-v1` on this repository and are fetched only by the opt-in
  `model.install` command once it exists (Phase 2 item 2); nothing ships inside the app.
- **The pins are verified, not believed.** Candidate exports were rejected until each pinned file was
  proven to load *and run* under `tract-onnx` 0.23.10: the quantized CLIP pair and YOLOS-tiny answer
  real inputs with all-finite values; the 2019 zoo tiny-yolov3 (TF-style Squeeze), zoo ssd-mobilenet
  (unimplemented Loop), yolos as published and the transformers.js exports as published (symbolic
  `batch_size`/`num_channels` dims that tract cannot type) were all found and rejected — hence the
  pinned derivatives. YOLO-family/RT-DETR models under AGPL (YOLOv5/v8) and non-permissive licences
  (MobileCLIP's Apple ML licence) were excluded by the owner's sign-off.
- **The transformation is documented, not hidden.** Each pinned file is a derivative of an upstream
  export (Xenova's): the exporter's symbolic input shapes concretized (batch 1) and shape-inference
  annotations stripped, which changes no weights; upstream digests were checked against the
  publishers' recorded hashes before deriving, and GitHub records the same SHA-256 for each release
  asset. ATTRIBUTION.md's "Data and models" section now states all of this, keeps the sentence that
  no weights ship with the app, and records the I/O contracts (including that the COCO-91 label map
  and the CLIP BPE vocabulary ship with the inference slice).
- **The store and the manifest keep each other honest.** New tests: the shipped manifest satisfies
  its own `parse`, every shipped licence is one the owner approved (MIT/Apache-2.0), the pin is one
  detector plus one embedder pair with distinct digests, every pin is real-world sized (>10 MB — a
  fake few-byte pin would verify garbage perfectly), and every shipped id appears in ATTRIBUTION.md.
  `xtask attribution` learned the difference between the model-pins table and a dependency table, and
  now checks every release URL named in `manifest.rs` against ATTRIBUTION.md instead.
- ROADMAP's Phase 2 item 1 and the M8 manifest row ticked/named in the same commit; RELEASE.md's
  "no bundled models" bullet now says pins exist and what is still open (downloader, inference).
- 614 tests (4 new); ci, snapshot, deny green. No UI change, so no run_ui or screenshot work
  applies; the screenshot harness does not exist yet (Phase 3 item 9).

## 2026-11-23 (twenty-sixth) — Phase 2 item 2: the consent-gated model downloader (SL-F02)

- **`crates/engine/src/models.rs` is the only thing in the repository that fetches a model.** It was placed
  deliberately outside `sortlens-ai` — whose `no_network` test still holds, untouched — and deliberately
  desktop-only, with two new target-gated
  dependencies (`ureq` over rustls for HTTPS with a `Range` header; `craft-atomic` for the Windows-safe
  rename and the fsync before it).
- **Resumable, and the resume is tested.** The bytes stream into `<library>/models/<digest>.<kind>.part` in
  64 KiB chunks; a download that stops — cancelled between chunks, or the network dropped mid-body — leaves
  the part exactly where it got to. The next attempt sends `Range: bytes=<len>-` and appends. Three
  dishonest servers are answered by code, not luck: one that ignores the range (a fresh 200 body triggers a
  restart, never a splice), one that truncates (the part is kept, the error says "running it again resumes"),
  and one that sends more than the manifest promised (the part is removed — it can never be a resume point).
- **The digest is the contract, enforced twice.** A finished part is verified against the manifest
  (streaming, 64 KiB at a time — a 166 MB model is never held in RAM) before it is renamed into place under
  its digest name, and the renamed file is read back and verified again. A file that is not the model never
  becomes a model, and it never sits where a reader would find one.
- **`sortlens-ai` gains two pure, offline additions**: an incremental SHA-256 (`Hasher`, pinned equal to the
  one-shot over every padding boundary 55/56/64/65/…) and `Model::verify_file`, the streaming check that
  `Store::status` now uses — so a Settings row no longer reads a model file into memory to answer
  "is it installed".
- **Consent is a state, checked before anything moves.** `Engine::models_consent` starts `false` every
  session; the app persists the switch in Settings → AI (`prefs.ai.download_consent`) and restores it at
  launch. `model.install` refuses without it from every face — UI, CLI, MCP — with a refusal that names the
  setting. The engine answers the model verbs from `self.model_manifest` (the shipped pins; a test swaps in
  a loopback manifest, because no test can serve bytes under a digest it does not know).
- **The surfaces**: Settings → AI lists every pinned model with licence · kind · size and its verified
  state; the Install buttons exist only behind the consent switch; a download draws its progress and a Stop
  that keeps the partial file. Activity gained a `Model` job whose detail is the byte count while running
  and the install receipt when done. CLI: `models`, `model-consent on|off`, `model-install <id>` (a
  progress verb like `sharpen`), `model-remove <id>`, plus a `models` JSON schema. MCP: a `model.status`
  example and executable examples for `model.cancel`/`model.remove`; `model.install` and `model.consent`
  carry notes (a proven example cannot download).
- **Tests that fail when the behaviour is absent** (8 module tests + 5 engine tests, all over loopback
  servers — no network): the verified install under its digest name; interrupted-then-resumed with the
  Range request asserted on the wire; the 200-restart; wrong bytes refused with the part removed; a
  mid-copy cancel keeping the part byte-for-byte and resuming from it; a stale too-long part not trusted;
  a manifest without a URL refused before any fetch; the worker polled to an install; consent refusal
  naming Settings; install/remove/progress and the row cache following the store; the UI gate driven by a
  real pointer click on the checkbox in the headless `run_ui` harness (Install absent without consent,
  present after, gone again when consent is withdrawn; `Action::SetModelConsent` pushed both ways).
- `cargo deny` gained one licence decision, written in `deny.toml` where it belongs: webpki-roots' bundled
  TLS root list is CDLA-Permissive-2.0 — permissive community *data*, no code under it. ATTRIBUTION.md
  moved `ureq` and `craft-atomic` to "Linked today".
- ROADMAP's Phase 2 item 2 and the M8 manifest row ticked in the same commit; RELEASE.md's model bullet now
  says the downloader exists and what is still open (inference); UX.md's dialog table and ARCHITECTURE.md
  §9 + the store layout gained the downloader and the `.part` file.
- 633 tests (19 new); ci, snapshot (two goldens updated: the schema list gained `models`), deny green.
  CLI verified by hand against a fresh library: `models` lists three pins, `model-install` without consent
  refuses naming Settings, `model-remove` reports nothing to reclaim.
- **Not verified against the real release**: a live download of the pinned 13/89/64 MB files from the
  `models-v1` GitHub release (this run keeps the tree offline by choice; the loopback suite covers the
  protocol, and the digest pins make any wrong byte a refusal, not a usable model).

## 2026-11-23 (twenty-ninth) — every keyword says who wrote it (SL-D12, Phase 2 item 4)

- `sortlens_core::TagSource`: `Manual`, `Ai`, `Xmp`, and `Unrecorded`. The fourth is the honest one: a library
  written before sources were kept has keywords nobody recorded an origin for, and migrating those to `Manual`
  would invent a decision. It is **derived** — a tag with no entry in the map — never stored, so no snapshot
  ever claims it. `parse`/`id`/`label`/`explanation` and a serde round-trip test.
- `catalog::Photo.tag_sources: BTreeMap<String, TagSource>` with `#[serde(default, skip_serializing_if)]`, so a
  snapshot from an older build replays and rewrites **byte-identically** (asserted in a test that reads the
  snapshot, checks no `tag_sources` key was invented, reopens, and asks the filter for `Manual` — it answers 0).
  `adopt_review` marks a sidecar's keywords `Xmp`; `tag_source`/`carries_source`/`prune_tag_sources` are the
  read side.
- New additive `Op::TagSourced`, committed in the same `Op::Group` as the `Op::Tagged` it describes. That choice
  is what keeps one ⌘Z for a tag *and* its receipt without breaking the 2-tuple `tagged` records already on disk.
  Its `photo_count` is **0** and a group's summary skips it, because a keyword typed by hand is one change, not
  two — without that, every ordinary tag read "Keyworded 1 photo (+1 more change(s))" and a 40-photo keyword
  counted as 80. `Op::name` for a group holding one real change now returns that change's name, so the change
  list says `tagged`, not `group`.
- Pruning runs **after the whole op**, not inside the apply arm: a group sets the words and then their provenance,
  and an undo replays both in reverse, so mid-op pruning dropped a source that the next step was about to restore
  (found by the undo half of the rename test, not by reading the code).
- Spelling out where each source comes from: `Engine::set_tag_from` (hand-typed is `Manual`), `accept_suggestion`
  (`Ai`), sidecar adoption at import (`Xmp`). `rename_tag` carries a source to the new name and `delete_tag` takes
  it away, both inside the group they already committed — a renamed AI keyword is still AI, and a deleted word
  leaves nothing claiming it.
- The **catalog revision is deliberately not bumped** by provenance: `rev` is what refuses a stale sidecar
  writer, and provenance never reaches a sidecar. `docs/BEHAVIOUR.md` §4 now says that, because §1's "every
  annotation change increments the revision" is otherwise a contradiction.
- Surfaces: `Filter::tag_source` ("some keyword came from there", so one accepted suggestion among ten typed
  words counts), the filter strip's cycling **Source** chip with its own tooltip per source, the keyword chip
  printing `cat · AI` instead of hiding the fact in a hover, `sortlens-cli list --source` / `page --source` with a
  refused typo listing the real names, `sortlens-cli`'s MCP `library.page` note, and `library.page`'s filter JSON.
- Tests: core enum round-trip; catalog (partial-group undo, rename carries, delete clears, ghost map pruned,
  legacy snapshot → `Unrecorded` and *not* `Manual`, sidecar → `Xmp`, filter semantics); engine (Manual vs AI
  through `library.page`, one undo for word plus origin, rename/delete); UI (`Context::run_ui` clicks the Source
  chip twice — Source → Manual → AI — asserting the query actually narrowed and the chip relabelled, then finds
  `guessed · AI` painted on the keyword chip); goldens regenerated and read: the only visible changes are the
  new `tag_sources` field, the extra ops in the log, and `group · Flagged` becoming `flag · Flagged`.
- 674 tests; ci, deny, snapshot, a11y, docs, layers, roots, attribution green.
- **Not verified by hand**: how the Source chip sits next to the other seven in the strip at a narrow width, and
  whether `cat · AI` crowds a long keyword. There is still no `cargo xtask screenshots` (Phase 3 item 9, which is
  explicitly to be done before the rest of the visual work), so the visual evidence is the headless paint.

## 2026-11-24 (seventy-first) — the release builds the phone's APK, and the workflow file is a checked file at last

- **Why this came up.** The seventieth entry closed with "everything buildable left is done", but its own
  reasoning had folded two different blockers into one: the `.ipa` leg waits on the Apple identity, while
  the **APK leg needs no external secret at all** — a sideloaded Android APK is signed with a self-signed
  keystore, and the build runs on a runner that already carries the NDK. The roadmap's 3b still had it open,
  so it was the one buildable item left in the owner's Phase 0 → Phase 5 plan.
- **Shipped.** `.github/workflows/release.yml` gained an `android` job: `cargo apk build --release` from
  the crate's own `package.metadata.android` (the manifest is the specification cargo-apk reads), the SDK
  pieces the metadata names installed by `sdkmanager` (`platforms;android-34`, `build-tools;34.0.0`,
  `ndk;26.3.11579264`) rather than trusted from the runner image, then `zipalign` + `apksigner` with a
  keystore generated for the run and a signature verified before upload. The APK rides the same one-runner
  checksum step and release attachment as every other artefact. Two latent defects in the never-run
  workflow were fixed in the same commit: `verify` had no Linux windowing headers (the `cargo xtask ci`
  it runs needs them on Ubuntu, as `ci.yml`'s own job does) and it used a `dtolnay/rust-toolchain@1.95.0`
  ref where the checked `ci.yml` uses `@master` with the toolchain as an input; the release body — which
  still said "no bundled models, no remote client, no localisation yet", three stale claims — was rewritten
  to say what ships.
- **Checked, not just written.** `cargo xtask workflows` now reads `release.yml` the way it reads
  `ci.yml` and `agent-intake.yml`: a release with no `cargo apk build` step, a `publish` not gated on a
  tag (a manual run from `main` must never create a release — that gate is new too, which is why a
  dispatch can be used to exercise the pipeline), checksums that leave the APK out, and an attachment
  that leaves it out are each refused with their file named. The subset reader refuses flow collections,
  which caught `needs: [build, android]` and made the `on.push.tags` list a block list. Tests: the shipped
  file passes, and a fixture release loses the APK build, the tag gate, the attachment and the checksum
  coverage one at a time, each naming its own refusal.
- **Reconciled.** ROADMAP 3b moved the APK out of "Still open" into a shipped bullet and split the rest
  by its real unblocker (device verification and the iOS Photos library need the NDK/Xcode machine; the
  `.ipa` needs the Apple identity; an AAB needs Play tooling; an in-place-update key is a release secret
  the owner can add); RELEASE.md's phone bullet says what a release run now attaches and what still waits.
- **Verification status.** `cargo test -p xtask workflow` (15, including the four new ones) and the
  checker's read of the shipped file are green here; the full gates below. The one thing this checkout
  cannot do is run the workflow itself — that is what a manual dispatch after the push is for, and the
  result is read with `gh run` rather than assumed.

## 2026-11-24 (seventy-second) — the APK leg compiles: on a phone, the trash is the library's own folder

- **Why this came up.** The release workflow's first real run (a manual dispatch after the seventy-first
  push, run 45513471896) failed exactly where a `cargo check` could never see it: linking. The `trash`
  crate does not compile for `aarch64-linux-android` — its platform module has no freedesktop-less
  answer — and the phone's delete path asks for `Mode::Trash`.
- **Shipped.** `Mode::Trash` on Android and iOS now moves the file into the library's own
  `.sortlens/trash` (the name `crates/scan` refuses to descend into, so a rejected photograph never
  reappears in the grid), through the same `move_into` the `Move` mode uses, after the same `audit_one`
  re-check — a file whose bytes or mtime moved since it was indexed is refused here too, even when the
  function is called outside `apply`. The receipt still says `Trash`, and `undo` refuses a trash receipt
  on every platform, so the desktop's semantics are unchanged and nothing destroyable is destroyed: the
  file sits in the folder for a person to recover by hand. The dependency is target-gated to the
  platforms the crate speaks (`windows`/`macos`/`linux-not-android`); the workspace pin stays.
- **Tests.** `the_phone_trash_moves_into_the_librarys_own_skipped_folder` runs on the host on every
  platform (a phone path nobody can test would be the placeholder the plan forbids): the move preserves
  the relative path, the bytes read back, a changed-size file is refused and stays where it was, and the
  undo report refuses the receipt while leaving the file alone. `the_phone_trash_folder_is_skipped_by_
  the_scan` in `crates/engine` is the one place both sides of the cross-crate contract run: a library
  containing `.sortlens/trash` scans clean, the folder refused (hidden on Unix, a system dir on
  Windows), never descended into.
- **Reconciled.** ROADMAP 3b gained the shipped bullet and re-split "Still open" by its real unblockers;
  `docs/BEHAVIOUR.md` §3 says what the trash mode does on a phone.
- **Next.** Re-dispatch the release workflow; `verify` and the desktop matrix were already past `android`
  when it failed, so the APK job is the only leg unproven.

## 2026-11-25 (seventy-third) — the release signs the APK it builds and uploads what it packages

- **Why this came up.** The seventy-second entry fixed the compile the first dispatch run (45513471896)
  stopped at, and left the APK leg "unproven" with a note to re-dispatch. The re-dispatch (45519830389)
  came back with two failures that no local gate could see, and they were different in kind: the APK
  build reached the last step and stopped there, while the three desktop jobs passed and attached
  nothing at all.
- **A release build of the APK refuses to sign without a key.** `cargo apk build --release` aligns the
  APK and signs it with `apksigner` itself, and stops with `Configure a release keystore via
  `[package.metadata.android.signing.release]`` when it has none — which is the state a public repo is
  in on purpose. cargo-apk 0.10 reads `CARGO_APK_RELEASE_KEYSTORE` and its `_PASSWORD` before the
  manifest (checked in the crate's own `src/apk.rs`), so the android job now generates a keystore for
  the run, names it to `cargo apk`, and lets cargo-apk align and sign. The old `zipalign` + `apksigner`
  pair is gone: it ran after a build that never produced an APK, and re-aligning a signed APK is the
  one way to lose a signature that was already correct.
- **The three desktop jobs uploaded nothing and passed.** `path: dist/*.tar.gz* dist/*.zip*` was read by
  `upload-artifact` as one newline-separated list — so as one glob with a space in it, which matches no
  file. The step warned, the action default (`if-no-files-found: warn`) kept the job green, and the run
  ended with zero artifacts: a `publish` on a tag would have had nothing to checksum or attach. Both
  upload steps now carry one glob per line.
- **The checker could not have caught either.** `cargo xtask workflows` now refuses an APK build whose
  step is not given `CARGO_APK_RELEASE_KEYSTORE=`, a release with no `keytool -genkeypair` and no
  `apksigner verify`, an `upload-artifact` whose `path` line holds more than one glob, and a `publish`
  that never downloads the built artefacts — four rules, four fixtures, plus the shipped file.
- **Verified on the run itself** (45527734155, commit `09cd112`): every job green, `publish` skipped as
  designed on a dispatch; the log says `Verifying alignment of .../target/release/apk/sortlens-mobile.apk
  (4)`, `Signing ... with keystore /home/runner/work/_temp/sortlens.keystore`, and `apksigner verify
  --print-certs` printed `Signer #1 certificate DN: CN=Sortlens` with SHA-256 `61be0349ac75…`. Four
  artifacts exist where there were none: `android-apk` 71.3 MiB, `x86_64-unknown-linux-gnu` 138.3 MiB,
  `x86_64-pc-windows-msvc` 33.9 MiB, `aarch64-apple-darwin` 35.7 MiB.
- **Files.** `.github/workflows/release.yml` (the keystore step, the two variables, the verify step, one
  glob per line, a header that says why); `xtask/src/workflow.rs` (four rules, the module doc, the
  fixture and four tests); `docs/RELEASE.md` (what a release run signs with and why the archive step
  uploads one glob per line); `docs/ROADMAP.md` (the 3b bullet rewritten around the two bugs and the
  verified run).
- **Tests.** `cargo test -p xtask workflow` (16 pass): `a_release_that_puts_two_globs_on_one_upload_line_
  `is_refused`, `a_release_that_builds_the_apk_without_a_keystore_is_refused`,
  `a_publish_that_downloads_nothing_is_refused`, and the three older release tests still refuse an
  unattached APK, an ungated `publish` and a release with no `cargo apk build`. `cargo xtask ci` green
  through every gate, `cargo xtask workflows` included.
- **Next.** The `publish` job has still never run: it needs a `v*` tag, and a release is a decision a
  person makes. Everything above it — `verify`, the four builds, the signing, the artifact upload — is
  now proven by a real run. Phase 4 item 3b keeps its named unblockers: a device build, the iOS Photos
  library, an `.ipa` (Apple identity), an AAB (Play tooling), and a release key the owner can add so one
  APK updates another in place.

## 2026-11-26 (seventy-fourth) — every verb that answers one object answers `--json`, and the flags it reads are the flags it honours

- **Why this came up.** The roadmap row said `page` prints JSON and "there is no `--json` for the other
  verbs". That was half stale — forty schemas already existed — and half a live bug, and the bug was the
  expensive half: the JSON form of `list` put `--tag` at the top level of the request, and
  `library.page` reads only `args["filter"]`, so `list --json --tag dunes` answered **the whole
  library**. `--source` was not read at all on that path. The plain form built a real `Filter` and
  filtered; the JSON form silently answered a different question than the one asked. The plain `page`
  arm dropped `--tag` the same way (its filter carried only `tag_source`).
- **One mapper, so the two spellings cannot drift.** The `list`/`page` arm of `json_verb` builds the
  same `Filter` the plain `list` does — `tags`, `tag_source`, offset/limit/sort — and the plain
  `page` arm now just calls the mapper. An unknown source is refused with the real ones rather than
  treated as "no filter": the class of bug a filter must not have. Golden steps `45`–`49` pin the
  behaviour: `tag 1 dunes` → `list --json --tag dunes` is 1 photo, `--source manual` is 1,
  `--source unrecorded` is 0, and `--source guessed` is a refusal naming `manual, ai, xmp, unrecorded`.
- **The eleven verbs that had no schema now have one**, each the engine answer it already produced:
  `page`, `insights`, `report`, `refresh`, `compact`, `backup`, `restore`, `move`, `unmove`,
  `model-consent` and `update-consent`. `report` already printed its object as JSON without the flag;
  it now answers it with the flag too. The two consent switches gained `changed` like every other write,
  which exposed a truth worth stating in the golden: the CLI keeps consent for the run only, so a fresh
  process setting `off` (the default) reports `changed: false` — the engine test pins the in-session
  repeat that reports `false` the second time.
- **Four verbs were reachable only through `--json`** — `label`, `mark`, `marks`, `jobs` — and now
  print prose like the rest. `move`'s schema note now says the destination is absolute (the engine
  refuses a relative one, and the existing golden `25` pins that refusal; the new golden uses an absolute
  path and follows it with `unmove`, so the move/unmove pair is exercised end to end for the first time).
- **The usage text was wrong, not just thin.** Its `--json` paragraph sat *between* `backup` and
  `restore`, three verbs were missing, and it named none of the newer commands; it is rewritten, and its
  final paragraph now says the JSON form takes the same flags as the plain form and lists the passes that
  refuse.
- **Tests.** Twenty-five new golden steps (`45`–`69`) cover every new schema, the keyword-filtered page,
  the unknown-source refusal, a pass that still refuses, a no-op consent switch, and the move/unmove pair.
  The contract test now names 23 one-object writes (up from 15), eight passes and `diagnostics` as verbs
  that must *refuse* `--json`, so "no schema" is a decision with a test rather than an omission. The
  support report's `os`/`arch` strings are normalised like the diagnostics bundle's, so the new golden is
  portable; JSON-escaped Windows paths are normalised too. A new contract test reads the help text a person
  sees and requires every schema verb to appear in it — it failed on `schemas` itself before the row was
  exempted, which is the proof it bites.
- **Reconciled.** `docs/CLI.md` (verb lists, the pass list, a "the same flags" paragraph, the test
  section), `docs/TOOLING.md` (the "pin five schemas" claim and a refusal example that quoted a verb list
  from before forty schemas existed), `docs/ROADMAP.md` (the row ticked with what it really was; the
  "pin five of the schemas" parenthetical corrected).
- **Verified.** Clippy `-D warnings` on the two changed crates; the engine's five consent-filtered tests;
  the goldens regenerated and **read**, which is how the `--source ai` step was found to be named "a
  source nothing has" while the fixture did have an AI tag (it now asks for `unrecorded`), and how the
  relative `move` destination was found to be refused rather than moved. `cargo xtask ci`, `snapshot`
  and `deny` green (the CI exit code read from a log file, not from a pipeline's); no screenshot change,
  since nothing painted changed. Because the slice touches `apps/**`, the release workflow was dispatched
  once on this commit — run `45536428707` — and is checked next tick, not watched.
- **Next.** Check run `45536428707` with one `gh run view`. Then `docs/ROADMAP.md`'s last buildable honest
  row: **SL-L08, localised numbers and dates** —
  counts, file sizes and dates are formatted with English rules whatever the picker says. The four other
  catalogues and the RTL screenshots still wait for a translation pass and an Arabic font build.

## 2026-11-26 (seventy-fifth) — the screenshots gate compares against a set named for the adapter, and a red run hands over what it painted

- **Why this came up.** `CI` on `main` was red, and had been for six commits: the loop read release runs
  and never this one, so the one red it could not see was the one it could have fixed. Run
  `45540115487` on `b1e7578` named two jobs — `screenshots` and `mobile` — and this tick took
  `screenshots`, because the work for it was already in the tree (an uncommitted `ci.yml` and
  `crates/ui/tests/screenshots.rs`, finished here rather than left as a second tick's inheritance).
- **The screenshots job was comparing one adapter's paint against another's.** The committed baselines
  are painted by the reference rasteriser; the runner has no GPU, so wgpu finds Mesa's software Vulkan
  (lavapipe), which paints a few pixels differently. The old gate answered that with a byte-for-byte
  comparison across two different painters, which can only be red. `SORTLENS_SNAPSHOT_SET` now names a
  baseline **set**: with a set named, every scene compares against `tests/snapshots/<set>/<name>.png`.
  The tolerance does not move — both are compared byte for byte — so a set is either painted by that
  adapter or absent, and the job cannot go green by being forgiven.
- **The set name becomes a directory, so it is sanitised.** It arrives from a workflow written by hand
  and lands under `tests/snapshots/`, so `set_dir` lower-cases it and folds every run of anything that
  is not a letter or a digit into one dash: `Mesa LLVM 15.0` is one folder, and `../..` is no folder at
  all instead of an escape from the baselines. A blank or unset value is the committed set, which is
  what a developer's own machine uses.
- **A red run now leaves its paint behind.** `if-no-files-found: ignore` guards an upload of
  `crates/ui/tests/snapshots/**/*.new.png` as the `screenshot-set` artifact, `if: always()`. A set
  nobody has committed cannot compare, and the frames it failed on are exactly the frames that set
  needs: `gh run download <id>` gets them, a person reads them, and they are committed under the set's
  directory. Without that step a red `screenshots` job had nothing to fix it with.
- **Verified.** `cargo xtask workflows` accepts the edited `ci.yml` (`xtask workflows: ok`) — the
  checker already refuses a gate that cannot fail, and an `if` this one is not. `cargo xtask ci` green.
  The new test `a_baseline_set_scopes_every_scene_to_its_own_directory` passes and would fail if
  `scoped_name` stopped prefixing the set: it pins the empty, blank, plain, mixed-case and escape
  spellings. No baseline was regenerated and none is read differently — the reference set is unchanged,
  which is the point of a set.
- **Files.** `crates/ui/tests/screenshots.rs` (`SORTLENS_SNAPSHOT_SET`, `baseline_name`, `scoped_name`,
  `set_dir`, one test, the two `try_snapshot` call sites); `.github/workflows/ci.yml` (the set's value and
  the artifact upload); `docs/ARCHITECTURE.md` (what a set is and how one gets committed);
  `docs/ROADMAP.md` (the Mac row's "separate baseline set" — that mechanism exists now, the Mac set and
  the embedded symbol faces do not; and the gate's own row says how it compares and what it uploads).
- **Next.** The `lavapipe` set is not committed yet: this commit is what paints it. Read the `CI` run
  for it, `gh run download <id> -n screenshot-set`, read the frames and commit them under
  `crates/ui/tests/snapshots/lavapipe/`. **The other red job is still red** and is the tick after:
  `mobile` (android, `aarch64-linux-android`) stops in `ring`'s build script with `failed to find tool
  "aarch64-linux-android-clang"` — the job's comment says the Ubuntu runner carries the Android NDK, and
  its log says nothing on PATH answers to that name, so the job has to install and name the NDK's clang
  (or state honestly that the target is unverified here) rather than assume it.

## Next

Phase 0 is done. The queue below is the order work runs in, one vertical slice per commit, each with a test
that fails when the behaviour is absent, its ROADMAP row ticked and a CHECKPOINT entry in the same commit.
Gates every commit: `. scripts/env.sh && cargo xtask ci`, `snapshot`, `deny`, and `mobile` when `crates/core`,
`crates/ui` or phone-facing crates change (`mobile` needs the Android NDK and Xcode, so a machine without them
says so rather than skipping it silently). The full text is in `docs/ROADMAP.md` "Completion phases".

**Phase 1 — core engine and media gaps**
1. **Video poster frames** (SL-B11 part 2) — **done 2026-11-22**, see the twenty-first entry.
2. Stale-revision rejection (SL-D11) + the stale-revision and badge-contrast tests that close M3 —
   **done 2026-11-23**, see the twenty-second entry.
3. `Recently Added` (SL-A14 part 2): `added` timestamp, snapshot migration, catalog/engine/sidebar —
   **done 2026-11-23**, see the twenty-third entry.
4. PNG export (SL-B24): licence-approved encoder, export dialog and CLI — **done 2026-11-23**, see the
   twenty-fourth entry.

**Phase 2 — real local AI tagging**
5. Pin real ONNX models (detector + CLIP/MobileCLIP) in `Manifest::shipped()` and `ATTRIBUTION.md` —
   **done 2026-11-23**, see the twenty-fifth entry.
6. `model.install`/`model.remove` consent-gated resumable downloader in `sortlens-engine`; Settings → AI —
   **done 2026-11-23**, see the twenty-sixth entry.
7. `tract-onnx` inference (SL-F03): the runtime, the detector path over 384 px previews, NMS +
   `suggest::merge` — **done 2026-11-23**, see the twenty-seventh entry. Part 2 (3b) — per-pack text
   embeddings cached at install (CLIP BPE tokenisation) and image embeddings, matched through
   `suggest::merge` — **done 2026-11-23**, see the twenty-eighth entry; the real downloaded pins were
   loaded and run once as that slice's first gate.
8. Tag provenance (SL-D12) with snapshot migration; filter by source in UI and CLI —
   **done 2026-11-23**, see the twenty-ninth entry.

**Phase 2 is complete.** Phase 3 starts here, in this order:

9. **`egui_kittest` screenshot harness + `cargo xtask screenshots`**, Charcoal and Paper baselines for Empty
   Welcome, Library (Justified and Square), Cull, Detail, Compare, Similar, Places and Insights — **done
   2026-11-23**, see the thirtieth entry. 18 baselines; `egui_kittest` does exist at the fork revision, so no
   fallback rig was needed. It found the unnamed sort combo and the missing arrow glyphs (both fixed) and
   recorded three visual defects under 3B.6.

**Phase 3 — UI/UX overhaul**
9. `egui_kittest` screenshot harness, `cargo xtask screenshots`, Charcoal and Paper baselines (do this before 10–15)
   — **done 2026-11-23**, see the thirtieth entry.
10. `WidgetPalette` mapping + monochrome SVG icon set — **done 2026-11-23**, see the thirty-first entry:
    34 glyphs in `assets/icons/` (36 after 3B.2), `crates/ui/src/icons.rs`, wired into all five surfaces, and
    `Tokens::palette()` is the one widget-state mapping. The four value chips in the top bar (theme, density,
    layout, motion) are still words on purpose: they print a value, and 3B.2's Appearance menu is where they move.
11. **Top bar declutter and the `⌘K` command palette — done 2026-11-23**, see the thirty-second entry:
    21 palette rows, every one a `Ready` registry command; the field switches mode in place; the bar's eight
    glyphs; the Filter badge and toggle; Appearance as one menu; density/layout in the canvas header; 36 glyphs;
    20 baselines including the two new palette scenes. Named as not shipped there: the app-mark library menu
    and view history (the UX table used to imply both).
12. **Sidebar scopes and the utility strip (3B.3) — done 2026-11-23**, see the thirty-third entry: six
    scopes with live counts (Missing is new, backed by `Filter::missing`), and the four dialog rows moved to an
    exactly-sized foot whose words and handlers are the palette's. Named as not shipped there: the per-row
    "filtered" marker and the footer's storage summary.
13. **The bottom bar (3B.4) — done 2026-11-23**, see the thirty-fourth entry: nine destinations with a measured
    narrow-window collapse, the annotation pill, the size slider and zoom/`?` controls, and three new `Ready`
    view commands.
14. **The right panel (3B.5) — done 2026-11-23**, see the thirty-fifth entry: `RightTab::ALL` is four with
    Suggestions between Tags and Activity, `right_tools` is the 30-point far-edge rail that is painted whether the
    body is open or not, the tab row is gone and a header names the active tool. Proposals are grouped into
    Objects / Scene / EXIF for the whole selection, with confidence, partial-selection counts, `Accept all` per
    family and `×` per row; accepting a family is one ⌘Z (`Engine::accept_suggestions` over one `Op::Group`) and
    dismissal is a stored, undoable `Op::SuggestionDismissed` with `suggest.dismiss` in the registry, an MCP
    example and a CLI verb. Two new baselines plus all 25 regenerated and read. Named as not shipped there: the
    rail's 14-point glyphs in 24-point rows are a mouse target, not a touch one, and `Accept all` beside a long
    family name is unverified at the panel's 250-point minimum.
15. **The visual defects (3B.6), in this order.** (a) the Welcome bullets are `•` text where they should be the
    icon set's own glyphs — **done 2026-11-23**, see the thirty-sixth entry: `folder`/`history`/`reject` in one
    16-point column, an exact-count `run_ui` test, two baselines regenerated and read. (b) the strip on an empty
    library, and its flat-text controls — **done 2026-11-23**, see the thirty-seventh entry: hidden while the
    catalog is empty (a filter that hides everything keeps it), stars as glyphs, the label's own colour as a
    swatch, the state's own flag, a chevron on the keyword menu, and a wrapping row that found `Clear` was
    unreachable at 1200 px with both docks open. (b') the canvas under a filter that hides every photograph —
    **done 2026-11-23**, see the thirty-eighth entry: `canvas_empty` splits the catalog's total from the filtered
    count, so a narrowed-to-zero library paints *Nothing matches these filters*, the library's own count and one
    `Clear filters` button, an actually-empty catalog still paints the Welcome card, and `Not now` finally stays
    dismissed. Two baselines at 1200 px read. (c) Places ships the offline coastline basemap and falls back to the
    user's `tiles/` and `places.txt`, and opens fitted instead of at zoom 5 — **done 2026-11-23**, see the
    thirty-ninth entry: the Natural Earth 1:110m outline embedded in `sortlens-geo` with its credit on screen, split
    at the antimeridian and projected once, `bounds_centre`/`fit_zoom` replacing the fixed zoom and the mean centre,
    a `tiles/` folder still winning, a chosen zoom never overwritten, seven geo tests, two shell tests and both Places
    baselines read. (d) the sidebar's pinned foot overlapped the keyword and collection rows above it at ~620 px
    — **done 2026-11-23**, see the fortieth entry: the sections live in one `ScrollArea` sized to the room the foot
    left, a test intersects every painted text rect with its own clip rect so "visible" means visible, a wheel event
    proves the rows under the fold are reachable while the foot stays put, and `short-sidebar-charcoal`/`-paper` were
    added and read. (h) the filter strip's `N shown` summary was painted under the `Clear` chip at 1280 px with
    both docks open — **done 2026-11-23**, see the forty-first entry: the count is measured first and given a
    reserved column on the right, the chips wrap in the room that is actually theirs, and a test sweeps the strip
    from 320 to 760 points a step at a time asserting no painted text ever touches the count (it fails at 326 on
    the old layout); all 29 baselines regenerated and read. (e) the canvas header's title ran under the
    right-aligned sort combo at 900 px with both docks open — **done 2026-11-23**, see the forty-second entry: one
    `Sides::shrink_left` row that measures the controls first, a count that steps aside when it will not fit, chips
    offered only above 430 points, a 401-width sweep that fails at 900 on the old layout, and two new
    `narrow-canvas-*` baselines at 900×620 read. (f) the cull keyword row clipped under the bottom bar at 900 px —
    **done 2026-11-23**, see the forty-third entry: the block under the photograph is a `Panel::bottom` with an exact
    height claimed before the canvas, `cull_foot_height` is arithmetic over fixed-height rows with a measured live
    line, the keyword band is two chip rows that scrolls so sixty keywords cannot resize the photograph, the two live
    lines became one, and a test at 1440×900 asserts every cull text is visible inside the column and above the bar.
    **(g) is done** — see the forty-fourth entry: the arrows are triangles in both faces, measured, tested twice
    and read in new baselines. (g) was *observed*, so it is "the baseline is clean and the geometry test says so"
    and it is.

**Phase 4 — platform, keymap, remote**
16. `craft-keymap` editor in Settings → Shortcuts — **done 2026-11-24**, see the forty-fifth entry.
17. Platform finishing, in this order — **2a `.icns`/`.ico` via `cargo xtask icons` done 2026-11-24**, see
    the forty-sixth entry; **2b the High-Contrast preset (SL-L09) done 2026-11-24**, see the forty-seventh
    entry; **2c the opt-in `craft-update` (SL-I10) done 2026-11-24**, see the forty-eighth entry; **2d the
    `craft-i18n` scaffolding (SL-L07) done 2026-11-24**, see the forty-ninth entry.
18. `sortlens-server` + the phone client with a two-process loopback integration harness in CI — the
    server half, the harness and the desktop's start/stop surface are **done 2026-11-24** (fiftieth and
    fifty-first entries); the in-memory catalog path a paired device needs is **done 2026-11-24**
    (fifty-second entry); the device-side protocol, `library.snapshot` on the wire and the client's
    keep-or-remove ack are **done 2026-11-24** (fifty-third entry); the server's thumbnail route and
    the client's `thumbnail` fetch are **done 2026-11-24** (fifty-fourth entry); the native
    `apps/sortlens-mobile` was added the same day (fifty-seventh entry). Its settings persistence is **done 2026-11-24** (sixty-fifth entry). The phone's frame — a window under 600 points put away both docks, wordless bar, one full-width sheet at a time — is **done 2026-11-24** (sixty-sixth entry). The canvas at that width — the far-edge rail standing down, a two-row filter strip, a tray whose every control is on screen — is **done 2026-11-24** (sixty-seventh entry). The send half, the pairing's connection test and the phone's own screens shipped the same day (sixty-eighth and sixty-ninth entries), and the phone's embedded font set is trimmed to the shipped languages' scripts (seventieth entry). The release workflow now builds and attaches the Android APK (seventy-first entry), and the phone trash that leg exposed is a library-folder move (seventy-second entry). What remains of the row is the iOS Photos library, a device build, the `.ipa` (Apple identity) and an AAB — all named with their unblockers in the 3b row itself.

**Phase 5 — autonomous GitHub maintenance**
19. Issue/PR templates — **done 2026-11-24**, see the fifty-eighth entry.
20. `docs/AUTONOMOUS_AGENT.md` — **done 2026-11-24**, see the fifty-ninth entry.
21. `agent-intake.yml` and a PR-gating `ci.yml` (`ci`, `snapshot`, `deny`, `mobile`, `screenshots`) —
    **done 2026-11-24**, see the fifty-ninth entry. **Phase 5 is complete.**

Stop condition: every row under "Completion phases" ticked. Never move a buildable feature into a
"Deliberately absent" table to close a milestone.

Note for the next run: this checkout builds on Windows (x86_64, `x86_64-pc-windows-msvc`) with its own
toolchain in `.toolchain` — `. scripts/env.sh && cargo xtask ci` works as documented, and so do
`snapshot`, `deny` and `screenshots` (23 scenes, green as of the sixty-seventh entry; the two phone scenes at
390×780 are in that count). There is no `cargo xtask wasm` in this workspace, whatever a plan may ask for. `cargo xtask
mobile` cannot run here: there is no Android NDK and no Xcode, so both phone targets stay *unverified on a
device* rather than silently skipped — that is why the roadmap's 3b still names it as open. On a Mac the
screenshots are a separate known problem: a Mac without a craft-faces symbols font paints `⌘Shift+R`
instead of the ASCII baseline and makes `craft-keymap`'s editor scene panic on `⇧` (fifty-sixth entry), so
making the shots a Mac gate means the harness embedding the symbol faces and a second baseline set.
