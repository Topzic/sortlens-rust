//! Screenshot baselines for the shell, painted by the same egui fork the window uses.
//!
//! `cargo xtask screenshots` runs this file; `cargo xtask screenshots --update` rewrites the
//! baselines under `tests/snapshots/` (it is `UPDATE_SNAPSHOTS=1` for `egui_kittest`, the suite's
//! own snapshot crate, so the diffing and the tolerance live in `kittest.toml` rather than here).
//!
//! The harness is opt-in (`sortlens-ui` feature `screenshots`) for one reason: it renders through a
//! real GPU adapter. `cargo xtask ci` stays headless, and this file compiles to nothing without the
//! feature — so the baselines are a separate gate, which is what ROADMAP's Phase 3 item 9 asks for
//! and what the later Phase 3 slices use as their proof.
//!
//! Every scene is the real shell: `sortlens_ui::show` inside an `eframe::App`, the same entry the
//! window uses, at a fixed 1440×900 with `pixels_per_point = 1.0`. The library behind it is a
//! deterministic [`sortlens_testkit::Fixture`]: real JPEGs from a real encoder, fixed ratings,
//! flags, keywords, collections, real EXIF locations and two duplicate frames so Similar has a group.
//! Nothing here mocks the UI — only the photographs.
#![cfg(feature = "screenshots")]
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use egui_kittest::Harness;
use sortlens_core::{ColorLabel, Flag};
use sortlens_engine::{Engine, PhotoId, SIMILAR_THRESHOLD};
use sortlens_testkit::{Fixture, Pattern};
use sortlens_ui::{GridMode, ThemeKind, Thumbnails, UiState, View};

/// The window the baselines are painted at, in points at `pixels_per_point = 1.0`.
const WIDTH: f32 = 1440.0;
const HEIGHT: f32 = 900.0;

/// Both ends of the range the colour presets cover: the neutral dark default and the light one.
/// High Contrast is not in this pair; it has its own test below, because a preset that exists for
/// low vision is checked on the surfaces a person with low vision actually uses, not sprinkled
/// across every scene.
const THEMES: [ThemeKind; 2] = [ThemeKind::Charcoal, ThemeKind::Paper];

/// The shell, exactly as the app drives it: one state, one engine, one texture cache.
struct Scene {
    state: UiState,
    engine: Engine,
    thumbs: Thumbnails,
}

impl eframe::App for Scene {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // `show` is what the window calls; ignoring its actions here is deliberate, because this
        // scene is painted, not driven. A scene that asked for something while merely settling
        // would be a stray event, so this is also the assertion that the baseline is inert.
        let actions = sortlens_ui::show(ui, &mut self.state, &mut self.engine, &mut self.thumbs);
        assert!(actions.is_empty(), "nothing was clicked, so nothing should be asked for");
    }
}

/// A scanned library on disk, plus the library folder the engine keeps its catalog in.
struct Library {
    fixture: Fixture,
    root: String,
    photos: Vec<PhotoId>,
    /// Between scenes the engine rests here; a scene takes it and hands it back, so the catalog is
    /// scanned once and every baseline paints the same ids.
    engine: Option<Engine>,
}

impl Library {
    fn cache(&self) -> PathBuf {
        self.fixture.root().join("Thumbnails")
    }

    fn take(&mut self) -> Engine {
        self.engine.take().expect("the engine is between scenes")
    }

    fn keep(&mut self, engine: Engine) {
        self.engine = Some(engine);
    }
}

