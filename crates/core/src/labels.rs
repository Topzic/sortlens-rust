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
}
