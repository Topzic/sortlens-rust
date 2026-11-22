//! Boxes, label maps and non-maximum suppression.
//!
//! A detector's raw output is a pile of overlapping guesses. Everything here is a pure function over
//! numbers, so the rules can be tested with synthetic boxes and no model at all — which is the only way
//! to know whether a suggestion is wrong because of the network or because of the arithmetic.
//!
//! Coordinates are **normalised** (0..1 across the image) rather than pixels: a model can be fed a 320
//! px or a 640 px copy of the same photo, and the boxes must mean the same thing either way.

use std::cmp::Ordering;

/// A rectangle in normalised image coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Box {
    /// Left edge, 0 at the left of the frame.
    pub x: f32,
    /// Top edge, 0 at the top of the frame.
    pub y: f32,
    /// Width, as a fraction of the frame.
    pub w: f32,
    /// Height, as a fraction of the frame.
    pub h: f32,
}

impl Box {
    /// A box from its edges.
    #[must_use]
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Box {
        Box { x, y, w, h }
    }

    /// A box from two corners, in any order.
    #[must_use]
    pub fn from_corners(x1: f32, y1: f32, x2: f32, y2: f32) -> Box {
        Box {
            x: x1.min(x2),
            y: y1.min(y2),
            w: (x2 - x1).abs(),
            h: (y2 - y1).abs(),
        }
    }

    /// Area, in frame fractions.
    #[must_use]
    pub fn area(&self) -> f32 {
        (self.w.max(0.0)) * (self.h.max(0.0))
    }

    /// The overlap of two boxes as a share of their union: 0 when apart, 1 when identical.
    #[must_use]
    pub fn iou(&self, other: &Box) -> f32 {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = (self.x + self.w).min(other.x + other.w);
        let bottom = (self.y + self.h).min(other.y + other.h);
        let overlap = (right - left).max(0.0) * (bottom - top).max(0.0);
        let union = self.area() + other.area() - overlap;
        if union <= 0.0 {
            // Two degenerate boxes: nothing to compare. Treating them as identical would let a broken
            // detector's zero-size output suppress everything.
            return 0.0;
        }
        (overlap / union).clamp(0.0, 1.0)
    }

    /// Whether the box has no area, or an area a NaN made meaningless.
    #[must_use]
    pub fn is_degenerate(&self) -> bool {
        // `matches!` rather than `area() <= 0.0`, because that is false for a NaN and a NaN box is
        // exactly the broken output this is here to catch.
        !matches!(self.area().partial_cmp(&0.0), Some(Ordering::Greater))
    }
}

/// One guess from a detector.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    /// Index into the model's label map.
    pub class: u16,
    /// Confidence, 0..1.
    pub score: f32,
    /// Where it is.
    pub found: Box,
}

impl Detection {
    /// A guess.
    #[must_use]
    pub fn new(class: u16, score: f32, found: Box) -> Detection {
        Detection { class, score, found }
    }
}

/// How much two boxes may overlap before the weaker one is dropped.
///
/// 0.45 is what the YOLO family's own post-processing uses; changing it changes how many boxes a
/// crowd produces, so it is a constant with a name rather than a literal at a call site.
pub const IOU_THRESHOLD: f32 = 0.45;

/// A confidence below this is noise: below it, a detector is describing the background.
pub const MIN_SCORE: f32 = 0.25;

/// Sort key that never calls `partial_cmp` on a NaN and never reorders equal scores.
///
/// Recall is filtered for NaN before this runs; the tie-breakers are what make a run reproducible.
fn by_score(a: &Detection, b: &Detection) -> Ordering {
    b.score
        .total_cmp(&a.score)
        .then_with(|| a.class.cmp(&b.class))
        .then_with(|| a.found.y.total_cmp(&b.found.y))
        .then_with(|| a.found.x.total_cmp(&b.found.x))
}

/// Non-maximum suppression, per class.
///
/// Per class is the part that matters: a person standing in front of a car overlaps it almost
/// completely, and suppressing across classes would delete one of them. Inside a class, the highest
/// scoring box wins and anything overlapping it above `iou_threshold` goes.
///
/// Boxes below `min_score`, with a NaN score, or with no area are dropped before anything is compared.
#[must_use]
pub fn suppress(detections: &[Detection], iou_threshold: f32, min_score: f32) -> Vec<Detection> {
    let mut kept: Vec<Detection> = Vec::new();
    let mut candidates: Vec<Detection> = detections
        .iter()
        .filter(|detection| detection.score.is_finite())
        .filter(|detection| detection.score >= min_score)
        .filter(|detection| !detection.found.is_degenerate())
        .cloned()
        .collect();
    candidates.sort_by(by_score);

    let mut classes: Vec<u16> = candidates.iter().map(|detection| detection.class).collect();
    classes.sort_unstable();
    classes.dedup();

    for class in classes {
        let mut survivors: Vec<Detection> = Vec::new();
        for candidate in candidates.iter().filter(|detection| detection.class == class) {
            let overlaps = survivors.iter().any(|kept| kept.found.iou(&candidate.found) > iou_threshold);
            if !overlaps {
                survivors.push(candidate.clone());
            }
        }
        kept.extend(survivors);
    }
    // A stable, comparable order for the caller and for tests: best first, then by class and place.
    kept.sort_by(by_score);
    kept
}