/// Build the library every baseline paints: twelve photographs, two of them identical.
///
/// The shapes vary on purpose — a 3:2 frame, a portrait, a panorama, a square — because the
/// justified grid is only worth a baseline if the rows it builds are visibly different. Colours are
/// distinct so a thumbnail painted into the wrong cell is visible at a glance.
fn library() -> Library {
    // A relative path on purpose: the Places view prints the library's tiles folder, and a baseline
    // must not carry the machine's temporary directory. `Fixture::new` would put a process id in it.
    let fixture = Fixture::at("tests/.screenshots");
    let root = fixture.root().join("Photos");

    // file, folder, pattern, width, height, optional EXIF location. A gradient phase per frame, because
    // twelve solid colours would be one structurally identical similar group; IMG_0010 repeats
    // IMG_0009's bytes, which is the duplicate pair the Similar baseline shows.
    let frames: [(&str, &str, Pattern, usize, usize, Option<(f64, f64)>); 12] = [
        ("IMG_0001.jpg", "Trip", Pattern::Gradient(1), 96, 64, Some((50.82, -0.13))),
        ("IMG_0002.jpg", "Studio", Pattern::Gradient(2), 64, 96, None),
        ("IMG_0003.jpg", "Trip", Pattern::Gradient(3), 120, 60, Some((50.83, -0.14))),
        ("IMG_0004.jpg", "Trip", Pattern::Gradient(4), 64, 64, Some((50.81, -0.12))),
        ("IMG_0005.jpg", "Trip", Pattern::Gradient(5), 96, 64, Some((50.84, -0.15))),
        ("IMG_0006.jpg", "Studio", Pattern::Gradient(6), 96, 64, None),
        ("IMG_0007.jpg", "Studio", Pattern::Gradient(7), 80, 64, None),
        ("IMG_0008.jpg", "Studio", Pattern::Gradient(8), 64, 96, Some((-12.05, -77.04))),
        ("IMG_0009.jpg", "Trip", Pattern::Gradient(9), 96, 64, Some((-12.06, -77.03))),
        ("IMG_0010.jpg", "Trip", Pattern::Gradient(9), 96, 64, Some((-12.06, -77.03))),
        ("IMG_0011.jpg", "Trip", Pattern::Gradient(11), 96, 64, None),
        ("IMG_0012.jpg", "Studio", Pattern::Gradient(12), 72, 96, None),
    ];
    for (name, folder, pattern, width, height, place) in frames {
        let rel = format!("Photos/{folder}/{name}");
        fixture.jpeg_with_pattern(&rel, width, height, pattern, place);
    }

    // Two of the twelve say who took them and how. A fixture JPEG normally carries no camera at all, and without one
    // the Suggestions tab could only ever show the pack's family — the grouping would be untested by the one artefact
    // that is supposed to prove it. Rewritten before the scan, so the catalog sees one version of each file.
    fixture.jpeg_with_camera(
        "Photos/Trip/IMG_0001.jpg",
        96,
        64,
        Pattern::Gradient(1),
        sortlens_testkit::Camera {
            make: "Nikon",
            model: "Z6",
            iso: Some(6400),
            focal: Some(24.0),
            place: Some((50.82, -0.13)),
        },
    );
    fixture.jpeg_with_camera(
        "Photos/Studio/IMG_0002.jpg",
        64,
        96,
        Pattern::Gradient(2),
        sortlens_testkit::Camera {
            make: "Fujifilm",
            model: "X-T5",
            iso: None,
            focal: Some(56.0),
            place: None,
        },
    );

    let library_dir = fixture.root().join("Library");
    let mut engine = Engine::open(&library_dir).expect("library opens");
    engine.begin_scan(root.to_string_lossy().to_string()).expect("scan starts");
    // Drain the finished-scan report here rather than in a scene: the shell turns it into a notice
    // naming a temporary folder, and a baseline must not carry a temporary folder's name.
    for _ in 0..1200 {
        if engine.poll().is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(engine.stats().total, 12, "every fixture frame is indexed");
    assert!(engine.poll().is_none(), "the scan report is drained before any scene paints");

    let photos: Vec<PhotoId> = engine.all_photos().iter().map(|photo| photo.id).collect();
    assert_eq!(photos.len(), 12);
    assert!(
        engine.all_photos().iter().any(|photo| photo.width == 120 && photo.height == 60),
        "the fixture's EXIF dimensions reach the record, so the justified rows have real aspects"
    );

    // A library that has been lived in: ratings, two picks, a reject, keywords, and a collection.
    // Each is a real op through the engine, so the sidebar counts are the real counts.
    engine.rate(&photos[0..5], 5).expect("rate");
    engine.rate(&photos[5..8], 3).expect("rate");
    engine.flag(&photos[0..2], Flag::Pick).expect("pick");
    engine.flag(&photos[11..12], Flag::Reject).expect("reject");
    engine.set_tag(&photos[0..1], "sea", true).expect("keyword");
    engine.set_tag(&photos[2..3], "sea", true).expect("keyword");
    engine.set_tag(&photos[7..8], "portrait", true).expect("keyword");
    let keepers = engine.create_collection("Keepers").expect("collection");
    engine.add_to_collection(&photos[0..4], keepers).expect("collection members");

    // A keyword pack, switched on, because a fixture JPEG carries no EXIF: without it the Suggestions tab and the
    // detail view's suggestion row would be empty in every baseline, and an empty panel is not evidence that the
    // panel works. Two rules, one per fixture folder, so a selection spanning both folders shows two families at
    // once and a keyword that applies to only part of the selection prints its count.
    std::fs::create_dir_all(library_dir.join(sortlens_engine::PACKS_DIR)).expect("packs dir");
    std::fs::write(
        library_dir.join(sortlens_engine::PACKS_DIR).join("coast.json"),
        r#"{"name":"Coast","licence":"CC0-1.0","keywords":[{"keyword":"shoreline","when":{"folder_contains":"Trip"}},{"keyword":"golden hour","when":{"folder_contains":"Trip"}},{"keyword":"studio light","when":{"folder_contains":"Studio"}}]}"#,
    )
    .expect("write pack");
    engine.set_pack("Coast", true).expect("enable the pack");
    assert!(
        !engine.suggestions(photos[0]).is_empty(),
        "the Suggestions baselines paint real proposals, not an empty state"
    );

    // The hash pass runs once, before anything is painted, so the Similar baseline is a settled
    // result rather than a progress bar.
    engine.begin_similar(SIMILAR_THRESHOLD).expect("hash pass starts");
    for _ in 0..2400 {
        if engine.poll_similar(SIMILAR_THRESHOLD).is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!engine.similar_groups().is_empty(), "the duplicate pair is a group");

    Library {
        root: root.to_string_lossy().to_string(),
        photos,
        fixture,
        engine: Some(engine),
    }
}

/// One scene on a fresh state: a harness owns its scene, so a scene is painted once.
fn new_scene(kind: ThemeKind, engine: Engine, cache: &Path) -> Scene {
    Scene {
        state: UiState::new(kind, 0.0),
        engine,
        thumbs: Thumbnails::new(cache.to_path_buf()),
    }
}

/// Scope the scene to the library's own folder.
///
/// The folder tree, the breadcrumb and the folder-scoped counts are part of what a baseline is for,
/// and opening a folder is the normal way a person looks at a shoot. The welcome scene stays
/// unscoped, because a first launch has no folder at all.
fn scoped(mut scene: Scene, root: &str) -> Scene {
    scene.state.root = Some(root.to_string());
    scene
}

/// Let the decode workers land, then let the shell settle.
///
/// `Harness::run` stops when nothing asks for another frame, and a thumbnail still decoding is
/// exactly that: silence. So the harness is stepped until the texture cache has been unchanged and
/// the queue empty for a while, and only then settled by `run` — twice, because settling can ask for
/// a thumbnail (the detail view prefetches the next frame) that was not queued before.
fn settle(harness: &mut Harness<'_, Scene>) {
    pump(harness);
    harness.run();
    pump(harness);
    assert_eq!(harness.state().thumbs.queue().0, 0, "every thumbnail asked for was decoded");
}

fn pump(harness: &mut Harness<'_, Scene>) {
    let mut quiet = 0;
    let mut resident = usize::MAX;
    for _ in 0..1200 {
        harness.step();
        let now = harness.state().thumbs.resident();
        let queued = harness.state().thumbs.queue().0;
        if now == resident && queued == 0 {
            quiet += 1;
            if quiet >= 20 {
                return;
            }
        } else {
            quiet = 0;
            resident = now;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The variable that names a baseline set other than the committed one.
///
/// The baselines at `tests/snapshots/<name>.png` are painted by the reference rasteriser. A run whose
/// adapter paints the same scene a few pixels differently — CI's Mesa software Vulkan is the case this
/// exists for — names its own set, and every scene then compares against
/// `tests/snapshots/<set>/<name>.png`. What changes is the set, never the tolerance: both are compared
/// byte for byte, so a set is either painted by that adapter or absent.
const SNAPSHOT_SET_ENV: &str = "SORTLENS_SNAPSHOT_SET";

/// The baseline a scene compares against, given the set this run names.
///
/// An unset or blank set is the committed one, which is what a developer's own machine compares with.
fn baseline_name(name: &str) -> String {
    let set = std::env::var(SNAPSHOT_SET_ENV).unwrap_or_default();
    scoped_name(&set, name)
}

/// The same, with the set passed in: edition 2024 makes `set_var` unsafe and this file forbids unsafe
/// code, so the rule is testable only by handing it the value.
fn scoped_name(set: &str, name: &str) -> String {
    let set = set_dir(set);
    if set.is_empty() {
        name.to_string()
    } else {
        format!("{set}/{name}")
    }
}

/// A set name as a directory: lower case, one dash per run of anything that is not a letter or a digit.
///
/// The value comes from a workflow written by hand, and it becomes a directory under
/// `tests/snapshots/` — so `Mesa LLVM 15.0` is one folder rather than two, and `../..` is no folder
/// at all instead of an escape from the baselines.
fn set_dir(set: &str) -> String {
    let mut out = String::new();
    for ch in set.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Paint one scene at the fixed window size and compare it with its baseline.
///
/// Returns the engine, so the next scene paints the same library without scanning it again.
fn shoot(kind: ThemeKind, scene: Scene, name: &str) -> Engine {
    shoot_at(kind, scene, name, egui::vec2(WIDTH, HEIGHT))
}

/// The same, at another window size: the narrow-window layout is a baseline too, not a claim.
fn shoot_at(kind: ThemeKind, scene: Scene, name: &str, size: egui::Vec2) -> Engine {
    let mut harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(1.0)
        .with_max_steps(240)
        .with_context_setup(move |ctx| sortlens_ui::apply(ctx, kind))
        .wgpu()
        .build_eframe(|_cc| scene);
    settle(&mut harness);
    harness
        .try_snapshot(baseline_name(name))
        .expect("the baseline matches, or its .new/.diff companions say exactly where it does not");
    harness.into_state().engine
}

/// The same, with tofu instead of a panic for glyphs the CI font build does not have.
///
/// The Settings dialog's language picker names every language in itself, so it paints Arabic and
/// Japanese whether or not the machine has those faces — which CI's does not. The baseline is
/// still a full read of the section; the tofu boxes are the documented gap (RTL screenshots wait
/// for an Arabic-capable font build), not a silent defect.
fn shoot_at_lenient(kind: ThemeKind, scene: Scene, name: &str, size: egui::Vec2) -> Engine {
    let mut harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(1.0)
        .with_max_steps(240)
        .with_context_setup(move |ctx| sortlens_ui::apply(ctx, kind))
        .allow_missing_glyphs()
        .wgpu()
        .build_eframe(|_cc| scene);
    settle(&mut harness);
    harness
        .try_snapshot(baseline_name(name))
        .expect("the baseline matches, or its .new/.diff companions say exactly where it does not");
    harness.into_state().engine
}

/// Which set a scene compares against, and what a set name may be.
///
/// These are the two rules the gate rests on when it runs anywhere but the reference machine, and they cost
/// a second to check: no paint, no adapter, no window.
#[test]
fn a_baseline_set_scopes_every_scene_to_its_own_directory() {
    assert_eq!(scoped_name("", "library-charcoal"), "library-charcoal");
    assert_eq!(scoped_name("   ", "library-charcoal"), "library-charcoal");
    assert_eq!(scoped_name("lavapipe", "library-charcoal"), "lavapipe/library-charcoal");
    // A hand-written workflow value still lands in one plain directory: case, spaces and dots are not a
    // path with a stray folder in it.
    assert_eq!(scoped_name("Mesa LLVM 15.0", "x"), "mesa-llvm-15-0/x");
    assert_eq!(scoped_name("  //  ", "x"), "x", "a name with no letter or digit is no set");
    assert_eq!(scoped_name("../..", "x"), "x", "a set cannot climb out of the baselines");
}

/// The Suggestions tab, with real proposals from the switched-on pack for a selection spanning both folders.
///
/// This is Phase 3B.5's visual proof: the fourth tab on the far-edge rail, the panel's header naming it, one family
/// per source with its confidence, and the `2×` that says a keyword applies to part of the selection.
#[test]
fn suggestions_paints_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        // Picked by what each record says, not by id: the scan enumerates the filesystem and the ids do not follow the
        // order this file writes the frames in. The two camera frames plus one that has only the pack, so the baseline
        // shows both families, a count that is not the selection size, and the difference between them at a glance.
        let engine = library.take();
        let mut chosen: Vec<PhotoId> = engine
            .all_photos()
            .iter()
            .filter(|photo| !photo.camera.is_empty())
            .map(|photo| photo.id)
            .collect();
        assert_eq!(chosen.len(), 2, "two frames carry a camera");
        let silent = engine
            .all_photos()
            .iter()
            .find(|photo| photo.camera.is_empty())
            .map(|photo| photo.id)
            .expect("a frame with no camera exists");
        chosen.push(silent);
        let mut scene = scoped(new_scene(kind, engine, &library.cache()), &library.root);
        scene.state.right_tab = sortlens_ui::RightTab::Suggestions;
        scene.state.selection.select_many(&chosen);
        library.keep(shoot(kind, scene, &format!("suggestions-{}", kind_id(kind))));
    }
}

/// Settings → Sync & devices, in the state the desktop's Start button ends in: the port and the
/// live pairing code the desktop learned from the process's own stdout, and one paired device
/// with the Revoke button that stays disabled while the server holds its own token store.
/// Painted taller than the window because the Settings dialog is longer than one screen: the
/// section is the dialog's tail, and a baseline that clips its subject proves nothing.
#[test]
fn settings_sync_paints_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        scene.state.settings_open = true;
        scene.state.server = sortlens_ui::ServerSurface {
            running: true,
            port: Some(52_114),
            code: Some("448821".to_owned()),
            devices: vec![sortlens_ui::DeviceRow {
                token: "a1b2c3d4".to_owned(),
                device: "Ian's phone".to_owned(),
                scopes: "read,annotate".to_owned(),
                since: "2026-11-24".to_owned(),
            }],
        };
        library.keep(shoot_at_lenient(
            kind,
            scene,
            &format!("settings-sync-{}", kind_id(kind)),
            egui::vec2(900.0, 1700.0),
        ));
    }
}

/// Settings at a laptop's height: the dialog is longer than 800 points, and it used to run past the
/// bottom of the screen, putting Updates, Sync and the keyword packs out of reach. The body scrolls
/// inside the dialog now, so the frame ends above the docks and the body keeps its own scrollbar.
#[test]
fn a_short_window_scrolls_the_settings_dialog_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        scene.state.settings_open = true;
        // Lenient for the language row's own scripts, as the 1700-point Settings baseline is.
        library.keep(shoot_at_lenient(
            kind,
            scene,
            &format!("settings-short-{}", kind_id(kind)),
            egui::vec2(900.0, 620.0),
        ));
    }
}

