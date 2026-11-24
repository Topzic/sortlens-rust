//! The page: one eframe shell over the in-memory engine, paired to its host.
//!
//! A browser page is a device (SL-J02). It cannot read a disk, so its library is
//! [`RemoteLibrary`] — an engine in memory, hydrated from the snapshot the paired server
//! serves over the wire — and its grid is painted from the same PNG bytes the wire carries
//! for thumbnails, decoded through the desktop's own codec. The protocol it speaks is
//! [`sortlens_remote::web::WebClient`]'s; the state it keeps between sessions is
//! [`crate::shell::SavedSession`] in browser storage.

#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions, Vec2};
use js_sys::Uint8Array;
use wasm_bindgen::{JsCast, closure::Closure};
use wasm_bindgen_futures::JsFuture;

use serde_json::json;

use crate::RemoteLibrary;
use crate::shell::{Phase, ShellState};
use sortlens_core::PhotoId;
use sortlens_remote::client::{LocalCopy, UploadItem};
use sortlens_remote::web::WebClient;
use sortlens_ui::{ThemeKind, apply as apply_theme, fonts};

/// The storage key the page keeps its session under.
const SESSION_KEY: &str = "sortlens.session";
/// The thumbnail edge the grid asks for — the same tier the desktop's grid paints.
const GRID_EDGE: u32 = 256;
/// How many thumbnail fetches may be on the wire at once. A page that fires one per photo
/// per frame floods the host; four fills a grid in about a second.
const THUMB_CONCURRENCY: usize = 4;
/// The tile the grid paints each photograph in.
const TILE: f32 = 160.0;

/// Where a finished call lands: `spawn_local` cannot borrow the shell, so every job pushes
/// its answer here and asks for a repaint; the next frame drains it.
#[derive(Debug)]
enum Outcome {
    /// The pairing call answered: the token, or why it was refused.
    Paired(Result<String, String>),
    /// The snapshot call answered: the wire state, or why it was refused.
    Snapshot(Result<String, String>),
    /// One thumbnail answered: the photo and its PNG bytes, or why it was refused.
    Thumb(PhotoId, Result<Vec<u8>, String>),
    /// One photograph — picked or dropped — was read into bytes, or could not be read.
    Read { name: String, answer: Result<Vec<u8>, String> },
    /// One upload answered: the page's own id, and the name the photograph landed under,
    /// or the server's reason it did not.
    Uploaded { id: u64, answer: Result<String, String> },
    /// The re-index the page asked for after its uploads landed (SL-J08), or why it was
    /// refused — the snapshot refresh follows it either way.
    Rescan(Result<(), String>),
}

/// The page: one engine in memory, one state machine, one form.
pub struct WebShell {
    state: ShellState,
    library: Option<RemoteLibrary>,
    outcomes: Rc<RefCell<Vec<Outcome>>>,
    /// The host field of the pairing form, prefilled with the origin that served the page.
    base: String,
    code: String,
    device: String,
    /// A pairing or snapshot call is on the wire; the form's button waits.
    busy: bool,
    thumbs: HashMap<PhotoId, TextureHandle>,
    thumb_failed: HashSet<PhotoId>,
    thumb_inflight: usize,
    /// How many upload transfers are on the wire — the page refreshes its snapshot when
    /// the last one lands, so the new photographs join the grid.
    uploads_inflight: usize,
    /// The hidden `<input type="file">` the Add button clicks, and the change listener
    /// that feeds its files into [`Outcome::Read`]. Both stay alive as long as the page.
    picker: Option<Rc<web_sys::HtmlInputElement>>,
    _picker_change: Option<Closure<dyn FnMut(web_sys::Event)>>,
}

impl WebShell {
    /// The page's first frame: restore what a previous session saved and open from it — a
    /// paired page shows its last snapshot immediately, then refreshes from the wire.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        fonts::install(&cc.egui_ctx);
        apply_theme(&cc.egui_ctx, ThemeKind::Charcoal);

