//! Interface strings through craft-i18n: the English text is the lookup key (SL-L07 scaffolding,
//! Phase 4 item 2d).
//!
//! Every routed label is an English literal; a missing translation is the English itself, so a
//! catalogue a translation has not reached yet never breaks the interface. The scaffold ships
//! Arabic, Japanese, Korean, Thai, Vietnamese and Traditional Chinese — all six languages the
//! design names — and the direction
//! plumbing the rest stand on: [`apply_direction`] resolves the interface direction from the
//! chosen language (overridable with `CRAFT_UI_DIRECTION`, the way `craft-ui` checks a mirrored
//! layout before a right-to-left catalogue exists) and installs it on the context, and the
//! shell's panels place themselves at the reading start or end through `craft-ui`'s helpers.
//!
//! The shape follows the suite's own pattern (craft font-manager's panel `i18n` module): one
//! [`I18N`] built once from the embedded catalogues, a thread-local current language, and the
//! `tl!`/`fmt`/`tn` helpers call sites use.

use std::borrow::Cow;
use std::cell::Cell;
use std::sync::LazyLock;

use craft_i18n::{Embedded, FileSize, I18n, Lang};

/// The embedded translation catalogues. The shared column is empty: Sortlens' rows are all its
/// own until they graduate into `craft-i18n`. Missing rows are the English text, never an error.
static CATALOGS: &Embedded = &[
    ("ar", "", include_str!("i18n/ar.tsv")),
    ("ja", "", include_str!("i18n/ja.tsv")),
    ("ko", "", include_str!("i18n/ko.tsv")),
    ("th", "", include_str!("i18n/th.tsv")),
    ("vi", "", include_str!("i18n/vi.tsv")),
    ("zh-TW", "", include_str!("i18n/zh-TW.tsv")),
];

/// The interface languages offered, in picker order: English first, then the embedded ones as
/// `I18n::languages` sorts them.
pub const LANGUAGES: &[&str] = &["en", "ar", "ja", "ko", "th", "vi", "zh-TW"];

/// The catalogues, parsed on each locale's first lookup. Bad rows are logged once, as a warning
/// naming the line, and that row shows in English.
pub static I18N: LazyLock<I18n> = LazyLock::new(|| {
    let (i18n, problems) = I18n::new(CATALOGS, LANGUAGES);
    for problem in problems {
        log::warn!("translation catalog: {problem}; that string shows in English");
    }
    i18n
});

thread_local! {
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::EN) };
}

/// The interface language.
#[must_use]
pub fn current() -> Lang {
    CURRENT.get()
}

/// Switch the interface language. Call [`apply_direction`] afterwards, or let the next frame's
/// call do it: the shell applies the direction every frame, so the panels move as soon as the
/// language changes.
pub fn set_current(lang: Lang) {
    CURRENT.set(lang);
}

/// The language with this code, if the interface offers it (`en`, `ar`, `ja`, `ko`, `th`, `zh-TW`).
#[must_use]
pub fn from_code(code: &str) -> Option<Lang> {
    I18N.from_code(code)
}

/// Translates a literal: the English text is the key.
macro_rules! tl {
    ($s:expr) => {
        $crate::i18n::I18N.t($crate::i18n::current(), $s)
    };
}
pub(crate) use tl;

/// Translates `template` and fills its `{name}` placeholders, with bidi isolates so a value
/// cannot reorder the sentence around itself.
pub fn fmt(template: &str, args: &[(&str, &str)]) -> String {
    craft_i18n::fmt(I18N.t(current(), template), args)
}

/// A count with its noun, `{n}` in the English forms and the region's digits.
pub fn tn(n: u64, one: &str, other: &str) -> String {
    I18N.trn(current(), n, one, other)
}

/// A download size, `1.5 MB`, in the region's format.
#[must_use]
pub fn size(bytes: u64) -> String {
    I18N.file_size(current(), bytes, FileSize::DECIMAL)
}