/// The empty state a first launch shows: no folder, no photographs, the welcome copy.
#[test]
fn empty_welcome_paints_in_both_themes() {
    let library = library();
    for kind in THEMES {
        let dir = library.fixture.root().join(format!("Empty-{}", kind_id(kind)));
        let mut engine = Engine::open(&dir).expect("an empty library opens");
        assert_eq!(engine.stats().total, 0);
        let scene = new_scene(kind, engine, &library.cache());
        // The scene's engine is a fresh, empty library, so it is dropped rather than handed back.
        drop(shoot(kind, scene, &format!("welcome-{}", kind_id(kind))));
    }
}

/// The library grid in both layouts: justified rows and the square lattice, same photographs.
#[test]
fn library_grid_paints_both_layouts_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        library.keep(shoot(kind, scene, &format!("library-justified-{}", kind_id(kind))));

        let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        scene.state.grid_mode = GridMode::Square;
        library.keep(shoot(kind, scene, &format!("library-square-{}", kind_id(kind))));
    }
}

/// The accessibility preset on the two ends of the shell: the grid with its marks, and the empty
/// welcome. Phase 4 item 2b's visual proof — pure black and white at doubled strokes, checked like
/// any other preset rather than asserted to be readable in prose.
#[test]
fn high_contrast_paints_the_grid_and_the_welcome() {
    let mut library = library();
    let kind = ThemeKind::HighContrast;
    let scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
    library.keep(shoot(kind, scene, &format!("library-justified-{}", kind_id(kind))));

    let dir = library.fixture.root().join(format!("Empty-{}", kind_id(kind)));
    let mut engine = Engine::open(&dir).expect("an empty library opens");
    assert_eq!(engine.stats().total, 0);
    let scene = new_scene(kind, engine, &library.cache());
    drop(shoot(kind, scene, &format!("welcome-{}", kind_id(kind))));
}

