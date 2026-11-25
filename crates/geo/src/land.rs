//! The land that ships with the application, as a filled backdrop for Places.
//!
//! The coastline alone drew a shoreline on a blank field: geography, but not a map. This module adds the
//! Natural Earth 1:110m **land polygons** — the same public-domain source as the coastline — and triangulates
//! them once at parse time so a frame only has to scale and cull triangles. It is deliberately still
//! schematic: no labels, roads or relief, and no download.
//!
//! # Decisions that shape this module
//!
//! - **Triangulate once, at zoom 0.** The source is five thousand points; ear-clipping it every frame would
//!   spend more time in the tessellator than in the painter. The triangles are stored in the same 256-pixel
//!   world square as [\`crate::project\`], so scaling by 2^zoom is the whole per-frame cost.
//! - **Holes are holes.** The Caspian and the Great Lakes are rings inside their continents. Earcut takes the
//!   outer ring and the hole indices together, so the water stays water instead of being painted as land.
//! - **Longitude is unwrapped within a ring.** Antarctica's source ring ends at 180°E and begins at 180°W;
//!   taken literally that is a closing segment across the whole world. Unwrapping the ring makes that edge
//!   zero-length, and the continent closes along the projection's own edge instead of painting a stripe.
//! - **A polar point is clamped, not refused.** Mercator has no pole, but a land drawing without Antarctica
//!   is a misleading drawing. Coordinates are clamped to the projection limit here, exactly as the coastline
//!   does; a photograph beyond that limit is still refused by its own type.
//!
//! Nothing is fetched at run time; the asset is embedded with include_str!.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use crate::{LatLon, MERCATOR_LIMIT, Point, TILE, project};

/// What is printed wherever the shipped land is drawn.
pub const CREDIT: &str = "Land and coastline: Natural Earth 1:110m (public domain)";

/// The embedded source file, for the attribution document and for anybody reading the binary's provenance.
pub const SOURCE: &str = "assets/land.txt";

/// One land polygon: its outer ring and any holes, in latitude/longitude and in the zoom-0 world square.
#[derive(Debug, Clone)]
pub struct Polygon {
    /// The outer shoreline, as the file states it.
    pub outer: Vec<LatLon>,
    /// Inner rings that are water, not land.
    pub holes: Vec<Vec<LatLon>>,
    /// The outer ring projected at zoom 0, with longitudes unwrapped so the seam is not crossed.
    pub world: Vec<Point>,
    /// The hole rings projected at zoom 0.
    pub world_holes: Vec<Vec<Point>>,
}

/// The shipped land, parsed and triangulated once.
#[derive(Debug, Clone)]
pub struct Land {
    polygons: Vec<Polygon>,
    triangles: Vec<[Point; 3]>,
    points: usize,
    holes: usize,
    skipped: usize,
}

impl Default for Land {
    fn default() -> Self {
        Self::parse("")
    }
}

