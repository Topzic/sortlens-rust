//! Sharpness, exposure and composition measures that drive best-pick and soft-focus.
//!
//! This crate answers one question so far — **how much fine detail does this photograph have?** — because a
//! blurry frame is the failure people most want found for them, and the answer has to be a number the rest of the
//! app can store, compare across a library and re-derive when the algorithm changes.
//!
//! # How the score works, and why it is shaped this way
//!
//! The score is the **variance of the 4-neighbour Laplacian of the luma channel, divided by the squared mean
//! luma**. Each part of that sentence is a decision:
//!
//! - **Laplacian, not edge energy.** A sharp boundary and a high-contrast bokeh background both produce strong
//!   first-derivative response, so Sobel-style energy ranks a blurred subject over a sharp one more often than it
//!   should. The Laplacian is a second derivative: it is near zero across a smooth ramp however steep that ramp
//!   is, and large wherever the signal changes direction — which is what detail is.
//! - **Variance, not the mean of the response.** A mean would let one very sharp edge in a mostly flat frame carry
//!   the whole score; variance says the whole frame has to be busy, which is what "in focus" looks like.
//! - **Divided by the squared mean luma.** The Laplacian's amplitude scales with the brightness of the frame, so
//!   its variance scales with brightness *squared*. Dividing by the squared mean makes the score independent of
//!   exposure: a correctly exposed frame and a copy exposed one stop down score the same, so an underexposed sharp
//!   photograph is not reported as soft.
//! - **Measured at a fixed analysis edge.** [`ANALYSIS_EDGE`]. Sharpness is a property of the photograph, not of
//!   the copy you happen to have: scoring a 45 MP original and a 1024-pixel preview must give the same answer, or
//!   the previews would rank differently from the originals. Every image is box-filtered down to the same grid
//!   before anything is measured.
//!
//! The result is **unitless**. It is not a percentage and not a 0–100 score, because those invite comparisons that
//! do not hold: the same frame shot at two focal lengths, or in two lighting conditions, legitimately scores
//! differently. What it is good for is *ordering* — this frame is softer than that one, and this run of frames is
//! softer than the run beside it. Thresholds belong to the scan that uses them, not to this crate.
//!
//! # Versioning
//!
//! [`Score`] carries a [`SCORE_VERSION`]. A stored score is only comparable with a freshly computed one when their
//! versions match, and [`needs_rescore`] is the one place that decides — so changing the algorithm means bumping the
//! version, and the next scan re-scores everything instead of silently mixing two scales in one sort. A library
//! where half the rows came from one formula is worse than a library with no scores at all, because nothing about
//! the numbers would look wrong.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The version of the scoring algorithm.
///
/// Bump this whenever a change alters the number a given photograph produces — a different kernel, a different
/// normalisation, a different analysis size. It is not bumped for refactors that leave the arithmetic alone, and it
/// is checked by [`needs_rescore`] rather than trusted.
pub const SCORE_VERSION: u16 = 1;

/// The long edge, in pixels, that every image is reduced to before it is measured.
///
/// 256 is the smallest edge at which the Laplacian still sees individual edges in a 1-pixel-wide detail rather than
/// the box filter's own averaging, and it makes the score of one photograph independent of *which copy* was measured
/// for copies whose long edges are related by a whole-number ratio (a 2048-pixel original and its 1024-pixel preview
/// reduce by 8 and by 4 and measure alike).
///
/// It does not make the score independent of the reduction factor itself: a copy reduced by a fractional factor is
/// averaged differently by the box filter, so it can score modestly higher or lower. Correcting for that would mean
/// measuring at several edges and combining them, which buys precision this measure does not need — it ranks
/// frames within one library, and it is honest about being a ranking rather than a calibration.
pub const ANALYSIS_EDGE: u32 = 256;

/// The mean luma below which a frame cannot be judged.
///
/// A near-black frame has no detail to measure, so its Laplacian variance is dominated by sensor noise and JPEG
/// blocking — both of which read as *sharpness* to this measure. Reporting a score there would put every night shot
/// at the top of the "sharpest" list, so the honest answer is that this frame cannot be judged and the scan must say
/// so. "Cannot tell" is not "in focus" and it is not "blurry".
pub const MIN_MEAN_LUMA: f32 = 6.0;

/// A photo's sharpness, as measured, with the version of the algorithm that produced it.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Score {
    /// The unitless measure: Laplacian variance over squared mean luma. Higher is more detailed.
    pub value: f32,
    /// Which version of the algorithm produced [`Score::value`].
    pub version: u16,
}

