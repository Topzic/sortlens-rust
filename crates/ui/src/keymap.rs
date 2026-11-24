//! The keymap: one table listing every binding the shell implements.
//!
//! Bindings used to live only in the `if key_pressed(...)` calls scattered through the shell, which
//! meant the only way to learn a shortcut was to read the source. This table is what the overlay
//! draws, and it is cross-checked against the command registry so a renamed shortcut cannot leave
//! the help wrong: a binding that names a command must agree with that command's declared shortcut.
//!
//! Arrow keys are spelled with the triangles `◀`/`▶` rather than `←`/`→`: the keycap column paints
//! in the monospace face, which has both, but the palette and the hint lines paint the same strings
//! in the proportional face, which has neither — so one spelling has to render in both. The test
//! at the foot of this file is the guard, not this comment.
//!
//! ## The runtime half
//!
//! [`BINDINGS`] is the documentation; [`Keymap`] is the machine. Every table row that names a
//! command and a single chord becomes a rebindable [`craft_keymap::Bindable`] (SL-I09), and the
//! rows that cannot — ranges like `0–5`, the mouse rows, `Esc` and the arrows — stay hardcoded in
//! the shell and are shown read-only in the editor as [`craft_keymap::ui::FixedRow`]s, so a
//! rebinding can never promise what dispatch will not do. The two flag rows bind one registry
//! command with parameters, which is what [`catalog_ref`] spells. The shell reads its keys
//! through [`plain_pressed`] and [`consume`], the palette and the overlay paint the current
//! chords through [`painted`] and [`row_keys`], and `craft_keymap`'s own dialog owns rebinding,
//! conflict detection and set import/export.

#![forbid(unsafe_code)]

use craft_keymap::{Bindable, Binding as CraftBinding, Chord, CommandRef, Key, Keymap, Overrides, Platform, Scope, Scopes, Trigger};
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
    binding("Ctrl+K", "Open the command palette, or close it", Context::Any, None),
    binding("Ctrl+,", "Open settings", Context::Any, Some("app.settings")),
    binding("Ctrl+Q", "Quit, asking about unsaved work", Context::Any, Some("app.quit")),
    binding("Ctrl+Z", "Undo the last change", Context::Any, Some("edit.undo")),
    binding("Ctrl+Shift+Z", "Redo the last undone change", Context::Any, Some("edit.redo")),
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
    binding(
        "T",
        "Apply the keyword you last added to the selection",
        Context::Library,
        Some("photo.tagFromPrev"),
    ),
    binding(
        "Click",
        "Select one photo; ⌘-click adds or removes, Shift-click takes the run",
        Context::Library,
        None,
    ),
    binding("Ctrl+A", "Select every photo in view", Context::Library, None),
    binding("Drag", "Marquee-select from the grid background", Context::Library, None),
    binding("G", "Grid layout: justified rows or a square lattice", Context::Library, None),
    binding("D", "Open the detail view", Context::Library, Some("view.detail")),
    binding("C", "Start a cull session from the filters", Context::Library, Some("cull.start")),
    binding("I", "Open insights", Context::Library, Some("view.insights")),
    binding("M", "Open the map", Context::Library, Some("view.places")),
    binding("S", "Open the similar-photo search", Context::Library, Some("view.similar")),
    // Detail view.
    binding("▶ / Space", "Next photo in the sorted order", Context::Detail, None),
    binding("◀", "Previous photo in the sorted order", Context::Detail, None),
    binding("Z", "Zoom to fit or 1:1", Context::Detail, None),
    binding("C", "Compare with the next photo", Context::Detail, None),
    binding("D / Esc", "Close the detail view", Context::Detail, None),
    // Cull session.
    binding("P", "Keep this photo", Context::Cull, None),
    binding("X", "Reject this photo", Context::Cull, None),
    binding("Space", "Leave it undecided and move on", Context::Cull, None),
    binding("U", "Mark undecided again", Context::Cull, None),
    binding("◀", "Step back one photo", Context::Cull, Some("cull.back")),
    binding(
        "T",
        "Apply the keyword you last added to this photo",
        Context::Cull,
        Some("photo.tagFromPrev"),
    ),
    binding("Ctrl+Enter", "Apply the staged rejections", Context::Cull, Some("cull.apply")),
    binding("Z", "Zoom to fit or 2×", Context::Cull, None),
    binding("Alt+drag", "Pan while zoomed", Context::Cull, None),
    // Compare.
    binding("C", "Close the compare", Context::Compare, None),
    binding("S", "Swap the two sides", Context::Compare, None),
    binding("▶ / ◀", "Step the pair together", Context::Compare, None),
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

