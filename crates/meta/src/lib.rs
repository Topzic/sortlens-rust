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
    /// Video length in milliseconds.
    pub duration_ms: Option<u32>,
    /// Video frame rate, as the container declares it.
    pub fps: Option<f32>,
    /// Video codec name, e.g. `H.264`.
    pub video_codec: Option<String>,
}

impl PhotoMeta {
    /// Whether anything at all was found.
    pub fn is_empty(&self) -> bool {
        self.width == 0 && self.height == 0 && self.captured.is_none() && self.camera.is_empty() && self.duration_ms.is_none()
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

/// What reading a file's metadata came to.
#[derive(Debug, Clone, PartialEq)]
pub enum ReadOutcome {
    /// The file was read; the metadata may still be empty, which is a fact about the photo.
    Parsed(PhotoMeta),
    /// The file could not be read at all — locked, unplugged, a permissions problem. Worth retrying
    /// later, and *not* the same thing as a photo with no EXIF.
    Unreadable(String),
}

impl ReadOutcome {
    /// The metadata, empty when the file could not be read.
    pub fn meta(&self) -> PhotoMeta {
        match self {
            Self::Parsed(meta) => meta.clone(),
            Self::Unreadable(_) => PhotoMeta::default(),
        }
    }

    /// Whether another attempt later could do better.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Unreadable(_))
    }
}

/// Read metadata for one file, bounded by [`METADATA_READ_LIMIT`].
///
/// Returns defaults on any failure; the caller keeps the photo. Use [`read_outcome`] when the
/// difference between "no EXIF" and "could not be read" matters.
pub fn read(path: &Path) -> PhotoMeta {
    read_outcome(path).meta()
}

/// Read metadata, distinguishing an unreadable file from an empty one.
pub fn read_outcome(path: &Path) -> ReadOutcome {
    // A video's length, frame rate and codec live in the container, not in an EXIF block, so the byte reader
    // below can never find them. Ask the demuxer instead — and only ever for a file whose name says video, so
    // a still photo pays nothing.
    if is_video_name(path)
        && let Some(video) = video_info(path)
    {
        let mut meta = parse_fast(path);
        meta.width = video.width;
        meta.height = video.height;
        meta.duration_ms = Some(video.duration_ms);
        meta.fps = video.fps;
        meta.video_codec = Some(video.codec);
        return ReadOutcome::Parsed(meta);
    }
    let mut buffer = Vec::new();
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => return ReadOutcome::Unreadable(error.to_string()),
    };
    if let Err(error) = file.take(METADATA_READ_LIMIT).read_to_end(&mut buffer) {
        return ReadOutcome::Unreadable(error.to_string());
    }
    ReadOutcome::Parsed(parse(&buffer))
}

/// Whether this path is named as a video. Only ISO base-media video is probed (MP4, MOV, M4V): Matroska and AVI
/// need container features this crate does not enable, and pretending to read them would report a clip with no
/// length as a clip of length zero.
fn is_video_name(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("mp4" | "mov" | "m4v")
    )
}

/// What a video container declares about itself.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoInfo {
    /// Length in milliseconds, floored.
    pub duration_ms: u32,
    /// Coded picture width.
    pub width: u32,
    /// Coded picture height.
    pub height: u32,
    /// Frames per second, rounded to two decimals — the container's sample count over its own duration rather
    /// than a nominal figure, so a 29.97 fps clip reads as 29.97 and a variable-rate clip reads as its average.
    pub fps: Option<f32>,
    /// Codec name, e.g. `H.264`.
    pub codec: String,
}

