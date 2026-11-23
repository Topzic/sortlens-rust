//! The shell's icon set: one monochrome SVG per meaning, painted in the interface's own colours.
//!
//! Every glyph is a project-owned 24×24 stroke drawing under `assets/icons/`, embedded with
//! `include_bytes!` so a zero-config build has icons and no file is read at run time. `craft_ui::icons`
//! prepares the bytes once (white `currentColor`, 1.75 stroke, so a tint means the theme colour and not
//! a second palette), and `egui_extras`' SVG loader rasterises them to the exact size each slot asks
//! for — including at `pixels_per_point` 2, where a bitmap would blur.
//!
//! The names are the contract: [`Icon::name`] is the file stem, a test fails if the catalog and the
//! folder disagree, and [`Icon::ALL`] is what that test walks. An icon is added here *and* in the
//! folder in one change, or not at all.

use std::sync::OnceLock;

use craft_ui::icons::{SvgIconSet, SvgStyle};
use egui::{Color32, Image, ImageSource, Rect, Response, Ui, Vec2};

use crate::a11y;
use crate::theme::Tokens;

/// Where the installed loader is remembered, so it is installed once per context and not once per frame.
const INSTALLED: &str = "sortlens_ui::icons::installed";

/// The embedded catalog: file stem, then the drawing.
///
/// Kept as one table rather than a `match` so the assets and the enum cannot drift: the test below
/// compares the table's names against [`Icon::ALL`] and against the folder on disk.
const CATALOG: &[(&str, &[u8])] = &[
    ("activity", include_bytes!("../../../assets/icons/activity.svg")),
    ("bell", include_bytes!("../../../assets/icons/bell.svg")),
    ("chart", include_bytes!("../../../assets/icons/chart.svg")),
    ("clock", include_bytes!("../../../assets/icons/clock.svg")),
    ("close", include_bytes!("../../../assets/icons/close.svg")),
    ("contrast", include_bytes!("../../../assets/icons/contrast.svg")),
    ("density", include_bytes!("../../../assets/icons/density.svg")),
    ("droplet", include_bytes!("../../../assets/icons/droplet.svg")),
    ("film", include_bytes!("../../../assets/icons/film.svg")),
    ("flag", include_bytes!("../../../assets/icons/flag.svg")),
    ("folder", include_bytes!("../../../assets/icons/folder.svg")),
    ("folder-plus", include_bytes!("../../../assets/icons/folder-plus.svg")),
    ("health", include_bytes!("../../../assets/icons/health.svg")),
    ("history", include_bytes!("../../../assets/icons/history.svg")),
    ("info", include_bytes!("../../../assets/icons/info.svg")),
    ("judge", include_bytes!("../../../assets/icons/judge.svg")),
    ("keyboard", include_bytes!("../../../assets/icons/keyboard.svg")),
    ("layers", include_bytes!("../../../assets/icons/layers.svg")),
    ("library", include_bytes!("../../../assets/icons/library.svg")),
    ("panel", include_bytes!("../../../assets/icons/panel.svg")),
    ("pencil", include_bytes!("../../../assets/icons/pencil.svg")),
    ("pick", include_bytes!("../../../assets/icons/pick.svg")),
    ("pin", include_bytes!("../../../assets/icons/pin.svg")),
    ("reject", include_bytes!("../../../assets/icons/reject.svg")),
    ("rows", include_bytes!("../../../assets/icons/rows.svg")),
    ("similar", include_bytes!("../../../assets/icons/similar.svg")),
    ("sliders", include_bytes!("../../../assets/icons/sliders.svg")),
    ("sparkle", include_bytes!("../../../assets/icons/sparkle.svg")),
    ("square", include_bytes!("../../../assets/icons/square.svg")),
    ("star", include_bytes!("../../../assets/icons/star.svg")),
    ("tag", include_bytes!("../../../assets/icons/tag.svg")),
    ("trash", include_bytes!("../../../assets/icons/trash.svg")),
    ("warning", include_bytes!("../../../assets/icons/warning.svg")),
    ("wave", include_bytes!("../../../assets/icons/wave.svg")),
];

/// One glyph. The variant name is the meaning; the catalog name is the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// The whole library.
    Library,
    /// Recently added.
    Clock,
    /// A pick.
    Pick,
    /// A rating.
    Star,
    /// A reject.
    Reject,
    /// Library health.
    Health,
    /// Recently deleted.
    Trash,
    /// The change log.
    History,
    /// Keyboard shortcuts.
    Keyboard,
    /// A folder.
    Folder,
    /// Collections.
    Layers,
    /// Memories and resurfacing.
    Sparkle,
    /// The cull review.
    Judge,
    /// Places.
    Pin,
    /// Insights.
    Chart,
    /// Similar-photo results.
    Similar,
    /// Details.
    Info,
    /// Keywords.
    Tag,
    /// Background jobs.
    Activity,
    /// Settings.
    Sliders,
    /// The right panel.
    Panel,
    /// The theme.
    Contrast,
    /// Grid density.
    Density,
    /// A uniform grid.
    Square,
    /// A justified grid.
    Rows,
    /// Reduced motion.
    Wave,
    /// Add a folder.
    FolderPlus,
    /// Alerts.
    Bell,
    /// A flag filter.
    Flag,
    /// A colour-label filter.
    Droplet,
    /// The media kind filter.
    Film,
    /// Edited photos.
    Pencil,
    /// Something missing or wrong.
    Warning,
    /// Clear this filter.
    Close,
}

