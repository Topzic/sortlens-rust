//! A folder of map tiles, and the licence line that has to travel with it.
//!
//! The application ships no map data and fetches none: a basemap is a folder the person puts on their disk, in the
//! arrangement every tile archive already uses — `{z}/{x}/{y}.png`, which is what falls out of unzipping one.
//!
//! **The attribution is not optional.** A map without its licence line is not shippable, so a tile folder that does
//! not say where its tiles came from is refused outright rather than drawn with a blank corner: an unlabelled map
//! quietly implies the application made it. The refusal names the file to write, because a refusal that only says
//! "no" leaves the person to guess.
//!
//! Nothing here decodes an image or draws anything. It answers three questions — where is the tile for this cell, what
//! zooms are here, and who says so — and leaves the pixels to whoever is painting.

use std::path::{Path, PathBuf};

/// The file, in the tile folder, whose contents are printed wherever the map is shown.
pub const ATTRIBUTION_FILE: &str = "attribution.txt";

/// The deepest zoom a folder may claim.
///
/// A directory name is not automatically a zoom level: `255` parses as a `u8` and would be accepted as one, and a view
/// that offers zoom 255 offers a billion billion tiles that are not there. Real tile sets stop around 22; thirty is
/// deep enough for any archive and shallow enough that a typo is a typo.
pub const MAX_TILE_ZOOM: u8 = 30;

/// How much of that file is read. An attribution line is a sentence, not a document; the bound stops a stray
/// multi-gigabyte file in the tiles folder from being read into memory on a frame.
pub const MAX_ATTRIBUTION_BYTES: u64 = 4 * 1024;

/// Why a tile folder could not be used.
#[derive(Debug)]
pub enum TileError {
    /// There is no folder there.
    Missing(PathBuf),
    /// The folder has no attribution file.
    MissingAttribution(PathBuf),
    /// The attribution file is there and says nothing.
    EmptyAttribution(PathBuf),
    /// The attribution file could not be read.
    Unreadable { path: PathBuf, reason: String },
    /// The folder has an attribution but no zoom level of tiles.
    NoTiles(PathBuf),
}

impl TileError {
    /// What to tell the person who has to fix it.
    pub fn message(&self) -> String {
        match self {
            Self::Missing(path) => format!("there is no tile folder at {}", path.display()),
            Self::MissingAttribution(path) => format!(
                "{} has no {ATTRIBUTION_FILE}; write the tiles' credit and licence into it, because an \
                 unlabelled map implies the application made it",
                path.display()
            ),
            Self::EmptyAttribution(path) => {
                format!("{} is empty; a map needs its credit and licence written down", path.display())
            }
            Self::Unreadable { path, reason } => format!("could not read {}: {reason}", path.display()),
            Self::NoTiles(path) => format!(
                "{} has no {{z}}/{{x}}/{{y}}.png tiles in it, so there is nothing to draw",
                path.display()
            ),
        }
    }
}

impl std::fmt::Display for TileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for TileError {}

/// An open tile folder: the attribution, the zooms it has, and where a cell lives.
#[derive(Debug, Clone)]
pub struct Tileset {
    root: PathBuf,
    attribution: String,
    min_zoom: u8,
    max_zoom: u8,
}