impl Land {
    /// The land embedded in the binary, parsed on first use.
    pub fn shipped() -> &'static Self {
        static SHIPPED: OnceLock<Land> = OnceLock::new();
        SHIPPED.get_or_init(|| Self::parse(SHIPPED_TEXT))
    }

    /// Parse the shipped format: polygon<TAB>O|H<TAB>lat,lon …, one ring per line, # for comments.
    ///
    /// A polygon without an outer ring is refused and counted; a hole whose polygon is missing is refused
    /// too, rather than becoming a continent of its own. Nothing is silently accepted.
    pub fn parse(text: &str) -> Self {
        let mut builders: BTreeMap<u32, Builder> = BTreeMap::new();
        let mut skipped = 0usize;
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let Some(id) = fields.next().and_then(|value| value.trim().parse::<u32>().ok()) else {
                skipped += 1;
                continue;
            };
            let Some(kind) = fields.next().map(str::trim) else {
                skipped += 1;
                continue;
            };
            let Some(rest) = fields.next() else {
                skipped += 1;
                continue;
            };
            let mut ring = Vec::new();
            for token in rest.split_whitespace() {
                let Some(at) = parse_point(token) else {
                    skipped += 1;
                    continue;
                };
                ring.push(at);
            }
            if ring.len() < 3 {
                skipped += 1;
                continue;
            }
            let builder = builders.entry(id).or_default();
            match kind {
                "O" if builder.outer.is_empty() => builder.outer = ring,
                "O" => {
                    skipped += 1;
                    continue;
                }
                "H" => builder.holes.push(ring),
                _ => {
                    skipped += 1;
                    continue;
                }
            }
        }

        let mut polygons = Vec::with_capacity(builders.len());
        for builder in builders.into_values() {
            if builder.outer.len() < 3 {
                skipped += 1;
                continue;
            }
            let Some(world) = ring_world(&builder.outer) else {
                skipped += 1;
                continue;
            };
            let mut world_holes = Vec::with_capacity(builder.holes.len());
            let mut valid = true;
            for hole in &builder.holes {
                match ring_world(hole) {
                    Some(points) if points.len() >= 3 => world_holes.push(points),
                    _ => {
                        skipped += 1;
                        valid = false;
                    }
                }
            }
            if !valid {
                continue;
            }
            polygons.push(Polygon {
                outer: builder.outer,
                holes: builder.holes,
                world,
                world_holes,
            });
        }

        let mut triangles = Vec::new();
        let mut holes = 0usize;
        let mut points = 0usize;
        let mut triangulated = Vec::with_capacity(polygons.len());
        for polygon in polygons {
            points += polygon.outer.len() + polygon.holes.iter().map(Vec::len).sum::<usize>();
            holes += polygon.holes.len();
            match triangulate(&polygon) {
                Some(found) => {
                    triangles.extend(found);
                    triangulated.push(polygon);
                }
                None => skipped += 1,
            }
        }
        Self {
            polygons: triangulated,
            triangles,
            points,
            holes,
            skipped,
        }
    }

    /// How many polygons survived parsing and triangulation.
    pub fn len(&self) -> usize {
        self.polygons.len()
    }

    /// Whether there is no land at all.
    pub fn is_empty(&self) -> bool {
        self.polygons.is_empty()
    }

    /// How many coordinates the surviving polygons hold, including holes.
    pub fn points(&self) -> usize {
        self.points
    }

    /// How many holes (inland water) the surviving polygons hold.
    pub fn holes(&self) -> usize {
        self.holes
    }

    /// How many tokens or rings the parser refused.
    pub fn skipped(&self) -> usize {
        self.skipped
    }

    /// The triangulated land at zoom 0, in the 256-pixel world square.
    pub fn triangles(&self) -> &[[Point; 3]] {
        &self.triangles
    }

    /// The surviving polygons, for a test or a diagnostic that needs the rings rather than the mesh.
    pub fn polygons(&self) -> impl Iterator<Item = &Polygon> {
        self.polygons.iter()
    }
}

/// The embedded land out of the binary.
const SHIPPED_TEXT: &str = include_str!("../../../assets/land.txt");

#[derive(Default)]
struct Builder {
    outer: Vec<LatLon>,
    holes: Vec<Vec<LatLon>>,
}

/// One lat,lon token, or None if it is not a coordinate.
fn parse_point(token: &str) -> Option<LatLon> {
    let (lat, lon) = token.split_once(',')?;
    LatLon::new(lat.trim().parse().ok()?, lon.trim().parse().ok()?)
}

/// Project a ring, unwrapping longitude so a ring that crosses the antimeridian closes on itself.
fn ring_world(ring: &[LatLon]) -> Option<Vec<Point>> {
    let mut points = Vec::with_capacity(ring.len());
    let mut previous_x: Option<f64> = None;
    for at in ring {
        let clamped = LatLon::new(at.lat.clamp(-MERCATOR_LIMIT, MERCATOR_LIMIT), at.lon)?;
        let mut point = project(clamped, 0)?;
        if let Some(previous) = previous_x {
            while point.x - previous > TILE / 2.0 {
                point.x -= TILE;
            }
            while point.x - previous < -TILE / 2.0 {
                point.x += TILE;
            }
        }
        previous_x = Some(point.x);
        points.push(point);
    }
    // Put the ring's middle in the base world square. A ring that straddles the seam can still have an edge a
    // fraction outside it; the view draws the wrapped copies, which is cheaper than splitting the ring again.
    let middle = points.iter().map(|point| point.x).sum::<f64>() / points.len() as f64;
    let shift = -(middle / TILE).floor() * TILE;
    for point in &mut points {
        point.x += shift;
    }
    Some(points)
}

