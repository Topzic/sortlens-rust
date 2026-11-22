//! Camera hints: what the EXIF already knows, said as keywords.
//!
//! This is the part of tagging that needs no model, no download and no network, and it is often the
//! most useful part: "which photos did I take with the 35mm", "which ones were shot wide open at
//! midnight" are answerable from the file's own metadata.
//!
//! Every hint here is a **proposal**, in the same shape a detector's guess takes, so the review panel
//! merges them without caring where they came from — and, like every other suggestion, nothing is
//! written until someone accepts it.
//!
//! The rules are deliberately few and explicable. A rule that cannot be stated in a sentence is a rule
//! that will surprise the person using it.

use crate::suggest::{Candidate, Source};

/// What a photo's metadata says, reduced to the fields these rules need.
///
/// Deliberately not `sortlens_meta::PhotoMeta`: this crate sits below the catalog and must not depend
/// on it, and a struct with six fields is easier to test than a whole metadata record.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Hints {
    /// Camera make and model, e.g. `FUJIFILM X-T5`.
    pub camera: String,
    /// Lens model.
    pub lens: String,
    /// ISO speed.
    pub iso: Option<u32>,
    /// Exposure time in seconds.
    pub shutter: Option<f64>,
    /// Focal length in millimetres.
    pub focal: Option<f64>,
}

/// ISO at or above this is a low-light shot. 3200 is where most sensors start to show it.
pub const LOW_LIGHT_ISO: u32 = 3200;

/// A shutter this slow or slower is a long exposure: 1/15 s is the usual hand-holding limit.
///
/// Seconds go *up* as the shutter gets slower, so a longer exposure is a larger number — the
/// comparison is `>=`, and getting it backwards is exactly the bug this constant's name is meant to
/// prevent.
pub const LONG_EXPOSURE_AT_OR_ABOVE: f64 = 1.0 / 15.0;

/// The upper bound of "wide", in millimetres: 35mm is the classic reportage focal length.
pub const WIDE_MAX_MM: f64 = 35.0;

/// The upper bound of "normal", in millimetres: 70mm is where portraits start.
pub const NORMAL_MAX_MM: f64 = 70.0;

/// Confidence attached to a camera or lens name, which is read from the file rather than guessed.
const NAMED_SCORE: f32 = 0.95;

/// Confidence attached to a derived band (wide/normal/tele) — a real observation, but a coarse one.
const BAND_SCORE: f32 = 0.7;

/// Confidence attached to a lighting or exposure hint.
const CONDITION_SCORE: f32 = 0.8;

impl Hints {
    /// Build from the fields the rules use.
    #[must_use]
    pub fn new(camera: &str, lens: &str, iso: Option<u32>, shutter: Option<f64>, focal: Option<f64>) -> Hints {
        Hints {
            camera: camera.to_string(),
            lens: lens.to_string(),
            iso,
            shutter,
            focal,
        }
    }

    /// Whether there is anything to say at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        normalise(&self.camera).is_empty()
            && normalise(&self.lens).is_empty()
            && self.iso.is_none()
            && self.shutter.is_none()
            && self.focal.is_none()
    }
}

/// Collapse whitespace and lowercase, which is how the catalog stores a keyword.
fn normalise(text: &str) -> String {
    text.split_whitespace().collect::<Vec<&str>>().join(" ").to_lowercase()
}

/// The focal-length band a shot belongs to, or `None` when the focal length is not known or is not a
/// number a lens could have.
#[must_use]
pub fn focal_band(focal_mm: Option<f64>) -> Option<&'static str> {
    let focal = focal_mm?;
    if !focal.is_finite() || focal <= 0.0 {
        return None;
    }
    if focal <= WIDE_MAX_MM {
        Some("wide")
    } else if focal <= NORMAL_MAX_MM {
        Some("normal")
    } else {
        Some("telephoto")
    }
}

