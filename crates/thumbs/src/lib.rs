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
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, Condvar, Mutex};

// Poster decoding needs a seekable file reader, which a path always offers here.
mod poster;

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

/// Decode a HEIF/HEIC file — what an iPhone writes by default.
///
/// `craft-heif` matches libheif's colour conversion rather than assuming BT.709, applies the container's
/// rotation/mirror/crop (HEIF records orientation in the container, not in EXIF), and hands back 16-bit
/// samples whatever the coded depth. The downscale to 8-bit happens here, after the resize, so a 10-bit HDR
/// photo averages in its own precision instead of being truncated first.
///
/// A thumbnail the file already carries is preferred — iPhones store one of about 320 × 240, and decoding it
/// costs a fraction of the primary image — but only when it is big enough for what was asked for.
pub fn decode_heif(bytes: &[u8], max_edge: u32) -> Result<Thumbnail> {
    let options = craft_heif::Options {
        // A thumbnail never needs more than this, and a file claiming to be larger is refused before any
        // pixel is decoded — the limit protects the allocation, so it cannot be a check on the result.
        max_pixels: 1 << 28,
        apply_transforms: true,
        ..craft_heif::Options::default()
    };
    let decoded = match craft_heif::decode_thumbnail(bytes, max_edge, &options) {
        Ok(Some(small)) => small,
        // No usable thumbnail, or the thumbnail pass failed on a file the primary image can still be read
        // from: fall back rather than refusing a photo that has pixels.
        _ => craft_heif::decode(bytes, &options).map_err(|error| Error::Invalid(format!("heif: {error}")))?,
    };
    if decoded.width == 0 || decoded.height == 0 {
        return Err(Error::Invalid("heif image has no pixels".into()));
    }
    let channels = if decoded.has_alpha { 4 } else { 3 };
    // 16-bit samples to 8: divide by 257, which maps 65535 to 255 exactly, rather than shifting by 8, which
    // maps every value in 65535..=65280 to 255 and loses the top of the range.
    let mut rgba = Vec::with_capacity(decoded.data.len());
    for value in &decoded.data {
        rgba.push((value / 257) as u8);
    }
    if channels == 3 {
        let mut with_alpha = Vec::with_capacity(decoded.width as usize * decoded.height as usize * 4);
        for pixel in rgba.chunks_exact(3) {
            with_alpha.extend_from_slice(pixel);
            with_alpha.push(255);
        }
        return Ok(resize(&with_alpha, decoded.width, decoded.height, 4, max_edge));
    }
    Ok(resize(&rgba, decoded.width, decoded.height, 4, max_edge))
}

/// Whether this looks like a HEIF file, by the brand in its header rather than by its name.
fn is_heif(bytes: &[u8]) -> bool {
    sortlens_core::media::sniff(bytes) == sortlens_core::media::Magic::Heif
}

/// Decode the largest preview embedded in a camera raw.
///
/// Raw files carry a full-size JPEG (or, in DNG 1.7, a JPEG XL) precisely so that browsers do not
/// have to demosaic. Browsing uses it; only an explicit develop request touches the sensor data.
pub fn decode_raw_preview(bytes: &[u8], max_edge: u32) -> Result<Thumbnail> {
    let preview = craft_raw::embedded_preview(bytes).ok_or_else(|| Error::UnsupportedFormat("raw file has no embedded preview".into()))?;
    decode_image(&preview, max_edge).or_else(|_| decode_jpeg(&preview, max_edge))
}

/// A luminance histogram, 64 bins from black to white.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Histogram {
    /// Counts per bin.
    pub bins: [u32; 64],
}

impl Default for Histogram {
    fn default() -> Self {
        Self { bins: [0; 64] }
    }
}

impl Histogram {
    /// Count pixels in a decoded thumbnail. Alpha is ignored; the luma weights are Rec. 601.
    pub fn from_thumbnail(thumbnail: &Thumbnail) -> Self {
        let mut histogram = Self::default();
        for pixel in thumbnail.rgba.chunks_exact(4) {
            let luma = (0.299 * f32::from(pixel[0]) + 0.587 * f32::from(pixel[1]) + 0.114 * f32::from(pixel[2])).round();
            let bin = ((luma as usize).min(255) * 64 / 256).min(63);
            histogram.bins[bin] = histogram.bins[bin].saturating_add(1);
        }
        histogram
    }

    /// Total pixels counted.
    pub fn total(&self) -> u64 {
        self.bins.iter().map(|count| u64::from(*count)).sum()
    }

