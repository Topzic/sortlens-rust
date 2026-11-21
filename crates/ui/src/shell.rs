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
    Align, Align2, Button, Color32, FontId, Frame, Key, Layout, Margin, Rect, RichText, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2,
    pos2, vec2,
};
use sortlens_core::{Flag, MediaKind, PhotoId};
use sortlens_cull::gesture::Gesture;
use sortlens_engine::{Decision, Engine, Filter, Photo, Sort, TagOrder};

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
            Self::Insights => None,
            Self::Similar => None,
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
    /// The grid order changed and should be remembered.
    SetSort(Sort),
}

/// What Apply does with rejected photos.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DeleteMode {
    /// Send files to the operating system's trash.
    #[default]
    Trash,
    /// Leave the files and record the decision.
    TagOnly,
}

impl DeleteMode {
    /// Every mode the UI can choose. Moving files arrives with the move feature.
    pub const ALL: [Self; 2] = [Self::Trash, Self::TagOnly];

    /// Label for the radio and the confirmation.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Trash => "Move to the Trash",
            Self::TagOnly => "Tag only, move nothing",
        }
    }

    /// The engine's mode.
    pub fn mode(self) -> sortlens_cull::trash::Mode {
        match self {
            Self::Trash => sortlens_cull::trash::Mode::Trash,
            Self::TagOnly => sortlens_cull::trash::Mode::TagOnly,
        }
    }
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
    /// Keywords the view is narrowed to, chosen in the sidebar or the filter strip.
    pub tag_filters: Vec<String>,
    /// Whether several keywords must all match or just one of them.
    pub tag_mode: sortlens_engine::TagMode,
    /// How the keyword list is ordered.
    pub tag_order: TagOrder,
    /// The sort the whole library was last shown with, restored when a collection closes.
    pub library_sort: Sort,
    /// The search the whole library was last narrowed with.
    pub library_search: String,
    /// The first-run wizard was dismissed for this session.
    pub welcome_dismissed: bool,
    /// The first scan has finished and its result is being shown.
    pub welcome_ready: Option<String>,
    /// Search text inside the keyword picker.
    pub tag_picker_query: String,
    /// How close two hashes must be to count as the same subject.
    pub similar_threshold: u32,
    /// How far apart two colour fingerprints may be before they are different pictures.
    pub similar_colour: u32,
    /// The cull summary sheet is open.
    pub summary_open: bool,
    /// A finished Apply is waiting to be acknowledged.
    pub receipt_open: bool,
    /// Where a canvas drag began, while one is in flight.
    pub drag_from: Option<egui::Pos2>,
    /// Zoom and pan of the cull canvas.
    pub cull_view: sortlens_cull::viewport::View,
    /// The last decision, announced once so a screen reader hears the result.
    pub announce: Option<String>,
    /// The window was asked to close while work was in flight.
    pub quit_open: bool,
    /// Text in the keyword editor.
    pub tag_input: String,
    /// Ask the keyword field for focus on the next frame.
    pub tag_focus: bool,
    /// What Apply does with rejects.
    pub delete_mode: DeleteMode,
    /// Whether the Apply confirmation is showing.
    pub apply_open: bool,
    /// Whether the rejects bin is showing.
    pub deleted_open: bool,
    /// Whether the library health dialog is showing.
    pub health_open: bool,
    /// Last health check: (checked, present, missing).
    pub health: Option<(usize, usize, usize)>,
    /// Relative folder inside the chosen root, with a trailing slash, or empty for the whole root.
    pub folder: String,
    /// Order the grid is shown in.
    pub sort: Sort,
    /// Only photos, raws or videos.
    pub kind_filter: Option<MediaKind>,
    /// Show only edited photos, or only unedited ones.
    pub edited_filter: Option<bool>,
    /// Whether the single-photo detail view is open.
    pub detail_open: bool,
    /// Zoom the detail view to 1:1 instead of fitting the window.
    pub detail_zoom: bool,
    /// Only this collection, chosen in the sidebar.
    pub collection_filter: Option<sortlens_core::CollectionId>,
    /// Collection being renamed, with its draft name.
    pub rename_target: Option<(sortlens_core::CollectionId, String)>,
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
            tag_filters: Vec::new(),
            tag_mode: sortlens_engine::TagMode::Any,
            tag_order: TagOrder::Count,
            library_sort: Sort::Natural,
            library_search: String::new(),
            welcome_dismissed: false,
            welcome_ready: None,
            tag_picker_query: String::new(),
            similar_threshold: sortlens_engine::SIMILAR_THRESHOLD,
            similar_colour: sortlens_engine::SIMILAR_COLOUR,
            summary_open: false,
            receipt_open: false,
            drag_from: None,
            cull_view: sortlens_cull::viewport::View::default(),
            announce: None,
            quit_open: false,
            tag_input: String::new(),
            tag_focus: false,
            delete_mode: DeleteMode::Trash,
            apply_open: false,
            deleted_open: false,
            health_open: false,
            health: None,
            folder: String::new(),
            sort: Sort::Natural,
            kind_filter: None,
            edited_filter: None,
            detail_open: false,
            detail_zoom: false,
            collection_filter: None,
            rename_target: None,
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
            tags: self.tag_filters.clone(),
            tag_mode: self.tag_mode,
            collection: self.collection_filter,
            folder: (self.root.is_some() && !self.folder.is_empty()).then(|| self.folder.clone()),
            kind: self.kind_filter,
            edited: self.edited_filter,
            include_missing: self.show_missing,
            ..Filter::default()
        }
    }

    /// How many filter dimensions are narrowing the view, for the header badge.
    pub fn active_filter_count(&self) -> usize {
        let mut count = 0;
        count += usize::from(self.min_rating > 0);
        count += usize::from(self.flag_filter.is_some());
        count += usize::from(self.label_filter.is_some());
        count += usize::from(!self.tag_filters.is_empty());
        count += usize::from(self.collection_filter.is_some());
        count += usize::from(self.kind_filter.is_some());
        count += usize::from(self.edited_filter.is_some());
        count += usize::from(self.show_missing);
        count += usize::from(self.root.is_some());
        count += usize::from(!self.folder.is_empty());
        count += usize::from(!self.search.is_empty());
        count
    }

    /// Whether any filter is narrowing the view.
    pub fn has_filters(&self) -> bool {
        self.min_rating > 0
            || self.flag_filter.is_some()
            || self.label_filter.is_some()
            || !self.tag_filters.is_empty()
            || self.collection_filter.is_some()
            || !self.folder.is_empty()
            || self.kind_filter.is_some()
            || self.edited_filter.is_some()
            || self.show_missing
            || self.root.is_some()
            || !self.search.is_empty()
    }

    /// Remove every filter except the search text.
    pub fn clear_filters(&mut self) {
        self.min_rating = 0;
        self.flag_filter = None;
        self.label_filter = None;
        self.tag_filters.clear();
        if self.collection_filter.take().is_some() {
            self.sort = self.library_sort;
            self.search = self.library_search.clone();
        }
        self.folder.clear();
        self.kind_filter = None;
        self.edited_filter = None;
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

    let had_photos = engine.stats().total;
    if let Some(done) = engine.poll() {
        if had_photos == 0 && done.added > 0 {
            // The first scan found the library; say so once, then get out of the way.
            let noun = if done.added == 1 { "photo" } else { "photos" };
            state.welcome_ready = Some(format!("Indexed {} {noun} in {}", done.added, folder_name(&done.root)));
        }
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
    if engine.poll_similar_with(state.similar_threshold, state.similar_colour).is_some() {
        let groups = engine.similar_groups().len();
        state.notice(match groups {
            0 => "No similar groups found".to_string(),
            1 => "Found 1 similar group".to_string(),
            other => format!("Found {other} similar groups"),
        });
    }
    thumbs.poll(ui.ctx());
    if engine.is_scanning() || engine.hash_progress().is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(80));
    }

    top_bar(ui, state, &t, engine, &mut actions);
    bottom_bar(ui, state, &t, engine, &mut actions);
    if state.left_open {
        left_panel(ui, state, &t, engine, &mut actions);
    }
    if state.right_open {
        right_panel(ui, state, &t, engine);
    }
    canvas(ui, state, &t, engine, thumbs, &mut actions);

    // A finished cull explains itself; so does a finished Apply.
    if engine.receipt().is_some() && !state.receipt_open {
        state.receipt_open = true;
    }
    if engine.cull_complete() && !state.summary_open && !state.apply_open {
        state.summary_open = true;
    }
    if state.quit_open {
        quit_dialog(ui, state, &t, engine, &mut actions);
    }
    if state.summary_open {
        summary_dialog(ui, state, &t, engine);
    }
    if state.receipt_open {
        receipt_dialog(ui, state, &t, engine);
    }
    if state.settings_open {
        settings_dialog(ui, state, &t, engine, &mut actions);
    }
    if state.about_open {
        about_dialog(ui, state, &t, engine);
    }
    if state.apply_open {
        apply_dialog(ui, state, &t, engine);
    }
    if state.deleted_open {
        deleted_dialog(ui, state, &t, engine);
    }
    if state.health_open {
        health_dialog(ui, state, &t, engine);
    }
    if state.rename_target.is_some() {
        rename_dialog(ui, state, &t, engine);
    }

    if state.live_notice().is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
    }
    actions
}