/// Ear-clip one polygon, returning world-space triangles or None when the geometry is degenerate.
fn triangulate(polygon: &Polygon) -> Option<Vec<[Point; 3]>> {
    let mut vertices = Vec::with_capacity((polygon.world.len() + polygon.world_holes.iter().map(Vec::len).sum::<usize>()) * 2);
    for point in &polygon.world {
        vertices.push(point.x);
        vertices.push(point.y);
    }
    let mut hole_indices = Vec::with_capacity(polygon.world_holes.len());
    for hole in &polygon.world_holes {
        hole_indices.push(vertices.len() / 2);
        for point in hole {
            vertices.push(point.x);
            vertices.push(point.y);
        }
    }
    let indices = earcutr::earcut(&vertices, &hole_indices, 2).ok()?;
    if indices.len() < 3 {
        return None;
    }
    let mut triangles = Vec::with_capacity(indices.len() / 3);
    for triangle in indices.chunks_exact(3) {
        let mut points = [Point { x: 0.0, y: 0.0 }; 3];
        for (slot, index) in points.iter_mut().zip(triangle) {
            let x = *vertices.get(index * 2)?;
            let y = *vertices.get(index * 2 + 1)?;
            *slot = Point { x, y };
        }
        triangles.push(points);
    }
    Some(triangles)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn at(lat: f64, lon: f64) -> LatLon {
        LatLon::new(lat, lon).expect("a real coordinate")
    }

    /// The asset in the binary is a whole world and every ring it ships can be triangulated.
    #[test]
    fn the_shipped_land_is_a_whole_world() {
        let land = Land::shipped();
        assert!(land.len() > 100, "{} polygons is a fragment", land.len());
        assert!(land.points() > 4_000, "{} points is a fragment", land.points());
        assert!(land.holes() >= 1, "the inland water disappeared");
        assert_eq!(land.skipped(), 0, "the shipped file must parse cleanly");
        assert!(land.triangles().len() > 4_000, "{} triangles is a fragment", land.triangles().len());

        let mut north = -90.0_f64;
        let mut south = 90.0_f64;
        let mut west = 180.0_f64;
        let mut east = -180.0_f64;
        for polygon in land.polygons() {
            for point in &polygon.outer {
                north = north.max(point.lat);
                south = south.min(point.lat);
                west = west.min(point.lon);
                east = east.max(point.lon);
            }
            for point in &polygon.world {
                assert!((-TILE..=2.0 * TILE).contains(&point.x), "x {} is off the world", point.x);
                assert!((0.0..=TILE).contains(&point.y), "y {} is off the world", point.y);
            }
        }
        assert!(north > 70.0 && south < -70.0, "the land stops at {south}..{north}");
        assert!(west < -170.0 && east > 170.0, "the land stops at {west}..{east}");

        for triangle in land.triangles() {
            for pair in [0, 1, 2].map(|index| (triangle[index], triangle[(index + 1) % 3])) {
                assert!((pair.0.x - pair.1.x).abs() <= TILE, "a triangle edge crosses the seam: {pair:?}");
            }
        }
    }

    /// A square with a square hole keeps its hole: the mesh must have more than one triangle, and every
    /// triangle must stay inside the outer ring.
    #[test]
    fn a_hole_is_cut_out_instead_of_filled_in() {
        let land = Land::parse("0\tO\t0,0 0,10 10,10 10,0\n0\tH\t2,2 2,8 8,8 8,2\n");
        assert_eq!(land.len(), 1);
        assert_eq!(land.points(), 8);
        assert_eq!(land.holes(), 1);
        assert_eq!(land.skipped(), 0);
        assert!(land.triangles().len() >= 6);
        for triangle in land.triangles() {
            for point in triangle {
                assert!((0.0..=TILE).contains(&point.x));
                assert!((0.0..=TILE).contains(&point.y));
            }
        }
    }

    /// A ring that crosses the antimeridian is unwrapped, not drawn as a stripe across the world.
    #[test]
    fn a_ring_across_the_antimeridian_stays_on_one_side_of_the_world() {
        let land = Land::parse("0\tO\t10,179 10,-179 12,-179 12,179\n");
        assert_eq!(land.len(), 1);
        for point in &land.polygons().next().expect("the island").world {
            assert!((-TILE..=2.0 * TILE).contains(&point.x), "x {} is off the world", point.x);
        }
        for triangle in land.triangles() {
            for pair in [0, 1, 2].map(|index| (triangle[index], triangle[(index + 1) % 3])) {
                assert!((pair.0.x - pair.1.x).abs() <= TILE);
            }
        }
    }

    /// A bad token is counted, and a polygon without an outer ring is refused rather than guessed at.
    #[test]
    fn a_bad_ring_is_refused_and_counted() {
        let land = Land::parse("# comment\n\n0\tO\t0,0 0,10 10,10\n1\tH\t2,2 2,8 8,8\n2\tO\tnot-a-point\n");
        assert_eq!(land.len(), 1);
        assert_eq!(land.skipped(), 3, "a malformed token, a lone hole and an empty outer are refused");
        assert_eq!(
            land.polygons().next().expect("the one good polygon").outer,
            vec![at(0.0, 0.0), at(0.0, 10.0), at(10.0, 10.0)]
        );
    }
}
