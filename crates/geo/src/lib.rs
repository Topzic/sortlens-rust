//! Geotag parsing, offline gazetteer, clustering, and the tile folder that goes under them.
//!
//! Turn a latitude and longitude into a point on a **Web Mercator** map at a zoom level, group nearby points into
//! **clusters** that stay stable from frame to frame, look a coordinate up in an **offline gazetteer** so a photo can be
//! remembered as "Brighton" rather than 50.82°N 0.14°W, and find the **tiles** for a cell in a folder on disk.
//!
//! # Decisions that shape this crate
//!
//! - **Web Mercator, because tiles are Mercator.** Every offline basemap and every MBTiles archive is in the same
//!   projection, so projecting here is the only place the maths lives; a view that projected differently would draw
//!   photos beside the wrong coastline at high latitudes and nowhere near it at high zoom.
//! - **Mercator has no pole.** The projection diverges at ±90°, so the tile scheme stops at ±85.05112878°. A
//!   photograph geotagged beyond that limit is **refused rather than clamped**: clamping would draw it at the top
//!   edge as if it had been taken at 85°N, which is a wrong statement about a real photograph rather than a missing
//!   one. Every returned coordinate is inside the world square, so callers never have to check.
//! - **Clustering is a grid, and it is deterministic.** Points are assigned to a cell of fixed pixel size at the
//!   current zoom and each cell becomes a cluster. The output is sorted by a stable key, so the same photos produce
//!   the same clusters in the same order on every run — a cluster dot that jumps between refreshes is a bug the eye
//!   notices long before the code does.
//! - **The basemap is a folder, and its credit is not optional.** `tiles::Tileset` reads a `{z}/{x}/{y}.png` folder
//!   with an `attribution.txt` beside it, and refuses a folder without one: an unlabelled map implies the application
//!   made it. Nothing is fetched at runtime and no map data ships with the application.
//! - **The gazetteer is a file, never a network call.** This crate parses a tab-separated file the user puts in the
//!   library; it holds no place data of its own and downloads nothing, so the app stays local, and the data's licence
//!   stays where its attribution can be read.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use sortlens_core::PhotoId;

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The latitude, in degrees, at which the Mercator projection stops.
///
/// It is the latitude of the square world that tile schemes use, not an arbitrary guard: beyond it the projection
/// runs away towards infinity, so a coordinate there cannot be placed on a map that is made of tiles.
pub const MERCATOR_LIMIT: f64 = 85.051_128_78;

/// The edge of one tile, in pixels, at zoom level 0.
pub const TILE: f64 = 256.0;

/// The pixel size of a cluster cell at the current zoom.
///
/// 64 pixels is one quarter of a tile: small enough that two clusters in the same town are separate dots, large
/// enough that a burst of frames from one spot is one dot rather than forty overlapping ones.
pub const CELL_PX: f64 = 64.0;

/// A coordinate that has been checked.
///
/// Constructing one is the only way to get a [`LatLon`], so a function that takes one never has to ask whether it is
/// a real coordinate.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
// Deserialising goes through the constructor rather than filling the fields: a snapshot, a log record or a hand-edited
// file must not be able to put a coordinate that is not a place into the catalog. `try_from` means a bad value is an
// error the loader reports, not a photograph drawn in the wrong ocean.
#[serde(try_from = "Unchecked")]
pub struct LatLon {
    /// Degrees north of the equator, −90 to 90.
    pub lat: f64,
    /// Degrees east of Greenwich, −180 to 180.
    pub lon: f64,
}

/// The two numbers as they appear in a file, before they are known to be a coordinate.
#[derive(serde::Deserialize)]
struct Unchecked {
    lat: f64,
    lon: f64,
}

impl TryFrom<Unchecked> for LatLon {
    type Error = String;

    fn try_from(value: Unchecked) -> Result<Self, Self::Error> {
        Self::new(value.lat, value.lon).ok_or_else(|| format!("{} , {} is not a coordinate", value.lat, value.lon))
    }
}

impl LatLon {
    /// Check a coordinate.
    ///
    /// `None` for anything that is not a real position: a non-finite number, a latitude off the globe, or a
    /// longitude outside ±180 (which is not a refusal about the place — it is a refusal about the record, because
    /// 185° east is 175° west written wrongly, and guessing which is not this function's job).
    pub fn new(lat: f64, lon: f64) -> Option<Self> {
        if !lat.is_finite() || !lon.is_finite() {
            return None;
        }
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return None;
        }
        Some(Self { lat, lon })
    }

    /// Whether this coordinate can be drawn on a Mercator map.
    ///
    /// True for almost everything; false for the polar caps, where the projection has no answer.
    pub fn is_mappable(self) -> bool {
        self.lat.abs() <= MERCATOR_LIMIT
    }

    /// Distance in kilometres to another coordinate, on a sphere.
    ///
    /// The haversine formula at the Earth's mean radius. Good to a fraction of a percent over any distance a
    /// photograph is likely to travel, and used only for "is this near that", never for a measurement.
    pub fn distance_km(self, other: Self) -> f64 {
        const EARTH_KM: f64 = 6_371.008_8;
        let (lat1, lat2) = (self.lat.to_radians(), other.lat.to_radians());
        let delta_lat = (other.lat - self.lat).to_radians();
        let delta_lon = (other.lon - self.lon).to_radians();
        let a = (delta_lat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
        2.0 * EARTH_KM * a.sqrt().clamp(-1.0, 1.0).asin().min(std::f64::consts::FRAC_PI_2)
    }
}