fn top_bar(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
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
                    let hint = match state
                        .collection_filter
                        .and_then(|id| engine.collections().into_iter().find(|collection| collection.id == id))
                    {
                        Some(collection) => format!("Search in {}", collection.name),
                        None => "Filter by name or folder".to_string(),
                    };
                    let search = ui.add_sized([240.0, 24.0], TextEdit::singleline(&mut state.search).hint_text(hint));
                    if search.changed() {
                        state.selected = None;
                    }
                    // Commit the search to the collection when the box is left or Enter is pressed,
                    // rather than writing an op for every keystroke.
                    if search.lost_focus() || ui.input(|input| input.key_pressed(Key::Enter) && search.has_focus()) {
                        remember_search(state, engine);
                    }
                    if state.collection_filter.is_none() {
                        state.library_search = state.search.clone();
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

            if nav_row(ui, t, "Library health", "", false).clicked() {
                state.health_open = true;
            }
            if let Some(root) = state.root.clone() {
                let folders = engine.subfolders(&root, &state.folder);
                if !folders.is_empty() {
                    nav_section(ui, t, "Subfolders");
                }
                for (name, count) in folders {
                    let prefix = format!("{}{name}/", state.folder);
                    let selected = state.folder == prefix;
                    if nav_row(ui, t, &name, &format_count(count as u64), selected).clicked() {
                        state.folder = if selected { parent_prefix(&state.folder) } else { prefix };
                    }
                }
            }

            let deleted = engine.deleted_batches().len();
            if nav_row(ui, t, "Recently Deleted", &format_count(deleted as u64), false).clicked() {
                state.deleted_open = true;
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
                    if state.root.as_deref() != Some(root.as_str()) {
                        state.folder.clear();
                    }
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

            ui.horizontal(|ui| {
                nav_section(ui, t, "Keywords");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(Button::new(RichText::new(state.tag_order.label()).size(9.5)).frame(false))
                        .on_hover_text("Switch between most used and A–Z")
                        .clicked()
                    {
                        state.tag_order = state.tag_order.flipped();
                    }
                });
            });
            let vocabulary = engine.tag_vocabulary_in(state.tag_order);
            if vocabulary.is_empty() {
                hint(ui, t, "Select a photo, press K and type a keyword.");
            }
            for (tag, count) in vocabulary.iter().take(12) {
                let selected = state.tag_filters.iter().any(|active| active == tag);
                if nav_row(ui, t, tag, &format_count(*count as u64), selected).clicked() {
                    if selected {
                        state.tag_filters.retain(|active| active != tag);
                    } else {
                        state.tag_filters.push(tag.clone());
                    }
                }
            }

            collections_section(ui, t, state, engine);
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
                crate::a11y::live_line(
                    ui,
                    &crate::a11y::library_status(total, usize::from(state.selected.is_some())),
                    t.text_muted,
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if state.root.is_some() && icon_button(ui, "All Photos", t).clicked() {
                        state.root = None;
                        state.folder.clear();
                    }
                    let active = state.active_filter_count();
                    if active > 0
                        && chip(ui, t, &format!("{active} active"), true)
                            .on_hover_text("Clear every filter")
                            .clicked()
                    {
                        state.search.clear();
                        state.clear_filters();
                    }
                    let mut chosen = state.sort;
                    egui::ComboBox::from_id_salt("sort_order")
                        .selected_text(RichText::new(state.sort.label()).size(11.5).color(t.text_muted))
                        .width(130.0)
                        .show_ui(ui, |ui| {
                            for sort in Sort::ALL {
                                ui.selectable_value(&mut chosen, sort, RichText::new(sort.label()).size(11.5));
                            }
                        });
                    if chosen != state.sort {
                        state.sort = chosen;
                        actions.push(Action::SetSort(chosen));
                        match state.collection_filter {
                            Some(id) => {
                                if let Err(error) = engine.set_collection_sort(id, chosen) {
                                    state.notice(error.to_string());
                                }
                            }
                            None => state.library_sort = chosen,
                        }
                    }
                });
            });
            if state.root.is_some() {
                breadcrumbs(ui, t, state);
            }
            ui.add_space(8.0);
            filter_strip(ui, t, state, engine, total);
            ui.add_space(10.0);

            if state.view == View::Library
                && let Some(message) = state.welcome_ready.clone()
            {
                Frame::NONE
                    .fill(t.card)
                    .stroke(Stroke::new(1.0, t.border))
                    .corner_radius(t.corner())
                    .inner_margin(Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(message).size(12.0).strong());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.small_button("Dismiss").clicked() {
                                    state.welcome_ready = None;
                                }
                                if ui.button(RichText::new("Start culling").size(11.5)).clicked() {
                                    match start_cull(engine, state) {
                                        Ok(total) => {
                                            state.view = View::Cull;
                                            state.welcome_ready = None;
                                            state.notice(format!("Culling {total} photos"));
                                        }
                                        Err(error) => state.notice(error),
                                    }
                                }
                            });
                        });
                    });
                ui.add_space(8.0);
            }

            match state.view {
                View::Library if state.detail_open => {
                    detail_view(ui, t, state, engine, thumbs);
                }
                View::Library => {
                    if total == 0 {
                        empty_library(ui, t, state, engine, actions);
                    } else {
                        grid(ui, t, state, engine, thumbs, &filter);
                    }
                }
                View::Cull => cull_view(ui, t, state, engine, thumbs),
                View::Insights => insights_view(ui, t, engine),
                View::Similar => similar_view(ui, t, state, engine, thumbs),
                other => planned_view(ui, t, other),
            }
            keyboard(ui, state, engine);
        });
}

