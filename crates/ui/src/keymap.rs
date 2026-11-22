//! The keymap: one table listing every binding the shell implements.
//!
//! Bindings used to live only in the `if key_pressed(...)` calls scattered through the shell, which
//! meant the only way to learn a shortcut was to read the source. This table is what the overlay
//! draws, and it is cross-checked against the command registry so a renamed shortcut cannot leave
//! the help wrong: a binding that names a command must agree with that command's declared shortcut.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Where a binding applies. The shell is modal in a few places, and saying so is most of the help.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Context {
    /// The library grid and its sidebar, when no dialog is open.
    Library,
    /// The full-window photo view.
    Detail,
    /// An open cull session, where P/X stage a decision instead of writing a flag.
    Cull,
    /// A side-by-side compare: the pair moves together.
    Compare,
    /// Anywhere, including while a dialog is open.
    Any,
}

impl Context {
    /// Section heading in the overlay.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Detail => "Detail view",
            Self::Cull => "Cull session",
            Self::Compare => "Compare",
            Self::Any => "Anywhere",
        }
    }

    /// One line describing the mode, printed under the heading.
    pub const fn note(self) -> &'static str {
        match self {
            Self::Library => "Nothing focused, no dialog open.",
            Self::Detail => "D from the grid; Esc closes it.",
            Self::Cull => "C starts a session from the current filters.",
            Self::Compare => "C in the detail view; S swaps sides.",
            Self::Any => "Also works with a dialog open.",
        }
    }

    /// Every context, in overlay order.
    pub const ALL: [Self; 5] = [Self::Any, Self::Library, Self::Detail, Self::Cull, Self::Compare];
}

/// One row of the keymap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    /// Keys as they are written on a keycap, e.g. `Ctrl+Shift+R` or `0–5`.
    pub keys: &'static str,
    /// What pressing them does, in the imperative.
    pub action: &'static str,
    /// Which mode it applies in.
    pub context: Context,
    /// The command this binding runs, when it runs one.
    pub command: Option<&'static str>,
}

impl Binding {
    /// Whether the row matches a search query, matching keys or action text.
    pub fn matches(&self, query: &str) -> bool {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }
        self.keys.to_lowercase().contains(&needle)
            || self.action.to_lowercase().contains(&needle)
            || self.context.label().to_lowercase().contains(&needle)
    }
}

/// Every binding the shell implements, in reading order within each context.
pub const BINDINGS: &[Binding] = &[
    // Anywhere.
    binding("?", "Show every shortcut", Context::Any, Some("app.shortcuts")),
    binding("Esc", "Close the dialog, overlay or detail view", Context::Any, None),
    binding("Ctrl+O", "Add a folder to the library", Context::Any, Some("library.addFolder")),
    binding(
        "Ctrl+Shift+R",
        "Rescan the library for changes",
        Context::Any,
        Some("library.rescan"),
    ),
    binding("Ctrl+Shift+N", "New collection", Context::Any, Some("collection.new")),
    binding("Ctrl+,", "Open settings", Context::Any, Some("app.settings")),
    binding("Ctrl+Q", "Quit, asking about unsaved work", Context::Any, Some("app.quit")),
    binding("Ctrl+Z", "Undo the last change", Context::Any, Some("edit.undo")),
    binding("P", "Flag as picked", Context::Library, Some("photo.flag")),
    binding("X", "Flag as rejected", Context::Library, Some("photo.flag")),
    binding("U", "Clear the flag", Context::Library, None),
    binding("0–5", "Set the star rating, 0 clearing it", Context::Library, Some("photo.rate")),
    binding(
        "6–9",
        "Colour label: red, yellow, green, blue; again to clear",
        Context::Library,
        Some("photo.label"),
    ),
    binding("K", "Jump to the keyword field", Context::Library, Some("photo.keyword")),
    binding("D", "Open the detail view", Context::Library, Some("view.detail")),
    binding("C", "Start a cull session from the filters", Context::Library, Some("cull.start")),
    binding("I", "Open insights", Context::Library, Some("view.insights")),
    binding("S", "Open the similar-photo search", Context::Library, Some("view.similar")),
    // Detail view.
    binding("→ / Space", "Next photo in the sorted order", Context::Detail, None),
    binding("←", "Previous photo in the sorted order", Context::Detail, None),
    binding("Z", "Zoom to fit or 1:1", Context::Detail, None),
    binding("C", "Compare with the next photo", Context::Detail, None),
    binding("D / Esc", "Close the detail view", Context::Detail, None),
    // Cull session.
    binding("P", "Keep this photo", Context::Cull, None),
    binding("X", "Reject this photo", Context::Cull, None),
    binding("Space", "Leave it undecided and move on", Context::Cull, None),
    binding("U", "Mark undecided again", Context::Cull, None),
    binding("←", "Step back one photo", Context::Cull, Some("cull.back")),
    binding("Ctrl+Enter", "Apply the staged rejections", Context::Cull, Some("cull.apply")),
    binding("Z", "Zoom to fit or 2×", Context::Cull, None),
    binding("Alt+drag", "Pan while zoomed", Context::Cull, None),
    // Compare.
    binding("C", "Close the compare", Context::Compare, None),
    binding("S", "Swap the two sides", Context::Compare, None),
    binding("→ / ←", "Step the pair together", Context::Compare, None),
];

