//! The command registry.
//!
//! One table drives the menus, the keyboard map, the ⌘K palette, the CLI and the MCP tools.
//! A command is either ready or planned for a named milestone; the UI renders planned commands
//! as disabled with that milestone in the tooltip, so nothing is ever a silent no-op.

use serde::{Deserialize, Serialize};

/// Which part of the app a command belongs to. Used for menu grouping and palette sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Group {
    Library,
    Review,
    Organise,
    View,
    App,
    Help,
}

impl Group {
    /// Section heading.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Review => "Review",
            Self::Organise => "Organise",
            Self::View => "View",
            Self::App => "App",
            Self::Help => "Help",
        }
    }

    /// Groups in menu order.
    pub const ALL: [Self; 6] = [Self::Library, Self::Review, Self::Organise, Self::View, Self::App, Self::Help];
}

/// Whether a command does something today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Implemented and callable.
    Ready,
    /// Designed and named, arriving in the given milestone. Rendered disabled.
    Planned(&'static str),
}

impl Status {
    /// Whether the command can run.
    pub const fn is_ready(self) -> bool {
        matches!(self, Self::Ready)
    }

    /// The milestone that will implement it, if any.
    pub const fn milestone(self) -> Option<&'static str> {
        match self {
            Self::Ready => None,
            Self::Planned(milestone) => Some(milestone),
        }
    }
}

/// A single callable operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    /// Stable dotted id, also the MCP tool name and the CLI subcommand path.
    pub id: &'static str,
    /// Menu and palette label.
    pub label: &'static str,
    /// Menu group.
    pub group: Group,
    /// Default shortcut in egui notation, if the command has one.
    pub shortcut: Option<&'static str>,
    /// Menu path, e.g. `Library > Add Folder…`.
    pub menu: &'static str,
    /// Ready or planned.
    pub status: Status,
}

impl CommandSpec {
    /// Whether the command can run today.
    pub const fn is_ready(&self) -> bool {
        self.status.is_ready()
    }

    /// The milestone that will implement it, if any.
    pub const fn milestone(&self) -> Option<&'static str> {
        self.status.milestone()
    }
}

/// Every command the application knows about, in menu order.
#[derive(Debug, Clone)]
pub struct Registry {
    commands: Vec<CommandSpec>,
}

