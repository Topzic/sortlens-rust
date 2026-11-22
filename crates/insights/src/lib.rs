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

/// The day with the most photos on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Day {
    /// Unix seconds at midnight UTC.
    pub at: i64,
    /// Photos captured that day.
    pub count: usize,
}

/// One notable fact, for the highlights strip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Highlight {
    /// What it is about.
    pub label: String,
    /// What it says.
    pub value: String,
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
    /// Photos per calendar month, January first.
    pub by_month: [usize; 12],
    /// The busiest single day, if any photo has a capture time.
    pub busiest_day: Option<Day>,
}

impl Insights {
    /// An empty view, for a library that is not open yet.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Build the view from photos, in whatever order they arrive.
    pub fn compute<'a>(photos: impl IntoIterator<Item = &'a Photo>) -> Self {
        let mut accumulator = Accumulator::new();
        for photo in photos {
            accumulator.push(photo);
        }
        accumulator.finish()
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

    /// The busiest month, 1–12, or `None` when nothing is dated.
    pub fn peak_month(&self) -> Option<u32> {
        let (index, count) = self.by_month.iter().enumerate().max_by_key(|(_, count)| **count)?;
        (*count > 0).then_some(index as u32 + 1)
    }

    /// The busiest season by capture count: winter, spring, summer or autumn (northern reading).
    pub fn peak_season(&self) -> Option<&'static str> {
        let seasons = [
            ("Winter", [11, 0, 1]),
            ("Spring", [2, 3, 4]),
            ("Summer", [5, 6, 7]),
            ("Autumn", [8, 9, 10]),
        ];
        let mut best: Option<(&'static str, usize)> = None;
        for (name, months) in seasons {
            let count: usize = months.iter().map(|month| self.by_month[*month]).sum();
            if count > 0 && best.is_none_or(|(_, peak)| count > peak) {
                best = Some((name, count));
            }
        }
        best.map(|(name, _)| name)
    }

    /// Up to four notable facts, most specific first.
    ///
    /// These are the numbers a photographer would otherwise have to eyeball off the charts: the month
    /// the library leans towards, the single busiest day, the camera that did most of the work, and
    /// how much has no date at all.
    pub fn highlights(&self) -> Vec<Highlight> {
        let mut highlights = Vec::new();
        if let Some(month) = self.peak_month() {
            let count = self.by_month[month as usize - 1];
            let season = self.peak_season().map(|season| format!(" · mostly {season}"));
            highlights.push(Highlight {
                label: "Busiest month".to_string(),
                value: format!("{} — {} photo(s){}", month_name(month), count, season.unwrap_or_default()),
            });
        }
        if let Some(day) = self.busiest_day
            && day.count > 1
        {
            highlights.push(Highlight {
                label: "Busiest day".to_string(),
                value: format!("{} — {} photo(s)", date_label(day.at), day.count),
            });
        }
        if let Some(camera) = self.cameras.first() {
            highlights.push(Highlight {
                label: "Most used camera".to_string(),
                value: format!("{} — {} photo(s)", camera.name, camera.count),
            });
        }
        if self.undated > 0 {
            highlights.push(Highlight {
                label: "No capture date".to_string(),
                value: format!(
                    "{} photo(s), {:.0}% of the library",
                    self.undated,
                    share(self.undated, self.stats.total) * 100.0
                ),
            });
        }
        highlights.truncate(4);
        highlights
    }
}

/// Fold photos in one at a time, so a long pass can report how far it has come.
///
/// The same code backs both the one-shot [`Insights::compute`] and the engine's background pass —
/// one implementation, so a progress bar can never disagree with the numbers it is progressing
/// towards.
#[derive(Debug, Default)]
pub struct Accumulator {
    insights: Insights,
    cameras: HashMap<String, usize>,
    lenses: HashMap<String, usize>,
    keywords: HashMap<String, usize>,
    years: HashMap<i32, usize>,
    days: HashMap<i64, usize>,
}

impl Accumulator {
    /// A fresh pass.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many photos have been folded in.
    pub fn examined(&self) -> usize {
        self.insights.stats.total
    }

    /// Fold one photo in.
    pub fn push(&mut self, photo: &Photo) {
        self.insights.stats.total += 1;
        observe(&mut self.insights.stats, photo);
        // `observe` also counts the total, so take the double count back out.
        self.insights.stats.total -= 1;
        let insights = &mut self.insights;
        insights.by_rating[usize::from(photo.rating.min(5))] += 1;
        insights.by_kind[kind_index(photo.kind)] += 1;
        if let Some(label) = photo.label {
            insights.by_label[label_index(label)] += 1;
        }
        if photo.camera.is_empty() {
            insights.unknown_camera += 1;
        } else {
            *self.cameras.entry(photo.camera.clone()).or_default() += 1;
        }
        if !photo.lens.is_empty() {
            *self.lenses.entry(photo.lens.clone()).or_default() += 1;
        }
        for tag in &photo.tags {
            *self.keywords.entry(tag.clone()).or_default() += 1;
        }
        match photo.captured.and_then(civil) {
            Some((year, month, _day_of_month, day)) => {
                *self.years.entry(year).or_default() += 1;
                self.insights.by_month[(month - 1) as usize] += 1;
                *self.days.entry(day).or_default() += 1;
            }
            None => insights.undated += 1,
        }
    }

    /// Finish the pass and hand back the view.
    pub fn finish(mut self) -> Insights {
        self.insights.cameras = fold(self.cameras);
        self.insights.lenses = fold(self.lenses);
        self.insights.keywords = fold(self.keywords);
        let mut timeline: Vec<Year> = self.years.into_iter().map(|(year, count)| Year { year, count }).collect();
        timeline.sort_by_key(|entry| entry.year);
        self.insights.years = timeline;
        self.insights.busiest_day = self
            .days
            .into_iter()
            .max_by_key(|(day, count)| (*count, std::cmp::Reverse(*day)))
            .map(|(at, count)| Day { at, count });
        self.insights
    }
}

