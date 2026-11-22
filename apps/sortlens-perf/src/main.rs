//! Developer measurements, printed as a table: `cargo run -p sortlens-perf --release`.
//!
//! This exists so `docs/PERF.md` can quote numbers that anyone can reproduce with the same command on
//! their own machine, rather than numbers typed from memory. It is deliberately not part of `cargo xtask
//! ci`: a timing that fails on a loaded CI box teaches people to ignore failures.
//!
//! What it measures, and what it therefore cannot claim:
//!
//! * scan and index are real files on disk, through the same functions the app uses;
//! * thumbnails are decoded cold (empty cache) and read back warm (cache hit), through `sortlens-thumbs`;
//! * suggestions are the real per-photo pass over `sortlens-ai`, with no model — EXIF hints and packs
//!   only, which is all that ships;
//! * frames are `Context::run_ui` over the real `show()`, without a GPU and without texture uploads, so
//!   they measure layout and shape building, not what a display does with them. There is no vsync here
//!   and no 60 Hz budget to check against.

#![forbid(unsafe_code)]

use sortlens_testkit::Fixture;
use std::path::Path;
use std::time::{Duration, Instant};

/// What to measure. Small defaults so a run finishes while the coffee is still hot.
struct Options {
    photos: usize,
    thumbs: usize,
    frames: usize,
}

impl Options {
    fn parse() -> Result<Self, String> {
        let mut options = Options {
            photos: 400,
            thumbs: 40,
            frames: 30,
        };
        let mut args = std::env::args().skip(1);
        while let Some(flag) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("{flag} needs a number"))?
                .parse::<usize>()
                .map_err(|error| format!("{flag}: {error}"))?;
            match flag.as_str() {
                "--photos" => options.photos = value.max(1),
                "--thumbs" => options.thumbs = value.max(1),
                "--frames" => options.frames = value.max(1),
                other => return Err(format!("unknown flag {other}")),
            }
        }
        Ok(options)
    }
}

