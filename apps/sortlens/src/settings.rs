//! Persisted preferences.
//!
//! Every section is `#[serde(default)]` so a key added in a later milestone reads as its default
//! from an older `config.toml`, and a key this build does not know survives a round trip.

use craft_keymap::Profiles;
use craft_settings::Settings;
use serde::{Deserialize, Serialize};

/// The whole settings document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Prefs {
    pub appearance: Appearance,
    pub review: Review,
    pub library: Library,
    pub ai: Ai,
    pub remote: Remote,
    pub updates: Updates,
    /// Shortcut sets; the active set is what the keymap loads. Stored as the array craft-settings
    /// replaces whole, so a set the editor deleted stays deleted after saving.
    pub shortcuts: Profiles,
}

/// Look and feel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    /// Theme preset id: `charcoal`, `midnight`, `silver`, `paper`.
    pub theme: String,
    /// Grid density id: `compact`, `comfortable` or `roomy`. An unknown value reads as the default, so a
    /// settings file written by a later build cannot stop this one from opening.
    pub density: String,
    /// Grid layout id: `square` or `justified`. Unknown reads as the default, same rule as density.
    pub grid: String,
    /// Motion id: `full` or `reduced`. Reduced turns off the cull overlay's slide and fade; unknown reads as
    /// the default, same rule as the others.
    pub motion: String,
    /// Interface language code (BCP 47, lowercase): `en`, or an offered embedded language. An
    /// unknown code reads as English, so a settings file written by a later build cannot stop
    /// this one from opening.
    pub language: String,
    /// Interface scale, 1.0 is 100%.
    pub ui_scale: f32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "charcoal".into(),
            density: "comfortable".into(),
            grid: "justified".into(),
            motion: "full".into(),
            language: "en".into(),
            ui_scale: 1.0,
        }
    }
}

/// Cull behaviour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Review {
    /// Advance to the next frame after a decision.
    pub auto_advance: bool,
    /// Ask before the reject batch leaves the library.
    pub confirm_apply: bool,
    /// Show the histogram in the detail view.
    pub show_histogram: bool,
    /// Zoom to the focus point when a frame opens.
    pub zoom_to_focus: bool,
}

impl Default for Review {
    fn default() -> Self {
        Self {
            auto_advance: true,
            confirm_apply: true,
            show_histogram: true,
            zoom_to_focus: false,
        }
    }
}

/// Library and filesystem behaviour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    /// Pick up files that appear while the app is open.
    pub watch_folders: bool,
    /// Write ratings, flags and keywords to XMP sidecars.
    pub write_sidecars: bool,
    /// What Apply does with rejects: `trash` or `move`.
    pub delete_mode: String,
    /// Folder that receives moved rejects when `delete_mode` is `move`.
    pub reject_folder: String,
    /// Order the library grid opened with: `added`, `name`, `newest`, `oldest`, `largest`, `rating`.
    pub sort: String,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            watch_folders: true,
            write_sidecars: true,
            delete_mode: "trash".into(),
            reject_folder: String::new(),
            sort: "added".into(),
        }
    }
}

/// Local model behaviour.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Ai {
    /// Run tagging on import.
    pub enabled: bool,
    /// Model pack id; empty means the bundled default.
    pub model: String,
    /// Write accepted suggestions to XMP keywords.
    pub write_keywords: bool,
    /// The owner-approved opt-in for the one network call: downloading models. The files are
    /// digest-verified against the manifest; nothing downloads until a model is installed.
    pub download_consent: bool,
}

/// Phone and browser access.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Remote {
    /// Serve the library to paired devices.
    pub enabled: bool,
    /// Port for the local server; 0 picks one.
    pub port: u16,
}

/// Update checks (SL-I10).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Updates {
    /// The owner-approved opt-in for the other one network call: checking GitHub's releases for
    /// a newer Sortlens. Off by default, exactly like the model downloader's consent.
    pub check_consent: bool,
}