fn grid(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails, filter: &Filter) {
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
    let mut add_to: Option<(sortlens_core::PhotoId, sortlens_core::CollectionId)> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, rows, |ui, range| {
            ui.spacing_mut().item_spacing = vec2(gap, 0.0);
            let offset = range.start * columns;
            let limit = (range.end - range.start) * columns;
            let page = engine.page(filter, state.sort, offset, limit);
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
                                    state.detail_open = true;
                                }
                                response.context_menu(|ui| {
                                    ui.menu_button("Add to Collection", |ui| {
                                        let collections = engine.collections();
                                        if collections.is_empty() {
                                            ui.label(RichText::new("No collections yet").size(10.5));
                                        }
                                        for collection in collections {
                                            if ui.button(&collection.name).clicked() {
                                                add_to = Some((photo.id, collection.id));
                                                ui.close();
                                            }
                                        }
                                    });
                                });
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

    if let Some((photo, collection)) = add_to {
        match engine.add_to_collection(&[photo], collection) {
            Ok(_) => state.notice("Added to collection"),
            Err(error) => state.notice(error.to_string()),
        }
    }
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
    if state.detail_open {
        // Arrows, Space, Z, D and Esc belong to the detail view; rating and flag keys still work
        // below, which is why this does not return.
        detail_keyboard(ui, state, engine);
    }
    if cull_keyboard(ui, state, engine) {
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

    if !state.detail_open && ui.input(|input| input.key_pressed(Key::D) && !input.modifiers.command) {
        state.detail_open = true;
        return;
    }

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

fn filter_strip(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &Engine, total: usize) {
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
                if let Some(id) = state.collection_filter
                    && let Some(collection) = engine.collections().into_iter().find(|entry| entry.id == id)
                    && chip(ui, t, &collection.name, true).clicked()
                {
                    state.collection_filter = None;
                }
                let mut drop_tag = None;
                for tag in state.tag_filters.clone() {
                    if chip(ui, t, &format!("#{tag}"), true).clicked() {
                        drop_tag = Some(tag);
                    }
                }
                if let Some(tag) = drop_tag {
                    state.tag_filters.retain(|active| *active != tag);
                }
                ui.menu_button(RichText::new("+ keyword").size(10.5), |ui| {
                    ui.add(
                        TextEdit::singleline(&mut state.tag_picker_query)
                            .hint_text("Find a keyword")
                            .desired_width(180.0),
                    );
                    ui.separator();
                    let needle = state.tag_picker_query.trim().to_lowercase();
                    let vocabulary: Vec<(String, usize)> = engine
                        .tag_vocabulary_in(state.tag_order)
                        .into_iter()
                        .filter(|(tag, _)| needle.is_empty() || tag.contains(&needle))
                        .collect();
                    if vocabulary.is_empty() {
                        ui.label(RichText::new("No keywords").size(10.5));
                    }
                    for (tag, count) in vocabulary.iter().take(24) {
                        let active = state.tag_filters.iter().any(|chosen| chosen == tag);
                        let label = format!("{}{tag}  ({count})", if active { "✓ " } else { "" });
                        if ui.add(Button::new(RichText::new(label).size(11.0)).frame(false)).clicked() {
                            if active {
                                state.tag_filters.retain(|chosen| chosen != tag);
                            } else {
                                state.tag_filters.push(tag.clone());
                            }
                        }
                    }
                });
                if state.tag_filters.len() > 1 {
                    let label = format!("match {}", state.tag_mode.label());
                    if chip(ui, t, &label, state.tag_mode == sortlens_engine::TagMode::All).clicked() {
                        state.tag_mode = state.tag_mode.flipped();
                    }
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

                let kind_label = match state.kind_filter {
                    None => "All kinds".to_string(),
                    Some(MediaKind::Image) => "Photos".to_string(),
                    Some(MediaKind::Raw) => "RAW".to_string(),
                    Some(MediaKind::Video) => "Videos".to_string(),
                };
                if chip(ui, t, &kind_label, state.kind_filter.is_some()).clicked() {
                    state.kind_filter = match state.kind_filter {
                        None => Some(MediaKind::Image),
                        Some(MediaKind::Image) => Some(MediaKind::Raw),
                        Some(MediaKind::Raw) => Some(MediaKind::Video),
                        Some(MediaKind::Video) => None,
                    };
                }

                let edited_label = match state.edited_filter {
                    None => "Edited".to_string(),
                    Some(true) => "Edited only".to_string(),
                    Some(false) => "Unedited".to_string(),
                };
                if chip(ui, t, &edited_label, state.edited_filter.is_some()).clicked() {
                    state.edited_filter = match state.edited_filter {
                        None => Some(true),
                        Some(true) => Some(false),
                        Some(false) => None,
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

/// Which step of the first-run wizard the library is on, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Welcome {
    /// No roots yet: the only useful action is adding a folder.
    Add,
    /// A scan is running: show what has been found so far.
    Indexing,
    /// The first scan finished and found something.
    Ready,
}

/// Decide the step from what the library is doing.
///
/// The wizard is derived every frame rather than remembered, so it cannot get stuck: once photos
/// exist and nothing is scanning it is gone, whether or not anybody clicked anything.
pub fn welcome_stage(roots: usize, scanning: bool, total: usize, dismissed: bool) -> Option<Welcome> {
    if dismissed {
        return None;
    }
    if scanning {
        return Some(Welcome::Indexing);
    }
    if roots == 0 || total == 0 {
        return Some(Welcome::Add);
    }
    None
}

fn empty_library(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, actions: &mut Vec<Action>) {
    let stage = welcome_stage(
        engine.roots().len(),
        engine.is_scanning(),
        engine.stats().total,
        state.welcome_dismissed,
    )
    .unwrap_or(Welcome::Add);
    let height = ui.available_height();
    ui.allocate_ui_with_layout(vec2(ui.available_width(), height), Layout::top_down(Align::Center), |ui| {
        ui.add_space((height * 0.14).clamp(24.0, 140.0));
        match stage {
            Welcome::Add => welcome_add(ui, t, state, actions),
            Welcome::Indexing => welcome_indexing(ui, t, engine),
            Welcome::Ready => {}
        }
    });
}

/// Step one: what Sortlens does, and the one button worth pressing.
fn welcome_add(ui: &mut Ui, t: &Tokens, state: &mut UiState, actions: &mut Vec<Action>) {
    ui.label(RichText::new("Welcome to Sortlens").size(22.0).strong());
    ui.add_space(6.0);
    ui.label(
        RichText::new("Point it at a folder of photos and it builds a catalog beside your files.")
            .size(12.5)
            .color(t.text_muted),
    );
    ui.add_space(16.0);
    ui.allocate_ui(vec2(420.0, 0.0), |ui| {
        for (title, detail) in [
            ("Your files stay put", "No upload, no rename, no moving. Sortlens only reads."),
            (
                "Everything is undoable",
                "Ratings, flags and keywords live in one log you can step back.",
            ),
            (
                "Rejects wait for you",
                "Culling stages decisions; nothing is trashed until you apply.",
            ),
        ] {
            ui.horizontal(|ui| {
                ui.label(RichText::new("·").size(12.0).color(t.accent));
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).size(12.0).strong());
                    ui.label(RichText::new(detail).size(10.5).color(t.text_faint));
                });
            });
            ui.add_space(6.0);
        }
    });
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui
            .add(Button::new(RichText::new("Add Folder…").size(12.5)).min_size(vec2(130.0, 30.0)))
            .clicked()
        {
            actions.push(Action::PickFolder);
        }
        if ui
            .add(Button::new(RichText::new("Not now").size(12.0)).fill(Color32::TRANSPARENT))
            .clicked()
        {
            state.welcome_dismissed = true;
        }
    });
    ui.add_space(10.0);
    ui.label(
        RichText::new("JPEG, HEIC, TIFF, PNG, WebP, GIF and camera raw · video poster frames arrive later")
            .size(10.5)
            .color(t.text_faint),
    );
}

/// Step two: what has been found so far, including the first thumbnails.
fn welcome_indexing(ui: &mut Ui, t: &Tokens, engine: &Engine) {
    ui.label(RichText::new("Indexing…").size(20.0).strong());
    ui.add_space(4.0);
    let (files, dirs) = engine
        .scan_progress()
        .map(|progress| (progress.files, progress.dirs))
        .unwrap_or((0, 0));
    ui.label(
        RichText::new(format!(
            "{} files in {} folders so far. Reading sizes and content keys; nothing is moved.",
            format_count(files as u64),
            format_count(dirs as u64)
        ))
        .size(12.0)
        .color(t.text_muted),
    );
    ui.add_space(10.0);
    ui.add(
        egui::ProgressBar::new(if dirs == 0 { 0.05 } else { 0.35 })
            .desired_width(320.0)
            .text("scanning"),
    );
    ui.add_space(14.0);
    ui.label(RichText::new("Found so far").size(11.0).color(t.text_faint));
    ui.horizontal(|ui| {
        let recent = engine.page(&Filter::default(), Sort::Newest, 0, 8).photos;
        if recent.is_empty() {
            ui.label(RichText::new("Nothing yet").size(11.0).color(t.text_faint));
        }
        for photo in recent {
            let (rect, _) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::hover());
            ui.painter().rect_filled(rect, t.corner(), t.thumb_bg);
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                truncate(&photo.name, 10),
                FontId::proportional(9.0),
                t.text_faint,
            );
        }
    });
}