        let saved = storage()
            .and_then(|storage| storage.get_item(SESSION_KEY).ok().flatten())
            .map(|text| crate::shell::SavedSession::parse(&text))
            .unwrap_or_default();
        let state = ShellState::restored(saved);
        let library = state.snapshot.as_deref().and_then(|snapshot| RemoteLibrary::hydrate(snapshot).ok());

        let mut shell = Self {
            state,
            library,
            outcomes: Rc::new(RefCell::new(Vec::new())),
            base: origin(),
            code: String::new(),
            device: "browser".to_string(),
            busy: false,
            thumbs: HashMap::new(),
            thumb_failed: HashSet::new(),
            thumb_inflight: 0,
            uploads_inflight: 0,
            picker: None,
            _picker_change: None,
        };
        if let Some((picker, change)) = install_picker(shell.outcomes.clone(), &cc.egui_ctx) {
            shell.picker = Some(picker);
            shell._picker_change = Some(change);
        }
        shell.fetch_snapshot(&cc.egui_ctx);
        shell
    }

    // ---------------------------------------------------------------- the jobs

    /// Pair with the host's current code. Every job builds its own client: a
    /// [`WebClient`] is a base URL and a token, so calls never share one borrow.
    fn pair(&mut self, ctx: &Context) {
        let code = self.code.trim().to_string();
        if code.is_empty() || self.busy {
            return;
        }
        let device = if self.device.trim().is_empty() {
            "browser".to_string()
        } else {
            self.device.trim().to_string()
        };
        let base = self.base.trim().trim_end_matches('/').to_string();
        let outcomes = self.outcomes.clone();
        let ctx = ctx.clone();
        self.busy = true;
        wasm_bindgen_futures::spawn_local(async move {
            let mut client = WebClient::new(base);
            let answer = client.pair(&code, &device).await;
            let token = answer.as_ref().ok().and_then(|()| client.token().map(str::to_string));
            let outcome = Outcome::Paired(match (answer, token) {
                (Ok(()), Some(token)) => Ok(token),
                (Ok(()), None) => Err("the pairing answer carried no token".to_string()),
                (Err(error), _) => Err(error.to_string()),
            });
            outcomes.borrow_mut().push(outcome);
            ctx.request_repaint();
        });
    }

    /// Ask the host for a fresh snapshot; the page catches up on every boot.
    fn fetch_snapshot(&mut self, ctx: &Context) {
        let Some(token) = self.state.token.clone() else { return };
        if self.busy {
            return;
        }
        let base = self.base.trim().trim_end_matches('/').to_string();
        let outcomes = self.outcomes.clone();
        let ctx = ctx.clone();
        self.busy = true;
        wasm_bindgen_futures::spawn_local(async move {
            let mut client = WebClient::paired(token, base);
            let answer = client.snapshot().await;
            outcomes
                .borrow_mut()
                .push(Outcome::Snapshot(answer.map_err(|error| error.to_string())));
            ctx.request_repaint();
        });
    }

    /// Ask the host to re-index the album the uploads landed in (SL-J08) — the wire way a
    /// device says "the album changed because of me".
    fn rescan(&mut self, ctx: &Context) {
        let Some(token) = self.state.token.clone() else { return };
        let base = self.base.trim().trim_end_matches('/').to_string();
        let outcomes = self.outcomes.clone();
        let ctx = ctx.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let mut client = WebClient::paired(token, base);
            let answer = client.call("library.rescan", &json!({})).await;
            outcomes
                .borrow_mut()
                .push(Outcome::Rescan(answer.map(|_| ()).map_err(|error| error.to_string())));
            ctx.request_repaint();
        });
    }

    /// Fetch one grid thumbnail under the paired token.
    fn fetch_thumb(&mut self, ctx: &Context, id: PhotoId) {
        let Some(token) = self.state.token.clone() else { return };
        let base = self.base.trim().trim_end_matches('/').to_string();
        let outcomes = self.outcomes.clone();
        let ctx = ctx.clone();
        self.thumb_inflight += 1;
        wasm_bindgen_futures::spawn_local(async move {
            let mut client = WebClient::paired(token, base);
            let answer = client.thumbnail(id, GRID_EDGE).await;
            outcomes
                .borrow_mut()
                .push(Outcome::Thumb(id, answer.map_err(|error| error.to_string())));
            ctx.request_repaint();
        });
    }

    /// Send one photograph the page chose: the resumable wire upload the client speaks,
    /// with the keep-or-remove decision riding on the item the moment it was queued.
    fn start_upload(&mut self, ctx: &Context, item: UploadItem) {
        let Some(token) = self.state.token.clone() else { return };
        let id = self.state.enqueue_upload(item.name.clone(), item.bytes.len());
        let base = self.base.trim().trim_end_matches('/').to_string();
        let outcomes = self.outcomes.clone();
        let ctx = ctx.clone();
        self.uploads_inflight += 1;
        wasm_bindgen_futures::spawn_local(async move {
            let mut client = WebClient::paired(token, base);
            let answer = match client.upload(&item).await {
                Ok(ack) => Ok(ack.saved),
                Err(error) => Err(error.to_string()),
            };
            outcomes.borrow_mut().push(Outcome::Uploaded { id, answer });
            ctx.request_repaint();
        });
    }

    // ---------------------------------------------------------------- the answers

    /// Drain what the jobs finished, in the order they finished.
    fn drain(&mut self, ctx: &Context) {
        let finished: Vec<Outcome> = {
            let mut outcomes = self.outcomes.borrow_mut();
            outcomes.drain(..).collect()
        };
        for outcome in finished {
            match outcome {
                Outcome::Paired(Ok(token)) => {
                    self.busy = false;
                    self.state.pair_finished(token);
                    self.persist();
                    self.fetch_snapshot(ctx);
                }
                Outcome::Paired(Err(message)) => {
                    self.busy = false;
                    self.state.failed(message);
                    self.persist();
                }
                Outcome::Snapshot(Ok(snapshot)) => {
                    self.busy = false;
                    match RemoteLibrary::hydrate(&snapshot) {
                        Ok(library) => {
                            self.library = Some(library);
                            self.state.snapshot_finished(snapshot);
                            self.thumbs.clear();
                            self.thumb_failed.clear();
                            self.persist();
                        }
                        Err(error) => self.state.failed(format!("the snapshot could not be opened: {error}")),
                    }
                }
                Outcome::Snapshot(Err(message)) => {
                    self.busy = false;
                    self.state.failed(message);
                }
                Outcome::Thumb(id, Ok(png)) => {
                    self.thumb_inflight = self.thumb_inflight.saturating_sub(1);
                    match texture(ctx, &png) {
                        Ok(handle) => {
                            self.thumbs.insert(id, handle);
                        }
                        Err(error) => {
                            self.thumb_failed.insert(id);
                            self.state.notice = Some(format!("the thumbnail could not be decoded: {error}"));
                        }
                    }
                }
                Outcome::Thumb(id, Err(message)) => {
                    self.thumb_inflight = self.thumb_inflight.saturating_sub(1);
                    self.thumb_failed.insert(id);
                    self.state.notice = Some(message);
                }
                Outcome::Read { name, answer: Ok(bytes) } => {
                    if bytes.is_empty() {
                        self.state.notice = Some(format!("{name} is empty, so there is nothing to send"));
                    } else {
                        let item = UploadItem::new(name, bytes, self.state.remove_after_upload);
                        self.start_upload(ctx, item);
                    }
                }
                Outcome::Read {
                    name,
                    answer: Err(message),
                } => {
                    self.state.notice = Some(format!("{name} could not be read: {message}"));
                }
                Outcome::Uploaded { id, answer: Ok(saved) } => {
                    self.uploads_inflight = self.uploads_inflight.saturating_sub(1);
                    self.state.upload_saved(id, saved);
                    // The last transfer landing is the moment the library changed: ask the
                    // host to re-index the album (SL-J08), and the snapshot refresh
                    // follows the rescan, so the new photographs join the grid.
                    if self.uploads_inflight == 0 {
                        self.rescan(ctx);
                    }
                }
                Outcome::Uploaded { id, answer: Err(message) } => {
                    self.uploads_inflight = self.uploads_inflight.saturating_sub(1);
                    self.state.upload_failed(id, message);
                }
                Outcome::Rescan(Ok(())) => {
                    // The re-index was granted; the snapshot is what the host holds now.
                    self.fetch_snapshot(ctx);
                }
                Outcome::Rescan(Err(message)) => {
                    // A refused re-index is worth naming: the uploads landed, but the host
                    // will not look at the album until someone asks it to.
                    self.state.notice = Some(format!("the re-index was refused: {message}"));
                    self.fetch_snapshot(ctx);
                }
            }
        }
    }

    /// Write the session to browser storage: the token and the snapshot, exactly as they are.
    fn persist(&self) {
        if let Some(storage) = storage() {
            let _ = storage.set_item(SESSION_KEY, &self.state.session().encode());
        }
    }

    /// The photographs the page currently holds, in catalog order.
    fn photos(&self) -> Vec<sortlens_engine::Photo> {
        self.library
            .as_ref()
            .map(|library| library.engine().all_photos())
            .unwrap_or_default()
    }

    /// Keep the grid's thumbnails coming: one fetch per free slot, never two for one photo.
    fn want_thumbs(&mut self, ctx: &Context, photos: &[sortlens_engine::Photo]) {
        let mut budget = THUMB_CONCURRENCY.saturating_sub(self.thumb_inflight);
        for photo in photos {
            if budget == 0 {
                break;
            }
            if self.thumbs.contains_key(&photo.id) || self.thumb_failed.contains(&photo.id) {
                continue;
            }
            self.fetch_thumb(ctx, photo.id);
            budget -= 1;
        }
    }

    // ---------------------------------------------------------------- the painting

    /// Take what the user dropped on the page this frame: each dropped file is one read
    /// job, and the outcome lands with the rest. The list is drained here, so a file can
    /// never be read twice.
    fn take_drops(&mut self, ctx: &Context) {
        let dropped = ctx.input(|input| input.raw.dropped_files.clone());
        ctx.input_mut(|input| input.raw.dropped_files.clear());
        for file in dropped {
            let name = file
                .path()
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "dropped photograph".to_string());
            let outcomes = self.outcomes.clone();
            let ctx = ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let answer = file.bytes_async().await;
                outcomes.borrow_mut().push(Outcome::Read { name, answer });
                ctx.request_repaint();
            });
        }
    }

    /// The uploads strip: the pick-or-drop chooser, the keep-or-remove toggle, and every
    /// row this session queued — what is moving, what landed, what was refused, and what
    /// the ack said about the page's own copy (SL-J05).
    fn uploads_strip(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let add = ui.add_enabled(self.picker.is_some(), egui::Button::new("Add photographs…"));
            if add.clicked() {
                if let Some(picker) = self.picker.as_ref() {
                    let _ = picker.click();
                }
            }
            ui.checkbox(&mut self.state.remove_after_upload, "Remove my copy when the host has it");
            ui.weak("or drop photographs on the page");
        });
        if !self.state.uploads.is_empty() {
            ui.add_space(2.0);
            for record in &self.state.uploads {
                let bytes = if record.total >= 1_048_576 {
                    format!("{:.1} MB", record.total as f64 / 1_048_576.0)
                } else {
                    format!("{} B", record.total)
                };
                let line = match &record.status {
                    crate::shell::UploadStatus::Sending => format!("{} · sending {}…", record.name, bytes),
                    crate::shell::UploadStatus::Saved { saved } => {
                        let copy = match record.decision {
                            LocalCopy::Keep => "your copy is kept",
                            LocalCopy::Remove => "your copy is removed",
                        };
                        format!("{} → saved as {saved} · {copy}", record.name)
                    }
                    crate::shell::UploadStatus::Failed { message } => format!("{} · not sent: {message}", record.name),
                };
                ui.weak(line);
            }
            if !self.state.uploads_inflight() && ui.button("Clear finished").clicked() {
                self.state.clear_finished_uploads();
            }
        }
    }

    fn pairing_form(&mut self, ui: &mut egui::Ui) {
        ui.heading("Sortlens");
        ui.add_space(4.0);
        ui.weak(
            "Pair this page with a Sortlens host: the library stays on the host, and the\
            page holds it in memory from the snapshot the host serves.",
        );
        ui.add_space(12.0);
        let enabled = !self.busy && !self.code.trim().is_empty() && !self.base.trim().is_empty();
        egui::Grid::new("pairing").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
            ui.label("Host");
            ui.add_enabled(!self.busy, egui::TextEdit::singleline(&mut self.base).desired_width(320.0));
            ui.end_row();
            ui.label("Pairing code");
            let code_field = egui::TextEdit::singleline(&mut self.code)
                .desired_width(160.0)
                .hint_text("six digits");
            let response = ui.add_enabled(!self.busy, code_field);
            if !self.code.is_empty() && response.changed() {
                // The code is digits on the wire; a stray space or letter is a mistake to
                // keep, not a mystery for the host to refuse.
                self.code = self.code.chars().filter(|c| c.is_ascii_digit()).collect();
            }
            ui.end_row();
            ui.label("Device name");
            ui.add_enabled(!self.busy, egui::TextEdit::singleline(&mut self.device).desired_width(200.0));
            ui.end_row();
        });
        ui.add_space(8.0);
        let pair = ui.add_enabled(enabled, egui::Button::new(if self.busy { "Pairing…" } else { "Pair" }));
        if pair.clicked() {
            let ctx = ui.ctx().clone();
            self.pair(&ctx);
        }
        ui.add_space(8.0);
        ui.weak(
            "The code is on the host's console (or its pairing file). It is spent once:\
            the token this page keeps is what carries every later call.",
        );
    }

    fn library_grid(&mut self, ui: &mut egui::Ui, photos: &[sortlens_engine::Photo]) {
        if photos.is_empty() {
            ui.weak("The host's library is empty.");
            return;
        }
        ui.weak(format!("{} photograph(s) from the host", photos.len()));
        ui.add_space(4.0);
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for photo in photos {
                    ui.vertical(|ui| {
                        match self.thumbs.get(&photo.id) {
                            Some(handle) => {
                                ui.add(egui::Image::new(handle).fit_to_exact_size(Vec2::splat(TILE)));
                            }
                            None => {
                                let (rect, _) = ui.allocate_exact_size(Vec2::splat(TILE), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 6.0, Color32::from_gray(34));
                                ui.painter().text(
                                    rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "…",
                                    egui::FontId::proportional(20.0),
                                    Color32::from_gray(90),
                                );
                            }
                        }
                        ui.add_space(2.0);
                        ui.set_min_width(TILE);
                        ui.add(egui::Label::new(&photo.name).truncate());
                    });
                    ui.add_space(6.0);
                }
            });
        });
    }
}

