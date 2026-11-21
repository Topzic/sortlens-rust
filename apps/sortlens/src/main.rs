//! Sortlens: the desktop application.
//!
//! `main` creates the window, applies the title-bar policy, opens the settings store and hands
//! control to [`App`]. Everything else lives in the crates.

#![forbid(unsafe_code)]

mod settings;

use craft_settings::{Location, store::SettingsStore};
use craft_window::Prefix;
use craft_window::policy::{Backend, Frame, TitleBarPref, apply as apply_titlebar, decide_for};
use settings::Prefs;
use sortlens_engine::Engine;
use sortlens_ui::{Action, ThemeKind, UiState, apply as apply_theme};

/// Left inset that clears the traffic lights in an integrated title bar.
const MAC_TRAFFIC_LIGHTS: f32 = 78.0;

fn folder_name(path: &str) -> String {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

/// Where the catalog lives. One library per user for now; libraries become a list in M5.
fn library_dir() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("SORTLENS_LIBRARY_DIR")
        && !dir.trim().is_empty()
    {
        return std::path::PathBuf::from(dir);
    }
    let base = if cfg!(target_os = "macos") {
        home().join("Library").join("Application Support")
    } else if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| home().join("AppData").join("Roaming"))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| home().join(".local").join("share"))
    };
    base.join("Sortlens").join("Library")
}

fn home() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn main() -> eframe::Result {
    let decision = decide_for(Prefix::new("SORTLENS"), Backend::from_env(), TitleBarPref::Integrated);
    let inset = match decision.frame {
        Frame::Integrated(_) => MAC_TRAFFIC_LIGHTS,
        _ => 10.0,
    };

    let viewport = apply_titlebar(
        egui::ViewportBuilder::default()
            .with_title("Sortlens")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([880.0, 560.0]),
        &decision.frame,
    );

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native("Sortlens", options, Box::new(move |cc| Ok(Box::new(App::new(cc, inset)))))
}

/// The application: one engine, one shell, one settings store.
struct App {
    engine: Engine,
    ui: UiState,
    store: SettingsStore<Prefs>,
    applied_theme: ThemeKind,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, title_inset: f32) -> Self {
        let (store, report) = SettingsStore::<Prefs>::open(Location::Auto);
        let kind = ThemeKind::from_id(&store.get().appearance.theme);
        apply_theme(&cc.egui_ctx, kind);

        let mut ui = UiState::new(kind, title_inset);
        if report.has_problems() {
            ui.notice("Settings could not be read in full; defaults are in use");
        }

        let engine = match Engine::open(library_dir()) {
            Ok(engine) => engine,
            Err(error) => {
                ui.notice(format!("The library could not be opened: {error}"));
                Engine::new()
            }
        };

        Self {
            engine,
            ui,
            store,
            applied_theme: kind,
        }
    }

    fn apply(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::SetTheme(kind) => {
                if kind != self.applied_theme {
                    apply_theme(ctx, kind);
                    self.applied_theme = kind;
                    self.store.update(|prefs| prefs.appearance.theme = kind.id().to_owned());
                }
            }
            Action::Quit => {
                let _ = self.store.flush();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Action::Notice(text) => self.ui.notice(text),
            Action::PickFolder => {
                if let Some(folder) = rfd::FileDialog::new().set_title("Add a folder to Sortlens").pick_folder() {
                    let path = folder.to_string_lossy().to_string();
                    match self.engine.begin_scan(path.clone()) {
                        Ok(()) => self.ui.notice(format!("Indexing {}…", folder_name(&path))),
                        Err(error) => self.ui.notice(error.to_string()),
                    }
                }
            }
            Action::Rescan(root) => match self.engine.begin_scan(root.clone()) {
                Ok(()) => self.ui.notice(format!("Rescanning {}…", folder_name(&root))),
                Err(error) => self.ui.notice(error.to_string()),
            },
            Action::RemoveRoot(root) => match self.engine.remove_root(&root) {
                Ok(count) => self.ui.notice(format!("Removed {count} photos from {}", folder_name(&root))),
                Err(error) => self.ui.notice(error.to_string()),
            },
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        let tick = self.store.tick(std::time::Instant::now());
        if let Some(deadline) = tick.next_deadline {
            ctx.request_repaint_after(deadline.saturating_duration_since(std::time::Instant::now()));
        }

        let actions = sortlens_ui::show(ui, &mut self.ui, &mut self.engine);
        for action in actions {
            self.apply(&ctx, action);
        }

        if ctx.input(|input| input.viewport().close_requested()) {
            let _ = self.engine.close();
            let _ = self.store.flush();
        }

        if self.engine.last_effect() == sortlens_engine::Effect::Quit {
            let _ = self.engine.close();
            let _ = self.store.flush();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