fn detail_keyboard(ui: &mut Ui, state: &mut UiState, engine: &mut Engine) {
    let mut close = false;
    let mut zoom = false;
    let mut step = 0isize;
    ui.input(|input| {
        if input.key_pressed(Key::Escape) || (input.key_pressed(Key::D) && !input.modifiers.command) {
            close = true;
        }
        if input.key_pressed(Key::Z) && !input.modifiers.command {
            zoom = true;
        }
        if input.key_pressed(Key::ArrowRight) || input.key_pressed(Key::Space) {
            step = 1;
        }
        if input.key_pressed(Key::ArrowLeft) {
            step = -1;
        }
    });
    if zoom {
        state.detail_zoom = !state.detail_zoom;
    }
    if close {
        state.detail_open = false;
    }
    if step != 0
        && let Some(id) = state.selected
        && let Some(next) = engine.neighbour(&state.filter(), state.sort, id, step)
    {
        state.selected = Some(next);
    }
}

/// One photo, large, with the facts a cull needs and arrows to walk the view.
fn detail_view(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails) {
    let filter = state.filter();
    let Some(id) = state.selected else {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(RichText::new("Nothing selected").size(16.0).strong());
            ui.label(
                RichText::new("Pick a photo in the grid, then press D.")
                    .size(11.5)
                    .color(t.text_muted),
            );
            if ui.button("Back to the grid").clicked() {
                state.detail_open = false;
            }
        });
        return;
    };
    let Some(photo) = engine.photo(id).cloned() else {
        state.detail_open = false;
        return;
    };
    let position = engine.position(&filter, state.sort, id);

    ui.horizontal(|ui| {
        ui.label(RichText::new(truncate(&photo.name, 42)).size(14.0).strong());
        if let Some((index, total)) = position {
            ui.label(RichText::new(format!("{index} of {total}")).size(11.5).color(t.text_muted));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button(RichText::new("Close  Esc").size(11.0)).clicked() {
                state.detail_open = false;
            }
            let zoom_label = if state.detail_zoom { "Fit  Z" } else { "1:1  Z" };
            if ui.button(RichText::new(zoom_label).size(11.0)).clicked() {
                state.detail_zoom = !state.detail_zoom;
            }
        });
    });
    ui.add_space(6.0);

    let footer = 118.0;
    let height = (ui.available_height() - footer).max(180.0);
    let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    ui.painter().rect_filled(area, t.corner(), t.thumb_bg);

    let aspect = if photo.width > 0 && photo.height > 0 {
        photo.width as f32 / photo.height as f32
    } else {
        1.5
    };
    let fitted = fit_rect(area.shrink(6.0), aspect);
    let target = if state.detail_zoom {
        // 1:1 shows the image at its pixel size, cropped by the available area.
        let width = photo.width.max(1) as f32;
        Rect::from_center_size(area.center(), vec2(width, width / aspect))
    } else {
        fitted
    };
    match thumbs.get(ui.ctx(), &photo, 2000).cloned() {
        Some(texture) => {
            let uv = cover_uv(texture.size_vec2(), target.size());
            let painter = ui.painter().with_clip_rect(area.intersect(ui.clip_rect()));
            painter.image(texture.id(), target, uv, Color32::WHITE);
        }
        None => {
            ui.painter()
                .text(area.center(), Align2::CENTER_CENTER, "Loading preview…", t.font(11.0), t.text_faint);
        }
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.add(Button::new(RichText::new("← Previous").size(11.0))).clicked()
            && let Some(previous) = engine.neighbour(&filter, state.sort, id, -1)
        {
            state.selected = Some(previous);
        }
        if ui.add(Button::new(RichText::new("Next →").size(11.0))).clicked()
            && let Some(next) = engine.neighbour(&filter, state.sort, id, 1)
        {
            state.selected = Some(next);
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!(
                "{}  ·  {}  ·  {}",
                photo.camera,
                photo.exposure(),
                sortlens_core::format_bytes(photo.size)
            ))
            .size(11.0)
            .color(t.text_muted),
        );
    });
    if !photo.tags.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for tag in &photo.tags {
                ui.label(RichText::new(format!("#{tag}")).size(10.5).color(t.text_faint));
            }
        });
    }

    ui.horizontal(|ui| {
        if let Some(histogram) = thumbs.histogram(&photo) {
            histogram_widget(ui, t, histogram);
        } else {
            ui.label(RichText::new("No preview to measure").size(10.0).color(t.text_faint));
        }
        ui.add_space(10.0);
        let mut jump = None;
        ui.vertical(|ui| {
            ui.label(RichText::new("Lookahead").size(10.0).color(t.text_faint));
            ui.horizontal(|ui| {
                for delta in [-2isize, -1, 1, 2] {
                    let Some(neighbour) = engine.neighbour(&filter, state.sort, id, delta) else {
                        continue;
                    };
                    let Some(other) = engine.photo(neighbour).cloned() else {
                        continue;
                    };
                    let (rect, response) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
                    ui.painter().rect_filled(rect, t.corner(), t.thumb_bg);
                    if let Some(texture) = thumbs.get(ui.ctx(), &other, 128).cloned() {
                        let uv = cover_uv(texture.size_vec2(), rect.size());
                        ui.painter().image(texture.id(), rect, uv, Color32::WHITE);
                    }
                    if response.clicked() {
                        jump = Some(neighbour);
                    }
                }
            });
        });
        if let Some(target) = jump {
            state.selected = Some(target);
        }
    });

    ui.label(
        RichText::new("P/X/U flag · 1–5 rate · ← → walk · Z zoom · Esc close")
            .size(10.0)
            .color(t.text_faint),
    );
}

/// A 64-bin luminance histogram, drawn as bars scaled to the tallest bin.
fn histogram_widget(ui: &mut Ui, t: &Tokens, histogram: &sortlens_thumbs::Histogram) {
    let size = vec2(190.0, 44.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, t.corner(), t.thumb_bg);
    let peak = histogram.peak().max(1) as f32;
    let width = rect.width() / histogram.bins.len() as f32;
    for (index, count) in histogram.bins.iter().enumerate() {
        let height = (*count as f32 / peak) * (rect.height() - 2.0);
        let bar = Rect::from_min_size(
            pos2(rect.left() + index as f32 * width, rect.bottom() - height),
            vec2(width.max(1.0) - 0.5, height),
        );
        painter.rect_filled(bar, 0.0, t.text_muted);
    }
}

/// Start a session over what the filters currently show. A session is a fixed queue: changing the
/// filters afterwards does not silently change what is being culled.
/// The parent of a relative folder prefix ("day1/sub/" → "day1/", "day1/" → "").
fn parent_prefix(folder: &str) -> String {
    let trimmed = folder.trim_end_matches('/');
    match trimmed.rsplit_once('/') {
        Some((parent, _)) => format!("{parent}/"),
        None => String::new(),
    }
}

/// Root › day1 › sub, each segment clickable.
fn breadcrumbs(ui: &mut Ui, t: &Tokens, state: &mut UiState) {
    let Some(root) = state.root.clone() else {
        return;
    };
    let mut target: Option<String> = None;
    ui.horizontal(|ui| {
        let at_root = state.folder.is_empty();
        if ui
            .add(
                Button::new(
                    RichText::new(folder_name(&root))
                        .size(11.0)
                        .color(if at_root { t.text } else { t.accent }),
                )
                .frame(false),
            )
            .clicked()
        {
            target = Some(String::new());
        }
        let mut prefix = String::new();
        for segment in state.folder.trim_end_matches('/').split('/').filter(|part| !part.is_empty()) {
            prefix.push_str(segment);
            prefix.push('/');
            ui.label(RichText::new("›").size(11.0).color(t.text_faint));
            let here = prefix.clone();
            let at_here = state.folder == here;
            if ui
                .add(Button::new(RichText::new(segment).size(11.0).color(if at_here { t.text } else { t.accent })).frame(false))
                .clicked()
            {
                target = Some(here);
            }
        }
        if !at_root {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.add(Button::new(RichText::new("Up one level").size(10.5)).frame(false)).clicked() {
                    target = Some(parent_prefix(&state.folder));
                }
            });
        }
    });
    if let Some(folder) = target {
        state.folder = folder;
    }
}

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
        .page(&state.filter(), state.sort, 0, QUEUE_CAP)
        .photos
        .iter()
        .map(|photo| photo.id)
        .collect();
    engine.begin_cull(&ids).map_err(|error| error.to_string())
}

