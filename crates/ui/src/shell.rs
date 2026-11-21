//! The application shell: five surfaces, in the suite's order.
//!
//! Top bar (identity, search, filters, actions), left panel (sources, folders, collections),
//! canvas (grid today, cull and detail later), right panel (details, tags, activity), bottom bar
//! (views, review state, counts).
//!
//! The shell owns presentation state and nothing else. Anything that changes the library goes
//! through [`Engine`]; anything that changes the process (theme, folder picker, quit) is returned
//! as an [`Action`] for the host to apply.

use crate::theme::{ThemeKind, Tokens};
use crate::thumbs::{GRID_SIZE, Thumbnails};
use egui::{
    Align, Align2, Button, Color32, Frame, Key, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2, pos2, vec2,
};
use sortlens_core::{Flag, MediaKind, PhotoId};
use sortlens_engine::{Decision, Engine, Filter, Photo, Sort};

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
    /// Ask the host for a folder to index.
    PickFolder,
    /// Rescan an already indexed root.
    Rescan(String),
    /// Remove a root from the library.
    RemoveRoot(String),
    /// Sidecar writing was switched on or off.
    SetSidecars(bool),
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
    /// Root filter, if the user picked one folder in the sidebar.
    pub root: Option<String>,
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
    /// Selected photo.
    pub selected: Option<PhotoId>,
    /// Sidebar row for "All Photos".
    pub all_photos_selected: bool,
    /// Minimum stars, 0 for any.
    pub min_rating: u8,
    /// Only this flag.
    pub flag_filter: Option<Flag>,
    /// Only this colour label.
    pub label_filter: Option<sortlens_core::ColorLabel>,
    /// Whether missing files are shown.
    pub show_missing: bool,
    /// Only this keyword, chosen in the sidebar or the filter strip.
    pub tag_filter: Option<String>,
    /// Text in the keyword editor.
    pub tag_input: String,
    /// Ask the keyword field for focus on the next frame.
    pub tag_focus: bool,
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
            root: None,
            left_open: true,
            left_w: t.left_w,
            right_open: true,
            right_tab: RightTab::Details,
            title_inset,
            settings_open: false,
            about_open: false,
            selected: None,
            all_photos_selected: true,
            min_rating: 0,
            flag_filter: None,
            label_filter: None,
            show_missing: false,
            tag_filter: None,
            tag_input: String::new(),
            tag_focus: false,
            notice: None,
        }
    }

    /// Show a message in the bottom bar for a few seconds.
    pub fn notice(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), std::time::Instant::now() + std::time::Duration::from_secs(4)));
    }

    fn live_notice(&self) -> Option<&str> {
        match &self.notice {
            Some((text, until)) if std::time::Instant::now() < *until => Some(text),
            _ => None,
        }
    }

    /// The filter implied by the sidebar, the search field and the filter bar.
    pub fn filter(&self) -> Filter {
        Filter {
            root: self.root.clone(),
            text: self.search.clone(),
            min_rating: self.min_rating,
            flag: self.flag_filter,
            label: self.label_filter,
            tag: self.tag_filter.clone(),
            include_missing: self.show_missing,
            ..Filter::default()
        }
    }

    /// Whether any filter is narrowing the view.
    pub fn has_filters(&self) -> bool {
        self.min_rating > 0
            || self.flag_filter.is_some()
            || self.label_filter.is_some()
            || self.tag_filter.is_some()
            || self.show_missing
            || self.root.is_some()
            || !self.search.is_empty()
    }

    /// Remove every filter except the search text.
    pub fn clear_filters(&mut self) {
        self.min_rating = 0;
        self.flag_filter = None;
        self.label_filter = None;
        self.tag_filter = None;
        self.show_missing = false;
        self.root = None;
    }
}

/// Draw the whole window.
///
/// Returns host actions; the caller applies them and, for a theme change, persists the choice.
pub fn show(ui: &mut Ui, state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails) -> Vec<Action> {
    let mut actions = Vec::new();
    let t = state.theme.tokens();

    if let Some(done) = engine.poll() {
        if done.cancelled {
            state.notice(format!("Cancelled indexing {}", folder_name(&done.root)));
        } else {
            let mut message = format!("Indexed {} photos in {}", done.added + done.updated, folder_name(&done.root));
            if done.missing > 0 {
                message.push_str(&format!(" · {} missing", done.missing));
            }
            if !done.errors.is_empty() {
                message.push_str(&format!(" · {} unreadable", done.errors.len()));
            }
            state.notice(message);
        }
    }
    thumbs.poll(ui.ctx());
    if engine.is_scanning() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
    }

    top_bar(ui, state, &t, &mut actions);
    bottom_bar(ui, state, &t, engine, &mut actions);
    if state.left_open {
        left_panel(ui, state, &t, engine, &mut actions);
    }
    if state.right_open {
        right_panel(ui, state, &t, engine);
    }
    canvas(ui, state, &t, engine, thumbs, &mut actions);

    if state.settings_open {
        settings_dialog(ui, state, &t, engine, &mut actions);
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
                        actions.push(Action::SetTheme(next_theme(state.theme)));
                    }
                    if icon_button(ui, "Add Folder…", t).clicked() {
                        actions.push(Action::PickFolder);
                    }

                    ui.add_space(8.0);
                    let search = ui.add_sized(
                        [240.0, 24.0],
                        TextEdit::singleline(&mut state.search).hint_text("Filter by name or folder"),
                    );
                    if search.changed() {
                        state.selected = None;
                    }
                });
            });
        });
}

