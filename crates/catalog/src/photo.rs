//! The photo record.
//!
//! One row per file that the catalog knows about. Paths are stored relative to their root so a
//! library keeps working when the root is remapped to another volume, and the content key is what
//! survives a rename.

use serde::{Deserialize, Serialize};
use sortlens_core::{ColorLabel, Flag, MediaKind, PhotoId, Review};
use sortlens_meta::PhotoMeta;

/// Everything Sortlens knows about one file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Photo {
    /// Stable identifier; never reused.
    pub id: PhotoId,
    /// Root this photo was found under, as an absolute path string.
    pub root: String,
    /// Path inside the root, `/`-separated.
    pub rel: String,
    /// File name, for display and sorting.
    pub name: String,
    /// Size in bytes.
    pub size: u64,
    /// Modification time, seconds since the Unix epoch.
    pub modified: Option<i64>,
    /// Image, raw or video.
    pub kind: MediaKind,
    /// Cheap content identity from the scan.
    pub key: u64,
    /// Stars, 0–5. Zero means unrated.
    #[serde(default)]
    pub rating: u8,
    /// Pick, reject or none.
    #[serde(default)]
    pub flag: Flag,
    /// Colour label, if any.
    #[serde(default)]
    pub label: Option<ColorLabel>,
    /// Filesystem path was missing at the last scan.
    #[serde(default)]
    pub missing: bool,
    /// Pixel width, once metadata has been read.
    #[serde(default)]
    pub width: u32,
    /// Pixel height, once metadata has been read.
    #[serde(default)]
    pub height: u32,
    /// Duration of a video, in milliseconds. `None` for a still or a container this build cannot read.
    #[serde(default)]
    pub duration_ms: Option<u32>,
    /// Frames per second of a video, as the container declares it.
    #[serde(default)]
    pub fps: Option<f32>,
    /// Video codec, e.g. `H.264` — the container's answer, not a decoder's guess.
    #[serde(default)]
    pub video_codec: Option<String>,
    /// Capture time, seconds since the Unix epoch, from EXIF when available.
    #[serde(default)]
    pub captured: Option<i64>,
    /// Camera make and model.
    #[serde(default)]
    pub camera: String,
    /// Lens model.
    #[serde(default)]
    pub lens: String,
    /// ISO speed.
    #[serde(default)]
    pub iso: Option<u32>,
    /// Aperture f-number.
    #[serde(default)]
    pub aperture: Option<f64>,
    /// Exposure time in seconds.
    #[serde(default)]
    pub shutter: Option<f64>,
    /// Focal length in millimetres.
    #[serde(default)]
    pub focal: Option<f64>,
    /// Keyword tags, sorted and deduplicated, lowercase as typed by the user.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether another application has edit settings for this photo.
    #[serde(default)]
    pub edited: bool,
    /// The metadata read failed when this photo was indexed, so it is queued for a retry.
    #[serde(default)]
    pub retry: bool,
    /// Where the camera said it was, when it said anything believable.
    ///
    /// Stored on the record like the capture date, and refreshed by every metadata read — so a re-index of a file
    /// whose EXIF gained a fix picks it up, and one whose bytes changed cannot keep an old dot on the map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<sortlens_geo::LatLon>,
    /// How much fine detail the pixels have, and which algorithm measured it.
    ///
    /// Stored on the photo rather than in a side map because a score belongs to *this file*: a scan that replaces
    /// the record because the file changed drops the score with it, which is right — the pixels changed, so the old
    /// measurement is about a photograph that is no longer there. `None` means it has never been judged, which is
    /// not the same as "blurry" and must never be shown as if it were.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sharpness: Option<sortlens_quality::Score>,
    /// How many times this photo's annotations have changed: rating, flag, label, keywords or capture
    /// time. A sidecar writer or another window can carry the revision it read and the next writer can
    /// refuse a record that has moved on since, rather than overwriting a newer annotation with an
    /// older one (SL-D11). Bumped only when a value actually changes, so a re-set of the same stars
    /// keeps the revision — the record did not change, so there is nothing a stale reader could
    /// clobber. Travels in the log and the snapshot, so a reopen reproduces the same numbers.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub rev: u64,
    /// When Sortlens recorded this photo entering the catalog, seconds since the Unix epoch.
    ///
    /// `None` for every record that predates the field: Sortlens never approximates it from the file's
    /// `mtime`, because "recently modified" is a different and more confusing question than "recently
    /// added". A `None` photo simply cannot answer the Recently Added question, and stays out of that
    /// source until new bytes bring it back through the scan, which records a real instant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added: Option<i64>,
}

/// Serde's `skip_serializing_if` for a plain `u64`, so a photo nobody has annotated writes nothing
/// for the revision and an older reader (which defaults it to 0) sees the same record.
fn is_zero(value: &u64) -> bool {
    *value == 0
}

