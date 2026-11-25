//! An MBTiles archive: the same tiles, in one SQLite file instead of a folder.
//!
//! MBTiles is what most tile downloaders and QGIS exports write: a SQLite database whose `tiles` table holds
//! `(zoom_level, tile_column, tile_row, tile_data)` rows and whose `metadata` table holds the set's name, format and —
//! the one this crate will not do without — its **attribution**. The archive is read and never written, so the file
//! can be shared with whatever tool made it.
//!
//! **`tile_row` is TMS, not XYZ.** MBTiles numbers rows from the south edge of the world and every tile scheme this
//! project otherwise uses numbers them from the north, so a lookup converts: `row = 2^z - 1 - y`. Getting this wrong
//! draws every tile upside down and still looks like a map, which is exactly the kind of bug a screenshot review
//! misses, so the conversion is pinned by a test that stores one tile and reads it back through the view's own row.
//!
//! **The credit is still not optional.** An archive whose `metadata` has no `attribution` row is refused unless an
//! `attribution.txt` sits beside it, the same file a tile folder requires. One rule for both sources means a map
//! cannot be drawn from a file that forgets to say whose data it is.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension};

use crate::tiles::{ATTRIBUTION_FILE, MAX_ATTRIBUTION_BYTES, MAX_TILE_BYTES, MAX_TILE_ZOOM, TileError};

/// The archive's name inside a library. One name, beside the catalog, for the same reason the tile folder is:
/// a basemap belongs to its library and moves with it.
pub const MBTILES_FILE: &str = "tiles.mbtiles";

/// One tile, bounded so a row that is not an image cannot be read into memory.
const TILE_SQL: &str =
    "SELECT tile_data FROM tiles WHERE zoom_level = ?1 AND tile_column = ?2 AND tile_row = ?3 AND length(tile_data) <= ?4";

/// An open MBTiles archive: where it is, who made it, and which zooms it holds.
#[derive(Debug, Clone)]
pub struct Mbtiles {
    path: PathBuf,
    attribution: String,
    zooms: Vec<u8>,
}

impl Mbtiles {
    /// Open an archive, or explain exactly what is wrong with it.
    ///
    /// The credit is read from `metadata.attribution` and, failing that, from an `attribution.txt` beside the file.
    /// Both absent is a refusal: a usable map whose licence is unknown is the state this type exists to prevent.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, TileError> {
        let path = path.as_ref().to_path_buf();
        if !path.is_file() {
            return Err(TileError::Missing(path));
        }
        let connection = connect(&path)?;
        let attribution = archive_attribution(&connection, &path)?;
        let zooms = archive_zooms(&connection, &path)?;
        Ok(Self { path, attribution, zooms })
    }

    /// The archive file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The credit and licence, from `metadata.attribution` or the `attribution.txt` beside it.
    pub fn attribution(&self) -> &str {
        &self.attribution
    }

    /// The lowest and highest zoom the archive holds.
    pub fn zoom_range(&self) -> (u8, u8) {
        (self.zooms[0], self.zooms[self.zooms.len() - 1])
    }

    /// The zooms the archive holds, ascending.
    pub fn zooms(&self) -> Vec<u8> {
        self.zooms.clone()
    }

    /// Whether the archive has tiles at one zoom.
    pub fn has_zoom(&self, zoom: u8) -> bool {
        self.zooms.contains(&zoom)
    }

    /// How many tiles the archive holds at one zoom.
    pub fn tile_count(&self, zoom: u8) -> usize {
        if !self.has_zoom(zoom) {
            return 0;
        }
        let Ok(connection) = connect(&self.path) else {
            return 0;
        };
        connection
            .query_row("SELECT COUNT(*) FROM tiles WHERE zoom_level = ?1", [i64::from(zoom)], |row| {
                row.get::<_, i64>(0)
            })
            .ok()
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or(0)
    }

    /// One tile's bytes at an XYZ cell, or `None` when the archive has no such tile.
    ///
    /// The row is converted from XYZ to TMS here and nowhere else. A blob larger than [`MAX_TILE_BYTES`] is treated
    /// as a hole rather than decoded: a tile is a 256-pixel image, and a gigabyte row in a file somebody downloaded
    /// is not one.
    pub fn tile(&self, zoom: u8, x: u32, y: u32) -> Option<Vec<u8>> {
        if zoom > MAX_TILE_ZOOM || !self.has_zoom(zoom) {
            return None;
        }
        let row = xyz_to_tms(zoom, y)?;
        let bound = i64::try_from(MAX_TILE_BYTES).unwrap_or(i64::MAX);
        let connection = connect(&self.path).ok()?;
        connection
            .query_row(TILE_SQL, rusqlite::params![i64::from(zoom), i64::from(x), row, bound], |row| {
                row.get::<_, Vec<u8>>(0)
            })
            .optional()
            .ok()
            .flatten()
    }

    /// Whether the archive has this cell, asked before decoding.
    pub fn contains(&self, zoom: u8, x: u32, y: u32) -> bool {
        self.tile(zoom, x, y).is_some()
    }
}

