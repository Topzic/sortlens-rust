//! The two charts the app draws: a column chart and a proportional row.
//!
//! Deliberately small. There is no chart crate behind this and no chart crate planned: the app needs
//! columns with labels, a one-pixel floor so a count of one is still visible, and horizontal shares.
//! Everything here is pure arithmetic over pixel heights, so the parts that are easy to get wrong —
//! scaling to zero, dividing by an empty library, a value that rounds to nothing — are tested without
//! a graphics context.
//!
//! **Heights are fixed pixels, never fractions of the window.** A chart that grows with the window
//! makes a library look different depending on how far the user dragged a panel, and it makes
//! screenshots incomparable. Callers pass a height from a token or a constant here.

#![forbid(unsafe_code)]

use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::theme::Tokens;

/// Height of a column chart, including its baseline.
pub const COLUMN_HEIGHT: f32 = 74.0;

/// Width of one column, including its gap.
pub const COLUMN_WIDTH: f32 = 16.0;

/// Height of one proportional row.
pub const SHARE_HEIGHT: f32 = 15.0;

/// One column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// Label under the column; empty for none.
    pub label: String,
    /// The value.
    pub value: usize,
    /// Draw this column in the accent colour.
    pub highlight: bool,
}

impl Column {
    /// A plain column.
    pub fn new(label: impl Into<String>, value: usize) -> Self {
        Self {
            label: label.into(),
            value,
            highlight: false,
        }
    }

    /// Mark the column as the peak or the current choice.
    pub fn highlighted(mut self) -> Self {
        self.highlight = true;
        self
    }
}

/// Pixel heights for a series, scaled so the tallest value fills `height`.
///
/// A non-zero value never rounds below one pixel: a bar that is invisible is worse than a bar that
/// is one pixel too tall, because the reader concludes there was nothing there.
pub fn heights(values: &[usize], height: f32) -> Vec<f32> {
    let peak = values.iter().copied().max().unwrap_or(0);
    let height = height.max(0.0);
    if peak == 0 {
        return vec![0.0; values.len()];
    }
    values
        .iter()
        .map(|value| {
            if *value == 0 {
                return 0.0;
            }
            // Integer arithmetic first so the peak lands exactly on `height` rather than a float's
            // width away from it, which is visible as a bar that stops short of its own axis.
            let scaled = height * (*value as f32) / (peak as f32);
            scaled.clamp(1.0_f32.min(height), height)
        })
        .collect()
}

/// The axis maximum to print above a chart: 1, 2 or 5 times a power of ten.
pub fn nice_max(peak: usize) -> usize {
    if peak <= 1 {
        return 1;
    }
    let magnitude = 10usize.pow((peak as f32).log10().floor() as u32);
    for factor in [1usize, 2, 5, 10] {
        let candidate = magnitude.saturating_mul(factor);
        if candidate >= peak {
            return candidate;
        }
    }
    peak
}

/// Draw columns with a baseline, labels and a hover tooltip on each.
///
/// Returns the height actually used, so a caller can lay out the next element without guessing.
pub fn columns(ui: &mut Ui, t: &Tokens, columns: &[Column], height: f32) -> f32 {
    let height = height.max(8.0);
    if columns.is_empty() {
        return 0.0;
    }
    let pixel_heights = heights(&columns.iter().map(|column| column.value).collect::<Vec<_>>(), height);
    let width = (columns.len() as f32 * COLUMN_WIDTH).max(COLUMN_WIDTH);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height + 14.0), Sense::hover());
    let baseline = rect.bottom() - 14.0;
    let painter = ui.painter();
    painter.line_segment(
        [pos2(rect.left(), baseline), pos2(rect.right(), baseline)],
        Stroke::new(1.0, t.border),
    );
    let peak = columns.iter().map(|column| column.value).max().unwrap_or(0);
    for (index, (column, tall)) in columns.iter().zip(pixel_heights.iter()).enumerate() {
        let left = rect.left() + index as f32 * COLUMN_WIDTH;
        let bar = Rect::from_min_max(pos2(left + 2.0, baseline - tall), pos2(left + COLUMN_WIDTH - 2.0, baseline));
        let fill = if column.highlight { t.accent } else { t.selection };
        if *tall > 0.0 {
            painter.rect_filled(bar, 1.0, fill);
        }
        if !column.label.is_empty() {
            painter.text(
                pos2(bar.center().x, baseline + 2.0),
                Align2::CENTER_TOP,
                &column.label,
                FontId::proportional(8.5),
                t.text_faint,
            );
        }
        // The tooltip carries the numbers: a chart you cannot read a value off is decoration.
        let hit = Rect::from_min_max(pos2(left, rect.top()), pos2(left + COLUMN_WIDTH, rect.bottom()));
        let response = ui.interact(hit, ui.scope_id().with(("column", index)), Sense::hover());
        let share = if peak == 0 {
            String::new()
        } else {
            format!(" · {:.0}%", column.value as f32 * 100.0 / peak as f32)
        };
        response.on_hover_text(format!(
            "{} — {}{share}",
            if column.label.is_empty() { "—" } else { &column.label },
            sortlens_core::format_count(column.value as u64)
        ));
    }
    height
}