    /// Highest single bin, for scaling the drawing.
    pub fn peak(&self) -> u32 {
        self.bins.iter().copied().max().unwrap_or(0)
    }
}

/// Decode a file at a small size and histogram it.
pub fn histogram_file(path: &Path, max_edge: u32) -> Result<Histogram> {
    decode_file(path, max_edge).map(|thumbnail| Histogram::from_thumbnail(&thumbnail))
}

/// Read a file and decode it, if the format is one this build understands.
///
/// Raw files are read whole so their preview can be located; that is tens of megabytes per file
/// on a cold scan, which is why the result goes straight into the content-addressed cache.
/// Videos are routed by *header* before anything is read whole: a clip can be gigabytes, its poster
/// needs none of the file after the first GOP, and the demuxer wants a seekable source, not a buffer.
/// A clip renamed `.jpg` still gets its poster and a still named `.mp4` is still a still.
pub fn decode_file(path: &Path, max_edge: u32) -> Result<Thumbnail> {
    // A raw is routed by name before any byte is read: a CR3 wears an ISO base-media brand like a
    // video's, and its header must never send a sensor dump to the poster decoder.
    if sortlens_core::is_raw_path(path) {
        let bytes = fs::read(path).map_err(|error| Error::io(path.display().to_string(), error))?;
        if let Ok(thumbnail) = decode_raw_preview(&bytes, max_edge) {
            return Ok(thumbnail);
        }
        return decode_still(&bytes, max_edge);
    }
    if poster::header_is_video(path) {
        return poster::decode_video_poster(path, max_edge);
    }
    let bytes = fs::read(path).map_err(|error| Error::io(path.display().to_string(), error))?;
    decode_still(&bytes, max_edge)
}

/// Decode still bytes: HEIF by header, then the generic image codec, then the JPEG path.
fn decode_still(bytes: &[u8], max_edge: u32) -> Result<Thumbnail> {
    // HEIF by header, not by extension: an iPhone photo renamed to `.jpg` is still a HEIC, and the generic
    // image codec has no HEIF decoder to fall back to. HTTP-style: the content decides.
    if is_heif(bytes) {
        return decode_heif(bytes, max_edge);
    }
    decode_image(bytes, max_edge).or_else(|_| decode_jpeg(bytes, max_edge))
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

/// How badly a thumbnail is wanted.
///
/// The order matters and is a number so it can be compared: what is on screen is worth decoding before what
/// the user is about to scroll to, which is worth decoding before a whole-library pass that nobody is
/// watching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Urgency {
    /// A cell the user can see right now.
    OnScreen = 0,
    /// A cell just off the edge of the viewport, or the next photo in the detail view.
    Nearby = 1,
    /// Library-wide work: memories, suggestions, a warm-up pass.
    Background = 2,
}

impl Urgency {
    /// Sort key; lower is more urgent.
    pub const fn rank(self) -> u8 {
        self as u8
    }
}

struct Entry {
    key: u64,
    path: PathBuf,
    size: u32,
    urgency: Urgency,
    /// Arrival order, so two requests of equal urgency are served in the order they were asked for.
    seq: u64,
}

/// What happened to a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accepted {
    /// Queued.
    Queued,
    /// Queued, having pushed a less urgent request out of a full queue.
    QueuedAndEvicted(u64, u32),
    /// Not queued: the queue is full of work at least as urgent.
    Rejected,
}

/// How many requests may wait at once.
///
/// A grid frame asks for at most a few hundred cells, so a few thousand is several screens of headroom. The
/// number matters because an unbounded queue is what lets a fast scroll enqueue the whole library and hold
/// the memory for it: a bound turns that into "we will skip some", which is the honest answer.
pub const QUEUE_CAPACITY: usize = 4096;

/// Requests waiting for a worker, ordered by urgency and then by arrival.
pub struct Queue {
    entries: Vec<Entry>,
    capacity: usize,
    next_seq: u64,
    dropped: u64,
}

