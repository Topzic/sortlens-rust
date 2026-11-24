//! What the phone remembers between launches.
//!
//! A phone has no configuration folder to inherit: the app's private data directory is the only
//! place it may write, and the OS ends the process whenever it feels like it. So the choices its
//! shell actually offers — the theme, how dense the grid is, its layout, whether motion plays, and
//! the order the library opens in — are kept in the suite's own `craft-settings` document,
//! `config.toml`, beside the catalog. The ids are the desktop's ids, so the same file means the
//! same thing on either.
//!
//! The document holds only what the phone can change today. A file written by the desktop carries
//! keys this build does not read; `craft-settings` keeps them, so choosing a theme on the phone
//! never drops a desktop setting it did not understand.

use std::path::{Path, PathBuf};

use craft_settings::{LoadReport, Settings, SettingsStore};
use serde::{Deserialize, Serialize};
use sortlens_engine::Sort;
use sortlens_ui::{Action, Density, GridMode, Motion, ThemeKind, UiState};

/// The whole settings document: the desktop's two sections, cut down to the phone's own choices.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhonePrefs {
    /// Look and feel.
    pub appearance: Appearance,
    /// The library grid.
    pub library: Library,
    /// The link to a paired desktop, if there is one.
    pub desktop: DesktopPrefs,
}

/// Look and feel: the same four ids the desktop stores, with the same defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    /// Theme preset id: `charcoal`, `midnight`, `silver`, `paper` or `high-contrast`.
    pub theme: String,
    /// Grid density id: `compact`, `comfortable` or `roomy`.
    pub density: String,
    /// Grid layout id: `square` or `justified`.
    pub grid: String,
    /// Motion id: `full` or `reduced`.
    pub motion: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "charcoal".into(),
            density: "comfortable".into(),
            grid: "justified".into(),
            motion: "full".into(),
        }
    }
}

/// The one library choice the phone's header offers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    /// The order the grid opens in: every id [`Sort`] names, including the desktop's spellings.
    pub sort: String,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            sort: Sort::Natural.id().into(),
        }
    }
}

/// The phone's link to a paired desktop (SL-J04/J05 from the app): where it is, and the token a
/// successful pairing left behind.
///
/// The code is never stored — it is a human carrier, not a credential — and an empty token means
/// unpaired, so a file from before a pairing opens unpaired and stays honest.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopPrefs {
    /// The desktop's address, as the person typed it (its `listening` line's host).
    pub host: String,
    /// The port the desktop's server announced.
    pub port: u16,
    /// The scoped bearer token pairing produced; empty until then.
    pub token: String,
    /// The device name the server recorded for this phone.
    pub device: String,
}

impl Settings for PhonePrefs {
    const APP_ID: &'static str = "sortlens";
    const SCHEMA_VERSION: u32 = 1;
}

/// The file: `config.toml` in the app's own directory, beside the library and never inside it.
#[must_use]
pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(craft_settings::store::CONFIG_FILE)
}

/// Open the phone's settings. Never fails: a damaged file is moved aside and the report says so.
#[must_use]
pub fn open(data_dir: &Path) -> (SettingsStore<PhonePrefs>, LoadReport) {
    SettingsStore::<PhonePrefs>::open_at(data_dir)
}

/// What to tell the person when the file could not be read in full.
#[must_use]
pub fn notice(report: &LoadReport) -> Option<&'static str> {
    report
        .has_problems()
        .then_some("Settings could not be read in full; defaults are in use")
}

impl PhonePrefs {
    /// The theme the file names; an unknown id reads as the default.
    #[must_use]
    pub fn theme(&self) -> ThemeKind {
        ThemeKind::from_id(&self.appearance.theme)
    }

    /// Apply the saved choices to a fresh shell. Every id is parsed the way the desktop parses it,
    /// so an unknown one falls back to its default rather than refusing to open.
    pub fn apply(&self, ui: &mut UiState) {
        ui.density = Density::from_id(&self.appearance.density);
        ui.grid_mode = GridMode::from_id(&self.appearance.grid);
        ui.motion = Motion::from_id(&self.appearance.motion);
        let sort = Sort::parse(&self.library.sort);
        ui.sort = sort;
        ui.library_sort = sort;
    }
}

