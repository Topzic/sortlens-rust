//! The coastline that ships with the application, and the two questions it answers.
//!
//! A field of dots on a blank rectangle looks like a map and says nothing about where anything is, so the
//! application now carries a **starter coastline**: the Natural Earth 1:110m coastline, which is public domain and
//! therefore the one world outline this project can ship without asking anybody's permission. It is embedded with
//! `include_str!`, so a zero-config build has geography and nothing is fetched at run time.
//!
//! # Decisions that shape this module
//!
//! - **Schematic, and it says so.** 1:110m is a world map for a wall, not a navigation chart: at street zoom the
//!   outline is a staircase of 0.001° steps and whole harbours are missing. The credit line printed beside it in the
//!   view is the whole credit, and the view offers the person's own `tiles/` folder for a real basemap. A starter
//!   outline that pretended to be a map would be worse than no outline.
//! - **The antimeridian splits a line; it never joins it.** A polyline that walks from 179°E to 179°W is two
//!   separate shorelines that the source data happens to keep in one file, and drawing the segment between them
//!   would paint a horizontal stripe across the entire world. The split happens at parse time so no later code has
//!   to remember.
//! - **Antarctica is drawn along the projection's own edge.** Six of the source's points sit below −85.05°, where
//!   Mercator diverges, so their latitude is clamped to the limit before projecting. A photograph beyond that
//!   latitude is still refused rather than clamped — a wrong statement about a photograph — but a coastline is a
//!   drawing of land, and the alternative here is a missing continent. The clamp is why every projected point is
//!   inside the world square.
//! - **Projection happens once, at zoom 0.** `screen_lines` multiplies by `2^zoom` and subtracts the viewport
//!   origin: five thousand points of tangent and logarithm per frame would be work done for a result that is
//!   already known, and a map that costs a millisecond of trigonometry a frame is a map that stutters.
//! - **Only the segments on screen are returned.** A polyline is cut into runs of consecutive visible segments, so
//!   the Antarctica outline does not hand egui three thousand points to clip — the caller gets geometry it can put
//!   straight into a painter.
//!
//! # Why these two functions live here
//!
//! [`bounds_centre`] and [`fit_zoom`] are the answer to "Places opens over the middle of an ocean with both clusters
//! off screen". The mean of the photographs' latitudes is not the middle of their spread — a library of Brighton
//! and Lima has a mean latitude of 19°N, hundreds of kilometres north of either — and a fixed opening zoom of 5 is a
//! guess about how far apart somebody's photographs are. Both are projection arithmetic, so both are testable
//! without a window, which is why they are not in the view.

use std::sync::OnceLock;

use crate::{LatLon, MERCATOR_LIMIT, Point, project, unproject};

/// What is printed wherever the shipped coastline is drawn.
///
/// The licence line travels with the drawing, exactly as a tile folder's does: Natural Earth asks for credit rather
/// than requiring it, and an unlabelled map implies the application made it.
pub const CREDIT: &str = "Coastline: Natural Earth 1:110m (public domain)";

/// The embedded source file, for the attribution document and for anybody reading the binary's provenance.
pub const SOURCE: &str = "assets/coastline.txt";

/// How much of the viewport [`fit_zoom`] keeps clear around the photographs, in pixels.
///
/// A dot at the very edge of the map is a dot whose label is cut off, and a spread that touches both edges reads as
/// "there is more beyond here" when there is not.
pub const FIT_PADDING: f64 = 28.0;

/// The deepest zoom [`fit_zoom`] will choose on its own.
///
/// The outline is drawn at 1:110m, so a fitted view deeper than this shows a staircase rather than a coast. A person
/// can still zoom past it with the `+` button: this bound is about what the view picks unasked, not about what it
/// permits.
pub const MAX_FIT_ZOOM: u8 = 10;

/// One shoreline: the coordinates as the file states them, and the same points projected at zoom 0.
///
/// Both are kept because they answer different questions. The coordinates are the honest data (a test can ask
/// whether the file really covers Antarctica); the zoom-0 points are what [`Coastline::screen_lines`] scales, and
/// building them at parse time is what keeps the frame free of trigonometry.
#[derive(Debug, Clone)]
struct Outline {
    coordinates: Vec<LatLon>,
    world: Vec<Point>,
}