impl Tileset {
    /// Open a tile folder, or explain exactly what is wrong with it.
    ///
    /// Both the attribution and at least one zoom level of tiles are required. A folder with tiles and no credit is
    /// refused, and so is a folder with a credit and no tiles: the first would draw a map the application appears to
    /// own, the second would promise a map and paint nothing.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, TileError> {
        let root = root.as_ref().to_path_buf();
        if !root.is_dir() {
            return Err(TileError::Missing(root));
        }
        let attribution_path = root.join(ATTRIBUTION_FILE);
        let attribution = match std::fs::read(&attribution_path) {
            Ok(bytes) => {
                let bytes = &bytes[..bytes.len().min(MAX_ATTRIBUTION_BYTES as usize)];
                String::from_utf8_lossy(bytes).trim().to_string()
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(TileError::MissingAttribution(root));
            }
            Err(error) => {
                return Err(TileError::Unreadable {
                    path: attribution_path,
                    reason: error.to_string(),
                });
            }
        };
        if attribution.is_empty() {
            return Err(TileError::EmptyAttribution(attribution_path));
        }
        let zooms = zoom_dirs(&root);
        let (Some(&min_zoom), Some(&max_zoom)) = (zooms.first(), zooms.last()) else {
            return Err(TileError::NoTiles(root));
        };
        Ok(Self {
            root,
            attribution,
            min_zoom,
            max_zoom,
        })
    }

    /// The tile folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The credit and licence, exactly as the folder's `attribution.txt` states them.
    pub fn attribution(&self) -> &str {
        &self.attribution
    }

    /// The lowest and highest zoom the folder actually has.
    pub fn zoom_range(&self) -> (u8, u8) {
        (self.min_zoom, self.max_zoom)
    }

    /// Whether a zoom level has tiles.
    ///
    /// The view asks rather than assumes, so zooming past the top of the set is a stated limit instead of a blank
    /// rectangle the person has to interpret.
    pub fn has_zoom(&self, zoom: u8) -> bool {
        self.root.join(zoom.to_string()).is_dir()
    }

    /// The zooms present, ascending.
    pub fn zooms(&self) -> Vec<u8> {
        zoom_dirs(&self.root)
    }

    /// The path of one tile, or `None` when the file is not there.
    ///
    /// Presence is checked once here rather than discovered as a decode failure per frame: a hole in the set is a hole
    /// to paint around, and asking the filesystem once is cheaper than a failed open every repaint.
    pub fn tile_path(&self, zoom: u8, x: u32, y: u32) -> Option<PathBuf> {
        if zoom > MAX_TILE_ZOOM {
            return None;
        }
        let path = self.root.join(zoom.to_string()).join(x.to_string()).join(format!("{y}.png"));
        path.is_file().then_some(path)
    }

    /// How many tile files the folder has at one zoom level, which is how the view knows the set is partial.
    pub fn tile_count(&self, zoom: u8) -> usize {
        let dir = self.root.join(zoom.to_string());
        let Ok(columns) = std::fs::read_dir(&dir) else {
            return 0;
        };
        columns
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .map(|entry| {
                std::fs::read_dir(entry.path())
                    .map(|rows| rows.flatten().filter(|row| row.path().is_file()).count())
                    .unwrap_or(0)
            })
            .sum()
    }
}

