//! The coastline that ships with the application, and the two framing questions it answers.
//!
//! A field of dots on a blank rectangle says nothing about where anything is, so the application carries a
//! **starter map**: Natural Earth 1:110m land polygons filled from [\`crate::land\`] and the matching coastline
//! stroked from this module. The data is public domain, which is why it is the one world map this project can
//! ship without asking anybody's permission. Both files are embedded with include_str!, so a zero-config build
//! has geography and nothing is fetched at run time.
//!
//! This module keeps the shoreline itself: the line a cartographer draws on top of the fill.
//!
//! # Decisions that shape this module
//!
//! - **Schematic, and it says so.** 1:110m is a world map for a wall, not a navigation chart: at street zoom
//!   the outline is a staircase of 0.001° steps and whole harbours are missing. The credit line printed beside
//!   it in the view is the whole credit, and the view offers the person's own tiles/ folder for a real basemap.
//! - **The antimeridian splits a line; it never joins it.** A polyline that walks from 179°E to 179°W is two
//!   separate shorelines that the source data happens to keep in one file, and drawing the segment between them
//!   would paint a horizontal stripe across the entire world. The split happens at parse time so no later code
//!   has to remember.
//! - **Projection happens once, at zoom 0.** The view scales these points by 2^zoom rather than running five
//!   thousand tangents and logarithms per frame. The coastline stores what the file says; the view projects.
//!
//! [\`bounds_centre\`] and [\`fit_zoom\`] live here because they answer "Places opens over the middle of an ocean
//! with both clusters off screen". Both are projection arithmetic, so both are testable without a window.

use std::sync::OnceLock;

use crate::{LatLon, Point, project, unproject};

/// What is printed wherever the shipped coastline is drawn.
pub const CREDIT: &str = "Coastline: Natural Earth 1:110m (public domain)";

/// The embedded source file, for the attribution document and for anybody reading the binary's provenance.
pub const SOURCE: &str = "assets/coastline.txt";

/// How much of the viewport [\`fit_zoom\`] keeps clear around the photographs, in pixels.
pub const FIT_PADDING: f64 = 28.0;

/// The deepest zoom [\`fit_zoom\`] will choose on its own.
///
/// The outline is drawn at 1:110m, so a fitted view deeper than this shows a staircase rather than a coast. A
/// person can still zoom past it with the + button: this bound is about what the view picks unasked, not about
/// what it permits.
pub const MAX_FIT_ZOOM: u8 = 10;

/// One shoreline as the file states it.
#[derive(Debug, Clone)]
struct Outline {
    coordinates: Vec<LatLon>,
}

/// The shipped coastline, parsed once.
#[derive(Debug, Clone)]
pub struct Coastline {
    outlines: Vec<Outline>,
    points: usize,
    skipped: usize,
}

impl Default for Coastline {
    fn default() -> Self {
        Self::parse("")
    }
}

impl Coastline {
    /// The coastline embedded in the binary, parsed on first use.
    pub fn shipped() -> &'static Self {
        static SHIPPED: OnceLock<Coastline> = OnceLock::new();
        SHIPPED.get_or_init(|| Self::parse(SHIPPED_TEXT))
    }

    /// Parse the shipped format: one polyline per line, lat,lon pairs separated by spaces, # for comments.
    ///
    /// A malformed token is skipped and counted; a line that is left with fewer than two points is not a
    /// shoreline and is counted too. Nothing is silently accepted: a coastline that quietly lost a continent
    /// would be a map that lies, and the count is what makes the loss visible.
    pub fn parse(text: &str) -> Self {
        let mut outlines: Vec<Outline> = Vec::new();
        let mut skipped = 0usize;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut run = Outline { coordinates: Vec::new() };
            for token in line.split_whitespace() {
                let Some(at) = parse_point(token) else {
                    skipped += 1;
                    continue;
                };
                // Two points either side of the antimeridian are two shores, not one: see the module docs.
                if let Some(previous) = run.coordinates.last()
                    && (at.lon - previous.lon).abs() > 180.0
                {
                    skipped += finish(&mut outlines, &mut run);
                }
                run.coordinates.push(at);
            }
            skipped += finish(&mut outlines, &mut run);
        }
        let points = outlines.iter().map(|outline| outline.coordinates.len()).sum();
        Self { outlines, points, skipped }
    }

    /// How many shorelines the coastline holds.
    pub fn len(&self) -> usize {
        self.outlines.len()
    }

    /// Whether there is no coastline at all.
    pub fn is_empty(&self) -> bool {
        self.outlines.is_empty()
    }

    /// How many coordinates those shorelines hold, after any antimeridian splits.
    pub fn points(&self) -> usize {
        self.points
    }

    /// How many tokens or lines the parser refused.
    pub fn skipped(&self) -> usize {
        self.skipped
    }

    /// Every shoreline as the file states it, in file order.
    pub fn outlines(&self) -> impl Iterator<Item = &[LatLon]> {
        self.outlines.iter().map(|outline| outline.coordinates.as_slice())
    }
}