/// The shipped coastline, parsed once.
///
/// `lines` and `points` are the counts a test and a status line quote; `skipped` is the number of tokens or lines the
/// parser refused, kept rather than hidden so a corrupt asset is visible instead of silently smaller.
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

    /// Parse the shipped format: one polyline per line, `lat,lon` pairs separated by spaces, `#` for comments.
    ///
    /// A malformed token is skipped and counted; a line that is left with fewer than two points is not a shoreline
    /// and is counted too. Nothing is silently accepted: a coastline that quietly lost a continent would be a map
    /// that lies, and the count is what makes the loss visible.
    pub fn parse(text: &str) -> Self {
        let mut outlines: Vec<Outline> = Vec::new();
        let mut skipped = 0usize;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut run = Outline {
                coordinates: Vec::new(),
                world: Vec::new(),
            };
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
                if !push(&mut run, at) {
                    skipped += 1;
                }
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

    /// The polylines to stroke for a viewport, in screen pixels relative to the viewport's top-left corner.
    ///
    /// Each returned polyline is a run of **consecutive segments that touch the viewport**, so a line that spans the
    /// world contributes only the part that is on the map, and no returned polyline is a world-crossing artefact. A
    /// viewport with no coastline in it (the middle of the Pacific at street zoom) returns nothing, and the caller
    /// paints its sea and moves on.
    pub fn screen_lines(&self, zoom: u8, origin: Point, width: f64, height: f64) -> Vec<Vec<Point>> {
        if !origin.x.is_finite() || !origin.y.is_finite() || width <= 0.0 || height <= 0.0 {
            return Vec::new();
        }
        let scale = 2.0_f64.powi(i32::from(zoom));
        let mut lines = Vec::new();
        for outline in &self.outlines {
            let mut run: Vec<Point> = Vec::new();
            for pair in outline.world.windows(2) {
                let a = on_screen(pair[0], scale, origin);
                let b = on_screen(pair[1], scale, origin);
                if segment_touches(a, b, width, height) {
                    if run.is_empty() {
                        run.push(a);
                    }
                    run.push(b);
                } else if !run.is_empty() {
                    lines.push(std::mem::take(&mut run));
                }
            }
            if !run.is_empty() {
                lines.push(run);
            }
        }
        lines
    }
}

/// The embedded coastline out of the binary.
const SHIPPED_TEXT: &str = include_str!("../../../assets/coastline.txt");

/// One `lat,lon` token, or `None` if it is not a coordinate.
fn parse_point(token: &str) -> Option<LatLon> {
    let (lat, lon) = token.split_once(',')?;
    LatLon::new(lat.trim().parse().ok()?, lon.trim().parse().ok()?)
}

/// Add a point to a run, projecting it with the latitude clamped to what Mercator can draw.
///
/// The coordinate and its projection are pushed together or not at all, so the two lists never disagree about which
/// point is which — a half-added point would shift every later segment onto its neighbour's shore.
fn push(run: &mut Outline, at: LatLon) -> bool {
    let Some(clamped) = LatLon::new(at.lat.clamp(-MERCATOR_LIMIT, MERCATOR_LIMIT), at.lon) else {
        return false;
    };
    let Some(world) = project(clamped, 0) else {
        return false;
    };
    run.coordinates.push(at);
    run.world.push(world);
    true
}

/// Close a run, returning how many points were dropped because a lone point is not a shoreline.
fn finish(outlines: &mut Vec<Outline>, run: &mut Outline) -> usize {
    if run.world.len() < 2 {
        let dropped = usize::from(!run.world.is_empty());
        run.coordinates.clear();
        run.world.clear();
        return dropped;
    }
    outlines.push(std::mem::replace(
        run,
        Outline {
            coordinates: Vec::new(),
            world: Vec::new(),
        },
    ));
    0
}

/// A zoom-0 world point as screen pixels at `scale`.
fn on_screen(at: Point, scale: f64, origin: Point) -> Point {
    Point {
        x: at.x * scale - origin.x,
        y: at.y * scale - origin.y,
    }
}

