//! L1: photo metadata.
//!
//! One struct, filled from EXIF through `craft-metadata`, with every field optional because
//! files in the wild are. Reading metadata never fails the scan: a file with no EXIF is a photo
//! with an empty camera, not an error.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// How much of a file is read looking for metadata. EXIF lives in the first block of every
/// format we support; reading a 60 MB raw in full to find it would make a scan four times slower.
pub const METADATA_READ_LIMIT: u64 = 8 * 1024 * 1024;

/// The facts we keep per photo.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhotoMeta {
    /// Pixel width before orientation.
    pub width: u32,
    /// Pixel height before orientation.
    pub height: u32,
    /// Capture time, seconds since the Unix epoch, in UTC.
    pub captured: Option<i64>,
    /// Camera make and model, e.g. `FUJIFILM X-T5`.
    pub camera: String,
    /// Lens model.
    pub lens: String,
    /// ISO speed.
    pub iso: Option<u32>,
    /// Aperture f-number.
    pub aperture: Option<f64>,
    /// Exposure time in seconds.
    pub shutter: Option<f64>,
    /// Focal length in millimetres.
    pub focal: Option<f64>,
}

impl PhotoMeta {
    /// Whether anything at all was found.
    pub fn is_empty(&self) -> bool {
        self.width == 0 && self.height == 0 && self.captured.is_none() && self.camera.is_empty()
    }

    /// Megapixels, rounded to one decimal, for the details panel.
    pub fn megapixels(&self) -> f32 {
        (self.width as f32 * self.height as f32 / 1_000_000.0 * 10.0).round() / 10.0
    }

    /// A one-line exposure summary, e.g. `1/250 s · f/2.8 · ISO 400 · 35 mm`.
    pub fn exposure_summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(shutter) = self.shutter {
            if shutter >= 1.0 {
                parts.push(format!("{shutter:.1} s"));
            } else if shutter > 0.0 {
                parts.push(format!("1/{} s", (1.0 / shutter).round() as u32));
            }
        }
        if let Some(aperture) = self.aperture {
            parts.push(format!("f/{aperture:.1}"));
        }
        if let Some(iso) = self.iso {
            parts.push(format!("ISO {iso}"));
        }
        if let Some(focal) = self.focal {
            parts.push(format!("{focal:.0} mm"));
        }
        parts.join(" · ")
    }
}

/// Read metadata for one file, bounded by [`METADATA_READ_LIMIT`].
///
/// Returns defaults on any failure; the caller keeps the photo.
pub fn read(path: &Path) -> PhotoMeta {
    let mut buffer = Vec::new();
    let Ok(file) = File::open(path) else {
        return PhotoMeta::default();
    };
    if file.take(METADATA_READ_LIMIT).read_to_end(&mut buffer).is_err() {
        return PhotoMeta::default();
    }
    parse(&buffer)
}

/// Parse metadata out of a byte buffer that starts at the beginning of a file.
pub fn parse(bytes: &[u8]) -> PhotoMeta {
    let metadata = craft_metadata::extract(bytes);
    let camera = match (metadata.make.as_deref(), metadata.model.as_deref()) {
        (Some(make), Some(model)) if !model.starts_with(make) => format!("{make} {model}"),
        (_, Some(model)) => model.to_string(),
        (Some(make), None) => make.to_string(),
        _ => String::new(),
    };
    let iso = metadata.iso.filter(|iso| *iso > 0);

    PhotoMeta {
        width: metadata.width.unwrap_or(0),
        height: metadata.height.unwrap_or(0),
        captured: metadata.capture_time.as_ref().and_then(epoch_seconds),
        camera,
        lens: metadata.lens_model.clone().unwrap_or_default(),
        iso,
        aperture: metadata.f_number.filter(|value| *value > 0.0),
        shutter: metadata.exposure_time.filter(|value| *value > 0.0),
        focal: metadata.focal_length.filter(|value| *value > 0.0),
    }
}

/// Convert a civil capture time to seconds since the Unix epoch.
///
/// Camera clocks are local time with an offset, or floating local time with none. With no
/// offset the value is taken as UTC, which is stable and never off by a time zone twice.
fn epoch_seconds(time: &craft_metadata::DateTime) -> Option<i64> {
    if time.year < 1900 {
        return None;
    }
    let days = days_from_civil(i64::from(time.year), i64::from(time.month), i64::from(time.day))?;
    let seconds = days * 86_400 + i64::from(time.hour) * 3_600 + i64::from(time.minute) * 60 + i64::from(time.second)
        - i64::from(time.offset_minutes.unwrap_or(0)) * 60;
    Some(seconds)
}

/// Format an epoch second as `2026-11-21 14:32` in UTC.
pub fn format_epoch(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let remainder = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = remainder / 3_600;
    let minute = (remainder % 3_600) / 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}")
}

/// Civil date from days since the epoch (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 { month_prime + 3 } else { month_prime - 9 };
    (year + i64::from(month <= 2), month, day)
}

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn epoch_conversion_matches_known_dates() {
        assert_eq!(days_from_civil(1970, 1, 1), Some(0));
        assert_eq!(days_from_civil(2000, 1, 1), Some(10957));
        assert_eq!(days_from_civil(2026, 11, 21), Some(20778));
    }

    #[test]
    fn epoch_round_trips_to_a_readable_date() {
        assert_eq!(format_epoch(0), "1970-01-01 00:00");
        assert_eq!(format_epoch(1_791_000_000), "2026-10-03 04:00");
    }

    #[test]
    fn empty_input_yields_empty_metadata() {
        let meta = parse(b"not a photo at all");
        assert!(meta.is_empty());
        assert_eq!(meta.camera, "");
    }

    #[test]
    fn exposure_summary_reads_like_a_camera_display() {
        let meta = PhotoMeta {
            shutter: Some(1.0 / 250.0),
            aperture: Some(2.8),
            iso: Some(400),
            focal: Some(35.0),
            ..PhotoMeta::default()
        };
        assert_eq!(meta.exposure_summary(), "1/250 s · f/2.8 · ISO 400 · 35 mm");
    }

    #[test]
    fn long_exposures_are_shown_in_seconds() {
        let meta = PhotoMeta {
            shutter: Some(2.0),
            ..PhotoMeta::default()
        };
        assert!(meta.exposure_summary().starts_with("2.0 s"));
    }

    #[test]
    fn megapixels_round_to_one_decimal() {
        let meta = PhotoMeta {
            width: 6000,
            height: 4000,
            ..PhotoMeta::default()
        };
        assert_eq!(meta.megapixels(), 24.0);
    }
}
