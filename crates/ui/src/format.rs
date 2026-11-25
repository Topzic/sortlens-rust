//! Numbers and dates the shell paints, in the interface language's own convention (SL-L08).
//!
//! The formatting region here is the interface language, not the operating system: Sortlens has
//! one written-language setting, and a reader who chooses Arabic expects Arabic-Indic digits and
//! the day first, whatever the computer's regional settings are. Every table below is explicit —
//! a language the picker offers without an entry fails a test rather than quietly painting
//! English shapes.
//!
//! English keeps exactly the shapes the shell has always painted: Latin digits, a narrow no-break
//! space every three, '.' as the decimal mark and '2026-11-21 14:32'. That is deliberate, not a
//! fallback: the committed scenes are English, and a shipped slice must not move a baseline it
//! did not mean to.

use crate::i18n;

/// How one interface language writes numbers and dates.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Style {
    /// The language code, as `i18n::LANGUAGES` spells it.
    pub(crate) lang: &'static str,
    /// The first of the ten digits this language writes.
    pub(crate) zero: char,
    /// Every three digits from the right; `None` groups nothing.
    pub(crate) group: Option<char>,
    /// The decimal mark.
    pub(crate) decimal: char,
    /// The order the date's fields are written in.
    pub(crate) order: Order,
    /// What stands between the date's fields.
    pub(crate) date_sep: char,
}

/// The order a date's fields are written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Order {
    YearMonthDay,
    DayMonthYear,
}

/// The styles offered, one per language the picker offers. `en` is the shape the app has always
/// painted; `ja` is Japan's comma grouping and slash date; `ar` is Egyptian Arabic's
/// Arabic-Indic digits with `٬` grouping, `٫` decimal and the day first.
const STYLES: &[Style] = &[
    Style {
        lang: "en",
        zero: '0',
        group: Some('\u{202f}'),
        decimal: '.',
        order: Order::YearMonthDay,
        date_sep: '-',
    },
    Style {
        lang: "ar",
        zero: '\u{660}',
        group: Some('\u{66c}'),
        decimal: '\u{66b}',
        order: Order::DayMonthYear,
        date_sep: '/',
    },
    Style {
        lang: "ja",
        zero: '0',
        group: Some(','),
        decimal: '.',
        order: Order::YearMonthDay,
        date_sep: '/',
    },
];

/// The style for a language code, English when the code is not one the picker offers (an
/// unknown saved code still paints something rather than failing a frame).
pub(crate) fn style(code: &str) -> &'static Style {
    STYLES.iter().find(|style| style.lang == code).unwrap_or(&STYLES[0])
}

/// The style of the interface language in force.
pub(crate) fn current() -> &'static Style {
    style(i18n::I18N.code(i18n::current()))
}

/// A whole number: `12\u{202f}408`, `12,408`, `١٢٬٤٠٨`.
pub(crate) fn count(n: u64) -> String {
    count_in(current(), n)
}

/// [`count`] in an explicit style, so a test does not have to switch the interface language.
pub(crate) fn count_in(style: &Style, n: u64) -> String {
    let digits = digits_in(style, n, 1);
    group_in(style, &digits)
}

/// A file size: `1.4 MB`, `١٫٤ MB`.
pub(crate) fn size(bytes: u64) -> String {
    size_in(current(), bytes)
}

/// [`size`] in an explicit style.
pub(crate) fn size_in(style: &Style, bytes: u64) -> String {
    let (value, unit, places) = sortlens_core::byte_measure(bytes);
    format!("{} {unit}", decimal_in(style, value, places))
}

/// A civil date and time: `2026-11-21 14:32`, `2026/11/21 14:32`, `٢١/١١/٢٠٢٦ ١٤:٣٢`.
pub(crate) fn date_time(seconds: i64) -> String {
    date_time_in(current(), seconds)
}

/// [`date_time`] in an explicit style.
pub(crate) fn date_time_in(style: &Style, seconds: i64) -> String {
    let at = sortlens_meta::civil_time(seconds);
    let year = digits_in(style, at.year.unsigned_abs(), 4);
    let month = digits_in(style, at.month.unsigned_abs(), 2);
    let day = digits_in(style, at.day.unsigned_abs(), 2);
    let sep = style.date_sep;
    let date = match style.order {
        Order::YearMonthDay => format!("{year}{sep}{month}{sep}{day}"),
        Order::DayMonthYear => format!("{day}{sep}{month}{sep}{year}"),
    };
    format!(
        "{date} {}:{}",
        digits_in(style, at.hour.unsigned_abs(), 2),
        digits_in(style, at.minute.unsigned_abs(), 2)
    )
}

