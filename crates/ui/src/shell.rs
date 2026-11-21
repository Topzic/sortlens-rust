//! The application shell: five surfaces, in the suite's order.
//!
//! Top bar (identity, search, filters, actions), left panel (sources, folders, collections),
//! canvas, right panel (details, tags, activity), bottom bar (views, review state, zoom).
//!
//! The shell owns presentation state and nothing else. Anything that changes the library goes
//! through [`Engine::run`]; anything that changes the process (theme, quit) is returned as an
//! [`Action`] for the host to apply.

use crate::theme::{ThemeKind, Tokens};
use egui::{Align, Button, Color32, Frame, Layout, Margin, RichText, Sense, Stroke, TextEdit, Ui, vec2};
use sortlens_engine::Engine;

/// A top-level destination in the bottom bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    /// The library grid: the default.
    #[default]
    Library,
    /// Full-screen cull review.
    Cull,
    /// Map of geotagged photos.
    Places,
    /// Library statistics.
    Insights,
    /// Results of a similar-photo search.
    Similar,
}

impl View {
    /// Every destination, in bottom-bar order.
    pub const ALL: [Self; 5] = [Self::Library, Self::Cull, Self::Places, Self::Insights, Self::Similar];

    /// Label for the bottom bar and the contextual header.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Cull => "Cull",
            Self::Places => "Places",
            Self::Insights => "Insights",
            Self::Similar => "Similar",
        }
    }

    /// The milestone that makes this view real, or `None` when it ships today.
    pub const fn milestone(self) -> Option<&'static str> {
        match self {
            Self::Library => None,
            Self::Cull => Some("M4"),
            Self::Places => Some("M5"),
            Self::Insights => Some("M6"),
            Self::Similar => Some("M6"),
        }
    }
}

/// Which tab the right panel shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightTab {
    /// Camera, exposure, file facts.
    #[default]
    Details,
    /// Keywords and collections.
    Tags,
    /// Background jobs.
    Activity,
}

impl RightTab {
    /// Tabs in order.
    pub const ALL: [Self; 3] = [Self::Details, Self::Tags, Self::Activity];

    /// Tab label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Details => "Details",
            Self::Tags => "Tags",
            Self::Activity => "Activity",
        }
    }
}

/// A request from the shell to the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Apply and persist a theme.
    SetTheme(ThemeKind),
    /// Close the window, flushing settings first.
    Quit,
    /// Show a transient message in the bottom bar.
    Notice(String),
}

/// Everything the shell remembers between frames.
#[derive(Debug, Clone)]
pub struct UiState {
    /// Active theme, mirrored from settings.
    pub theme: ThemeKind,
    /// Current destination.
    pub view: View,
    /// Search query.
    pub search: String,
    /// Whether the left panel is shown.
    pub left_open: bool,
    /// Width of the left panel.
    pub left_w: f32,
    /// Whether the right panel is shown.
    pub right_open: bool,
    /// Right panel tab.
    pub right_tab: RightTab,
    /// Left padding that clears the window controls in an integrated title bar.
    pub title_inset: f32,
    /// Settings dialog visibility.
    pub settings_open: bool,
    /// About dialog visibility.
    pub about_open: bool,
    /// A transient message and the instant it expires.
    pub notice: Option<(String, std::time::Instant)>,
}

impl UiState {
    /// Fresh state for a given theme and title-bar inset.
    pub fn new(theme: ThemeKind, title_inset: f32) -> Self {
        let t = theme.tokens();
        Self {
            theme,
            view: View::Library,
            search: String::new(),
            left_open: true,
            left_w: t.left_w,
            right_open: true,
            right_tab: RightTab::Details,
            title_inset,
            settings_open: false,
            about_open: false,
            notice: None,
        }
    }

    /// Show a message in the bottom bar for a few seconds.
    pub fn notice(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), std::time::Instant::now() + std::time::Duration::from_secs(4)));
    }

    /// The message still worth showing, if any.
    fn live_notice(&self) -> Option<&str> {
        match &self.notice {
            Some((text, until)) if std::time::Instant::now() < *until => Some(text),
            _ => None,
        }
    }
}

