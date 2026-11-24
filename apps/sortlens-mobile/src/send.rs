//! The phone's link to a paired desktop (SL-J04/J05 from the app).
//!
//! One link, one pairing, one queue. The protocol is `sortlens_remote`'s — the same wire the
//! loopback harness proves against the real server — and this module is only the state around it:
//! the pairing a person typed a code into, the queue a send action fills, and the surface the
//! shell's Settings section paints every frame. The link never blocks a frame: pairing and
//! sending each run on their own thread and are polled, the way the engine's scan is.
//!
//! The keep-or-remove toggle (SL-J05) rides on every queued photograph and the ack answers with
//! the decision. This module carries the "remove" half out through the engine's audited delete
//! path ([remove_from_phone]), which re-checks each file before touching it and sends it to the
//! phone's trash; a file whose bytes moved since it was indexed is refused rather than destroyed.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use sortlens_core::PhotoId;
use sortlens_cull::trash::Mode;
use sortlens_engine::Engine;
use sortlens_remote::client::{Client, TcpTransport};
use sortlens_remote::send::{SendItem, SendOutcome, SendQueue};
use sortlens_ui::SendSurface;

/// The name this phone records itself under when it pairs.
pub const DEVICE: &str = "Sortlens phone";

/// What a pairing attempt reported, once it lands.
type PairResult = Result<(String, String), String>;

/// One pairing attempt in flight: the address it aimed at and the worker's answer slot.
///
/// The address is kept here, not applied when the tap lands: a failed pairing must not leave the
/// phone pointing at a desktop it never paired with, and a successful one must remember the
/// address the token belongs to (the settings file stores both or neither).
struct PairAttempt {
    host: String,
    port: u16,
    answer: Arc<Mutex<Option<PairResult>>>,
}

/// One connection test in flight (`GET /health`, no token, SL-J07): the address it asked and
/// the worker's answer slot. It exists so a person can learn that the address is wrong before
/// a pairing code is spent on it — the test stores nothing either way.
struct Check {
    host: String,
    port: u16,
    answer: Arc<Mutex<Option<Result<(), String>>>>,
}

/// The link: the paired desktop's address and token, the queue in flight, and the last run's
/// sentence. Built from the settings document at launch, so a pairing survives the OS ending
/// the process.
pub struct DesktopLink {
    host: String,
    port: u16,
    token: Option<String>,
    device: String,
    /// The send run in flight, if there is one.
    queue: Option<SendQueue>,
    /// The pairing attempt in flight, if there is one.
    pairing: Option<PairAttempt>,
    /// The connection test in flight, if there is one.
    check: Option<Check>,
    /// What the last connection test answered, as the one sentence the section paints.
    check_last: Option<String>,
    /// What the last finished run produced, as the one sentence the section paints.
    last: Option<String>,
}

impl DesktopLink {
    /// Open the link from the settings document: paired exactly when a token is stored.
    #[must_use]
    pub fn open(host: &str, port: u16, token: &str, device: &str) -> Self {
        Self {
            host: host.to_owned(),
            port,
            token: (!token.is_empty()).then(|| token.to_owned()),
            device: device.to_owned(),
            queue: None,
            pairing: None,
            check: None,
            check_last: None,
            last: None,
        }
    }

    /// The desktop's address, for the settings document to keep current.
    #[must_use]
    pub fn config(&self) -> (&str, u16, &str, &str) {
        (&self.host, self.port, self.token.as_deref().unwrap_or(""), &self.device)
    }