// ---- The runtime half: the catalog the editor edits and the dispatch the shell reads ----

/// The one chord a table row binds, when it names a single chord at all.
///
/// The spellings that mean one press parse; the ranges (`0–5`), the mouse rows (`Click`, `Drag`,
/// `Alt+drag`) and the two-key rows (`▶ / Space`, `D / Esc`) do not, and those are exactly the
/// rows that must stay hardcoded: a rebind the shell never reads would be a lie in the editor.
/// The triangle is spelled here because the parser reads `←`/`→` aliases, not `◀`.
fn chord_of(keys: &str) -> Option<Chord> {
    match keys {
        "◀" => Some(Chord::plain(Key::Left)),
        "▶" => Some(Chord::plain(Key::Right)),
        other => Chord::parse(other).ok(),
    }
}

/// The keymap command a table row becomes. The two flag rows run the one registry command with
/// the parameter the registry documents, so the editor offers "Flag as picked" and "Flag as
/// rejected" as two rows rather than one key that means both.
fn catalog_ref(row: &Binding) -> Option<CommandRef> {
    let id = row.command?;
    chord_of(row.keys)?;
    Some(match (id, row.keys) {
        ("photo.flag", "P") => CommandRef::with_params("photo.flag", serde_json::json!({ "flag": "pick" })),
        ("photo.flag", "X") => CommandRef::with_params("photo.flag", serde_json::json!({ "flag": "reject" })),
        (id, _) => CommandRef::new(id),
    })
}

/// Every rebindable row of the table, in reading order, deduplicated by command: the two `T`
/// rows are one command, and the two flag rows are two parameterised commands.
pub fn catalog() -> Vec<Bindable> {
    let mut rows: Vec<Bindable> = Vec::new();
    for row in BINDINGS {
        let Some(reference) = catalog_ref(row) else { continue };
        if rows.iter().any(|entry| entry.command.key() == reference.key()) {
            continue;
        }
        let mut entry = Bindable::new(reference, row.action, Scope::app());
        // The editor groups the table by where it applies; the scope stays app-wide because the
        // shell's own modal gates decide where a key is read, exactly as they did by hand.
        entry = entry.with_path(vec![row.context.label().to_owned()]);
        if let Some(chord) = chord_of(row.keys) {
            entry = entry.with_default(CraftBinding::new(Trigger::single(chord)));
        }
        rows.push(entry);
    }
    rows
}

/// The rows the editor shows read-only: every table row that is not rebindable, in table order.
///
/// `chord` is set only where the fixed key cannot collide with a catalog default (a collision is
/// real — fixed `C` in the compare view really does share its key with the cull — and the
/// conflict bar is not the place to relitigate the factory table).
pub fn fixed_rows() -> Vec<craft_keymap::ui::FixedRow> {
    let rebindable = catalog();
    let mut rows = Vec::new();
    for row in BINDINGS {
        if catalog_ref(row).is_some() {
            continue;
        }
        let chord = chord_of(row.keys).filter(|chord| {
            !rebindable
                .iter()
                .any(|entry| entry.defaults.iter().any(|binding| binding.trigger.chord == *chord))
        });
        rows.push(craft_keymap::ui::FixedRow {
            group: row.context.label().to_owned(),
            label: row.action.to_owned(),
            keys: row.keys.to_owned(),
            chord: chord.map(Trigger::single),
        });
    }
    rows
}

/// The sets the app ships: one, named `Default`, holding the factory bindings and nothing else.
/// A user's first rebinding becomes the `Custom` set the dialog saves.
pub static BUILTINS: &[(&str, Overrides)] = &[("Default", Overrides::new())];

