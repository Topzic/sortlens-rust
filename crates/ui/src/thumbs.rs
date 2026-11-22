//! Texture cache for the grid.
//!
//! The engine hands us photos; this module turns their files into GPU textures without blocking
//! the frame. Requests go to [`sortlens_thumbs::Service`], results come back through `poll`, and
//! a bounded texture map keeps the working set to what the user is looking at.

use egui::{ColorImage, TextureHandle, TextureOptions};
use sortlens_engine::Photo;
use sortlens_thumbs::{Cache, Service, Urgency};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

/// Size the grid asks for.
pub const GRID_SIZE: u32 = sortlens_thumbs::GRID;
/// Size the detail view will ask for.
pub const DETAIL_SIZE: u32 = sortlens_thumbs::DETAIL;

/// How many textures stay resident. A 128 px RGBA thumbnail is 64 KiB, so 900 of them is about
/// 56 MiB of GPU memory: enough for several screens of scrolling, small enough to be polite.
const MAX_TEXTURES: usize = 900;

/// Which resident tier to paint while the wanted one is still decoding.
///
/// The rule is: exactly the wanted size, else the largest resident size below it, else the smallest above it
/// (painting a bigger picture into a smaller hole is sharp, merely wasteful — never wrong). `None` when nothing
/// for this photo is resident, because an empty rectangle is not a blur: a placeholder that says what it is
/// beats a smear that pretends to be the photo.
///
/// Pure, and separate from the texture map, so the choice can be tested without a GPU.
pub fn blur_tier(resident: &[(u64, u32)], key: u64, wanted: u32) -> Option<u32> {
    let sizes = resident.iter().filter(|(k, _)| *k == key).map(|(_, size)| *size);
    let mut below: Option<u32> = None;
    let mut above: Option<u32> = None;
    for size in sizes {
        if size == wanted {
            return Some(size);
        }
        if size < wanted && below.is_none_or(|best| size > best) {
            below = Some(size);
        }
        if size > wanted && above.is_none_or(|best| size < best) {
            above = Some(size);
        }
    }
    below.or(above)
}

/// Decoded thumbnails, keyed by content and size.
pub struct Thumbnails {
    service: Service,
    textures: HashMap<(u64, u32), TextureHandle>,
    order: VecDeque<(u64, u32)>,
    failed: HashMap<u64, String>,
    histograms: HashMap<u64, sortlens_thumbs::Histogram>,
}

