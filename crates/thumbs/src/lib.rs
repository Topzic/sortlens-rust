//! L2: preview decoding and the thumbnail cache.
//!
//! Two jobs, kept together because they share the same failure modes:
//!
//! * decode a file into RGBA at a bounded size, never upscaled;
//! * keep the result in a content-addressed disk cache that can be deleted at any time.
//!
//! Decoding runs on worker threads behind [`Service`] so the grid never blocks on a spinner.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use sortlens_core::{Error, Result};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};

/// Preview sizes. The grid asks for 128, the filmstrip and large grid for 384, the detail view
/// for 1024; nothing asks for more until the loupe.
pub const GRID: u32 = 128;
/// Large grid and filmstrip.
pub const LARGE: u32 = 384;
/// Detail view.
pub const DETAIL: u32 = 1024;

/// An RGBA image, ready for a texture upload.
#[derive(Debug, Clone, PartialEq)]
pub struct Thumbnail {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA8, tightly packed, top row first.
    pub rgba: Vec<u8>,
}

impl Thumbnail {
    /// Whether the buffer is consistent with the dimensions.
    pub fn is_consistent(&self) -> bool {
        self.rgba.len() == (self.width as usize) * (self.height as usize) * 4
    }
}

/// Decode a JPEG at no more than `max_edge` pixels on its long edge.
pub fn decode_jpeg(bytes: &[u8], max_edge: u32) -> Result<Thumbnail> {
    let limits = craft_jpeg::Limits::default();
    let decoded = craft_jpeg::decode(bytes, &limits, None).map_err(|error| Error::Invalid(format!("jpeg: {error}")))?;
    let (width, height) = (decoded.width, decoded.height);
    if width == 0 || height == 0 {
        return Err(Error::Invalid("jpeg has no pixels".into()));
    }
    let rgb = match &decoded.samples {
        craft_jpeg::Samples::U8(samples) => match decoded.components {
            3 => samples.clone(),
            1 => samples.iter().flat_map(|grey| [*grey, *grey, *grey]).collect(),
            components => return Err(Error::UnsupportedFormat(format!("{components}-component jpeg"))),
        },
        craft_jpeg::Samples::U16(_) => return Err(Error::UnsupportedFormat("12-bit jpeg".into())),
    };
    Ok(resize(&rgb, width, height, 3, max_edge))
}

/// Decode any raster format the suite understands, downscaled to `max_edge`.
///
/// The dispatcher covers PNG, TIFF, WebP, GIF, BMP and 8-bit JPEG. A file it refuses falls back to
/// the JPEG path, which also handles the 12-bit and CMYK cases.
pub fn decode_image(bytes: &[u8], max_edge: u32) -> Result<Thumbnail> {
    let image = craft_image_codecs::decode(bytes).map_err(|error| Error::Invalid(format!("image: {error}")))?;
    let (width, height) = (image.width(), image.height());
    if width == 0 || height == 0 {
        return Err(Error::Invalid("image has no pixels".into()));
    }
    let rgba = image.to_rgba8();
    Ok(resize(&rgba, width, height, 4, max_edge))
}

/// Read a file and decode it, if the format is one this build understands.
pub fn decode_file(path: &Path, max_edge: u32) -> Result<Thumbnail> {
    let bytes = fs::read(path).map_err(|error| Error::io(path.display().to_string(), error))?;
    decode_image(&bytes, max_edge).or_else(|_| decode_jpeg(&bytes, max_edge))
}