/// A duration in milliseconds as `0:12` or `1:02:03`.
///
/// Seconds are floored and the minutes are padded, which is what a clock does: a 59.9-second clip reads `0:59`,
/// not `1:00`, and no clip reads `0:5`.
pub fn format_duration(ms: u32) -> String {
    let total = ms / 1000;
    let (hours, minutes, seconds) = (total / 3_600, (total / 60) % 60, total % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

impl Photo {
    /// Build a record from what the walker knows.
    pub fn from_scan(id: PhotoId, root: &str, rel: &str, size: u64, modified: Option<i64>, kind: MediaKind, key: u64) -> Self {
        let name = rel.rsplit('/').next().unwrap_or(rel).to_string();
        Self {
            id,
            root: root.to_string(),
            rel: rel.to_string(),
            name,
            size,
            modified,
            kind,
            key,
            rating: 0,
            flag: Flag::None,
            label: None,
            missing: false,
            duration_ms: None,
            fps: None,
            video_codec: None,
            width: 0,
            height: 0,
            captured: None,
            camera: String::new(),
            lens: String::new(),
            iso: None,
            aperture: None,
            shutter: None,
            focal: None,
            retry: false,
            tags: Vec::new(),
            edited: false,
            sharpness: None,
            location: None,
            rev: 0,
            added: None,
        }
    }

    /// Copy the metadata facts onto the record.
    pub fn apply_meta(&mut self, meta: &PhotoMeta) {
        self.width = meta.width;
        self.height = meta.height;
        // A metadata read replaces the location rather than merging it: the file's EXIF is the only source, and a
        // successive read that finds no GPS means the location is gone, not that the old one should be kept.
        self.location = meta.location;
        self.captured = meta.captured;
        self.camera = meta.camera.clone();
        self.lens = meta.lens.clone();
        self.iso = meta.iso;
        self.aperture = meta.aperture;
        self.shutter = meta.shutter;
        self.focal = meta.focal;
        self.duration_ms = meta.duration_ms;
        self.fps = meta.fps;
        self.video_codec = meta.video_codec.clone();
    }

    /// A video's length as `0:12` or `1:02:03`, and an empty string for a still.
    ///
    /// Pure and shared by the cell, the detail view and the spoken description, so a cell cannot disagree with
    /// the inspector about how long a clip is.
    pub fn duration_label(&self) -> String {
        match self.duration_ms {
            Some(ms) => format_duration(ms),
            None => String::new(),
        }
    }

    /// Exposure summary for the details panel.
    pub fn exposure(&self) -> String {
        PhotoMeta {
            shutter: self.shutter,
            aperture: self.aperture,
            iso: self.iso,
            focal: self.focal,
            ..PhotoMeta::default()
        }
        .exposure_summary()
    }

    /// Absolute path, joined with `/` so it is stable to store and compare.
    pub fn path(&self) -> String {
        if self.rel.is_empty() {
            self.root.clone()
        } else {
            format!("{}/{}", self.root.trim_end_matches('/'), self.rel)
        }
    }

    /// The immediate parent folder relative to the root, for grouping in the grid.
    pub fn folder(&self) -> &str {
        match self.rel.rfind('/') {
            Some(index) => &self.rel[..index],
            None => "",
        }
    }

    /// Take rating, flag, label and keywords from a sidecar, for a photo the catalog has not seen.
    pub fn adopt_review(&mut self, review: &Review) {
        self.rating = review.rating.min(5);
        self.flag = review.flag;
        self.label = review.label;
        self.tags = review.tags.clone();
        self.tags.sort_unstable();
        self.tags.dedup();
    }

    /// Whether the photo carries a tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|existing| existing == tag)
    }

    /// Whether the photo carries any review state.
    pub fn is_reviewed(&self) -> bool {
        self.rating > 0 || self.flag != Flag::None || self.label.is_some()
    }

    /// Capture time as `2026-11-21 14:32`, when known.
    pub fn captured_label(&self) -> Option<String> {
        self.captured.map(sortlens_meta::format_epoch)
    }

    /// Timestamp used for ordering when EXIF is absent.
    pub fn timestamp(&self) -> i64 {
        self.captured.or(self.modified).unwrap_or(0)
    }

    /// When this photo entered the catalog, as `2026-11-21 14:32`, when Sortlens recorded it.
    pub fn added_label(&self) -> Option<String> {
        self.added.map(sortlens_meta::format_epoch)
    }
}

/// The current wall clock, as seconds since the Unix epoch.
///
/// The one place the catalog asks the clock, so tests and callers can share the same definition of
/// "now" instead of each subtracting 30 days from a different instant.
pub fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or(0)
}

/// Aggregate counts, recomputed as the catalog changes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stats {
    pub total: usize,
    pub rated: usize,
    pub picks: usize,
    pub rejects: usize,
    pub labelled: usize,
    pub tagged: usize,
    pub missing: usize,
    pub bytes: u64,
}

/// Durations read like a clock: minutes and seconds, hours only when there are hours.
#[test]
fn a_duration_reads_like_a_clock() {
    assert_eq!(format_duration(0), "0:00");
    assert_eq!(format_duration(999), "0:00");
    assert_eq!(format_duration(5_000), "0:05");
    assert_eq!(format_duration(59_900), "0:59", "floored, not rounded up into the next minute");
    assert_eq!(format_duration(60_000), "1:00");
    assert_eq!(format_duration(3_723_000), "1:02:03");
    assert_eq!(format_duration(3_600_000), "1:00:00", "an hour is not `60:00`");
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn name_and_folder_come_from_the_relative_path() {
        let photo = Photo::from_scan(PhotoId(1), "/photos", "2026/trip/IMG_1.jpg", 10, Some(5), MediaKind::Image, 7);
        assert_eq!(photo.name, "IMG_1.jpg");
        assert_eq!(photo.folder(), "2026/trip");
        assert_eq!(photo.path(), "/photos/2026/trip/IMG_1.jpg");
    }

    #[test]
    fn a_root_level_file_has_no_folder() {
        let photo = Photo::from_scan(PhotoId(1), "/photos/", "a.jpg", 10, None, MediaKind::Image, 7);
        assert_eq!(photo.folder(), "");
        assert_eq!(photo.path(), "/photos/a.jpg");
    }

    #[test]
    fn timestamp_prefers_capture_time() {
        let mut photo = Photo::from_scan(PhotoId(1), "/p", "a.jpg", 1, Some(100), MediaKind::Image, 1);
        assert_eq!(photo.timestamp(), 100);
        photo.captured = Some(50);
        assert_eq!(photo.timestamp(), 50);
    }
}