impl Icon {
    /// Every icon, in catalog order.
    pub const ALL: [Self; 34] = [
        Self::Activity,
        Self::Bell,
        Self::Chart,
        Self::Clock,
        Self::Close,
        Self::Contrast,
        Self::Density,
        Self::Droplet,
        Self::Film,
        Self::Flag,
        Self::Folder,
        Self::FolderPlus,
        Self::Health,
        Self::History,
        Self::Info,
        Self::Judge,
        Self::Keyboard,
        Self::Layers,
        Self::Library,
        Self::Panel,
        Self::Pencil,
        Self::Pick,
        Self::Pin,
        Self::Reject,
        Self::Rows,
        Self::Similar,
        Self::Sliders,
        Self::Sparkle,
        Self::Square,
        Self::Star,
        Self::Tag,
        Self::Trash,
        Self::Warning,
        Self::Wave,
    ];

    /// The catalog name, which is also the file stem.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Activity => "activity",
            Self::Bell => "bell",
            Self::Chart => "chart",
            Self::Clock => "clock",
            Self::Close => "close",
            Self::Contrast => "contrast",
            Self::Density => "density",
            Self::Droplet => "droplet",
            Self::Film => "film",
            Self::Flag => "flag",
            Self::Folder => "folder",
            Self::FolderPlus => "folder-plus",
            Self::Health => "health",
            Self::History => "history",
            Self::Info => "info",
            Self::Judge => "judge",
            Self::Keyboard => "keyboard",
            Self::Layers => "layers",
            Self::Library => "library",
            Self::Panel => "panel",
            Self::Pencil => "pencil",
            Self::Pick => "pick",
            Self::Pin => "pin",
            Self::Reject => "reject",
            Self::Rows => "rows",
            Self::Similar => "similar",
            Self::Sliders => "sliders",
            Self::Sparkle => "sparkle",
            Self::Square => "square",
            Self::Star => "star",
            Self::Tag => "tag",
            Self::Trash => "trash",
            Self::Warning => "warning",
            Self::Wave => "wave",
        }
    }
}

/// The prepared catalog. One allocation for the whole process: the bytes never change.
fn set() -> &'static SvgIconSet {
    static SET: OnceLock<SvgIconSet> = OnceLock::new();
    SET.get_or_init(|| {
        // White, so `Image::tint` (or the button's text colour) supplies the only colour an icon is
        // ever drawn in. The stroke is 1.75 because a 2-unit stroke at 14 px reads as a bold blob.
        SvgIconSet::new(
            CATALOG,
            SvgStyle {
                current_color: "#ffffff",
                stroke_width: Some("1.75"),
            },
            "bytes://sortlens/icons/",
            "warning",
        )
    })
}

/// Install the SVG loader on a context, once.
///
/// Called from [`crate::show`] every frame, and returns immediately after the first: the flag lives in
/// the context's own memory, so a second window in the same process installs its own loaders rather
/// than inheriting a claim that is about a different context.
pub fn install(ctx: &egui::Context) {
    let key = egui::Id::unique(INSTALLED);
    if ctx.data(|data| data.get_temp::<bool>(key)).unwrap_or(false) {
        return;
    }
    egui_extras::install_image_loaders(ctx);
    // Every icon is registered up front: the bytes loader is keyed by URI, so a glyph that has never
    // been asked for still resolves on the frame it first appears instead of one frame later.
    for (name, _) in CATALOG {
        if let ImageSource::Bytes { uri, bytes } = set().source(name) {
            ctx.include_bytes(uri, bytes);
        }
    }
    ctx.data_mut(|data| data.insert_temp(key, true));
}

/// The image for an icon, at a fixed square size in points.
pub fn image(icon: Icon, size: f32) -> Image<'static> {
    Image::new(set().source(icon.name())).fit_to_exact_size(Vec2::splat(size))
}

/// Paint an icon into a rectangle in one colour.
///
/// `paint_at` loads the texture at exactly the painted size times `pixels_per_point`, so a 14-point
/// icon is sharp on a 2× display and no larger than it needs to be in memory.
pub fn paint(ui: &Ui, icon: Icon, rect: Rect, tint: Color32) {
    Image::new(set().source(icon.name())).tint(tint).paint_at(ui, rect);
}

