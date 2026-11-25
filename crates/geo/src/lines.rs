//! The Natural Earth polylines that give the shipped map its interior detail.
//!
//! Land alone is a silhouette: a continent with no country borders or rivers reads as a cut-out, not a map. These
//! are the same public-domain Natural Earth source as the land, at 1:50m for the interior detail, reduced to
//! latitude/longitude runs and embedded in the binary. They are deliberately *not* a tile set: no roads, no labels,
//! no download.
//!
//! The format is one feature per line, `lat,lon` tokens separated by spaces. Comments start with `#`.

use std::sync::OnceLock;

use crate::LatLon;

/// What the attribution document and the map's credit call this layer.
pub const CREDIT: &str = "Borders, rivers and lakes: Natural Earth 1:50m (public domain)";

/// The embedded country-boundary source.
pub const BORDERS_SOURCE: &str = "assets/borders.txt";

/// The embedded river source.
pub const RIVERS_SOURCE: &str = "assets/rivers.txt";

/// A set of geographic polylines, parsed once.
#[derive(Debug, Clone, Default)]
pub struct Lines {
    lines: Vec<Vec<LatLon>>,
    points: usize,
    skipped: usize,
}

impl Lines {
    /// The country boundaries embedded in the binary.
    pub fn borders() -> &'static Self {
        static SHIPPED: OnceLock<Lines> = OnceLock::new();
        SHIPPED.get_or_init(|| Self::parse(BORDERS_TEXT))
    }

    /// The rivers embedded in the binary.
    pub fn rivers() -> &'static Self {
        static SHIPPED: OnceLock<Lines> = OnceLock::new();
        SHIPPED.get_or_init(|| Self::parse(RIVERS_TEXT))
    }

    /// Parse the shipped format. A line with fewer than two usable coordinates is refused and counted.
    pub fn parse(text: &str) -> Self {
        let mut lines = Vec::new();
        let mut points = 0usize;
        let mut skipped = 0usize;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut run = Vec::new();
            for token in line.split_whitespace() {
                match parse_point(token) {
                    Some(at) => run.push(at),
                    None => skipped += 1,
                }
            }
            if run.len() < 2 {
                skipped += 1;
                continue;
            }
            points += run.len();
            lines.push(run);
        }
        Self { lines, points, skipped }
    }

    /// How many polylines survived parsing.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// How many coordinates the surviving polylines hold.
    pub fn points(&self) -> usize {
        self.points
    }

    /// How many tokens or features the parser refused.
    pub fn skipped(&self) -> usize {
        self.skipped
    }

    /// The surviving polylines, in the file's order.
    pub fn lines(&self) -> impl Iterator<Item = &[LatLon]> {
        self.lines.iter().map(Vec::as_slice)
    }
}

const BORDERS_TEXT: &str = include_str!("../../../assets/borders.txt");
const RIVERS_TEXT: &str = include_str!("../../../assets/rivers.txt");

fn parse_point(token: &str) -> Option<LatLon> {
    let (lat, lon) = token.split_once(',')?;
    LatLon::new(lat.trim().parse().ok()?, lon.trim().parse().ok()?)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_line_needs_two_points_and_bad_tokens_are_counted() {
        let lines = Lines::parse("# comment\n1,2 3,4\n5,6\nnope\n7,8 9,10\n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines.points(), 4);
        assert!(lines.skipped() >= 2);
        assert_eq!(lines.lines().next().map(|line| line.len()), Some(2));
    }

    #[test]
    fn the_shipped_borders_and_rivers_are_not_empty() {
        assert!(!Lines::borders().is_empty(), "the country boundaries ship");
        assert!(!Lines::rivers().is_empty(), "the rivers ship");
        assert_eq!(Lines::borders().skipped(), 0);
        assert_eq!(Lines::rivers().skipped(), 0);
    }
}
