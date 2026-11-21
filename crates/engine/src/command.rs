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
                    shortcut: Some("6–0"),
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
                    shortcut: Some("Left"),
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
                    id: "cull.undo",
                    label: "Undo",
                    group: Group::Review,
                    shortcut: Some("Ctrl+Z"),
                    menu: "Review > Undo",
                    status: Status::Planned("M4"),
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
