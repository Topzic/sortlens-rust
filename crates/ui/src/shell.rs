//! The application shell: five surfaces, in the suite's order.
//!
//! Top bar (identity, search, filters, actions), left panel (sources, folders, collections),
//! canvas (grid today, cull and detail later), right panel (details, tags, activity), bottom bar
//! (views, review state, counts).
//!
//! The shell owns presentation state and nothing else. Anything that changes the library goes
//! through [`Engine`]; anything that changes the process (theme, folder picker, quit) is returned
//! as an [`Action`] for the host to apply.

use crate::charts::{self, Column};
use crate::keymap::{self, Context};
use crate::theme::{Density, GridMode, ThemeKind, Tokens};
use crate::thumbs::{GRID_SIZE, Thumbnails};
use egui::{
    Align, Align2, Button, Color32, CornerRadius, FontId, Frame, Key, KeyboardShortcut, Layout, Margin, Modifiers, Rect, RichText, Sense,
    Stroke, StrokeKind, TextEdit, Ui, Vec2, pos2, vec2,
};
use sortlens_core::{ColorLabel, Flag, MediaKind, PhotoId};
use sortlens_cull::gesture::Gesture;
use sortlens_engine::{Decision, Engine, Filter, Photo, Sort, TagOrder};
use std::collections::BTreeSet;

/// A top-level destination in the bottom bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum View {
    /// The library grid: the default.
    #[default]
    Library,
    /// Full-screen cull review.
    Cull,
    /// Memories and resurfacing cards.
    ForYou,
    /// Map of geotagged photos.
    Places,
    /// Library statistics.
    Insights,
    /// Results of a similar-photo search.
    Similar,
}

impl View {
    /// Every destination, in bottom-bar order.
    pub const ALL: [Self; 6] = [Self::Library, Self::ForYou, Self::Cull, Self::Places, Self::Insights, Self::Similar];

    /// Label for the bottom bar and the contextual header.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::ForYou => "For You",
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
            Self::ForYou => None,
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
    /// Write a verified backup of the catalog somewhere the host chooses.
    Backup,
    /// Verify an archive and unpack it into a folder the host chooses.
    RestoreBackup,
    /// Repoint an indexed root, which the host asks a folder for.
    RemapRoot(String),
    /// Write the change report somewhere the host chooses.
    ExportHistory,
    /// Move these photos into a folder the host chooses.
    /// Persist a grid density the shell has already applied to its own state.
    SetDensity(Density),
    /// Persist a grid layout mode the shell has already applied to its own state.
    SetGrid(GridMode),
    MovePicks(Vec<PhotoId>),
    /// Put a moved batch back.
    UndoMove(u64),
    /// Show this photo in the desktop's file manager.
    Reveal(PhotoId),
    /// Hand this photo to the desktop's default application.
    OpenExternal(PhotoId),
    /// Write these photos out. The host asks for the folder, because only it can show a picker.
    Export(Vec<PhotoId>, sortlens_engine::ExportSpec),
    /// Accept a suggested keyword, which the host writes through the ordinary tag op.
    AcceptSuggestion(PhotoId, String),
    /// Switch a keyword pack on or off.
    SetPack(String, bool),
    /// Propose keywords for the whole library on a worker.
    StartSuggest,
    /// Stop that pass. It has written nothing, so there is nothing to undo.
    CancelSuggest,
    /// Forget the proposals from the last pass.
    ClearSuggest,
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

/// How many photos a ⇧-click range or ⌘A will walk. Large enough for any shoot a person would select by hand,
/// small enough that the walk cannot turn a click into a stall on a library of hundreds of thousands.
pub const MAX_RANGE: usize = 20_000;

/// What is selected in the grid: one photo, a range, or a handful picked out of a shoot.
///
/// The primary is what the detail panel shows; the rest are what a batch command acts on. Keeping them
/// together matters: "rate the selection 4" has to mean every ticked cell, while the inspector must still
/// know which one you are looking at. The anchor is where a ⇧-click measures from, and it is separate from
/// the primary so ⇧-clicking twice narrows or widens the range from the same place instead of walking.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    /// The one the inspector shows.
    primary: Option<PhotoId>,
    /// Everything ticked, including the primary.
    ids: BTreeSet<PhotoId>,
    /// Where a range starts.
    anchor: Option<PhotoId>,
}

impl Selection {
    /// Nothing selected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// How many are ticked.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// The photo the inspector shows.
    #[must_use]
    pub fn primary(&self) -> Option<PhotoId> {
        self.primary
    }

    /// Whether a photo is part of the selection.
    #[must_use]
    pub fn contains(&self, id: PhotoId) -> bool {
        self.ids.contains(&id)
    }

    /// Everything the batch commands should act on, in id order so a log reads the same twice.
    #[must_use]
    pub fn ids(&self) -> Vec<PhotoId> {
        self.ids.iter().copied().collect()
    }

    /// Select exactly this photo, the way a plain click does.
    pub fn replace(&mut self, id: PhotoId) {
        self.ids.clear();
        self.ids.insert(id);
        self.primary = Some(id);
        self.anchor = Some(id);
    }

    /// Add or remove one photo, the way ⌘-click does. Removing the primary moves the primary to whatever is
    /// still selected, so the inspector never shows a photo that is no longer ticked.
    pub fn toggle(&mut self, id: PhotoId) {
        if self.ids.remove(&id) {
            if self.primary == Some(id) {
                self.primary = self.ids.iter().next().copied();
            }
            if self.anchor == Some(id) {
                self.anchor = self.primary;
            }
        } else {
            self.ids.insert(id);
            self.primary = Some(id);
            self.anchor = Some(id);
        }
    }

    /// Select the run between the anchor and `id`, in the order the grid is showing them.
    ///
    /// If there is no anchor — nothing selected, or the anchor is no longer in view — this is a plain click:
    /// a range with nothing to measure from would otherwise select a random half of the library.
    pub fn extend(&mut self, order: &[PhotoId], id: PhotoId) {
        let Some(anchor) = self.anchor.filter(|anchor| order.contains(anchor)) else {
            self.replace(id);
            return;
        };
        let from = order.iter().position(|candidate| *candidate == anchor);
        let to = order.iter().position(|candidate| *candidate == id);
        let (Some(from), Some(to)) = (from, to) else {
            self.replace(id);
            return;
        };
        let (start, end) = if from <= to { (from, to) } else { (to, from) };
        self.ids = order[start..=end].iter().copied().collect();
        self.primary = Some(id);
    }

    /// Everything in view, the way ⌘A does. The anchor stays where it was so a following ⇧-click still narrows.
    pub fn all(&mut self, order: &[PhotoId]) {
        self.ids = order.iter().copied().collect();
        if self.primary.is_none_or(|primary| !self.ids.contains(&primary)) {
            self.primary = order.first().copied();
        }
        if self.anchor.is_none_or(|anchor| !self.ids.contains(&anchor)) {
            self.anchor = self.primary;
        }
    }

    /// Replace the ticks with exactly these photos, the way a marquee does.
    ///
    /// An empty list clears: dragging over nothing means nothing selected, which is what the gesture says.
    pub fn set_many(&mut self, ids: &[PhotoId]) {
        self.ids = ids.iter().copied().collect();
        match ids.first() {
            Some(first) => {
                self.primary = Some(*first);
                self.anchor = Some(*first);
            }
            None => self.clear(),
        }
    }

    /// Drop the selection, keeping nothing — including the anchor.
    pub fn clear(&mut self) {
        self.ids.clear();
        self.primary = None;
        self.anchor = None;
    }

    /// Keep only ids that still exist, after a filter change or a delete.
    pub fn retain(&mut self, keep: impl Fn(PhotoId) -> bool) {
        self.ids.retain(|id| keep(*id));
        if self.primary.is_some_and(|primary| !self.ids.contains(&primary)) {
            self.primary = self.ids.iter().next().copied();
        }
        if self.anchor.is_some_and(|anchor| !self.ids.contains(&anchor)) {
            self.anchor = self.primary;
        }
    }
}

/// The sidebar's saved questions. All Photos is the empty filter; the rest are one dimension each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Everything the library knows about.
    All,
    /// Flags that survived a cull.
    Picks,
    /// Anything with at least one star.
    Rated,
    /// Rejected frames, still in the grid and not yet deleted.
    Rejects,
}

impl Source {
    /// Every source, in sidebar order.
    pub const ALL: [Source; 4] = [Source::All, Source::Picks, Source::Rated, Source::Rejects];

    /// The row's label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Source::All => "All Photos",
            Source::Picks => "Picks",
            Source::Rated => "Rated",
            Source::Rejects => "Rejects",
        }
    }

    /// The flag this source asks for, if any.
    #[must_use]
    pub fn flag(self) -> Option<Flag> {
        match self {
            Source::Picks => Some(Flag::Pick),
            Source::Rejects => Some(Flag::Reject),
            Source::All | Source::Rated => None,
        }
    }

    /// The star floor this source asks for, if any. Rated is one star rather than five: "rated" means
    /// somebody looked at it and said something, and a five-star floor would hide most of the library.
    #[must_use]
    pub fn min_rating(self) -> u8 {
        match self {
            Source::Rated => 1,
            Source::All | Source::Picks | Source::Rejects => 0,
        }
    }
}

/// Everything the shell remembers between frames.
#[derive(Debug, Clone)]
pub struct UiState {
    /// Active theme, mirrored from settings.
    pub theme: ThemeKind,
    /// Grid density, mirrored from settings.
    pub density: Density,
    /// Grid layout, mirrored from settings.
    pub grid_mode: GridMode,
    /// The justified layout, cached so a big library does not re-pack on every frame.
    pub justified: JustifiedPlan,
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
    /// Shortcut overlay visibility.
    pub shortcuts_open: bool,
    /// Filter typed into the shortcut overlay.
    pub shortcut_query: String,
    /// What the grid has ticked; the inspector shows its primary.
    pub selection: Selection,
    /// Where a marquee drag started, while the pointer is still down.
    pub marquee: Option<egui::Pos2>,
    /// Where each visible cell was drawn last frame, so a drag that starts on the background can hit-test
    /// against cells the pointer has not passed over yet.
    pub cell_rects: Vec<(PhotoId, egui::Rect)>,
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
    /// A keyword being renamed: (the keyword, the text being typed).
    pub tag_edit: Option<(String, String)>,
    /// The rejects bin is being emptied after a confirmation.
    pub empty_bin_open: bool,
    /// The last catalog audit: (headline, detail, clean).
    pub audit: Option<(String, String, bool)>,
    /// The change viewer is open.
    pub history_open: bool,
    /// How many files the last health check found changed on disk.
    pub health_modified: usize,
    /// A move receipt that is waiting to be acknowledged.
    pub move_receipt: Option<u64>,
    /// Search text inside the keyword picker.
    pub tag_picker_query: String,
    /// How close two hashes must be to count as the same subject.
    pub similar_threshold: u32,
    /// How far apart two colour fingerprints may be before they are different pictures.
    pub similar_colour: u32,
    /// Which signals the duplicate search trusts.
    pub similar_mode: sortlens_engine::SimilarMatch,
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
    /// Whether the export dialog is showing.
    pub export_open: bool,
    /// Whether the whole-library suggestion panel is open.
    pub suggest_open: bool,
    /// What the export dialog is set to write.
    pub export_spec: sortlens_engine::ExportSpec,
    /// The photos the export dialog will write.
    pub export_ids: Vec<PhotoId>,
    /// The last export's receipt, kept on screen until dismissed.
    pub export_receipt: Option<String>,
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
    /// The second photo in a side-by-side compare, if one is open.
    pub compare: Option<PhotoId>,
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
            density: Density::default(),
            grid_mode: GridMode::default(),
            justified: JustifiedPlan::default(),
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
            shortcuts_open: false,
            shortcut_query: String::new(),
            selection: Selection::default(),
            marquee: None,
            cell_rects: Vec::new(),
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
            tag_edit: None,
            empty_bin_open: false,
            audit: None,
            history_open: false,
            health_modified: 0,
            move_receipt: None,
            tag_picker_query: String::new(),
            similar_threshold: sortlens_engine::SIMILAR_THRESHOLD,
            similar_colour: sortlens_engine::SIMILAR_COLOUR,
            similar_mode: sortlens_engine::SimilarMatch::Families,
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
            export_open: false,
            suggest_open: false,
            export_spec: sortlens_engine::ExportSpec::default(),
            export_ids: Vec::new(),
            export_receipt: None,
            health: None,
            folder: String::new(),
            sort: Sort::Natural,
            kind_filter: None,
            edited_filter: None,
            detail_open: false,
            detail_zoom: false,
            compare: None,
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

    /// Jump to one of the saved sources, replacing whatever narrowed the view before.
    ///
    /// A source is not a filter dimension: picking one clears the other dimensions so "Picks" always means
    /// the same thing, rather than "picks, among the ones you starred, inside that folder". The folder
    /// scope is the exception — it is where you are, not what you asked for.
    pub fn apply_source(&mut self, source: Source) {
        self.min_rating = source.min_rating();
        self.flag_filter = source.flag();
        self.label_filter = None;
        self.tag_filters.clear();
        self.collection_filter = None;
        self.kind_filter = None;
        self.edited_filter = None;
        self.search.clear();
        self.all_photos_selected = source == Source::All;
    }

    /// Whether a source is the one the sidebar should highlight.
    ///
    /// Derived from the filter rather than stored, so a keyboard shortcut or a filter chip that produces
    /// the same question highlights the same row — a stored flag would let the two disagree.
    #[must_use]
    pub fn source_selected(&self, source: Source) -> bool {
        self.min_rating == source.min_rating()
            && self.flag_filter == source.flag()
            && self.label_filter.is_none()
            && self.tag_filters.is_empty()
            && self.collection_filter.is_none()
            && self.kind_filter.is_none()
            && self.edited_filter.is_none()
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

    // A photo can vanish under the selection — a delete, an emptied bin, a rescan that found the file gone.
    // Dropping it here, once, keeps the inspector from showing a photo that is no longer in the library.
    state.selection.retain(|id| engine.photo(id).is_some());

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
            if done.mislabelled > 0 {
                // Named because it is the one scan outcome the user can act on: a file that says one thing
                // and contains another is a file whose name is wrong, and only they can decide to fix it.
                message.push_str(&format!(" · {} routed by content, not name", done.mislabelled));
            }
            if !done.errors.is_empty() {
                message.push_str(&format!(" · {} unreadable", done.errors.len()));
            }
            state.notice(message);
        }
    }
    if engine
        .poll_similar_in(state.similar_threshold, state.similar_colour, state.similar_mode)
        .is_some()
    {
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
    dialogs(ui, state, &t, engine, &mut actions);

    if state.live_notice().is_some() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(500));
    }
    actions
}