/// A point in world pixels at a zoom level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Pixels from the left edge of the world square.
    pub x: f64,
    /// Pixels from the top edge of the world square.
    pub y: f64,
}

/// Project a coordinate to world pixels at `zoom`.
///
/// `None` for a coordinate Mercator cannot place — see [`MERCATOR_LIMIT`]. The result is always inside the world
/// square `0..256·2^zoom`, so a caller can multiply, subtract and clip without checking anything else.
pub fn project(coordinate: LatLon, zoom: u8) -> Option<Point> {
    if !coordinate.is_mappable() {
        return None;
    }
    let world = TILE * 2.0_f64.powi(i32::from(zoom));
    let x = (coordinate.lon + 180.0) / 360.0 * world;
    let lat = coordinate.lat.to_radians();
    let y = (1.0 - (lat.tan() + 1.0 / lat.cos()).ln() / std::f64::consts::PI) / 2.0 * world;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some(Point {
        x: x.clamp(0.0, world),
        y: y.clamp(0.0, world),
    })
}

/// Turn world pixels at `zoom` back into a coordinate.
///
/// The inverse of [`project`], used by "click on the map to search here" and by tests that round-trip. A point
/// outside the world square is clamped into it rather than refused, because it comes from a pointer and the
/// coordinate under it is a real one either way.
pub fn unproject(point: Point, zoom: u8) -> Option<LatLon> {
    if !point.x.is_finite() || !point.y.is_finite() {
        return None;
    }
    let world = TILE * 2.0_f64.powi(i32::from(zoom));
    let x = point.x.clamp(0.0, world);
    let y = point.y.clamp(0.0, world);
    let lon = x / world * 360.0 - 180.0;
    let n = std::f64::consts::PI * (1.0 - 2.0 * y / world);
    let lat = n.sinh().atan().to_degrees();
    LatLon::new(lat, lon)
}

/// A rectangle in screen pixels, relative to the top-left of the map's viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenRect {
    /// Distance from the viewport's left edge.
    pub x: f64,
    /// Distance from the viewport's top edge.
    pub y: f64,
    /// Width in pixels.
    pub w: f64,
    /// Height in pixels.
    pub h: f64,
}

/// One tile of the basemap and where it belongs on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileCell {
    /// The zoom level, so a placement can be checked against the set it came from.
    pub zoom: u8,
    /// The tile column, from the west edge of the world.
    pub x: u32,
    /// The tile row, from the north edge.
    pub y: u32,
}

/// The world pixel at the viewport's top-left corner, for a viewport centred on `centre`.
///
/// This is the one piece of state a map needs: it positions every tile and every dot, so it is computed once per
/// frame and passed around rather than each caller deriving its own centre and disagreeing by half a pixel. The
/// result may be negative or beyond the world square when the viewport is larger than the world or hangs over an
/// edge, which is exactly why [`visible_tiles`] clips rather than trusting the origin.
pub fn viewport_origin(zoom: u8, centre: LatLon, viewport_width: f64, viewport_height: f64) -> Option<Point> {
    let point = project(centre, zoom)?;
    Some(Point {
        x: point.x - viewport_width / 2.0,
        y: point.y - viewport_height / 2.0,
    })
}

/// Where a coordinate sits on screen, given the viewport's origin.
///
/// `None` when Mercator cannot place it at all, and `None` when the point is off screen: a dot drawn outside the
/// viewport is a dot at the edge pretending to be somewhere it is not.
pub fn screen_position(at: LatLon, zoom: u8, origin: Point, viewport_width: f64, viewport_height: f64) -> Option<Point> {
    let point = project(at, zoom)?;
    let screen = Point {
        x: point.x - origin.x,
        y: point.y - origin.y,
    };
    (screen.x >= 0.0 && screen.y >= 0.0 && screen.x <= viewport_width && screen.y <= viewport_height).then_some(screen)
}