/// Write a choice into the document: the one place an action becomes a stored id.
///
/// Returns true when the action was a setting at all. Kept out of [`App`](crate::App) so the
/// mapping is tested without a window: the round-trip test drives these same actions and reads the
/// file back.
pub(crate) fn record(store: &mut SettingsStore<PhonePrefs>, action: &Action) -> bool {
    match action {
        Action::SetTheme(kind) => {
            store.update(|prefs| prefs.appearance.theme = kind.id().to_owned());
        }
        Action::SetDensity(density) => {
            store.update(|prefs| prefs.appearance.density = density.id().to_owned());
        }
        Action::SetGrid(mode) => {
            store.update(|prefs| prefs.appearance.grid = mode.id().to_owned());
        }
        Action::SetMotion(motion) => {
            store.update(|prefs| prefs.appearance.motion = motion.id().to_owned());
        }
        Action::SetSort(sort) => {
            store.update(|prefs| prefs.library.sort = sort.id().to_owned());
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use sortlens_testkit::Fixture;

    /// Write the whole document through the real file and read it back, the way a restart does.
    fn round_trip(fixture: &Fixture, prefs: &PhonePrefs) -> SettingsStore<PhonePrefs> {
        let (mut store, _) = open(fixture.root());
        store.set(prefs.clone());
        store.flush().expect("the settings file is writable");
        let (store, report) = open(fixture.root());
        assert!(!report.has_problems(), "a file this build wrote reads back cleanly");
        store
    }

    /// The shell as a launch would build it from the file.
    fn applied(store: &SettingsStore<PhonePrefs>) -> UiState {
        let mut ui = UiState::new(store.get().theme(), 0.0);
        store.get().apply(&mut ui);
        ui
    }

    #[test]
    fn defaults_are_the_desktops_own() {
        let prefs = PhonePrefs::default();
        assert_eq!(prefs.appearance.theme, ThemeKind::Charcoal.id());
        assert_eq!(prefs.appearance.density, Density::default().id());
        assert_eq!(prefs.appearance.grid, GridMode::default().id());
        assert_eq!(prefs.appearance.motion, Motion::default().id());
        assert_eq!(prefs.library.sort, Sort::Natural.id());
        // And the defaults really are what an empty shell opens with, not an id that reads as some
        // other value: a phone with no file behaves exactly like a desktop with no file.
        let mut ui = UiState::new(prefs.theme(), 0.0);
        prefs.apply(&mut ui);
        assert_eq!(ui.density, Density::default());
        assert_eq!(ui.grid_mode, GridMode::default());
        assert_eq!(ui.motion, Motion::default());
        assert_eq!(ui.sort, Sort::Natural);
    }

    /// Every id the shell can offer comes back as itself. A document that stored a theme and read a
    /// default would fail here, one variant at a time.
    #[test]
    fn every_id_the_shell_offers_survives_a_restart() {
        let fixture = Fixture::new("phone-settings-ids");
        for theme in ThemeKind::ALL {
            let prefs = PhonePrefs {
                appearance: Appearance {
                    theme: theme.id().to_owned(),
                    ..Appearance::default()
                },
                ..PhonePrefs::default()
            };
            assert_eq!(round_trip(&fixture, &prefs).get().theme(), theme, "theme {}", theme.id());
        }
        for density in Density::ALL {
            let prefs = PhonePrefs {
                appearance: Appearance {
                    density: density.id().to_owned(),
                    ..Appearance::default()
                },
                ..PhonePrefs::default()
            };
            let ui = applied(&round_trip(&fixture, &prefs));
            assert_eq!(ui.density, density, "density {}", density.id());
        }
        for mode in GridMode::ALL {
            let prefs = PhonePrefs {
                appearance: Appearance {
                    grid: mode.id().to_owned(),
                    ..Appearance::default()
                },
                ..PhonePrefs::default()
            };
            let ui = applied(&round_trip(&fixture, &prefs));
            assert_eq!(ui.grid_mode, mode, "grid {}", mode.id());
        }
        for motion in Motion::ALL {
            let prefs = PhonePrefs {
                appearance: Appearance {
                    motion: motion.id().to_owned(),
                    ..Appearance::default()
                },
                ..PhonePrefs::default()
            };
            let ui = applied(&round_trip(&fixture, &prefs));
            assert_eq!(ui.motion, motion, "motion {}", motion.id());
        }
        for sort in Sort::ALL {
            let prefs = PhonePrefs {
                library: Library {
                    sort: sort.id().to_owned(),
                },
                ..PhonePrefs::default()
            };
            let ui = applied(&round_trip(&fixture, &prefs));
            assert_eq!(ui.sort, sort, "sort {}", sort.id());
            assert_eq!(ui.library_sort, sort, "the library's own order follows the file");
        }
    }

    /// A choice made with a tap reaches the file, and an action that is not a setting never writes.
    /// This drives the same helper [`App::apply`](crate::App) drives, so a missing arm fails here.
    #[test]
    fn a_choice_becomes_an_id_in_the_file_and_other_actions_do_not() {
        let fixture = Fixture::new("phone-settings-record");
        let (mut store, _) = open(fixture.root());
        assert!(record(&mut store, &Action::SetDensity(Density::Compact)));
        assert!(record(&mut store, &Action::SetGrid(GridMode::Square)));
        assert!(record(&mut store, &Action::SetMotion(Motion::Reduced)));
        assert!(record(&mut store, &Action::SetSort(Sort::Newest)));
        assert!(record(&mut store, &Action::SetTheme(ThemeKind::Paper)));
        assert!(!record(&mut store, &Action::Notice("hello".into())), "a notice is not a setting");
        assert!(!record(&mut store, &Action::PickFolder), "adding photos is not a setting");
        store.flush().expect("the settings file is writable");

        let (store, report) = open(fixture.root());
        assert!(!report.has_problems());
        assert_eq!(store.get().theme(), ThemeKind::Paper);
        let ui = applied(&store);
        assert_eq!(ui.density, Density::Compact);
        assert_eq!(ui.grid_mode, GridMode::Square);
        assert_eq!(ui.motion, Motion::Reduced);
        assert_eq!(ui.sort, Sort::Newest);
    }

    /// An id from a later build reads as its default, and the file is not treated as damaged.
    #[test]
    fn an_unknown_id_reads_as_its_default() {
        let fixture = Fixture::new("phone-settings-future");
        let text = "schema_version = 1\n\n[appearance]\ntheme = \"paper\"\ndensity = \"gigantic\"\ngrid = \"mosaic\"\n";
        std::fs::write(path(fixture.root()), text).expect("a writable fixture");

        let (store, report) = open(fixture.root());
        assert!(!report.has_problems(), "an unknown value is not a damaged file");
        assert_eq!(store.get().theme(), ThemeKind::Paper, "a known id still decides");
        let ui = applied(&store);
        assert_eq!(ui.density, Density::default(), "an unknown density reads as the default");
        assert_eq!(ui.grid_mode, GridMode::default(), "an unknown layout reads as the default");
    }

    /// A file with keys this build has never heard of keeps them: saving a theme on the phone must
    /// not throw away a setting the desktop wrote.
    #[test]
    fn a_later_builds_keys_stay_in_the_file() {
        let fixture = Fixture::new("phone-settings-keys");
        let text = "schema_version = 1\n\n[appearance]\ntheme = \"charcoal\"\n\n[experimental]\nshiny = true\n";
        std::fs::write(path(fixture.root()), text).expect("a writable fixture");

        let (mut store, _) = open(fixture.root());
        store.update(|prefs| prefs.appearance.theme = ThemeKind::Silver.id().to_owned());
        store.flush().expect("the settings file is writable");
        let written = std::fs::read_to_string(path(fixture.root())).expect("the file this build wrote");
        assert!(written.contains("shiny"), "an unknown key stays in the file:\n{written}");
        assert!(written.contains("silver"), "and this build's change is there too:\n{written}");
    }

    /// A damaged file is moved aside, the defaults are used, and the person is told.
    #[test]
    fn a_damaged_file_is_moved_aside_and_says_so() {
        let fixture = Fixture::new("phone-settings-damaged");
        std::fs::write(path(fixture.root()), "this is not [[[ toml").expect("a writable fixture");

        let (store, report) = open(fixture.root());
        assert!(report.broken_moved_to.is_some(), "the damaged file is kept, not deleted");
        assert_eq!(*store.get(), PhonePrefs::default(), "defaults are in use");
        assert_eq!(notice(&report), Some("Settings could not be read in full; defaults are in use"));
        // A clean file says nothing at all.
        let clean = Fixture::new("phone-settings-clean");
        let (_, report) = open(clean.root());
        assert_eq!(notice(&report), None);
    }

    /// The document sits beside the library, never inside it: clearing the catalog cannot take the
    /// preferences with it.
    #[test]
    fn the_file_sits_beside_the_library_and_never_inside_it() {
        let fixture = Fixture::new("phone-settings-path");
        let file = path(fixture.root());
        assert_eq!(file, fixture.root().join("config.toml"));
        assert!(!file.starts_with(crate::places::library_dir(fixture.root())));
    }
}
