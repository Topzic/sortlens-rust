//! Keyword packs: vocabularies as data, not as code.
//!
//! A pack is a small JSON file someone can write, copy or share:
//!
//! ```json
//! {
//!   "name": "Travel",
//!   "licence": "CC0-1.0",
//!   "keywords": [
//!     { "keyword": "japan", "when": { "folder_contains": "Japan" } },
//!     { "keyword": "night", "when": { "min_iso": 3200 } },
//!     { "keyword": "street", "when": { "focal_band": "wide", "camera_contains": "X100" } }
//!   ]
//! }
//! ```
//!
//! Each keyword carries a *matcher* saying when it applies, and every field in the matcher must match.
//! A keyword with no matcher is refused: a rule that always matches would tag the whole library from one
//! typo, and there is no other feature in the app that can do that much damage in one line.
//!
//! Nothing here writes: packs produce proposals exactly like the EXIF hints and a detector's guesses, and
//! the same panel decides what becomes a keyword.

use serde::Deserialize;

use crate::suggest::{Candidate, Source};

/// Confidence attached to a pack match. Above a focal band (a coarse observation) and below a camera or
/// lens name (read from the file itself), because a pack is someone else's idea of what a folder means.
pub const PACK_SCORE: f32 = 0.75;

/// A named vocabulary.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Pack {
    /// The name shown in settings.
    pub name: String,
    /// What the pack's words may be used under, which is why a pack is not just a word list.
    pub licence: String,
    /// The rules.
    pub keywords: Vec<Rule>,
}

/// One keyword and the conditions under which it applies.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Rule {
    /// The keyword to propose.
    pub keyword: String,
    /// When it applies. At least one field must be set.
    pub when: When,
}

/// Conditions, all of which must hold. Fields match case-insensitively.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct When {
    /// The photo's folder, or any part of its path, contains this.
    #[serde(default)]
    pub folder_contains: Option<String>,
    /// The camera make or model contains this.
    #[serde(default)]
    pub camera_contains: Option<String>,
    /// The lens model contains this.
    #[serde(default)]
    pub lens_contains: Option<String>,
    /// The ISO is at least this.
    #[serde(default)]
    pub min_iso: Option<u32>,
    /// The focal length band is this one of `wide`, `normal`, `telephoto`.
    #[serde(default)]
    pub focal_band: Option<String>,
}

/// What a pack is allowed to look at.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Context {
    /// The photo's path relative to its root, so folder names are visible.
    pub folder: String,
    /// Camera make and model.
    pub camera: String,
    /// Lens model.
    pub lens: String,
    /// ISO speed.
    pub iso: Option<u32>,
    /// Focal length in millimetres.
    pub focal: Option<f64>,
}

/// Why a pack file could not be used.
#[derive(Debug, Clone, PartialEq)]
pub enum PackError {
    /// The file is not JSON, or not this shape.
    Malformed(String),
    /// A required field is blank.
    Blank(&'static str),
    /// A rule has no conditions, so it would match every photo.
    Unconditional(String),
    /// A rule names a band that does not exist.
    UnknownBand(String),
    /// The pack has no rules, so it would do nothing.
    Empty,
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(detail) => write!(f, "the pack is not valid JSON in this shape: {detail}"),
            Self::Blank(field) => write!(f, "the pack has no {field}"),
            Self::Unconditional(keyword) => write!(f, "“{keyword}” has no conditions, so it would tag every photo — give it a when"),
            Self::UnknownBand(band) => write!(f, "“{band}” is not a focal band (wide, normal, telephoto)"),
            Self::Empty => write!(f, "the pack has no keywords"),
        }
    }
}

impl std::error::Error for PackError {}

impl When {
    /// Whether every condition holds for `context`.
    #[must_use]
    pub fn matches(&self, context: &Context) -> bool {
        let contains = |haystack: &str, needle: &Option<String>| match needle {
            Some(needle) => haystack.to_lowercase().contains(&needle.to_lowercase()),
            None => true,
        };
        let band = self
            .focal_band
            .as_ref()
            .is_none_or(|band| crate::hints::focal_band(context.focal).is_some_and(|actual| actual.eq_ignore_ascii_case(band)));
        let iso = self.min_iso.is_none_or(|min| context.iso.is_some_and(|iso| iso >= min));
        contains(&context.folder, &self.folder_contains)
            && contains(&context.camera, &self.camera_contains)
            && contains(&context.lens, &self.lens_contains)
            && iso
            && band
    }