fn cull_keyboard(ui: &mut Ui, state: &mut UiState, engine: &mut Engine) -> bool {
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
        let left = engine.cull().map(|session| session.summary().unjudged).unwrap_or_default();
        state.announce = Some(crate::a11y::decision_sentence(decision, left));
    } else if back {
        engine.cull_back();
        state.announce = Some("Back one photo".to_string());
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
                RichText::new("P pick · X reject · Space defer · U clear · ← back · drag up/down")
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
            if ui.add(Button::new(RichText::new("Summary").size(11.0))).clicked() {
                state.summary_open = true;
            }
            if ui
                .add(Button::new(RichText::new("End session").size(11.0)))
                .on_hover_text("Discard staged decisions; nothing has been written yet")
                .clicked()
            {
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
    let (area, area_response) = ui.allocate_exact_size(vec2(ui.available_width(), image_height), Sense::drag());

    let fitted_size = fitted_size(area, current.and_then(|id| engine.photo(id)));
    let (gesture, drag_progress) = canvas_interaction(state, engine, ui, area, &area_response, fitted_size, current);

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
            let zoomed = state.cull_view.is_zoomed();
            let shown = if zoomed {
                let wanted = fitted.size() * state.cull_view.zoom;
                Rect::from_center_size(fitted.center() + vec2(state.cull_view.offset.0, state.cull_view.offset.1), wanted)
            } else {
                fitted
            };
            let painter = painter.with_clip_rect(if zoomed { area } else { painter.clip_rect() });
            match thumbs.get(ui.ctx(), &photo, if zoomed { 2000 } else { 1600 }).cloned() {
                Some(texture) => {
                    let uv = cover_uv(texture.size_vec2(), shown.size());
                    painter.image(texture.id(), shown, uv, Color32::WHITE);
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
            if state.cull_view.is_zoomed() {
                painter.text(
                    area.right_top() + vec2(-10.0, 10.0),
                    Align2::RIGHT_TOP,
                    format!("{:.1}× · Z to fit", state.cull_view.zoom),
                    t.font(10.5),
                    t.text_muted,
                );
            }
            painter.text(
                area.left_bottom() + vec2(10.0, -6.0),
                Align2::LEFT_BOTTOM,
                truncate(&photo.rel, 70),
                t.font(11.0),
                t.text_muted,
            );
            if gesture != Gesture::None {
                let tint = match gesture {
                    Gesture::Pick => t.pick,
                    _ => t.reject,
                };
                painter.rect_filled(
                    area,
                    t.corner(),
                    Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), (drag_progress * 70.0) as u8),
                );
                painter.text(
                    area.center(),
                    Align2::CENTER_CENTER,
                    gesture.label(),
                    t.font(26.0),
                    Color32::from_rgba_unmultiplied(tint.r(), tint.g(), tint.b(), (drag_progress * 255.0) as u8),
                );
            } else if drag_progress > 0.05 {
                painter.text(
                    area.center(),
                    Align2::CENTER_CENTER,
                    "drag further or sideways to cancel",
                    t.font(11.0),
                    t.text_faint,
                );
            }
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
                .add_enabled(summary.rejects > 0, Button::new(RichText::new("Apply rejects…").size(11.5)))
                .on_disabled_hover_text("Reject a photo first; picks and defers need no apply")
                .clicked()
            {
                state.apply_open = true;
            }
            if ui.add(Button::new(RichText::new("Back").size(11.5))).clicked() {
                engine.cull_back();
            }
        });
    });
    // Spoken feedback: the queue position and what the photo now carries. Assistive technology
    // reads this politely, so it never interrupts anything else being read.
    let position = if total == 0 { 0 } else { (cursor + 1).min(total) };
    let status_colour = match decision {
        Decision::Pick => t.pick,
        Decision::Reject => t.reject,
        _ => t.text_muted,
    };
    crate::a11y::live_line(ui, &crate::a11y::cull_status(position, total, decision), status_colour);

    if let Some(choice) = decision_now {
        engine.cull_decide(choice);
        let left = engine.cull().map(|session| session.summary().unjudged).unwrap_or_default();
        state.announce = Some(crate::a11y::decision_sentence(choice, left));
    }
    if let Some(said) = state.announce.clone() {
        crate::a11y::live_line(ui, &said, t.text_muted);
    }
    if summary.changed() > 0 {
        ui.add_space(4.0);

        ui.label(
            RichText::new(format!(
                "{} photos staged. {} Apply to record the decisions — you can undo it as one step.",
                summary.changed(),
                state.delete_mode.label().to_lowercase()
            ))
            .size(10.5)
            .color(t.text_faint),
        );
    }
}

/// Near-duplicate groups, hashed on demand and cached by content key.
fn similar_view(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails) {
    let progress = engine.hash_progress();
    let threshold = state.similar_threshold;
    let groups = engine.similar_groups().len();
    ui.horizontal(|ui| {
        let label = if progress.is_some() { "Hashing…" } else { "Find similar photos" };
        if ui
            .add_enabled(progress.is_none(), Button::new(RichText::new(label).size(11.5)))
            .clicked()
        {
            match engine.begin_similar(threshold) {
                Ok(0) => state.notice("Groups up to date".to_string()),
                Ok(queued) => state.notice(format!("Hashing {queued} previews")),
                Err(error) => state.notice(error.to_string()),
            }
        }
        if let Some((done, total)) = progress {
            ui.add(egui::ProgressBar::new(if total == 0 { 0.0 } else { done as f32 / total as f32 }).desired_width(180.0));
            ui.label(RichText::new(format!("{done} of {total}")).size(10.5).color(t.text_muted));
            if ui.small_button("Cancel").clicked() {
                engine.cancel_similar();
            }
        } else {
            ui.label(
                RichText::new(format!("{groups} groups · {} identical bits", threshold))
                    .size(10.5)
                    .color(t.text_faint),
            );
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let value = state.similar_threshold as u64;
            let mut slider = value;
            if ui.add(egui::Slider::new(&mut slider, 0..=20).text("strictness")).changed() {
                state.similar_threshold = slider as u32;
            }
        });
    });
    ui.add_space(8.0);

    let found: Vec<sortlens_engine::SimilarGroup> = engine.similar_groups().to_vec();
    if found.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("No groups yet").size(15.0).strong());
            ui.label(
                RichText::new("Hash the library to find bursts and re-edits.")
                    .size(11.0)
                    .color(t.text_faint),
            );
        });
    } else {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for group in &found {
                Frame::NONE
                    .fill(t.card)
                    .stroke(Stroke::new(1.0, t.border))
                    .corner_radius(t.corner())
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{} photos", group.len())).size(11.5).strong());
                            ui.label(
                                RichText::new(format!("spread {} bits", group.spread))
                                    .size(10.0)
                                    .color(t.text_faint),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui.small_button("Select all but the best").clicked() {
                                    state.selected = group.others().into_iter().next();
                                    state.notice(format!("{} set aside", group.len() - 1));
                                }
                            });
                        });
                        ui.horizontal(|ui| {
                            for member in group.members.iter().take(8) {
                                let Some(photo) = engine.photo(member.id).cloned() else {
                                    continue;
                                };
                                let (rect, response) = ui.allocate_exact_size(vec2(64.0, 64.0), Sense::click());
                                ui.painter().rect_filled(rect, t.corner(), t.thumb_bg);
                                if let Some(texture) = thumbs.get(ui.ctx(), &photo, 128).cloned() {
                                    let uv = cover_uv(texture.size_vec2(), rect.size());
                                    ui.painter().image(texture.id(), rect, uv, Color32::WHITE);
                                }
                                if member.id == group.best {
                                    ui.painter().text(
                                        rect.left_top() + vec2(2.0, 2.0),
                                        Align2::LEFT_TOP,
                                        "best",
                                        FontId::proportional(9.5),
                                        t.text,
                                    );
                                }
                                if response.clicked() {
                                    state.selected = Some(member.id);
                                }
                            }
                        });
                    });
                ui.add_space(6.0);
            }
        });
    }
}

