//! Persisted preferences.
//!
//! Every section is `#[serde(default)]` so a key added in a later milestone reads as its default
//! from an older `config.toml`, and a key this build does not know survives a round trip.

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
    /// Interface scale, 1.0 is 100%.
    pub ui_scale: f32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "charcoal".into(),
            density: "comfortable".into(),
            grid: "justified".into(),
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
        assert!(!prefs.remote.enabled, "the server is opt-in");
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
}
