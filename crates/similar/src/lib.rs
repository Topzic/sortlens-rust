//! Similar-photo grouping.
//!
//! The library hands over identities and hashes; this crate decides which photos belong together and
//! which one of each group is worth keeping. Grouping is pairwise and deterministic: the same inputs
//! in the same order always produce the same groups, so a refresh never reshuffles the view.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use std::collections::HashMap;

use sortlens_core::PhotoId;
use sortlens_hash::{Colour, Hash};

/// Distance under which two photos count as the same subject, in bits of 64.
pub const DEFAULT_THRESHOLD: u32 = 10;

/// Colour tolerance above which two photos are treated as different pictures, 0–255 per channel.
pub const DEFAULT_COLOUR: u32 = 42;

/// One cluster of near-duplicates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// Members, best first.
    pub members: Vec<Member>,
    /// The photo worth keeping.
    pub best: PhotoId,
    /// The largest distance between the best photo and any other member.
    pub spread: u32,
}

impl Group {
    /// How many photos the group holds.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Whether the group is empty; never true for a group that came out of [`group`].
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Every photo except the best one, which is what a cleanup would set aside.
    pub fn others(&self) -> Vec<PhotoId> {
        self.members.iter().map(|member| member.id).filter(|id| *id != self.best).collect()
    }
}

/// What the grouping needs to know about one photo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Member {
    /// Catalog identity.
    pub id: PhotoId,
    /// Perceptual hash.
    pub hash: Hash,
    /// Colour fingerprint, when the preview was decoded far enough to measure one.
    pub colour: Option<Colour>,
    /// Pixel count, used to prefer the sharper original.
    pub pixels: u64,
    /// Stars, used as the tie-break after size.
    pub rating: u8,
    /// File size, the last tie-break.
    pub bytes: u64,
}

/// Which signals a grouping trusts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Match {
    /// Structure close, colour agreeing when both sides have a palette. The default.
    #[default]
    Families,
    /// Palette alone: the same scene, differently framed, cropped or resized, where the layout hash
    /// disagrees but the colours do not.
    Colour,
    /// Either signal: the widest net, and the one that finds both kinds in one pass.
    Either,
}

impl Match {
    /// Stable name for the command layer.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Families => "families",
            Self::Colour => "colour",
            Self::Either => "either",
        }
    }

    /// Name for the picker.
    pub const fn label(self) -> &'static str {
        self.word().en()
    }

    /// One line explaining what it lets through.
    pub const fn hint(self) -> &'static str {
        self.sense().en()
    }

    /// The picker's word, as the piece a screen is written from rather than the English text.
    ///
    /// A surface that translates by piece matches on this and cannot go stale: the sentence this
    /// crate answers and the one a catalogue paints are the same piece, and a test in the shell
    /// fails when its literal drifts from this crate's.
    pub const fn word(self) -> MatchWord {
        match self {
            Self::Families => MatchWord::Structure,
            Self::Colour => MatchWord::Colour,
            Self::Either => MatchWord::Either,
        }
    }

    /// What the mode lets through, as the piece the line under the picker is written from.
    pub const fn sense(self) -> MatchSense {
        match self {
            Self::Families => MatchSense::SafeDefault,
            Self::Colour => MatchSense::PaletteAnywhere,
            Self::Either => MatchSense::WidestNet,
        }
    }

    /// Every mode, in picker order.
    pub const ALL: [Self; 3] = [Self::Families, Self::Colour, Self::Either];

    /// Parse a name from the command layer.
    pub fn parse(name: &str) -> Self {
        match name {
            "colour" => Self::Colour,
            "either" => Self::Either,
            _ => Self::Families,
        }
    }
}

/// The picker's word for a mode: the piece a translated picker is built from, not the English
/// text itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchWord {
    /// Structure close, colour agreeing when both sides have a palette.
    Structure,
    /// Palette alone, whatever the layout.
    Colour,
    /// Either signal.
    Either,
}

impl MatchWord {
    /// The English word, which is what [`Match::label`] answers.
    pub const fn en(self) -> &'static str {
        match self {
            Self::Structure => "Structure",
            Self::Colour => "Colour",
            Self::Either => "Either",
        }
    }
}

/// What a mode lets through: the piece the line under the picker is written from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchSense {
    /// Layout and palette both agree.
    SafeDefault,
    /// Palette agrees anywhere in the frame.
    PaletteAnywhere,
    /// Either signal — the widest net.
    WidestNet,
}