/// The zoom levels in a tile folder: directory names that are plain numbers, sorted.
fn zoom_dirs(root: &Path) -> Vec<u8> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut zooms: Vec<u8> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().to_str().and_then(|name| name.parse::<u8>().ok()))
        .filter(|zoom| *zoom <= MAX_TILE_ZOOM)
        .collect();
    zooms.sort_unstable();
    zooms.dedup();
    zooms
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// A tile folder with an attribution and one tile at one zoom, built on disk.
    fn write(name: &str, attribution: Option<&str>, zooms: &[u8]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("sortlens-tiles-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("root");
        if let Some(text) = attribution {
            std::fs::write(root.join(ATTRIBUTION_FILE), text).expect("attribution");
        }
        for zoom in zooms {
            let cell = root.join(zoom.to_string()).join("1");
            std::fs::create_dir_all(&cell).expect("cell");
            std::fs::write(cell.join("2.png"), b"not really a png").expect("tile");
        }
        root
    }

    #[test]
    fn a_folder_with_tiles_and_a_credit_opens_and_answers_questions() {
        let root = write("good", Some("© Example Maps, CC-BY 4.0\n"), &[3, 11]);
        let set = Tileset::open(&root).expect("opens");
        assert_eq!(
            set.attribution(),
            "© Example Maps, CC-BY 4.0",
            "the credit is kept, minus the trailing newline"
        );
        assert_eq!(set.zoom_range(), (3, 11), "the range is what is there, not what was hoped for");
        assert_eq!(set.zooms(), vec![3, 11]);
        assert!(set.has_zoom(11));
        assert!(!set.has_zoom(10), "a gap is a gap");
        assert!(set.tile_path(3, 1, 2).is_some());
        // A cell the set does not have is `None` rather than a path that will not open.
        assert!(set.tile_path(3, 1, 3).is_none());
        assert!(set.tile_path(4, 1, 2).is_none(), "no such zoom");
        assert_eq!(set.tile_count(3), 1);
        assert_eq!(set.tile_count(10), 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A map that does not say who made it is refused, and the refusal names the file to write.
    #[test]
    fn a_folder_of_tiles_without_a_credit_is_refused_and_says_what_to_do() {
        let root = write("nocardit", None, &[3]);
        let error = Tileset::open(&root).expect_err("no attribution, no map");
        assert!(matches!(error, TileError::MissingAttribution(_)));
        let message = error.message();
        assert!(message.contains(ATTRIBUTION_FILE), "names the file: {message}");
        assert!(message.contains("licence"), "says why: {message}");

        // Whitespace is not a credit.
        let blank = write("blank", Some("  \n\t\n"), &[3]);
        let error = Tileset::open(&blank).expect_err("a blank credit is no credit");
        assert!(matches!(error, TileError::EmptyAttribution(_)));
        assert!(error.message().contains("empty"));

        // A credit with no tiles is refused too: it would promise a map and paint nothing.
        let empty = write("creditonly", Some("© Example Maps"), &[]);
        let error = Tileset::open(&empty).expect_err("nothing to draw");
        assert!(matches!(error, TileError::NoTiles(_)));
        assert!(error.message().contains("z}/{x}/{y}"));

        let nowhere = std::env::temp_dir().join("sortlens-tiles-absent-not-created");
        assert!(matches!(Tileset::open(&nowhere), Err(TileError::Missing(_))));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A number that is not a zoom level is not a zoom level, and the file is only read as far as it is needed.
    #[test]
    fn stray_directories_are_not_zooms_and_a_huge_credit_is_bounded() {
        let root = write("stray", Some("x"), &[5]);
        std::fs::create_dir_all(root.join("tiles")).expect("dir");
        std::fs::create_dir_all(root.join("255")).expect("dir");
        std::fs::create_dir_all(root.join("256")).expect("dir");
        let set = Tileset::open(&root).expect("opens");
        // `tiles` is not a number, and `255` and `256` are numbers that cannot be zoom levels: a set claiming zoom 255
        // would let a view offer a billion billion tiles that are not there.
        assert_eq!(set.zoom_range(), (5, 5));
        assert!(set.tile_path(255, 0, 0).is_none());

        let huge = write("huge", None, &[1]);
        let long = "y".repeat(MAX_ATTRIBUTION_BYTES as usize * 2);
        std::fs::write(huge.join(ATTRIBUTION_FILE), &long).expect("write");
        let set = Tileset::open(&huge).expect("opens");
        assert_eq!(set.attribution().len(), MAX_ATTRIBUTION_BYTES as usize, "read up to the bound");

        // Bytes that are not valid UTF-8 are shown lossily rather than refusing the folder: a mangled credit is
        // still a credit, and refusing it would hide a map over a mis-encoded copyright symbol.
        let lossy = write("lossy", None, &[1]);
        std::fs::write(lossy.join(ATTRIBUTION_FILE), b"\xff\xfe Example").expect("write");
        let set = Tileset::open(&lossy).expect("opens");
        assert!(set.attribution().contains("Example"));

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&huge);
        let _ = std::fs::remove_dir_all(&lossy);
    }
}