impl Thumbnails {
    /// Start the decode workers, caching under `cache_dir`.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            service: Service::new(Cache::new(cache_dir), 2),
            textures: HashMap::new(),
            order: VecDeque::new(),
            failed: HashMap::new(),
            histograms: HashMap::new(),
        }
    }

    /// The same, with an explicit queue bound.
    ///
    /// Only a test has a reason to say so: it is the only way to make a bound of four thousand requests bite
    /// inside a few frames, and a bound nobody has seen bite is a bound nobody has checked.
    pub fn with_queue_bound(cache_dir: PathBuf, workers: usize, capacity: usize) -> Self {
        Self {
            service: Service::with_capacity(Cache::new(cache_dir), workers, capacity),
            textures: HashMap::new(),
            order: VecDeque::new(),
            failed: HashMap::new(),
            histograms: HashMap::new(),
        }
    }

    /// How much decode work is waiting, and how much was refused, for the activity stack and for tests.
    pub fn queued(&self) -> usize {
        self.service.queued()
    }

    /// How many requests have been refused because the queue was full of work at least as urgent.
    pub fn dropped(&self) -> u64 {
        self.service.dropped()
    }

    /// Number of textures currently resident, for diagnostics.
    pub fn resident(&self) -> usize {
        self.textures.len()
    }

    /// The texture for a photo at a given size, requesting it if needed.
    pub fn get(&mut self, ctx: &egui::Context, photo: &Photo, size: u32) -> Option<&TextureHandle> {
        let key = (photo.key, size);
        if self.textures.contains_key(&key) {
            return self.textures.get(&key);
        }
        if !self.failed.contains_key(&photo.key) {
            self.service.request(photo.key, PathBuf::from(photo.path()), size);
        }
        let _ = ctx;
        None
    }

    /// The sharp texture for a photo if it is ready, otherwise the best tier that is — a blur-up.
    ///
    /// Returns the texture and the size it was actually decoded at, so the caller can say whether what it is
    /// painting is the real thing. Asking for a big tier (the detail view's 1600) waits on a full decode; the
    /// 128 tier the grid already made is in memory, and painting it scaled is the difference between a soft
    /// photograph and a "Loading preview…" line for a second.
    pub fn sharp_or_blur(&mut self, ctx: &egui::Context, photo: &Photo, size: u32) -> Option<(TextureHandle, u32)> {
        if let Some(texture) = self.textures.get(&(photo.key, size)) {
            return Some((texture.clone(), size));
        }
        // Ask for the sharp one whether or not a blur is resident: the blur is a bridge, not an answer.
        let resident: Vec<(u64, u32)> = self.textures.keys().copied().collect();
        let blur = blur_tier(&resident, photo.key, size);
        self.get(ctx, photo, size);
        blur.and_then(|tier| self.textures.get(&(photo.key, tier)).map(|texture| (texture.clone(), tier)))
    }

    /// Ask for a thumbnail that is not on screen yet, at the "nearly visible" urgency.
    ///
    /// The detail view asks for the next photo with this, so arrowing forward finds it already decoded while a
    /// library-wide pass waits behind it. A refusal at the queue bound is not an error and need not be
    /// reported: the caller asks again when the photo is actually shown.
    pub fn prefetch(&mut self, photo: &Photo, size: u32) {
        if self.textures.contains_key(&(photo.key, size)) || self.failed.contains_key(&photo.key) {
            return;
        }
        let _ = self
            .service
            .request_with(photo.key, PathBuf::from(photo.path()), size, Urgency::Nearby);
    }

    /// How many decodes are waiting, and how many were refused at the bound — for diagnostics and for tests
    /// that want to prove the grid does not enqueue a whole library.
    pub fn queue(&self) -> (usize, u64) {
        (self.service.queued(), self.service.dropped())
    }

    /// Luminance histogram for a photo, computed once and cached by content key.
    ///
    /// This decodes a small copy on the calling thread, so it is only asked for by the detail view
    /// (one photo, once), never by the grid.
    pub fn histogram(&mut self, photo: &Photo) -> Option<&sortlens_thumbs::Histogram> {
        if let std::collections::hash_map::Entry::Vacant(entry) = self.histograms.entry(photo.key) {
            let path = PathBuf::from(photo.path());
            entry.insert(sortlens_thumbs::histogram_file(&path, 256).ok()?);
        }
        self.histograms.get(&photo.key)
    }

    /// Upload everything that finished decoding.
    pub fn poll(&mut self, ctx: &egui::Context) -> usize {
        let loaded = self.service.poll();
        let count = loaded.len();
        for item in loaded {
            let Some(thumbnail) = item.thumbnail else {
                if let Some(error) = item.error {
                    self.failed.insert(item.key, error);
                }
                continue;
            };
            let image = ColorImage::from_rgba_unmultiplied([thumbnail.width as usize, thumbnail.height as usize], &thumbnail.rgba);
            let handle = ctx.load_texture(format!("thumb-{}-{}", item.key, item.size), image, TextureOptions::LINEAR);
            self.textures.insert((item.key, item.size), handle);
            self.order.push_back((item.key, item.size));
        }
        while self.order.len() > MAX_TEXTURES {
            if let Some(key) = self.order.pop_front() {
                self.textures.remove(&key);
            }
        }
        if count > 0 {
            ctx.request_repaint();
        }
        count
    }

    /// Why a photo has no thumbnail, if it failed.
    pub fn failure(&self, key: u64) -> Option<&str> {
        self.failed.get(&key).map(String::as_str)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// The blur-up picks the largest tier below the wanted one, and never a tier belonging to another photo.
    #[test]
    fn a_blur_up_picks_the_best_resident_tier() {
        let resident = vec![(7_u64, 128_u32), (7, 384), (8, 1024)];
        // Nothing for this photo: no blur, because an empty rectangle is not a blur.
        assert_eq!(blur_tier(&resident, 9, 1600), None);
        // No larger tier than 384: the largest below wins, which is the sharpest thing available.
        assert_eq!(blur_tier(&resident, 7, 1600), Some(384));
        // Exactly what was wanted, even when other tiers exist.
        assert_eq!(blur_tier(&resident, 7, 384), Some(384));
        // A tier below the request: still the best below.
        assert_eq!(blur_tier(&resident, 7, 1024), Some(384));
        // Only a bigger tier resident (the detail view zoomed back out): painting it down is sharp, not wrong.
        assert_eq!(blur_tier(&[(7_u64, 1600_u32)], 7, 128), Some(1600));
        // And another photo's tier is never borrowed.
        assert_eq!(blur_tier(&resident, 8, 1600), Some(1024));
    }

    /// Blur-up end to end: once a grid-sized tier is decoded, the detail view's big request paints that
    /// instead of waiting for the full decode.
    #[test]
    fn the_detail_view_paints_the_grid_tier_while_the_big_one_decodes() {
        let dir = std::env::temp_dir().join(format!("sortlens-blurup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let fixture = sortlens_testkit::Fixture::new("blurup");
        let path = fixture.jpeg("IMG_0001.jpg", [90, 140, 200], 96, 64);
        let photo = photo_for(&path);

        let ctx = egui::Context::default();
        // A separate instance with an empty texture map: the "nothing resident" case, without waiting on a
        // decode that would make the next assertion a race against a worker thread.
        let mut nothing = Thumbnails::new(dir.join("empty-cache"));
        assert!(
            nothing.sharp_or_blur(&ctx, &photo, 1600).is_none(),
            "nothing resident yet, so no blur: the placeholder is honest"
        );
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        // The grid asks for its tier; wait for the real decoder to finish it.
        thumbs.get(&ctx, &photo, 128);
        for _ in 0..800 {
            if thumbs.poll(&ctx) > 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(thumbs.resident() >= 1, "the grid tier is now a texture");
        let (texture, tier) = thumbs.sharp_or_blur(&ctx, &photo, 1600).expect("the grid tier stands in");
        assert_eq!(tier, 128, "a 128 tier was painted where 1600 was asked for");
        assert!(texture.size()[0] <= 128, "and it is genuinely the small one");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A photo pointing at a real file, since the cache is keyed by content and the decoder by path.
    fn photo_for(path: &std::path::Path) -> Photo {
        Photo {
            id: sortlens_engine::PhotoId(1),
            root: path.to_string_lossy().to_string(),
            rel: String::new(),
            name: "IMG_0001.jpg".to_string(),
            size: 1,
            modified: None,
            kind: sortlens_engine::MediaKind::Image,
            key: 42,
            rating: 0,
            flag: sortlens_engine::Flag::None,
            label: None,
            missing: false,
            duration_ms: None,
            fps: None,
            video_codec: None,
            width: 96,
            height: 64,
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
            sharpness: None,
        }
    }
}
