//! File-kind vocabulary: which extensions are photos, raws, videos or sidecars.
//!
//! This lives in L0 because both the scanner and the thumbnail decoder need the same answer, and
//! two lists that drift apart produce files that index but never preview.

use crate::labels::MediaKind;
use std::path::Path;

/// Extensions treated as camera raw.
const RAW_EXTENSIONS: &[&str] = &[
    "3fr", "ari", "arw", "bay", "cr2", "cr3", "crw", "dcr", "dng", "erf", "fff", "iiq", "k25", "kdc", "mef", "mos", "mrw", "nef", "nrw",
    "orf", "pef", "raf", "raw", "rw2", "rwl", "sr2", "srf", "srw", "x3f",
];

/// Extensions treated as rendered images.
const IMAGE_EXTENSIONS: &[&str] = &[
    "avif", "bmp", "gif", "heic", "heif", "jpe", "jpeg", "jpg", "jxl", "png", "qoi", "tif", "tiff", "webp",
];

/// Extensions treated as video.
const VIDEO_EXTENSIONS: &[&str] = &["avi", "m4v", "mkv", "mov", "mp4", "mts", "m2ts", "webm", "wmv"];

/// Sidecar and cache files that belong to a photo but are not photos.
const SIDECAR_EXTENSIONS: &[&str] = &["aae", "xmp", "pp3", "dop", "on1", "cos", "thm", "lrf"];

/// Classify by extension. `None` for anything that is not a photo, raw, video or sidecar.
pub fn kind_from_extension(extension: &str) -> Option<MediaKind> {
    let extension = extension.to_ascii_lowercase();
    if RAW_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Raw)
    } else if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Video)
    } else if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        Some(MediaKind::Image)
    } else {
        None
    }
}

/// What a file's first bytes say it is.
///
/// Deliberately coarse: `Tiff` covers every raw container built on TIFF (NEF, CR2, DNG, ARW…), because the
/// difference between them is in the metadata, not the header, and guessing "this is a Nikon" from a byte
/// order mark would be a guess presented as a fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Magic {
    /// `FF D8 FF` — JPEG, and every raw whose embedded preview is one.
    Jpeg,
    /// The eight-byte PNG signature.
    Png,
    /// `II*\0` or `MM\0*` — TIFF and the raw formats built on it.
    Tiff,
    /// A HEIF or AVIF file: an ISO base-media file whose brand names HEIF or AVIF.
    Heif,
    /// `RIFF….WEBP`.
    Webp,
    /// An ISO base-media file whose brand names a video track.
    Video,
    /// An ISO base-media file whose brand names a raw format — CR3 is the one that exists.
    Raw,
    /// Nothing recognised, or a file too short to hold a signature.
    Unknown,
}

/// How many bytes [`sniff`] needs. Long enough for the ISO box header plus a brand.
pub const SNIFF_BYTES: usize = 16;

/// Read the kind of file a header claims to be.
#[must_use]
pub fn sniff(head: &[u8]) -> Magic {
    if head.starts_with(&[0xff, 0xd8, 0xff]) {
        return Magic::Jpeg;
    }
    if head.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        return Magic::Png;
    }
    // CR2 and ORF put their own tag after the TIFF magic, but they are TIFF at the container level.
    if head.starts_with(b"II*\0") || head.starts_with(b"MM\0*") {
        return Magic::Tiff;
    }
    if head.starts_with(b"RIFF") && head.len() >= 12 && &head[8..12] == b"WEBP" {
        return Magic::Webp;
    }
    // ISO base media: a box length, "ftyp", then the major brand at byte 8.
    if head.len() >= 12 && &head[4..8] == b"ftyp" {
        let brand: [u8; 4] = match head[8..12].try_into() {
            Ok(brand) => brand,
            Err(_) => return Magic::Unknown,
        };
        return match &brand {
            b"heic" | b"heix" | b"hevc" | b"hevx" | b"mif1" | b"msf1" | b"avif" | b"avis" => Magic::Heif,
            // CR3 is a raw in an ISO base-media coat: without this it would be routed as a video and every
            // thumbnail request would fail with "unsupported format", which reads like a broken file.
            b"crx " => Magic::Raw,
            b"isom" | b"iso2" | b"iso4" | b"iso5" | b"iso6" | b"mp41" | b"mp42" | b"avc1" | b"M4V " | b"M4VH" | b"M4VP" | b"qt  "
            | b"3gp4" | b"3gp5" | b"mmp4" | b"MSnv" => Magic::Video,
            // An unrecognised brand is not evidence of a video; guessing would route podcasts and sidecars
            // into the grid.
            _ => Magic::Unknown,
        };
    }
    Magic::Unknown
}