/// A label, a proportional bar and a count.
///
/// `whole` is the denominator: the library total for a share of everything, the peak for a
/// comparison of rows. Zero is refused rather than divided by.
pub fn share_row(ui: &mut Ui, t: &Tokens, label: &str, count: usize, whole: usize) {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width().min(300.0), SHARE_HEIGHT), Sense::hover());
    let painter = ui.painter();
    painter.text(
        pos2(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(10.5),
        t.text_muted,
    );
    let track = Rect::from_min_max(
        pos2(rect.left() + 92.0, rect.center().y - 4.0),
        pos2(rect.right(), rect.center().y + 4.0),
    );
    painter.rect_filled(track, 2.0, t.thumb_bg);
    let share = share_of(count, whole);
    if count > 0 {
        let filled = Rect::from_min_size(track.min, vec2(track.width() * share.clamp(0.02, 1.0), track.height()));
        painter.rect_filled(filled, 2.0, t.selection);
    }
    painter.text(
        pos2(rect.right(), rect.center().y),
        Align2::RIGHT_CENTER,
        sortlens_core::format_count(count as u64),
        FontId::proportional(10.5),
        t.text_faint,
    );
    response.on_hover_text(format!(
        "{label}: {} of {} ({:.1}%)",
        sortlens_core::format_count(count as u64),
        sortlens_core::format_count(whole as u64),
        share * 100.0
    ));
}

/// A share in 0–1, refusing to divide by nothing.
pub fn share_of(count: usize, whole: usize) -> f32 {
    if whole == 0 {
        0.0
    } else {
        (count as f32 / whole as f32).clamp(0.0, 1.0)
    }
}

/// A one-line summary under a chart, e.g. "peak 42 in June".
pub fn caption(ui: &mut Ui, t: &Tokens, text: &str) {
    ui.label(RichText::new(text).size(10.5).color(t.text_faint));
}

/// A faint box drawn behind an empty chart, so the reader sees a chart that has nothing in it rather
/// than a layout that lost its chart.
pub fn empty(ui: &mut Ui, t: &Tokens, text: &str, height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width().min(300.0), height.max(24.0)), Sense::hover());
    ui.painter()
        .rect(rect, 3.0, Color32::TRANSPARENT, Stroke::new(1.0, t.border), StrokeKind::Inside);
    ui.painter()
        .text(rect.center(), Align2::CENTER_CENTER, text, FontId::proportional(10.5), t.text_faint);
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn the_tallest_bar_fills_the_chart_and_the_others_keep_their_proportion() {
        let heights = heights(&[10, 5, 1], 100.0);
        assert_eq!(heights[0], 100.0);
        assert_eq!(heights[1], 50.0);
        assert!(heights[2] >= 1.0, "a count of one is still visible: {:?}", heights);
    }

    #[test]
    fn an_empty_or_all_zero_series_has_no_heights_and_no_division() {
        assert!(heights(&[], 100.0).is_empty());
        assert_eq!(heights(&[0, 0, 0], 100.0), vec![0.0, 0.0, 0.0]);
        assert_eq!(share_of(0, 0), 0.0);
        assert_eq!(share_of(5, 0), 0.0, "a share of nothing is not infinity");
        assert_eq!(share_of(5, 10), 0.5);
        assert_eq!(share_of(15, 10), 1.0, "clamped, not above one");
    }

    #[test]
    fn a_tiny_height_still_produces_a_visible_bar() {
        // A 2-pixel chart is silly, but a 2-pixel bar must not round to nothing.
        let heights = heights(&[3, 1], 2.0);
        assert_eq!(heights[0], 2.0);
        assert!(heights[1] >= 1.0, "{heights:?}");
    }

    #[test]
    fn axis_maxima_are_round_numbers_that_cover_the_peak() {
        assert_eq!(nice_max(0), 1);
        assert_eq!(nice_max(1), 1);
        assert_eq!(nice_max(2), 2);
        assert_eq!(nice_max(3), 5);
        assert_eq!(nice_max(7), 10);
        assert_eq!(nice_max(12), 20);
        assert_eq!(nice_max(240), 500);
        for peak in [1usize, 2, 9, 17, 99, 100, 4_321] {
            let axis = nice_max(peak);
            assert!(axis >= peak, "{peak} does not fit under {axis}");
            assert!(
                axis <= peak.saturating_mul(5).max(1),
                "{axis} is not the nearest round number for {peak}"
            );
        }
    }

    /// Draw the charts headlessly: egui runs without a window, so painting is exercised — bad rect
    /// maths, an empty series or a one-pixel chart would fail here rather than in front of a user.
    #[test]
    fn the_charts_paint_without_a_window() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let columns = vec![Column::new("J", 3), Column::new("F", 0), Column::new("M", 19).highlighted()];
        let mut drawn = 0.0;
        // `run_ui` drives one frame without a window; the returned output is discarded because the
        // point of the test is that painting a chart does not panic or produce nonsense geometry.
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                drawn = columns_of(ui, &tokens, &columns);
                share(ui, &tokens);
                empty_chart(ui, &tokens);
            });
        });
        // A headless frame still produces texture uploads; epaint asserts if they are dropped unread.
        output.textures_delta.clear();
        assert_eq!(drawn, COLUMN_HEIGHT);
    }

    fn columns_of(ui: &mut Ui, t: &Tokens, series: &[Column]) -> f32 {
        self::columns(ui, t, series, COLUMN_HEIGHT)
    }

    fn share(ui: &mut Ui, t: &Tokens) {
        share_row(ui, t, "Rated", 3, 10);
        share_row(ui, t, "Nothing", 0, 0);
    }

    fn empty_chart(ui: &mut Ui, t: &Tokens) {
        empty(ui, t, "Nothing dated yet", COLUMN_HEIGHT);
    }

    #[test]
    fn a_column_remembers_whether_it_is_the_peak() {
        let plain = Column::new("Jun", 12);
        let peak = Column::new("Jun", 12).highlighted();
        assert!(!plain.highlight);
        assert!(peak.highlight);
        assert_eq!(plain.value, 12);
    }
}