/// The Missing scope: the one source that asks the filesystem rather than an annotation.
///
/// One frame's file is removed from disk and the health check marks the record, so the sidebar's Missing
/// row counts a file that really went away and the grid holds that record as it is painted for a file
/// nobody can open. This is Phase 3B.3's visual proof: the six scopes on top, the utility strip at the
/// foot, and a scope whose count is not the library total.
#[test]
fn missing_scope_paints_in_both_themes() {
    let mut library = library();
    // The file leaves the library; the record stays, marked missing. A path rather than an index,
    // because the fixture's scan order is the filesystem's, not this file's.
    std::fs::remove_file(Path::new(&library.root).join("Studio/IMG_0006.jpg")).expect("remove a frame");
    let mut engine = library.take();
    engine.check_health().expect("health check");
    assert_eq!(engine.stats().missing, 1, "one frame's file is gone");
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, engine, &library.cache()), &library.root);
        scene.state.apply_source(sortlens_ui::Source::Missing);
        // The scope's own question, asked of the same engine the baseline is painted from.
        assert_eq!(
            scene.engine.count(&sortlens_engine::Filter {
                missing: Some(true),
                ..sortlens_engine::Filter::default()
            }),
            1,
            "the scope holds the one file that is gone"
        );
        engine = shoot(kind, scene, &format!("missing-{}", kind_id(kind)));
    }
    library.keep(engine);
}

