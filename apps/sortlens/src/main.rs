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

        Self {
            engine: Engine::new(),
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
            let _ = self.store.flush();
        }
    }
}