impl Registry {
    /// The M0 table. Grows one milestone at a time; ids never change once shipped.
    pub fn v0() -> Self {
        Self {
            commands: vec![
                CommandSpec {
                    id: "library.addFolder",
                    label: "Add Folder…",
                    group: Group::Library,
                    shortcut: Some("Ctrl+O"),
                    menu: "Library > Add Folder…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.rescan",
                    label: "Rescan Folder",
                    group: Group::Library,
                    shortcut: Some("Ctrl+Shift+R"),
                    menu: "Library > Rescan Folder",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.import",
                    label: "Import from Card…",
                    group: Group::Library,
                    shortcut: Some("Ctrl+Shift+I"),
                    menu: "Library > Import from Card…",
                    status: Status::Planned("M1"),
                },
                CommandSpec {
                    id: "library.findMissing",
                    label: "Find Missing Files",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Find Missing Files",
                    status: Status::Planned("M7"),
                },
                CommandSpec {
                    id: "library.page",
                    label: "Read a Page",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Read a Page",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "photo.rate",
                    label: "Set Rating",
                    group: Group::Review,
                    shortcut: Some("0–5"),
                    menu: "Review > Set Rating",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "photo.flag",
                    label: "Pick or Reject",
                    group: Group::Review,
                    shortcut: Some("P / X"),
                    menu: "Review > Pick or Reject",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "photo.label",
                    label: "Colour Label",
                    group: Group::Review,
                    shortcut: Some("6–9"),
                    menu: "Review > Colour Label",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "edit.undo",
                    label: "Undo",
                    group: Group::Review,
                    shortcut: Some("Ctrl+Z"),
                    menu: "Review > Undo",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.removeRoot",
                    label: "Remove Folder",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Remove Folder",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.subfolders",
                    label: "Subfolders",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Subfolders",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.health",
                    label: "Check Library",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Check Library",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.relocate",
                    label: "Find Moved Photos",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Find Moved Photos",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.removeMissing",
                    label: "Remove Missing Photos",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Remove Missing Photos",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.deleted",
                    label: "Recently Deleted",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Recently Deleted",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "edit.movePicks",
                    label: "Move Picks to Folder…",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Move Picks to Folder…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "edit.undoMove",
                    label: "Put Moved Photos Back",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Put Moved Photos Back",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "suggest.start",
                    label: "Suggest Keywords for the Library",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Suggest Keywords for the Library",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "suggest.progress",
                    label: "How Far the Suggestions Have Come",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > How Far the Suggestions Have Come",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "suggest.clear",
                    label: "Clear Suggested Keywords",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Clear Suggested Keywords",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "suggest.cancel",
                    label: "Stop Suggesting",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Stop Suggesting",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "suggest.results",
                    label: "Suggested Keywords",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Suggested Keywords",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "packs.list",
                    label: "Keyword Packs",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Keyword Packs",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "packs.set",
                    label: "Switch a Keyword Pack On or Off",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Switch a Keyword Pack On or Off",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "suggest.list",
                    label: "Suggest Keywords",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Suggest Keywords",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "suggest.accept",
                    label: "Accept a Suggested Keyword",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Accept a Suggested Keyword",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "edit.export",
                    label: "Export Picks…",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Export Picks…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "edit.cancelExport",
                    label: "Stop Exporting",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Stop Exporting",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "edit.moves",
                    label: "Recent Moves",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Recent Moves",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "app.diagnostics",
                    label: "Write a Diagnostics Bundle",
                    group: Group::Help,
                    shortcut: None,
                    menu: "Help > Diagnostics Bundle",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.roots",
                    label: "Indexed Folders",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Indexed Folders",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "tag.keywords",
                    label: "Keywords",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Keywords",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "tag.registry",
                    label: "Keyword Registry",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Keyword Registry",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "tag.alias",
                    label: "Link Keyword Alias",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Link Keyword Alias",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "insights.progress",
                    label: "Insights Progress",
                    group: Group::View,
                    shortcut: None,
                    menu: "View > Insights Progress",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "photo.delete",
                    label: "Delete Photos",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Delete Photos",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "photo.sharpness",
                    label: "Measure Sharpness",
                    group: Group::View,
                    shortcut: None,
                    menu: "View > Measure Sharpness",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "photo.sharpnessProgress",
                    label: "Sharpness Progress",
                    group: Group::View,
                    shortcut: None,
                    menu: "View > Sharpness Progress",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "insights.start",
                    label: "Count the Library",
                    group: Group::View,
                    shortcut: None,
                    menu: "View > Count the Library",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.changed",
                    label: "Photos Changed on Disk",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Photos Changed on Disk",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.refreshChanged",
                    label: "Refresh Changed Photos",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Refresh Changed Photos",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.retryMetadata",
                    label: "Retry Unreadable Metadata",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Retry Unreadable Metadata",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.readOnly",
                    label: "Is the Library Read-Only?",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Read-Only",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.history",
                    label: "Recent Changes",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Recent Changes",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.exportHistory",
                    label: "Export Change Report…",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Export Change Report…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.audit",
                    label: "Check Catalog Integrity",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Check Catalog Integrity",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.compact",
                    label: "Compact Catalog",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Compact Catalog",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.remapRoot",
                    label: "Point at a New Folder…",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Point at a New Folder…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.backup",
                    label: "Back Up Library…",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Back Up Library…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.restoreBackup",
                    label: "Restore Backup into Folder…",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Restore Backup into Folder…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.forgetBatch",
                    label: "Forget Batch",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Forget Batch",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.clearDeleted",
                    label: "Empty Rejects Bin",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Empty Rejects Bin",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.restoreBatch",
                    label: "Restore Batch",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Restore Batch",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.jobs",
                    label: "Activity",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Activity",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "library.stats",
                    label: "Library Statistics",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Library Statistics",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "photo.keyword",
                    label: "Add Keyword…",
                    group: Group::Organise,
                    shortcut: Some("K"),
                    menu: "Organise > Add Keyword…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "photo.tagFromPrev",
                    label: "Apply Last Keyword",
                    group: Group::Organise,
                    shortcut: Some("T"),
                    menu: "Organise > Apply Last Keyword",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "folder.mark",
                    label: "Pin or Colour a Folder",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Pin or Colour a Folder",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "folder.marks",
                    label: "Folder Marks",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Folder Marks",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "collection.new",
                    label: "New Collection…",
                    group: Group::Organise,
                    shortcut: Some("Ctrl+Shift+N"),
                    menu: "Organise > New Collection…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "collection.addTo",
                    label: "Add to Collection…",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Add to Collection…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "collection.colour",
                    label: "Set Collection Colour",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Collection Colour",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "collection.favourite",
                    label: "Favourite Collection",
                    group: Group::Library,
                    shortcut: None,
                    menu: "Library > Favourite Collection",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "tag.rename",
                    label: "Rename Keyword…",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Rename Keyword…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "collection.search",
                    label: "Search in Collection",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Search in Collection",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "collection.sort",
                    label: "Sort Collection",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Sort Collection",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "collection.removeFrom",
                    label: "Remove from Collection",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Remove from Collection",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "collection.rename",
                    label: "Rename Collection…",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Rename Collection…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "collection.delete",
                    label: "Delete Collection",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Delete Collection",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "cull.start",
                    label: "Start Cull",
                    group: Group::Review,
                    shortcut: Some("C"),
                    menu: "Review > Start Cull",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "cull.decide",
                    label: "Cull Decision",
                    group: Group::Review,
                    shortcut: None,
                    menu: "Review > Cull Decision",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "cull.back",
                    label: "Previous Photo",
                    group: Group::Review,
                    shortcut: Some("←"),
                    menu: "Review > Previous Photo",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "cull.summary",
                    label: "Cull Progress",
                    group: Group::Review,
                    shortcut: None,
                    menu: "Review > Cull Progress",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "cull.clear",
                    label: "Discard Cull Session",
                    group: Group::Review,
                    shortcut: None,
                    menu: "Review > Discard Cull Session",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "cull.apply",
                    label: "Apply Rejects",
                    group: Group::Review,
                    shortcut: Some("Ctrl+Enter"),
                    menu: "Review > Apply Rejects",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "view.detail",
                    label: "Detail",
                    group: Group::View,
                    shortcut: Some("D"),
                    menu: "View > Detail",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "view.similar",
                    label: "Similar Photos",
                    group: Group::View,
                    shortcut: Some("S"),
                    menu: "View > Similar Photos",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "similar.start",
                    label: "Find Similar Photos",
                    group: Group::View,
                    shortcut: None,
                    menu: "View > Find Similar Photos",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "similar.groups",
                    label: "Similar Groups",
                    group: Group::View,
                    shortcut: None,
                    menu: "View > Similar Groups",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "view.places",
                    label: "Places",
                    group: Group::View,
                    shortcut: Some("M"),
                    menu: "View > Places",
                    status: Status::Planned("M5"),
                },
                CommandSpec {
                    id: "view.insights",
                    label: "Insights",
                    group: Group::View,
                    shortcut: Some("I"),
                    menu: "View > Insights",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "app.settings",
                    label: "Settings…",
                    group: Group::App,
                    shortcut: Some("Ctrl+,"),
                    menu: "Sortlens > Settings…",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "photo.reveal",
                    label: "Show in Folder",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Show in Folder",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "photo.open",
                    label: "Open in Default App",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Open in Default App",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "photo.setCaptured",
                    label: "Set Capture Date",
                    group: Group::Organise,
                    shortcut: None,
                    menu: "Organise > Set Capture Date",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "memories.list",
                    label: "Resurface Memories",
                    group: Group::View,
                    shortcut: None,
                    menu: "View > Resurface Memories",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "app.shortcuts",
                    label: "Keyboard Shortcuts",
                    group: Group::Help,
                    shortcut: Some("?"),
                    menu: "Help > Keyboard Shortcuts",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "app.about",
                    label: "About Sortlens",
                    group: Group::Help,
                    shortcut: None,
                    menu: "Sortlens > About Sortlens",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "app.supportReport",
                    label: "Copy Support Report",
                    group: Group::Help,
                    shortcut: None,
                    menu: "Help > Copy Support Report",
                    status: Status::Ready,
                },
                CommandSpec {
                    id: "app.quit",
                    label: "Quit Sortlens",
                    group: Group::App,
                    shortcut: Some("Ctrl+Q"),
                    menu: "Sortlens > Quit Sortlens",
                    status: Status::Ready,
                },
            ],
        }
    }