fn main() -> Result<(), String> {
    let options = Options::parse()?;
    let root = std::env::temp_dir().join(format!("sortlens-perf-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).map_err(|error| format!("{}: {error}", root.display()))?;

    println!(
        "sortlens-perf: {} photos, {} thumbnails, {} frames",
        options.photos, options.thumbs, options.frames
    );
    println!("host: {} {}", std::env::consts::OS, std::env::consts::ARCH);
    match std::thread::available_parallelism() {
        Ok(threads) => println!("logical cores: {}", threads.get()),
        Err(error) => println!("logical cores: unknown ({error})"),
    }
    println!(
        "build: {}",
        if cfg!(debug_assertions) {
            "debug (timings are not representative)"
        } else {
            "release"
        }
    );

    // Fixtures: real JPEGs, one per photo, in two folders so the scan has a tree to walk rather than a list.
    let fixture = Fixture::new("perf");
    let built = Instant::now();
    let mut files = Vec::with_capacity(options.photos);
    for index in 0..options.photos {
        let rel = format!("shoot-{}/IMG_{index:05}.jpg", index % 4);
        let jpeg = noisy_jpeg(index, 1200, 800);
        files.push(fixture.write(&rel, &jpeg));
    }
    println!("fixtures: {} in {:?}", options.photos, built.elapsed());
    println!();

    println!("{:<34} {:>12} {:>14} {:>10}", "measurement", "time", "per item", "items/s");

    // 1. Scan: directory walk plus metadata, no decoding.
    let started = Instant::now();
    let report = sortlens_scan::scan_root(fixture.root()).map_err(|error| error.to_string())?;
    let scan = started.elapsed();
    row("scan (walk + stat)", scan, report.files.len(), "files");

    // 2. Index: scan through the engine, which also reads EXIF and applies the catalog op.
    let library = root.join("library");
    let mut engine = sortlens_engine::Engine::open(&library).map_err(|error| error.to_string())?;
    let started = Instant::now();
    engine
        .begin_scan(fixture.root().to_string_lossy().to_string())
        .map_err(|error| error.to_string())?;
    let indexed = poll_scan(&mut engine)?;
    let index = started.elapsed();
    row("index (scan + exif + catalog)", index, indexed, "photos");
    println!("{:<34} {:>12}", "  photos in catalog", engine.stats().total);

    // 2b. Cold open: what launching the app costs before it can draw anything. The library is dropped
    // first, so this reads the snapshot and the recent log rather than reusing what is in memory.
    drop(engine);
    let started = Instant::now();
    let mut reopened = sortlens_engine::Engine::open(&library).map_err(|error| error.to_string())?;
    let opened = started.elapsed();
    let counted = reopened.stats().total;
    row("open library (snapshot + log)", opened, counted, "photos");
    engine = reopened;
    println!();

    // 3. Cold thumbnails: empty cache, so every decode is real work.
    let cache = root.join("thumbs");
    let started = Instant::now();
    let mut cold = 0;
    for path in files.iter().take(options.thumbs) {
        if sortlens_thumbs::decode_file(path, sortlens_thumbs::GRID).is_ok() {
            cold += 1;
        }
    }
    let cold_time = started.elapsed();
    if let Some(first) = files.first() {
        let _ = sortlens_thumbs::decode_file(first, sortlens_thumbs::GRID);
        println!("{:<34} {:>12}", "  sample file", human_size(file_size(first)));
    }
    row("thumbnail decode, cold (128 px)", cold_time, cold, "thumbs");

    // 4. Warm thumbnails: the cache path the grid actually hits after the first scroll.
    let cache = sortlens_thumbs::Cache::new(&cache);
    let mut keys = Vec::new();
    for (index, path) in files.iter().take(options.thumbs).enumerate() {
        if let Ok(thumbnail) = sortlens_thumbs::decode_file(path, sortlens_thumbs::GRID) {
            let key = index as u64 + 1;
            if cache.put(key, sortlens_thumbs::GRID, &thumbnail).is_ok() {
                keys.push(key);
            }
        }
    }
    let started = Instant::now();
    let mut warm = 0;
    for key in &keys {
        if cache.get(*key, sortlens_thumbs::GRID).is_some() {
            warm += 1;
        }
    }
    row("thumbnail read, warm (cache hit)", started.elapsed(), warm, "thumbs");
    let _ = sortlens_thumbs::Cache::clear(&cache);

    // 4b. The path the grid actually uses: two worker threads behind the service, so this is the rate at
    // which a first scroll fills, including the handoff and the cache write.
    let mut service = sortlens_thumbs::Service::new(sortlens_thumbs::Cache::new(root.join("service-thumbs")), 2);
    let started = Instant::now();
    for (index, path) in files.iter().take(options.thumbs).enumerate() {
        service.request(index as u64 + 11, path.clone(), sortlens_thumbs::GRID);
    }
    let mut loaded = 0;
    let deadline = Instant::now() + Duration::from_secs(600);
    while loaded < options.thumbs && Instant::now() < deadline {
        loaded += service.poll().len();
        if loaded < options.thumbs {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    row("thumbnail service, 2 workers", started.elapsed(), loaded, "thumbs");
    println!();

    // 5. Suggestions: the no-model half, which is what ships for every photo.
    let ids: Vec<sortlens_core::PhotoId> = engine
        .page(
            &sortlens_engine::Filter::default(),
            sortlens_engine::Sort::Name,
            0,
            engine.count(&sortlens_engine::Filter::default()),
        )
        .photos
        .iter()
        .map(|photo| photo.id)
        .collect();
    let started = Instant::now();
    let suggested = engine.suggestions_for_all(&ids);
    let suggest = started.elapsed();
    let offered: usize = suggested.iter().map(|(_, list)| list.len()).sum();
    row("suggestions, as indexed", suggest, suggested.len(), "photos");
    println!("{:<34} {:>12}", "  proposals offered", offered);
    println!(
        "  fixtures carry no EXIF, so that pass had nothing to read; the cost below is a pass \n  \
         with real camera, lens, ISO and focal length on every photo"
    );
    let exif: Vec<sortlens_engine::Photo> = (0..suggested.len().max(1))
        .map(|index| {
            let mut photo = photo_with_exif(sortlens_core::PhotoId(index as u64 + 1), index);
            photo.camera = "X-T5".to_string();
            photo.lens = "XF 23mm f/1.4".to_string();
            photo.iso = Some(6400);
            photo.focal = Some(23.0);
            photo.shutter = Some(1.0 / 30.0);
            photo
        })
        .collect();
    let started = Instant::now();
    let mut proposals = 0;
    for photo in &exif {
        // The same composition the engine performs, spelled out here because `Engine::suggestions` reads
        // from the catalog and these photos exist only in memory.
        let hints = sortlens_ai::Hints::new(&photo.camera, &photo.lens, photo.iso, photo.shutter, photo.focal);
        proposals += sortlens_ai::merge(&sortlens_ai::hints(&hints), &photo.tags, sortlens_ai::Caps::default()).len();
    }
    let exif_pass = started.elapsed();
    row("suggestions, with EXIF", exif_pass, exif.len(), "photos");
    println!("{:<34} {:>12}", "  proposals offered", proposals);
    println!();

    // 6. Headless frames: the real shell, one frame per measurement, no GPU, no texture uploads.
    let ctx = egui::Context::default();
    let mut state = sortlens_ui::UiState::new(sortlens_ui::ThemeKind::Midnight, 0.0);
    let mut thumbs = sortlens_ui::Thumbnails::new(root.join("ui-thumbs"));
    let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1440.0, 900.0));
    let input = egui::RawInput {
        screen_rect: Some(viewport),
        ..egui::RawInput::default()
    };
    let mut frame_times = Vec::with_capacity(options.frames);
    for _ in 0..options.frames {
        let started = Instant::now();
        let mut output = ctx.run_ui(input.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let _ = sortlens_ui::show(ui, &mut state, &mut engine, &mut thumbs);
            });
        });
        // No window means nowhere for a texture to go; drop it rather than leak it into the timing.
        output.textures_delta.clear();
        frame_times.push(started.elapsed());
    }
    // The first frame installs the fonts and allocates every area, so it is reported on its own rather
    // than folded into a percentile that would then describe startup rather than scrolling.
    let first = frame_times.remove(0);
    frame_times.sort_unstable();
    let median = frame_times[frame_times.len() / 2];
    let p95 = frame_times[(frame_times.len() * 95 / 100).min(frame_times.len() - 1)];
    println!(
        "{:<34} {:>12} {:>14} {:>10}",
        "grid frame, 1440x900 (headless)",
        format!("{:.2} ms", median.as_secs_f64() * 1000.0),
        format!("p95 {:.2} ms", p95.as_secs_f64() * 1000.0),
        format!("{:.0}/s", 1.0 / median.as_secs_f64().max(f64::MIN_POSITIVE))
    );
    println!(
        "{:<34} {:>12}",
        "  first frame (fonts, first layout)",
        format!("{:.2} ms", first.as_secs_f64() * 1000.0)
    );
    println!("{:<34} {:>12}", "  textures resident", thumbs.resident());

    let _ = std::fs::remove_dir_all(&root);
    Ok(())
}

