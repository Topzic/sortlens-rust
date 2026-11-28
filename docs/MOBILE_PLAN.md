# Sortlens phone replacement plan

**Status: proposed; implementation not yet complete.** The owner wants the existing phone app recreated as a Rust-based Sortlens phone app, replacing the current desktop-shaped mobile shell. The target combines an Apple Photos-inspired phone experience with Sortlens culling and secure access to the user's Sortlens servers from anywhere. This is a product direction for SL-J10, not a claim that the present app already has these capabilities.

## How this doc is kept current

Each numbered "Progress" subsection below is one tick of the **continuous coder**
scheduled task — `Sortlens: phone replacement loop`. The contract reproduced
from the scheduler lives at
[`docs/scheduled_prompts/sortlens-continue-loop.md`](scheduled_prompts/sortlens-continue-loop.md)
and the live lease id is named in the **Lease note** at the bottom of
[`CHECKPOINT.md`](../CHECKPOINT.md). Every tick commits and pushes its slice
before re-enabling the lease, and on the bootstrap tick adds this doc to git on
its own so subsequent ticks read a tracked starting point. A progress note
that exists only in the working tree is not finished.

## Evidence and limits

- The owner's screenshots show: Library (light and dark, photo count, select/filter controls, Years/Months/Days/All Photos, four-column grid), For You memory/highlight cards, Albums with folder and album rows and counts, Search with recents and keyboard, Stats, a map with photo clusters, and a full-screen swipe-review view with progress, reject/skip/undo/keep. They are visual references, not proof of underlying behavior.
- The named reference is `/Users/iancunningham/Documents/sortlens-mac-handoff-20260504-190347-main`. On 2026-11-28 macOS returned `Operation not permitted` when listing it. **The original app's source, screen flows, exact feature inventory, data model, backend compatibility and migration path have not been audited.** Do not claim complete parity from screenshots. Once access is granted, inspect its README, manifests, mobile source, server/client, configuration and tests; write an evidence-backed feature/behavior matrix here and adjust the order below. Distinguish implemented features from mockups and unsupported flows. Never copy credentials, user data or proprietary assets into this repository.
- `apps/sortlens-mobile/src/lib.rs` currently hosts the shared desktop shell; it uses its own engine and data directory. `places.rs` looks in the iOS sandbox's Documents folder, not the system Photos library. Pairing and sending exist in `send.rs`; the phone does not yet expose a remote-server library browser. `crates/remote/src/client.rs` contains client helpers for snapshot and thumbnails; `apps/sortlens-server` exposes allow-listed commands, thumbnails, and uploads. These are building blocks, not evidence of end-to-end phone browsing.
- **Security constraint:** today's `TcpTransport` and `sortlens-server` use plain HTTP/TCP, including pairing codes and bearer tokens; the phone stores the token in its settings file. LAN binding is not safe Internet access. Do not expose the existing server or forward its port to the Internet, or claim that it works securely from anywhere. The replacement requires authenticated encrypted transport, protected credentials and a reviewed remote-access deployment before external use.
- Xcode 27.2 with the iOS 27.2 SDK, a booted iPhone 17 simulator and a paired iPhone 15 were present on 2026-11-28; one Apple Development identity exists. The simulator launch works after the scene-lifecycle fix recorded under M1, and the window now reports the phone's safe-area insets (62 pt top, 34 pt bottom) instead of zero — the shared shell now lays out inside them (M2 progress), but the pinned winit fork's iOS `inner_size()` sizes the host's wgpu surface to 402×778 while the window layer is 402×874, so the frame is stretched and its bottom 96 pt cropped — M2's progress has the measurements. The earlier launchable bundle relied on falsified `LC_BUILD_VERSION ... sdk 18.0` metadata; that is gone. A GPU limit clamp exists as an uncommitted change in `apps/sortlens-mobile/src/entry.rs`; it is part of the build that launched, and must be preserved and verified rather than assumed committed. A real-device launch was not attempted: installing over the existing device app needs a decision about bundle identity and its data (M10), and signing stays the owner's. Toolchains and devices can change: probe before reporting a blocker. A development profile is not a distribution identity or a reproducible release pipeline.