    /// Every command, in menu order.
    pub fn all(&self) -> &[CommandSpec] {
        &self.commands
    }

    /// Commands that can run today.
    pub fn ready(&self) -> impl Iterator<Item = &CommandSpec> {
        self.commands.iter().filter(|command| command.is_ready())
    }

    /// Commands in one group.
    pub fn group(&self, group: Group) -> impl Iterator<Item = &CommandSpec> {
        self.commands.iter().filter(move |command| command.group == group)
    }

    /// Look up by id.
    pub fn get(&self, id: &str) -> Option<&CommandSpec> {
        self.commands.iter().find(|command| command.id == id)
    }

    /// Look up by id, for callers that must fail loudly on a typo.
    pub fn expect(&self, id: &str) -> Result<&CommandSpec, crate::Error> {
        self.get(id).ok_or_else(|| crate::Error::Invalid(format!("unknown command: {id}")))
    }

    /// How many commands are implemented.
    pub fn ready_count(&self) -> usize {
        self.ready().count()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::v0()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let registry = Registry::v0();
        let mut ids: Vec<_> = registry.all().iter().map(|command| command.id).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate command id");
    }

    #[test]
    fn every_planned_command_names_a_milestone() {
        for command in Registry::v0().all() {
            if let Some(milestone) = command.milestone() {
                assert!(milestone.starts_with('M'), "{} has a bad milestone", command.id);
            }
        }
    }

