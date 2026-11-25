//! The map's own pieces: tile textures, dot sizes, and which dot a click landed on.
//!
//! Kept apart from the view for the same reason [`crate::sharpness`] is: the arithmetic that decides *where* a dot
//! goes and *what was clicked* is worth testing on its own, and a test that has to build a window to check a hit test
//! is a test nobody writes twice.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use sortlens_geo::TileCell;
use sortlens_geo::tiles::TileSet;

/// The zoom a map opens at: a country and its neighbours, which is the scale a photo library's spread is legible at.
pub const DEFAULT_ZOOM: u8 = 5;

/// The radius of the smallest dot, in pixels. Small enough that a single photograph is a point rather than a blob.
pub const MIN_DOT: f32 = 4.5;

/// How much each doubling of a dot's photo count adds to its radius.
///
/// Square-root growth, so area tracks the count: a dot standing for four hundred photographs must not be a hundred
/// times the width of one, or it covers the coastline it is meant to be beside.
pub const DOT_GROWTH: f32 = 2.2;

/// The radius a dot stops growing at. Past this it is a region, and a bigger circle stops meaning anything.
pub const MAX_DOT: f32 = 26.0;

/// How many decoded tiles are kept.
///
/// A viewport of 1920×1080 at 256-pixel tiles holds about forty; eight screens' worth is enough to pan back and forth
/// without decoding again, and small enough that a long session does not grow a texture atlas without end.
pub const CACHE_LIMIT: usize = 512;

/// How far a dot may be from a named place and still borrow its name.
///
/// Thirty kilometres is a city and its surroundings: near enough that "Brighton" is the honest answer for a photograph
/// taken on the South Downs, far enough that a village must be the village the photographs are actually in. The
/// gazetteer file decides what exists; this decides how much slack a match is given.
pub const LABEL_KM: f64 = 30.0;

/// A tile larger than this is not a tile. The bound is read before the file is, so a stray archive in the tiles folder
/// cannot be decoded into memory on a frame.
pub const MAX_TILE_BYTES: u64 = 8 * 1024 * 1024;

/// The radius of a dot standing for `count` photographs.
pub fn dot_radius(count: usize) -> f32 {
    let count = count.max(1) as f32;
    MIN_DOT + DOT_GROWTH * count.sqrt()
}

/// One dot on screen, and the cluster it stands for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dot {
    /// The index of the cluster in the list the dots were built from.
    ///
    /// Carried explicitly rather than implied by position: a cluster that is off screen produces no dot, so a dot's
    /// index in the drawn list is **not** its cluster's index. The first version of the view assumed it was, which
    /// meant that with a photo in Sydney and the map over Brighton, clicking the one dot shown selected the wrong
    /// photographs.
    pub cluster: usize,
    /// Screen position, relative to the map's top-left.
    pub at: (f32, f32),
    /// Radius in pixels.
    pub radius: f32,
}

/// The dots to draw for a set of clusters, at a zoom level and viewport origin.
///
/// Clusters Mercator cannot place, and clusters outside the viewport, are left out — and each dot remembers which
/// cluster it came from, so the caller never has to assume the indices line up.
pub fn dots(clusters: &[sortlens_geo::Cluster], zoom: u8, origin: sortlens_geo::Point, width: f32, height: f32) -> Vec<Dot> {
    let mut dots = Vec::new();
    for (index, cluster) in clusters.iter().enumerate() {
        let Some(at) = sortlens_geo::screen_position(cluster.centre, zoom, origin, f64::from(width), f64::from(height)) else {
            continue;
        };
        #[allow(clippy::cast_possible_truncation)]
        dots.push(Dot {
            cluster: index,
            at: (at.x as f32, at.y as f32),
            radius: dot_radius(cluster.len()),
        });
    }
    dots
}