/// The month name, 1–12.
pub fn month_name(month: u32) -> &'static str {
    const NAMES: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    NAMES.get(month as usize - 1).copied().unwrap_or("Unknown")
}

/// `2024-08-14`, from unix seconds.
pub fn date_label(seconds: i64) -> String {
    match civil(seconds) {
        Some((year, month, day, _)) => format!("{year:04}-{month:02}-{day:02}"),
        None => "unknown".to_string(),
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
fn fold<K: AsRef<str>>(counts: HashMap<K, usize>) -> Vec<Row> {
    let mut rows: Vec<Row> = counts
        .into_iter()
        .map(|(name, count)| Row {
            name: name.as_ref().to_string(),
            count,
        })
        .collect();
    rows.sort_by(|left, right| right.count.cmp(&left.count).then_with(|| left.name.cmp(&right.name)));
    rows.truncate(BREAKDOWN);
    rows
}

/// The UTC year, month (1–12), day of month and the day's first second, from a capture time.
///
/// Capture times are already normalised to the epoch by the metadata reader, so a plain division is
/// enough; the walk keeps leap years correct without a date dependency.
fn civil(epoch: i64) -> Option<(i32, u32, u32, i64)> {
    const DAY: i64 = 86_400;
    if epoch <= 0 {
        return None;
    }
    let day_number = epoch / DAY;
    let mut days = day_number;
    let mut year = 1970i32;
    loop {
        let length = if is_leap(year) { 366 } else { 365 };
        if days < length {
            break;
        }
        days -= length;
        year += 1;
        if year > 9999 {
            return None;
        }
    }
    const MONTHS: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u32;
    for (index, length) in MONTHS.iter().enumerate() {
        let length = if index == 1 && is_leap(year) { 29 } else { *length };
        if days < length {
            break;
        }
        days -= length;
        month += 1;
    }
    Some((year, month, days as u32 + 1, day_number * DAY))
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
        assert_eq!(civil(951_782_400).map(|(year, ..)| year), Some(2000));
        assert_eq!(civil(1_704_067_200).map(|(year, ..)| year), Some(2024));
        assert_eq!(civil(1_704_067_200).map(|(_, month, day, _)| (month, day)), Some((1, 1)));
        assert_eq!(civil(1_707_523_200).map(|(_, month, day, _)| (month, day)), Some((2, 10)));
        assert_eq!(civil(0), None, "epoch zero is unknown, not 1970");
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

    #[test]
    fn months_and_seasons_come_out_of_the_capture_times() {
        // 2024-02-10, 2024-02-20, 2024-07-04 and 2024-12-25, in unix seconds.
        let dates = [1_707_523_200i64, 1_708_387_200, 1_720_051_200, 1_735_084_800];
        let photos: Vec<Photo> = dates
            .iter()
            .enumerate()
            .map(|(index, at)| {
                let mut photo = photo(&format!("{index}.jpg"), "X-T5", "", 0);
                photo.captured = Some(*at);
                photo
            })
            .collect();
        let insights = Insights::compute(&photos);
        assert_eq!(insights.by_month.iter().sum::<usize>(), 4);
        assert_eq!(insights.by_month[1], 2, "two February frames");
        assert_eq!(insights.by_month[6], 1);
        assert_eq!(insights.by_month[11], 1);
        assert_eq!(insights.peak_month(), Some(2));
        assert_eq!(insights.peak_season(), Some("Winter"));
        assert_eq!(month_name(2), "February");
        assert_eq!(month_name(99), "Unknown");
    }

    #[test]
    fn highlights_say_what_the_charts_cannot() {
        let mut first = photo("a.jpg", "X-T5", "", 5);
        first.captured = Some(1_707_523_200); // 2024-02-10
        let mut second = photo("b.jpg", "X-T5", "", 0);
        second.captured = Some(1_707_609_600); // 2024-02-11, one day later
        let mut third = photo("c.jpg", "X-T5", "", 0);
        third.captured = Some(1_707_523_200); // same day as the first
        let undated = photo("d.jpg", "", "", 0);

        let insights = Insights::compute([&first, &second, &third, &undated]);
        let highlights = insights.highlights();
        let labels: Vec<&str> = highlights.iter().map(|row| row.label.as_str()).collect();
        assert!(labels.contains(&"Busiest month"));
        assert!(labels.contains(&"Busiest day"));
        assert!(labels.contains(&"Most used camera"));
        assert!(labels.contains(&"No capture date"), "one photo has no date");
        assert!(highlights.len() <= 4);
        let day = highlights.iter().find(|row| row.label == "Busiest day").expect("day row");
        assert!(day.value.contains("2024-02-10"), "{}", day.value);
        assert!(day.value.contains('2'), "two photos that day: {}", day.value);
        assert_eq!(date_label(0), "unknown");
    }

    #[test]
    fn the_accumulator_agrees_with_the_one_shot_pass() {
        let photos: Vec<Photo> = (0..40)
            .map(|index| {
                let mut photo = photo(&format!("{index}.jpg"), "GR III", "28mm", (index % 5) as u8);
                photo.captured = Some(1_700_000_000 + i64::from(index) * 86_400);
                photo
            })
            .collect();
        let one_shot = Insights::compute(&photos);

        let mut accumulator = Accumulator::new();
        for photo in &photos {
            accumulator.push(photo);
        }
        assert_eq!(accumulator.examined(), 40);
        assert_eq!(accumulator.finish(), one_shot);
    }
}