impl Score {
    /// Whether this score came from the current algorithm.
    pub fn is_current(&self) -> bool {
        self.version == SCORE_VERSION
    }
}

/// Whether a stored score has to be recomputed, given the algorithm this build uses.
///
/// The three cases are deliberately distinct: nothing stored means it has never been measured, a stored score from
/// another version cannot be compared with today's, and a finite score from this version is usable as it stands. A
/// non-finite value counts as needing a re-score — a score of `NaN` would sort unpredictably and is not a score.
pub fn needs_rescore(stored: Option<&Score>) -> bool {
    match stored {
        None => true,
        Some(score) => !score.is_current() || !score.value.is_finite(),
    }
}

/// Measure an RGBA image of `width × height` pixels.
///
/// The crate holds no decoder and does no IO: the caller decodes (the engine already has a thumbnail service) and
/// hands the pixels over. `None` means the frame cannot be judged — a near-black frame (see [`MIN_MEAN_LUMA`]), an
/// image too small to have an interior, or a buffer that does not match its dimensions. The caller must report that
/// as unscored rather than as sharp or as blurred.
pub fn sharpness(rgba: &[u8], width: u32, height: u32) -> Option<Score> {
    let (luma, width, height) = luma_grid(rgba, width, height)?;
    let value = score_luma(&luma, width, height)?;
    Some(Score {
        value,
        version: SCORE_VERSION,
    })
}

/// Measure an already-grey image of `width × height` samples, each 0–255.
///
/// Exposed so a caller that already has a luma plane (a RAW preview, a future video frame) does not decode twice.
pub fn sharpness_of_luma(luma: &[u8], width: u32, height: u32) -> Option<Score> {
    if width < 3 || height < 3 || luma.len() < (width as usize) * (height as usize) {
        return None;
    }
    let grid: Vec<f32> = luma.iter().map(|&sample| f32::from(sample)).collect();
    let value = score_luma(&grid, width, height)?;
    Some(Score {
        value,
        version: SCORE_VERSION,
    })
}

/// Reduce an RGBA image to a box-filtered greyscale grid no larger than [`ANALYSIS_EDGE`] on its long edge.
///
/// The box filter matters: point-sampling a 45 MP frame down to 256 pixels would alias fine detail into noise and
/// score the aliasing instead of the detail. Every source pixel contributes exactly once, so the result does not
/// depend on where the sample grid happens to land.
fn luma_grid(rgba: &[u8], width: u32, height: u32) -> Option<(Vec<f32>, u32, u32)> {
    if width == 0 || height == 0 {
        return None;
    }
    let needed = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
    if rgba.len() < needed {
        return None;
    }
    let longest = width.max(height);
    // A small image is measured as it is; upscaling would invent detail.
    let edge = ANALYSIS_EDGE.min(longest);
    let scale = f64::from(edge) / f64::from(longest);
    let out_w = ((f64::from(width) * scale).round() as u32).max(1);
    let out_h = ((f64::from(height) * scale).round() as u32).max(1);
    let mut grid = Vec::with_capacity((out_w as usize) * (out_h as usize));
    for row in 0..out_h {
        let top = row * height / out_h;
        let bottom = (((row + 1) * height) / out_h).max(top + 1).min(height);
        for column in 0..out_w {
            let left = column * width / out_w;
            let right = (((column + 1) * width) / out_w).max(left + 1).min(width);
            let mut total: u64 = 0;
            let mut count: u64 = 0;
            for y in top..bottom {
                for x in left..right {
                    let index = ((y as usize) * (width as usize) + (x as usize)) * 4;
                    let pixel = &rgba[index..index + 4];
                    // Rec. 601 luma, in integers: 299 R + 587 G + 114 B over 1000.
                    let luma = (299 * u64::from(pixel[0]) + 587 * u64::from(pixel[1]) + 114 * u64::from(pixel[2])) / 1000;
                    total += luma.min(255);
                    count += 1;
                }
            }
            // A cell that covers no pixels cannot happen — the ranges are clamped to at least one pixel — but a
            // zero here would be a divide by zero rather than a wrong score, so it is refused by returning empty.
            if count > 0 {
                grid.push(total as f32 / count as f32);
            }
        }
    }
    Some((grid, out_w, out_h))
}