/// The embedded coastline out of the binary.
const SHIPPED_TEXT: &str = include_str!("../../../assets/coastline.txt");

/// One lat,lon token, or None if it is not a coordinate.
fn parse_point(token: &str) -> Option<LatLon> {
    let (lat, lon) = token.split_once(',')?;
    LatLon::new(lat.trim().parse().ok()?, lon.trim().parse().ok()?)
}

/// Close a run, returning how many points were dropped because a lone point is not a shoreline.
fn finish(outlines: &mut Vec<Outline>, run: &mut Outline) -> usize {
    if run.coordinates.len() < 2 {
        let dropped = usize::from(!run.coordinates.is_empty());
        run.coordinates.clear();
        return dropped;
    }
    outlines.push(Outline {
        coordinates: std::mem::take(&mut run.coordinates),
    });
    0
}

/// The centre of the box the photographs occupy, in projected space.
///
/// Not the mean: the mean of Brighton (50.8°N) and Lima (12°S) is 19°N, which is 1,400 km north of the box's
/// middle, and a view centred there pushes the northern dot towards the edge. The box's middle is the point
/// that leaves the same margin at every side, which is what "show me where these are" means. None when no
/// photograph can be projected at all.
pub fn bounds_centre(points: &[LatLon]) -> Option<LatLon> {
    let (min_x, min_y, max_x, max_y) = span(points)?;
    unproject(
        Point {
            x: (min_x + max_x) / 2.0,
            y: (min_y + max_y) / 2.0,
        },
        0,
    )
}

