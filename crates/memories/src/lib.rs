//! Resurfacing: what happened today, what happened this week, and what was never looked at.
//!
//! Three rules shape everything here.
//!
//! - **Dates are UTC.** Capture times are stored as Unix seconds with no zone, because EXIF carries
//!   a local wall-clock time and no reliable offset. Reading them as UTC means "on this day" can be a
//!   few hours out at the edge of a day for a library shot near midnight. Saying so is better than
//!   inventing an offset nobody recorded.
//! - **Nothing is random.** The same library and the same day always produce the same cards, so a
//!   card can be explained: "24 June, three years ago" is checkable in the grid.
//! - **A card knows why it exists.** Every card carries the rule that produced it and the count it
//!   was cut down from, so a card that says three photos can be traced back to the thirty it chose
//!   between.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use sortlens_core::{Flag, PhotoId};

/// How many photos a card shows.
pub const CARD_PHOTOS: usize = 24;

/// A photo old enough to be a memory rather than news.
pub const FORGOTTEN_AGE_DAYS: i64 = 365;

/// One day in the calendar, as this crate reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    /// Calendar year.
    pub year: i64,
    /// Month, 1–12.
    pub month: u32,
    /// Day of the month, 1–31.
    pub day: u32,
}

/// Days since 1970-01-01 to a civil date. Howard Hinnant's `civil_from_days`.
///
/// Written out rather than pulled in: a date library would be a dependency for fifteen lines of
/// arithmetic that never change, and the crate has a test pinning the leap-day cases that go wrong.
pub fn civil(seconds: i64) -> Date {
    let days = seconds.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    Date {
        year: if month <= 2 { y + 1 } else { y },
        month,
        day,
    }
}

/// The calendar year a timestamp falls in, for callers that only need the year.
pub fn civil_year(seconds: i64) -> i64 {
    civil(seconds).year
}

/// Whether a year has 29 February.
pub const fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Days in a month, leap years included.
pub const fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Seconds at midnight UTC on a date.
pub fn midnight(date: Date) -> i64 {
    let y = date.year;
    let m = i64::from(date.month);
    let d = i64::from(date.day);
    // Days from the civil epoch, the inverse of `civil`.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    days * 86_400
}

/// Month name, as cards print it.
pub const fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "",
    }
}

/// `1 photo`, `2 photos`.
fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// The cheap facts a card needs about one photo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    /// Catalog id.
    pub id: PhotoId,
    /// Capture time, Unix seconds.
    pub captured: Option<i64>,
    /// Stars.
    pub rating: u8,
    /// Pick, reject or none.
    pub flag: Flag,
    /// Pixels, used to pick the sharpest of a burst; zero when unknown.
    pub pixels: u64,
}

impl Candidate {
    /// Build one from the fields a card cares about.
    pub const fn new(id: PhotoId, captured: Option<i64>, rating: u8, flag: Flag, pixels: u64) -> Self {
        Self {
            id,
            captured,
            rating,
            flag,
            pixels,
        }
    }

    /// Whether the photo was ever judged: rated or flagged. A card that resurfaces photos should
    /// leave the ones already judged alone.
    pub const fn judged(&self) -> bool {
        self.rating > 0 || !matches!(self.flag, Flag::None)
    }
}

/// Why a card exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardKind {
    /// Photos taken on this month and day, in earlier years.
    OnThisDay,
    /// Photos taken in the seven days up to today, in earlier years.
    ThisWeek,
    /// Photos never rated or flagged and at least a year old.
    Forgotten,
}

impl CardKind {
    /// Stable id for commands, settings and tests.
    pub const fn id(self) -> &'static str {
        match self {
            Self::OnThisDay => "onThisDay",
            Self::ThisWeek => "thisWeek",
            Self::Forgotten => "forgotten",
        }
    }

    /// Heading.
    pub const fn label(self) -> &'static str {
        match self {
            Self::OnThisDay => "On this day",
            Self::ThisWeek => "This week in past years",
            Self::Forgotten => "Never looked at",
        }
    }
}

/// One card of photos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// Why it exists.
    pub kind: CardKind,
    /// Heading, which may name the day or the span.
    pub title: String,
    /// The photos, best first.
    pub photos: Vec<PhotoId>,
    /// How many photos the rule matched before the card was cut down.
    pub matched: usize,
    /// The oldest year on the card, for "since 2019" style detail.
    pub first_year: Option<i64>,
}