/// The measure itself: variance of the Laplacian, over squared mean luma.
fn score_luma(luma: &[f32], width: u32, height: u32) -> Option<f32> {
    if width < 3 || height < 3 {
        return None;
    }
    let (w, h) = (width as usize, height as usize);
    if luma.len() < w * h {
        return None;
    }
    let mut total = 0.0_f64;
    let mut count = 0.0_f64;
    for row in 0..h {
        for column in 0..w {
            if let Some(&sample) = luma.get(row * w + column) {
                total += f64::from(sample);
                count += 1.0;
            }
        }
    }
    if count == 0.0 {
        return None;
    }
    let mean = total / count;
    if mean < f64::from(MIN_MEAN_LUMA) {
        return None;
    }

    let mut sum = 0.0_f64;
    let mut sum_squares = 0.0_f64;
    let mut interior = 0.0_f64;
    for row in 1..h - 1 {
        for column in 1..w - 1 {
            let centre = f64::from(luma[row * w + column]);
            let up = f64::from(luma[(row - 1) * w + column]);
            let down = f64::from(luma[(row + 1) * w + column]);
            let left = f64::from(luma[row * w + column - 1]);
            let right = f64::from(luma[row * w + column + 1]);
            let laplacian = 4.0 * centre - up - down - left - right;
            sum += laplacian;
            sum_squares += laplacian * laplacian;
            interior += 1.0;
        }
    }
    if interior == 0.0 {
        return None;
    }
    let mean_response = sum / interior;
    let variance = (sum_squares / interior - mean_response * mean_response).max(0.0);
    let score = variance / (mean * mean);
    // A finite number or nothing: an infinite or NaN score would sort unpredictably, and a caller cannot tell it
    // apart from a real one. The cast saturates rather than wrapping, and the finite check rejects what it makes.
    #[allow(clippy::cast_possible_truncation)]
    let score = score as f32;
    Some(score).filter(|value| value.is_finite())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// RGBA pixels from a luma plane, so a test can build exactly the image it means to measure.
    fn rgba_of(luma: &[u8]) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(luma.len() * 4);
        for &sample in luma {
            rgba.extend_from_slice(&[sample, sample, sample, 255]);
        }
        rgba
    }

    /// Vertical black-and-white bars: maximum detail, and detail the box filter preserves at any size.
    fn bars(width: u32, height: u32, period: u32) -> Vec<u8> {
        let mut luma = Vec::with_capacity((width as usize) * (height as usize));
        for _ in 0..height {
            for x in 0..width {
                luma.push(if (x / period).is_multiple_of(2) { 20 } else { 235 });
            }
        }
        luma
    }

    /// A box blur over the luma plane, which is what defocus does to detail: neighbouring samples are averaged.
    fn blur(luma: &[u8], width: u32, height: u32, radius: u32) -> Vec<u8> {
        let (w, h, r) = (width as i64, height as i64, radius as i64);
        let mut out = Vec::with_capacity(luma.len());
        for y in 0..h {
            for x in 0..w {
                let mut total = 0u32;
                let mut count = 0u32;
                for dy in -r..=r {
                    for dx in -r..=r {
                        let (sx, sy) = (x + dx, y + dy);
                        if sx < 0 || sy < 0 || sx >= w || sy >= h {
                            continue;
                        }
                        total += u32::from(luma[(sy as usize) * (width as usize) + (sx as usize)]);
                        count += 1;
                    }
                }
                out.push((total / count.max(1)) as u8);
            }
        }
        out
    }

    /// The score has to separate a sharp frame from a blurred one, which is the whole reason it exists.
    #[test]
    fn a_blurred_frame_scores_far_below_the_sharp_one_it_came_from() {
        let (w, h) = (256, 192);
        let sharp = bars(w, h, 4);
        let soft = blur(&sharp, w, h, 3);

        let sharp_score = sharpness_of_luma(&sharp, w, h).expect("sharp").value;
        let soft_score = sharpness_of_luma(&soft, w, h).expect("soft").value;
        assert!(sharp_score > 0.0, "bars have detail: {sharp_score}");
        assert!(
            soft_score * 4.0 < sharp_score,
            "a blurred copy must score at least four times lower: {soft_score} vs {sharp_score}"
        );

        // And the heavier the blur, the lower the score: the measure orders, which is what it is for.
        let softer = blur(&sharp, w, h, 6);
        let softer_score = sharpness_of_luma(&softer, w, h).expect("softer").value;
        assert!(
            softer_score < soft_score,
            "more blur must score lower: {softer_score} vs {soft_score}"
        );
    }

    /// A featureless frame has no detail to report — not a low score, a zero one. It is measured, not refused.
    #[test]
    fn a_flat_frame_scores_zero_rather_than_refusing() {
        let luma = vec![128u8; 256 * 192];
        let score = sharpness_of_luma(&luma, 256, 192).expect("a flat grey frame is measurable");
        assert_eq!(score.value, 0.0, "no detail is exactly zero detail");
    }

    /// The exposure normalisation is the reason an underexposed sharp frame is not called soft.
    #[test]
    fn halving_the_exposure_does_not_change_the_score() {
        let (w, h) = (256, 192);
        let bright = bars(w, h, 4);
        let dim: Vec<u8> = bright.iter().map(|&sample| sample / 2).collect();
        let bright_score = sharpness_of_luma(&bright, w, h).expect("bright").value;
        let dim_score = sharpness_of_luma(&dim, w, h).expect("dim").value;
        assert!(dim_score > 0.0);
        let ratio = bright_score / dim_score;
        assert!(
            (0.9..1.1).contains(&ratio),
            "halving every sample must leave the score alone, got a ratio of {ratio}"
        );
    }

    /// Two copies of one photograph at different sizes must score the same, or the previews rank differently
    /// from the originals — which would make the whole library's ordering depend on which copy was scanned.
    ///
    /// Both copies reduce to 256 by a whole-number factor (4 and 2), which is the case the fixed analysis edge is
    /// designed for. A fractional reduction is measured differently by the box filter and is not asserted here.
    #[test]
    fn the_same_photograph_scores_the_same_at_two_sizes() {
        let large_score = sharpness(&rgba_of(&bars(1024, 768, 16)), 1024, 768).expect("large").value;
        let small_score = sharpness(&rgba_of(&bars(512, 384, 8)), 512, 384).expect("small").value;
        assert!(large_score > 0.0 && small_score > 0.0);
        let ratio = (large_score / small_score).max(small_score / large_score);
        assert!(
            ratio < 1.05,
            "an exact half-size copy must score alike, but gave {large_score} and {small_score}"
        );
    }

    /// The fixed edge is what makes that true: without it, the same content at two sizes measures differently
    /// because a smaller copy has less absolute detail. This pins the mechanism, not just the result.
    #[test]
    fn measuring_without_a_fixed_edge_would_not_be_comparable() {
        let large = bars(1024, 768, 16);
        let small = bars(512, 384, 8);
        // The score of the *native* buffers, at their own sizes: not what `sharpness` does, but it shows the
        // difference the reduction removes. Halving the dimensions of the same pattern changes the number.
        let native_large = sharpness_of_luma(&large, 1024, 768).expect("large").value;
        let native_small = sharpness_of_luma(&small, 512, 384).expect("small").value;
        assert!(
            (native_large / native_small - 1.0).abs() > 0.5,
            "at their own sizes the two copies differ sharply ({native_large} vs {native_small}), which is what \
             the fixed edge exists to prevent"
        );
    }

    /// A deterministic stand-in for a property test: the invariants every pattern must obey, over patterns nobody
    /// wrote by hand.
    ///
    /// The workspace has no property-testing dependency, and adding one for two invariants would cost a package and
    /// its attribution; a fixed-seed generator proves the same thing for the same two rules and is reproducible on
    /// failure, which is the part a property test is actually for.
    #[test]
    fn every_pattern_scores_finite_and_non_negative() {
        let mut seed: u64 = 0x5eed_1234_abcd;
        let mut next = |bound: u64| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % bound.max(1)
        };
        for case in 0..48 {
            let width = 3 + next(60) as u32;
            let height = 3 + next(60) as u32;
            let luma: Vec<u8> = (0..(width as usize * height as usize)).map(|_| (16 + next(224)) as u8).collect();
            match sharpness_of_luma(&luma, width, height) {
                Some(score) => {
                    assert!(score.value.is_finite(), "case {case}: a score is finite");
                    assert!(score.value >= 0.0, "case {case}: variance cannot be negative, got {}", score.value);
                }
                None => assert!(
                    width < 3 || height < 3,
                    "case {case}: a mid-tone pattern is never refused ({width}x{height})"
                ),
            }
        }
    }

    /// The exposure invariance is a property, not a coincidence of one pattern: it has to hold for whatever the
    /// frame contains, because the frames it will be asked about are photographs.
    #[test]
    fn exposure_invariance_holds_for_arbitrary_patterns() {
        let mut seed: u64 = 0xfeed_9876_5432;
        let mut next = |bound: u64| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            (seed >> 33) % bound.max(1)
        };
        for case in 0..24 {
            let (width, height) = (64u32, 48u32);
            // Bright enough that halving it still leaves the frame above the judgement floor.
            let luma: Vec<u8> = (0..(width as usize * height as usize)).map(|_| (64 + next(160)) as u8).collect();
            let half: Vec<u8> = luma.iter().map(|&sample| sample / 2).collect();
            let (Some(full), Some(dim)) = (sharpness_of_luma(&luma, width, height), sharpness_of_luma(&half, width, height)) else {
                panic!("case {case}: both exposures are mid-tone and must be judged");
            };
            if full.value < 1e-6 {
                continue; // A flat frame has no ratio to compare.
            }
            let ratio = full.value / dim.value;
            assert!(
                (0.85..1.15).contains(&ratio),
                "case {case}: halving every sample must leave the score alone, got {ratio} ({} vs {})",
                full.value,
                dim.value
            );
        }
    }

    /// A frame too dark to judge is refused as unmeasurable, not scored as sharp because of its noise.
    #[test]
    fn a_near_black_frame_cannot_be_judged() {
        // Noise at the bottom of the range: without the darkness guard this reads as detail.
        let mut luma = Vec::with_capacity(256 * 192);
        for index in 0..256 * 192 {
            luma.push(if index % 2 == 0 { 1 } else { 2 });
        }
        assert_eq!(sharpness_of_luma(&luma, 256, 192), None, "too dark to judge");
        // Just above the floor it is judged, so the guard is a floor and not a blanket refusal.
        let dim: Vec<u8> = luma.iter().map(|&sample| sample * 8).collect();
        assert!(sharpness_of_luma(&dim, 256, 192).is_some(), "8 is above the floor");
    }

    /// An image with no interior, or a buffer that lies about its size, is refused rather than measured wrongly.
    #[test]
    fn an_image_with_no_interior_is_refused() {
        assert_eq!(sharpness_of_luma(&[100; 4], 2, 2), None);
        assert_eq!(sharpness_of_luma(&[100; 9], 3, 3).map(|s| s.value), Some(0.0));
        assert_eq!(sharpness(&[0; 10], 16, 16), None, "a short buffer is not an image");
        assert_eq!(sharpness(&[], 0, 0), None);
    }

    /// Versioning exists so a library scored by an older formula is re-scored rather than mixed with today's.
    #[test]
    fn a_score_from_another_version_has_to_be_recomputed() {
        let current = Score {
            value: 1.5,
            version: SCORE_VERSION,
        };
        assert!(!needs_rescore(Some(&current)), "today's score is usable");

        let older = Score {
            value: 1.5,
            version: SCORE_VERSION.wrapping_sub(1),
        };
        assert!(needs_rescore(Some(&older)), "an older formula is not comparable");

        let broken = Score {
            value: f32::NAN,
            version: SCORE_VERSION,
        };
        assert!(needs_rescore(Some(&broken)), "a NaN is not a score");
        assert!(needs_rescore(None), "never measured");
    }

    /// The score is stored, so it has to round-trip as plain JSON and refuse a field it does not know.
    #[test]
    fn a_score_round_trips_through_json() {
        let score = Score {
            value: 2.25,
            version: SCORE_VERSION,
        };
        let text = serde_json::to_string(&score).expect("serialise");
        assert_eq!(serde_json::from_str::<Score>(&text).expect("parse"), score);
        assert!(serde_json::from_str::<Score>(r#"{"value":1.0,"version":1,"sharpness":"high"}"#).is_err());
    }

    /// End to end on a real decode: a fixture JPEG is measured, and the result is a versioned, finite score.
    #[test]
    fn a_real_jpeg_decodes_and_scores() {
        let fixture = sortlens_testkit::Fixture::new("quality-score");
        let path = fixture.jpeg("frame.jpg", [30, 120, 200], 320, 240);
        let thumbnail = sortlens_thumbs::decode_file(&path, sortlens_thumbs::LARGE).expect("decode");
        let score = sharpness(&thumbnail.rgba, thumbnail.width, thumbnail.height).expect("a mid-tone frame can be judged");
        assert!(score.value.is_finite(), "a score is a finite number");
        assert_eq!(score.version, SCORE_VERSION);
        assert!(score.is_current());
        assert!(
            score.value < 1.0,
            "a flat colour JPEG has almost no detail, so it must not score high: {}",
            score.value
        );
    }
}
