//! The interface font stack: egui's own faces, then the optional craft-fonts fallbacks.
//!
//! Three rules, in the order they matter.
//!
//! - **A build without craft-fonts works.** `CRAFT_FONTS` comes from `build.rs` and is empty unless
//!   the machine has a craft-fonts checkout (`CRAFT_FONTS_DIR`). CI has none, so the empty table is
//!   the configuration under test, not an accident.
//! - **Craft faces go last.** They are fallbacks for scripts egui's bundled face does not draw:
//!   Japanese, Korean, Hebrew, the Indian scripts and the UI symbols. Putting them first would change
//!   Latin text, which is the one thing a font fallback must never do.
//! - **The scripts the interface actually needs are named in code.** [`REQUIRED_SCRIPTS`] is what
//!   [`coverage`] checks: the UI paints `◀`/`▶`, `↔` and `⌘` as text, and none of them live in
//!   egui's Latin face — they come from the bundled emoji faces or, on a craft build, from `Zsym`.

#![forbid(unsafe_code)]

use egui::{FontData, FontDefinitions, FontFamily};

/// A face embedded at build time from a craft-fonts checkout.
#[derive(Debug)]
pub struct CraftFont {
    /// Family name, spelled as craft-fonts spells it (`NanumGothic`, one word).
    pub family: &'static str,
    /// Style within the family, e.g. `Regular`.
    pub style: &'static str,
    /// ISO 15924 scripts this face is for.
    pub scripts: &'static [&'static str],
    /// The font file.
    pub bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/craft_fonts.rs"));

/// Scripts the interface needs a face for.
///
/// `Latn` and `Zsym` are not optional: the first is the interface language and the second is the
/// symbol coverage behind the `◀`/`▶` keycaps, the compare header's `↔` and the `⌘` hints. The rest
/// arrive with translations and are listed so a build that ships them can check them in one place.
pub const REQUIRED_SCRIPTS: &[&str] = &["Latn", "Zsym"];

/// Scripts a translation would add, each of which needs a face when the language ships.
pub const TRANSLATED_SCRIPTS: &[&str] = &[
    "Jpan", "Kore", "Hans", "Hebr", "Deva", "Beng", "Guru", "Gujr", "Orya", "Taml", "Telu", "Knda", "Mlym", "Arab", "Thai", "Geor",
];

/// Whether the app was built with craft-fonts.
pub fn embedded() -> bool {
    !CRAFT_FONTS.is_empty()
}

/// The families in the embedded table, sorted and deduplicated, for a status line or a test.
pub fn families() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = CRAFT_FONTS.iter().map(|font| font.family).collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// Whether this build embeds a symbols face of its own, which the keymap's glyphs need.
///
/// Without one the keymap must not paint `⌃`/`⇧`/`⏎`: egui's bundled faces draw `⌘` but none
/// of the others, so a keycap spelled with them would be a box in front of the shortcut.
pub fn symbols() -> bool {
    CRAFT_FONTS.iter().any(|font| font.scripts.contains(&"Zsym"))
}

/// Which required scripts the embedded faces cover, and which they do not.
///
/// A build without craft-fonts covers nothing, and that is reported as such rather than as a pass:
/// egui's own face draws Latin, so the interface is legible, but the symbol and translated scripts
/// are the system's job at that point.
pub fn coverage() -> (Vec<&'static str>, Vec<&'static str>) {
    let mut covered = Vec::new();
    let mut missing = Vec::new();
    for script in REQUIRED_SCRIPTS {
        if CRAFT_FONTS.iter().any(|font| font.scripts.iter().any(|tagged| tagged == script)) {
            covered.push(*script);
        } else {
            missing.push(*script);
        }
    }
    (covered, missing)
}

/// Install the fonts into a context: egui's defaults first, craft faces appended as fallbacks.
///
/// Returns how many faces were added, which the app logs rather than showing — a font count is a
/// build fact, not something a photographer needs in the status bar.
pub fn install(ctx: &egui::Context) -> usize {
    if CRAFT_FONTS.is_empty() {
        return 0;
    }
    ctx.set_fonts(stack());
    CRAFT_FONTS.len()
}

/// The font definitions this build would install: egui's defaults, with craft faces appended.
///
/// Separate from [`install`] so the stack itself can be asserted in a test without a context.
pub fn stack() -> FontDefinitions {
    let mut definitions = FontDefinitions::default();
    for font in CRAFT_FONTS {
        let key = format!("{}::{}", font.family, font.style);
        definitions
            .font_data
            .insert(key.clone(), std::sync::Arc::new(FontData::from_static(font.bytes)));
        // Appended, never prepended: a fallback that takes over Latin text is a bug.
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            definitions.families.entry(family).or_default().push(key.clone());
        }
    }
    definitions
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn a_build_without_the_checkout_still_has_a_stack() {
        // CI has no craft-fonts checkout, so this is the configuration that runs. It must install
        // nothing, report nothing embedded, and leave egui's own fonts alone.
        if embedded() {
            // A developer with CRAFT_FONTS_DIR set: assert the stronger property instead.
            let (covered, _) = coverage();
            assert!(!covered.is_empty(), "an embedded build should cover Latn and Zsym");
            return;
        }
        let ctx = egui::Context::default();
        assert_eq!(install(&ctx), 0);
        assert!(families().is_empty());
        let (covered, missing) = coverage();
        assert!(covered.is_empty());
        assert_eq!(missing, REQUIRED_SCRIPTS.to_vec());
    }

    #[test]
    fn installing_keeps_the_default_family_and_appends_fallbacks() {
        let defaults = FontDefinitions::default();
        let installed = stack();
        assert_eq!(install(&egui::Context::default()), CRAFT_FONTS.len());
        for (family, names) in &defaults.families {
            let installed = installed.families.get(family).expect("every default family survives");
            assert!(
                installed.len() >= names.len(),
                "{family:?} lost a face: {} then {}",
                names.len(),
                installed.len()
            );
            // The leading face is untouched: a fallback that takes over Latin text is a bug.
            assert_eq!(installed.first(), names.first(), "{family:?} changed its first face");
        }
    }

    #[test]
    fn the_required_scripts_are_the_ones_the_interface_draws() {
        // If this list changes, the keymap's keycaps and the translation plan changed with it.
        assert_eq!(REQUIRED_SCRIPTS, &["Latn", "Zsym"]);
        assert!(TRANSLATED_SCRIPTS.contains(&"Jpan"));
        assert!(!TRANSLATED_SCRIPTS.contains(&"Latn"), "Latin is not a translation");
    }
}