/// The names a detector's class indices mean.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LabelMap {
    labels: Vec<String>,
}

impl LabelMap {
    /// Parse a label map: one name per line, `index name` or `index: name`, `#` for comments.
    ///
    /// Indices must be dense from zero — a map with a hole is a model that will mislabel everything
    /// after the hole, and guessing is worse than refusing.
    pub fn parse(text: &str) -> Result<LabelMap, String> {
        let mut labels: Vec<String> = Vec::new();
        for (line_number, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (index, name) = match split_index(line) {
                Some(pair) => pair,
                // A bare line is a name: the whole line, already trimmed of surrounding whitespace.
                None => (labels.len(), line.to_string()),
            };
            if index != labels.len() {
                return Err(format!(
                    "line {}: index {index} does not follow {} — the label map has a hole",
                    line_number + 1,
                    labels.len()
                ));
            }
            labels.push(name);
        }
        if labels.is_empty() {
            return Err("the label map is empty".to_string());
        }
        Ok(LabelMap { labels })
    }

    /// Every label, in index order.
    #[must_use]
    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// The name for a class index, when the model has one.
    #[must_use]
    pub fn name(&self, class: u16) -> Option<&str> {
        self.labels.get(usize::from(class)).map(String::as_str)
    }

    /// How many classes the map describes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.labels.len()
    }

    /// Whether the map describes nothing, which `parse` never returns.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }
}

