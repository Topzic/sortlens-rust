//! Colour tokens and theme presets.
//!
//! Four presets, matching the suite's naming: Charcoal and Midnight are dark, Silver and Paper
//! are light. Every widget colour comes from [`Tokens`]; no panel hard-codes an `Rgba`.
//!
//! The accent is the one place Sortlens differs from its siblings: a warm amber reads as
//! "keep/select" next to photographic neutrals without competing with the images.

use egui::{Color32, CornerRadius, FontFamily, FontId, Stroke, Visuals};
use serde::{Deserialize, Serialize};

/// Which preset is active. Persisted as a string in `config.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeKind {
    /// Neutral dark grey, the default.
    #[default]
    Charcoal,
    /// Blue-black, for editing in a dark room.
    Midnight,
    /// Warm light grey.
    Silver,
    /// Near-white, for bright rooms and projectors.
    Paper,
}

impl ThemeKind {
    /// Every preset, in menu order.
    pub const ALL: [Self; 4] = [Self::Charcoal, Self::Midnight, Self::Silver, Self::Paper];

    /// Stable id used in settings, the command line and logs.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Charcoal => "charcoal",
            Self::Midnight => "midnight",
            Self::Silver => "silver",
            Self::Paper => "paper",
        }
    }

    /// Label for menus.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Charcoal => "Charcoal",
            Self::Midnight => "Midnight",
            Self::Silver => "Silver",
            Self::Paper => "Paper",
        }
    }

    /// Parse a persisted id, falling back to the default rather than failing.
    pub fn from_id(id: &str) -> Self {
        Self::ALL.into_iter().find(|kind| kind.id() == id).unwrap_or_default()
    }

    /// Whether this preset is a light theme, which decides the window chrome and scrollbars.
    pub const fn is_light(self) -> bool {
        matches!(self, Self::Silver | Self::Paper)
    }

    /// The token set for this preset.
    pub const fn tokens(self) -> Tokens {
        match self {
            Self::Charcoal => Tokens::CHARCOAL,
            Self::Midnight => Tokens::MIDNIGHT,
            Self::Silver => Tokens::SILVER,
            Self::Paper => Tokens::PAPER,
        }
    }
}

/// Every colour and metric the shell draws with.
///
/// Panels take a `&Tokens` and nothing else; a new preset is a new constant here, not a new
/// branch in the drawing code.
#[derive(Debug, Clone, Copy)]
pub struct Tokens {
    /// Window and canvas background.
    pub bg: Color32,
    /// Title bar, toolbars and status bar.
    pub chrome: Color32,
    /// Side panels and docked tool panels.
    pub panel: Color32,
    /// Cards, popovers and rows that need to lift off the panel.
    pub card: Color32,
    /// Hairlines and separators.
    pub border: Color32,
    /// Primary text.
    pub text: Color32,
    /// Secondary text: counts, labels, metadata keys.
    pub text_muted: Color32,
    /// Disabled text and placeholder copy.
    pub text_faint: Color32,
    /// The one accent: selection, progress, active tool.
    pub accent: Color32,
    /// Text on top of [`Self::accent`].
    pub accent_fg: Color32,
    /// Selected row or active segment background.
    pub selection: Color32,
    /// Hovered row.
    pub hover: Color32,
    /// Pressed row.
    pub active: Color32,
    /// Pick flag.
    pub pick: Color32,
    /// Reject flag, also the destructive action colour.
    pub reject: Color32,
    /// Grid thumbnail background, one step away from the canvas.
    pub thumb_bg: Color32,
    /// Drop shadow, used sparingly.
    pub shadow: Color32,
    /// Height of the top bar.
    pub top_bar_h: f32,
    /// Height of the bottom bar.
    pub bottom_bar_h: f32,
    /// Width of the left panel at first launch.
    pub left_w: f32,
    /// Width of the right panel at first launch.
    pub right_w: f32,
    /// Grid gap between thumbnails.
    pub grid_gap: f32,
    /// Corner radius for panels and cards.
    pub radius: u8,
}

impl Tokens {
    /// Neutral dark grey. The default, tuned so thumbnails keep their apparent brightness.
    pub const CHARCOAL: Self = Self {
        bg: Color32::from_rgb(0x1b, 0x1b, 0x1b),
        chrome: Color32::from_rgb(0x24, 0x24, 0x24),
        panel: Color32::from_rgb(0x20, 0x20, 0x20),
        card: Color32::from_rgb(0x2a, 0x2a, 0x2a),
        border: Color32::from_rgb(0x38, 0x38, 0x38),
        text: Color32::from_rgb(0xe9, 0xe9, 0xe9),
        text_muted: Color32::from_rgb(0x9d, 0x9d, 0x9d),
        text_faint: Color32::from_rgb(0x6b, 0x6b, 0x6b),
        accent: Color32::from_rgb(0xe0, 0x98, 0x2f),
        accent_fg: Color32::from_rgb(0x1b, 0x14, 0x08),
        selection: Color32::from_rgb(0x3a, 0x31, 0x1e),
        hover: Color32::from_rgb(0x2e, 0x2e, 0x2e),
        active: Color32::from_rgb(0x34, 0x34, 0x34),
        pick: Color32::from_rgb(0x5d, 0xc0, 0x7a),
        reject: Color32::from_rgb(0xe0, 0x5c, 0x5c),
        thumb_bg: Color32::from_rgb(0x15, 0x15, 0x15),
        shadow: Color32::from_black_alpha(90),
        top_bar_h: 44.0,
        bottom_bar_h: 40.0,
        left_w: 264.0,
        right_w: 296.0,
        grid_gap: 4.0,
        radius: 6,
    };