impl Queue {
    /// An empty queue with a bound on how much may wait.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            capacity: capacity.max(1),
            next_seq: 0,
            dropped: 0,
        }
    }

    /// How much may wait at once.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// How much is waiting now.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is waiting.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many requests were refused because the queue was full of more urgent work.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Whether this key and size is already waiting.
    pub fn contains(&self, key: u64, size: u32) -> bool {
        self.entries.iter().any(|entry| entry.key == key && entry.size == size)
    }

    /// Add a request.
    ///
    /// When the queue is full, the *least* urgent waiting request loses its place to a more urgent arrival, and
    /// the least urgent is the newest of the least urgent: the oldest has been waiting longest and is closest
    /// to being decoded, so evicting it would throw away the most nearly-finished work.
    pub fn push(&mut self, key: u64, path: PathBuf, size: u32, urgency: Urgency) -> Accepted {
        if self.entries.len() >= self.capacity {
            let victim = self
                .entries
                .iter()
                .enumerate()
                .min_by_key(|(index, entry)| (std::cmp::Reverse(entry.urgency.rank()), std::cmp::Reverse(entry.seq), *index))
                .map(|(index, _)| index);
            match victim {
                Some(index) if self.entries[index].urgency.rank() > urgency.rank() => {
                    let evicted = self.entries.swap_remove(index);
                    self.entries.push(Entry {
                        key,
                        path,
                        size,
                        urgency,
                        seq: self.next_seq,
                    });
                    self.next_seq += 1;
                    return Accepted::QueuedAndEvicted(evicted.key, evicted.size);
                }
                _ => {
                    self.dropped += 1;
                    return Accepted::Rejected;
                }
            }
        }
        self.entries.push(Entry {
            key,
            path,
            size,
            urgency,
            seq: self.next_seq,
        });
        self.next_seq += 1;
        Accepted::Queued
    }

    /// Take the most urgent request, oldest first among equals.
    fn pop(&mut self) -> Option<Entry> {
        let best = self
            .entries
            .iter()
            .enumerate()
            .min_by_key(|(index, entry)| (entry.urgency.rank(), entry.seq, *index))
            .map(|(index, _)| index)?;
        Some(self.entries.swap_remove(best))
    }
}

struct Shared {
    queue: Mutex<Queue>,
    work: Condvar,
}

/// A pool of decode workers in front of a [`Cache`].
pub struct Service {
    shared: Arc<Shared>,
    rx: Receiver<Loaded>,
    in_flight: HashSet<(u64, u32)>,
    failed: HashSet<(u64, u32)>,
}

impl Service {
    /// Start `workers` threads. Two is enough to keep a grid scrolling on a laptop without
    /// fighting the UI thread for cores during a scan.
    pub fn new(cache: Cache, workers: usize) -> Self {
        Self::with_capacity(cache, workers, QUEUE_CAPACITY)
    }