/// `value` in this style's digits, left-padded with its zero to `width` digits.
fn digits_in(style: &Style, value: u64, width: usize) -> String {
    let text = value.to_string();
    let mut out = String::with_capacity(width);
    for _ in text.len()..width {
        out.push(style.zero);
    }
    for ch in text.chars() {
        out.push(style.digit(ch));
    }
    out
}

/// `value` at `places` digits after the decimal mark, in this style.
fn decimal_in(style: &Style, value: f64, places: usize) -> String {
    let scale = 10u64.pow(places as u32);
    let scaled = (value * scale as f64).round() as u64;
    let mut out = group_in(style, &digits_in(style, scaled / scale, 1));
    if places > 0 {
        out.push(style.decimal);
        out.push_str(&digits_in(style, scaled % scale, places));
    }
    out
}

/// `digits` with this style's grouping mark every three from the right.
fn group_in(style: &Style, digits: &str) -> String {
    let Some(group) = style.group else {
        return digits.to_owned();
    };
    let chars: Vec<char> = digits.chars().collect();
    let mut out = String::with_capacity(digits.len() + chars.len() / 3);
    for (index, ch) in chars.iter().enumerate() {
        if index > 0 && (chars.len() - index).is_multiple_of(3) {
            out.push(group);
        }
        out.push(*ch);
    }
    out
}

impl Style {
    /// One of the Latin ten, written in this style's digits.
    fn digit(&self, ch: char) -> char {
        ch.to_digit(10)
            .and_then(|digit| char::from_u32(self.zero as u32 + digit))
            .unwrap_or(ch)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// Every language the picker offers has a written style, and every style belongs to a
    /// language the picker offers: adding a language fails here until its numbers and dates
    /// are decided.
    #[test]
    fn every_offered_language_has_one_style() {
        for code in i18n::LANGUAGES {
            assert!(
                STYLES.iter().any(|style| style.lang == *code),
                "{code} is offered but has no number or date style"
            );
        }
        for style in STYLES {
            assert!(i18n::LANGUAGES.contains(&style.lang), "{} is not offered", style.lang);
        }
    }

    /// English is byte-identical to the neutral formatter every count already went through, so
    /// routing the shell through this module moves no English pixel.
    #[test]
    fn english_keeps_the_shape_the_shell_already_painted() {
        let english = style("en");
        for n in [0, 1, 7, 999, 1_000, 12_408, 1_234_567] {
            assert_eq!(count_in(english, n), sortlens_core::format_count(n), "count {n}");
        }
        assert_eq!(date_time_in(english, 1_791_000_000), sortlens_meta::format_epoch(1_791_000_000));
    }

    /// The digits, the grouping and the decimal mark each language writes.
    #[test]
    fn numbers_follow_the_language() {
        assert_eq!(count_in(style("en"), 12_408), "12\u{202f}408");
        assert_eq!(count_in(style("ja"), 12_408), "12,408");
        assert_eq!(count_in(style("ar"), 12_408), "١٢٬٤٠٨");
        assert_eq!(count_in(style("ar"), 7), "٧");
        assert_eq!(size_in(style("en"), 1_400_000), "1.4 MB");
        assert_eq!(size_in(style("ja"), 1_400_000), "1.4 MB");
        assert_eq!(size_in(style("ar"), 1_400_000), "١٫٤ MB");
        assert_eq!(size_in(style("ar"), 812_000_000_000), "٨١٢ GB");
    }

    /// The date's field order follows the language, and its digits come with it.
    #[test]
    fn dates_follow_the_language_order() {
        let seconds = 1_791_000_000; // 2026-10-03 04:00 UTC
        assert_eq!(date_time_in(style("en"), seconds), "2026-10-03 04:00");
        assert_eq!(date_time_in(style("ja"), seconds), "2026/10/03 04:00");
        assert_eq!(date_time_in(style("ar"), seconds), "٠٣/١٠/٢٠٢٦ ٠٤:٠٠");
        // An epoch second far in the past still writes its year at four digits.
        assert_eq!(date_time_in(style("en"), -1_000_000_000), "1938-04-24 22:13");
    }

    /// A language code nobody offers paints English shapes rather than nothing.
    #[test]
    fn an_unknown_code_falls_back_to_english() {
        assert_eq!(style("xx").lang, "en");
        assert_eq!(count_in(style("xx"), 1_000), "1\u{202f}000");
    }
}
