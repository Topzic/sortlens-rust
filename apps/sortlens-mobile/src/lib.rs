//! Sortlens for phones and tablets (SL-J10).
//!
//! One engine, one shell, one catalog format: this crate is the desktop application's host with the
//! desktop removed — no title bar, no file dialogs, no reveal-in-folder, no sync server. What the
//! phone adds is where it looks for photographs ([`places`]), where it keeps the choices the shell
//! offers ([`prefs`]), its link to a paired desktop ([`send`]), and the two entry points a phone
//! host calls ([`entry`]). Everything that decides, ranks, culls or paints is `sortlens-engine`
//! and `sortlens-ui`, the same code the desktop runs.

#![deny(unsafe_code)]

pub mod entry;
pub mod places;
pub mod prefs;
pub mod send;

use std::path::PathBuf;

use craft_settings::store::SettingsStore;
use sortlens_engine::Engine;
use sortlens_ui::{Action, ThemeKind, Thumbnails, UiState, apply as apply_theme, i18n};

use crate::prefs::PhonePrefs;
use crate::send::DesktopLink;

/// The application: one engine, one shell, one settings file.
pub struct App {
    engine: Engine,
    ui: UiState,
    /// The choices the shell offers, kept on the device: the file is what a restart opens from.
    store: SettingsStore<PhonePrefs>,
    thumbs: Thumbnails,
    /// The theme the context currently carries.
    applied_theme: ThemeKind,
    /// The link to a paired desktop, built from the settings document the file reopens.
    link: DesktopLink,
}

impl App {
    /// Open the library and the settings under `data_dir` and style the context. The shell's own
    /// title inset is zero: a phone has no traffic lights to clear.
    #[must_use]
    pub fn new(cc: &eframe::CreationContext<'_>, data_dir: PathBuf) -> Self {
        // The settings first, so the very first frame is already the theme the person chose and the
        // grid opens in the order they left it. A file that could not be read in full is reported
        // before anything else, because its defaults are what is on screen.
        let (store, report) = prefs::open(&data_dir);
        let kind = store.get().theme();
        sortlens_ui::fonts::install(&cc.egui_ctx);
        apply_theme(&cc.egui_ctx, kind);
        let mut ui = UiState::new(kind, 0.0);
        store.get().apply(&mut ui);
        if let Some(text) = prefs::notice(&report) {
            ui.notice(text);
        }
        let engine = match Engine::open(places::library_dir(&data_dir)) {
            Ok(engine) => engine,
            Err(error) => {
                ui.notice(i18n::fmt(
                    "The library could not be opened: {error}",
                    &[("error", &error.to_string())],
                ));
                Engine::new()
            }
        };
        let desktop = store.get().desktop.clone();
        Self {
            engine,
            ui,
            store,
            thumbs: Thumbnails::new(places::cache_dir(&data_dir)),
            applied_theme: kind,
            link: DesktopLink::open(&desktop.host, desktop.port, &desktop.token, &desktop.device),
        }
    }

    /// Begin indexing every folder on the device that holds the user's photographs.
    fn add_device_photos(&mut self) {
        let roots = places::photo_roots();
        if roots.is_empty() {
            self.ui.notice(i18n::tr("No photo folders on this device are readable yet"));
            return;
        }
        for root in roots {
            let text = root.to_string_lossy().to_string();
            match self.engine.begin_scan(text.clone()) {
                Ok(()) => {
                    let folder = places::folder_name(&text);
                    self.ui.notice(i18n::fmt("Indexing {folder}…", &[("folder", &folder)]));
                }
                Err(error) => self.ui.notice(error.to_string()),
            }
        }
    }

