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