/// The tiles that cover a viewport, with screen rectangles.
///
/// **The world does not wrap here.** Columns are clamped to the world square rather than repeated, because a map
/// that wraps draws the same photographs twice: at the antimeridian the seam is a real edge of the tile set, and
/// repeating it would put a second copy of Fiji beside the first with nothing to say which was which. A viewport
/// wider than the world shows the world and empty space beside it.
///
/// Tile 0,0 is the north-west corner, which is the order every tile set uses.
pub fn visible_tiles(zoom: u8, origin: Point, viewport_width: f64, viewport_height: f64) -> Vec<TileCell> {
    if !origin.x.is_finite() || !origin.y.is_finite() || viewport_width <= 0.0 || viewport_height <= 0.0 {
        return Vec::new();
    }
    let columns = 1u64 << u32::from(zoom);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let first_x = (origin.x / TILE).floor().max(0.0) as u64;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let first_y = (origin.y / TILE).floor().max(0.0) as u64;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let last_x = ((origin.x + viewport_width) / TILE).ceil().max(0.0) as u64;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let last_y = ((origin.y + viewport_height) / TILE).ceil().max(0.0) as u64;
    let mut cells = Vec::new();
    for y in first_y..last_y.min(columns) {
        for x in first_x..last_x.min(columns) {
            #[allow(clippy::cast_possible_truncation)]
            cells.push(TileCell {
                zoom,
                x: x as u32,
                y: y as u32,
            });
        }
    }
    cells
}

/// Where one tile's screen rectangle starts, given the viewport's origin.
pub fn tile_rect(cell: TileCell, origin: Point) -> ScreenRect {
    ScreenRect {
        x: f64::from(cell.x) * TILE - origin.x,
        y: f64::from(cell.y) * TILE - origin.y,
        w: TILE,
        h: TILE,
    }
}

/// One dot on the map: a cell of the grid and the photos inside it.
#[derive(Debug, Clone, PartialEq)]
pub struct Cluster {
    /// The grid cell, as integers at this zoom. Stable, and used as the sort key after the count.
    pub cell: (i64, i64),
    /// The photos in the cell, in the order given.
    pub members: Vec<PhotoId>,
    /// The centre of the photos, which is where the dot is drawn.
    pub centre: LatLon,
    /// The northernmost, southernmost, easternmost and westernmost latitude/longitude in the cell.
    pub bounds: (f64, f64, f64, f64),
}

impl Cluster {
    /// How many photos the dot stands for.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Whether the dot stands for nothing; never true for a cluster this crate produced.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Whether the dot is a single photograph rather than a group.
    pub fn is_single(&self) -> bool {
        self.members.len() == 1
    }
}

/// Group photos into cells at `zoom`.
///
/// Photos Mercator cannot place are left out rather than clamped: a dot at the top edge for a photograph taken at
/// the pole would be a claim about where it was taken that the map is not entitled to make.
///
/// The result is sorted by count (largest first) and then by cell, so it is identical for the same input in any
/// order. Clusters are what a map draws, and a map whose dots move when nothing changed is a map people stop
/// trusting.
pub fn cluster(photos: &[(PhotoId, LatLon)], zoom: u8) -> Vec<Cluster> {
    let mut cells: std::collections::BTreeMap<(i64, i64), Vec<(PhotoId, LatLon)>> = std::collections::BTreeMap::new();
    for (id, coordinate) in photos {
        let Some(point) = project(*coordinate, zoom) else {
            continue;
        };
        #[allow(clippy::cast_possible_truncation)]
        let cell = ((point.x / CELL_PX).floor() as i64, (point.y / CELL_PX).floor() as i64);
        cells.entry(cell).or_default().push((*id, *coordinate));
    }

    let mut clusters: Vec<Cluster> = cells
        .into_iter()
        .map(|(cell, members)| {
            let count = members.len() as f64;
            let sum_lat: f64 = members.iter().map(|(_, at)| at.lat).sum();
            let sum_lon: f64 = members.iter().map(|(_, at)| at.lon).sum();
            // The average can only fail to be a coordinate if the inputs were not, and they were checked when they
            // were built; falling back to the first member keeps a dot on the map rather than dropping it.
            let centre = LatLon::new(sum_lat / count, sum_lon / count)
                .or_else(|| members.first().map(|(_, at)| *at))
                .unwrap_or(LatLon { lat: 0.0, lon: 0.0 });
            let bounds = members
                .iter()
                .fold((f64::MIN, f64::MAX, f64::MIN, f64::MAX), |(north, south, east, west), (_, at)| {
                    (north.max(at.lat), south.min(at.lat), east.max(at.lon), west.min(at.lon))
                });
            Cluster {
                cell,
                members: members.into_iter().map(|(id, _)| id).collect(),
                centre,
                bounds,
            }
        })
        .collect();
    // Largest first, then by cell: a stable order that does not depend on how the photos arrived.
    clusters.sort_by(|left, right| right.members.len().cmp(&left.members.len()).then(left.cell.cmp(&right.cell)));
    clusters
}