fn bottom_bar(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    let stats = engine.stats();
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
                    if response.clicked() {
                        match view.milestone() {
                            Some(milestone) => actions.push(Action::Notice(format!("{} arrives in {milestone}", view.label()))),
                            None => state.view = view,
                        }
                    }
                }

                ui.add_space(12.0);
                match engine.scan_progress() {
                    Some(progress) if engine.is_scanning() => {
                        ui.label(
                            RichText::new(format!(
                                "Indexing · {} files · {} folders",
                                format_count(progress.files as u64),
                                format_count(progress.dirs as u64)
                            ))
                            .size(11.5)
                            .color(t.accent),
                        );
                    }
                    _ => {
                        ui.label(RichText::new(summary(&stats)).size(11.5).color(t.text_muted));
                    }
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(text) = state.live_notice() {
                        ui.label(RichText::new(text.to_owned()).size(11.5).color(t.accent));
                        ui.add_space(10.0);
                    }
                    if let Some(selected) = state.selected.and_then(|id| engine.photo(id)) {
                        ui.label(RichText::new(format!("{}★", selected.rating)).size(11.5).color(t.text_muted));
                        ui.label(RichText::new(selected.name.clone()).size(11.5).color(t.text_muted));
                    } else {
                        ui.label(RichText::new("No selection").size(11.5).color(t.text_faint));
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new("Fit").size(11.5).color(t.text_faint));
                });
            });
        });
}

fn left_panel(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    egui::Panel::left("left_panel")
        .resizable(true)
        .default_size(state.left_w)
        .size_range(210.0..=380.0)
        .frame(Frame::NONE.fill(t.panel).inner_margin(Margin::same(10)))
        .show(ui, |ui| {
            state.left_w = ui.available_width();
            ui.horizontal(|ui| {
                ui.label(RichText::new("Library").size(12.0).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(Button::new("+").small()).on_hover_text("Add a folder to index").clicked() {
                        actions.push(Action::PickFolder);
                    }
                });
            });
            ui.add_space(6.0);
            ui.separator();

            let stats = engine.stats();
            nav_section(ui, t, "Sources");
            if nav_row(ui, t, "All Photos", &format_count(stats.total as u64), state.root.is_none()).clicked() {
                state.root = None;
                state.all_photos_selected = true;
            }
            if let Some(progress) = engine.scan_progress()
                && engine.is_scanning()
            {
                ui.label(
                    RichText::new(format!("Scanning · {} files", format_count(progress.files as u64)))
                        .size(11.0)
                        .color(t.accent),
                );
            }

            nav_section(ui, t, "Folders");
            let roots: Vec<(String, usize)> = engine.counts_by_root();
            if roots.is_empty() {
                hint(ui, t, "No folders yet. Add one to index a shoot.");
            }
            for (root, count) in roots {
                let selected = state.root.as_deref() == Some(root.as_str());
                let response = nav_row(ui, t, &folder_name(&root), &format_count(count as u64), selected);
                if response.clicked() {
                    state.root = Some(root.clone());
                    state.all_photos_selected = false;
                }
                response.context_menu(|ui| {
                    if ui.button("Rescan").clicked() {
                        actions.push(Action::Rescan(root.clone()));
                        ui.close();
                    }
                    if ui.button("Remove from library").clicked() {
                        actions.push(Action::RemoveRoot(root.clone()));
                        ui.close();
                    }
                });
            }

            nav_section(ui, t, "Keywords");
            let vocabulary = engine.tag_vocabulary();
            if vocabulary.is_empty() {
                hint(ui, t, "Select a photo, press K and type a keyword.");
            }
            for (tag, count) in vocabulary.iter().take(12) {
                let selected = state.tag_filter.as_deref() == Some(tag.as_str());
                if nav_row(ui, t, tag, &format_count(*count as u64), selected).clicked() {
                    state.tag_filter = if selected { None } else { Some(tag.clone()) };
                }
            }

            nav_section(ui, t, "Collections");
            hint(ui, t, "Collections arrive in M5.");
        });
}

