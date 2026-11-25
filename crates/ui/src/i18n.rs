//! Interface strings through craft-i18n: the English text is the lookup key (SL-L07 scaffolding,
//! Phase 4 item 2d).
//!
//! Every routed label is an English literal; a missing translation is the English itself, so a
//! catalogue a translation has not reached yet never breaks the interface. The scaffold ships
//! Arabic and Japanese — the first of the six languages the design names — and the direction
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
static CATALOGS: &Embedded = &[("ar", "", include_str!("i18n/ar.tsv")), ("ja", "", include_str!("i18n/ja.tsv"))];

/// The interface languages offered, in picker order: English first, then the embedded ones as
/// `I18n::languages` sorts them.
pub const LANGUAGES: &[&str] = &["en", "ar", "ja"];

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

/// The language with this code, if the interface offers it (`en`, `ar`, `ja`).
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
        // row is missing shows English just as quietly as a missing label does.
        let sources = [include_str!("shell.rs"), include_str!("theme.rs"), include_str!("sharpness.rs")];
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
        for code in ["ar", "ja"] {
            let lang = I18N.match_tag(code).unwrap_or_else(|| panic!("{code} is offered"));
            let missing: Vec<&String> = keys.iter().filter(|k| I18N.t(lang, k) == *k).collect();
            assert!(missing.is_empty(), "{code} is missing {} routed rows: {missing:#?}", missing.len());
        }
    }

    /// The shortcut editor's own words are the keymap crate's list, and both catalogues answer
    /// every one: the dialog looks them up at paint time, so the source-reading audit above cannot
    /// see them, and this is the guard that keeps a new crate string from arriving in English only.
    /// A plural is an `@plural` row keyed by its two English forms, checked for its row rather than
    /// for its output, because the English fallback would answer just as happily.
    #[test]
    fn every_keymap_editor_string_is_in_both_catalogues() {
        let mut missing = Vec::new();
        for code in ["ar", "ja"] {
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
            vec![("en".into(), "English"), ("ar".into(), "العربية"), ("ja".into(), "日本語")]
        );
        set_current(I18N.match_tag("ar").unwrap());
        assert!(matches!(resolve_direction(), craft_ui::direction::Direction::Rtl));
        set_current(I18N.match_tag("ja").unwrap());
        assert!(matches!(resolve_direction(), craft_ui::direction::Direction::Ltr));
        set_current(Lang::EN);
    }
}
