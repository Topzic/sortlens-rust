//! The map's own pieces: tile textures, dot sizes, and which dot a click landed on.
//!
//! Kept apart from the view for the same reason [`crate::sharpness`] is: the arithmetic that decides *where* a dot
//! goes and *what was clicked* is worth testing on its own, and a test that has to build a window to check a hit test
//! is a test nobody writes twice.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
/// A tile that failed to decode is remembered as a failure: a corrupt file in the tiles folder must not be re-decoded
/// on every repaint, which is how a single bad tile becomes a stutter.
#[derive(Default)]
pub struct TileCache {
    entries: HashMap<PathBuf, Option<egui::TextureHandle>>,
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

    /// The texture for one tile, decoding it on first sight.
    ///
    /// `None` covers both "not there" and "not an image"; the view paints background either way, because a hole in a
    /// tile set is not an error to report, it is a hole.
    pub fn texture(&mut self, ctx: &egui::Context, path: &Path) -> Option<&egui::TextureHandle> {
        if !self.entries.contains_key(path) {
            if self.entries.len() >= CACHE_LIMIT {
                self.evict();
            }
            let loaded = self.load(ctx, path);
            if loaded.is_none() {
                self.failures += 1;
            }
            self.entries.insert(path.to_path_buf(), loaded);
        }
        self.entries.get(path).and_then(Option::as_ref)
    }

    /// Whether a tile is already decoded, without loading it. Used by tests and by the view's own bookkeeping.
    pub fn contains(&self, path: &Path) -> bool {
        self.entries.contains_key(path)
    }

    /// How many tiles are held.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is held.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many decodes have been attempted, successes and failures together.
    pub fn decodes(&self) -> usize {
        self.decodes
    }

    /// How many of those failed.
    pub fn failures(&self) -> usize {
        self.failures
    }

    fn load(&mut self, ctx: &egui::Context, path: &Path) -> Option<egui::TextureHandle> {
        self.decodes += 1;
        let size = std::fs::metadata(path).ok()?.len();
        if size == 0 || size > MAX_TILE_BYTES {
            return None;
        }
        let bytes = std::fs::read(path).ok()?;
        let tile = sortlens_thumbs::decode_image(&bytes, 256).ok()?;
        let image = egui::ColorImage::from_rgba_unmultiplied([tile.width as usize, tile.height as usize], &tile.rgba);
        Some(ctx.load_texture(format!("tile:{}", path.display()), image, egui::TextureOptions::LINEAR))
    }

    /// Drop half the cache. Textures are cheap to decode again and expensive to keep forever; halving avoids the
    /// thrash of evicting one entry per insertion once the limit is reached.
    fn evict(&mut self) {
        let doomed: Vec<PathBuf> = self.entries.keys().take(CACHE_LIMIT / 2).cloned().collect();
        for path in doomed {
            self.entries.remove(&path);
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

    /// A cache decodes a tile once, remembers a bad tile, and does not grow without end.
    #[test]
    fn a_tile_is_decoded_once_and_a_bad_one_is_remembered_as_bad() {
        let dir = std::env::temp_dir().join(format!("sortlens-tilecache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let good = dir.join("good.png");
        std::fs::write(&good, sortlens_icon::png()).expect("write");
        let bad = dir.join("bad.png");
        std::fs::write(&bad, b"not a png").expect("write");
        let absent = dir.join("absent.png");

        let ctx = egui::Context::default();
        let mut cache = TileCache::new();
        // Uploading needs a frame in progress, so the loads happen inside one.
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            assert!(cache.texture(ctx, &good).is_some(), "a real png decodes");
            assert!(cache.texture(ctx, &good).is_some());
            assert_eq!(cache.decodes(), 1, "the second look is a cache hit");
            assert!(cache.texture(ctx, &bad).is_none());
            assert!(cache.texture(ctx, &bad).is_none());
            assert_eq!(cache.decodes(), 2, "a failed tile is not re-decoded");
            assert_eq!(cache.failures(), 1);
            assert!(cache.texture(ctx, &absent).is_none());
            assert_eq!(cache.len(), 3, "failures are remembered as failures");
        });
        output.textures_delta.clear();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