fn right_panel(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine) {
    egui::Panel::right("right_panel")
        .resizable(true)
        .default_size(t.right_w)
        .size_range(250.0..=420.0)
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
                RightTab::Details => match state.selected.and_then(|id| engine.photo(id)) {
                    Some(photo) => details(ui, t, photo),
                    None => hint(ui, t, "Select a photo to see camera, exposure and file details."),
                },
                RightTab::Tags => match state.selected.and_then(|id| engine.photo(id)).cloned() {
                    Some(photo) => tags(ui, t, state, engine, &photo),
                    None => hint(ui, t, "Keywords and collections for the selection appear here."),
                },
                RightTab::Activity => {
                    let jobs = engine.jobs();
                    if jobs.is_empty() {
                        hint(ui, t, "Nothing has run yet. Scans and exports appear here.");
                    }
                    for job in jobs.iter().rev().take(8) {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(job.kind.label()).size(10.0).color(t.text_faint));
                            ui.label(RichText::new(truncate(&job.label, 22)).size(11.5).color(t.text));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let colour = match job.status {
                                    sortlens_engine::JobStatus::Running => t.accent,
                                    sortlens_engine::JobStatus::Done => t.pick,
                                    sortlens_engine::JobStatus::Cancelled => t.text_faint,
                                    sortlens_engine::JobStatus::Failed => t.reject,
                                };
                                ui.label(RichText::new(job.status.label()).size(10.0).color(colour));
                                ui.label(
                                    RichText::new(format!("{:.1}s", job.elapsed().as_secs_f64()))
                                        .size(10.0)
                                        .color(t.text_faint),
                                );
                            });
                        });
                        ui.label(RichText::new(job.detail()).size(10.5).color(t.text_muted));
                        if job.status.is_running() {
                            if ui.button(RichText::new("Cancel").size(10.5)).clicked() {
                                engine.cancel_scan();
                            }
                        } else if let Some(error) = &job.error {
                            ui.label(RichText::new(truncate(error, 60)).size(10.0).color(t.reject));
                        }
                        ui.add_space(6.0);
                    }

                    ui.separator();
                    let stats = engine.stats();
                    detail_row(ui, t, "Indexed", &format_count(stats.total as u64));
                    detail_row(ui, t, "On disk", &sortlens_core::format_bytes(stats.bytes));
                    detail_row(ui, t, "Missing", &format_count(stats.missing as u64));
                    if !engine.warnings().is_empty() {
                        ui.add_space(6.0);
                        for warning in engine.warnings().iter().rev().take(4) {
                            ui.label(RichText::new(truncate(warning, 70)).size(10.0).color(t.text_faint));
                        }
                    }
                }
            }
        });
}

fn details(ui: &mut Ui, t: &Tokens, photo: &Photo) {
    ui.label(RichText::new(photo.name.clone()).size(13.0).strong());
    ui.add_space(6.0);
    let kind = match photo.kind {
        MediaKind::Raw => "Raw",
        MediaKind::Image => "Image",
        MediaKind::Video => "Video",
    };
    detail_row(ui, t, "Kind", kind);
    detail_row(ui, t, "Folder", if photo.folder().is_empty() { "—" } else { photo.folder() });
    detail_row(ui, t, "Size", &sortlens_core::format_bytes(photo.size));
    if photo.width > 0 && photo.height > 0 {
        detail_row(ui, t, "Dimensions", &format!("{} × {}", photo.width, photo.height));
    }
    if !photo.camera.is_empty() {
        detail_row(ui, t, "Camera", &photo.camera);
        detail_row(ui, t, "Lens", if photo.lens.is_empty() { "—" } else { &photo.lens });
        detail_row(ui, t, "Exposure", &photo.exposure());
    }
    if let Some(captured) = photo.captured_label() {
        detail_row(ui, t, "Captured", &captured);
    }
    detail_row(ui, t, "Rating", &format!("{} of 5", photo.rating));
    detail_row(ui, t, "Flag", photo.flag.id());
    detail_row(ui, t, "Label", photo.label.map(|label| label.label()).unwrap_or("—"));
    detail_row(ui, t, "Status", if photo.missing { "missing" } else { "present" });
    ui.add_space(8.0);
    ui.label(
        RichText::new("Press 1–5 to rate, 0 to clear, P to pick, X to reject, U to unflag.")
            .size(10.5)
            .color(t.text_faint),
    );
}