/// Which dot a point landed on, if any.
///
/// Every dot within reach is a candidate and the **largest** wins, not the nearest: where a big dot overhangs a small
/// one, the person is aiming at the big one, and resolving by distance would hand the click to the sliver underneath.
/// Ties go to the earlier dot, so the answer is stable frame to frame.
pub fn dot_at(dots: &[Dot], at: (f32, f32)) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (index, dot) in dots.iter().enumerate() {
        let distance = ((at.0 - dot.at.0).powi(2) + (at.1 - dot.at.1).powi(2)).sqrt();
        // A couple of pixels of grace: a dot is small and a pointer is not a single pixel.
        if distance > dot.radius + 2.0 {
            continue;
        }
        match best {
            Some((_, best_radius)) if dot.radius <= best_radius => {}
            _ => best = Some((index, dot.radius)),
        }
    }
    best.map(|(index, _)| index)
}

/// The name to print beside a dot, if the gazetteer has one near enough.
///
/// `None` means "no place recorded near here", which is *not* the middle of the ocean and not an unknown name: the
/// dot then says its coordinates, because a coordinate is a true answer and an invented place name is not.
pub fn place_label(gazetteer: &sortlens_geo::Gazetteer, at: sortlens_geo::LatLon) -> Option<&sortlens_geo::Place> {
    gazetteer.nearest(at, LABEL_KM)
}

/// What the notice says when a dot is clicked.
///
/// Written as one function so the two forms cannot drift: a named dot says where it is by name and a coordinate, an
/// unnamed one says the coordinate alone rather than pretending to know a place.
#[must_use]
pub fn selection_notice(count: usize, at: sortlens_geo::LatLon, name: Option<&str>) -> String {
    let coordinates = format!(
        "{:.2}{} {:.2}{}",
        at.lat.abs(),
        if at.lat >= 0.0 { 'N' } else { 'S' },
        at.lon.abs(),
        if at.lon >= 0.0 { 'E' } else { 'W' }
    );
    match name {
        Some(name) => format!("Selected {count} photo(s) near {name} ({coordinates})"),
        None => format!("Selected {count} photo(s) at {coordinates}"),
    }
}

/// A gazetteer held between frames, with the file's stamp so a changed file is re-read.
///
/// The stamp is size and modification time: a file edited with the same length in the same nanosecond is not a case
/// worth a checksum on every frame, and reading tens of megabytes per frame to notice is not a case worth anything.
#[derive(Default)]
pub struct Names {
    loaded: Option<sortlens_geo::Gazetteer>,
    stamp: Option<(u64, Option<SystemTime>)>,
    error: Option<String>,
}

impl std::fmt::Debug for Names {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Names")
            .field("places", &self.loaded.as_ref().map(sortlens_geo::Gazetteer::len))
            .field("error", &self.error)
            .finish()
    }
}

impl Clone for Names {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl Names {
    /// An empty naming table.
    pub fn new() -> Self {
        Self::default()
    }

    /// The gazetteer for a file, reloading it when the file has changed.
    ///
    /// `None` when there is no file at all. When the file exists but cannot be read, the reason is kept — the view
    /// shows it, because a places file that silently does nothing is worse than no places file.
    pub fn for_file(&mut self, path: Option<&Path>) -> Option<&sortlens_geo::Gazetteer> {
        let Some(path) = path else {
            self.loaded = None;
            self.stamp = None;
            self.error = None;
            return None;
        };
        let stamp = std::fs::metadata(path)
            .ok()
            .map(|metadata| (metadata.len(), metadata.modified().ok()));
        if stamp != self.stamp {
            self.stamp = stamp;
            self.error = None;
            match sortlens_geo::Gazetteer::load(path) {
                Ok(gazetteer) => self.loaded = Some(gazetteer),
                // A file that is not there is not a fault: most libraries have no places file, and a red line saying
                // so would teach the person to ignore the red line. The states worth reporting are a file that is
                // present and cannot be used.
                Err(sortlens_geo::GazetteerError::Missing(_)) => self.loaded = None,
                Err(error) => {
                    self.loaded = None;
                    self.error = Some(error.message());
                }
            }
        }
        self.loaded.as_ref()
    }