/// Box-filter resize of an interleaved image down to `max_edge`.
///
/// A box filter is the right default for thumbnails: it averages every source pixel that lands in
/// a destination pixel, so a 60 MP frame does not alias into noise. Upscaling never happens.
pub fn resize(samples: &[u8], width: u32, height: u32, channels: usize, max_edge: u32) -> Thumbnail {
    let max_edge = max_edge.max(16);
    let longest = width.max(height);
    if longest <= max_edge {
        return to_rgba(samples, width, height, channels);
    }
    let scale = max_edge as f32 / longest as f32;
    let out_w = ((width as f32 * scale).round() as u32).max(1);
    let out_h = ((height as f32 * scale).round() as u32).max(1);
    let mut rgba = vec![0u8; (out_w as usize) * (out_h as usize) * 4];

    let x_ratio = width as f32 / out_w as f32;
    let y_ratio = height as f32 / out_h as f32;

    for out_y in 0..out_h {
        let y0 = (out_y as f32 * y_ratio) as u32;
        let y1 = (((out_y + 1) as f32 * y_ratio).ceil() as u32).clamp(y0 + 1, height);
        for out_x in 0..out_w {
            let x0 = (out_x as f32 * x_ratio) as u32;
            let x1 = (((out_x + 1) as f32 * x_ratio).ceil() as u32).clamp(x0 + 1, width);
            let mut sums = [0u32; 4];
            let mut count = 0u32;
            for y in y0..y1 {
                for x in x0..x1 {
                    let index = ((y as usize) * (width as usize) + (x as usize)) * channels;
                    for channel in 0..3 {
                        if index + channel < samples.len() {
                            sums[channel] += u32::from(samples[index + channel]);
                        }
                    }
                    if channels == 4 && index + 3 < samples.len() {
                        sums[3] += u32::from(samples[index + 3]);
                    }
                    count += 1;
                }
            }
            let count = count.max(1);
            let out = ((out_y as usize) * (out_w as usize) + (out_x as usize)) * 4;
            rgba[out] = (sums[0] / count) as u8;
            rgba[out + 1] = (sums[1] / count) as u8;
            rgba[out + 2] = (sums[2] / count) as u8;
            rgba[out + 3] = if channels == 4 { (sums[3] / count) as u8 } else { 255 };
        }
    }
    Thumbnail {
        width: out_w,
        height: out_h,
        rgba,
    }
}

fn to_rgba(samples: &[u8], width: u32, height: u32, channels: usize) -> Thumbnail {
    let pixels = (width as usize) * (height as usize);
    let mut rgba = vec![255u8; pixels * 4];
    for pixel in 0..pixels {
        let source = pixel * channels;
        let target = pixel * 4;
        for channel in 0..3 {
            if source + channel < samples.len() {
                rgba[target + channel] = samples[source + channel];
            }
        }
        if channels == 4 {
            rgba[target + 3] = samples.get(source + 3).copied().unwrap_or(255);
        }
    }
    Thumbnail { width, height, rgba }
}

/// The on-disk cache. Files are named by content key and size, so a rescan reuses them and a
/// renamed file stays warm.
#[derive(Debug, Clone)]
pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    /// A cache rooted at `dir`; the directory is created on first write.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Directory holding the cache files.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Path for one thumbnail.
    pub fn path(&self, key: u64, size: u32) -> PathBuf {
        let bucket = key % 256;
        self.dir.join(format!("{bucket:02x}")).join(format!("{key:016x}-{size}.rgba"))
    }

    /// Load a cached thumbnail, if present and well formed.
    pub fn get(&self, key: u64, size: u32) -> Option<Thumbnail> {
        let bytes = fs::read(self.path(key, size)).ok()?;
        if bytes.len() < 8 {
            return None;
        }
        let width = u32::from_le_bytes(bytes[0..4].try_into().ok()?);
        let height = u32::from_le_bytes(bytes[4..8].try_into().ok()?);
        let rgba = bytes[8..].to_vec();
        let thumbnail = Thumbnail { width, height, rgba };
        thumbnail.is_consistent().then_some(thumbnail)
    }

    /// Store a thumbnail atomically.
    pub fn put(&self, key: u64, size: u32, thumbnail: &Thumbnail) -> std::io::Result<()> {
        let path = self.path(key, size);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut bytes = Vec::with_capacity(8 + thumbnail.rgba.len());
        bytes.extend_from_slice(&thumbnail.width.to_le_bytes());
        bytes.extend_from_slice(&thumbnail.height.to_le_bytes());
        bytes.extend_from_slice(&thumbnail.rgba);
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, &bytes)?;
        fs::rename(&tmp, &path)
    }

    /// Delete every cached file. Never touches the library.
    pub fn clear(&self) -> std::io::Result<()> {
        if self.dir.exists() {
            fs::remove_dir_all(&self.dir)?;
        }
        fs::create_dir_all(&self.dir)
    }
}