/// Draw the whole window.
///
/// Returns host actions; the caller applies them and, for a theme change, persists the choice.
pub fn show(ui: &mut Ui, state: &mut UiState, engine: &mut Engine) -> Vec<Action> {
    let mut actions = Vec::new();
    let t = state.theme.tokens();

    top_bar(ui, state, &t, &mut actions);
    bottom_bar(ui, state, &t, &mut actions);
    if state.left_open {
        left_panel(ui, state, &t, &mut actions);
    }
    if state.right_open {
        right_panel(ui, state, &t);
    }
    canvas(ui, state, &t, engine, &mut actions);

    if state.settings_open {
        settings_dialog(ui, state, &t, &mut actions);
    }
    if state.about_open {
        about_dialog(ui, state, &t, engine);
    }

    if state.live_notice().is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
    }
    actions
}

fn top_bar(ui: &mut Ui, state: &mut UiState, t: &Tokens, actions: &mut Vec<Action>) {
    egui::Panel::top("top_bar")
        .exact_size(t.top_bar_h)
        .frame(Frame::NONE.fill(t.chrome).inner_margin(Margin::symmetric(10, 0)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(state.title_inset);
                ui.label(RichText::new("Sortlens").size(14.0).strong());
                ui.add_space(10.0);
                ui.label(RichText::new("Local library").size(12.0).color(t.text_muted));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, "Settings", t).clicked() {
                        state.settings_open = true;
                    }
                    if icon_button(ui, "Details", t).clicked() {
                        state.right_open = !state.right_open;
                    }
                    if icon_button(ui, state.theme.label(), t).clicked() {
                        let next = next_theme(state.theme);
                        actions.push(Action::SetTheme(next));
                    }
                    if icon_button(ui, "Filters", t).clicked() {
                        state.notice("Filtering arrives with ratings in M3");
                    }

                    ui.add_space(8.0);
                    let search = ui.add_sized(
                        [240.0, 24.0],
                        TextEdit::singleline(&mut state.search).hint_text("Search photos, folders, collections"),
                    );
                    if search.changed() {
                        // M3 wires this to the query engine; the field is real, the search is not yet.
                        state.notice("Search arrives with the query engine in M3");
                    }
                });
            });
        });
}

fn bottom_bar(ui: &mut Ui, state: &mut UiState, t: &Tokens, actions: &mut Vec<Action>) {
    egui::Panel::bottom("bottom_bar")
        .exact_size(t.bottom_bar_h)
        .frame(Frame::NONE.fill(t.chrome).inner_margin(Margin::symmetric(10, 0)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                for view in View::ALL {
                    let selected = state.view == view;
                    let response = ui.add_enabled(
                        view.milestone().is_none() || selected,
                        Button::new(RichText::new(view.label()).size(12.0))
                            .fill(if selected { t.selection } else { Color32::TRANSPARENT })
                            .stroke(Stroke::NONE),
                    );
                    if let Some(milestone) = view.milestone() {
                        if response.clicked() {
                            actions.push(Action::Notice(format!("{} arrives in {milestone}", view.label())));
                        }
                    } else if response.clicked() {
                        state.view = view;
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("0 photos").size(11.5).color(t.text_muted));
                    ui.add_space(6.0);
                    ui.label(RichText::new("Fit").size(11.5).color(t.text_muted));
                    if let Some(text) = state.live_notice() {
                        ui.add_space(12.0);
                        ui.label(RichText::new(text.to_owned()).size(11.5).color(t.accent));
                    }
                });
            });
        });
}