fn tags(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, photo: &Photo) {
    match photo.label {
        Some(label) => {
            ui.label(RichText::new(label.label()).size(12.0).color(t.text));
        }
        None => hint(ui, t, "No colour label."),
    }
    ui.add_space(8.0);

    ui.label(RichText::new("Keywords").size(12.0).strong().color(t.text));
    let mut remove = None;
    if photo.tags.is_empty() {
        hint(ui, t, "No keywords yet.");
    }
    ui.horizontal_wrapped(|ui| {
        for tag in &photo.tags {
            let chip = Button::new(RichText::new(format!("{tag}  ×")).size(11.0).color(t.text))
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(t.corner());
            if ui.add(chip).on_hover_text("Remove keyword").clicked() {
                remove = Some(tag.clone());
            }
        }
    });
    if let Some(tag) = remove {
        match engine.set_tag(&[photo.id], &tag, false) {
            Ok(_) => state.notice(format!("Removed keyword {tag}")),
            Err(error) => state.notice(error.to_string()),
        }
    }
    ui.add_space(6.0);

    ui.horizontal(|ui| {
        let field = ui.add(
            TextEdit::singleline(&mut state.tag_input)
                .hint_text("Add keyword")
                .desired_width(150.0)
                .font(egui::TextStyle::Small),
        );
        if state.tag_focus {
            field.request_focus();
            state.tag_focus = false;
        }
        let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
        let clicked = ui.add(Button::new(RichText::new("Add").size(11.0))).clicked();
        if (entered || clicked) && !state.tag_input.trim().is_empty() {
            match engine.set_tag(&[photo.id], &state.tag_input, true) {
                Ok(0) => state.notice("That keyword is already on this photo"),
                Ok(_) => {
                    state.notice(format!("Added keyword {}", state.tag_input.trim().to_lowercase()));
                    state.tag_input.clear();
                }
                Err(error) => state.notice(error.to_string()),
            }
        }
    });

    let suggestions: Vec<(String, usize)> = engine
        .tag_vocabulary()
        .into_iter()
        .filter(|(tag, _)| !photo.has_tag(tag))
        .take(8)
        .collect();
    if !suggestions.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new("Used elsewhere").size(10.5).color(t.text_faint));
        let mut add = None;
        ui.horizontal_wrapped(|ui| {
            for (tag, _) in &suggestions {
                if ui
                    .add(Button::new(RichText::new(format!("+ {tag}")).size(10.5)).fill(t.card))
                    .clicked()
                {
                    add = Some(tag.clone());
                }
            }
        });
        if let Some(tag) = add
            && let Err(error) = engine.set_tag(&[photo.id], &tag, true)
        {
            state.notice(error.to_string());
        }
    }
}

fn canvas(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, thumbs: &mut Thumbnails, actions: &mut Vec<Action>) {
    egui::CentralPanel::default()
        .frame(Frame::NONE.fill(t.bg).inner_margin(Margin::same(14)))
        .show(ui, |ui| {
            let filter = state.filter();
            let total = engine.count(&filter);
            ui.horizontal(|ui| {
                ui.label(RichText::new(state.view.label()).size(16.0).strong());
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("{} photos", format_count(total as u64)))
                        .size(12.0)
                        .color(t.text_muted),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if state.root.is_some() && icon_button(ui, "All Photos", t).clicked() {
                        state.root = None;
                    }
                    ui.label(RichText::new("Added order").size(11.5).color(t.text_faint));
                });
            });
            ui.add_space(8.0);
            filter_strip(ui, t, state, total);
            ui.add_space(10.0);

            match state.view {
                View::Library => {
                    if total == 0 {
                        empty_library(ui, t, state, engine, actions);
                    } else {
                        grid(ui, t, state, engine, thumbs, &filter);
                    }
                }
                View::Cull => cull_view(ui, t, state, engine, thumbs),
                other => planned_view(ui, t, other),
            }
            keyboard(ui, state, engine);
        });
}

fn grid(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &Engine, thumbs: &mut Thumbnails, filter: &Filter) {
    let gap = t.grid_gap;
    let available = ui.available_width();
    let min_cell = 132.0;
    let columns = (((available + gap) / (min_cell + gap)).floor() as usize).max(1);
    let cell = ((available - gap * (columns.saturating_sub(1)) as f32) / columns as f32)
        .floor()
        .max(64.0);
    let caption = 24.0;
    let row_height = cell + caption + gap;
    let total = engine.count(filter);
    let rows = total.div_ceil(columns);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, rows, |ui, range| {
            ui.spacing_mut().item_spacing = vec2(gap, 0.0);
            let offset = range.start * columns;
            let limit = (range.end - range.start) * columns;
            let page = engine.page(filter, Sort::Natural, offset, limit);
            for row in 0..(range.end - range.start) {
                ui.horizontal(|ui| {
                    for column in 0..columns {
                        let index = row * columns + column;
                        match page.photos.get(index) {
                            Some(photo) => {
                                let selected = state.selected == Some(photo.id);
                                let response = photo_cell(ui, t, photo, selected, cell, caption, thumbs);
                                if response.clicked() {
                                    state.selected = Some(photo.id);
                                    state.right_tab = RightTab::Details;
                                }
                                if response.double_clicked() {
                                    state.notice("The detail view arrives in M4");
                                }
                            }
                            None => {
                                let (_, response) = ui.allocate_exact_size(vec2(cell, cell + caption), Sense::hover());
                                let _ = response;
                            }
                        }
                    }
                });
                ui.add_space(gap);
            }
        });
}