/// A thumbnail that finished loading, or the reason it did not.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// Content key the request was made with.
    pub key: u64,
    /// Requested size.
    pub size: u32,
    /// The image, when decoding succeeded.
    pub thumbnail: Option<Thumbnail>,
    /// The failure, when it did not.
    pub error: Option<String>,
}

struct Request {
    key: u64,
    path: PathBuf,
    size: u32,
}

/// A pool of decode workers in front of a [`Cache`].
pub struct Service {
    tx: Sender<Request>,
    rx: Receiver<Loaded>,
    in_flight: HashSet<(u64, u32)>,
    failed: HashSet<(u64, u32)>,
}

impl Service {
    /// Start `workers` threads. Two is enough to keep a grid scrolling on a laptop without
    /// fighting the UI thread for cores during a scan.
    pub fn new(cache: Cache, workers: usize) -> Self {
        let (request_tx, request_rx) = channel::<Request>();
        let (loaded_tx, loaded_rx) = channel::<Loaded>();
        let request_rx = std::sync::Arc::new(std::sync::Mutex::new(request_rx));
        for index in 0..workers.max(1) {
            let cache = cache.clone();
            let rx = request_rx.clone();
            let tx = loaded_tx.clone();
            let _ = std::thread::Builder::new().name(format!("sortlens-thumb-{index}")).spawn(move || {
                loop {
                    let request = match rx.lock() {
                        Ok(guard) => guard.recv(),
                        Err(_) => return,
                    };
                    let Ok(request) = request else {
                        return;
                    };
                    let loaded = load(&cache, &request);
                    if tx.send(loaded).is_err() {
                        return;
                    }
                }
            });
        }
        Self {
            tx: request_tx,
            rx: loaded_rx,
            in_flight: HashSet::new(),
            failed: HashSet::new(),
        }
    }

    /// Ask for a thumbnail. Duplicate requests and known failures are ignored.
    pub fn request(&mut self, key: u64, path: PathBuf, size: u32) {
        if self.failed.contains(&(key, size)) || !self.in_flight.insert((key, size)) {
            return;
        }
        if self.tx.send(Request { key, path, size }).is_err() {
            self.in_flight.remove(&(key, size));
        }
    }

    /// Whether a request for this key and size is still running.
    pub fn is_pending(&self, key: u64, size: u32) -> bool {
        self.in_flight.contains(&(key, size))
    }