impl Card {
    /// One line under the heading.
    pub fn detail(&self) -> String {
        let years = match self.first_year {
            Some(year) => format!(" · since {year}"),
            None => String::new(),
        };
        if self.matched > self.photos.len() {
            format!("{} of {}{years}", self.photos.len(), plural(self.matched, "photo"))
        } else {
            format!("{}{years}", plural(self.photos.len(), "photo"))
        }
    }

    /// Whether the card has anything to show.
    pub fn is_empty(&self) -> bool {
        self.photos.is_empty()
    }
}

/// Everything the For You view shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Memories {
    /// Non-empty cards, in display order.
    pub cards: Vec<Card>,
}

impl Memories {
    /// Whether there is nothing to resurface.
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Total photos across every card.
    pub fn photo_count(&self) -> usize {
        self.cards.iter().map(|card| card.photos.len()).sum()
    }

    /// One line for the header or the CLI.
    pub fn headline(&self) -> String {
        if self.is_empty() {
            return "Nothing to resurface yet".to_string();
        }
        let kinds: Vec<&str> = self.cards.iter().map(|card| card.kind.label()).collect();
        format!("{} card(s): {}", self.cards.len(), kinds.join(", "))
    }

    /// Look up one card.
    pub fn card(&self, kind: CardKind) -> Option<&Card> {
        self.cards.iter().find(|card| card.kind == kind)
    }

    /// Build every card for a given day.
    ///
    /// `today` is the day the cards are about, as Unix seconds; callers pass the current time, and
    /// tests pass a fixed one. `limit` is how many photos a card shows.
    pub fn build(candidates: &[Candidate], today: i64, limit: usize) -> Self {
        let limit = limit.max(1);
        let date = civil(today);
        let cards = vec![
            on_this_day(candidates, date, limit),
            this_week(candidates, date, limit),
            forgotten(candidates, today, limit),
        ];
        Self {
            cards: cards.into_iter().filter(|card| !card.is_empty()).collect(),
        }
    }
}

/// Whether a photo's date answers to a card for `month`/`day`.
///
/// 29 February only exists every fourth year, so in the other years the 28th stands in for it rather
/// than the card disappearing for three years out of four.
fn matches_day(taken: Date, month: u32, day: u32) -> bool {
    if taken.month != month {
        return false;
    }
    taken.day == day || (month == 2 && day == 29 && taken.day == 28)
}

/// Photos taken on the same month and day in an earlier year.
fn on_this_day(candidates: &[Candidate], date: Date, limit: usize) -> Card {
    let mut hits: Vec<&Candidate> = candidates
        .iter()
        .filter(|candidate| {
            candidate
                .captured
                .map(civil)
                .is_some_and(|taken| taken.year < date.year && matches_day(taken, date.month, date.day))
        })
        .collect();
    hits.sort_by(best_first);
    let first_year = hits
        .iter()
        .filter_map(|candidate| candidate.captured.map(|at| civil(at).year))
        .min();
    let matched = hits.len();
    hits.truncate(limit);
    Card {
        kind: CardKind::OnThisDay,
        title: format!("{} {}", month_name(date.month), date.day),
        photos: hits.iter().map(|candidate| candidate.id).collect(),
        matched,
        first_year,
    }
}

/// Photos taken in the seven days ending today, in an earlier year.
fn this_week(candidates: &[Candidate], date: Date, limit: usize) -> Card {
    let end = midnight(date);
    let start = end - 6 * 86_400;
    // Only the month and day matter, so the window lands on the same week in every earlier year and
    // a window that spans new year wraps rather than going empty.
    let (from, to) = (civil(start), civil(end));
    let mut hits: Vec<&Candidate> = candidates
        .iter()
        .filter(|candidate| {
            candidate
                .captured
                .map(civil)
                .is_some_and(|taken| taken.year < date.year && in_window(from, to, taken))
        })
        .collect();
    hits.sort_by(best_first);
    let first_year = hits
        .iter()
        .filter_map(|candidate| candidate.captured.map(|at| civil(at).year))
        .min();
    let matched = hits.len();
    hits.truncate(limit);
    Card {
        kind: CardKind::ThisWeek,
        title: CardKind::ThisWeek.label().to_string(),
        photos: hits.iter().map(|candidate| candidate.id).collect(),
        matched,
        first_year,
    }
}

