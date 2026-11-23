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

/// How big a grid cell is, which is the only thing that decides how many photos fit.
///
/// Three steps rather than a slider: a photographer picks a way of looking at a shoot, and the difference
/// between 104 and 108 pixels is not a choice anybody wants to make. The numbers are the *minimum* cell
/// width; the grid then divides the window evenly so the right edge is never ragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Density {
    /// Many small cells, for triaging a whole shoot.
    Compact,
    /// The default: five or six across a laptop window.
    #[default]
    Comfortable,
    /// Large cells, for judging a frame without opening it.
    Roomy,
}

/// How the grid lays out a row.
///
/// Square is a uniform lattice: every cell the same size, which is fastest to scan and the right default for
/// triage. Justified keeps each photo's own shape and gives every row the full width, which is what you want
/// when the photographs matter more than the grid.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum GridMode {
    /// Uniform cells, one photo per cell, no cropping.
    Square,
    /// Rows of equal height: each photo keeps its aspect ratio and the row fills the width.
    ///
    /// The default, because the photographs are the point: a square lattice crops what the photographer
    /// framed, and a library is not a contact sheet of tiles.
    #[default]
    Justified,
}

impl GridMode {
    /// Both modes, in the order the chip cycles through them.
    pub const ALL: [Self; 2] = [Self::Square, Self::Justified];

    /// Stable id used in settings.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Square => "square",
            Self::Justified => "justified",
        }
    }

    /// Label for the chip.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Square => "Square",
            Self::Justified => "Justified",
        }
    }

    /// The glyph that shows which layout is in force, so the canvas header needs no word for it.
    pub const fn icon(self) -> crate::icons::Icon {
        match self {
            Self::Square => crate::icons::Icon::Square,
            Self::Justified => crate::icons::Icon::Rows,
        }
    }

    /// Parse a persisted id; an unknown value is the default, never an error.
    pub fn from_id(id: &str) -> Self {
        Self::ALL.into_iter().find(|mode| mode.id() == id).unwrap_or_default()
    }

    /// The other mode — what the chip does when clicked.
    pub const fn next(self) -> Self {
        match self {
            Self::Square => Self::Justified,
            Self::Justified => Self::Square,
        }
    }
}

impl Density {
    /// Every step, in the order the chip cycles through them.
    pub const ALL: [Self; 3] = [Self::Compact, Self::Comfortable, Self::Roomy];

    /// Stable id used in settings.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Comfortable => "comfortable",
            Self::Roomy => "roomy",
        }
    }

    /// Label for the chip and Settings.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Comfortable => "Comfortable",
            Self::Roomy => "Roomy",
        }
    }

    /// Smallest cell edge in points.
    pub const fn min_cell(self) -> f32 {
        match self {
            Self::Compact => 104.0,
            Self::Comfortable => 132.0,
            Self::Roomy => 184.0,
        }
    }

    /// Parse a persisted id. An unknown value is the default, not an error: a settings file written by a
    /// later build must not stop this one from opening.
    pub fn from_id(id: &str) -> Self {
        Self::ALL.into_iter().find(|step| step.id() == id).unwrap_or_default()
    }

    /// The next step, wrapping — what the chip does when clicked.
    pub const fn next(self) -> Self {
        match self {
            Self::Compact => Self::Comfortable,
            Self::Comfortable => Self::Roomy,
            Self::Roomy => Self::Compact,
        }
    }
}

/// How much movement the cull overlay is allowed to make.
///
/// Two values rather than a slider, because the question a person is actually answering is "is this helping me
/// or making me ill", and a person who needs less motion needs *none* of it, not 30% of it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Motion {
    /// Label fades in, then the photo slides away (~230 ms in total).
    #[default]
    Full,
    /// No slide, no fade: the label appears at once and the next photo is already there. For vestibular
    /// sensitivity, migraine, and anyone who simply does not want the animation.
    Reduced,
}

impl Motion {
    /// Every setting, in menu order.
    pub const ALL: [Self; 2] = [Self::Full, Self::Reduced];