impl eframe::App for WebShell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.drain(&ctx);

        // One surface, not stacked panels: a device page is one column — header, notice,
        // then either the pairing form or the grid. A web page gets no window chrome, so
        // there is no bar to dock; the header is the page's own first row.
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| match self.state.phase {
                Phase::Unpaired => {
                    ui.strong("Sortlens");
                    ui.separator();
                    ui.weak("Not paired");
                }
                Phase::Connecting => {
                    ui.strong("Sortlens");
                    ui.separator();
                    ui.weak(if self.state.token.is_some() {
                        "Opening the library…"
                    } else {
                        "Pairing…"
                    });
                }
                Phase::Open => {
                    ui.strong("Sortlens");
                    ui.separator();
                    let photos = self.photos();
                    ui.weak(format!("{} photograph(s) from the host", photos.len()));
                    if ui.add_enabled(!self.busy, egui::Button::new("Refresh")).clicked() {
                        self.fetch_snapshot(&ctx);
                    }
                }
            });
            if let Some(notice) = &self.state.notice {
                ui.colored_label(Color32::KHAKI, notice);
            }
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(8.0);

            match self.state.phase {
                Phase::Open => {
                    let photos = self.photos();
                    self.want_thumbs(&ctx, &photos);
                    self.take_drops(&ctx);
                    self.uploads_strip(ui);
                    ui.add_space(4.0);
                    self.library_grid(ui, &photos);
                }
                Phase::Unpaired | Phase::Connecting => {
                    self.pairing_form(ui);
                }
            }
        });
    }
}