    /// Why the file could not be read, if it could not.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// How many place names are held.
    pub fn len(&self) -> usize {
        self.loaded.as_ref().map_or(0, sortlens_geo::Gazetteer::len)
    }

    /// Whether no name is held.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A zoom to actually use, given the set's range.
///
/// Zooming past the top of a tile set is clamped rather than refused: the tiles stop, the dots keep going, and the
/// view says which is which. A map that stops responding to the + button has not told anybody why.
pub fn clamp_zoom(wanted: u8, range: (u8, u8)) -> u8 {
    let (low, high) = range;
    // The bounds are put in order first: a set whose range arrives backwards is still a set, and `clamp` panics when
    // its minimum is above its maximum — an assertion failure in a paint path is not a way to learn that.
    let high = high.max(low).max(1);
    let low = low.max(1).min(high);
    wanted.clamp(low, high)
}

/// Decoded tiles, kept between frames.
///
/// Both a tile that failed to decode and one the set simply does not carry are remembered, so neither is re-read on
/// every repaint — how a single bad tile, or a hole in an archive, becomes a stutter. They are not the same kind of
/// absence, though: a hole is painted as background and counted by neither [`TileCache::decodes`] nor
/// [`TileCache::failures`], because a partial map is normal and a corrupted tile is a fault.
#[derive(Default)]
pub struct TileCache {
    /// The basemap the entries came from. A different folder or archive clears the cache rather than painting one
    /// library's tiles over another's, which is a wrong map with no error message.
    source: Option<PathBuf>,
    entries: HashMap<TileCell, Option<egui::TextureHandle>>,
    /// Counted so the cache limit has something to measure rather than a guess.
    decodes: usize,
    failures: usize,
}

/// Cloning would duplicate every texture handle without duplicating the GPU memory behind it, which is a cache that
/// looks free and is not. The cache is therefore moved with the state that owns it, never copied.
impl Clone for TileCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for TileCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TileCache")
            .field("entries", &self.entries.len())
            .field("decodes", &self.decodes)
            .field("failures", &self.failures)
            .finish()
    }
}