/// Totals, distributions and breakdowns, all recomputed from the catalog.
fn insights_view(ui: &mut Ui, t: &Tokens, engine: &mut Engine) {
    let insights = engine.insights();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            figure(ui, t, "Photos", &format_count(insights.stats.total as u64));
            figure(ui, t, "On disk", &sortlens_core::format_bytes(insights.stats.bytes));
            figure(ui, t, "Rated", &format!("{:.0}%", insights.rated_share() * 100.0));
            figure(ui, t, "Keyworded", &format!("{:.0}%", insights.tagged_share() * 100.0));
            figure(ui, t, "Picks", &format_count(insights.stats.picks as u64));
            figure(ui, t, "Rejects", &format_count(insights.stats.rejects as u64));
            figure(ui, t, "Missing", &format_count(insights.stats.missing as u64));
        });
        ui.add_space(14.0);

        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(300.0);
                ui.label(RichText::new("Ratings").size(12.0).strong());
                for stars in (0..=5).rev() {
                    let count = insights.by_rating[stars];
                    bar_row(
                        ui,
                        t,
                        &if stars == 0 { "Unrated".to_string() } else { "★".repeat(stars) },
                        count,
                        insights.stats.total,
                    );
                }
                ui.add_space(12.0);
                ui.label(RichText::new("Labels").size(12.0).strong());
                for (index, label) in sortlens_core::ColorLabel::ALL.iter().enumerate() {
                    bar_row(ui, t, label.label(), insights.by_label[index], insights.stats.total);
                }
                ui.add_space(12.0);
                ui.label(RichText::new("Kinds").size(12.0).strong());
                for (index, name) in ["Images", "Raw", "Video"].iter().enumerate() {
                    bar_row(ui, t, name, insights.by_kind[index], insights.stats.total);
                }
            });
            ui.add_space(18.0);
            ui.vertical(|ui| {
                breakdown(ui, t, "Cameras", &insights.cameras, insights.unknown_camera);
                ui.add_space(12.0);
                breakdown(ui, t, "Lenses", &insights.lenses, 0);
                ui.add_space(12.0);
                breakdown(ui, t, "Keywords", &insights.keywords, 0);
            });
            ui.add_space(18.0);
            ui.vertical(|ui| {
                ui.set_width(240.0);
                ui.label(RichText::new("Timeline").size(12.0).strong());
                if insights.years.is_empty() {
                    ui.label(RichText::new("No capture dates yet").size(11.0).color(t.text_faint));
                }
                let peak = insights.peak_year().max(1);
                for year in &insights.years {
                    bar_row(ui, t, &year.year.to_string(), year.count, peak);
                }
                if insights.undated > 0 {
                    ui.label(
                        RichText::new(format!("{} with no capture date", insights.undated))
                            .size(10.5)
                            .color(t.text_faint),
                    );
                }
            });
        });
    });
}

/// One headline number with a caption.
fn figure(ui: &mut Ui, t: &Tokens, caption: &str, value: &str) {
    ui.allocate_ui(vec2(112.0, 46.0), |ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(value).size(18.0).strong());
            ui.label(RichText::new(caption).size(10.0).color(t.text_faint));
        });
    });
}

/// A label, a proportional bar and a count.
fn bar_row(ui: &mut Ui, t: &Tokens, label: &str, count: usize, whole: usize) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width().min(300.0), 15.0), Sense::hover());
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
    let share = if whole == 0 { 0.0 } else { count as f32 / whole as f32 };
    if count > 0 {
        let filled = Rect::from_min_size(track.min, vec2(track.width() * share.clamp(0.02, 1.0), track.height()));
        painter.rect_filled(filled, 2.0, t.selection);
    }
    painter.text(
        pos2(rect.right(), rect.center().y),
        Align2::RIGHT_CENTER,
        format_count(count as u64),
        FontId::proportional(10.5),
        t.text_faint,
    );
}

/// A titled list of the most common values, scaled to the largest.
fn breakdown(ui: &mut Ui, t: &Tokens, title: &str, rows: &[sortlens_engine::InsightRow], extra: usize) {
    ui.label(RichText::new(title).size(12.0).strong());
    if rows.is_empty() {
        ui.label(RichText::new("Nothing recorded yet").size(11.0).color(t.text_faint));
        return;
    }
    let peak = sortlens_engine::Insights::peak(rows).max(1);
    for row in rows {
        bar_row(ui, t, &row.name, row.count, peak);
    }
    if extra > 0 {
        ui.label(
            RichText::new(format!("{extra} with no {}", title.to_lowercase().trim_end_matches('s')))
                .size(10.5)
                .color(t.text_faint),
        );
    }
}

/// Store the open collection's current search, so reopening it resumes there.
fn remember_search(state: &mut UiState, engine: &mut Engine) {
    let Some(id) = state.collection_filter else {
        return;
    };
    let stored = engine
        .collections()
        .into_iter()
        .find(|collection| collection.id == id)
        .and_then(|collection| collection.search);
    if stored.as_deref() == Some(state.search.trim()).filter(|text| !text.is_empty())
        || (stored.is_none() && state.search.trim().is_empty())
    {
        return;
    }
    if let Err(error) = engine.set_collection_search(id, &state.search) {
        state.notice(error.to_string());
    }
}

/// Collections: favourites first, each with its colour, and the actions that change both.
fn collections_section(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine) {
    nav_section(ui, t, "Collections");
    let collections = engine.collections();
    if collections.is_empty() {
        hint(ui, t, "Collect photos into named sets without moving them.");
    }
    let mut delete = None;
    let mut rename = None;
    let mut recolour: Option<(sortlens_core::CollectionId, Option<sortlens_core::ColorLabel>)> = None;
    let mut pin: Option<(sortlens_core::CollectionId, bool)> = None;
    for collection in &collections {
        let selected = state.collection_filter == Some(collection.id);
        let name = format!("{}{}", if collection.favourite { "★ " } else { "" }, collection.name);
        let row = nav_row(ui, t, &name, &format_count(collection.len() as u64), selected);
        if let Some(colour) = collection.colour {
            // A dot before the name, so a colour is scannable without opening anything.
            let dot = pos2(row.rect.left() + 3.0, row.rect.center().y);
            ui.painter().circle_filled(dot, 3.0, t.label(colour));
        }
        if row.clicked() {
            if selected {
                state.collection_filter = None;
            } else {
                state.collection_filter = Some(collection.id);
                // Reopening a collection puts the sort back to how it was last browsed.
                state.sort = collection.sort.as_deref().map(Sort::parse).unwrap_or(Sort::Natural);
            }
        }
        row.context_menu(|ui| {
            let pin_label = if collection.favourite {
                "Remove from favourites"
            } else {
                "Add to favourites"
            };
            if ui.button(pin_label).clicked() {
                pin = Some((collection.id, !collection.favourite));
                ui.close();
            }
            ui.menu_button("Colour", |ui| {
                if ui.button("None").clicked() {
                    recolour = Some((collection.id, None));
                    ui.close();
                }
                for label in sortlens_core::ColorLabel::ALL {
                    if ui.button(label.label()).clicked() {
                        recolour = Some((collection.id, Some(label)));
                        ui.close();
                    }
                }
            });
            if ui.button("Rename…").clicked() {
                rename = Some((collection.id, collection.name.clone()));
                ui.close();
            }
            if ui.button("Delete collection").clicked() {
                delete = Some(collection.id);
                ui.close();
            }
        });
    }
    if let Some((id, colour)) = recolour {
        match engine.set_collection_colour(id, colour) {
            Ok(()) => state.notice(match colour {
                Some(label) => format!("Collection is {}", label.label().to_lowercase()),
                None => "Collection colour cleared".to_string(),
            }),
            Err(error) => state.notice(error.to_string()),
        }
    }
    if let Some((id, favourite)) = pin {
        match engine.set_collection_favourite(id, favourite) {
            Ok(()) => state.notice(if favourite {
                "Pinned to the top".to_string()
            } else {
                "Unpinned".to_string()
            }),
            Err(error) => state.notice(error.to_string()),
        }
    }
    if let Some((id, name)) = rename {
        state.rename_target = Some((id, name));
    }
    if let Some(id) = delete {
        match engine.delete_collection(id) {
            Ok(()) => {
                if state.collection_filter == Some(id) {
                    state.collection_filter = None;
                }
                state.notice("Collection deleted — the photos stay in the library");
            }
            Err(error) => state.notice(error.to_string()),
        }
    }
    if ui
        .add(Button::new(RichText::new("+ New collection").size(10.5)).fill(Color32::TRANSPARENT))
        .clicked()
    {
        let name = format!("Collection {}", collections.len() + 1);
        match engine.create_collection(&name) {
            Ok(id) => {
                state.collection_filter = Some(id);
                state.rename_target = Some((id, name));
            }
            Err(error) => state.notice(error.to_string()),
        }
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

/// Human-friendly age for a batch.
fn age_label(applied_at: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or(applied_at);
    let seconds = (now - applied_at).max(0);
    match seconds {
        0..=59 => "just now".to_string(),
        60..=3599 => format!("{} min ago", seconds / 60),
        3600..=86_399 => format!("{} h ago", seconds / 3600),
        _ => format!("{} days ago", seconds / 86_400),
    }
}

fn deleted_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine) {
    let mut open = state.deleted_open;
    egui::Window::new("Recently Deleted")
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .default_width(420.0)
        .show(ui.ctx(), |ui| {
            let batches: Vec<(u64, String, String, String)> = engine
                .deleted_batches()
                .iter()
                .rev()
                .map(|batch| (batch.id, batch.summary(), age_label(batch.applied_at), batch.mode.clone()))
                .collect();
            if batches.is_empty() {
                hint(ui, t, "Nothing has been applied yet.");
                ui.add_space(4.0);

                ui.label(
                    RichText::new("Applying a cull batch lists it here so you can put it back.")
                        .size(10.5)
                        .color(t.text_faint),
                );
                return;
            }
            let mut restore = None;
            for (id, summary, age, mode) in batches {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(summary).size(12.0).color(t.text));
                        ui.label(RichText::new(format!("{age} · {mode}")).size(10.5).color(t.text_faint));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(RichText::new("Restore").size(11.0)).clicked() {
                            restore = Some(id);
                        }
                    });
                });
                ui.separator();
            }
            if let Some(id) = restore {
                match engine.restore_batch(id) {
                    Ok(count) => state.notice(format!("Restored {count} photo(s)")),
                    Err(error) => state.notice(error.to_string()),
                }
            }
            ui.label(
                RichText::new("Files sent to the system Trash are restored from the Trash itself.")
                    .size(10.5)
                    .color(t.text_faint),
            );
        });
    state.deleted_open = open;
}