/// The runtime keymap: the catalog under the user's overrides, on this platform.
pub fn build(overrides: Overrides) -> Keymap {
    Keymap::new(catalog(), Scopes::new(), Platform::current()).with_overrides(overrides)
}

/// The keys a command's current bindings spell, in the shell's own style: the triangles for the
/// arrows, the platform's modifier words, joined the way the registry spells two bindings. `None`
/// when the command is not in the catalog or has lost its bindings, so the caller falls back to
/// the registry's declared shortcut.
pub fn painted(keymap: &Keymap, id: &str) -> Option<String> {
    let bindings = keymap.bindings(&CommandRef::new(id));
    if bindings.is_empty() {
        return None;
    }
    let platform = keymap.platform();
    Some(
        bindings
            .iter()
            .map(|binding| spell(&binding.trigger, platform))
            .collect::<Vec<_>>()
            .join(" / "),
    )
}

/// One trigger in keycap spelling: the arrows keep their triangles, everything else reads as the
/// platform spells it (`Ctrl+Shift+R`).
pub fn spell(trigger: &Trigger, platform: Platform) -> String {
    match trigger.chord.key {
        Key::Left => "◀".to_owned(),
        Key::Right => "▶".to_owned(),
        Key::Up => "▲".to_owned(),
        Key::Down => "▼".to_owned(),
        _ => trigger.display(platform),
    }
}

/// The keys an overlay row paints now. A rebindable row paints the command's current bindings, so
/// the help cannot go stale after a rebind; every other row paints its own, fixed spelling.
pub fn row_keys(keymap: &Keymap, row: &Binding) -> String {
    let Some(chord) = chord_of(row.keys) else {
        return row.keys.to_owned();
    };
    let Some(id) = row.command else { return row.keys.to_owned() };
    let entry = keymap
        .catalog()
        .iter()
        .find(|entry| entry.command.id == id && entry.defaults.iter().any(|binding| binding.trigger.chord == chord));
    match entry {
        Some(entry) => painted(keymap, &entry.command.id).unwrap_or_else(|| "-".to_owned()),
        None => row.keys.to_owned(),
    }
}

/// The press test for one event and one binding: a plain binding matches its key with no command
/// modifier held (what `key_pressed` meant, before the keymap could be changed); any other
/// binding reads exactly through `craft_keymap`'s layout.
fn binding_runs(keymap: &Keymap, binding: &CraftBinding, event: &egui::Event) -> bool {
    let egui::Event::Key {
        key,
        pressed: true,
        modifiers,
        ..
    } = event
    else {
        return false;
    };
    let Some(pressed) = Key::from_egui(*key) else { return false };
    let chord = &binding.trigger.chord;
    if chord.mods.is_none() {
        // Plain: the shell's own semantics — the letter fires however Shift or Alt ride along,
        // as long as the command modifier does not.
        !modifiers.command && !modifiers.mac_cmd && chord.key == pressed
    } else {
        craft_keymap::egui::matches(chord, *key, modifiers, keymap.platform())
    }
}

/// Whether this frame's input presses the command under its current plain-key bindings.
///
/// This is the runtime twin of the `key_pressed` calls the shell used to hand-write: a key press
/// with no command modifier held, repeats included, nothing consumed — the focus gates in the
/// shell decide who is typing. A chord the user rebinds with modifiers reads exactly (through the
/// layout), which is how a `Shift+S` rebind keeps its Shift.
pub fn plain_pressed(keymap: &Keymap, input: &egui::InputState, command: &CommandRef) -> bool {
    input
        .events
        .iter()
        .any(|event| keymap.bindings(command).iter().any(|binding| binding_runs(keymap, binding, event)))
}