    /// Whether any condition is set at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.folder_contains.is_none()
            && self.camera_contains.is_none()
            && self.lens_contains.is_none()
            && self.min_iso.is_none()
            && self.focal_band.is_none()
    }
}

impl Pack {
    /// Read a pack, refusing anything that could misbehave quietly.
    pub fn parse(text: &str) -> Result<Pack, PackError> {
        let pack: Pack = serde_json::from_str(text).map_err(|error| PackError::Malformed(error.to_string()))?;
        pack.check()?;
        Ok(pack)
    }

    fn check(&self) -> Result<(), PackError> {
        if self.name.trim().is_empty() {
            return Err(PackError::Blank("name"));
        }
        if self.licence.trim().is_empty() {
            return Err(PackError::Blank("licence"));
        }
        if self.keywords.is_empty() {
            return Err(PackError::Empty);
        }
        for rule in &self.keywords {
            let keyword = rule.keyword.trim();
            if keyword.is_empty() {
                return Err(PackError::Blank("keyword"));
            }
            if rule.when.is_empty() {
                return Err(PackError::Unconditional(keyword.to_string()));
            }
            if let Some(band) = &rule.when.focal_band
                && !matches!(band.to_lowercase().as_str(), "wide" | "normal" | "telephoto")
            {
                return Err(PackError::UnknownBand(band.clone()));
            }
        }
        Ok(())
    }

    /// The keywords this pack proposes for one photo, in the order the pack lists them.
    #[must_use]
    pub fn candidates(&self, context: &Context) -> Vec<Candidate> {
        self.keywords
            .iter()
            .filter(|rule| rule.when.matches(context))
            .map(|rule| Candidate::new(rule.keyword.trim(), PACK_SCORE, Source::Pack))
            .collect()
    }