/// Whether `taken`'s month and day fall inside the window from `start` to `end`, comparing only
/// month and day so the window lands on the same week in every earlier year.
fn in_window(start: Date, end: Date, taken: Date) -> bool {
    let key = |date: Date| (date.month, date.day);
    let inside = |value: (u32, u32)| {
        if (start.month, start.day) <= (end.month, end.day) {
            key(start) <= value && value <= key(end)
        } else {
            // The window crosses new year, so it wraps around it.
            value >= key(start) || value <= key(end)
        }
    };
    inside(key(taken))
}

/// Photos never rated and never flagged, at least a year old.
fn forgotten(candidates: &[Candidate], today: i64, limit: usize) -> Card {
    let oldest = today - FORGOTTEN_AGE_DAYS * 86_400;
    let mut hits: Vec<&Candidate> = candidates
        .iter()
        .filter(|candidate| !candidate.judged() && candidate.captured.is_some_and(|at| at <= oldest))
        .collect();
    // Oldest first: the longest-neglected photos are the ones worth surfacing.
    hits.sort_by_key(|candidate| (candidate.captured.unwrap_or_default(), std::cmp::Reverse(candidate.pixels)));
    let first_year = hits
        .iter()
        .filter_map(|candidate| candidate.captured.map(|at| civil(at).year))
        .min();
    let matched = hits.len();
    hits.truncate(limit);
    Card {
        kind: CardKind::Forgotten,
        title: CardKind::Forgotten.label().to_string(),
        photos: hits.iter().map(|candidate| candidate.id).collect(),
        matched,
        first_year,
    }
}