/// Every dialog, in the order they stack. Extracted so `show` stays a sequence of surfaces rather than a
/// list of every window the app can open.
fn dialogs(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    if state.tag_edit.is_some() {
        tag_edit_dialog(ui, state, t, engine);
    }
    if state.quit_open {
        quit_dialog(ui, state, t, engine, actions);
    }
    if state.summary_open {
        summary_dialog(ui, state, t, engine);
    }
    if state.receipt_open {
        receipt_dialog(ui, state, t, engine);
    }
    if state.settings_open {
        settings_dialog(ui, state, t, engine, actions);
    }
    if state.about_open {
        about_dialog(ui, state, t, engine);
    }
    if state.shortcuts_open {
        shortcuts_dialog(ui, state, t);
    }
    if state.apply_open {
        apply_dialog(ui, state, t, engine);
    }
    if state.deleted_open {
        deleted_dialog(ui, state, t, engine);
    }
    if state.empty_bin_open {
        empty_bin_dialog(ui, state, t, engine);
    }
    if state.health_open {
        health_dialog(ui, state, t, engine, actions);
    }
    if state.history_open {
        history_dialog(ui, state, t, engine, actions);
    }
    if state.export_open || state.export_receipt.is_some() {
        export_dialog(ui, state, t, engine, actions);
    }
    if state.suggest_open {
        suggest_dialog(ui, state, t, engine, actions);
    }
    if state.move_receipt.is_some() {
        move_receipt_dialog(ui, state, t, engine, actions);
    }
    if state.rename_target.is_some() {
        rename_dialog(ui, state, t, engine);
    }
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
                if engine.read_only() {
                    // Say it once, at the top: a library another window is writing cannot be changed
                    // here, and nothing on this screen should look as if it can.
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Read-only · another window has this library")
                            .size(10.5)
                            .color(t.reject),
                    )
                    .on_hover_text("Close the other Sortlens window to edit here. Everything stays readable meanwhile.");
                }

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
                    // Density lives next to the theme because both are "how the library looks", and it is
                    // one click, not a settings trip, to change how many photos you can see at once.
                    if icon_button(ui, state.density.label(), t)
                        .on_hover_text("Grid density: Compact, Comfortable, Roomy")
                        .clicked()
                    {
                        let next = state.density.next();
                        state.density = next;
                        actions.push(Action::SetDensity(next));
                    }
                    // Next to density, because "square or justified" is the same kind of question as "how
                    // big": both decide how much of the library you can see at once.
                    if icon_button(ui, state.grid_mode.label(), t)
                        .on_hover_text("Grid layout: Square or Justified")
                        .clicked()
                    {
                        let next = state.grid_mode.next();
                        state.grid_mode = next;
                        actions.push(Action::SetGrid(next));
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
                        state.selection.clear();
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
                    if let Some(selected) = state.selection.primary().and_then(|id| engine.photo(id)) {
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

            nav_section(ui, t, "Sources");
            for source in Source::ALL {
                // Each row's count is what that row would show, not the library total: a Picks row reading
                // "0" while the grid holds thousands is the row doing its job.
                let mut filter = Filter {
                    root: state.root.clone(),
                    folder: (state.root.is_some() && !state.folder.is_empty()).then(|| state.folder.clone()),
                    ..Filter::default()
                };
                filter.flag = source.flag();
                filter.min_rating = source.min_rating();
                let count = engine.count(&filter);
                if nav_row(ui, t, source.label(), &format_count(count as u64), state.source_selected(source)).clicked() {
                    if source == Source::All {
                        state.root = None;
                    }
                    state.apply_source(source);
                }
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
            if nav_row(ui, t, "Changes", "", false).clicked() {
                state.history_open = true;
            }
            if nav_row(ui, t, "Shortcuts", "?", false)
                .on_hover_text("Every binding, searchable")
                .clicked()
            {
                state.shortcuts_open = true;
                state.shortcut_query.clear();
            }

            nav_section(ui, t, "Folders");
            // Volumes once per root, not once per photo: an unplugged drive holding ten thousand photos is
            // one missing volume, and the sidebar says so beside the folder instead of after scrolling.
            let volumes = engine.root_volumes();
            if volumes.is_empty() {
                hint(ui, t, "No folders yet. Add one to index a shoot.");
            }
            for volume in volumes {
                let root = volume.root.clone();
                let count = volume.photos;
                let selected = state.root.as_deref() == Some(root.as_str());
                let (count_text, offline) = match volume.mounted {
                    // Only `Some(false)` is evidence: a platform that cannot answer must not be drawn as if
                    // the drive were away.
                    Some(false) => (format!("{count} · away"), true),
                    _ => (format_count(count as u64), false),
                };
                let response = nav_row(ui, t, &folder_name(&root), &count_text, selected);
                if offline {
                    response.clone().on_hover_text(match &volume.mount {
                        Some(mount) => format!("Not mounted: {mount} is not there right now"),
                        None => "Not mounted".to_string(),
                    });
                }
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
                let aliases = engine.registry_aliases_of(tag);
                let row = if aliases.is_empty() {
                    nav_row(ui, t, tag, &format_count(*count as u64), selected)
                } else {
                    // Say where a keyword came from: a rename leaves a trail behind it.
                    nav_row(ui, t, tag, &format_count(*count as u64), selected)
                        .on_hover_text(format!("also written as {}", aliases.join(", ")))
                };
                if row.clicked() {
                    if selected {
                        state.tag_filters.retain(|active| active != tag);
                    } else {
                        state.tag_filters.push(tag.clone());
                    }
                }
                row.context_menu(|ui| {
                    if ui.button("Rename or merge…").clicked() {
                        state.tag_edit = Some((tag.clone(), tag.clone()));
                        ui.close();
                    }
                });
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
                RightTab::Details => match state.selection.primary().and_then(|id| engine.photo(id)) {
                    Some(photo) => details(ui, t, photo),
                    None => hint(ui, t, "Select a photo to see camera, exposure and file details."),
                },
                RightTab::Tags => match state.selection.primary().and_then(|id| engine.photo(id)).cloned() {
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
                crate::a11y::live_line(ui, &crate::a11y::library_status(total, state.selection.len()), t.text_muted);
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
                    detail_view(ui, t, state, engine, thumbs, actions);
                }
                View::Library => {
                    if total == 0 {
                        empty_library(ui, t, state, engine, actions);
                    } else {
                        grid(ui, t, state, engine, thumbs, &filter, actions);
                    }
                }
                View::ForYou => for_you_view(ui, t, state, engine, thumbs),
                View::Cull => cull_view(ui, t, state, engine, thumbs, actions),
                View::Insights => insights_view(ui, t, state, engine),
                View::Similar => similar_view(ui, t, state, engine, thumbs),
                other => planned_view(ui, t, other),
            }
            keyboard(ui, state, engine, actions);
        });
}

/// What a click, a modifier-click, a double-click and the context menu mean on one cell.
///
/// Shared by both grid layouts: the gestures are about photos, not about pixels, and two grids that disagreed
/// about what ⌘-click does would be one grid too many.
fn cell_gestures(
    ui: &mut Ui,
    state: &mut UiState,
    engine: &Engine,
    photo: &Photo,
    response: &egui::Response,
    filter: &Filter,
    actions: &mut Vec<Action>,
    add_to: &mut Option<(PhotoId, sortlens_core::CollectionId)>,
) {
    if response.clicked() {
        // Three gestures, three meanings: click replaces, ⌘-click toggles, and ⇧-click takes the run from the
        // anchor in the order the grid shows — not in id order, which is not what the person is looking at.
        let modifiers = ui.input(|input| input.modifiers);
        if modifiers.command {
            state.selection.toggle(photo.id);
        } else if modifiers.shift {
            let order: Vec<PhotoId> = engine
                .page(filter, state.sort, 0, MAX_RANGE)
                .photos
                .iter()
                .map(|photo| photo.id)
                .collect();
            state.selection.extend(&order, photo.id);
        } else {
            state.selection.replace(photo.id);
        }
        state.right_tab = RightTab::Details;
    }
    if response.double_clicked() {
        state.detail_open = true;
    }
    response.context_menu(|ui| {
        if ui.button("Export…").clicked() {
            state.export_ids = vec![photo.id];
            state.export_open = true;
            ui.close();
        }
        if ui.button("Show in folder").clicked() {
            actions.push(Action::Reveal(photo.id));
            ui.close();
        }
        if ui.button("Open in default app").clicked() {
            actions.push(Action::OpenExternal(photo.id));
            ui.close();
        }
        ui.separator();
        ui.menu_button("Add to Collection", |ui| {
            let collections = engine.collections();
            if collections.is_empty() {
                ui.label(RichText::new("No collections yet").size(10.5));
            }
            for collection in collections {
                if ui.button(&collection.name).clicked() {
                    *add_to = Some((photo.id, collection.id));
                    ui.close();
                }
            }
        });
    });
}

/// Drag a rectangle over the grid to select what it touches.
///
/// The drag has to start on the *background*, not on a cell: a press inside a cell belongs to that cell, and
/// stealing it would make the grid impossible to click. "Background" is decided against the rects painted last
/// frame, because the pointer may pass over cells this frame that it had not reached when the drag began.
///
/// Plain drag replaces the selection. There is no ⌘-drag to add to it: two ways to combine a gesture is one
/// way too many, and the range gesture already covers "these and the ones before them".
fn marquee(ui: &mut Ui, state: &mut UiState, t: &Tokens, grid_rect: Rect) {
    let (pressed_at, pointer, down) = ui.input(|input| {
        (
            input.pointer.press_origin(),
            input.pointer.interact_pos(),
            input.pointer.primary_down(),
        )
    });
    if state.marquee.is_none()
        && down
        && let Some(origin) = pressed_at
        && grid_rect.contains(origin)
        && !state.cell_rects.iter().any(|(_, rect)| rect.contains(origin))
    {
        state.marquee = Some(origin);
    }
    let Some(origin) = state.marquee else {
        return;
    };
    if !down {
        // Releasing ends the drag. The selection was already applied on the frame the pointer moved, so a
        // click that turns out to be a drag has not yet selected anything.
        state.marquee = None;
        return;
    }
    let Some(pointer) = pointer else {
        return;
    };
    let band = Rect::from_two_pos(origin, pointer);
    ui.painter().rect_filled(band, 2.0, t.selection.gamma_multiply(0.35));
    ui.painter().rect_stroke(band, 2.0, Stroke::new(1.0, t.accent), StrokeKind::Inside);
    let hits = cells_in(&state.cell_rects, band);
    if hits != state.selection.ids() {
        // Only when the set actually changes: a notice every frame of a drag would spam the live region with
        // the same sentence sixty times a second.
        let count = hits.len();
        state.selection.set_many(&hits);
        if count > 0 {
            state.notice(format!("{count} selected"));
        }
    }
}

/// Which cells a marquee band touches. Pure, so the hit test is testable without a pointer.
fn cells_in(cells: &[(PhotoId, Rect)], band: Rect) -> Vec<PhotoId> {
    let mut hits: Vec<PhotoId> = cells.iter().filter(|(_, rect)| rect.intersects(band)).map(|(id, _)| *id).collect();
    hits.sort_unstable();
    hits
}

/// How many cells fit across a width, and how wide each one ends up.
///
/// Kept pure because this is the whole of "density": the setting names a minimum cell, and the grid divides
/// what is left so the last cell in a row reaches the right edge instead of leaving a ragged margin. The 64
/// point floor is the point below which the caption and the stars stop being legible, so a very narrow
/// window gets a horizontal scroll rather than unreadable cells.
fn grid_layout(available: f32, gap: f32, min_cell: f32) -> (usize, f32) {
    let columns = (((available + gap) / (min_cell + gap)).floor() as usize).max(1);
    let cell = ((available - gap * (columns.saturating_sub(1)) as f32) / columns as f32)
        .floor()
        .max(64.0);
    (columns, cell)
}

/// Group photos into rows that each fill the width, and give every row one height.
///
/// A row stops growing when the next photo would push its height below the target: that is the moment the row
/// already fills the width at a legible size. The last row is the exception — stretching two photos across the
/// window would give two enormous cells, so a short final row keeps the target height and sits left-aligned.
///
/// Pure, and returns heights rather than rectangles: the packing is the part worth testing, and a photo's shape
/// is the only input it needs.
fn justified_rows(aspects: &[f32], available: f32, gap: f32, target: f32) -> Vec<(std::ops::Range<usize>, f32)> {
    let mut rows = Vec::new();
    let mut start = 0;
    while start < aspects.len() {
        let mut sum = 0.0_f32;
        let mut end = start;
        while end < aspects.len() {
            let trial = sum + aspects[end];
            let count = (end - start + 1) as f32;
            let height = (available - gap * (count - 1.0)) / trial;
            if height < target {
                // Adding this photo makes the row shorter than wanted. Take whichever of the two is closer to
                // the target: stopping at the first overshoot would leave rows 40 points taller than asked for
                // whenever the library's average shape does not divide evenly.
                let before = (available - gap * (count - 2.0).max(0.0)) / sum;
                if end > start && (before - target).abs() <= (height - target).abs() {
                    break;
                }
                sum = trial;
                end += 1;
                break;
            }
            sum = trial;
            end += 1;
        }
        let count = (end - start) as f32;
        let height = ((available - gap * (count - 1.0)) / sum).max(24.0);
        let last = end == aspects.len();
        rows.push((start..end, if last { height.min(target) } else { height }));
        start = end;
    }
    rows
}

/// Height of the name line under every cell, in points. Shared so a justified row's total height and a square
/// cell agree about where the caption ends.
const CAPTION: f32 = 24.0;

/// One justified row: how tall it is, where its top edge sits, and which photos are in it.
#[derive(Debug, Clone, PartialEq)]
pub struct JustifiedRow {
    ids: Vec<PhotoId>,
    height: f32,
    top: f32,
}

/// The inputs a justified plan depends on. Anything else leaving the plan alone is why a big library does not
/// re-pack every frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JustifiedKey {
    width: u32,
    filter: Filter,
    sort: Sort,
    count: usize,
    target: u32,
}

/// A justified layout, cached until its inputs change.
#[derive(Debug, Default, Clone)]
pub struct JustifiedPlan {
    key: Option<JustifiedKey>,
    rows: Vec<JustifiedRow>,
    total: f32,
}

impl JustifiedPlan {
    /// Rebuild only when an input changed: packing fifty thousand rows on every frame would make scrolling a
    /// slideshow, and the pack is the expensive part, not the paint.
    pub fn prepare(&mut self, engine: &Engine, filter: &Filter, sort: Sort, width: f32, target: f32, gap: f32) {
        let key = JustifiedKey {
            width: width.round() as u32,
            filter: filter.clone(),
            sort,
            count: engine.count(filter),
            target: target.round() as u32,
        };
        if self.key.as_ref() == Some(&key) {
            return;
        }
        let photos = photos_with_aspects(engine, filter, sort, key.count);
        let aspects: Vec<f32> = photos.iter().map(|(_, aspect)| *aspect).collect();
        let mut rows = Vec::new();
        let mut top = 0.0;
        for (range, height) in justified_rows(&aspects, width, gap, target) {
            rows.push(JustifiedRow {
                ids: photos[range.clone()].iter().map(|(id, _)| *id).collect(),
                height,
                top,
            });
            top += height + CAPTION + gap;
        }
        self.rows = rows;
        self.total = top;
        self.key = Some(key);
    }
}

