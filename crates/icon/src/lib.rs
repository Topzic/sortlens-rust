//! The application icon.
//!
//! The mark is the engraved aperture in [`MARK`]: one mountain seen through the blades, on an indigo
//! tile. `rgba` is those pixels, `png` and `svg` are the same pixels for the window and for the file a
//! person can open, and `ico` and `icns` pack fresh reductions of them into the committed packaging
//! files (`cargo xtask icons` rewrites those two; without `--update` it checks them). A test holds the
//! committed files to these bytes, so none of them can drift into a different picture.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

/// Edge length of the master raster, big enough for the largest slot any desktop asks for.
pub const SIZE: usize = 512;

/// The engraved aperture, RGBA8, [`SIZE`] on a side, top row first. The rounded corners are
/// transparent; everything else is the tile.
const MARK: &[u8] = include_bytes!("mark.rgba");

/// A one-line description, used as the SVG's accessible name.
pub const DESCRIPTION: &str = "Sortlens icon: an engraved aperture on an indigo tile, one mountain in the opening";

/// The mark at [`SIZE`], RGBA8, top row first.
#[must_use]
pub fn rgba() -> Vec<u8> {
    MARK.to_vec()
}

/// The mark at any square edge length, RGBA8, top row first.
///
/// Smaller sizes are an area average of the master, so a 16 px tile is the same picture as the
/// window icon and not a second drawing.
#[must_use]
pub fn render(size: usize) -> Vec<u8> {
    if size == 0 {
        return Vec::new();
    }
    if size == SIZE {
        return rgba();
    }
    let mut out = vec![0_u8; size * size * 4];
    for y in 0..size {
        let y0 = y * SIZE / size;
        let y1 = ((y + 1) * SIZE / size).max(y0 + 1).min(SIZE);
        for x in 0..size {
            let x0 = x * SIZE / size;
            let x1 = ((x + 1) * SIZE / size).max(x0 + 1).min(SIZE);
            let mut sum = [0_u32; 4];
            let mut count = 0_u32;
            for sy in y0..y1 {
                for sx in x0..x1 {
                    let index = (sy * SIZE + sx) * 4;
                    for channel in 0..4 {
                        sum[channel] += u32::from(MARK[index + channel]);
                    }
                    count += 1;
                }
            }
            let index = (y * size + x) * 4;
            for channel in 0..4 {
                out[index + channel] = u8::try_from(sum[channel] / count).unwrap_or(0);
            }
        }
    }
    out
}

/// The same mark as `assets/icon.svg`, for the file a human can open.
///
/// The picture is the PNG, carried inside the SVG, so the file shows the engraving rather than a
/// second drawing of it.
#[must_use]
pub fn svg() -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {SIZE} {SIZE}\" role=\"img\" aria-label=\"{DESCRIPTION}\">\n  <image width=\"{SIZE}\" height=\"{SIZE}\" href=\"data:image/png;base64,{}\"/>\n</svg>\n",
        base64(&png())
    )
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

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut index = 0;
    while index + 3 <= bytes.len() {
        let value = (u32::from(bytes[index]) << 16) | (u32::from(bytes[index + 1]) << 8) | u32::from(bytes[index + 2]);
        for shift in [18, 12, 6, 0] {
            out.push(TABLE[((value >> shift) & 0x3f) as usize] as char);
        }
        index += 3;
    }
    let rest = bytes.len() - index;
    if rest == 1 {
        let value = u32::from(bytes[index]) << 16;
        out.push(TABLE[((value >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((value >> 12) & 0x3f) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rest == 2 {
        let value = (u32::from(bytes[index]) << 16) | (u32::from(bytes[index + 1]) << 8);
        out.push(TABLE[((value >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((value >> 12) & 0x3f) as usize] as char);
        out.push(TABLE[((value >> 6) & 0x3f) as usize] as char);
        out.push('=');
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

    /// The committed SVG and the readme's PNG are this picture, not files someone edited once.
    ///
    /// `SORTLENS_UPDATE_ICON=1` rewrites them when the mark changes on purpose; CI runs it without.
    #[test]
    fn the_committed_svg_is_the_drawing() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let update = std::env::var("SORTLENS_UPDATE_ICON").is_ok_and(|value| value == "1");
        let svg_path = assets.join("icon.svg");
        let png_path = assets.join("icon.png");
        if update {
            std::fs::write(&svg_path, svg()).expect("rewrite assets/icon.svg");
            std::fs::write(&png_path, png()).expect("rewrite assets/icon.png");
            return;
        }
        let on_disk = std::fs::read_to_string(&svg_path).expect("assets/icon.svg");
        assert_eq!(
            on_disk,
            svg(),
            "assets/icon.svg no longer matches the mark: SORTLENS_UPDATE_ICON=1 cargo test -p sortlens-icon"
        );
        let png_on_disk = std::fs::read(&png_path).expect("assets/icon.png");
        assert_eq!(
            png_on_disk,
            png(),
            "assets/icon.png no longer matches the mark: SORTLENS_UPDATE_ICON=1 cargo test -p sortlens-icon"
        );
    }

    /// The PNG really is a PNG, and a decoder that was not written for it reads the engraving back.
    #[test]
    fn the_window_icon_decodes_to_the_mark() {
        let thumbnail = sortlens_thumbs::decode_image(&png(), SIZE as u32).expect("decode");
        assert_eq!((thumbnail.width, thumbnail.height), (SIZE as u32, SIZE as u32));
        assert!(thumbnail.is_consistent());
        assert_eq!(thumbnail.rgba, rgba(), "the PNG is the master raster");
        assert_eq!(thumbnail.rgba[3], 0, "the rounded corner is transparent");
        let center = (SIZE / 2 * SIZE + SIZE / 2) * 4;
        assert_eq!(thumbnail.rgba[center + 3], 255, "the aperture is opaque");
    }

    /// The committed packaging icons are this picture too, not files someone dropped in once.
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
                    "assets/{name} no longer matches the mark: SORTLENS_UPDATE_ICON=1 cargo test -p sortlens-icon"
                );
            }
        }
    }

    /// The `.ico` parses as an ICONDIR whose entries are the sizes a Windows desktop asks for, and
    /// every embedded PNG decodes to exactly that square of the mark.
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
            assert_eq!((thumbnail.width, thumbnail.height), (size, size), "the {size} px entry is that square");
            assert!(thumbnail.is_consistent(), "the {size} px entry is a real image");
            let rendered = render(size as usize);
            let at = size as usize / 2;
            let index = (at * size as usize + at) * 4;
            assert_eq!(
                &thumbnail.rgba[index..index + 4],
                &rendered[index..index + 4],
                "the {size} px entry is the aperture, not another picture"
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
            let rendered = render(expected as usize);
            let at = expected as usize / 2;
            let index = (at * expected as usize + at) * 4;
            assert_eq!(
                &thumbnail.rgba[index..index + 4],
                &rendered[index..index + 4],
                "{name} is the aperture, not another picture"
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

    /// A square, opaque in the middle, transparent at the corners: the tile, not a filled rectangle.
    #[test]
    fn the_icon_is_square_and_rounded() {
        let pixels = rgba();
        assert_eq!(pixels.len(), SIZE * SIZE * 4);
        let alpha = |x: usize, y: usize| pixels[(y * SIZE + x) * 4 + 3];
        assert_eq!(alpha(0, 0), 0);
        assert_eq!(alpha(SIZE - 1, SIZE - 1), 0);
        assert_eq!(alpha(SIZE / 2, 24), 255, "the top of the tile, away from the corners");
    }
}
