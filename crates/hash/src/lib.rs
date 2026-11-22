//! Perceptual hashes for duplicate and near-duplicate detection.
//!
//! Two hashes are kept because they fail differently: a difference hash survives exposure and
//! colour shifts but is confused by crops, while an average hash survives small edits but shrugs at
//! structure. A pair counts as similar when either is close, which is the rule the view uses.
//!
//! Hashing works on a small greyscale copy, so it is cheap enough to run over a whole library and
//! stable across the thumbnail sizes the app happens to have decoded.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use sortlens_thumbs::Thumbnail;

/// Width and height a source is reduced to before hashing.
const GRID_WIDTH: u32 = 9;
const GRID_HEIGHT: u32 = 8;

/// Perceptual identity of one image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hash {
    /// Difference hash: the sign of every horizontal neighbour pair.
    pub difference: u64,
    /// Average hash: every cell against the image mean.
    pub average: u64,
}

impl Hash {
    /// Compute both hashes from a decoded thumbnail of any size.
    pub fn of(thumbnail: &Thumbnail) -> Option<Self> {
        let cells = cells(thumbnail)?;
        Some(Self {
            difference: difference(&cells),
            average: average(&cells),
        })
    }

    /// How far apart two images are, as the smaller of the two hash distances.
    ///
    /// Zero means the two reduced images are identical under both hashes. A zero average hash means
    /// the reduced image was uniform — no cell above the mean — which carries no structure at all,
    /// so it is ignored rather than allowed to make every flat image look like every other.
    pub fn distance(self, other: Self) -> u32 {
        let difference = hamming(self.difference, other.difference);
        if self.average == 0 || other.average == 0 {
            return difference;
        }
        difference.min(hamming(self.average, other.average))
    }
}

/// Hash one file by decoding a small copy of it.
///
/// RAWs use their embedded preview, exactly like the grid, so a raw and its exported JPEG hash
/// alike. Files whose preview cannot be decoded are left out rather than guessed at.
pub fn hash_file(path: &std::path::Path, max_edge: u32) -> Option<Hash> {
    let thumbnail = sortlens_thumbs::decode_file(path, max_edge.max(64)).ok()?;
    Hash::of(&thumbnail)
}

/// Hash and colour-measure one file in a single decode.
pub fn analyse_file(path: &std::path::Path, max_edge: u32) -> Option<(Hash, Colour)> {
    let thumbnail = sortlens_thumbs::decode_file(path, max_edge.max(64)).ok()?;
    Some((Hash::of(&thumbnail)?, Colour::of(&thumbnail)?))
}

/// A coarse colour fingerprint: a 3×3 grid of mean RGB.
///
/// Hashes measure structure only, so two different photographs with the same layout of light and
/// dark — a page of text, a striped shirt — look alike to them. Colour separates those cases: same
/// shapes in a different palette are not the same picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colour {
    /// Mean red per cell.
    pub red: [u8; 9],
    /// Mean green per cell.
    pub green: [u8; 9],
    /// Mean blue per cell.
    pub blue: [u8; 9],
}

impl Colour {
    /// Measure a decoded thumbnail of any size.
    ///
    /// Each cell is the mean of the pixels it covers, using the same box filter as the hashes, so
    /// every pixel contributes to exactly one cell.
    pub fn of(thumbnail: &Thumbnail) -> Option<Self> {
        let width = thumbnail.width;
        let height = thumbnail.height;
        if width == 0 || height == 0 || thumbnail.rgba.len() < (width as usize * height as usize * 4) {
            return None;
        }
        let columns = ranges(width, 3);
        let rows = ranges(height, 3);
        let mut colour = Self {
            red: [0; 9],
            green: [0; 9],
            blue: [0; 9],
        };
        for (row, (top, bottom)) in rows.iter().enumerate() {
            for (column, (left, right)) in columns.iter().enumerate() {
                let mut sums = (0u32, 0u32, 0u32);
                for y in *top..*bottom {
                    for x in *left..*right {
                        let index = ((y as usize * width as usize) + x as usize) * 4;
                        sums.0 += u32::from(thumbnail.rgba[index]);
                        sums.1 += u32::from(thumbnail.rgba[index + 1]);
                        sums.2 += u32::from(thumbnail.rgba[index + 2]);
                    }
                }
                let count = ((bottom - top) * (right - left)).max(1);
                let cell = row * 3 + column;
                colour.red[cell] = (sums.0 / count).min(255) as u8;
                colour.green[cell] = (sums.1 / count).min(255) as u8;
                colour.blue[cell] = (sums.2 / count).min(255) as u8;
            }
        }
        Some(colour)
    }

