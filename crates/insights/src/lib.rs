//! Library statistics.
//!
//! Everything here is derived from the catalog on demand: no second store, no index to keep in
//! sync, and every number can be reconciled against the grid by applying the same filter. The
//! counts are cheap enough to recompute whenever the view opens.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use std::collections::HashMap;

use sortlens_catalog::{Photo, Stats};
use sortlens_core::{ColorLabel, MediaKind};

/// How many rows a breakdown keeps before folding the rest into nothing.
pub const BREAKDOWN: usize = 8;

/// One row of a breakdown, most common first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// What is being counted, as shown to people.
    pub name: String,
    /// How many photos have it.
    pub count: usize,
}

/// A year on the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Year {
    /// Calendar year.
    pub year: i32,
    /// Photos captured then.
    pub count: usize,
}

/// Everything the insights view shows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Insights {
    /// Totals and byte count, shared with the sidebar.
    pub stats: Stats,
    /// Photos per star rating, index 0 standing for unrated.
    pub by_rating: [usize; 6],
    /// Photos per colour label, in shortcut order.
    pub by_label: [usize; 5],
    /// Photos per media kind: images, raw, video.
    pub by_kind: [usize; 3],
    /// Cameras, most used first.
    pub cameras: Vec<Row>,
    /// Lenses, most used first.
    pub lenses: Vec<Row>,
    /// Keywords, most used first.
    pub keywords: Vec<Row>,
    /// Capture years, oldest first.
    pub years: Vec<Year>,
    /// Photos with no capture time at all.
    pub undated: usize,
    /// Photos with no camera recorded.
    pub unknown_camera: usize,
}

impl Insights {
    /// An empty view, for a library that is not open yet.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build the view from photos, in whatever order they arrive.
    pub fn compute<'a>(photos: impl IntoIterator<Item = &'a Photo>) -> Self {
        let mut insights = Self::default();
        let mut cameras: HashMap<&str, usize> = HashMap::new();
        let mut lenses: HashMap<&str, usize> = HashMap::new();
        let mut keywords: HashMap<&str, usize> = HashMap::new();
        let mut years: HashMap<i32, usize> = HashMap::new();

        for photo in photos {
            observe(&mut insights.stats, photo);
            insights.by_rating[usize::from(photo.rating.min(5))] += 1;
            insights.by_kind[kind_index(photo.kind)] += 1;
            if let Some(label) = photo.label {
                insights.by_label[label_index(label)] += 1;
            }
            if photo.camera.is_empty() {
                insights.unknown_camera += 1;
            } else {
                *cameras.entry(photo.camera.as_str()).or_default() += 1;
            }
            if !photo.lens.is_empty() {
                *lenses.entry(photo.lens.as_str()).or_default() += 1;
            }
            for tag in &photo.tags {
                *keywords.entry(tag.as_str()).or_default() += 1;
            }
            match photo.captured.and_then(year_of) {
                Some(year) => *years.entry(year).or_default() += 1,
                None => insights.undated += 1,
            }
        }

        insights.cameras = fold(cameras);
        insights.lenses = fold(lenses);
        insights.keywords = fold(keywords);
        let mut timeline: Vec<Year> = years.into_iter().map(|(year, count)| Year { year, count }).collect();
        timeline.sort_by_key(|entry| entry.year);
        insights.years = timeline;
        insights
    }

    /// Photos with at least one star.
    pub const fn rated(&self) -> usize {
        self.stats.rated
    }

    /// Photos nobody has judged yet.
    pub const fn unjudged(&self) -> usize {
        self.stats.total - self.stats.rated - self.stats.picks - self.stats.rejects
    }

    /// Share of the library that is rated, 0.0 to 1.0.
    pub fn rated_share(&self) -> f32 {
        share(self.stats.rated, self.stats.total)
    }

    /// Share of the library carrying a keyword, 0.0 to 1.0.
    pub fn tagged_share(&self) -> f32 {
        share(self.stats.tagged, self.stats.total)
    }

    /// The busiest year, for scaling the timeline.
    pub fn peak_year(&self) -> usize {
        self.years.iter().map(|entry| entry.count).max().unwrap_or(0)
    }

    /// The largest count in a breakdown, for scaling bars.
    pub fn peak(rows: &[Row]) -> usize {
        rows.iter().map(|row| row.count).max().unwrap_or(0)
    }
}

/// Fold one photo into the totals.
fn observe(stats: &mut Stats, photo: &Photo) {
    stats.total += 1;
    stats.bytes = stats.bytes.saturating_add(photo.size);
    stats.rated += usize::from(photo.rating > 0);
    stats.picks += usize::from(photo.flag == sortlens_core::Flag::Pick);
    stats.rejects += usize::from(photo.flag == sortlens_core::Flag::Reject);
    stats.labelled += usize::from(photo.label.is_some());
    stats.tagged += usize::from(!photo.tags.is_empty());
    stats.missing += usize::from(photo.missing);
}

fn share(part: usize, whole: usize) -> f32 {
    if whole == 0 {
        return 0.0;
    }
    part as f32 / whole as f32
}

const fn kind_index(kind: MediaKind) -> usize {
    match kind {
        MediaKind::Image => 0,
        MediaKind::Raw => 1,
        MediaKind::Video => 2,
    }
}

fn label_index(label: ColorLabel) -> usize {
    ColorLabel::ALL.iter().position(|known| *known == label).unwrap_or(0)
}

/// Sort by count, then name, and keep the first [`BREAKDOWN`].
fn fold(counts: HashMap<&str, usize>) -> Vec<Row> {
    let mut rows: Vec<Row> = counts
        .into_iter()
        .map(|(name, count)| Row {
            name: name.to_string(),
            count,
        })
        .collect();
    rows.sort_by(|left, right| right.count.cmp(&left.count).then_with(|| left.name.cmp(&right.name)));
    rows.truncate(BREAKDOWN);
    rows
}