/// Whether the frame runs the command under its current bindings, reading exactly through
/// `craft_keymap`'s layout and removing the press events that ran, so no widget reads the same
/// press twice — what `consume_shortcut` did, minus egui's "extra Shift and Alt are ignored"
/// rule, which a rebindable keymap cannot afford: a rebound `Ctrl+Alt+O` would otherwise still
/// answer plain `Ctrl+O`.
pub fn consume(keymap: &Keymap, input: &mut egui::InputState, command: &CommandRef) -> bool {
    let mut ran = false;
    let mut taken: Vec<usize> = Vec::new();
    for (index, event) in input.events.iter().enumerate() {
        let matched = keymap
            .bindings(command)
            .into_iter()
            .find(|binding| binding_runs(keymap, binding, event));
        let Some(binding) = matched else { continue };
        ran = true;
        // Only the command chords are taken out of the frame's input, so no widget reads the same
        // press twice; plain letters stay readable, exactly as `key_pressed` left them.
        if !binding.trigger.chord.mods.is_none() {
            taken.push(index);
        }
    }
    for index in taken.into_iter().rev() {
        input.events.remove(index);
    }
    ran
}

/// The two parameterised flag commands, as the catalog spells them, for the shell's dispatch.
pub fn flag_pick() -> CommandRef {
    CommandRef::with_params("photo.flag", serde_json::json!({ "flag": "pick" }))
}