fn photo_cell(ui: &mut Ui, t: &Tokens, photo: &Photo, selected: bool, cell: f32, caption: f32, thumbs: &mut Thumbnails) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(cell, cell + caption), Sense::click());
    let image = Rect::from_min_size(rect.min, vec2(cell, cell));
    let painter = ui.painter();

    painter.rect_filled(image, t.corner(), if response.hovered() { t.hover } else { t.thumb_bg });

    let texture = thumbs.get(ui.ctx(), photo, GRID_SIZE).cloned();
    match &texture {
        Some(texture) => {
            let uv = cover_uv(texture.size_vec2(), image.size());
            painter.image(texture.id(), image, uv, Color32::WHITE);
        }
        None => {
            let glyph = match photo.kind {
                MediaKind::Raw => "RAW",
                MediaKind::Video => "VID",
                MediaKind::Image => "IMG",
            };
            painter.text(
                image.center() - vec2(0.0, 4.0),
                Align2::CENTER_CENTER,
                glyph,
                t.font(10.0),
                t.text_faint,
            );
        }
    }

    if photo.flag != Flag::None {
        let colour = t.flag(photo.flag);
        let badge = Rect::from_min_size(image.min + vec2(5.0, 5.0), vec2(7.0, 7.0));
        painter.rect_filled(badge, 3.0, colour);
    }

    // Stars sit along the bottom edge of the thumbnail so the eye can scan a whole row.
    for star in 0..5 {
        let centre = image.left_bottom() + vec2(9.0 + star as f32 * 10.0, -9.0);
        let colour = if star < photo.rating { t.star() } else { t.text_faint };
        painter.circle_filled(centre, 2.6, colour);
    }

    painter.text(
        rect.left_top() + vec2(2.0, cell + 5.0),
        Align2::LEFT_TOP,
        truncate(&photo.name, (cell / 6.2) as usize),
        t.font(11.0),
        t.text,
    );

    if selected {
        painter.rect_stroke(image, t.corner(), Stroke::new(2.0, t.accent), StrokeKind::Inside);
    }
    response
}

fn keyboard(ui: &mut Ui, state: &mut UiState, engine: &mut Engine) {
    if ui.memory(|memory| memory.focused()).is_some() {
        return;
    }
    if cull_keyboard(ui, engine) {
        return;
    }
    // Starting a session works before anything is selected: the queue comes from the filters.
    // A session restored from a previous run is opened rather than replaced.
    if ui.input(|input| input.key_pressed(Key::C) && !input.modifiers.command) {
        state.view = View::Cull;
        if engine.cull().is_none() {
            match start_cull(engine, state) {
                Ok(total) => state.notice(format!("Culling {total} photos")),
                Err(error) => state.notice(error),
            }
        }
        return;
    }
    let Some(id) = state.selected else {
        return;
    };

    let mut rating = None;
    let mut flag = None;
    let mut keyword = false;
    let culling = engine.cull().is_some();
    ui.input(|input| {
        for (key, stars) in [
            (Key::Num0, 0u8),
            (Key::Num1, 1),
            (Key::Num2, 2),
            (Key::Num3, 3),
            (Key::Num4, 4),
            (Key::Num5, 5),
        ] {
            if input.key_pressed(key) && !input.modifiers.command {
                rating = Some(stars);
            }
        }
        // In a cull session P/X/U belong to the session, which stages decisions instead of
        // writing flags one photo at a time.
        if !culling && input.key_pressed(Key::P) {
            flag = Some(Flag::Pick);
        }
        if !culling && input.key_pressed(Key::X) {
            flag = Some(Flag::Reject);
        }
        if !culling && input.key_pressed(Key::U) {
            flag = Some(Flag::None);
        }
        if input.key_pressed(Key::K) && !input.modifiers.command {
            keyword = true;
        }
    });

    if keyword {
        state.right_tab = RightTab::Tags;
        state.right_open = true;
        state.tag_focus = true;
    }

    if let Some(stars) = rating {
        match engine.rate(&[id], stars) {
            Ok(_) => state.notice(if stars == 0 {
                "Cleared rating".into()
            } else {
                format!("Rated {stars}★")
            }),
            Err(error) => state.notice(error.to_string()),
        }
    }
    if let Some(flag) = flag {
        match engine.flag(&[id], flag) {
            Ok(_) => state.notice(match flag {
                Flag::Pick => "Picked".to_string(),
                Flag::Reject => "Rejected".to_string(),
                Flag::None => "Unflagged".to_string(),
            }),
            Err(error) => state.notice(error.to_string()),
        }
    }
}

fn filter_strip(ui: &mut Ui, t: &Tokens, state: &mut UiState, total: usize) {
    Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(t.corner())
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let rating = if state.min_rating == 0 {
                    "Rating".to_string()
                } else {
                    format!("{}★ and up", state.min_rating)
                };
                if let Some(tag) = state.tag_filter.clone()
                    && chip(ui, t, &format!("#{tag}"), true).clicked()
                {
                    state.tag_filter = None;
                }
                if chip(ui, t, &rating, state.min_rating > 0).clicked() {
                    state.min_rating = (state.min_rating + 1) % 6;
                }

                let flag_label = match state.flag_filter {
                    None => "Flags".to_string(),
                    Some(Flag::Pick) => "Picks".to_string(),
                    Some(Flag::Reject) => "Rejects".to_string(),
                    Some(Flag::None) => "Unflagged".to_string(),
                };
                if chip(ui, t, &flag_label, state.flag_filter.is_some()).clicked() {
                    state.flag_filter = match state.flag_filter {
                        None => Some(Flag::Pick),
                        Some(Flag::Pick) => Some(Flag::Reject),
                        Some(Flag::Reject) => Some(Flag::None),
                        Some(Flag::None) => None,
                    };
                }

                let label_text = match state.label_filter {
                    None => "Colour".to_string(),
                    Some(label) => label.label().to_string(),
                };
                if chip(ui, t, &label_text, state.label_filter.is_some()).clicked() {
                    state.label_filter = match state.label_filter {
                        None => Some(sortlens_core::ColorLabel::ALL[0]),
                        Some(current) => {
                            let index = sortlens_core::ColorLabel::ALL
                                .iter()
                                .position(|label| *label == current)
                                .unwrap_or(0);
                            sortlens_core::ColorLabel::ALL.get(index + 1).copied()
                        }
                    };
                }

                if chip(ui, t, "Missing", state.show_missing).clicked() {
                    state.show_missing = !state.show_missing;
                }

                if state.has_filters() && chip(ui, t, "Clear", true).clicked() {
                    state.search.clear();
                    state.clear_filters();
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{} shown", format_count(total as u64)))
                            .size(11.0)
                            .color(t.text_faint),
                    );
                });
            });
        });
}