impl MatchSense {
    /// The English sentence, which is what [`Match::hint`] answers.
    pub const fn en(self) -> &'static str {
        match self {
            Self::SafeDefault => "Same layout and the same palette — the safe default.",
            Self::PaletteAnywhere => "Same palette anywhere in the frame, whatever the layout.",
            Self::WidestNet => "Structure or palette; the widest net, more false pairs.",
        }
    }
}

/// Whether two members share a palette within `colour`.
///
/// Both sides need a fingerprint: colour-only matching refuses to guess when a preview could not be
/// measured.
pub fn palette_matches(left: &Member, right: &Member, colour: u32) -> bool {
    match (left.colour, right.colour) {
        (Some(left), Some(right)) => left.distance(right) <= colour,
        _ => false,
    }
}

/// Whether two members are the same picture under both signals.
///
/// Structure has to be close, and colour has to agree when both sides have a fingerprint: a page of
/// text and a striped shirt have the same light and dark layout but never the same palette.
pub fn matches(left: &Member, right: &Member, threshold: u32, colour: u32) -> bool {
    if left.hash.distance(right.hash) > threshold {
        return false;
    }
    match (left.colour, right.colour) {
        (Some(left), Some(right)) => left.distance(right) <= colour,
        _ => true,
    }
}

/// Group near-duplicates, largest group first, then by the best member's id so the order is stable.
///
/// Photos are chained: if A matches B and B matches C, all three are one group even when A and C are
/// further apart than the threshold. That is the useful reading for a burst, where the frames at
/// either end drift a little from each other.
pub fn group(members: &[Member], threshold: u32) -> Vec<Group> {
    group_with(members, threshold, DEFAULT_COLOUR)
}

/// Group with an explicit colour tolerance.
///
/// The search is exact and does not compare every pair: candidates come from a BK-tree over the
/// 64-bit hash — a metric index on Hamming distance, so a radius-r search only visits nodes whose
/// edge ranges can still reach r. A ten-thousand-photo library needs a few thousand comparisons
/// rather than fifty million, while still finding every pair inside the radius.
pub fn group_with(members: &[Member], threshold: u32, colour: u32) -> Vec<Group> {
    group_in(members, Match::Families, threshold, colour)
}

/// Group under an explicit set of signals.
///
/// Structure pairs come from the BK-tree, palette pairs from a brightness-sorted sliding window: if
/// two palettes differ in mean brightness by more than the tolerance they cannot be within it, so
/// sorting by brightness and stopping the scan at that distance is exact rather than approximate.
pub fn group_in(members: &[Member], mode: Match, threshold: u32, colour: u32) -> Vec<Group> {
    let mut parents: Vec<usize> = (0..members.len()).collect();
    if matches!(mode, Match::Families | Match::Either) {
        let mut tree = BkTree::default();
        for (index, member) in members.iter().enumerate() {
            for candidate in tree.search(member.hash.difference, threshold) {
                if matches(&members[candidate], member, threshold, colour) {
                    union(&mut parents, index, candidate);
                }
            }
            tree.insert(member.hash.difference, index);
        }
    }
    if matches!(mode, Match::Colour | Match::Either) {
        let mut ordered: Vec<(u32, usize)> = members
            .iter()
            .enumerate()
            .filter_map(|(index, member)| member.colour.map(|palette| (palette.mean(), index)))
            .collect();
        ordered.sort_unstable();
        for (position, (brightness, index)) in ordered.iter().enumerate() {
            for (other_brightness, other) in ordered.iter().skip(position + 1) {
                if other_brightness.saturating_sub(*brightness) > colour {
                    break;
                }
                if palette_matches(&members[*other], &members[*index], colour) {
                    union(&mut parents, *index, *other);
                }
            }
        }
    }

    let mut buckets: HashMap<usize, Vec<usize>> = HashMap::new();
    for index in 0..members.len() {
        let root = find(&mut parents, index);
        buckets.entry(root).or_default().push(index);
    }

    let mut groups: Vec<Group> = buckets
        .into_values()
        .filter(|bucket| bucket.len() > 1)
        .map(|bucket| build(members, &bucket))
        .collect();
    groups.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.best.cmp(&right.best)));
    groups
}

/// Photos within the threshold of one chosen photo, for "find similar to this".
pub fn around(members: &[Member], target: PhotoId, threshold: u32) -> Option<Group> {
    let anchor = members.iter().find(|member| member.id == target)?;
    let bucket: Vec<usize> = members
        .iter()
        .enumerate()
        .filter(|(_, member)| matches(anchor, member, threshold, DEFAULT_COLOUR))
        .map(|(index, _)| index)
        .collect();
    if bucket.len() < 2 {
        return Some(Group {
            members: vec![*anchor],
            best: anchor.id,
            spread: 0,
        });
    }
    Some(build(members, &bucket))
}