/// Shorthand for the table above; `const fn` so `BINDINGS` stays a plain constant.
const fn binding(keys: &'static str, action: &'static str, context: Context, command: Option<&'static str>) -> Binding {
    Binding {
        keys,
        action,
        context,
        command,
    }
}

/// Bindings matching a search query, in table order.
pub fn search(query: &str) -> Vec<&'static Binding> {
    BINDINGS.iter().filter(|binding| binding.matches(query)).collect()
}

/// Bindings for one context, in table order.
pub fn in_context(context: Context) -> Vec<&'static Binding> {
    BINDINGS.iter().filter(|binding| binding.context == context).collect()
}

/// How many bindings the overlay lists, for the header and the tests.
pub fn len() -> usize {
    BINDINGS.len()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn every_row_is_filled_in() {
        assert!(len() >= 25, "the keymap should cover the shell: {}", len());
        for binding in BINDINGS {
            assert!(!binding.keys.trim().is_empty());
            assert!(!binding.action.trim().is_empty(), "{} has no action", binding.keys);
            // Actions read as instructions, not as labels.
            let first = binding.action.chars().next().expect("action");
            assert!(first.is_uppercase(), "{}: {}", binding.keys, binding.action);
        }
        // The overlay must be reachable from inside the overlay.
        assert!(BINDINGS.iter().any(|binding| binding.keys == "?"));
        for context in Context::ALL {
            assert!(!in_context(context).is_empty(), "{} is empty", context.label());
        }
    }

    #[test]
    fn search_matches_keys_and_words_in_either_case() {
        let zoom = search("zoom");
        assert!(zoom.iter().any(|binding| binding.keys == "Z"), "{zoom:?}");
        assert_eq!(search("ZOOM").len(), zoom.len());
        let shifted = search("ctrl+shift");
        assert_eq!(shifted.len(), 2, "rescan and new collection: {shifted:?}");
        assert_eq!(search("   ").len(), len(), "a blank query shows everything");
        assert!(search("nothing-is-bound-to-this").is_empty());
    }

    #[test]
    fn a_binding_that_names_a_command_agrees_with_the_registry() {
        let registry = sortlens_engine::command::Registry::v0();
        for binding in BINDINGS {
            let Some(id) = binding.command else {
                continue;
            };
            let command = registry
                .all()
                .iter()
                .find(|command| command.id == id)
                .unwrap_or_else(|| panic!("{id} is not in the command registry"));
            assert!(
                command.is_ready(),
                "{id} is {} but the keymap presents it as working",
                command.milestone().unwrap_or("not ready")
            );
            let declared = command.shortcut.unwrap_or_default();
            assert!(
                declared.split('/').any(|part| part.trim() == binding.keys),
                "{id}: the overlay says {} but the command says {declared}",
                binding.keys
            );
        }
    }

    #[test]
    fn every_ready_command_shortcut_is_documented_here() {
        let registry = sortlens_engine::command::Registry::v0();
        let documented: Vec<&str> = BINDINGS.iter().map(|binding| binding.keys).collect();
        for command in registry.ready() {
            let Some(shortcut) = command.shortcut else {
                continue;
            };
            for part in shortcut.split('/') {
                let keys = part.trim();
                assert!(
                    documented.contains(&keys),
                    "{} ({}) is bound to {keys} but the overlay does not list it",
                    command.id,
                    command.label
                );
            }
        }
    }

    #[test]
    fn two_bindings_in_one_context_never_share_a_key() {
        for context in Context::ALL {
            let mut keys: Vec<&str> = in_context(context).iter().map(|binding| binding.keys).collect();
            let count = keys.len();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(count, keys.len(), "{} repeats a key", context.label());
        }
    }
}