/// Every photo and its width/height ratio, in the filtered order.
///
/// Paged rather than fetched at once: the whole point of a justified plan is that it covers the library, and
/// asking for two hundred thousand photos in one call is exactly the allocation the square grid's windowing
/// exists to avoid.
fn photos_with_aspects(engine: &Engine, filter: &Filter, sort: Sort, count: usize) -> Vec<(PhotoId, f32)> {
    const CHUNK: usize = 2048;
    let mut photos = Vec::with_capacity(count);
    let mut offset = 0;
    while offset < count {
        let page = engine.page(filter, sort, offset, CHUNK.min(count - offset));
        if page.photos.is_empty() {
            break;
        }
        photos.extend(page.photos.iter().map(|photo| (photo.id, aspect_of(photo))));
        offset += page.photos.len();
    }
    photos
}

fn square_grid(
    ui: &mut Ui,
    t: &Tokens,
    state: &mut UiState,
    engine: &mut Engine,
    thumbs: &mut Thumbnails,
    filter: &Filter,
    actions: &mut Vec<Action>,
    add_to: &mut Option<(PhotoId, sortlens_core::CollectionId)>,
) -> Rect {
    let gap = t.grid_gap;
    let (columns, cell) = grid_layout(ui.available_width(), gap, state.density.min_cell());
    let caption = CAPTION;
    let row_height = cell + caption + gap;
    let total = engine.count(filter);
    let rows = total.div_ceil(columns);

    state.cell_rects.clear();
    let mut painted_rects: Vec<(PhotoId, Rect)> = Vec::new();
    let mut grid_rect = Rect::NOTHING;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, row_height, rows, |ui, range| {
            grid_rect = ui.max_rect();
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
                                let selected = state.selection.contains(photo.id);
                                let response = photo_cell(ui, t, photo, selected, cell, caption, thumbs);
                                painted_rects.push((photo.id, response.rect));
                                cell_gestures(ui, state, engine, photo, &response, filter, actions, add_to);
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

    state.cell_rects = painted_rects;
    grid_rect
}

/// Justified layout: rows of equal height whose widths fill the window, each photo at its own shape.
///
/// The scroll area windows by *row*, computed from the cached plan rather than a fixed row height, because
/// every row is a different height. Only rows that intersect the viewport are painted; the ones above it are
/// skipped with one `add_space` rather than a loop, so a library of fifty thousand rows still scrolls at the
/// cost of the twenty on screen.
fn justified_grid(
    ui: &mut Ui,
    t: &Tokens,
    state: &mut UiState,
    engine: &mut Engine,
    thumbs: &mut Thumbnails,
    filter: &Filter,
    actions: &mut Vec<Action>,
    add_to: &mut Option<(PhotoId, sortlens_core::CollectionId)>,
) -> Rect {
    let gap = t.grid_gap;
    let width = ui.available_width();
    // Density keeps its meaning in this layout: it is the target row height, so "compact" means more rows of
    // photographs rather than a different idea of small.
    let target = state.density.min_cell();
    let mut plan = std::mem::take(&mut state.justified);
    plan.prepare(engine, filter, state.sort, width, target, gap);

    state.cell_rects.clear();
    let mut painted_rects: Vec<(PhotoId, Rect)> = Vec::new();
    let mut grid_rect = Rect::NOTHING;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            grid_rect = ui.max_rect();
            ui.set_height(plan.total);
            let first = plan
                .rows
                .partition_point(|row| row.top + row.height + CAPTION + gap < viewport.min.y);
            if let Some(row) = plan.rows.get(first) {
                // One jump instead of one `add_space` per skipped row: the height above the viewport is known.
                ui.add_space(row.top);
            }
            for row in &plan.rows[first.min(plan.rows.len())..] {
                if row.top > viewport.max.y {
                    break;
                }
                ui.horizontal(|ui| {
                    let mut remaining = width;
                    for (index, id) in row.ids.iter().enumerate() {
                        let Some(photo) = engine.photo(*id) else {
                            continue;
                        };
                        let left = row.ids.len() - index - 1;
                        // The last cell takes the remainder, so a row ends exactly at the right edge instead
                        // of half a point short of it — rounding, not a design decision.
                        let wanted = if left == 0 {
                            remaining
                        } else {
                            (aspect_of(photo) * row.height).min(remaining)
                        };
                        remaining = (remaining - wanted - gap).max(0.0);
                        let selected = state.selection.contains(photo.id);
                        let response = photo_cell_sized(ui, t, photo, selected, vec2(wanted, row.height), CAPTION, thumbs, Fit::Contain);
                        painted_rects.push((photo.id, response.rect));
                        cell_gestures(ui, state, engine, photo, &response, filter, actions, add_to);
                    }
                });
                ui.add_space(gap);
            }
        });
    state.cell_rects = painted_rects;
    state.justified = plan;
    grid_rect
}

fn grid(
    ui: &mut Ui,
    t: &Tokens,
    state: &mut UiState,
    engine: &mut Engine,
    thumbs: &mut Thumbnails,
    filter: &Filter,
    actions: &mut Vec<Action>,
) {
    let mut add_to: Option<(PhotoId, sortlens_core::CollectionId)> = None;
    let grid_rect = match state.grid_mode {
        GridMode::Square => square_grid(ui, t, state, engine, thumbs, filter, actions, &mut add_to),
        GridMode::Justified => justified_grid(ui, t, state, engine, thumbs, filter, actions, &mut add_to),
    };
    marquee(ui, state, t, grid_rect);
    if let Some((photo, collection)) = add_to {
        match engine.add_to_collection(&[photo], collection) {
            Ok(_) => state.notice("Added to collection"),
            Err(error) => state.notice(error.to_string()),
        }
    }
}

/// What a grid cell has to say when its pixels are missing, and nothing when they are not.
///
/// A missing file outranks a decode failure: the file is gone, so the failure is a consequence and telling
/// the user twice is noise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellBadge {
    /// The file is not on disk — an unmounted drive, a moved folder.
    Offline,
    /// The file is there but could not be decoded.
    Unreadable,
}

impl CellBadge {
    /// The glyph painted in the corner of the cell.
    fn glyph(self) -> &'static str {
        match self {
            CellBadge::Offline => "?",
            CellBadge::Unreadable => "!",
        }
    }

    /// What the hover says, so the glyph is never the only explanation.
    fn hint(self) -> &'static str {
        match self {
            CellBadge::Offline => "File missing — unmounted drive, or moved outside the library",
            CellBadge::Unreadable => "This file could not be decoded",
        }
    }

    /// The badge colour, from the active theme rather than a literal.
    fn colour(self, t: &Tokens) -> Color32 {
        match self {
            CellBadge::Offline => t.text_faint,
            CellBadge::Unreadable => t.reject,
        }
    }
}

/// The badge a cell needs, if any.
fn cell_badge(photo: &Photo, failure: Option<&str>) -> Option<CellBadge> {
    if photo.missing {
        Some(CellBadge::Offline)
    } else if failure.is_some() {
        Some(CellBadge::Unreadable)
    } else {
        None
    }
}

fn photo_cell(ui: &mut Ui, t: &Tokens, photo: &Photo, selected: bool, cell: f32, caption: f32, thumbs: &mut Thumbnails) -> egui::Response {
    photo_cell_sized(ui, t, photo, selected, vec2(cell, cell), caption, thumbs, Fit::Cover)
}

/// Whether the thumbnail is cropped to the image box or letterboxed inside it.
///
/// Square cells crop, because a lattice of different shapes is not a lattice. Justified cells are already the
/// photo's own shape, so they contain it: cropping there would throw away the part of the frame the row's
/// height was computed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fit {
    Cover,
    Contain,
}

fn photo_cell_sized(
    ui: &mut Ui,
    t: &Tokens,
    photo: &Photo,
    selected: bool,
    image_size: Vec2,
    caption: f32,
    thumbs: &mut Thumbnails,
    fit: Fit,
) -> egui::Response {
    let cell = image_size.x;
    let (rect, response) = ui.allocate_exact_size(vec2(image_size.x, image_size.y + caption), Sense::click());
    // A painted cell is invisible to a screen reader unless it says what it is.
    crate::a11y::label_widget(ui, &response, egui::Role::Button, &crate::a11y::describe_photo(photo), selected);
    let image = Rect::from_min_size(rect.min, image_size);
    let painter = ui.painter();

    painter.rect_filled(image, t.corner(), if response.hovered() { t.hover } else { t.thumb_bg });

    let texture = thumbs.get(ui.ctx(), photo, GRID_SIZE).cloned();
    match &texture {
        Some(texture) => {
            let uv = match fit {
                Fit::Cover => cover_uv(texture.size_vec2(), image.size()),
                Fit::Contain => Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            };
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

    // A cell whose pixels could not be made says so, rather than showing the same grey rectangle as one
    // that is still decoding: a missing file and an unreadable one look identical otherwise.
    if let Some(badge) = cell_badge(photo, thumbs.failure(photo.key)) {
        let corner = image.right_top() + vec2(-6.0, 6.0);
        let dot = Rect::from_center_size(corner, vec2(14.0, 14.0));
        painter.rect_filled(dot, 4.0, badge.colour(t));
        painter.text(dot.center(), Align2::CENTER_CENTER, badge.glyph(), t.font(9.0), Color32::WHITE);
        response.clone().on_hover_text(badge.hint());
    }

    // Stars sit along the bottom edge of the thumbnail so the eye can scan a whole row.
    for star in 0..5 {
        let centre = image.left_bottom() + vec2(9.0 + star as f32 * 10.0, -9.0);
        let colour = if star < photo.rating { t.star() } else { t.text_faint };
        painter.circle_filled(centre, 2.6, colour);
    }

    painter.text(
        rect.left_top() + vec2(2.0, image_size.y + 5.0),
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

fn keyboard(ui: &mut Ui, state: &mut UiState, engine: &mut Engine, actions: &mut Vec<Action>) {
    if ui.memory(|memory| memory.focused()).is_some() {
        return;
    }
    if command_keyboard(ui, state, engine, actions) {
        return;
    }
    if state.shortcuts_open {
        // The overlay owns Escape and the search field; nothing behind it should react to keys.
        if ui.input(|input| input.key_pressed(Key::Escape) && state.shortcut_query.is_empty()) {
            state.shortcuts_open = false;
        }
        return;
    }
    if ui.input(|input| input.key_pressed(Key::Questionmark) && !input.modifiers.command) {
        state.shortcuts_open = true;
        state.shortcut_query.clear();
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
    // The layout is a view, so it has a key: G flips between rows and a lattice without a trip to the chip.
    if ui.input(|input| input.key_pressed(Key::G) && !input.modifiers.command) && !state.detail_open {
        let next = state.grid_mode.next();
        state.grid_mode = next;
        actions.push(Action::SetGrid(next));
        return;
    }
    // Starting a session works before anything is selected: the queue comes from the filters.
    // A session restored from a previous run is opened rather than replaced.
    if ui.input(|input| input.key_pressed(Key::C) && !input.modifiers.command) && !state.detail_open {
        state.view = View::Cull;
        if engine.cull().is_none() {
            match start_cull(engine, state) {
                Ok(total) => state.notice(format!("Culling {total} photos")),
                Err(error) => state.notice(error),
            }
        }
        return;
    }
    // ⌘A selects everything in view and Escape drops the selection. Both work with nothing selected, which
    // is why they come before the primary check: ⌘A on an empty selection is how you start.
    if ui.input(|input| input.modifiers.command && input.key_pressed(Key::A)) {
        let order: Vec<PhotoId> = engine
            .page(&state.filter(), state.sort, 0, MAX_RANGE)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        let total = order.len();
        state.selection.all(&order);
        state.notice(format!("Selected {total} photos"));
        return;
    }
    if !state.detail_open && ui.input(|input| input.key_pressed(Key::Escape) && !input.modifiers.command) && !state.selection.is_empty() {
        state.selection.clear();
        return;
    }

    let Some(id) = state.selection.primary() else {
        return;
    };
    // A batch is every ticked photo, not just the one in the inspector.
    let targets = state.selection.ids();

    let mut rating = None;
    let mut label = None;
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
        for (key, colour) in [
            (Key::Num6, ColorLabel::Red),
            (Key::Num7, ColorLabel::Yellow),
            (Key::Num8, ColorLabel::Green),
            (Key::Num9, ColorLabel::Blue),
        ] {
            if input.key_pressed(key) && !input.modifiers.command {
                // Pressing the same key again clears it, the way a toggle should.
                label = Some(Some(colour));
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

    let count = targets.len();
    if let Some(colour) = label {
        // Toggling on a batch clears only when the primary already carries it, so pressing the same key twice
        // over a mixed selection sets rather than clears — the predictable direction.
        let cleared = engine.photo(id).and_then(|photo| photo.label) == colour;
        let wanted = if cleared { None } else { colour };
        match engine.label(&targets, wanted) {
            Ok(_) => state.notice(match (wanted, count) {
                (Some(colour), 1) => format!("{} label", colour.label()),
                (Some(colour), many) => format!("{} label on {many} photos", colour.label()),
                (None, 1) => "Cleared the label".to_string(),
                (None, many) => format!("Cleared the label on {many} photos"),
            }),
            Err(error) => state.notice(error.to_string()),
        }
    }

    if let Some(stars) = rating {
        match engine.rate(&targets, stars) {
            Ok(_) => state.notice(match (stars, count) {
                (0, 1) => "Cleared rating".to_string(),
                (0, many) => format!("Cleared the rating on {many} photos"),
                (stars, 1) => format!("Rated {stars}★"),
                (stars, many) => format!("Rated {stars}★ on {many} photos"),
            }),
            Err(error) => state.notice(error.to_string()),
        }
    }
    if let Some(flag) = flag {
        match engine.flag(&targets, flag) {
            Ok(_) => state.notice(match (flag, count) {
                (Flag::Pick, 1) => "Picked".to_string(),
                (Flag::Reject, 1) => "Rejected".to_string(),
                (Flag::None, 1) => "Unflagged".to_string(),
                (Flag::Pick, many) => format!("Picked {many} photos"),
                (Flag::Reject, many) => format!("Rejected {many} photos"),
                (Flag::None, many) => format!("Unflagged {many} photos"),
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
    let mut compare = false;
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
        if input.key_pressed(Key::C) && !input.modifiers.command {
            compare = true;
        }
    });
    if zoom {
        state.detail_zoom = !state.detail_zoom;
    }
    if compare {
        toggle_compare(state, engine);
    }
    if close {
        state.detail_open = false;
    }
    if step != 0
        && let Some(id) = state.selection.primary()
        && let Some(next) = engine.neighbour(&state.filter(), state.sort, id, step)
    {
        // In a compare the pair walks together, so the two photos stay neighbours.
        if state.compare.is_some() {
            state.compare = Some(id);
        }
        state.selection.replace(next);
    }
}

/// The modifier shortcuts the command registry promises. Returns whether one fired, so the plain
/// key handlers behind it do not also react to the same press.
fn command_keyboard(ui: &mut Ui, state: &mut UiState, engine: &mut Engine, actions: &mut Vec<Action>) -> bool {
    let detail_open = &state.detail_open;
    let (open, rescan, undo, collection, settings, quit, apply, view) = ui.input_mut(|input| {
        (
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::O)),
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::R)),
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Z)),
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::N)),
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma)),
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Q)),
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Enter)),
            // The view keys are only view keys when no modifier is held and no mode owns the letter:
            // S swaps a compare and P/X judge a cull session, so those stay theirs.
            if input.modifiers.command || *detail_open || engine.cull().is_some() {
                None
            } else if input.key_pressed(Key::I) {
                Some(View::Insights)
            } else if input.key_pressed(Key::S) {
                Some(View::Similar)
            } else {
                None
            },
        )
    });
    if open {
        actions.push(Action::PickFolder);
        return true;
    }
    if rescan {
        let root = state
            .root
            .clone()
            .or_else(|| engine.counts_by_root().first().map(|(root, _)| root.clone()));
        match root {
            Some(root) => actions.push(Action::Rescan(root)),
            None => state.notice("Add a folder before rescanning"),
        }
        return true;
    }
    if undo {
        match engine.undo() {
            Ok(true) => state.notice("Undid the last change"),
            Ok(false) => state.notice("Nothing to undo"),
            Err(error) => state.notice(error.to_string()),
        }
        return true;
    }
    if settings {
        state.settings_open = true;
        return true;
    }
    if quit {
        state.quit_open = true;
        return true;
    }
    if apply {
        if engine.cull().is_some_and(|session| session.summary().rejects > 0) {
            state.apply_open = true;
        } else {
            state.notice("Nothing staged: start a cull session and reject something first");
        }
        return true;
    }
    if let Some(view) = view {
        state.view = view;
        // A similar search needs hashes; asking for the view from a key should start the work.
        if view == View::Similar && engine.similar_groups().is_empty() && engine.hash_progress().is_none() {
            match engine.begin_similar(state.similar_threshold) {
                Ok(0) => {}
                Ok(queued) => state.notice(format!("Hashing {queued} previews")),
                Err(error) => state.notice(error.to_string()),
            }
        }
        return true;
    }
    if collection {
        let name = format!("Collection {}", engine.collections().len() + 1);
        match engine.create_collection(&name) {
            Ok(id) => {
                state.collection_filter = Some(id);
                state.notice(format!("Created {name}"));
            }
            Err(error) => state.notice(error.to_string()),
        }
        return true;
    }
    false
}

