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
    let mut parents: Vec<usize> = (0..members.len()).collect();
    let mut tree = BkTree::default();
    for (index, member) in members.iter().enumerate() {
        for candidate in tree.search(member.hash.difference, threshold) {
            if matches(&members[candidate], member, threshold, colour) {
                union(&mut parents, index, candidate);
            }
        }
        tree.insert(member.hash.difference, index);
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
}