/// The kind a file should be treated as, given what its name claims and what its bytes say.
///
/// Returns the kind and whether the header contradicted the extension. Only a *contradiction* is corrected:
/// a `.jpg` holding JPEG data (including one holding TIFF data, which is what a raw named `.jpg` is not)
/// stays as named, because renaming a raw to `.jpg` is not a claim this reader can adjudicate. What it can
/// adjudicate is the case that silently produces a broken thumbnail: a raw that is really a JPEG preview, a
/// `.jpg` that is really a video, a `.mov` that is really a still.
#[must_use]
pub fn resolve_kind(claimed: MediaKind, magic: Magic) -> (MediaKind, bool) {
    let actual = match magic {
        Magic::Jpeg | Magic::Png | Magic::Webp => Some(MediaKind::Image),
        Magic::Heif => Some(MediaKind::Image),
        Magic::Tiff | Magic::Raw => Some(MediaKind::Raw),
        Magic::Video => Some(MediaKind::Video),
        Magic::Unknown => None,
    };
    match actual {
        // A TIFF named `.tif` is an image, not a raw: the extension knew which TIFF it was, and the header
        // cannot tell them apart. Keeping the claim avoids turning every scanned TIFF into a raw.
        Some(MediaKind::Raw) if claimed == MediaKind::Image && magic == Magic::Tiff => (claimed, false),
        Some(actual) if actual != claimed => (actual, true),
        _ => (claimed, false),
    }
}

/// Whether the extension names a sidecar rather than a photo.
pub fn is_sidecar_extension(extension: &str) -> bool {
    SIDECAR_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
}

/// Whether a path looks like a camera raw, and therefore needs a preview rather than a decode.
pub fn is_raw_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| kind_from_extension(extension) == Some(MediaKind::Raw))
}

/// The extension of a path, lowercased.
pub fn extension_of(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn extensions_map_to_kinds_case_insensitively() {
        assert_eq!(kind_from_extension("JPG"), Some(MediaKind::Image));
        assert_eq!(kind_from_extension("cr3"), Some(MediaKind::Raw));
        assert_eq!(kind_from_extension("MOV"), Some(MediaKind::Video));
        assert_eq!(kind_from_extension("txt"), None);
    }

    /// The headers that matter, and the two the reader must not over-claim.
    #[test]
    fn a_header_names_its_family_and_only_its_family() {
        assert_eq!(sniff(&[0xff, 0xd8, 0xff, 0xe0]), Magic::Jpeg);
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"), Magic::Png);
        assert_eq!(sniff(b"II*\0\x10\0\0\0"), Magic::Tiff);
        assert_eq!(sniff(b"MM\0*\0\0\0\x10"), Magic::Tiff);
        // CR2 is TIFF with its own tag, and stops at the container level on purpose.
        assert_eq!(sniff(b"II*\0\x10\0\0\0CR\x02\0"), Magic::Tiff);
        assert_eq!(sniff(b"RIFF\x24\0\0\0WEBPVP8 "), Magic::Webp);
        assert_eq!(sniff(b"\0\0\0\x18ftypheic\0\0\0\0"), Magic::Heif);
        assert_eq!(sniff(b"\0\0\0\x18ftypavif\0\0\0\0"), Magic::Heif);
        assert_eq!(sniff(b"\0\0\0\x18ftypmp42\0\0\0\0"), Magic::Video);
        assert_eq!(sniff(b"\0\0\0\x18ftypqt  \0\0\0\0"), Magic::Video);
        assert_eq!(sniff(b"\0\0\0\x18ftypcrx \0\0\0\0"), Magic::Raw, "CR3 is a raw in a box");
        assert_eq!(
            sniff(b"\0\0\0\x18ftypjunk\0\0\0\0"),
            Magic::Unknown,
            "an unknown brand is not a video"
        );
        assert_eq!(sniff(b"not a photo at all"), Magic::Unknown);
        assert_eq!(sniff(&[]), Magic::Unknown);
        assert_eq!(sniff(&[0xff, 0xd8]), Magic::Unknown, "a truncated signature is not a claim");
    }

    /// Routing follows the contradiction, not the wish.
    #[test]
    fn a_contradicted_extension_is_corrected_and_an_ambiguous_one_is_not() {
        // A raw that is really a JPEG preview: the case that silently produced a broken thumbnail.
        assert_eq!(resolve_kind(MediaKind::Raw, Magic::Jpeg), (MediaKind::Image, true));
        // A still named as a video, and a video named as a still.
        assert_eq!(resolve_kind(MediaKind::Video, Magic::Jpeg), (MediaKind::Image, true));
        assert_eq!(resolve_kind(MediaKind::Image, Magic::Video), (MediaKind::Video, true));
        // CR3 named `.cr3` agrees; named `.jpg` it is corrected, because a raw preview is not a still.
        assert_eq!(resolve_kind(MediaKind::Raw, Magic::Raw), (MediaKind::Raw, false));
        assert_eq!(resolve_kind(MediaKind::Image, Magic::Raw), (MediaKind::Raw, true));
        // A TIFF named `.tif` stays an image: the header cannot tell TIFF from NEF, and the name can.
        assert_eq!(resolve_kind(MediaKind::Image, Magic::Tiff), (MediaKind::Image, false));
        // Agreement, and the unknown header, change nothing.
        assert_eq!(resolve_kind(MediaKind::Image, Magic::Jpeg), (MediaKind::Image, false));
        assert_eq!(resolve_kind(MediaKind::Raw, Magic::Unknown), (MediaKind::Raw, false));
    }

    #[test]
    fn sidecars_are_not_photos() {
        assert!(is_sidecar_extension("XMP"));
        assert_eq!(kind_from_extension("xmp"), None);
    }

    #[test]
    fn raw_paths_are_recognised() {
        assert!(is_raw_path(Path::new("/photos/IMG_1.CR3")));
        assert!(!is_raw_path(Path::new("/photos/IMG_1.jpg")));
        assert!(!is_raw_path(Path::new("/photos/no-extension")));
    }
}