/// Turn metadata into proposals.
///
/// Order is fixed — camera, lens, band, light, exposure — so two runs over the same file produce the
/// same list, and the panel does not reshuffle when a scan order changes.
#[must_use]
pub fn hints(meta: &Hints) -> Vec<Candidate> {
    let mut out = Vec::new();
    let camera = normalise(&meta.camera);
    if !camera.is_empty() {
        out.push(Candidate::new(&camera, NAMED_SCORE, Source::ExifHint));
    }
    let lens = normalise(&meta.lens);
    // A lens name that only repeats the camera is not news.
    if !lens.is_empty() && lens != camera {
        out.push(Candidate::new(&lens, NAMED_SCORE, Source::ExifHint));
    }
    if let Some(band) = focal_band(meta.focal) {
        out.push(Candidate::new(band, BAND_SCORE, Source::ExifHint));
    }
    if meta.iso.is_some_and(|iso| iso >= LOW_LIGHT_ISO) {
        out.push(Candidate::new("low light", CONDITION_SCORE, Source::ExifHint));
    }
    if meta
        .shutter
        .is_some_and(|shutter| shutter.is_finite() && shutter > 0.0 && shutter >= LONG_EXPOSURE_AT_OR_ABOVE)
    {
        out.push(Candidate::new("long exposure", CONDITION_SCORE, Source::ExifHint));
    }
    out
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn tags(meta: &Hints) -> Vec<String> {
        hints(meta).into_iter().map(|candidate| candidate.tag).collect()
    }

    #[test]
    fn a_camera_and_lens_become_keywords_in_lowercase() {
        let meta = Hints::new("FUJIFILM  X-T5 ", "XF 35mmF1.4 R", None, None, None);
        assert_eq!(tags(&meta), vec!["fujifilm x-t5", "xf 35mmf1.4 r"]);
        let scores: Vec<f32> = hints(&meta).iter().map(|candidate| candidate.score).collect();
        assert!(scores.iter().all(|score| (*score - NAMED_SCORE).abs() < 1e-6));
        assert!(
            hints(&meta).iter().all(|candidate| candidate.source == Source::ExifHint),
            "every hint says where it came from"
        );
    }

    #[test]
    fn a_lens_that_only_repeats_the_camera_is_not_suggested_twice() {
        let meta = Hints::new("iPhone 15 Pro", "IPHONE 15 PRO", None, None, None);
        assert_eq!(tags(&meta), vec!["iphone 15 pro"]);
    }

    #[test]
    fn focal_lengths_fall_into_three_bands_and_the_edges_belong_to_the_shorter_one() {
        assert_eq!(focal_band(Some(14.0)), Some("wide"));
        assert_eq!(focal_band(Some(WIDE_MAX_MM)), Some("wide"), "35mm is still wide");
        assert_eq!(focal_band(Some(WIDE_MAX_MM + 0.1)), Some("normal"));
        assert_eq!(focal_band(Some(NORMAL_MAX_MM)), Some("normal"), "70mm is still normal");
        assert_eq!(focal_band(Some(NORMAL_MAX_MM + 0.1)), Some("telephoto"));
        assert_eq!(focal_band(Some(600.0)), Some("telephoto"));
    }

    #[test]
    fn a_focal_length_that_cannot_exist_produces_no_band() {
        assert_eq!(focal_band(None), None);
        assert_eq!(focal_band(Some(0.0)), None);
        assert_eq!(focal_band(Some(-35.0)), None);
        assert_eq!(focal_band(Some(f64::NAN)), None);
    }

    #[test]
    fn a_high_iso_is_low_light_and_the_threshold_is_inclusive() {
        let just_below = Hints::new("", "", Some(LOW_LIGHT_ISO - 1), None, None);
        assert!(tags(&just_below).is_empty());
        let at = Hints::new("", "", Some(LOW_LIGHT_ISO), None, None);
        assert_eq!(tags(&at), vec!["low light"]);
        let above = Hints::new("", "", Some(12800), None, None);
        assert_eq!(tags(&above), vec!["low light"]);
    }

    #[test]
    fn a_slow_shutter_is_a_long_exposure_but_a_fast_one_is_not() {
        let long = Hints::new("", "", None, Some(2.0), None);
        assert_eq!(tags(&long), vec!["long exposure"]);
        let borderline = Hints::new("", "", None, Some(LONG_EXPOSURE_AT_OR_ABOVE), None);
        assert_eq!(tags(&borderline), vec!["long exposure"], "1/15 s counts");
        let fast = Hints::new("", "", None, Some(1.0 / 1000.0), None);
        assert!(tags(&fast).is_empty());
    }

    #[test]
    fn a_nonsense_shutter_produces_no_hint() {
        for shutter in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let meta = Hints::new("", "", None, Some(shutter), None);
            assert!(tags(&meta).is_empty(), "{shutter} should say nothing");
        }
    }

    #[test]
    fn a_full_metadata_record_says_everything_in_a_fixed_order() {
        let meta = Hints::new("Nikon Z6", "NIKKOR Z 24-70mm f/4 S", Some(6400), Some(4.0), Some(24.0));
        assert_eq!(
            tags(&meta),
            vec!["nikon z6", "nikkor z 24-70mm f/4 s", "wide", "low light", "long exposure"]
        );
        // Twice in a row is the same list: nothing here depends on a hash map's order.
        assert_eq!(tags(&meta), tags(&meta));
    }

    #[test]
    fn no_metadata_is_no_hints() {
        let empty = Hints::default();
        assert!(empty.is_empty());
        assert!(hints(&empty).is_empty());
        assert!(Hints::new("   ", "", None, None, None).is_empty(), "whitespace is not a camera");
    }

    #[test]
    fn a_hint_can_feed_the_merger_and_pick_up_the_photo_s_existing_keywords() {
        use crate::suggest::{Caps, merge};
        let meta = Hints::new("Nikon Z6", "", Some(6400), None, None);
        let existing = vec!["Nikon Z6".to_string()];
        let merged = merge(&hints(&meta), &existing, Caps::default());
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].tag, "low light");
        assert_eq!(merged[0].source, Source::ExifHint);
    }
}