/// A literal in the interface language, for the host's own words (SL-L07).
///
/// The shell routes its labels through `tl!`; the desktop app builds its notices and the titles of
/// its native file dialogs in its own file, where no `tl!` site paints them and the catalogue audit
/// cannot read them. This is the same English-key lookup those sites use, so one row serves both.
#[must_use]
pub fn tr(text: &str) -> &str {
    I18N.t(current(), text)
}

/// A number in the interface language's own digits and grouping, for a sentence built with
/// [`fmt`].
#[must_use]
pub fn count(n: u64) -> String {
    crate::format::count(n)
}

/// The `Moved` headline in the reader's language, from a count and the folder the files went to.
///
/// The receipt dialog and the host's own notice both say where the files went, and they must not
/// word it differently: one photo and several are two English rows a language may disagree about,
/// so each is a template and the count and folder are its placeholders.
#[must_use]
pub fn moved_headline(count: usize, folder: &str) -> String {
    let digits = self::count(count as u64);
    if count == 1 {
        fmt("{n} photo moved to {folder}", &[("n", &digits), ("folder", folder)])
    } else {
        fmt("{n} photos moved to {folder}", &[("n", &digits), ("folder", folder)])
    }
}

/// The catalogue as `craft-keymap`'s dialog words: its own strings, the command labels and the
/// menu paths look up the rows the rest of the shell paints, so the shortcut editor speaks the
/// interface language without a second table (SL-L07). The language is read when the adapter is
/// made, which is once per frame, after Settings has already changed it.
pub struct CatalogText {
    lang: Lang,
}

impl CatalogText {
    /// An adapter over the current interface language.
    #[must_use]
    pub fn new() -> Self {
        Self { lang: current() }
    }
}

impl Default for CatalogText {
    fn default() -> Self {
        Self::new()
    }
}

impl craft_keymap::ui::Text for CatalogText {
    fn ui<'a>(&'a self, source: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(I18N.t(self.lang, source))
    }

    fn plural(&self, n: u64, one: &str, other: &str) -> String {
        I18N.trn(self.lang, n, one, other)
    }

    fn command<'a>(&'a self, b: &'a craft_keymap::Bindable) -> Cow<'a, str> {
        Cow::Borrowed(I18N.t(self.lang, b.label.as_str()))
    }

    fn segment<'a>(&'a self, s: &'a str) -> Cow<'a, str> {
        Cow::Borrowed(I18N.t(self.lang, s))
    }

    fn rtl(&self) -> bool {
        I18N.rtl(self.lang)
    }
}

/// The interface direction for the chosen language under the app's direction setting, and the
/// same override the suite checks mirrored layouts with (`CRAFT_UI_DIRECTION=rtl|ltr`).
#[must_use]
pub fn resolve_direction() -> craft_ui::direction::Direction {
    craft_ui::direction::resolve(I18N.code(current()), craft_ui::direction::Setting::Language)
}