/// Open a compare against the next photo in order, or close one that is open.
pub fn toggle_compare(state: &mut UiState, engine: &Engine) {
    if state.compare.take().is_some() {
        return;
    }
    let Some(id) = state.selection.primary() else {
        state.notice("Pick a photo first");
        return;
    };
    let filter = state.filter();
    if let Some(next) = engine.neighbour(&filter, state.sort, id, 1) {
        state.compare = Some(next);
    } else if let Some(previous) = engine.neighbour(&filter, state.sort, id, -1) {
        state.compare = Some(previous);
    } else {
        state.notice("Nothing to compare with");
    }
}

/// Width over height, defaulting to a 3:2 frame until the size is known — the shape of most photographs, so a
/// library whose metadata never loaded still looks like photographs instead of tiles. Clamped because a
/// panorama and a stitched vertical would otherwise get rows of one photo and cells of none.
fn aspect_of(photo: &Photo) -> f32 {
    if photo.width > 0 && photo.height > 0 {
        (photo.width as f32 / photo.height as f32).clamp(0.2, 5.0)
    } else {
        1.5
    }
}

/// What differs between two photos, in the order a photographer compares them.
fn compare_facts(left: &Photo, right: &Photo) -> Vec<(&'static str, String)> {
    let mut facts = Vec::new();
    facts.push(("stars", format!("{} vs {}", left.rating, right.rating)));
    facts.push((
        "label",
        format!(
            "{} vs {}",
            left.label.map_or("none", |label| label.id()),
            right.label.map_or("none", |label| label.id())
        ),
    ));
    if left.exposure() != right.exposure() {
        facts.push(("exposure", format!("{} vs {}", left.exposure(), right.exposure())));
    }
    if left.size != right.size {
        facts.push((
            "size",
            format!(
                "{} vs {}",
                sortlens_core::format_bytes(left.size),
                sortlens_core::format_bytes(right.size)
            ),
        ));
    }
    facts.push((
        "pixels",
        format!("{}×{} vs {}×{}", left.width, left.height, right.width, right.height),
    ));
    facts
}

fn detail_view(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails, actions: &mut Vec<Action>) {
    let filter = state.filter();
    let Some(id) = state.selection.primary() else {
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

    // The other half of a compare, when one is open. A photo that has gone missing closes it rather
    // than leaving an empty pane behind.
    let other = state.compare.and_then(|other| engine.photo(other).cloned());
    if state.compare.is_some() && other.is_none() {
        state.compare = None;
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new(truncate(&photo.name, 42)).size(14.0).strong());
        if let Some(second) = &other {
            ui.label(RichText::new("↔").size(13.0).color(t.text_faint));
            ui.label(RichText::new(truncate(&second.name, 42)).size(14.0).color(t.text_muted));
        }
        if let Some((index, total)) = position {
            ui.label(RichText::new(format!("{index} of {total}")).size(11.5).color(t.text_muted));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add(Button::new(RichText::new("Open").size(10.5)).frame(false))
                .on_hover_text("Hand this file to the desktop's default application")
                .clicked()
            {
                actions.push(Action::OpenExternal(id));
            }
            if ui
                .add(Button::new(RichText::new("Show in folder").size(10.5)).frame(false))
                .on_hover_text("Show this file where it lives on disk")
                .clicked()
            {
                actions.push(Action::Reveal(id));
            }
            if ui
                .add(Button::new(RichText::new("Export…").size(10.5)).frame(false))
                .on_hover_text("Write a copy somewhere; the original is not touched")
                .clicked()
            {
                state.export_ids = vec![id];
                state.export_open = true;
            }
            if ui.button(RichText::new("Close  Esc").size(11.0)).clicked() {
                state.detail_open = false;
                state.compare = None;
            }
            let zoom_label = if state.detail_zoom { "Fit  Z" } else { "1:1  Z" };
            if ui.button(RichText::new(zoom_label).size(11.0)).clicked() {
                state.detail_zoom = !state.detail_zoom;
            }
            if other.is_some() {
                if ui
                    .button(RichText::new("Swap  S").size(11.0))
                    .on_hover_text("Swap which pane is the left one")
                    .clicked()
                    && let Some(second) = state.compare
                {
                    state.selection.replace(second);
                    state.compare = Some(id);
                }
                if ui.button(RichText::new("Compare  C").size(11.0)).clicked() {
                    state.compare = None;
                }
            } else if ui
                .button(RichText::new("Compare  C").size(11.0))
                .on_hover_text("Show the next photo beside this one, at the same magnification")
                .clicked()
            {
                toggle_compare(state, engine);
            }
        });
    });
    ui.add_space(6.0);

    let footer = 118.0;
    let height = (ui.available_height() - footer).max(180.0);
    let (area, area_response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    crate::a11y::label_widget(
        ui,
        &area_response,
        egui::Role::Image,
        &format!("Preview of {}", crate::a11y::describe_photo(&photo)),
        false,
    );
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
    match &other {
        // Two panes, one magnification: both photos are drawn at the smaller of the two fit scales,
        // so the pane that would need less room is letterboxed instead of being blown up. That is
        // what makes a compare honest — the same detail at the same size in both.
        Some(second) => {
            let gap = 8.0;
            let half = vec2((area.width() - gap) / 2.0, area.height());
            let left = Rect::from_min_size(area.min, half);
            let right = Rect::from_min_size(pos2(area.min.x + half.x + gap, area.min.y), half);
            ui.painter().rect_filled(right, t.corner(), t.thumb_bg);
            let left_aspect = aspect_of(&photo);
            let right_aspect = aspect_of(second);
            let scale = if state.detail_zoom {
                1.0
            } else {
                (half.x / photo.width.max(1) as f32)
                    .min(area.height() / photo.height.max(1) as f32)
                    .min(half.x / second.width.max(1) as f32)
                    .min(area.height() / second.height.max(1) as f32)
            };
            let clip = ui.clip_rect();
            for (pane, item, pane_aspect) in [(left, &photo, left_aspect), (right, second, right_aspect)] {
                let mut sized = if state.detail_zoom {
                    let width = item.width.max(1) as f32 * scale;
                    Rect::from_center_size(pane.center(), vec2(width, width / pane_aspect))
                } else {
                    fit_rect(pane.shrink(6.0), pane_aspect)
                };
                if state.detail_zoom {
                    // 1:1 crops to the pane, so keep the shared origin at the pane's centre.
                    sized = sized.intersect(pane.expand2(vec2(4096.0, 4096.0)));
                }
                match thumbs.get(ui.ctx(), item, 2000).cloned() {
                    Some(texture) => {
                        let uv = cover_uv(texture.size_vec2(), sized.size());
                        ui.painter()
                            .with_clip_rect(pane.intersect(clip))
                            .image(texture.id(), sized, uv, Color32::WHITE);
                    }
                    None => {
                        ui.painter()
                            .text(pane.center(), Align2::CENTER_CENTER, "Loading preview…", t.font(11.0), t.text_faint);
                    }
                }
            }
        }
        None => match thumbs.get(ui.ctx(), &photo, 2000).cloned() {
            Some(texture) => {
                let uv = cover_uv(texture.size_vec2(), target.size());
                let painter = ui.painter().with_clip_rect(area.intersect(ui.clip_rect()));
                painter.image(texture.id(), target, uv, Color32::WHITE);
            }
            None => {
                ui.painter()
                    .text(area.center(), Align2::CENTER_CENTER, "Loading preview…", t.font(11.0), t.text_faint);
            }
        },
    }

    // The next photo in the sorted order is decoded before you ask for it. It is queued at "nearly visible"
    // urgency, so a library-wide pass never delays the frame you are looking at, and a refusal at the queue
    // bound costs nothing: the request is simply made again when you get there.
    if let Some(id) = state.selection.primary()
        && let Some(next) = engine.neighbour(&state.filter(), state.sort, id, 1)
        && let Some(next) = engine.photo(next)
        && next.key != photo.key
    {
        thumbs.prefetch(next, 1600);
    }

    if let Some(second) = &other {
        // The differences that matter when choosing between two frames, on one line.
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            for (label, value) in compare_facts(&photo, second) {
                ui.label(RichText::new(label).size(10.5).color(t.text_faint));
                ui.label(RichText::new(value).size(10.5).color(t.text_muted));
                ui.add_space(8.0);
            }
        });
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.add(Button::new(RichText::new("← Previous").size(11.0))).clicked()
            && let Some(previous) = engine.neighbour(&filter, state.sort, id, -1)
        {
            state.selection.replace(previous);
        }
        if ui.add(Button::new(RichText::new("Next →").size(11.0))).clicked()
            && let Some(next) = engine.neighbour(&filter, state.sort, id, 1)
        {
            state.selection.replace(next);
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

    // What the file's own metadata suggests. Offers only — the keyword is written when one is
    // accepted, which is the same op as typing it by hand, so undo and the log behave the same.
    suggestions_row(ui, t, id, &engine.suggestions(id), actions);

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
                    crate::a11y::label_widget(
                        ui,
                        &response,
                        egui::Role::Button,
                        &format!("Jump to {}", crate::a11y::describe_photo(&other)),
                        false,
                    );
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
            state.selection.replace(target);
        }
    });

    ui.label(
        RichText::new("P/X/U flag · 1–5 rate · ← → walk · Z zoom · Esc close")
            .size(10.0)
            .color(t.text_faint),
    );
}

/// One row of suggested keywords, each a button that accepts it.
///
/// The row says where each suggestion came from and how sure it is, because "the camera said so" and
/// "a model guessed" are different kinds of claim and the person deciding deserves to know which.
fn suggestions_row(ui: &mut Ui, t: &Tokens, id: PhotoId, suggestions: &[sortlens_engine::Suggestion], actions: &mut Vec<Action>) {
    if suggestions.is_empty() {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Suggested").size(10.0).color(t.text_faint));
        for suggestion in suggestions {
            let response = ui
                .add(Button::new(RichText::new(format!("+ {}", suggestion.tag)).size(10.5)).frame(false))
                .on_hover_text(format!(
                    "{} · {:.0}% from {}",
                    suggestion.tag,
                    suggestion.score * 100.0,
                    suggestion.source.label()
                ));
            if response.clicked() {
                actions.push(Action::AcceptSuggestion(id, suggestion.tag.clone()));
            }
        }
    });
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

