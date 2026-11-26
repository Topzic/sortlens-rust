//! A dependency-free locale fallback for the desktop targets that are neither
//! Apple nor Android.
//!
//! Upstream uses ICU4X 2.4 here. This fork replaces it with a small ASCII-level
//! implementation so the map renderer can coexist with applications pinned to
//! ICU4X 2.3 (Sortlens' `craft-i18n`). Collation is case- and diacritic-aware
//! only in the simple sense; number formatting groups thousands and honours the
//! fraction limits, with the currency code or unit name appended. Map labels
//! remain readable, and no locale data is loaded.

use std::cmp::Ordering;

use super::{FractionDigits, Sensitivity, Style};

/// A collator that compares the strings themselves.
pub(crate) struct Collator {
    case: bool,
    resolved_locale: String,
}

impl Collator {
    pub(crate) fn new(tag: &str, sensitivity: Sensitivity) -> Result<Self, String> {
        let language = tag.split(['-', '_', '@']).next().filter(|part| !part.is_empty()).unwrap_or("en");
        Ok(Self { case: sensitivity.case, resolved_locale: language.to_owned() })
    }

    pub(crate) fn compare(&self, lhs: &str, rhs: &str) -> Ordering {
        if self.case {
            lhs.cmp(rhs)
        } else {
            lhs.to_lowercase().cmp(&rhs.to_lowercase())
        }
    }

    pub(crate) fn resolved_locale(&self) -> &str {
        &self.resolved_locale
    }
}

/// Formats a number with the platform's plain decimal form.
pub(crate) fn format_number(
    number: f64,
    _locale: &str,
    style: Style<'_>,
    digits: FractionDigits,
) -> Result<String, String> {
    let (min, max) = digits.resolve(None);
    let mut out = format!("{number:.*}", usize::from(max));
    if out.contains('.') {
        while out.ends_with('0') && out.split('.').nth(1).is_some_and(|fraction| fraction.len() > usize::from(min)) {
            out.pop();
        }
        if out.ends_with('.') {
            out.pop();
        }
    }
    let (integer, fraction) = match out.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (out.as_str(), None),
    };
    let (sign, integer) = match integer.strip_prefix('-') {
        Some(integer) => ("-", integer),
        None => ("", integer),
    };
    let mut grouped = String::with_capacity(integer.len() + integer.len() / 3 + 8);
    grouped.push_str(sign);
    for (index, ch) in integer.chars().enumerate() {
        if index > 0 && (integer.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    if let Some(fraction) = fraction {
        grouped.push('.');
        grouped.push_str(fraction);
    }
    match style {
        Style::Decimal => {}
        Style::Currency(code) => {
            grouped.push(' ');
            grouped.push_str(code);
        }
        Style::Unit(unit) => {
            grouped.push(' ');
            grouped.push_str(unit.unit);
            if let Some(per) = unit.per {
                grouped.push('/');
                grouped.push_str(per);
            }
        }
    }
    Ok(grouped)
}