fn chip(ui: &mut Ui, t: &Tokens, label: &str, active: bool) -> egui::Response {
    ui.add(
        Button::new(RichText::new(label).size(11.0).color(if active { t.text } else { t.text_muted }))
            .fill(if active { t.selection } else { Color32::TRANSPARENT })
            .stroke(if active { Stroke::new(1.0, t.accent) } else { Stroke::NONE }),
    )
}

fn empty_library(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &Engine, actions: &mut Vec<Action>) {
    let height = ui.available_height();
    ui.allocate_ui_with_layout(vec2(ui.available_width(), height), Layout::top_down(Align::Center), |ui| {
        ui.add_space((height * 0.18).clamp(24.0, 160.0));
        if engine.is_scanning() {
            ui.label(RichText::new("Indexing…").size(19.0).strong());
            ui.add_space(4.0);
            ui.label(
                RichText::new("Reading file sizes and content keys. Nothing is moved.")
                    .size(12.5)
                    .color(t.text_muted),
            );
            return;
        }
        ui.label(RichText::new("No photos yet").size(19.0).strong());
        ui.add_space(4.0);
        ui.label(
            RichText::new("Add a folder and Sortlens indexes it in place. Nothing is uploaded, moved or renamed.")
                .size(12.5)
                .color(t.text_muted),
        );
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if ui
                .add(Button::new(RichText::new("Add Folder…").size(12.5)).min_size(vec2(120.0, 28.0)))
                .clicked()
            {
                actions.push(Action::PickFolder);
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

/// Start a session over what the filters currently show. A session is a fixed queue: changing the
/// filters afterwards does not silently change what is being culled.
/// The largest rect of a given aspect ratio that fits inside `bounds`, centred.
fn fit_rect(bounds: Rect, aspect: f32) -> Rect {
    let aspect = if aspect.is_finite() && aspect > 0.0 { aspect } else { 1.5 };
    let (mut width, mut height) = (bounds.width(), bounds.height());
    if width / height > aspect {
        width = height * aspect;
    } else {
        height = width / aspect;
    }
    Rect::from_center_size(bounds.center(), vec2(width, height))
}

fn start_cull(engine: &mut Engine, state: &UiState) -> std::result::Result<usize, String> {
    const QUEUE_CAP: usize = 500;
    let ids: Vec<sortlens_core::PhotoId> = engine
        .page(&state.filter(), Sort::Natural, 0, QUEUE_CAP)
        .photos
        .iter()
        .map(|photo| photo.id)
        .collect();
    engine.begin_cull(&ids).map_err(|error| error.to_string())
}

fn cull_keyboard(ui: &mut Ui, engine: &mut Engine) -> bool {
    if engine.cull().is_none() {
        return false;
    }
    let mut decision = None;
    let mut back = false;
    ui.input(|input| {
        if input.key_pressed(Key::P) && !input.modifiers.command {
            decision = Some(Decision::Pick);
        }
        if input.key_pressed(Key::X) && !input.modifiers.command {
            decision = Some(Decision::Reject);
        }
        if input.key_pressed(Key::Space) {
            decision = Some(Decision::Deferred);
        }
        if input.key_pressed(Key::U) && !input.modifiers.command {
            decision = Some(Decision::Unjudged);
        }
        if input.key_pressed(Key::ArrowLeft) {
            back = true;
        }
    });
    if let Some(decision) = decision {
        engine.cull_decide(decision);
    } else if back {
        engine.cull_back();
    }
    true
}

fn cull_view(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails) {
    let snapshot = engine.cull().map(|session| {
        let queue = session.queue().to_vec();
        let cursor = session.cursor();
        let summary = session.summary();
        let current = session.current();
        let up_next: Vec<sortlens_core::PhotoId> = queue.iter().skip(cursor + 1).take(4).copied().collect();
        let decision = current.map(|id| session.decision(id)).unwrap_or_default();
        (current, decision, cursor, queue.len(), summary, up_next)
    });

    let Some((current, decision, cursor, total, summary, up_next)) = snapshot else {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(RichText::new("Cull").size(20.0).strong());
            ui.add_space(6.0);
            let photos = engine.count(&state.filter());
            if photos == 0 {
                ui.label(RichText::new("No photos match the current filters.").size(12.0).color(t.text_muted));
                return;
            }
            ui.label(
                RichText::new(format!("{photos} photos in view. Nothing moves until you apply."))
                    .size(12.0)
                    .color(t.text_muted),
            );
            ui.add_space(10.0);
            if ui
                .add(Button::new(RichText::new(format!("Start culling {photos} photos")).size(12.5)))
                .clicked()
            {
                match start_cull(engine, state) {
                    Ok(total) => state.notice(format!("Culling {total} photos")),
                    Err(error) => state.notice(error),
                }
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new("P pick · X reject · Space defer · U clear · ← back")
                    .size(10.5)
                    .color(t.text_faint),
            );
        });
        return;
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new("Cull").size(16.0).strong());
        ui.add_space(6.0);
        let position = if total == 0 { 0 } else { (cursor + 1).min(total) };
        ui.label(RichText::new(format!("{position} of {total}")).size(12.5).color(t.text_muted));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.add(Button::new(RichText::new("End session").size(11.0))).clicked() {
                engine.cull_clear();
            }
            ui.label(
                RichText::new(format!(
                    "{} picked · {} rejected · {} deferred",
                    summary.picks, summary.rejects, summary.deferred
                ))
                .size(11.0)
                .color(t.text_faint),
            );
        });
    });

    // Progress through the queue.
    let progress = if total == 0 {
        0.0
    } else {
        (total - summary.unjudged) as f32 / total as f32
    };
    let (bar, _) = ui.allocate_exact_size(vec2(ui.available_width(), 3.0), Sense::hover());
    ui.painter().rect_filled(bar, t.corner(), t.border);
    let mut filled = bar;
    filled.set_width(bar.width() * progress);
    ui.painter().rect_filled(filled, t.corner(), t.accent);
    ui.add_space(8.0);

    let strip_height = 84.0;
    let action_height = 46.0;
    let image_height = (ui.available_height() - strip_height - action_height - 16.0).max(160.0);
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), image_height), Sense::hover());

    match current.and_then(|id| engine.photo(id).cloned()) {
        Some(photo) => {
            let painter = ui.painter();
            painter.rect_filled(area, t.corner(), t.thumb_bg);
            let aspect = if photo.width > 0 && photo.height > 0 {
                photo.width as f32 / photo.height as f32
            } else {
                1.5
            };
            let fitted = fit_rect(area.shrink(8.0), aspect);
            match thumbs.get(ui.ctx(), &photo, 1600).cloned() {
                Some(texture) => {
                    let uv = cover_uv(texture.size_vec2(), fitted.size());
                    painter.image(texture.id(), fitted, uv, Color32::WHITE);
                }
                None => {
                    painter.text(fitted.center(), Align2::CENTER_CENTER, &photo.rel, t.font(11.0), t.text_faint);
                }
            }
            let badge = match decision {
                Decision::Pick => Some(("PICK", t.pick)),
                Decision::Reject => Some(("REJECT", t.reject)),
                Decision::Deferred => Some(("DEFERRED", t.text_muted)),
                Decision::Unjudged => None,
            };
            if let Some((text, colour)) = badge {
                painter.text(fitted.left_top() + vec2(10.0, 10.0), Align2::LEFT_TOP, text, t.font(11.0), colour);
            }
            painter.text(
                area.left_bottom() + vec2(10.0, -6.0),
                Align2::LEFT_BOTTOM,
                truncate(&photo.rel, 70),
                t.font(11.0),
                t.text_muted,
            );
        }
        None => {
            ui.painter().rect_filled(area, t.corner(), t.thumb_bg);
        }
    }

    ui.add_space(6.0);
    // Up next: the four photos behind the current one, for a jump without losing the queue.
    ui.horizontal(|ui| {
        ui.label(RichText::new("Up next").size(10.5).color(t.text_faint));
        let mut jump = None;
        for id in up_next {
            if let Some(photo) = engine.photo(id).cloned() {
                let (rect, response) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::click());
                ui.painter().rect_filled(rect, t.corner(), t.thumb_bg);
                if let Some(texture) = thumbs.get(ui.ctx(), &photo, 160).cloned() {
                    let uv = cover_uv(texture.size_vec2(), rect.size());
                    ui.painter().image(texture.id(), rect, uv, Color32::WHITE);
                }
                if response.clicked() {
                    jump = Some(id);
                }
            }
        }
        if let Some(id) = jump {
            engine.cull_jump(id);
        }
        if summary.is_complete() {
            ui.label(RichText::new("Session complete").size(11.0).color(t.accent));
        }
    });

    ui.add_space(4.0);
    let mut decision_now = None;
    ui.horizontal(|ui| {
        let buttons = [
            ("Pick", Decision::Pick, t.pick, "P"),
            ("Reject", Decision::Reject, t.reject, "X"),
            ("Defer", Decision::Deferred, t.text_muted, "Space"),
            ("Clear", Decision::Unjudged, t.text_faint, "U"),
        ];
        for (label, choice, colour, key) in buttons {
            let text = RichText::new(format!("{label}  {key}")).size(11.5).color(colour);
            let button = Button::new(text)
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(t.corner());
            if ui.add(button).clicked() {
                decision_now = Some(choice);
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add_enabled(false, Button::new(RichText::new("Apply rejects…").size(11.5)))
                .on_disabled_hover_text("Apply arrives with the trash executor in M4")
                .clicked()
            {
                decision_now = None;
            }
            if ui.add(Button::new(RichText::new("Back").size(11.5))).clicked() {
                engine.cull_back();
            }
        });
    });
    if let Some(choice) = decision_now {
        engine.cull_decide(choice);
    }
    if summary.is_complete() && summary.changed() > 0 {
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "{} photos staged. Apply moves them to the bin once the trash executor lands.",
                summary.changed()
            ))
            .size(10.5)
            .color(t.text_faint),
        );
    }
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
            ui.label(RichText::new(format!("Planned for {milestone}.")).size(12.0).color(t.text_muted));
        },
    );
}