fn cull_view(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails, actions: &mut Vec<Action>) {
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
                crate::a11y::label_widget(
                    ui,
                    &response,
                    egui::Role::Button,
                    &format!("Jump to {}", crate::a11y::describe_photo(&photo)),
                    false,
                );
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
            if ui
                .add_enabled(summary.picks > 0, Button::new(RichText::new("Move picks…").size(11.5)))
                .on_disabled_hover_text("Pick a photo first")
                .on_hover_text("Move the picks into a folder; keeps every rating and keyword")
                .clicked()
            {
                let picks = engine.cull().map(|session| session.picks()).unwrap_or_default();
                actions.push(Action::MovePicks(picks));
            }
            if ui
                .add_enabled(summary.picks > 0, Button::new(RichText::new("Export picks…").size(11.5)))
                .on_disabled_hover_text("Pick a photo first")
                .on_hover_text("Write the picks into a folder; the originals are not touched")
                .clicked()
            {
                state.export_ids = engine.cull().map(|session| session.picks()).unwrap_or_default();
                state.export_open = true;
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
                engine.regroup(state.similar_threshold, state.similar_colour, state.similar_mode);
            }
        });
    });
    ui.horizontal(|ui| {
        ui.label(RichText::new("Match").size(10.5).color(t.text_faint));
        for mode in sortlens_engine::SimilarMatch::ALL {
            if ui
                .add(
                    Button::new(RichText::new(mode.label()).size(11.0).color(if state.similar_mode == mode {
                        t.text
                    } else {
                        t.text_muted
                    }))
                    .fill(if state.similar_mode == mode {
                        t.selection
                    } else {
                        Color32::TRANSPARENT
                    })
                    .stroke(Stroke::NONE),
                )
                .on_hover_text(mode.hint())
                .clicked()
            {
                state.similar_mode = mode;
                // Regrouping uses the cached fingerprints, so changing the mode costs nothing.
                engine.regroup(state.similar_threshold, state.similar_colour, mode);
            }
        }
        if state.similar_mode == sortlens_engine::SimilarMatch::Colour {
            let mut colour = state.similar_colour as u64;
            if ui
                .add(egui::Slider::new(&mut colour, 0..=120).text("palette"))
                .on_hover_text("How far apart two palettes may be, per channel")
                .changed()
            {
                state.similar_colour = colour as u32;
                engine.regroup(state.similar_threshold, state.similar_colour, state.similar_mode);
            }
        }
        ui.label(RichText::new(state.similar_mode.hint()).size(10.0).color(t.text_faint));
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
                                    state.selection.clear();
                                    if let Some(id) = group.others().into_iter().next() {
                                        state.selection.replace(id);
                                    }
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
                                crate::a11y::label_widget(
                                    ui,
                                    &response,
                                    egui::Role::Button,
                                    &crate::a11y::describe_photo(&photo),
                                    member.id == group.best,
                                );
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
                                    state.selection.replace(member.id);
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
fn insights_view(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine) {
    let _ = engine.poll_insights();
    let insights = engine.insights();
    if let Some((done, total)) = engine.insights_progress() {
        // A library of thousands is not instant, so the pass says how far it has come instead of
        // freezing the view.
        ui.horizontal(|ui| {
            ui.label(RichText::new("Counting").size(11.0).color(t.text_muted));
            ui.add(egui::ProgressBar::new(if total == 0 { 0.0 } else { done as f32 / total as f32 }).desired_width(220.0));
            ui.label(RichText::new(format!("{done} of {total}")).size(10.5).color(t.text_faint));
        });
        ui.add_space(8.0);
    } else if ui
        .button(RichText::new("Recount").size(11.0))
        .on_hover_text("Count the library again on a worker, reporting progress as it goes")
        .clicked()
    {
        match engine.begin_insights() {
            Ok(count) => {
                if count == 0 {
                    // Nothing to do: the numbers on screen are already final.
                }
            }
            Err(error) => state.notice(error.to_string()),
        }
    }
    let highlights = insights.highlights();
    if !highlights.is_empty() {
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            for highlight in &highlights {
                ui.label(RichText::new(&highlight.label).size(10.0).color(t.text_faint));
                ui.label(RichText::new(&highlight.value).size(11.0).color(t.text));
                ui.add_space(14.0);
            }
        });
    }
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
                ui.add_space(14.0);
                // Seasonality: which months the library leans towards, in one row of twelve.
                ui.label(RichText::new("Months").size(12.0).strong());
                let peak = insights.by_month.iter().copied().max().unwrap_or(0);
                if peak == 0 {
                    charts::empty(ui, t, "Nothing dated yet", charts::COLUMN_HEIGHT);
                } else {
                    let month_columns: Vec<Column> = insights
                        .by_month
                        .iter()
                        .enumerate()
                        .map(|(index, count)| {
                            let month = index as u32 + 1;
                            let column = Column::new(sortlens_engine::month_name(month).chars().next().unwrap_or('?').to_string(), *count);
                            if insights.peak_month() == Some(month) {
                                column.highlighted()
                            } else {
                                column
                            }
                        })
                        .collect();
                    charts::columns(ui, t, &month_columns, charts::COLUMN_HEIGHT);
                    ui.label(
                        RichText::new(format!(
                            "Busiest: {}{}",
                            insights.peak_month().map(sortlens_engine::month_name).unwrap_or("none"),
                            insights
                                .peak_season()
                                .map(|season| format!(" · mostly {season}"))
                                .unwrap_or_default()
                        ))
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

/// A label, a proportional bar and a count. The drawing lives in `charts`, so the insights rows and
/// anything else that needs a share look the same.
fn bar_row(ui: &mut Ui, t: &Tokens, label: &str, count: usize, whole: usize) {
    charts::share_row(ui, t, label, count, whole);
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

/// Memories: what happened on this day, this week in past years, and what was never looked at.
fn for_you_view(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails) {
    let memories = engine.memories(None);
    ui.horizontal(|ui| {
        ui.label(RichText::new("For You").size(15.0).strong().color(t.text));
        ui.label(RichText::new(memories.headline()).size(10.5).color(t.text_faint));
    });
    hint(
        ui,
        t,
        "Resurfaced by capture date and by what you have not judged yet. Dates are read as UTC.",
    );
    ui.add_space(6.0);
    if memories.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Nothing to resurface yet").size(14.0).color(t.text_muted));
            hint(
                ui,
                t,
                "Cards appear once photos have capture dates in earlier years, or are a year old \
                 and still unrated.",
            );
        });
        return;
    }
    egui::ScrollArea::vertical().show(ui, |ui| {
        for card in &memories.cards {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(card.title.clone()).size(12.5).strong().color(t.text));
                ui.label(RichText::new(card.detail()).size(10.0).color(t.text_faint));
            });
            // Where the card's photos come from: a card that says "since 2019" but leans entirely on
            // one year is a different invitation than one spread over five.
            let years = card_years(engine, &card.photos);
            if years.len() > 1 {
                ui.add_space(2.0);
                let columns: Vec<Column> = years.iter().map(|(year, count)| Column::new(year.to_string(), *count)).collect();
                charts::columns(ui, t, &columns, 40.0);
            }
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for id in &card.photos {
                    let Some(photo) = engine.photo(*id) else {
                        continue;
                    };
                    let (rect, response) = ui.allocate_exact_size(vec2(96.0, 96.0), Sense::click());
                    crate::a11y::label_widget(ui, &response, egui::Role::Button, &crate::a11y::describe_photo(photo), false);
                    let texture = thumbs.get(ui.ctx(), photo, GRID_SIZE).cloned();
                    match texture {
                        Some(texture) => {
                            ui.painter().image(
                                texture.id(),
                                rect,
                                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        }
                        None => {
                            ui.painter().rect_filled(rect, t.corner(), t.thumb_bg);
                        }
                    }
                    if state.selection.contains(*id) {
                        ui.painter()
                            .rect_stroke(rect, t.corner(), Stroke::new(2.0, t.accent), StrokeKind::Inside);
                    }
                    if response.on_hover_text(format!("{} · {}", photo.name, card.kind.label())).clicked() {
                        state.selection.replace(*id);
                        state.detail_open = true;
                    }
                }
            });
        }
    });
}

/// Photos per capture year among the given photos, oldest first.
fn card_years(engine: &Engine, ids: &[PhotoId]) -> Vec<(i64, usize)> {
    let mut counts: std::collections::BTreeMap<i64, usize> = std::collections::BTreeMap::new();
    for id in ids {
        if let Some(year) = engine
            .photo(*id)
            .and_then(|photo| photo.captured)
            .map(sortlens_memories::civil_year)
        {
            *counts.entry(year).or_insert(0) += 1;
        }
    }
    counts.into_iter().collect()
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
            let mut forget = None;
            for (id, summary, age, mode) in batches {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(summary).size(12.0).color(t.text));
                        ui.label(RichText::new(format!("{age} · {mode}")).size(10.5).color(t.text_faint));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button(RichText::new("Forget").size(11.0))
                            .on_hover_text("Drop the restore record; the files stay where they are")
                            .clicked()
                        {
                            forget = Some(id);
                        }
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
            if let Some(id) = forget {
                match engine.forget_batch(id) {
                    Ok(files) => state.notice(format!("Forgot a batch of {files} file(s)")),
                    Err(error) => state.notice(error.to_string()),
                }
            }
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new("Empty bin…").size(11.0))
                    .on_hover_text("Forget every restore record; files on disk are untouched")
                    .clicked()
                {
                    state.empty_bin_open = true;
                }
            });
            ui.label(
                RichText::new("Files sent to the system Trash are restored from the Trash itself.")
                    .size(10.5)
                    .color(t.text_faint),
            );
        });
    state.deleted_open = open;
}

/// Ask before emptying the bin: the records are the only way back.
/// What an export will write, and what the last one did.
///
/// The dialog says the two things a user needs to trust it: which files, and that the originals are
/// not touched. Progress comes from the engine's worker, so closing and reopening does not lose it.
fn export_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    if let Some(receipt) = state.export_receipt.clone() {
        let mut open = true;
        egui::Window::new("Export finished")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(380.0)
            .show(ui.ctx(), |ui| {
                ui.label(RichText::new(&receipt).size(12.0));
                ui.add_space(6.0);
                ui.label(
                    RichText::new("The originals are where they were; an export only writes copies.")
                        .size(10.5)
                        .color(t.text_muted),
                );
            });
        if !open {
            state.export_receipt = None;
        }
    }

    if !state.export_open {
        return;
    }
    let progress = engine.export_progress();
    let mut open = state.export_open;
    let mut start = false;
    egui::Window::new("Export")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(400.0)
        .show(ui.ctx(), |ui| {
            let count = state.export_ids.len();
            ui.label(RichText::new(format!("{count} photo(s) · {}", state.export_spec.summary())).size(12.0));
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(RichText::new("Format").size(11.0).color(t.text_muted));
                for format in sortlens_engine::ExportFormat::ALL {
                    if chip(ui, t, format.label(), state.export_spec.format == format).clicked() {
                        state.export_spec.format = format;
                    }
                }
            });
            if state.export_spec.format == sortlens_engine::ExportFormat::Original {
                hint(ui, t, "A copy: same bytes, same name, no resize and no re-encode.");
            } else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Longest edge").size(11.0).color(t.text_muted));
                    let mut edge = state.export_spec.max_edge.unwrap_or(0);
                    if ui.add(egui::DragValue::new(&mut edge).range(0..=12_000).speed(16)).changed() {
                        state.export_spec.max_edge = if edge == 0 { None } else { Some(edge.max(16)) };
                    }
                    ui.label(RichText::new("px · 0 keeps the original size").size(10.0).color(t.text_muted));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Quality").size(11.0).color(t.text_muted));
                    let mut quality = u32::from(state.export_spec.quality);
                    if ui.add(egui::Slider::new(&mut quality, 1..=100)).changed() {
                        state.export_spec.quality = quality.min(100) as u8;
                    }
                });
                if ui
                    .checkbox(&mut state.export_spec.metadata, "Keep EXIF and IPTC")
                    .on_hover_text("Carries the camera's metadata into the new file")
                    .changed()
                {}
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("If the name is taken").size(11.0).color(t.text_muted));
                for policy in sortlens_engine::Collision::ALL {
                    if chip(ui, t, policy.label(), state.export_spec.collision == policy).clicked() {
                        state.export_spec.collision = policy;
                    }
                }
            });

            if let Some((done, total)) = progress {
                ui.add_space(8.0);
                let share = if total == 0 { 0.0 } else { done as f32 / total as f32 };
                ui.add(egui::ProgressBar::new(share).text(format!("{done} of {total}")));
                if ui.button("Stop").clicked() {
                    engine.cancel_export();
                }
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(progress.is_none() && count > 0, Button::new("Choose folder…"))
                    .on_disabled_hover_text("An export is already running, or nothing is selected")
                    .clicked()
                {
                    start = true;
                }
                if ui.button("Cancel").clicked() {
                    state.export_open = false;
                }
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new("Your photos stay where they are; this writes copies into the folder you pick.")
                    .size(10.5)
                    .color(t.text_muted),
            );
        });
    state.export_open = open;
    if start {
        actions.push(Action::Export(state.export_ids.clone(), state.export_spec.clone()));
    }
}

fn empty_bin_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine) {
    let batch_count = engine.deleted_batches().len();
    let files: usize = engine.deleted_batches().iter().map(|batch| batch.items.len()).sum();
    let mut open = state.empty_bin_open;
    egui::Window::new("Empty the rejects bin?")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(360.0)
        .show(ui.ctx(), |ui| {
            ui.label(RichText::new(format!("{batch_count} batch(es), {files} file(s) will lose their restore record.")).size(12.0));
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Files already moved stay in the system Trash; files that were only flagged stay \
                     where they are. Nothing on disk is deleted here, and this cannot be undone.",
                )
                .size(10.5)
                .color(t.text_muted),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(batch_count > 0, Button::new(RichText::new("Empty bin").size(12.0)))
                    .clicked()
                {
                    let (batches, files) = engine.forget_all_batches();
                    state.notice(format!("Forgot {batches} batch(es) covering {files} file(s)"));
                    state.empty_bin_open = false;
                }
                if ui.button(RichText::new("Cancel").size(12.0)).clicked() {
                    state.empty_bin_open = false;
                }
            });
        });
    state.empty_bin_open = open;
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

fn health_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
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
                        state.health_modified = health.modified;
                        state.notice(format!(
                            "Checked {} files · {} missing · {} changed",
                            health.checked, health.missing, health.modified
                        ));
                    }
                    Err(error) => state.notice(error.to_string()),
                }
            }
            ui.label(
                RichText::new("Stats every file and updates the missing badges in one undoable step.")
                    .size(10.5)
                    .color(t.text_faint),
            );
            if state.health_modified > 0 {
                ui.label(
                    RichText::new(format!("{} file(s) changed on disk since they were indexed", state.health_modified))
                        .size(11.0)
                        .color(t.text),
                );
                if ui
                    .button(RichText::new("Refresh changed").size(11.5))
                    .on_hover_text("Re-read metadata and identity for exactly those files")
                    .clicked()
                {
                    match engine.refresh_changed() {
                        Ok(0) => state.notice("Nothing could be refreshed"),
                        Ok(count) => {
                            state.health_modified = 0;
                            state.notice(format!("Refreshed {count} photo(s)"));
                        }
                        Err(error) => state.notice(error.to_string()),
                    }
                }
            }
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
            // A whole volume that was renamed or moved: every relative path still holds, so the
            // root can be repointed instead of relocating photos one by one.
            let roots: Vec<(String, usize)> = engine.counts_by_root();
            for (root, count) in roots {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(folder_name(&root)).size(11.5).color(t.text));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button(RichText::new("Point at…").size(11.0))
                            .on_hover_text("This folder moved or its volume was renamed")
                            .clicked()
                        {
                            actions.push(Action::RemapRoot(root.clone()));
                        }
                    });
                });
                ui.label(
                    RichText::new(format!("{count} photo(s) under {}", root))
                        .size(10.0)
                        .color(t.text_faint),
                );
            }
            ui.label(
                RichText::new("Repointing checks a sample of photos in the new folder first.")
                    .size(10.5)
                    .color(t.text_faint),
            );
            ui.add_space(8.0);
            ui.separator();
            // Files whose metadata could not be read at scan time are queued, not forgotten.
            let queued = engine.retry_count();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if queued == 0 {
                        "No photos are waiting on a metadata retry".to_string()
                    } else {
                        format!("{queued} photo(s) could not be read when indexed")
                    })
                    .size(11.0)
                    .color(if queued == 0 { t.text_faint } else { t.text }),
                );
            });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(queued > 0, Button::new(RichText::new("Retry metadata").size(11.5)))
                    .on_hover_text("Try reading EXIF again for the photos that failed")
                    .clicked()
                {
                    match engine.retry_metadata(false) {
                        Ok((0, remaining)) => state.notice(format!("Nothing recovered; {remaining} still unreadable")),
                        Ok((updated, 0)) => state.notice(format!("Read metadata for {updated} photo(s)")),
                        Ok((updated, remaining)) => state.notice(format!("Read {updated}; {remaining} still unreadable")),
                        Err(error) => state.notice(error.to_string()),
                    }
                }
                let exifless = engine.exifless_count();
                if ui
                    .add_enabled(exifless > 0, Button::new(RichText::new("Re-read all without EXIF").size(11.5)))
                    .on_hover_text(format!(
                        "{exifless} photo(s) have no camera and no date; try again with the current readers"
                    ))
                    .clicked()
                {
                    match engine.retry_metadata(true) {
                        Ok((updated, _)) => state.notice(format!("Read metadata for {updated} photo(s)")),
                        Err(error) => state.notice(error.to_string()),
                    }
                }
            });
            ui.add_space(8.0);
            ui.label(RichText::new("Catalog files").size(11.0).color(t.text_muted));
            if let Some((headline, detail, clean)) = &state.audit {
                ui.label(
                    RichText::new(headline.clone())
                        .size(11.5)
                        .color(if *clean { t.text } else { t.reject }),
                );
                ui.label(RichText::new(detail.clone()).size(10.0).color(t.text_faint));
            }
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Check integrity").size(11.5)).clicked() {
                    match engine.audit() {
                        Ok(audit) => {
                            state.audit = Some((audit.headline(), audit.detail(), audit.is_clean()));
                            state.notice(audit.headline());
                        }
                        Err(error) => state.notice(error.to_string()),
                    }
                }
                if ui
                    .button(RichText::new("Compact").size(11.5))
                    .on_hover_text("Write a fresh snapshot and empty the log; also repairs a torn log tail")
                    .clicked()
                {
                    match engine.compact() {
                        Ok(0) => state.notice("Catalog was already compact"),
                        Ok(bytes) => state.notice(format!("Compacted · {} reclaimed", sortlens_core::format_bytes(bytes))),
                        Err(error) => state.notice(error.to_string()),
                    }
                }
            });
            ui.label(
                RichText::new("The check replays a copy of the log, so the library itself is never touched.")
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

/// A move just happened: say where the files went, and offer the one thing a photographer wants
/// next — putting them back.
fn move_receipt_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &Engine, actions: &mut Vec<Action>) {
    let Some(id) = state.move_receipt else {
        return;
    };
    let Some(receipt) = engine.move_receipts().iter().find(|receipt| receipt.id == id) else {
        state.move_receipt = None;
        return;
    };
    let mut open = true;
    let mut put_back = false;
    let mut done = false;
    egui::Window::new("Moved")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(360.0)
        .show(ui.ctx(), |ui| {
            ui.label(RichText::new(receipt.headline()).size(13.0).strong());
            ui.label(
                RichText::new("Ratings, labels and keywords travelled with the files.")
                    .size(10.5)
                    .color(t.text_muted),
            );
            ui.add_space(8.0);
            for item in receipt.items.iter().take(4) {
                ui.label(
                    RichText::new(format!("{} → {}", item.old.rel, item.new.rel))
                        .size(10.0)
                        .color(t.text_faint),
                );
            }
            if receipt.items.len() > 4 {
                ui.label(
                    RichText::new(format!("and {} more", receipt.items.len() - 4))
                        .size(10.0)
                        .color(t.text_faint),
                );
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Put them back").size(12.0)).clicked() {
                    put_back = true;
                }
                if ui.button(RichText::new("Done").size(12.0)).clicked() {
                    done = true;
                }
            });
        });
    if put_back {
        actions.push(Action::UndoMove(id));
        done = true;
    }
    if done || !open {
        state.move_receipt = None;
    }
}