    /// How many keywords the pack can propose, for the settings row.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keywords.len()
    }

    /// Whether the pack proposes nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keywords.is_empty()
    }

    /// A file name for this pack: lowercase words joined by dashes, so two packs with the same name do
    /// not silently replace each other in a folder listing.
    #[must_use]
    pub fn slug(&self) -> String {
        let mut slug = String::new();
        for word in self.name.split(|character: char| !character.is_alphanumeric()) {
            if word.is_empty() {
                continue;
            }
            if !slug.is_empty() {
                slug.push('-');
            }
            slug.push_str(&word.to_lowercase());
        }
        if slug.is_empty() { "pack".to_string() } else { slug }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    const TRAVEL: &str = r#"{
        "name": "Travel",
        "licence": "CC0-1.0",
        "keywords": [
            { "keyword": "japan", "when": { "folder_contains": "Japan" } },
            { "keyword": "night", "when": { "min_iso": 3200 } },
            { "keyword": "street", "when": { "focal_band": "wide", "camera_contains": "X100" } }
        ]
    }"#;

    fn context(folder: &str, camera: &str, iso: Option<u32>, focal: Option<f64>) -> Context {
        Context {
            folder: folder.to_string(),
            camera: camera.to_string(),
            lens: String::new(),
            iso,
            focal,
        }
    }

    fn tags(pack: &Pack, context: &Context) -> Vec<String> {
        pack.candidates(context).into_iter().map(|candidate| candidate.tag).collect()
    }

    #[test]
    fn a_pack_reads_and_proposes_each_keyword_only_where_it_applies() {
        let pack = Pack::parse(TRAVEL).expect("parse");
        assert_eq!(pack.name, "Travel");
        assert_eq!(pack.licence, "CC0-1.0");
        assert_eq!(pack.len(), 3);
        assert_eq!(pack.slug(), "travel");

        // "street" needs a wide focal length as well as the camera, so it appears only with one.
        assert_eq!(
            tags(&pack, &context("Trips/2023/Japan", "FUJIFILM X100V", None, None)),
            vec!["japan"]
        );
        assert_eq!(
            tags(&pack, &context("Trips/2023/Japan", "FUJIFILM X100V", None, Some(23.0))),
            vec!["japan", "street"]
        );
        assert_eq!(tags(&pack, &context("Trips/Norway", "Nikon Z6", Some(6400), None)), vec!["night"]);
        assert!(tags(&pack, &context("Trips/Norway", "Nikon Z6", Some(100), Some(600.0))).is_empty());
        assert!(
            pack.candidates(&context("Japan", "", None, None))
                .iter()
                .all(|candidate| candidate.source == Source::Pack),
            "a pack keyword says it came from a pack"
        );
        // Two conditions are an AND, not an OR: an X100 with a normal lens is not street.
        assert!(tags(&pack, &context("", "FUJIFILM X100V", None, Some(50.0))).is_empty());
        assert_eq!(tags(&pack, &context("", "FUJIFILM X100V", None, Some(35.0))), vec!["street"]);
    }

    #[test]
    fn matching_ignores_case_on_both_sides() {
        let pack = Pack::parse(TRAVEL).expect("parse");
        assert_eq!(
            tags(&pack, &context("japan", "fujifilm x100v", None, Some(23.0))),
            vec!["japan", "street"]
        );
    }

    #[test]
    fn a_rule_without_conditions_is_refused() {
        let text = r#"{ "name": "Bad", "licence": "CC0-1.0", "keywords": [ { "keyword": "everything", "when": {} } ] }"#;
        let error = Pack::parse(text).expect_err("must refuse");
        assert!(matches!(error, PackError::Unconditional(ref keyword) if keyword == "everything"));
        assert!(error.to_string().contains("would tag every photo"));
    }

    #[test]
    fn blank_fields_empty_packs_and_unknown_bands_are_refused() {
        for (text, expected) in [
            (r#"{ "name": "", "licence": "CC0-1.0", "keywords": [] }"#, "no name"),
            (r#"{ "name": "P", "licence": " ", "keywords": [] }"#, "no licence"),
            (r#"{ "name": "P", "licence": "CC0-1.0", "keywords": [] }"#, "no keywords"),
            (
                r#"{ "name": "P", "licence": "CC0-1.0", "keywords": [ { "keyword": " ", "when": { "min_iso": 1 } } ] }"#,
                "no keyword",
            ),
            (
                r#"{ "name": "P", "licence": "CC0-1.0", "keywords": [ { "keyword": "x", "when": { "focal_band": "macro" } } ] }"#,
                "not a focal band",
            ),
        ] {
            let error = Pack::parse(text).expect_err(text);
            assert!(error.to_string().contains(expected), "{error} should mention {expected}");
        }
    }

    #[test]
    fn malformed_json_and_unknown_fields_are_refused() {
        assert!(matches!(Pack::parse("not json"), Err(PackError::Malformed(_))));
        let unknown = r#"{ "name": "P", "licence": "CC0-1.0", "keywords": [ { "keyword": "x", "when": { "iso_at_least": 400 } } ] }"#;
        assert!(
            matches!(Pack::parse(unknown), Err(PackError::Malformed(_))),
            "a misspelled condition must not be silently ignored"
        );
    }

    #[test]
    fn a_pack_with_no_iso_never_matches_a_minimum() {
        let pack =
            Pack::parse(r#"{ "name": "P", "licence": "CC0-1.0", "keywords": [ { "keyword": "night", "when": { "min_iso": 3200 } } ] }"#)
                .expect("parse");
        assert!(tags(&pack, &context("", "", None, None)).is_empty());
        assert_eq!(tags(&pack, &context("", "", Some(3200), None)), vec!["night"]);
    }

    #[test]
    fn a_band_condition_needs_a_focal_length_on_the_photo() {
        let pack = Pack::parse(
            r#"{ "name": "P", "licence": "CC0-1.0", "keywords": [ { "keyword": "wide", "when": { "focal_band": "wide" } } ] }"#,
        )
        .expect("parse");
        assert!(tags(&pack, &context("", "", None, None)).is_empty(), "no focal length, no band");
    }

    #[test]
    fn a_pack_feeds_the_merger_and_respects_the_keywords_already_there() {
        use crate::suggest::{Caps, merge};
        let pack = Pack::parse(TRAVEL).expect("parse");
        let offered = pack.candidates(&context("Japan", "X100", None, Some(23.0)));
        assert_eq!(offered.len(), 2);
        let merged = merge(&offered, &["japan".to_string()], Caps::default());
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].tag, "street");
        assert_eq!(merged[0].source, Source::Pack);
    }
}