impl Settings for Prefs {
    const APP_ID: &'static str = "sortlens";
    const SCHEMA_VERSION: u32 = 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_documented_ones() {
        let prefs = Prefs::default();
        assert_eq!(prefs.appearance.theme, "charcoal");
        assert_eq!(prefs.appearance.density, "comfortable");
        assert!(prefs.review.auto_advance);
        assert!(prefs.library.write_sidecars, "XMP sidecars are on by default");
        assert_eq!(prefs.library.sort, "added");
        assert_eq!(prefs.library.delete_mode, "trash");
        assert!(!prefs.ai.enabled, "models are opt-in");
        assert!(!prefs.ai.download_consent, "model downloads are opt-in");
        assert!(!prefs.remote.enabled, "the server is opt-in");
        assert!(!prefs.updates.check_consent, "update checks are opt-in");
    }

    /// Every grid mode survives a trip through the settings file, and an unknown one reads as the default.
    #[test]
    fn every_grid_mode_round_trips_and_a_future_one_falls_back() {
        for mode in sortlens_ui::GridMode::ALL {
            let mut prefs = Prefs::default();
            prefs.appearance.grid = mode.id().to_owned();
            let written = toml::to_string(&prefs).expect("write");
            let read: Prefs = toml::from_str(&written).expect("parse");
            assert_eq!(sortlens_ui::GridMode::from_id(&read.appearance.grid), mode);
        }
        let read: Prefs = toml::from_str("[appearance]\ngrid = \"mosaic\"\n").expect("parse");
        assert_eq!(
            sortlens_ui::GridMode::from_id(&read.appearance.grid),
            sortlens_ui::GridMode::default()
        );
    }

    /// Every preset — including High Contrast, whose id has a hyphen the enum's serde name would not
    /// have produced — survives a trip through the settings file, and an unknown one reads as the default.
    #[test]
    fn every_theme_round_trips_and_a_future_one_falls_back() {
        for kind in sortlens_ui::ThemeKind::ALL {
            let mut prefs = Prefs::default();
            prefs.appearance.theme = kind.id().to_owned();
            let text = toml::to_string(&prefs).expect("serialise");
            let read: Prefs = toml::from_str(&text).expect("parse");
            assert_eq!(sortlens_ui::ThemeKind::from_id(&read.appearance.theme), kind);
        }
        let read: Prefs = toml::from_str("[appearance]\ntheme = \"high-contrast\"\n").expect("parse");
        assert_eq!(
            sortlens_ui::ThemeKind::from_id(&read.appearance.theme),
            sortlens_ui::ThemeKind::HighContrast,
            "the persisted id is the one the menus and logs use"
        );
        let read: Prefs = toml::from_str("[appearance]\ntheme = \"neon\"\n").expect("parse");
        assert_eq!(
            sortlens_ui::ThemeKind::from_id(&read.appearance.theme),
            sortlens_ui::ThemeKind::Charcoal,
            "a settings file from a later build must not stop this one opening"
        );
    }

    /// Every density the grid knows about survives a trip through the settings file, and an unknown one reads
    /// as the default rather than stopping the app from opening.
    #[test]
    fn every_density_round_trips_and_a_future_one_falls_back() {
        for step in sortlens_ui::Density::ALL {
            let mut prefs = Prefs::default();
            prefs.appearance.density = step.id().to_owned();
            let text = toml::to_string(&prefs).expect("serialise");
            let read: Prefs = toml::from_str(&text).expect("parse");
            assert_eq!(sortlens_ui::Density::from_id(&read.appearance.density), step);
        }
        let read: Prefs = toml::from_str("[appearance]\ndensity = \"gigantic\"\n").expect("parse");
        assert_eq!(
            sortlens_ui::Density::from_id(&read.appearance.density),
            sortlens_ui::Density::Comfortable,
            "a settings file from a later build must not stop this one opening"
        );
    }