fn rename_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine) {
    let Some((id, mut name)) = state.rename_target.clone() else {
        return;
    };
    let mut open = true;
    egui::Window::new("Rename collection")
        .collapsible(false)
        .resizable(false)
        .default_width(300.0)
        .show(ui.ctx(), |ui| {
            let field = ui.add(TextEdit::singleline(&mut name).desired_width(f32::INFINITY));
            field.request_focus();
            let entered = ui.input(|input| input.key_pressed(Key::Enter));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if (ui.button("Save").clicked() || entered) && !name.trim().is_empty() {
                    match engine.rename_collection(id, &name) {
                        Ok(()) => state.notice(format!("Renamed to {}", name.trim())),
                        Err(error) => state.notice(error.to_string()),
                    }
                    open = false;
                }
                if ui.button("Cancel").clicked() {
                    open = false;
                }
            });
            let _ = t;
        });
    state.rename_target = if open { Some((id, name)) } else { None };
}

fn health_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine) {
    let mut open = state.health_open;
    egui::Window::new("Library health")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(400.0)
        .show(ui.ctx(), |ui| {
            match state.health {
                Some((checked, present, missing)) => {
                    ui.label(
                        RichText::new(format!("{present} of {checked} files present · {missing} missing"))
                            .size(12.5)
                            .strong(),
                    );
                }
                None => hint(ui, t, "Not checked yet."),
            }
            ui.add_space(8.0);
            if ui.button(RichText::new("Check library").size(11.5)).clicked() {
                match engine.check_health() {
                    Ok(health) => {
                        state.health = Some((health.checked, health.present, health.missing));
                        state.notice(format!("Checked {} files · {} missing", health.checked, health.missing));
                    }
                    Err(error) => state.notice(error.to_string()),
                }
            }
            ui.label(
                RichText::new("Stats every file and updates the missing badges in one undoable step.")
                    .size(10.5)
                    .color(t.text_faint),
            );
            ui.add_space(8.0);
            if ui.button(RichText::new("Find moved photos").size(11.5)).clicked() {
                match engine.relocate_missing() {
                    Ok(0) => state.notice("No moved photos found under their roots"),
                    Ok(count) => state.notice(format!("Relocated {count} photo(s)")),
                    Err(error) => state.notice(error.to_string()),
                }
            }
            ui.label(
                RichText::new("Matches missing photos by content key under the same root.")
                    .size(10.5)
                    .color(t.text_faint),
            );
            ui.add_space(8.0);
            if ui.button(RichText::new("Remove missing from library").size(11.5)).clicked() {
                match engine.remove_missing() {
                    Ok(0) => state.notice("Nothing is marked missing"),
                    Ok(count) => state.notice(format!("Removed {count} missing photo(s) — undo restores them")),
                    Err(error) => state.notice(error.to_string()),
                }
            }
            ui.label(
                RichText::new("Removes catalog entries only; the files are already gone.")
                    .size(10.5)
                    .color(t.text_faint),
            );
        });
    state.health_open = open;
}

fn apply_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine) {
    let (rejects, picks) = match engine.cull() {
        Some(session) => {
            let summary = session.summary();
            (summary.rejects, summary.picks)
        }
        None => (0, 0),
    };
    let mut open = state.apply_open;
    egui::Window::new("Apply cull batch")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(380.0)
        .show(ui.ctx(), |ui| {
            ui.label(RichText::new(format!("{rejects} rejects · {picks} picks")).size(13.0).strong());
            ui.add_space(6.0);
            ui.label(RichText::new("What happens to the rejects").size(11.0).color(t.text_muted));
            for mode in DeleteMode::ALL {
                if ui.radio(state.delete_mode == mode, mode.label()).clicked() {
                    state.delete_mode = mode;
                }
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new("Files that changed on disk since they were indexed stop the whole batch.")
                    .size(10.5)
                    .color(t.text_faint),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(rejects + picks > 0, Button::new(RichText::new("Apply").size(12.0)))
                    .clicked()
                {
                    match engine.cull_apply(state.delete_mode.mode()) {
                        Ok(report) => state.notice(format!(
                            "Applied {} photo(s){}",
                            report.applied.len(),
                            if report.has_failures() {
                                format!(", {} failed", report.failed.len())
                            } else {
                                String::new()
                            }
                        )),
                        Err(error) => state.notice(error.to_string()),
                    }
                    state.apply_open = false;
                }
                if ui.button(RichText::new("Cancel").size(12.0)).clicked() {
                    state.apply_open = false;
                }
            });
        });
    state.apply_open = open;
}

/// The size the picture would have at fit, which is what zoom is relative to.
fn fitted_size(area: Rect, photo: Option<&Photo>) -> (f32, f32) {
    let aspect = match photo {
        Some(photo) if photo.width > 0 && photo.height > 0 => photo.width as f32 / photo.height as f32,
        _ => 1.5,
    };
    let fitted = fit_rect(area.shrink(8.0), aspect);
    (fitted.width(), fitted.height())
}