    /// Mean absolute difference per channel and cell, 0–255.
    ///
    /// Averaging keeps the number comparable across grids: a tolerance means the same thing for a
    /// flat wall and for a busy street.
    pub fn distance(self, other: Self) -> u32 {
        let mut total: u32 = 0;
        for cell in 0..9 {
            total += u32::from(self.red[cell].abs_diff(other.red[cell]));
            total += u32::from(self.green[cell].abs_diff(other.green[cell]));
            total += u32::from(self.blue[cell].abs_diff(other.blue[cell]));
        }
        total / 27
    }

    /// Overall brightness, 0–255: the mean of every channel and cell.
    ///
    /// Colour search uses this as a sort key: if two palettes differ in brightness by more than the
    /// tolerance they cannot be within that tolerance cell by cell, because the mean of the
    /// differences can never exceed the mean of the absolutes. Sorting by it turns a quadratic scan
    /// into a sliding window.
    pub fn mean(self) -> u32 {
        let mut total: u32 = 0;
        for cell in 0..9 {
            total += u32::from(self.red[cell]) + u32::from(self.green[cell]) + u32::from(self.blue[cell]);
        }
        total / 27
    }
}

/// Count set bits in `left XOR right`.
pub fn hamming(left: u64, right: u64) -> u32 {
    (left ^ right).count_ones()
}

/// Reduce a thumbnail to a `GRID_WIDTH × GRID_HEIGHT` greyscale grid, 0–255 per cell.
///
/// Each cell is the mean of the source pixels it covers, which is a box filter: every source pixel
/// contributes exactly once, so the result does not depend on where the sample grid happens to land.
fn cells(thumbnail: &Thumbnail) -> Option<Vec<u8>> {
    let width = thumbnail.width;
    let height = thumbnail.height;
    if width == 0 || height == 0 || thumbnail.rgba.len() < (width as usize * height as usize * 4) {
        return None;
    }
    let mut cells = Vec::with_capacity((GRID_WIDTH * GRID_HEIGHT) as usize);
    for row in 0..GRID_HEIGHT {
        for column in 0..GRID_WIDTH {
            let left = column * width / GRID_WIDTH;
            let right = (((column + 1) * width) / GRID_WIDTH).max(left + 1).min(width);
            let top = row * height / GRID_HEIGHT;
            let bottom = (((row + 1) * height) / GRID_HEIGHT).max(top + 1).min(height);
            let mut total: u64 = 0;
            let mut count: u64 = 0;
            for y in top..bottom {
                for x in left..right {
                    let index = ((y as usize * width as usize) + x as usize) * 4;
                    let pixel = &thumbnail.rgba[index..index + 4];
                    // Rec. 601 luma, in integers: 299 R + 587 G + 114 B over 1000.
                    let luma = (299 * u64::from(pixel[0]) + 587 * u64::from(pixel[1]) + 114 * u64::from(pixel[2])) / 1000;
                    total += luma;
                    count += 1;
                }
            }
            cells.push(total.checked_div(count).map_or(0, |mean| mean.min(255) as u8));
        }
    }
    Some(cells)
}

