//! Turning guesses into suggestions.
//!
//! A suggestion is a proposal, not a change: nothing here writes to the library. The rules are the
//! interesting part, and they are all in [`merge`]:
//!
//! - a tag the photo already has is never suggested again, because the user has already said it;
//! - two sources proposing the same tag are one suggestion at the higher score;
//! - short tags are dropped: "a" and "of" are what a captioner says when it has nothing to say;
//! - the list is capped, ordered by confidence and then alphabetically, so the same inputs always
//!   produce the same panel.

use crate::detect::{Box, MIN_SCORE};

/// Where a suggestion came from, so the panel can say why it is there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    /// A detector found the thing in the picture.
    Detector,
    /// A camera hint: the EXIF said this already.
    ExifHint,
    /// An embedding compared the picture with the named packs.
    Pack,
    /// A captioner said it in words.
    Caption,
}

impl Source {
    /// What to show beside a suggestion.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Source::Detector => "detector",
            Source::ExifHint => "camera",
            Source::Pack => "pack",
            Source::Caption => "caption",
        }
    }

    /// Rank used to settle a tie between two sources for the same tag: a camera hint outranks a guess.
    #[must_use]
    fn rank(self) -> u8 {
        match self {
            Source::ExifHint => 0,
            Source::Detector => 1,
            Source::Pack => 2,
            Source::Caption => 3,
        }
    }
}

/// One thing some source thinks might be true about a photo.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// The keyword it would add.
    pub tag: String,
    /// How sure the source is, 0..1.
    pub score: f32,
    /// Which source.
    pub source: Source,
    /// Where it was seen, when the source knows.
    pub found: Option<Box>,
}

impl Candidate {
    /// A candidate with no location.
    #[must_use]
    pub fn new(tag: &str, score: f32, source: Source) -> Candidate {
        Candidate {
            tag: tag.to_string(),
            score,
            source,
            found: None,
        }
    }

    /// A candidate found at a box.
    #[must_use]
    pub fn at(tag: &str, score: f32, source: Source, found: Box) -> Candidate {
        Candidate {
            tag: tag.to_string(),
            score,
            source,
            found: Some(found),
        }
    }
}

/// A proposal the review panel can show.
#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    /// The keyword it would add.
    pub tag: String,
    /// The best confidence any source offered.
    pub score: f32,
    /// The source that offered it.
    pub source: Source,
    /// Where it was seen, when there is a box.
    pub found: Option<Box>,
}

/// The pieces a suggestion's line is joined from: the keyword, the source that offered it,
/// and the confidence as a percentage. A panel words these in the reader's language; the
/// English is one of the many sentences these same pieces can be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SuggestionLine<'a> {
    /// The keyword it would add.
    pub tag: &'a str,
    /// Which source.
    pub source: Source,
    /// The confidence, 0..100.
    pub percent: f32,
}

impl SuggestionLine<'_> {
    /// The English line, exactly the sentence the panel printed before it was translated.
    #[must_use]
    pub fn english(&self) -> String {
        format!("{} · {} {:.0}%", self.tag, self.source.label(), self.percent)
    }
}

impl Suggestion {
    /// The pieces of the panel's line, so a translation can put them in its own order.
    #[must_use]
    pub fn line_kind(&self) -> SuggestionLine<'_> {
        SuggestionLine {
            tag: &self.tag,
            source: self.source,
            percent: self.score * 100.0,
        }
    }

    /// One line for the panel.
    #[must_use]
    pub fn line(&self) -> String {
        self.line_kind().english()
    }
}

/// How many suggestions a photo may show, and how sure a source has to be.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Caps {
    /// Maximum suggestions per photo. More than this is a panel nobody reads.
    pub per_photo: usize,
    /// Minimum confidence.
    pub min_score: f32,
    /// Shortest tag that will be suggested; one- and two-letter tags are noise.
    pub min_tag_len: usize,
}

impl Default for Caps {
    fn default() -> Self {
        Caps {
            per_photo: 8,
            min_score: MIN_SCORE,
            min_tag_len: 3,
        }
    }
}

/// The cap the plan commits to: a panel a person can actually read.
pub const DEFAULT_PER_PHOTO: usize = 8;