// ---------------------------------------------------------------- helpers

/// The page's browser storage, when it can be reached.
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// The hidden `<input type="file">` the Add button clicks, plus its change listener: the
/// listener turns each picked file into one read job whose [`Outcome::Read`] the next
/// frame drains, then clears the input so picking the same file again still fires. The
/// element and the closure are returned alive; the caller keeps them for the page's life.
fn install_picker(
    outcomes: Rc<RefCell<Vec<Outcome>>>,
    ctx: &Context,
) -> Option<(Rc<web_sys::HtmlInputElement>, Closure<dyn FnMut(web_sys::Event)>)> {
    let document = web_sys::window()?.document()?;
    let input: web_sys::HtmlInputElement = document.create_element("input").ok()?.dyn_into().ok()?;
    let _ = input.set_attribute("type", "file");
    let _ = input.set_attribute("multiple", "multiple");
    let _ = input.style().set_property("display", "none");

    let listener = input.clone();
    let job_ctx = ctx.clone();
    let change = Closure::<dyn FnMut(web_sys::Event)>::new(move |_event: web_sys::Event| {
        let Some(files) = listener.files() else { return };
        for index in 0..files.length() {
            let Some(file) = files.get(index) else { continue };
            let name = file.name();
            let outcomes = outcomes.clone();
            let ctx = job_ctx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let answer = read_file(&file).await;
                outcomes.borrow_mut().push(Outcome::Read { name, answer });
                ctx.request_repaint();
            });
        }
        listener.set_value("");
    });
    let target: &web_sys::EventTarget = input.unchecked_ref();
    target
        .add_event_listener_with_callback("change", change.as_ref().unchecked_ref())
        .ok()?;
    if let Some(body) = document.body() {
        let _ = body.append_child(&input);
    }
    Some((Rc::new(input), change))
}