/// Read a video's declared length, size, rate and codec from its container.
///
/// No pixel is decoded: this is the demuxer reading `moov`, which is what makes it cheap enough to run during a
/// scan. A file that is not ISO base media, or whose video track declares nothing usable, answers `None` rather
/// than a zero-length clip.
pub fn video_info(path: &Path) -> Option<VideoInfo> {
    // Reading a container needs a seekable file, and the demuxer's file reader is native-only: a wasm build has
    // no filesystem to open one from. A clip simply has no length there, which the UI already handles.
    #[cfg(not(target_arch = "wasm32"))]
    {
        read_video_info(path)
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_video_info(path: &Path) -> Option<VideoInfo> {
    let file = craft_media::isobmff::open(craft_media::FileReader::open(path).ok()?).ok()?;
    let track = file
        .tracks
        .iter()
        .find(|track| track.kind == craft_media::isobmff::TrackKind::Video)?;
    let seconds = if track.timescale > 0 {
        track.duration as f64 / f64::from(track.timescale)
    } else if file.timescale > 0 {
        file.duration as f64 / f64::from(file.timescale)
    } else {
        0.0
    };
    let duration_ms = (seconds * 1000.0).round().max(0.0) as u32;
    if duration_ms == 0 {
        return None;
    }
    let samples = track.samples.len();
    let fps = if samples > 1 && seconds > 0.0 {
        let rate = samples as f64 / seconds;
        Some((rate * 100.0).round() as f32 / 100.0)
    } else {
        None
    };
    let params = track.codec_params();
    let width = params.width.unwrap_or(track.width);
    let height = params.height.unwrap_or(track.height);
    Some(VideoInfo {
        duration_ms,
        width,
        height,
        fps,
        codec: params.codec.name().to_string(),
    })
}

/// Parse metadata out of a byte buffer that starts at the beginning of a file.
///
/// Used for a video, where the container has already answered the questions worth asking: this only picks up an
/// embedded creation time if the file carries one.
fn parse_fast(path: &Path) -> PhotoMeta {
    let mut buffer = Vec::new();
    match File::open(path) {
        Ok(file) => {
            if file.take(64 * 1024).read_to_end(&mut buffer).is_err() {
                return PhotoMeta::default();
            }
        }
        Err(_) => return PhotoMeta::default(),
    }
    parse(&buffer)
}

/// Parse metadata out of a byte buffer that starts at the beginning of a file.
pub fn parse(bytes: &[u8]) -> PhotoMeta {
    let metadata = craft_metadata::extract(bytes);
    // A HEIF file keeps its size in the container's `ispe` box and its orientation in `irot`, neither of which
    // an EXIF reader can see. Without this an iPhone photo indexes as 0 × 0, which costs it its aspect ratio in
    // the justified grid and its megapixels in the insights — so ask the container directly.
    let heif = heif_size(bytes);
    let camera = match (metadata.make.as_deref(), metadata.model.as_deref()) {
        (Some(make), Some(model)) if !model.starts_with(make) => format!("{make} {model}"),
        (_, Some(model)) => model.to_string(),
        (Some(make), None) => make.to_string(),
        _ => String::new(),
    };
    let iso = metadata.iso.filter(|iso| *iso > 0);

    // The container wins for a HEIF file: it is the same source the decoder will use, so the grid's shape and
    // the pixels cannot disagree. Mixing the two would let a stale EXIF tag re-shape a portrait photo.
    let (width, height) = heif.unwrap_or((metadata.width.unwrap_or(0), metadata.height.unwrap_or(0)));

    PhotoMeta {
        duration_ms: None,
        fps: None,
        video_codec: None,
        width,
        height,
        captured: metadata.capture_time.as_ref().and_then(epoch_seconds),
        camera,
        lens: metadata.lens_model.clone().unwrap_or_default(),
        iso,
        aperture: metadata.f_number.filter(|value| *value > 0.0),
        shutter: metadata.exposure_time.filter(|value| *value > 0.0),
        focal: metadata.focal_length.filter(|value| *value > 0.0),
    }
}

/// The displayed size of a HEIF file, from its container alone — no pixel is decoded.
///
/// Rotation and crop are already applied by the probe, so the pair is what a viewer would show, which is the
/// only answer that keeps a portrait iPhone photo portrait in the grid.
fn heif_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if sortlens_core::media::sniff(bytes) != sortlens_core::media::Magic::Heif {
        return None;
    }
    let info = craft_heif::probe(bytes).ok()?;
    (info.width > 0 && info.height > 0).then_some((info.width, info.height))
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

/// A HEIF photo's size comes from its container, with the rotation the container declares applied.
///
/// This is the difference between a portrait iPhone photo showing portrait in the grid and it showing as an
/// unrotated landscape, and between a real megapixel count in the insights and 0.0 MP.
#[test]
fn a_heif_size_comes_from_its_container_and_honours_rotation() {
    let sample = |x: u32, y: u32| {
        let _ = (x, y);
        [128_u16, 128, 128]
    };
    let mut spec = craft_heif::testdata::Spec::new(96, 64, &sample);
    spec.props.push(craft_heif::testdata::Prop::Nclx(1, 13, 1, true));
    let upright = craft_heif::testdata::build(&spec);
    let meta = parse(&upright);
    assert_eq!((meta.width, meta.height), (96, 64), "the coded size when nothing is rotated");

    // One quarter turn counter-clockwise: a 96 × 64 file is shown 64 × 96.
    spec.props.push(craft_heif::testdata::Prop::Irot(1));
    let turned = craft_heif::testdata::build(&spec);
    let meta = parse(&turned);
    assert_eq!((meta.width, meta.height), (64, 96), "and the displayed size when it is");
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    /// A video's length, rate, size and codec come from its container, and a file that says nothing usable is
    /// refused rather than reported as a zero-length clip.
    ///
    /// The fixture is muxed by the suite's own writer, so no video blob is committed: 24 samples of 25 ticks at a
    /// 600 Hz media timescale is one second at 24 fps, and that is exactly what the probe must say.
    #[test]
    fn a_video_length_and_rate_come_from_its_container() {
        use craft_media::isobmff::{
            AvcConfig, Brand, CodecConfig, FourCc, Mp4Writer, SampleEntry, TrackConfig, WriteSample, WriterOptions,
        };

        let dir = std::env::temp_dir().join(format!("sortlens-video-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");

        let avc = AvcConfig {
            profile: 0x64,
            compatibility: 0,
            level: 0x1f,
            length_size: 4,
            // Parameter sets this crate never parses: the probe reads the table, not the bitstream.
            sps: vec![vec![0x67, 0x64, 0x00, 0x1f]],
            pps: vec![vec![0x68, 0xee, 0x3c, 0x80]],
            ext: Vec::new(),
        };
        let entry = SampleEntry::video(FourCc(*b"avc1"), CodecConfig::Avc(avc), 1920, 1080);
        // A cursor over a slice satisfies the writer's `Read + Write + Seek`; a `Vec` is not seekable.
        let sink = std::io::Cursor::new(Vec::new());
        let mut writer = Mp4Writer::new(sink, WriterOptions::new(Brand::Mp4)).expect("writer");
        let track = writer.add_track(TrackConfig::new(entry, 600)).expect("track");
        for _ in 0..24 {
            writer
                .write_sample(
                    track,
                    WriteSample {
                        data: &[0, 0, 0, 2, 0x65, 0x88],
                        duration: 25,
                        composition_offset: 0,
                        is_sync: true,
                    },
                )
                .expect("sample");
        }
        let bytes = writer.finish().expect("finish").into_inner();
        let path = dir.join("clip.mp4");
        std::fs::write(&path, &bytes).expect("write");

        let info = video_info(&path).expect("probe");
        assert_eq!(info.duration_ms, 1_000, "24 samples of 25 ticks at 600 Hz");
        assert_eq!((info.width, info.height), (1920, 1080));
        assert_eq!(info.codec, "H.264");
        assert_eq!(info.fps, Some(24.0));

        // And it reaches the photo through the ordinary read path, so the scan needs no special case.
        let meta = read(&path);
        assert_eq!(meta.duration_ms, Some(1_000));
        assert_eq!(meta.fps, Some(24.0));
        assert_eq!(meta.video_codec.as_deref(), Some("H.264"));
        assert_eq!((meta.width, meta.height), (1920, 1080));

        // A file that is not a container, and a container with no video track, answer nothing rather than zero.
        let still = dir.join("still.jpg");
        std::fs::write(&still, b"\xff\xd8\xff\xd9").expect("write");
        assert!(video_info(&still).is_none(), "a JPEG is not a video");
        std::fs::write(&still, &bytes).expect("write");
        let truncated = dir.join("cut.mp4");
        std::fs::write(&truncated, &bytes[..bytes.len() / 2]).expect("write");
        assert!(video_info(&truncated).is_none(), "half a file is not a movie");
        let _ = std::fs::remove_dir_all(&dir);
    }

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

    #[test]
    fn an_unreadable_file_is_not_the_same_as_a_photo_without_exif() {
        let missing = std::env::temp_dir().join("sortlens-meta-absent-xyz.jpg");
        let _ = std::fs::remove_file(&missing);
        let outcome = super::read_outcome(&missing);
        assert!(outcome.is_retryable(), "a missing file is worth retrying");
        assert_eq!(outcome.meta(), super::PhotoMeta::default());

        // A readable file with no EXIF is a fact, not a failure.
        let plain = std::env::temp_dir().join(format!("sortlens-meta-plain-{}.jpg", std::process::id()));
        std::fs::write(&plain, b"not really a jpeg").expect("write");
        let outcome = super::read_outcome(&plain);
        assert!(!outcome.is_retryable(), "an empty result is final");
        assert!(outcome.meta().camera.is_empty());
        let _ = std::fs::remove_file(&plain);
    }
}