/// The detail view, alone and beside a second frame: the canvas, the facts, the histogram and the
/// compare header are all visible in the two baselines.
#[test]
fn detail_and_compare_paint_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        scene.state.view = View::Detail;
        scene.state.selection.replace(library.photos[0]);
        library.keep(shoot(kind, scene, &format!("detail-{}", kind_id(kind))));

        let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        scene.state.view = View::Compare;
        scene.state.selection.replace(library.photos[0]);
        scene.state.compare = Some(library.photos[1]);
        library.keep(shoot(kind, scene, &format!("compare-{}", kind_id(kind))));
    }
}

/// Cull: the canvas, the up-next strip and the action bar with live counts.
#[test]
fn cull_paints_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let mut engine = library.take();
        engine.begin_cull(&library.photos).expect("a cull session starts");
        let mut scene = scoped(new_scene(kind, engine, &library.cache()), &library.root);
        scene.state.view = View::Cull;
        library.keep(shoot(kind, scene, &format!("cull-{}", kind_id(kind))));
    }
}

/// Similar: a settled hash pass with its duplicate group in the list.
#[test]
fn similar_paints_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        scene.state.view = View::Similar;
        library.keep(shoot(kind, scene, &format!("similar-{}", kind_id(kind))));
    }
}

/// Places: the shipped coastline under dots from real EXIF, with the credit line that travels with it.
///
/// The scene pins zoom 3 rather than letting the view fit the library, because a baseline must not move
/// when the fitting arithmetic is touched: the fit itself is proved by `sortlens-geo`'s unit tests and
/// by the shell's `run_ui` test, and this baseline is here to show what the outline and the dots look
/// like in both presets. With the pin removed the view fits Brighton and Lima into the viewport on its
/// own, which is the behaviour the fit exists for.
#[test]
fn places_paints_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        scene.state.view = View::Places;
        scene.state.places_zoom = 3;
        scene.state.places_zoom_chosen = true;
        library.keep(shoot(kind, scene, &format!("places-{}", kind_id(kind))));
    }
}