fn settings_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
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
            ui.label(RichText::new("Library").size(11.0).color(t.text_muted));
            let mut write_sidecars = engine.write_sidecars();
            if ui.checkbox(&mut write_sidecars, "Write XMP sidecars beside photos").changed() {
                engine.set_write_sidecars(write_sidecars);
                actions.push(Action::SetSidecars(write_sidecars));
            }
            ui.label(
                RichText::new("Ratings, labels, keywords and the cull decision go into a .xmp file next to each photo.")
                    .size(10.5)
                    .color(t.text_faint),
            );
            ui.add_space(8.0);
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
    if selected || response.hovered() {
        ui.painter()
            .rect_filled(rect, t.corner(), if selected { t.selection } else { t.hover });
    }
    let width = rect.width() - 12.0;
    ui.painter().text(
        rect.left_center() + vec2(6.0, 0.0),
        Align2::LEFT_CENTER,
        truncate(label, (width / 6.4) as usize),
        t.font(12.0),
        t.text,
    );
    ui.painter().text(
        rect.right_center() - vec2(6.0, 0.0),
        Align2::RIGHT_CENTER,
        count,
        t.font(11.0),
        t.text_faint,
    );
    response
}

fn detail_row(ui: &mut Ui, t: &Tokens, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(key).size(11.0).color(t.text_faint));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).size(11.5).color(t.text));
        });
    });
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