fn left_panel(ui: &mut Ui, state: &mut UiState, t: &Tokens, actions: &mut Vec<Action>) {
    egui::Panel::left("left_panel")
        .resizable(true)
        .default_size(state.left_w)
        .size_range(200.0..=380.0)
        .frame(Frame::NONE.fill(t.panel).inner_margin(Margin::same(10)))
        .show(ui, |ui| {
            state.left_w = ui.available_width();
            ui.horizontal(|ui| {
                ui.label(RichText::new("Library").size(12.0).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let add = ui
                        .add_enabled(false, Button::new("+").small())
                        .on_disabled_hover_text("Adding folders arrives in M1");
                    if add.clicked() {
                        actions.push(Action::Notice("Adding folders arrives in M1".into()));
                    }
                });
            });
            ui.add_space(6.0);
            ui.separator();

            nav_section(ui, t, "Sources");
            nav_row(ui, t, "This Mac", "0", true);
            nav_row(ui, t, "Camera Card", "—", false);

            nav_section(ui, t, "Folders");
            if engine_is_empty() {
                hint(ui, t, "No folders yet. Add one to index a shoot.");
            }

            nav_section(ui, t, "Collections");
            hint(ui, t, "Collections arrive in M5.");
        });
}

fn right_panel(ui: &mut Ui, state: &mut UiState, t: &Tokens) {
    egui::Panel::right("right_panel")
        .resizable(true)
        .default_size(t.right_w)
        .size_range(240.0..=420.0)
        .frame(Frame::NONE.fill(t.panel).inner_margin(Margin::same(10)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for tab in RightTab::ALL {
                    let active = state.right_tab == tab;
                    if ui
                        .add(
                            Button::new(
                                RichText::new(tab.label())
                                    .size(11.5)
                                    .color(if active { t.text } else { t.text_muted }),
                            )
                            .fill(if active { t.selection } else { Color32::TRANSPARENT })
                            .stroke(Stroke::NONE),
                        )
                        .clicked()
                    {
                        state.right_tab = tab;
                    }
                }
            });
            ui.separator();
            ui.add_space(4.0);
            match state.right_tab {
                RightTab::Details => {
                    hint(ui, t, "Select a photo to see camera, exposure and file details.");
                }
                RightTab::Tags => {
                    hint(ui, t, "Keywords and collections for the selection appear here.");
                }
                RightTab::Activity => {
                    hint(ui, t, "Background jobs appear here: indexing, previews, exports.");
                }
            }
        });
}

fn canvas(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    egui::CentralPanel::default()
        .frame(Frame::NONE.fill(t.bg).inner_margin(Margin::same(14)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(state.view.label()).size(16.0).strong());
                ui.add_space(6.0);
                ui.label(RichText::new("0 photos").size(12.0).color(t.text_muted));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("Grid").size(11.5).color(t.text_muted));
                });
            });
            ui.add_space(8.0);
            filter_strip(ui, t, state, actions);
            ui.add_space(10.0);

            match state.view {
                View::Library => empty_library(ui, t, state, engine, actions),
                other => planned_view(ui, t, other),
            }
        });
}

fn filter_strip(ui: &mut Ui, t: &Tokens, state: &mut UiState, actions: &mut Vec<Action>) {
    Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(t.corner())
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                for label in ["Rating", "Flags", "Colour", "Kind", "Edited"] {
                    let response = ui
                        .add_enabled(
                            false,
                            Button::new(RichText::new(label).size(11.0))
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE),
                        )
                        .on_disabled_hover_text("Filters arrive with ratings in M3");
                    if response.clicked() {
                        actions.push(Action::Notice("Filters arrive in M3".into()));
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("No filters").size(11.0).color(t.text_faint));
                });
            });
        });
    let _ = state;
}

fn empty_library(ui: &mut Ui, t: &Tokens, state: &mut UiState, _engine: &mut Engine, actions: &mut Vec<Action>) {
    let height = ui.available_height();
    ui.allocate_ui_with_layout(vec2(ui.available_width(), height), Layout::top_down(Align::Center), |ui| {
        ui.add_space((height * 0.18).clamp(24.0, 160.0));
        ui.label(RichText::new("No photos yet").size(19.0).strong());
        ui.add_space(4.0);
        ui.label(
            RichText::new("Add a folder and Sortlens indexes it in place. Nothing is uploaded, moved or renamed.")
                .size(12.5)
                .color(t.text_muted),
        );
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            let add = ui
                .add_enabled(
                    false,
                    Button::new(RichText::new("Add Folder…").size(12.5)).min_size(vec2(120.0, 28.0)),
                )
                .on_disabled_hover_text("Folder indexing arrives in M1");
            if add.clicked() {
                actions.push(Action::Notice("Folder indexing arrives in M1".into()));
            }
            if ui.button(RichText::new("Settings").size(12.5)).clicked() {
                state.settings_open = true;
            }
        });
        ui.add_space(18.0);
        ui.label(
            RichText::new("Supported: JPEG, HEIC, TIFF, PNG, WebP and camera raw.")
                .size(11.0)
                .color(t.text_faint),
        );
    });
}