/// Command mode: the ⌘K palette hung under the search field, grouped by the registry's own groups
/// and printing the shortcuts the registry declares. This is the top bar's new shape too — the mark,
/// the glyph buttons and the one field — so the two baselines are the proof of Phase 3 item 11.
#[test]
fn command_palette_paints_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        scene.state.palette_open = true;
        scene.state.palette_query = "library".to_string();
        library.keep(shoot(kind, scene, &format!("palette-{}", kind_id(kind))));
    }
}

/// The shortcuts overlay, narrowed to the arrow keys: every keycap the search can surface.
///
/// This is 3B.6 (g)'s proof: the triangles that replaced `←`/`→` — glyphs the proportional face a
/// default build runs on does not have — are visible as real glyphs in both presets, in the keycap
/// column the overlay paints in monospace. The query is the triangles themselves, so the three rows
/// that spell them are what the baseline is of; the full table is covered by the headless glyph
/// audits, and an empty-query run of this scene showed it clean before this became the subject.
#[test]
fn shortcuts_paint_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        scene.state.shortcuts_open = true;
        scene.state.shortcut_query = "◀".to_string();
        library.keep(shoot(kind, scene, &format!("shortcuts-{}", kind_id(kind))));
    }
}

/// The shortcut editor (SL-I09): craft-keymap's dialog over the app's catalog, with the set row,
/// the table grouped the way the overlay groups it, and the chords the runtime keymap holds.
///
/// The editor is the surface Settings → Shortcuts opens; the baseline is of its first screen, the
/// factory bindings, so a rebind that left the overlay and palette stale would show here too.
#[test]
fn shortcut_editor_paints_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        scene.state.shortcut_editor = Some(craft_keymap::ui::DialogState::new());
        library.keep(shoot(kind, scene, &format!("shortcut-editor-{}", kind_id(kind))));
    }
}