    /// Stable id used in settings.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Reduced => "reduced",
        }
    }

    /// Label for the chip and Settings.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Full => "Motion",
            Self::Reduced => "Reduced motion",
        }
    }

    /// Parse a persisted id, falling back to the default rather than failing.
    pub fn from_id(id: &str) -> Self {
        Self::ALL.into_iter().find(|motion| motion.id() == id).unwrap_or_default()
    }

    /// The other setting — what the chip does when clicked.
    pub const fn next(self) -> Self {
        match self {
            Self::Full => Self::Reduced,
            Self::Reduced => Self::Full,
        }
    }

    /// Whether the overlay animates at all.
    pub const fn animates(self) -> bool {
        matches!(self, Self::Full)
    }
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

    /// The egui theme this preset paints under: the half of `Style` egui keeps two copies of.
    pub const fn egui_theme(self) -> egui::Theme {
        if self.is_light() { egui::Theme::Light } else { egui::Theme::Dark }
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
        text_faint: Color32::from_rgb(0x6f, 0x6a, 0x63),
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
        text_faint: Color32::from_rgb(0x6b, 0x65, 0x5e),
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

    /// The five widget states as the suite's own palette type.
    ///
    /// `craft_ui::theme::WidgetPalette` is what suite widgets read, and the mapping lives here rather
    /// than in `apply` so there is exactly one answer to "what colour is a hovered button": the token
    /// set. `open` is a card because an open menu is a surface, not a pressed button — the accent ring
    /// on `active` is the only state that has to shout.
    pub const fn palette(&self) -> craft_ui::theme::WidgetPalette {
        craft_ui::theme::WidgetPalette {
            noninteractive: self.panel,
            inactive: self.card,
            hovered: self.hover,
            active: self.active,
            open: self.card,
        }
    }

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
    ctx.set_theme(kind.egui_theme());

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
            widget.bg_stroke = Stroke::new(1.0, t.border);
        }
        // The five state fills come from the palette, not from five lines that could disagree with
        // `Tokens::palette`. `apply_to` keeps the borders and expansion set above; the accent ring on
        // the pressed state is the one exception, because a press is the only state that must shout.
        t.palette()
            .apply_to(&mut style.visuals.widgets, t.corner(), Stroke::new(1.0, t.text));
        style.visuals.widgets.active.bg_stroke = Stroke::new(1.0, t.accent);

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

    /// The palette is the token set, and `apply` really installs it: a suite widget reading `Visuals`
    /// gets the same five fills a Sortlens widget draws by hand. The hover and press states must differ
    /// from the resting one in every preset — a palette whose states collapse is a palette that hides
    /// every interaction.
    #[test]
    fn the_widget_palette_is_the_token_set_and_apply_installs_it() {
        for kind in ThemeKind::ALL {
            let t = kind.tokens();
            let palette = t.palette();
            assert_eq!(palette.noninteractive, t.panel, "{}", kind.id());
            assert_eq!(palette.inactive, t.card, "{}", kind.id());
            assert_eq!(palette.hovered, t.hover, "{}", kind.id());
            assert_eq!(palette.active, t.active, "{}", kind.id());
            assert_eq!(palette.open, t.card, "{}", kind.id());
            assert_ne!(palette.hovered, palette.inactive, "{} hover is invisible", kind.id());
            assert_ne!(palette.active, palette.hovered, "{} press is invisible", kind.id());

            let ctx = egui::Context::default();
            apply(&ctx, kind);
            let widgets = ctx.style_of(kind.egui_theme()).visuals.widgets.clone();
            for (state, expected) in [
                (widgets.noninteractive, palette.noninteractive),
                (widgets.inactive, palette.inactive),
                (widgets.hovered, palette.hovered),
                (widgets.active, palette.active),
                (widgets.open, palette.open),
            ] {
                assert_eq!(state.bg_fill, expected, "{} strong fill", kind.id());
                assert_eq!(state.weak_bg_fill, expected, "{} weak fill", kind.id());
                assert_eq!(state.fg_stroke.color, t.text, "{} foreground", kind.id());
                assert_eq!(state.corner_radius, t.corner(), "{} radius", kind.id());
            }
            // The one documented exception: the pressed state wears the accent ring, which is what
            // makes a press visible without a shadow.
            assert_eq!(widgets.active.bg_stroke.color, t.accent, "{} press ring", kind.id());
            for state in [widgets.noninteractive, widgets.inactive, widgets.hovered, widgets.open] {
                assert_eq!(state.bg_stroke.color, t.border, "{} border", kind.id());
            }
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

    /// WCAG 2 relative luminance and contrast ratio, so the badges are judged by the same measure
    /// a contrast checker would use rather than by eye in a code review.
    fn channel(value: u8) -> f64 {
        let c = f64::from(value) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    fn luminance(colour: Color32) -> f64 {
        0.2126 * channel(colour.r()) + 0.7152 * channel(colour.g()) + 0.0722 * channel(colour.b())
    }

    fn contrast(a: Color32, b: Color32) -> f64 {
        let (light, dark) = ((luminance(a) + 0.05), (luminance(b) + 0.05));
        (light / dark).max(dark / light)
    }

    /// The marks a grid cell paints against its own theme-controlled background hold 3:1 (WCAG
    /// graphics): the cell badge's dot against `thumb_bg` — the worst background a badge is ever
    /// drawn over, because a badge exists exactly when there is no photograph — and the white
    /// glyph against the dot. The flag dots are painted straight onto the cell, so they are judged
    /// the same way. Silver and Paper's faint grey used to fail the dot test (2.2 and 2.7 to 1);
    /// this test is why they are darker now, and it fails again if a preset regresses.
    #[test]
    fn badge_and_flag_marks_hold_three_to_one_against_a_bad_cell() {
        let white = Color32::WHITE;
        for kind in ThemeKind::ALL {
            let t = kind.tokens();
            // The Offline badge dot (`text_faint`) on a cell with no pixels, and its `?` glyph.
            assert!(
                contrast(t.text_faint, t.thumb_bg) >= 3.0,
                "{} offline badge dot is invisible on a dead cell: {:.2}",
                kind.id(),
                contrast(t.text_faint, t.thumb_bg)
            );
            assert!(
                contrast(white, t.text_faint) >= 3.0,
                "{} badge glyph unreadable on the offline dot: {:.2}",
                kind.id(),
                contrast(white, t.text_faint)
            );
            // The Unreadable badge dot (`reject`) and its glyph.
            assert!(
                contrast(t.reject, t.thumb_bg) >= 3.0,
                "{} unreadable badge dot is invisible on a dead cell: {:.2}",
                kind.id(),
                contrast(t.reject, t.thumb_bg)
            );
            assert!(
                contrast(white, t.reject) >= 3.0,
                "{} badge glyph unreadable on the unreadable dot: {:.2}",
                kind.id(),
                contrast(white, t.reject)
            );
            // The flag dots, painted straight onto the cell.
            assert!(
                contrast(t.pick, t.thumb_bg) >= 3.0,
                "{} pick dot is invisible on a dead cell: {:.2}",
                kind.id(),
                contrast(t.pick, t.thumb_bg)
            );
            assert!(
                contrast(t.reject, t.thumb_bg) >= 3.0,
                "{} reject dot is invisible on a dead cell: {:.2}",
                kind.id(),
                contrast(t.reject, t.thumb_bg)
            );
        }
    }
}