impl TileCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// The texture for one cell of a basemap, decoding it on first sight.
    ///
    /// The bytes come from [`TileSet::tile`], so a folder and an archive are the same path here. `None` covers both
    /// "not there" and "not an image"; the view paints background either way, because a hole in a tile set is not an
    /// error to report, it is a hole.
    pub fn texture(&mut self, ctx: &egui::Context, set: &TileSet, cell: TileCell) -> Option<&egui::TextureHandle> {
        if self.source.as_deref() != Some(set.path()) {
            self.entries.clear();
            self.source = Some(set.path().to_path_buf());
        }
        if !self.entries.contains_key(&cell) {
            if self.entries.len() >= CACHE_LIMIT {
                self.evict();
            }
            // Only bytes the set actually carries can be a decode failure. A tile that is not there is a hole in the
            // basemap — a partial tile set, or a viewport wider than the set's coverage — and counting it as a failure
            // made every such map look corrupted: the Places paint test's whole claim ("every fixture tile was a real
            // PNG") reported 980 failures for the tiles it never wrote, while the fixture's own tiles decoded fine.
            let loaded = match set.tile(cell.zoom, cell.x, cell.y) {
                Some(bytes) => {
                    let texture = self.load(ctx, cell, &bytes);
                    if texture.is_none() {
                        self.failures += 1;
                    }
                    texture
                }
                None => None,
            };
            self.entries.insert(cell, loaded);
        }
        self.entries.get(&cell).and_then(Option::as_ref)
    }

    /// Whether a tile is already decoded, without loading it. Used by tests and by the view's own bookkeeping.
    pub fn contains(&self, cell: TileCell) -> bool {
        self.entries.contains_key(&cell)
    }

    /// How many tiles are held.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is held.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many tiles carrying bytes have been decoded, successes and failures together.
    ///
    /// A cell the set does not carry is not a decode: there were no bytes to read.
    pub fn decodes(&self) -> usize {
        self.decodes
    }

    /// How many tiles that carried bytes would not decode — a corrupted file, not a hole.
    pub fn failures(&self) -> usize {
        self.failures
    }

    fn load(&mut self, ctx: &egui::Context, cell: TileCell, bytes: &[u8]) -> Option<egui::TextureHandle> {
        self.decodes += 1;
        if bytes.is_empty() || bytes.len() > MAX_TILE_BYTES as usize {
            return None;
        }
        let tile = sortlens_thumbs::decode_image(bytes, 256).ok()?;
        let image = egui::ColorImage::from_rgba_unmultiplied([tile.width as usize, tile.height as usize], &tile.rgba);
        Some(ctx.load_texture(
            format!("tile:{}:{}:{}", cell.zoom, cell.x, cell.y),
            image,
            egui::TextureOptions::LINEAR,
        ))
    }

    /// Drop half the cache. Textures are cheap to decode again and expensive to keep forever; halving avoids the
    /// thrash of evicting one entry per insertion once the limit is reached.
    fn evict(&mut self) {
        let doomed: Vec<TileCell> = self.entries.keys().take(CACHE_LIMIT / 2).copied().collect();
        for cell in doomed {
            self.entries.remove(&cell);
        }
    }
}
#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_dot_grows_with_its_count_and_then_stops() {
        assert!(dot_radius(1) < dot_radius(4));
        assert!(dot_radius(4) < dot_radius(400));
        // Area, not width: four times the photographs is twice the radius, not four times.
        let single = dot_radius(1) - MIN_DOT;
        let quadruple = dot_radius(4) - MIN_DOT;
        assert!((quadruple / single - 2.0).abs() < 0.01, "got {}", quadruple / single);
        assert_eq!(dot_radius(1_000_000), MAX_DOT.max(dot_radius(1_000_000)));
        // A count of zero is one dot's worth, not a negative radius.
        assert_eq!(dot_radius(0), dot_radius(1));
    }

    #[test]
    fn a_click_finds_the_dot_under_it_and_prefers_the_big_one() {
        let dots = [
            Dot {
                cluster: 0,
                at: (10.0, 10.0),
                radius: 5.0,
            },
            Dot {
                cluster: 1,
                at: (100.0, 100.0),
                radius: 5.0,
            },
            Dot {
                cluster: 2,
                at: (12.0, 10.0),
                radius: 14.0,
            },
        ];
        assert_eq!(dot_at(&dots, (11.0, 10.0)), Some(2), "the big dot overhangs the small one");
        assert_eq!(dot_at(&dots, (100.0, 100.0)), Some(1));
        assert_eq!(dot_at(&dots, (60.0, 60.0)), None, "the sea is not a dot");
        // The grace of a couple of pixels: a click a pixel outside a radius-5 dot still lands on it.
        assert_eq!(dot_at(&dots, (100.0, 106.0)), Some(1));
        assert_eq!(dot_at(&dots, (100.0, 109.0)), None, "four pixels outside a radius-5 dot is the sea");
        assert_eq!(dot_at(&[], (0.0, 0.0)), None);
    }

    /// A cluster off screen produces no dot, and the dots that remain still point at the right clusters.
    ///
    /// This is the bug that made a test worth writing: the view built its dot list by skipping clusters it could not
    /// place and then indexed back into the cluster list by *position*, so one off-screen cluster shifted every dot
    /// onto its neighbour's photographs.
    #[test]
    fn a_dot_points_at_its_own_cluster_even_when_another_is_off_screen() {
        let brighton = sortlens_geo::LatLon::new(50.8225, -0.1372).expect("a place");
        let sydney = sortlens_geo::LatLon::new(-33.8688, 151.2093).expect("a place");
        let clusters = vec![
            sortlens_geo::Cluster {
                cell: (0, 0),
                members: vec![sortlens_core::PhotoId(1)],
                centre: sydney,
                bounds: (-33.0, -34.0, 151.0, 152.0),
            },
            sortlens_geo::Cluster {
                cell: (1, 1),
                members: vec![sortlens_core::PhotoId(2), sortlens_core::PhotoId(3)],
                centre: brighton,
                bounds: (50.0, 51.0, -1.0, 0.0),
            },
        ];
        let zoom = 6;
        let origin = sortlens_geo::viewport_origin(zoom, brighton, 800.0, 600.0).expect("origin");
        let dots = dots(&clusters, zoom, origin, 800.0, 600.0);
        assert_eq!(dots.len(), 1, "Sydney is off screen and gets no dot");
        assert_eq!(
            dots[0].cluster, 1,
            "the dot remembers it is the Brighton cluster, not the Sydney one"
        );
        assert_eq!(clusters[dots[0].cluster].members.len(), 2);
        // And the hit test resolves to that dot, so the click selects Brighton's two photographs.
        let hit = dot_at(&dots, dots[0].at).expect("its own centre");
        assert_eq!(
            clusters[dots[hit].cluster].members,
            vec![sortlens_core::PhotoId(2), sortlens_core::PhotoId(3)]
        );
    }

    #[test]
    fn zooming_past_the_top_of_a_tile_set_is_clamped_not_refused() {
        assert_eq!(clamp_zoom(5, (2, 12)), 5);
        assert_eq!(clamp_zoom(30, (2, 12)), 12, "the tiles stop, the dots do not");
        assert_eq!(clamp_zoom(0, (2, 12)), 2);
        // A set with one zoom is one zoom, and zoom 0 means the world in a single tile: usable, so kept.
        assert_eq!(clamp_zoom(9, (0, 0)), 1);
    }

    /// A dot borrows a name when the gazetteer has one near enough, and says its coordinate when it does not.
    #[test]
    fn a_name_is_borrowed_only_when_there_is_one_near_enough() {
        let gazetteer = sortlens_geo::Gazetteer::parse("Brighton\t51\t0\tGB\nLima\t-12\t-77\n");
        assert_eq!(gazetteer.len(), 2);
        let brighton = sortlens_geo::LatLon::new(50.8225, -0.1372).expect("a place");
        let named = place_label(&gazetteer, brighton).expect("Brighton is 20 km away");
        assert_eq!(named.name, "Brighton");

        // The middle of the Atlantic: no name, and no invented one.
        let ocean = sortlens_geo::LatLon::new(25.0, -40.0).expect("a place");
        assert_eq!(place_label(&gazetteer, ocean), None);

        // The notice has two forms and neither of them lies: a named dot says the name *and* the coordinate, so the
        // name cannot be mistaken for a finer resolution than the file has.
        assert_eq!(
            selection_notice(42, brighton, Some(&named.name)),
            "Selected 42 photo(s) near Brighton (50.82N 0.14W)"
        );
        assert_eq!(selection_notice(1, ocean, None), "Selected 1 photo(s) at 25.00N 40.00W");
        // The southern and western halves of the world keep their signs on the label.
        let sydney = sortlens_geo::LatLon::new(-33.8688, 151.2093).expect("a place");
        assert_eq!(selection_notice(3, sydney, None), "Selected 3 photo(s) at 33.87S 151.21E");
    }

    /// A gazetteer is parsed once and re-parsed only when the file changes.
    #[test]
    fn a_names_table_reloads_when_the_file_changes() {
        let dir = std::env::temp_dir().join(format!("sortlens-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("places.txt");

        let mut names = Names::new();
        assert!(names.for_file(None).is_none(), "no file, no names");
        assert!(names.is_empty());

        std::fs::write(&path, "Brighton\t51\t0\n").expect("write");
        assert_eq!(names.for_file(Some(&path)).map(sortlens_geo::Gazetteer::len), Some(1));
        assert_eq!(names.len(), 1);
        assert!(names.error().is_none());

        // A second call with an unchanged file is a `metadata` call, not a re-read: the count stays the same and the
        // parse is not repeated (a re-parse is not observable, but an unchanged *stamp* is, and the file is larger
        // after the next write, which is what forces the reload).
        assert_eq!(names.for_file(Some(&path)).map(sortlens_geo::Gazetteer::len), Some(1));

        std::fs::write(&path, "Brighton\t51\t0\nLima\t-12\t-77\nHove\t51\t0\n").expect("write");
        assert_eq!(
            names.for_file(Some(&path)).map(sortlens_geo::Gazetteer::len),
            Some(3),
            "the changed file is re-read"
        );

        // A file that disappears stops naming anything rather than keeping the last names it had: a place list that
        // outlives its file would put names on a map the person deleted the authority for. And its absence is not an
        // error, because having no places file is the normal state of a library.
        std::fs::remove_file(&path).expect("remove");
        assert!(names.for_file(Some(&path)).is_none());
        assert!(names.is_empty());
        assert_eq!(names.error(), None, "no file is not a fault");

        // An unreadable file keeps its reason for the view to show.
        let directory_instead = dir.join("places.txt");
        std::fs::create_dir_all(&directory_instead).expect("dir");
        names.for_file(Some(&directory_instead));
        assert!(
            names.error().is_some(),
            "a places path that is a directory is reported, not ignored"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A cache decodes a tile once, remembers a bad tile, and does not grow without end.
    #[test]
    fn a_tile_is_decoded_once_and_a_bad_one_is_remembered_as_bad() {
        let root = std::env::temp_dir().join(format!("sortlens-tilecache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let folder = root.join("tiles");
        let good = folder.join("3").join("1");
        std::fs::create_dir_all(&good).expect("dir");
        std::fs::write(folder.join("attribution.txt"), "© Example Maps").expect("credit");
        std::fs::write(good.join("2.png"), sortlens_icon::png()).expect("write");
        let bad = folder.join("3").join("1").join("3.png");
        std::fs::write(&bad, b"not a png").expect("write");
        let set = TileSet::open(&root).expect("a folder with a credit");
        let png = TileCell { zoom: 3, x: 1, y: 2 };
        let corrupt = TileCell { zoom: 3, x: 1, y: 3 };
        let absent = TileCell { zoom: 3, x: 1, y: 4 };

        let ctx = egui::Context::default();
        let mut cache = TileCache::new();
        // Uploading needs a frame in progress, so the loads happen inside one.
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            assert!(cache.texture(ctx, &set, png).is_some(), "a real png decodes");
            assert!(cache.texture(ctx, &set, png).is_some());
            assert_eq!(cache.decodes(), 1, "the second look is a cache hit");
            assert!(cache.texture(ctx, &set, corrupt).is_none());
            assert!(cache.texture(ctx, &set, corrupt).is_none());
            assert_eq!(cache.decodes(), 2, "a failed tile is not re-decoded");
            assert_eq!(cache.failures(), 1);
            assert!(cache.texture(ctx, &set, absent).is_none());
            assert_eq!(cache.len(), 3, "failures are remembered as failures");
            // A tile the folder does not carry is a hole in the map, not a broken tile: it is remembered (so it is not
            // stat'ed again) but it is neither a decode nor a failure. Counting it as one made a partial set look
            // corrupted, which is what the Places paint test was reporting.
            assert_eq!(cache.failures(), 1, "a hole is not a corrupted tile");
            assert_eq!(cache.decodes(), 2, "a hole is not a decode attempt");
        });
        output.textures_delta.clear();
        let _ = std::fs::remove_dir_all(&root);
    }
}