/// Merge candidates into the suggestions a photo will be offered.
///
/// `existing` is the photo's current keywords, compared case-insensitively and trimmed, because the
/// catalog's keyword names are normalised that way.
#[must_use]
pub fn merge(candidates: &[Candidate], existing: &[String], caps: Caps) -> Vec<Suggestion> {
    let mut known: Vec<String> = existing
        .iter()
        .map(|tag| tag.trim().to_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect();
    known.sort();
    known.dedup();

    let mut merged: Vec<Suggestion> = Vec::new();
    for candidate in candidates {
        let tag = candidate.tag.trim().to_lowercase();
        if tag.chars().count() < caps.min_tag_len {
            continue;
        }
        if !candidate.score.is_finite() || candidate.score < caps.min_score {
            continue;
        }
        if known.binary_search(&tag).is_ok() {
            continue;
        }
        match merged.iter_mut().find(|suggestion| suggestion.tag == tag) {
            Some(existing) => {
                // Better score wins; equal scores are settled by source rank so the panel does not
                // depend on the order the sources happened to run in.
                let better_score = candidate.score > existing.score;
                let tie = (candidate.score - existing.score).abs() < f32::EPSILON;
                if better_score || (tie && candidate.source.rank() < existing.source.rank()) {
                    existing.score = candidate.score;
                    existing.source = candidate.source;
                    if candidate.found.is_some() {
                        existing.found = candidate.found;
                    }
                }
            }
            None => merged.push(Suggestion {
                tag,
                score: candidate.score,
                source: candidate.source,
                found: candidate.found,
            }),
        }
    }

    merged.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.source.rank().cmp(&b.source.rank()))
            .then_with(|| a.tag.cmp(&b.tag))
    });
    merged.truncate(caps.per_photo);
    merged
}

