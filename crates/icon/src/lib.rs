//! The application icon, drawn in code.
//!
//! There was an `assets/icon.svg` and nothing loaded it — the kind of asset that looks finished in a file
//! listing and is missing everywhere a user meets the app. This crate is the one place the mark is
//! defined: `rgba` renders it to pixels for the window, `svg` renders the same geometry for the committed
//! file, and a test holds the two together, so neither can drift into a description of something else.
//!
//! No dependency and no compressor: the PNG writer emits stored deflate blocks, which is several hundred
//! kilobytes for one 512 px image but is byte-identical on every machine and crate version. An icon is
//! written once at build time, and determinism is worth more here than a small file.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

/// Edge length of the raster, big enough for the largest slot any desktop asks for.
pub const SIZE: usize = 512;

/// Samples per axis per pixel. Four is enough that a rounded corner reads as a curve at 32 px.
const SAMPLES: usize = 4;

/// The corner radius of the tile, as a fraction of its edge.
const TILE_RADIUS: f64 = 0.22;

/// One painted rectangle: position and size as fractions of the tile, then a colour.
type Shape = (f64, f64, f64, f64, [u8; 3]);

/// The mark: three stacked prints on a dark rounded tile, a red lens on the front one.
///
/// Coordinates are fractions of the tile so the raster and the SVG cannot disagree about where anything
/// is. The stack is the product in one picture: a library of frames, and the one you are deciding about on
/// top.
fn mark() -> Vec<Shape> {
    let tile = [28_u8, 28, 31];
    let mut shapes: Vec<Shape> = vec![(0.0, 0.0, 1.0, 1.0, tile)];
    shapes.push((0.20, 0.20, 0.52, 0.52, [126, 124, 120]));
    shapes.push((0.26, 0.26, 0.52, 0.52, [206, 203, 196]));
    shapes.push((0.32, 0.32, 0.52, 0.52, [242, 240, 235]));
    shapes.push((0.58, 0.58, 0.17, 0.17, [198, 46, 46]));
    shapes
}

/// A one-line description, used as the SVG's accessible name.
pub const DESCRIPTION: &str = "Sortlens icon: dark rounded tile, three stacked prints, red lens on the front one";

/// Render the mark to RGBA8, `SIZE × SIZE`, top row first.
#[must_use]
pub fn rgba() -> Vec<u8> {
    let shapes = mark();
    let mut pixels = vec![0_u8; SIZE * SIZE * 4];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let mut sum = [0.0_f64; 4];
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let px = (x as f64 + (sx as f64 + 0.5) / SAMPLES as f64) / SIZE as f64;
                    let py = (y as f64 + (sy as f64 + 0.5) / SAMPLES as f64) / SIZE as f64;
                    if let Some(colour) = sample(px, py, &shapes) {
                        sum[0] += f64::from(colour[0]);
                        sum[1] += f64::from(colour[1]);
                        sum[2] += f64::from(colour[2]);
                        sum[3] += 255.0;
                    }
                }
            }
            let samples = (SAMPLES * SAMPLES) as f64;
            let coverage = sum[3] / (255.0 * samples);
            let index = (y * SIZE + x) * 4;
            if coverage > 0.0 {
                // Average the colour over covered samples only, so a half-covered edge is not half dark.
                for channel in 0..3 {
                    pixels[index + channel] = (sum[channel] / (coverage * samples)).round().clamp(0.0, 255.0) as u8;
                }
            }
            pixels[index + 3] = (coverage * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
    pixels
}

/// The topmost shape covering a point, or `None` where the tile has been rounded away.
fn sample(x: f64, y: f64, shapes: &[Shape]) -> Option<[u8; 3]> {
    if !inside_rounded(x, y, 0.0, 0.0, 1.0, 1.0, TILE_RADIUS) {
        return None;
    }
    let mut colour = None;
    for (left, top, width, height, fill) in shapes {
        // The prints share the tile's corner language, a little tighter.
        if inside_rounded(x, y, *left, *top, *width, *height, TILE_RADIUS * 0.55) {
            colour = Some(*fill);
        }
    }
    colour
}

/// Whether a point is inside a rounded rectangle, with the radius clamped to half the short side.
fn inside_rounded(x: f64, y: f64, left: f64, top: f64, width: f64, height: f64, radius: f64) -> bool {
    if x < left || y < top || x > left + width || y > top + height {
        return false;
    }
    let radius = radius.min(width / 2.0).min(height / 2.0);
    if radius <= 0.0 {
        return true;
    }
    let cx = x.clamp(left + radius, left + width - radius);
    let cy = y.clamp(top + radius, top + height - radius);
    let (dx, dy) = (x - cx, y - cy);
    dx * dx + dy * dy <= radius * radius
}

