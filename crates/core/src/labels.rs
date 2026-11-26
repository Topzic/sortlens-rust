//! Rating-adjacent vocabulary: stars, flags, colour labels, media kind.

use serde::{Deserialize, Serialize};

/// A colour label, in the fixed order the suite uses everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorLabel {
    Red,
    Yellow,
    Green,
    Blue,
    Purple,
}

impl ColorLabel {
    /// Every label, in menu order.
    pub const ALL: [Self; 5] = [Self::Red, Self::Yellow, Self::Green, Self::Blue, Self::Purple];

    /// The labels that carry a number key: 6–9, in that order.
    ///
    /// Purple is deliberately left off the number row: `0` is the "clear the rating" key that
    /// Sortlens has always used, and a label on the same key would make one of them unreachable.
    pub const KEYED: [Self; 4] = [Self::Red, Self::Yellow, Self::Green, Self::Blue];

    /// Stable lowercase name, used in the op log and in XMP.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Purple => "purple",
        }
    }

    /// Human label for menus and tooltips.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
        }
    }
}

/// The tri-state cull flag. `None` means unflagged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Flag {
    #[default]
    None,
    Pick,
    Reject,
}

impl Flag {
    /// Stable name for the op log and XMP (`xmp:Label` is separate: see [`ColorLabel`]).
    pub const fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Pick => "pick",
            Self::Reject => "reject",
        }
    }
}

/// The review decision for one photo, as it travels between the scanner, the catalog and sidecars.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    /// Stars, 0–5.
    #[serde(default)]
    pub rating: u8,
    /// The cull decision.
    #[serde(default)]
    pub flag: Flag,
    /// Colour label.
    #[serde(default)]
    pub label: Option<ColorLabel>,
    /// Keywords, sorted and deduplicated.
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Review {
    /// Whether nothing has been decided yet.
    pub fn is_empty(&self) -> bool {
        self.rating == 0 && self.flag == Flag::None && self.label.is_none() && self.tags.is_empty()
    }
}

/// What a catalog entry actually is. Drives which decoder and which preview path runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MediaKind {
    /// A camera raw file; previews come from the embedded JPEG unless develop is on.
    Raw,
    /// A rendered image: JPEG, HEIF, PNG, TIFF, WebP.
    Image,
    /// A video file; the grid shows a poster frame.
    Video,
}

impl MediaKind {
    /// Whether the file needs a raw decoder rather than a raster decoder.
    pub const fn is_raw(self) -> bool {
        matches!(self, Self::Raw)
    }
}

/// Where a keyword came from.
///
/// A keyword's provenance is the reason to trust it: one typed by a person is a decision, one a pack proposed and
/// somebody accepted is a proposal that was reviewed, and one read out of the file's own sidecar was written by
/// another application. The four are not a ranking — the point is that they can be told apart and filtered on.
///
/// [`Self::Unrecorded`] exists for the honest case, not as a fallback: a library written before this build kept no
/// sources, and calling those keywords `Manual` would invent a decision nobody made. A tag with no entry in the
/// photo's map is exactly that — unrecorded — so the variant is derived, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TagSource {
    /// Typed, pasted or applied by hand — including by a script calling the same command.
    Manual,
    /// Proposed by the model or a keyword pack and accepted by a person.
    Ai,
    /// Read from the file's own sidecar when the photo was first indexed.
    Xmp,
    /// Carried in a library written before sources were recorded.
    Unrecorded,
}

impl TagSource {
    /// The recorded sources, in the order the filter offers them.
    pub const ALL: [Self; 3] = [Self::Manual, Self::Ai, Self::Xmp];

    /// Every source the chips and the filter can name, with the derived [`Self::Unrecorded`] last.
    ///
    /// [`Self::ALL`] is what a library records; the fourth is derived for keywords written before
    /// sources were kept, and the filter still has to offer it so an old library is findable. A chip
    /// that can show it, the cycle it steps through and a catalogue that must answer its word all
    /// need the four in one order, so that order lives here instead of in each caller's `chain`.
    pub const EVERY: [Self; 4] = [Self::Manual, Self::Ai, Self::Xmp, Self::Unrecorded];

    /// Stable lowercase name, for the op log, the snapshot and filter JSON.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Ai => "ai",
            Self::Xmp => "xmp",
            Self::Unrecorded => "unrecorded",
        }
    }

    /// Human label for menus, chips and tooltips.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Manual => "Manual",
            Self::Ai => "AI",
            Self::Xmp => "XMP",
            Self::Unrecorded => "Not recorded",
        }
    }

    /// One sentence for a hover, saying what the word means rather than repeating it.
    pub const fn explanation(self) -> &'static str {
        match self {
            Self::Manual => "Typed in Sortlens, or applied by a command that does the same",
            Self::Ai => "Proposed by the model or a keyword pack, and accepted",
            Self::Xmp => "Read from the file's own sidecar when it was indexed",
            Self::Unrecorded => "This library predates recorded sources",
        }
    }

    /// Parse the stable name, for a CLI flag or a filter string.
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "manual" => Some(Self::Manual),
            "ai" => Some(Self::Ai),
            "xmp" => Some(Self::Xmp),
            "unrecorded" | "unknown" => Some(Self::Unrecorded),
            _ => None,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn every_label_has_a_unique_id() {
        let mut ids: Vec<_> = ColorLabel::ALL.iter().map(|l| l.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ColorLabel::ALL.len());
    }

    #[test]
    fn default_flag_is_none() {
        assert_eq!(Flag::default(), Flag::None);
    }

    #[test]
    fn every_source_survives_its_own_name() {
        assert!(
            TagSource::EVERY.starts_with(&TagSource::ALL),
            "the recorded sources keep their order"
        );
        assert_eq!(TagSource::EVERY[3], TagSource::Unrecorded, "the derived one comes last");
        for source in TagSource::EVERY {
            assert_eq!(TagSource::parse(source.id()), Some(source), "{}", source.id());
            assert!(!source.explanation().is_empty(), "{} has no explanation", source.id());
        }
        assert_eq!(TagSource::parse("UNKNOWN"), Some(TagSource::Unrecorded), "the old word still reads");
        assert_eq!(TagSource::parse("guess"), None, "a source nobody can produce is not a source");
        let text = serde_json::to_string(&TagSource::Ai).expect("serialise");
        assert_eq!(text, "\"ai\"", "the log writes the stable name, not the enum");
    }
}