/// Print one measured row: total time, time per item and items per second, all from the same run.
///
/// Microunits when the number is small: a suggestion pass costs microseconds per photo, and printing
/// `0.00 ms` for it would look like a measurement that did not run.
fn row(label: &str, elapsed: Duration, items: usize, noun: &str) {
    let seconds = elapsed.as_secs_f64();
    let per_item = if items == 0 { 0.0 } else { seconds / items as f64 };
    let rate = if seconds == 0.0 { 0.0 } else { items as f64 / seconds };
    let total = if seconds < 0.010 {
        format!("{:.1} µs", seconds * 1_000_000.0)
    } else {
        format!("{:.0} ms", seconds * 1000.0)
    };
    let each = if per_item < 0.010 {
        format!("{:.1} µs/{noun}", per_item * 1_000_000.0)
    } else {
        format!("{:.2} ms/{noun}", per_item * 1000.0)
    };
    println!("{label:<34} {total:>12} {each:>14} {rate:>10.0}");
}

/// A JPEG with real detail in it.
///
/// `Fixture::jpeg` paints one colour, which compresses to almost nothing and decodes without doing any
/// real work — a thumbnail rate measured on it would be a rate for the easiest possible file. This draws a
/// gradient with deterministic noise, so the decode has coefficients to reconstruct and a Huffman stream
/// to walk, and the same seed gives the same file on every machine.
fn noisy_jpeg(seed: usize, width: usize, height: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(width * height * 3);
    let mut state = 0x2545_f491_4f6c_dd1d_u64 ^ (seed as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    for y in 0..height {
        for x in 0..width {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let noise = (state >> 33) as u8 / 4;
            let base = ((x * 255 / width.max(1)) + (y * 255 / height.max(1))) / 2;
            let base = u8::try_from(base).unwrap_or(u8::MAX);
            pixels.push(base.wrapping_add(noise));
            pixels.push(base.wrapping_add(noise / 2).wrapping_add(40));
            pixels.push(255_u8.saturating_sub(base).wrapping_add(noise / 3));
        }
    }
    // Valid input, fixed parameters: an encoder failure here would be a bug in the measurement, and the
    // caller has no better answer than an empty file, which the scan would then skip and silently shrink
    // every number below. So it is an error, not a default.
    match craft_jpeg::encode_baseline(&pixels, width, height, 3, 85, craft_jpeg::ChromaSubsampling::S444, &[]) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot encode the measurement fixtures: {error}");
            std::process::exit(1);
        }
    }
}

/// A photo carrying the EXIF the hint pass reads, built in memory: the fixtures are encoded without it.
fn photo_with_exif(id: sortlens_core::PhotoId, index: usize) -> sortlens_engine::Photo {
    let name = format!("IMG_{index:05}.jpg");
    sortlens_engine::Photo {
        id,
        root: "/photos".to_string(),
        rel: name.clone(),
        name,
        size: 1024,
        modified: None,
        kind: sortlens_core::MediaKind::Image,
        key: index as u64 + 1,
        rating: 0,
        flag: sortlens_core::Flag::None,
        label: None,
        missing: false,
        width: 1200,
        height: 800,
        captured: None,
        camera: String::new(),
        lens: String::new(),
        iso: None,
        aperture: None,
        shutter: None,
        focal: None,
        tags: Vec::new(),
        edited: false,
        retry: false,
    }
}

fn poll_scan(engine: &mut sortlens_engine::Engine) -> Result<usize, String> {
    let deadline = Instant::now() + Duration::from_secs(600);
    loop {
        if let Some(done) = engine.poll() {
            return Ok(done.added + done.updated);
        }
        if Instant::now() > deadline {
            return Err("scan did not finish within ten minutes".to_string());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn human_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.0} kB", bytes as f64 / 1024.0)
    }
}