/// Install the resolved direction on the context: egui's own chrome (submenus, scroll bars,
/// collapsing headers, window title bars) mirrors with it through the suite fork's
/// `Style::right_to_left`. Called every frame — cheap, and it survives a theme change, because
/// installing a whole new `Style` resets the field.
pub fn apply_direction(ctx: &egui::Context) {
    craft_ui::direction::apply(ctx, resolve_direction());
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use std::path::Path;

    use super::*;

    /// The catalogues parse without a single bad row, and every language the picker offers is
    /// known and complete for the strings routed through `tl!` this slice. Missing rows show in
    /// English, so a gap is a listing here rather than a broken screen — but a routed key with no
    /// row is a translation debt the test names, because the route was chosen so it could be
    /// checked.
    #[test]
    fn catalogs_load_and_routed_keys_have_rows() {
        assert!(I18N.check_all().is_empty(), "{:#?}", I18N.check_all());
        // The English source text the routes are called with, read straight out of the module that
        // routes them — the same audit craft font-manager runs on its own panel. Both routes are
        // read: a literal (`tl!`) and a sentence with placeholders (`fmt`), because a template whose
        // row is missing shows English just as quietly as a missing label does. The chart painters
        // live in `charts.rs`, so their two hover templates are read too.
        let sources = [
            include_str!("shell.rs"),
            include_str!("theme.rs"),
            include_str!("sharpness.rs"),
            include_str!("charts.rs"),
            include_str!("a11y.rs"),
            include_str!("places.rs"),
        ];
        let mut keys: Vec<String> = Vec::new();
        for source in sources {
            for marker in ["tl!(", "crate::i18n::fmt("] {
                for part in source.split(marker).skip(1) {
                    if let Some(key) = part.trim_start().strip_prefix('"').and_then(|p| p.find('"').map(|end| &p[..end])) {
                        keys.push(key.to_owned());
                    }
                }
            }
        }
        keys.sort();
        keys.dedup();
        assert!(
            keys.len() > 40,
            "only {} routed keys found; the audit is not reading the sources",
            keys.len()
        );
        for code in ["ar", "ja", "ko", "th", "vi", "zh-TW"] {
            let lang = I18N.match_tag(code).unwrap_or_else(|| panic!("{code} is offered"));
            // Presence, not difference: `Sortlens` is the product's name in every language, and a
            // row that keeps the English word is still a row a translator was handed. What must
            // fail here is a routed key that no catalogue answers at all.
            let missing: Vec<&String> = keys.iter().filter(|k| I18N.get(lang, "", k.as_str()).is_none()).collect();
            assert!(missing.is_empty(), "{code} is missing {} routed rows: {missing:#?}", missing.len());
        }
    }

    /// Every literal the shell and the host app paint, raise as a notice or hand a native dialog
    /// is a catalogue row (SL-L07).
    ///
    /// The sweeps in shell.rs read a fixed list of files, so a file the list never named would
    /// paint English unseen; the audit above reads the routes, so a call that never asked the
    /// catalogues is invisible to it too. This is the whole-source audit the row asks for: it
    /// walks crates/ui/src and apps/sortlens/src from disk, so a file that lands later is read
    /// the day it lands; it cuts each file's #[cfg(test)] modules, because a fixture's English is
    /// not chrome; and it holds three sinks to the six catalogues — the literal a paint call hands
    /// a widget (a bare one, and every format! inside the call, because a painter takes its text
    /// as the third argument), the literal a notice still spells once its tl!/fmt/tr spans are
    /// masked, and the title or filter name a native dialog is given. The product's name is the
    /// one exception, and it is checked for staleness. What no text sweep reads is a literal moved
    /// into a binding first; the trees hold none of those today.
    #[test]
    fn every_painted_literal_in_the_shell_and_the_host_comes_from_the_catalogues() {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root = std::fs::canonicalize(&workspace).unwrap_or(workspace);
        let mut files = 0usize;
        let mut calls = 0usize;
        let mut found: Vec<(String, String)> = Vec::new();
        for tree in ["crates/ui/src", "apps/sortlens/src"] {
            for (name, source) in sources(&root.join(tree)) {
                files += 1;
                let (literals, seen) = swept_literals(&without_test_modules(&source));
                calls += seen;
                found.extend(literals.into_iter().map(|literal| (name.clone(), literal)));
            }
        }
        assert!(files >= 12, "the audit read only {files} sources, so it is not walking both trees");
        assert!(calls >= 800, "the audit read only {calls} paint, notice and dialog calls");
        let mut hand_spelled = Vec::new();
        for (file, literal) in &found {
            if EXCEPTIONS.iter().any(|(exception, _)| exception == literal) {
                continue;
            }
            // A row exists for it or it does not: either way nothing asked the catalogue, so the
            // site paints its own English in every language. This is why the audit asks for the
            // route rather than for the row — a sentence that was un-routed still has its row.
            hand_spelled.push(format!("{file}: {literal:?}"));
        }
        assert!(
            hand_spelled.is_empty(),
            "{} literals the shell or the host spells by hand instead of asking a catalogue: {hand_spelled:#?}",
            hand_spelled.len()
        );
        for (literal, reason) in EXCEPTIONS {
            assert!(
                found.iter().any(|(_, seen)| seen == literal),
                "the audit no longer reads {literal:?}, so its exception ({reason}) is stale"
            );
        }
    }

    /// What may paint beside the catalogues: the product's name, which every catalogue spells the
    /// same. The audit above fails when an entry stops being painted, so it cannot outlive its
    /// site.
    const EXCEPTIONS: &[(&str, &str)] = &[(
        "Sortlens",
        "the product's name, which every catalogue spells the same and a translator is not asked to change",
    )];

    /// The calls that hand a literal to a painted surface, and the two native dialog calls the
    /// host uses for its own titles and file filters.
    const PAINT_CALLS: &[&str] = &[
        "RichText::new(",
        "Label::new(",
        "Button::new(",
        "ui.button(",
        "ui.small_button(",
        "ui.heading(",
        "ui.label(",
        "ui.selectable_label(",
        "ui.selectable_value(",
        "ui.checkbox(",
        "ui.radio_value(",
        "ui.small(",
        "ui.strong(",
        "ui.monospace(",
        "ui.weak(",
        "ui.hyperlink(",
        "ui.link(",
        "ui.menu_button(",
        "ui.collapsing(",
        "ui.text_edit_singleline(",
        "ui.group(",
        "Window::new(",
        "painter().text(",
        "painter.text(",
        ".hover(",
        "on_hover_text(",
        ".tooltip(",
        ".prefix(",
        ".suffix(",
        ".hint_text(",
        ".text(",
    ];

    /// The native dialog's title and its file filter's name.
    const DIALOG_CALLS: &[&str] = &["set_title(", ".add_filter("];

    /// Every .rs file under dir, recursively, as (path, text): sorted so the audit reads the tree
    /// the same way twice, and skipping the AppleDouble ._* sidecars an exFAT checkout leaves
    /// beside the sources, which are not UTF-8 and not source.
    fn sources(dir: &Path) -> Vec<(String, String)> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut paths: Vec<std::path::PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        paths.sort();
        let mut out = Vec::new();
        for path in paths {
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_owned();
            if name.starts_with("._") {
                continue;
            }
            if path.is_dir() {
                out.extend(sources(&path));
            } else if path.extension().is_some_and(|extension| extension == "rs")
                && let Ok(text) = std::fs::read_to_string(&path)
            {
                out.push((path.display().to_string(), text));
            }
        }
        out
    }

    /// The literals a source hands a surface, with the placeholders dropped, and how many call
    /// sites the sweep read — the second number is what keeps a broken reader from passing.
    fn swept_literals(source: &str) -> (Vec<String>, usize) {
        let mut literals = Vec::new();
        let mut calls = 0usize;
        for call in PAINT_CALLS {
            for open in call_sites(source, call) {
                calls += 1;
                let argument = call_argument(source, open);
                if let Some(literal) = leading_literal(argument) {
                    literals.push(literal);
                }
                literals.extend(format_literals(argument));
            }
        }
        for call in DIALOG_CALLS {
            for open in call_sites(source, call) {
                calls += 1;
                if let Some(literal) = leading_literal(call_argument(source, open)) {
                    literals.push(literal);
                }
            }
        }
        for open in call_sites(source, ".notice(") {
            calls += 1;
            literals.extend(notice_literals(call_argument(source, open)));
        }
        literals.retain(|literal| spells_words(literal));
        (literals, calls)
    }

    /// The offset of every call's opening paren.
    fn call_sites(source: &str, call: &str) -> Vec<usize> {
        let mut sites = Vec::new();
        let mut cursor = 0;
        while let Some(found) = source[cursor..].find(call) {
            let open = cursor + found + call.len() - 1;
            sites.push(open);
            cursor = open + 1;
        }
        sites
    }

    /// The same source with its top-level #[cfg(test)] modules cut out: a fixture's own English is
    /// not painted chrome, and the sweeps read the widgets outside those modules.
    fn without_test_modules(source: &str) -> String {
        let mut out = String::with_capacity(source.len());
        let mut skipping = false;
        for line in source.lines() {
            if !skipping && line.starts_with("#[cfg(test)]") {
                skipping = true;
                continue;
            }
            if skipping {
                if line == "}" {
                    skipping = false;
                }
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    /// The argument of a call whose paren is at open, cut at its matching close paren and skipping
    /// over quoted strings so a bracket inside a literal does not end the call early.
    fn call_argument(source: &str, open: usize) -> &str {
        let bytes = source.as_bytes();
        let mut depth = 0usize;
        let mut quoted = false;
        let mut escaped = false;
        for (offset, byte) in bytes[open..].iter().enumerate() {
            if quoted {
                if escaped {
                    escaped = false;
                } else if *byte == b'\\' {
                    escaped = true;
                } else if *byte == b'"' {
                    quoted = false;
                }
                continue;
            }
            match *byte {
                b'"' => quoted = true,
                b'(' => depth += 1,
                b')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return &source[open + 1..open + offset];
                    }
                }
                _ => {}
            }
        }
        ""
    }

    /// The literal an argument starts with, if it starts with one at all: a value, a tl! or a
    /// helper call paints no literal of its own.
    fn leading_literal(argument: &str) -> Option<String> {
        let rest = argument.trim_start().strip_prefix('"')?;
        let end = rest.find('"')?;
        Some(rest[..end].to_owned())
    }

    /// The literals of every format! inside an argument: a painter takes its text as the third
    /// argument, so a sweep that read only the first would miss it.
    fn format_literals(argument: &str) -> Vec<String> {
        let mut literals = Vec::new();
        let mut cursor = 0;
        while let Some(found) = argument[cursor..].find("format!(") {
            let after = cursor + found + "format!(".len();
            if let Some(literal) = leading_literal(&argument[after..]) {
                literals.push(literal);
            }
            cursor = after;
        }
        literals
    }

    /// What a notice still spells by hand: its argument with the spans a catalogue already answers
    /// blanked out, then every literal left in it. A match arm, an if/else or a format! is chrome;
    /// the routed keys' rows are the audit above.
    fn notice_literals(argument: &str) -> Vec<String> {
        let masked = masked_routes(argument);
        let mut literals = Vec::new();
        let mut rest = masked.as_str();
        while let Some(start) = rest.find('"') {
            let after = &rest[start + 1..];
            let Some(end) = after.find('"') else {
                break;
            };
            literals.push(after[..end].to_owned());
            rest = &after[end + 1..];
        }
        literals
    }

    /// The argument with every tl!, i18n::fmt and i18n::tr call removed whole — the spans whose
    /// literals are catalogue keys, which the route audit already holds to a row.
    fn masked_routes(argument: &str) -> String {
        let mut spans: Vec<(usize, usize)> = Vec::new();
        for marker in ["tl!(", "i18n::fmt(", "crate::i18n::fmt(", "i18n::tr(", "crate::i18n::tr("] {
            let mut cursor = 0;
            while let Some(found) = argument[cursor..].find(marker) {
                let open = cursor + found + marker.len() - 1;
                spans.push((open, open + call_argument(argument, open).len() + 2));
                cursor = open + 1;
            }
        }
        spans.sort_unstable();
        let mut out = String::with_capacity(argument.len());
        let mut copied = 0usize;
        for (start, end) in spans {
            if end <= copied {
                continue;
            }
            out.push_str(&argument[copied..start]);
            copied = end;
        }
        out.push_str(&argument[copied..]);
        out
    }

    /// Whether a literal spells anything once its placeholders are dropped: a literal that is only
    /// punctuation (a multiplication sign, a path separator, a key cap beside a value) paints no
    /// word of its own and is not a candidate.
    fn spells_words(literal: &str) -> bool {
        let mut words = String::with_capacity(literal.len());
        let mut depth = 0usize;
        for ch in literal.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth = depth.saturating_sub(1),
                _ if depth == 0 => words.push(ch),
                _ => {}
            }
        }
        words.chars().any(|ch| ch.is_ascii_alphabetic())
    }

    /// The shortcut editor's own words are the keymap crate's list, and both catalogues answer
    /// every one: the dialog looks them up at paint time, so the source-reading audit above cannot
    /// see them, and this is the guard that keeps a new crate string from arriving in English only.
    /// A plural is an `@plural` row keyed by its two English forms, checked for its row rather than
    /// for its output, because the English fallback would answer just as happily.
    #[test]
    fn every_keymap_editor_string_is_in_both_catalogues() {
        let mut missing = Vec::new();
        for code in ["ar", "ja", "ko", "th", "vi", "zh-TW"] {
            let lang = I18N.match_tag(code).unwrap_or_else(|| panic!("{code} is offered"));
            for source in craft_keymap::ui::STRINGS {
                // Presence, not difference: a key name or an order-only template translates to
                // itself on purpose, and the row is what says the translator has seen it.
                if I18N.get(lang, "", source).is_none() {
                    missing.push(format!("{code}: {source}"));
                }
            }
            for (one, other) in craft_keymap::ui::PLURALS {
                let key = format!("{one}|{other}");
                if I18N.get(lang, "@plural", &key).is_none() {
                    missing.push(format!("{code}: @plural {key}"));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "the shortcut editor still shows English for {} keys: {missing:#?}",
            missing.len()
        );
    }

    /// Every sort order's name is a catalogue row in each of the six languages. The label is a
    /// value from `sortlens-catalog`, so the source-reading audit above cannot see it; this is
    /// the guard that keeps the combo's seven choices from arriving in English only.
    #[test]
    fn every_sort_order_has_a_catalogue_row() {
        for code in ["ar", "ja", "ko", "th", "vi", "zh-TW"] {
            let lang = I18N.match_tag(code).unwrap_or_else(|| panic!("{code} is offered"));
            for sort in sortlens_engine::Sort::ALL {
                assert!(
                    I18N.get(lang, "", sort.label()).is_some(),
                    "{code} has no row for the sort order {:?}",
                    sort.label()
                );
            }
        }
    }

    /// The picker's languages are exactly what is embedded, each named in itself, and the
    /// direction each resolves to follows its script — Arabic right to left, Japanese left to
    /// right — with the environment override winning over both.
    #[test]
    fn the_picker_names_every_language_and_directions_follow_the_script() {
        let names: Vec<(String, &str)> = I18N
            .languages()
            .into_iter()
            .map(|l| (I18N.code(l).to_owned(), I18N.name(l)))
            .collect();
        assert_eq!(
            names,
            vec![
                ("en".into(), "English"),
                ("ar".into(), "العربية"),
                ("ja".into(), "日本語"),
                ("ko".into(), "한국어"),
                ("th".into(), "ไทย"),
                ("vi".into(), "Tiếng Việt"),
                ("zh-tw".into(), "繁體中文")
            ]
        );
        set_current(I18N.match_tag("ar").unwrap());
        assert!(matches!(resolve_direction(), craft_ui::direction::Direction::Rtl));
        set_current(I18N.match_tag("ja").unwrap());
        assert!(matches!(resolve_direction(), craft_ui::direction::Direction::Ltr));
        set_current(I18N.match_tag("ko").unwrap());
        assert!(matches!(resolve_direction(), craft_ui::direction::Direction::Ltr));
        set_current(I18N.match_tag("th").unwrap());
        assert!(matches!(resolve_direction(), craft_ui::direction::Direction::Ltr));
        set_current(I18N.match_tag("vi").unwrap());
        assert!(matches!(resolve_direction(), craft_ui::direction::Direction::Ltr));
        set_current(Lang::EN);
    }
}