    /// Try to pair: trade the code the desktop printed for a scoped token, on the worker
    /// thread. Poll [`DesktopLink::poll_pair`] until it answers.
    pub fn pair(&mut self, host: &str, port: u16, code: &str) {
        if self.queue.as_ref().is_some_and(|queue| queue.report().running) || self.pairing.is_some() {
            return;
        }
        let answer: Arc<Mutex<Option<PairResult>>> = Arc::new(Mutex::new(None));
        let worker = answer.clone();
        let address = host.to_owned();
        let worker_host = address.clone();
        let code = code.to_owned();
        let spawned = std::thread::Builder::new().name("sortlens-pair".into()).spawn(move || {
            let mut client = Client::new(TcpTransport::new(format!("{worker_host}:{port}")));
            let result = match client.pair(&code, DEVICE) {
                Ok(()) => Ok((
                    client.token().unwrap_or_default().to_owned(),
                    client.device().unwrap_or(DEVICE).to_owned(),
                )),
                Err(error) => Err(error.to_string()),
            };
            if let Ok(mut slot) = worker.lock() {
                *slot = Some(result);
            }
        });
        if let Err(error) = spawned {
            // A thread that never starts still owes the caller an answer; say so where the next
            // poll reads it instead of leaving an attempt that never lands.
            if let Ok(mut slot) = answer.lock() {
                *slot = Some(Err(format!("the pairing thread could not start: {error}")));
            }
        }
        self.pairing = Some(PairAttempt {
            host: address,
            port,
            answer,
        });
    }

    /// Read the pairing attempt's answer, if it landed this frame. The caller persists the
    /// token (or shows the reason); the code is never kept past this call.
    ///
    /// The attempt is taken only once its answer is in hand: a frame that polls before the worker
    /// has written one must leave the attempt for the next frame, or the handle — and with it the
    /// answer — would be dropped by the first repaint after the tap.
    pub fn poll_pair(&mut self) -> Option<PairResult> {
        let attempt = self.pairing.as_ref()?;
        let answer = attempt.answer.lock().ok()?.take()?;
        let host = attempt.host.clone();
        let port = attempt.port;
        self.pairing = None;
        if let Ok((token, device)) = &answer {
            // The token and the address it belongs to land together: a settings file with one and
            // not the other would reopen paired against nothing.
            self.host = host;
            self.port = port;
            self.token = Some(token.clone());
            self.device = device.clone();
            self.last = None;
            // The pairing's own answer is now the truth about this address; the check's
            // sentence would only contradict it.
            self.check_last = None;
        }
        Some(answer)
    }

    /// A pairing attempt is still in flight.
    #[must_use]
    pub fn pairing(&self) -> bool {
        self.pairing.is_some()
    }

    /// Ask the desktop at `host:port` whether it is there: `GET /health`, no token, on the
    /// worker thread. Run before a code is spent, because a wrong address fails the pairing
    /// after it and says nothing better. Poll [`DesktopLink::poll_test`] for the sentence;
    /// nothing is stored either way, and a pairing, a send or another test in flight
    /// answers nothing rather than piling threads on the same server.
    pub fn test(&mut self, host: &str, port: u16) {
        if self.pairing.is_some() || self.queue.as_ref().is_some_and(|queue| queue.report().running) || self.check.is_some() {
            return;
        }
        let answer: Arc<Mutex<Option<Result<(), String>>>> = Arc::new(Mutex::new(None));
        let worker = answer.clone();
        let addr = format!("{host}:{port}");
        let spawned = std::thread::Builder::new().name("sortlens-check".into()).spawn(move || {
            let mut client = Client::new(TcpTransport::new(addr));
            let result = client.health().map(|_| ()).map_err(|error| error.to_string());
            if let Ok(mut slot) = worker.lock() {
                *slot = Some(result);
            }
        });
        if let Err(error) = spawned {
            // A thread that never starts still owes the caller an answer; the sentence the
            // next poll reads is the reason.
            if let Ok(mut slot) = answer.lock() {
                *slot = Some(Err(format!("the test thread could not start: {error}")));
            }
        }
        self.check = Some(Check {
            host: host.to_owned(),
            port,
            answer,
        });
    }

    /// Read the connection test's sentence, if it landed this frame. The sentence names the
    /// address it tested — which need not be the paired one — and nothing is stored either
    /// way: the pairing's own answer will be the truth about that.
    pub fn poll_test(&mut self) -> Option<String> {
        let check = self.check.as_ref()?;
        let answer = check.answer.lock().ok()?.take()?;
        let host = check.host.clone();
        let port = check.port;
        self.check = None;
        let sentence = match answer {
            Ok(()) => format!("The desktop at {host}:{port} answered"),
            Err(reason) => format!("No answer from {host}:{port}: {reason}"),
        };
        self.check_last = Some(sentence.clone());
        Some(sentence)
    }