/// One line describing a set of suggestions, for a panel header or a receipt.
#[must_use]
pub fn summary(suggestions: &[Suggestion]) -> String {
    if suggestions.is_empty() {
        return "no suggestions".to_string();
    }
    let names: Vec<&str> = suggestions.iter().take(4).map(|suggestion| suggestion.tag.as_str()).collect();
    let more = suggestions.len().saturating_sub(names.len());
    if more == 0 {
        format!("{} suggestion(s): {}", suggestions.len(), names.join(", "))
    } else {
        format!("{} suggestion(s): {} and {more} more", suggestions.len(), names.join(", "))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn candidates(tags: &[(&str, f32, Source)]) -> Vec<Candidate> {
        tags.iter()
            .map(|(tag, score, source)| Candidate::new(tag, *score, *source))
            .collect()
    }

    #[test]
    fn a_tag_the_photo_already_has_is_not_suggested() {
        let found = candidates(&[("beach", 0.9, Source::Detector), ("dog", 0.8, Source::Detector)]);
        let merged = merge(&found, &["BEACH".to_string()], Caps::default());
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].tag, "dog");
    }

    #[test]
    fn two_sources_agreeing_is_one_suggestion_at_the_higher_score() {
        let found = candidates(&[("harbour", 0.6, Source::Detector), ("harbour", 0.85, Source::Caption)]);
        let merged = merge(&found, &[], Caps::default());
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].tag, "harbour");
        assert!((merged[0].score - 0.85).abs() < 1e-6);
        assert_eq!(merged[0].source, Source::Caption);
    }

    #[test]
    fn a_camera_hint_wins_a_tie_against_a_guess() {
        let found = candidates(&[("harbour", 0.7, Source::Detector), ("harbour", 0.7, Source::ExifHint)]);
        let merged = merge(&found, &[], Caps::default());
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].source, Source::ExifHint, "the camera is not guessing");
    }

    #[test]
    fn the_cap_truncates_the_weakest_and_keeps_the_order() {
        let found: Vec<Candidate> = (0..20)
            .map(|index| Candidate::new(&format!("tag{index:02}"), 1.0 - index as f32 * 0.01, Source::Detector))
            .collect();
        let merged = merge(&found, &[], Caps::default());
        assert_eq!(merged.len(), DEFAULT_PER_PHOTO);
        assert_eq!(merged[0].tag, "tag00");
        assert_eq!(merged[7].tag, "tag07");
        let scores: Vec<f32> = merged.iter().map(|suggestion| suggestion.score).collect();
        assert!(scores.windows(2).all(|pair| pair[0] >= pair[1]), "ordered by confidence");
    }

    #[test]
    fn weak_and_tiny_tags_are_dropped() {
        let found = candidates(&[
            ("dog", 0.9, Source::Detector),
            ("cat", 0.1, Source::Detector),
            ("of", 0.99, Source::Caption),
            ("  ", 0.99, Source::Caption),
            ("NaN", f32::NAN, Source::Caption),
        ]);
        let merged = merge(&found, &[], Caps::default());
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].tag, "dog");
    }

    #[test]
    fn tags_are_normalised_before_they_are_compared() {
        let found = candidates(&[("  Harbour ", 0.9, Source::Detector), ("harbour", 0.8, Source::Pack)]);
        let merged = merge(&found, &["harbour".to_string()], Caps::default());
        assert!(merged.is_empty(), "trimming and case folding happen before the dedupe");
    }

    #[test]
    fn the_same_inputs_in_any_order_give_the_same_panel() {
        let found = candidates(&[
            ("dog", 0.7, Source::Detector),
            ("cat", 0.7, Source::Detector),
            ("bird", 0.7, Source::Pack),
            ("tree", 0.7, Source::Detector),
        ]);
        let mut reversed = found.clone();
        reversed.reverse();
        let first = merge(&found, &[], Caps::default());
        let second = merge(&reversed, &[], Caps::default());
        assert_eq!(first, second, "a tie is broken by source rank then name, never by input order");
    }

    #[test]
    fn a_location_travels_with_the_suggestion() {
        let found = vec![
            Candidate::new("dog", 0.6, Source::Detector),
            Candidate::at("dog", 0.8, Source::Detector, Box::new(0.1, 0.1, 0.2, 0.2)),
            Candidate::at("dog", 0.5, Source::Detector, Box::new(0.6, 0.6, 0.1, 0.1)),
        ];
        let merged = merge(&found, &[], Caps::default());
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].found, Some(Box::new(0.1, 0.1, 0.2, 0.2)), "the best box survives");
    }

    #[test]
    fn a_summary_says_how_many_and_which() {
        assert_eq!(summary(&[]), "no suggestions");
        let two = merge(
            &candidates(&[("dog", 0.9, Source::Detector), ("cat", 0.8, Source::Detector)]),
            &[],
            Caps::default(),
        );
        assert_eq!(summary(&two), "2 suggestion(s): dog, cat");
        let many = merge(
            &(0..9)
                .map(|index| Candidate::new(&format!("tag{index}"), 0.9, Source::Detector))
                .collect::<Vec<Candidate>>(),
            &[],
            Caps::default(),
        );
        assert_eq!(summary(&many), "8 suggestion(s): tag0, tag1, tag2, tag3 and 4 more");
    }

    #[test]
    fn a_suggestion_line_names_its_source_and_confidence() {
        let suggestion = Suggestion {
            tag: "dog".into(),
            score: 0.87,
            source: Source::Detector,
            found: None,
        };
        assert_eq!(suggestion.line(), "dog · detector 87%");
        assert_eq!(Source::ExifHint.label(), "camera");
    }

    /// The line is joined from its pieces for every source, so a panel that reads the pieces
    /// words the same claim the report's sentence does — and cannot disagree with it.
    #[test]
    fn the_line_is_joined_from_its_piece() {
        let cases = [
            ("dog", 0.87, Source::Detector, "dog · detector 87%"),
            ("nikon z6", 0.9, Source::ExifHint, "nikon z6 · camera 90%"),
            ("beach", 0.5, Source::Pack, "beach · pack 50%"),
            ("a captioner's words", 0.04, Source::Caption, "a captioner's words · caption 4%"),
        ];
        for (tag, score, source, english) in cases {
            let suggestion = Suggestion {
                tag: tag.into(),
                score,
                source,
                found: None,
            };
            assert_eq!(suggestion.line_kind().tag, tag);
            assert_eq!(suggestion.line_kind().source, source);
            assert!((suggestion.line_kind().percent - score * 100.0).abs() < 0.001);
            assert_eq!(suggestion.line_kind().english(), english, "{tag}");
            assert_eq!(suggestion.line(), english, "the sentence and its pieces disagree");
        }
    }
}