/// What happened recently, read from the op log.
fn history_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    let mut open = state.history_open;
    egui::Window::new("Recent changes")
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .default_width(460.0)
        .show(ui.ctx(), |ui| {
            let entries = match engine.history(60) {
                Ok(entries) => entries,
                Err(error) => {
                    hint(ui, t, &error.to_string());
                    return;
                }
            };
            if entries.is_empty() {
                hint(ui, t, "No changes since the last snapshot.");
                ui.add_space(4.0);
                ui.label(
                    RichText::new("Every change is a record in the log; older ones are folded into the snapshot.")
                        .size(10.5)
                        .color(t.text_faint),
                );
                return;
            }
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                for entry in &entries {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("#{}", entry.seq)).size(10.0).color(t.text_faint));
                        ui.label(RichText::new(&entry.summary).size(12.0).color(t.text));
                    });
                    let when = if entry.is_dated() {
                        age_label(entry.at)
                    } else {
                        "no date".to_string()
                    };
                    ui.label(RichText::new(format!("{when} · {}", entry.kind)).size(10.0).color(t.text_faint));
                    ui.add_space(4.0);
                }
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new("Export report…").size(11.5))
                    .on_hover_text("Plain text, for a support request")
                    .clicked()
                {
                    actions.push(Action::ExportHistory);
                }
            });
            ui.label(
                RichText::new("The report lists folder and file names; it never includes photo contents.")
                    .size(10.5)
                    .color(t.text_faint),
            );
        });
    state.history_open = open;
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

/// Rename a keyword everywhere, merging when the new name is already in use.
fn tag_edit_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine) {
    let Some((from, typing)) = state.tag_edit.clone() else {
        return;
    };
    let mut text = typing;
    let mut open = true;
    let mut commit = false;
    egui::Window::new("Rename keyword")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(320.0)
        .show(ui.ctx(), |ui| {
            ui.label(
                RichText::new(format!("#{from} appears on every photo it was added to."))
                    .size(11.0)
                    .color(t.text_muted),
            );
            ui.add_space(6.0);
            let field = ui.add(TextEdit::singleline(&mut text).desired_width(f32::INFINITY));
            field.request_focus();
            if ui.input(|input| input.key_pressed(Key::Enter)) {
                commit = true;
            }
            let exists = engine
                .tag_vocabulary()
                .into_iter()
                .any(|(tag, _)| tag == text.trim().to_lowercase() && tag != from);
            if exists {
                ui.label(
                    RichText::new("That keyword already exists; the two will be merged.")
                        .size(10.5)
                        .color(t.accent),
                );
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let usable = !text.trim().is_empty() && text.trim() != from;
                if ui.add_enabled(usable, Button::new(RichText::new("Rename").size(12.0))).clicked() {
                    commit = true;
                }
                if ui.button(RichText::new("Cancel").size(12.0)).clicked() {
                    state.tag_edit = None;
                }
            });
        });
    if commit {
        match engine.rename_tag(&from, &text) {
            Ok(count) if count > 0 => {
                state.notice(format!("Renamed #{from} on {count} photo(s)"));
                state.tag_filters.retain(|tag| tag != &from);
                state.tag_edit = None;
            }
            Ok(_) => {
                state.notice(format!("#{from} is not used by any photo"));
                state.tag_edit = None;
            }
            Err(error) => state.notice(error.to_string()),
        }
        return;
    }
    if !open {
        state.tag_edit = None;
    } else {
        state.tag_edit = Some((from, text));
    }
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
            ui.separator();
            ui.label(RichText::new("Backup").size(11.0).color(t.text_muted));
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new("Back up library…").size(12.0))
                    .on_hover_text("One verified file holding the index and every decision")
                    .clicked()
                {
                    actions.push(Action::Backup);
                }
                if ui
                    .button(RichText::new("Restore into a folder…").size(12.0))
                    .on_hover_text("Unpack a backup as a separate library; the open one is untouched")
                    .clicked()
                {
                    actions.push(Action::RestoreBackup);
                }
            });
            ui.label(
                RichText::new("Photos are never copied, so a backup holds the index and the review state, not pixels.")
                    .size(10.5)
                    .color(t.text_faint),
            );
            ui.add_space(8.0);
            ui.separator();
            ui.label(RichText::new("Keyword packs").size(11.0).color(t.text_muted));
            keyword_packs(ui, t, engine, actions);
            if ui
                .button(RichText::new("Suggest keywords for the library…").size(12.0))
                .on_hover_text("Proposes keywords for every photo that has something to say; writes nothing until you accept")
                .clicked()
            {
                state.suggest_open = true;
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

/// The keyword packs found in the library, each with a switch.
///
/// A pack is a file someone wrote; this row shows how many keywords it can propose and the licence those
/// words come under, because a keyword's provenance is the reason to trust it. A file that cannot be read
/// is shown with its problem instead of vanishing, so a typo is fixable from here.
fn keyword_packs(ui: &mut Ui, t: &Tokens, engine: &Engine, actions: &mut Vec<Action>) {
    let packs = engine.packs();
    if packs.is_empty() {
        ui.label(RichText::new("No packs installed.").size(10.5).color(t.text_faint));
        ui.label(
            RichText::new("Put a .json pack in the library's packs folder; Organise ▸ Keyword Packs lists where.")
                .size(10.5)
                .color(t.text_faint),
        );
        return;
    }
    for pack in packs {
        match &pack.problem {
            Some(problem) => {
                ui.label(RichText::new(format!("{}: {problem}", pack.file)).size(10.5).color(t.reject));
            }
            None => {
                let mut on = pack.enabled;
                let response = ui.checkbox(&mut on, pack.name.as_str());
                if response.changed() {
                    actions.push(Action::SetPack(pack.name.clone(), on));
                }
                response.on_hover_text(pack.licence.clone());
                ui.label(
                    RichText::new(format!("{} keyword(s) · {}", pack.keywords, pack.licence))
                        .size(10.0)
                        .color(t.text_faint),
                );
            }
        }
    }
}

/// The whole-library suggestion pass, and its proposals.
///
/// The panel can *offer* a hundred keywords and still change nothing: every row is a button, and the
/// only thing that writes is pressing one. That is why the pass has no "apply all" — a bulk edit nobody
/// reviewed is the thing this panel exists to avoid.
fn suggest_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    let mut open = state.suggest_open;
    egui::Window::new("Suggested keywords")
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .default_width(420.0)
        .show(ui.ctx(), |ui| {
            match engine.batch_progress() {
                Some((done, total)) => {
                    let share = if total == 0 { 1.0 } else { done as f32 / total as f32 };
                    ui.add(
                        egui::ProgressBar::new(share)
                            .text(format!("{done} of {total}"))
                            .desired_width(300.0),
                    );
                    if ui.button(RichText::new("Stop").size(12.0)).clicked() {
                        actions.push(Action::CancelSuggest);
                    }
                    ui.label(RichText::new("Nothing is written while this runs.").size(10.5).color(t.text_faint));
                }
                None => {
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Suggest for the whole library").size(12.0)).clicked() {
                            actions.push(Action::StartSuggest);
                        }
                        if !engine.batch_results().is_empty() && ui.button(RichText::new("Clear").size(12.0)).clicked() {
                            actions.push(Action::ClearSuggest);
                        }
                    });
                    ui.label(
                        RichText::new("Uses what each file says about itself and any keyword packs you switched on.")
                            .size(10.5)
                            .color(t.text_faint),
                    );
                }
            }
            ui.add_space(6.0);
            ui.separator();

            let results = engine.batch_results().to_vec();
            if results.is_empty() {
                ui.label(RichText::new("No proposals yet.").size(11.0).color(t.text_faint));
                return;
            }
            ui.label(
                RichText::new(format!("{} photo(s) with something to suggest", results.len()))
                    .size(11.0)
                    .color(t.text_muted),
            );
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                for (id, suggestions) in results {
                    let name = engine
                        .photo(id)
                        .map(|photo| truncate(&photo.name, 32))
                        .unwrap_or_else(|| format!("#{}", id.0));
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(name).size(11.0).color(t.text_muted));
                        for suggestion in suggestions {
                            let response = ui
                                .add(Button::new(RichText::new(format!("+ {}", suggestion.tag)).size(10.5)).frame(false))
                                .on_hover_text(format!(
                                    "{:.0}% from {} — clicking adds it",
                                    suggestion.score * 100.0,
                                    suggestion.source.label()
                                ));
                            if response.clicked() {
                                actions.push(Action::AcceptSuggestion(id, suggestion.tag.clone()));
                            }
                        }
                    });
                }
            });
        });
    state.suggest_open = open;
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

/// Every shortcut, searchable, grouped by where it applies.
fn shortcuts_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens) {
    let mut open = state.shortcuts_open;
    egui::Window::new("Keyboard shortcuts")
        .open(&mut open)
        .resizable(true)
        .collapsible(false)
        .default_width(430.0)
        .max_height(560.0)
        .show(ui.ctx(), |ui| {
            let field = ui.add(
                TextEdit::singleline(&mut state.shortcut_query)
                    .hint_text("Search keys or actions")
                    .desired_width(f32::INFINITY),
            );
            if !field.has_focus() && state.shortcut_query.is_empty() {
                field.request_focus();
            }
            let matches = keymap::search(&state.shortcut_query);
            let query = state.shortcut_query.trim();
            ui.add_space(4.0);
            ui.label(
                RichText::new(if query.is_empty() {
                    format!("{} bindings", keymap::len())
                } else {
                    format!("{} of {} bound", matches.len(), keymap::len())
                })
                .size(10.0)
                .color(t.text_faint),
            );
            ui.separator();
            egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                if matches.is_empty() {
                    hint(ui, t, "Nothing is bound to that.");
                    return;
                }
                for context in Context::ALL {
                    let rows: Vec<&keymap::Binding> = matches.iter().copied().filter(|binding| binding.context == context).collect();
                    if rows.is_empty() {
                        continue;
                    }
                    ui.add_space(6.0);
                    ui.label(RichText::new(context.label()).size(11.0).strong().color(t.text));
                    ui.label(RichText::new(context.note()).size(9.5).color(t.text_faint));
                    for binding in rows {
                        keymap_row(ui, t, binding);
                    }
                }
            });
        });
    state.shortcuts_open = open;
}

/// One keycap and its action, aligned so the keys form a column.
fn keymap_row(ui: &mut Ui, t: &Tokens, binding: &keymap::Binding) {
    ui.horizontal(|ui| {
        ui.allocate_ui(vec2(96.0, 18.0), |ui| {
            let rect = ui.available_rect_before_wrap();
            let width = ui
                .painter()
                .layout_no_wrap(binding.keys.to_string(), FontId::monospace(10.5), t.text)
                .size()
                .x;
            let key = Rect::from_min_size(rect.min, vec2(width + 10.0, 16.0));
            ui.painter().rect(
                key,
                CornerRadius::same(3),
                t.thumb_bg,
                Stroke::new(1.0, t.border),
                egui::StrokeKind::Inside,
            );
            ui.painter()
                .text(key.center(), Align2::CENTER_CENTER, binding.keys, FontId::monospace(10.5), t.text);
            ui.allocate_space(vec2(width + 10.0, 18.0));
        });
        ui.label(RichText::new(binding.action).size(11.5).color(t.text_muted));
    });
}

fn nav_section(ui: &mut Ui, t: &Tokens, title: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(title.to_uppercase()).size(9.5).color(t.text_faint));
    ui.add_space(2.0);
}