/// Soft Focus: the relative sharpness review as its own destination, with stored scores.
///
/// A library with measured scores shows the histogram and the cut it would select; this is the proof that
/// Phase 3B.4's peer view is the review that used to be the last thing you scrolled to on Insights.
#[test]
fn soft_focus_paints_in_both_themes() {
    let mut library = library();
    let mut engine = library.take();
    let scores: Vec<(PhotoId, sortlens_quality::Score)> = library
        .photos
        .iter()
        .enumerate()
        .map(|(index, id)| {
            (
                *id,
                sortlens_quality::Score {
                    value: 0.4 + index as f32 * 0.35,
                    version: sortlens_quality::SCORE_VERSION,
                },
            )
        })
        .collect();
    engine.set_scores(&scores).expect("scores");
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, engine, &library.cache()), &library.root);
        scene.state.view = View::SoftFocus;
        engine = shoot(kind, scene, &format!("soft-focus-{}", kind_id(kind)));
    }
    library.keep(engine);
}

/// Insights: the figures and the distributions, from the same library the grid shows.
#[test]
fn insights_paints_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        scene.state.view = View::Insights;
        library.keep(shoot(kind, scene, &format!("insights-{}", kind_id(kind))));
    }
}

/// A window too narrow for nine labelled destinations: the row collapses to the current one plus a
/// menu, and the pill and the right-hand controls keep their places. Phase 3B.4's other half.
#[test]
fn a_narrow_window_collapses_the_tab_row() {
    let mut library = library();
    let kind = ThemeKind::Charcoal;
    let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
    // The panels are closed so the baseline is about the bar rather than about a canvas squeezed to
    // 340 px by two docks. A narrow window with its docks hidden is a real way to use the app.
    scene.state.left_open = false;
    scene.state.right_open = false;
    library.keep(shoot_at(kind, scene, "narrow-charcoal", egui::vec2(900.0, 620.0)));
}

/// A phone: 390 points wide, both docks put away, and the grid on the screen.
///
/// The scene starts with the docks open, the way a desktop does, so the baseline is also the proof that a window
/// this narrow puts them away by itself instead of painting two panels and no canvas. What a phone does not have
/// yet — a canvas header, a bottom bar and a grid tray that are still the desktop's — is open work, recorded in the
/// roadmap rather than described here as done.
#[test]
fn a_phone_window_opens_on_the_grid() {
    let mut library = library();
    let kind = ThemeKind::Charcoal;
    let scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
    library.keep(shoot_at(kind, scene, "phone-charcoal", egui::vec2(390.0, 780.0)));
}

/// The sidebar on a phone: the dock takes the screen, and the bar keeps the glyph that gives it back.
///
/// One dock at a time is the phone's rule, so the inspector is not behind it: the baseline is what a tap on the
/// library glyph answers, and the grid is what a second tap returns to.
#[test]
fn a_phone_sidebar_takes_the_screen() {
    let mut library = library();
    let kind = ThemeKind::Charcoal;
    let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
    scene.state.narrow = true;
    scene.state.left_open = true;
    scene.state.right_open = false;
    library.keep(shoot_at(kind, scene, "phone-sidebar-charcoal", egui::vec2(390.0, 780.0)));
}