    /// The actions a phone answers; the desktop-only ones say so rather than doing nothing.
    fn apply(&mut self, ctx: &egui::Context, action: Action) {
        // Every appearance or order choice is written to the document before it is shown, so the
        // next launch opens the way this one ended: the OS may end the process at any moment, and
        // `tick` is what saves on a debounce (the desktop's own rule).
        prefs::record(&mut self.store, &action);
        match action {
            Action::SetTheme(kind) if kind != self.applied_theme => {
                apply_theme(ctx, kind);
                self.applied_theme = kind;
            }
            Action::SetTheme(_) => {}
            Action::SetDensity(density) => self.ui.density = density,
            Action::SetGrid(mode) => self.ui.grid_mode = mode,
            Action::SetMotion(motion) => self.ui.motion = motion,
            Action::SetSort(sort) => self.ui.sort = sort,
            Action::Notice(text) => self.ui.notice(text),
            Action::PickFolder => self.add_device_photos(),
            Action::Rescan(root) => match self.engine.begin_scan(root.clone()) {
                Ok(()) => {
                    let folder = places::folder_name(&root);
                    self.ui.notice(i18n::fmt("Rescanning {folder}…", &[("folder", &folder)]));
                }
                Err(error) => self.ui.notice(error.to_string()),
            },
            Action::RemoveRoot(root) => match self.engine.remove_root(&root) {
                Ok(count) => {
                    let n = i18n::count(count as u64);
                    let folder = places::folder_name(&root);
                    self.ui
                        .notice(i18n::fmt("Removed {n} photos from {folder}", &[("n", &n), ("folder", &folder)]));
                }
                Err(error) => self.ui.notice(error.to_string()),
            },
            Action::StartSuggest => match self.engine.begin_batch_suggest() {
                Ok(0) => self.ui.notice(i18n::tr("Nothing to suggest: the library is empty")),
                Ok(count) => {
                    let n = i18n::count(count as u64);
                    self.ui.notice(i18n::fmt("Looking at {n} photo(s)…", &[("n", &n)]));
                }
                Err(error) => self.ui.notice(error.to_string()),
            },
            Action::CancelSuggest => self.engine.cancel_batch(),
            Action::ClearSuggest => self.engine.clear_batch(),
            Action::AcceptSuggestion(id, tag) => match self.engine.accept_suggestion(id, &tag) {
                Ok(true) => self.ui.notice(i18n::fmt("Tagged “{tag}”", &[("tag", &tag)])),
                Ok(false) => self.ui.notice(i18n::fmt("Already tagged “{tag}”", &[("tag", &tag)])),
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
                    let n = i18n::count(count as u64);
                    self.ui.notice(i18n::fmt("Put {n} photo(s) back", &[("n", &n)]));
                }
                Err(error) => self.ui.notice(error.to_string()),
            },
            Action::SendPair { host, port, code } => {
                self.link.pair(&host, port, &code);
                self.ui.notice(i18n::tr("Pairing with the desktop…"));
            }
            Action::SendTest { host, port } => {
                self.link.test(&host, port);
                let port = port.to_string();
                self.ui
                    .notice(i18n::fmt("Testing {host}:{port}…", &[("host", &host), ("port", &port)]));
            }
            Action::SendForget => {
                self.link.forget();
                self.save_link();
                self.ui
                    .notice(i18n::tr("The desktop is forgotten; pairing again needs a fresh code"));
            }
            Action::SendToDesktop(ids, remove) => {
                let items = send::items_for(&ids, &self.engine, remove);
                if items.is_empty() {
                    self.ui
                        .notice(i18n::tr("Nothing to send: the chosen photos have no readable files"));
                } else {
                    let n = i18n::count(items.len() as u64);
                    self.ui.notice(i18n::fmt("Sending {n} photo(s) to the desktop…", &[("n", &n)]));
                    self.link.send(items);
                }
            }
            Action::SendCancel => {
                self.link.cancel();
                self.ui.notice(i18n::tr("Stopping the send…"));
            }
            // The desktop's file dialogs, its sync server and its settings file have no phone
            // counterpart yet; each is named so a tap is never silent.
            _ => self.ui.notice(i18n::tr("That is not available on the phone yet")),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // The settings save on a debounce and the tick is what fires it; a phone that went idle
        // after a tap would otherwise hold the change in memory until the OS ends the process.
        let tick = self.store.tick(std::time::Instant::now());
        if let Some(deadline) = tick.next_deadline {
            ctx.request_repaint_after(deadline.saturating_duration_since(std::time::Instant::now()));
        }
        if let Some(count) = self.engine.poll_batch() {
            let n = i18n::count(count as u64);
            self.ui.notice(i18n::fmt("{n} photo(s) have something to suggest", &[("n", &n)]));
            ctx.request_repaint();
        }
        if self.engine.batch_progress().is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
        self.poll_link(&ctx);
        let actions = sortlens_ui::show(ui, &mut self.ui, &mut self.engine, &mut self.thumbs);
        for action in actions {
            self.apply(&ctx, action);
        }
    }

    /// Every change is appended to the catalog's log as it is made, so the phone loses nothing when
    /// the OS ends the process; this only pushes the buffered tails out on a clean exit.
    fn on_exit(&mut self) {
        let _ = self.engine.close();
        let _ = self.store.flush();
    }
}

impl App {
    /// Write the link's current pairing into the settings document, the one place the token
    /// survives the OS ending the process.
    fn save_link(&mut self) {
        let (host, port, token, device) = self.link.config();
        let host = host.to_owned();
        let token = token.to_owned();
        let device = device.to_owned();
        self.store.update(move |prefs| {
            prefs.desktop = crate::prefs::DesktopPrefs { host, port, token, device };
        });
    }