    /// Blue-black, lower contrast floor than Charcoal.
    pub const MIDNIGHT: Self = Self {
        bg: Color32::from_rgb(0x0f, 0x12, 0x17),
        chrome: Color32::from_rgb(0x16, 0x1a, 0x21),
        panel: Color32::from_rgb(0x12, 0x16, 0x1c),
        card: Color32::from_rgb(0x1b, 0x20, 0x28),
        border: Color32::from_rgb(0x2a, 0x31, 0x3b),
        text: Color32::from_rgb(0xdf, 0xe4, 0xea),
        text_muted: Color32::from_rgb(0x92, 0x9c, 0xa8),
        text_faint: Color32::from_rgb(0x5f, 0x68, 0x74),
        accent: Color32::from_rgb(0xf0, 0xa5, 0x3c),
        accent_fg: Color32::from_rgb(0x14, 0x10, 0x07),
        selection: Color32::from_rgb(0x2c, 0x2a, 0x1c),
        hover: Color32::from_rgb(0x1e, 0x24, 0x2c),
        active: Color32::from_rgb(0x24, 0x2b, 0x34),
        pick: Color32::from_rgb(0x5f, 0xc8, 0x86),
        reject: Color32::from_rgb(0xe3, 0x63, 0x63),
        thumb_bg: Color32::from_rgb(0x0a, 0x0c, 0x10),
        shadow: Color32::from_black_alpha(110),
        ..Self::CHARCOAL
    };

    /// Warm light grey.
    pub const SILVER: Self = Self {
        bg: Color32::from_rgb(0xec, 0xeb, 0xe8),
        chrome: Color32::from_rgb(0xf4, 0xf3, 0xf0),
        panel: Color32::from_rgb(0xee, 0xec, 0xe8),
        card: Color32::from_rgb(0xfa, 0xf9, 0xf7),
        border: Color32::from_rgb(0xd3, 0xd0, 0xca),
        text: Color32::from_rgb(0x24, 0x22, 0x1f),
        text_muted: Color32::from_rgb(0x6c, 0x67, 0x60),
        text_faint: Color32::from_rgb(0x96, 0x91, 0x89),
        accent: Color32::from_rgb(0xa8, 0x62, 0x00),
        accent_fg: Color32::from_rgb(0xff, 0xff, 0xff),
        selection: Color32::from_rgb(0xf2, 0xe4, 0xc8),
        hover: Color32::from_rgb(0xe4, 0xe1, 0xdb),
        active: Color32::from_rgb(0xda, 0xd6, 0xcf),
        pick: Color32::from_rgb(0x2f, 0x7d, 0x46),
        reject: Color32::from_rgb(0xb3, 0x2d, 0x2d),
        thumb_bg: Color32::from_rgb(0xdc, 0xd9, 0xd3),
        shadow: Color32::from_black_alpha(40),
        ..Self::CHARCOAL
    };

    /// Near-white, maximum separation between UI and image.
    pub const PAPER: Self = Self {
        bg: Color32::from_rgb(0xf7, 0xf7, 0xf5),
        chrome: Color32::from_rgb(0xff, 0xff, 0xfd),
        panel: Color32::from_rgb(0xfa, 0xfa, 0xf8),
        card: Color32::from_rgb(0xff, 0xff, 0xff),
        border: Color32::from_rgb(0xdd, 0xdd, 0xd9),
        text: Color32::from_rgb(0x1a, 0x1a, 0x18),
        text_muted: Color32::from_rgb(0x63, 0x63, 0x5e),
        text_faint: Color32::from_rgb(0x8e, 0x8e, 0x88),
        accent: Color32::from_rgb(0x96, 0x54, 0x00),
        accent_fg: Color32::from_rgb(0xff, 0xff, 0xff),
        selection: Color32::from_rgb(0xf7, 0xe9, 0xcd),
        hover: Color32::from_rgb(0xef, 0xef, 0xeb),
        active: Color32::from_rgb(0xe6, 0xe6, 0xe1),
        pick: Color32::from_rgb(0x25, 0x6b, 0x3b),
        reject: Color32::from_rgb(0x9e, 0x24, 0x24),
        thumb_bg: Color32::from_rgb(0xe8, 0xe8, 0xe3),
        shadow: Color32::from_black_alpha(28),
        ..Self::CHARCOAL
    };

