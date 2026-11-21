//! Zoom and pan for the cull canvas.
//!
//! The arithmetic lives here rather than in the view so the clamps can be tested: a pan that walked
//! the picture off the screen, or a zoom that shrank below fit, would leave someone staring at an
//! empty panel with no way back.

#![forbid(unsafe_code)]

/// Smallest zoom, which is the fitted picture.
pub const MIN_ZOOM: f32 = 1.0;

/// Largest zoom; past this a 24 MP frame is soft everywhere and the point is lost.
pub const MAX_ZOOM: f32 = 8.0;

/// How much one step of the wheel multiplies the zoom.
pub const WHEEL_STEP: f32 = 1.12;

/// Where the picture sits inside the canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Zoom over the fitted size, 1.0 meaning "fit".
    pub zoom: f32,
    /// Offset from the centre, in screen points.
    pub offset: (f32, f32),
}

impl Default for View {
    fn default() -> Self {
        Self {
            zoom: MIN_ZOOM,
            offset: (0.0, 0.0),
        }
    }
}

impl View {
    /// Whether the picture is zoomed past fit.
    pub fn is_zoomed(&self) -> bool {
        self.zoom > MIN_ZOOM + f32::EPSILON
    }

    /// Back to fit, centred.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Multiply the zoom, keeping the offset inside the new bounds.
    ///
    /// `area` is the canvas size and `fitted` the size the picture would have at zoom 1. Returns
    /// whether anything changed.
    pub fn zoom_by(&mut self, factor: f32, area: (f32, f32), fitted: (f32, f32)) -> bool {
        let wanted = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        if (wanted - self.zoom).abs() < f32::EPSILON {
            return false;
        }
        self.zoom = wanted;
        if !self.is_zoomed() {
            self.offset = (0.0, 0.0);
            return true;
        }
        self.offset = self.clamp(area, fitted);
        true
    }

    /// One wheel step, positive meaning "zoom in".
    pub fn wheel(&mut self, notches: f32, area: (f32, f32), fitted: (f32, f32)) -> bool {
        if notches == 0.0 {
            return false;
        }
        let factor = WHEEL_STEP.powf(notches);
        self.zoom_by(factor, area, fitted)
    }

    /// Move the picture by a drag delta, clamped so it always covers the canvas where possible.
    pub fn pan(&mut self, delta: (f32, f32), area: (f32, f32), fitted: (f32, f32)) -> bool {
        if !self.is_zoomed() {
            return false;
        }
        let before = self.offset;
        self.offset = (before.0 + delta.0, before.1 + delta.1);
        self.offset = self.clamp(area, fitted);
        (self.offset.0 - before.0).abs() > f32::EPSILON || (self.offset.1 - before.1).abs() > f32::EPSILON
    }

    /// The picture's size on screen at the current zoom.
    pub fn size(&self, fitted: (f32, f32)) -> (f32, f32) {
        (fitted.0 * self.zoom, fitted.1 * self.zoom)
    }

    /// Keep the picture from being dragged away from the canvas edge.
    ///
    /// When the zoomed picture is smaller than the canvas on an axis — possible when only one
    /// dimension grew — the offset is forced to zero instead of allowing a gap.
    fn clamp(&self, area: (f32, f32), fitted: (f32, f32)) -> (f32, f32) {
        let size = self.size(fitted);
        let slack_x = ((size.0 - area.0) / 2.0).max(0.0);
        let slack_y = ((size.1 - area.1) / 2.0).max(0.0);
        (self.offset.0.clamp(-slack_x, slack_x), self.offset.1.clamp(-slack_y, slack_y))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    const AREA: (f32, f32) = (800.0, 600.0);
    const FITTED: (f32, f32) = (800.0, 600.0);

    #[test]
    fn a_fresh_view_is_fitted_and_centred() {
        let view = View::default();
        assert!(!view.is_zoomed());
        assert_eq!(view.offset, (0.0, 0.0));
        assert_eq!(view.size(FITTED), FITTED);
    }

    #[test]
    fn zoom_clamps_between_fit_and_the_limit() {
        let mut view = View::default();
        assert!(view.zoom_by(100.0, AREA, FITTED));
        assert_eq!(view.zoom, MAX_ZOOM);
        assert!(view.zoom_by(0.0001, AREA, FITTED));
        assert_eq!(view.zoom, MIN_ZOOM);
        assert_eq!(view.offset, (0.0, 0.0), "zooming back out recentres");
        assert!(!view.zoom_by(0.5, AREA, FITTED), "already at the floor");
    }

    #[test]
    fn panning_stops_at_the_picture_edge() {
        let mut view = View::default();
        view.zoom_by(2.0, AREA, FITTED);
        assert!(view.pan((10_000.0, 10_000.0), AREA, FITTED));
        // At 2x the picture is 1600×1200 over a 800×600 canvas: 400 points of slack each way.
        assert_eq!(view.offset, (400.0, 300.0));
        assert!(!view.pan((50.0, 50.0), AREA, FITTED), "already at the edge");
        assert!(view.pan((-10_000.0, -10_000.0), AREA, FITTED));
        assert_eq!(view.offset, (-400.0, -300.0));
    }

    #[test]
    fn a_pan_does_nothing_while_fitted() {
        let mut view = View::default();
        assert!(!view.pan((100.0, 100.0), AREA, FITTED));
        assert_eq!(view.offset, (0.0, 0.0));
    }

    #[test]
    fn zooming_out_pulls_the_picture_back_into_view() {
        let mut view = View::default();
        view.zoom_by(4.0, AREA, FITTED);
        view.pan((-10_000.0, 0.0), AREA, FITTED);
        let before = view.offset.0;
        view.zoom_by(0.5, AREA, FITTED);
        assert!(view.offset.0 > before, "the offset shrinks with the slack");
        view.reset();
        assert!(!view.is_zoomed());
        assert_eq!(view.offset, (0.0, 0.0));
    }

    #[test]
    fn the_wheel_steps_and_ignores_zero() {
        let mut view = View::default();
        assert!(!view.wheel(0.0, AREA, FITTED));
        assert!(view.wheel(1.0, AREA, FITTED));
        assert!((view.zoom - WHEEL_STEP).abs() < 1e-5);
        assert!(view.wheel(-1.0, AREA, FITTED));
        assert!((view.zoom - MIN_ZOOM).abs() < 1e-5);
    }
}