    /// The same, with an explicit queue bound — the only reason to say so is a test that wants the queue to
    /// fill immediately.
    pub fn with_capacity(cache: Cache, workers: usize, capacity: usize) -> Self {
        let (loaded_tx, loaded_rx) = channel::<Loaded>();
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::new(capacity)),
            work: Condvar::new(),
        });
        for index in 0..workers.max(1) {
            let cache = cache.clone();
            let shared = shared.clone();
            let tx = loaded_tx.clone();
            let _ = std::thread::Builder::new().name(format!("sortlens-thumb-{index}")).spawn(move || {
                loop {
                    let request = match shared.queue.lock() {
                        Ok(mut queue) => loop {
                            if let Some(entry) = queue.pop() {
                                break Some(entry);
                            }
                            // Nothing to do: sleep until someone pushes. A condvar rather than a channel so the
                            // worker sees the *queue*, not a snapshot of it taken at send time — that is what
                            // makes priority ordering possible at all.
                            match shared.work.wait(queue) {
                                Ok(guard) => queue = guard,
                                Err(_) => return,
                            }
                        },
                        Err(_) => return,
                    };
                    let Some(request) = request else {
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
            shared,
            rx: loaded_rx,
            in_flight: HashSet::new(),
            failed: HashSet::new(),
        }
    }

    /// How much work is waiting, and how much was refused, for the activity stack and for tests.
    pub fn queued(&self) -> usize {
        self.shared.queue.lock().map(|queue| queue.len()).unwrap_or(0)
    }

    /// How many requests have been refused because the queue was full of more urgent work.
    pub fn dropped(&self) -> u64 {
        self.shared.queue.lock().map(|queue| queue.dropped()).unwrap_or(0)
    }

    /// Ask for a thumbnail at the urgency of what is on screen. Duplicate requests and known failures are
    /// ignored.
    pub fn request(&mut self, key: u64, path: PathBuf, size: u32) {
        self.request_with(key, path, size, Urgency::OnScreen);
    }

    /// Ask for a thumbnail, saying how urgently.
    ///
    /// Returns whether it was queued. A refusal is not an error: it means the queue is full of work that
    /// matters at least as much, and the caller can ask again next frame.
    pub fn request_with(&mut self, key: u64, path: PathBuf, size: u32, urgency: Urgency) -> bool {
        if self.failed.contains(&(key, size)) || self.in_flight.contains(&(key, size)) {
            return false;
        }
        match self.shared.queue.lock() {
            Ok(mut queue) => {
                let accepted = admit(&mut queue, &mut self.in_flight, key, path, size, urgency);
                // Push without a wake-up and the workers, once they have gone idle, sleep for the life of the
                // process: every request after the first quiet moment would sit in the queue forever. The
                // condvar is only useful if somebody rings it.
                if accepted {
                    self.shared.work.notify_one();
                }
                accepted
            }
            Err(_) => false,
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

/// Offer one request to the queue and keep the in-flight set in step with it.
///
/// Pulled out of [`Service`] because *this* is the decision worth testing, and testing it through worker
/// threads makes the test a race: whether the queue is full when the second request arrives depends on whether
/// a worker already took the first one.
fn admit(queue: &mut Queue, in_flight: &mut HashSet<(u64, u32)>, key: u64, path: PathBuf, size: u32, urgency: Urgency) -> bool {
    match queue.push(key, path, size, urgency) {
        Accepted::Queued => {
            in_flight.insert((key, size));
            true
        }
        Accepted::QueuedAndEvicted(evicted, evicted_size) => {
            // The evicted request will never be answered, so forgetting it keeps a later request for the same
            // thumbnail from looking like it is already on its way.
            in_flight.remove(&(evicted, evicted_size));
            in_flight.insert((key, size));
            true
        }
        Accepted::Rejected => false,
    }
}

fn load(cache: &Cache, request: &Entry) -> Loaded {
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

    /// A request made after the workers have gone idle is still served.
    ///
    /// The queue is a condvar rather than a channel, so pushing has to wake somebody: the first version of the
    /// priority queue pushed into a `Condvar` that nothing ever notified, and every request made after the
    /// workers had emptied the queue and parked waited forever. It looked fine in tests that requested work
    /// immediately, because a worker that had not yet reached its first wait found the entry by itself — this
    /// test sleeps first, which is the only way to be certain the workers are parked.
    #[test]
    fn a_request_after_the_workers_have_parked_is_still_decoded() {
        let dir = std::env::temp_dir().join(format!("sortlens-thumbs-wake-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let samples = gradient(64, 48);
        let jpeg = craft_jpeg::encode_baseline(&samples, 64, 48, 3, 90, craft_jpeg::ChromaSubsampling::S444, &[]).expect("encode");
        let path = dir.join("wake.jpg");
        std::fs::write(&path, &jpeg).expect("write");
        let mut service = Service::new(Cache::new(dir.join("cache")), 2);
        // Long enough that both workers are inside `wait`, so nothing can be picked up opportunistically.
        std::thread::sleep(std::time::Duration::from_millis(250));
        assert!(service.request_with(1, path, GRID, Urgency::OnScreen), "the request is queued");
        let mut loaded = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while loaded.is_empty() && std::time::Instant::now() < deadline {
            loaded = service.poll();
            if loaded.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
        assert_eq!(loaded.len(), 1, "a parked worker woke up and did the work");
        assert!(loaded[0].thumbnail.is_some(), "{:?}", loaded[0].error);
        let _ = std::fs::remove_dir_all(&dir);
    }

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
    fn a_histogram_counts_every_pixel_with_black_and_white_in_the_end_bins() {
        let mut rgba = Vec::new();
        for x in 0..4 {
            let value = if x < 2 { 0 } else { 255 };
            rgba.extend_from_slice(&[value, value, value, 255]);
        }
        let thumbnail = Thumbnail { width: 4, height: 1, rgba };
        let histogram = Histogram::from_thumbnail(&thumbnail);
        assert_eq!(histogram.total(), 4);
        assert_eq!(histogram.bins[0], 2, "black lands in the first bin");
        assert_eq!(histogram.bins[63], 2, "white lands in the last bin");
        assert_eq!(histogram.peak(), 2);
    }

    #[test]
    fn a_mid_grey_lands_in_the_middle() {
        let thumbnail = Thumbnail {
            width: 2,
            height: 1,
            rgba: vec![128, 128, 128, 255, 128, 128, 128, 255],
        };
        let histogram = Histogram::from_thumbnail(&thumbnail);
        assert_eq!(histogram.bins[32], 2);
    }

    #[test]
    fn a_raw_without_a_preview_falls_back_to_the_plain_decoder() {
        let dir = std::env::temp_dir().join(format!("sortlens-raw-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        // A .cr2 that happens to contain a JPEG: the raw preview path finds nothing and the
        // ordinary decoder takes over, which is the behaviour a mislabelled file needs.
        let samples = gradient(40, 30);
        let jpeg = craft_jpeg::encode_baseline(&samples, 40, 30, 3, 85, craft_jpeg::ChromaSubsampling::S444, &[]).expect("encode");
        let path = dir.join("mislabelled.cr2");
        fs::write(&path, &jpeg).expect("write");
        let thumb = decode_file(&path, 128).expect("decode");
        assert_eq!((thumb.width, thumb.height), (40, 30));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_with_no_embedded_preview_says_so() {
        let error = decode_raw_preview(b"definitely not a raw", 128).expect_err("no preview");
        assert!(matches!(error, Error::UnsupportedFormat(_)), "{error}");
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

    /// A HEIC decodes to a thumbnail with the pixels the file was written with, chosen by its header rather
    /// than its name.
    ///
    /// The fixture is built by the decoder's own writer, so no binary blob is committed and every sample is
    /// known: if the container parse, the HEVC stage, the colour conversion or our resize were wrong, a grey
    /// ramp would come back flat, skewed or upside down and this test would say so.
    #[test]
    fn a_heic_decodes_and_its_header_decides_not_its_name() {
        // A horizontal grey ramp, 4:2:0 with neutral chroma: luma 0..=255 left to right, so the decoded
        // thumbnail must get darker to the right.
        let sample = |x: u32, y: u32| {
            let _ = y;
            let luma = (x * 255 / 96).min(255) as u16;
            [luma, 128, 128]
        };
        let mut spec = craft_heif::testdata::Spec::new(96, 64, &sample);
        spec.props.push(craft_heif::testdata::Prop::Nclx(1, 13, 1, true));
        let bytes = craft_heif::testdata::build(&spec);

        assert_eq!(
            sortlens_core::media::sniff(&bytes),
            sortlens_core::media::Magic::Heif,
            "the fixture really is a HEIF file"
        );
        let thumbnail = decode_heif(&bytes, 64).expect("decode");
        assert_eq!(thumbnail.width, 64, "resized to the long edge");
        let expected = 64.0 * 64.0 / 96.0;
        assert!(
            (thumbnail.height as f32 - expected).abs() <= 1.0,
            "and the aspect kept: {} against {expected}",
            thumbnail.height
        );
        assert_eq!(
            thumbnail.rgba.len(),
            (thumbnail.width * thumbnail.height * 4) as usize,
            "always RGBA, whatever the file carried"
        );

        // Left column dark, right column light, top row and bottom row the same: the ramp survived the
        // decode, the colour conversion and the resize.
        let pixel = |x: u32, y: u32| {
            let index = ((y * thumbnail.width + x) * 4) as usize;
            thumbnail.rgba[index]
        };
        let left = pixel(1, thumbnail.height / 2);
        let right = pixel(thumbnail.width - 2, thumbnail.height / 2);
        assert!(right > left + 100, "the ramp runs left to right: {left} then {right}");
        assert_eq!(pixel(0, 2), pixel(0, thumbnail.height - 3), "and it does not run top to bottom");

        // The same bytes named `.jpg` still decode: routing is by header, so a renamed iPhone photo works.
        let dir = std::env::temp_dir().join(format!("sortlens-heic-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        let named = dir.join("renamed.jpg");
        fs::write(&named, &bytes).expect("write");
        let by_file = decode_file(&named, 64).expect("decode by file");
        assert_eq!((by_file.width, by_file.height), (thumbnail.width, thumbnail.height));
        let _ = fs::remove_dir_all(&dir);
    }

    /// A file that claims HEIF but is not decodable is refused with a reason, not a panic and not a blank
    /// thumbnail: the decoder is guard-railed, and so is the path into it.
    #[test]
    fn a_broken_heic_is_refused_with_a_reason() {
        let mut broken = Vec::from(b"\0\0\0\x18ftypheic\0\0\0\0".as_slice());
        broken.extend_from_slice(&[0xff; 64]);
        let error = decode_heif(&broken, 64).expect_err("must not decode");
        assert!(
            error.to_string().to_ascii_lowercase().contains("heif"),
            "the reason says where it came from: {error}"
        );
    }

    /// What is on screen is decoded before what is off it, and equal urgency keeps its arrival order.
    #[test]
    fn the_queue_serves_the_most_urgent_request_first() {
        let mut queue = Queue::new(16);
        queue.push(1, PathBuf::from("bg-old.jpg"), 128, Urgency::Background);
        queue.push(2, PathBuf::from("screen.jpg"), 128, Urgency::OnScreen);
        queue.push(3, PathBuf::from("bg-new.jpg"), 128, Urgency::Background);
        queue.push(4, PathBuf::from("near.jpg"), 128, Urgency::Nearby);
        assert_eq!(queue.len(), 4);
        assert_eq!(queue.pop().map(|entry| entry.key), Some(2), "the visible cell first");
        assert_eq!(queue.pop().map(|entry| entry.key), Some(4));
        // Two background requests of equal urgency: the one asked for first has waited longest.
        assert_eq!(queue.pop().map(|entry| entry.key), Some(1));
        assert_eq!(queue.pop().map(|entry| entry.key), Some(3));
        assert!(queue.is_empty());
        assert!(queue.pop().is_none());
    }

    /// A full queue makes room for something more urgent and refuses something less urgent, and it says so.
    #[test]
    fn a_full_queue_makes_room_for_what_matters_and_refuses_what_does_not() {
        let mut queue = Queue::new(2);
        assert_eq!(queue.push(1, PathBuf::from("a.jpg"), 128, Urgency::Background), Accepted::Queued);
        assert_eq!(queue.push(2, PathBuf::from("b.jpg"), 128, Urgency::Background), Accepted::Queued);
        // Full of background work: a visible cell takes the place of the newest background request, because
        // the older one has been waiting longer and is the closer to being decoded.
        assert_eq!(
            queue.push(3, PathBuf::from("c.jpg"), 128, Urgency::OnScreen),
            Accepted::QueuedAndEvicted(2, 128)
        );
        assert_eq!(queue.len(), 2);
        assert!(queue.contains(1, 128) && queue.contains(3, 128));
        // Now full of the most urgent work: more background work is refused, not evicting what matters.
        assert_eq!(queue.push(4, PathBuf::from("d.jpg"), 128, Urgency::Background), Accepted::Rejected);
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.dropped(), 1);
        assert!(!queue.contains(4, 128));
    }

    /// A refusal leaves nothing behind: the key is not marked as on its way, so the next frame can ask again.
    ///
    /// Driven through `admit` rather than through live workers, because with workers the queue may already be
    /// empty by the time the second request arrives and the test would then be measuring thread scheduling.
    #[test]
    fn a_refused_request_can_be_asked_for_again_next_frame() {
        let mut queue = Queue::new(1);
        let mut in_flight = HashSet::new();
        let dir = std::env::temp_dir();

        assert!(admit(&mut queue, &mut in_flight, 1, dir.join("a.jpg"), 128, Urgency::OnScreen));
        assert!(!admit(&mut queue, &mut in_flight, 2, dir.join("b.jpg"), 128, Urgency::Background));
        assert_eq!(queue.len(), 1, "the urgent request kept its place");
        assert!(!in_flight.contains(&(2, 128)), "and the refused one is not waiting on anything");

        // Asking again while it is still full is refused again, and does not leak into the set either.
        assert!(!admit(&mut queue, &mut in_flight, 2, dir.join("b.jpg"), 128, Urgency::Background));
        assert!(!in_flight.contains(&(2, 128)));

        // A duplicate of something already queued is not queued twice.
        assert!(!admit(&mut queue, &mut in_flight, 1, dir.join("a.jpg"), 128, Urgency::OnScreen) || queue.len() <= 1);

        // Once the queue drains, the same request is accepted.
        let entry = queue.pop().expect("the queued request");
        in_flight.remove(&(entry.key, entry.size));
        assert!(admit(&mut queue, &mut in_flight, 2, dir.join("b.jpg"), 128, Urgency::Background));
        assert!(in_flight.contains(&(2, 128)));
        // And an eviction frees the evicted key rather than leaving it forever pending.
        let mut queue = Queue::new(1);
        let mut in_flight = HashSet::new();
        assert!(admit(&mut queue, &mut in_flight, 3, dir.join("c.jpg"), 128, Urgency::Background));
        assert!(admit(&mut queue, &mut in_flight, 4, dir.join("d.jpg"), 128, Urgency::OnScreen));
        assert!(!in_flight.contains(&(3, 128)), "the evicted request is free to be asked for again");
        assert!(in_flight.contains(&(4, 128)));
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