    /// Collect everything that finished since the last call.
    pub fn poll(&mut self) -> Vec<Loaded> {
        let mut ready = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(loaded) => {
                    self.in_flight.remove(&(loaded.key, loaded.size));
                    if loaded.thumbnail.is_none() {
                        self.failed.insert((loaded.key, loaded.size));
                    }
                    ready.push(loaded);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        ready
    }
}

fn load(cache: &Cache, request: &Request) -> Loaded {
    if let Some(thumbnail) = cache.get(request.key, request.size) {
        return Loaded {
            key: request.key,
            size: request.size,
            thumbnail: Some(thumbnail),
            error: None,
        };
    }
    match decode_file(&request.path, request.size) {
        Ok(thumbnail) => {
            let _ = cache.put(request.key, request.size, &thumbnail);
            Loaded {
                key: request.key,
                size: request.size,
                thumbnail: Some(thumbnail),
                error: None,
            }
        }
        Err(error) => Loaded {
            key: request.key,
            size: request.size,
            thumbnail: None,
            error: Some(error.to_string()),
        },
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn gradient(width: u32, height: u32) -> Vec<u8> {
        (0..width * height * 3).map(|index| (index % 251) as u8).collect()
    }

    #[test]
    fn resize_never_upscales() {
        let samples = gradient(64, 32);
        let thumb = resize(&samples, 64, 32, 3, 128);
        assert_eq!((thumb.width, thumb.height), (64, 32));
        assert!(thumb.is_consistent());
    }

    #[test]
    fn resize_keeps_aspect_ratio() {
        let samples = gradient(1000, 500);
        let thumb = resize(&samples, 1000, 500, 3, 128);
        assert_eq!(thumb.width, 128);
        assert_eq!(thumb.height, 64);
        assert!(thumb.is_consistent());
    }

    #[test]
    fn portrait_images_are_bounded_on_their_long_edge() {
        let samples = gradient(500, 1000);
        let thumb = resize(&samples, 500, 1000, 3, 128);
        assert_eq!(thumb.height, 128);
        assert_eq!(thumb.width, 64);
    }

    #[test]
    fn a_real_jpeg_decodes_and_is_not_upscaled() {
        let samples = gradient(64, 48);
        let jpeg = craft_jpeg::encode_baseline(&samples, 64, 48, 3, 90, craft_jpeg::ChromaSubsampling::S444, &[]).expect("encode");
        let thumb = decode_jpeg(&jpeg, 128).expect("decode");
        assert_eq!((thumb.width, thumb.height), (64, 48));
        assert!(thumb.is_consistent());
        assert!(thumb.rgba.chunks(4).any(|pixel| pixel[0] > 0));
    }

    #[test]
    fn a_large_jpeg_is_downscaled_to_the_requested_edge() {
        let samples = gradient(800, 400);
        let jpeg = craft_jpeg::encode_baseline(&samples, 800, 400, 3, 85, craft_jpeg::ChromaSubsampling::S420, &[]).expect("encode");
        let thumb = decode_jpeg(&jpeg, 128).expect("decode");
        assert_eq!(thumb.width, 128);
        assert_eq!(thumb.height, 64);
    }

    #[test]
    fn png_round_trips_through_the_dispatcher() {
        let samples = gradient(96, 64);
        let image = craft_image_codecs::Image::from_u8(96, 64, craft_image_codecs::ChannelLayout::Rgb, samples).expect("image");
        let png = craft_image_codecs::encode(
            &image,
            craft_image_codecs::Format::Png,
            &craft_image_codecs::EncodeOptions::default(),
        )
        .expect("encode png");
        let thumb = decode_image(&png, 128).expect("decode png");
        assert_eq!((thumb.width, thumb.height), (96, 64));
        assert!(thumb.is_consistent());
    }

    #[test]
    fn tiff_round_trips_and_is_downscaled() {
        let samples = gradient(400, 300);
        let image = craft_image_codecs::Image::from_u8(400, 300, craft_image_codecs::ChannelLayout::Rgb, samples).expect("image");
        let tiff = craft_image_codecs::encode(
            &image,
            craft_image_codecs::Format::Tiff,
            &craft_image_codecs::EncodeOptions::default(),
        )
        .expect("encode tiff");
        let thumb = decode_image(&tiff, 128).expect("decode tiff");
        assert_eq!(thumb.width, 128);
        assert!(thumb.height <= 128);
        assert!(thumb.is_consistent());
    }

    #[test]
    fn alpha_is_carried_through_the_resize() {
        let mut samples = vec![255u8; 32 * 32 * 4];
        for pixel in samples.chunks_mut(4) {
            pixel[3] = 64;
        }
        let thumb = resize(&samples, 32, 32, 4, 16);
        assert!(thumb.rgba.chunks(4).all(|pixel| pixel[3] == 64));
    }

    #[test]
    fn cache_round_trips_and_ignores_broken_entries() {
        let dir = std::env::temp_dir().join(format!("sortlens-thumbs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let cache = Cache::new(&dir);
        let thumb = resize(&gradient(300, 200), 300, 200, 3, 64);
        assert!(cache.get(42, 64).is_none());
        cache.put(42, 64, &thumb).expect("put");
        let loaded = cache.get(42, 64).expect("cached");
        assert_eq!(loaded, thumb);

        let path = cache.path(42, 64);
        fs::write(&path, b"trunc").expect("write");
        assert!(cache.get(42, 64).is_none(), "a torn cache file reads as a miss");
        cache.clear().expect("clear");
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_service_decodes_on_its_workers() {
        let dir = std::env::temp_dir().join(format!("sortlens-service-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let mut service = Service::new(Cache::new(&dir), 1);
        let missing = dir.join("nope.jpg");
        service.request(1, missing, 128);
        let mut loaded = Vec::new();
        for _ in 0..200 {
            loaded = service.poll();
            if !loaded.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(loaded.len(), 1);
        assert!(loaded[0].thumbnail.is_none());
        assert!(loaded[0].error.is_some());
        assert!(!service.is_pending(1, 128));
        let _ = fs::remove_dir_all(&dir);
    }
}