/// Whether a segment is close enough to the viewport to be worth handing to the painter.
///
/// The bounding-box test is deliberately generous by a pixel: a segment just outside the edge still ends where the
/// next visible run begins, and dropping it leaves a visible millimetre gap along the border.
fn segment_touches(a: Point, b: Point, width: f64, height: f64) -> bool {
    const MARGIN: f64 = 1.0;
    let min_x = a.x.min(b.x);
    let max_x = a.x.max(b.x);
    let min_y = a.y.min(b.y);
    let max_y = a.y.max(b.y);
    max_x >= -MARGIN && min_x <= width + MARGIN && max_y >= -MARGIN && min_y <= height + MARGIN
}

/// The centre of the box the photographs occupy, in projected space.
///
/// Not the mean: the mean of Brighton (50.8°N) and Lima (12°S) is 19°N, which is 1,400 km north of the box's middle,
/// and a view centred there pushes the northern dot towards the edge. The box's middle is the point that leaves the
/// same margin at every side, which is what "show me where these are" means. `None` when no photograph can be
/// projected at all.
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

/// The deepest zoom at which every photograph fits inside the viewport, with [`FIT_PADDING`] clear on every side.
///
/// `highest` is the deepest zoom the caller is willing to use — the top of a tile folder's range, or [`MAX_FIT_ZOOM`]
/// when the shipped coastline is the basemap — and the answer never exceeds it, nor [`MAX_FIT_ZOOM`] itself. A
/// photograph stack at one point has no spread to fit, so the deepest allowed zoom is the answer; an empty slice is
/// the same case, because a caller with nothing to fit wants the map it would have opened with.
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
    // Zoom 0 is 256 pixels of world, so a span of `s` pixels at zoom `z` is `s·2^z`: the deepest zoom that still fits
    // is `floor(log2(available / s))`, with a one-pixel floor so a flat spread is not an infinite zoom.
    let fit = (width / span_x.max(1.0)).min(height / span_y.max(1.0));
    let zoom = fit.log2().floor();
    if !zoom.is_finite() || zoom >= f64::from(highest) {
        return highest;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let zoom = zoom as u8;
    zoom
}