/// A place from the gazetteer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// The name as the gazetteer spells it.
    pub name: String,
    /// Latitude in degrees, as written in the file.
    pub lat: i64,
    /// Longitude in degrees, as written in the file. Kept as the file's integer for exact comparison.
    pub lon: i64,
    /// The country code or name the file carries, when it carries one.
    pub country: Option<String>,
}

impl Place {
    /// The place as a coordinate, when the file's numbers are a real position.
    fn coordinate(&self) -> Option<LatLon> {
        #[allow(clippy::cast_precision_loss)]
        LatLon::new(self.lat as f64, self.lon as f64)
    }
}

/// An offline gazetteer: place names bucketed into one-degree cells.
///
/// The file format is tab-separated, one place per line, `name<TAB>lat<TAB>lon[<TAB>country]`, with `#` comments and
/// blank lines ignored. That is deliberately the plainest thing that can be read with `cut`, so a user can build one
/// from a published list with a shell one-liner and see exactly what the app will read.
#[derive(Debug, Clone, Default)]
pub struct Gazetteer {
    /// Places by one-degree cell, so a lookup touches a handful of candidates rather than every place on Earth.
    cells: std::collections::BTreeMap<(i64, i64), Vec<Place>>,
    /// How many places were read, including ones a lookup will never return.
    places: usize,
    /// Lines that could not be read, for the "your file has rounded corners" report.
    skipped: usize,
}

/// How big a gazetteer file may be.
///
/// A world gazetteer of four million places is on the order of a hundred megabytes; sixty-four is enough for that and
/// small enough that pointing the app at the wrong file (a disk image, a log) is refused instead of read into memory.
pub const MAX_GAZETTEER_BYTES: u64 = 64 * 1024 * 1024;

/// Why a gazetteer file could not be read.
#[derive(Debug)]
pub enum GazetteerError {
    /// There is no file there.
    Missing(std::path::PathBuf),
    /// The file is larger than [`MAX_GAZETTEER_BYTES`].
    TooLarge { path: std::path::PathBuf, bytes: u64 },
    /// The file could not be read.
    Unreadable { path: std::path::PathBuf, reason: String },
}

impl GazetteerError {
    /// What to tell the person who has to fix it.
    pub fn message(&self) -> String {
        match self {
            Self::Missing(path) => format!("there is no gazetteer at {}", path.display()),
            Self::TooLarge { path, bytes } => format!(
                "{} is {bytes} bytes, larger than the {MAX_GAZETTEER_BYTES}-byte limit; a gazetteer is a list of \
                 place names, not a disk image",
                path.display()
            ),
            Self::Unreadable { path, reason } => format!("could not read {}: {reason}", path.display()),
        }
    }
}

impl std::fmt::Display for GazetteerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for GazetteerError {}