fn planned_view(ui: &mut Ui, t: &Tokens, view: View) {
    let milestone = view.milestone().unwrap_or("a later milestone");
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), ui.available_height()),
        Layout::top_down(Align::Center),
        |ui| {
            ui.add_space(80.0);
            ui.label(RichText::new(format!("{} is not built yet", view.label())).size(17.0).strong());
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("Planned for {milestone}. The command and its shortcuts exist already."))
                    .size(12.0)
                    .color(t.text_muted),
            );
        },
    );
}

fn settings_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, actions: &mut Vec<Action>) {
    let mut open = state.settings_open;
    egui::Window::new("Settings")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(360.0)
        .show(ui.ctx(), |ui| {
            ui.label(RichText::new("Appearance").size(11.0).color(t.text_muted));
            for kind in ThemeKind::ALL {
                if ui.radio(state.theme == kind, kind.label()).clicked() && state.theme != kind {
                    actions.push(Action::SetTheme(kind));
                }
            }
            ui.add_space(8.0);
            ui.separator();
            ui.label(
                RichText::new("More settings arrive with the features they configure.")
                    .size(11.0)
                    .color(t.text_faint),
            );
        });
    state.settings_open = open;
}

fn about_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &Engine) {
    let mut open = state.about_open;
    egui::Window::new("About Sortlens")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(340.0)
        .show(ui.ctx(), |ui| {
            ui.label(RichText::new("Sortlens").size(17.0).strong());
            ui.label(
                RichText::new(format!("Version {}", sortlens_core::VERSION))
                    .size(12.0)
                    .color(t.text_muted),
            );
            ui.add_space(6.0);
            ui.label("Fast, local culling for photographers who shoot thousands of images.");
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!(
                    "{} commands, {} ready today.",
                    engine.registry().all().len(),
                    engine.registry().ready_count()
                ))
                .size(11.0)
                .color(t.text_faint),
            );
        });
    state.about_open = open;
}

fn nav_section(ui: &mut Ui, t: &Tokens, title: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(title.to_uppercase()).size(9.5).color(t.text_faint));
    ui.add_space(2.0);
}

fn nav_row(ui: &mut Ui, t: &Tokens, label: &str, count: &str, selected: bool) -> egui::Response {
    let height = 24.0;
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let hovered = response.hovered();
    if selected || hovered {
        ui.painter()
            .rect_filled(rect, t.corner(), if selected { t.selection } else { t.hover });
    }
    let text_color = t.text;
    ui.painter().text(
        rect.left_center() + vec2(6.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        t.font(12.0),
        text_color,
    );
    ui.painter().text(
        rect.right_center() - vec2(6.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        count,
        t.font(11.0),
        t.text_faint,
    );
    response
}

fn hint(ui: &mut Ui, t: &Tokens, text: &str) {
    ui.label(RichText::new(text).size(11.0).color(t.text_faint));
}

fn icon_button(ui: &mut Ui, label: &str, t: &Tokens) -> egui::Response {
    ui.add(
        Button::new(RichText::new(label).size(11.5).color(t.text_muted))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE),
    )
}

fn next_theme(kind: ThemeKind) -> ThemeKind {
    let index = ThemeKind::ALL.iter().position(|candidate| *candidate == kind).unwrap_or(0);
    ThemeKind::ALL[(index + 1) % ThemeKind::ALL.len()]
}

fn engine_is_empty() -> bool {
    true
}
