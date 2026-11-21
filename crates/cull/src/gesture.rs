//! Pointer gestures for culling without the keyboard.
//!
//! A drag is a decision only when it is far enough and mostly vertical: a sideways swipe is someone
//! scrolling the strip or scrubbing the film, and must never flag a photo by accident. The rules are
//! a pure function so the view can show the same verdict while the finger is still down.

#![forbid(unsafe_code)]

/// How far a drag has to travel before it counts, in points.
pub const THRESHOLD: f32 = 84.0;

/// How much more vertical than horizontal a drag must be.
pub const DOMINANCE: f32 = 1.4;

/// What a drag means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Gesture {
    /// Too short, or mostly sideways: do nothing.
    #[default]
    None,
    /// Upward: keep the photo.
    Pick,
    /// Downward: reject it.
    Reject,
}

impl Gesture {
    /// The decision this gesture stages, once it is committed.
    pub const fn decision(self) -> Option<super::Decision> {
        match self {
            Self::None => None,
            Self::Pick => Some(super::Decision::Pick),
            Self::Reject => Some(super::Decision::Reject),
        }
    }

    /// Label for the overlay.
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Pick => "Keep",
            Self::Reject => "Reject",
        }
    }
}

/// Classify a drag by its offset from where it started, in screen points.
///
/// `delta` is positive to the right and positive downward, the way a pointer reports it, so a drag
/// toward the top of the screen is negative.
pub fn classify(delta: (f32, f32)) -> Gesture {
    let (dx, dy) = (delta.0.abs(), delta.1.abs());
    if dy < THRESHOLD || dx * DOMINANCE > dy {
        return Gesture::None;
    }
    if delta.1 < 0.0 { Gesture::Pick } else { Gesture::Reject }
}

/// How far the drag has travelled toward the threshold, clamped to 0.0–1.0.
pub fn progress(delta: (f32, f32)) -> f32 {
    let dy = delta.1.abs();
    let dx = delta.0.abs();
    if dx * DOMINANCE > dy {
        return 0.0;
    }
    (dy / THRESHOLD).clamp(0.0, 1.0)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_short_drag_means_nothing() {
        assert_eq!(classify((0.0, -20.0)), Gesture::None);
        assert_eq!(classify((0.0, 83.0)), Gesture::None);
        assert_eq!(progress((0.0, 42.0)), 0.5);
    }

    #[test]
    fn direction_chooses_the_decision() {
        assert_eq!(classify((0.0, -THRESHOLD)), Gesture::Pick);
        assert_eq!(classify((0.0, THRESHOLD)), Gesture::Reject);
        assert_eq!(Gesture::Pick.decision(), Some(super::super::Decision::Pick));
        assert_eq!(Gesture::Reject.decision(), Some(super::super::Decision::Reject));
        assert!(Gesture::None.decision().is_none());
        assert_eq!(Gesture::Pick.label(), "Keep");
    }

    #[test]
    fn a_sideways_drag_is_never_a_decision() {
        assert_eq!(classify((200.0, -100.0)), Gesture::None, "mostly horizontal");
        assert_eq!(classify((-150.0, 150.0)), Gesture::None, "diagonal, ambiguous");
        assert_eq!(progress((200.0, -100.0)), 0.0);
        assert_eq!(classify((60.0, -120.0)), Gesture::Pick, "clearly vertical");
    }

    #[test]
    fn progress_never_exceeds_one() {
        assert_eq!(progress((0.0, -10_000.0)), 1.0);
        assert_eq!(progress((0.0, 0.0)), 0.0);
    }
}
