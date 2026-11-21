//! Similar-photo grouping.
//!
//! The library hands over identities and hashes; this crate decides which photos belong together and
//! which one of each group is worth keeping. Grouping is pairwise and deterministic: the same inputs
//! in the same order always produce the same groups, so a refresh never reshuffles the view.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use std::collections::HashMap;

use sortlens_core::PhotoId;
use sortlens_hash::Hash;

/// Distance under which two photos count as the same subject, in bits of 64.
pub const DEFAULT_THRESHOLD: u32 = 10;

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
    /// Pixel count, used to prefer the sharper original.
    pub pixels: u64,
    /// Stars, used as the tie-break after size.
    pub rating: u8,
    /// File size, the last tie-break.
    pub bytes: u64,
}

/// Group near-duplicates, largest group first, then by the best member's id so the order is stable.
///
/// Photos are chained: if A matches B and B matches C, all three are one group even when A and C are
/// further apart than the threshold. That is the useful reading for a burst, where the frames at
/// either end drift a little from each other.
pub fn group(members: &[Member], threshold: u32) -> Vec<Group> {
    let mut parents: Vec<usize> = (0..members.len()).collect();
    for left in 0..members.len() {
        for right in left + 1..members.len() {
            if members[left].hash.distance(members[right].hash) <= threshold {
                union(&mut parents, left, right);
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
        .filter(|(_, member)| member.hash.distance(anchor.hash) <= threshold)
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
            pixels,
            rating,
            bytes: pixels,
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