## Product and architecture decisions

1. **Rust-first, phone-native product.** Keep the catalog/domain, culling rules, sync and protocol in testable Rust shared with desktop/Android where appropriate. Build a dedicated mobile navigation and presentation layer, not the desktop shell squeezed into a phone. Evaluate egui's real-device safe areas, navigation, gestures, accessibility, image throughput and iOS lifecycle in the first vertical slice. Use a small platform bridge/host where OS APIs demand it; if a native UIKit/SwiftUI presentation is required for fidelity and accessibility, record the boundary and rationale rather than pretending it is pure Rust. Do not start a second unrelated application or change the desktop UX to fit the phone.
2. **Sources are explicit.** Support the iOS Photos library and separately named remote Sortlens servers, each with its own authorization, identity, status and cache. Specify whether the library is combined or source-scoped after the handoff audit; never silently merge identities or imply a server is connected. Multiple servers and offline behavior must be designed explicitly, not assumed.
3. **Safety.** Viewing, rating, flagging and staging a review do not silently delete originals. Destructive operations require an explicit confirmation, scope, permissions and a reversible or clearly explained outcome. Test on fixtures/copies, not the owner's photo library. Preserve catalog and signing data on upgrades; prove any migration and rollback before replacement.
4. **Appearance and access.** Match the reference's hierarchy and interactions without copying Apple's artwork. Respect safe areas, system bars, keyboard, rotation, dark/light and system appearance, dynamic type and VoiceOver; never paint beneath controls or hide data behind a floating tab bar. Network and photo/location permissions are opt-in and explain why they are needed.
5. **Connectivity.** Make a verified private-network route (for example an owner's VPN) the first remote deployment option, and design authenticated TLS plus trusted server identity for direct connections as a separate, reviewed option. Neither requires choosing a paid service now. Expiring pairing and scoped, revocable credentials must work over encrypted channels; store device secrets in the OS secure store. Define discovery, DNS, NAT, certificate trust and reconnect behavior, and do not imply a server is reachable when the Mac is asleep or offline. No unauthenticated public endpoints, hard-coded secrets or silent telemetry.

## Ordered work and acceptance gates

Each unchecked item is a real deliverable. Ship small vertical slices in dependency order; tests and an honest device result belong to each slice. Avoid checking a box based only on mock data, a compile, a screenshot or an untested design.

- [ ] **M0 — Reference audit and parity matrix.** Access the handoff project; list each real screen, entry/exit path, gestures, settings, local/remote source, data and security behavior with file paths and observed vs inferred status. Map these to Rust equivalents and explicitly list gaps, unknowns and migration implications. Include the supplied screenshots. Obtain any missing product decisions instead of inventing behavior. This audit gates claims of feature-complete replacement, but platform-foundation work below can proceed independently.
- [ ] **M1 — Reproducible iOS host and device baseline.** Provide a maintained, documented build/install/run path for both Apple Silicon simulator and real device using Xcode and Rust. Resolve the current UIKit scene-lifecycle incompatibility properly (not by falsifying SDK metadata), configure bundle metadata and Photos/local-network usage descriptions, and handle GPU capability differences. Establish safe-area, lifecycle, foreground/background and error reporting. Keep signing identities and profiles out of the repo. Tests: compile/link both targets; simulator launch and a real-device launch, with build/OS/version and observed result recorded. No desktop or Android regression. Identify which signing/distribution steps still need the owner.
  **Progress (2026-11-28): the scene-lifecycle incompatibility is fixed honestly, and the simulator path is one command.** `apps/sortlens-mobile/ios/main.m` adopts scenes: the bundle's manifest (generated by `cargo xtask ios`) names a `UIWindowSceneDelegate`, and a bridge on `-[UIWindow initWithFrame:]` assigns the foreground `UIWindowScene` to winit 0.30's scene-less window, because iOS 26+ never displays a window without one. This is not the old SDK-metadata falsification: the app is linked against SDK 27.2 and stamps 27.2. `cargo xtask ios sim` builds the release static library for the simulator's architecture, writes `Info.plist` (scene manifest, Photos and local-network usage descriptions, orientations, launch screen; bundle id `app.sortlens.mobile`), compiles the host, links it against the SDK `xcrun` reports with no `platform_version` override, installs and launches on the booted simulator. `cargo xtask ios device` builds and links the `aarch64-apple-ios` bundle and stops, leaving signing to the owner. Verified: the app launches where the un-adopted build trapped in `___UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption`; the installed bundle's log names `app.sortlens.mobile-default` as its scene and realizes the scene's safe-area settings, a prototype of the same bridge measured insets 62/34 where the scene-less window reported 0/0, and the screenshot shows the same shell rendering as before, still under the status bar until M2 applies the insets. The build sets `IPHONEOS_DEPLOYMENT_TARGET=15.0` so cc-rs's C objects (ring, sqlite, tract) no longer carry the SDK's 27.2 stamp; the link is warning-free and the executable stamps `minos 15.0, sdk 27.2`. `cargo test -p sortlens-mobile` passes. Tests in `xtask/src/ios.rs` hold the plist keys and usage strings, the link using the given SDK with no faked platform version, every framework and system library, and both targets. The simulator target `aarch64-apple-ios-sim` must be installed in the project toolchain (`rustup target add aarch64-apple-ios-sim`; `scripts/bootstrap.sh` does not add it); the task names that command when it is missing. **Remaining:** install and launch on a real device (the arm64 build and link are verified; bundle-identity decision, signing identity and profile still need the owner), foreground/background transitions and error reporting, and the shell applying the safe areas (M2). The device bundle is unsigned.
- [ ] **M2 — Phone shell and navigation.** Replace the desktop-shaped phone surface with Library / For You / Albums and the separate Search affordance shown in the references. Build correct iPhone safe-area insets, floating bottom navigation and scroll padding, dark/light switching, back navigation, loading/empty/error states and accessibility. Screenshot and interaction tests at several small/large device sizes and light/dark, including status bar, keyboard, landscape and large text. First prove a single responsive Library surface on a real iPhone before expanding it.
  **Progress (2026-11-28): the shell lays itself out inside the phone's safe area.** The iOS host reports UIKit's insets through egui-winit (`SafeAreaInsets` 62 top / 34 bottom on the iPhone 17 simulator, zero on desktop and Android); `App::ui` clamps them (`content_rect`/`safe_bands`), gives the shared shell a child `Ui` over the visible area, paints the reserved strips in the bar chrome instead of end-of-screen black, and passes the bottom inset to the shell so the notice toast rises above the home indicator (`UiState::safe_bottom`). An inset larger than the window never inverts the rect. On device the app's own coordinates are proven right by the log: `insets=SafeAreaInsets(..top 62, bottom 34) screen=[[0,0]-[402,874]] content=[[0,62]-[402,840]]`. Tests: `the_shell_is_laid_out_inside_the_phones_safe_area` (portrait and landscape rects, band edges, the child's clip), `rotation_moves_the_insets_and_a_desktop_keeps_its_window`, `an_inset_larger_than_the_window_never_inverts_the_content_rect`, and the toast test also covers a 390×844 phone with a 34 pt bottom strip. **The blocker this exposed, in the host rather than this layout:** the pinned winit fork's iOS `Window::inner_size()` returns the safe-area size (402×778), so eframe sizes the wgpu surface to 1206×2334 while the window's `CAMetalLayer` is 402×874 with `contentsGravity=resize`; the presented frame is magnified ~1.124× and its bottom ~96 pt (including the shared shell's bottom bar) is cropped. An app-side `pixels_per_point` compensation was tried and reverted: egui's zoom compounds it and it rescales the insets. The fix belonged in the fork (return the window bounds from `inner_size`, keep `safe_area()` separate) or in eframe's surface sizing; the note below records the fix.
  - **The surface-size blocker above is resolved (2026-11-28, M2 progress).** The pinned winit is vendored at `vendor/winit` with one documented iOS geometry patch (`vendor/winit/PATCH.md`): `Window::inner_size()` and `inner_position()` report the window's full screen frame, while egui-winit still reads `keyWindow.safeAreaInsets` independently, so the shell keeps its 62/34 pt layout. eframe now configures the wgpu surface at 1206×2622 for the 402×874 window, matching the `CAMetalLayer`; the ~1.124× magnification and the missing bottom bar are gone. Verified on the booted iPhone 17 simulator (iOS 27.0): `cargo xtask ios sim` built and launched, and the 1206×2622 screenshot shows the Library pill and the whole bottom bar inside the window. Touch reaches the shell: a Maestro tap on `Add Folder…` started a scan, showed the toast `Indexed 0 photos in Documents` and added two notices (the bell badge went 2 → 4). Taps on `Not now` (at its center and at 35% and 45%) and on the Search field produced no visible change; those controls need a follow-up. The bell tap did not open the notices panel. The same screenshot shows the desktop welcome card still wider than 402 pt and clipped at the left; that is the phone-first shell work (M2), not this host fix.
  - **The phone's first screen is phone-shaped, and Files can hand it photos (2026-11-28, M2 progress).**
    The card the 402-point simulator showed was the desktop one: a 420-point promise block inside a 402-point
    window, its icons and borders clipped at both edges. A window narrower than 600 points now paints
    `welcome_add_phone` instead — one column the width of the phone, `Welcome to Sortlens` at 26 points, the
    promises as two wrapped sentences (`Your files stay put` over `No upload, no rename, no moving. Sortlens
    only reads.`), a full-width accent `Add photos` row 48 points tall, a 44-point `Not now`, and the formats
    line wrapped under them; the desktop card is untouched, so 900-point windows keep the layout they had.
    Dismissing the card over an empty library no longer leaves a phone with a blank screen and no way in —
    a phone has no toolbar where the desktop keeps its Add Folder: `phone_add_prompt` keeps `No photos yet`
    and the same `Add photos` action. `Add photos` is the host's existing `Action::PickFolder`, which on iOS
    scans the app's own Documents folder (`places::photo_roots`); the generated bundle now enables
    `UIFileSharingEnabled` and `LSSupportsOpeningDocumentsInPlace`, so that folder is
    `On My iPhone → Sortlens` in Files and a photo is read where it lies instead of copied out of reach.
    Verified on the booted iPhone 17 simulator (iOS 27.0, `cargo xtask ios sim`): the 1206×2622 screenshot
    shows the card unclipped and the two full-width rows; a tap on `Add photos` indexed `icon.png` (the
    repository's own app icon, copied into the app's Documents as a fixture — no user photo) and the grid
    paints its thumbnail under `1 photo`, with the notices badge raised; the installed `Info.plist` reports
    both File Sharing keys `true`. **The `Not now` and Search taps recorded above were a measuring error, not
    a shell defect:** the percentages were read off a scaled screenshot preview, so egui received
    `pos=(160,480)` for a card whose `Not now` rect is `[[129,468.7]-[191,492.7]]`; a tap on that centre logs
    `hovered=true clicked=true` and dismisses the card. The `Add Folder…` tap that scanned was measured before
    the safe area moved the card up 14 points, which is why the later taps on the same percentages missed.
    Tests: `the_phones_welcome_card_fits_the_window_and_its_not_now_button_dismisses_it` (every painted word
    inside the phone, the accent row at least 44 points tall and 300 wide, the press dismisses and leaves the
    quiet action), `the_phones_add_photos_button_asks_for_the_phones_photo_folders`, and
    `a_phone_that_dismissed_the_card_still_has_a_way_to_add_photos`; `xtask` holds the two plist keys in
    `the_files_app_can_reach_the_phones_photo_folder`. Accessibility is the open gap: Maestro's
    `tapOn: "Not now"` finds no matching element because egui's accesskit tree does not reach the iOS
    accessibility server, so VoiceOver would read the screen as one element until that bridge exists. Tests: `xtask/tests/vendored_winit.rs` fails if `inner_size` sizes from the safe area again; `cargo test -p xtask` (60 pass), `cargo clippy -p xtask` and `cargo clippy -p sortlens-mobile --all-targets -- -D warnings` pass, and the full `cargo xtask ci` (fmt, clippy, workspace tests and the repository checks) passes.
  - **The phone's controls are in the iOS accessibility tree (2026-11-28, M2 progress).** egui already names
    its custom-painted cells and publishes AccessKit, but eframe builds the platform adapter only with its
    `accesskit` feature, and the phone crate did not ask for it — so UIAccessibility saw one unnamed window.
    `apps/sortlens-mobile` now enables `eframe/accesskit` on the iOS target alone; eframe constructs
    `accesskit_winit`'s adapter, whose iOS backend subclasses the winit view and publishes egui's tree through
    UIAccessibility. The feature is target-scoped because `accesskit_winit` rejects Android's native-activity
    backend (eframe's own `compile_error!`), so a workspace-wide enable would break the APK. Verified on the
    booted iPhone 17 simulator (iOS 27.0): Maestro's `assertVisible: "Welcome to Sortlens"` failed before the
    change (the hierarchy held no named element) and passed after it; the same flow then tapped the named
    `Not now` button — not a coordinate — and asserted `No photos yet`. `cargo xtask mobile` checks both the
    Android target and the `aarch64-apple-ios` device target, proving Android keeps its native-activity
    backend and no accesskit. Test: `the_ios_host_builds_the_accessibility_tree_but_android_does_not` reads
    the phone manifest's target tables. This is the tree, not VoiceOver's reading order: an actual VoiceOver
    pass on a real device remains M10's acceptance work.
  - **The phone's bottom bar is a phone's bar (2026-11-28, M2 progress).** At 402 points the desktop row
    collapsed into a single dropdown naming the current destination, beside the rating pill and the help
    glyph — the desktop's bar wearing a phone. Below 600 points `phone_bar` now paints a 54-point row of four
    fingertip destinations (Library, For You, Cull, Insights), each a 20-point glyph over its 10.5-point name,
    with the current one filled in `t.selection`, and `More` on the end whose popup still lists all nine
    destinations, so the phone gives up width and not reach. The notice toast clears the phone bar's 54 points
    instead of the desktop's 40, and desktop windows (and the 600–820-point collapsed menu) are untouched.
    Test: `the_phones_bar_is_a_thumb_row_that_still_reaches_every_destination` paints the shell in the
    simulator's 402×840 content rectangle, requires each word inside the bar, requires the chosen fill to be
    at least 44 points tall, presses `For You` through the shell and asserts the view changed, then opens
    `More` and finds the five destinations the row leaves out. On the booted iPhone 17 (iOS 27.0)
    `cargo xtask ios sim` built, installed and launched it; the 1206×2622 screenshot shows the row under the
    grid with `Library` filled and the home-indicator strip clear, and a Maestro flow tapped `Insights`,
    `For You` and `Library` **by accessibility label** — every step COMPLETED, the last asserting
    `Welcome to Sortlens`. **Follow-up, fixed 2026-11-28.** That flow could not see `Soft Focus` with the
    menu plainly open: `Popup::menu` nested the popup's accessibility node under `More`, and
    `accesskit_ios` treats a button as a leaf, so every descendant was unreachable by VoiceOver. The menu is
    now built without that parent; the next bullet has the cause, the test and the simulator run.
    `cargo xtask ci` passes; screenshot baselines were not regenerated, because the committed phone set
    is already stale from other agents' uncommitted copy and folding this row in would mix two changes in one
    frame — `cargo xtask screenshots` is that gate. **This is not yet the reference's Library / For You /
    Albums row:** there is no Albums surface to
    name until M5's albums exist, and Search is still the top bar's field rather than a screen of its own.
  - **Search is a phone screen of its own (2026-11-28, M2 progress).** The bar's field is dropped below 140
    points, and that left a 402-point phone with no way to search at all. The bar now carries a magnifier
    (`Icon::Search`, the forty-second glyph) whose tap replaces the bar with `phone_search_bar`: one field
    the width of the phone and `Cancel` on a 52-point bar. The query is the same `state.search` the desktop
    field writes, so the grid narrows as the person types; opening asks for the caret, `Cancel` clears the
    query (without writing it over a folder's remembered search) and gives the bar back, and the desktop
    field and wordmark are untouched. Verified on the booted
    iPhone 17 simulator (iOS 27.0, `cargo xtask ios sim`): a Maestro flow tapped `Search photos` by
    accessibility label, asserted `Cancel`, typed `zzzz` and watched the canvas count fall from `1 photo`
    to `0 photos` — a live filter, not a mock — then tapped `Cancel` by label and asserted the count and
    the bar returned (every step COMPLETED; the fixture was the repository's own `assets/icon.png`, not a
    user photo). Tests: `the_phone_opens_a_full_width_search_field_that_cancel_clears` (390×780: the bar
    has no field before, the glyph opens one whose recorded rect is at least 250 points wide inside the
    52-point bar, typed text lands, `Cancel` closes and clears), and the dock test now counts the extra
    glyph. **Not yet the reference's Search screen:** recents and indexed results are M5's, and a query
    filters the loaded library rather than a search index, so this slice claims the affordance and the
    live filter, not the reference's search semantics.
  - **`More`'s rows are in the iOS accessibility tree (2026-11-28, M2 progress).** The phone bar depends on
    `More` for five destinations, but its popup's painted rows never reached VoiceOver or Maestro.
    `Popup::menu(&response)` anchors the popup area's accessibility node to the trigger (`anchor_widget`),
    and `accesskit_ios`'s element filter makes any node with a click or focus action a leaf, so the `More`
    button hid every descendant from the iOS tree. `phone_more` now builds the same popup with
    `Popup::new(...)` and sets the kind, top-down layout, menu style, gap, width and frame explicitly,
    leaving `anchor_widget` unset; the menu's rows are children of the window instead of the button. The
    paint and the actions are unchanged, and the desktop's collapsed menu still uses `Popup::menu`. The
    trade-off is the keyboard's arrow-key entry into this one phone menu; touch and VoiceOver use their own
    focus, and the desktop menu keeps its keyboard path. Test:
    `the_phones_more_menu_reaches_the_ios_accessibility_tree` enables AccessKit, opens `More` through the
    shell, walks the menu's parents under the same click/focus rule `accesskit_ios` applies, and requires
    `Detail`, `Compare`, `Places`, `Similar` and `Soft Focus` reachable from the window root. It fails
    on `Popup::menu` (the menu's parent is a `Button`) and passes on the new construction. On the booted
    iPhone 17 simulator (iOS 27.0, `cargo xtask ios sim`), a Maestro flow opened `More` and asserted
    `Detail` and `Soft Focus` **by accessibility label**, tapped `Soft Focus`, asserted
    `More, now Soft Focus`, reopened the menu, visited `Insights` and returned to `Library` — every step
    COMPLETED, with the repository's own `icon.png` as the fixture; the 1206×2622 screenshot shows the menu
    unchanged.
  - **The phone reads its Photos library through PhotoKit, on the person's tap and with a bound (2026-11-28, M3 progress).**
    `apps/sortlens-mobile/ios/photos.m` is a small C/Objective-C bridge over PhotoKit; `apps/sortlens-mobile/src/photos.rs`
    asks it for read access and reports the answer as `NotDetermined`/`Restricted`/`Denied`/`Limited`/`Authorized`/`Unavailable`,
    failing closed on a status this build does not know. The host hands its five functions to Rust at launch
    (`sortlens_photos_install` → `sortlens_mobile_register_photos`), so the Rust library has no platform symbols of its own
    to resolve and the app link is where the halves meet. A tap on `Add photos` raises the system prompt once, only when
    the person has not answered; the next frames read the answer and continue automatically, so a grant needs no second
    tap. A denial shows `Photo access is off. Turn on Photos in Settings to use your library.`; a limited choice says
    the library is limited and continues with the assets it covers. The bridge exports at most 120 of the most recent
    image assets (`photos::IMPORT_LIMIT`) as 1400-pixel JPEG derivatives at quality 0.82 into
    `<data dir>/Cache/photos`, with the capture date as EXIF `DateTimeOriginal` and the asset's location as EXIF GPS;
    the file name is the capture time plus the identifier's unique leading characters, so a re-import overwrites the same
    derivative instead of duplicating it. `networkAccessAllowed = NO`, so an iCloud-only asset that is not downloaded is
    skipped and the import's notice reports `Imported n of total`, never a false success. Originals are never copied out
    of place, moved or deleted. The import dir is outside the engine's library dir; when it has files, the app scans it
    with the phone's own folders, one engine scan at a time.
    Verified on the booted iPhone 17 simulator (iOS 27.0, `cargo xtask ios sim`): the prompt shows the bundle's usage
    sentence and the library preview; `Allow Full Access` then indexed all 9 disposable assets (6 stock simulator photos
    plus the repository icon in three rotations) without another tap, and the 1206×2622 screenshot shows their thumbnails
    under `9 photos`. The derivatives are 3.0 MB in total, their names are unique and readable (for example
    `2009-10-09 17-09-20 B84E8479.jpg`), the 1400×930 derivative keeps `2009:10:09 17:09:20` as its EXIF original date,
    and GPS is present on the five assets that have it in the library. Revoking Photos in Settings and tapping `Add photos`
    then showed the off guidance without adding photos. `cargo test -p sortlens-mobile` (30 pass) and
    `cargo clippy -p sortlens-mobile --all-targets -- -D warnings` pass; the `photos` unit tests pin the fail-closed
    status mapping and that an unknown export state is idle, and `xtask`'s `the_photo_bridge_is_built_into_the_host`
    compiles both host sources, requires the five bridge symbols and the registration call in `main.m`.
    **Not yet M3:** there is no paging past 120 assets, no cancellation of a running export, and no eviction policy for the
    cache; limited-selection changes while the app runs are read on the next tap, not observed live; videos, RAW and HEIC
    are not separately handled (ImageIO decodes what it can, and unreadable assets are skipped); thumbnails are the shared
    engine's, not a new bounded phone cache. The real-device test with a small owner-approved library is still M10's
    acceptance work.
- [ ] **M3 — Local Photos source.** Bridge PhotoKit through a minimal, auditable platform layer; request read permission only on user action; handle denied, limited, granted and revoked access and changes from Settings. Page assets and albums, maintain stable local identifiers, preserve metadata and avoid copying originals by default. Add a bounded, cancelable thumbnail cache and detail loader; handle iCloud-only assets, videos, RAW/HEIC, permission changes and limited-selection updates honestly. Tests include simulator fixtures and a real device with a small user-approved test library; no deleting the user's assets.
- [ ] **M4 — Library and viewing.** Date-aware Years/Months/Days/All Photos and virtualized grid, counts, selection/filter and photo detail, matching the audited reference semantics. Define sort, time zone, missing dates, edits and media badges. Test pagination, 30k+ assets, bounded memory, fast scrolling, restoration and source switching; exercise representative taps and VoiceOver on device.
- [ ] **M5 — Albums, Search and For You.** Implement reference-backed album hierarchy/counts, navigation and local/remote semantics; indexed, debounced search, recents/clear and useful empty/error states; memory and highlight cards with reproducible grouping and privacy-safe local processing. Test counts and results against known fixtures, permission changes and offline states, then verify the flows on device. Do not invent smart-album or search behavior before M0.
- [ ] **M6 — Swipe culling and statistics.** Implement full-screen review, current/total, reference-backed gestures/buttons, keep/reject/skip, undo, session resume and a separate confirmation for any destructive apply. Compute Stats and review progress from actual source state, not demo numbers; define cross-device conflicts and pending writes. Verify undo/restart, interrupted sessions, denied permissions and multi-device conflict using disposable media.
- [ ] **M7 — Places.** Map photo clusters and counts, zoom/selection, map style/location controls and a useful offline/denied-location state. Use authorized, attributed map data; network tiles require consent. Never upload GPS data implicitly. Test clustering, selection, zoom and absence of GPS; verify safe areas and map controls on device.
- [ ] **M8 — Secure server connection and remote viewing.** Threat-model pairing, authentication, authorization, transport, discovery, credential storage, replay/revocation, server identity and deployment before enabling remote use. Replace or wrap plaintext transport with authenticated encryption; validate certificate/peer identity, rate-limit pairing with cryptographic randomness, use per-device scoped revocable tokens and secure storage. Provide explicit user-controlled source selection and connectivity state. Implement paged remote library, thumbnails and an appropriately bounded detail/preview protocol; support timeouts, cancel, cache limits, offline/stale state and server restart. Verify real server integration, unauthorized and revoked requests, MITM/certificate failure, VPN or TLS route from a second network, and that neither secrets nor originals leak to logs. Do not treat the existing upload-only phone UI as remote browsing.
- [ ] **M9 — Server actions and transfer.** After M8, implement reference-backed remote rating/flagging/culling, multi-server permissions, offline/queued edits and conflict policy; preserve the existing resumable, integrity-checked upload and make keep/remove after acknowledgement explicit. Test interrupted uploads, duplicates, authorization, concurrent edits, retries, re-pair/revoke and no data loss. Clearly distinguish local Photos, cached previews and authoritative server files.
- [ ] **M10 — Replacement readiness.** Complete the M0 parity matrix; migrate only verified settings/data with a reversible upgrade plan; retain an independent rollback/export path, never overwrite an existing app/data without understanding bundle ID implications. Add repeatable signing, device testing and release packaging; record personal-team limitations, distribution requirements and owner decisions. Run project CI, snapshots, applicable screenshot/mobile/security checks and manual real-iPhone flows, then run an independently observed acceptance pass. Do not claim release readiness until failures and blockers are reported.

## Next slice and execution rules

1. Get access to the handoff folder and do M0. In parallel only with that dependency, M1's reproducible host and safe launch are valid foundation work. Verify current files and devices first; keep the existing uncommitted `entry.rs` change and any other agent's edits intact.
   **Next (2026-11-28, after the PhotoKit import):** the handoff folder still returns `ls: /Users/iancunningham/Documents/sortlens-mac-handoff-20260504-190347-main: Operation not permitted` (macOS EPERM from macOS TCC; Python's `os.listdir` reports `[Errno 1] Operation not permitted`), so M0 remains blocked and its parity matrix stays unwritten. Done so far: (a) **the controls are in the iOS accessibility tree**; (b) **the phone's bottom bar is a fingertip row**; (c) **Search is its own phone screen**; (d) **`More`'s rows are in the iOS accessibility tree** (the M2 progress above). Next, in order: (e) the rest of M3 — paging past the 120 most recent assets, cancel and cache eviction, live limited-selection changes, videos/RAW/HEIC, then the `UIFileSharingEnabled` folder opened in Files by eye; (f) Albums, only once M5's album hierarchy exists — naming an Albums tab before then would be a placeholder. VoiceOver's own reading order on a real device is still M10's acceptance pass.
2. On each scheduled run, read this plan and current repository state, coordinate with active agents, choose one bounded unchecked slice, implement it with a failing-then-passing test and verify the relevant device/simulator behavior when available. Record evidence and the exact next slice; update this file and `docs/ROADMAP.md` when behavior or status changes. The slice ends with `git add`-ing the files this tick owns, a signed-off-by-style commit on the current branch, and `git push` exit 0 — the lease is re-enabled only after the push succeeds. Other agents' uncommitted files may stay dirty in the working tree; that is allowed and must not be staged.
3. Never discard, overwrite, commit or push another person's changes, credentials, certificates, photo libraries or unrelated files. Do not force a clean tree or ship fake checks. If the handoff remains inaccessible, report it and pursue only independent work; do not fabricate its feature inventory. If a genuine blocker prevents the next slice, describe the required access or decision rather than silently ending the whole roadmap.
4. The very first scheduled tick that lands this contract on `main` must include `docs/MOBILE_PLAN.md` itself in a bootstrap commit so the doc is tracked; subsequent ticks read this file from git at the start of the run. The scheduler's STEP 0 disables the lease and STEP LAST re-enables it only after the push — see `docs/scheduled_prompts/sortlens-continue-loop.md` for the full body.