/// Wheel zoom, ⌥-drag pan, Z to fit, and a plain drag deciding the photo.
///
/// Returns the gesture to draw while the finger is down and how far it has travelled.
fn canvas_interaction(
    state: &mut UiState,
    engine: &mut Engine,
    ui: &Ui,
    area: Rect,
    response: &egui::Response,
    fitted: (f32, f32),
    current: Option<sortlens_core::PhotoId>,
) -> (Gesture, f32) {
    let size = (area.width(), area.height());
    let mut delta = (0.0, 0.0);
    if response.dragged()
        && let Some(origin) = state.drag_from
        && let Some(pointer) = response.interact_pointer_pos()
    {
        delta = (pointer.x - origin.x, pointer.y - origin.y);
    }
    if response.drag_started() {
        state.drag_from = response.interact_pointer_pos();
        delta = (0.0, 0.0);
    }

    if response.hovered() {
        let notches = ui.input(|input| input.smooth_scroll_delta.y / 50.0);
        state.cull_view.wheel(notches, size, fitted);
    }
    let panning = ui.input(|input| input.modifiers.alt || input.modifiers.command);
    if panning && response.dragged() && state.cull_view.pan(delta, size, fitted) {
        delta = (0.0, 0.0);
    }
    if ui.input(|input| input.key_pressed(Key::Z) && !input.modifiers.command) {
        if state.cull_view.is_zoomed() {
            state.cull_view.reset();
        } else {
            state.cull_view.zoom_by(2.0, size, fitted);
        }
    }

    if !response.drag_stopped() {
        return (sortlens_cull::gesture::classify(delta), sortlens_cull::gesture::progress(delta));
    }
    let gesture = sortlens_cull::gesture::classify(delta);
    if gesture.decision().is_some() && current.is_some() {
        engine.cull_decide(match gesture {
            Gesture::Pick => Decision::Pick,
            _ => Decision::Reject,
        });
        state.notice(match gesture {
            Gesture::Pick => "Kept".to_string(),
            _ => "Rejected".to_string(),
        });
    }
    state.drag_from = None;
    (Gesture::None, 0.0)
}

/// The quit guard: what would be lost, and the two ways out.
fn quit_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &Engine, actions: &mut Vec<Action>) {
    let reasons = engine.unsaved();
    let mut open = state.quit_open;
    egui::Window::new("Quit Sortlens?")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(380.0)
        .show(ui.ctx(), |ui| {
            for reason in &reasons {
                ui.label(RichText::new(reason).size(11.5).color(t.text_muted));
                ui.add_space(4.0);
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Keep working").size(12.0)).clicked() {
                    state.quit_open = false;
                }
                if ui
                    .button(RichText::new("Quit anyway").size(12.0))
                    .on_hover_text("Close the window now; unstaged work is dropped")
                    .clicked()
                {
                    state.quit_open = false;
                    actions.push(Action::Quit);
                }
            });
        });
    state.quit_open = open;
}

/// What the session came to, and what can be done with it.
fn summary_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine) {
    let Some(session) = engine.cull() else {
        state.summary_open = false;
        return;
    };
    let summary = session.summary();
    let mut open = state.summary_open;
    egui::Window::new("Cull summary")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(340.0)
        .show(ui.ctx(), |ui| {
            ui.label(RichText::new(summary.headline()).size(14.0).strong());
            ui.add_space(8.0);
            for (label, count) in [
                ("Picks", summary.picks),
                ("Rejects", summary.rejects),
                ("Deferred", summary.deferred),
                ("Not judged", summary.unjudged),
            ] {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(label).size(11.5).color(t.text_muted));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format_count(count as u64)).size(11.5));
                    });
                });
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!(
                    "{} photo(s) will change on Apply; {} deferral(s) are never written.",
                    summary.changed(),
                    summary.deferred
                ))
                .size(10.5)
                .color(t.text_faint),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(summary.changed() > 0, Button::new(RichText::new("Apply…").size(12.0)))
                    .clicked()
                {
                    state.apply_open = true;
                    state.summary_open = false;
                }
                if ui
                    .add_enabled(summary.unjudged > 0, Button::new(RichText::new("Keep culling").size(12.0)))
                    .clicked()
                {
                    state.summary_open = false;
                }
                if ui
                    .add_enabled(!summary.is_complete(), Button::new(RichText::new("Jump to next").size(12.0)))
                    .on_hover_text("Skip to the first photo without a decision")
                    .clicked()
                {
                    engine.cull_next_unjudged();
                    state.summary_open = false;
                }
                if ui
                    .button(RichText::new("Discard").size(12.0))
                    .on_hover_text("Drop every staged decision and restore the flags")
                    .clicked()
                {
                    engine.cull_clear();
                    state.summary_open = false;
                    state.notice("Cull discarded".to_string());
                }
            });
        });
    state.summary_open = open;
}

/// The receipt for a finished Apply, with the way back.
fn receipt_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine) {
    let Some(receipt) = engine.receipt().cloned() else {
        state.receipt_open = false;
        return;
    };
    let mut open = state.receipt_open;
    let mut restore = false;
    let mut show_bin = false;
    let mut done = false;
    egui::Window::new("Batch applied")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(360.0)
        .show(ui.ctx(), |ui| {
            ui.label(RichText::new(receipt.headline()).size(14.0).strong());
            ui.label(RichText::new(receipt.detail()).size(11.0).color(t.text_muted));
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!("Batch {} · {} file(s) kept on disk", receipt.batch_id, receipt.kept))
                    .size(10.5)
                    .color(t.text_faint),
            );
            if receipt.failed > 0 {
                ui.label(
                    RichText::new(format!("{} file(s) could not be handled", receipt.failed))
                        .size(10.5)
                        .color(t.reject),
                );
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(receipt.is_restorable(), Button::new(RichText::new("Put them back").size(12.0)))
                    .clicked()
                {
                    restore = true;
                }
                if ui.button(RichText::new("Recently Deleted").size(12.0)).clicked() {
                    show_bin = true;
                }
                if ui.button(RichText::new("Done").size(12.0)).clicked() {
                    done = true;
                }
            });
        });
    if restore {
        match engine.restore_batch(receipt.batch_id) {
            Ok(_) => {
                state.notice("Restored the batch".to_string());
                done = true;
            }
            Err(error) => state.notice(error.to_string()),
        }
    }
    if show_bin {
        state.deleted_open = true;
        done = true;
    }
    if !open || done {
        engine.clear_receipt();
        state.receipt_open = false;
    }
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
            ui.label(RichText::new("Applying a cull batch").size(11.0).color(t.text_muted));
            for mode in DeleteMode::ALL {
                if ui.radio(state.delete_mode == mode, mode.label()).clicked() {
                    state.delete_mode = mode;
                }
            }
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
    fn the_welcome_wizard_follows_what_the_library_is_doing() {
        // Nothing added yet: the only useful step is adding a folder.
        assert_eq!(welcome_stage(0, false, 0, false), Some(Welcome::Add));
        // A root that turned out to be empty is the same situation.
        assert_eq!(welcome_stage(1, false, 0, false), Some(Welcome::Add));
        // Scanning wins, even with photos already indexed elsewhere.
        assert_eq!(welcome_stage(1, true, 40, false), Some(Welcome::Indexing));
        // Photos, no scan: the grid takes over and the wizard is gone.
        assert_eq!(welcome_stage(1, false, 12, false), None);
        // Dismissed stays dismissed whatever else happens.
        assert_eq!(welcome_stage(0, false, 0, true), None);
        assert_eq!(welcome_stage(1, true, 0, true), None);
    }

    #[test]
    fn closing_a_collection_restores_the_library_search() {
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.library_search = "birthday".to_string();
        state.search = "beach".to_string();
        state.collection_filter = Some(sortlens_core::CollectionId(7));
        state.clear_filters();
        assert_eq!(state.search, "birthday", "the library's own search comes back");
    }

    #[test]
    fn closing_a_collection_restores_the_library_sort() {
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.library_sort = Sort::Newest;
        state.sort = Sort::Largest;
        state.collection_filter = Some(sortlens_core::CollectionId(4));
        state.clear_filters();
        assert!(state.collection_filter.is_none());
        assert_eq!(state.sort, Sort::Newest, "the library's own order comes back");
    }

    #[test]
    fn the_active_filter_badge_counts_dimensions() {
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        assert_eq!(state.active_filter_count(), 0);
        state.min_rating = 3;
        state.tag_filters = vec!["beach".into(), "sunset".into()];
        state.show_missing = true;
        assert_eq!(state.active_filter_count(), 3, "two keywords count as one dimension");
        state.clear_filters();
        assert_eq!(state.active_filter_count(), 0, "clearing removes everything");
    }

    #[test]
    fn parent_prefix_walks_up_one_level_at_a_time() {
        assert_eq!(parent_prefix("day1/sub/"), "day1/");
        assert_eq!(parent_prefix("day1/"), "");
        assert_eq!(parent_prefix(""), "");
    }

    #[test]
    fn a_nonsense_aspect_falls_back_to_three_by_two() {
        let bounds = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 300.0));
        let fitted = fit_rect(bounds, 0.0);
        assert!((fitted.width() / fitted.height() - 1.5).abs() < 0.001);
    }
}
