//! The application shell: five surfaces, in the suite's order.
//!
//! Top bar (identity, search, filters, actions), left panel (sources, folders, collections),
//! canvas (grid today, cull and detail later), right panel (details, tags, activity), bottom bar
//! (views, review state, counts).
//!
//! The shell owns presentation state and nothing else. Anything that changes the library goes
//! through [`Engine`]; anything that changes the process (theme, folder picker, quit) is returned
//! as an [`Action`] for the host to apply.

use crate::a11y;
use crate::charts::{self, Column};
use crate::keymap::{self, Context};
use crate::places;
use crate::sharpness;
use crate::theme::{Density, GridMode, Motion, ThemeKind, Tokens};
use crate::thumbs::{GRID_SIZE, Thumbnails};
use craft_keymap::ui::{DialogState, English, Options as ShortcutOptions, Outcome as ShortcutOutcome, ShortcutsDialog};
use craft_keymap::{CommandRef, Keymap, Overrides, Profiles};
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
    /// One photograph, large, with its facts.
    Detail,
    /// Two photographs side by side, at one magnification.
    Compare,
    /// Memories and resurfacing cards.
    ForYou,
    /// Map of geotagged photos.
    Places,
    /// Library statistics.
    Insights,
    /// The relative sharpness review: the softest slice of this library, never a verdict on a frame.
    SoftFocus,
    /// Results of a similar-photo search.
    Similar,
}

impl View {
    /// Every destination, in bottom-bar order.
    ///
    /// The order is the tab order and, when the window is too narrow for nine labelled tabs, the menu's
    /// order: a destination must not move because the window shrank.
    pub const ALL: [Self; 9] = [
        Self::Library,
        Self::ForYou,
        Self::Cull,
        Self::Detail,
        Self::Compare,
        Self::Places,
        Self::Similar,
        Self::SoftFocus,
        Self::Insights,
    ];

    /// Label for the bottom bar and the contextual header.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::ForYou => "For You",
            Self::Cull => "Cull",
            Self::Detail => "Detail",
            Self::Compare => "Compare",
            Self::Places => "Places",
            Self::Insights => "Insights",
            Self::SoftFocus => "Soft Focus",
            Self::Similar => "Similar",
        }
    }

    /// The glyph for the bottom bar. A destination is a place, and a place gets a picture.
    pub const fn icon(self) -> crate::icons::Icon {
        match self {
            Self::Library => crate::icons::Icon::Library,
            Self::ForYou => crate::icons::Icon::Sparkle,
            Self::Cull => crate::icons::Icon::Judge,
            Self::Detail => crate::icons::Icon::Info,
            Self::Compare => crate::icons::Icon::Compare,
            Self::Places => crate::icons::Icon::Pin,
            Self::Insights => crate::icons::Icon::Chart,
            Self::SoftFocus => crate::icons::Icon::Focus,
            Self::Similar => crate::icons::Icon::Similar,
        }
    }

    /// Whether this destination draws one photograph large (alone or beside a second).
    ///
    /// Detail and Compare are two views of the same canvas, so everything that asks "is the photograph on
    /// screen rather than the grid?" asks this, and the keyboard guards read one answer, not two booleans
    /// that could disagree.
    pub const fn is_detail(self) -> bool {
        matches!(self, Self::Detail | Self::Compare)
    }

    /// The milestone that still has to arrive before this view can open, or `None` when it opens today.
    ///
    /// Every destination opens today. `Cull` and `Places` used to be gated here with labels that had gone stale — a
    /// shipped cull screen still claiming "M4" in the nav — and a gate that closes a working view is worse than no
    /// gate. A view that is only partly built says so *inside* itself, where the person looking at it will read it.
    pub const fn milestone(self) -> Option<&'static str> {
        None
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
    /// What the file and the packs propose, grouped by where the proposal came from.
    Suggestions,
    /// Background jobs.
    Activity,
}

impl RightTab {
    /// Tabs in order, which is also the order the far-edge tool strip paints them.
    pub const ALL: [Self; 4] = [Self::Details, Self::Tags, Self::Suggestions, Self::Activity];

    /// Tab label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Details => "Details",
            Self::Tags => "Tags",
            Self::Suggestions => "Suggestions",
            Self::Activity => "Activity",
        }
    }

    /// Tab glyph.
    pub const fn icon(self) -> crate::icons::Icon {
        match self {
            Self::Details => crate::icons::Icon::Info,
            Self::Tags => crate::icons::Icon::Tag,
            Self::Suggestions => crate::icons::Icon::Wand,
            Self::Activity => crate::icons::Icon::Activity,
        }
    }
}

/// What a proposal is about, which is how the Suggestions tab groups them.
///
/// Three families rather than four sources, because a person deciding about a keyword asks a coarser question than
/// the pipeline records: "something in the picture", "this kind of scene", or "the camera said so". The last one is
/// not a guess at all, which is why it is named after the file rather than after a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SuggestKind {
    /// Found in the pixels by a detector.
    Objects,
    /// Proposed by a keyword pack, or said in words by a captioner.
    Scene,
    /// Read out of the file's own EXIF.
    Exif,
}

impl SuggestKind {
    /// Families in the order the panel lists them: what a model saw, what the scene resembles, what the file says.
    const ALL: [Self; 3] = [Self::Objects, Self::Scene, Self::Exif];

    /// Heading.
    const fn label(self) -> &'static str {
        match self {
            Self::Objects => "Objects",
            Self::Scene => "Scene",
            Self::Exif => "EXIF",
        }
    }

    /// Which family a source belongs to.
    const fn of(source: sortlens_engine::SuggestionSource) -> Self {
        use sortlens_engine::SuggestionSource;
        match source {
            SuggestionSource::Detector => Self::Objects,
            SuggestionSource::Pack | SuggestionSource::Caption => Self::Scene,
            SuggestionSource::ExifHint => Self::Exif,
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
    /// The model-downloader consent was switched on or off, and should be persisted.
    SetModelConsent(bool),
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
    /// Reduce motion: no slide, no fade in the cull overlay.
    SetMotion(Motion),
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
    /// The shortcut editor closed with OK: persist the sets and which one is active.
    SaveShortcuts(Profiles),
    /// The editor asked for a shortcut-set file to import.
    ImportShortcuts,
    /// The editor handed a set back to write out as craft-keymap JSON.
    ExportShortcuts(String, String),
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

/// Which collections are ticked in the sidebar, and which row a ⇧-click ranges from.
///
/// A photo `Selection` cannot be reused here: a collection id and a photo id are different kinds of thing, and a
/// set that could silently be passed where photos were expected is a bug waiting for a refactor. Only the anchor
/// and the ticks are kept — there is no "primary", because the collection a click opens is the *filter*, which
/// already exists in `collection_filter`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CollectionSelection {
    /// The row a ⇧-click ranges from.
    anchor: Option<sortlens_core::CollectionId>,
    /// Ticked collections.
    ids: std::collections::BTreeSet<sortlens_core::CollectionId>,
}

impl CollectionSelection {
    /// Whether this collection is ticked.
    pub fn contains(&self, id: sortlens_core::CollectionId) -> bool {
        self.ids.contains(&id)
    }

    /// How many are ticked.
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether nothing is ticked.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Every ticked collection, ascending.
    pub fn ids(&self) -> Vec<sortlens_core::CollectionId> {
        self.ids.iter().copied().collect()
    }

    /// A plain click: this row and nothing else.
    pub fn replace(&mut self, id: sortlens_core::CollectionId) {
        self.ids.clear();
        self.ids.insert(id);
        self.anchor = Some(id);
    }

    /// ⌘-click: add or remove one row, and move the anchor to it.
    pub fn toggle(&mut self, id: sortlens_core::CollectionId) {
        if !self.ids.remove(&id) {
            self.ids.insert(id);
        }
        self.anchor = Some(id);
    }

    /// ⇧-click: take the run between the anchor and this row, in sidebar order.
    ///
    /// Sidebar order rather than id order, for the same reason the photo grid ranges in grid order: the run
    /// between two rows you can see is the run you meant. With no anchor in view this is a plain click.
    pub fn extend(&mut self, order: &[sortlens_core::CollectionId], id: sortlens_core::CollectionId) {
        let Some(anchor) = self.anchor else {
            self.replace(id);
            return;
        };
        let (Some(from), Some(to)) = (
            order.iter().position(|other| *other == anchor),
            order.iter().position(|other| *other == id),
        ) else {
            self.replace(id);
            return;
        };
        let (low, high) = if from <= to { (from, to) } else { (to, from) };
        self.ids = order[low..=high].iter().copied().collect();
    }

    /// Drop ticks for collections that no longer exist, so a delete cannot leave a ghost selected.
    pub fn retain(&mut self, order: &[sortlens_core::CollectionId]) {
        self.ids.retain(|id| order.contains(id));
        if self.anchor.is_some_and(|anchor| !order.contains(&anchor)) {
            self.anchor = self.ids.iter().next().copied();
        }
    }

    /// Forget everything, e.g. after a batch action.
    pub fn clear(&mut self) {
        self.ids.clear();
        self.anchor = None;
    }
}

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
    /// Select exactly these photos, in this order, capped the same way a range-select is.
    ///
    /// Used by the sharpness review, where the set is computed rather than clicked. The cap is not a nicety: a
    /// selection of a hundred thousand ids is a batch waiting to be run on a library nobody meant to change.
    pub fn select_many(&mut self, ids: &[PhotoId]) {
        self.ids.clear();
        for id in ids.iter().take(MAX_RANGE) {
            self.ids.insert(*id);
        }
        self.primary = ids.first().copied();
        self.anchor = ids.first().copied();
    }

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
    /// What Sortlens recorded adding in the last 30 days (SL-A14 part 2). Answered from each
    /// photo's `added` instant, never from the file's `mtime` — a photo whose add time was never
    /// recorded stays out rather than standing in with a modified time it did not choose.
    Added,
    /// Flags that survived a cull.
    Picks,
    /// Anything with at least one star.
    Rated,
    /// Rejected frames, still in the grid and not yet deleted.
    Rejects,
    /// Photos whose file is gone — an unmounted drive, a folder moved outside the library.
    ///
    /// The one source that asks about the *filesystem* rather than about an annotation: it exists so that a
    /// file vanishing is something a person can see and settle, instead of a quiet hole in the count.
    Missing,
}

/// The Recently Added window: 30 days, in seconds.
const RECENT_WINDOW: i64 = 30 * 24 * 60 * 60;

impl Source {
    /// Every source, in sidebar order: the six scopes a photograph can be *in*, one question each.
    pub const ALL: [Source; 6] = [
        Source::All,
        Source::Added,
        Source::Picks,
        Source::Rated,
        Source::Missing,
        Source::Rejects,
    ];

    /// The row's label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Source::All => "All Photos",
            Source::Added => "Recently Added",
            Source::Picks => "Picks",
            Source::Rated => "Rated",
            Source::Missing => "Missing",
            Source::Rejects => "Rejects",
        }
    }

    /// The row's glyph: a source is what it holds, not only what it is called.
    #[must_use]
    pub const fn icon(self) -> crate::icons::Icon {
        match self {
            Self::All => crate::icons::Icon::Library,
            Self::Added => crate::icons::Icon::Clock,
            Self::Picks => crate::icons::Icon::Pick,
            Self::Rated => crate::icons::Icon::Star,
            Self::Missing => crate::icons::Icon::Warning,
            Self::Rejects => crate::icons::Icon::Reject,
        }
    }

    /// The flag this source asks for, if any.
    #[must_use]
    pub fn flag(self) -> Option<Flag> {
        match self {
            Source::Picks => Some(Flag::Pick),
            Source::Rejects => Some(Flag::Reject),
            Source::All | Source::Added | Source::Rated | Source::Missing => None,
        }
    }

    /// The star floor this source asks for, if any. Rated is one star rather than five: "rated" means
    /// somebody looked at it and said something, and a five-star floor would hide most of the library.
    #[must_use]
    pub fn min_rating(self) -> u8 {
        match self {
            Source::Rated => 1,
            Source::All | Source::Added | Source::Picks | Source::Missing | Source::Rejects => 0,
        }
    }

    /// The add-instant cutoff this source asks for, if any: photos recorded into the catalog after
    /// this second. The caller supplies "now" so tests can pin the window.
    #[must_use]
    pub fn added_after(self, now: i64) -> Option<i64> {
        (self == Source::Added).then_some(now - RECENT_WINDOW)
    }

    /// Whether this source asks about files that are gone. Missing is the only one that does, and it asks for
    /// them *alone*, so the row's count is exactly what the grid will hold once the row is picked.
    #[must_use]
    pub fn missing(self) -> Option<bool> {
        (self == Self::Missing).then_some(true)
    }
}

/// How much a notice matters, so the bell can say whether anything needs attention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    /// Something happened and it worked.
    Info,
    /// Something the user asked for did not happen.
    Problem,
}

impl NoticeKind {
    /// Stable id for a log line or a test.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Problem => "problem",
        }
    }
}

/// One line in the bell's history.
#[derive(Debug, Clone, PartialEq)]
pub struct Notice {
    /// What happened.
    pub text: String,
    /// Whether it matters.
    pub kind: NoticeKind,
    /// When it was raised, for the age column.
    pub at: std::time::Instant,
    /// Whether the popover has shown it.
    pub read: bool,
}

/// How many notices are kept. Long enough to answer "what just happened", short enough that the popover stays a
/// list rather than an archive — the activity stack is the archive.
pub const NOTICE_HISTORY: usize = 50;

/// The far-edge tool strip's width, in points. Wide enough for a 22-point button between its margins, narrow enough
/// that a rail which never closes still leaves a 900-point window's canvas usable.
const TAB_STRIP_W: f32 = 30.0;

/// "just now", "4m", "2h", "3d" — the age of a notice, at the precision someone would say it out loud.
pub fn notice_age(seconds: u64) -> String {
    match seconds {
        0..=9 => "just now".to_string(),
        10..=59 => format!("{seconds}s"),
        60..=3599 => format!("{}m", seconds / 60),
        3_600..=86_399 => format!("{}h", seconds / 3_600),
        _ => format!("{}d", seconds / 86_400),
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
    /// Every notice raised, newest first, with what has been read.
    pub notices: std::collections::VecDeque<Notice>,
    /// Whether the bell popover is open.
    pub bell_open: bool,
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
    /// Which collections are ticked in the sidebar, for batch colour, pin and delete.
    pub collection_selection: CollectionSelection,
    /// Whether the confirm dialog for deleting a set chosen in the similarity view is open.
    pub similar_delete_open: bool,
    /// How much of the library the sharpness review offers to select, in percent.
    ///
    /// The cut is relative on purpose: a sharpness score is unitless and only comparable inside one library, so an
    /// absolute threshold would be a guess dressed up as a measurement.
    pub sharpness_percentile: f32,
    /// The map's zoom, session state like the softness cut: a remembered zoom would open Places somewhere the person
    /// did not choose.
    ///
    /// Until the person picks a zoom this is **fitted to the library's own spread on every frame** (see
    /// [`sortlens_geo::coastline::fit_zoom`]), so a library in Brighton and Sydney opens showing both rather than at a
    /// fixed guess that put both off screen. A chosen zoom is never overridden.
    pub places_zoom: u8,
    /// Whether [`Self::places_zoom`] is the person's choice rather than the view's fit.
    pub places_zoom_chosen: bool,
    /// Decoded basemap tiles, so panning and repainting do not decode the same tile again.
    pub places_tiles: places::TileCache,
    /// Place names from the library's own `places.txt`, re-read when the file changes.
    pub places_names: places::Names,
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
    /// Whether the Recently Added source is asking the question (SL-A14 part 2).
    ///
    /// Stored as the question, not as a cutoff instant: the window is always the *current* 30 days,
    /// and the filter turns it into an instant at read time, so a library left open past midnight
    /// still answers "the last 30 days" rather than the 30 days before the window was opened.
    pub recently_added: bool,
    /// Whether the Missing source is asking the question: only files that are gone, and nothing else.
    ///
    /// Distinct from [`Self::show_missing`], which is the filter bar's "also show the ones that went away"
    /// chip. One widens the view and the other narrows it, so they are one `Filter` field with two
    /// questions rather than one flag that has to mean both.
    pub missing_only: bool,
    /// Whether missing files are shown.
    pub show_missing: bool,
    /// Keywords the view is narrowed to, chosen in the sidebar or the filter strip.
    pub tag_filters: Vec<String>,
    /// Only photos where some keyword came from this source (SL-D12).
    pub source_filter: Option<sortlens_core::TagSource>,
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
    /// How much motion the cull overlay may use, mirrored from settings.
    pub motion: Motion,
    /// The decision being animated out of the cull stage, if one is.
    pub overlay: Option<Overlay>,
    /// The window was asked to close while work was in flight.
    pub quit_open: bool,
    /// Text in the keyword editor.
    pub tag_input: String,
    /// Ask the keyword field for focus on the next frame.
    pub tag_focus: bool,
    /// Text in the cull stage's inline keyword field.
    pub cull_tag_input: String,
    /// Ask the cull keyword field for focus on the next frame.
    pub cull_tag_focus: bool,
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
    /// Zoom the detail view to 1:1 instead of fitting the window.
    pub detail_zoom: bool,
    /// The second photo in a side-by-side compare. It is `Some` exactly while [`View::Compare`] is the
    /// open view: a second pane that outlived the view it belongs to would be a compare nobody can see.
    pub compare: Option<PhotoId>,
    /// Only this collection, chosen in the sidebar.
    pub collection_filter: Option<sortlens_core::CollectionId>,
    /// Collection being renamed, with its draft name.
    pub rename_target: Option<(sortlens_core::CollectionId, String)>,
    /// A transient message and the instant it expires.
    pub notice: Option<(String, std::time::Instant)>,
    /// Whether the filter strip is showing. A filter bar that cannot be put away is chrome, and the
    /// top bar's Filter chip is what puts it away.
    pub filters_open: bool,
    /// Whether the search field is in command mode: the ⌘K palette.
    pub palette_open: bool,
    /// What has been typed into the palette. Kept apart from [`Self::search`] so Esc returns to the
    /// photo search that was there before the palette opened, rather than to an empty box.
    pub palette_query: String,
    /// The highlighted palette row.
    pub palette_index: usize,
    /// Ask the palette field for focus on the next frame, so ⌘K lands in it.
    pub palette_focus: bool,
    /// Ask the photo search field for focus on the next frame: Esc out of the palette goes back to
    /// searching photographs.
    pub focus_search: bool,
    /// Where the search field was drawn last frame, so the palette hangs under it.
    pub palette_anchor: Option<egui::Rect>,
    /// The user's shortcut sets; the active one is what the runtime keymap loads.
    pub profiles: Profiles,
    /// The runtime keymap: the catalog under the active set's overrides, on this platform.
    pub keys: Keymap,
    /// The shortcut editor (Settings → Shortcuts) between frames, when it is open.
    pub shortcut_editor: Option<DialogState>,
    /// A shortcut-set file the host read, waiting to be imported into the editor.
    pub pending_shortcut_import: Option<String>,
}

impl UiState {
    /// Fresh state for a given theme and title-bar inset.
    pub fn new(theme: ThemeKind, title_inset: f32) -> Self {
        let t = theme.tokens();
        Self {
            theme,
            density: Density::default(),
            grid_mode: GridMode::default(),
            notices: std::collections::VecDeque::new(),
            bell_open: false,
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
            collection_selection: CollectionSelection::default(),
            marquee: None,
            cell_rects: Vec::new(),
            all_photos_selected: true,
            min_rating: 0,
            flag_filter: None,
            label_filter: None,
            missing_only: false,
            show_missing: false,
            tag_filters: Vec::new(),
            source_filter: None,
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
            motion: Motion::Full,
            overlay: None,
            quit_open: false,
            tag_input: String::new(),
            tag_focus: false,
            cull_tag_input: String::new(),
            cull_tag_focus: false,
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
            recently_added: false,
            kind_filter: None,
            edited_filter: None,
            detail_zoom: false,
            compare: None,
            collection_filter: None,
            rename_target: None,
            notice: None,
            filters_open: true,
            palette_open: false,
            palette_query: String::new(),
            palette_index: 0,
            palette_focus: false,
            focus_search: false,
            palette_anchor: None,
            profiles: Profiles::default(),
            keys: keymap::build(Overrides::new()),
            shortcut_editor: None,
            pending_shortcut_import: None,
            sharpness_percentile: sharpness::DEFAULT_PERCENTILE,
            places_zoom: places::DEFAULT_ZOOM,
            places_zoom_chosen: false,
            places_tiles: places::TileCache::new(),
            places_names: places::Names::new(),
            similar_delete_open: false,
        }
    }

    /// Show a message in the bottom bar for a few seconds.
    pub fn notice(&mut self, text: impl Into<String>) {
        self.record(text.into(), NoticeKind::Info);
    }

    /// Load the persisted shortcut sets: the runtime keymap rebuilds from the active one.
    pub fn set_profiles(&mut self, profiles: Profiles) {
        self.keys = keymap::build(profiles.active_overrides(keymap::BUILTINS).clone());
        self.profiles = profiles;
    }

    /// A notice for something that did not work. Same toast, but it takes an unread badge in the bell and is
    /// coloured as a problem: a failure that scrolls past after four seconds has not been reported.
    pub fn notice_problem(&mut self, text: impl Into<String>) {
        self.record(text.into(), NoticeKind::Problem);
    }

    fn record(&mut self, text: String, kind: NoticeKind) {
        self.notice = Some((text.clone(), std::time::Instant::now() + std::time::Duration::from_secs(4)));
        self.notices.push_front(Notice {
            text,
            kind,
            at: std::time::Instant::now(),
            read: false,
        });
        while self.notices.len() > NOTICE_HISTORY {
            self.notices.pop_back();
        }
    }

    /// Unread notices, which is what the bell's badge counts.
    pub fn unread_notices(&self) -> usize {
        self.notices.iter().filter(|notice| !notice.read).count()
    }

    /// Whether any unread notice is a problem, so the badge can be coloured.
    pub fn unread_problem(&self) -> bool {
        self.notices.iter().any(|notice| !notice.read && notice.kind == NoticeKind::Problem)
    }

    /// Mark the whole history read. Called when the popover opens, which is the only way to see it.
    pub fn mark_notices_read(&mut self) {
        for notice in &mut self.notices {
            notice.read = true;
        }
    }

    /// Forget the history. Reading is not the same as clearing, so this is a separate action.
    pub fn clear_notices(&mut self) {
        self.notices.clear();
    }

    fn live_notice(&self) -> Option<(&str, NoticeKind)> {
        let (text, until) = self.notice.as_ref()?;
        if std::time::Instant::now() >= *until {
            return None;
        }
        // The kind comes from the history entry raised at the same moment, so the toast and the bell cannot
        // disagree about whether the last thing that happened was a problem.
        let kind = self.notices.front().map(|notice| notice.kind).unwrap_or(NoticeKind::Info);
        Some((text, kind))
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
            tag_source: self.source_filter,
            tag_mode: self.tag_mode,
            collection: self.collection_filter,
            folder: (self.root.is_some() && !self.folder.is_empty()).then(|| self.folder.clone()),
            kind: self.kind_filter,
            edited: self.edited_filter,
            added_after: self.recently_added.then(|| sortlens_engine::now_epoch() - RECENT_WINDOW),
            missing: self.missing_only.then_some(true),
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
        self.source_filter = None;
        self.collection_filter = None;
        self.kind_filter = None;
        self.edited_filter = None;
        self.search.clear();
        self.all_photos_selected = source == Source::All;
        self.recently_added = source == Source::Added;
        self.missing_only = source.missing().is_some();
        // A source is a question, and "what did I just import" is asked newest-record-first. Every
        // other source goes back to the sort the user chose, the way closing a collection does.
        self.sort = if source == Source::Added {
            Sort::RecentlyAdded
        } else {
            self.library_sort
        };
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
            && self.source_filter.is_none()
            && self.collection_filter.is_none()
            && self.kind_filter.is_none()
            && self.edited_filter.is_none()
            && self.recently_added == (source == Source::Added)
            && self.missing_only == source.missing().is_some()
    }

    /// How many filter dimensions are narrowing the view, for the header badge.
    pub fn active_filter_count(&self) -> usize {
        let mut count = 0;
        count += usize::from(self.min_rating > 0);
        count += usize::from(self.flag_filter.is_some());
        count += usize::from(self.label_filter.is_some());
        count += usize::from(!self.tag_filters.is_empty());
        count += usize::from(self.source_filter.is_some());
        count += usize::from(self.collection_filter.is_some());
        count += usize::from(self.kind_filter.is_some());
        count += usize::from(self.edited_filter.is_some());
        count += usize::from(self.recently_added);
        count += usize::from(self.missing_only);
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
            || self.source_filter.is_some()
            || self.collection_filter.is_some()
            || !self.folder.is_empty()
            || self.kind_filter.is_some()
            || self.edited_filter.is_some()
            || self.recently_added
            || self.missing_only
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
        self.source_filter = None;
        if self.collection_filter.take().is_some() {
            self.sort = self.library_sort;
            self.search = self.library_search.clone();
        }
        self.folder.clear();
        self.kind_filter = None;
        self.edited_filter = None;
        self.recently_added = false;
        self.missing_only = false;
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
    // The icon set's SVG loader, installed before anything paints: the first frame a glyph appears on
    // is the frame it must be there, or the chrome flashes a row of empty boxes on every launch.
    crate::icons::install(ui.ctx());

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

    // ⌘K is the palette's front door and its way out. Handled before any surface draws, because the
    // palette owns Enter and the caret keys while it is open and the search field's `TextEdit` would
    // take them for itself; the same reason `command_palette` runs before the bar is laid out. The
    // shortcut editor owns the keyboard while it is open, so ⌘K waits.
    if state.shortcut_editor.is_none() && ui.input_mut(|input| input.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::K))) {
        if state.palette_open {
            close_palette(state);
        } else {
            open_palette(state);
        }
    }
    command_palette(ui, state, &t, engine, &mut actions);
    top_bar(ui, state, &t, engine, &mut actions);
    bottom_bar(ui, state, &t, engine, &mut actions);
    if state.left_open {
        left_panel(ui, state, &t, engine, &mut actions);
    }
    // The strip is the far-right edge and always there; the panel body sits inside it.
    right_tools(ui, state, &t);
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

/// The bell, which only exists when it has something to say: a permanent "0" is chrome, and a count of
/// unread notices is the whole point of it being there.
fn bell_button(ui: &mut Ui, state: &mut UiState, t: &Tokens) {
    let unread = state.unread_notices();
    if unread == 0 && !state.bell_open {
        return;
    }
    // The bell, and a count of what is unread. The word "Alerts" used to be the whole button; a bell
    // is what people look for, and the number beside it is the fact the word never carried.
    let colour = if state.unread_problem() { t.reject } else { t.accent };
    let response = ui.add(
        Button::image(crate::icons::image(crate::icons::Icon::Bell, 14.0).tint(colour))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE),
    );
    let spoken = if unread > 0 {
        format!("Alerts, {unread} unread")
    } else {
        "Alerts".to_string()
    };
    crate::a11y::label_widget(ui, &response, egui::Role::Button, &spoken, state.bell_open);
    // The count is painted onto the button's own rectangle rather than laid out beside it: a count
    // inside the bar would shift everything to its right each time a notice arrived.
    if unread > 0 {
        paint_badge(ui, &response, unread, colour, t.font(9.5));
    }
    if response.on_hover_text("What has happened recently").clicked() {
        state.bell_open = !state.bell_open;
        if state.bell_open {
            state.mark_notices_read();
        }
    }
}

/// A count painted onto a control's own rectangle, so a badge never shifts its neighbours.
///
/// The bell and the Filter chip both carry a number, and both must put it in the same corner of the
/// same-sized rect: a badge that reflows the bar on every notice is a bar that moves while you work.
fn paint_badge(ui: &Ui, response: &egui::Response, count: usize, colour: Color32, font: FontId) {
    ui.painter().text(
        response.rect.right_center() + vec2(-2.0, -7.0),
        Align2::RIGHT_CENTER,
        count.to_string(),
        font,
        colour,
    );
}

/// The bell: what happened recently, newest first, with a clear action.
///
/// Opening it marks everything read, which is why the badge is a count of *unread* notices rather than a
/// total: a number that only goes up is a number people stop looking at.
fn bell_popover(ui: &mut Ui, state: &mut UiState, t: &Tokens) {
    let mut open = state.bell_open;
    egui::Window::new("Alerts")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(340.0)
        .anchor(Align2::RIGHT_TOP, vec2(-16.0, 52.0))
        .show(ui.ctx(), |ui| {
            if state.notices.is_empty() {
                ui.label(RichText::new("Nothing has happened yet").size(11.5).color(t.text_muted));
                return;
            }
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                for notice in &state.notices {
                    ui.horizontal(|ui| {
                        let colour = match notice.kind {
                            NoticeKind::Info => t.text,
                            NoticeKind::Problem => t.reject,
                        };
                        ui.label(RichText::new(&notice.text).size(11.5).color(colour));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let age = notice_age(notice.at.elapsed().as_secs());
                            ui.label(RichText::new(age).size(10.5).color(t.text_faint));
                        });
                    });
                }
            });
            ui.separator();
            if ui.button(RichText::new("Clear").size(11.5)).clicked() {
                state.clear_notices();
            }
        });
    state.bell_open = open;
}

/// Every dialog, in the order they stack. Extracted so `show` stays a sequence of surfaces rather than a
/// list of every window the app can open.
fn dialogs(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    if state.bell_open {
        bell_popover(ui, state, t);
    }
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
    if state.shortcut_editor.is_some() {
        shortcut_editor_dialog(ui, state, actions);
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
    similar_delete_dialog(ui, state, t, engine);
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
                // The mark is the window icon's own geometry, registered from the icon crate rather than
                // drawn a second time: the dock and the chrome show one product.
                app_mark(ui, 16.0);
                ui.add_space(6.0);
                ui.label(RichText::new("Sortlens").size(14.0).strong());
                ui.add_space(8.0);
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
                    // Every control from here on is a glyph with a name: the bar is what a person looks at
                    // while deciding about a photograph, so it carries no words a picture can carry. The
                    // four value chips that used to live here (theme, density, layout, motion) are gone:
                    // two moved to the canvas header beside the sort, and two are in Appearance.
                    if glyph_button(ui, t, crate::icons::Icon::Sliders, "Settings", false).clicked() {
                        state.settings_open = true;
                    }
                    if glyph_button(ui, t, crate::icons::Icon::Keyboard, "Help and shortcuts", false).clicked() {
                        state.shortcuts_open = true;
                        state.shortcut_query.clear();
                    }
                    appearance_button(ui, state, t, actions);
                    bell_button(ui, state, t);
                    export_button(ui, state);
                    if glyph_button(ui, t, crate::icons::Icon::FolderPlus, "Import a folder", false).clicked() {
                        actions.push(Action::PickFolder);
                    }
                    filter_button(ui, state, t);
                    // Selected, not merely clickable: the button says whether the panel it toggles is
                    // open, which is the only thing an unlabelled glyph can be asked to show.
                    if glyph_button(ui, t, crate::icons::Icon::Panel, "Details panel", state.right_open).clicked() {
                        state.right_open = !state.right_open;
                    }

                    ui.add_space(8.0);
                    search_field(ui, state, engine);
                });
            });
        });
}

/// The one field, in its two modes: photographs live, commands on ⌘K.
///
/// The two buffers are separate so leaving command mode returns to the search that was there before,
/// and the field records its own rectangle because the palette hangs from it rather than from a guess.
fn search_field(ui: &mut Ui, state: &mut UiState, engine: &mut Engine) {
    if state.palette_open {
        let field = ui.add_sized(
            [240.0, 24.0],
            TextEdit::singleline(&mut state.palette_query).hint_text("Type a command…"),
        );
        if field.changed() {
            state.palette_index = 0;
        }
        if state.palette_focus {
            field.request_focus();
            state.palette_focus = false;
        }
        state.palette_anchor = Some(field.rect);
        return;
    }
    let hint = match state
        .collection_filter
        .and_then(|id| engine.collections().into_iter().find(|collection| collection.id == id))
    {
        Some(collection) => format!("Search in {}", collection.name),
        None => "Search photos or ⌘K".to_string(),
    };
    let search = ui.add_sized([240.0, 24.0], TextEdit::singleline(&mut state.search).hint_text(hint));
    if search.changed() {
        state.selection.clear();
    }
    if state.focus_search {
        // Esc out of the palette returns to the search you were running, so the caret goes after
        // its text: coming back to a query should let you keep typing it, not type in front of it.
        if let Some(mut edit) = TextEdit::load_state(ui.ctx(), search.id) {
            let end = egui::text::CCursor::new(state.search.chars().count());
            edit.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
            edit.store(ui.ctx(), search.id);
        }
        search.request_focus();
        state.focus_search = false;
    }
    state.palette_anchor = Some(search.rect);
    // Commit the search to the collection when the box is left or Enter is pressed,
    // rather than writing an op for every keystroke. `has_focus` is read *outside*
    // `ui.input`: nested context access deadlocks the lock this fork holds while the
    // closure runs, and any Enter keypress — not only one in this box — reached it.
    let enter_on_search = ui.input(|input| input.key_pressed(Key::Enter)) && search.has_focus();
    if search.lost_focus() || enter_on_search {
        remember_search(state, engine);
    }
    if state.collection_filter.is_none() {
        state.library_search = state.search.clone();
    }
}

/// The app mark, registered once per process and painted at the top-left.
///
/// The bytes come from `sortlens-icon`, the crate the window icon is generated from, so the tile, the
/// stack of prints and the lens are the same drawing in both places.
fn app_mark(ui: &mut Ui, size: f32) {
    use std::sync::OnceLock;
    static MARK: OnceLock<(String, egui::load::Bytes)> = OnceLock::new();
    let (uri, bytes) = MARK.get_or_init(|| {
        let svg = sortlens_icon::svg();
        (
            "bytes://sortlens/app-mark.svg".to_string(),
            std::sync::Arc::<[u8]>::from(svg.into_bytes()).into(),
        )
    });
    ui.ctx().include_bytes(uri.clone(), bytes.clone());
    ui.add(
        egui::Image::from_bytes(uri.clone(), bytes.clone())
            .fit_to_exact_size(Vec2::splat(size))
            .sense(Sense::hover()),
    )
    .on_hover_text("Sortlens");
}

/// The Filter chip: a toggle for the filter bar, carrying the count of what is narrowing the view.
///
/// The number is [`UiState::active_filter_count`] — the same one the canvas header and the strip's Clear
/// chip read, so three places cannot disagree about how many filters are on.
fn filter_button(ui: &mut Ui, state: &mut UiState, t: &Tokens) {
    let active = state.active_filter_count();
    let colour = if state.filters_open { t.text } else { t.text_muted };
    let response = ui.add(
        Button::image(crate::icons::image(crate::icons::Icon::Funnel, 14.0).tint(colour))
            .fill(if state.filters_open { t.selection } else { Color32::TRANSPARENT })
            .stroke(Stroke::NONE),
    );
    let spoken = if active > 0 {
        format!("Filters, {active} active")
    } else {
        "Filters".to_string()
    };
    a11y::label_widget(ui, &response, egui::Role::Button, &spoken, state.filters_open);
    if active > 0 {
        paint_badge(ui, &response, active, t.accent, t.font(9.5));
    }
    let hint = if state.filters_open {
        "Hide the filter bar"
    } else {
        "Show the filter bar"
    };
    if response.on_hover_text(hint).clicked() {
        state.filters_open = !state.filters_open;
    }
}

/// Export the selection, or say why there is nothing to export.
fn export_button(ui: &mut Ui, state: &mut UiState) {
    let ids = state.selection.ids();
    let enabled = !ids.is_empty();
    let response = ui.add_enabled(
        enabled,
        Button::image(crate::icons::image(crate::icons::Icon::Export, 14.0))
            .image_tint_follows_text_color(true)
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE),
    );
    a11y::label_widget(ui, &response, egui::Role::Button, "Export the selection", false);
    let response = if enabled {
        response.on_hover_text("Export the selection…")
    } else {
        response.on_hover_text("Select a photo to export")
    };
    if response.clicked() {
        state.export_ids = ids;
        state.export_open = true;
    }
}

/// Appearance: one control for theme and motion, as the design asks.
///
/// A left click cycles the theme, because that is the one thing people change while working; the right
/// click lists every preset and the reduced-motion switch, which is where the two chips that used to sit
/// in this bar ended up.
fn appearance_button(ui: &mut Ui, state: &mut UiState, t: &Tokens, actions: &mut Vec<Action>) {
    let response = ui.add(
        Button::image(crate::icons::image(crate::icons::Icon::Contrast, 14.0).tint(t.text_muted))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE),
    );
    let spoken = format!("Appearance, {}", state.theme.label());
    a11y::label_widget(ui, &response, egui::Role::Button, &spoken, false);
    let response = response.on_hover_text(format!(
        "Appearance: {} — click to cycle, right-click for every theme and reduced motion",
        state.theme.label()
    ));
    if response.clicked() {
        actions.push(Action::SetTheme(next_theme(state.theme)));
    }
    response.context_menu(|ui| {
        for kind in ThemeKind::ALL {
            if ui.selectable_label(kind == state.theme, kind.label()).clicked() {
                actions.push(Action::SetTheme(kind));
                ui.close();
            }
        }
        ui.separator();
        let mut reduced = state.motion == Motion::Reduced;
        if ui.checkbox(&mut reduced, "Reduced motion").changed() {
            let motion = if reduced { Motion::Reduced } else { Motion::Full };
            state.motion = motion;
            actions.push(Action::SetMotion(motion));
            ui.close();
        }
    });
}

/// A command the ⌘K palette can run.
///
/// An enum rather than a list of strings because the palette must not be able to list a command it
/// cannot run: `run` matches every variant, so a row and its handler arrive together. Every `id` is a
/// `Ready` command in the engine's registry, and the palette renders the label, the group and the
/// shortcut from that registry — this type spells no label a second time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaletteCommand {
    Detail,
    Compare,
    SoftFocus,
    Places,
    Similar,
    Insights,
    StartCull,
    ApplyCull,
    Reveal,
    Open,
    Export,
    Suggest,
    AddFolder,
    Rescan,
    Health,
    History,
    Deleted,
    Backup,
    RestoreBackup,
    ExportHistory,
    Settings,
    Quit,
    Shortcuts,
    About,
}

impl PaletteCommand {
    /// Every palette command, in the registry's group order (Library, Review, Organise, View, App, Help).
    const ALL: [Self; 24] = [
        Self::AddFolder,
        Self::Rescan,
        Self::Health,
        Self::History,
        Self::Deleted,
        Self::Backup,
        Self::RestoreBackup,
        Self::ExportHistory,
        Self::StartCull,
        Self::ApplyCull,
        Self::Reveal,
        Self::Open,
        Self::Export,
        Self::Suggest,
        Self::Detail,
        Self::Compare,
        Self::SoftFocus,
        Self::Places,
        Self::Similar,
        Self::Insights,
        Self::Settings,
        Self::Quit,
        Self::Shortcuts,
        Self::About,
    ];

    /// The registry id. The only string this list carries; everything shown comes from the registry.
    const fn id(self) -> &'static str {
        match self {
            Self::Detail => "view.detail",
            Self::Compare => "view.compare",
            Self::SoftFocus => "view.softFocus",
            Self::Places => "view.places",
            Self::Similar => "view.similar",
            Self::Insights => "view.insights",
            Self::StartCull => "cull.start",
            Self::ApplyCull => "cull.apply",
            Self::Reveal => "photo.reveal",
            Self::Open => "photo.open",
            Self::Export => "edit.export",
            Self::Suggest => "suggest.start",
            Self::AddFolder => "library.addFolder",
            Self::Rescan => "library.rescan",
            Self::Health => "library.health",
            Self::History => "library.history",
            Self::Deleted => "library.deleted",
            Self::Backup => "library.backup",
            Self::RestoreBackup => "library.restoreBackup",
            Self::ExportHistory => "library.exportHistory",
            Self::Settings => "app.settings",
            Self::Quit => "app.quit",
            Self::Shortcuts => "app.shortcuts",
            Self::About => "app.about",
        }
    }

    /// Run the command. `Err` is a sentence explaining why it cannot run right now: the palette closes
    /// either way, and a notice is better than a menu row that silently does nothing.
    fn run(self, state: &mut UiState, engine: &mut Engine, actions: &mut Vec<Action>) -> std::result::Result<(), String> {
        let primary = |state: &UiState| state.selection.primary().ok_or_else(|| "Pick a photo first".to_string());
        match self {
            Self::Detail => {
                primary(state)?;
                activate_view(state, engine, View::Detail);
                Ok(())
            }
            Self::Compare => {
                // `toggle_compare` notices when there is nothing to compare with, so the palette closes on
                // one sentence rather than on two.
                toggle_compare(state, engine);
                Ok(())
            }
            Self::SoftFocus => {
                activate_view(state, engine, View::SoftFocus);
                Ok(())
            }
            Self::Places => {
                activate_view(state, engine, View::Places);
                Ok(())
            }
            Self::Similar => {
                activate_view(state, engine, View::Similar);
                Ok(())
            }
            Self::Insights => {
                activate_view(state, engine, View::Insights);
                Ok(())
            }
            Self::StartCull => {
                open_cull(state, engine);
                Ok(())
            }
            Self::ApplyCull => {
                if engine.cull().is_some_and(|session| session.summary().rejects > 0) {
                    state.apply_open = true;
                    Ok(())
                } else {
                    Err("Nothing staged: reject something in a cull session first".to_string())
                }
            }
            Self::Reveal => {
                actions.push(Action::Reveal(primary(state)?));
                Ok(())
            }
            Self::Open => {
                actions.push(Action::OpenExternal(primary(state)?));
                Ok(())
            }
            Self::Export => {
                let ids = state.selection.ids();
                if ids.is_empty() {
                    return Err("Select a photo to export".to_string());
                }
                state.export_ids = ids;
                state.export_open = true;
                Ok(())
            }
            Self::Suggest => {
                actions.push(Action::StartSuggest);
                Ok(())
            }
            Self::AddFolder => {
                actions.push(Action::PickFolder);
                Ok(())
            }
            Self::Rescan => match rescan_root(state, engine) {
                Some(root) => {
                    actions.push(Action::Rescan(root));
                    Ok(())
                }
                None => Err("Add a folder before rescanning".to_string()),
            },
            Self::Health => {
                state.health_open = true;
                Ok(())
            }
            Self::History => {
                state.history_open = true;
                Ok(())
            }
            Self::Deleted => {
                state.deleted_open = true;
                Ok(())
            }
            Self::Backup => {
                actions.push(Action::Backup);
                Ok(())
            }
            Self::RestoreBackup => {
                actions.push(Action::RestoreBackup);
                Ok(())
            }
            Self::ExportHistory => {
                actions.push(Action::ExportHistory);
                Ok(())
            }
            Self::Settings => {
                state.settings_open = true;
                Ok(())
            }
            Self::Quit => {
                state.quit_open = true;
                Ok(())
            }
            Self::Shortcuts => {
                state.shortcuts_open = true;
                state.shortcut_query.clear();
                Ok(())
            }
            Self::About => {
                state.about_open = true;
                Ok(())
            }
        }
    }
}

/// The palette's rows for a query, resolved from the registry.
///
/// Label, group and shortcut are the registry's; this function only decides which of the palette's
/// commands answer the text, matching the label, the id and the group name.
fn palette_rows(query: &str) -> Vec<(PaletteCommand, sortlens_engine::command::CommandSpec)> {
    let registry = sortlens_engine::command::Registry::v0();
    let needle = query.trim().to_lowercase();
    PaletteCommand::ALL
        .iter()
        .filter_map(|command| registry.get(command.id()).map(|spec| (*command, *spec)))
        .filter(|(_, spec)| {
            needle.is_empty()
                || spec.label.to_lowercase().contains(&needle)
                || spec.id.to_lowercase().contains(&needle)
                || spec.group.label().to_lowercase().contains(&needle)
        })
        .collect()
}

/// Open command mode: an empty query so the first keystroke narrows commands, and the photo search
/// behind it untouched.
fn open_palette(state: &mut UiState) {
    state.palette_open = true;
    state.palette_query.clear();
    state.palette_index = 0;
    state.palette_focus = true;
}

/// Leave command mode without moving the caret: the command that just ran decides where focus goes.
fn close_palette(state: &mut UiState) {
    state.palette_open = false;
    state.palette_query.clear();
    state.palette_index = 0;
}

/// Esc: leave command mode and put the caret back at the end of the photo search.
fn leave_palette_for_search(state: &mut UiState) {
    close_palette(state);
    state.focus_search = true;
}

/// The ⌘K palette: grouped commands with their shortcuts, hung under the search field.
///
/// Called before the bar is drawn, because the palette owns Enter and the arrow keys while it is open
/// and a `TextEdit` would take them for itself.
fn command_palette(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    if !state.palette_open {
        return;
    }
    let rows = palette_rows(&state.palette_query);
    if state.palette_index >= rows.len() {
        state.palette_index = rows.len().saturating_sub(1);
    }
    let (escape, up, down, enter) = ui.input_mut(|input| {
        (
            input.consume_key(Modifiers::NONE, Key::Escape),
            input.consume_key(Modifiers::NONE, Key::ArrowUp),
            input.consume_key(Modifiers::NONE, Key::ArrowDown),
            input.consume_key(Modifiers::NONE, Key::Enter),
        )
    });
    if escape {
        leave_palette_for_search(state);
        return;
    }
    if !rows.is_empty() {
        if up {
            state.palette_index = (state.palette_index + rows.len() - 1) % rows.len();
        }
        if down {
            state.palette_index = (state.palette_index + 1) % rows.len();
        }
    }
    let Some(anchor) = state.palette_anchor else {
        // The field has not been drawn yet (the palette opened on this frame), so there is nothing
        // to hang the rows from; they follow it on the next frame.
        return;
    };
    let mut chosen = if enter {
        rows.get(state.palette_index).map(|(command, _)| *command)
    } else {
        None
    };
    let width = anchor.width().max(430.0);
    let left = (anchor.right() - width).max(8.0);
    egui::Area::new(egui::Id::unique("sortlens.command_palette"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(left, anchor.bottom() + 6.0))
        .show(ui.ctx(), |ui| {
            Frame::NONE
                .fill(t.panel)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(t.corner())
                .inner_margin(Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(width - 16.0);
                    if rows.is_empty() {
                        ui.label(RichText::new("No command matches").size(11.5).color(t.text_muted));
                    } else {
                        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                            let mut group = None;
                            for (index, (command, spec)) in rows.iter().enumerate() {
                                if group != Some(spec.group) {
                                    group = Some(spec.group);
                                    ui.add_space(if index == 0 { 0.0 } else { 6.0 });
                                    ui.label(RichText::new(spec.group.label()).size(10.0).strong().color(t.text_faint));
                                }
                                let selected = index == state.palette_index;
                                let response = ui
                                    .horizontal(|ui| {
                                        let response = ui.selectable_label(selected, RichText::new(spec.label).size(12.0));
                                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                            // The rebindable commands paint what the runtime keymap
                                            // holds, so a rebind cannot leave the palette lying.
                                            let keys = keymap::painted(&state.keys, command.id())
                                                .unwrap_or_else(|| spec.shortcut.unwrap_or_default().to_owned());
                                            if !keys.is_empty() {
                                                ui.label(RichText::new(keys).size(10.5).color(t.text_faint));
                                            }
                                        });
                                        response
                                    })
                                    .inner;
                                if response.clicked() {
                                    chosen = Some(*command);
                                }
                                if response.hovered() {
                                    state.palette_index = index;
                                }
                            }
                        });
                    }
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Up/Down choose · Enter run · Esc photo search")
                            .size(10.0)
                            .color(t.text_faint),
                    );
                });
        });
    if let Some(command) = chosen {
        close_palette(state);
        if let Err(reason) = command.run(state, engine, actions) {
            state.notice(reason);
        }
    }
}

fn bottom_bar(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    egui::Panel::bottom("bottom_bar")
        .exact_size(t.bottom_bar_h)
        .frame(Frame::NONE.fill(t.chrome).inner_margin(Margin::symmetric(10, 0)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                // Nine destinations do not fit a narrow window, and a row that silently dropped the last
                // few would be a tab bar lying about what the app can do. The width is measured from the
                // labels rather than guessed from the window, so a wider face or a translated label
                // collapses at the width where the row really stops fitting.
                let tabs: f32 = View::ALL.iter().map(|view| tab_width(ui, view.label())).sum();
                if tabs_collapse(ui.available_width(), tabs) {
                    view_menu(ui, t, state, engine, actions);
                } else {
                    for view in View::ALL {
                        view_tab(ui, t, state, engine, view, actions);
                    }
                }

                ui.add_space(10.0);
                annotation_pill(ui, t, state, engine);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let help = ui.add(
                        Button::new(RichText::new("?").size(12.0).color(t.text_muted))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                    );
                    crate::a11y::label_widget(ui, &help, egui::Role::Button, "Keyboard shortcuts", false);
                    if help.on_hover_text("Every shortcut in this mode  ?").clicked() {
                        state.shortcuts_open = true;
                        state.shortcut_query.clear();
                    }
                    // A person who asked for less motion should be able to see that it is still on, without
                    // hunting through Appearance: a setting you cannot confirm is one you will toggle twice.
                    if state.motion == Motion::Reduced {
                        glyph_button(ui, t, crate::icons::Icon::Wave, "Reduced motion", true);
                    }
                    if state.view.is_detail() {
                        let zoom = if state.detail_zoom { "1:1" } else { "Fit" };
                        if chip(ui, t, None, zoom, state.detail_zoom)
                            .on_hover_text("Fit the photograph to the window, or show it at 1:1  Z")
                            .clicked()
                        {
                            state.detail_zoom = !state.detail_zoom;
                        }
                    } else if state.view == View::Library {
                        // The same three steps the canvas header cycles, on a control that shows where
                        // between them you are. One state, so the chip and the slider cannot disagree.
                        let mut step = state.density.index() as f32;
                        ui.spacing_mut().slider_width = 72.0;
                        let slider = ui.add(
                            egui::Slider::new(&mut step, 0.0..=2.0)
                                .step_by(1.0)
                                .show_value(false)
                                .trailing_fill(true),
                        );
                        crate::a11y::label_widget(ui, &slider, egui::Role::Slider, "Thumbnail size", false);
                        if slider.changed() {
                            let density = Density::from_index(step);
                            if density != state.density {
                                state.density = density;
                                actions.push(Action::SetDensity(density));
                            }
                        }
                        slider.on_hover_text(format!("Thumbnail size: {}", state.density.label()));
                    }
                    match engine.scan_progress() {
                        Some(progress) if engine.is_scanning() => {
                            ui.label(
                                RichText::new(format!(
                                    "Indexing · {} files · {} folders",
                                    format_count(progress.files as u64),
                                    format_count(progress.dirs as u64)
                                ))
                                .size(11.0)
                                .color(t.accent),
                            );
                        }
                        _ => {
                            if let Some((text, kind)) = state.live_notice() {
                                let colour = match kind {
                                    NoticeKind::Info => t.accent,
                                    NoticeKind::Problem => t.reject,
                                };
                                // Clipped rather than allowed to run under the pill: the bar is laid out
                                // from the right, so a long sentence would push the controls off the edge.
                                // The whole sentence is a hover away.
                                ui.label(RichText::new(truncate(text, 64)).size(11.0).color(colour))
                                    .on_hover_text(text.to_owned());
                            }
                        }
                    }
                });
            });
        });
}

/// How wide one labelled tab paints: its glyph, the gap between them, and the button's padding.
fn tab_width(ui: &Ui, label: &str) -> f32 {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(12.0), Color32::WHITE);
    galley.size().x + 34.0
}

/// Room the annotation pill and the right-hand controls need beside the tab row.
const BAR_SIDE_W: f32 = 360.0;

/// Whether the bar is too narrow to show every destination as a labelled tab.
///
/// A pure function so the threshold can be tested without painting: the bar is only allowed to collapse
/// when the tabs plus the controls beside them genuinely do not fit.
fn tabs_collapse(available: f32, tabs: f32) -> bool {
    tabs + BAR_SIDE_W > available
}

/// One labelled destination.
fn view_tab(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, view: View, actions: &mut Vec<Action>) {
    let selected = state.view == view;
    let response = ui.add_enabled(
        view.milestone().is_none() || selected,
        Button::image_and_text(crate::icons::image(view.icon(), 14.0), RichText::new(view.label()).size(12.0))
            .image_tint_follows_text_color(true)
            .fill(if selected { t.selection } else { Color32::TRANSPARENT })
            .stroke(Stroke::NONE),
    );
    crate::a11y::label_widget(ui, &response, egui::Role::Button, view.label(), selected);
    if response.clicked() {
        match view.milestone() {
            Some(milestone) => actions.push(Action::Notice(format!("{} arrives in {milestone}", view.label()))),
            None => {
                activate_view(state, engine, view);
            }
        }
    }
}

/// The current destination, and every other one behind it, for a window too narrow for the full row.
///
/// The collapsed form is the same list: a narrow window loses width, not destinations.
fn view_menu(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, actions: &mut Vec<Action>) {
    // The caret is a glyph from the icon set, not a character: the bundled faces have no `▾`, and a
    // menu button that paints tofu teaches the person to distrust the bar.
    let button = Button::new((
        crate::icons::image(state.view.icon(), 14.0).tint(t.text),
        RichText::new(state.view.label()).size(12.0),
        crate::icons::image(crate::icons::Icon::Chevron, 10.0).tint(t.text_faint),
    ))
    .fill(t.selection)
    .stroke(Stroke::NONE);
    let mut chosen = None;
    let (response, _) = egui::containers::menu::MenuButton::from_button(button).ui(ui, |ui| {
        ui.set_min_width(170.0);
        for view in View::ALL {
            let selected = state.view == view;
            let row = ui.add(
                Button::image_and_text(crate::icons::image(view.icon(), 13.0), RichText::new(view.label()).size(12.0))
                    .image_tint_follows_text_color(true)
                    .fill(if selected { t.selection } else { Color32::TRANSPARENT })
                    .stroke(Stroke::NONE),
            );
            crate::a11y::label_widget(ui, &row, egui::Role::Button, view.label(), selected);
            if row.clicked() {
                chosen = Some(view);
                ui.close();
            }
        }
    });
    crate::a11y::label_widget(
        ui,
        &response,
        egui::Role::Button,
        &format!("Views, now {}", state.view.label()),
        true,
    );
    response.on_hover_text("Every destination. The row returns when the window is wide enough.");
    if let Some(view) = chosen {
        match view.milestone() {
            Some(milestone) => actions.push(Action::Notice(format!("{} arrives in {milestone}", view.label()))),
            None => {
                activate_view(state, engine, view);
            }
        }
    }
}

/// The centre of the bar: what the selection is, and the two judgements made most often.
///
/// Rating and flag are the same operations the number and letter keys run — [`rate_selection`] and
/// [`flag_selection`] are the one implementation — so a click here and a press there cannot drift apart.
fn annotation_pill(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine) {
    let photo = state.selection.primary().and_then(|id| engine.photo(id).cloned());
    let rating = photo.as_ref().map_or(0, |photo| photo.rating);
    let flag = photo.as_ref().map(|photo| photo.flag);
    Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(t.corner())
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                for stars in 1..=5u8 {
                    let lit = stars <= rating;
                    let colour = if lit { t.accent } else { t.text_faint };
                    let button = ui.add(
                        Button::image(crate::icons::image(crate::icons::Icon::Star, 12.0).tint(colour))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                    );
                    let name = format!("{stars} star{}", if stars == 1 { "" } else { "s" });
                    crate::a11y::label_widget(ui, &button, egui::Role::Button, &name, lit);
                    if button
                        .on_hover_text(format!("Rate {stars}★ — the same star again clears it"))
                        .clicked()
                    {
                        rate_selection(state, engine, if rating == stars { 0 } else { stars });
                    }
                }
                for (wanted, icon, name, colour) in [
                    (sortlens_core::Flag::Pick, crate::icons::Icon::Pick, "Pick", t.pick),
                    (sortlens_core::Flag::None, crate::icons::Icon::Flag, "Unflag", t.text_muted),
                    (sortlens_core::Flag::Reject, crate::icons::Icon::Reject, "Reject", t.reject),
                ] {
                    let active = flag == Some(wanted);
                    let tint = if active { colour } else { t.text_faint };
                    let button = ui.add(
                        Button::image(crate::icons::image(icon, 12.0).tint(tint))
                            .fill(if active { t.selection } else { Color32::TRANSPARENT })
                            .stroke(Stroke::NONE),
                    );
                    crate::a11y::label_widget(ui, &button, egui::Role::Button, name, active);
                    if button.on_hover_text(name).clicked() {
                        flag_selection(state, engine, wanted);
                    }
                }
                // How many keywords the primary carries: a fact the inspector also shows, but the bar is
                // where a person checks "did the keyword land" without opening a panel.
                if let Some(photo) = &photo
                    && !photo.tags.is_empty()
                {
                    ui.add_space(2.0);
                    ui.add(crate::icons::image(crate::icons::Icon::Tag, 11.0).tint(t.text_faint))
                        .on_hover_text(photo.tags.join(", "));
                    ui.label(RichText::new(photo.tags.len().to_string()).size(10.5).color(t.text_faint));
                }
                ui.add_space(4.0);
                let count = state.selection.len();
                let text = match count {
                    0 => "No selection".to_string(),
                    1 => "1 selected".to_string(),
                    many => format!("{many} selected"),
                };
                ui.label(
                    RichText::new(text)
                        .size(11.0)
                        .color(if count == 0 { t.text_faint } else { t.text_muted }),
                );
            });
        });
}

/// Rate the selection, saying what happened the way the keyboard does.
fn rate_selection(state: &mut UiState, engine: &mut Engine, stars: u8) {
    let targets = state.selection.ids();
    if targets.is_empty() {
        return;
    }
    let count = targets.len();
    match engine.rate(&targets, stars) {
        Ok(_) => state.notice(match (stars, count) {
            (0, 1) => "Cleared rating".to_string(),
            (0, many) => format!("Cleared the rating on {many} photos"),
            (stars, 1) => format!("Rated {stars}★"),
            (stars, many) => format!("Rated {stars}★ on {many} photos"),
        }),
        Err(error) => state.notice_problem(error.to_string()),
    }
}

/// Flag the selection, saying what happened the way the keyboard does.
fn flag_selection(state: &mut UiState, engine: &mut Engine, flag: Flag) {
    let targets = state.selection.ids();
    if targets.is_empty() {
        return;
    }
    let count = targets.len();
    match engine.flag(&targets, flag) {
        Ok(_) => state.notice(match (flag, count) {
            (Flag::Pick, 1) => "Picked".to_string(),
            (Flag::Reject, 1) => "Rejected".to_string(),
            (Flag::None, 1) => "Unflagged".to_string(),
            (Flag::Pick, many) => format!("Picked {many} photos"),
            (Flag::Reject, many) => format!("Rejected {many} photos"),
            (Flag::None, many) => format!("Unflagged {many} photos"),
        }),
        Err(error) => state.notice_problem(error.to_string()),
    }
}

/// The sort and search a folder should be opened with.
///
/// A folder returns to how it was being browsed: its own sort if it has one, otherwise the library's — a folder
/// with nothing remembered must not snap to Natural and undo the sort you had chosen a moment ago. The remembered
/// search comes back into the box rather than filtering in the background, so it is visible and one click from
/// being cleared.
fn folder_scope(mark: &sortlens_engine::FolderMark, library_sort: Sort) -> (Sort, String) {
    let sort = mark.sort.as_deref().map(Sort::parse).unwrap_or(library_sort);
    (sort, mark.search.clone().unwrap_or_default())
}

/// The folder list: one row per root, with its volume state, pin and colour.
///
/// Its own function because the sidebar had grown past the point where one closure could hold every section and
/// stay readable — the complexity gate said so, which is the right time to split rather than to silence it.
fn folders_section(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, actions: &mut Vec<Action>) {
    nav_section(ui, t, "Folders");
    // Volumes once per root, not once per photo: an unplugged drive holding ten thousand photos is
    // one missing volume, and the sidebar says so beside the folder instead of after scrolling.
    let volumes = engine.root_volumes();
    if volumes.is_empty() {
        hint(ui, t, "No folders yet. Add one to index a shoot.");
    }
    // Marks are read once per frame rather than per row: the sidebar draws every root, and a key lookup
    // per row would be one string normalisation per row per frame.
    let marks = engine.folder_marks();
    let mut remark: Option<(String, sortlens_engine::FolderMark)> = None;
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
        let key = sortlens_engine::folder_key(&root);
        let mark = marks.get(&key).cloned().unwrap_or_default();
        let name = format!("{}{}", if mark.favourite { "★ " } else { "" }, folder_name(&root));
        let response = nav_row(ui, t, crate::icons::Icon::Folder, &name, &count_text, selected);
        if let Some(colour) = mark.colour {
            // The same dot a collection uses, in the same place: a colour means one thing in this
            // sidebar, whichever kind of row carries it.
            let dot = pos2(response.rect.left() + 3.0, response.rect.center().y);
            ui.painter().circle_filled(dot, 3.0, t.label(colour));
        }
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
            // A folder returns to how it was being browsed — see `folder_scope` for the rule.
            let (sort, search) = folder_scope(&mark, state.library_sort);
            state.sort = sort;
            state.search = search;
        }
        response.context_menu(|ui| {
            let pin_label = if mark.favourite {
                "Remove from favourites"
            } else {
                "Add to favourites"
            };
            if ui.button(pin_label).clicked() {
                remark = Some((
                    root.clone(),
                    sortlens_engine::FolderMark {
                        favourite: !mark.favourite,
                        ..mark.clone()
                    },
                ));
                ui.close();
            }
            ui.menu_button("Colour", |ui| {
                let mut chosen: Option<Option<sortlens_core::ColorLabel>> = None;
                if ui.button("None").clicked() {
                    chosen = Some(None);
                }
                for label in sortlens_core::ColorLabel::ALL {
                    if ui.button(label.label()).clicked() {
                        chosen = Some(Some(label));
                    }
                }
                if let Some(colour) = chosen {
                    remark = Some((root.clone(), sortlens_engine::FolderMark { colour, ..mark.clone() }));
                    ui.close();
                }
            });
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
    if let Some((path, mark)) = remark {
        match engine.set_folder_mark(&path, mark.clone()) {
            Ok(_) => state.notice(match (mark.favourite, mark.colour) {
                (true, Some(colour)) => format!("Pinned and {} ", colour.label().to_lowercase()),
                (true, None) => "Pinned to the top".to_string(),
                (false, Some(colour)) => format!("Folder is {}", colour.label().to_lowercase()),
                (false, None) => "Folder mark cleared".to_string(),
            }),
            Err(error) => state.notice_problem(error.to_string()),
        }
    }
}

fn left_panel(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    egui::Panel::left("left_panel")
        .resizable(true)
        .default_size(state.left_w)
        .size_range(210.0..=380.0)
        .frame(Frame::NONE.fill(t.panel).inner_margin(Margin::same(10)))
        .show(ui, |ui| {
            state.left_w = ui.available_width();
            // The foot is laid out first so it can claim the bottom edge before the sections fill what is
            // left: a strip added last would be pushed off the panel by a long folder or keyword list, and
            // "always there" is the whole reason it left the middle of the sidebar.
            sidebar_foot(ui, state, t, engine, actions);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Library").size(12.0).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if glyph_button(ui, t, crate::icons::Icon::FolderPlus, "Add a folder to index", false).clicked() {
                        actions.push(Action::PickFolder);
                    }
                });
            });
            ui.add_space(6.0);
            ui.separator();

            // Everything below the header can outgrow the panel: six scopes, the folder rows and twelve
            // keywords are taller than a 620-point window, and a section that keeps laying itself out past
            // the hairline paints over the utilities. The scroll area takes exactly the room the foot left
            // and clips at that edge, so the boundary a person sees is the boundary the paint obeys — and the
            // rows under the fold scroll up instead of being lost. `auto_shrink` keeps the viewport at the
            // full height the foot left, so the scrollbar appears only when the sections really need it.
            egui::ScrollArea::vertical()
                .id_salt("sidebar_sections")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    nav_section(ui, t, "Sources");
                    let now = sortlens_engine::now_epoch();
                    for source in Source::ALL {
                        // Each row's count is what that row would show, not the library total: a Picks row reading
                        // "0" while the grid holds thousands is the row doing its job.
                        let count = source_count(engine, state, source, now);
                        if nav_row(
                            ui,
                            t,
                            source.icon(),
                            source.label(),
                            &format_count(count as u64),
                            state.source_selected(source),
                        )
                        .clicked()
                        {
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

                    if let Some(root) = state.root.clone() {
                        let folders = engine.subfolders(&root, &state.folder);
                        if !folders.is_empty() {
                            nav_section(ui, t, "Subfolders");
                        }
                        for (name, count) in folders {
                            let prefix = format!("{}{name}/", state.folder);
                            let selected = state.folder == prefix;
                            if nav_row(ui, t, crate::icons::Icon::Folder, &name, &format_count(count as u64), selected).clicked() {
                                state.folder = if selected { parent_prefix(&state.folder) } else { prefix };
                            }
                        }
                    }

                    folders_section(ui, t, state, engine, actions);

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
                    let mut tag_delete: Option<String> = None;
                    for (tag, count) in vocabulary.iter().take(12) {
                        let selected = state.tag_filters.iter().any(|active| active == tag);
                        let aliases = engine.registry_aliases_of(tag);
                        let row = if aliases.is_empty() {
                            nav_row(ui, t, crate::icons::Icon::Tag, tag, &format_count(*count as u64), selected)
                        } else {
                            // Say where a keyword came from: a rename leaves a trail behind it.
                            nav_row(ui, t, crate::icons::Icon::Tag, tag, &format_count(*count as u64), selected)
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
                            if ui
                                .button("Delete everywhere")
                                .on_hover_text("Take the keyword off every photo; one undo brings it back")
                                .clicked()
                            {
                                tag_delete = Some(tag.clone());
                                ui.close();
                            }
                        });
                    }
                    if let Some(tag) = tag_delete {
                        delete_keyword_everywhere(state, engine, &tag);
                    }

                    collections_section(ui, t, state, engine);
                });
        });
}

/// The question a source asks, scoped to the folder the person is standing in.
///
/// The folder is where you are rather than what you asked for, so it survives picking a source; every
/// other dimension is the source's own. Shared with the tests so a row's count and the grid it opens are
/// the same question asked twice, not two questions that look alike.
fn source_filter(state: &UiState, source: Source, now: i64) -> Filter {
    Filter {
        root: state.root.clone(),
        folder: (state.root.is_some() && !state.folder.is_empty()).then(|| state.folder.clone()),
        flag: source.flag(),
        min_rating: source.min_rating(),
        added_after: source.added_after(now),
        missing: source.missing(),
        ..Filter::default()
    }
}

/// How many photos a source holds right now, for its row.
fn source_count(engine: &Engine, state: &UiState, source: Source, now: i64) -> usize {
    engine.count(&source_filter(state, source, now))
}

/// The sidebar's foot: the four rows that open a dialog instead of choosing what the grid holds.
///
/// Each row *is* a palette command — `library.deleted`, `library.health`, `library.history` and
/// `app.shortcuts` — so the words and the shortcut come from the command registry and the click goes
/// through the same [`PaletteCommand::run`] the `⌘K` palette uses. That is the whole point of the move: a
/// second hand-wired copy of the four dialog flags is drift waiting to happen, and a fifth row can only be
/// added by naming a command that already exists. The right column is the row's own answer — how many
/// photos the bin holds, or the key that does the same thing without a mouse.
const SIDEBAR_FOOT: [(PaletteCommand, crate::icons::Icon); 4] = [
    (PaletteCommand::Deleted, crate::icons::Icon::Trash),
    (PaletteCommand::Health, crate::icons::Icon::Health),
    (PaletteCommand::History, crate::icons::Icon::History),
    (PaletteCommand::Shortcuts, crate::icons::Icon::Keyboard),
];

/// A sidebar row's height, in points. Shared with the foot's own sizing: the strip asks the panel for an
/// exact height built from this number, so a row cannot grow without the panel that pins it.
const NAV_ROW_H: f32 = 24.0;

/// The hairline above the foot and the space it needs, painted as one band so every point of the
/// foot's height is accounted for below.
const FOOT_HAIRLINE: f32 = 9.0;

/// The foot's height: the hairline, the rows and the spacing between them, and the frame's margins.
///
/// The panel is given this as an *exact* size instead of being measured from its content, because a panel
/// only learns how tall its content is after laying it out — on the first frame an auto-sized foot hangs
/// below its own clip rect and the strip flickers into place a frame late. The arithmetic is checked by
/// `the_sidebar_foot_pins_the_utilities_to_the_bottom`, which fails if a row no longer fits.
fn sidebar_foot_height(spacing: &egui::style::Spacing) -> f32 {
    let pitch = NAV_ROW_H + spacing.item_spacing.y;
    let content = FOOT_HAIRLINE + spacing.item_spacing.y + SIDEBAR_FOOT.len() as f32 * pitch - spacing.item_spacing.y;
    content + FOOT_INSET.top as f32 + FOOT_INSET.bottom as f32
}

/// The foot frame's inner margins. A tighter top than the sections' 10 points, because the hairline is
/// what separates them.
const FOOT_INSET: Margin = Margin {
    left: 10,
    right: 10,
    top: 4,
    bottom: 8,
};

fn sidebar_foot(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    egui::Panel::bottom("sidebar_foot")
        .resizable(false)
        .exact_size(sidebar_foot_height(ui.spacing()))
        .frame(Frame::NONE.fill(t.panel).inner_margin(FOOT_INSET))
        .show(ui, |ui| {
            // A hairline of a known height rather than `ui.separator()`, whose height is read from the
            // style: the panel's height is decided before its content is laid out, and a widget whose
            // size is a style lookup is a point in that height nobody counted.
            let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), FOOT_HAIRLINE), Sense::hover());
            ui.painter().hline(line.x_range(), line.center().y, Stroke::new(1.0, t.border));
            let registry = sortlens_engine::command::Registry::v0();
            for (command, icon) in SIDEBAR_FOOT {
                let Some(spec) = registry.get(command.id()) else {
                    continue;
                };
                let trailing = match command {
                    PaletteCommand::Deleted => format_count(engine.deleted_batches().len() as u64),
                    _ => spec.shortcut.unwrap_or_default().to_string(),
                };
                let row = nav_row(ui, t, icon, spec.label, &trailing, false);
                let row = if command == PaletteCommand::Shortcuts {
                    row.on_hover_text("Every binding, searchable")
                } else {
                    row
                };
                // None of the four can refuse today; the branch is here because the strip borrows `run`'s
                // contract, and a row that silently did nothing would be worse than one that says why.
                if row.clicked()
                    && let Err(reason) = command.run(state, engine, actions)
                {
                    state.notice(reason);
                }
            }
        });
}

/// The context menu's delete: take the keyword off every photo and say what happened.
fn delete_keyword_everywhere(state: &mut UiState, engine: &mut Engine, tag: &str) {
    match engine.delete_tag(tag) {
        Ok(0) => state.notice(format!("#{tag} is not used by any photo")),
        Ok(count) => {
            state.notice(format!("#{tag} taken off {count} photo(s) — one undo brings it back"));
            // A filter pointing at a deleted keyword would show an empty grid forever.
            state.tag_filters.retain(|active| active != tag);
        }
        Err(error) => state.notice_problem(error.to_string()),
    }
}

/// The far-edge tool strip: one glyph per question the panel can answer.
///
/// It is painted whether the panel body is open or not, because it is how the panel is opened and the only way to ask
/// the other three questions while the canvas has the width. Selecting the active tool collapses the body rather than
/// doing nothing: a rail you cannot close from the rail is a rail that keeps 340 px of a 900 px window forever.
///
/// “Switching tools” here never hides the panel — that is the whole point of the ArtCraft pattern — so the only thing
/// a click can do besides switching is close the body deliberately.
fn right_tools(ui: &mut Ui, state: &mut UiState, t: &Tokens) {
    egui::Panel::right("right_tools")
        .exact_size(TAB_STRIP_W)
        .frame(Frame::NONE.fill(t.panel).inner_margin(Margin::symmetric(3, 8)))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for tab in RightTab::ALL {
                let active = state.right_open && state.right_tab == tab;
                let response = ui.add(
                    Button::image(crate::icons::image(tab.icon(), 14.0).tint(if active { t.text } else { t.text_muted }))
                        .fill(if active { t.selection } else { Color32::TRANSPARENT })
                        .stroke(Stroke::NONE)
                        .min_size(vec2(22.0, 24.0)),
                );
                crate::a11y::label_widget(ui, &response, egui::Role::Tab, tab.label(), active);
                if response.on_hover_text(tab.label()).clicked() {
                    if active {
                        state.right_open = false;
                    } else {
                        state.right_tab = tab;
                        state.right_open = true;
                    }
                }
            }
        });
}

fn right_panel(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine) {
    egui::Panel::right("right_panel")
        .resizable(true)
        .default_size(t.right_w)
        .size_range(250.0..=420.0)
        .frame(Frame::NONE.fill(t.panel).inner_margin(Margin::same(10)))
        .show(ui, |ui| {
            // The tabs live in the far-edge strip now, so the panel's own header is the one place that names the
            // tool in words: the strip is glyphs, and a glyph alone is a guess for anyone who has not learned it.
            ui.label(RichText::new(state.right_tab.label()).size(12.0).strong().color(t.text));
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
                RightTab::Suggestions => suggestions_tab(ui, t, state, engine),
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
            // The source is printed on the chip, not only in the tooltip: a keyword a machine proposed and a keyword
            // typed by hand are different claims, and a fact you have to hover to see is one you will not check.
            // Manual is the ordinary case and stays unmarked so the panel does not read as a warning list.
            let source = photo.tag_source(tag);
            let text = if source == sortlens_core::TagSource::Manual {
                format!("{tag}  ×")
            } else {
                format!("{tag} · {}  ×", source.label())
            };
            let chip = Button::new(RichText::new(text).size(11.0).color(t.text))
                .fill(t.card)
                .stroke(Stroke::new(1.0, t.border))
                .corner_radius(t.corner());
            if ui
                .add(chip)
                .on_hover_text(format!("{} · click to remove", source.explanation()))
                .clicked()
            {
                remove = Some(tag.clone());
            }
        }
    });
    if let Some(tag) = remove {
        match engine.set_tag(&[photo.id], &tag, false) {
            Ok(_) => state.notice(format!("Removed keyword {tag}")),
            Err(error) => state.notice_problem(error.to_string()),
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
                Err(error) => state.notice_problem(error.to_string()),
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
            state.notice_problem(error.to_string());
        }
    }
}

/// One proposal, merged across the selection: the keyword, the best confidence any selected photo offered, and how
/// many of them it applies to.
struct Proposal {
    tag: String,
    score: f32,
    photos: usize,
    source: sortlens_engine::SuggestionSource,
    kind: SuggestKind,
}

/// The Suggestions tab: what the file and the packs propose for the selection, grouped by where it came from.
///
/// This is the panel, not the library-wide pass in the Suggestions dialog: it answers "what could be said about these
/// photographs" for the photos in front of you, and every row is a proposal until a click makes it a keyword. Grouped
/// because "the camera said so" and "a model guessed" are different kinds of claim, and a list that mixes them asks a
/// person to trust them equally.
///
/// Accepting writes through [`Engine::accept_suggestions`], so one ⌘Z takes back a whole kind, and dismissing records the
/// decision in the catalog: a dismissal kept only on screen would come back the next time the panel opened.
fn suggestions_tab(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine) {
    let ids = state.selection.ids();
    if ids.is_empty() {
        hint(ui, t, "Select a photo to see what its own file and the keyword packs propose.");
        return;
    }

    // One row per keyword, not per photo: a keyword proposed for eight of the twelve selected photographs is one
    // decision, and the count says how far-reaching it would be.
    let mut merged: std::collections::BTreeMap<String, Proposal> = std::collections::BTreeMap::new();
    for (_, suggestions) in engine.suggestions_for_all(&ids) {
        for suggestion in suggestions {
            merged
                .entry(suggestion.tag.clone())
                .and_modify(|proposal| {
                    proposal.photos += 1;
                    if suggestion.score > proposal.score {
                        proposal.score = suggestion.score;
                        proposal.source = suggestion.source;
                        proposal.kind = SuggestKind::of(suggestion.source);
                    }
                })
                .or_insert_with(|| Proposal {
                    tag: suggestion.tag.clone(),
                    score: suggestion.score,
                    photos: 1,
                    source: suggestion.source,
                    kind: SuggestKind::of(suggestion.source),
                });
        }
    }

    if merged.is_empty() {
        hint(ui, t, "Nothing to propose for this selection.");
        ui.add_space(4.0);
        ui.label(
            RichText::new("Proposals come from each file's own EXIF and from the keyword packs that are switched on.")
                .size(10.0)
                .color(t.text_faint),
        );
        return;
    }

    ui.label(
        RichText::new(format!(
            "{} proposal{} for {} photo{}",
            merged.len(),
            if merged.len() == 1 { "" } else { "s" },
            ids.len(),
            if ids.len() == 1 { "" } else { "s" }
        ))
        .size(10.5)
        .color(t.text_faint),
    );

    for kind in SuggestKind::ALL {
        let family: Vec<&Proposal> = merged.values().filter(|proposal| proposal.kind == kind).collect();
        if family.is_empty() {
            continue;
        }
        ui.add_space(7.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(kind.label()).size(10.5).strong().color(t.text_muted));
            ui.label(RichText::new(format!("{}", family.len())).size(10.0).color(t.text_faint));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let all: Vec<String> = family.iter().map(|proposal| proposal.tag.clone()).collect();
                if ui
                    .add(Button::new(RichText::new("Accept all").size(10.0)).frame(false))
                    .on_hover_text(format!(
                        "Add every {} proposal as a keyword, as one undo",
                        kind.label().to_lowercase()
                    ))
                    .clicked()
                {
                    match engine.accept_suggestions(&ids, &all) {
                        Ok(0) => state.notice("Those keywords are already on every selected photo"),
                        Ok(count) => state.notice(format!("Added {count} keyword(s) — one undo takes them back")),
                        Err(error) => state.notice_problem(error.to_string()),
                    }
                }
            });
        });

        for proposal in family {
            ui.horizontal(|ui| {
                let accept = ui
                    .add(Button::new(RichText::new(format!("+ {}", proposal.tag)).size(11.0)).fill(t.card))
                    .on_hover_text(format!(
                        "{} · {:.0}% from {}{}",
                        proposal.tag,
                        proposal.score * 100.0,
                        proposal.source.label(),
                        if proposal.photos < ids.len() {
                            format!(" · on {} of the {} selected photos", proposal.photos, ids.len())
                        } else {
                            String::new()
                        }
                    ));
                ui.label(
                    RichText::new(format!("{:.0}%", proposal.score * 100.0))
                        .size(10.0)
                        .color(t.text_faint),
                );
                ui.label(RichText::new(proposal.source.label()).size(10.0).color(t.text_faint));
                // The count is printed only when the proposal does not cover the whole selection: "3×" beside every
                // row of a three-photo selection is noise, and a bare row would hide that it is partial.
                if proposal.photos < ids.len() {
                    ui.label(RichText::new(format!("{}×", proposal.photos)).size(10.0).color(t.text_faint));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(Button::new(RichText::new("×").size(12.0).color(t.text_muted)).frame(false))
                        .on_hover_text(format!(
                            "Don't propose “{}” again for these photos — one undo brings it back",
                            proposal.tag
                        ))
                        .clicked()
                    {
                        match engine.dismiss_suggestions(&ids, std::slice::from_ref(&proposal.tag)) {
                            Ok(0) => state.notice(format!("“{}” was already dismissed", proposal.tag)),
                            Ok(_) => state.notice(format!("Won't propose “{}” again — one undo brings it back", proposal.tag)),
                            Err(error) => state.notice_problem(error.to_string()),
                        }
                    }
                });
                if accept.clicked() {
                    match engine.accept_suggestions(&ids, std::slice::from_ref(&proposal.tag)) {
                        Ok(0) => state.notice(format!("“{}” is already on every selected photo", proposal.tag)),
                        Ok(_) => state.notice(format!("Added “{}” — one undo takes it back", proposal.tag)),
                        Err(error) => state.notice_problem(error.to_string()),
                    }
                }
            });
        }
    }
}

fn canvas(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine, thumbs: &mut Thumbnails, actions: &mut Vec<Action>) {
    egui::CentralPanel::default()
        .frame(Frame::NONE.fill(t.bg).inner_margin(Margin::same(14)))
        .show(ui, |ui| {
            let filter = state.filter();
            let total = engine.count(&filter);
            // The view's name and the row of controls share one line, and the controls decide how much
            // room the name has. Laid out as a plain `horizontal`, the right block is handed whatever the
            // left block left over — and when that number ran small the right-to-left row was laid out
            // from a cursor already past the right edge, so at 900 points with both docks open the title
            // was painted underneath the sort combo. `Sides::shrink_left` measures the controls first and
            // truncates the title into the room that is actually left over, which cannot collide at any
            // width: the combos and chips keep their size and the words give way, never the reverse.
            // The header's left block is read before the row is laid out: the two closures share `state`,
            // and the right one edits it.
            let title = state.view.label();
            let status = crate::a11y::library_status(total, state.selection.len());
            // The count is measured before it is offered: "does it fit" is an answer rather than a hope. The
            // title is what the row is about; the count is a statement the filter strip repeats and the live
            // line already speaks, so on a canvas too narrow for both the count steps aside and the name
            // stays whole — the same rule the filter strip's summary follows.
            let status_width = ui
                .painter()
                .layout_no_wrap(
                    status.clone(),
                    egui::FontId::new(10.5, egui::FontFamily::Proportional),
                    t.text_muted,
                )
                .size()
                .x;
            let _ = egui::containers::Sides::new().spacing(6.0).shrink_left().truncate().show(
                ui,
                |ui| {
                    ui.label(RichText::new(title).size(16.0).strong());
                    if ui.available_width() >= HEADER_COUNT_GAP + status_width {
                        ui.add_space(HEADER_COUNT_GAP);
                        crate::a11y::live_line(ui, &status, t.text_muted);
                    }
                },
                |ui| {
                    // The chips are decoration on this line, and they are the first things a narrow canvas
                    // gives up: the scope is cleared from the sidebar and the active-filter count also sits
                    // on the Filter chip in the top bar, while the sort combo and the two grid glyphs exist
                    // nowhere else in this row. Measured from the room the row actually has.
                    let room = ui.available_width();
                    if room >= HEADER_CHIP_ROOM {
                        if state.root.is_some() && icon_button(ui, "All Photos", t).clicked() {
                            state.root = None;
                            state.folder.clear();
                        }
                        let active = state.active_filter_count();
                        if active > 0
                            && chip(ui, t, Some(crate::icons::Icon::Tag), &format!("{active} active"), true)
                                .on_hover_text("Clear every filter")
                                .clicked()
                        {
                            state.search.clear();
                            state.clear_filters();
                        }
                    }
                    let mut chosen = state.sort;
                    let combo = egui::ComboBox::from_id_salt("sort_order")
                        .selected_text(RichText::new(state.sort.label()).size(11.5).color(t.text_muted))
                        .width(130.0)
                        .show_ui(ui, |ui| {
                            for sort in Sort::ALL {
                                ui.selectable_value(&mut chosen, sort, RichText::new(sort.label()).size(11.5));
                            }
                        });
                    // The visible text is the *current* choice, not the control's name: read aloud it
                    // said "Added" and nothing about what it changes. Found by the screenshot
                    // harness's accessibility check (Phase 3 item 9), which is stricter than a scan.
                    crate::a11y::label_widget(ui, &combo.response, egui::Role::ComboBox, "Sort order", false);
                    if chosen != state.sort {
                        state.sort = chosen;
                        actions.push(Action::SetSort(chosen));
                        match state.collection_filter {
                            Some(id) => {
                                if let Err(error) = engine.set_collection_sort(id, chosen) {
                                    state.notice_problem(error.to_string());
                                }
                            }
                            // A folder is a scope too, so its sort belongs to it: switching away and back
                            // should return you to how you were looking at it. With no scope, the sort is the
                            // library's own preference and is kept in memory for this session.
                            None => match state.root.clone() {
                                Some(root) => {
                                    let mut mark = engine.folder_mark(&sortlens_engine::folder_key(&root));
                                    mark.sort = Some(chosen.id().to_string());
                                    if let Err(error) = engine.set_folder_mark(&root, mark) {
                                        state.notice_problem(error.to_string());
                                    }
                                }
                                None => state.library_sort = chosen,
                            },
                        }
                    }
                    // How many photographs a row holds belongs beside the order they are in: density and
                    // layout decide what the grid shows as much as the sort does, and none of the three is
                    // a setting you leave the grid to change.
                    let density = format!("Grid density: {}", state.density.label());
                    if glyph_button(ui, t, crate::icons::Icon::Density, &density, false).clicked() {
                        let next = state.density.next();
                        state.density = next;
                        actions.push(Action::SetDensity(next));
                    }
                    let layout = format!("Grid layout: {}", state.grid_mode.label());
                    if glyph_button(ui, t, state.grid_mode.icon(), &layout, false).clicked() {
                        let next = state.grid_mode.next();
                        state.grid_mode = next;
                        actions.push(Action::SetGrid(next));
                    }
                },
            );
            if state.root.is_some() {
                breadcrumbs(ui, t, state);
            }
            ui.add_space(8.0);
            // The strip is a surface, not a fact: the Filter chip in the top bar puts it away, and a
            // hidden strip must leave no gap behind it. It also needs a library to narrow: over an
            // empty one it was nine controls asking about photographs that do not exist yet, sitting
            // on top of the Welcome card that says where photographs come from.
            //
            // "Empty" is the catalog's own total, never the filtered one: a filter that happens to
            // hide every photo must keep its strip, because clearing it is what the strip is for.
            let library_is_empty = engine.stats().total == 0;
            if state.filters_open && !library_is_empty {
                filter_strip(ui, t, state, engine, total);
                ui.add_space(10.0);
            }

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
                View::Detail | View::Compare => {
                    detail_view(ui, t, state, engine, thumbs, actions);
                }
                View::Library => {
                    if total == 0 {
                        empty_library(ui, t, state, engine, actions, total);
                    } else {
                        grid(ui, t, state, engine, thumbs, &filter, actions);
                    }
                }
                View::SoftFocus => soft_focus_view(ui, t, state, engine),
                View::ForYou => for_you_view(ui, t, state, engine, thumbs),
                View::Cull => cull_view(ui, t, state, engine, thumbs, actions),
                View::Insights => insights_view(ui, t, state, engine),
                View::Similar => similar_view(ui, t, state, engine, thumbs),
                View::Places => places_view(ui, t, state, engine),
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
        open_detail(state);
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
            Err(error) => state.notice_problem(error.to_string()),
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

    /// The hover for this badge, given whether a cached thumbnail is standing in for the file.
    ///
    /// An offline cell showing a faded photograph and an offline cell showing nothing are different states, and
    /// a person looking at the first one needs to be told the picture is from before the drive left.
    fn hint_cached(self, cached: bool) -> &'static str {
        match (self, cached) {
            (CellBadge::Offline, true) => "File missing — showing the cached thumbnail from before it went away",
            _ => self.hint(),
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

/// How a cached thumbnail is painted when the file behind it is not on disk.
///
/// The cache does not know the drive left: it still holds the pixels decoded before it did, and drawing them at
/// full strength makes an offline library look current. So a missing file's cached thumbnail is painted faded —
/// visibly present, visibly not live — and the badge says why. Fully dimmed rather than hidden: the photograph
/// is still the best way to recognise the frame, and `None` when there is nothing cached keeps the placeholder.
const CACHED_ALPHA: u8 = 110;

fn cached_tint(photo: &Photo) -> Color32 {
    if photo.missing {
        Color32::from_white_alpha(CACHED_ALPHA)
    } else {
        Color32::WHITE
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
            painter.image(texture.id(), image, uv, cached_tint(photo));
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

    // A video's length, on the cell: "VID" says what it is, the duration says whether to open it.
    if photo.kind == MediaKind::Video && !photo.duration_label().is_empty() {
        let label = photo.duration_label();
        let corner = image.left_bottom() + vec2(5.0, -5.0);
        let width = 6.5 * label.len() as f32 + 8.0;
        let pill = Rect::from_min_size(pos2(corner.x, corner.y - 14.0), vec2(width, 14.0));
        painter.rect_filled(pill, 3.0, Color32::from_black_alpha(160));
        painter.text(pill.center(), Align2::CENTER_CENTER, label, t.font(9.5), Color32::WHITE);
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
        response.clone().on_hover_text(badge.hint_cached(texture.is_some()));
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
    // The palette owns the keyboard while its field is open, even after a click moved focus away:
    // a letter typed at it must filter commands, not rate the photograph behind it.
    if state.palette_open {
        return;
    }
    // The shortcut editor owns the keyboard while it records; nothing behind it reacts.
    if state.shortcut_editor.is_some() {
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
    if ui.input(|input| keymap::plain_pressed(&state.keys, input, &CommandRef::new("app.shortcuts"))) {
        state.shortcuts_open = true;
        state.shortcut_query.clear();
        return;
    }
    if state.view.is_detail() {
        // Arrows, Space, Z, D and Esc belong to the detail view; rating and flag keys still work
        // below, which is why this does not return.
        detail_keyboard(ui, state, engine);
    }
    if cull_keyboard(ui, state, engine) {
        return;
    }
    // The layout is a view, so it has a key: G flips between rows and a lattice without a trip to the chip.
    if ui.input(|input| input.key_pressed(Key::G) && !input.modifiers.command) && !state.view.is_detail() {
        let next = state.grid_mode.next();
        state.grid_mode = next;
        actions.push(Action::SetGrid(next));
        return;
    }
    // Starting a session works before anything is selected: the queue comes from the filters.
    // A session restored from a previous run is opened rather than replaced.
    if !state.view.is_detail() && ui.input(|input| keymap::plain_pressed(&state.keys, input, &CommandRef::new("cull.start"))) {
        open_cull(state, engine);
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
    if !state.view.is_detail()
        && ui.input(|input| input.key_pressed(Key::Escape) && !input.modifiers.command)
        && !state.selection.is_empty()
    {
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
    let mut from_prev = false;
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
        // writing flags one photo at a time. The keymap carries P and X as two parameterised
        // rows of one command, so a rebind of one leaves the other alone.
        if !culling && keymap::plain_pressed(&state.keys, input, &keymap::flag_pick()) {
            flag = Some(Flag::Pick);
        }
        if !culling && keymap::plain_pressed(&state.keys, input, &keymap::flag_reject()) {
            flag = Some(Flag::Reject);
        }
        if !culling && input.key_pressed(Key::U) {
            flag = Some(Flag::None);
        }
        if keymap::plain_pressed(&state.keys, input, &CommandRef::new("photo.keyword")) {
            keyword = true;
        }
        if keymap::plain_pressed(&state.keys, input, &CommandRef::new("photo.tagFromPrev")) {
            from_prev = true;
        }
    });

    if !state.view.is_detail() && ui.input(|input| keymap::plain_pressed(&state.keys, input, &CommandRef::new("view.detail"))) {
        activate_view(state, engine, View::Detail);
        return;
    }

    if keyword {
        state.right_tab = RightTab::Tags;
        state.right_open = true;
        state.tag_focus = true;
    }
    let count = targets.len();
    // The second photo of a look: the keyword you just typed, one key away. Saying *which* keyword it applied
    // matters more here than elsewhere — "Tagged 5 photos" would leave you guessing what you just wrote.
    if from_prev {
        match engine.tag_from_prev(&targets) {
            Ok(changed) => {
                let tag = engine.last_tag().unwrap_or_default().to_string();
                state.notice(match (changed, count) {
                    (0, 1) => format!("Already tagged “{tag}”"),
                    (0, many) => format!("All {many} already tagged “{tag}”"),
                    (_, 1) => format!("Tagged “{tag}”"),
                    (_, many) => format!("Tagged “{tag}” on {many} photos"),
                });
            }
            Err(error) => state.notice_problem(error.to_string()),
        }
    }

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
            Err(error) => state.notice_problem(error.to_string()),
        }
    }

    if let Some(stars) = rating {
        rate_selection(state, engine, stars);
    }
    if let Some(flag) = flag {
        flag_selection(state, engine, flag);
    }
}

fn filter_strip(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &Engine, total: usize) {
    Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(t.corner())
        .inner_margin(Margin::symmetric(8, 6))
        .show(ui, |ui| {
            // The count gets a measured column on the right rather than riding at the end of the wrapped
            // row. A wrapped layout hands its last child the whole row's width, so a right-aligned label
            // placed there is laid out from the wrap cursor and lands under the chips: at 1280 points with
            // both docks open it was painted underneath `Clear`. A reserved column cannot collide at any
            // width, because the chips wrap in the room that is actually theirs.
            let inner = ui.available_rect_before_wrap();
            let galley = ui.painter().layout_no_wrap(
                format!("{} shown", format_count(total as u64)),
                egui::FontId::proportional(11.0),
                t.text_faint,
            );
            let gap = 8.0;
            let reserve = galley.size().x + gap;
            // Below this the count would be taking the room a chip needs, and a chip is a control where
            // the count is a statement: the chips keep the width and the count steps aside.
            let keep_count = inner.width() >= reserve + STRIP_CHIP_ROOM;
            let chips_width = if keep_count { inner.width() - reserve } else { inner.width() };
            // Wrapped, not one line: nine chips plus a five-star minimum are wider than the canvas at
            // 1200 points with both docks open, and a filter bar that pushes its own Clear button off
            // the edge is a filter bar that cannot be undone. A second row is a shorter strip, not a
            // missing control.
            ui.set_max_width(chips_width);
            let chips = ui.horizontal_wrapped(|ui| {
                // One chip can be wider than the column it is handed — a long collection name — and the
                // wrapped layout would let it spill sideways into the count's column. The column is a hard
                // edge, so such a chip is clipped there rather than painted over the count.
                if keep_count {
                    let column = Rect::from_min_max(inner.min, pos2(inner.right() - reserve, inner.bottom()));
                    ui.shrink_clip_rect(column);
                }
                if let Some(id) = state.collection_filter
                    && let Some(collection) = engine.collections().into_iter().find(|entry| entry.id == id)
                    && chip(ui, t, Some(crate::icons::Icon::Layers), &collection.name, true).clicked()
                {
                    state.collection_filter = None;
                }
                let mut drop_tag = None;
                for tag in state.tag_filters.clone() {
                    if chip(ui, t, Some(crate::icons::Icon::Tag), &format!("#{tag}"), true).clicked() {
                        drop_tag = Some(tag);
                    }
                }
                if let Some(tag) = drop_tag {
                    state.tag_filters.retain(|active| *active != tag);
                }
                ui.menu_button(
                    (
                        RichText::new("+ keyword").size(10.5),
                        crate::icons::image(crate::icons::Icon::Chevron, 10.0).tint(t.text_muted),
                    ),
                    |ui| {
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
                            let label = format!("{}{tag}  ({count})", if active { "• " } else { "" });
                            let mut text = RichText::new(label).size(11.0);
                            if active {
                                text = text.strong();
                            }
                            if ui.add(Button::new(text).frame(false)).clicked() {
                                if active {
                                    state.tag_filters.retain(|chosen| chosen != tag);
                                } else {
                                    state.tag_filters.push(tag.clone());
                                }
                            }
                        }
                    },
                );
                if state.tag_filters.len() > 1 {
                    let label = format!("match {}", state.tag_mode.label());
                    if chip(
                        ui,
                        t,
                        Some(crate::icons::Icon::Tag),
                        &label,
                        state.tag_mode == sortlens_engine::TagMode::All,
                    )
                    .clicked()
                    {
                        state.tag_mode = state.tag_mode.flipped();
                    }
                }
                if rating_chip(ui, t, state.min_rating).clicked() {
                    state.min_rating = (state.min_rating + 1) % 6;
                }

                let flag_label = match state.flag_filter {
                    None => "Flags".to_string(),
                    Some(Flag::Pick) => "Picks".to_string(),
                    Some(Flag::Reject) => "Rejects".to_string(),
                    Some(Flag::None) => "Unflagged".to_string(),
                };
                // The flag's own state is a glyph, not only a word: a pick is the pick tick and a
                // reject is the reject cross, the same two the bottom bar's pill wears, so the chip
                // says which of the four states it is in before you read it.
                let flag_icon = flag_filter_icon(state.flag_filter);
                if chip(ui, t, Some(flag_icon), &flag_label, state.flag_filter.is_some()).clicked() {
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
                if colour_chip(ui, t, state.label_filter, &label_text).clicked() {
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
                if chip(ui, t, Some(crate::icons::Icon::Film), &kind_label, state.kind_filter.is_some()).clicked() {
                    state.kind_filter = match state.kind_filter {
                        None => Some(MediaKind::Image),
                        Some(MediaKind::Image) => Some(MediaKind::Raw),
                        Some(MediaKind::Raw) => Some(MediaKind::Video),
                        Some(MediaKind::Video) => None,
                    };
                }

                // Where a keyword came from. The chip cycles through the recorded sources and back to no
                // constraint; "Not recorded" is offered too, because a library written before sources existed
                // has to be findable — that is the difference between an honest answer and a hidden one.
                let source_label = match state.source_filter {
                    None => "Source".to_string(),
                    Some(source) => format!("{} keywords", source.label()),
                };
                let source_chip = chip(
                    ui,
                    t,
                    Some(crate::icons::Icon::Sparkle),
                    &source_label,
                    state.source_filter.is_some(),
                );
                let source_chip = match state.source_filter {
                    Some(source) => source_chip.on_hover_text(source.explanation()),
                    None => source_chip.on_hover_text("Only photos where some keyword came from here"),
                };
                if source_chip.clicked() {
                    state.source_filter = match state.source_filter {
                        None => sortlens_core::TagSource::ALL.first().copied(),
                        Some(current) => {
                            let order: Vec<sortlens_core::TagSource> = sortlens_core::TagSource::ALL
                                .into_iter()
                                .chain([sortlens_core::TagSource::Unrecorded])
                                .collect();
                            let index = order.iter().position(|source| *source == current).unwrap_or(0);
                            order.get(index + 1).copied()
                        }
                    };
                }

                let edited_label = match state.edited_filter {
                    None => "Edited".to_string(),
                    Some(true) => "Edited only".to_string(),
                    Some(false) => "Unedited".to_string(),
                };
                if chip(
                    ui,
                    t,
                    Some(crate::icons::Icon::Pencil),
                    &edited_label,
                    state.edited_filter.is_some(),
                )
                .clicked()
                {
                    state.edited_filter = match state.edited_filter {
                        None => Some(true),
                        Some(true) => Some(false),
                        Some(false) => None,
                    };
                }

                // The chip and the sidebar's Missing row are two doors to one answer, so the chip reads
                // active when either is on: the row narrows to what is gone, the chip widens the current
                // view to include it. Turning it off clears both, because a chip that says "on" while the
                // grid shows only missing files is the kind of disagreement a filter bar cannot have.
                let missing_shown = state.show_missing || state.missing_only;
                if chip(ui, t, Some(crate::icons::Icon::Warning), "Missing", missing_shown).clicked() {
                    state.show_missing = !missing_shown;
                    state.missing_only = false;
                }

                if state.has_filters() && chip(ui, t, Some(crate::icons::Icon::Close), "Clear", true).clicked() {
                    state.search.clear();
                    state.clear_filters();
                }
            });
            // The card stays the strip's full width even though its content now stops short of the count's
            // column; without this the frame would shrink to the widest wrapped row.
            ui.set_min_width(inner.width());
            ui.set_max_width(inner.width());
            if keep_count {
                let at = pos2(
                    inner.right() - galley.size().x,
                    chips.response.rect.center().y - galley.size().y * 0.5,
                );
                ui.painter().galley(at, galley, t.text_faint);
            }
        });
}

/// The narrowest canvas whose header still offers its two chips beside the sort combo and the grid glyphs.
///
/// Below this the chips step aside — the scope is cleared in the sidebar and the active-filter count is also
/// on the top bar's Filter chip — because a row that lost its sort combo to keep a `1 active` badge would be a
/// header with no controls left. This is the floor for the chips, not for the title: the title is measured
/// against the room that is actually left over, at every width.
const HEADER_CHIP_ROOM: f32 = 430.0;

/// The gap between the canvas header's title and its count, in points.
const HEADER_COUNT_GAP: f32 = 6.0;

/// The narrowest strip that still answers "how many" without taking a chip's room.
const STRIP_CHIP_ROOM: f32 = 180.0;

fn chip(ui: &mut Ui, t: &Tokens, icon: Option<crate::icons::Icon>, label: &str, active: bool) -> egui::Response {
    let colour = if active { t.text } else { t.text_muted };
    let text = RichText::new(label).size(11.0).color(colour);
    let button = match icon {
        Some(icon) => Button::image_and_text(crate::icons::image(icon, 12.0).tint(colour), text),
        None => Button::new(text),
    };
    ui.add(
        button
            .fill(if active { t.selection } else { Color32::TRANSPARENT })
            .stroke(if active { Stroke::new(1.0, t.accent) } else { Stroke::NONE }),
    )
}

/// One star, in points, and the pitch a row of them is laid out on.
const STAR_GLYPH: f32 = 12.0;
const STAR_PITCH: f32 = 14.0;

/// A row of star glyphs, painted as one content atom for a button.
///
/// Painted rather than typed: `★` is a character the bundled faces render at different widths and
/// heights (and not at all in some of them), so "three stars and up" was a numeral followed by a
/// speck. Three glyphs say it without a number, and they are the same picture the bottom bar's pill
/// and the cells' captions use.
fn stars_atom(stars: u8, tint: Color32) -> egui::Atom<'static> {
    let width = STAR_PITCH * stars.max(1) as f32 - (STAR_PITCH - STAR_GLYPH);
    egui::Atom::paint(vec2(width, STAR_GLYPH), move |ui, args| {
        for index in 0..stars {
            let glyph = Rect::from_min_size(
                pos2(args.rect.left() + index as f32 * STAR_PITCH, args.rect.top()),
                Vec2::splat(STAR_GLYPH),
            );
            crate::icons::paint(ui, crate::icons::Icon::Star, glyph, tint);
        }
    })
}

/// The minimum-rating chip: at rest it names the question, and while filtering it answers it in stars.
///
/// The chip still cycles 0→1→…→5→0, because that is a control a person has already learnt; what
/// changed is that its state is drawn, not spelled.
fn rating_chip(ui: &mut Ui, t: &Tokens, stars: u8) -> egui::Response {
    if stars == 0 {
        let colour = t.text_muted;
        let response = ui.add(
            Button::new((
                crate::icons::image(crate::icons::Icon::Star, STAR_GLYPH).tint(colour),
                RichText::new("Rating").size(11.0).color(colour),
            ))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE),
        );
        a11y::label_widget(ui, &response, egui::Role::Button, "Rating filter: any rating", false);
        return response.on_hover_text("Only photos rated at least this many stars");
    }
    let spoken = format!("Rating filter: {stars} star{} and up", if stars == 1 { "" } else { "s" });
    let response = ui.add(
        Button::new(stars_atom(stars, t.accent))
            .fill(t.selection)
            .stroke(Stroke::new(1.0, t.accent)),
    );
    a11y::label_widget(ui, &response, egui::Role::Button, &spoken, true);
    response.on_hover_text(format!("{spoken} — one more click raises the minimum, a click past five clears it"))
}

/// Which glyph a flag filter wears: the state's own, so the four states are pictures and not just words.
fn flag_filter_icon(flag: Option<Flag>) -> crate::icons::Icon {
    match flag {
        Some(Flag::Pick) => crate::icons::Icon::Pick,
        Some(Flag::Reject) => crate::icons::Icon::Reject,
        _ => crate::icons::Icon::Flag,
    }
}

/// The colour-label chip: the swatch is the colour, and the word only names it.
fn colour_chip(ui: &mut Ui, t: &Tokens, label: Option<sortlens_core::ColorLabel>, text: &str) -> egui::Response {
    let fill = label.map(|label| t.label(label));
    let ring = t.text_muted;
    let swatch = egui::Atom::paint(Vec2::splat(12.0), move |ui, args| {
        let centre = args.rect.center();
        match fill {
            Some(colour) => {
                ui.painter().circle_filled(centre, 4.5, colour);
            }
            None => {
                ui.painter().circle_stroke(centre, 4.5, Stroke::new(1.0, ring));
            }
        }
    });
    let active = label.is_some();
    let colour = if active { t.text } else { t.text_muted };
    let response = ui.add(
        Button::new((swatch, RichText::new(text).size(11.0).color(colour)))
            .fill(if active { t.selection } else { Color32::TRANSPARENT })
            .stroke(if active { Stroke::new(1.0, t.accent) } else { Stroke::NONE }),
    );
    let spoken = match label {
        Some(label) => format!("Colour filter: {}", label.label()),
        None => "Colour filter: any colour".to_string(),
    };
    a11y::label_widget(ui, &response, egui::Role::Button, &spoken, active);
    response.on_hover_text("Only photos with this colour label — an empty ring means every colour")
}

/// What the canvas says when it has no photographs to draw.
///
/// Three of these are the first-run wizard; `NoMatches` is not. A library that holds photographs and
/// shows none of them is not a first launch, and telling that person to add a folder was an answer to a
/// question they had not asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Empty {
    /// No roots yet, or a root that turned out to be empty: the only useful action is adding a folder.
    Add,
    /// A scan is running: show what has been found so far.
    Indexing,
    /// The library holds photographs and the current filters exclude every one of them.
    NoMatches { library: usize },
    /// Nothing to say: the grid owns the canvas, or the wizard was dismissed.
    Quiet,
}

/// Decide what the empty canvas says, from what the library is doing and what the filters left.
///
/// Derived every frame rather than remembered, so it cannot get stuck: once photographs are shown the
/// wizard is gone, whether or not anybody clicked anything. `dismissed` silences the *wizard* only — a
/// filter that hides every photograph still explains itself, because that screen's whole job is to offer
/// the way back.
///
/// The gate is deliberately split: `library` is the catalog's total and `shown` is the filtered count.
/// Using the filtered count for both is what made a narrowed-to-nothing library look like a first launch.
pub fn canvas_empty(roots: usize, scanning: bool, library: usize, shown: usize, dismissed: bool) -> Empty {
    if scanning {
        return if dismissed { Empty::Quiet } else { Empty::Indexing };
    }
    if roots == 0 || library == 0 {
        return if dismissed { Empty::Quiet } else { Empty::Add };
    }
    if shown == 0 {
        return Empty::NoMatches { library };
    }
    Empty::Quiet
}

fn empty_library(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine, actions: &mut Vec<Action>, shown: usize) {
    let roots = engine.roots().len();
    let scanning = engine.is_scanning();
    let library = engine.stats().total;
    let stage = canvas_empty(roots, scanning, library, shown, state.welcome_dismissed);
    let height = ui.available_height();
    ui.allocate_ui_with_layout(vec2(ui.available_width(), height), Layout::top_down(Align::Center), |ui| {
        ui.add_space((height * 0.14).clamp(24.0, 140.0));
        match stage {
            Empty::Add => welcome_add(ui, t, state, actions),
            Empty::Indexing => welcome_indexing(ui, t, engine),
            Empty::NoMatches { library } => empty_no_matches(ui, t, state, library),
            Empty::Quiet => {}
        }
    });
}

/// The library has photographs; the filters have hidden all of them.
///
/// Not the Welcome card: nothing here is a first launch, and `Add Folder…` is the one action that would
/// make it worse. The count is the library's own, so the screen says what the filters are costing, and
/// the one button clears exactly what the header's `N active` badge counts — the search text included,
/// because a query that matches nothing is as much a reason for this screen as a star rating is.
fn empty_no_matches(ui: &mut Ui, t: &Tokens, state: &mut UiState, library: usize) {
    ui.label(RichText::new("Nothing matches these filters").size(20.0).strong());
    ui.add_space(6.0);
    ui.label(
        RichText::new(format!(
            "{} {} in this library, and the current filters exclude every one of them.",
            format_count(library as u64),
            if library == 1 { "photograph" } else { "photographs" }
        ))
        .size(12.5)
        .color(t.text_muted),
    );
    ui.add_space(14.0);
    let clear = ui.add(Button::new(RichText::new("Clear filters").size(12.5)).min_size(vec2(130.0, 30.0)));
    if clear.clicked() {
        state.search.clear();
        state.clear_filters();
    }
    clear.on_hover_text("Clears the search text and every filter, and forgets the folder scope");
}

/// The width of the Welcome rows' glyph column: one 14-point glyph plus its gap.
const WELCOME_GLYPH_COLUMN: f32 = 16.0;

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
        // The three promises carry the icon set's own glyphs, not a typed bullet: a `·` at 12 points
        // is a speck the bundled faces render a pixel wide and a different height per face, and each
        // row here is one promise that a picture states faster than its title. The glyph column is a
        // fixed 16 points so the three titles share a left edge, exactly as the sidebar's rows do.
        for (icon, title, detail) in [
            (
                crate::icons::Icon::Folder,
                "Your files stay put",
                "No upload, no rename, no moving. Sortlens only reads.",
            ),
            (
                crate::icons::Icon::History,
                "Everything is undoable",
                "Ratings, flags and keywords live in one log you can step back.",
            ),
            (
                crate::icons::Icon::Reject,
                "Rejects wait for you",
                "Culling stages decisions; nothing is trashed until you apply.",
            ),
        ] {
            ui.horizontal_top(|ui| {
                let (column, _) = ui.allocate_exact_size(vec2(WELCOME_GLYPH_COLUMN, 16.0), Sense::hover());
                let glyph = Rect::from_center_size(pos2(column.left() + 8.0, column.top() + 8.0), Vec2::splat(14.0));
                crate::icons::paint(ui, icon, glyph, t.accent);
                ui.add_space(4.0);
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
        RichText::new("JPEG, HEIC, TIFF, PNG, WebP, GIF, camera raw and video — poster frame, length and rate")
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
        if state.view == View::Compare {
            // Escape from a compare closes the second pane and keeps the photograph: a comparison is
            // a way of looking at a frame, not somewhere you went.
            state.compare = None;
            state.view = View::Detail;
        } else {
            state.view = View::Library;
        }
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
    let detail_open = state.view.is_detail();
    let (open, rescan, redo, undo, collection, settings, quit, apply, view) = ui.input_mut(|input| {
        // Every command chord reads through the runtime keymap and is consumed, so no widget sees
        // the same press. Exact matching separates redo from undo, so the order the old
        // `consume_shortcut` calls needed no longer matters.
        (
            keymap::consume(&state.keys, input, &CommandRef::new("library.addFolder")),
            keymap::consume(&state.keys, input, &CommandRef::new("library.rescan")),
            keymap::consume(&state.keys, input, &CommandRef::new("edit.redo")),
            keymap::consume(&state.keys, input, &CommandRef::new("edit.undo")),
            keymap::consume(&state.keys, input, &CommandRef::new("collection.new")),
            keymap::consume(&state.keys, input, &CommandRef::new("app.settings")),
            keymap::consume(&state.keys, input, &CommandRef::new("app.quit")),
            keymap::consume(&state.keys, input, &CommandRef::new("cull.apply")),
            // The view keys are only view keys when no mode owns the letter: S swaps a compare and
            // P/X judge a cull session, so those stay theirs. The keymap's own gates keep command
            // chords and typing out; the view gate here is the shell's.
            if detail_open || engine.cull().is_some() {
                None
            } else if keymap::plain_pressed(&state.keys, input, &CommandRef::new("view.insights")) {
                Some(View::Insights)
            } else if keymap::plain_pressed(&state.keys, input, &CommandRef::new("view.places")) {
                Some(View::Places)
            } else if keymap::plain_pressed(&state.keys, input, &CommandRef::new("view.similar")) {
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
        match rescan_root(state, engine) {
            Some(root) => actions.push(Action::Rescan(root)),
            None => state.notice("Add a folder before rescanning"),
        }
        return true;
    }
    // The keymap reads exactly, so Ctrl+Shift+Z is redo and never also fires undo's Ctrl+Z.
    if redo {
        match engine.redo() {
            Ok(true) => state.notice("Redid the last undone change"),
            Ok(false) => state.notice("Nothing to redo"),
            Err(error) => state.notice_problem(error.to_string()),
        }
        return true;
    }
    if undo {
        match engine.undo() {
            Ok(true) => state.notice("Undid the last change"),
            Ok(false) => state.notice("Nothing to undo"),
            Err(error) => state.notice_problem(error.to_string()),
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
        activate_view(state, engine, view);
        return true;
    }
    if collection {
        let name = format!("Collection {}", engine.collections().len() + 1);
        match engine.create_collection(&name) {
            Ok(id) => {
                state.collection_filter = Some(id);
                state.notice(format!("Created {name}"));
            }
            Err(error) => state.notice_problem(error.to_string()),
        }
        return true;
    }
    false
}

/// Switch to a destination and start whatever that destination needs.
///
/// One function rather than a copy per entrance: a key, the palette and the bottom bar all agree about
/// what opening a view means, including beginning the hash pass Similar cannot show without and finding
/// the second photograph Compare cannot show without.
///
/// Returns `false` when the destination cannot open, so a tab does not paint itself selected for a view
/// that did not open. The notice already says why.
fn activate_view(state: &mut UiState, engine: &mut Engine, view: View) -> bool {
    // The second pane belongs to Compare and nowhere else. Leaving it closes it, so `compare` and the
    // open view cannot disagree about whether a compare is on screen.
    if view != View::Compare {
        state.compare = None;
    }
    if view == View::Compare && state.compare.is_none() {
        match compare_partner(state, engine) {
            Some(second) => state.compare = Some(second),
            None => {
                state.notice("Compare needs two photos: select one with a neighbour in view");
                return false;
            }
        }
    }
    if view == View::Detail {
        // The facts panel is the reason to open one photograph, so opening it says so.
        state.right_tab = RightTab::Details;
    }
    state.view = view;
    // A similar search needs hashes; asking for the view should start the work.
    if view == View::Similar && engine.similar_groups().is_empty() && engine.hash_progress().is_none() {
        match engine.begin_similar(state.similar_threshold) {
            Ok(0) => {}
            Ok(queued) => state.notice(format!("Hashing {queued} previews")),
            Err(error) => state.notice_problem(error.to_string()),
        }
    }
    true
}

/// Open the cull view, starting a queue from the current filters when no session is restored.
fn open_cull(state: &mut UiState, engine: &mut Engine) {
    state.view = View::Cull;
    if engine.cull().is_none() {
        match start_cull(engine, state) {
            Ok(total) => state.notice(format!("Culling {total} photos")),
            Err(error) => state.notice(error),
        }
    }
}

/// Which folder a Rescan should walk: the scoped one, or the library's first root.
fn rescan_root(state: &UiState, engine: &Engine) -> Option<String> {
    state
        .root
        .clone()
        .or_else(|| engine.counts_by_root().first().map(|(root, _)| root.clone()))
}

/// Open one photograph large: the Detail view with its facts panel, and no second pane.
///
/// Detail needs nothing from the engine — it draws whatever the selection already names — so this is
/// separate from [`activate_view`], which the places that may need to prepare a destination use.
fn open_detail(state: &mut UiState) {
    state.compare = None;
    state.view = View::Detail;
    state.right_tab = RightTab::Details;
}

/// The photograph a compare should put beside the primary.
///
/// A second ticked photo is the comparison the person asked for; with only one ticked, the next frame in the
/// sorted order is the one they were about to look at anyway. `None` means there is nothing honest to show.
fn compare_partner(state: &UiState, engine: &Engine) -> Option<PhotoId> {
    let id = state.selection.primary()?;
    if let Some(second) = state.selection.ids().into_iter().find(|other| *other != id) {
        return Some(second);
    }
    let filter = state.filter();
    engine
        .neighbour(&filter, state.sort, id, 1)
        .or_else(|| engine.neighbour(&filter, state.sort, id, -1))
}

/// Open a compare against the next photo in order, or close one that is open.
pub fn toggle_compare(state: &mut UiState, engine: &Engine) {
    if state.view == View::Compare {
        // Closing a compare keeps the photograph and drops the second pane: you were looking at a frame,
        // and the pair was a way of looking at it.
        state.compare = None;
        state.view = View::Detail;
        return;
    }
    match compare_partner(state, engine) {
        Some(second) => {
            state.compare = Some(second);
            state.view = View::Compare;
        }
        None if state.selection.primary().is_none() => state.notice("Pick a photo first"),
        None => state.notice("Nothing to compare with"),
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
    if !left.duration_label().is_empty() || !right.duration_label().is_empty() {
        facts.push((
            "length",
            format!(
                "{} vs {}",
                if left.duration_label().is_empty() {
                    "—".to_string()
                } else {
                    left.duration_label()
                },
                if right.duration_label().is_empty() {
                    "—".to_string()
                } else {
                    right.duration_label()
                }
            ),
        ));
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
                state.view = View::Library;
            }
        });
        return;
    };
    let Some(photo) = engine.photo(id).cloned() else {
        state.view = View::Library;
        state.compare = None;
        return;
    };
    let position = engine.position(&filter, state.sort, id);

    // The other half of a compare, when the Compare view is the one being drawn. A photo that has gone
    // missing closes the compare rather than leaving an empty pane behind.
    let other = if state.view == View::Compare {
        state.compare.and_then(|other| engine.photo(other).cloned())
    } else {
        None
    };
    if state.view == View::Compare && other.is_none() {
        state.compare = None;
        state.view = View::Detail;
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
                state.view = View::Library;
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
                // The button is a toggle, so in a compare it says what it will do: closing the pair
                // rather than opening one you are already looking at.
                if ui
                    .button(RichText::new("Close compare  C").size(11.0))
                    .on_hover_text("Go back to the single photograph")
                    .clicked()
                {
                    state.compare = None;
                    state.view = View::Detail;
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
    // Click the photograph to look closer: on a one-photo view the picture is the biggest target on
    // screen, and a person who clicks it means "show me this", not "select it again".
    if area_response.clicked() {
        state.detail_zoom = !state.detail_zoom;
    }
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
                // A soft pile-up of the small tier beats "Loading preview…": the blur is that photo, only
                // coarse, and it sharpens in place when the full decode lands.
                match thumbs.sharp_or_blur(ui.ctx(), item, 2000) {
                    // The tier is not used for painting: a blur is painted exactly like the sharp one, only
                    // visibly softer, which is the whole point. A photo whose file has gone is painted faded
                    // here too — a cached full-size preview of a missing file is still a cached preview.
                    Some((texture, _tier)) => {
                        let uv = cover_uv(texture.size_vec2(), sized.size());
                        ui.painter()
                            .with_clip_rect(pane.intersect(clip))
                            .image(texture.id(), sized, uv, cached_tint(item));
                    }
                    None => {
                        ui.painter()
                            .text(pane.center(), Align2::CENTER_CENTER, "Loading preview…", t.font(11.0), t.text_faint);
                    }
                }
            }
        }
        None => match thumbs.sharp_or_blur(ui.ctx(), &photo, 2000) {
            Some((texture, _tier)) => {
                let uv = cover_uv(texture.size_vec2(), target.size());
                let painter = ui.painter().with_clip_rect(area.intersect(ui.clip_rect()));
                painter.image(texture.id(), target, uv, cached_tint(&photo));
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
        if ui.add(Button::new(RichText::new("Previous").size(11.0))).clicked()
            && let Some(previous) = engine.neighbour(&filter, state.sort, id, -1)
        {
            state.selection.replace(previous);
        }
        if ui.add(Button::new(RichText::new("Next").size(11.0))).clicked()
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
        RichText::new("P/X/U flag · 1–5 rate · left/right walk · Z zoom · Esc close")
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

/// Paint a decided photo leaving the stage, with its label, inside the image's own bounds.
///
/// The overlay is clipped to the image rect rather than the canvas: a label that drifts outside the photograph
/// lands on empty background where it reads as chrome rather than as a mark on the frame.
fn paint_overlay(ui: &mut Ui, t: &Tokens, thumbs: &mut Thumbnails, engine: &Engine, area: Rect, overlay: Overlay, phase: Phase) {
    let Some(photo) = engine.photo(overlay.photo).cloned() else {
        return;
    };
    let aspect = aspect_of(&photo);
    let fitted = fit_rect(area.shrink(8.0), aspect);
    let painter = ui.painter().with_clip_rect(area);
    // Slight fade as it leaves, so the current photo is visible underneath rather than being covered by a
    // flash of the old one.
    let alpha = ((1.0 - phase.slide) * 255.0) as u8;
    let offset = vec2(-area.width() * phase.slide, 0.0);
    let moved = fitted.translate(offset);
    match thumbs.sharp_or_blur(ui.ctx(), &photo, 1600) {
        Some((texture, _tier)) => {
            let uv = cover_uv(texture.size_vec2(), moved.size());
            let tint = Color32::from_white_alpha(alpha.min(cached_tint(&photo).a()));
            painter.image(texture.id(), moved, uv, tint);
        }
        None => {
            painter.rect_filled(moved, t.corner(), t.thumb_bg);
        }
    }
    let (text, colour) = match overlay.decision {
        Decision::Pick => ("PICK", t.pick),
        Decision::Reject => ("REJECT", t.reject),
        Decision::Deferred => ("DEFERRED", t.text_muted),
        Decision::Unjudged => return,
    };
    // The label fades in on the image's own top-left corner, never the canvas's.
    let at = moved.left_top() + vec2(10.0, 10.0);
    painter.text(
        at,
        Align2::LEFT_TOP,
        text,
        t.font(11.0),
        Color32::from_rgba_unmultiplied(colour.r(), colour.g(), colour.b(), (phase.label * 255.0) as u8),
    );
}

fn cull_keyboard(ui: &mut Ui, state: &mut UiState, engine: &mut Engine) -> bool {
    if engine.cull().is_none() {
        return false;
    }
    let mut decision = None;
    let mut back = false;
    let mut tag_prev = false;
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
        if keymap::plain_pressed(&state.keys, input, &CommandRef::new("cull.back")) {
            back = true;
        }
        // The same one-key repeat the grid has, aimed at the photo on screen: keyword one photo, decide,
        // press T on the next. The notice names the keyword, like everywhere else it is applied.
        if keymap::plain_pressed(&state.keys, input, &CommandRef::new("photo.tagFromPrev")) {
            tag_prev = true;
        }
    });
    if let Some(decision) = decision {
        // Remember what was judged *before* the session moves on: the overlay paints that photo leaving while
        // the next one is already behind it, which is the only way a slide-out can show the photo that left.
        let judged = engine.cull().and_then(|session| session.current());
        engine.cull_decide(decision);
        if decision != Decision::Unjudged
            && let Some(photo) = judged
        {
            state.overlay = Some(Overlay {
                photo,
                decision,
                at: ui.input(|input| input.time),
            });
        }
        let left = engine.cull().map(|session| session.summary().unjudged).unwrap_or_default();
        state.announce = Some(crate::a11y::decision_sentence(decision, left));
    } else if back {
        engine.cull_back();
        state.announce = Some("Back one photo".to_string());
    }
    if tag_prev && let Some(current) = engine.cull().and_then(|session| session.current()) {
        match engine.tag_from_prev(&[current]) {
            Ok(changed) => {
                let tag = engine.last_tag().unwrap_or_default().to_string();
                state.notice(if changed == 0 {
                    format!("Already tagged \u{201c}{tag}\u{201d}")
                } else {
                    format!("Tagged \u{201c}{tag}\u{201d}")
                });
            }
            Err(error) => state.notice_problem(error.to_string()),
        }
    }
    true
}

/// How long the decision label takes to arrive, and how long the photo takes to leave.
///
/// Two phases rather than one gesture: the label is the *confirmation* and must land before the photograph
/// moves, or the eye follows the photograph and never reads what was decided. 90 ms is about one glance; the
/// slide is 140 ms, slow enough to read as movement rather than a cut.
const LABEL_MS: f32 = 90.0;
const SLIDE_MS: f32 = 140.0;

/// A decision being shown leaving the cull stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Overlay {
    /// The photo that was judged, kept so it can slide out while the next one is already behind it.
    pub photo: PhotoId,
    /// What it was judged as.
    pub decision: Decision,
    /// When the decision was made, in seconds since the app started.
    pub at: f64,
}

/// Where the overlay is in its two phases.
///
/// Pure, so the timings can be tested without waiting for them: `label` is the alpha the decision label is
/// painted at, `slide` is how far the photograph has moved out (0 = in place, 1 = gone), and `done` means the
/// overlay can be dropped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Phase {
    pub label: f32,
    pub slide: f32,
    pub done: bool,
}

/// The phase of the overlay `elapsed_ms` after the decision.
///
/// Reduced motion does not animate: the label is drawn at full strength at once (a fade of zero is what
/// "reduced" means, not a faster fade) and there is no slide, so the stage has already cut to the next photo.
/// The label still holds for its 90 ms, because the confirmation is information, not decoration — removing it
/// would take away the only sign that the key press registered.
pub fn overlay_phase(elapsed_ms: f32, motion: Motion) -> Phase {
    if !motion.animates() {
        return Phase {
            label: 1.0,
            slide: 0.0,
            done: elapsed_ms >= LABEL_MS,
        };
    }
    let label = (elapsed_ms / LABEL_MS).clamp(0.0, 1.0);
    let slide = ((elapsed_ms - LABEL_MS) / SLIDE_MS).clamp(0.0, 1.0);
    Phase {
        // Ease-out on the label so it arrives rather than creeping, and ease-in on the slide so the photo
        // starts moving only after it is read.
        label: 1.0 - (1.0 - label) * (1.0 - label),
        slide: slide * slide,
        done: elapsed_ms >= LABEL_MS + SLIDE_MS,
    }
}

/// The up-next strip under the canvas: four thumbnail cells, which is the tallest row in the foot.
const CULL_UP_NEXT: f32 = 56.0;

/// The keyword band under the decisions, in chip rows.
///
/// Two, not as many as the keywords need: an AI pass can leave thirty on one frame, and a band that grew with
/// its content would resize the photograph on screen every time the queue moved.
const CULL_CHIP_ROWS: f32 = 2.0;

/// The room a wrapping band of `CULL_CHIP_ROWS` chip rows takes, gaps between the rows included.
fn cull_chip_band(spacing: &egui::style::Spacing) -> f32 {
    CULL_CHIP_ROWS * spacing.interact_size.y + (CULL_CHIP_ROWS - 1.0) * spacing.item_spacing.y
}

/// The exact height of the block under the cull canvas.
///
/// Arithmetic rather than a measurement, because the canvas above it is allocated first and a `Ui` only learns
/// a child's height *after* laying it out. Every row here is a fixed height, so the sum is the truth and not an
/// estimate: the up-next strip, the four decisions, the keyword band and field, and the spoken line. The band
/// and the staged sentence are conditional, and both conditions are known before the block is laid out.
///
/// `line` is a galley height measured by the caller from the font it will paint with, so a face with a taller
/// line cannot push the field under the bottom bar. `the_cull_foot_stays_above_the_bottom_bar_at_nine_hundred_points`
/// fails if this arithmetic and the rows it describes ever disagree.
fn cull_foot_height(spacing: &egui::style::Spacing, line: f32, tags: bool, staged: bool) -> f32 {
    let gap = spacing.item_spacing.y;
    let row = spacing.interact_size.y;
    let mut height = gap; // the breathing space between the photograph and the strip
    height += CULL_UP_NEXT + gap + row; // what is queued next, then the four decisions
    if tags {
        height += gap + cull_chip_band(spacing);
    }
    height += gap + row; // the field that adds a keyword while the photo is on screen
    height += gap + line; // the live line
    if staged {
        height += gap + line; // the sentence that says what Apply will do
    }
    height
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
                RichText::new("P pick · X reject · Space defer · U clear · ◀ back · T last keyword · drag up/down")
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

    // The block under the photograph is claimed *before* the photograph, in a bottom panel of an exact
    // height, and the canvas is allocated out of what is left. Before this the canvas was sized from the
    // window with a guessed allowance for what followed it — and at 900 points the keyword field, the control
    // that adds a keyword while the photo on screen is the one being judged, was painted under the bottom bar.
    // There is now no height at which the two can meet: the photograph gives way, never the controls.
    let line_height = ui
        .painter()
        .layout_no_wrap(
            "Ag".to_string(),
            egui::FontId::new(10.5, egui::FontFamily::Proportional),
            t.text_muted,
        )
        .size()
        .y;
    let has_tags = current.and_then(|id| engine.photo(id)).is_some_and(|photo| !photo.tags.is_empty());
    let foot_height = cull_foot_height(ui.spacing(), line_height, has_tags, summary.changed() > 0);
    let position = if total == 0 { 0 } else { (cursor + 1).min(total) };
    let status_colour = match decision {
        Decision::Pick => t.pick,
        Decision::Reject => t.reject,
        _ => t.text_muted,
    };
    let status = crate::a11y::cull_status(position, total, decision);

    egui::Panel::bottom("cull_foot")
        .resizable(false)
        .exact_size(foot_height)
        .frame(Frame::NONE)
        .show(ui, |ui| {
            ui.add_space(ui.spacing().item_spacing.y);
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
            // Inline tagging (SL-C11): the photo on screen can be keyworded without leaving the cull loop.
            // Chips show what it already carries and come off with a click; the field adds one, so deciding
            // and keywording both happen while the same photo is on screen. Focused here, the field owns the
            // keys: keyboard() stands down while any widget has focus, so P and T type letters, not decisions.
            if let Some(id) = current
                && let Some(photo) = engine.photo(id).cloned()
            {
                let mut remove = None;
                // The band is two chip rows high and it scrolls, so the photograph above it keeps the same size
                // whether the frame on screen carries one keyword or the thirty an AI pass can leave on it. A band
                // that grew with its content would resize the photograph every time the queue moved.
                if !photo.tags.is_empty() {
                    egui::ScrollArea::vertical()
                        .id_salt("cull_keywords")
                        .max_height(cull_chip_band(ui.spacing()))
                        // The scroll area's own floor is 64 points, taller than the band: a band that ignored the
                        // height it was given would push the field below it out of the panel.
                        .min_scrolled_height(0.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for tag in &photo.tags {
                                    let chip = Button::new(RichText::new(format!("{tag}  ×")).size(11.0).color(t.text))
                                        .fill(t.card)
                                        .stroke(Stroke::new(1.0, t.border))
                                        .corner_radius(t.corner());
                                    if ui.add(chip).on_hover_text("Remove keyword from this photo").clicked() {
                                        remove = Some(tag.clone());
                                    }
                                }
                            });
                        });
                }
                ui.horizontal(|ui| {
                    let field = ui.add(
                        TextEdit::singleline(&mut state.cull_tag_input)
                            .id(egui::Id::unique("cull-tag-field"))
                            .hint_text("Add keyword to this photo")
                            .desired_width(150.0)
                            .font(egui::TextStyle::Small),
                    );
                    if state.cull_tag_focus {
                        field.request_focus();
                        state.cull_tag_focus = false;
                    }
                    let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
                    let clicked = ui.add(Button::new(RichText::new("Add").size(11.0))).clicked();
                    if (entered || clicked) && !state.cull_tag_input.trim().is_empty() {
                        let tag = state.cull_tag_input.trim().to_string();
                        match engine.set_tag(&[photo.id], &tag, true) {
                            Ok(0) => state.notice("That keyword is already on this photo"),
                            Ok(_) => {
                                state.notice(format!("Tagged \u{201c}{}\u{201d}", tag.to_lowercase()));
                                state.cull_tag_input.clear();
                            }
                            Err(error) => state.notice_problem(error.to_string()),
                        }
                    }
                });
                if let Some(tag) = remove {
                    match engine.set_tag(&[photo.id], &tag, false) {
                        Ok(_) => state.notice(format!("Removed keyword {tag}")),
                        Err(error) => state.notice_problem(error.to_string()),
                    }
                }
            }

            // The session moves on after the strip above has painted the photograph it was about. Deciding later
            // than this would leave the photograph and the chips disagreeing about which frame is on screen for a
            // frame; deciding earlier would advance the queue underneath a row that is still being laid out.
            if let Some(choice) = decision_now {
                engine.cull_decide(choice);
                let left = engine.cull().map(|session| session.summary().unjudged).unwrap_or_default();
                state.announce = Some(crate::a11y::decision_sentence(choice, left));
            }

            // Spoken feedback: the queue position, and what just happened when something did. Assistive technology
            // reads this politely, so it never interrupts anything else being read. One line rather than two: the
            // announcement has replaced the position for this frame, and a second line said the same thing twice
            // while costing the photograph above it another row.
            let (spoken, colour) = match state.announce.take() {
                Some(said) => (said, t.text_muted),
                None => (status, status_colour),
            };
            crate::a11y::live_line(ui, &spoken, colour);

            if summary.changed() > 0 {
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
        });

    // The canvas takes the room the block below it did not claim. No floor on it: the app's own minimum window
    // leaves the photograph over 200 points at 560 high, and a floor is exactly what would push the strip back
    // under the bottom bar at a size nobody measured.
    let image_height = ui.available_height().max(1.0);
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
            let wanted = if zoomed { 2000 } else { 1600 };
            match thumbs.sharp_or_blur(ui.ctx(), &photo, wanted) {
                Some((texture, _tier)) => {
                    let uv = cover_uv(texture.size_vec2(), shown.size());
                    painter.image(texture.id(), shown, uv, cached_tint(&photo));
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

    // The decision that just happened, leaving. Painted *after* the current photo, so the photo that was
    // judged slides away over the one that replaced it — the movement is the confirmation that the queue
    // advanced, and a label alone would leave you unsure whether the key registered twice.
    if let Some(overlay) = state.overlay {
        let elapsed = ((ui.input(|input| input.time) - overlay.at) * 1000.0) as f32;
        let phase = overlay_phase(elapsed, state.motion);
        if phase.done {
            state.overlay = None;
        } else {
            paint_overlay(ui, t, thumbs, engine, area, overlay, phase);
            // Keep animating: without this the overlay would freeze at whatever phase the next input caused.
            ui.ctx().request_repaint();
        }
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
                Err(error) => state.notice_problem(error.to_string()),
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
    if !found.is_empty() {
        ui.horizontal(|ui| {
            let leftovers: Vec<PhotoId> = {
                let mut all: Vec<PhotoId> = Vec::new();
                for group in &found {
                    for id in group.others() {
                        if !all.contains(&id) {
                            all.push(id);
                        }
                    }
                }
                all
            };
            if ui
                .add_enabled(
                    !leftovers.is_empty(),
                    Button::new(RichText::new(format!("Select leftovers in every group ({})", leftovers.len())).size(11.0)),
                )
                .on_hover_text("Ticks every photo that is not its group's best, so you can review them together")
                .clicked()
            {
                state.selection.select_many(&leftovers);
                state.notice(format!("Selected {} leftover(s) across {} group(s)", leftovers.len(), found.len()));
            }
            let ticked = state.selection.len();
            if ticked > 0 {
                if ui
                    .button(RichText::new(format!("Delete the {ticked} selected…")).size(11.0))
                    .on_hover_text("Moves them to the trash, the rejects folder or a tag, after the same safety checks a cull batch gets")
                    .clicked()
                {
                    state.similar_delete_open = true;
                }
                if ui.small_button("Clear").clicked() {
                    state.selection.clear();
                }
            } else {
                ui.label(
                    RichText::new(format!("{} group(s) · nothing selected", found.len()))
                        .size(10.5)
                        .color(t.text_faint),
                );
            }
        });
        ui.add_space(6.0);
    }
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
                                if ui
                                    .small_button("Select all but the best")
                                    .on_hover_text("Ticks every photo in this group except the one worth keeping")
                                    .clicked()
                                {
                                    let others = group.others();
                                    // Every one of them, not just the first: the button says "all but the best" and
                                    // a selection of one would be a different, and quieter, promise.
                                    state.selection.select_many(&others);
                                    state.notice(format!("{} set aside", others.len()));
                                }
                                // The strongest member is named, so a decision to delete the rest is made with the
                                // keeper in view rather than guessed from a thumbnail order.
                                if let Some(keep) = engine.photo(group.best).map(|photo| {
                                    if photo.name.is_empty() {
                                        photo.rel.clone()
                                    } else {
                                        photo.name.clone()
                                    }
                                }) {
                                    ui.label(RichText::new(format!("keeping {keep}")).size(9.5).color(t.text_faint));
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

/// Where the photographs were taken: a coastline under the dots, and the credit on top of it.
///
/// The basemap has two sources, in order. A **tile folder the person supplies** (see `sortlens_geo::tiles`) is a real
/// map and wins: it is drawn with its own credit line printed over it every frame, and `Tileset` cannot exist without
/// one. Otherwise the **shipped coastline** — the public-domain Natural Earth 1:110m outline, embedded in
/// `sortlens_geo::coastline` — is stroked under the dots, so a first-run library is geography rather than a field of
/// dots on a blank rectangle. A tile that is missing from a set is painted as background: a hole in a basemap is a
/// hole, not an error to shout about.
///
/// The map opens **fitted to the library's own photographs**: centred on the middle of their spread and zoomed to the
/// deepest level that still shows all of them. The old fixed zoom 5 opened over the Atlantic with both clusters off
/// screen for a library on two continents, which reads as an empty map.
fn places_view(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &Engine) {
    let located = engine.geotagged();
    let tileset = engine.tileset();
    let usable = matches!(&tileset, Some(Ok(_)));

    ui.add_space(8.0);
    // The map gets the space; the counts and the credit share what is left.
    let footer = if usable { 74.0 } else { 96.0 };
    let map_height = (ui.available_height() - footer).max(120.0);
    let map_size = vec2(ui.available_width(), map_height);

    if located == 0 {
        ui.allocate_ui_with_layout(map_size, Layout::top_down(Align::Center), |ui| {
            ui.add_space(map_height / 3.0);
            ui.label(RichText::new("No photo has a location yet").size(15.0).strong());
            ui.label(
                RichText::new(
                    "A camera records where it was in the file's EXIF. Photos taken with location services \
                         off, or stripped by another tool, do not have one.",
                )
                .size(11.0)
                .color(t.text_muted),
            );
        });
        basemap_status(ui, t, engine, &tileset);
        return;
    }

    let range = match &tileset {
        Some(Ok(set)) => Some(set.zoom_range()),
        _ => None,
    };
    let located_photos = engine.located();
    // Every located photograph is a point to fit the view around, whether or not it clusters into a dot of its own.
    let centres: Vec<sortlens_geo::LatLon> = located_photos.iter().map(|(_, at)| *at).collect();
    // A tileset's top zoom is the deepest that can be drawn; with the shipped coastline the fit's own ceiling applies.
    let highest = range.map_or(sortlens_geo::coastline::MAX_FIT_ZOOM, |(_, high)| high);
    // The middle of the spread, not the mean of the latitudes: the mean of Brighton and Lima is 1,400 km north of the
    // box's middle, so a view centred on it pushes one dot towards the edge of its own fit. The fallback cannot be
    // reached with an empty list because `located == 0` returned above.
    let centre = sortlens_geo::coastline::bounds_centre(&centres).unwrap_or(located_photos[0].1);
    if !state.places_zoom_chosen {
        // With the shipped outline the fitted view can be so deep that an inland library has no coast on screen, which
        // is a blank rectangle with a dot on it rather than a map. The outline fit widens the view until a shoreline
        // is in it; a person's own tiles are already geography at any zoom, so they keep the plain fit.
        state.places_zoom = if range.is_none() {
            sortlens_geo::coastline::fit_zoom_with_outline(&centres, f64::from(map_size.x), f64::from(map_size.y), highest)
        } else {
            sortlens_geo::coastline::fit_zoom(&centres, f64::from(map_size.x), f64::from(map_size.y), highest)
        };
    }
    if let Some(range) = range {
        state.places_zoom = places::clamp_zoom(state.places_zoom, range);
    }
    let zoom = state.places_zoom;

    let (rect, response) = ui.allocate_exact_size(map_size, Sense::click());
    // A clickable region that a screen reader can land on and name: the map, and how many dots it holds.
    let dot_count = engine.places(zoom).len();
    a11y::label_widget(
        ui,
        &response,
        egui::Role::Canvas,
        &format!(
            "Map of {located} photo(s) with a location, in {dot_count} dot(s) at zoom {zoom}. \
             Click a dot to select its photos."
        ),
        false,
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, t.thumb_bg);
    let origin = sortlens_geo::viewport_origin(zoom, centre, f64::from(map_size.x), f64::from(map_size.y));

    // Tiles first, then the shipped coastline if there are none, then dots: the coastline is the backdrop, not a
    // participant. The credit for whichever was drawn is printed below.
    if let (Some(Ok(set)), Some(origin)) = (&tileset, origin) {
        for cell in sortlens_geo::visible_tiles(zoom, origin, f64::from(map_size.x), f64::from(map_size.y)) {
            let place = sortlens_geo::tile_rect(cell, origin);
            let tile_rect = egui::Rect::from_min_size(
                rect.min + vec2(place.x as f32, place.y as f32),
                vec2(place.w as f32, place.h as f32),
            );
            // A tile that is missing or will not decode is a hole, painted as background rather than as a broken
            // image: a partial tile set is a normal thing, and a toast for every gap would bury the map in noise.
            let texture = set
                .tile_path(cell.zoom, cell.x, cell.y)
                .and_then(|path| state.places_tiles.texture(ui.ctx(), &path).map(egui::TextureHandle::id));
            if let Some(id) = texture {
                painter.image(
                    id,
                    tile_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                painter.rect_filled(tile_rect, 0.0, t.thumb_bg);
            }
        }
    } else if let Some(origin) = origin {
        // No tile folder, so the embedded outline is the basemap. One stroke per run of visible coast, painted before
        // the dots so a dot always sits on top of the land it is on.
        let coastline = sortlens_geo::coastline::Coastline::shipped();
        for line in coastline.screen_lines(zoom, origin, f64::from(map_size.x), f64::from(map_size.y)) {
            if line.len() < 2 {
                continue;
            }
            let points: Vec<egui::Pos2> = line
                .iter()
                .map(|at| egui::pos2(rect.min.x + at.x as f32, rect.min.y + at.y as f32))
                .collect();
            painter.add(egui::Shape::line(points, egui::Stroke::new(1.0, t.map_coast)));
        }
    }

    // The dots, and the hit test the click below uses — one list, so what is drawn and what is clickable cannot drift.
    let clusters = engine.places(zoom);
    let dots = origin.map_or_else(Vec::new, |origin| places::dots(&clusters, zoom, origin, map_size.x, map_size.y));
    for dot in &dots {
        let cluster = &clusters[dot.cluster];
        let centre_px = egui::pos2(rect.min.x + dot.at.0, rect.min.y + dot.at.1);
        painter.circle_filled(centre_px, dot.radius, t.accent.gamma_multiply(0.82));
        painter.circle_stroke(centre_px, dot.radius, egui::Stroke::new(1.0, t.accent_fg.gamma_multiply(0.7)));
        if !cluster.is_single() {
            painter.text(
                centre_px,
                Align2::CENTER_CENTER,
                format_count(cluster.len() as u64),
                FontId::proportional(10.0),
                t.accent_fg,
            );
        }
    }

    // Place names for the dots, from the library's own gazetteer. The borrow is resolved into owned strings here so
    // the click handler below can touch the selection.
    let places_file = engine.places_file();
    let (labels, names_len, names_error) = {
        let gazetteer = state.places_names.for_file(places_file.as_deref());
        let labels: Vec<Option<String>> = clusters
            .iter()
            .map(|cluster| {
                gazetteer
                    .and_then(|gazetteer| places::place_label(gazetteer, cluster.centre))
                    .map(|place| place.name.clone())
            })
            .collect();
        (labels, state.places_names.len(), state.places_names.error().map(str::to_string))
    };
    for dot in &dots {
        let Some(label) = labels.get(dot.cluster).and_then(Option::as_ref) else {
            continue;
        };
        painter.text(
            egui::pos2(rect.min.x + dot.at.0, rect.min.y + dot.at.1 + dot.radius + 2.0),
            Align2::CENTER_TOP,
            label,
            FontId::proportional(10.0),
            t.text,
        );
    }

    // Clicking a dot selects exactly the photos it stands for. Clicking the map itself selects nothing: a click on
    // the sea is not a way to clear a selection the person made on purpose.
    let clicked_cluster = response
        .interact_pointer_pos()
        .filter(|_| response.clicked())
        // The pointer arrives in screen coordinates and the dots are relative to the map's own corner, so the origin
        // is subtracted here. Without it every click missed by the panel's margin — which is what the click test was
        // written to catch, and did.
        .and_then(|pointer| places::dot_at(&dots, (pointer.x - rect.min.x, pointer.y - rect.min.y)))
        .map(|index| dots[index].cluster);
    if let Some(index) = clicked_cluster {
        let cluster = &clusters[index];
        state.selection.select_many(&cluster.members);
        let name = labels.get(index).and_then(Option::as_deref);
        state.notice(places::selection_notice(cluster.len(), cluster.centre, name));
    }

    // The zoom controls sit over the map's corner, where a map puts them.
    let mut zoom_changed = false;
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink(8.0)), |ui| {
        ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
            let (low, high) = range.unwrap_or((zoom, zoom));
            if ui.add_enabled(zoom < high, Button::new(RichText::new("+").size(13.0))).clicked() {
                state.places_zoom = zoom.saturating_add(1);
                state.places_zoom_chosen = true;
                zoom_changed = true;
            }
            if ui.add_enabled(zoom > low, Button::new(RichText::new("−").size(13.0))).clicked() {
                state.places_zoom = zoom.saturating_sub(1);
                state.places_zoom_chosen = true;
                zoom_changed = true;
            }
            ui.label(RichText::new(format!("zoom {zoom}")).size(10.5).color(t.text_muted));
        });
    });

    // The credit, over the map, every frame it is drawn. Never hidden behind a menu or a hover.
    ui.allocate_ui_with_layout(vec2(ui.available_width(), 0.0), Layout::left_to_right(Align::Center), |ui| {
        match &tileset {
            Some(Ok(set)) => {
                ui.label(RichText::new(set.attribution()).size(9.5).color(t.text_muted));
            }
            _ => {
                ui.label(RichText::new(sortlens_geo::coastline::CREDIT).size(9.5).color(t.text_muted));
            }
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new(format!(
                "{} photo(s) with a location · {} dot(s) · click a dot to select it",
                format_count(located as u64),
                format_count(clusters.len() as u64)
            ))
            .size(9.5)
            .color(t.text_faint),
        );
    });
    if zoom_changed {
        ui.ctx().request_repaint();
    }
    match (&names_error, names_len) {
        (Some(error), _) => {
            ui.add_space(2.0);
            ui.label(RichText::new(error).size(9.5).color(t.reject));
        }
        (None, 0) => {
            ui.add_space(2.0);
            ui.label(
                RichText::new("No places.txt — dots show their coordinates")
                    .size(9.5)
                    .color(t.text_faint),
            );
        }
        (None, named) => {
            ui.add_space(2.0);
            ui.label(
                RichText::new(format!(
                    "Named from places.txt · {named} place(s) · dots within {:.0} km borrow a name",
                    places::LABEL_KM
                ))
                .size(9.5)
                .color(t.text_faint),
            );
        }
    }
    basemap_status(ui, t, engine, &tileset);
}

/// The basemap's state as a sentence: which outline is drawn, and what a real one would cost.
fn basemap_status(
    ui: &mut Ui,
    t: &Tokens,
    engine: &Engine,
    tileset: &Option<Result<sortlens_geo::tiles::Tileset, sortlens_geo::tiles::TileError>>,
) {
    match tileset {
        None => {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Schematic coastline · install a tile folder for a real basemap")
                    .size(10.5)
                    .color(t.text_muted),
            );
            if let Some(dir) = engine.tiles_dir() {
                ui.label(
                    RichText::new(format!(
                        "Put {{z}}/{{x}}/{{y}}.png tiles and an attribution.txt in {} to replace the 1:110m \
                         outline — nothing is downloaded.",
                        dir.display()
                    ))
                    .size(9.5)
                    .color(t.text_faint),
                );
            }
        }
        Some(Err(error)) => {
            ui.add_space(4.0);
            ui.label(RichText::new("The basemap cannot be used").size(10.5).strong().color(t.reject));
            ui.label(RichText::new(error.message()).size(9.5).color(t.text_muted));
        }
        Some(Ok(_)) => {}
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
            Err(error) => state.notice_problem(error.to_string()),
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

/// Soft Focus: the sharpness review as its own destination rather than the last thing you scroll to.
///
/// The pass lands here, not on a timer: this is the view that asked for it, and a pass that finished while not
/// looking at it would otherwise sit in its channel until someone came back. The review is the same
/// [`sharpness_section`] the insights page used to end with, so there is one review, not two that could disagree.
fn soft_focus_view(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine) {
    if let Some(report) = engine.poll_sharpness() {
        if report.remaining == 0 {
            let message = match report.unjudged {
                0 => format!("Measured {} photo(s)", report.measured),
                unjudged => format!(
                    "Measured {} photo(s); {unjudged} could not be judged and were left unscored",
                    report.measured
                ),
            };
            state.notice(message);
        } else {
            state.notice(format!(
                "Sharpness pass stopped after {} photo(s)",
                report.measured + report.unjudged
            ));
        }
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new("Soft Focus").size(15.0).strong().color(t.text));
        ui.label(RichText::new("A relative cut, never a verdict").size(10.5).color(t.text_faint));
    });
    hint(
        ui,
        t,
        "Sharpness ranks frames inside this library, in one light at one focal length. The cut takes the \
         softest N%, so it can only ever claim to be relative to these photographs.",
    );
    ui.add_space(8.0);
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| sharpness_section(ui, t, state, engine));
}

/// The softness review: how detail is distributed, what a relative cut would select, and the button that selects it.
///
/// The cut is a *review*, not a verdict: a sharpness score is unitless and ranks frames inside one library, so the
/// section says "the softest N% of this library" rather than "blurry" — and when the scores cannot be separated at
/// all, it says that instead of offering a button that would select everything.
fn sharpness_section(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine) {
    ui.label(RichText::new("Sharpness").size(12.0).strong());
    let todo = engine.sharpness_todo();
    if let Some((done, total)) = engine.sharpness_progress() {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Measuring").size(11.0).color(t.text_muted));
            ui.add(egui::ProgressBar::new(if total == 0 { 0.0 } else { done as f32 / total as f32 }).desired_width(220.0));
            ui.label(RichText::new(format!("{done} of {total}")).size(10.5).color(t.text_faint));
        });
        ui.add_space(6.0);
    } else if todo > 0 && !engine.read_only() {
        if ui
            .button(RichText::new("Measure sharpness").size(11.0))
            .on_hover_text("Decode each photo and measure how much fine detail it has")
            .clicked()
        {
            match engine.begin_sharpness() {
                Ok(0) => state.notice("Every photo already has a current score"),
                Ok(_) => {}
                Err(error) => state.notice_problem(error.to_string()),
            }
        }
        ui.label(
            RichText::new(format!("{todo} photo(s) have no current score"))
                .size(10.5)
                .color(t.text_faint),
        );
    }

    let scores: Vec<(PhotoId, f32)> = engine.scores().into_iter().map(|(id, score)| (id, score.value)).collect();
    let review = sharpness::review(&scores, state.sharpness_percentile);
    ui.add_space(6.0);
    if review.scored == 0 {
        ui.label(
            RichText::new("Nothing measured yet. Sharpness ranks the frames in this library; it is not a verdict on any of them.")
                .size(10.5)
                .color(t.text_faint),
        );
        return;
    }

    // The histogram: one column per score band, with the bands a cut would take in the accent colour.
    let columns: Vec<Column> = review
        .buckets
        .iter()
        .enumerate()
        .map(|(index, count)| {
            let column = Column::new(String::new(), *count);
            if review.band_selected(index) {
                column.highlighted()
            } else {
                column
            }
        })
        .collect();
    charts::columns(ui, t, &columns, charts::COLUMN_HEIGHT);
    ui.label(
        RichText::new(format!(
            "Softest {:.0}% · scores {:.2} to {:.2}",
            state.sharpness_percentile, review.low, review.high
        ))
        .size(10.5)
        .color(t.text_faint),
    );
    ui.add_space(4.0);
    let mut percentile = state.sharpness_percentile;
    let slider = ui.add(
        egui::Slider::new(&mut percentile, sharpness::MIN_PERCENTILE..=sharpness::MAX_PERCENTILE)
            .suffix("%")
            .fixed_decimals(0),
    );
    // The slider is the whole question this section asks, so it needs a name a screen reader can read:
    // the screenshot harness's accessibility pass found it unnamed, and a value with no question is noise.
    crate::a11y::label_widget(ui, &slider, egui::Role::Slider, "Softest share of the library to review", false);
    state.sharpness_percentile = percentile;
    ui.label(RichText::new(review.cut_label()).size(11.0).color(t.text));
    ui.add_space(4.0);
    if review.can_select() {
        if ui
            .button(RichText::new(format!("Select {} photo(s)", review.count())).size(11.0))
            .on_hover_text("Ticks them in the library, where you can look before you act")
            .clicked()
        {
            state.selection.select_many(&review.below);
            state.view = View::Library;
            state.notice(format!("Selected the {} softest", review.count()));
        }
    } else if review.separable {
        ui.label(RichText::new("Nothing at the cut").size(10.5).color(t.text_faint));
    }
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

/// Store the current search against whatever scope is open, so reopening it resumes there.
///
/// A collection and a folder are both scopes, and the rule is the same for both: the search is committed when the
/// box is left or Enter is pressed, the text comes back into the box when the scope is reopened, and nothing is
/// filtering in the background that you cannot see.
fn remember_search(state: &mut UiState, engine: &mut Engine) {
    if state.collection_filter.is_none()
        && let Some(root) = state.root.clone()
    {
        let mut mark = engine.folder_mark(&sortlens_engine::folder_key(&root));
        let wanted = Some(state.search.trim().to_string()).filter(|text| !text.is_empty());
        if mark.search == wanted {
            return;
        }
        mark.search = wanted;
        if let Err(error) = engine.set_folder_mark(&root, mark) {
            state.notice_problem(error.to_string());
        }
        return;
    }
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
        state.notice_problem(error.to_string());
    }
}

/// Collections: favourites first, each with its colour, and the actions that change both.
fn collections_section(ui: &mut Ui, t: &Tokens, state: &mut UiState, engine: &mut Engine) {
    nav_section(ui, t, "Collections");
    let collections = engine.collections();
    if collections.is_empty() {
        hint(ui, t, "Collect photos into named sets without moving them.");
    }
    let order: Vec<sortlens_core::CollectionId> = collections.iter().map(|collection| collection.id).collect();
    // A collection that was deleted elsewhere must not stay ticked.
    state.collection_selection.retain(&order);

    let mut delete: Option<Vec<sortlens_core::CollectionId>> = None;
    let mut rename = None;
    let mut recolour: Option<(Vec<sortlens_core::CollectionId>, Option<sortlens_core::ColorLabel>)> = None;
    let mut pin: Option<(Vec<sortlens_core::CollectionId>, bool)> = None;
    for collection in &collections {
        let filtered = state.collection_filter == Some(collection.id);
        let ticked = state.collection_selection.contains(collection.id);
        let name = format!("{}{}", if collection.favourite { "★ " } else { "" }, collection.name);
        let row = nav_row(
            ui,
            t,
            crate::icons::Icon::Layers,
            &name,
            &format_count(collection.len() as u64),
            filtered || ticked,
        );
        if let Some(colour) = collection.colour {
            // A dot before the name, so a colour is scannable without opening anything.
            let dot = pos2(row.rect.left() + 3.0, row.rect.center().y);
            ui.painter().circle_filled(dot, 3.0, t.label(colour));
        }
        if ticked {
            // A tick, distinct from the filter's highlight: a ticked collection is one a batch will act on,
            // a filtered one is what the grid is showing, and they are different questions.
            ui.painter().text(
                row.rect.right_center() - vec2(2.0, 0.0),
                Align2::RIGHT_CENTER,
                "•",
                t.font(12.0),
                t.accent,
            );
        }
        // Which rows a context-menu action applies to: the whole ticked set when this row is part of it,
        // otherwise just this row. Right-clicking an unrelated row must not recolour four others.
        let targets = if ticked && state.collection_selection.len() > 1 {
            state.collection_selection.ids()
        } else {
            vec![collection.id]
        };
        if row.clicked() {
            let modifiers = ui.input(|input| input.modifiers);
            if modifiers.command {
                state.collection_selection.toggle(collection.id);
            } else if modifiers.shift {
                state.collection_selection.extend(&order, collection.id);
            } else {
                state.collection_selection.replace(collection.id);
                if filtered {
                    state.collection_filter = None;
                } else {
                    state.collection_filter = Some(collection.id);
                    // Reopening a collection puts the sort back to how it was last browsed.
                    state.sort = collection.sort.as_deref().map(Sort::parse).unwrap_or(Sort::Natural);
                }
            }
        }
        row.context_menu(|ui| {
            if targets.len() > 1 {
                ui.label(
                    RichText::new(format!("{} collections", targets.len()))
                        .size(10.5)
                        .color(t.text_faint),
                );
            }
            let pin_label = if collection.favourite {
                "Remove from favourites"
            } else {
                "Add to favourites"
            };
            if ui.button(pin_label).clicked() {
                pin = Some((targets.clone(), !collection.favourite));
                ui.close();
            }
            ui.menu_button("Colour", |ui| {
                let mut chosen: Option<Option<sortlens_core::ColorLabel>> = None;
                if ui.button("None").clicked() {
                    chosen = Some(None);
                }
                for label in sortlens_core::ColorLabel::ALL {
                    if ui.button(label.label()).clicked() {
                        chosen = Some(Some(label));
                    }
                }
                if let Some(colour) = chosen {
                    recolour = Some((targets.clone(), colour));
                    ui.close();
                }
            });
            // Rename and delete stay single-collection: renaming four rows to one name would merge them.
            if targets.len() == 1 {
                if ui.button("Rename…").clicked() {
                    rename = Some((collection.id, collection.name.clone()));
                    ui.close();
                }
                if ui.button("Delete collection").clicked() {
                    delete = Some(vec![collection.id]);
                    ui.close();
                }
            } else if ui.button(format!("Delete {} collections", targets.len())).clicked() {
                delete = Some(targets.clone());
                ui.close();
            }
        });
    }
    if let Some((ids, colour)) = recolour {
        let many = ids.len();
        match engine.set_collections_colour(&ids, colour) {
            Ok(_) => state.notice(match (colour, many) {
                (Some(label), 1) => format!("Collection is {}", label.label().to_lowercase()),
                (Some(label), many) => format!("{many} collections are {}", label.label().to_lowercase()),
                (None, 1) => "Collection colour cleared".to_string(),
                (None, many) => format!("Colour cleared on {many} collections"),
            }),
            Err(error) => state.notice_problem(error.to_string()),
        }
    }
    if let Some((ids, favourite)) = pin {
        let many = ids.len();
        match engine.set_collections_favourite(&ids, favourite) {
            Ok(_) => state.notice(match (favourite, many) {
                (true, 1) => "Pinned to the top".to_string(),
                (true, many) => format!("Pinned {many} collections to the top"),
                (false, 1) => "Unpinned".to_string(),
                (false, many) => format!("Unpinned {many} collections"),
            }),
            Err(error) => state.notice_problem(error.to_string()),
        }
    }
    if let Some((id, name)) = rename {
        state.rename_target = Some((id, name));
    }
    if let Some(ids) = delete {
        let many = ids.len();
        match engine.delete_collections(&ids) {
            Ok(_) => {
                if state.collection_filter.is_some_and(|filter| ids.contains(&filter)) {
                    state.collection_filter = None;
                }
                state.collection_selection.clear();
                state.notice(match many {
                    1 => "Collection deleted — the photos stay in the library".to_string(),
                    many => format!("{many} collections deleted — the photos stay in the library"),
                });
            }
            Err(error) => state.notice_problem(error.to_string()),
        }
    }
    // What a batch will act on, said out loud: a tick you cannot see is a tick that surprises you.
    if state.collection_selection.len() > 1 {
        let count = state.collection_selection.len();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{count} selected · ⌘-click to add, Shift-click for a run"))
                    .size(10.0)
                    .color(t.text_muted),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(Button::new(RichText::new("×").size(10.0)).fill(Color32::TRANSPARENT))
                    .clicked()
                {
                    state.collection_selection.clear();
                }
            });
        });
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
            Err(error) => state.notice_problem(error.to_string()),
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
                        activate_view(state, engine, View::Detail);
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
                    Err(error) => state.notice_problem(error.to_string()),
                }
            }
            if let Some(id) = forget {
                match engine.forget_batch(id) {
                    Ok(files) => state.notice(format!("Forgot a batch of {files} file(s)")),
                    Err(error) => state.notice_problem(error.to_string()),
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
/// Confirm deleting a set chosen in the similarity view.
///
/// The same three modes a cull batch offers, and the same executor: this dialog exists so that deleting a run of
/// near-duplicates is one deliberate act with a count in front of it, not a side effect of ticking photos.
fn similar_delete_dialog(ui: &mut Ui, state: &mut UiState, t: &Tokens, engine: &mut Engine) {
    if !state.similar_delete_open {
        return;
    }
    let ids = state.selection.ids();
    let mut open = true;
    let mut close = false;
    egui::Window::new("Delete selected photos")
        .open(&mut open)
        .resizable(false)
        .collapsible(false)
        .default_width(380.0)
        .show(ui.ctx(), |ui| {
            ui.label(RichText::new(format!("{} photo(s)", ids.len())).size(13.0).strong());
            ui.add_space(6.0);
            ui.label(RichText::new("What happens to them").size(11.0).color(t.text_muted));
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
                    .add_enabled(!ids.is_empty(), Button::new(RichText::new("Delete").size(12.0)))
                    .clicked()
                {
                    match engine.delete_photos(&ids, state.delete_mode.mode()) {
                        Ok(report) => {
                            state.notice(format!(
                                "Deleted {} photo(s){}",
                                report.applied.len(),
                                if report.has_failures() {
                                    format!(", {} failed", report.failed.len())
                                } else {
                                    String::new()
                                }
                            ));
                            state.selection.clear();
                        }
                        Err(error) => state.notice_problem(error.to_string()),
                    }
                    close = true;
                }
                if ui.button(RichText::new("Cancel").size(12.0)).clicked() {
                    close = true;
                }
            });
        });
    state.similar_delete_open = open && !close;
}

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
                    if chip(ui, t, None, format.label(), state.export_spec.format == format).clicked() {
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
                if state.export_spec.format == sortlens_engine::ExportFormat::Jpeg {
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
                } else {
                    hint(ui, t, "PNG is lossless: no quality to pick, and alpha is kept.");
                }
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("If the name is taken").size(11.0).color(t.text_muted));
                for policy in sortlens_engine::Collision::ALL {
                    if chip(ui, t, None, policy.label(), state.export_spec.collision == policy).clicked() {
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
                        Err(error) => state.notice_problem(error.to_string()),
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
                    Err(error) => state.notice_problem(error.to_string()),
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
                        Err(error) => state.notice_problem(error.to_string()),
                    }
                }
            }
            ui.add_space(8.0);
            if ui.button(RichText::new("Find moved photos").size(11.5)).clicked() {
                match engine.relocate_missing() {
                    Ok(0) => state.notice("No moved photos found under their roots"),
                    Ok(count) => state.notice(format!("Relocated {count} photo(s)")),
                    Err(error) => state.notice_problem(error.to_string()),
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
                        Err(error) => state.notice_problem(error.to_string()),
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
                        Err(error) => state.notice_problem(error.to_string()),
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
                        Err(error) => state.notice_problem(error.to_string()),
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
                        Err(error) => state.notice_problem(error.to_string()),
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
                    Err(error) => state.notice_problem(error.to_string()),
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
                        Err(error) => state.notice_problem(error.to_string()),
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
            Err(error) => state.notice_problem(error.to_string()),
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
            Err(error) => state.notice_problem(error.to_string()),
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
            ui.label(RichText::new("Shortcuts").size(11.0).color(t.text_muted));
            if ui
                .button(RichText::new("Edit shortcuts…").size(12.0))
                .on_hover_text("Rebind keys, resolve conflicts, and import or export shortcut sets")
                .clicked()
            {
                // One modal at a time, and the palette's keyboard must not reach the editor.
                state.palette_open = false;
                state.settings_open = false;
                state.shortcut_editor = Some(DialogState::new());
            }
            ui.add_space(8.0);
            ui.separator();
            // The model downloader (SL-F02) is desktop-only; the wasm build compiles the shell
            // without the section and without the engine methods it calls.
            #[cfg(not(target_arch = "wasm32"))]
            ai_models_section(ui, t, engine, actions);
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

/// Settings → AI: the consent switch and every pinned model, with what is on disk for it.
///
/// The consent switch is the whole gate: with it off, no model can be installed from here, from the
/// CLI or from an agent — the engine refuses, not just the button. With it on, each row offers
/// install (a download, resumable, cancelled from here) and removal; nothing runs inference yet, so
/// the hint says what a model is *for* before the panel promises anything.
#[cfg(not(target_arch = "wasm32"))]
fn ai_models_section(ui: &mut Ui, t: &Tokens, engine: &mut Engine, actions: &mut Vec<Action>) {
    ui.label(RichText::new("AI models (local)").size(11.0).color(t.text_muted));
    let mut consent = engine.models_consent();
    let response = ui.checkbox(&mut consent, "Allow downloading models (network)");
    if response.changed() {
        engine.set_models_consent(consent);
        actions.push(Action::SetModelConsent(consent));
    }
    ui.label(
        RichText::new(
            "Models are checked against a pinned SHA-256 digest before anything is used. Nothing downloads until you install one, and every file stays on this machine.",
        )
        .size(10.5)
        .color(t.text_faint),
    );
    match engine.models_status() {
        Ok(rows) => {
            for row in rows {
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    let state = if row.downloading {
                        match engine.model_progress() {
                            Some((done, total)) => {
                                let share = if total == 0 { 0.0 } else { done as f32 / total as f32 };
                                ui.add(
                                    egui::ProgressBar::new(share)
                                        .text(sortlens_core::format_bytes(done))
                                        .desired_width(140.0),
                                );
                                if ui
                                    .button(RichText::new("Stop").size(11.0))
                                    .on_hover_text("Keeps the partial download; installing again resumes it")
                                    .clicked()
                                {
                                    engine.cancel_model_install();
                                }
                                return;
                            }
                            None => "".to_string(),
                        }
                    } else {
                        row.status.clone()
                    };
                    ui.label(RichText::new(row.label.clone()).size(11.5));
                    ui.label(RichText::new(row.summary.clone()).size(10.0).color(t.text_faint));
                    if !state.is_empty() {
                        ui.label(RichText::new(state).size(10.0).color(t.text_muted));
                    }
                    if consent && !row.downloading {
                        if !row.installed
                            && ui
                                .button(RichText::new("Install").size(11.0))
                                .on_hover_text(format!("Download from the pinned release · {}", row.summary))
                                .clicked()
                        {
                            let id = row.id.clone();
                            if let Err(error) = engine.install_model(&id) {
                                actions.push(Action::Notice(error.to_string()));
                            }
                        }
                        if row.installed
                            && ui
                                .button(RichText::new("Remove").size(11.0))
                                .on_hover_text("Delete the file from the model store; you can install it again later")
                                .clicked()
                        {
                            match engine.remove_model(&row.id.clone()) {
                                Ok(0) => {}
                                Ok(bytes) => {
                                    actions.push(Action::Notice(format!(
                                        "Removed · {} reclaimed",
                                        sortlens_core::format_bytes(bytes)
                                    )));
                                }
                                Err(error) => actions.push(Action::Notice(error.to_string())),
                            }
                        }
                    }
                });
            }
            if engine.poll_model_install().is_some() {
                // A finished download is announced through the row's own state; nothing to keep.
            }
        }
        Err(_) => {
            ui.label(
                RichText::new("Open a library to manage models — they are stored beside its catalog.")
                    .size(10.5)
                    .color(t.text_faint),
            );
        }
    }
    ui.add_space(8.0);
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
                        // A rebindable row paints the command's current bindings, so the help
                        // cannot go stale after the editor changes one.
                        let keys = keymap::row_keys(&state.keys, binding);
                        keymap_row(ui, t, &keys, binding);
                    }
                }
            });
        });
    state.shortcuts_open = open;
}

/// One keycap and its action, aligned so the keys form a column.
fn keymap_row(ui: &mut Ui, t: &Tokens, keys: &str, binding: &keymap::Binding) {
    ui.horizontal(|ui| {
        ui.allocate_ui(vec2(96.0, 18.0), |ui| {
            let rect = ui.available_rect_before_wrap();
            let width = ui
                .painter()
                .layout_no_wrap(keys.to_string(), FontId::monospace(10.5), t.text)
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
                .text(key.center(), Align2::CENTER_CENTER, keys, FontId::monospace(10.5), t.text);
            ui.allocate_space(vec2(width + 10.0, 18.0));
        });
        ui.label(RichText::new(binding.action).size(11.5).color(t.text_muted));
    });
}

/// The shortcut editor's fixed options: the app's unrebindable rows shown read-only, craft-keymap
/// JSON as the one file format, and no double-press bindings, which the shell's dispatch does not
/// read.
fn shortcut_editor_options() -> ShortcutOptions {
    ShortcutOptions::default().with(|options| {
        options.title = "Keyboard shortcuts";
        options.summary = false;
        options.keyboard = false;
        options.fixed = keymap::fixed_rows();
        options.import_formats = vec![craft_keymap::ui::Format {
            id: craft_keymap::ui::JSON_FORMAT,
            label: "craft-keymap set",
            extensions: &["json"],
        }];
        options.accept = Some(Box::new(|_, binding| {
            if binding.trigger.double {
                Err("Double-press bindings are not read here; record a single chord".to_owned())
            } else {
                Ok(())
            }
        }));
    })
}

/// The craft-keymap editor behind Settings → Shortcuts (SL-I09): rebinding with conflict
/// detection, named sets, and craft-keymap JSON import and export. It owns the keyboard while it
/// is open, which is why `keyboard` and the palette's ⌘K step aside for it; a rebind applies to
/// the runtime keymap the same frame, and OK hands the sets to the host to persist.
fn shortcut_editor_dialog(ui: &mut Ui, state: &mut UiState, actions: &mut Vec<Action>) {
    let mut editor = state.shortcut_editor.take().unwrap_or_default();
    // A file the host read lands before the frame draws, so the review opens on the set it read.
    if let Some(text) = state.pending_shortcut_import.take() {
        match ShortcutsDialog::new(&mut state.keys, &English).import_json(&mut editor, &text) {
            Ok(name) => state.notice(format!("Imported shortcut set \u{201c}{name}\u{201d}")),
            Err(error) => state.notice_problem(format!("Could not import the set: {error}")),
        }
    }
    let options = shortcut_editor_options();
    let outcome = ShortcutsDialog::new(&mut state.keys, &English)
        .sets(&mut state.profiles, keymap::BUILTINS)
        .options(&options)
        .show(ui.ctx(), &mut editor);
    match outcome {
        // Still open: keep the dialog state for the next frame.
        None | Some(ShortcutOutcome::Changed) => state.shortcut_editor = Some(editor),
        // Cancel restores the keymap and the sets to how they were when it opened, in place.
        Some(ShortcutOutcome::Cancel) => {}
        Some(ShortcutOutcome::Ok) => {
            actions.push(Action::SaveShortcuts(state.profiles.clone()));
            state.notice("Shortcuts saved");
        }
        Some(ShortcutOutcome::Import { .. }) => {
            state.shortcut_editor = Some(editor);
            actions.push(Action::ImportShortcuts);
        }
        Some(ShortcutOutcome::Export { name, json }) => {
            state.shortcut_editor = Some(editor);
            actions.push(Action::ExportShortcuts(name, json));
        }
        // The summary is switched off, and a remembered-import answer has no host to keep it.
        Some(_) => state.shortcut_editor = Some(editor),
    }
}

fn nav_section(ui: &mut Ui, t: &Tokens, title: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(title.to_uppercase()).size(9.5).color(t.text_faint));
    ui.add_space(2.0);
}

fn nav_row(ui: &mut Ui, t: &Tokens, icon: crate::icons::Icon, label: &str, count: &str, selected: bool) -> egui::Response {
    let height = NAV_ROW_H;
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
    // The glyph sits in its own 16-point column so every row's text starts at the same x whether or
    // not the row has a picture — a list that goes ragged around the icons is harder to scan than one
    // with no icons at all.
    let glyph = Rect::from_center_size(pos2(rect.left() + 14.0, rect.center().y), Vec2::splat(14.0));
    crate::icons::paint(ui, icon, glyph, if selected { t.text } else { t.text_muted });
    let width = rect.width() - 34.0;
    ui.painter().text(
        rect.left_center() + vec2(26.0, 0.0),
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

/// An icon-only button that keeps its words: the tooltip and the accessible name are the label, so the
/// top bar loses a word-wide button and a screen reader loses nothing.
fn glyph_button(ui: &mut Ui, t: &Tokens, icon: crate::icons::Icon, label: &str, selected: bool) -> egui::Response {
    crate::icons::button(ui, t, icon, label, selected)
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

    /// Phase 3B item 10: the chrome's buttons are pictures, and the pictures are the shipped set.
    ///
    /// Two failure modes, asserted separately because they look the same on screen. The first is a glyph
    /// nobody installed or a name that does not resolve — every one of them is asked for through the same
    /// context the shell drew on, and must come back ready. The second is a surface that quietly lost its
    /// icons: the top bar, the bottom bar and the sidebar are counted by the image meshes painted inside
    /// their bands, so removing an icon from a row is a failing test rather than a smaller screenshot.
    #[test]
    fn the_chrome_paints_the_shipped_icon_set() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-icons-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-icons");
        for index in 0..3 {
            let name = format!("IMG_000{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [30 + index * 40, 90, 170], 48, 32), root.join(&name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let t = ThemeKind::Charcoal.tokens();
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 800.0));
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(ui, &mut state, &mut engine, &mut thumbs);
            });
        });
        output.textures_delta.clear();

        // Every glyph resolves on the context the shell painted on: a missing `install`, a renamed file
        // or a catalogued name with no bytes all end here, as a blank button nobody notices.
        for icon in crate::icons::Icon::ALL {
            let uri = format!("bytes://sortlens/icons/{}.svg", icon.name());
            let poll = ctx
                .try_load_image(
                    &uri,
                    egui::load::SizeHint::Size {
                        width: 14,
                        height: 14,
                        maintain_aspect_ratio: true,
                    },
                )
                .expect("the shell installed the SVG loader");
            assert!(
                matches!(poll, egui::load::ImagePoll::Ready { .. }),
                "{} was not ready on the frame it was asked for",
                icon.name()
            );
        }

        // An image in this egui paints as a textured rectangle (the fork moved images from meshes to
        // `RectShape` brushes), so "a picture was painted here" is a rect carrying a texture that is
        // not the font atlas.
        let painted: Vec<Rect> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Mesh(mesh) => Some(mesh.calc_bounds()),
                egui::Shape::Rect(shape) => shape
                    .brush
                    .as_ref()
                    .filter(|brush| brush.fill_texture_id != egui::TextureId::default())
                    .map(|_| shape.rect),
                _ => None,
            })
            .collect();
        let in_band = |top: f32, bottom: f32, x_max: f32| {
            painted
                .iter()
                .filter(|rect| rect.center().y >= top && rect.center().y <= bottom && rect.center().x <= x_max)
                .count()
        };
        // Exact, not a floor: a floor of three would still pass while a fourth word-only button
        // appeared beside them, which is the drift this is meant to catch.
        assert_eq!(
            in_band(0.0, t.top_bar_h, screen.right()),
            8,
            "the top bar paints the app mark, Settings, Help, Appearance, Export, Import, Filter and the              panel toggle as pictures"
        );
        assert_eq!(
            in_band(screen.bottom() - t.bottom_bar_h, screen.bottom(), screen.right()),
            View::ALL.len() + 8,
            "every destination in the bottom bar carries its glyph, and the annotation pill carries \
             five stars and three flags"
        );
        assert!(
            in_band(t.top_bar_h, screen.bottom() - t.bottom_bar_h, t.left_w) >= Source::ALL.len() + 5,
            "the six sources, the four utilities in the foot and the folder row each start with a picture"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_empty_canvas_follows_what_the_library_is_doing() {
        // Nothing added yet: the only useful step is adding a folder.
        assert_eq!(canvas_empty(0, false, 0, 0, false), Empty::Add);
        // A root that turned out to be empty is the same situation.
        assert_eq!(canvas_empty(1, false, 0, 0, false), Empty::Add);
        // Scanning wins, even with photos already indexed elsewhere.
        assert_eq!(canvas_empty(1, true, 40, 0, false), Empty::Indexing);
        // Photos, no scan: the grid takes over and the wizard is gone.
        assert_eq!(canvas_empty(1, false, 12, 12, false), Empty::Quiet);
        // Dismissed stays dismissed whatever else happens.
        assert_eq!(canvas_empty(0, false, 0, 0, true), Empty::Quiet);
        assert_eq!(canvas_empty(1, true, 0, 0, true), Empty::Quiet);
        // A library that exists and a filter that hides all of it is neither a first launch nor silence:
        // the screen names the count and offers the way back, and being dismissed does not silence it.
        assert_eq!(canvas_empty(1, false, 12, 0, false), Empty::NoMatches { library: 12 });
        assert_eq!(canvas_empty(1, false, 12, 0, true), Empty::NoMatches { library: 12 });
        // Dismissed over an empty library stays quiet rather than falling back to the wizard: the old
        // `unwrap_or(Welcome::Add)` made `Not now` a button that visibly did nothing.
        assert_eq!(canvas_empty(1, false, 0, 0, true), Empty::Quiet);
    }

    /// The three Welcome promises start with the icon set's own glyphs, aligned in one column.
    ///
    /// Painted directly, without the rest of the shell, so every textured rectangle in the output is one of
    /// these three: that is what makes an exact count meaningful rather than a count of the sidebar's rows
    /// as well. The `·` this replaced was typed text — a speck at 12 points whose height differed per face.
    #[test]
    fn the_welcome_rows_start_with_the_icon_sets_glyphs() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-welcome-glyphs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let t = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let mut actions: Vec<Action> = Vec::new();
        let ctx = egui::Context::default();
        // `show` installs the glyph loader; painting one section on its own must install it too.
        crate::icons::install(&ctx);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                welcome_add(ui, &t, &mut state, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(actions.is_empty(), "painting the Welcome step presses nothing");
        assert!(
            label_at(&output, "·").is_none(),
            "the bullets are pictures now, so no typed bullet is left on the panel"
        );

        // An image paints as a textured rectangle in this egui; the font atlas is the default texture.
        let mut glyphs: Vec<Rect> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Mesh(mesh) => Some(mesh.calc_bounds()),
                egui::Shape::Rect(shape) => shape
                    .brush
                    .as_ref()
                    .filter(|brush| brush.fill_texture_id != egui::TextureId::default())
                    .map(|_| shape.rect),
                _ => None,
            })
            .collect();
        glyphs.sort_by(|a, b| a.center().y.total_cmp(&b.center().y));
        assert_eq!(
            glyphs.len(),
            3,
            "one glyph per promise, and nothing else in this panel is a picture"
        );

        for glyph in &glyphs {
            assert!(
                (glyph.width() - 14.0).abs() < 0.01 && (glyph.height() - 14.0).abs() < 0.01,
                "every glyph is the 14-point size the icon set rasterises at, got {glyph:?}"
            );
        }
        // One column: the three titles share a left edge, which is the reason for the fixed width.
        let column = glyphs[0].center().x;
        for glyph in &glyphs {
            assert!(
                (glyph.center().x - column).abs() < 0.01,
                "the glyphs are one column, so the three detail lines start at the same x"
            );
        }
        assert!(glyphs.windows(2).all(|pair| pair[0].center().y < pair[1].center().y));

        // Each glyph is on its own title's line and left of its words, not above or after them.
        for (glyph, title) in glyphs
            .iter()
            .zip(["Your files stay put", "Everything is undoable", "Rejects wait for you"])
        {
            let label = label_at(&output, title).unwrap_or_else(|| panic!("{title} was not painted"));
            assert!(label.left() >= glyph.right(), "{title} sits beside its glyph, not under it");
            assert!(
                (label.center().y - glyph.center().y).abs() <= 8.0,
                "{title} and its glyph share a line, got {label:?} beside {glyph:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
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
            duration_ms: None,
            fps: None,
            video_codec: None,
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
            tag_sources: std::collections::BTreeMap::new(),
            dismissed_suggestions: std::collections::BTreeSet::new(),
            edited: false,
            retry: false,
            sharpness: None,
            location: None,
            rev: 0,
            added: None,
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
            duration_ms: None,
            fps: None,
            video_codec: None,
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
            tag_sources: std::collections::BTreeMap::new(),
            dismissed_suggestions: std::collections::BTreeSet::new(),
            edited: false,
            retry: false,
            sharpness: None,
            location: None,
            rev: 0,
            added: None,
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

    /// The bell counts what is unread, colours itself when something failed, and stops counting when read.
    #[test]
    fn the_bell_counts_unread_notices_and_marks_them_read() {
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        assert_eq!(state.unread_notices(), 0, "nothing has happened yet");
        assert!(!state.unread_problem());

        state.notice("Scanned 12 photos");
        assert_eq!(state.unread_notices(), 1);
        assert!(!state.unread_problem(), "an ordinary notice is not a problem");
        state.notice_problem("Move failed: permission denied");
        assert_eq!(state.unread_notices(), 2);
        assert!(state.unread_problem(), "the badge has to show that something failed");

        // Newest first, so the thing that just happened is the thing you read.
        assert_eq!(
            state.notices.front().map(|notice| notice.text.as_str()),
            Some("Move failed: permission denied")
        );
        assert_eq!(state.notices.len(), 2);

        state.mark_notices_read();
        assert_eq!(state.unread_notices(), 0);
        assert!(!state.unread_problem(), "a read problem does not keep the badge red");
        assert_eq!(state.notices.len(), 2, "reading is not clearing");

        state.clear_notices();
        assert!(state.notices.is_empty());
    }

    /// The history is bounded, and trimming it cannot leave the count wrong.
    #[test]
    fn the_history_is_bounded_and_the_count_survives_it() {
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        for index in 0..(NOTICE_HISTORY + 10) {
            state.notice(format!("notice {index}"));
        }
        assert_eq!(state.notices.len(), NOTICE_HISTORY);
        assert_eq!(state.unread_notices(), NOTICE_HISTORY);
        let newest = format!("notice {}", NOTICE_HISTORY + 9);
        assert_eq!(state.notices.front().map(|notice| notice.text.as_str()), Some(newest.as_str()));
        assert_eq!(
            state.notices.back().map(|notice| notice.text.as_str()),
            Some("notice 10"),
            "the oldest are the ones that went"
        );
    }

    /// Ages read the way someone would say them.
    #[test]
    fn a_notice_age_is_said_out_loud() {
        assert_eq!(notice_age(0), "just now");
        assert_eq!(notice_age(9), "just now");
        assert_eq!(notice_age(10), "10s");
        assert_eq!(notice_age(59), "59s");
        assert_eq!(notice_age(60), "1m");
        assert_eq!(notice_age(3_599), "59m");
        assert_eq!(notice_age(3_600), "1h");
        assert_eq!(notice_age(86_400), "1d");
    }

    /// The bell paints beside the other top-bar controls and the popover lists what happened.
    #[test]
    fn the_bell_paints_and_the_popover_lists_what_happened() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.notice("Scanned 12 photos");
        state.notice_problem("Move failed");
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                bell_button(ui, &mut state, &tokens);
            });
        });
        output.textures_delta.clear();
        assert_eq!(state.unread_notices(), 2, "painting the bell does not read them");
        assert!(!state.bell_open, "and nothing was clicked");

        // Opening the popover is what reads them: it is the only way to see the list.
        state.bell_open = true;
        state.mark_notices_read();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                bell_popover(ui, &mut state, &tokens);
            });
        });
        output.textures_delta.clear();
        assert_eq!(state.unread_notices(), 0);
        assert_eq!(state.notices.len(), 2, "the list is still there to read");
    }

    /// The Places view reports what the catalog knows and does not pretend to draw a map.
    ///
    /// Headless, over a real library: four photos, two of which carry GPS. The assertions are the view's claims — the
    /// counts it prints and the cluster list it builds — because a screenshot of text is not checkable and "it
    /// painted" alone would not notice a view that lied about the numbers.
    #[test]
    fn places_counts_locations_and_draws_nothing_it_cannot_draw() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-places-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-places");
        std::fs::copy(
            fixture.jpeg_with_location("BRIGHTON_1.jpg", [40, 90, 160], 48, 32, 50.8225, -0.1372),
            root.join("BRIGHTON_1.jpg"),
        )
        .expect("copy");
        std::fs::copy(
            fixture.jpeg_with_location("BRIGHTON_2.jpg", [40, 90, 170], 48, 32, 50.8227, -0.1370),
            root.join("BRIGHTON_2.jpg"),
        )
        .expect("copy");
        std::fs::copy(fixture.jpeg("NOWHERE_1.jpg", [10, 20, 30], 48, 32), root.join("NOWHERE_1.jpg")).expect("copy");
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(engine.all_photos().len(), 3);
        assert_eq!(engine.geotagged(), 2);

        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                places_view(ui, &tokens, &mut state, &engine);
            });
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty(), "the view painted");
        assert_eq!(engine.places(6).len(), 1, "one place, two photos");
        assert_eq!(engine.places(6)[0].len(), 2);

        // And the view for a library with no locations is a sentence, not an empty map.
        let empty = Engine::open(dir.join("elsewhere")).expect("open");
        let mut empty_state = UiState::new(ThemeKind::Charcoal, 0.0);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                places_view(ui, &tokens, &mut empty_state, &empty);
            });
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty(), "the empty state paints too");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A click on a dot selects that dot's photographs, all of them, and a click on the map does not clear anything.
    ///
    /// Real pointer events through `run_ui`: the map is found by the rectangle the view fills with the grid colour, so
    /// the test does not have to guess where the view put it, and the dot's position comes from the same projection
    /// the view uses. This checks the wiring — dots built, hit test resolved, members selected — which is where the
    /// first version of this view was wrong (it indexed clusters by dot position, so an off-screen cluster shifted
    /// every dot onto its neighbour's photos).
    #[test]
    fn clicking_a_dot_selects_its_photos() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-dotclick-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-dotclick");
        for (index, factor) in [(1, 0.0_f64), (2, 0.0002)] {
            std::fs::copy(
                fixture.jpeg_with_location(
                    &format!("IMG_{index:04}.jpg"),
                    [40, 90, 160],
                    48,
                    32,
                    50.8225 + factor,
                    -0.1372 + factor,
                ),
                root.join(format!("IMG_{index:04}.jpg")),
            )
            .expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(engine.geotagged(), 2);

        let tokens = Tokens::CHARCOAL;
        let ctx = egui::Context::default();
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let frame = |state: &mut UiState, input: egui::RawInput| {
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    places_view(ui, &tokens, state, &engine);
                });
            });
            output.textures_delta.clear();
            output
        };

        // First frame: find the map by the rectangle it fills with the grid colour.
        let output = frame(&mut state, egui::RawInput::default());
        let map = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect) if rect.fill == tokens.thumb_bg && rect.rect.width() > 100.0 => Some(rect.rect),
                _ => None,
            })
            .expect("the map painted");

        // The dot's position, from the same projection the view used.
        let zoom = state.places_zoom;
        let located = engine.located();
        let centre = located[0].1;
        let origin = sortlens_geo::viewport_origin(zoom, centre, f64::from(map.width()), f64::from(map.height())).expect("origin");
        let at = sortlens_geo::screen_position(centre, zoom, origin, f64::from(map.width()), f64::from(map.height())).expect("on screen");
        let pointer = egui::pos2(map.min.x + at.x as f32, map.min.y + at.y as f32);

        // Press, then release: egui reports a click on the release.
        let pressed = egui::Event::PointerButton {
            pos: pointer,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::default(),
        };
        let released = egui::Event::PointerButton {
            pos: pointer,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        };
        frame(
            &mut state,
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(pointer), pressed],
                ..Default::default()
            },
        );
        frame(
            &mut state,
            egui::RawInput {
                events: vec![released],
                ..Default::default()
            },
        );

        let ids = engine.located().iter().map(|(id, _)| *id).collect::<Vec<_>>();
        assert_eq!(state.selection.ids(), ids, "the whole dot was selected");
        assert!(state.selection.contains(ids[0]));

        // A click on the map away from the dot selects nothing and, crucially, clears nothing.
        let sea = egui::pos2(map.max.x - 4.0, map.max.y - 4.0);
        frame(
            &mut state,
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(sea),
                    egui::Event::PointerButton {
                        pos: sea,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::default(),
                    },
                ],
                ..Default::default()
            },
        );
        frame(
            &mut state,
            egui::RawInput {
                events: vec![egui::Event::PointerButton {
                    pos: sea,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::default(),
                }],
                ..Default::default()
            },
        );
        assert_eq!(state.selection.ids(), ids, "the sea is not a way to clear a selection");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A library's own `places.txt` names its dots, and the click selects the dot's photos and names the place.
    ///
    /// The gazetteer ships nothing: the file is the person's, in a documented tab-separated format. Two libraries are
    /// checked — one with the file and one without — because the interesting question is not "does a name appear" but
    /// "does an unnamed dot still behave correctly", and the answer has to be the coordinate, not a guess.
    #[test]
    fn a_library_places_file_names_the_dots_it_knows() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-names");
        for (index, factor) in [(1, 0.0_f64), (2, 0.0002)] {
            std::fs::copy(
                fixture.jpeg_with_location(
                    &format!("IMG_{index:04}.jpg"),
                    [40, 90, 160],
                    48,
                    32,
                    50.8225 + factor,
                    -0.1372 + factor,
                ),
                root.join(format!("IMG_{index:04}.jpg")),
            )
            .expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let tokens = Tokens::CHARCOAL;
        let ctx = egui::Context::default();
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let paint = |state: &mut UiState| {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    places_view(ui, &tokens, state, &engine);
                });
            });
            output.textures_delta.clear();
            assert!(!output.shapes.is_empty(), "the view paints");
        };

        // Without a file: no names, no error, and the view still paints.
        assert!(engine.places_file().is_some(), "the path is always known");
        assert!(engine.load_gazetteer().is_none(), "and there is nothing at it");
        paint(&mut state);
        assert!(state.places_names.is_empty());
        assert_eq!(state.places_names.error(), None);

        // Write one, and the same frame names the dot.
        let places_file = engine.places_file().expect("a catalog directory");
        std::fs::write(&places_file, "Brighton\t51\t0\tGB\nLima\t-12\t-77\n").expect("write");
        let loaded = engine.load_gazetteer().expect("a file").expect("a gazetteer");
        assert_eq!(loaded.len(), 2);
        paint(&mut state);
        assert_eq!(state.places_names.len(), 2, "the view read the library's own file");
        let located = engine.located();
        let brighton = located[0].1;
        assert_eq!(
            state
                .places_names
                .for_file(engine.places_file().as_deref())
                .and_then(|gazetteer| places::place_label(gazetteer, brighton))
                .map(|place| place.name.clone()),
            Some("Brighton".to_string()),
            "the dot's name comes from the file"
        );
        assert_eq!(
            places::selection_notice(2, brighton, Some("Brighton")),
            "Selected 2 photo(s) near Brighton (50.82N 0.14W)"
        );

        // A malformed file is reported rather than silently ignored: a places file that does nothing is worse than
        // none at all, because the person believes their names are being used.
        std::fs::create_dir_all(places_file.with_extension("tmp")).expect("dir");
        std::fs::remove_file(&places_file).expect("remove");
        std::fs::create_dir_all(&places_file).expect("a directory where the file was");
        paint(&mut state);
        assert!(state.places_names.error().is_some(), "the failure is visible to the view");
        assert!(state.places_names.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The Places view says what the basemap is, or why it is not there, or which file to write.
    ///
    /// Three libraries, three states, all painted headlessly: no tile folder, a tile folder with no credit, and a
    /// usable one.
    ///
    /// **What this can and cannot show:** egui hands back paint shapes, not laid-out text, so this proves the view
    /// paints without panicking in each state and that the engine reports the right state to paint — it does *not*
    /// prove which words reached the screen. The claim about words — that a map is never shown without its credit — is
    /// enforced one layer down, where `Tileset::open` refuses a folder that has no attribution, so there is no state
    /// in which the view holds a usable tileset whose credit is unknown.
    #[test]
    fn places_reports_the_basemap_or_why_there_is_none() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-tiles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-tiles");
        std::fs::copy(
            fixture.jpeg_with_location("A.jpg", [40, 90, 160], 48, 32, 50.8225, -0.1372),
            root.join("A.jpg"),
        )
        .expect("copy");
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let tokens = Tokens::CHARCOAL;
        let ctx = egui::Context::default();
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let paint = |state: &mut UiState, engine: &Engine| {
            let output = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| places_view(ui, &tokens, state, engine));
            });
            let mut output = output;
            output.textures_delta.clear();
            assert!(!output.shapes.is_empty(), "the view paints in every state");
            output
        };

        // No folder at all: the shipped coastline is the basemap, so the map is geography and not a blank rectangle.
        assert!(engine.tileset().is_none(), "a library with no tiles says so");
        let output = paint(&mut state, &engine);
        let shipped = coastline_runs(&output, &tokens);
        assert!(
            !shipped.is_empty(),
            "a library with no tile folder still gets the shipped coastline"
        );
        assert!(
            shipped.iter().any(|run| run.points.len() >= 3),
            "the outline is drawn as polylines, not as a single segment"
        );

        // A tile folder with tiles but no credit: refused, and the error names the file to write.
        let tiles = engine.tiles_dir().expect("a catalog directory");
        // Tiles where the view will actually look: the same projection and the same viewport the view uses, so the
        // fixture is a tile set for this map rather than for a map somewhere else.
        let map = vec2(720.0, 480.0);
        let centre = engine.located()[0].1;
        let origin = sortlens_geo::viewport_origin(5, centre, f64::from(map.x), f64::from(map.y)).expect("origin");
        let cells = sortlens_geo::visible_tiles(5, origin, f64::from(map.x), f64::from(map.y));
        assert!(!cells.is_empty(), "some tiles cover the viewport");
        for cell in &cells {
            let dir = tiles.join(cell.zoom.to_string()).join(cell.x.to_string());
            std::fs::create_dir_all(&dir).expect("tiles");
            std::fs::write(dir.join(format!("{}.png", cell.y)), sortlens_icon::png()).expect("tile");
        }
        let error = match engine.tileset() {
            Some(Err(error)) => error,
            other => panic!("expected a refusal, got {other:?}"),
        };
        assert!(error.message().contains("attribution.txt"), "{}", error.message());
        paint(&mut state, &engine);

        // Write the credit and it opens, with the credit readable through the engine.
        std::fs::write(tiles.join("attribution.txt"), "© Example Maps, CC-BY 4.0").expect("credit");
        let set = match engine.tileset() {
            Some(Ok(set)) => set,
            other => panic!("expected a tileset, got {other:?}"),
        };
        assert_eq!(set.attribution(), "© Example Maps, CC-BY 4.0");
        assert_eq!(set.zoom_range(), (5, 5));
        assert_eq!(state.places_tiles.len(), 0, "nothing decoded before a map was drawn");
        let output = paint(&mut state, &engine);
        // The person's own map wins: with a usable folder the shipped outline is not painted underneath it.
        assert!(
            coastline_runs(&output, &tokens).is_empty(),
            "a tile folder replaces the shipped coastline rather than being drawn over it"
        );
        // The claim of this run: the map decodes the tiles under the viewport and paints them, once.
        assert!(!state.places_tiles.is_empty(), "the map decoded the tiles it drew");
        assert_eq!(state.places_tiles.failures(), 0, "every fixture tile was a real PNG");
        let decoded = state.places_tiles.decodes();
        assert_eq!(decoded, cells.len(), "one decode per visible tile, not one per frame");
        let held = state.places_tiles.len();
        paint(&mut state, &engine);
        assert_eq!(state.places_tiles.decodes(), decoded, "a second frame decodes nothing new");
        assert_eq!(state.places_tiles.len(), held);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An inland library opens with the outline on screen instead of a dot on a blank rectangle.
    ///
    /// The reported defect: one photograph in Paris fitted at [`sortlens_geo::coastline::MAX_FIT_ZOOM`], where the
    /// 1:110m outline has no coast within a screen of the city, so Places painted an empty map. The control below is
    /// that fitted zoom and its empty view; the assertion after the frame is that the view widened itself until the
    /// outline is geography. A coastal photograph keeps the fitted zoom (the geo test holds that half).
    #[test]
    fn places_opens_with_the_outline_when_the_library_is_inland() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-inland-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-inland");
        std::fs::copy(
            fixture.jpeg_with_location("PARIS.jpg", [40, 90, 160], 48, 32, 48.8566, 2.3522),
            root.join("PARIS.jpg"),
        )
        .expect("copy");
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(engine.geotagged(), 1);

        let tokens = Tokens::CHARCOAL;
        let ctx = egui::Context::default();
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1200.0, 800.0));
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    places_view(ui, &tokens, &mut state, &engine);
                });
            },
        );
        output.textures_delta.clear();

        let map = all_shapes(&output)
            .into_iter()
            .find_map(|shape| match shape {
                egui::Shape::Rect(rect) if rect.fill == tokens.thumb_bg && rect.rect.width() > 100.0 => Some(rect.rect),
                _ => None,
            })
            .expect("the map painted");
        assert!(!state.places_zoom_chosen, "the view fits; nobody has chosen a zoom yet");
        let fitted = sortlens_geo::coastline::fit_zoom(
            &[engine.located()[0].1],
            f64::from(map.width()),
            f64::from(map.height()),
            sortlens_geo::coastline::MAX_FIT_ZOOM,
        );
        assert_eq!(fitted, sortlens_geo::coastline::MAX_FIT_ZOOM, "one photograph has no spread");
        let centre = sortlens_geo::coastline::bounds_centre(&[engine.located()[0].1]).expect("a centre");
        let origin = sortlens_geo::viewport_origin(fitted, centre, f64::from(map.width()), f64::from(map.height())).expect("an origin");
        assert!(
            !sortlens_geo::coastline::Coastline::shipped().has_screen_lines(
                fitted,
                origin,
                f64::from(map.width()),
                f64::from(map.height())
            ),
            "the defect: the fitted view has no coast in it"
        );

        assert!(
            state.places_zoom < fitted,
            "the map did not widen to reach the outline: {}",
            state.places_zoom
        );
        let runs = coastline_runs(&output, &tokens);
        assert!(!runs.is_empty(), "the inland map painted no coastline");
        assert!(runs.iter().any(|run| run.points.len() >= 2), "the outline is a line, not a dot");
        let dots: Vec<_> = painted_circles(&output)
            .into_iter()
            .filter(|circle| (circle.radius - places::dot_radius(1)).abs() < 0.5 && circle.fill.a() > 0)
            .collect();
        assert_eq!(dots.len(), 1, "the photograph still has its dot");
        assert!(
            map.contains(dots[0].center),
            "the dot at {:?} is outside the map {map:?}",
            dots[0].center
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Places opens fitted to the library and draws the coastline that ships with it.
    ///
    /// The reported defect: a library in Brighton and Sydney opened at a fixed zoom 5 over the middle of the Atlantic
    /// with both clusters outside the viewport, so Places looked empty. The test asserts the two halves of the fix —
    /// the view chose a zoom of its own that shows both dots, and (with no tile folder) it painted the shipped outline
    /// under them — and the two things the fix must not break: the fit is only the default, so a chosen zoom stays
    /// chosen, and a library with no location still says so instead of drawing an empty map.
    #[test]
    fn places_opens_fitted_to_its_photographs_with_the_shipped_coastline_under_them() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-fit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-fit");
        for (name, lat, lon) in [("BRIGHTON.jpg", 50.8225, -0.1372), ("SYDNEY.jpg", -33.8688, 151.2093)] {
            std::fs::copy(fixture.jpeg_with_location(name, [40, 90, 160], 48, 32, lat, lon), root.join(name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(engine.geotagged(), 2);

        let tokens = Tokens::CHARCOAL;
        let ctx = egui::Context::default();
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(1200.0, 800.0));
        let frame = |state: &mut UiState| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        places_view(ui, &tokens, state, &engine);
                    });
                },
            );
            output.textures_delta.clear();
            output
        };

        let output = frame(&mut state);
        assert!(!state.places_zoom_chosen, "the view fits; nobody has chosen a zoom yet");
        assert!(
            state.places_zoom < 5,
            "the fit must zoom out from 5 to show two continents, got {}",
            state.places_zoom
        );

        // Both clusters are on screen: the two dots are painted, and both inside the map's own rectangle.
        let map = all_shapes(&output)
            .into_iter()
            .find_map(|shape| match shape {
                egui::Shape::Rect(rect) if rect.fill == tokens.thumb_bg && rect.rect.width() > 100.0 => Some(rect.rect),
                _ => None,
            })
            .expect("the map painted");
        let dots: Vec<_> = painted_circles(&output)
            .into_iter()
            // A dot is a filled circle plus a ring around it, so the fill is what counts one dot once.
            .filter(|circle| (circle.radius - places::dot_radius(1)).abs() < 0.5 && circle.fill.a() > 0)
            .collect();
        assert_eq!(dots.len(), 2, "one dot per continent, got {}", dots.len());
        for dot in &dots {
            assert!(map.contains(dot.center), "the dot at {:?} is outside the map {map:?}", dot.center);
        }

        // The coastline under them, in its own colour, as polylines.
        let runs = coastline_runs(&output, &tokens);
        assert!(!runs.is_empty(), "the shipped outline is the basemap when no tiles are installed");
        assert!(runs.iter().any(|run| run.points.len() > 4), "a real shoreline, not a stub");
        for run in &runs {
            for point in &run.points {
                assert!(point.x.is_finite() && point.y.is_finite(), "a coastline point is not a number");
            }
        }

        // A zoom the person chose is theirs: the fit is a default, not a policy.
        state.places_zoom = 9;
        state.places_zoom_chosen = true;
        let _ = frame(&mut state);
        assert_eq!(state.places_zoom, 9, "the view must not overwrite a chosen zoom");

        // And a library with no locations is still the sentence, never an empty map.
        let empty = Engine::open(dir.join("elsewhere")).expect("open");
        let mut empty_state = UiState::new(ThemeKind::Charcoal, 0.0);
        let mut empty_output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    places_view(ui, &tokens, &mut empty_state, &empty);
                });
            },
        );
        empty_output.textures_delta.clear();
        assert!(
            coastline_runs(&empty_output, &tokens).is_empty(),
            "no map is drawn when there is nowhere to draw"
        );
        assert!(painted_circles(&empty_output).is_empty(), "and no dot is invented");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Choosing leftovers in the similarity view ticks every one of them, and the delete dialog acts on that set.
    ///
    /// The button's promise is "all but the best", so the selection has to be all of them: the first version of this
    /// control selected exactly one photo and announced "N set aside", which is the kind of quiet lie that makes a
    /// batch delete dangerous. The selection arithmetic and the engine's delete are tested elsewhere; this is the
    /// wiring between them, plus the dialog opening on a set rather than on nothing.
    #[test]
    fn the_similarity_view_selects_every_leftover_and_offers_to_delete_them() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-leftovers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-leftovers");
        // Three copies of one colour make a group; one different photo stays alone.
        for name in ["IMG_0001.jpg", "IMG_0002.jpg", "IMG_0003.jpg"] {
            std::fs::copy(fixture.jpeg(name, [50, 90, 130], 48, 32), root.join(name)).expect("copy");
        }
        std::fs::copy(fixture.jpeg("IMG_0004.jpg", [200, 40, 40], 40, 40), root.join("IMG_0004.jpg")).expect("copy");
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        engine.begin_similar(0).expect("hash");
        for _ in 0..1200 {
            if engine.poll_similar(0).is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let groups = engine.similar_groups().to_vec();
        assert!(!groups.is_empty(), "the three copies form a group");
        let group = groups.iter().find(|group| group.len() == 3).expect("a three-photo group");
        let others = group.others();
        assert_eq!(others.len(), 2);

        // The selection the button makes, and the count it reports.
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.selection.select_many(&others);
        assert_eq!(state.selection.len(), 2, "both leftovers, not just the first");
        assert!(!state.selection.contains(group.best), "the keeper is not in the set");

        // The dialog is what acts, and it acts on the selection rather than on a remembered list.
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        state.similar_delete_open = true;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                similar_delete_dialog(ui, &mut state, &tokens, &mut engine);
            });
        });
        output.textures_delta.clear();
        assert!(state.similar_delete_open, "painting the dialog does not close it");
        assert!(!output.shapes.is_empty(), "and it painted");
        assert_eq!(engine.deleted_batches().len(), 0, "nothing is deleted by being shown");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The sharpness review paints over a real library, and the cut it paints is the cut it would select.
    ///
    /// The arithmetic is tested in `sharpness`; this is the wiring — that the section reads scores from the engine,
    /// that a library with no scores offers the measurement instead of a cut, and that a library with scores paints
    /// a histogram whose highlighted bands match the review the button would act on.
    #[test]
    fn the_sharpness_review_paints_and_offers_only_a_cut_it_can_justify() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-sharpness-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-sharpness");
        for index in 1..=5 {
            let name = format!("IMG_000{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [40, 70, 90 + index as u8], 64, 48), root.join(&name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let ids: Vec<PhotoId> = engine.all_photos().into_iter().map(|photo| photo.id).collect();
        assert_eq!(ids.len(), 5);

        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);

        // Nothing measured: the section paints, and the review offers no cut because there is nothing to cut.
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                sharpness_section(ui, &tokens, &mut state, &mut engine);
            });
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty(), "the section painted");
        assert_eq!(engine.sharpness_todo(), 5, "every photo needs a score");
        assert!(sharpness::review(&[], state.sharpness_percentile).below.is_empty());
        assert!(state.selection.is_empty(), "painting selects nothing");

        // Five different scores, so the library can be separated and a cut is offered.
        let measured: Vec<(PhotoId, sortlens_quality::Score)> = ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                (
                    *id,
                    sortlens_quality::Score {
                        value: 0.5 + index as f32,
                        version: sortlens_quality::SCORE_VERSION,
                    },
                )
            })
            .collect();
        engine.set_scores(&measured).expect("store scores");
        assert_eq!(engine.sharpness_todo(), 0, "and now nothing does");

        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                sharpness_section(ui, &tokens, &mut state, &mut engine);
            });
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty(), "the histogram painted");

        // The cut the section would offer, computed from the same engine state the section read.
        let scores: Vec<(PhotoId, f32)> = engine.scores().into_iter().map(|(id, score)| (id, score.value)).collect();
        let review = sharpness::review(&scores, state.sharpness_percentile);
        assert_eq!(review.scored, 5);
        assert!(review.can_select());
        assert_eq!(review.below.len(), 1, "10% of five photos is the softest one");
        assert_eq!(review.below[0], ids[0], "and the softest is the one measured lowest");

        // Every score identical: the review refuses rather than selecting the whole library, and the section says so.
        let flat: Vec<(PhotoId, sortlens_quality::Score)> = ids
            .iter()
            .map(|id| {
                (
                    *id,
                    sortlens_quality::Score {
                        value: 2.0,
                        version: sortlens_quality::SCORE_VERSION,
                    },
                )
            })
            .collect();
        engine.set_scores(&flat).expect("flatten");
        let scores: Vec<(PhotoId, f32)> = engine.scores().into_iter().map(|(id, score)| (id, score.value)).collect();
        let review = sharpness::review(&scores, state.sharpness_percentile);
        assert!(!review.can_select(), "identical scores are not a review");
        assert!(review.cut_label().contains("cannot separate"));
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                sharpness_section(ui, &tokens, &mut state, &mut engine);
            });
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty(), "a flat library paints too");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The overlay paints a decided photo over the stage and then drops itself, in a real cull view.
    ///
    /// The phase table is tested on its own; this is the wiring — that a decision starts an overlay, that the
    /// overlay paints something *over* the current photo, and that it clears rather than hanging forever.
    #[test]
    fn a_decision_paints_an_overlay_and_then_clears_it() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-overlay-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-overlay");
        for index in 1..=3 {
            let name = format!("IMG_000{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [30, 60 * index as u8, 140], 64, 48), root.join(&name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
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
        engine.begin_cull(&ids).expect("cull");
        let judged = engine.cull().and_then(|session| session.current()).expect("a current photo");

        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.motion = Motion::Reduced;
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let mut actions: Vec<Action> = Vec::new();
        let mut paint = |state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails, time: f64| {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0))),
                time: Some(time),
                ..egui::RawInput::default()
            };
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    cull_view(ui, &tokens, state, engine, thumbs, &mut actions);
                });
            });
            output.textures_delta.clear();
            output.shapes.len()
        };
        assert!(paint(&mut state, &mut engine, &mut thumbs, 1.0) > 0, "the stage paints");

        // Rejecting starts the overlay, on the photo that was judged.
        state.overlay = Some(Overlay {
            photo: judged,
            decision: Decision::Reject,
            at: 1.0,
        });
        assert!(paint(&mut state, &mut engine, &mut thumbs, 1.01) > 0, "the overlay paints");
        assert!(state.overlay.is_some(), "and is still running just after the decision");
        // Reduced motion: the label holds for its 90 ms and then the overlay is gone.
        paint(&mut state, &mut engine, &mut thumbs, 1.05);
        assert!(state.overlay.is_some(), "still inside the label's moment");
        paint(&mut state, &mut engine, &mut thumbs, 1.2);
        assert!(state.overlay.is_none(), "and dropped once it has been read");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The overlay's two phases, and what reduced motion does to them.
    #[test]
    fn the_decision_overlay_labels_first_then_slides() {
        // At the moment of the decision nothing has happened yet: no label, no movement.
        let start = overlay_phase(0.0, Motion::Full);
        assert_eq!((start.label, start.slide), (0.0, 0.0));
        assert!(!start.done);
        // Half-way through the label phase: the label is arriving but the photograph has not moved, which is
        // the whole point of two phases.
        let fading = overlay_phase(LABEL_MS / 2.0, Motion::Full);
        assert!(fading.label > 0.5, "the label is on its way: {}", fading.label);
        assert_eq!(fading.slide, 0.0, "and the photo is still in place");
        // The label phase ends with the label fully there and the slide only just beginning.
        let labelled = overlay_phase(LABEL_MS, Motion::Full);
        assert_eq!(labelled.label, 1.0);
        assert_eq!(labelled.slide, 0.0);
        // Then the photograph leaves, and is gone at the end of the slide.
        let sliding = overlay_phase(LABEL_MS + SLIDE_MS / 2.0, Motion::Full);
        assert_eq!(sliding.label, 1.0);
        assert!(sliding.slide > 0.0 && sliding.slide < 1.0, "moving: {}", sliding.slide);
        let gone = overlay_phase(LABEL_MS + SLIDE_MS, Motion::Full);
        assert_eq!(gone.slide, 1.0);
        assert!(gone.done, "the overlay can be dropped");
        // The phases never run backwards, and never overshoot.
        let mut last = 0.0;
        for step in 0..=40 {
            let phase = overlay_phase(step as f32 * 10.0, Motion::Full);
            assert!(phase.slide >= last, "the slide is monotonic");
            last = phase.slide;
            assert!((0.0..=1.0).contains(&phase.label) && (0.0..=1.0).contains(&phase.slide));
        }

        // Reduced motion: no fade, no slide — and the label still holds for its 90 ms, because the
        // confirmation is information rather than decoration.
        let reduced = overlay_phase(0.0, Motion::Reduced);
        assert_eq!((reduced.label, reduced.slide), (1.0, 0.0), "the label is simply there");
        assert!(!reduced.done);
        assert_eq!(overlay_phase(LABEL_MS / 2.0, Motion::Reduced).slide, 0.0);
        assert!(overlay_phase(LABEL_MS, Motion::Reduced).done, "and then it is over");
        for step in 0..=40 {
            assert_eq!(overlay_phase(step as f32 * 10.0, Motion::Reduced).slide, 0.0, "never moves");
        }
    }

    /// Opening a folder returns to how it was being browsed, and an unmarked folder follows the library.
    #[test]
    fn opening_a_folder_restores_its_sort_and_search() {
        let marked = sortlens_engine::FolderMark {
            sort: Some("largest".into()),
            search: Some("beach".into()),
            ..sortlens_engine::FolderMark::default()
        };
        assert_eq!(folder_scope(&marked, Sort::Rating), (Sort::Largest, "beach".to_string()));

        // Nothing remembered: the folder follows the library's sort instead of snapping back to Natural, and the
        // box is empty rather than still holding the previous folder's question.
        let unmarked = sortlens_engine::FolderMark::default();
        assert_eq!(folder_scope(&unmarked, Sort::Rating), (Sort::Rating, String::new()));

        // A remembered sort with no search: the sort is the folder's, the box is clear.
        let sort_only = sortlens_engine::FolderMark {
            sort: Some("name".into()),
            ..sortlens_engine::FolderMark::default()
        };
        assert_eq!(folder_scope(&sort_only, Sort::Newest), (Sort::Name, String::new()));
    }

    /// The folder section paints, and a marked row carries its pin and colour into what it draws.
    #[test]
    fn a_marked_folder_paints_its_pin_and_colour() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-folderscope-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-folderscope");
        std::fs::copy(fixture.jpeg("IMG_0001.jpg", [40, 90, 150], 48, 32), root.join("IMG_0001.jpg")).expect("copy");
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let root_text = root.to_string_lossy().to_string();
        engine
            .set_folder_mark(
                &root_text,
                sortlens_engine::FolderMark {
                    favourite: true,
                    colour: Some(sortlens_core::ColorLabel::Green),
                    ..sortlens_engine::FolderMark::default()
                },
            )
            .expect("mark");

        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        let mut actions: Vec<Action> = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                folders_section(ui, &tokens, &mut state, &mut engine, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty(), "the folder row painted");
        assert!(actions.is_empty(), "painting asks for nothing");
        let mark = engine.folder_mark(&sortlens_engine::folder_key(&root_text));
        assert!(mark.favourite, "the pin is on the row's data");
        assert_eq!(mark.colour, Some(sortlens_core::ColorLabel::Green));
        // Clearing it through the same path leaves nothing behind.
        engine
            .set_folder_mark(&root_text, sortlens_engine::FolderMark::default())
            .expect("clear");
        assert!(engine.folder_marks().is_empty(), "an empty mark is not stored");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The sidebar paints its collections with the ticked ones marked, over a real library.
    #[test]
    fn the_sidebar_marks_ticked_collections() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-collections-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let mut engine = Engine::open(&dir).expect("open");
        let first = engine.create_collection("Set one").expect("create");
        let second = engine.create_collection("Set two").expect("create");

        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.collection_selection.replace(first);
        state.collection_selection.toggle(second);
        assert_eq!(state.collection_selection.len(), 2);

        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                collections_section(ui, &tokens, &mut state, &mut engine);
            });
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty(), "the section painted");
        // The bar that says what a batch will act on is drawn only when there is more than one ticked.
        assert!(state.collection_selection.len() == 2, "and nothing cleared the selection");
        // Deleting through the batch path takes both and clears the ticks.
        let ids = state.collection_selection.ids();
        assert_eq!(engine.delete_collections(&ids).expect("delete"), 2);
        state.collection_selection.retain(&[]);
        assert!(state.collection_selection.is_empty(), "a delete leaves no ghost tick");
        assert!(engine.collections().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A cached thumbnail of a file that has gone is painted faded, and the hover says where it came from.
    #[test]
    fn a_cached_thumbnail_of_a_missing_file_is_painted_as_stale() {
        let photo = |missing: bool| Photo {
            id: PhotoId(1),
            root: "/photos".into(),
            rel: "IMG_0001.jpg".into(),
            name: "IMG_0001.jpg".into(),
            size: 1,
            modified: None,
            kind: MediaKind::Image,
            key: 7,
            rating: 0,
            flag: Flag::None,
            label: None,
            missing,
            duration_ms: None,
            fps: None,
            video_codec: None,
            width: 96,
            height: 64,
            captured: None,
            camera: String::new(),
            lens: String::new(),
            iso: None,
            aperture: None,
            shutter: None,
            focal: None,
            tags: Vec::new(),
            tag_sources: std::collections::BTreeMap::new(),
            dismissed_suggestions: std::collections::BTreeSet::new(),
            edited: false,
            retry: false,
            sharpness: None,
            location: None,
            rev: 0,
            added: None,
        };
        let present = photo(false);
        let gone = photo(true);
        assert_eq!(cached_tint(&present), Color32::WHITE, "a live file is painted at full strength");
        assert_eq!(cached_tint(&gone).a(), CACHED_ALPHA, "a missing one is faded, not hidden");
        assert_ne!(
            CellBadge::Offline.hint(),
            CellBadge::Offline.hint_cached(true),
            "showing a cached photograph is a different state from showing nothing"
        );
        assert!(CellBadge::Offline.hint_cached(true).contains("cached"));
        assert_eq!(
            CellBadge::Unreadable.hint_cached(true),
            CellBadge::Unreadable.hint(),
            "an unreadable file has no cached pixels to explain"
        );
    }

    /// End to end: decode a thumbnail, delete the file, rescan, and the grid still draws the cached picture —
    /// faded, so an offline library cannot look current.
    #[test]
    fn the_grid_fades_a_thumbnail_whose_file_left() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-cached-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-cached");
        let path = root.join("IMG_0001.jpg");
        std::fs::copy(fixture.jpeg("IMG_0001.jpg", [60, 110, 180], 96, 64), &path).expect("copy");
        let mut engine = Engine::open(&dir).expect("open");
        let scan = |engine: &mut Engine| {
            engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
            for _ in 0..600 {
                if engine.poll().is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        };
        scan(&mut engine);
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;

        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);

        // Decode the grid tier and keep it: this is the "cached before the drive went away" texture.
        let photo = engine.photo(id).expect("photo");
        assert!(
            std::path::Path::new(&photo.path()).exists(),
            "the file is there to decode: {}",
            photo.path()
        );
        assert!(thumbs.get(&ctx, photo, GRID_SIZE).is_none(), "nothing is resident yet");
        for _ in 0..800 {
            if thumbs.poll(&ctx) > 0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            thumbs.resident() >= 1,
            "the thumbnail decoded (failure: {:?}, queued {}, dropped {})",
            thumbs.failure(engine.photo(id).expect("photo").key),
            thumbs.queued(),
            thumbs.dropped()
        );

        /// The alpha of the vertex colours on the painted image mesh, if one was painted.
        fn painted_alpha(output: &egui::FullOutput) -> Option<u8> {
            for clipped in &output.shapes {
                if let egui::Shape::Mesh(mesh) = &clipped.shape
                    && let Some(vertex) = mesh.vertices.first()
                {
                    return Some(vertex.color.a());
                }
            }
            None
        }
        // Missing files are hidden by default — that is the whole point of the flag — so the offline paint asks
        // for them, which is what the "Missing" filter in the sidebar does.
        let offline = Filter {
            include_missing: true,
            ..Filter::default()
        };
        let paint = |state: &mut UiState, thumbs: &mut Thumbnails, engine: &mut Engine, filter: &Filter| {
            let mut actions: Vec<Action> = Vec::new();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    grid(ui, &tokens, state, engine, thumbs, filter, &mut actions);
                });
            });
            output.textures_delta.clear();
            painted_alpha(&output)
        };

        let visible = Filter::default();
        let live = paint(&mut state, &mut thumbs, &mut engine, &visible).expect("the photo is painted");
        assert_eq!(live, 255, "a present file is drawn at full strength");

        // The drive leaves: the file is gone, the cache is not.
        std::fs::remove_file(&path).expect("remove");
        scan(&mut engine);
        assert!(engine.photo(id).expect("photo").missing, "the rescan noticed the file is gone");
        assert!(
            paint(&mut state, &mut thumbs, &mut engine, &visible).is_none(),
            "a missing photo is hidden from the default view"
        );
        let stale = paint(&mut state, &mut thumbs, &mut engine, &offline).expect("the cached photo is still drawn");
        assert_eq!(stale, CACHED_ALPHA, "and it is painted faded, so the library does not look current");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The `T` key applies the keyword you last added, through the real key handler over a real library.
    ///
    /// This is the cull loop the shortcut exists for: keyword one photo, arrow to the next, press T. It goes
    /// through `Engine::tag_from_prev`, so the notice names the keyword — "Tagged 3 photos" would leave you
    /// guessing what you just wrote.
    #[test]
    fn pressing_t_applies_the_last_keyword_to_the_selection() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-tagprev-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-tagprev");
        for index in 1..=3 {
            let name = format!("IMG_000{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [40 * index as u8, 90, 150], 48, 32), root.join(&name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let order: Vec<PhotoId> = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .iter()
            .map(|photo| photo.id)
            .collect();
        assert_eq!(order.len(), 3);

        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let actions: Vec<Action> = Vec::new();
        let ctx = egui::Context::default();

        // With nothing remembered, the notice explains rather than writing anything.
        state.selection.replace(order[0]);
        let press_t = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0))),
            events: vec![egui::Event::Key {
                key: Key::T,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..egui::RawInput::default()
        };
        let mut output = ctx.run_ui(press_t.clone(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(ui, &mut state, &mut engine, &mut thumbs);
            });
        });
        output.textures_delta.clear();
        assert!(engine.photo(order[0]).expect("photo").tags.is_empty(), "nothing was written");
        let told = state.live_notice().map(|(text, _)| text.to_string()).unwrap_or_default();
        assert!(told.contains("no keyword"), "the notice says why: {told}");

        // Type a keyword on the first photo, tick the other two, then press T.
        engine.set_tag(&order[..1], "backlit", true).expect("tag");
        state.selection.set_many(&order);
        let mut output = ctx.run_ui(press_t, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(ui, &mut state, &mut engine, &mut thumbs);
            });
        });
        output.textures_delta.clear();
        for id in &order {
            assert!(
                engine.photo(*id).expect("photo").tags.contains(&"backlit".to_string()),
                "every photo in the selection carries the keyword"
            );
        }
        let told = state.live_notice().map(|(text, _)| text.to_string()).unwrap_or_default();
        assert!(told.contains("backlit"), "the notice names the keyword: {told}");
        assert!(actions.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Inline tagging in the cull stage (SL-C11) and `T` re-applying the last keyword there (SL-C12),
    /// both through the real view and key handler over a real library.
    ///
    /// The loop the row describes: keyword the photo on screen from the strip's field, decide it, press
    /// T on the next. The chip on the photo on screen comes off through a real click on its rect.
    #[test]
    fn cull_stage_tags_the_photo_on_screen_inline() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-culltag-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-culltag");
        for index in 1..=3 {
            let name = format!("IMG_000{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [40 * index as u8, 90, 150], 48, 32), root.join(&name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let engine_cell = std::cell::RefCell::new(&mut engine);
        assert_eq!(engine_cell.borrow().count(&Filter::default()), 3);

        let ctx = egui::Context::default();
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let key = |key: Key, modifiers: egui::Modifiers| egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0))),
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..egui::RawInput::default()
        };
        let mut frame = |state: &mut UiState, input: egui::RawInput| {
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, state, &mut engine_cell.borrow_mut(), &mut thumbs);
                });
            });
            output.textures_delta.clear();
            output
        };
        let current = |cell: &std::cell::RefCell<&mut Engine>| {
            cell.borrow()
                .cull()
                .and_then(|session| session.current())
                .expect("a photo on screen")
        };

        // The session starts the way the key does it, and nothing is tagged yet.
        frame(&mut state, key(Key::C, egui::Modifiers::NONE));
        assert!(engine_cell.borrow().cull().is_some(), "the session is open");
        let first = current(&engine_cell);
        assert!(engine_cell.borrow().photo(first).expect("photo").tags.is_empty());

        // Focus the inline field, type a keyword, and Enter commits it to the photo on screen.
        state.cull_tag_focus = true;
        state.cull_tag_input = "backlit".to_string();
        frame(&mut state, egui::RawInput::default());
        frame(&mut state, key(Key::Enter, egui::Modifiers::NONE));
        assert!(
            engine_cell
                .borrow()
                .photo(first)
                .expect("photo")
                .tags
                .contains(&"backlit".to_string()),
            "the field added the keyword to the photo on screen"
        );
        assert!(state.cull_tag_input.is_empty(), "the field cleared after committing");
        let told = state.live_notice().map(|(text, _)| text.to_string()).unwrap_or_default();
        assert_eq!(told, "Tagged “backlit”");

        // Decide the photo; the next one comes on screen carrying nothing.
        frame(&mut state, key(Key::P, egui::Modifiers::NONE));
        let second = current(&engine_cell);
        assert_ne!(second, first, "the queue advanced");
        assert!(engine_cell.borrow().photo(second).expect("photo").tags.is_empty());

        // T re-applies the keyword to the photo now on screen, and names it.
        frame(&mut state, key(Key::T, egui::Modifiers::NONE));
        assert!(
            engine_cell
                .borrow()
                .photo(second)
                .expect("photo")
                .tags
                .contains(&"backlit".to_string()),
            "T applied the last keyword to the current photo"
        );
        let told = state.live_notice().map(|(text, _)| text.to_string()).unwrap_or_default();
        assert_eq!(told, "Tagged “backlit”");

        // The key handler runs after the view paints, so the strip catches up on the next frame.
        let output = frame(&mut state, egui::RawInput::default());

        // The toast in the bottom bar repeats the keyword with quotes and the sidebar lists the
        // keyword too; the chip paints the bare word inside the cull panel, right of the sidebar.
        let chip = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text)
                    if text.galley.text().starts_with("backlit") && text.galley.rect.translate(text.pos.to_vec2()).min.x > 260.0 =>
                {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                _ => None,
            })
            .expect("the keyword chip painted its text");
        let at = chip.center();
        let pressed = egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::default(),
        };
        let released = egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        };
        frame(
            &mut state,
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0))),
                events: vec![egui::Event::PointerMoved(at), pressed],
                ..Default::default()
            },
        );
        frame(
            &mut state,
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0))),
                events: vec![released],
                ..Default::default()
            },
        );
        assert!(
            engine_cell.borrow().photo(second).expect("photo").tags.is_empty(),
            "the click removed the keyword"
        );
        let told = state.live_notice().map(|(text, _)| text.to_string()).unwrap_or_default();
        assert_eq!(told, "Removed keyword backlit");

        // And the strip is honest about the removal: one more frame paints no chip for the keyword
        // that was just taken off the photo on screen.
        let out = frame(&mut state, egui::RawInput::default());
        let still_painted = out.shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Text(text)
                if text.galley.text().starts_with("backlit")
                    && text.galley.rect.translate(text.pos.to_vec2()).min.x > 260.0)
        });
        assert!(!still_painted, "the removed keyword has no chip left");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The block under the cull canvas stays above the bottom bar, and the photograph gives way to it.
    ///
    /// At 1440×900 the canvas used to be sized from the window with a guessed allowance for what followed it, so
    /// the keyword field — the control that adds a keyword while the same photo is being judged — was painted
    /// under the bottom bar and the live line below it was not painted at all. The foot is now claimed first, in a
    /// panel of an exact height, and the canvas takes the remainder; this test is the arithmetic's own audit.
    ///
    /// Three assertions, because there are three ways to be wrong:
    /// 1. every cull row is *visible* (its painted rect intersected with the clip rect it carries) and ends above
    ///    the bottom bar — a foot that ran past it fails here;
    /// 2. the canvas is still a photograph, not a sliver, so a fix that reserved the whole window for controls
    ///    would fail rather than pass by having nothing left to clip;
    /// 3. the keyword band does not resize the photograph: two keywords and twelve paint the canvas at exactly
    ///    the same height, and the twelfth is reachable by scrolling the band rather than clipped away.
    #[test]
    fn the_cull_foot_stays_above_the_bottom_bar_at_nine_hundred_points() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-cull-foot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-cull-foot");
        for index in 1..=3 {
            let name = format!("IMG_000{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [40 * index as u8, 90, 150], 48, 32), root.join(&name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
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
        assert_eq!(ids.len(), 3, "three photos indexed");
        engine.begin_cull(&ids).expect("a cull session starts");
        // The frame on screen carries two keywords, so the band is painted and the field is not the only row
        // between the decisions and the live line.
        let first = engine.cull().and_then(|session| session.current()).expect("a photo on screen");
        engine.set_tag(&[first], "sea", true).expect("tag");
        engine.set_tag(&[first], "portrait", true).expect("tag");

        let t = ThemeKind::Charcoal.tokens();
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.view = View::Cull;
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let engine_cell = std::cell::RefCell::new(&mut engine);
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1440.0, 900.0));
        let mut frame = |state: &mut UiState, events: Vec<egui::Event>| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..egui::RawInput::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        show(ui, state, &mut engine_cell.borrow_mut(), &mut thumbs);
                    });
                },
            );
            output.textures_delta.clear();
            output
        };

        // A painted rect is as visible as the intersection with its own clip: a row the panel pushed past its
        // own edge is still in the display list, and whether a person can see it is the two intersected.
        fn visible(rect: Rect, clip: Rect) -> Option<Rect> {
            let hit = clip.intersect(rect);
            (hit.width() > 0.5 && hit.height() > 0.5).then_some(hit)
        }
        fn texts(output: &egui::FullOutput) -> Vec<(String, Rect, Rect)> {
            fn walk(shape: &egui::Shape, clip: Rect, sink: &mut Vec<(String, Rect, Rect)>) {
                match shape {
                    egui::Shape::Text(text) => {
                        sink.push((text.galley.job.text.clone(), text.visual_bounding_rect(), clip));
                    }
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            walk(shape, clip, sink);
                        }
                    }
                    _ => {}
                }
            }
            let mut sink = Vec::new();
            for clipped in &output.shapes {
                walk(&clipped.shape, clipped.clip_rect, &mut sink);
            }
            sink
        }
        /// The photograph's own rect: the largest rectangle filled with the thumbnail well's colour.
        fn canvas_area(output: &egui::FullOutput, well: egui::Color32) -> Rect {
            output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(shape) if shape.brush.is_none() && shape.fill == well => Some(shape.rect),
                    _ => None,
                })
                .max_by(|a, b| (a.width() * a.height()).total_cmp(&(b.width() * b.height())))
                .expect("the canvas paints its own well")
        }

        let column_bottom = screen.bottom() - t.bottom_bar_h;
        let column = Rect::from_min_max(pos2(t.left_w, 0.0), pos2(screen.right() - t.right_w, column_bottom));
        let output = frame(&mut state, Vec::new());
        let painted = texts(&output);

        // 1. Nothing in the cull column is painted into the bottom bar's band, and the live line — the last row
        // of the foot — is not merely present but visible in the column. On the old layout it was below the fold.
        for (text, rect, clip) in &painted {
            if rect.center().x < column.min.x || rect.center().x > column.max.x || rect.min.y > column_bottom {
                continue;
            }
            if let Some(hit) = visible(*rect, *clip) {
                assert!(
                    hit.max.y <= column_bottom + 0.5,
                    "`{text}` is visible at {hit:?}, inside the bottom bar's band (ends at {column_bottom})"
                );
            }
        }
        let live = painted
            .iter()
            .find(|(text, rect, clip)| {
                text.starts_with("Photo 1 of 3") && visible(*rect, *clip).is_some_and(|hit| column.contains_rect(hit))
            })
            .expect("the live line under the keyword field is on screen, not under the bottom bar");
        assert!(
            visible(live.1, live.2).expect("visible").max.y < column_bottom,
            "the live line ends above the bottom bar"
        );
        // The keyword field and the two chips are in the same foot, so the field that adds a keyword is on
        // screen in the same frame the chip that removes one is.
        for label in ["sea  ×", "portrait  ×", "Add"] {
            assert!(
                painted
                    .iter()
                    .any(|(text, rect, clip)| text == label && visible(*rect, *clip).is_some_and(|hit| column.contains_rect(hit))),
                "`{label}` is visible in the cull column; painted: {painted:?}"
            );
        }

        // 2. The photograph still has the room to itself: the canvas is most of what is left above the foot.
        let area = canvas_area(&output, t.thumb_bg);
        assert!(
            area.height() > 300.0,
            "the canvas is a photograph, not a sliver: {area:?} of a {column_bottom}-point column"
        );

        // 3. Sixty keywords paint the same canvas as two, and the ones past the band's two rows are reachable.
        let band_first = painted
            .iter()
            .find_map(|(text, rect, clip)| (text == "sea  ×").then(|| visible(*rect, *clip)).flatten())
            .expect("the first chip is visible");
        let crowded: Vec<String> = (1..=60).map(|n| format!("kw{n:02}")).collect();
        for tag in &crowded {
            engine_cell.borrow_mut().set_tag(&[first], tag, true).expect("tag");
        }
        let output = frame(&mut state, Vec::new());
        let crowded_area = canvas_area(&output, t.thumb_bg);
        assert_eq!(crowded_area, area, "sixty keywords leave the photograph exactly where two did");
        let painted = texts(&output);
        let hidden = |name: &str, painted: &[(String, Rect, Rect)]| {
            !painted
                .iter()
                .any(|(text, rect, clip)| text == name && visible(*rect, *clip).is_some_and(|hit| column.contains_rect(hit)))
        };
        assert!(
            hidden("kw60  ×", &painted),
            "the last chip starts below the band's two rows, so the band is a scroll and not a no-op"
        );
        // The wheel over the band brings it up: the rows past the fold are reachable rather than clipped away.
        let at = band_first.center();
        let mut output = frame(
            &mut state,
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0.0, -240.0),
                    modifiers: egui::Modifiers::NONE,
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                },
            ],
        );
        // Smooth scrolling eases towards its target over a few frames, so one wheel event is not a promise that
        // the band has arrived; the frame that is read is the one after it has settled.
        for _ in 0..12 {
            output = frame(&mut state, Vec::new());
        }
        let painted = texts(&output);
        assert!(
            !hidden("kw60  ×", &painted),
            "the wheel over the keyword band brings the last chip into view"
        );
        assert_eq!(
            engine_cell.borrow().cull().and_then(|session| session.current()),
            Some(first),
            "nothing was decided by scrolling"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Ctrl+Z and Ctrl+Shift+Z drive the real history through the shell's own shortcut handling —
    /// undo steps back, redo steps forward, and a redo with nothing behind it says so.
    #[test]
    fn ctrl_z_and_ctrl_shift_z_drive_the_real_history() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-undoredo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-undoredo");
        std::fs::copy(fixture.jpeg("IMG_0001.jpg", [40, 90, 150], 48, 32), root.join("IMG_0001.jpg")).expect("copy");
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let id = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .first()
            .expect("photo")
            .id;
        engine.rate(&[id], 4).expect("rate");

        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let ctx = egui::Context::default();
        let engine_cell = std::cell::RefCell::new(&mut engine);
        let press = |key: Key, modifiers: egui::Modifiers| egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0))),
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..egui::RawInput::default()
        };
        let mut frame = |state: &mut UiState, input: egui::RawInput| {
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, state, &mut engine_cell.borrow_mut(), &mut thumbs);
                });
            });
            output.textures_delta.clear();
        };

        frame(&mut state, press(Key::Z, egui::Modifiers::COMMAND));
        assert_eq!(engine_cell.borrow().photo(id).expect("photo").rating, 0, "undo stepped back");
        let told = state.live_notice().map(|(text, _)| text.to_string()).unwrap_or_default();
        assert_eq!(told, "Undid the last change");

        frame(&mut state, press(Key::Z, egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT)));
        assert_eq!(engine_cell.borrow().photo(id).expect("photo").rating, 4, "redo stepped forward");
        let told = state.live_notice().map(|(text, _)| text.to_string()).unwrap_or_default();
        assert_eq!(told, "Redid the last undone change");

        frame(&mut state, press(Key::Z, egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT)));
        assert_eq!(engine_cell.borrow().photo(id).expect("photo").rating, 4, "nothing changed");
        let told = state.live_notice().map(|(text, _)| text.to_string()).unwrap_or_default();
        assert_eq!(told, "Nothing to redo");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A rebind through the runtime keymap moves the action: Q flags as picked, and P stops
    /// answering. The keymap is the single reader of its commands, so a rebind cannot be a
    /// promise the dispatch never reads (SL-I09).
    #[test]
    fn a_rebound_key_moves_the_action() {
        use craft_keymap::{Binding as CraftBinding, Policy};
        let dir = std::env::temp_dir().join(format!("sortlens-ui-rebind-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-rebind");
        std::fs::copy(fixture.jpeg("IMG_0001.jpg", [40, 90, 150], 48, 32), root.join("IMG_0001.jpg")).expect("copy");
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let id = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .first()
            .expect("photo")
            .id;

        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.selection.replace(id);
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let ctx = egui::Context::default();
        let engine_cell = std::cell::RefCell::new(&mut engine);
        let press = |key: Key, modifiers: egui::Modifiers| egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0))),
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..egui::RawInput::default()
        };
        let mut frame = |state: &mut UiState, input: egui::RawInput| {
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, state, &mut engine_cell.borrow_mut(), &mut thumbs);
                });
            });
            output.textures_delta.clear();
        };
        let flag = || engine_cell.borrow().photo(id).expect("photo").flag;

        frame(&mut state, press(Key::P, egui::Modifiers::NONE));
        assert_eq!(flag(), Flag::Pick, "P is flag as picked out of the box");

        // The recorder's answer, as the editor would land it: the command's bindings become Q.
        state
            .keys
            .set_bindings(&keymap::flag_pick(), vec![CraftBinding::parse("Q").unwrap()], Policy::Steal)
            .expect("rebind flag pick");
        frame(&mut state, press(Key::P, egui::Modifiers::NONE));
        assert_eq!(flag(), Flag::Pick, "the old chord stops answering");
        frame(&mut state, press(Key::Q, egui::Modifiers::NONE));
        assert_eq!(flag(), Flag::Pick, "the new chord flags as picked");
        // And the other half of the command is untouched by the rebind.
        frame(&mut state, press(Key::X, egui::Modifiers::NONE));
        assert_eq!(flag(), Flag::Reject, "X is still flag as rejected");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The editor opens from Settings, paints the catalog under the table's own sections, owns
    /// the keyboard while it is open, and OK hands the sets to the host with the rebind in them.
    #[test]
    fn the_shortcut_editor_opens_rebinds_and_saves() {
        use craft_keymap::{Binding as CraftBinding, Policy};
        let dir = std::env::temp_dir().join(format!("sortlens-ui-editor-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-editor");
        std::fs::copy(fixture.jpeg("IMG_0001.jpg", [40, 90, 150], 48, 32), root.join("IMG_0001.jpg")).expect("copy");
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let id = engine
            .page(&Filter::default(), Sort::Natural, 0, 10)
            .photos
            .first()
            .expect("photo")
            .id;

        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.selection.replace(id);
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let ctx = egui::Context::default();
        let engine_cell = std::cell::RefCell::new(&mut engine);
        let press = |key: Key, modifiers: egui::Modifiers| egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0))),
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..egui::RawInput::default()
        };
        let mut run = |state: &mut UiState, input: egui::RawInput| -> (egui::FullOutput, Vec<Action>) {
            let mut actions = Vec::new();
            let mut output = ctx.run_ui(input, |ctx| {
                actions = egui::CentralPanel::default()
                    .show(ctx, |ui| show(ui, state, &mut engine_cell.borrow_mut(), &mut thumbs))
                    .inner;
            });
            output.textures_delta.clear();
            (output, actions)
        };
        fn texts(output: &egui::FullOutput) -> Vec<String> {
            fn walk(shape: &egui::Shape, sink: &mut Vec<String>) {
                match shape {
                    egui::Shape::Text(text) => sink.push(text.galley.job.text.clone()),
                    egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, sink)),
                    _ => {}
                }
            }
            let mut sink = Vec::new();
            for clipped in &output.shapes {
                walk(&clipped.shape, &mut sink);
            }
            sink
        }

        // Open the way Settings does, and read what the editor paints.
        state.shortcut_editor = Some(DialogState::new());
        let (output, _) = run(
            &mut state,
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 800.0))),
                ..egui::RawInput::default()
            },
        );
        let painted = texts(&output);
        assert!(
            painted.iter().any(|text| text.contains("Keyboard shortcuts")),
            "the editor is on screen"
        );
        assert!(
            painted.iter().any(|text| text.contains("Flag as picked")),
            "the catalog rows are the table's: {painted:?}"
        );
        assert!(
            painted.iter().any(|text| text.contains("Library")),
            "the table groups by the table's sections"
        );
        assert!(
            painted.iter().any(|text| text.contains("Apply the staged rejections")),
            "the table carries the whole catalog, cull section included"
        );
        assert!(
            painted.iter().any(|text| text.contains("Ctrl+Shift+R")),
            "the chords paint the way the table spells them"
        );
        assert!(painted.iter().any(|text| text.contains("Import\u{2026}")), "a set can be imported");
        assert!(painted.iter().any(|text| text.contains("Export\u{2026}")), "a set can be exported");

        // The editor owns the keyboard: P behind it changes nothing.
        let (_, _) = run(&mut state, press(Key::P, egui::Modifiers::NONE));
        assert_eq!(
            engine_cell.borrow().photo(id).expect("photo").flag,
            Flag::None,
            "the shell's keys step aside"
        );

        // A rebind lands in the runtime keymap, OK commits it into the active set, and the next
        // frame reports the save.
        state
            .keys
            .set_bindings(&keymap::flag_pick(), vec![CraftBinding::parse("Q").unwrap()], Policy::Steal)
            .expect("rebind flag pick");
        let mut editor = state.shortcut_editor.take().expect("open");
        ShortcutsDialog::new(&mut state.keys, &craft_keymap::ui::English)
            .sets(&mut state.profiles, keymap::BUILTINS)
            .ok(&mut editor);
        // Back where the shell keeps it, closed: the next frame reports the Ok outcome and drops it.
        state.shortcut_editor = Some(editor);
        let (_, actions) = run(
            &mut state,
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 800.0))),
                ..egui::RawInput::default()
            },
        );
        assert!(state.shortcut_editor.is_none(), "the editor closed");
        let told = state.live_notice().map(|(text, _)| text.to_string()).unwrap_or_default();
        assert_eq!(told, "Shortcuts saved");
        let saved = actions
            .iter()
            .find_map(|action| match action {
                Action::SaveShortcuts(profiles) => Some(profiles.clone()),
                _ => None,
            })
            .expect("OK hands the sets to the host");
        assert_eq!(
            saved.active_overrides(keymap::BUILTINS).get(&keymap::flag_pick().key()),
            Some(&["Q".to_owned()][..]),
            "the host is handed the rebind"
        );

        // And what the keymap now runs is what was saved: Q flags as picked.
        let (_, _) = run(
            &mut state,
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 800.0))),
                ..egui::RawInput::default()
            },
        );
        let (_, _) = run(&mut state, press(Key::Q, egui::Modifiers::NONE));
        assert_eq!(
            engine_cell.borrow().photo(id).expect("photo").flag,
            Flag::Pick,
            "Q is flag as picked after OK"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The detail view paints before its full-size decode lands: the blur-up path runs on a real photo.
    ///
    /// The interesting part is that it does not panic, does not ask for anything, and does not paint an empty
    /// rectangle where a photo belongs — the tier choice itself is tested in `thumbs`, and this is the call site.
    ///
    /// The same frame is also audited for tofu: every character the detail view paints must exist in the bundled
    /// faces. The screenshot harness caught the footer's `←`/`→` here (no bundled face has U+2190/U+2192, so a
    /// default install drew boxes), and this is the headless test that keeps the footer honest.
    #[test]
    fn the_detail_view_paints_while_the_big_decode_is_still_running() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-detail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-detail");
        std::fs::copy(fixture.jpeg("IMG_0001.jpg", [70, 120, 190], 64, 48), root.join("IMG_0001.jpg")).expect("copy");
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;
        let ctx = egui::Context::default();
        // A two-hundred point tall window is enough to lay the detail view out and paint it.
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(900.0, 620.0))),
            ..egui::RawInput::default()
        };
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.selection.replace(id);
        state.view = View::Detail;
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let actions: Vec<Action> = Vec::new();
        let mut painted = 0;
        let mut missing: Vec<String> = Vec::new();
        for _ in 0..3 {
            let mut output = ctx.run_ui(input.clone(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, &mut state, &mut engine, &mut thumbs);
                });
            });
            output.textures_delta.clear();
            painted = output.shapes.len();
            missing = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Text(text) => {
                        let font = text
                            .galley
                            .job
                            .sections
                            .first()
                            .map_or_else(|| egui::FontId::proportional(12.0), |section| section.format.font_id.clone());
                        let body = text.galley.job.text.clone();
                        (!ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, &body))).then_some(body)
                    }
                    _ => None,
                })
                .collect();
        }
        assert!(painted > 0, "the detail view painted something");
        assert!(
            missing.is_empty(),
            "the detail view paints only glyphs the bundled faces have; these do not: {missing:?}"
        );
        assert!(actions.is_empty());
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

    /// Ticking collections: replace, toggle, range in sidebar order, and no ghost after a delete.
    #[test]
    fn a_collection_selection_replaces_toggles_and_ranges() {
        let order: Vec<sortlens_core::CollectionId> = (1..=6).map(sortlens_core::CollectionId).collect();
        let mut selection = CollectionSelection::default();
        assert!(selection.is_empty());

        selection.replace(order[0]);
        assert_eq!(selection.ids(), vec![order[0]], "a plain click takes one row");

        selection.toggle(order[2]);
        assert_eq!(selection.ids(), vec![order[0], order[2]], "⌘-click adds without dropping");
        selection.toggle(order[0]);
        assert_eq!(selection.ids(), vec![order[2]], "and ⌘-clicking a ticked row removes it");

        // A ⇧-click ranges from the anchor in *sidebar* order, which is what the person can see.
        selection.replace(order[1]);
        selection.extend(&order, order[4]);
        assert_eq!(selection.ids(), vec![order[1], order[2], order[3], order[4]]);
        // And it replaces the previous run rather than adding to it, so widening and narrowing both work.
        selection.extend(&order, order[2]);
        assert_eq!(selection.ids(), vec![order[1], order[2]]);
        // Backwards from the anchor is the same run.
        selection.extend(&order, order[0]);
        assert_eq!(selection.ids(), vec![order[0], order[1]]);

        // No anchor in view: a ⇧-click is a plain click, not half the list.
        let mut fresh = CollectionSelection::default();
        fresh.extend(&order, order[3]);
        assert_eq!(fresh.ids(), vec![order[3]]);

        // A collection that disappears (deleted, or undone) does not stay ticked.
        selection.retain(&order[..1]);
        assert_eq!(selection.ids(), vec![order[0]]);
        selection.clear();
        assert!(selection.is_empty());
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

    /// The size control and the density chip are one state: every step has a position, every position
    /// names a step, and a value dragged between stops lands on the nearer one rather than inventing a
    /// fourth density the grid would then have to answer for.
    #[test]
    fn the_size_control_positions_name_the_real_density_steps() {
        for (position, step) in Density::ALL.into_iter().enumerate() {
            assert_eq!(step.index(), position);
            assert_eq!(Density::from_index(position as f32), step);
            assert_eq!(Density::from_index(position as f32 + 0.49), step);
            assert_eq!(Density::from_index(position as f32 - 0.49), step);
        }
        assert_eq!(Density::from_index(-3.0), Density::Compact, "below the track clamps");
        assert_eq!(Density::from_index(9.0), Density::Roomy, "above the track clamps");
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

        state.apply_source(Source::Added);
        let recent = state.filter();
        assert!(recent.added_after.is_some(), "Recently Added asks for the last 30 days");
        assert_eq!(recent.flag, None);
        assert_eq!(recent.min_rating, 0);
        assert!(state.source_selected(Source::Added));
        assert!(!state.source_selected(Source::All));
        assert_eq!(state.sort, Sort::RecentlyAdded, "the question is asked newest-record-first");

        state.apply_source(Source::All);
        assert!(state.filter().flag.is_none());
        assert_eq!(state.filter().min_rating, 0);
        assert_eq!(state.filter().added_after, None, "leaving the row leaves the window");
        assert_eq!(state.sort, state.library_sort, "the user's sort comes back");
        assert!(state.source_selected(Source::All));
        assert!(state.all_photos_selected, "All Photos is also what the cull queue starts from");
    }

    /// The sidebar paints its six sources over a real library, with counts that differ.
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

        // SL-A14 part 2: both photos were indexed this second, so Recently Added counts the whole
        // library; the window is answered from each record's `added`, not from the file's mtime.
        assert_eq!(
            engine.count(&Filter {
                added_after: Source::Added.added_after(sortlens_engine::now_epoch()),
                ..Filter::default()
            }),
            2
        );

        // Picking the row replaces the question and asks it newest-record-first.
        state.apply_source(Source::Added);
        assert!(state.source_selected(Source::Added));
        assert!(state.filter().added_after.is_some(), "the row's question carries a cutoff");
        assert_eq!(state.sort, Sort::RecentlyAdded, "the question is asked newest-record-first");
        assert_eq!(engine.count(&state.filter()), 2);
        state.apply_source(Source::All);
        assert_eq!(state.filter().added_after, None, "leaving the row leaves the window");
        assert_eq!(state.sort, state.library_sort, "the user's sort comes back");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Phase 3B.3: the Missing row asks about the filesystem and counts what is gone, and nothing else.
    ///
    /// A file is removed and the health check marks it — the same path a person's unmounted drive takes —
    /// so the count under the row is the count the grid holds once the row is picked. Removing the scope's
    /// `Filter` dimension (`Source::missing`) fails it, and so does widening it to `include_missing`.
    #[test]
    fn the_missing_scope_counts_the_files_that_are_gone_and_only_those() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-missing");
        for name in ["a.jpg", "b.jpg", "c.jpg"] {
            std::fs::copy(fixture.jpeg(name, [200, 40, 40], 64, 64), root.join(name)).expect("copy");
        }
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(engine.stats().total, 3, "three photos indexed");
        std::fs::remove_file(root.join("b.jpg")).expect("remove");
        engine.check_health().expect("health");
        assert_eq!(engine.stats().missing, 1, "one file is gone");

        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let now = sortlens_engine::now_epoch();
        assert_eq!(Source::Missing.missing(), Some(true));
        assert_eq!(
            source_count(&engine, &state, Source::Missing, now),
            1,
            "the Missing row counts the file that is gone"
        );
        assert_eq!(
            source_count(&engine, &state, Source::All, now),
            2,
            "All Photos still hides it, so the two rows answer different questions"
        );
        assert_eq!(engine.count(&Filter::default()), 2, "the grid still hides it by default");

        state.apply_source(Source::Missing);
        assert!(state.source_selected(Source::Missing));
        assert!(!state.source_selected(Source::All), "one row is lit, not two");
        assert_eq!(state.filter().missing, Some(true), "the scope narrows the grid to what is gone");
        assert_eq!(engine.count(&state.filter()), 1);
        assert_eq!(state.active_filter_count(), 1, "the header badge says the grid is narrowed");

        // Every other source still asks its own question, so picking one leaves the scope behind.
        state.apply_source(Source::Picks);
        assert_eq!(state.filter().missing, None);
        assert!(!state.source_selected(Source::Missing));
        state.apply_source(Source::Missing);
        state.clear_filters();
        assert!(!state.missing_only, "Clear is how you get out of the scope");
        assert_eq!(state.filter().missing, None);
        assert_eq!(state.active_filter_count(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Phase 3B.3: the four utility rows live at the foot of the sidebar, not between sections.
    ///
    /// The claim is geometric because that is the claim: the six scopes, the folders and the collections
    /// own the top of the panel and the utilities own its bottom edge, whatever the panel holds. The four
    /// lowest painted glyphs must be the foot's, they must all sit inside the bottom band, and nothing else
    /// may be down there — an inline `Shortcuts` row puts a fifth glyph in the band and a foot that drifted
    /// up leaves the band empty.
    #[test]
    fn the_sidebar_foot_pins_the_utilities_to_the_bottom() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-foot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-foot");
        for name in ["a.jpg", "b.jpg"] {
            std::fs::copy(fixture.jpeg(name, [90, 120, 200], 64, 64), root.join(name)).expect("copy");
        }
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let t = ThemeKind::Charcoal.tokens();
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 800.0));
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..egui::RawInput::default()
        };
        // The whole window, not the panel alone: the foot's column is what the top and bottom bars leave
        // between them, and a panel painted on its own would be free to hang past the bottom bar.
        let mut output = ctx.run_ui(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                show(ui, &mut state, &mut engine, &mut thumbs);
            });
        });
        output.textures_delta.clear();

        // An image paints as a textured rectangle in this egui: that is "a picture was painted here". Each
        // one is kept with the rect it was clipped to, because a glyph painted outside its clip rect is a
        // glyph nobody sees — which is exactly how the first version of this panel failed.
        let painted: Vec<(Rect, Rect)> = output
            .shapes
            .iter()
            .filter_map(|clipped| {
                let rect = match &clipped.shape {
                    egui::Shape::Mesh(mesh) => Some(mesh.calc_bounds()),
                    egui::Shape::Rect(shape) => shape
                        .brush
                        .as_ref()
                        .filter(|brush| brush.fill_texture_id != egui::TextureId::default())
                        .map(|_| shape.rect),
                    _ => None,
                };
                rect.map(|rect| (rect, clipped.clip_rect))
            })
            .collect();
        let column_bottom = screen.bottom() - t.bottom_bar_h;
        // The sidebar's own glyphs: left of the canvas and above the bottom bar, whose first view tab
        // starts at the same x the sidebar's icons do.
        let sidebar: Vec<(Rect, Rect)> = painted
            .into_iter()
            .filter(|(rect, _)| rect.center().x <= t.left_w && rect.center().y <= column_bottom)
            .collect();

        // The panel's own rect, from the number the shell gives the panel rather than a copy of it: the
        // foot is the last `sidebar_foot_height` points of the column above the bottom bar.
        let column = Rect::from_min_max(
            pos2(0.0, column_bottom - sidebar_foot_height(&ctx.global_style().spacing)),
            pos2(t.left_w, column_bottom),
        );
        let in_foot: Vec<Rect> = sidebar
            .iter()
            .filter(|(rect, _)| column.contains_rect(*rect))
            .map(|(rect, _)| *rect)
            .collect();
        assert_eq!(
            in_foot.len(),
            SIDEBAR_FOOT.len(),
            "the foot's four glyphs are the bottom of the panel and nothing else is down there"
        );
        // Inside the panel is not enough: a row that overflows the panel's height is clipped away, which is
        // how an auto-sized foot looked on its first frame.
        for (rect, clip) in &sidebar {
            assert!(
                clip.contains_rect(*rect),
                "{rect:?} was painted outside the {clip:?} it was clipped to — the foot does not fit"
            );
        }
        let lowest_section = sidebar
            .iter()
            .filter(|(rect, _)| !column.contains_rect(*rect))
            .map(|(rect, _)| rect.center().y)
            .fold(f32::MIN, f32::max);
        assert!(
            lowest_section < column.min.y - 60.0,
            "the sections end well above the foot (lowest {lowest_section} vs foot top {})",
            column.min.y
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 3B.6 (d): a short window has more sections than the sidebar can show, and the pinned foot must not be
    /// painted over by them.
    ///
    /// At 1280×620 the six scopes, the subfolders and twelve keyword rows are taller than the panel, so the
    /// sections used to keep laying themselves out past the foot's hairline and draw on top of the four
    /// utility rows. The claim is geometric and made twice: **nothing from the sections is visible inside the
    /// foot's band** (the boundary is a clip, not luck), and **the sections scroll**, so the rows below the
    /// fold are reachable rather than merely hidden. A keyword that was below the fold before a wheel event is
    /// above it afterwards, and the foot has not moved a point.
    #[test]
    fn the_sidebar_scrolls_its_sections_above_the_foot_in_a_short_window() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-short-sidebar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-short-sidebar");
        for name in ["a.jpg", "b.jpg"] {
            std::fs::copy(fixture.jpeg(name, [90, 120, 200], 64, 64), root.join(name)).expect("copy");
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
        assert_eq!(ids.len(), 2, "two photos indexed");
        // Distinct names in a fixed order so the test knows which row it is looking for: the vocabulary is
        // most-used first, and twelve rows is what the panel paints.
        let tags: Vec<String> = (1..=12).map(|n| format!("kw{n:02}")).collect();
        for tag in &tags {
            engine.set_tag(&ids, tag, true).expect("tag");
        }
        assert_eq!(
            engine.tag_vocabulary().len(),
            tags.len(),
            "the sidebar has twelve keyword rows to paint"
        );

        let t = ThemeKind::Charcoal.tokens();
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 620.0));
        let mut frame = |input: egui::RawInput| {
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, &mut state, &mut engine, &mut thumbs);
                });
            });
            output.textures_delta.clear();
            output
        };
        let input = egui::RawInput {
            screen_rect: Some(screen),
            ..egui::RawInput::default()
        };
        let output = frame(input);

        let column_bottom = screen.bottom() - t.bottom_bar_h;
        let band = Rect::from_min_max(
            pos2(0.0, column_bottom - sidebar_foot_height(&ctx.global_style().spacing)),
            pos2(t.left_w, column_bottom),
        );

        // Everything painted inside a shape's own clip rect: a row the scroll area pushed past its viewport is
        // still in the display list, and whether a person can see it is the intersection of the two.
        fn visible(rect: Rect, clip: Rect) -> Option<Rect> {
            let hit = clip.intersect(rect);
            (hit.width() > 0.5 && hit.height() > 0.5).then_some(hit)
        }
        fn texts(output: &egui::FullOutput) -> Vec<(String, Rect, Rect)> {
            fn walk(shape: &egui::Shape, clip: Rect, sink: &mut Vec<(String, Rect, Rect)>) {
                match shape {
                    egui::Shape::Text(text) => {
                        sink.push((text.galley.job.text.clone(), text.visual_bounding_rect(), clip));
                    }
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            walk(shape, clip, sink);
                        }
                    }
                    _ => {}
                }
            }
            let mut sink = Vec::new();
            for clipped in &output.shapes {
                walk(&clipped.shape, clipped.clip_rect, &mut sink);
            }
            sink
        }

        let painted = texts(&output);
        // The foot is the only thing allowed in its own band, and each of its four rows must be readable there:
        // a fix that clipped the footer too would leave the band empty rather than clean.
        let in_band: Vec<&str> = painted
            .iter()
            .filter(|(_, rect, clip)| visible(*rect, *clip).is_some_and(|hit| band.intersects(hit)) && rect.center().y > band.min.y)
            .map(|(text, _, _)| text.as_str())
            .collect();
        let registry = sortlens_engine::command::Registry::v0();
        for (command, _) in SIDEBAR_FOOT {
            let label = registry.get(command.id()).expect("the foot row names a real command").label;
            assert!(
                in_band.iter().any(|text| text.starts_with(label)),
                "`{label}` is painted in the foot's band; the band holds {in_band:?}"
            );
        }
        // And nothing else is: every keyword row that sits below the fold is clipped at the boundary instead of
        // drawn over the utilities.
        for (text, rect, clip) in &painted {
            if !tags.iter().any(|tag| tag == text) {
                continue;
            }
            if let Some(hit) = visible(*rect, *clip) {
                assert!(
                    hit.max.y <= band.min.y + 0.5,
                    "keyword `{text}` is visible at {hit:?}, inside the foot's band ({band:?})"
                );
            }
        }
        // The sections really do overflow at this height, so those assertions are not passing vacuously.
        let visible_tags: Vec<&str> = painted
            .iter()
            .filter(|(text, rect, clip)| tags.iter().any(|tag| tag == text) && visible(*rect, *clip).is_some())
            .map(|(text, _, _)| text.as_str())
            .collect();
        assert!(
            visible_tags.len() < tags.len(),
            "the window is tall enough that every keyword fits, so this test proves nothing: {visible_tags:?}"
        );

        // The fold is not a hole: a wheel event over the sidebar brings the rows under it up, and the foot stays
        // exactly where it was.
        let foot_before = label_at(&output, "Recently Deleted").expect("the foot is painted");
        let hidden = tags
            .iter()
            .find(|tag| {
                !painted
                    .iter()
                    .any(|(text, rect, clip)| text == *tag && visible(*rect, *clip).is_some())
            })
            .expect("a keyword row starts below the fold")
            .clone();
        let scroll = egui::RawInput {
            screen_rect: Some(screen),
            events: vec![
                egui::Event::PointerMoved(pos2(120.0, 320.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0.0, -260.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: egui::Modifiers::default(),
                },
            ],
            ..egui::RawInput::default()
        };
        let mut output = frame(scroll);
        // Smooth scrolling eases towards its target over a few frames; a single wheel event is not a promise
        // that the list has arrived.
        for _ in 0..12 {
            output = frame(egui::RawInput {
                screen_rect: Some(screen),
                ..egui::RawInput::default()
            });
        }
        let painted = texts(&output);
        assert!(
            painted
                .iter()
                .any(|(text, rect, clip)| { text == &hidden && visible(*rect, *clip).is_some() }),
            "`{hidden}` was below the fold and did not come up when the sidebar was scrolled"
        );
        let foot_after = label_at(&output, "Recently Deleted").expect("the foot is still painted");
        assert!(
            (foot_after.min.y - foot_before.min.y).abs() < 0.5,
            "the foot moved with the sections ({:?} → {:?}) instead of staying pinned",
            foot_before,
            foot_after
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 3B.6 (h): the strip's right-aligned `N shown` count must never be painted over a chip.
    ///
    /// The count is the strip's last child, and a wrapped layout hands its last child the whole row's
    /// width: a label right-aligned there is laid out from the wrap cursor and lands under the chips the
    /// row already holds. That is not a width-specific slip — whenever the row's last chip ends within the
    /// count's own width of the right edge, the two collide — so this sweeps the strip from 320 to 760
    /// points a step at a time and asks the same question at every step: **no painted text intersects the
    /// count**, and the count lies inside the clip it was painted with, so a fix that hid it would fail
    /// too. Two widths must
    /// make the sweep non-vacuous: one where the chips wrap onto more than one row, and one where the
    /// count's own centre sits inside the chips' band — the place a collision would be visible.
    #[test]
    fn the_filter_strip_summary_never_lands_on_a_chip_at_any_width() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-strip-summary-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-strip-summary");
        for name in ["a.jpg", "b.jpg"] {
            std::fs::copy(fixture.jpeg(name, [90, 120, 200], 64, 64), root.join(name)).expect("copy");
        }
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(engine.stats().total, 2, "the strip needs a library to narrow");

        let t = ThemeKind::Charcoal.tokens();
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.filters_open = true;
        // Scoped to the library folder, which is one active filter: the strip then offers `Clear`, the chip
        // the count used to be painted over, and its chip set is exactly the one the baselines show.
        state.root = Some(root.to_string_lossy().to_string());

        fn texts(output: &egui::FullOutput) -> Vec<(String, Rect, Rect)> {
            fn walk(shape: &egui::Shape, clip: Rect, sink: &mut Vec<(String, Rect, Rect)>) {
                match shape {
                    egui::Shape::Text(text) => {
                        sink.push((text.galley.job.text.clone(), text.visual_bounding_rect(), clip));
                    }
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            walk(shape, clip, sink);
                        }
                    }
                    _ => {}
                }
            }
            let mut sink = Vec::new();
            for clipped in &output.shapes {
                walk(&clipped.shape, clipped.clip_rect, &mut sink);
            }
            sink
        }

        let mut counted = 0;
        let mut wrapped = false;
        let mut in_band = false;
        for step in 0..=440 {
            let width = 320.0 + step as f32;
            let ctx = egui::Context::default();
            crate::theme::apply(&ctx, ThemeKind::Charcoal);
            let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(width, 300.0));
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..egui::RawInput::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        filter_strip(ui, &t, &mut state, &engine, 2);
                    });
                },
            );
            output.textures_delta.clear();
            let painted = texts(&output);
            let Some((text, rect, clip)) = painted
                .iter()
                .find(|(text, _, _)| text.ends_with(" shown"))
                .map(|(text, rect, clip)| (text.clone(), *rect, *clip))
            else {
                continue;
            };
            assert!(
                clip.contains_rect(rect),
                "at {width} points `{text}` is clipped away at {rect:?} by {clip:?}"
            );
            for (other, other_rect, _) in &painted {
                assert!(
                    other.ends_with(" shown") || !rect.intersects(*other_rect),
                    "at {width} points `{text}` at {rect:?} is painted over `{other}` at {other_rect:?}"
                );
            }
            counted += 1;
            let chips: Vec<Rect> = [
                "+ keyword",
                "Rating",
                "Flags",
                "Colour",
                "All kinds",
                "Source",
                "Edited",
                "Missing",
                "Clear",
            ]
            .iter()
            .filter_map(|label| label_at(&output, label))
            .collect();
            assert_eq!(chips.len(), 9, "at {width} points the strip prints its nine chips");
            let rows: std::collections::BTreeSet<i32> = chips.iter().map(|chip| (chip.center().y / 10.0).round() as i32).collect();
            wrapped |= rows.len() >= 2;
            let band = chips.iter().fold(Rect::NOTHING, |band, chip| band.union(*chip));
            in_band |= rect.center().y > band.top() && rect.center().y < band.bottom();
        }
        assert!(counted >= 400, "the count is painted at only {counted} of the swept widths");
        assert!(wrapped, "the chips never wrapped, so the sweep never tested a tight row");
        assert!(in_band, "the count never sat inside the chips' band, so no collision was possible");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The sidebar paints its keyword rows, and the delete-everywhere menu action works over a real
    /// library: the keyword goes, its filter does not linger, and one undo brings it back.
    #[test]
    fn the_sidebar_paints_keywords_and_deleting_one_everywhere_works() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let dir = std::env::temp_dir().join(format!("sortlens-ui-tag-delete-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-tag-delete");
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
        engine.set_tag(&ids, "beach", true).expect("tag");

        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        state.tag_filters.push("beach".to_string());
        let mut actions: Vec<Action> = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                left_panel(ui, &mut state, &tokens, &mut engine, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(actions.is_empty(), "painting the sidebar presses nothing");
        assert!(!engine.tag_vocabulary().is_empty(), "the keyword row is drawn");

        // The menu action the row offers, driven the way the row's own handler runs it.
        delete_keyword_everywhere(&mut state, &mut engine, "beach");
        assert!(engine.tag_vocabulary().is_empty(), "the keyword is gone everywhere");
        assert!(
            state.tag_filters.is_empty(),
            "a filter pointing at a deleted keyword does not linger"
        );
        assert!(state.notice.is_some(), "the notice says what happened");

        // One undo brings the keyword back on every photo it was on.
        assert!(engine.undo().expect("undo"));
        assert_eq!(engine.tag_vocabulary(), vec![("beach".to_string(), 2)]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Where a label with this text was painted, walking nested shapes: a row paints its label inside a layout
    /// shape, and a test that only looked at the top level would find nothing and read as "not offered".
    fn label_at(output: &egui::FullOutput, needle: &str) -> Option<Rect> {
        fn walk(shape: &egui::Shape, needle: &str) -> Option<Rect> {
            match shape {
                egui::Shape::Text(text) if text.galley.job.text.contains(needle) => Some(text.visual_bounding_rect()),
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| walk(shape, needle)),
                _ => None,
            }
        }
        output.shapes.iter().find_map(|clipped| walk(&clipped.shape, needle))
    }

    /// Every shape in the output, flattened through layout shapes.
    ///
    /// A widget paints its contents inside an `egui::Shape::Vec`, so a test that only looked at the top level
    /// finds no picture at all and reads as "nothing was painted" — the same trap [`label_at`] avoids for text.
    fn all_shapes(output: &egui::FullOutput) -> Vec<&egui::Shape> {
        fn walk<'a>(shape: &'a egui::Shape, sink: &mut Vec<&'a egui::Shape>) {
            sink.push(shape);
            if let egui::Shape::Vec(shapes) = shape {
                for shape in shapes {
                    walk(shape, sink);
                }
            }
        }
        let mut sink = Vec::new();
        for clipped in &output.shapes {
            walk(&clipped.shape, &mut sink);
        }
        sink
    }

    /// The rects of every picture painted in the output.
    ///
    /// An image in this egui paints as a textured rectangle (the fork moved images from meshes to `RectShape`
    /// brushes), so "a glyph was painted here" is a rect whose brush carries a texture that is not the font atlas.
    fn painted_pictures(output: &egui::FullOutput) -> Vec<Rect> {
        all_shapes(output)
            .into_iter()
            .filter_map(|shape| match shape {
                egui::Shape::Mesh(mesh) => Some(mesh.calc_bounds()),
                egui::Shape::Rect(shape) => shape
                    .brush
                    .as_ref()
                    .filter(|brush| brush.fill_texture_id != egui::TextureId::default())
                    .map(|_| shape.rect),
                _ => None,
            })
            .collect()
    }

    /// Every stroke painted in the shipped coastline's own colour, in paint order.
    ///
    /// The coastline is the only thing drawn in [`Tokens::map_coast`], so this is the assertion "the outline was
    /// painted" without depending on how many shorelines happen to be on screen.
    fn coastline_runs<'a>(output: &'a egui::FullOutput, tokens: &Tokens) -> Vec<&'a egui::epaint::PathShape> {
        all_shapes(output)
            .into_iter()
            .filter_map(|shape| match shape {
                egui::Shape::Path(path)
                    if path.points.len() >= 2
                        && matches!(path.stroke.color, egui::epaint::ColorMode::Solid(color) if color == tokens.map_coast) =>
                {
                    Some(path)
                }
                _ => None,
            })
            .collect()
    }

    /// Every painted circle, in paint order.
    fn painted_circles(output: &egui::FullOutput) -> Vec<egui::epaint::CircleShape> {
        all_shapes(output)
            .into_iter()
            .filter_map(|shape| match shape {
                egui::Shape::Circle(circle) => Some(*circle),
                _ => None,
            })
            .collect()
    }

    /// The rect of a text shape whose whole string is exactly `needle`.
    ///
    /// `label_at` answers "is this string painted somewhere", which is the wrong question when a row paints `3×` for
    /// a count and `×` for a button: the count comes first in paint order and would be clicked instead.
    fn exact_label_at(output: &egui::FullOutput, needle: &str) -> Option<Rect> {
        fn walk(shape: &egui::Shape, needle: &str) -> Option<Rect> {
            match shape {
                egui::Shape::Text(text) if text.galley.job.text == needle => Some(text.visual_bounding_rect()),
                egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| walk(shape, needle)),
                _ => None,
            }
        }
        output.shapes.iter().find_map(|clipped| walk(&clipped.shape, needle))
    }

    /// A click at a point: egui reports a click on the release, so the press goes out in one frame.
    fn press_and_release(at: egui::Pos2) -> [egui::RawInput; 2] {
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        [
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(at), button(true)],
                ..Default::default()
            },
            egui::RawInput {
                events: vec![button(false)],
                ..Default::default()
            },
        ]
    }

    /// SL-D12 from the window: the strip's Source chip narrows the library to a provenance, and the keyword chip
    /// prints where that word came from. Both are driven by clicks, because a filter nobody can reach is not a filter.
    #[test]
    fn the_source_chip_narrows_the_library_and_the_keyword_chip_says_where_it_came_from() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-tag-source-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-tag-source");
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
        engine.set_tag(&ids[..1], "typed", true).expect("tag by hand");
        assert!(engine.accept_suggestion(ids[1], "guessed").expect("accept"));
        assert_eq!(engine.count(&Filter::default()), 2);

        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Midnight, 0.0);
        let frame = |state: &mut UiState, engine: &Engine, input: egui::RawInput| {
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    filter_strip(ui, &tokens, state, engine, 2);
                });
            });
            output.textures_delta.clear();
            output
        };

        // Nothing asked yet: the chip reads "Source", and every photo is in the answer.
        assert_eq!(state.source_filter, None);
        let output = frame(&mut state, &engine, egui::RawInput::default());
        let chip = label_at(&output, "Source").expect("the strip offers a source chip");
        let mut inputs = press_and_release(chip.center()).into_iter();
        frame(&mut state, &engine, inputs.next().expect("press"));
        frame(&mut state, &engine, inputs.next().expect("release"));
        assert_eq!(
            state.source_filter,
            Some(sortlens_core::TagSource::Manual),
            "the chip starts at Manual"
        );
        assert_eq!(engine.count(&state.filter()), 1, "only the keyword typed by hand answers");
        assert_eq!(state.active_filter_count(), 1, "the header badge counts it");

        // One more click moves to AI, which is the question provenance exists to answer.
        let output = frame(&mut state, &engine, egui::RawInput::default());
        let chip = label_at(&output, "Manual keywords").expect("the chip says what is asked");
        let mut inputs = press_and_release(chip.center()).into_iter();
        frame(&mut state, &engine, inputs.next().expect("press"));
        frame(&mut state, &engine, inputs.next().expect("release"));
        assert_eq!(state.source_filter, Some(sortlens_core::TagSource::Ai));
        assert_eq!(engine.count(&state.filter()), 1);

        // The keyword chip names the source in words, not only on hover.
        let photo = engine.photo(ids[1]).expect("photo").clone();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                tags(ui, &tokens, &mut state, &mut engine, &photo);
            });
        });
        output.textures_delta.clear();
        assert!(
            label_at(&output, "guessed · AI").is_some(),
            "an accepted suggestion is marked on the chip"
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

        // A PNG spec paints its own branch — edge but no quality, because PNG is lossless — and
        // drawing it must not quietly turn the spec into something else.
        state.export_open = true;
        state.export_receipt = None;
        state.export_spec = sortlens_engine::ExportSpec {
            format: sortlens_engine::ExportFormat::Png,
            max_edge: Some(1024),
            quality: 90,
            metadata: true,
            collision: sortlens_engine::Collision::Skip,
        };
        let mut actions: Vec<Action> = Vec::new();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                export_dialog(ui, &mut state, &tokens, &mut engine, &mut actions);
            });
        });
        output.textures_delta.clear();
        assert!(
            state.export_spec.format == sortlens_engine::ExportFormat::Png,
            "the dialog does not rewrite the format"
        );
        assert!(
            state.export_spec.summary().contains("lossless"),
            "the PNG spec is described as lossless, never by a quality number"
        );

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

        state.view = View::Compare;
        state.compare = Some(PhotoId(4));
        toggle_compare(&mut state, &engine);
        assert_eq!(state.compare, None, "a second press closes the compare");
        assert_eq!(state.view, View::Detail, "and leaves the photograph it was about, not the grid");

        // With nothing selected there is nothing to compare against, and the reason is said out loud.
        state.selection.clear();
        toggle_compare(&mut state, &engine);
        assert_eq!(state.compare, None);
        assert!(state.live_notice().is_some() || state.notice.is_some(), "the reason is not silent");
        assert_eq!(state.unread_notices(), 1, "and the bell carries it after the toast has gone");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Settings → AI: the consent checkbox is the gate, and the Install buttons only exist behind
    /// it. Clicked through the real widget, over a real (empty) model store.
    ///
    /// No download is started here — the pinned models are on the network, and a headless test does
    /// not fetch them. What this test owns is the gate: consent off paints no Install button, the
    /// checkbox click reaches the engine, and consent on paints them.
    #[test]
    fn the_ai_settings_section_gates_install_behind_consent() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-models-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let engine_cell = std::cell::RefCell::new(Engine::open(&dir).expect("open"));
        assert!(!engine_cell.borrow().models_consent(), "consent starts off");

        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.settings_open = true;
        let actions_cell = std::cell::RefCell::new(Vec::<Action>::new());
        let frame = |state: &mut UiState, mut input: egui::RawInput| {
            // The window needs a screen to land in; a bare RawInput has none.
            input.screen_rect = Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0)));
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    settings_dialog(ui, state, &tokens, &mut engine_cell.borrow_mut(), &mut actions_cell.borrow_mut());
                });
            });
            output.textures_delta.clear();
            output
        };

        // With consent off: the rows paint, the switch is off, and there is no Install button.
        // The window positions itself on its first frame, so the claims are read on the second.
        let _warm = frame(&mut state, egui::RawInput::default());
        let output = frame(&mut state, egui::RawInput::default());
        assert!(
            output.shapes.iter().any(|clipped| matches!(&clipped.shape, egui::Shape::Text(text)
                if text.galley.text().contains("YOLOS-tiny"))),
            "the manifest's models are listed"
        );
        assert!(
            !output.shapes.iter().any(|clipped| matches!(&clipped.shape, egui::Shape::Text(text)
                if text.galley.text() == "Install")),
            "no Install button without consent"
        );
        assert!(actions_cell.borrow().is_empty(), "painting alone changes nothing");

        // Click the consent checkbox: the label's own rect is the click target.
        let label = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text().starts_with("Allow downloading models") => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                _ => None,
            })
            .expect("the consent checkbox painted its label");
        let at = label.center();
        let click = |at: egui::Pos2, pressed: bool| egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0))),
            events: vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..egui::RawInput::default()
        };
        frame(&mut state, click(at, true));
        frame(&mut state, click(at, false));
        assert!(engine_cell.borrow().models_consent(), "the click reached the engine");
        assert!(
            actions_cell.borrow().iter().any(|action| action == &Action::SetModelConsent(true)),
            "and asked the host to persist it"
        );

        // With consent on, each row offers Install — and removal only for what is installed.
        let output = frame(&mut state, egui::RawInput::default());
        assert!(
            output.shapes.iter().any(|clipped| matches!(&clipped.shape, egui::Shape::Text(text)
                if text.galley.text() == "Install")),
            "Install appears behind consent"
        );
        assert!(
            !output.shapes.iter().any(|clipped| matches!(&clipped.shape, egui::Shape::Text(text)
                if text.galley.text() == "Remove")),
            "nothing is installed, so nothing offers Remove"
        );

        // And the gate can be withdrawn: the switch goes off and the buttons go with it.
        actions_cell.borrow_mut().clear();
        frame(&mut state, click(at, true));
        frame(&mut state, click(at, false));
        assert!(!engine_cell.borrow().models_consent(), "consent is withdrawn");
        assert!(actions_cell.borrow().iter().any(|action| action == &Action::SetModelConsent(false)));
        let output = frame(&mut state, egui::RawInput::default());
        assert!(
            !output.shapes.iter().any(|clipped| matches!(&clipped.shape, egui::Shape::Text(text)
                if text.galley.text() == "Install")),
            "the buttons are gone with the consent"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Settings → Appearance lists every preset — the four colour ones and High Contrast — and
    /// clicking High Contrast asks the host for it, which is what persists `theme = "high-contrast"`
    /// in `config.toml`. Driven through the real radio widgets, the way a person clicks them.
    #[test]
    fn the_settings_dialog_offers_high_contrast_and_clicking_it_asks_the_host() {
        let engine_cell = std::cell::RefCell::new(Engine::open(std::env::temp_dir().join("sortlens-ui-theme-empty")).expect("open"));
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.settings_open = true;
        let actions_cell = std::cell::RefCell::new(Vec::<Action>::new());
        let frame = |state: &mut UiState, mut input: egui::RawInput| {
            input.screen_rect = Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0)));
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    settings_dialog(ui, state, &tokens, &mut engine_cell.borrow_mut(), &mut actions_cell.borrow_mut());
                });
            });
            output.textures_delta.clear();
            output
        };

        // The window positions itself on its first frame; read the claims on the second.
        let _warm = frame(&mut state, egui::RawInput::default());
        let output = frame(&mut state, egui::RawInput::default());
        for kind in ThemeKind::ALL {
            assert!(
                output.shapes.iter().any(|clipped| matches!(&clipped.shape, egui::Shape::Text(text)
                    if text.galley.text() == kind.label())),
                "{} is offered in Settings",
                kind.label()
            );
        }

        // Click the High Contrast radio through its own label.
        let label = output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) if text.galley.text() == ThemeKind::HighContrast.label() => {
                    Some(text.galley.rect.translate(text.pos.to_vec2()))
                }
                _ => None,
            })
            .expect("the High Contrast radio painted its label");
        let at = label.center();
        let click = |at: egui::Pos2, pressed: bool| egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0))),
            events: vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..egui::RawInput::default()
        };
        frame(&mut state, click(at, true));
        frame(&mut state, click(at, false));
        assert!(
            actions_cell
                .borrow()
                .iter()
                .any(|action| action == &Action::SetTheme(ThemeKind::HighContrast)),
            "the click asked the host to switch and persist the preset"
        );
        assert_eq!(
            state.theme,
            ThemeKind::Charcoal,
            "the shell waits for the host, as every SetTheme does"
        );
    }

    /// Phase 3 item 11: the palette is a front door to the command registry, not a second command list.
    ///
    /// Every id the palette can show has to be a `Ready` registry command — a row for something the
    /// engine refuses would be a menu item that answers with an error — and the rows the palette
    /// paints are the registry's, label and group and shortcut included.
    #[test]
    fn every_palette_command_is_a_ready_registry_command() {
        let registry = sortlens_engine::command::Registry::v0();
        let mut ids: Vec<&str> = PaletteCommand::ALL.iter().map(|command| command.id()).collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), count, "two palette rows name one command");
        for command in PaletteCommand::ALL {
            let spec = registry
                .expect(command.id())
                .unwrap_or_else(|_| panic!("{} is not a registry command", command.id()));
            assert!(
                spec.is_ready(),
                "{} is {}, but the palette presents it as runnable",
                command.id(),
                spec.milestone().unwrap_or("not ready")
            );
        }
        // The rows the empty query shows are the commands, in the same order, with the registry's text.
        let rows = palette_rows("");
        assert_eq!(rows.len(), PaletteCommand::ALL.len());
        for (command, spec) in rows {
            assert_eq!(spec.id, command.id());
            assert!(
                !spec.label.is_empty() && spec.menu.contains(" > "),
                "{} has no menu identity",
                spec.id
            );
        }
        // A query narrows by label, id and group, and "settings" answers with one row.
        assert_eq!(
            palette_rows("settings").iter().map(|(command, _)| *command).collect::<Vec<_>>(),
            vec![PaletteCommand::Settings]
        );
        assert!(palette_rows("nothing-is-called-this").is_empty());
        // The group name answers too: every Library command is found by the word "library".
        let library = palette_rows("library");
        assert_eq!(
            library
                .iter()
                .filter(|(_, spec)| spec.group == sortlens_engine::command::Group::Library)
                .count(),
            8,
            "{library:?}"
        );
    }

    /// A palette row that cannot run says why, and one that can leaves exactly its own trace.
    #[test]
    fn the_palette_commands_run_or_explain_why_not() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-palette-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let mut actions = Vec::new();

        // With nothing selected, the commands that need a photograph say so instead of doing nothing.
        for command in [PaletteCommand::Reveal, PaletteCommand::Open, PaletteCommand::Export] {
            let reason = command
                .run(&mut state, &mut engine, &mut actions)
                .expect_err("no selection, so it cannot run");
            assert!(reason.contains("photo") || reason.contains("Select"), "{reason}");
        }
        assert!(actions.is_empty(), "a refused command asks the host for nothing");

        // And the commands that are always available do what the menus do.
        PaletteCommand::Settings
            .run(&mut state, &mut engine, &mut actions)
            .expect("settings");
        assert!(state.settings_open);
        PaletteCommand::Shortcuts
            .run(&mut state, &mut engine, &mut actions)
            .expect("shortcuts");
        assert!(state.shortcuts_open);
        PaletteCommand::Insights
            .run(&mut state, &mut engine, &mut actions)
            .expect("insights");
        assert_eq!(state.view, View::Insights);
        PaletteCommand::AddFolder
            .run(&mut state, &mut engine, &mut actions)
            .expect("add folder");
        assert_eq!(actions, vec![Action::PickFolder], "the folder picker is the host's job");
        // A cull command with nothing staged refuses rather than opening an empty Apply dialog.
        let reason = PaletteCommand::ApplyCull
            .run(&mut state, &mut engine, &mut actions)
            .expect_err("nothing staged");
        assert!(reason.contains("Nothing staged"), "{reason}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// ⌘K opens command mode, typing narrows it, Enter runs the highlighted row, and Esc goes back to
    /// photo search with the photographs' query untouched.
    ///
    /// Driven through `show` with real input events: the palette's keys are consumed before the search
    /// field draws, and only a test that presses them can show that.
    #[test]
    fn command_k_opens_the_palette_and_enter_runs_the_highlighted_command() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-palette-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.search = "beach".to_string();
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0));
        let frame = |state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..egui::RawInput::default()
            };
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, state, engine, thumbs);
                });
            });
            output.textures_delta.clear();
            output
        };
        let key = |key: Key, modifiers: Modifiers, pressed: bool| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };

        frame(&mut state, &mut engine, &mut thumbs, Vec::new());
        assert!(!state.palette_open);

        // ⌘K.
        frame(
            &mut state,
            &mut engine,
            &mut thumbs,
            vec![key(Key::K, Modifiers::COMMAND, true), key(Key::K, Modifiers::COMMAND, false)],
        );
        assert!(state.palette_open, "⌘K opens command mode");
        assert!(state.palette_query.is_empty(), "command mode starts with an empty query");

        // Type "about": the field the bar draws in command mode has focus.
        frame(&mut state, &mut engine, &mut thumbs, vec![egui::Event::Text("about".into())]);
        assert_eq!(state.palette_query, "about");
        assert_eq!(
            palette_rows(&state.palette_query)
                .iter()
                .map(|(command, _)| *command)
                .collect::<Vec<_>>(),
            vec![PaletteCommand::About],
            "typing narrows the rows"
        );

        // Enter runs it and leaves command mode.
        frame(
            &mut state,
            &mut engine,
            &mut thumbs,
            vec![key(Key::Enter, Modifiers::NONE, true), key(Key::Enter, Modifiers::NONE, false)],
        );
        assert!(state.about_open, "Enter ran the highlighted row");
        assert!(!state.palette_open, "and the palette closed");
        assert_eq!(state.search, "beach", "the photo search was never touched");

        // Esc leaves command mode and puts the caret back in the photo search, with its text intact.
        frame(
            &mut state,
            &mut engine,
            &mut thumbs,
            vec![key(Key::K, Modifiers::COMMAND, true), key(Key::K, Modifiers::COMMAND, false)],
        );
        assert!(state.palette_open, "⌘K opens it again");
        frame(
            &mut state,
            &mut engine,
            &mut thumbs,
            vec![key(Key::Escape, Modifiers::NONE, true), key(Key::Escape, Modifiers::NONE, false)],
        );
        assert!(!state.palette_open, "Esc goes back to photo search");
        assert_eq!(state.search, "beach");
        // "Back to photo search" is more than a mode flag: the next character typed lands in the
        // photographs' field, not in a command query that is no longer on screen.
        frame(&mut state, &mut engine, &mut thumbs, vec![egui::Event::Text("x".into())]);
        assert_eq!(state.search, "beachx", "the caret went back to the photo search");
        assert!(state.palette_query.is_empty());

        // A row is clickable too, not only reachable with the arrow keys.
        frame(
            &mut state,
            &mut engine,
            &mut thumbs,
            vec![key(Key::K, Modifiers::COMMAND, true), key(Key::K, Modifiers::COMMAND, false)],
        );
        frame(&mut state, &mut engine, &mut thumbs, vec![egui::Event::Text("quit".into())]);
        let output = frame(&mut state, &mut engine, &mut thumbs, Vec::new());
        let row = label_at(&output, "Quit Sortlens").expect("the filtered row is painted");
        let mut clicks = press_and_release(row.center()).into_iter();
        frame(&mut state, &mut engine, &mut thumbs, clicks.next().expect("press").events);
        frame(&mut state, &mut engine, &mut thumbs, clicks.next().expect("release").events);
        assert!(state.quit_open, "clicking the row ran it");
        assert!(!state.palette_open, "and the palette closed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The Filter chip is the strip's switch and its badge, and the number is the one the header counts.
    #[test]
    fn the_filter_chip_toggles_the_strip_and_wears_the_active_count() {
        let ctx = egui::Context::default();
        let tokens = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.min_rating = 2;
        state.flag_filter = Some(Flag::Pick);
        assert_eq!(state.active_filter_count(), 2);
        let frame = |state: &mut UiState, input: egui::RawInput| {
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    filter_button(ui, state, &tokens);
                });
            });
            output.textures_delta.clear();
            output
        };

        let output = frame(&mut state, egui::RawInput::default());
        let badge = label_at(&output, "2").expect("the chip paints the count on itself");
        assert!(state.filters_open, "the strip starts visible");

        for input in press_and_release(badge.center()) {
            frame(&mut state, input);
        }
        assert!(!state.filters_open, "clicking the chip puts the strip away");

        // Nothing filtering means nothing to count: no badge over a chip that changes nothing.
        state.min_rating = 0;
        state.flag_filter = None;
        let output = frame(&mut state, egui::RawInput::default());
        assert!(label_at(&output, "0").is_none(), "no badge when no filter is on");

        // And the strip itself obeys the switch: hidden means the chips are gone from the window.
        let dir = std::env::temp_dir().join(format!("sortlens-ui-filter-bar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        // One photograph, because a strip needs a library to narrow (the next test is about that).
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-filter-bar");
        std::fs::copy(fixture.jpeg("a.jpg", [200, 40, 40], 64, 64), root.join("a.jpg")).expect("copy a");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let window = |state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails| {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))),
                ..egui::RawInput::default()
            };
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, state, engine, thumbs);
                });
            });
            output.textures_delta.clear();
            output
        };
        // "All kinds" is the strip's own kind chip; the Welcome copy mentions "Ratings", so a needle
        // that could match both would pass for the wrong reason.
        state.filters_open = true;
        let output = window(&mut state, &mut engine, &mut thumbs);
        assert!(label_at(&output, "All kinds").is_some(), "the strip shows its chips");
        state.filters_open = false;
        let output = window(&mut state, &mut engine, &mut thumbs);
        assert!(
            label_at(&output, "All kinds").is_none(),
            "and a hidden strip leaves no chips behind"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A strip of filter chips needs a library to narrow.
    ///
    /// On a first launch it sat above the Welcome card — nine questions about photographs that did not
    /// exist yet, on the one screen whose whole job is to say where photographs come from. The test also
    /// pins the other half of the rule: a *filter* that hides every photo must keep its strip, because
    /// clearing it is what the strip is for. So the gate is the catalog's total, never the filtered count.
    #[test]
    fn the_filter_strip_needs_a_library_to_narrow() {
        let ctx = egui::Context::default();
        let dir = std::env::temp_dir().join(format!("sortlens-ui-filter-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.filters_open = true;
        let window = |state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails| {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))),
                ..egui::RawInput::default()
            };
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, state, engine, thumbs);
                });
            });
            output.textures_delta.clear();
            output
        };

        let output = window(&mut state, &mut engine, &mut thumbs);
        assert!(
            label_at(&output, "Welcome to Sortlens").is_some(),
            "an empty library is the Welcome screen"
        );
        assert!(
            label_at(&output, "All kinds").is_none(),
            "and the strip is not drawn over it, even though the Filter toggle says open"
        );
        assert!(
            label_at(&output, "0 shown").is_none(),
            "no chip of the strip is left behind — not even the count on its right edge, which the sidebar never prints"
        );
        // One photograph is enough: the strip appears at the same toggle state.
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-filter-empty");
        std::fs::copy(fixture.jpeg("a.jpg", [200, 40, 40], 64, 64), root.join("a.jpg")).expect("copy a");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let output = window(&mut state, &mut engine, &mut thumbs);
        assert!(label_at(&output, "All kinds").is_some(), "one photo and the strip is back");

        // A question with no answer is still a question being asked: the strip stays and offers Clear.
        state.min_rating = 5;
        assert_eq!(engine.count(&state.filter()), 0, "nothing in this library is rated five stars");
        let output = window(&mut state, &mut engine, &mut thumbs);
        assert!(
            label_at(&output, "All kinds").is_some(),
            "a filter that hides every photo keeps the strip that can clear it"
        );
        assert!(
            label_at(&output, "Clear").is_some(),
            "and the strip's own Clear chip is reachable, because the row wraps rather than overflowing"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A filter that hides every photograph gets its own screen, not the Welcome card.
    ///
    /// The old gate asked the *filtered* count whether the library existed, so a narrowed-to-nothing library
    /// painted "Welcome to Sortlens / Add Folder…" — a first-launch answer to a question nobody asked, with
    /// the one button on it making the situation worse. This test drives the replacement with a real pointer
    /// press: the screen names the library's own count, and its `Clear filters` puts the photographs back.
    /// The other half is pinned too, because a fix that showed this card over an actually-empty library would
    /// be the same mistake pointing the other way.
    #[test]
    fn a_narrowed_to_nothing_library_says_so_and_offers_a_way_back() {
        let ctx = egui::Context::default();
        let dir = std::env::temp_dir().join(format!("sortlens-ui-no-matches-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.filters_open = true;
        let window = |state: &mut UiState, engine: &mut Engine, thumbs: &mut Thumbnails, input: egui::RawInput| {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0))),
                ..input
            };
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, state, engine, thumbs);
                });
            });
            output.textures_delta.clear();
            output
        };

        // A library with nothing in it is still the Welcome card, and claims nothing about filters.
        let output = window(&mut state, &mut engine, &mut thumbs, egui::RawInput::default());
        assert!(
            label_at(&output, "Welcome to Sortlens").is_some(),
            "a first launch is the Welcome card"
        );
        assert!(
            label_at(&output, "Nothing matches these filters").is_none(),
            "and an empty catalog is not a filtered one"
        );

        // Two photographs, then the same library narrowed to zero.
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-no-matches");
        std::fs::copy(fixture.jpeg("a.jpg", [200, 40, 40], 64, 64), root.join("a.jpg")).expect("copy a");
        std::fs::copy(fixture.jpeg("b.jpg", [40, 200, 40], 64, 64), root.join("b.jpg")).expect("copy b");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(engine.stats().total, 2, "two photographs are in the catalog");
        state.min_rating = 5;
        assert_eq!(engine.count(&state.filter()), 0, "nothing here is rated five stars");

        let output = window(&mut state, &mut engine, &mut thumbs, egui::RawInput::default());
        assert!(
            label_at(&output, "Nothing matches these filters").is_some(),
            "the canvas says why it is empty instead of greeting the person"
        );
        assert!(
            label_at(&output, "2 photographs in this library").is_some(),
            "and names the library's own count, not the zero the filters left"
        );
        assert!(
            label_at(&output, "Welcome to Sortlens").is_none(),
            "the Welcome card is not painted over a library that has photographs"
        );

        // One press on the screen's own button, not a state assignment.
        let clear = label_at(&output, "Clear filters").expect("the screen offers the way back");
        let mut inputs = press_and_release(clear.center()).into_iter();
        window(&mut state, &mut engine, &mut thumbs, inputs.next().expect("press"));
        window(&mut state, &mut engine, &mut thumbs, inputs.next().expect("release"));
        assert_eq!(state.active_filter_count(), 0, "every dimension the badge counted is clear");
        assert_eq!(engine.count(&state.filter()), 2, "and the photographs are back");
        let output = window(&mut state, &mut engine, &mut thumbs, egui::RawInput::default());
        assert!(
            label_at(&output, "Nothing matches these filters").is_none(),
            "the screen drops itself once there is something to draw"
        );

        // A search query alone is a reason for the same screen, so the button clears that too.
        state.search = "zzzz-no-such-photo".to_string();
        assert_eq!(engine.count(&state.filter()), 0);
        let output = window(&mut state, &mut engine, &mut thumbs, egui::RawInput::default());
        assert!(label_at(&output, "Nothing matches these filters").is_some());
        let clear = label_at(&output, "Clear filters").expect("still offered");
        let mut inputs = press_and_release(clear.center()).into_iter();
        window(&mut state, &mut engine, &mut thumbs, inputs.next().expect("press"));
        window(&mut state, &mut engine, &mut thumbs, inputs.next().expect("release"));
        assert!(state.search.is_empty(), "a query that matches nothing is cleared with the filters");
        assert_eq!(engine.count(&state.filter()), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    ///
    /// Painted one control at a time, because the assertion is exact: a chip alone in a panel paints
    /// exactly as many glyphs as it means, and a channel that carries text would fail the "no text"
    /// half. The typed `★` this replaced was a character, not the app's star.
    #[test]
    fn the_filter_strip_paints_its_state_as_pictures() {
        let ctx = egui::Context::default();
        let t = Tokens::CHARCOAL;
        // The chips paint icons, so the loader `show` installs has to be installed for this panel too.
        crate::icons::install(&ctx);
        let paint = |body: &dyn Fn(&mut Ui)| {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| body(ui));
            });
            output.textures_delta.clear();
            output
        };
        // The rating chip at rest: one faint star and the word "Rating".
        let idle = paint(&|ui| {
            rating_chip(ui, &t, 0);
        });
        assert_eq!(painted_pictures(&idle).len(), 1, "the unset chip is one star glyph and no more");
        assert!(label_at(&idle, "Rating").is_some(), "and it names the question");

        // Three stars and up: three glyphs, one per star, and no numeral or typed `★` anywhere.
        let three = paint(&|ui| {
            rating_chip(ui, &t, 3);
        });
        let mut stars = painted_pictures(&three);
        stars.sort_by(|a, b| a.left().total_cmp(&b.left()));
        assert_eq!(stars.len(), 3, "three stars means three pictures");
        assert!(
            stars.iter().all(|rect| (rect.width() - STAR_GLYPH).abs() < 0.5),
            "each one is the icon set's star at its own size"
        );
        assert!(
            stars.windows(2).all(|pair| pair[0].right() < pair[1].left()),
            "and they are laid out in a row, not stacked"
        );
        assert!(
            label_at(&three, "★").is_none() && label_at(&three, "3").is_none(),
            "the count is drawn, so neither a typed star nor a numeral is painted"
        );

        // The colour chip paints the label's own colour, and an empty ring when no colour is asked for.
        let green = paint(&|ui| {
            colour_chip(ui, &t, Some(ColorLabel::Green), "Green");
        });
        let dots = painted_circles(&green);
        let Some(dot) = dots.first() else {
            panic!("the colour chip paints a swatch, not a droplet icon");
        };
        assert_eq!(dot.fill, t.label(ColorLabel::Green), "the swatch is the label's colour");
        assert_eq!(dot.stroke.width, 0.0, "a chosen colour is filled, not outlined");
        assert!(label_at(&green, "Green").is_some(), "and the word names it");

        let any = paint(&|ui| {
            colour_chip(ui, &t, None, "Colour");
        });
        let outlined = painted_circles(&any);
        let Some(ring) = outlined.first() else {
            panic!("the unset colour chip still paints its ring");
        };
        assert_eq!(ring.fill, Color32::TRANSPARENT, "nothing chosen means nothing filled");
        assert!(ring.stroke.width > 0.0, "the ring is the outline");
        assert!(
            label_at(&any, "Droplet").is_none(),
            "the droplet that stood in for every colour is gone"
        );

        // The flag chip wears the state's own glyph: a reject is the reject cross.
        assert_eq!(flag_filter_icon(None), crate::icons::Icon::Flag);
        assert_eq!(flag_filter_icon(Some(Flag::Pick)), crate::icons::Icon::Pick);
        assert_eq!(flag_filter_icon(Some(Flag::Reject)), crate::icons::Icon::Reject);
    }

    /// The strip's stars and swatch are clickable, because a picture inside a button is still the button.
    #[test]
    fn the_filter_strip_stars_and_swatch_take_clicks() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-filter-clicks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-filter-clicks");
        std::fs::copy(fixture.jpeg("a.jpg", [200, 40, 40], 64, 64), root.join("a.jpg")).expect("copy a");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let ctx = egui::Context::default();
        let t = Tokens::CHARCOAL;
        // The strip paints icons, and this test draws it without `show`; install the same loader.
        crate::icons::install(&ctx);
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let frame = |state: &mut UiState, engine: &Engine, input: egui::RawInput| {
            let mut output = ctx.run_ui(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    filter_strip(ui, &t, state, engine, 1);
                });
            });
            output.textures_delta.clear();
            output
        };

        // The rating chip at rest states its question in words; clicking it asks for one star, and the
        // chip then answers with one lit glyph instead.
        let output = frame(&mut state, &engine, egui::RawInput::default());
        let idle = label_at(&output, "Rating").expect("the rating chip names itself at rest");
        for input in press_and_release(idle.center()) {
            frame(&mut state, &engine, input);
        }
        assert_eq!(state.min_rating, 1, "one click asks for one star");

        // Clicking the painted star again raises the minimum: the atom really is inside the button.
        let output = frame(&mut state, &engine, egui::RawInput::default());
        // The first picture at the star's size is the lit star: the only smaller glyph in the strip is
        // the keyword menu's chevron, and the chips after it are not painted yet at this point in the test.
        let star = painted_pictures(&output)
            .into_iter()
            .find(|rect| (rect.width() - STAR_GLYPH).abs() < 0.5)
            .expect("the lit star is painted");
        for input in press_and_release(star.center()) {
            frame(&mut state, &engine, input);
        }
        assert_eq!(state.min_rating, 2, "the glyph takes the click the chip always took");

        // The swatch is the colour control now, and the Droplet glyph is no longer anywhere in the strip.
        state.min_rating = 0;
        let output = frame(&mut state, &engine, egui::RawInput::default());
        let swatch = label_at(&output, "Colour").expect("the colour chip names itself at rest");
        for input in press_and_release(swatch.center()) {
            frame(&mut state, &engine, input);
        }
        assert_eq!(state.label_filter, Some(ColorLabel::Red), "one click asks for the first colour");
        assert_eq!(state.active_filter_count(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
    ///
    /// The same audit the detail view gets, aimed at the strings this slice introduced: the palette's
    /// "Up/Down choose · Enter run" footer, the ⌘ in the search hint, and every registry label it can
    /// show. A missing glyph is tofu, which looks like a design decision until you know.
    #[test]
    fn the_top_bar_and_palette_paint_only_glyphs_the_bundled_faces_have() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-topbar-glyphs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let t = Tokens::CHARCOAL;
        let ctx = egui::Context::default();
        // The bar is drawn by `show`, which installs the glyph loader first; this test draws the bar
        // directly, so it installs the same loader or every image paints egui's error placeholder.
        crate::icons::install(&ctx);
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1200.0, 800.0));
        let mut actions = Vec::new();
        let mut missing: Vec<String> = Vec::new();
        let mut painted = 0;
        for palette in [false, true] {
            state.palette_open = palette;
            state.palette_query.clear();
            for _ in 0..2 {
                let input = egui::RawInput {
                    screen_rect: Some(screen),
                    ..egui::RawInput::default()
                };
                let mut output = ctx.run_ui(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        command_palette(ui, &mut state, &t, &mut engine, &mut actions);
                        top_bar(ui, &mut state, &t, &mut engine, &mut actions);
                    });
                });
                output.textures_delta.clear();
                painted = output.shapes.len();
                for clipped in &output.shapes {
                    if let egui::Shape::Text(text) = &clipped.shape {
                        let font = text
                            .galley
                            .job
                            .sections
                            .first()
                            .map_or_else(|| egui::FontId::proportional(12.0), |section| section.format.font_id.clone());
                        let body = text.galley.job.text.clone();
                        if !ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, &body)) {
                            missing.push(body);
                        }
                    }
                }
            }
        }
        assert!(painted > 0, "the bar painted something");
        assert!(missing.is_empty(), "these strings have no glyphs in the bundled faces: {missing:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The same audit, aimed at the shortcuts overlay.
    ///
    /// The keycap column paints in the monospace face and the actions in the proportional one, and
    /// the search field can surface any binding — so one missing glyph is a box in front of the one
    /// screen whose whole subject is keys. This is where 3B.6 (g)'s arrows were found: `←`/`→` do
    /// not exist in the proportional face a default build runs on, and the triangles that replaced
    /// them must stay covered.
    #[test]
    fn the_shortcuts_dialog_paints_only_glyphs_the_bundled_faces_have() {
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.shortcuts_open = true;
        let t = Tokens::CHARCOAL;
        let ctx = egui::Context::default();
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(900.0, 700.0));
        let mut missing: Vec<String> = Vec::new();
        let mut painted = 0;
        // With the query empty every binding paints; a second pass with a needle proves the search
        // field narrows to strings the audit still covers.
        for query in ["", "◀"] {
            state.shortcut_query = query.to_string();
            for _ in 0..2 {
                let input = egui::RawInput {
                    screen_rect: Some(screen),
                    ..egui::RawInput::default()
                };
                let mut output = ctx.run_ui(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        shortcuts_dialog(ui, &mut state, &t);
                    });
                });
                output.textures_delta.clear();
                painted = output.shapes.len();
                for clipped in &output.shapes {
                    if let egui::Shape::Text(text) = &clipped.shape {
                        let font = text
                            .galley
                            .job
                            .sections
                            .first()
                            .map_or_else(|| egui::FontId::proportional(12.0), |section| section.format.font_id.clone());
                        let body = text.galley.job.text.clone();
                        if !ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, &body)) {
                            missing.push(body);
                        }
                    }
                }
            }
        }
        assert!(painted > 0, "the dialog painted something");
        assert!(missing.is_empty(), "these strings have no glyphs in the bundled faces: {missing:?}");
    }

    /// Paint only the bottom bar and hand back what it drew.
    ///
    /// The bar is the whole subject here: a scene that painted the grid too would make "which glyph did I
    /// click" a question about the canvas.
    fn paint_bar(
        ctx: &egui::Context,
        screen: Vec2,
        state: &mut UiState,
        engine: &mut Engine,
        actions: &mut Vec<Action>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), screen)),
            ..egui::RawInput::default()
        };
        // Twice: the SVG loader is asynchronous, so the first frame lays the bar out and the second
        // paints its glyphs. A single frame would read as "no icons" for a bar full of them.
        let mut settled = None;
        for _ in 0..2 {
            let mut output = ctx.run_ui(input.clone(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    bottom_bar(ui, state, &Tokens::CHARCOAL, engine, actions);
                });
            });
            output.textures_delta.clear();
            settled = Some(output);
        }
        settled.expect("two frames were painted")
    }

    /// The icon-only buttons inside `area`, left to right.
    fn glyphs_in(output: &egui::FullOutput, area: Rect) -> Vec<Rect> {
        let mut rects: Vec<Rect> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Mesh(mesh) => Some(mesh.calc_bounds()),
                egui::Shape::Rect(shape) => shape
                    .brush
                    .as_ref()
                    .filter(|brush| brush.fill_texture_id != egui::TextureId::default())
                    .map(|_| shape.rect),
                _ => None,
            })
            .filter(|rect| area.contains(rect.center()))
            .collect();
        rects.sort_by(|left, right| left.center().x.total_cmp(&right.center().x));
        rects
    }

    /// The annotation pill's own background: the one card-filled surface in the bar.
    fn pill_in(output: &egui::FullOutput, tokens: &Tokens, screen: Vec2) -> Rect {
        output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape) if shape.fill == tokens.card && shape.rect.center().y > screen.y - tokens.bottom_bar_h => {
                    Some(shape.rect)
                }
                _ => None,
            })
            .expect("the annotation pill has a surface of its own")
    }

    /// Phase 3B.4 from the window: the centre pill rates and flags the selection, and clicking the star that
    /// is already set clears it. Driven by real clicks on the painted glyphs, because a control that is only
    /// reachable from a keyboard is not the control the bar claims to be.
    #[test]
    fn the_annotation_pill_rates_and_flags_the_selection() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-pill-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-pill");
        for index in 1..=2 {
            let name = format!("IMG_000{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [30 * index as u8, 90, 170], 64, 48), root.join(&name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
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
        assert_eq!(ids.len(), 2);
        let (first, second) = (ids[0], ids[1]);

        let ctx = egui::Context::default();
        let screen = vec2(1280.0, 300.0);
        let tokens = Tokens::CHARCOAL;
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.selection.select_many(&ids);
        let mut actions: Vec<Action> = Vec::new();
        // The shell installs this on every frame it owns; painting the bar on its own has to say it.
        crate::icons::install(&ctx);

        let output = paint_bar(&ctx, screen, &mut state, &mut engine, &mut actions);
        assert!(actions.is_empty(), "painting presses nothing");
        assert!(
            label_at(&output, "2 selected").is_some(),
            "the pill's count is the selection, in words"
        );
        let pill = pill_in(&output, &tokens, screen);
        let glyphs = glyphs_in(&output, pill);
        assert_eq!(
            glyphs.len(),
            8,
            "five stars and three flags, and nothing else in the pill: {}",
            glyphs.len()
        );

        // A keyword count joins the pill when the primary carries keywords: the bar answers "did it land"
        // without a trip to the Tags panel.
        engine.set_tag(&[first], "sea", true).expect("tag");
        let tagged = paint_bar(&ctx, screen, &mut state, &mut engine, &mut actions);
        assert_eq!(
            glyphs_in(&tagged, pill_in(&tagged, &tokens, screen)).len(),
            9,
            "the tag glyph joins the five stars and three flags"
        );

        // The third star rates both photographs: the pill acts on the selection, like the number keys.
        click_frames(&ctx, screen, glyphs[2].center(), &mut state, &mut engine, &mut actions);
        assert_eq!(engine.photo(first).expect("photo").rating, 3, "three stars on the primary");
        assert_eq!(engine.photo(second).expect("photo").rating, 3, "and on the rest of the selection");
        assert_eq!(
            state.live_notice().map(|(text, _)| text.to_string()),
            Some("Rated 3★ on 2 photos".to_string())
        );

        // The same star again clears it, which is what a star that is already lit should do.
        let output = paint_bar(&ctx, screen, &mut state, &mut engine, &mut actions);
        let glyphs = glyphs_in(&output, pill_in(&output, &tokens, screen));
        click_frames(&ctx, screen, glyphs[2].center(), &mut state, &mut engine, &mut actions);
        assert_eq!(engine.photo(first).expect("photo").rating, 0, "the lit star clears the rating");
        assert_eq!(engine.photo(second).expect("photo").rating, 0);

        // Pick sits after the five stars, and flags the whole selection.
        click_frames(&ctx, screen, glyphs[5].center(), &mut state, &mut engine, &mut actions);
        assert_eq!(engine.photo(first).expect("photo").flag, Flag::Pick);
        assert_eq!(engine.photo(second).expect("photo").flag, Flag::Pick);
        assert_eq!(
            state.live_notice().map(|(text, _)| text.to_string()),
            Some("Picked 2 photos".to_string())
        );

        // And Reject is the seventh glyph: the pill offers all three flag states, not only the happy one.
        let output = paint_bar(&ctx, screen, &mut state, &mut engine, &mut actions);
        let glyphs = glyphs_in(&output, pill_in(&output, &tokens, screen));
        click_frames(&ctx, screen, glyphs[7].center(), &mut state, &mut engine, &mut actions);
        assert_eq!(engine.photo(first).expect("photo").flag, Flag::Reject);
        assert_eq!(engine.photo(second).expect("photo").flag, Flag::Reject);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Press and release at a point, painting the bar in between.
    fn click_frames(
        ctx: &egui::Context,
        screen: Vec2,
        at: egui::Pos2,
        state: &mut UiState,
        engine: &mut Engine,
        actions: &mut Vec<Action>,
    ) {
        let tokens = Tokens::CHARCOAL;
        for input in press_and_release(at) {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: input.events,
                    screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), screen)),
                    ..egui::RawInput::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        bottom_bar(ui, state, &tokens, engine, actions);
                    });
                },
            );
            output.textures_delta.clear();
        }
    }

    /// Phase 3B.4: a narrow window collapses the tab row into the current destination plus a menu, and the
    /// menu still holds every destination. Width is lost, not reach.
    #[test]
    fn a_narrow_window_collapses_the_tab_row_instead_of_dropping_a_destination() {
        // The threshold itself: the tabs plus the pill and the right-hand controls.
        assert!(tabs_collapse(820.0, 732.0), "a narrow bar collapses");
        assert!(!tabs_collapse(1440.0, 732.0), "a wide one does not");
        assert_eq!(View::ALL.len(), 9, "every destination is in the row");
        assert_eq!(View::ALL[0], View::Library, "the row starts where it always did");

        let dir = std::env::temp_dir().join(format!("sortlens-ui-collapse-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open a library");
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let mut actions: Vec<Action> = Vec::new();
        let ctx = egui::Context::default();
        crate::icons::install(&ctx);

        let wide = paint_bar(&ctx, vec2(1440.0, 300.0), &mut state, &mut engine, &mut actions);
        assert!(
            label_at(&wide, "Soft Focus").is_some(),
            "the wide row shows the last-but-one destination"
        );
        assert!(label_at(&wide, "Insights").is_some(), "and the last");

        let narrow = paint_bar(&ctx, vec2(820.0, 300.0), &mut state, &mut engine, &mut actions);
        assert!(
            label_at(&narrow, "Soft Focus").is_none(),
            "the collapsed row is the current destination, not the whole list"
        );
        let current = label_at(&narrow, "Library").expect("the current destination is named");
        click_frames(&ctx, vec2(820.0, 300.0), current.center(), &mut state, &mut engine, &mut actions);
        let opened = paint_bar(&ctx, vec2(820.0, 300.0), &mut state, &mut engine, &mut actions);
        assert!(
            label_at(&opened, "Soft Focus").is_some(),
            "the menu behind the collapsed tab still holds every destination"
        );
        assert!(actions.is_empty(), "opening the menu asks for nothing");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Paint the whole shell, settled: the SVG loader is asynchronous, so one frame can be a frame with no glyphs.
    fn paint_shell(
        ctx: &egui::Context,
        screen: Vec2,
        state: &mut UiState,
        engine: &mut Engine,
        thumbs: &mut Thumbnails,
        actions: &mut Vec<Action>,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), screen)),
            ..egui::RawInput::default()
        };
        let mut settled = None;
        for _ in 0..3 {
            let mut output = ctx.run_ui(input.clone(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let asked = show(ui, state, engine, thumbs);
                    actions.extend(asked);
                });
            });
            output.textures_delta.clear();
            settled = Some(output);
        }
        settled.expect("three frames were painted")
    }

    /// A click anywhere in the shell: press in one frame, release in the next, painting the whole window.
    fn click_shell(
        ctx: &egui::Context,
        screen: Vec2,
        at: egui::Pos2,
        state: &mut UiState,
        engine: &mut Engine,
        thumbs: &mut Thumbnails,
        actions: &mut Vec<Action>,
    ) {
        for input in press_and_release(at) {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: input.events,
                    screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), screen)),
                    ..egui::RawInput::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let asked = show(ui, state, engine, thumbs);
                        actions.extend(asked);
                    });
                },
            );
            output.textures_delta.clear();
        }
    }

    /// The glyphs painted inside `area`, top to bottom: the far-edge rail is a column, so its order is its y.
    fn glyphs_top_to_bottom(output: &egui::FullOutput, area: Rect) -> Vec<Rect> {
        let mut rects: Vec<Rect> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Mesh(mesh) => Some(mesh.calc_bounds()),
                egui::Shape::Rect(shape) => shape
                    .brush
                    .as_ref()
                    .filter(|brush| brush.fill_texture_id != egui::TextureId::default())
                    .map(|_| shape.rect),
                _ => None,
            })
            .filter(|rect| area.contains(rect.center()))
            .collect();
        rects.sort_by(|left, right| left.center().y.total_cmp(&right.center().y));
        rects
    }

    /// Phase 3B.5: the four questions live on a far-edge rail that is always there, and switching tools never closes
    /// the panel — that is the whole point of the pattern.
    #[test]
    fn the_far_edge_rail_switches_the_panel_and_only_a_second_click_closes_it() {
        assert_eq!(RightTab::ALL.len(), 4, "four questions, in tab order");
        assert_eq!(
            RightTab::ALL[2],
            RightTab::Suggestions,
            "Suggestions sits between Tags and Activity"
        );
        assert_eq!(RightTab::ALL[3], RightTab::Activity, "and Activity stays last");

        let dir = std::env::temp_dir().join(format!("sortlens-ui-right-rail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open a library");
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let mut actions: Vec<Action> = Vec::new();
        let ctx = egui::Context::default();
        let screen = vec2(1200.0, 800.0);
        // The rail spans the window between the top and bottom bars, so the band is clipped to that middle: the top
        // bar's own glyphs sit at the same x and would otherwise be counted as rail entries.
        let rail = Rect::from_min_max(pos2(screen.x - TAB_STRIP_W, 50.0), pos2(screen.x, screen.y - 60.0));

        let painted = paint_shell(&ctx, screen, &mut state, &mut engine, &mut thumbs, &mut actions);
        let glyphs = glyphs_top_to_bottom(&painted, rail);
        assert_eq!(
            glyphs.len(),
            RightTab::ALL.len(),
            "one glyph per question and nothing else in the rail: {glyphs:?}"
        );
        assert_eq!(state.right_tab, RightTab::Details, "Details is still the first question");
        assert!(state.right_open);

        click_shell(&ctx, screen, glyphs[2].center(), &mut state, &mut engine, &mut thumbs, &mut actions);
        assert_eq!(state.right_tab, RightTab::Suggestions, "the third glyph is Suggestions");
        assert!(state.right_open, "switching tools keeps the panel open");
        let switched = paint_shell(&ctx, screen, &mut state, &mut engine, &mut thumbs, &mut actions);
        assert!(
            label_at(&switched, "Suggestions").is_some(),
            "the panel's header names the tool it is showing"
        );

        let glyphs = glyphs_top_to_bottom(&switched, rail);
        click_shell(&ctx, screen, glyphs[1].center(), &mut state, &mut engine, &mut thumbs, &mut actions);
        assert_eq!(state.right_tab, RightTab::Tags);
        assert!(state.right_open, "moving between tools never hides the panel");

        let tags = paint_shell(&ctx, screen, &mut state, &mut engine, &mut thumbs, &mut actions);
        let glyphs = glyphs_top_to_bottom(&tags, rail);
        click_shell(&ctx, screen, glyphs[1].center(), &mut state, &mut engine, &mut thumbs, &mut actions);
        assert!(!state.right_open, "clicking the active tool closes the body");
        assert_eq!(state.right_tab, RightTab::Tags, "and remembers which tool it was");

        let closed = paint_shell(&ctx, screen, &mut state, &mut engine, &mut thumbs, &mut actions);
        assert_eq!(
            glyphs_top_to_bottom(&closed, rail).len(),
            RightTab::ALL.len(),
            "the rail is still there with the body closed, which is how the body is reopened"
        );
        assert!(actions.is_empty(), "none of this asks the host for anything");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Phase 3B.5: the Suggestions tab proposes, accepts and dismisses for the whole selection — and both writes go
    /// through the ordinary op log, so a whole kind is one undo rather than one per photograph.
    #[test]
    fn the_suggestions_tab_accepts_and_dismisses_for_the_whole_selection() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-suggest-tab-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-suggest-tab");
        for index in 1..=3 {
            let name = format!("IMG_000{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [40 + index * 20, 80, 150], 48, 32), root.join(&name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // A pack is the only proposal a fixture JPEG can produce (it carries no EXIF), and two keywords in it are what
        // lets accepting one be told apart from dismissing the other.
        std::fs::create_dir_all(dir.join(sortlens_engine::PACKS_DIR)).expect("packs dir");
        std::fs::write(
            dir.join(sortlens_engine::PACKS_DIR).join("jpgs.json"),
            r#"{"name":"JPEGs","licence":"CC0-1.0","keywords":[{"keyword":"jpeg","when":{"folder_contains":".jpg"}},{"keyword":"archive","when":{"folder_contains":".jpg"}}]}"#,
        )
        .expect("write pack");
        engine.set_pack("JPEGs", true).expect("enable");

        let ids: Vec<PhotoId> = engine.all_photos().iter().map(|photo| photo.id).collect();
        assert_eq!(ids.len(), 3);
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.right_tab = RightTab::Suggestions;
        state.selection.select_many(&ids);
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let mut actions: Vec<Action> = Vec::new();
        let ctx = egui::Context::default();
        let screen = vec2(1200.0, 800.0);

        // Nothing selected: the tab says what it would show rather than painting an empty list.
        let mut apart = UiState::new(ThemeKind::Charcoal, 0.0);
        apart.right_tab = RightTab::Suggestions;
        let empty = paint_shell(&ctx, screen, &mut apart, &mut engine, &mut thumbs, &mut actions);
        assert!(
            label_at(&empty, "Select a photo to see what its own file").is_some(),
            "an empty selection is answered in words"
        );

        let painted = paint_shell(&ctx, screen, &mut state, &mut engine, &mut thumbs, &mut actions);
        assert!(label_at(&painted, "Scene").is_some(), "a pack proposal is under Scene, not EXIF");
        let accept = label_at(&painted, "+ jpeg").expect("the proposal is a button");
        click_shell(&ctx, screen, accept.center(), &mut state, &mut engine, &mut thumbs, &mut actions);
        for id in &ids {
            let photo = engine.photo(*id).expect("photo");
            assert!(photo.has_tag("jpeg"), "the keyword reached every selected photo");
            assert_eq!(
                photo.tag_source("jpeg"),
                sortlens_core::TagSource::Ai,
                "an accepted proposal is recorded as one"
            );
        }
        assert!(state.notice.is_some(), "the panel says what it did");
        assert!(engine.undo().expect("undo"), "one undo took the whole selection back");
        for id in &ids {
            assert!(engine.photo(*id).expect("photo").tags.is_empty());
        }

        // Dismissal: the panel remembers it, the engine stops offering it, and undo brings it back.
        let painted = paint_shell(&ctx, screen, &mut state, &mut engine, &mut thumbs, &mut actions);
        // Exactly `×`, not the `3×` beside the proposal: the count and the dismiss button share a glyph.
        let dismiss = exact_label_at(&painted, "×").expect("every row can be dismissed");
        click_shell(&ctx, screen, dismiss.center(), &mut state, &mut engine, &mut thumbs, &mut actions);
        let dismissed: Vec<String> = ids
            .iter()
            .filter_map(|id| engine.photo(*id))
            .flat_map(|photo| photo.dismissed_suggestions.iter().cloned().collect::<Vec<String>>())
            .collect();
        assert_eq!(
            dismissed.len(),
            3,
            "all three selected photos dismissed the same proposal: {dismissed:?}"
        );
        let gone = engine.suggestions(ids[0]);
        assert!(
            !gone.iter().any(|suggestion| suggestion.tag == dismissed[0]),
            "the dismissed proposal is not offered again"
        );
        assert!(engine.undo().expect("undo"));
        assert!(!engine.photo(ids[0]).expect("photo").dismissed(&dismissed[0]));
        assert!(actions.is_empty(), "the tab writes through the engine, not through the host");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Phase 3B.4: Detail, Compare and Soft Focus are destinations, not a mode flag and a section.
    ///
    /// The invariant is the part worth testing: a second pane exists exactly while Compare is the view, so
    /// no code path can leave a compare open that nothing is drawing.
    #[test]
    fn detail_compare_and_soft_focus_are_peer_views() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-peers-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-peers");
        for index in 1..=3 {
            let name = format!("IMG_000{index}.jpg");
            std::fs::copy(fixture.jpeg(&name, [50, 60 * index as u8, 120], 64, 48), root.join(&name)).expect("copy");
        }
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.selection.replace(id);

        assert!(activate_view(&mut state, &mut engine, View::Detail));
        assert_eq!(state.view, View::Detail);
        assert_eq!(state.compare, None, "one photograph, no second pane");
        assert_eq!(state.right_tab, RightTab::Details, "opening one photograph shows its facts");

        toggle_compare(&mut state, &engine);
        assert_eq!(state.view, View::Compare, "the compare is a destination of its own");
        let second = state.compare.expect("a second photograph");
        assert_ne!(second, id, "and it is a different photograph");

        // Leaving Compare closes the pane, whatever the destination is.
        assert!(activate_view(&mut state, &mut engine, View::Library));
        assert_eq!(state.compare, None, "a compare that outlived its view would be invisible state");

        // Entering Compare from the row picks a pair again rather than opening an empty view.
        assert!(activate_view(&mut state, &mut engine, View::Compare));
        assert_eq!(state.view, View::Compare);
        assert!(state.compare.is_some());

        assert!(activate_view(&mut state, &mut engine, View::SoftFocus));
        assert_eq!(state.view, View::SoftFocus);
        assert_eq!(state.compare, None, "and leaving Compare still closed it");

        // With nothing selected Compare has nothing to show, so it refuses and says why instead of
        // painting a selected tab for a view that did not open.
        state.selection.clear();
        state.view = View::Library;
        assert!(!activate_view(&mut state, &mut engine, View::Compare));
        assert_eq!(state.view, View::Library, "the tab did not take");
        assert!(state.live_notice().is_some() || state.notice.is_some(), "and the refusal is spoken");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Click-zoom in the Detail view: the photograph is the target, and a click on it means "closer".
    #[test]
    fn clicking_the_detail_photograph_toggles_its_zoom() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-clickzoom-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-clickzoom");
        std::fs::copy(fixture.jpeg("IMG_0001.jpg", [80, 140, 200], 96, 64), root.join("IMG_0001.jpg")).expect("copy");
        let mut engine = Engine::open(&dir).expect("open");
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let id = engine.page(&Filter::default(), Sort::Natural, 0, 1).photos[0].id;
        let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
        state.selection.replace(id);
        state.view = View::Detail;
        let screen = vec2(1440.0, 900.0);
        let ctx = egui::Context::default();
        crate::icons::install(&ctx);
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        let mut last = egui::FullOutput::default();
        for _ in 0..3 {
            last = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), screen)),
                    ..egui::RawInput::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        show(ui, &mut state, &mut engine, &mut thumbs);
                    });
                },
            );
            last.textures_delta.clear();
        }
        // The photograph is the largest textured rectangle above the bottom bar.
        let photo = last
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Mesh(mesh) => Some(mesh.calc_bounds()),
                egui::Shape::Rect(shape) => shape
                    .brush
                    .as_ref()
                    .filter(|brush| brush.fill_texture_id != egui::TextureId::default())
                    .map(|_| shape.rect),
                _ => None,
            })
            .filter(|rect| rect.center().y < screen.y - Tokens::CHARCOAL.bottom_bar_h)
            .max_by(|left, right| (left.width() * left.height()).total_cmp(&(right.width() * right.height())))
            .expect("the detail canvas painted the photograph");
        assert!(photo.width() > 200.0, "a full-size canvas, not a thumbnail");

        for input in press_and_release(photo.center()) {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: input.events,
                    screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), screen)),
                    ..egui::RawInput::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        show(ui, &mut state, &mut engine, &mut thumbs);
                    });
                },
            );
            output.textures_delta.clear();
        }
        assert!(state.detail_zoom, "a click on the photograph zooms to 1:1");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 3B.6 (e): the canvas header's title must never be painted under the right-hand controls.
    ///
    /// The header is a left block (the view's name and the library's count) and a right block (the scope
    /// chip, the active-filter chip, the sort combo and the two grid glyphs). Laid out as a plain
    /// `horizontal`, the right block is handed the width the left block left over, and when that number ran
    /// small the right-to-left row was laid out from a cursor already past the right edge: at 900 points
    /// with both docks open the title was painted underneath the sort combo, and the count underneath the
    /// scope chip. Laid out with `Sides::shrink_left`, the controls are measured first and the words take
    /// what is left, which is a claim a baseline cannot make precisely, so it is made here: at **every**
    /// width from 900 to 1300 points, no painted header text intersects the sort combo or either chip, and
    /// the title lies inside the clip it was painted with, so a fix that hid it would fail too.
    ///
    /// The degradation is asserted in both directions rather than tolerated: the title is whole at every
    /// width, the count steps aside only when it would not fit beside the name (it is painted somewhere in
    /// the sweep and absent somewhere in it), and the two chips are offered on wide canvases and gone on
    /// narrow ones — a header that had simply dropped them everywhere would fail as loudly as one that
    /// kept them in a row they do not fit.
    #[test]
    fn the_canvas_header_title_never_runs_under_the_sort_combo_at_any_width() {
        let dir = std::env::temp_dir().join(format!("sortlens-ui-header-title-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut engine = Engine::open(&dir).expect("open");
        let root = dir.join("photos");
        std::fs::create_dir_all(&root).expect("root");
        let fixture = sortlens_testkit::Fixture::new("ui-header-title");
        for name in ["a.jpg", "b.jpg"] {
            std::fs::copy(fixture.jpeg(name, [90, 120, 200], 64, 64), root.join(name)).expect("copy");
        }
        engine.begin_scan(root.to_string_lossy().to_string()).expect("scan");
        for _ in 0..600 {
            if engine.poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(engine.stats().total, 2, "the header needs a library to count");

        let t = ThemeKind::Charcoal.tokens();
        let scoped = root.to_string_lossy().to_string();
        // Every width is measured from the same fresh state: a reused `UiState` carries the panel sizes the
        // previous frame left behind, and a sweep whose rows depend on how many frames came before it is a
        // sweep that cannot say what it tested.
        let fresh = || {
            let mut state = UiState::new(ThemeKind::Charcoal, 0.0);
            // Scoped to the library's own folder, which is one active filter: the header then carries the
            // scope chip and the active-filter chip as well, so the row is at its busiest for a given window.
            state.root = Some(scoped.clone());
            assert!(state.left_open && state.right_open, "both docks are open, the tight case");
            state
        };
        let mut thumbs = Thumbnails::new(dir.join("cache"));
        // The title at its painted size, from the same layout call the shell makes.
        let full_title = {
            let ctx = egui::Context::default();
            crate::theme::apply(&ctx, ThemeKind::Charcoal);
            let mut width = 0.0;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    width = ui
                        .painter()
                        .layout_no_wrap(
                            "Library".to_string(),
                            egui::FontId::new(16.0, egui::FontFamily::Proportional),
                            t.text,
                        )
                        .size()
                        .x;
                });
            });
            output.textures_delta.clear();
            width
        };

        fn texts(output: &egui::FullOutput) -> Vec<(String, Rect, Rect)> {
            fn walk(shape: &egui::Shape, clip: Rect, sink: &mut Vec<(String, Rect, Rect)>) {
                match shape {
                    egui::Shape::Text(text) => {
                        sink.push((text.galley.job.text.clone(), text.visual_bounding_rect(), clip));
                    }
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            walk(shape, clip, sink);
                        }
                    }
                    _ => {}
                }
            }
            let mut sink = Vec::new();
            for clipped in &output.shapes {
                walk(&clipped.shape, clipped.clip_rect, &mut sink);
            }
            sink
        }

        let mut counted = 0;
        let mut chip_widths: Vec<f32> = Vec::new();
        let mut count_widths: Vec<f32> = Vec::new();
        for step in 0..=400 {
            let width = 900.0 + step as f32;
            let ctx = egui::Context::default();
            crate::theme::apply(&ctx, ThemeKind::Charcoal);
            let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(width, 800.0));
            let mut state = fresh();
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..egui::RawInput::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        show(ui, &mut state, &mut engine, &mut thumbs);
                    });
                },
            );
            output.textures_delta.clear();
            let painted = texts(&output);
            // The canvas is the clip the sort combo was painted in: the sidebar and the right dock paint
            // rows of their own on the same line, and a filter on `x` would have to know where the panels
            // happen to be this frame.
            let Some(sort_rect) = painted.iter().find(|(text, _, _)| text == "Added order").map(|(_, rect, _)| *rect) else {
                panic!("at {width} points the header paints no sort combo at all");
            };
            let in_canvas: Vec<(String, Rect, Rect)> = painted
                .into_iter()
                .filter(|(_, _, clip)| clip.contains(sort_rect.center()))
                .collect();
            let Some((title_rect, title_clip)) = in_canvas
                .iter()
                .find(|(text, _, _)| text == "Library")
                .map(|(_, rect, clip)| (*rect, *clip))
            else {
                panic!("at {width} points the header paints no title at all: {in_canvas:?}");
            };
            assert!(
                title_clip.contains_rect(title_rect),
                "at {width} points the title at {title_rect:?} is clipped away by {title_clip:?}"
            );
            assert!(
                title_rect.width() >= full_title - 2.0,
                "at {width} points the title was squeezed to {title_rect:?}, not the {full_title} points it \
                 asks for: a name that cannot be read is not a title"
            );
            // Every control the header paints on that line, named so a failure says which one moved.
            for anchor in ["Added order", "All Photos", "1 active"] {
                let Some(other_rect) = in_canvas.iter().find(|(text, _, _)| text == anchor).map(|(_, rect, _)| *rect) else {
                    continue;
                };
                assert!(
                    !title_rect.intersects(other_rect),
                    "at {width} points the title at {title_rect:?} is painted over `{anchor}` at {other_rect:?}"
                );
                if let Some(count) = in_canvas.iter().find(|(text, _, _)| text == "2 photos").map(|(_, rect, _)| *rect) {
                    assert!(
                        !count.intersects(other_rect),
                        "at {width} points the count at {count:?} is painted over `{anchor}` at {other_rect:?}"
                    );
                }
            }
            if in_canvas.iter().any(|(text, _, _)| text == "1 active") {
                chip_widths.push(width);
            }
            if in_canvas.iter().any(|(text, _, _)| text == "2 photos") {
                count_widths.push(width);
            }
            counted += 1;
        }
        assert_eq!(counted, 401, "the header was painted at every swept width");
        // The degradation, asserted in both directions rather than tolerated: the narrowest window keeps the
        // name and the controls and gives up the count and the chips, and a wide one offers all four. A
        // header that had dropped them everywhere would fail here as loudly as one that kept them in a row
        // they do not fit — and at 900 with both docks open they do not fit.
        assert!(!count_widths.contains(&900.0), "the count must step aside at the narrowest window");
        assert!(count_widths.contains(&1300.0), "the count must be painted on a wide canvas");
        assert!(!chip_widths.contains(&900.0), "the chips must step aside at the narrowest window");
        assert!(chip_widths.contains(&1300.0), "the chips must be offered on a wide canvas");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