/// Split `"3 car"` or `"3: car"` into an index and a name, or say it is a bare name.
fn split_index(line: &str) -> Option<(usize, String)> {
    let (head, tail) = match line.find(|character: char| !character.is_ascii_digit()) {
        Some(0) | None => return None,
        Some(index) => line.split_at(index),
    };
    let name = tail.trim_start_matches([':', ' ', '\t']).trim();
    if name.is_empty() {
        return None;
    }
    head.parse::<usize>().ok().map(|index| (index, name.to_string()))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    fn detection(class: u16, score: f32, box_: Box) -> Detection {
        Detection::new(class, score, box_)
    }

    #[test]
    fn iou_is_zero_apart_one_identical_and_half_overlapped() {
        let a = Box::new(0.0, 0.0, 0.5, 0.5);
        let far = Box::new(0.6, 0.6, 0.2, 0.2);
        assert_eq!(a.iou(&far), 0.0);
        assert!((a.iou(&a) - 1.0).abs() < 1e-6);
        // Half of `a` overlapped by a box of the same size: intersection 0.25, union 0.75.
        let half = Box::new(0.25, 0.0, 0.5, 0.5);
        assert!((a.iou(&half) - (0.25f32 / 0.75)).abs() < 1e-6, "{}", a.iou(&half));
    }

    #[test]
    fn a_zero_area_box_cannot_suppress_anything() {
        let flat = Box::new(0.2, 0.2, 0.0, 0.0);
        let real = Box::new(0.0, 0.0, 1.0, 1.0);
        assert!(flat.is_degenerate());
        assert_eq!(flat.iou(&real), 0.0);
        // A broken detector that emits 1000 empty boxes must not erase the one real one.
        let mut detections: Vec<Detection> = (0..1000).map(|_| detection(0, 0.99, flat)).collect();
        detections.push(detection(0, 0.5, real));
        let kept = suppress(&detections, IOU_THRESHOLD, MIN_SCORE);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].found, real, "the real box survived the debris");
    }

    #[test]
    fn overlapping_boxes_of_one_class_collapse_to_the_best() {
        let good = detection(0, 0.9, Box::new(0.0, 0.0, 0.4, 0.4));
        let also_good = detection(0, 0.8, Box::new(0.02, 0.02, 0.4, 0.4));
        let elsewhere = detection(0, 0.7, Box::new(0.5, 0.5, 0.2, 0.2));
        let kept = suppress(&[also_good, good.clone(), elsewhere.clone()], IOU_THRESHOLD, MIN_SCORE);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].found, good.found, "the highest score comes first");
        assert_eq!(kept[1].found, elsewhere.found);
    }

    #[test]
    fn a_person_in_front_of_a_car_keeps_both() {
        // The case cross-class suppression gets wrong.
        let person = detection(0, 0.9, Box::new(0.1, 0.1, 0.5, 0.8));
        let car = detection(2, 0.85, Box::new(0.1, 0.1, 0.5, 0.8));
        let kept = suppress(&[person, car], IOU_THRESHOLD, MIN_SCORE);
        assert_eq!(kept.len(), 2, "suppression is per class");
        assert_eq!(kept[0].class, 0);
        assert_eq!(kept[1].class, 2);
    }

    #[test]
    fn weak_guesses_and_nonsense_are_dropped_before_comparison() {
        let strong = detection(0, 0.9, Box::new(0.0, 0.0, 0.2, 0.2));
        let weak = detection(1, 0.1, Box::new(0.5, 0.5, 0.2, 0.2));
        let nan = detection(2, f32::NAN, Box::new(0.7, 0.7, 0.2, 0.2));
        let kept = suppress(&[weak, nan, strong], IOU_THRESHOLD, MIN_SCORE);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].class, 0);
    }

    #[test]
    fn an_empty_input_is_not_an_error() {
        assert!(suppress(&[], IOU_THRESHOLD, MIN_SCORE).is_empty());
    }

    #[test]
    fn the_threshold_decides_how_many_boxes_a_crowd_keeps() {
        let a = detection(0, 0.9, Box::new(0.0, 0.0, 0.4, 0.4));
        let b = detection(0, 0.8, Box::new(0.2, 0.0, 0.4, 0.4)); // IoU = 1/7 ≈ 0.143
        assert_eq!(suppress(&[a.clone(), b.clone()], 0.5, MIN_SCORE).len(), 2, "apart enough");
        assert_eq!(suppress(&[a, b], 0.1, MIN_SCORE).len(), 1, "one is enough");
    }

    #[test]
    fn equal_scores_still_produce_the_same_order_every_run() {
        let detections: Vec<Detection> = (0..8u16)
            .map(|index| detection(index % 2, 0.5, Box::new(f32::from(index) * 0.1, 0.0, 0.05, 0.05)))
            .collect();
        let first = suppress(&detections, IOU_THRESHOLD, MIN_SCORE);
        let mut shuffled = detections.clone();
        shuffled.reverse();
        let second = suppress(&shuffled, IOU_THRESHOLD, MIN_SCORE);
        assert_eq!(first, second, "ties are broken by class and position, not by input order");
    }

    #[test]
    fn a_label_map_parses_both_forms_and_ignores_comments() {
        let map = LabelMap::parse("# coco\n0 person\n1: bicycle\ncar\n\n  # trailers\n3 dog  \n").expect("parse");
        assert_eq!(map.len(), 4);
        assert_eq!(map.name(0), Some("person"));
        assert_eq!(map.name(1), Some("bicycle"));
        assert_eq!(map.name(2), Some("car"));
        assert_eq!(map.name(3), Some("dog"), "an indexed name is trimmed of its separator");
        assert_eq!(map.name(4), None);
        assert!(!map.is_empty());
        assert_eq!(map.labels()[0], "person");
    }

    #[test]
    fn a_label_map_with_a_hole_is_refused() {
        let error = LabelMap::parse("0 person\n2 car\n").expect_err("hole");
        assert!(error.contains("does not follow"), "{error}");
    }

    #[test]
    fn an_empty_label_map_is_refused() {
        assert!(LabelMap::parse("").is_err());
        assert!(LabelMap::parse("# only comments\n\n").is_err());
    }

    #[test]
    fn an_index_without_a_name_is_read_as_a_bare_label() {
        // "7" alone is a name, not an index with nothing after it.
        let map = LabelMap::parse("person\n7\n").expect("parse");
        assert_eq!(map.name(1), Some("7"));
    }

    #[test]
    fn boxes_can_be_built_from_corners_in_either_order() {
        let a = Box::from_corners(0.4, 0.6, 0.1, 0.2);
        assert!((a.x - 0.1).abs() < 1e-6 && (a.y - 0.2).abs() < 1e-6);
        // Float subtraction of 0.6 - 0.2 is not exactly 0.4, which is why boxes are compared with a
        // tolerance and never with `==`.
        assert!((a.w - 0.3).abs() < 1e-6 && (a.h - 0.4).abs() < 1e-6, "{a:?}");
        assert!((a.area() - 0.12).abs() < 1e-6);
    }
}