/// Read one picked or dropped file into bytes, through the browser's own array buffer.
async fn read_file(file: &web_sys::File) -> Result<Vec<u8>, String> {
    let buffer = JsFuture::from(file.array_buffer())
        .await
        .map_err(|error| format!("the file could not be read: {error:?}"))?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

/// The origin that served the page — the pairing form's default host.
fn origin() -> String {
    web_sys::window()
        .and_then(|window| window.location().origin().ok())
        .unwrap_or_default()
}

/// Decode wire PNG bytes into a texture, through the desktop's own codec.
fn texture(ctx: &Context, png: &[u8]) -> Result<TextureHandle, String> {
    let image = craft_image_codecs::decode(png).map_err(|error| format!("image: {error}"))?;
    let (width, height) = (image.width(), image.height());
    if width == 0 || height == 0 {
        return Err("the thumbnail has no pixels".to_string());
    }
    let rgba = image.to_rgba8();
    let handle = ctx.load_texture(
        format!("thumb-{width}x{height}"),
        ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba),
        TextureOptions::LINEAR,
    );
    Ok(handle)
}

/// Boot the page: find the canvas the HTML provides and run the shell in it.
pub async fn boot() -> Result<(), wasm_bindgen::JsValue> {
    let canvas = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("sortlens-canvas"))
        .ok_or_else(|| wasm_bindgen::JsValue::from_str("the page has no #sortlens-canvas canvas"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    let runner = eframe::WebRunner::new();
    runner
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|cc| Ok(Box::new(WebShell::new(cc)))),
        )
        .await
}