/// The UTC year a timestamp falls in.
///
/// Capture times are already normalised to the epoch by the metadata reader, so a plain division
/// is enough; the civil-date helper in `sortlens-meta` is for display, not bucketing.
fn year_of(epoch: i64) -> Option<i32> {
    const DAY: i64 = 86_400;
    const YEAR: i64 = 365;
    if epoch <= 0 {
        return None;
    }
    // Days since 1970, then walk the calendar so leap years stay correct.
    let mut days = epoch / DAY;
    let mut year = 1970i32;
    loop {
        let length = if is_leap(year) { 366 } else { YEAR };
        if days < length {
            return Some(year);
        }
        days -= length;
        year += 1;
        if year > 9999 {
            return None;
        }
    }
}

const fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use sortlens_core::{Flag, PhotoId};

    fn photo(name: &str, camera: &str, lens: &str, rating: u8) -> Photo {
        let mut photo = Photo::from_scan(PhotoId(1), "/photos", name, 100, Some(1_700_000_000), MediaKind::Image, 7);
        photo.camera = camera.to_string();
        photo.lens = lens.to_string();
        photo.rating = rating;
        photo
    }

    #[test]
    fn totals_reconcile_with_the_photo_count() {
        let photos = vec![
            photo("a.jpg", "X-T5", "23mm", 3),
            photo("b.jpg", "X-T5", "56mm", 0),
            photo("c.jpg", "GR III", "", 0),
        ];
        let insights = Insights::compute(&photos);
        assert_eq!(insights.stats.total, 3);
        assert_eq!(insights.stats.rated, 1);
        assert_eq!(insights.stats.bytes, 300);
        assert_eq!(insights.by_rating.iter().sum::<usize>(), 3);
        assert_eq!(insights.by_rating[3], 1);
        assert_eq!(insights.by_rating[0], 2);
        assert_eq!(insights.unknown_camera, 0);
        assert_eq!(
            insights.cameras[0],
            Row {
                name: "X-T5".into(),
                count: 2
            }
        );
        assert_eq!(insights.lenses[0].name, "23mm");
        assert_eq!(insights.unjudged(), 2);
    }

    #[test]
    fn labels_kinds_and_flags_are_counted_per_photo() {
        let mut picked = photo("a.jpg", "X-T5", "23mm", 5);
        picked.flag = Flag::Pick;
        picked.label = Some(ColorLabel::Green);
        picked.tags = vec!["beach".into(), "sunset".into()];
        let mut rejected = photo("b.raw", "X-T5", "23mm", 0);
        rejected.kind = MediaKind::Raw;
        rejected.flag = Flag::Reject;
        rejected.label = Some(ColorLabel::Green);
        rejected.tags = vec!["beach".into()];

        let insights = Insights::compute([&picked, &rejected]);
        assert_eq!(insights.stats.picks, 1);
        assert_eq!(insights.stats.rejects, 1);
        assert_eq!(insights.stats.labelled, 2);
        assert_eq!(insights.stats.tagged, 2);
        assert_eq!(insights.by_label[label_index(ColorLabel::Green)], 2);
        assert_eq!(insights.by_kind[0], 1);
        assert_eq!(insights.by_kind[1], 1);
        assert_eq!(
            insights.keywords[0],
            Row {
                name: "beach".into(),
                count: 2
            }
        );
        assert_eq!(insights.keywords[1].name, "sunset");
        assert_eq!(Insights::peak(&insights.keywords), 2);
    }

    #[test]
    fn years_are_bucketed_through_leap_years() {
        let mut early = photo("a.jpg", "", "", 0);
        early.captured = Some(0); // 1970-01-01, epoch zero is still valid
        let mut leap = photo("b.jpg", "", "", 0);
        leap.captured = Some(951_782_400); // 2000-02-29
        let mut late = photo("c.jpg", "", "", 0);
        late.captured = Some(1_704_067_200); // 2024-01-01
        let mut undated = photo("d.jpg", "", "", 0);
        undated.captured = None;

        let insights = Insights::compute([&early, &leap, &late, &undated]);
        assert_eq!(insights.unknown_camera, 4);
        assert_eq!(insights.undated, 2, "epoch zero is treated as unknown, not 1970");
        assert_eq!(insights.years.len(), 2);
        assert_eq!(insights.years[0].year, 2000, "the leap day lands in 2000");
        assert_eq!(insights.years[1].year, 2024);
        assert_eq!(year_of(951_782_400), Some(2000));
        assert_eq!(year_of(1_704_067_200), Some(2024));
        assert_eq!(insights.peak_year(), 1);
    }

    #[test]
    fn an_empty_library_has_no_shares() {
        let insights = Insights::compute(std::iter::empty());
        assert_eq!(insights.stats.total, 0);
        assert_eq!(insights.rated_share(), 0.0);
        assert_eq!(insights.tagged_share(), 0.0);
        assert_eq!(insights.peak_year(), 0);
        assert_eq!(insights.cameras.len(), 0);
        assert_eq!(insights.unjudged(), 0);
    }

    #[test]
    fn breakdowns_fold_past_the_cap() {
        let photos: Vec<Photo> = (0..BREAKDOWN + 4)
            .map(|index| photo(&format!("{index}.jpg"), &format!("camera{index}"), "", 0))
            .collect();
        let insights = Insights::compute(&photos);
        assert_eq!(insights.cameras.len(), BREAKDOWN);
        assert_eq!(insights.cameras[0].count, 1);
    }
}
