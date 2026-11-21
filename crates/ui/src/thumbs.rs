//! Texture cache for the grid.
//!
//! The engine hands us photos; this module turns their files into GPU textures without blocking
//! the frame. Requests go to [`sortlens_thumbs::Service`], results come back through `poll`, and
//! a bounded texture map keeps the working set to what the user is looking at.

use egui::{ColorImage, TextureHandle, TextureOptions};
use sortlens_engine::Photo;
use sortlens_thumbs::{Cache, Service};
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
}

impl Thumbnails {
    /// Start the decode workers, caching under `cache_dir`.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            service: Service::new(Cache::new(cache_dir), 2),
            textures: HashMap::new(),
            order: VecDeque::new(),
            failed: HashMap::new(),
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