    /// A font at a given size in the interface family.
    pub fn font(&self, size: f32) -> FontId {
        FontId::new(size, FontFamily::Proportional)
    }

    /// Corner radius as egui wants it.
    pub fn corner(&self) -> CornerRadius {
        CornerRadius::same(self.radius)
    }

    /// Colour for a rating star that is set.
    pub fn star(&self) -> Color32 {
        self.accent
    }

    /// Colour for a flag, including the unflagged state.
    /// Colour used for a rating-adjacent label, in the suite's fixed order.
    pub fn label(&self, label: sortlens_core::ColorLabel) -> Color32 {
        match label {
            sortlens_core::ColorLabel::Red => Color32::from_rgb(214, 84, 74),
            sortlens_core::ColorLabel::Yellow => Color32::from_rgb(214, 176, 62),
            sortlens_core::ColorLabel::Green => Color32::from_rgb(96, 170, 104),
            sortlens_core::ColorLabel::Blue => Color32::from_rgb(82, 132, 202),
            sortlens_core::ColorLabel::Purple => Color32::from_rgb(150, 108, 196),
        }
    }

    pub fn flag(&self, flag: sortlens_core::Flag) -> Color32 {
        match flag {
            sortlens_core::Flag::Pick => self.pick,
            sortlens_core::Flag::Reject => self.reject,
            sortlens_core::Flag::None => self.text_faint,
        }
    }
}

/// Install a preset on the context: text styles, spacing, and the five widget states.
///
/// Each state is written explicitly here because Sortlens has six more token roles than
/// egui's `Visuals` (thumb wells, pick/reject, shadow). Suite widgets that arrive with
/// `craft-ui` (docking chrome, dialogs) read the same `Visuals`, so the two stay consistent.
pub fn apply(ctx: &egui::Context, kind: ThemeKind) {
    let t = kind.tokens();
    let dark = !kind.is_light();
    ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });

    ctx.all_styles_mut(|style| {
        style.visuals = if dark { Visuals::dark() } else { Visuals::light() };
        style.visuals.dark_mode = dark;
        style.visuals.panel_fill = t.panel;
        style.visuals.window_fill = t.chrome;
        style.visuals.extreme_bg_color = t.thumb_bg;
        style.visuals.faint_bg_color = t.card;
        style.visuals.override_text_color = Some(t.text);
        style.visuals.window_stroke = Stroke::new(1.0, t.border);
        style.visuals.selection.bg_fill = t.selection;
        style.visuals.selection.stroke = Stroke::new(1.0, t.accent);
        style.visuals.hyperlink_color = t.accent;

        for widget in [
            &mut style.visuals.widgets.noninteractive,
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
            &mut style.visuals.widgets.open,
        ] {
            widget.corner_radius = t.corner();
            widget.fg_stroke = Stroke::new(1.0, t.text);
            widget.bg_stroke = Stroke::new(1.0, t.border);
        }
        style.visuals.widgets.noninteractive.bg_fill = t.panel;
        style.visuals.widgets.noninteractive.weak_bg_fill = t.panel;
        style.visuals.widgets.inactive.bg_fill = t.card;
        style.visuals.widgets.inactive.weak_bg_fill = t.hover;
        style.visuals.widgets.hovered.bg_fill = t.hover;
        style.visuals.widgets.hovered.weak_bg_fill = t.hover;
        style.visuals.widgets.active.bg_fill = t.active;
        style.visuals.widgets.active.weak_bg_fill = t.active;
        style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, t.accent);
        style.visuals.widgets.open.bg_fill = t.card;
        style.visuals.widgets.open.weak_bg_fill = t.card;

        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.interact_size.y = 24.0;

        style.text_styles.insert(egui::TextStyle::Heading, t.font(19.0));
        style.text_styles.insert(egui::TextStyle::Body, t.font(13.0));
        style.text_styles.insert(egui::TextStyle::Button, t.font(13.0));
        style.text_styles.insert(egui::TextStyle::Small, t.font(11.0));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace));
    });
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        for kind in ThemeKind::ALL {
            assert_eq!(ThemeKind::from_id(kind.id()), kind);
        }
        assert_eq!(ThemeKind::from_id("nonsense"), ThemeKind::Charcoal);
    }

    #[test]
    fn light_presets_have_dark_text() {
        for kind in ThemeKind::ALL {
            let t = kind.tokens();
            let text_is_dark = t.text.r() < 0x80;
            assert_eq!(text_is_dark, kind.is_light(), "{} text/value mismatch", kind.id());
        }
    }

    #[test]
    fn accents_are_visible_against_the_background() {
        for kind in ThemeKind::ALL {
            let t = kind.tokens();
            let delta = (i32::from(t.accent.r()) - i32::from(t.bg.r())).abs()
                + (i32::from(t.accent.g()) - i32::from(t.bg.g())).abs()
                + (i32::from(t.accent.b()) - i32::from(t.bg.b())).abs();
            assert!(delta > 90, "{} accent too close to background", kind.id());
        }
    }
}