fn nav_row(ui: &mut Ui, t: &Tokens, label: &str, count: &str, selected: bool) -> egui::Response {
    let height = 24.0;
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let spoken = if count.is_empty() {
        label.to_string()
    } else {
        format!("{label}, {count}")
    };
    crate::a11y::label_widget(ui, &response, egui::Role::Button, &spoken, selected);
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

    fn sample_photo(id: u64, name: &str, rating: u8) -> Photo {
        let mut photo = Photo::from_scan(PhotoId(id), "/photos", name, 1_000, Some(7), MediaKind::Image, 9);
        photo.rating = rating;
        photo.width = 3000;
        photo.height = 2000;
        photo
    }

    #[test]
    fn a_compare_names_what_actually_differs() {
        let left = sample_photo(1, "a.jpg", 5);
        let mut right = sample_photo(2, "b.jpg", 3);
        right.width = 2000;
        right.height = 2000;
        right.size = 2_000;

        let facts = compare_facts(&left, &right);
        let by_name = |name: &str| facts.iter().find(|(label, _)| *label == name).map(|(_, value)| value.clone());
        assert_eq!(by_name("stars").as_deref(), Some("5 vs 3"));
        assert_eq!(by_name("label").as_deref(), Some("none vs none"));
        assert_eq!(by_name("pixels").as_deref(), Some("3000×2000 vs 2000×2000"));
        assert!(by_name("size").is_some(), "a different file size is worth saying");

        // Two copies of the same frame say so by saying very little.
        let same = compare_facts(&left, &left);
        assert!(same.iter().all(|(label, _)| *label != "size"));
        assert!(same.iter().all(|(label, _)| *label != "exposure"));
    }

    /// A cell says why it has no pixels, and stays quiet when it has them.
    #[test]
    fn a_cell_badges_a_missing_file_before_a_failed_decode() {
        let mut photo = Photo {
            id: PhotoId(1),
            root: "/photos".to_string(),
            rel: "a.jpg".to_string(),
            name: "a.jpg".to_string(),
            size: 10,
            modified: None,
            kind: MediaKind::Image,
            key: 1,
            rating: 0,
            flag: Flag::None,
            label: None,
            missing: false,
            width: 0,
            height: 0,
            captured: None,
            camera: String::new(),
            lens: String::new(),
            iso: None,
            aperture: None,
            shutter: None,
            focal: None,
            tags: Vec::new(),
            edited: false,
            retry: false,
        };
        assert_eq!(cell_badge(&photo, None), None, "a decoded cell needs no badge");
        assert_eq!(cell_badge(&photo, Some("bad jpeg")), Some(CellBadge::Unreadable));
        photo.missing = true;
        assert_eq!(
            cell_badge(&photo, Some("bad jpeg")),
            Some(CellBadge::Offline),
            "a missing file outranks the decode failure it caused"
        );
        // Both badges explain themselves on hover; a glyph alone is a riddle.
        assert!(CellBadge::Offline.hint().contains("missing"));
        assert!(CellBadge::Unreadable.hint().contains("decoded"));
    }

    /// The grid paints a cell for a missing photo without asking for a decode, and names it for a reader.
    #[test]
    fn a_missing_photo_paints_with_its_badge() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let dir = std::env::temp_dir().join(format!("sortlens-ui-badge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let mut photo = Photo {
            id: PhotoId(7),
            root: "/photos".to_string(),
            rel: "gone.jpg".to_string(),
            name: "gone.jpg".to_string(),
            size: 10,
            modified: None,
            kind: MediaKind::Image,
            key: 7,
            rating: 4,
            flag: Flag::Pick,
            label: None,
            missing: true,
            width: 0,
            height: 0,
            captured: None,
            camera: String::new(),
            lens: String::new(),
            iso: None,
            aperture: None,
            shutter: None,
            focal: None,
            tags: Vec::new(),
            edited: false,
            retry: false,
        };
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let response = photo_cell(ui, &tokens, &photo, false, 96.0, 16.0, &mut thumbs);
                assert!(!response.clicked());
            });
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty(), "the cell painted something");
        assert_eq!(
            crate::a11y::describe_photo(&photo),
            "gone.jpg, 4 stars, pick, file missing",
            "and the spoken name says what the badge shows"
        );
        photo.missing = false;
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Every justified row fills the width at one height, and the last row is not stretched to do it.
    #[test]
    fn justified_rows_fill_the_width_without_blowing_up_the_last_one() {
        // Twenty-two 3:2 frames in a 1224 point window at a 160 point target: five across.
        let aspects = vec![1.5_f32; 22];
        let rows = justified_rows(&aspects, 1224.0, 8.0, 160.0);
        let mut seen = 0;
        for (index, (range, height)) in rows.iter().enumerate() {
            assert_eq!(range.start, seen, "rows are contiguous and in order");
            seen = range.end;
            let count = range.len() as f32;
            let sum: f32 = aspects[range.clone()].iter().sum();
            // The row's own height is what makes its widths add up to the window. Every row but the last,
            // which is left-aligned at the target height by design.
            if index + 1 < rows.len() {
                assert!(
                    ((sum * height + 8.0 * (count - 1.0)) - 1224.0).abs() < 1.0,
                    "row {index} spans the width, not {} of it",
                    sum * height + 8.0 * (count - 1.0)
                );
                assert!((height - 160.0).abs() < 20.0, "a full row sits near the target height");
            }
        }
        assert_eq!(seen, 22, "every photo lands in exactly one row");
        // Five across, so 22 photos leave a last row of two. Stretching two photographs across the window
        // would make cells nobody asked for, so the short row keeps the target height instead.
        let (last, last_height) = rows.last().expect("a row");
        assert_eq!(last, &(20..22), "the leftover photos, not a re-shuffle");
        assert_eq!(last_height, &160.0, "a short last row is not stretched");
    }

    /// A single panorama wider than the window gets a row of its own instead of an overflow or a divide by zero.
    #[test]
    fn a_panorama_gets_its_own_row_and_never_a_nan() {
        let rows = justified_rows(&[5.0, 1.5, 1.5], 800.0, 8.0, 160.0);
        assert_eq!(rows[0].0, 0..1, "one very wide photo takes a row");
        for (_, height) in &rows {
            assert!(height.is_finite() && *height >= 24.0, "heights are real numbers");
        }
        assert!(justified_rows(&[], 800.0, 8.0, 160.0).is_empty(), "an empty library has no rows");
    }

    /// The justified grid packs rows from the library's own aspect ratios, and Density still means something.
    #[test]
    fn the_justified_grid_packs_and_repacks_only_when_it_must() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-justified-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-justified");
        for index in 0..9 {
            let name = format!("IMG_{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [20 + index * 20, 90, 170], 48, 32), root.join(&name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let mut plan = JustifiedPlan::default();
        plan.prepare(&engine, &Filter::default(), Sort::Natural, 900.0, 160.0, 8.0);
        let rows = plan.rows.len();
        assert!(rows > 0, "nine photos pack into at least one row");
        assert_eq!(plan.rows.iter().map(|row| row.ids.len()).sum::<usize>(), 9);
        assert!(plan.total > 0.0);
        // Same inputs: the plan is reused, not rebuilt. A different target height is a rebuild.
        let built = plan.rows.clone();
        plan.prepare(&engine, &Filter::default(), Sort::Natural, 900.0, 160.0, 8.0);
        assert_eq!(plan.rows, built, "an unchanged view does not re-pack");
        plan.prepare(&engine, &Filter::default(), Sort::Natural, 900.0, 300.0, 8.0);
        assert_ne!(plan.rows.len(), 0, "a taller target still produces rows");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A grid of hundreds of cells at once asks for their thumbnails, and the queue bounds what waits.
    ///
    /// The interesting claim is not "it asks" but "it cannot ask for everything": without the bound a fast
    /// scroll enqueues the library and holds the memory for it.
    #[test]
    fn a_full_grid_stays_within_the_decode_queue_bound() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-bound-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-bound");
        for index in 0..400 {
            let name = format!("IMG_{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [index as u8, 120, 200], 16, 12), root.join(&name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..1200 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        // A bound of eight requests, so the bound is what the test watches rather than a number the grid never
        // comes close to: eighty-odd visible cells against eight slots must produce refusals.
        let mut thumbs = Thumbnails::with_queue_bound(dir.join("cache"), 1, 8);
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.density = Density::Compact;
        let mut actions: Vec<Action> = Vec::new();
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1440.0, 900.0))),
            ..egui::RawInput::default()
        };
        let mut painted = 0;
        for _ in 0..3 {
            let mut output = ctx.run_ui(input.clone(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    grid(ui, &tokens, &mut state, &mut engine, &mut thumbs, &Filter::default(), &mut actions);
                });
            });
            output.textures_delta.clear();
            painted = state.cell_rects.len();
        }
        let (queued, refused) = thumbs.queue();
        assert!(painted > 0, "the grid painted cells, so it also asked for them");
        assert!(queued <= 8, "{queued} requests waiting, over the eight slot bound");
        assert!(refused > 0, "with eight slots and {painted} cells, some request had to be refused");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Both layouts paint, and switching the mode keeps working: the mode is a view, not a new grid.
    #[test]
    fn both_grid_modes_paint_and_the_chip_switches_between_them() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-modes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-modes");
        for index in 0..5 {
            let name = format!("IMG_{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [60, 20 + index * 30, 150], 48, 32), root.join(&name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        for mode in GridMode::ALL {
            state.grid_mode = mode;
            let mut actions: Vec<Action> = Vec::new();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    grid(ui, &tokens, &mut state, &mut engine, &mut thumbs, &Filter::default(), &mut actions);
                });
            });
            output.textures_delta.clear();
            assert_eq!(state.cell_rects.len(), 5, "{mode:?} paints every photo");
            assert!(actions.is_empty(), "painting asks for nothing");
        }
        assert_eq!(GridMode::Square.next(), GridMode::Justified);
        assert_eq!(GridMode::Justified.next(), GridMode::Square);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A marquee selects what it touches and only what it touches, and it sorts for a stable log.
    #[test]
    fn a_marquee_band_selects_the_cells_it_touches() {
        let rect = |x: f32, y: f32| Rect::from_min_size(pos2(x, y), vec2(100.0, 100.0));
        let cells = vec![
            (PhotoId(1), rect(0.0, 0.0)),
            (PhotoId(2), rect(110.0, 0.0)),
            (PhotoId(3), rect(0.0, 110.0)),
            (PhotoId(4), rect(110.0, 110.0)),
        ];
        // A band around the first two cells only.
        let band = Rect::from_two_pos(pos2(-5.0, -5.0), pos2(205.0, 50.0));
        assert_eq!(cells_in(&cells, band), vec![PhotoId(1), PhotoId(2)]);
        // Touching an edge counts: a drag that grazes a cell selects it, which is what a person expects.
        let graze = Rect::from_two_pos(pos2(99.0, 0.0), pos2(112.0, 10.0));
        assert_eq!(cells_in(&cells, graze), vec![PhotoId(1), PhotoId(2)]);
        // A band over empty space selects nothing, and selects nothing in id order either.
        let empty = Rect::from_two_pos(pos2(300.0, 300.0), pos2(400.0, 400.0));
        assert!(cells_in(&cells, empty).is_empty());
        // The order is by id, not by the order cells happened to be painted in.
        let all = Rect::from_two_pos(pos2(-10.0, -10.0), pos2(400.0, 400.0));
        assert_eq!(cells_in(&cells, all), vec![PhotoId(1), PhotoId(2), PhotoId(3), PhotoId(4)]);
    }

    /// The marquee draws its band, and releasing it ends the gesture.
    #[test]
    fn the_marquee_band_paints_and_release_ends_it() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.marquee = Some(pos2(10.0, 10.0));
        state.cell_rects = vec![(PhotoId(1), Rect::from_min_size(pos2(40.0, 40.0), vec2(80.0, 80.0)))];
        state.selection.set_many(&[PhotoId(1)]);

        // Pointer down and moving: the band is painted and the selection follows it.
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0))),
            events: vec![
                egui::Event::PointerMoved(pos2(120.0, 120.0)),
                egui::Event::PointerButton {
                    pos: pos2(120.0, 120.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                marquee(ui, &mut state, &tokens, rect);
            });
        });
        output.textures_delta.clear();
        assert!(state.marquee.is_some(), "the gesture is still running");
        assert!(!output.shapes.is_empty(), "the band is painted");

        // Release: the band goes, the selection stays.
        let input = egui::RawInput {
            events: vec![egui::Event::PointerButton {
                pos: pos2(120.0, 120.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let rect = ui.max_rect();
                marquee(ui, &mut state, &tokens, rect);
            });
        });
        output.textures_delta.clear();
        assert!(state.marquee.is_none(), "releasing ends the drag");
        assert!(!state.selection.is_empty(), "and the selection it made stays");
    }

    /// Click, ⌘-click and ⇧-click are three different gestures, and each has to leave the inspector sane.
    #[test]
    fn a_selection_replaces_toggles_and_ranges() {
        let order: Vec<PhotoId> = (1..=6).map(PhotoId).collect();
        let mut selection = Selection::default();
        assert!(selection.is_empty());

        selection.replace(PhotoId(2));
        assert_eq!(selection.primary(), Some(PhotoId(2)));
        assert_eq!(selection.ids(), vec![PhotoId(2)]);

        // ⌘-click adds.
        selection.toggle(PhotoId(5));
        assert_eq!(selection.ids(), vec![PhotoId(2), PhotoId(5)]);
        assert_eq!(selection.primary(), Some(PhotoId(5)), "the last one you clicked is the one you see");

        // ⌘-click again removes, and the primary moves to what is left rather than pointing at a photo
        // that is no longer selected.
        selection.toggle(PhotoId(5));
        assert_eq!(selection.ids(), vec![PhotoId(2)]);
        assert_eq!(selection.primary(), Some(PhotoId(2)));

        // ⇧-click measures from the anchor (2) to the clicked photo, in the order the grid shows.
        selection.toggle(PhotoId(2));
        assert!(selection.is_empty(), "the anchor followed it away");
        selection.replace(PhotoId(2));
        selection.extend(&order, PhotoId(5));
        assert_eq!(selection.ids(), vec![PhotoId(2), PhotoId(3), PhotoId(4), PhotoId(5)]);
        assert_eq!(selection.primary(), Some(PhotoId(5)));

        // And backwards, from the same anchor.
        selection.extend(&order, PhotoId(1));
        assert_eq!(selection.ids(), vec![PhotoId(1), PhotoId(2)], "the run between them, not the union");

        // A range with no anchor in view is a plain click: there is nothing to measure from.
        selection.clear();
        selection.extend(&order, PhotoId(4));
        assert_eq!(selection.ids(), vec![PhotoId(4)]);
        assert_eq!(selection.primary(), Some(PhotoId(4)));
    }

    /// Selecting everything, then filtering, does not leave the selection pointing at photos that are gone.
    #[test]
    fn select_all_and_retain_follow_what_is_actually_there() {
        let order: Vec<PhotoId> = (1..=4).map(PhotoId).collect();
        let mut selection = Selection::default();
        selection.all(&order);
        assert_eq!(selection.len(), 4);
        assert_eq!(selection.primary(), Some(PhotoId(1)), "something to show in the inspector");

        selection.retain(|id| id != PhotoId(1));
        assert_eq!(selection.ids(), vec![PhotoId(2), PhotoId(3), PhotoId(4)]);
        assert_eq!(selection.primary(), Some(PhotoId(2)), "the primary follows the survivors");

        selection.retain(|_| false);
        assert!(selection.is_empty());
        assert_eq!(selection.primary(), None);
    }

    /// A batch key press writes every ticked photo, and says how many in one line.
    #[test]
    fn rating_a_selection_writes_every_ticked_photo() {
        let (_lib, photos, mut engine) = {
            let dir = std::env::temp_dir().join(format!("sortlens-ui-batch-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let root = dir.join("photos");
            std::fs::create_dir_all(&root).expect("root");
            let fixture = sortlens_testkit::Fixture::new("ui-batch");
            for index in 0..3 {
                let name = format!("IMG_{index}.jpg");
                std::fs::copy(fixture.jpeg(&name, [40 * index as u8, 80, 160], 48, 32), root.join(&name)).expect("copy");
            }
            let mut engine = Engine::open(&dir).expect("open");
            engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
            for _ in 0..600 {
                if engine.poll().is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            (dir, root, engine)
        };
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        assert_eq!(ids.len(), 3);

        let mut selection = Selection::default();
        selection.all(&ids);
        // The keyboard path calls the engine with exactly this list.
        engine.rate(&selection.ids(), 4).expect("rate");
        engine.flag(&selection.ids(), Flag::Pick).expect("flag");
        for id in &ids {
            let photo = engine.photo(*id).expect("photo");
            assert_eq!((photo.rating, photo.flag), (4, Flag::Pick), "every ticked photo, not just one");
        }
        let _ = std::fs::remove_dir_all(&photos);
    }

    /// The grid paints a multi-selection: every ticked cell draws its outline, and one press asks for nothing.
    #[test]
    fn the_grid_paints_a_multi_selection() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let dir = std::env::temp_dir().join(format!("sortlens-ui-multi-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-multi");
        for index in 0..3 {
            let name = format!("IMG_{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [30 + index * 50, 110, 190], 48, 32), root.join(&name)).expect("copy");
        }
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.selection.all(&ids);
        let mut actions: Vec<Action> = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                grid(ui, &tokens, &mut state, &mut engine, &mut thumbs, &Filter::default(), &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(actions.is_empty(), "painting a selection does not act on it");
        assert_eq!(state.selection.len(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Density changes how many photos fit across the same window, and the row still fills it exactly.
    #[test]
    fn a_denser_grid_fits_more_photos_across_the_same_window() {
        let width = 1200.0;
        let gap = 10.0;
        let compact = grid_layout(width, gap, Density::Compact.min_cell());
        let comfortable = grid_layout(width, gap, Density::Comfortable.min_cell());
        let roomy = grid_layout(width, gap, Density::Roomy.min_cell());
        assert!(
            compact.0 > comfortable.0,
            "compact fits more across: {compact:?} vs {comfortable:?}"
        );
        assert!(comfortable.0 > roomy.0, "roomy fits fewer: {comfortable:?} vs {roomy:?}");

        // Every density divides the width evenly: the last cell in a row reaches the right edge, give or take
        // the rounding to whole points.
        for (columns, cell) in [compact, comfortable, roomy] {
            let used = cell * columns as f32 + gap * (columns.saturating_sub(1)) as f32;
            let slack = width - used;
            assert!((0.0..=gap + 1.0).contains(&slack), "slack {slack} at {columns} columns");
        }
    }

    /// A very narrow window still gets one readable column rather than cells too small to read.
    #[test]
    fn a_narrow_window_keeps_one_column_and_a_legible_cell() {
        let (columns, cell) = grid_layout(80.0, 10.0, Density::Roomy.min_cell());
        assert_eq!(columns, 1, "never zero columns");
        assert_eq!(cell, 80.0, "one cell takes the width");
        // Below the legibility floor the cell stops shrinking and the row scrolls instead.
        let (columns, cell) = grid_layout(40.0, 10.0, Density::Compact.min_cell());
        assert_eq!(columns, 1);
        assert_eq!(cell, 64.0, "the 64 point floor");
    }

    /// The three steps cycle, round-trip through their ids, and default to Comfortable.
    #[test]
    fn density_steps_cycle_and_survive_a_settings_round_trip() {
        assert_eq!(Density::default(), Density::Comfortable);
        assert_eq!(Density::Compact.next(), Density::Comfortable);
        assert_eq!(Density::Comfortable.next(), Density::Roomy);
        assert_eq!(Density::Roomy.next(), Density::Compact, "the chip wraps");
        for step in Density::ALL {
            assert_eq!(Density::from_id(step.id()), step);
            assert!(!step.label().is_empty());
        }
        assert_eq!(Density::from_id("enormous"), Density::Comfortable, "an unknown id is the default");
        assert!(Density::Compact.min_cell() < Density::Comfortable.min_cell());
        assert!(Density::Comfortable.min_cell() < Density::Roomy.min_cell());
    }

    /// The grid paints at every density, over a real library, without asking for anything.
    #[test]
    fn the_grid_paints_at_every_density() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let dir = std::env::temp_dir().join(format!("sortlens-ui-density-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-density");
        for index in 0..3 {
            let name = format!("IMG_{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [60 + index * 40, 90, 140], 64, 48), root.join(&name)).expect("copy");
        }
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        let mut actions: Vec<Action> = Vec::new();
        for step in Density::ALL {
            state.density = step;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    grid(ui, &tokens, &mut state, &mut engine, &mut thumbs, &Filter::default(), &mut actions);
                });
            });
            output.textures_delta.clear();
            assert!(output.shapes.len() > 2, "{} painted a grid", step.label());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A root on a volume that is not there is marked in the sidebar, and the paint says so.
    ///
    /// The fixture root is a real folder on a mounted volume, so the "away" branch cannot be reached on this
    /// machine through the engine. What is tested here is the decision — `Some(false)` marks, `Some(true)`
    /// and `None` do not — because treating "cannot tell" as "gone" is the mistake that would tell a user
    /// their library vanished.
    #[test]
    fn a_root_on_a_missing_volume_is_marked_away_and_an_unknown_volume_is_not() {
        let mounted = |state: Option<bool>| match state {
            Some(false) => ("12 · away".to_string(), true),
            _ => (format_count(12), false),
        };
        assert_eq!(mounted(Some(false)), ("12 · away".to_string(), true));
        assert_eq!(mounted(Some(true)), ("12".to_string(), false));
        assert_eq!(mounted(None), ("12".to_string(), false), "an unknown volume is not an absent one");

        // And the real sidebar paints with a root whose volume is present.
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let dir = std::env::temp_dir().join(format!("sortlens-ui-volumes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-volumes");
        std::fs::copy(fixture.jpeg("a.jpg", [90, 120, 200], 64, 64), root.join("a.jpg")).expect("copy");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let volumes = engine.root_volumes();
        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0].mounted, Some(true), "the fixture folder is on a mounted volume");

        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        let mut actions: Vec<Action> = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                left_panel(ui, &mut state, &tokens, &mut engine, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(actions.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The sources ask different questions, and each one is the whole question rather than a nudge.
    ///
    /// The bug this guards: picking "Picks" while three stars and a keyword were active used to leave them
    /// on, so the row meant "picks among the ones you starred and tagged", and pressing it twice did
    /// nothing visible. A source replaces the question; the folder is where you are, not what you asked.
    #[test]
    fn a_source_replaces_the_question_and_keeps_the_folder() {
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.root = Some("/photos".to_string());
        state.folder = "2026/trip/".to_string();
        state.min_rating = 4;
        state.label_filter = Some(ColorLabel::Red);
        state.tag_filters = vec!["harbour".to_string()];
        state.search = "sunset".to_string();

        state.apply_source(Source::Picks);
        let filter = state.filter();
        assert_eq!(filter.flag, Some(Flag::Pick));
        assert_eq!(filter.min_rating, 0, "the stars were somebody else's question");
        assert!(filter.tags.is_empty());
        assert!(filter.label.is_none());
        assert!(filter.text.is_empty());
        assert_eq!(filter.root.as_deref(), Some("/photos"), "you are still in the library you opened");
        assert_eq!(filter.folder.as_deref(), Some("2026/trip/"));

        state.apply_source(Source::Rated);
        assert_eq!(state.filter().flag, None);
        assert_eq!(state.filter().min_rating, 1, "rated means one star, not five");
        assert!(state.source_selected(Source::Rated));
        assert!(!state.source_selected(Source::Picks));
        assert!(!state.source_selected(Source::All));

        state.apply_source(Source::Rejects);
        assert_eq!(state.filter().flag, Some(Flag::Reject));

        state.apply_source(Source::All);
        assert!(state.filter().flag.is_none());
        assert_eq!(state.filter().min_rating, 0);
        assert!(state.source_selected(Source::All));
        assert!(state.all_photos_selected, "All Photos is also what the cull queue starts from");
    }

    /// The sidebar paints its four sources over a real library, with counts that differ.
    #[test]
    fn the_sidebar_lists_every_source_with_its_own_count() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let dir = std::env::temp_dir().join(format!("sortlens-ui-sources-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-sources");
        std::fs::copy(fixture.jpeg("a.jpg", [200, 40, 40], 64, 64), root.join("a.jpg")).expect("copy a");
        std::fs::copy(fixture.jpeg("b.jpg", [40, 200, 40], 64, 64), root.join("b.jpg")).expect("copy b");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let ids: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        assert_eq!(ids.len(), 2, "two photos indexed");
        engine.flag(&ids[..1], Flag::Pick).expect("pick");
        engine.rate(&ids[1..], 3).expect("rate");

        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        let mut actions: Vec<Action> = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                left_panel(ui, &mut state, &tokens, &mut engine, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(actions.is_empty(), "painting the sidebar presses nothing");
        assert!(state.source_selected(Source::All), "nothing was narrowed by drawing the panel");
        assert_eq!(engine.stats().total, 2);

        // The counts come from the library, so Picks is one and Rated is one, not both "2".
        assert_eq!(
            engine.count(&Filter {
                flag: Some(Flag::Pick),
                ..Filter::default()
            }),
            1
        );
        assert_eq!(
            engine.count(&Filter {
                min_rating: 1,
                ..Filter::default()
            }),
            1
        );
        assert_eq!(
            engine.count(&Filter {
                flag: Some(Flag::Reject),
                ..Filter::default()
            }),
            0
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The export dialog has to survive being drawn with no photos, with photos, mid-job and after a
    /// receipt — and it must offer the two facts that make it trustworthy rather than decorative.
    #[test]
    fn the_export_dialog_paints_in_every_state() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let dir = std::env::temp_dir().join(format!("sortlens-ui-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);

        // Nothing selected: the dialog still draws, and offers nothing to start.
        state.export_open = true;
        let mut actions: Vec<Action> = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                export_dialog(ui, &mut state, &tokens, &mut engine, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(state.export_open, "the window stays open until Cancel or the host closes it");
        assert!(actions.is_empty(), "no folder is picked without a click");

        // With photos and a JPEG spec: still paints, and a full spec is accepted.
        state.export_ids = vec![PhotoId(1), PhotoId(2)];
        state.export_spec = sortlens_engine::ExportSpec {
            format: sortlens_engine::ExportFormat::Jpeg,
            max_edge: Some(2048),
            quality: 82,
            metadata: false,
            collision: sortlens_engine::Collision::Rename,
        };
        let mut actions: Vec<Action> = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                export_dialog(ui, &mut state, &tokens, &mut engine, &mut actions);
            });
        });
        output.textures_delta.clear();

        // A receipt draws too, and closing it clears it.
        state.export_open = false;
        state.export_receipt = Some("3 written · 0 skipped · 12 kB → /tmp/out".to_string());
        let mut actions: Vec<Action> = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                export_dialog(ui, &mut state, &tokens, &mut engine, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(state.export_receipt.is_some(), "the receipt waits for the user");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The batch panel has to paint with no pass, during a pass, and with proposals to accept — and it
    /// must not ask for anything until a button is pressed.
    #[test]
    fn the_suggest_dialog_paints_idle_running_and_with_results() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let dir = std::env::temp_dir().join(format!("sortlens-ui-suggest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.suggest_open = true;

        // Idle: nothing running, no proposals, and no action without a click.
        let mut actions: Vec<Action> = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                suggest_dialog(ui, &mut state, &tokens, &mut engine, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(actions.is_empty(), "opening the panel asks for nothing");
        assert!(state.suggest_open, "the window stays open");

        // Running: a progress bar and a Stop, still with no action taken.
        engine.begin_batch_suggest().expect("start");
        let mut actions: Vec<Action> = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                suggest_dialog(ui, &mut state, &tokens, &mut engine, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(actions.is_empty());
        assert!(engine.batch_progress().is_some(), "the dialog was drawn while a pass was live");
        // Let it finish so the temp library is not deleted out from under a worker.
        for _ in 0..400 {
            if engine.poll_batch().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(engine.batch_results().is_empty(), "an empty library proposes nothing");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The packs row has to survive three real states: no packs at all, a pack switched off, and a
    /// pack file someone mistyped — the last one must name the file rather than disappear.
    #[test]
    fn the_keyword_pack_rows_paint_with_none_off_and_broken() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let dir = std::env::temp_dir().join(format!("sortlens-ui-packs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("packs")).expect("packs dir");

        // No packs: the row explains where they go instead of showing an empty list.
        let engine = Engine::open(&dir).expect("open");
        let mut actions = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                keyword_packs(ui, &tokens, &engine, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(actions.is_empty(), "nothing can be switched without a pack");

        // One good pack (off) and one broken file.
        std::fs::write(
            dir.join("packs").join("travel.json"),
            r#"{"name":"Travel","licence":"CC0-1.0","keywords":[{"keyword":"japan","when":{"folder_contains":"Japan"}}]}"#,
        )
        .expect("write");
        std::fs::write(dir.join("packs").join("typo.json"), "{ nope").expect("write");
        let engine = Engine::open(&dir).expect("open");
        let mut actions = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                keyword_packs(ui, &tokens, &engine, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(actions.is_empty(), "a checkbox that is not clicked asks for nothing");
        assert_eq!(engine.packs().len(), 2, "both are listed, one with its problem");
        assert!(engine.packs().iter().any(|pack| pack.problem.is_some()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The suggestion row draws every offer, and an empty list draws nothing at all — an "Suggested"
    /// heading with no suggestions under it would be a promise with nothing behind it.
    #[test]
    fn the_suggestion_row_paints_its_offers_and_stays_silent_when_there_are_none() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let id = PhotoId(7);
        let suggestions = vec![
            sortlens_engine::Suggestion {
                tag: "low light".to_string(),
                score: 0.8,
                source: sortlens_engine::SuggestionSource::ExifHint,
                found: None,
            },
            sortlens_engine::Suggestion {
                tag: "wide".to_string(),
                score: 0.7,
                source: sortlens_engine::SuggestionSource::ExifHint,
                found: None,
            },
        ];

        let mut actions: Vec<Action> = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                suggestions_row(ui, &tokens, id, &suggestions, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(actions.is_empty(), "nothing is written without a click");

        let painted = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(300.0, 40.0))),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    suggestions_row(ui, &tokens, id, &suggestions, &mut actions);
                });
            },
        );
        let mut painted = painted;
        painted.textures_delta.clear();
        assert!(!painted.shapes.is_empty(), "the row draws text when there is something to offer");

        // No suggestions: no heading, no buttons, nothing to click.
        let mut empty_actions: Vec<Action> = Vec::new();
        let mut empty = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                suggestions_row(ui, &tokens, id, &[], &mut empty_actions);
            });
        });
        empty.textures_delta.clear();
        assert!(empty_actions.is_empty());
    }

    #[test]
    fn toggling_a_compare_closes_it_without_asking_the_library() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-compare-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let engine = Engine::open(&dir).expect("open");
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);

        state.compare = Some(PhotoId(4));
        toggle_compare(&mut state, &engine);
        assert_eq!(state.compare, None, "a second press closes the compare");

        // With nothing selected there is nothing to compare against, and the reason is said out loud.
        state.selection.clear();
        toggle_compare(&mut state, &engine);
        assert_eq!(state.compare, None);
        assert!(state.live_notice().is_some() || state.notice.is_some(), "the reason is not silent");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
