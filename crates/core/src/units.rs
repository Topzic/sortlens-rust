//! Formatting helpers. Tabular and locale-independent on purpose: the grid shows these in
//! columns, so they must not jitter.

/// Format a byte count the way a photo app does: `999 B`, `1.4 MB`, `812 GB`.
///
/// Decimal units (1 kB = 1000 B) because camera cards and operating systems report decimals.
pub fn format_bytes(bytes: u64) -> String {
    let (value, unit, places) = byte_measure(bytes);
    match places {
        1 => format!("{value:.1} {unit}"),
        _ => format!("{value:.0} {unit}"),
    }
}

/// The number a byte count is written as, its unit, and the digits after the decimal mark.
///
/// [`format_bytes`] writes that number with a `.`; an interface that writes numbers its own
/// way (SL-L08) asks for the parts instead of parsing the sentence back apart.
#[must_use]
pub fn byte_measure(bytes: u64) -> (f64, &'static str, usize) {
    const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
    if bytes < 1000 {
        return (bytes as f64, "B", 0);
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1000.0;
        unit += 1;
    }
    (value, UNITS[unit], usize::from(value < 10.0))
}

/// Format a count with thin thousands separators: `12 408`.
///
/// A narrow no-break space keeps the digits from wrapping, and the result is stable width for
/// column alignment at typical library sizes.
pub fn format_count(count: u64) -> String {
    let digits = count.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn bytes_switch_units_at_a_thousand() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(1000), "1.0 kB");
        assert_eq!(format_bytes(1_400_000), "1.4 MB");
        assert_eq!(format_bytes(812_000_000_000), "812 GB");
    }

    #[test]
    fn counts_group_in_threes() {
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1\u{202f}000");
        assert_eq!(format_count(12_408), "12\u{202f}408");
    }
}