    /// A connection test is still in flight.
    #[must_use]
    pub fn checking(&self) -> bool {
        self.check.is_some()
    }

    /// Forget the paired desktop: token, address and recorded name are dropped. A run already in
    /// flight is stopped rather than left uploading under a token this phone no longer keeps.
    pub fn forget(&mut self) {
        if let Some(queue) = &self.queue {
            queue.cancel();
        }
        self.pairing = None;
        self.check = None;
        self.check_last = None;
        self.host.clear();
        self.port = 0;
        self.token = None;
        self.device.clear();
        self.last = None;
    }

    /// Queue a run: every item is sent in order on the worker thread, the keep-or-remove
    /// choice riding on each. One run at a time; the person stops it explicitly.
    pub fn send(&mut self, items: Vec<SendItem>) {
        let Some(token) = self.token.clone() else { return };
        if self.queue.as_ref().is_some_and(|queue| queue.report().running) {
            return;
        }
        self.last = None;
        self.queue = Some(SendQueue::spawn(
            format!("{host}:{port}", host = self.host, port = self.port),
            token,
            items,
        ));
    }

    /// Stop the in-flight run. The photograph it is on finishes or stays a server-side
    /// partial; the rest are never attempted.
    pub fn cancel(&self) {
        if let Some(queue) = &self.queue {
            queue.cancel();
        }
    }

    /// What the shell paints this frame: the link's state, the run's progress and the last
    /// finished run's sentence. Reading the surface never consumes the run's outcome; the host
    /// takes that through [`DesktopLink::take_outcome`] so it can carry the ack's decisions out.
    pub fn surface(&self) -> SendSurface {
        let report = self.queue.as_ref().map(SendQueue::report).unwrap_or_default();
        SendSurface {
            paired: self.token.is_some(),
            host: self.host.clone(),
            port: self.port,
            sending: report.running,
            done: report.done,
            total: report.total,
            current: report.current,
            last: self.last.clone(),
            checking: self.check.is_some(),
            check: self.check_last.clone(),
        }
    }

    /// The finished run's result, once. The host carries [`SendOutcome::to_remove`] out through
    /// [`remove_from_phone`] and then records the whole story with [`DesktopLink::set_last`].
    pub fn take_outcome(&mut self) -> Option<SendOutcome> {
        self.queue.as_mut()?.take_outcome()
    }

    /// Record the sentence the Settings section paints for the last finished run. The host owns
    /// the wording because only it knows whether the ack's removals could be carried out.
    pub fn set_last(&mut self, text: String) {
        self.last = Some(text);
    }
}

/// The one sentence a finished run earns, failures named the way a notice would read them. The
/// ack's remove decisions are still decisions here — the host appends what it could carry out.
pub(crate) fn sentence(outcome: &SendOutcome) -> String {
    let mut text = if outcome.sent == 0 {
        "Nothing was sent".to_owned()
    } else {
        format!(
            "Sent {} photo{}: {} kept, {} to remove",
            outcome.sent,
            if outcome.sent == 1 { "" } else { "s" },
            outcome.kept,
            outcome.removed
        )
    };
    if !outcome.failed.is_empty() {
        text.push_str(&format!("; {} failed: {}", outcome.failed.len(), outcome.failed[0].1.clone()));
    }
    if let Some(error) = &outcome.error {
        text.push_str("; ");
        text.push_str(error);
    }
    if outcome.cancelled {
        text.push_str(" (stopped)");
    }
    text
}

/// What carrying the ack's remove decisions out produced.
pub struct Removal {
    /// The coda for the run's sentence; empty when the run kept everything.
    pub coda: String,
    /// The notice to show when there was removal work, already phrased for the person.
    pub notice: Option<String>,
}