    #[test]
    fn menu_paths_are_namespaced() {
        for command in Registry::v0().all() {
            assert!(command.menu.contains(" > "), "{} has no menu path", command.id);
        }
    }

    /// The source of the engine, so the test below can read what `run` actually handles.
    const ENGINE_SOURCE: &str = include_str!("lib.rs");

    /// Every command id that appears as an arm of `Engine::run`.
    ///
    /// The only way to know that a `Ready` command can be called is to look at the other half of the
    /// pair — the registry says what *should* exist and `run` says what *does*. Reading the source in
    /// a test is blunt, but it is the honest audit: it found `library.page` callable with no registry
    /// entry (invisible to the palette and to MCP) and `view.detail`/`view.similar` registered with no
    /// handler at all, which nobody would have found by grepping one file at a time.
    fn handled_in_run() -> Vec<String> {
        let start = ENGINE_SOURCE
            .find("pub fn run(&mut self, command: &str")
            .expect("Engine::run exists");
        let body = &ENGINE_SOURCE[start..];
        let end = body.find("\n    /// ").unwrap_or(body.len());
        let arms = &body[..end];
        let mut ids = Vec::new();
        // Arms look like `"a.b" => { … }`, `"a.b" => Ok(…)`, or `"a.b" | "c.d" => …`.
        for line in arms.lines() {
            let trimmed = line.trim_start();
            if !(trimmed.starts_with('"') || trimmed.starts_with("Self::")) {
                continue;
            }
            if !trimmed.contains("=>") && !trimmed.starts_with('"') {
                continue;
            }
            for part in trimmed.split("=>").next().unwrap_or_default().split('|') {
                let part = part.trim().trim_start_matches("Self::");
                if let Some(rest) = part.strip_prefix('"')
                    && let Some(id) = rest.split('"').next()
                    && id.contains('.')
                {
                    ids.push(id.to_string());
                }
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    #[test]
    fn every_ready_command_has_a_handler_and_every_handler_is_a_command() {
        let registry = Registry::v0();
        let handled = handled_in_run();
        assert!(handled.len() > 30, "the audit found only {} arms", handled.len());

        let missing: Vec<&str> = registry
            .ready()
            .map(|command| command.id)
            .filter(|id| !handled.iter().any(|handled| handled == id))
            .collect();
        assert!(missing.is_empty(), "ready but nothing in Engine::run handles them: {missing:?}");

        let unlisted: Vec<&str> = handled
            .iter()
            .map(String::as_str)
            .filter(|id| !registry.all().iter().any(|command| command.id == *id))
            .collect();
        assert!(
            unlisted.is_empty(),
            "Engine::run handles these, but no command names them, so no menu, palette or tool \
             can reach them: {unlisted:?}"
        );
    }

    #[test]
    fn shortcuts_are_unique_among_ready_commands() {
        let registry = Registry::v0();
        let mut shortcuts: Vec<_> = registry.ready().filter_map(|command| command.shortcut).collect();
        shortcuts.sort_unstable();
        let count = shortcuts.len();
        shortcuts.dedup();
        assert_eq!(shortcuts.len(), count, "two ready commands share a shortcut");
    }
}