impl Gazetteer {
    /// Read a gazetteer file from disk.
    ///
    /// The size is checked before the bytes are read, so a mistake in the path costs a `metadata` call rather than a
    /// gigabyte of memory. A file that parses to nothing is *not* an error here: [`Self::is_empty`] and
    /// [`Self::skipped`] tell the caller, because a gazetteer with three malformed lines is still a gazetteer.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, GazetteerError> {
        let path = path.as_ref().to_path_buf();
        let metadata = match std::fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(GazetteerError::Missing(path));
            }
            Err(error) => {
                return Err(GazetteerError::Unreadable {
                    path,
                    reason: error.to_string(),
                });
            }
        };
        if metadata.len() > MAX_GAZETTEER_BYTES {
            return Err(GazetteerError::TooLarge {
                path,
                bytes: metadata.len(),
            });
        }
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(Self::parse(&text)),
            Err(error) => Err(GazetteerError::Unreadable {
                path,
                reason: error.to_string(),
            }),
        }
    }

    /// Parse a gazetteer file.
    pub fn parse(text: &str) -> Self {
        let mut gazetteer = Self::default();
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.split('\t');
            let (Some(name), Some(lat), Some(lon)) = (fields.next(), fields.next(), fields.next()) else {
                gazetteer.skipped += 1;
                continue;
            };
            let (Ok(lat), Ok(lon)) = (lat.trim().parse::<i64>(), lon.trim().parse::<i64>()) else {
                gazetteer.skipped += 1;
                continue;
            };
            let name = name.trim();
            if name.is_empty() {
                gazetteer.skipped += 1;
                continue;
            }
            let country = fields.next().map(str::trim).filter(|text| !text.is_empty()).map(str::to_string);
            let place = Place {
                name: name.to_string(),
                lat,
                lon,
                country,
            };
            if place.coordinate().is_none() {
                gazetteer.skipped += 1;
                continue;
            }
            gazetteer.cells.entry((lat, lon)).or_default().push(place);
            gazetteer.places += 1;
        }
        gazetteer
    }

    /// How many places were read.
    pub fn len(&self) -> usize {
        self.places
    }

    /// Whether the gazetteer holds nothing.
    pub fn is_empty(&self) -> bool {
        self.places == 0
    }

    /// How many lines could not be read.
    pub fn skipped(&self) -> usize {
        self.skipped
    }

    /// The nearest place to a coordinate, within `max_km`.
    ///
    /// `None` when nothing in the file is near enough: "no place recorded" is not "the middle of the ocean", and the
    /// caller must be able to tell them apart. Ties are broken by name so the answer does not depend on file order.
    pub fn nearest(&self, at: LatLon, max_km: f64) -> Option<&Place> {
        if !max_km.is_finite() || max_km <= 0.0 {
            return None;
        }
        // One degree of latitude is ~111 km, so a radius is at most a few cells; a generous pad covers longitude
        // compression towards the poles and any rounding in how the file spells its coordinates.
        #[allow(clippy::cast_possible_truncation)]
        let pad = (max_km / 111.0).ceil() as i64 + 1;
        #[allow(clippy::cast_possible_truncation)]
        let (cell_lat, cell_lon) = (at.lat.round() as i64, at.lon.round() as i64);
        let mut best: Option<(&Place, f64)> = None;
        for lat in (cell_lat - pad)..=(cell_lat + pad) {
            for lon in (cell_lon - pad)..=(cell_lon + pad) {
                let Some(places) = self.cells.get(&(lat, lon)) else {
                    continue;
                };
                for place in places {
                    let Some(coordinate) = place.coordinate() else {
                        continue;
                    };
                    let distance = at.distance_km(coordinate);
                    if distance > max_km {
                        continue;
                    }
                    let better = match &best {
                        None => true,
                        Some((best_place, best_distance)) => {
                            distance < *best_distance || (distance == *best_distance && place.name < best_place.name)
                        }
                    };
                    if better {
                        best = Some((place, distance));
                    }
                }
            }
        }
        best.map(|(place, _)| place)
    }
}