fn build(members: &[Member], bucket: &[usize]) -> Group {
    let mut sorted: Vec<Member> = bucket.iter().map(|index| members[*index]).collect();
    // Best first: resolution, then stars, then bytes, then the older id so the pick is stable.
    sorted.sort_by(|left, right| {
        right
            .pixels
            .cmp(&left.pixels)
            .then_with(|| right.rating.cmp(&left.rating))
            .then_with(|| right.bytes.cmp(&left.bytes))
            .then_with(|| left.id.cmp(&right.id))
    });
    let best = sorted
        .first()
        .map(|member| member.hash)
        .unwrap_or(Hash { difference: 0, average: 0 });
    let spread = sorted.iter().map(|member| member.hash.distance(best)).max().unwrap_or(0);
    Group {
        best: sorted.first().map(|member| member.id).unwrap_or(PhotoId(0)),
        members: sorted,
        spread,
    }
}

/// A vantage-point tree over 64-bit hashes, ordered by Hamming distance.
///
/// Every node keeps one hash and the indices of members that share it. Children are split by the
/// distance to that hash, which is what lets a search skip whole subtrees: Hamming distance obeys the
/// triangle inequality, so anything at distance `d` from the node can only be within `r` of the query
/// when `d` sits in `[q - r, q + r]`.
#[derive(Debug, Default)]
pub struct BkTree {
    root: Option<Box<Node>>,
}

#[derive(Debug)]
struct Node {
    hash: u64,
    members: Vec<usize>,
    children: Vec<(u32, Box<Node>)>,
}

impl BkTree {
    /// Whether the tree holds anything.
    pub fn is_empty(&self) -> bool {
        self.root.is_none()
    }

    /// Add a member index under its hash.
    pub fn insert(&mut self, hash: u64, index: usize) {
        match &mut self.root {
            None => {
                self.root = Some(Box::new(Node {
                    hash,
                    members: vec![index],
                    children: Vec::new(),
                }));
            }
            Some(node) => node.insert(hash, index),
        }
    }

    /// Every member index whose hash is within `radius` bits of `query`.
    pub fn search(&self, query: u64, radius: u32) -> Vec<usize> {
        let mut found = Vec::new();
        if let Some(node) = &self.root {
            node.search(query, radius, &mut found);
        }
        found
    }
}

impl Node {
    fn insert(&mut self, hash: u64, index: usize) {
        if hash == self.hash {
            self.members.push(index);
            return;
        }
        let distance = sortlens_hash::hamming(hash, self.hash);
        if let Some((_, child)) = self.children.iter_mut().find(|(edge, _)| *edge == distance) {
            child.insert(hash, index);
            return;
        }
        self.children.push((
            distance,
            Box::new(Node {
                hash,
                members: vec![index],
                children: Vec::new(),
            }),
        ));
    }

    fn search(&self, query: u64, radius: u32, found: &mut Vec<usize>) {
        let distance = sortlens_hash::hamming(query, self.hash);
        if distance <= radius {
            found.extend_from_slice(&self.members);
        }
        let low = distance.saturating_sub(radius);
        let high = distance.saturating_add(radius);
        for (edge, child) in &self.children {
            if *edge >= low && *edge <= high {
                child.search(query, radius, found);
            }
        }
    }
}

fn find(parents: &mut [usize], mut index: usize) -> usize {
    while parents[index] != index {
        parents[index] = parents[parents[index]];
        index = parents[index];
    }
    index
}

