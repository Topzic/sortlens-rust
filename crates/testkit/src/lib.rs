//! Synthetic libraries, fixtures and assertion helpers for tests.
//!
//! Everything here is deterministic on purpose: the golden-output job in `apps/sortlens-cli` diffs
//! what the engine reports, and a fixture that produced different bytes on every run would make that
//! diff meaningless. The image bytes come from a real JPEG encoder, so a fixture photo behaves like a
//! photo — it decodes, it hashes, it gets a resolution — without shipping binary assets.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

pub mod onnx;

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Quality used for fixture JPEGs. Fixed, because changing it changes every content key.
pub const QUALITY: u8 = 90;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A temporary folder tree on disk, removed when dropped.
///
/// The name carries a counter and the process id, so two tests running at once cannot share a folder;
/// the *contents* are what stays deterministic.
#[derive(Debug)]
pub struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    /// Create an empty named fixture folder.
    #[must_use]
    pub fn new(name: &str) -> Fixture {
        let tick = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("sortlens-{name}-{}-{tick}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);
        Fixture { dir }
    }

    /// The fixture root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.dir
    }

    /// Write a solid-colour JPEG at `rel`, returning its path.
    ///
    /// A solid colour keeps the file tiny and its content key stable; the colour is what tells two
    /// fixtures apart in a duplicate-search test.
    pub fn jpeg(&self, rel: &str, colour: [u8; 3], width: usize, height: usize) -> PathBuf {
        let pixels: Vec<u8> = (0..width * height).flat_map(|_| [colour[0], colour[1], colour[2]]).collect();
        let encoded = craft_jpeg::encode_baseline(&pixels, width, height, 3, QUALITY, craft_jpeg::ChromaSubsampling::S444, &[]);
        // A fixture that cannot be encoded is a broken fixture; the helpers are for tests, and a
        // silent empty file would make the test assert nothing. `encode_baseline` is fed valid input.
        let bytes = match encoded {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("fixture: could not encode {rel}: {error:?}");
                return self.dir.join(rel);
            }
        };
        self.write(rel, &bytes)
    }

    /// Write a solid-colour JPEG that carries a GPS block at `rel`, returning its path.
    ///
    /// The EXIF is written by the same library the application reads, and embedded as a JPEG APP1 segment, so a
    /// fixture travels the whole path a camera's file travels: JPEG → EXIF → degrees/minutes/seconds → a coordinate.
    /// `altitude` is written as a positive height above sea level, which is what a camera with a fix records.
    pub fn jpeg_with_location(&self, rel: &str, colour: [u8; 3], width: usize, height: usize, lat: f64, lon: f64) -> PathBuf {
        let metadata = craft_metadata::Metadata {
            gps: Some(craft_metadata::Gps {
                latitude: lat,
                longitude: lon,
                altitude: Some(0.0),
            }),
            ..Default::default()
        };
        let mut payload = b"Exif\0\0".to_vec();
        payload.extend_from_slice(&craft_metadata::write_exif(&metadata));
        let pixels: Vec<u8> = (0..width * height).flat_map(|_| [colour[0], colour[1], colour[2]]).collect();
        let encoded = craft_jpeg::encode_baseline(
            &pixels,
            width,
            height,
            3,
            QUALITY,
            craft_jpeg::ChromaSubsampling::S444,
            &[(0xE1, payload)],
        );
        let bytes = match encoded {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("fixture: could not encode {rel}: {error:?}");
                return self.dir.join(rel);
            }
        };
        self.write(rel, &bytes)
    }

    /// Write arbitrary bytes at `rel`, returning its path.
    pub fn write(&self, rel: &str, bytes: &[u8]) -> PathBuf {
        let path = self.dir.join(rel);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, bytes);
        path
    }

    /// Write an XMP sidecar for `photo_rel`, named the convention way.
    ///
    /// The sidecar replaces the photo's extension (`IMG_1234.CR3` -> `IMG_1234.xmp`), which is the
    /// rule in `docs/BEHAVIOUR.md` §4 — not `IMG_1234.CR3.xmp`.
    pub fn sidecar(&self, photo_rel: &str, xml: &str) -> PathBuf {
        let stem = photo_rel.rsplit_once('.').map_or(photo_rel, |(stem, _)| stem);
        self.write(&format!("{stem}.xmp"), xml.as_bytes())
    }

    /// A path inside the fixture, without creating anything.
    #[must_use]
    pub fn path(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_fixture_writes_the_same_bytes_twice() {
        let first = Fixture::new("same");
        let second = Fixture::new("same");
        let a = first.jpeg("a.jpg", [200, 30, 40], 16, 12);
        let b = second.jpeg("a.jpg", [200, 30, 40], 16, 12);
        let one = fs::read(a);
        let two = fs::read(b);
        assert_eq!(one.ok(), two.ok());
    }

    #[test]
    fn a_different_colour_is_a_different_file() {
        let fixture = Fixture::new("colour");
        let red = fs::read(fixture.jpeg("red.jpg", [220, 20, 20], 8, 8));
        let blue = fs::read(fixture.jpeg("blue.jpg", [20, 20, 220], 8, 8));
        assert_ne!(red.ok(), blue.ok());
    }

    #[test]
    fn a_sidecar_replaces_the_extension() {
        let fixture = Fixture::new("sidecar");
        let sidecar = fixture.sidecar("Trip/a.jpg", "<x/>");
        assert!(sidecar.ends_with("Trip/a.xmp"));
        assert_eq!(fs::read_to_string(sidecar).ok().as_deref(), Some("<x/>"));
    }

    #[test]
    fn a_fixture_folder_is_removed_when_it_goes_out_of_scope() {
        let path = {
            let fixture = Fixture::new("gone");
            fixture.jpeg("a.jpg", [10, 10, 10], 4, 4);
            fixture.root().to_path_buf()
        };
        assert!(!path.exists());
    }
}