/// A canvas squeezed by both docks: the header keeps its name whole and its controls, and gives up the rest.
///
/// 3B.6 (e) was reported at 900 points with both docks open, where the header's left block was handed the
/// width the right block left over and, when that ran out, was painted underneath the sort combo. Laid out
/// with `Sides::shrink_left` the controls are measured first: the name is whole, the sort combo and the two
/// grid glyphs are all still there, and the count and the two chips — the row's statements, both repeated
/// elsewhere — are the things that step aside. This is the baseline a person can read: at 1440 the same
/// header shows all four.
#[test]
fn a_narrow_canvas_keeps_the_header_name_whole() {
    let mut library = library();
    for kind in THEMES {
        let scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        library.keep(shoot_at(
            kind,
            scene,
            &format!("narrow-canvas-{}", kind_id(kind)),
            egui::vec2(900.0, 620.0),
        ));
    }
}

/// A short window with the sidebar open: the sections scroll above the pinned foot instead of painting over it.
///
/// 3B.6 (d) was reported at ~620 px of height, where the six scopes, the two subfolders and the keyword and
/// collection rows are taller than the panel. The baseline is the proof a person can read: the four utility
/// rows sit on the bottom edge with clean space above them, the last visible section row is clipped at the
/// hairline rather than drawn across it, and the scroll bar says the rest is still there.
#[test]
fn a_short_window_scrolls_the_sidebar_above_its_foot_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        library.keep(shoot_at(
            kind,
            scene,
            &format!("short-sidebar-{}", kind_id(kind)),
            egui::vec2(1280.0, 620.0),
        ));
    }
}

/// The filter strip while it is asking something: three stars and up, and one colour label.
///
/// The resting strip appears in every other baseline; this is the one that proves its *active* state is
/// pictures — three lit stars instead of a numeral and a typed `★`, the label's own colour in the
/// swatch instead of a droplet — and, at 1200 points with both docks open, that the row wraps rather
/// than pushing `Clear` off the canvas edge.
#[test]
fn the_filter_strip_paints_its_state_in_both_themes() {
    let mut library = library();
    let photos = library.photos.clone();
    let mut engine = library.take();
    // Two of the five-star frames carry a colour label, so the filtered answer is not empty and the
    // grid behind the strip is two photographs rather than the Welcome card.
    engine.label(&photos[0..2], Some(ColorLabel::Green)).expect("label");
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, engine, &library.cache()), &library.root);
        scene.state.min_rating = 3;
        scene.state.label_filter = Some(ColorLabel::Green);
        engine = shoot_at(kind, scene, &format!("filters-{}", kind_id(kind)), egui::vec2(1200.0, 800.0));
    }
    library.keep(engine);
}

/// A library filtered down to nothing: the screen that explains it, with the count and one way back.
///
/// Painted at 1200×800 with both docks open, the width at which the filter strip's own row has to wrap: the strip
/// above says `Videos · 0 shown`, so the picture is the whole sentence — the question that was asked, the
/// library's real size, and the single button that answers it. The Welcome card must not be in this baseline.
#[test]
fn a_filtered_to_nothing_library_explains_itself_in_both_themes() {
    let mut library = library();
    for kind in THEMES {
        let mut scene = scoped(new_scene(kind, library.take(), &library.cache()), &library.root);
        // The fixture is twelve JPEGs, so a video-only view is zero of twelve — a real filter, not a fake one.
        scene.state.kind_filter = Some(sortlens_core::MediaKind::Video);
        assert_eq!(
            scene.engine.count(&scene.state.filter()),
            0,
            "the baseline's scene really is filtered to nothing"
        );
        library.keep(shoot_at(
            kind,
            scene,
            &format!("no-matches-{}", kind_id(kind)),
            egui::vec2(1200.0, 800.0),
        ));
    }
}

/// A stable name fragment per preset, so a baseline file name does not depend on enum order.
fn kind_id(kind: ThemeKind) -> &'static str {
    match kind {
        ThemeKind::Charcoal => "charcoal",
        ThemeKind::Midnight => "midnight",
        ThemeKind::Silver => "silver",
        ThemeKind::Paper => "paper",
        ThemeKind::HighContrast => "high-contrast",
    }
}