    /// The link's own answers between frames: a pairing that landed (its token is kept on the
    /// device, its reason is said), a finished run (its remove decisions handed to the library's
    /// audited delete path), and the surface the Settings section paints.
    fn poll_link(&mut self, ctx: &egui::Context) {
        if let Some(answer) = self.link.poll_pair() {
            match answer {
                Ok((_, device)) => {
                    self.save_link();
                    self.ui
                        .notice(i18n::fmt("Paired with the desktop as {device}", &[("device", &device)]));
                }
                Err(reason) => self.ui.notice(i18n::fmt("Pairing failed: {reason}", &[("reason", &reason)])),
            }
            ctx.request_repaint();
        }
        if let Some(sentence) = self.link.poll_test() {
            self.ui.notice(sentence);
            ctx.request_repaint();
        }
        if let Some(outcome) = self.link.take_outcome() {
            let removal = send::remove_from_phone(&mut self.engine, &outcome, sortlens_cull::trash::Mode::Trash);
            let mut text = send::sentence(&outcome);
            text.push_str(&removal.coda);
            self.link.set_last(text);
            if let Some(notice) = removal.notice {
                self.ui.notice(notice);
            }
            ctx.request_repaint();
        }
        self.ui.send = self.link.surface();
        if self.ui.send.sending {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
    }
}

/// Run the application in whatever window the platform host provides.
///
/// # Errors
/// The windowing system could not start.
pub fn run(options: eframe::NativeOptions, data_dir: PathBuf) -> eframe::Result {
    eframe::run_native("Sortlens", options, Box::new(move |cc| Ok(Box::new(App::new(cc, data_dir)))))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use sortlens_ui::i18n;

    /// Every route the phone host's own words take, read out of the files themselves: `i18n::tr(`
    /// literals and `i18n::fmt(` templates in `lib.rs` and in `prefs.rs`. The shell's catalogue
    /// audit in `crates/ui` cannot read this crate, so the phone carries its own listing — a key
    /// routed here with no row is a notice that quietly stays English, which is the debt SL-L07
    /// exists to pay.
    fn routed_keys() -> Vec<String> {
        let mut keys = Vec::new();
        // A route's literal may sit on the next line, so the whitespace after the parenthesis is
        // skipped before the opening quote is required.
        // Split here rather than on a literal marker, so these two lines do not list themselves.
        let markers = [["i18n", "::tr("].concat(), ["i18n", "::fmt("].concat()];
        for source in [include_str!("lib.rs"), include_str!("prefs.rs")] {
            for marker in &markers {
                for part in source.split(marker.as_str()).skip(1) {
                    if let Some(rest) = part.trim_start().strip_prefix('"')
                        && let Some(end) = rest.find('"')
                    {
                        keys.push(rest[..end].to_owned());
                    }
                }
            }
        }
        keys
    }

    /// The phone's routes are listed here and every one has a row in all six catalogues.
    #[test]
    fn the_hosts_words_have_a_row_in_every_catalogue() {
        let keys = routed_keys();
        for key in [
            "Settings could not be read in full; defaults are in use",
            "The library could not be opened: {error}",
            "No photo folders on this device are readable yet",
            "Indexing {folder}…",
            "Rescanning {folder}…",
            "Removed {n} photos from {folder}",
            "Nothing to suggest: the library is empty",
            "Looking at {n} photo(s)…",
            "Tagged “{tag}”",
            "Already tagged “{tag}”",
            "Put {n} photo(s) back",
            "Pairing with the desktop…",
            "Testing {host}:{port}…",
            "The desktop is forgotten; pairing again needs a fresh code",
            "Nothing to send: the chosen photos have no readable files",
            "Sending {n} photo(s) to the desktop…",
            "Stopping the send…",
            "That is not available on the phone yet",
            "{n} photo(s) have something to suggest",
            "Paired with the desktop as {device}",
            "Pairing failed: {reason}",
        ] {
            assert!(keys.iter().any(|seen| seen == key), "the listing lost {key:?}");
        }
        assert!(
            keys.len() >= 21,
            "only {} routes found; the audit is not reading the sources",
            keys.len()
        );
        for code in ["ar", "ja", "ko", "th", "vi", "zh-TW"] {
            let lang = i18n::from_code(code).expect("an offered interface language");
            for key in &keys {
                assert_ne!(i18n::I18N.t(lang, key), key, "{code} has no row for the phone host's {key:?}");
            }
        }
    }

    /// The phone's pairing and send notices read the interface language, and a count keeps the
    /// reader's own digits: the same rows the desktop's host paints, reached from a crate its
    /// catalogue audit cannot read.
    #[test]
    fn the_hosts_notices_read_the_interface_language() {
        let english = i18n::from_code("en").expect("English");
        let arabic = i18n::from_code("ar").expect("Arabic");
        i18n::set_current(english);
        assert_eq!(
            i18n::fmt("Pairing failed: {reason}", &[("reason", "denied")]),
            "Pairing failed: denied"
        );
        assert_eq!(
            i18n::tr("That is not available on the phone yet"),
            "That is not available on the phone yet"
        );
        i18n::set_current(arabic);
        let sent = i18n::fmt("Sending {n} photo(s) to the desktop…", &[("n", &i18n::count(3))]);
        assert!(sent.contains('٣'), "the count is in Arabic-Indic digits: {sent}");
        assert!(sent.contains("سطح المكتب"), "the noun is Arabic: {sent}");
        assert!(!sent.contains("photo"), "no English noun is left: {sent}");
        assert_ne!(i18n::tr("Stopping the send…"), "Stopping the send…");
        i18n::set_current(english);
    }
}
