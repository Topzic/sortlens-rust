//! The sharpness review: what a relative threshold would select, and whether it means anything.
//!
//! This module holds the arithmetic and no egui at all, because the interesting part of a review surface is not how
//! it looks — it is *which photographs a cut would pick*, and a question that important should be answerable in a
//! test rather than only by eye.
//!
//! # Why the threshold is relative
//!
//! A sharpness score is unitless: it ranks frames within one library, in one lighting condition, at one focal
//! length. There is no absolute number that means "blurry" across libraries, because a soft-looking 45 MP frame at
//! f/11 can score higher than a perfectly sharp phone photo. So the review asks a question the numbers can actually
//! answer — *which are the softest in this library* — rather than inventing a calibration nothing here has measured.
//!
//! # Why the count can exceed the percentage
//!
//! The cut is a **value**: everything at or below the score sitting at the requested percentile. When many photos
//! share that score, the selection is larger than the percentage suggests, and [`Review::ties`] says so. A rank cut
//! ("take exactly the lowest N") would quietly pick an arbitrary subset of the tie — the same photograph selected or
//! not depending on its id — which is worse than a selection that is honestly bigger than asked for.
//!
//! # Why "all the same" selects nothing
//!
//! If every score in the library is identical, "the softest 10%" has no meaning: the cut equals every score, so a
//! value cut would select the entire library. Selecting everything is an action with consequences, not a review, so
//! [`Review::separable`] is false and the caller must say that a relative threshold cannot separate this library
//! instead of offering a button that would select all of it.

use sortlens_core::PhotoId;

/// How many columns the histogram has.
pub const BUCKETS: usize = 24;

/// The smallest share the review offers, in percent.
pub const MIN_PERCENTILE: f32 = 1.0;

/// The largest share the review offers, in percent.
///
/// Half the library is the point at which "review the softest" stops being a review and becomes "select whatever is
/// below average", which is not a useful question.
pub const MAX_PERCENTILE: f32 = 50.0;

/// The share the review starts at.
pub const DEFAULT_PERCENTILE: f32 = 10.0;

/// What a relative cut would select.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Review {
    /// Photos that have a score at all.
    pub scored: usize,
    /// Histogram counts, low score first; always [`BUCKETS`] long.
    pub buckets: Vec<usize>,
    /// The lowest score, which is the left edge of the first column.
    pub low: f32,
    /// The highest score, which is the right edge of the last column.
    pub high: f32,
    /// The score at the requested percentile: everything at or below it is selected.
    pub cut: f32,
    /// The photos at or below the cut, softest first, ties broken by id so the order is stable.
    pub below: Vec<PhotoId>,
    /// How many photos sit exactly at the cut's score, which is why the count can exceed the percentage.
    pub ties: usize,
    /// How many histogram bands the cut covers, counting the band the cut sits inside.
    ///
    /// The painting uses this rather than recomputing the band edges from the score: two places deriving the same
    /// boundary is how a histogram ends up highlighting a column the button will not select.
    pub selected_bands: usize,
    /// Whether the scores can be separated at all. False when every score is identical.
    pub separable: bool,
}

impl Review {
    /// How many photos a press would select.
    pub fn count(&self) -> usize {
        self.below.len()
    }

    /// The share of the scored library the cut selects, 0–1.
    pub fn share(&self) -> f32 {
        if self.scored == 0 {
            0.0
        } else {
            self.count() as f32 / self.scored as f32
        }
    }

    /// Whether the band at `index` is one the cut covers.
    pub fn band_selected(&self, index: usize) -> bool {
        self.separable && index < self.selected_bands
    }

    /// Whether a selection should be offered at all.
    pub fn can_select(&self) -> bool {
        self.separable && !self.below.is_empty()
    }

    /// The cut in words, for the label under the histogram.
    pub fn cut_label(&self) -> String {
        if self.scored == 0 {
            return "Nothing is scored yet".to_string();
        }
        if !self.separable {
            return "Every photo scored the same, so a relative cut cannot separate them".to_string();
        }
        match self.count() {
            1 if self.ties > 1 => format!("1 photo at the cut, and {} others share its score", self.ties - 1),
            1 => "1 photo at or below the cut".to_string(),
            count => format!("{count} photos at or below the cut ({:.0}%)", self.share() * 100.0),
        }
    }
}