/// Carry out the ack's remove decisions through the engine's audited delete path.
///
/// The mode is the host's choice; the phone passes Trash, so a removal is recoverable outside
/// Sortlens and a file whose bytes moved since it was indexed is refused rather than destroyed.
pub fn remove_from_phone(engine: &mut Engine, outcome: &SendOutcome, mode: Mode) -> Removal {
    if outcome.to_remove.is_empty() {
        return Removal {
            coda: String::new(),
            notice: None,
        };
    }
    let ids: Vec<PhotoId> = outcome.to_remove.iter().copied().map(PhotoId).collect();
    let chosen = ids.len();
    match engine.delete_photos(&ids, mode) {
        Ok(report) => {
            let removed = report.applied.len();
            let failed = report.failed.len();
            let reason = report.failed.first().map(|(_, reason)| reason.clone());
            let mut coda = if removed == chosen {
                format!("; removed {removed} from this phone")
            } else if removed == 0 {
                format!("; none of the {chosen} were removed from this phone")
            } else {
                format!("; removed {removed} of {chosen} from this phone")
            };
            if let Some(reason) = &reason {
                coda.push_str(&format!(" ({failed} could not be removed: {reason})"));
            }
            let notice = match &reason {
                Some(reason) => format!("Could not remove {failed} photo(s): {reason}"),
                None => format!("Removed {removed} photo(s) from this phone"),
            };
            Removal {
                coda,
                notice: Some(notice),
            }
        }
        Err(error) => Removal {
            coda: format!("; none of the {chosen} were removed from this phone: {error}"),
            notice: Some(format!("Could not remove {chosen} photo(s): {error}")),
        },
    }
}