fn folder_name(path: &str) -> String {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

/// UV rectangle that fills the cell while keeping the image's aspect ratio (cover, not contain).
fn cover_uv(image: Vec2, target: Vec2) -> Rect {
    if image.x <= 0.0 || image.y <= 0.0 || target.x <= 0.0 || target.y <= 0.0 {
        return Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    }
    let image_aspect = image.x / image.y;
    let target_aspect = target.x / target.y;
    if image_aspect > target_aspect {
        let keep = target_aspect / image_aspect;
        let inset = (1.0 - keep) / 2.0;
        Rect::from_min_max(pos2(inset, 0.0), pos2(1.0 - inset, 1.0))
    } else {
        let keep = image_aspect / target_aspect;
        let inset = (1.0 - keep) / 2.0;
        Rect::from_min_max(pos2(0.0, inset), pos2(1.0, 1.0 - inset))
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    let max_chars = max_chars.max(4);
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars - 1).collect();
    out.push('…');
    out
}

fn summary(stats: &sortlens_engine::Stats) -> String {
    let mut parts = vec![format!("{} photos", format_count(stats.total as u64))];
    if stats.picks > 0 {
        parts.push(format!("{} picks", format_count(stats.picks as u64)));
    }
    if stats.rejects > 0 {
        parts.push(format!("{} rejects", format_count(stats.rejects as u64)));
    }
    if stats.missing > 0 {
        parts.push(format!("{} missing", format_count(stats.missing as u64)));
    }
    parts.join(" · ")
}

fn format_count(count: u64) -> String {
    sortlens_core::format_count(count)
}

#[allow(dead_code)]
fn cell_size(width: f32, columns: usize, gap: f32) -> Vec2 {
    vec2(((width - gap * columns.saturating_sub(1) as f32) / columns as f32).max(1.0), 0.0)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn fit_rect_keeps_the_aspect_and_stays_inside() {
        let bounds = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0));
        let portrait = fit_rect(bounds, 0.5);
        assert!((portrait.width() / portrait.height() - 0.5).abs() < 0.001);
        assert!(bounds.contains_rect(portrait));
        assert_eq!(portrait.center(), bounds.center());

        let wide = fit_rect(bounds, 2.0);
        assert!(wide.width() <= bounds.width() + 0.001);
        assert!(bounds.contains_rect(wide));
    }

    #[test]
    fn a_nonsense_aspect_falls_back_to_three_by_two() {
        let bounds = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 300.0));
        let fitted = fit_rect(bounds, 0.0);
        assert!((fitted.width() / fitted.height() - 1.5).abs() < 0.001);
    }
}
