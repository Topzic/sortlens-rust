//! The application icon, drawn in code.
//!
//! There was an `assets/icon.svg` and nothing loaded it — the kind of asset that looks finished in a file
//! listing and is missing everywhere a user meets the app. This crate is the one place the mark is
//! defined: `rgba` renders it to pixels for the window, `svg` renders the same geometry for the committed
//! file, and `ico` and `icns` pack fresh renders of it into the committed Windows and macOS packaging
//! files (`cargo xtask icons` rewrites those two; `cargo xtask icons` without `--update` checks them).
//! A test holds all three committed files to the drawing, so neither can drift into a description of
//! something else.
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
    render(SIZE)
}

/// Render the mark at any square edge length, RGBA8, top row first.
///
/// The geometry is defined in fractions of the tile, so there is one drawing and every size is a
/// fresh render of it — not a resample of a larger raster, which is how a curve becomes a stair.
#[must_use]
pub fn render(size: usize) -> Vec<u8> {
    let shapes = mark();
    let mut pixels = vec![0_u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0.0_f64; 4];
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let px = (x as f64 + (sx as f64 + 0.5) / SAMPLES as f64) / size as f64;
                    let py = (y as f64 + (sy as f64 + 0.5) / SAMPLES as f64) / size as f64;
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
            let index = (y * size + x) * 4;
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

/// A PNG of the mark at any square edge length, for the slots inside `.ico` and `.icns` files.
#[must_use]
pub fn png_at(size: u32) -> Vec<u8> {
    png_of(size, size, &render(size as usize))
}

/// The square sizes a Windows `.ico` carries, smallest first.
const ICO_SIZES: &[u32] = &[16, 32, 48, 64, 128, 256];

/// The `.icns` chunks a macOS dock reads, PNG representations from 32 to 512.
const ICNS_SIZES: &[(&str, u32)] = &[("ic11", 32), ("ic12", 64), ("ic07", 128), ("ic08", 256), ("ic09", 512)];

/// A Windows `.ico`: PNG-encoded entries from 16 to 256 px, per the ICONDIR layout.
///
/// Every entry is PNG, which Windows Vista and every tool since reads directly; a full
/// bitmap copy of each size would multiply the file several times over for nothing. Offsets and
/// lengths are little-endian, the way the format was written in 1995.
#[must_use]
pub fn ico() -> Vec<u8> {
    let pngs: Vec<Vec<u8>> = ICO_SIZES.iter().map(|&size| png_at(size)).collect();
    let mut out = Vec::new();
    out.extend(0_u16.to_le_bytes()); // reserved
    out.extend(1_u16.to_le_bytes()); // type: an icon, not a cursor
    out.extend(u16::try_from(pngs.len()).unwrap_or(0).to_le_bytes());
    let mut offset = 6 + 16 * pngs.len();
    for (size, png) in ICO_SIZES.iter().zip(&pngs) {
        // 256 does not fit the one-byte edge field, and 0 is the spec's spelling of 256.
        let edge = u8::try_from(*size).unwrap_or(0);
        out.push(edge);
        out.push(edge);
        out.push(0); // colours in the palette: none
        out.push(0); // reserved
        out.extend(1_u16.to_le_bytes()); // planes
        out.extend(32_u16.to_le_bytes()); // bits per pixel
        out.extend(u32::try_from(png.len()).unwrap_or(0).to_le_bytes());
        out.extend(u32::try_from(offset).unwrap_or(0).to_le_bytes());
        offset += png.len();
    }
    for png in &pngs {
        out.extend_from_slice(png);
    }
    out
}

/// A macOS `.icns`: the `icns` header, then one PNG per slot from 32 to 512 px.
///
/// The chunk names pick the representations the Dock and Finder actually ask for — 32 and 64 are
/// the @2x slots for the 16 and 32 px list views, 128 and 256 the Finder icons, 512 the Get Info
/// and preview slot — and the container is big-endian, the way everything Apple wrote is.
#[must_use]
pub fn icns() -> Vec<u8> {
    let pngs: Vec<Vec<u8>> = ICNS_SIZES.iter().map(|&(_, size)| png_at(size)).collect();
    let body: u32 = pngs.iter().map(|png| png.len() as u32 + 8).sum();
    let mut out = Vec::new();
    out.extend_from_slice(b"icns");
    out.extend((body + 8).to_be_bytes());
    for ((name, _), png) in ICNS_SIZES.iter().zip(&pngs) {
        out.extend_from_slice(name.as_bytes());
        out.extend((png.len() as u32 + 8).to_be_bytes());
        out.extend_from_slice(png);
    }
    out
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

    /// The committed packaging icons are this drawing too, not files someone dropped in once.
    ///
    /// `assets/icon.ico` and `assets/icon.icns` are generated bytes, exactly like the SVG; the same
    /// env var rewrites them when the mark changes on purpose, and CI runs the test without it, so a
    /// stale packaging file fails the suite instead of shipping a mark nobody drew.
    #[test]
    fn the_committed_packaging_icons_are_the_drawing() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let update = std::env::var("SORTLENS_UPDATE_ICON").is_ok_and(|value| value == "1");
        for (name, bytes) in [("icon.ico", ico()), ("icon.icns", icns())] {
            let path = assets.join(name);
            if update {
                std::fs::write(&path, &bytes).unwrap_or_else(|error| panic!("rewrite {name}: {error}"));
            } else {
                let on_disk = std::fs::read(&path).unwrap_or_else(|error| panic!("read {name}: {error}"));
                assert_eq!(
                    on_disk, bytes,
                    "assets/{name} no longer matches the drawing: SORTLENS_UPDATE_ICON=1 cargo test -p sortlens-icon"
                );
            }
        }
    }

    /// The `.ico` parses as an ICONDIR whose entries are the sizes a Windows desktop asks for, and
    /// every embedded PNG decodes to exactly that square of the drawing.
    #[test]
    fn the_ico_carries_one_png_per_desktop_size() {
        let ico = ico();
        let u16_at = |offset: usize| u16::from_le_bytes([ico[offset], ico[offset + 1]]);
        assert_eq!(u16_at(0), 0, "reserved");
        assert_eq!(u16_at(2), 1, "type: icon");
        assert_eq!(u16_at(4), 6, "six entries");
        let mut offset = 6 + 16 * ICO_SIZES.len();
        for (entry, &size) in ICO_SIZES.iter().enumerate() {
            let base = 6 + entry * 16;
            let edge = ico[base];
            assert_eq!(edge, if size == 256 { 0 } else { size as u8 }, "edge byte for {size}");
            assert_eq!(ico[base + 1], edge, "square entry for {size}");
            assert_eq!(ico[base + 2], 0, "no palette for {size}");
            assert_eq!(ico[base + 3], 0, "reserved byte for {size}");
            assert_eq!(u16_at(base + 4), 1, "planes for {size}");
            assert_eq!(u16_at(base + 6), 32, "bits per pixel for {size}");
            let length = u32::from_le_bytes(ico[base + 8..base + 12].try_into().unwrap()) as usize;
            assert_eq!(
                u32::from_le_bytes(ico[base + 12..base + 16].try_into().unwrap()) as usize,
                offset,
                "the {size} px entry sits where the directory said"
            );
            let embedded = &ico[offset..offset + length];
            let thumbnail =
                sortlens_thumbs::decode_image(embedded, size).unwrap_or_else(|error| panic!("decode the {size} px entry: {error}"));
            assert_eq!(
                (thumbnail.width, thumbnail.height),
                (size, size),
                "the {size} px entry is that square"
            );
            assert!(thumbnail.is_consistent(), "the {size} px entry is a real image");
            // 40% of the tile is the front print at every size, clear of the lens (58%+) and of
            // every corner radius, so this one pixel proves the entry is the drawing, not noise.
            let at = (0.40 * f64::from(size)) as usize;
            let index = (at * size as usize + at) * 4;
            assert_eq!(
                &thumbnail.rgba[index..index + 4],
                &[242, 240, 235, 255],
                "the {size} px entry is the front print at 40% of the tile"
            );
            offset += length;
        }
        assert_eq!(offset, ico.len(), "nothing after the last entry");
    }

    /// The `.icns` parses as a header plus PNG chunks that add up to exactly the file, and each
    /// chunk decodes to the slot its name names.
    #[test]
    fn the_icns_carries_the_mac_slots() {
        let icns = icns();
        assert_eq!(&icns[0..4], b"icns");
        let total = u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize;
        assert_eq!(total, icns.len(), "the header's length is the file's length");
        let mut offset = 8;
        let mut seen = Vec::new();
        while offset < total {
            let name = std::str::from_utf8(&icns[offset..offset + 4]).unwrap();
            let length = u32::from_be_bytes(icns[offset + 4..offset + 8].try_into().unwrap()) as usize;
            assert!(length >= 8 && offset + length <= total, "{name} stays inside the file");
            let png = &icns[offset + 8..offset + length];
            let expected = ICNS_SIZES.iter().find(|(chunk, _)| *chunk == name).map(|&(_, size)| size).unwrap();
            let thumbnail = sortlens_thumbs::decode_image(png, expected).unwrap_or_else(|error| panic!("decode {name}: {error}"));
            assert_eq!(
                (thumbnail.width, thumbnail.height),
                (expected, expected),
                "{name} is the {expected} px slot"
            );
            assert!(thumbnail.is_consistent(), "{name} is a real image");
            let at = (0.40 * f64::from(expected)) as usize;
            let index = (at * expected as usize + at) * 4;
            assert_eq!(
                &thumbnail.rgba[index..index + 4],
                &[242, 240, 235, 255],
                "{name} is the front print at 40% of the tile"
            );
            seen.push(name.to_string());
            offset += length;
        }
        assert_eq!(seen, ICNS_SIZES.iter().map(|(name, _)| (*name).to_string()).collect::<Vec<_>>());
    }

    /// Two renders agree byte for byte, which is what makes the build script's cache honest.
    #[test]
    fn the_same_bytes_every_time() {
        assert_eq!(png(), png());
        assert_eq!(rgba(), rgba());
        assert_eq!(render(64), render(64), "a fresh size is as deterministic as the big one");
        assert_eq!(ico(), ico());
        assert_eq!(icns(), icns());
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