pub mod tiles;

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn at(lat: f64, lon: f64) -> LatLon {
        LatLon::new(lat, lon).expect("a real coordinate")
    }

    /// A stored coordinate goes through the constructor, so a corrupt record is an error rather than a wrong dot.
    #[test]
    fn a_coordinate_that_is_not_a_place_cannot_be_loaded_from_a_file() {
        let good = r#"{"lat": 51.5, "lon": -0.12}"#;
        let back: LatLon = serde_json::from_str(good).expect("a real coordinate parses");
        assert!((back.lat - 51.5).abs() < 1e-12);
        let text = serde_json::to_string(&back).expect("serialise");
        assert_eq!(serde_json::from_str::<LatLon>(&text).expect("round trip"), back);

        for bad in [
            r#"{"lat": 91.0, "lon": 0.0}"#,
            r#"{"lat": 0.0, "lon": 181.0}"#,
            r#"{"lat": "north", "lon": 0.0}"#,
        ] {
            assert!(serde_json::from_str::<LatLon>(bad).is_err(), "{bad} should not load");
        }
    }

    #[test]
    fn a_coordinate_off_the_globe_is_refused_rather_than_folded_onto_it() {
        assert!(LatLon::new(0.0, 0.0).is_some());
        assert!(LatLon::new(51.5, -0.12).is_some());
        assert!(LatLon::new(90.0, 180.0).is_some(), "the corners are real, if unmappable");
        assert_eq!(LatLon::new(91.0, 0.0), None);
        assert_eq!(LatLon::new(0.0, 181.0), None, "181 east is not a place");
        assert_eq!(LatLon::new(f64::NAN, 0.0), None);
        assert_eq!(LatLon::new(0.0, f64::INFINITY), None);
    }

    #[test]
    fn the_projection_puts_the_known_places_where_the_tiles_do() {
        // At zoom 0 the world is one 256-pixel tile: Greenwich on the vertical centre line, the equator on the
        // horizontal one. Any other answer means the map is drawn beside the coastline, not on it.
        let origin = project(at(0.0, 0.0), 0).expect("mappable");
        assert!((origin.x - 128.0).abs() < 1e-9, "x was {}", origin.x);
        assert!((origin.y - 128.0).abs() < 1e-9, "y was {}", origin.y);

        // The antimeridian is the right edge, the north limit the top edge.
        let east = project(at(0.0, 180.0), 0).expect("mappable");
        assert!((east.x - 256.0).abs() < 1e-6);
        let north = project(at(MERCATOR_LIMIT, 0.0), 0).expect("mappable");
        assert!(north.y.abs() < 1e-3, "the limit maps to the top edge, got {}", north.y);

        // A zoom level doubles the world in each direction.
        let twice = project(at(0.0, 0.0), 1).expect("mappable");
        assert!((twice.x - 256.0).abs() < 1e-9);
        assert!((twice.y - 256.0).abs() < 1e-9);
    }

    #[test]
    fn the_poles_are_refused_because_mercator_has_no_answer_there() {
        // A real coordinate that the projection cannot place: refused, not clamped to the top edge.
        assert!(LatLon::new(89.0, 0.0).is_some(), "89 north is a real place");
        assert!(!at(89.0, 0.0).is_mappable());
        assert_eq!(project(at(89.0, 0.0), 4), None);
        assert_eq!(project(at(-89.0, 0.0), 4), None);
        assert!(project(at(MERCATOR_LIMIT, 0.0), 4).is_some());
    }

    #[test]
    fn projection_round_trips_to_within_a_pixel() {
        let samples = [
            (0.0, 0.0),
            (51.5074, -0.1278),
            (-33.8688, 151.2093),
            (64.1466, -21.9426),
            (-54.8019, -68.303),
        ];
        for (lat, lon) in samples {
            let coordinate = at(lat, lon);
            for zoom in [0u8, 3, 10, 18] {
                let point = project(coordinate, zoom).expect("mappable");
                let back = unproject(point, zoom).expect("a coordinate");
                // A pixel at zoom 18 is a few centimetres, so the tolerance is the projection's, not the test's.
                assert!(
                    (back.lat - lat).abs() < 1e-6 && (back.lon - lon).abs() < 1e-6,
                    "round trip at zoom {zoom} gave {back:?} for {lat},{lon}"
                );
            }
        }
    }

    #[test]
    fn photos_in_one_cell_become_one_dot_and_the_centre_is_their_middle() {
        let photos = vec![
            (PhotoId(1), at(51.500, -0.120)),
            (PhotoId(2), at(51.502, -0.118)),
            (PhotoId(3), at(51.504, -0.116)),
        ];
        let clusters = cluster(&photos, 12);
        assert_eq!(clusters.len(), 1, "a few metres apart is one dot at zoom 12");
        let cluster = &clusters[0];
        assert_eq!(cluster.len(), 3);
        assert_eq!(cluster.members, vec![PhotoId(1), PhotoId(2), PhotoId(3)]);
        assert!((cluster.centre.lat - 51.502).abs() < 1e-9);
        assert!((cluster.centre.lon - -0.118).abs() < 1e-9);
        assert_eq!(cluster.bounds, (51.504, 51.500, -0.116, -0.120));
        assert!(!cluster.is_single());
    }

    /// A gazetteer is read from a file, refused if it is missing or implausibly large, and tallied when it has holes.
    #[test]
    fn a_gazetteer_file_is_read_or_refused_with_a_reason() {
        let dir = std::env::temp_dir().join(format!("sortlens-gazetteer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");

        let good = dir.join("places.txt");
        std::fs::write(
            &good,
            "# name\tlat\tlon\tcountry\nBrighton\t51\t0\tGB\nLima\t-12\t-77\nbroken line\n\t\t\n",
        )
        .expect("write");
        let gazetteer = Gazetteer::load(&good).expect("loads");
        assert_eq!(gazetteer.len(), 2);
        // One malformed line, counted rather than silently dropped. The whitespace-only line is *blank*, not broken:
        // a line of tabs has no content to misread, so it belongs with the empty lines.
        assert_eq!(gazetteer.skipped(), 1, "the broken line is counted");
        let brighton = gazetteer
            .nearest(LatLon::new(50.8225, -0.1372).expect("place"), 30.0)
            .expect("Brighton is 20 km away");
        assert_eq!(brighton.name, "Brighton");
        assert_eq!(brighton.country.as_deref(), Some("GB"));

        let missing = Gazetteer::load(dir.join("absent.txt")).expect_err("no file, no gazetteer");
        assert!(matches!(missing, GazetteerError::Missing(_)));
        assert!(missing.message().contains("absent.txt"));

        // A file that is too large is refused before it is read: the whole point of the check is not to read it.
        let huge = dir.join("huge.txt");
        let file = std::fs::File::create(&huge).expect("create");
        file.set_len(MAX_GAZETTEER_BYTES + 1).expect("sparse");
        let error = Gazetteer::load(&huge).expect_err("too large");
        assert!(matches!(error, GazetteerError::TooLarge { .. }));
        assert!(error.message().contains("disk image"));

        // An empty file is a gazetteer with nothing in it, which is a state the caller can report, not an error.
        let empty = dir.join("empty.txt");
        std::fs::write(&empty, "").expect("write");
        let gazetteer = Gazetteer::load(&empty).expect("still a gazetteer");
        assert!(gazetteer.is_empty());
        assert_eq!(gazetteer.nearest(LatLon::new(0.0, 0.0).expect("place"), 100.0), None);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A viewport asks for exactly the tiles under it, in tile order, clipped to the world square.
    #[test]
    fn a_viewport_covers_the_tiles_under_it_and_nothing_else() {
        let brighton = LatLon::new(50.8225, -0.1372).expect("a place");
        // Zoom 0 is one tile wide, so a world-sized viewport is exactly that one tile.
        let origin = viewport_origin(0, brighton, 256.0, 256.0).expect("origin");
        assert_eq!(
            visible_tiles(0, origin, 256.0, 256.0),
            vec![TileCell { zoom: 0, x: 0, y: 0 }],
            "the whole world in one tile"
        );

        // A viewport of one tile at zoom 2 may straddle up to four, and always at least one.
        let origin = viewport_origin(2, brighton, 256.0, 256.0).expect("origin");
        let cells = visible_tiles(2, origin, 256.0, 256.0);
        assert!((1..=4).contains(&cells.len()), "got {} tiles", cells.len());
        for cell in &cells {
            assert_eq!(cell.zoom, 2);
            assert!(cell.x < 4 && cell.y < 4, "{cell:?} is inside the world");
            let rect = tile_rect(*cell, origin);
            // Every returned tile overlaps the viewport; none is a tile away.
            assert!(rect.x < 256.0 && rect.x + rect.w > 0.0, "{rect:?} is on screen horizontally");
            assert!(rect.y < 256.0 && rect.y + rect.h > 0.0, "{rect:?} is on screen vertically");
        }
        // Tile order is row-major, so a caller that paints in order paints bottom-up over nothing.
        let mut sorted = cells.clone();
        sorted.sort_by_key(|cell| (cell.y, cell.x));
        assert_eq!(cells, sorted);

        // A viewport larger than the world gets each world tile once, not a ring of repeats.
        let origin = viewport_origin(1, brighton, 4096.0, 4096.0).expect("origin");
        let cells = visible_tiles(1, origin, 4096.0, 4096.0);
        assert_eq!(cells.len(), 4, "zoom 1 is four tiles, and the world does not wrap");
        assert_eq!(cells.iter().filter(|cell| cell.x == 1 && cell.y == 0).count(), 1);

        // Nonsense input is no tiles, not a panic or one tile at the origin.
        assert!(visible_tiles(3, origin, 0.0, 100.0).is_empty());
        assert!(visible_tiles(3, Point { x: f64::NAN, y: 0.0 }, 100.0, 100.0).is_empty());
    }

    /// The screen position is the world position minus the origin, and a point off screen is not drawn.
    #[test]
    fn a_coordinate_lands_where_the_origin_says() {
        let centre = LatLon::new(50.8225, -0.1372).expect("a place");
        let zoom = 8;
        let (w, h) = (512.0, 512.0);
        let origin = viewport_origin(zoom, centre, w, h).expect("origin");
        // The centre of the viewport is the centre of the map.
        let screen = screen_position(centre, zoom, origin, w, h).expect("on screen");
        assert!((screen.x - w / 2.0).abs() < 1e-6, "got {}", screen.x);
        assert!((screen.y - h / 2.0).abs() < 1e-6, "got {}", screen.y);
        // A point a long way away is not on screen, so it is not returned as a dot at the edge.
        let sydney = LatLon::new(-33.8688, 151.2093).expect("a place");
        assert_eq!(screen_position(sydney, zoom, origin, w, h), None);
        // A coordinate Mercator cannot place is `None` even inside the viewport.
        let pole = LatLon::new(89.0, -0.1372).expect("a latitude");
        assert_eq!(screen_position(pole, zoom, origin, w, h), None);
    }

    /// A grid splits two close photos that happen to fall either side of a cell edge — stated, not discovered.
    ///
    /// This is what a grid clustering does, and it is the right trade for a map: the alternative (chaining neighbours
    /// until everything within a radius is one dot) is not deterministic from frame to frame, and a dot that jumps
    /// when a photo is added is worse than a dot that is occasionally in two pieces. The test exists so that the
    /// behaviour is a decision in the record rather than a surprise in a bug report.
    #[test]
    fn two_photos_either_side_of_a_cell_edge_are_two_dots() {
        // Zoom 0 is one 256-pixel tile, so a cell is a quarter of the world in each direction.
        // Find two coordinates just either side of the x boundary at CELL_PX.
        let boundary_lon = (CELL_PX / TILE) * 360.0 - 180.0;
        let left = at(0.0, boundary_lon - 0.001);
        let right = at(0.0, boundary_lon + 0.001);
        let clusters = cluster(&[(PhotoId(1), left), (PhotoId(2), right)], 0);
        assert_eq!(clusters.len(), 2, "a few hundred metres apart, but a cell apart");
        // A few kilometres the other way, well inside one cell, and they are one dot.
        let together = cluster(
            &[
                (PhotoId(3), at(0.0, boundary_lon - 1.0)),
                (PhotoId(4), at(0.0, boundary_lon - 1.001)),
            ],
            0,
        );
        assert_eq!(together.len(), 1);
    }

    /// The promise that matters for a map: the same photos, in any order, give the same dots in the same order.
    #[test]
    fn clustering_is_deterministic_whatever_order_the_photos_arrive_in() {
        let mut photos: Vec<(PhotoId, LatLon)> = Vec::new();
        for index in 0..40u64 {
            // Spread across several cells, with a deliberate crowd in one.
            let lat = 51.5 + (index % 8) as f64 * 0.05;
            let lon = -0.12 + (index % 5) as f64 * 0.07;
            photos.push((PhotoId(index + 1), at(lat, lon)));
        }
        let first = cluster(&photos, 10);
        let mut shuffled = photos.clone();
        shuffled.reverse();
        shuffled.swap(3, 17);
        let second = cluster(&shuffled, 10);
        assert_eq!(first.len(), second.len());
        assert_eq!(
            first.iter().map(|cluster| (cluster.cell, cluster.len())).collect::<Vec<_>>(),
            second.iter().map(|cluster| (cluster.cell, cluster.len())).collect::<Vec<_>>(),
            "the same cells in the same order"
        );
        assert!(first[0].len() >= first.last().map(Cluster::len).unwrap_or(0), "largest first");
    }

    #[test]
    fn a_photo_the_projection_cannot_place_is_left_off_the_map() {
        let photos = vec![(PhotoId(1), at(89.0, 0.0)), (PhotoId(2), at(51.5, -0.12))];
        let clusters = cluster(&photos, 6);
        assert_eq!(clusters.len(), 1, "the polar photo is not drawn at the top edge");
        assert_eq!(clusters[0].members, vec![PhotoId(2)]);
    }

    #[test]
    fn distance_is_the_great_circle_and_not_a_straight_line_in_degrees() {
        // London to Paris is about 344 km; a degree-count would say 3.5.
        let london = at(51.5074, -0.1278);
        let paris = at(48.8566, 2.3522);
        let distance = london.distance_km(paris);
        assert!((330.0..360.0).contains(&distance), "got {distance} km");
        assert_eq!(london.distance_km(london), 0.0);
        // Antipodal-ish: across the globe is roughly half the circumference.
        let far = at(-51.5074, 179.8722);
        assert!((london.distance_km(far) - 20_000.0).abs() < 500.0);
    }

    const GAZETTEER: &str = "\
# name\tlat\tlon\tcountry
Brighton\t51\t0\tGB
Hove\t51\t0\tGB
Trent Lakes\t45\t-78\tCA
Nowhere\t0\t0

broken line with too few fields
Not A Number\tx\t0
";

    #[test]
    fn a_gazetteer_reads_places_and_says_what_it_could_not_read() {
        let gazetteer = Gazetteer::parse(GAZETTEER);
        assert_eq!(gazetteer.len(), 4, "four readable places");
        assert_eq!(gazetteer.skipped(), 2, "two lines were not places");
        assert!(!gazetteer.is_empty());
        assert!(Gazetteer::parse("# only a comment\n").is_empty());
    }

    #[test]
    fn the_nearest_place_is_found_within_the_radius_or_not_at_all() {
        let gazetteer = Gazetteer::parse(GAZETTEER);
        // Just off Brighton, in the sea: the nearest recorded place is Brighton, alphabetically ahead of Hove.
        // A kilometre off the coast, within a thirty-kilometre radius.
        let place = gazetteer.nearest(at(51.01, 0.01), 30.0).expect("a place near Brighton");
        assert_eq!(place.name, "Brighton");
        assert_eq!(place.country.as_deref(), Some("GB"));

        // Nothing within a kilometre of a point in the middle of the ocean.
        assert_eq!(gazetteer.nearest(at(20.0, -40.0), 100.0), None);
        // And a radius of zero is not a radius.
        assert_eq!(gazetteer.nearest(at(51.0, 0.0), 0.0), None);
        assert_eq!(gazetteer.nearest(at(51.0, 0.0), f64::NAN), None);
    }

    #[test]
    fn a_tie_is_broken_by_name_so_file_order_does_not_decide() {
        let one = Gazetteer::parse("Hove\t51\t0\tGB\nBrighton\t51\t0\tGB\n");
        let other = Gazetteer::parse("Brighton\t51\t0\tGB\nHove\t51\t0\tGB\n");
        assert_eq!(
            one.nearest(at(51.0, 0.0), 5.0).map(|place| place.name.clone()),
            Some("Brighton".into())
        );
        assert_eq!(
            one.nearest(at(51.0, 0.0), 5.0),
            other.nearest(at(51.0, 0.0), 5.0),
            "the same file in another order answers the same"
        );
    }
}