/// The same mark as `assets/icon.svg`, for the file a human can open.
#[must_use]
pub fn svg() -> String {
    let mut svg = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\" role=\"img\" aria-label=\"{DESCRIPTION}\">\n");
    for (index, (left, top, width, height, fill)) in mark().iter().enumerate() {
        let radius = if index == 0 { TILE_RADIUS } else { TILE_RADIUS * 0.55 };
        svg.push_str(&format!(
            "  <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"{:.1}\" fill=\"#{:02x}{:02x}{:02x}\"/>\n",
            left * 100.0,
            top * 100.0,
            width * 100.0,
            height * 100.0,
            radius * 100.0,
            fill[0],
            fill[1],
            fill[2]
        ));
    }
    svg.push_str("</svg>\n");
    svg
}

/// A PNG of the rendered mark: RGBA8, one filter byte per row, a zlib stream of stored blocks.
#[must_use]
pub fn png() -> Vec<u8> {
    png_of(SIZE as u32, SIZE as u32, &rgba())
}

fn png_of(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(height as usize * (1 + width as usize * 4));
    for row in 0..height as usize {
        raw.push(0); // filter: none
        let start = row * width as usize * 4;
        raw.extend_from_slice(&pixels[start..start + width as usize * 4]);
    }
    let mut zlib = vec![0x78, 0x01];
    for (index, chunk) in raw.chunks(65_535).enumerate() {
        let last = (index + 1) * 65_535 >= raw.len();
        zlib.push(u8::from(last));
        let length = chunk.len() as u16;
        zlib.extend_from_slice(&length.to_le_bytes());
        zlib.extend_from_slice(&(!length).to_le_bytes());
        zlib.extend_from_slice(chunk);
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &zlib);
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 { 0xedb8_8320 ^ (crc >> 1) } else { crc >> 1 };
        }
    }
    crc ^ 0xffff_ffff
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1_u32, 0_u32);
    for byte in bytes {
        a = (a + u32::from(*byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;

    /// The committed SVG is this drawing, not a file someone edited once.
    ///
    /// The window icon is generated at build time from `rgba()`, so the SVG is the only copy on disk that
    /// could go stale — and the way to find out is to compare it, not to trust that it was regenerated.
    /// `SORTLENS_UPDATE_ICON=1` rewrites the file when the mark changes on purpose; CI runs it without.
    #[test]
    fn the_committed_svg_is_the_drawing() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/icon.svg");
        if std::env::var("SORTLENS_UPDATE_ICON").is_ok_and(|value| value == "1") {
            std::fs::write(&path, svg()).expect("rewrite assets/icon.svg");
            return;
        }
        let on_disk = std::fs::read_to_string(&path).expect("assets/icon.svg");
        assert_eq!(
            on_disk,
            svg(),
            "assets/icon.svg no longer matches the drawing: SORTLENS_UPDATE_ICON=1 cargo test -p sortlens-icon"
        );
    }

    /// The PNG really is a PNG, and a decoder that was not written for it can read it.
    #[test]
    fn the_window_icon_decodes_to_the_mark() {
        let thumbnail = sortlens_thumbs::decode_image(&png(), SIZE as u32).expect("decode");
        assert_eq!((thumbnail.width, thumbnail.height), (SIZE as u32, SIZE as u32));
        assert!(thumbnail.is_consistent());
        let pixel = |x: usize, y: usize| {
            let index = (y * SIZE + x) * 4;
            [
                thumbnail.rgba[index],
                thumbnail.rgba[index + 1],
                thumbnail.rgba[index + 2],
                thumbnail.rgba[index + 3],
            ]
        };
        assert_eq!(pixel(2, 2)[3], 0, "the rounded corner is transparent");
        assert_eq!(pixel(SIZE / 2, SIZE / 2), [242, 240, 235, 255], "the front print");
        // The lens sits at 58% + half of 17%, which is where the red has to be.
        let lens = (0.665 * SIZE as f64) as usize;
        assert_eq!(pixel(lens, lens), [198, 46, 46, 255], "the lens");
    }

    /// Two renders agree byte for byte, which is what makes the build script's cache honest.
    #[test]
    fn the_same_bytes_every_time() {
        assert_eq!(png(), png());
        assert_eq!(rgba(), rgba());
    }

    /// A square, opaque in the middle, transparent at the corners: the shape, not just a colour.
    #[test]
    fn the_icon_is_square_and_rounded() {
        let pixels = rgba();
        assert_eq!(pixels.len(), SIZE * SIZE * 4);
        let alpha = |x: usize, y: usize| pixels[(y * SIZE + x) * 4 + 3];
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(SIZE - 1, SIZE - 1), 0);
        assert_eq!(alpha(SIZE / 2, 4), 255, "the top edge, away from the corners");
    }
}