/// Order photos so the best leads: stars, then a pick over nothing, then pixels, then recency.
fn best_first(left: &&Candidate, right: &&Candidate) -> std::cmp::Ordering {
    right
        .rating
        .cmp(&left.rating)
        .then_with(|| {
            let rank = |flag: Flag| match flag {
                Flag::Pick => 2u8,
                Flag::None => 1,
                Flag::Reject => 0,
            };
            rank(right.flag).cmp(&rank(left.flag))
        })
        .then_with(|| right.pixels.cmp(&left.pixels))
        .then_with(|| right.captured.unwrap_or_default().cmp(&left.captured.unwrap_or_default()))
        .then_with(|| left.id.cmp(&right.id))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn at(year: i64, month: u32, day: u32) -> i64 {
        midnight(Date { year, month, day }) + 12 * 3_600
    }

    fn candidate(id: u64, captured: Option<i64>, rating: u8) -> Candidate {
        Candidate::new(PhotoId(id), captured, rating, Flag::None, 1_000)
    }

    #[test]
    fn dates_round_trip_and_leap_days_are_real() {
        for (seconds, year, month, day) in [
            (0, 1970, 1, 1),
            (1_704_067_200, 2024, 1, 1),
            (1_709_164_800, 2024, 2, 29),
            (1_735_689_599, 2024, 12, 31),
            (-86_400, 1969, 12, 31),
        ] {
            let date = civil(seconds);
            assert_eq!((date.year, date.month, date.day), (year, month, day));
            assert_eq!(midnight(date), seconds.div_euclid(86_400) * 86_400);
        }
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29, "400-year rule");
        assert_eq!(days_in_month(1900, 2), 28, "100-year rule");
        assert_eq!(month_name(6), "June");
        assert_eq!(month_name(13), "");
    }

    #[test]
    fn a_year_can_be_read_off_a_timestamp() {
        assert_eq!(civil_year(at(2019, 7, 4)), 2019);
        assert_eq!(civil_year(0), 1970);
        assert_eq!(civil_year(-1), 1969);
        assert_eq!(civil_year(at(2024, 2, 29)), 2024);
    }

    #[test]
    fn on_this_day_finds_the_same_date_in_earlier_years_only() {
        let today = at(2024, 6, 24);
        let candidates = vec![
            candidate(1, Some(at(2021, 6, 24)), 0),
            candidate(2, Some(at(2023, 6, 24)), 4),
            candidate(3, Some(at(2023, 6, 25)), 0),
            candidate(4, Some(at(2024, 6, 24)), 0), // today, not a memory
            candidate(5, None, 0),                  // undated
        ];
        let memories = Memories::build(&candidates, today, CARD_PHOTOS);
        let card = memories.card(CardKind::OnThisDay).expect("card");
        assert_eq!(card.title, "June 24");
        assert_eq!(card.photos, vec![PhotoId(2), PhotoId(1)], "best first");
        assert_eq!(card.first_year, Some(2021));
        assert_eq!(card.detail(), "2 photos · since 2021");
    }

    #[test]
    fn a_leap_day_card_shows_february_28_in_other_years() {
        let today = at(2025, 2, 28); // 2025 has no 29 February
        let candidates = vec![candidate(1, Some(at(2024, 2, 28)), 0)];
        let memories = Memories::build(&candidates, today, CARD_PHOTOS);
        let card = memories.card(CardKind::OnThisDay).expect("card");
        assert_eq!(card.photos, vec![PhotoId(1)]);
        assert_eq!(card.detail(), "1 photo · since 2024", "one photo is not one photos");
    }

    #[test]
    fn this_week_is_a_seven_day_window_that_survives_new_year() {
        // 3 January, looking back over 28 December – 3 January.
        let today = at(2024, 1, 3);
        let candidates = vec![
            candidate(1, Some(at(2022, 12, 30)), 0),
            candidate(2, Some(at(2023, 1, 2)), 0),
            candidate(3, Some(at(2023, 1, 10)), 0), // outside
        ];
        let memories = Memories::build(&candidates, today, CARD_PHOTOS);
        let card = memories.card(CardKind::ThisWeek).expect("card");
        assert_eq!(card.photos, vec![PhotoId(2), PhotoId(1)]);
    }

    #[test]
    fn forgotten_is_unjudged_and_at_least_a_year_old() {
        let today = at(2024, 6, 24);
        let mut rejected = candidate(4, Some(at(2020, 1, 1)), 0);
        rejected.flag = Flag::Reject;
        let candidates = vec![
            candidate(1, Some(at(2020, 1, 1)), 0), // forgotten: oldest first
            candidate(2, Some(at(2022, 5, 1)), 0),
            candidate(3, Some(at(2023, 9, 1)), 3), // rated
            candidate(5, Some(at(2024, 6, 1)), 0), // too new
            rejected,
        ];
        let memories = Memories::build(&candidates, today, CARD_PHOTOS);
        let card = memories.card(CardKind::Forgotten).expect("card");
        assert_eq!(card.photos, vec![PhotoId(1), PhotoId(2)]);
        assert_eq!(card.matched, 2);
    }

    #[test]
    fn a_card_says_how_much_it_left_out() {
        let today = at(2024, 6, 24);
        let candidates: Vec<Candidate> = (1..=10).map(|id| candidate(id, Some(at(2020, 6, 24)), 0)).collect();
        let memories = Memories::build(&candidates, today, 3);
        let card = memories.card(CardKind::OnThisDay).expect("card");
        assert_eq!(card.photos.len(), 3);
        assert_eq!(card.matched, 10);
        assert_eq!(card.detail(), "3 of 10 photos · since 2020");
        assert_eq!(memories.card(CardKind::Forgotten).expect("card").photos.len(), 3);
        // The same ten photos also answer "this week" and "never looked at", so they appear on all
        // three cards: overlap is the point, and the header count is per card, not unique photos.
        assert_eq!(memories.cards.len(), 3);
        for card in &memories.cards {
            assert_eq!(card.matched, 10, "{:?}", card.kind);
            assert_eq!(card.photos.len(), 3);
        }
        assert_eq!(memories.photo_count(), 9);
    }

    #[test]
    fn an_empty_library_says_so_without_inventing_cards() {
        let memories = Memories::build(&[], at(2024, 6, 24), CARD_PHOTOS);
        assert!(memories.is_empty());
        assert_eq!(memories.headline(), "Nothing to resurface yet");
        assert!(memories.card(CardKind::OnThisDay).is_none());
    }

    #[test]
    fn the_same_day_always_produces_the_same_cards() {
        let today = at(2024, 6, 24);
        let candidates = vec![
            candidate(1, Some(at(2021, 6, 24)), 0),
            candidate(2, Some(at(2023, 6, 21)), 0),
            candidate(3, Some(at(2019, 3, 1)), 0),
        ];
        let first = Memories::build(&candidates, today, CARD_PHOTOS);
        let second = Memories::build(&candidates, today, CARD_PHOTOS);
        assert_eq!(first, second);
        assert_eq!(first.cards.len(), 3);
        assert!(first.headline().contains("On this day"));
    }
}