fn union(parents: &mut [usize], left: usize, right: usize) {
    let left_root = find(parents, left);
    let right_root = find(parents, right);
    if left_root != right_root {
        // Keep the smaller index as the root so the result does not depend on iteration order.
        let (low, high) = if left_root < right_root {
            (left_root, right_root)
        } else {
            (right_root, left_root)
        };
        parents[high] = low;
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn member(id: u64, difference: u64, pixels: u64, rating: u8) -> Member {
        Member {
            id: PhotoId(id),
            hash: Hash { difference, average: 0 },
            colour: None,
            pixels,
            rating,
            bytes: pixels,
        }
    }

    fn coloured(id: u64, difference: u64, red: u8) -> Member {
        Member {
            colour: Some(Colour {
                red: [red; 9],
                green: [red; 9],
                blue: [red; 9],
            }),
            ..member(id, difference, 10, 0)
        }
    }

    /// The picker's word and the hint are the pieces, not a second table: a screen that translates
    /// by piece cannot drift from what `label`/`hint` answer, every mode names itself once, and no
    /// two modes can answer the same sentence.
    #[test]
    fn every_mode_answers_through_its_own_piece() {
        for mode in Match::ALL {
            assert_eq!(mode.label(), mode.word().en(), "{mode:?}");
            assert_eq!(mode.hint(), mode.sense().en(), "{mode:?}");
        }
        assert_eq!(
            Match::ALL.map(Match::word),
            [MatchWord::Structure, MatchWord::Colour, MatchWord::Either]
        );
        assert_eq!(
            Match::ALL.map(Match::sense),
            [MatchSense::SafeDefault, MatchSense::PaletteAnywhere, MatchSense::WidestNet]
        );
        assert_ne!(MatchWord::Structure.en(), MatchWord::Colour.en());
        assert_ne!(MatchWord::Colour.en(), MatchWord::Either.en());
        assert_ne!(MatchWord::Structure.en(), MatchWord::Either.en());
        assert_ne!(MatchSense::SafeDefault.en(), MatchSense::PaletteAnywhere.en());
        assert_ne!(MatchSense::PaletteAnywhere.en(), MatchSense::WidestNet.en());
        assert_ne!(MatchSense::SafeDefault.en(), MatchSense::WidestNet.en());
    }

    #[test]
    fn identical_hashes_form_one_group() {
        let members = vec![member(1, 0, 100, 0), member(2, 0, 200, 0), member(3, u64::MAX, 50, 0)];
        let groups = group(&members, DEFAULT_THRESHOLD);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
        assert_eq!(groups[0].best, PhotoId(2), "the larger file wins");
        assert_eq!(groups[0].others(), vec![PhotoId(1)]);
        assert_eq!(groups[0].spread, 0);
    }

    #[test]
    fn a_chain_of_close_matches_is_one_group() {
        // 1 ~ 2 and 2 ~ 3, while 1 and 3 are further apart than the threshold.
        let members = vec![member(1, 0b0000, 10, 0), member(2, 0b0011, 10, 0), member(3, 0b1111, 10, 0)];
        let groups = group(&members, 3);
        assert_eq!(groups.len(), 1, "chained matches stay together");
        assert_eq!(groups[0].len(), 3);
        assert!(groups[0].spread <= 6);
    }

    #[test]
    fn rating_breaks_a_size_tie() {
        let members = vec![member(1, 0, 100, 1), member(2, 0, 100, 5)];
        let groups = group(&members, 0);
        assert_eq!(groups[0].best, PhotoId(2), "the better rated frame is the pick");
    }

    #[test]
    fn singletons_are_not_groups() {
        let members = vec![member(1, 0, 10, 0), member(2, u64::MAX, 10, 0)];
        assert!(group(&members, 4).is_empty());
        assert_eq!(group(&members, 64).len(), 1, "a loose threshold merges everything");
    }

    #[test]
    fn grouping_is_stable_across_input_order() {
        let members = vec![
            member(1, 0, 10, 0),
            member(2, 0x00ff, 20, 0),
            member(3, u64::MAX, 30, 0),
            member(4, 0xff00, 40, 0),
        ];
        let forwards = group(&members, 8);
        let mut reversed = members.clone();
        reversed.reverse();
        let backwards = group(&reversed, 8);
        let ids = |groups: &[Group]| groups.iter().map(|group| group.best).collect::<Vec<_>>();
        assert_eq!(ids(&forwards), ids(&backwards));
    }

    #[test]
    fn the_index_finds_every_pair_inside_the_radius() {
        let mut tree = BkTree::default();
        let hashes: Vec<u64> = (0..64u64).map(|index| index.wrapping_mul(0x9E37_79B9_7F4A_7C15)).collect();
        for (index, hash) in hashes.iter().enumerate() {
            tree.insert(*hash, index);
        }
        assert!(!tree.is_empty());
        let query = hashes[7] ^ 0b1011;
        let mut found = tree.search(query, 4);
        found.sort_unstable();
        let mut expected: Vec<usize> = hashes
            .iter()
            .enumerate()
            .filter(|(_, hash)| sortlens_hash::hamming(query, **hash) <= 4)
            .map(|(index, _)| index)
            .collect();
        expected.sort_unstable();
        assert_eq!(found, expected, "the index agrees with brute force");
    }

    #[test]
    fn grouping_can_refuse_a_colour_mismatch() {
        // Same structure, opposite palettes: the structure hash alone would group them.
        let members = vec![coloured(1, 0, 220), coloured(2, 0, 20)];
        assert!(
            group(&members, 0).is_empty(),
            "identical structure in another palette is a different picture"
        );
        assert!(group_with(&members, 0, 40).is_empty(), "a tight tolerance still refuses it");
        assert_eq!(group_with(&members, 0, 255).len(), 1, "a loose tolerance allows it");
        // Without a fingerprint there is nothing to contradict the hash.
        let blind = vec![member(1, 0, 10, 0), member(2, 0, 10, 0)];
        assert_eq!(group(&blind, 0).len(), 1);
    }

    #[test]
    fn around_finds_the_neighbours_of_one_photo() {
        let members = vec![member(1, 0, 10, 0), member(2, 1, 10, 0), member(3, u64::MAX, 10, 0)];
        let found = around(&members, PhotoId(1), 4).expect("anchor exists");
        assert_eq!(found.len(), 2);
        assert!(found.others().contains(&PhotoId(2)));
        assert!(around(&members, PhotoId(9), 4).is_none());

        let lonely = around(&members, PhotoId(3), 1).expect("anchor exists");
        assert_eq!(lonely.len(), 1);
        assert_eq!(lonely.best, PhotoId(3));
    }

    fn member_with_colour(id: u64, difference: u64, rgb: [u8; 3]) -> Member {
        Member {
            id: PhotoId(id),
            hash: Hash {
                difference,
                average: 0x0F0F_0F0F_0F0F_0F0F,
            },
            colour: Some(Colour {
                red: [rgb[0]; 9],
                green: [rgb[1]; 9],
                blue: [rgb[2]; 9],
            }),
            pixels: 1_000,
            rating: 0,
            bytes: 100,
        }
    }

    #[test]
    fn colour_only_finds_the_same_palette_whatever_the_layout() {
        // Two very different layouts — the structure hash disagrees — but the same palette.
        let dark = member_with_colour(1, 0, [20, 20, 20]);
        let light = member_with_colour(2, 0xFFFF_FFFF_FFFF_FFFF, [22, 21, 20]);
        let members = [dark, light];

        assert!(
            group_in(&members, Match::Families, 10, 40).is_empty(),
            "structure alone refuses them"
        );
        let by_colour = group_in(&members, Match::Colour, 10, 40);
        assert_eq!(by_colour.len(), 1, "the palette agrees");
        assert_eq!(by_colour[0].len(), 2);
        assert_eq!(group_in(&members, Match::Either, 10, 40).len(), 1);
    }

    #[test]
    fn colour_only_keeps_different_palettes_apart() {
        let red = member_with_colour(1, 0, [200, 30, 30]);
        let blue = member_with_colour(2, 0, [30, 30, 200]);
        let members = [red, blue];
        assert!(group_in(&members, Match::Colour, 10, 40).is_empty());
        assert_eq!(
            group_in(&members, Match::Families, 0, 40).len(),
            0,
            "identical structure but a different palette is not the same photo"
        );
    }

    #[test]
    fn colour_only_needs_a_palette_on_both_sides() {
        let mut measured = member_with_colour(1, 0, [90, 90, 90]);
        let mut unmeasured = member_with_colour(2, 0, [90, 90, 90]);
        unmeasured.colour = None;
        assert!(group_in(&[measured, unmeasured], Match::Colour, 0, 40).is_empty());

        // Once the preview can be measured it joins.
        let second = member_with_colour(2, 0, [90, 90, 90]);
        measured.colour = Some(Colour {
            red: [90; 9],
            green: [90; 9],
            blue: [90; 9],
        });
        assert_eq!(group_in(&[measured, second], Match::Colour, 0, 40).len(), 1);
    }

    #[test]
    fn the_brightness_window_does_not_miss_a_pair_it_should_find() {
        // A chain of palettes one step apart: every adjacent pair is inside the tolerance, and the
        // sliding window has to walk far enough to see it.
        let members: Vec<Member> = (0..8)
            .map(|step| {
                let value = 40 + step * 8;
                member_with_colour(step as u64 + 1, step as u64 * 0x1111, [value, value, value])
            })
            .collect();
        let groups = group_in(&members, Match::Colour, 64, 10);
        assert_eq!(groups.len(), 1, "the chain is one group");
        assert_eq!(groups[0].len(), 8);

        // A tighter tolerance than the step breaks every link: nothing is within four per channel.
        assert!(group_in(&members, Match::Colour, 64, 4).is_empty());
        // And at exactly the step, neighbours still chain into one group.
        assert_eq!(group_in(&members, Match::Colour, 64, 8).len(), 1);
    }

    #[test]
    fn mode_names_round_trip() {
        for mode in Match::ALL {
            assert_eq!(Match::parse(mode.id()), mode);
            assert!(!mode.label().is_empty());
            assert!(!mode.hint().is_empty());
        }
        assert_eq!(Match::parse("nonsense"), Match::Families);
    }
}