/// Which photos a relative cut would select, given every stored score.
///
/// `scores` may arrive in any order; the result is deterministic because ties are broken by id.
pub fn review(scores: &[(PhotoId, f32)], percentile: f32) -> Review {
    let percentile = if percentile.is_finite() {
        percentile.clamp(MIN_PERCENTILE, MAX_PERCENTILE)
    } else {
        DEFAULT_PERCENTILE
    };
    // Only finite scores take part: a NaN would make the sort and the histogram meaningless, and the engine never
    // stores one — but a review that trusted its input would be one bug away from nonsense.
    let mut ordered: Vec<(PhotoId, f32)> = scores.iter().copied().filter(|(_, score)| score.is_finite()).collect();
    if ordered.is_empty() {
        return Review {
            buckets: vec![0; BUCKETS],
            ..Review::default()
        };
    }
    ordered.sort_by(|left, right| left.1.total_cmp(&right.1).then(left.0.cmp(&right.0)));

    let low = ordered.first().map(|(_, score)| *score).unwrap_or(0.0);
    let high = ordered.last().map(|(_, score)| *score).unwrap_or(0.0);
    let separable = high > low;

    let mut buckets = vec![0usize; BUCKETS];
    if separable {
        let span = high - low;
        for (_, score) in &ordered {
            let position = (f64::from(*score - low) / f64::from(span) * (BUCKETS as f64 - 1.0)).round();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let index = (position.max(0.0) as usize).min(BUCKETS - 1);
            buckets[index] += 1;
        }
    } else {
        // Every score is identical, so the histogram is one full column. It is drawn honestly rather than hidden:
        // "they are all here" is the information.
        buckets[0] = ordered.len();
    }

    let scored = ordered.len();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let index = ((f64::from(percentile) / 100.0) * (scored as f64 - 1.0)).round() as usize;
    let index = index.min(scored - 1);
    let cut = ordered[index].1;

    // The band the cut sits in, computed exactly as the bucket loop did.
    let cut_band = if separable {
        let span = high - low;
        let position = (f64::from(cut - low) / f64::from(span) * (BUCKETS as f64 - 1.0)).round();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let index = (position.max(0.0) as usize).min(BUCKETS - 1);
        index + 1
    } else {
        BUCKETS
    };

    if !separable {
        return Review {
            scored,
            buckets,
            low,
            high,
            cut,
            below: Vec::new(),
            ties: ordered.iter().filter(|(_, score)| *score == cut).count(),
            selected_bands: 0,
            separable,
        };
    }

    let below: Vec<PhotoId> = ordered.iter().filter(|(_, score)| *score <= cut).map(|(id, _)| *id).collect();
    let ties = ordered.iter().filter(|(_, score)| *score == cut).count();
    Review {
        scored,
        buckets,
        low,
        high,
        cut,
        below,
        ties,
        selected_bands: cut_band,
        separable,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn ids(count: u64) -> Vec<PhotoId> {
        (1..=count).map(PhotoId).collect()
    }

    /// Scores 1..=10 attached to ids 1..=10.
    fn ascending() -> Vec<(PhotoId, f32)> {
        ids(10)
            .into_iter()
            .enumerate()
            .map(|(index, id)| (id, (index + 1) as f32))
            .collect()
    }

    #[test]
    fn a_tenth_of_a_ten_photo_library_is_the_two_softest() {
        let review = review(&ascending(), 10.0);
        assert_eq!(review.scored, 10);
        assert_eq!(review.low, 1.0);
        assert_eq!(review.high, 10.0);
        // round(0.1 * 9) = 1, so the cut sits on the second score and two photos are at or below it.
        assert_eq!(review.cut, 2.0);
        assert_eq!(review.below, vec![PhotoId(1), PhotoId(2)]);
        assert_eq!(review.ties, 1);
        assert!(review.can_select());
        assert_eq!(review.share(), 0.2);
    }

    /// The histogram describes the same numbers the cut acts on.
    #[test]
    fn the_histogram_counts_every_score_exactly_once() {
        let review = review(&ascending(), 25.0);
        assert_eq!(review.buckets.len(), BUCKETS);
        assert_eq!(review.buckets.iter().sum::<usize>(), 10, "every score is in one column");
        // The lowest score is in the first column and the highest in the last: the edges are inclusive.
        assert!(review.buckets[0] >= 1);
        assert!(review.buckets[BUCKETS - 1] >= 1);
    }

    /// The painted bands and the button come from one number, so a highlighted column is always one the button
    /// would select.
    #[test]
    fn the_selected_bands_are_the_bands_below_the_cut() {
        let tenth = review(&ascending(), DEFAULT_PERCENTILE);
        assert!(tenth.selected_bands >= 1, "the cut's own band is covered");
        assert!(tenth.selected_bands <= BUCKETS);
        // Every covered band is at or below the cut, and the one after it is not.
        assert!(tenth.band_selected(tenth.selected_bands - 1));
        assert!(!tenth.band_selected(tenth.selected_bands));
        // A cut higher up covers more bands: the number follows the cut rather than being fixed.
        let half = review(&ascending(), 50.0);
        assert!(half.selected_bands > tenth.selected_bands);
        // And nothing selected means nothing painted.
        let flat: Vec<(PhotoId, f32)> = ids(4).into_iter().map(|id| (id, 3.0)).collect();
        let flat_review = review(&flat, 10.0);
        assert_eq!(flat_review.selected_bands, 0);
        assert!(!flat_review.band_selected(0));
    }

    /// A tie at the cut makes the selection bigger than the percentage asked for, and the review says how much.
    #[test]
    fn ties_at_the_cut_are_reported_rather_than_hidden() {
        let ties = vec![
            (PhotoId(1), 1.0),
            (PhotoId(2), 1.0),
            (PhotoId(3), 1.0),
            (PhotoId(4), 5.0),
            (PhotoId(5), 5.0),
            (PhotoId(6), 5.0),
            (PhotoId(7), 9.0),
            (PhotoId(8), 9.0),
            (PhotoId(9), 9.0),
            (PhotoId(10), 9.0),
        ];
        let review = review(&ties, 10.0);
        assert_eq!(review.cut, 1.0);
        assert_eq!(review.count(), 3, "ten percent asked for one, but three share that score");
        assert_eq!(review.ties, 3, "and the review says why");
        // The lowest three scores are the same, so the order is by id rather than by chance.
        assert_eq!(review.below, vec![PhotoId(1), PhotoId(2), PhotoId(3)]);
    }

    /// A library where every score is identical cannot be separated by a relative cut, and the review refuses.
    #[test]
    fn identical_scores_select_nothing_and_say_why() {
        let flat: Vec<(PhotoId, f32)> = ids(6).into_iter().map(|id| (id, 4.0)).collect();
        let review = review(&flat, 10.0);
        assert_eq!(review.scored, 6);
        assert!(!review.separable);
        assert!(review.below.is_empty(), "selecting the whole library is not a review");
        assert!(!review.can_select());
        assert!(review.cut_label().contains("cannot separate"));
        assert_eq!(review.buckets.iter().sum::<usize>(), 6, "the histogram still tells the truth");
        assert_eq!(review.buckets[0], 6, "they are all in one column");
    }

    /// An empty library is not an error and not a selection.
    #[test]
    fn nothing_scored_selects_nothing() {
        let review = review(&[], 10.0);
        assert_eq!(review.scored, 0);
        assert!(!review.can_select());
        assert_eq!(review.cut_label(), "Nothing is scored yet");
        assert_eq!(review.buckets.iter().sum::<usize>(), 0);
    }

    /// The percentage is clamped rather than trusted: a slider is a UI widget, not a guarantee.
    #[test]
    fn a_percentile_outside_the_range_is_clamped() {
        let low = review(&ascending(), -5.0);
        assert_eq!(low.cut, 1.0, "clamped to the minimum, so only the softest is at the cut");
        // A slider that offered 900% would be a bug; clamped to the maximum it means "the lower half", which on ten
        // photos is six — round(0.5 * 9) = 5, and that score is included.
        let high = review(&ascending(), 900.0);
        assert_eq!(high.count(), 6);
    }

    /// A score that is not a number would poison the sort, so it takes no part.
    #[test]
    fn a_non_finite_score_is_ignored() {
        let mut scores = ascending();
        scores.push((PhotoId(99), f32::NAN));
        scores.push((PhotoId(98), f32::INFINITY));
        let review = review(&scores, 10.0);
        assert_eq!(review.scored, 10, "only the finite scores count");
        assert_eq!(review.high, 10.0);
        assert!(!review.below.contains(&PhotoId(99)));
    }
}