/// Split a length into `cells` contiguous ranges, each at least one pixel wide.
fn ranges(length: u32, cells: u32) -> Vec<(u32, u32)> {
    (0..cells)
        .map(|cell| {
            let start = cell * length / cells;
            let end = (((cell + 1) * length) / cells).max(start + 1).min(length);
            (start, end)
        })
        .collect()
}

/// Difference hash: one bit per horizontal neighbour pair, set when the left cell is brighter.
fn difference(cells: &[u8]) -> u64 {
    let mut hash = 0u64;
    let mut bit = 0;
    for row in 0..GRID_HEIGHT {
        for column in 0..GRID_WIDTH - 1 {
            let index = (row * GRID_WIDTH + column) as usize;
            if cells[index] > cells[index + 1] {
                hash |= 1 << bit;
            }
            bit += 1;
        }
    }
    hash
}

/// Average hash: one bit per cell, set when the cell is brighter than the mean.
fn average(cells: &[u8]) -> u64 {
    let total: u64 = cells.iter().map(|value| u64::from(*value)).sum();
    let mean = total / cells.len().max(1) as u64;
    let mut hash = 0u64;
    for (bit, value) in cells.iter().enumerate().take(64) {
        if u64::from(*value) > mean {
            hash |= 1 << bit;
        }
    }
    hash
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// A thumbnail painted from a per-pixel greyscale function.
    fn paint(width: u32, height: u32, shade: impl Fn(u32, u32) -> u8) -> Thumbnail {
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let value = shade(x, y);
                rgba.extend_from_slice(&[value, value, value, 255]);
            }
        }
        Thumbnail { width, height, rgba }
    }

    #[test]
    fn the_same_image_hashes_to_itself() {
        let thumbnail = paint(64, 64, |x, y| ((x * 3 + y * 5) % 256) as u8);
        let hash = Hash::of(&thumbnail).expect("hash");
        assert_eq!(hash.distance(hash), 0);
        assert_eq!(hamming(hash.difference, hash.difference), 0);
    }

    #[test]
    fn a_left_to_right_gradient_sets_every_difference_bit_exactly_once() {
        // Bright on the left, dark on the right: every neighbour pair descends.
        let thumbnail = paint(180, 80, |x, _| 255u8.saturating_sub((x * 255 / 180) as u8));
        let hash = Hash::of(&thumbnail).expect("hash");
        assert_eq!(hash.difference.count_ones(), 64, "all 64 neighbour pairs descend");
    }

    #[test]
    fn an_even_field_is_flat() {
        let flat = paint(32, 32, |_, _| 128);
        let hash = Hash::of(&flat).expect("hash");
        assert_eq!(hash.difference, 0, "no neighbour is brighter than another");
        assert_eq!(hash.average, 0, "nothing is above the mean");
    }

    #[test]
    fn a_brightened_copy_keeps_its_structure() {
        let original = paint(90, 60, |x, y| ((x * 7 + y * 3) % 200) as u8);
        let bright = paint(90, 60, |x, y| (((x * 7 + y * 3) % 200) + 30).min(255) as u8);
        let left = Hash::of(&original).expect("hash");
        let right = Hash::of(&bright).expect("hash");
        assert!(
            left.distance(right) <= 8,
            "a uniform lift should stay close, got {}",
            left.distance(right)
        );
    }

    #[test]
    fn horizontal_and_vertical_stripes_are_far_apart() {
        let horizontal = Hash::of(&paint(80, 80, |_, y| if (y / 10) % 2 == 0 { 240 } else { 20 })).expect("hash");
        let vertical = Hash::of(&paint(80, 80, |x, _| if (x / 10) % 2 == 0 { 240 } else { 20 })).expect("hash");
        assert!(
            horizontal.distance(vertical) > 16,
            "stripes in the other direction must not look alike, got {}",
            horizontal.distance(vertical)
        );
    }

    #[test]
    fn hashing_refuses_an_empty_thumbnail() {
        assert!(
            Hash::of(&Thumbnail {
                width: 0,
                height: 0,
                rgba: Vec::new()
            })
            .is_none()
        );
        assert!(
            Hash::of(&Thumbnail {
                width: 4,
                height: 4,
                rgba: vec![0; 8]
            })
            .is_none()
        );
    }

    #[test]
    fn a_flat_image_does_not_match_a_structured_one() {
        let flat = Hash { difference: 0, average: 0 };
        let structured = Hash {
            difference: 0,
            average: u64::MAX,
        };
        assert_eq!(flat.distance(structured), 0, "identical structure, no average signal");
        let opposite = Hash {
            difference: u64::MAX,
            average: 0,
        };
        assert_eq!(flat.distance(opposite), 64, "structure still separates them");
    }

    #[test]
    fn a_file_hashes_through_its_decoded_copy() {
        let folder = std::env::temp_dir().join(format!("sortlens-hash-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("dir");
        let gradient = encode(80, 60, |x, y| ((x * 3 + y) % 240) as u8);
        let stripes = encode(80, 60, |x, _| if (x / 8) % 2 == 0 { 240 } else { 16 });
        let first = folder.join("a.jpg");
        let copy = folder.join("b.jpg");
        let other = folder.join("c.jpg");
        std::fs::write(&first, &gradient).expect("write");
        std::fs::write(&copy, &gradient).expect("write");
        std::fs::write(&other, &stripes).expect("write");

        let left = hash_file(&first, 64).expect("hash a");
        let same = hash_file(&copy, 64).expect("hash b");
        let different = hash_file(&other, 64).expect("hash c");
        assert_eq!(left.distance(same), 0, "identical files agree");
        assert!(left.distance(different) > 12, "a different picture is not a duplicate");
        assert!(hash_file(&folder.join("missing.jpg"), 64).is_none());
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// Encode a greyscale test image as a baseline JPEG.
    fn encode(width: u32, height: u32, shade: impl Fn(u32, u32) -> u8) -> Vec<u8> {
        let mut luma = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                luma.push(shade(x, y));
            }
        }
        craft_jpeg::encode_baseline(
            &luma,
            width as usize,
            height as usize,
            1,
            92,
            craft_jpeg::ChromaSubsampling::S444,
            &[],
        )
        .expect("encode")
    }

    #[test]
    fn colour_separates_same_shapes_in_another_palette() {
        let red = paint(60, 60, |_, _| 0);
        let coloured = |red_value: u8, green: u8, blue: u8| {
            let mut rgba = Vec::with_capacity(60 * 60 * 4);
            for _ in 0..60 * 60 {
                rgba.extend_from_slice(&[red_value, green, blue, 255]);
            }
            Thumbnail {
                width: 60,
                height: 60,
                rgba,
            }
        };
        let warm = Colour::of(&coloured(220, 40, 30)).expect("colour");
        let cool = Colour::of(&coloured(30, 40, 220)).expect("colour");
        let warm_again = Colour::of(&coloured(222, 42, 32)).expect("colour");
        assert!(warm.distance(cool) > 100, "red and blue are far apart");
        assert!(warm.distance(warm_again) < 4, "a nudge in exposure stays close");
        assert!(Colour::of(&red).is_some());
        assert!(
            Colour::of(&Thumbnail {
                width: 0,
                height: 0,
                rgba: Vec::new()
            })
            .is_none()
        );
    }

    #[test]
    fn a_half_and_half_image_gives_two_different_column_means() {
        let thumbnail = paint(60, 30, |x, _| if x < 30 { 20 } else { 220 });
        let colour = Colour::of(&thumbnail).expect("colour");
        assert!(colour.red[0] < 40, "left cells are dark");
        assert!(colour.red[8] > 200, "right cells are bright");
        assert_eq!(colour.red[0], 20, "the first cell is entirely dark");
        assert_eq!(colour.red[1], 120, "the middle cell straddles the boundary");
    }

    #[test]
    fn hamming_counts_differing_bits() {
        assert_eq!(hamming(0b1011, 0b1001), 1);
        assert_eq!(hamming(0, u64::MAX), 64);
    }
}