/// The zoom-0 box around everything that can be projected: `(min_x, min_y, max_x, max_y)`.
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
    ///
    /// This is the test that would have caught a truncated `include_str!`, an export that switched to lon,lat, or a
    /// rounding pass that wrote `NaN`. It also pins the poles and both sides of the antimeridian, because a coastline
    /// missing Antarctica or the Pacific is not a world map.
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

        // Projection is where a mis-ordered pair would show up as a coordinate in the wrong ocean: every point of the
        // shipped file must land inside the world square, which is also the assertion that Antarctica's clamping
        // happened.
        for outline in &coastline.outlines {
            for point in &outline.world {
                assert!((0.0..=256.0).contains(&point.x), "x {} is off the world", point.x);
                assert!((0.0..=256.0).contains(&point.y), "y {} is off the world", point.y);
            }
        }
    }

    /// A line that steps across the antimeridian is split into two shores instead of drawn across the world.
    ///
    /// The failure this pins is loud and embarrassing: one segment from 179°E to 179°W is a stripe through every
    /// other continent, and it is what a naive parser produces because the source keeps such a walk in one line.
    #[test]
    fn a_line_across_the_antimeridian_is_two_shores_not_one_world_wide_stripe() {
        let coastline = Coastline::parse("10,170 10,179 10,-179 10,-170\n");
        assert_eq!(coastline.len(), 2, "the walk became two shorelines");
        let lengths: Vec<usize> = coastline.outlines().map(<[LatLon]>::len).collect();
        assert_eq!(lengths, vec![2, 2]);

        // And what is drawn agrees: at zoom 2 the world is 1024 px, so a run that crossed the seam would be a line
        // about a thousand pixels wide. No returned segment may be.
        let origin = Point { x: 0.0, y: 0.0 };
        let lines = coastline.screen_lines(2, origin, 1024.0, 1024.0);
        assert_eq!(lines.len(), 2);
        for line in &lines {
            for pair in line.windows(2) {
                assert!(
                    (pair[1].x - pair[0].x).abs() < 100.0,
                    "a {:?} wide segment is the seam being drawn",
                    (pair[1].x - pair[0].x).abs()
                );
            }
        }
    }

    /// A bad token is refused and counted, and a line left with one point is not a shoreline.
    #[test]
    fn a_bad_point_is_refused_and_counted_rather_than_silently_dropped() {
        let coastline = Coastline::parse("# comment\n\n10,10 10,11 not-a-point 10,12\n30,30 30,31\n95,10 95,11\n10,200 10,201\n10,10\n");
        // `not-a-point` is one refusal; the 95°N and 200°E tokens are four more; the one-point line is a sixth. The two
        // clean lines survive, and the first keeps the points either side of the refused token.
        assert_eq!(coastline.skipped(), 6);
        assert_eq!(coastline.len(), 2, "both clean lines survive");
        assert_eq!(coastline.points(), 5);
        let first: Vec<LatLon> = coastline.outlines().next().expect("the clean line").to_vec();
        assert_eq!(first, vec![at(10.0, 10.0), at(10.0, 11.0), at(10.0, 12.0)]);
    }

    /// Only the coastlines on screen are handed back, and their points are screen pixels.
    #[test]
    fn only_the_segments_on_screen_are_returned_and_they_are_in_screen_pixels() {
        // A vertical shore on the prime meridian, from 10°S to 10°N: at zoom 4 it is a few dozen pixels tall.
        let coastline = Coastline::parse("-10,0 0,0 10,0\n");
        let brighton = at(50.8, -0.14);
        let zoom = 4;
        let width = 800.0;
        let height = 600.0;
        let origin = crate::viewport_origin(zoom, brighton, width, height).expect("an origin");

        // Centred on Brighton, the equator is below the bottom edge: the shore is off screen and nothing is drawn.
        assert!(
            coastline.screen_lines(zoom, origin, width, height).is_empty(),
            "a shoreline in Africa is not on a map of the English Channel"
        );

        // Centred on the equator, the same shore is a line down the middle of the screen.
        let origin = crate::viewport_origin(zoom, at(0.0, 0.0), width, height).expect("an origin");
        let lines = coastline.screen_lines(zoom, origin, width, height);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 3);
        assert!((lines[0][1].x - width / 2.0).abs() < 1e-6, "the prime meridian is the middle");
        for point in &lines[0] {
            assert!(point.y >= 0.0 && point.y <= height, "y {} is off screen", point.y);
        }
        // An impossible viewport is not a panic in a paint path.
        assert!(coastline.screen_lines(zoom, origin, 0.0, height).is_empty());
        assert!(Coastline::default().screen_lines(zoom, origin, width, height).is_empty());
    }

    /// Places opens where the photographs are, not at a guess: the fit puts both clusters on screen where the old
    /// fixed zoom did not.
    ///
    /// This is the reported defect: a library of Brighton and Sydney opened at zoom 5 with both dots outside the
    /// viewport, so Places looked empty. The first half is the failing half — the old zoom really does lose them.
    #[test]
    fn fitting_the_view_puts_the_photographs_on_screen_where_a_fixed_zoom_loses_them() {
        let photos = [at(50.8225, -0.1372), at(-33.8688, 151.2093)];
        let width = 800.0;
        let height = 600.0;

        // The zoom Places used to open at, kept here as the control: the defect being fixed.
        let old = 5;
        let centre = bounds_centre(&photos).expect("a centre");
        let origin = crate::viewport_origin(old, centre, width, height).expect("an origin");
        let visible = photos
            .iter()
            .filter(|at| crate::screen_position(**at, old, origin, width, height).is_some())
            .count();
        assert_eq!(visible, 0, "the defect: neither dot is on screen at the fixed zoom");

        let zoom = fit_zoom(&photos, width, height, MAX_FIT_ZOOM);
        assert!(zoom < old, "the fit must zoom out, got {zoom}");
        let origin = crate::viewport_origin(zoom, centre, width, height).expect("an origin");
        for at in photos {
            assert!(
                crate::screen_position(at, zoom, origin, width, height).is_some(),
                "{at:?} is still off screen at zoom {zoom}"
            );
        }
        // The margin is honoured, not just the viewport: neither dot may sit in the padding.
        for at in photos {
            let point = crate::screen_position(at, zoom, origin, width, height).expect("on screen");
            assert!(
                point.x >= FIT_PADDING - 1.0 && point.x <= width - FIT_PADDING + 1.0,
                "x {} is inside the padding",
                point.x
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
        // A ceiling below the fit is a floor for the framing: a tile set that stops at 1 shows the world at 1.
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
        // Unmappable photographs are not a spread and must not be a panic.
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