/// Build the send items for the chosen photographs: name and path from the catalog, the
/// keep-or-remove choice the same for the whole batch. A photograph with no path — a file the
/// last scan found missing — is skipped here, and the queue's own read then cannot fail on it.
pub fn items_for(ids: &[PhotoId], engine: &Engine, remove: bool) -> Vec<SendItem> {
    ids.iter()
        .filter_map(|id| {
            let photo = engine.photo(*id)?;
            let path: PathBuf = engine.path_of(*id)?;
            Some(SendItem {
                id: id.0,
                name: photo.name.clone(),
                path,
                remove_after_upload: remove,
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::prefs::{DesktopPrefs, open as open_prefs};
    use sortlens_core::Flag;
    use sortlens_testkit::Fixture;

    /// An unpaired link answers an honest surface and queues nothing.
    #[test]
    fn an_unpaired_link_queues_nothing() {
        let mut link = DesktopLink::open("", 0, "", "");
        let surface = link.surface();
        assert!(!surface.paired);
        assert!(!surface.sending);
        assert!(surface.last.is_none());
        // The send is refused before any thread exists, and the surface stays the same.
        link.send(vec![SendItem {
            id: 1,
            name: "a.jpg".into(),
            path: PathBuf::from("nowhere.jpg"),
            remove_after_upload: false,
        }]);
        assert!(!link.surface().sending, "an unpaired send never starts a run");
        assert_eq!(link.config(), ("", 0, "", ""));
    }

    /// A failed pairing leaves the link unpaired: the token that never arrived is not stored,
    /// and the reason is what the caller reads.
    #[test]
    fn a_failed_pairing_stores_nothing() {
        // A port that accepts and closes at once: no token ever comes back, deterministically
        // and without waiting out a connect timeout.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the refusing port binds");
        let addr = listener.local_addr().expect("local addr").to_string();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_stop = stop.clone();
        std::thread::spawn(move || {
            while !worker_stop.load(std::sync::atomic::Ordering::SeqCst) {
                if let Ok((stream, _)) = listener.accept() {
                    drop(stream);
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(2));
                }
            }
        });
        let (host, port) = addr.rsplit_once(':').expect("an address with a port");
        let port: u16 = port.parse().expect("a port number");
        let mut link = DesktopLink::open("", 0, "", "");
        link.pair(host, port, "111111");
        let mut answer = None;
        for _ in 0..20_000 {
            if let Some(result) = link.poll_pair() {
                answer = Some(result);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        let answer = answer.expect("the pairing attempt answers");
        assert!(answer.is_err(), "a closed connection pairs nothing");
        assert!(!link.surface().paired, "a failed pairing leaves the link unpaired");
        assert!(!link.pairing(), "an answer in hand ends the attempt, whatever it says");
        assert!(link.poll_pair().is_none(), "the answer is read once, not twice");
    }

    /// A server that pairs one client: it reads the request head and answers with a token and
    /// the name the desktop recorded. Returns the address to dial and the thread to join.
    fn fake_pair_server(token: &'static str, device: &'static str) -> (std::net::SocketAddr, std::thread::JoinHandle<()>) {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the pairing port binds");
        let addr = listener.local_addr().expect("local addr");
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("a pairing client connects");
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                if stream.read(&mut byte).expect("the request head") == 0 {
                    return;
                }
                head.push(byte[0]);
            }
            // Drain the request body before closing: a socket closed with unread bytes resets
            // the connection on Windows, and the reset can discard the answer the client is
            // about to read.
            let length = String::from_utf8_lossy(&head)
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.trim()
                        .eq_ignore_ascii_case("Content-Length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            let mut body = vec![0u8; length];
            stream.read_exact(&mut body).expect("the request body");
            let answer = format!("{{\"ok\":true,\"token\":\"{token}\",\"device\":\"{device}\"}}");
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{answer}", answer.len());
            stream.write_all(response.as_bytes()).expect("the answer");
        });
        (addr, handle)
    }

    /// An answer that lands on the first poll is not lost, and the address the person typed
    /// lands with the token: a stored token always names the desktop it belongs to.
    #[test]
    fn a_pairing_answer_keeps_the_address_it_paired_with() {
        let (addr, server) = fake_pair_server("tok-9", "Ian's phone");
        let mut link = DesktopLink::open("", 0, "", "");
        link.pair(&addr.ip().to_string(), addr.port(), "448821");
        assert!(link.pairing(), "the attempt is in flight");
        let mut answer = None;
        for _ in 0..20_000 {
            if let Some(result) = link.poll_pair() {
                answer = Some(result);
                break;
            }
            // The first frame after the tap often polls before the worker has connected; the
            // attempt must survive that frame for the answer to arrive at all.
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let (token, device) = answer.expect("the pairing answers").expect("the server pairs it");
        server.join().expect("the fake server finishes");
        assert_eq!(token, "tok-9");
        assert_eq!(device, "Ian's phone");
        let host = addr.ip().to_string();
        assert_eq!(
            link.config(),
            (host.as_str(), addr.port(), "tok-9", "Ian's phone"),
            "the token and the address it belongs to land together"
        );
        assert!(link.surface().paired);
        assert!(!link.pairing(), "the answer ends the attempt");
        assert!(link.poll_pair().is_none(), "the answer is read once");
    }

    /// The link's config is what a settings document needs to reopen the pairing, and Forget
    /// empties it.
    #[test]
    fn the_config_round_trips_through_the_document_and_forget_empties_it() {
        let fixture = Fixture::new("phone-link-config");
        let (mut store, _) = open_prefs(fixture.root());
        store.update(|prefs: &mut crate::prefs::PhonePrefs| {
            prefs.desktop = DesktopPrefs {
                host: "192.168.1.10".into(),
                port: 52_114,
                token: "tok-1".into(),
                device: "Ian's phone".into(),
            };
        });
        store.flush().expect("the settings file is writable");
        let (store, report) = open_prefs(fixture.root());
        assert!(!report.has_problems());
        let d = &store.get().desktop;
        let mut link = DesktopLink::open(&d.host, d.port, &d.token, &d.device);
        assert!(link.surface().paired, "a stored token pairs the link");
        assert_eq!(link.config(), ("192.168.1.10", 52_114, "tok-1", "Ian's phone"));
        link.forget();
        let (host, port, token, device) = link.config();
        assert!(host.is_empty() && port == 0 && token.is_empty() && device.is_empty());
        assert!(!link.surface().paired);
    }

    /// A library with `photos` scanned JPEGs under its own fixture, and the fixture that holds
    /// them (kept alive by the caller so the files outlive the engine).
    fn scanned(name: &str, photos: usize) -> (Fixture, Engine) {
        let fixture = Fixture::new(name);
        for n in 0..photos {
            fixture.jpeg(&format!("Photos/{n}.jpg"), [30 + n as u8 * 20, 70, 150], 32, 32);
        }
        let mut engine = Engine::open(fixture.root().join("library")).expect("library opens");
        engine
            .run(
                "library.addFolder",
                &serde_json::json!({ "path": fixture.root().join("Photos").to_string_lossy() }),
            )
            .expect("scan starts");
        loop {
            if engine.poll().is_some() {
                continue;
            }
            if engine.scan_progress().is_some() {
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            break;
        }
        (fixture, engine)
    }

    /// The send items come from the engine's own catalog: name, path and the batch's one
    /// keep-or-remove choice, and a missing photograph is skipped rather than queued to fail.
    #[test]
    fn the_items_come_from_the_catalog() {
        let (_fixture, mut engine) = scanned("phone-link-items", 2);
        let all = engine.all_photos();
        assert_eq!(all.len(), 2);
        let ids: Vec<PhotoId> = all.iter().map(|photo| photo.id).collect();
        let items = items_for(&ids, &engine, true);
        assert_eq!(items.len(), 2);
        assert!(
            items.iter().all(|item| item.remove_after_upload),
            "the batch's choice rides on every item"
        );
        assert!(
            items.iter().all(|item| std::path::Path::new(&item.path).exists()),
            "every path is the file itself"
        );
        assert!(items.iter().all(|item| item.name.ends_with(".jpg")));
        // A photograph the catalog holds but the disk lost: skipped, not queued to fail.
        std::fs::remove_file(&items[0].path).expect("removable");
        engine.refresh_changed().expect("refresh");
        let after = engine.all_photos();
        let after_ids: Vec<PhotoId> = after.iter().map(|photo| photo.id).collect();
        let items = items_for(&after_ids, &engine, false);
        assert!(items.iter().all(|item| item.path.exists()), "a missing file is not queued");
        engine.close().expect("library closes");
    }

    /// The ack's remove choice reaches the library's audited delete path, for exactly the ids
    /// the ack named; a keep-only run leaves every file alone.
    #[test]
    fn the_ack_s_remove_choice_is_carried_out_on_the_phone() {
        let (fixture, mut engine) = scanned("phone-link-remove", 2);
        let all = engine.all_photos();
        let ids: Vec<PhotoId> = all.iter().map(|photo| photo.id).collect();
        assert_eq!(ids.len(), 2);

        let kept = SendOutcome {
            sent: 2,
            kept: 2,
            ..SendOutcome::default()
        };
        let nothing = remove_from_phone(&mut engine, &kept, Mode::TagOnly);
        assert_eq!(nothing.coda, "", "keep asks for no removal");
        assert!(nothing.notice.is_none(), "and says nothing about one");
        assert_eq!(engine.all_photos().len(), 2, "keep leaves both files in the library");

        let chosen = ids[0];
        let original = engine.path_of(chosen).expect("the chosen photo has a path");
        let outcome = SendOutcome {
            sent: 1,
            removed: 1,
            to_remove: vec![chosen.0],
            ..SendOutcome::default()
        };
        let rejects = fixture.root().join("rejects");
        let removal = remove_from_phone(&mut engine, &outcome, Mode::Move(rejects));
        assert_eq!(removal.coda, "; removed 1 from this phone");
        assert_eq!(
            removal.notice.as_deref(),
            Some("Removed 1 photo(s) from this phone"),
            "the person hears the removal, not just the settings section"
        );
        assert!(!original.exists(), "the phone's copy moved out");
        assert_eq!(
            engine.photo(chosen).expect("a moved photo stays in the catalog").flag,
            Flag::Reject,
            "the library records the removal the way the desktop does"
        );
        assert_eq!(engine.all_photos().len(), 1, "the kept photo is untouched");
        engine.close().expect("library closes");
    }

    /// A server that answers `GET /health` with the ok shape, over real TCP. Returns the
    /// address to dial and the thread to join.
    fn fake_health_server() -> (std::net::SocketAddr, std::thread::JoinHandle<()>) {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the health port binds");
        let addr = listener.local_addr().expect("local addr");
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("a health client connects");
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                if stream.read(&mut byte).expect("the request head") == 0 {
                    return;
                }
                head.push(byte[0]);
            }
            let answer = "{\"ok\":true}";
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{answer}", answer.len());
            stream.write_all(response.as_bytes()).expect("the answer");
        });
        (addr, handle)
    }

    /// Poll until the connection test lands, then give back its sentence.
    fn wait_test(link: &mut DesktopLink) -> Option<String> {
        for _ in 0..20_000 {
            if let Some(sentence) = link.poll_test() {
                return Some(sentence);
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        None
    }

    /// The connection test answers before any code is typed, and stores nothing either way:
    /// the pairing's own answer stays the only truth about the address.
    #[test]
    fn a_connection_test_answers_before_any_code() {
        let (addr, server) = fake_health_server();
        let host = addr.ip().to_string();
        let mut link = DesktopLink::open("", 0, "", "");
        assert!(!link.surface().paired, "the test begins from an unpaired link");
        link.test(&host, addr.port());
        assert!(link.checking(), "the test is in flight");
        let surface = link.surface();
        assert!(surface.checking, "the surface says the test is running");
        assert!(surface.check.is_none(), "nothing is painted before the answer");
        let sentence = wait_test(&mut link).expect("the test answers");
        server.join().expect("the fake server finishes");
        assert_eq!(sentence, format!("The desktop at {host}:{port} answered", port = addr.port()));
        assert!(link.surface().check.as_deref().is_some_and(|s| s.contains("answered")));
        assert!(!link.checking(), "the answer ends the test");
        assert!(!link.surface().paired, "the test pairs nothing");
        assert_eq!(link.config(), ("", 0, "", ""), "nothing is stored either way");
    }

    /// An address that answers nothing earns its own sentence, naming the address and the
    /// reason — the words a person reads before deciding to type a code at all.
    #[test]
    fn a_desktop_that_answers_nothing_says_so() {
        // Port 1 on loopback is refused; nothing else may be listening there.
        let mut link = DesktopLink::open("", 0, "", "");
        link.test("127.0.0.1", 1);
        let sentence = wait_test(&mut link).expect("the test answers");
        assert!(sentence.starts_with("No answer from 127.0.0.1:1: "), "{sentence}");
        assert!(link.surface().check.as_deref().is_some_and(|s| s.starts_with("No answer from")));
        assert_eq!(link.config(), ("", 0, "", ""), "a failed test stores nothing");
    }

    /// A pairing already in flight answers a second test with nothing: one thread talks to
    /// the server at a time, and the pairing's answer is the one that matters.
    #[test]
    fn a_test_does_not_pile_onto_a_pairing_in_flight() {
        let (addr, _server) = fake_pair_server("tok-9", "Ian's phone");
        let mut link = DesktopLink::open("", 0, "", "");
        link.pair(&addr.ip().to_string(), addr.port(), "448821");
        assert!(link.pairing(), "the pairing is in flight");
        link.test("127.0.0.1", 1);
        assert!(!link.checking(), "the test answers nothing while a pairing runs");
        // The pairing itself still lands and is taken, so the fake server's thread finishes.
        for _ in 0..20_000 {
            if link.poll_pair().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    /// A store opened fresh keeps an empty desktop section, and a file from a build that never
    /// heard of it opens unpaired rather than refusing.
    #[test]
    fn a_file_without_the_section_opens_unpaired() {
        let fixture = Fixture::new("phone-link-legacy");
        std::fs::write(
            crate::prefs::path(fixture.root()),
            "schema_version = 1\n\n[appearance]\ntheme = \"paper\"\n",
        )
        .expect("a writable fixture");
        let (store, report) = open_prefs(fixture.root());
        assert!(!report.has_problems(), "an older file is not damaged");
        let d = &store.get().desktop;
        let link = DesktopLink::open(&d.host, d.port, &d.token, &d.device);
        assert!(!link.surface().paired, "no token, no pairing");
    }
}
