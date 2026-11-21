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
    /// Grid density: `comfortable` or `compact`.
    pub density: String,
    /// Interface scale, 1.0 is 100%.
    pub ui_scale: f32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "charcoal".into(),
            density: "comfortable".into(),
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
}

impl Default for Library {
    fn default() -> Self {
        Self {
            watch_folders: true,
            write_sidecars: true,
            delete_mode: "trash".into(),
            reject_folder: String::new(),
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
        assert!(prefs.review.auto_advance);
        assert!(prefs.library.write_sidecars, "XMP sidecars are on by default");
        assert_eq!(prefs.library.delete_mode, "trash");
        assert!(!prefs.ai.enabled, "models are opt-in");
        assert!(!prefs.remote.enabled, "the server is opt-in");
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
