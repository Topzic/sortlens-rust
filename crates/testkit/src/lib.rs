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

/// How a fixture JPEG's pixels are made.
///
/// A solid colour is the cheapest fixture, and [`Pattern::Gradient`] exists because the similarity pass
/// reads structure: every solid-colour frame hashes the same, so a library of them is one enormous
/// group. A gradient with a distinct phase is a distinct structure, and two frames with the same phase
/// and size are byte-identical — which is how a duplicate pair is made on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    /// One colour everywhere.
    Solid([u8; 3]),
    /// A diagonal ramp, shifted by `phase`, so two phases are visibly different pictures.
    Gradient(u8),
}

impl Pattern {
    /// The RGB pixels, row-major: the same pattern and size always make the same bytes.
    #[must_use]
    pub fn pixels(self, width: usize, height: usize) -> Vec<u8> {
        let mut pixels = Vec::with_capacity(width * height * 3);
        for y in 0..height {
            for x in 0..width {
                match self {
                    Pattern::Solid(colour) => pixels.extend_from_slice(&colour),
                    Pattern::Gradient(phase) => {
                        let ramp = ((x * 255 / width.max(1)) + (y * 255 / height.max(1))) as u32;
                        let shift = u32::from(phase) * 7;
                        pixels.push(((ramp + shift) % 256) as u8);
                        pixels.push((ramp * 2 % 256) as u8);
                        pixels.push((255 + shift * 3 - ramp.min(255)) as u8);
                    }
                }
            }
        }
        pixels
    }
}

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
        Self::at(std::env::temp_dir().join(format!("sortlens-{name}-{}-{tick}", std::process::id())))
    }

    /// Create an empty fixture at a path the caller names, cleared first.
    ///
    /// [`Self::new`] puts the process id in the name so parallel tests cannot collide. A screenshot
    /// baseline needs the opposite: a path that reads the same on every run, because the UI prints it
    /// and the picture is compared byte for byte. Such a caller names its own path here — a relative
    /// one stays relative in everything the app displays.
    #[must_use]
    pub fn at(dir: impl Into<PathBuf>) -> Fixture {
        let dir = dir.into();
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
        self.encode(
            rel,
            Pattern::Solid(colour).pixels(width, height),
            width,
            height,
            &[(0xE1, {
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
                payload
            })],
        )
    }

    /// Write a JPEG carrying the metadata a camera and an editor write: pixel dimensions in XMP, and
    /// a location in EXIF when one is given.
    ///
    /// `jpeg` writes no metadata at all, so a fixture made with it indexes as 0 × 0 and loses its aspect
    /// ratio in the justified grid. The dimensions go in an XMP packet because that is where an editor
    /// records them and where the suite's EXIF writer does not; the location goes through the EXIF block
    /// [`Self::jpeg_with_location`] already uses. Both travel the path the app reads, so the header and the
    /// encoded pixels cannot disagree.
    pub fn jpeg_with_pattern(&self, rel: &str, width: usize, height: usize, pattern: Pattern, place: Option<(f64, f64)>) -> PathBuf {
        let mut segments: Vec<(u8, Vec<u8>)> = Vec::new();
        if let Some((latitude, longitude)) = place {
            let metadata = craft_metadata::Metadata {
                gps: Some(craft_metadata::Gps {
                    latitude,
                    longitude,
                    altitude: Some(0.0),
                }),
                ..Default::default()
            };
            let mut payload = b"Exif\0\0".to_vec();
            payload.extend_from_slice(&craft_metadata::write_exif(&metadata));
            segments.push((0xE1, payload));
        }
        let packet = format!(
            "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
             xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" \
             xmlns:tiff=\"http://ns.adobe.com/tiff/1.0/\" tiff:ImageWidth=\"{width}\" \
             tiff:ImageLength=\"{height}\"/></rdf:RDF></x:xmpmeta>"
        );
        let mut xmp = b"http://ns.adobe.com/xap/1.0/\0".to_vec();
        xmp.extend_from_slice(packet.as_bytes());
        segments.push((0xE1, xmp));
        self.encode(rel, pattern.pixels(width, height), width, height, &segments)
    }

    /// Encode a JPEG from already-built RGB pixels with the given APP segments.
    fn encode(&self, rel: &str, pixels: Vec<u8>, width: usize, height: usize, segments: &[(u8, Vec<u8>)]) -> PathBuf {
        let encoded = craft_jpeg::encode_baseline(&pixels, width, height, 3, QUALITY, craft_jpeg::ChromaSubsampling::S444, segments);
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

    /// A gradient is deterministic, two phases differ, and a repeated phase repeats byte for byte — the
    /// property the duplicate-pair fixture depends on.
    #[test]
    fn a_gradient_pattern_is_stable_and_phase_scoped() {
        let first = Fixture::new("gradient");
        let second = Fixture::new("gradient");
        let a = first.jpeg_with_pattern("a.jpg", 24, 16, Pattern::Gradient(3), None);
        let b = second.jpeg_with_pattern("a.jpg", 24, 16, Pattern::Gradient(3), None);
        assert_eq!(fs::read(&a).ok(), fs::read(&b).ok(), "same phase, same bytes");

        let other = first.jpeg_with_pattern("b.jpg", 24, 16, Pattern::Gradient(9), None);
        assert_ne!(fs::read(&a).ok(), fs::read(&other).ok(), "a different phase is a different file");
        assert_eq!(Pattern::Gradient(3).pixels(4, 4).len(), 4 * 4 * 3);
        assert_ne!(Pattern::Gradient(1).pixels(4, 4), Pattern::Gradient(2).pixels(4, 4));
    }

    /// The dimensions a fixture advertises are in a place the app's reader actually finds them, so a
    /// justified grid built from these files has real aspect ratios.
    #[test]
    fn a_patterned_fixture_carries_readable_dimensions_and_location() {
        let fixture = Fixture::new("dimensions");
        let path = fixture.jpeg_with_pattern("a.jpg", 120, 60, Pattern::Gradient(2), Some((50.82, -0.13)));
        let bytes = fs::read(path).ok();
        assert!(bytes.is_some(), "the fixture wrote a file");
        let Some(bytes) = bytes else { return };
        let metadata = craft_metadata::extract(&bytes);
        assert_eq!((metadata.width, metadata.height), (Some(120), Some(60)));
        let gps = metadata.gps;
        assert!(gps.is_some(), "the location survives");
        let Some(gps) = gps else { return };
        assert!((gps.latitude - 50.82).abs() < 0.01 && (gps.longitude + 0.13).abs() < 0.01);
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