pub fn flag_reject() -> CommandRef {
    CommandRef::with_params("photo.flag", serde_json::json!({ "flag": "reject" }))
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use craft_keymap::{Binding as CraftBinding, CommandRef};

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
        assert_eq!(shifted.len(), 3, "rescan, new collection and redo: {shifted:?}");
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

    /// Every string the overlay and the palette paint exists in the face that paints it.
    ///
    /// Keycaps paint in the monospace face, actions in the proportional one, and the registry's
    /// `shortcut` strings reach the proportional palette even though the overlay spells them in
    /// monospace — which is exactly how `←` survived until a palette row drew a box for it. Both
    /// faces are therefore asserted, per string, against the default stack a build without
    /// craft-fonts runs on.
    #[test]
    fn every_painted_keycap_and_action_exists_in_the_face_that_paints_it() {
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
        let mono = egui::FontId::monospace(10.5);
        let prop = egui::FontId::proportional(10.5);
        let has = |font: &egui::FontId, body: &str| ctx.fonts_mut(|fonts| fonts.has_glyphs(font, body));
        for binding in BINDINGS {
            assert!(has(&mono, binding.keys), "{}: the keycap face lacks a glyph", binding.keys);
            assert!(has(&prop, binding.action), "{}: the action face lacks a glyph", binding.action);
        }
        let registry = sortlens_engine::command::Registry::v0();
        for command in registry.ready() {
            let Some(shortcut) = command.shortcut else { continue };
            assert!(
                has(&prop, shortcut),
                "{} ({}) paints {shortcut} in the palette, which lacks a glyph",
                command.id,
                command.label
            );
        }
    }

    // ---- The runtime half ----

    /// The catalog is exactly the single-chord command rows of the table, under factory bindings,
    /// and every command it carries is a ready command in the registry.
    #[test]
    fn the_catalog_is_the_rebindable_rows_of_the_table() {
        let registry = sortlens_engine::command::Registry::v0();
        let keymap = build(Overrides::new());
        assert!(
            keymap.catalog().len() >= 15,
            "the rebindable commands are in it: {}",
            keymap.catalog().len()
        );
        for entry in keymap.catalog() {
            let spec = registry
                .get(&entry.command.id)
                .unwrap_or_else(|| panic!("{} is not in the command registry", entry.command.id));
            assert!(
                spec.is_ready(),
                "{} is {} but the editor would rebind it",
                entry.command.id,
                spec.milestone().unwrap_or("?")
            );
            assert!(!entry.defaults.is_empty(), "{} binds nothing by default", entry.command.key());
            assert!(
                !entry.paths.is_empty(),
                "{} has no section to sit under in the editor",
                entry.command.key()
            );
        }
        for row in BINDINGS {
            let Some(chord) = chord_of(row.keys) else { continue };
            let Some(id) = row.command else { continue };
            assert!(
                keymap
                    .catalog()
                    .iter()
                    .any(|entry| entry.command.id == id && entry.defaults.iter().any(|binding| binding.trigger.chord == chord)),
                "{} ({id}) is a single chord but not rebindable",
                row.keys
            );
        }
        // The ranges and the mouse rows are not in it: a rebind the shell never reads would lie.
        let rate = BINDINGS.iter().find(|binding| binding.keys == "0–5").expect("rate row");
        assert!(catalog_ref(rate).is_none(), "0–5 is not one chord");
        let click = BINDINGS.iter().find(|binding| binding.keys == "Click").expect("click row");
        assert!(catalog_ref(click).is_none());
    }

    /// The editor's fixed rows are the table's unrebindable rows, and none collides with a
    /// catalog default, so the conflict bar never argues with the factory table.
    #[test]
    fn the_fixed_rows_are_the_rest_of_the_table_and_clash_with_no_catalog_default() {
        let catalog = catalog();
        for row in fixed_rows() {
            assert!(
                BINDINGS
                    .iter()
                    .any(|binding| binding.keys == row.keys && binding.context.label() == row.group),
                "{} is not a table row",
                row.keys
            );
            if let Some(trigger) = row.chord {
                assert!(
                    !catalog
                        .iter()
                        .any(|entry| entry.defaults.iter().any(|binding| binding.trigger == trigger)),
                    "fixed {} sits on a catalog default",
                    row.keys
                );
            }
        }
    }

    /// A rebind moves dispatch: the old key stops answering, the new one starts.
    #[test]
    fn a_rebinding_moves_the_bindings() {
        let mut keymap = build(Overrides::new());
        assert!(
            keymap
                .bindings(&flag_pick())
                .iter()
                .any(|binding| binding.trigger.chord == Chord::plain(Key::P))
        );
        keymap
            .set_bindings(&flag_pick(), vec![CraftBinding::parse("Q").unwrap()], craft_keymap::Policy::Steal)
            .expect("rebind flag pick");
        assert!(
            keymap
                .bindings(&flag_pick())
                .iter()
                .any(|binding| binding.trigger.chord == Chord::plain(Key::Q))
        );
        assert!(
            !keymap
                .bindings(&flag_pick())
                .iter()
                .any(|binding| binding.trigger.chord == Chord::plain(Key::P))
        );
        // And a second key on one command is additive: the editor offers Keep Both, and both fire.
        keymap
            .assign(&flag_pick(), CraftBinding::parse("B").unwrap(), craft_keymap::Policy::Keep)
            .expect("second binding");
        assert_eq!(keymap.bindings(&flag_pick()).len(), 2, "Q and B both flag as picked");
    }

    /// Conflict detection is real: putting `cull.apply` on `Ctrl+Z` leaves two commands holding
    /// one chord, and the editor would say so.
    #[test]
    fn two_commands_on_one_chord_are_conflicts() {
        let mut keymap = build(Overrides::new());
        keymap
            .assign(
                &CommandRef::new("cull.apply"),
                CraftBinding::parse("Ctrl+Z").unwrap(),
                craft_keymap::Policy::Keep,
            )
            .expect("assigned keep");
        assert!(
            keymap.conflicts().iter().any(|conflict| {
                (conflict.a.0.id == "edit.undo" || conflict.b.0.id == "edit.undo")
                    && (conflict.a.0.id == "cull.apply" || conflict.b.0.id == "cull.apply")
            }),
            "Ctrl+Z is now held by undo and apply: {:#?}",
            keymap.conflicts()
        );
    }

    /// The dispatch helpers read what the keymap says: `Ctrl+O` opens, a rebind to `Ctrl+Alt+O`
    /// moves the chord, and the plain flag keys honour their command parameter.
    #[test]
    fn dispatch_reads_the_keymap_not_the_handwritten_table() {
        use craft_keymap::Policy;
        use std::cell::Cell;
        let ctx = egui::Context::default();
        ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
        let press = |key: egui::Key, modifiers: egui::Modifiers| egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..egui::RawInput::default()
        };
        let keymap = build(Overrides::new());
        let ran = Cell::new(false);
        let consumed = |keymap: &Keymap, command: &CommandRef| {
            let hit = &ran;
            let _ = ctx.run_ui(press(egui::Key::O, egui::Modifiers::COMMAND), |ctx| {
                hit.set(ctx.input_mut(|input| consume(keymap, input, command)));
            });
            ran.get()
        };
        assert!(consumed(&keymap, &CommandRef::new("library.addFolder")), "Ctrl+O opens");

        let mut rebound = build(Overrides::new());
        rebound
            .set_bindings(
                &CommandRef::new("library.addFolder"),
                vec![CraftBinding::parse("Ctrl+Alt+O").unwrap()],
                Policy::Steal,
            )
            .expect("rebind addFolder");
        assert!(
            !consumed(&rebound, &CommandRef::new("library.addFolder")),
            "the old chord stops working after the rebind"
        );

        let plain = Cell::new(false);
        let pressed = |keymap: &Keymap, command: &CommandRef, key: egui::Key, modifiers: egui::Modifiers| {
            let hit = &plain;
            let _ = ctx.run_ui(press(key, modifiers), |ctx| {
                hit.set(ctx.input(|input| plain_pressed(keymap, input, command)));
            });
            plain.get()
        };
        // The moved chord answers through `consume` as well.
        let moved_consume = Cell::new(false);
        let _ = ctx.run_ui(press(egui::Key::O, egui::Modifiers::COMMAND.plus(egui::Modifiers::ALT)), |ctx| {
            moved_consume.set(ctx.input_mut(|input| consume(&rebound, input, &CommandRef::new("library.addFolder"))));
        });
        assert!(moved_consume.get(), "the new chord opens");
        assert!(
            pressed(&keymap, &flag_pick(), egui::Key::P, egui::Modifiers::NONE),
            "P is flag as picked"
        );
        assert!(
            !pressed(&keymap, &flag_pick(), egui::Key::P, egui::Modifiers::COMMAND),
            "⌘P is not a flag: the command modifier keeps its own family"
        );
        assert!(
            pressed(&keymap, &flag_reject(), egui::Key::X, egui::Modifiers::NONE),
            "X is flag as rejected"
        );

        // The rebind moves the chord: the old `Ctrl+O` no longer answers anywhere, and the new
        // chord answers exactly, modifiers and all.
        let stale = pressed(
            &rebound,
            &CommandRef::new("library.addFolder"),
            egui::Key::O,
            egui::Modifiers::COMMAND,
        );
        assert!(!stale, "the old chord stops working after the rebind");
        let moved = pressed(
            &rebound,
            &CommandRef::new("library.addFolder"),
            egui::Key::O,
            egui::Modifiers::COMMAND.plus(egui::Modifiers::ALT),
        );
        assert!(moved, "the rebound chord answers through the same helper");
        rebound
            .set_bindings(
                &CommandRef::new("view.similar"),
                vec![CraftBinding::parse("Q").unwrap()],
                Policy::Steal,
            )
            .expect("rebind similar");
        assert!(
            pressed(&rebound, &CommandRef::new("view.similar"), egui::Key::Q, egui::Modifiers::NONE),
            "Q is similar after the rebind"
        );
        assert!(
            !pressed(&rebound, &CommandRef::new("view.similar"), egui::Key::S, egui::Modifiers::NONE),
            "S stopped being similar"
        );
    }

    /// The overlay and the palette paint the current bindings, not the factory ones.
    #[test]
    fn painted_keys_follow_the_rebinds() {
        let mut keymap = build(Overrides::new());
        assert_eq!(painted(&keymap, "library.addFolder").as_deref(), Some("Ctrl+O"));
        assert_eq!(
            painted(&keymap, "view.compare"),
            None,
            "a command outside the catalog falls back to the registry"
        );
        keymap
            .set_bindings(
                &CommandRef::new("library.addFolder"),
                vec![CraftBinding::parse("Ctrl+Alt+O").unwrap()],
                craft_keymap::Policy::Steal,
            )
            .expect("rebind");
        assert_eq!(painted(&keymap, "library.addFolder").as_deref(), Some("Ctrl+Alt+O"));
        // The ◀ row spells the triangle in the runtime text as in the table.
        assert_eq!(painted(&keymap, "cull.back").as_deref(), Some("◀"));
        let open_row = BINDINGS.iter().find(|binding| binding.keys == "Ctrl+O").expect("overlay row");
        assert_eq!(row_keys(&keymap, open_row), "Ctrl+Alt+O");
        let click_row = BINDINGS.iter().find(|binding| binding.keys == "Click").expect("mouse row");
        assert_eq!(row_keys(&keymap, click_row), "Click", "a fixed row paints its own spelling");
    }
}