    /// Reduced motion is a persisted setting like density, and an unknown value falls back to full motion
    /// rather than to no animation at all — a settings file from a later build must not change behaviour.
    #[test]
    fn every_motion_setting_round_trips_and_an_unknown_one_is_full() {
        for motion in sortlens_ui::Motion::ALL {
            let mut prefs = Prefs::default();
            prefs.appearance.motion = motion.id().to_owned();
            let text = toml::to_string(&prefs).expect("serialise");
            let read: Prefs = toml::from_str(&text).expect("parse");
            assert_eq!(sortlens_ui::Motion::from_id(&read.appearance.motion), motion);
        }
        let read: Prefs = toml::from_str("[appearance]\nmotion = \"bouncy\"\n").expect("parse");
        assert_eq!(sortlens_ui::Motion::from_id(&read.appearance.motion), sortlens_ui::Motion::Full);
        // And a settings file written before this setting existed still opens, with motion at its default.
        let old: Prefs = toml::from_str("[appearance]\ntheme = \"paper\"\n").expect("parse");
        assert_eq!(old.appearance.motion, "full");
    }

    /// The interface language persists the same way, and an unknown code reads as English rather
    /// than stopping the app from opening (SL-L07).
    #[test]
    fn the_language_setting_round_trips_and_an_unknown_one_is_english() {
        for language in sortlens_ui::i18n::LANGUAGES {
            let mut prefs = Prefs::default();
            prefs.appearance.language = (*language).to_owned();
            let text = toml::to_string(&prefs).expect("serialise");
            let read: Prefs = toml::from_str(&text).expect("parse");
            assert_eq!(read.appearance.language, *language);
            // Every persisted code is one the interface offers.
            assert!(sortlens_ui::i18n::from_code(&read.appearance.language).is_some(), "{language}");
        }
        // A code the interface does not know falls back to English at restore, not to an error.
        let read: Prefs = toml::from_str("[appearance]\nlanguage = \"tlh\"\n").expect("parse");
        assert!(sortlens_ui::i18n::from_code(&read.appearance.language).is_none());
        // And a settings file written before this setting existed still opens, in English.
        let old: Prefs = toml::from_str("[appearance]\ntheme = \"paper\"\n").expect("parse");
        assert_eq!(old.appearance.language, "en");
    }

    #[test]
    fn update_consent_round_trips_and_an_old_file_stays_off() {
        let mut prefs = Prefs::default();
        assert!(!prefs.updates.check_consent);
        prefs.updates.check_consent = true;
        let text = toml::to_string(&prefs).expect("serialise");
        let read: Prefs = toml::from_str(&text).expect("parse");
        assert!(read.updates.check_consent, "the opt-in survives the settings file");
        // A settings file written before the section existed reads with the consent off, which is
        // what "opt-in" means: nobody consented without saying so.
        let old: Prefs = toml::from_str("[appearance]\ntheme = \"paper\"\n").expect("parse");
        assert!(!old.updates.check_consent);
    }

    #[test]
    fn unknown_keys_are_kept() {
        let text = r#"
            [appearance]
            theme = "paper"
            future_key = 42
        "#;
        let prefs: Prefs = toml::from_str(text).expect("parses");
        assert_eq!(prefs.appearance.theme, "paper");
        assert_eq!(prefs.review, Review::default());
    }

    /// The shortcut sets survive a settings round trip, and a file written before the section
    /// existed opens with the factory bindings (SL-I09).
    #[test]
    fn shortcut_sets_round_trip_and_an_older_file_reads_the_defaults() {
        let mut profiles = Profiles::default();
        let mut rebound = craft_keymap::Overrides::new();
        assert!(rebound.insert("view.detail", vec!["E".to_owned()]), "the override is stored");
        profiles
            .store(rebound, sortlens_ui::keymap::BUILTINS, "Custom")
            .expect("stored as Custom");
        let prefs = Prefs {
            shortcuts: profiles,
            ..Prefs::default()
        };
        let text = toml::to_string(&prefs).expect("serialise");
        let read: Prefs = toml::from_str(&text).expect("parse");
        let loaded = read.shortcuts.active_overrides(sortlens_ui::keymap::BUILTINS);
        assert_eq!(loaded.get("view.detail"), Some(&["E".to_owned()][..]), "the rebind survives");

        let older: Prefs = toml::from_str("[appearance]\ntheme = \"paper\"\n").expect("parse");
        assert!(older.shortcuts.active_overrides(sortlens_ui::keymap::BUILTINS).is_empty());
    }
}