/// An icon-only button, with a spoken name that is not the glyph.
///
/// The label is the tooltip *and* the accessible name: `craft-ui`'s icon buttons take the two
/// separately because a screen reader must not be handed a file stem, and a tooltip that says
/// something different from the button's name is two answers to one question.
pub fn button(ui: &mut Ui, t: &Tokens, icon: Icon, label: &str, selected: bool) -> Response {
    let colour = if selected { t.text } else { t.text_muted };
    let response = ui.add(
        egui::Button::image(image(icon, 14.0).tint(colour))
            .fill(if selected { t.selection } else { Color32::TRANSPARENT })
            .stroke(egui::Stroke::NONE),
    );
    a11y::label_widget(ui, &response, egui::Role::Button, label, selected);
    response.on_hover_text(label)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// The enum, the catalog and the folder are three lists of the same set, and the merge that added
    /// one to only one of them is the ordinary mistake.
    #[test]
    fn the_enum_the_catalog_and_the_folder_name_the_same_icons() {
        let mut names: Vec<&str> = Icon::ALL.iter().map(|icon| icon.name()).collect();
        names.sort_unstable();
        let mut catalog: Vec<&str> = CATALOG.iter().map(|(name, _)| *name).collect();
        catalog.sort_unstable();
        assert_eq!(names, catalog, "Icon::ALL and CATALOG disagree");

        let folder = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/icons");
        let mut on_disk: Vec<String> = std::fs::read_dir(folder)
            .expect("the icon folder")
            .filter_map(|entry| {
                let entry = entry.expect("a file");
                let path = entry.path();
                (path.extension().and_then(|ext| ext.to_str()) == Some("svg"))
                    .then(|| path.file_stem().and_then(|stem| stem.to_str()).map(str::to_owned))
                    .flatten()
            })
            .collect();
        on_disk.sort();
        assert_eq!(catalog, on_disk, "every SVG needs a name, and every name an SVG");
    }

    /// A glyph is a stroke drawing with the conventions the preparation step rewrites, not a
    /// pre-coloured or pre-sized one: `currentColor` is what makes a tint possible, and a literal
    /// colour would paint the same grey on all four presets.
    #[test]
    fn every_glyph_is_tinted_by_current_colour_at_the_catalogued_stroke() {
        for (name, bytes) in CATALOG {
            let svg = std::str::from_utf8(bytes).expect("UTF-8");
            assert!(svg.contains("currentColor"), "{name} cannot be tinted");
            assert!(svg.contains(r#"stroke-width="2""#), "{name} has no 2-unit stroke to scale");
            assert!(svg.starts_with("<svg"), "{name} is not an SVG document");
            assert!(svg.contains(r#"viewBox="0 0 24 24""#), "{name} is not a 24-unit square");
            assert!(!svg.contains("rgb("), "{name} hard-codes a colour");
        }
    }

    /// The glyphs rasterise. A malformed path or a missing loader is a blank button nobody notices in
    /// review, so this asks the real loader — in a headless context, through the same `install` the
    /// shell calls — for every icon at two sizes.
    #[test]
    fn every_icon_rasterises_to_something_visible() {
        let ctx = egui::Context::default();
        install(&ctx);
        install(&ctx);
        for icon in Icon::ALL {
            for size in [14_u32, 32] {
                let uri = format!("bytes://sortlens/icons/{}.svg", icon.name());
                let poll = ctx
                    .try_load_image(
                        &uri,
                        egui::load::SizeHint::Size {
                            width: size,
                            height: size,
                            maintain_aspect_ratio: true,
                        },
                    )
                    .expect("the SVG loader is installed");
                let egui::load::ImagePoll::Ready { image } = poll else {
                    panic!("{} did not load synchronously", icon.name());
                };
                assert_eq!(image.size, [size as usize; 2], "{} is not square", icon.name());
                assert!(
                    image.pixels.iter().any(|pixel| pixel.a() > 0),
                    "{} painted nothing at {size} px",
                    icon.name()
                );
                assert!(
                    image.pixels.iter().any(|pixel| pixel.a() == 0),
                    "{} filled its whole box — the drawing is not a glyph",
                    icon.name()
                );
            }
        }
    }

    /// An icon-only button still says what it is. Without the label a screen reader gets an image with
    /// no name, which is the failure mode an icon set invites.
    #[test]
    fn an_icon_button_is_named_for_assistive_technology() {
        let ctx = egui::Context::default();
        // AccessKit is opt-in on a bare context; the window (and `egui_kittest`) turns it on.
        ctx.enable_accesskit();
        let mut seen = None;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            install(ctx);
            crate::theme::apply(ctx, crate::theme::ThemeKind::Charcoal);
            egui::CentralPanel::default().show(ctx, |ui| {
                let response = button(ui, &Tokens::CHARCOAL, Icon::Sliders, "Settings", false);
                seen = Some(ui.ctx().accesskit_node_builder(response.id, |node| node.label().map(str::to_owned)));
            });
        });
        output.textures_delta.clear();
        assert_eq!(seen.flatten().flatten().as_deref(), Some("Settings"), "the button speaks its label");
        assert!(!output.shapes.is_empty(), "and it painted");
    }
}
