//! Sortlens for phones and tablets (SL-J10).
//!
//! One engine, one shell, one catalog format: this crate is the desktop application's host with the
//! desktop removed — no title bar, no file dialogs, no reveal-in-folder, no sync server. What the
//! phone adds is where it looks for photographs ([`places`]) and the two entry points a phone
//! host calls ([`entry`]). Everything that decides, ranks, culls or paints is `sortlens-engine`
//! and `sortlens-ui`, the same code the desktop runs.

#![deny(unsafe_code)]

pub mod entry;
pub mod places;

use std::path::PathBuf;

use sortlens_engine::Engine;
use sortlens_ui::{Action, ThemeKind, Thumbnails, UiState, apply as apply_theme};

/// The phone's theme until settings are persisted on the device.
const THEME: ThemeKind = ThemeKind::Charcoal;

/// The application: one engine, one shell, no settings file yet.
pub struct App {
    engine: Engine,
    ui: UiState,
    thumbs: Thumbnails,
    /// The theme the context currently carries.
    applied_theme: ThemeKind,
}

impl App {
    /// Open the library under `data_dir` and style the context. The shell's own title inset is zero:
    /// a phone has no traffic lights to clear.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>, data_dir: PathBuf) -> Self {
        sortlens_ui::fonts::install(&cc.egui_ctx);
        apply_theme(&cc.egui_ctx, THEME);
        let mut ui = UiState::new(THEME, 0.0);
        let engine = match Engine::open(places::library_dir(&data_dir)) {
            Ok(engine) => engine,
            Err(error) => {
                ui.notice(format!("The library could not be opened: {error}"));
                Engine::new()
            }
        };
        Self {
            engine,
            ui,
            thumbs: Thumbnails::new(places::cache_dir(&data_dir)),
            applied_theme: THEME,
        }
    }

    /// Begin indexing every folder on the device that holds the user's photographs.
    fn add_device_photos(&mut self) {
        let roots = places::photo_roots();
        if roots.is_empty() {
            self.ui.notice("No photo folders on this device are readable yet");
            return;
        }
        for root in roots {
            let text = root.to_string_lossy().to_string();
            match self.engine.begin_scan(text.clone()) {
                Ok(()) => self.ui.notice(format!("Indexing {}…", places::folder_name(&text))),
                Err(error) => self.ui.notice(error.to_string()),
            }
        }
    }

    /// The actions a phone answers; the desktop-only ones say so rather than doing nothing.
    fn apply(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::SetTheme(kind) if kind != self.applied_theme => {
                apply_theme(ctx, kind);
                self.applied_theme = kind;
            }
            Action::SetTheme(_) => {}
            Action::SetDensity(_) | Action::SetGrid(_) | Action::SetMotion(_) => {}
            Action::SetSort(sort) => self.ui.sort = sort,
            Action::Notice(text) => self.ui.notice(text),
            Action::PickFolder => self.add_device_photos(),
            Action::Rescan(root) => match self.engine.begin_scan(root.clone()) {
                Ok(()) => self.ui.notice(format!("Rescanning {}…", places::folder_name(&root))),
                Err(error) => self.ui.notice(error.to_string()),
            },
            Action::RemoveRoot(root) => match self.engine.remove_root(&root) {
                Ok(count) => self
                    .ui
                    .notice(format!("Removed {count} photos from {}", places::folder_name(&root))),
                Err(error) => self.ui.notice(error.to_string()),
            },
            Action::StartSuggest => match self.engine.begin_batch_suggest() {
                Ok(0) => self.ui.notice("Nothing to suggest: the library is empty"),
                Ok(count) => self.ui.notice(format!("Looking at {count} photo(s)…")),
                Err(error) => self.ui.notice(error.to_string()),
            },
            Action::CancelSuggest => self.engine.cancel_batch(),
            Action::ClearSuggest => self.engine.clear_batch(),
            Action::AcceptSuggestion(id, tag) => match self.engine.accept_suggestion(id, &tag) {
                Ok(true) => self.ui.notice(format!("Tagged “{tag}”")),
                Ok(false) => self.ui.notice(format!("Already tagged “{tag}”")),
                Err(error) => self.ui.notice(error.to_string()),
            },
            Action::SetPack(name, on) => {
                if let Err(error) = self.engine.set_pack(&name, on) {
                    self.ui.notice(error.to_string());
                }
            }
            Action::UndoMove(id) => match self.engine.undo_move(id) {
                Ok(count) => {
                    self.ui.move_receipt = None;
                    self.ui.notice(format!("Put {count} photo(s) back"));
                }
                Err(error) => self.ui.notice(error.to_string()),
            },
            // The desktop's file dialogs, its sync server and its settings file have no phone
            // counterpart yet; each is named so a tap is never silent.
            _ => self.ui.notice("That is not available on the phone yet"),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(count) = self.engine.poll_batch() {
            self.ui.notice(format!("{count} photo(s) have something to suggest"));
            ctx.request_repaint();
        }
        if self.engine.batch_progress().is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
        let actions = sortlens_ui::show(ui, &mut self.ui, &mut self.engine, &mut self.thumbs);
        for action in actions {
            self.apply(&ctx, action);
        }
    }

    /// Every change is appended to the catalog's log as it is made, so the phone loses nothing when
    /// the OS ends the process; this only pushes the buffered tail out on a clean exit.
    fn on_exit(&mut self) {
        let _ = self.engine.close();
    }
}

/// Run the application in whatever window the platform host provides.
///
/// # Errors
/// The windowing system could not start.
pub fn run(options: eframe::NativeOptions, data_dir: PathBuf) -> eframe::Result {
    eframe::run_native("Sortlens", options, Box::new(move |cc| Ok(Box::new(App::new(cc, data_dir)))))
}