/// The deepest zoom at which every photograph fits inside the viewport, with [\`FIT_PADDING\`] clear on every side.
///
/// highest is the deepest zoom the caller is willing to use — the top of a tile folder's range, or
/// [\`MAX_FIT_ZOOM\`] when the shipped coastline is the basemap — and the answer never exceeds it, nor
/// [\`MAX_FIT_ZOOM\`] itself. A photograph stack at one point has no spread to fit, so the deepest allowed zoom is
/// the answer; an empty slice is the same case, because a caller with nothing to fit wants the map it would have
/// opened with.
pub fn fit_zoom(points: &[LatLon], width: f64, height: f64, highest: u8) -> u8 {
    let highest = highest.min(MAX_FIT_ZOOM);
    let width = width - 2.0 * FIT_PADDING;
    let height = height - 2.0 * FIT_PADDING;
    if width < 1.0 || height < 1.0 {
        return 0;
    }
    let Some((min_x, min_y, max_x, max_y)) = span(points) else {
        return highest;
    };
    let span_x = max_x - min_x;
    let span_y = max_y - min_y;
    // A single photograph, or a burst from one spot, has no spread to fit: the ceiling is the honest answer.
    if span_x <= 0.0 && span_y <= 0.0 {
        return highest;
    }
    // Zoom 0 is 256 pixels of world, so a span of s pixels at zoom z is s·2^z: the deepest zoom that still fits
    // is floor(log2(available / s)), with a one-pixel floor so a flat spread is not an infinite zoom.
    let fit = (width / span_x.max(1.0)).min(height / span_y.max(1.0));
    let zoom = fit.log2().floor();
    if !zoom.is_finite() || zoom >= f64::from(highest) {
        return highest;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let zoom = zoom as u8;
    zoom
}

/// The zoom-0 box around everything that can be projected: (min_x, min_y, max_x, max_y).
fn span(points: &[LatLon]) -> Option<(f64, f64, f64, f64)> {
    let mut span: Option<(f64, f64, f64, f64)> = None;
    for at in points {
        let Some(point) = project(*at, 0) else {
            continue;
        };
        span = Some(match span {
            None => (point.x, point.y, point.x, point.y),
            Some((min_x, min_y, max_x, max_y)) => (min_x.min(point.x), min_y.min(point.y), max_x.max(point.x), max_y.max(point.y)),
        });
    }
    span
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn at(lat: f64, lon: f64) -> LatLon {
        LatLon::new(lat, lon).expect("a real coordinate")
    }

    /// The asset in the binary is a whole world, not a fragment, and every one of its points parses.
    #[test]
    fn the_shipped_coastline_is_a_whole_world_and_parses_without_a_refusal() {
        let coastline = Coastline::shipped();
        assert!(coastline.len() > 100, "{} shorelines is a fragment", coastline.len());
        assert!(coastline.points() > 4_000, "{} points is a fragment", coastline.points());
        assert_eq!(coastline.skipped(), 0, "the shipped file must parse cleanly");
        assert!(!coastline.is_empty());

        let mut north = -90.0_f64;
        let mut south = 90.0_f64;
        let mut west = 180.0_f64;
        let mut east = -180.0_f64;
        for outline in coastline.outlines() {
            assert!(outline.len() >= 2, "a shoreline of one point is not a shoreline");
            for point in outline {
                north = north.max(point.lat);
                south = south.min(point.lat);
                west = west.min(point.lon);
                east = east.max(point.lon);
            }
        }
        assert!(north > 70.0 && south < -70.0, "the outline stops at {south}..{north}");
        assert!(west < -170.0 && east > 170.0, "the outline stops at {west}..{east}");
    }

    /// A line that steps across the antimeridian is split into two shores instead of drawn across the world.
    #[test]
    fn a_line_across_the_antimeridian_is_two_shores_not_one_world_wide_stripe() {
        let coastline = Coastline::parse("10,170 10,179 10,-179 10,-170\n");
        assert_eq!(coastline.len(), 2, "the walk became two shorelines");
        let lengths: Vec<usize> = coastline.outlines().map(<[LatLon]>::len).collect();
        assert_eq!(lengths, vec![2, 2]);
    }

    /// A bad token is refused and counted, and a line left with one point is not a shoreline.
    #[test]
    fn a_bad_point_is_refused_and_counted_rather_than_silently_dropped() {
        let coastline = Coastline::parse("# comment\n\n10,10 10,11 not-a-point 10,12\n30,30 30,31\n95,10 95,11\n10,200 10,201\n10,10\n");
        // not-a-point is one refusal; the 95°N and 200°E tokens are four more; the one-point line is a sixth. The
        // two clean lines survive, and the first keeps the points either side of the refused token.
        assert_eq!(coastline.skipped(), 6);
        assert_eq!(coastline.len(), 2, "both clean lines survive");
        assert_eq!(coastline.points(), 5);
        let first: Vec<LatLon> = coastline.outlines().next().expect("the clean line").to_vec();
        assert_eq!(first, vec![at(10.0, 10.0), at(10.0, 11.0), at(10.0, 12.0)]);
    }

    /// Places opens where the photographs are, not at a guess: the fit puts both clusters on screen.
    #[test]
    fn fitting_the_view_puts_the_photographs_on_screen_where_a_fixed_zoom_loses_them() {
        let photos = [at(50.8225, -0.1372), at(-33.8688, 151.2093)];
        let width = 800.0;
        let height = 600.0;
        let centre = bounds_centre(&photos).expect("a centre");
        let zoom = fit_zoom(&photos, width, height, MAX_FIT_ZOOM);
        assert!(zoom < 5, "the fit must zoom out, got {zoom}");
        let origin = project(centre, zoom).expect("an origin");
        let origin = Point {
            x: origin.x - width / 2.0,
            y: origin.y - height / 2.0,
        };
        for at in photos {
            let point = project(at, zoom).expect("on the world");
            let x = point.x - origin.x;
            let y = point.y - origin.y;
            assert!(
                x >= FIT_PADDING - 1.0 && x <= width - FIT_PADDING + 1.0,
                "x {x} is inside the padding"
            );
            assert!(
                y >= FIT_PADDING - 1.0 && y <= height - FIT_PADDING + 1.0,
                "y {y} is inside the padding"
            );
        }
    }

    /// The fit never goes deeper than the caller allows, and a single point is not an infinite zoom.
    #[test]
    fn the_fit_honours_the_callers_ceiling_and_survives_a_single_point() {
        let world = [at(-33.0, -180.0), at(60.0, 179.0)];
        assert_eq!(fit_zoom(&world, 800.0, 600.0, 12), 1, "a whole world fits at zoom 1");
        assert_eq!(fit_zoom(&world, 800.0, 600.0, 3), 1, "a ceiling above the fit does not change it");
        assert_eq!(fit_zoom(&world, 800.0, 600.0, 0), 0, "a ceiling at 0 decides");
        assert_eq!(
            fit_zoom(&[at(50.82, -0.13), at(-33.87, 151.21)], 800.0, 600.0, 1),
            1,
            "the tile set's top zoom wins when the spread would need more"
        );
        assert_eq!(
            fit_zoom(&[at(50.82, -0.13)], 800.0, 600.0, 18),
            MAX_FIT_ZOOM,
            "one photograph has no spread, so the ceiling answers"
        );
        assert_eq!(fit_zoom(&[], 800.0, 600.0, 18), MAX_FIT_ZOOM);
        assert_eq!(fit_zoom(&world, 10.0, 600.0, 12), 0, "a viewport smaller than its padding");
        assert_eq!(
            fit_zoom(&[at(88.0, 0.0)], 800.0, 600.0, 18),
            MAX_FIT_ZOOM,
            "a polar photograph is refused by the projection, not by the fit"
        );
    }

    /// The centre is the middle of the box, which is not the mean of the latitudes.
    #[test]
    fn the_centre_is_the_middle_of_the_spread_not_the_mean_of_the_latitudes() {
        let brighton = at(50.8225, -0.1372);
        let lima = at(-12.0464, -77.0428);
        let centre = bounds_centre(&[brighton, lima]).expect("a centre");
        assert!(
            centre.lat > lima.lat && centre.lat < brighton.lat,
            "the centre {} is outside the spread",
            centre.lat
        );
        // The mean would be 19.4°N, which is what the old view used. The box's middle is near the Mercator midpoint,
        // about 26°N here — the assertion is that it is well north of the mean, because that is the difference that
        // pushed the northern dot off the edge.
        let mean = (brighton.lat + lima.lat) / 2.0;
        assert!(centre.lat > mean + 2.0, "centre {} is not north of the mean {mean}", centre.lat);
        assert!((centre.lon - (brighton.lon + lima.lon) / 2.0).abs() < 1e-9, "longitude is linear");

        let single = bounds_centre(&[brighton]).expect("a single photograph has a centre");
        assert!((single.lat - brighton.lat).abs() < 1e-9 && (single.lon - brighton.lon).abs() < 1e-9);
        assert_eq!(bounds_centre(&[]), None);
        // A photograph the projection refuses is not part of the box.
        assert_eq!(bounds_centre(&[at(88.0, 0.0)]), None);
    }
}