/// `y`, counted from the north, as the row MBTiles counts from the south.
///
/// `None` when the cell is outside the world at that zoom: a tile row that cannot exist is not looked up, and a
/// subtraction with nothing under it would look like the south pole's row.
pub fn xyz_to_tms(zoom: u8, y: u32) -> Option<i64> {
    if zoom > MAX_TILE_ZOOM {
        return None;
    }
    let side = 1i64 << zoom;
    let y = i64::from(y);
    (y < side).then(|| side - 1 - y)
}

/// A read-only connection. Nothing in this crate can write an archive its owner downloaded.
///
/// The header is proved before the connection is handed back: SQLite opens a file lazily, so a text file renamed to
/// `.mbtiles` fails at its first query — and the first query here is attribution, whose absence would be reported as a
/// missing credit rather than as the wrong file.
fn connect(path: &Path) -> Result<Connection, TileError> {
    let connection =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(|error| {
            TileError::NotAnArchive {
                path: path.to_path_buf(),
                reason: error.to_string(),
            }
        })?;
    connection
        .query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
        .map_err(|error| TileError::NotAnArchive {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
    Ok(connection)
}

/// The credit: `metadata.attribution` first, then `attribution.txt` beside the file.
fn archive_attribution(connection: &Connection, path: &Path) -> Result<String, TileError> {
    let bound = i64::try_from(MAX_ATTRIBUTION_BYTES).unwrap_or(i64::MAX);
    let stored: Option<String> = connection
        .query_row(
            "SELECT value FROM metadata WHERE name = 'attribution' AND length(value) <= ?1",
            [bound],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or(None);
    if let Some(text) = stored.map(|text| text.trim().to_string()).filter(|text| !text.is_empty()) {
        return Ok(text);
    }
    let beside = path.with_file_name(ATTRIBUTION_FILE);
    let Ok(bytes) = std::fs::read(&beside) else {
        return Err(TileError::MissingArchiveAttribution(path.to_path_buf()));
    };
    let bytes = &bytes[..bytes.len().min(MAX_ATTRIBUTION_BYTES as usize)];
    let text = String::from_utf8_lossy(bytes).trim().to_string();
    if text.is_empty() {
        return Err(TileError::EmptyAttribution(beside));
    }
    Ok(text)
}

/// The zooms the `tiles` table holds, bounded by what a zoom level can be.
fn archive_zooms(connection: &Connection, path: &Path) -> Result<Vec<u8>, TileError> {
    let mut statement = connection
        .prepare("SELECT DISTINCT zoom_level FROM tiles ORDER BY zoom_level")
        .map_err(|error| TileError::NotAnArchive {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
    let rows = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|error| TileError::NotAnArchive {
            path: path.to_path_buf(),
            reason: error.to_string(),
        })?;
    let mut zooms = Vec::new();
    for row in rows {
        let Ok(zoom) = row else {
            continue;
        };
        if let Ok(zoom) = u8::try_from(zoom)
            && zoom <= MAX_TILE_ZOOM
        {
            zooms.push(zoom);
        }
    }
    zooms.sort_unstable();
    zooms.dedup();
    if zooms.is_empty() {
        return Err(TileError::EmptyArchive(path.to_path_buf()));
    }
    Ok(zooms)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// An archive with one tile: XYZ (3, 1, 2), stored as TMS row 5, and a metadata attribution when asked.
    fn write(name: &str, attribution: Option<&str>) -> PathBuf {
        let root = std::env::temp_dir().join(format!("sortlens-mbtiles-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        let path = root.join(MBTILES_FILE);
        let connection = Connection::open(&path).expect("database");
        connection.execute_batch(SCHEMA).expect("schema");
        if let Some(text) = attribution {
            connection
                .execute("INSERT INTO metadata (name, value) VALUES ('attribution', ?1)", [text])
                .expect("metadata");
        }
        connection.execute(TILE_INSERT, [b"png bytes".as_slice()]).expect("tile");
        path
    }

    const SCHEMA: &str = "CREATE TABLE metadata (name TEXT, value TEXT); CREATE TABLE tiles (zoom_level INTEGER, tile_column INTEGER, tile_row INTEGER, tile_data BLOB); CREATE UNIQUE INDEX tile_index ON tiles (zoom_level, tile_column, tile_row);";
    const TILE_INSERT: &str = "INSERT INTO tiles (zoom_level, tile_column, tile_row, tile_data) VALUES (3, 1, 5, ?1)";

    #[test]
    fn an_archive_answers_the_cell_the_view_asks_for_by_flipping_the_row() {
        let path = write("good", Some("© Example Maps, CC-BY 4.0\n"));
        let archive = Mbtiles::open(&path).expect("opens");
        assert_eq!(archive.attribution(), "© Example Maps, CC-BY 4.0");
        assert_eq!(archive.zoom_range(), (3, 3));
        assert_eq!(archive.zooms(), vec![3]);
        assert_eq!(archive.tile_count(3), 1);
        assert_eq!(archive.tile_count(2), 0);
        // Stored as TMS row 5, which is XYZ y = 2 at zoom 3: the flip is what the view's row means.
        assert_eq!(archive.tile(3, 1, 2), Some(b"png bytes".to_vec()));
        assert_eq!(archive.tile(3, 1, 5), None, "the TMS row is not an XYZ row");
        assert_eq!(archive.tile(2, 1, 2), None, "no such zoom");
        assert_eq!(archive.tile(30, 0, 0), None, "nothing at the top");
        let _ = std::fs::remove_dir_all(path.parent().expect("parent"));
    }

    #[test]
    fn an_archive_without_a_credit_is_refused_and_a_file_beside_it_is_the_credit() {
        let path = write("noattribution", None);
        let error = Mbtiles::open(&path).expect_err("no credit, no map");
        assert!(matches!(error, TileError::MissingArchiveAttribution(_)));
        let message = error.message();
        assert!(message.contains("attribution"), "names the row: {message}");
        assert!(message.contains(ATTRIBUTION_FILE), "names the file beside it: {message}");

        // The same credit file a tile folder needs is accepted beside the archive.
        let root = path.parent().expect("parent");
        std::fs::write(root.join(ATTRIBUTION_FILE), "© Another Map\n").expect("write");
        let archive = Mbtiles::open(&path).expect("opens with the file's credit");
        assert_eq!(archive.attribution(), "© Another Map");

        // A blank file is not a credit.
        std::fs::write(root.join(ATTRIBUTION_FILE), "  \n\t\n").expect("write");
        let error = Mbtiles::open(&path).expect_err("a blank credit is no credit");
        assert!(matches!(error, TileError::EmptyAttribution(_)));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_file_that_is_not_a_database_and_an_archive_with_no_tiles_are_refused_with_their_reason() {
        let root = std::env::temp_dir().join(format!("sortlens-mbtiles-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        let not_a_db = root.join("tiles.mbtiles");
        std::fs::write(&not_a_db, b"this is a text file, not a database").expect("write");
        let error = Mbtiles::open(&not_a_db).expect_err("not a database");
        assert!(matches!(error, TileError::NotAnArchive { .. }));
        assert!(error.message().contains("not an MBTiles archive"), "{}", error.message());

        // A database with the right tables and no rows is a map that would paint nothing.
        let empty = root.join("empty.mbtiles");
        let connection = Connection::open(&empty).expect("database");
        connection.execute_batch(SCHEMA).expect("schema");
        connection
            .execute("INSERT INTO metadata (name, value) VALUES ('attribution', 'x')", [])
            .expect("metadata");
        drop(connection);
        let error = Mbtiles::open(&empty).expect_err("nothing to draw");
        assert!(matches!(error, TileError::EmptyArchive(_)));

        let missing = root.join("absent.mbtiles");
        assert!(matches!(Mbtiles::open(&missing), Err(TileError::Missing(_))));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_row_conversion_is_xyz_from_the_north_and_refuses_a_row_past_the_world() {
        assert_eq!(xyz_to_tms(0, 0), Some(0));
        assert_eq!(xyz_to_tms(1, 0), Some(1));
        assert_eq!(xyz_to_tms(1, 1), Some(0));
        assert_eq!(xyz_to_tms(3, 2), Some(5));
        assert_eq!(xyz_to_tms(3, 8), None, "a row past the world is not the south edge");
        assert_eq!(xyz_to_tms(31, 0), None, "zooms stop at 30");
    }
}
