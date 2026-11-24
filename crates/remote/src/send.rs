//! The send queue a phone drives (SL-J04/J05): many photographs to the paired desktop's album,
//! over the one upload protocol, with the keep-or-remove decision riding on every item.
//!
//! `client::Client::upload` is one photograph, proven against the real server in the loopback
//! harness. What the phone app needs on top of it is a *run*: read each photograph's bytes from
//! the device, send it, survive one photograph failing without losing the rest, and report the
//! whole run's progress to an interface that repaints between items. That run lives here — beside
//! the client, not in the app — so the app cannot grow a second upload flow and so the loopback
//! harness can prove the queue against the real server process the way it proves the client.
//!
//! A run owns one worker thread. The interface never blocks on it: it polls [`SendQueue::report`]
//! each frame, cancels with [`SendQueue::cancel`], and reads the run's result once through
//! [`SendQueue::take_outcome`]. Nothing here deletes anything: the ack's
//! [`LocalCopy::Remove`](crate::client::LocalCopy::Remove) is a *decision the caller carries out*,
//! and the outcome names the ids that chose it ([SendOutcome::to_remove]). On a phone the app hands
//! those ids to its own audited delete path, because only the app holds the library — the queue
//! holds a path, not a catalog.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::client::{Client, ClientError, LocalCopy, TcpTransport, UploadItem};

/// One photograph queued for the desktop.
///
/// The bytes are *not* held here: a library of photographs is megabytes a queue should not pin in
/// memory, and the worker reads each file only when its turn comes.
#[derive(Debug, Clone)]
pub struct SendItem {
    /// The device's own photo id, echoed back in a failure so the interface can name the row.
    pub id: u64,
    /// The file name it asks to land under. The server sanitises and never overwrites, so the
    /// ack's published name is the truth, not this.
    pub name: String,
    /// Where the worker reads the bytes from.
    pub path: PathBuf,
    /// The keep-or-remove toggle (SL-J05), chosen when the item is queued and applied on the
    /// ack — including when the transfer needed a resume to get there, because the decision
    /// lives on the item, not the connection.
    pub remove_after_upload: bool,
}

/// What one finished run produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SendOutcome {
    /// Photographs the server accepted and verified.
    pub sent: usize,
    /// Of those, how many the toggle said keep on the device.
    pub kept: usize,
    /// Of those, how many the toggle said remove once the server held the file.
    pub removed: usize,
    /// Photographs the ack said to remove from the device, by the device's own id. The caller
    /// carries the decision out — this crate holds paths, not the library that can audit and
    /// delete a file — and one id appears here only when the server confirmed the bytes.
    pub to_remove: Vec<u64>,
    /// Photographs that did not land, with the reason each one failed: a file that cannot be
    /// read, or the server's own refusal for that file. The run continues past these.
    pub failed: Vec<(u64, String)>,
    /// True when the person cancelled the run: the remaining items were never attempted.
    pub cancelled: bool,
    /// A connection-level failure that ended the run early — the device could not reach the
    /// desktop at all, so retrying the remaining items would only queue the same failure.
    pub error: Option<String>,
}

/// The progress one run reports while it works, read every frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SendReport {
    /// A worker is alive and still has photographs to send.
    pub running: bool,
    /// Photographs finished this run — landed or failed, both count as finished.
    pub done: usize,
    /// Photographs queued this run.
    pub total: usize,
    /// The photograph the worker is on right now, by name.
    pub current: Option<String>,
}

struct Shared {
    running: bool,
    done: usize,
    total: usize,
    current: Option<String>,
    finished: Option<SendOutcome>,
}

/// A queued run, in flight on its own thread.
///
/// Dropping it does not stop the worker: the run finishes (or is cancelled) and its outcome sits
/// in the shared state until [`SendQueue::take_outcome`] reads it, so a phone that put the app in
/// the background does not lose the answer.
pub struct SendQueue {
    shared: Arc<Mutex<Shared>>,
    cancel: Arc<AtomicBool>,
}

impl SendQueue {
    /// Start a run: pair with the desktop at `addr` (the address its `listening` line announced)
    /// under the device's stored token, and send every item in order on the worker thread.
    pub fn spawn(addr: impl Into<String> + Send + 'static, token: impl Into<String> + Send + 'static, items: Vec<SendItem>) -> Self {
        let shared = Arc::new(Mutex::new(Shared {
            running: true,
            done: 0,
            total: items.len(),
            current: None,
            finished: None,
        }));
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_shared = shared.clone();
        let worker_cancel = cancel.clone();
        let spawned = std::thread::Builder::new()
            .name("sortlens-send".into())
            .spawn(move || run(worker_shared, worker_cancel, addr.into(), token.into(), items));
        if let Err(error) = spawned {
            // No worker means no run: report the reason once instead of a queue that claims to
            // be running forever with nothing behind it.
            finish(
                &shared,
                SendOutcome {
                    error: Some(format!("the send thread could not start: {error}")),
                    ..SendOutcome::default()
                },
            );
        }
        Self { shared, cancel }
    }

    /// The run's progress, for a frame to paint.
    #[must_use]
    pub fn report(&self) -> SendReport {
        let shared = self.shared.lock().ok();
        let Some(shared) = shared else {
            return SendReport::default();
        };
        SendReport {
            running: shared.running,
            done: shared.done,
            total: shared.total,
            current: shared.current.clone(),
        }
    }

    /// Ask the worker to stop after the photograph it is on. A photograph half-sent stays a
    /// server-side partial: the next run resumes it from the received count, nothing is lost.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// The finished run's result, once. `None` while it still works.
    pub fn take_outcome(&mut self) -> Option<SendOutcome> {
        self.shared.lock().ok()?.finished.take()
    }
}

/// The worker's whole body: connect, then one photograph at a time, reporting as it goes.
fn run(shared: Arc<Mutex<Shared>>, cancel: Arc<AtomicBool>, addr: String, token: String, items: Vec<SendItem>) {
    let mut outcome = SendOutcome::default();
    let mut client = Client::paired(token, TcpTransport::new(addr));
    if let Err(error) = client.health() {
        outcome.error = Some(error.to_string());
        finish(&shared, outcome);
        return;
    }
    for item in &items {
        if cancel.load(Ordering::SeqCst) {
            outcome.cancelled = true;
            break;
        }
        {
            let Ok(mut shared) = shared.lock() else { return };
            shared.current = Some(item.name.clone());
        }
        let stop = send_one(&mut client, item, &mut outcome);
        if let Ok(mut shared) = shared.lock() {
            shared.done += 1;
        }
        if stop {
            break;
        }
    }
    if let Ok(mut shared) = shared.lock() {
        shared.current = None;
    }
    finish(&shared, outcome);
}

/// Send one photograph, recording exactly one of the three ways it can go: landed, refused, or
/// the device could not reach the desktop at all. Returns true when the run must stop — a dead
/// connection fails every remaining item the same way, each after its own connect timeout.
fn send_one(client: &mut Client<TcpTransport>, item: &SendItem, outcome: &mut SendOutcome) -> bool {
    let bytes = match std::fs::read(&item.path) {
        Ok(bytes) => bytes,
        Err(error) => {
            outcome.failed.push((item.id, format!("the file could not be read: {error}")));
            return false;
        }
    };
    let upload = UploadItem::new(item.name.clone(), bytes, item.remove_after_upload);
    match client.upload(&upload) {
        Ok(ack) => {
            outcome.sent += 1;
            match ack.local_copy {
                LocalCopy::Keep => outcome.kept += 1,
                LocalCopy::Remove => {
                    outcome.removed += 1;
                    outcome.to_remove.push(item.id);
                }
            }
        }
        Err(ClientError::Connection(reason)) => {
            // The desktop is gone — every remaining item would fail the same way, each one
            // after a connect timeout, so the run ends here and the person reads one reason.
            let reason = format!("could not reach the server: {reason}");
            outcome.failed.push((item.id, reason.clone()));
            outcome.error = Some(reason);
            return true;
        }
        Err(error) => {
            outcome.failed.push((item.id, error.to_string()));
        }
    }
    false
}

/// Publish the result and let the report say the run is over.
fn finish(shared: &Mutex<Shared>, outcome: SendOutcome) {
    if let Ok(mut shared) = shared.lock() {
        shared.finished = Some(outcome);
        shared.current = None;
        shared.running = false;
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// A minimal upload server in-process, over real TCP: the same wire the phone speaks,
    /// without spawning `sortlens-server`. It keeps one partial per transfer id, answers an
    /// intermediate chunk with the received count, and answers the completing chunk — the one
    /// carrying `X-Content-Sha256` — with `saved`.
    struct FakeServer {
        addr: String,
        stop: Arc<AtomicBool>,
    }

    impl FakeServer {
        fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("the fake server binds");
            let addr = listener.local_addr().expect("local addr").to_string();
            let stop = Arc::new(AtomicBool::new(false));
            let worker_stop = stop.clone();
            std::thread::spawn(move || {
                let mut partials: HashMap<String, Vec<u8>> = HashMap::new();
                listener.set_nonblocking(true).expect("nonblocking");
                while !worker_stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            stream.set_nonblocking(false).expect("blocking");
                            if let Some(request) = read_request(&mut stream) {
                                let response = answer(&request, &mut partials);
                                let _ = stream.write_all(response.as_bytes());
                            }
                        }
                        Err(_) => std::thread::sleep(std::time::Duration::from_millis(2)),
                    }
                }
            });
            Self { addr, stop }
        }

        fn addr(&self) -> String {
            self.addr.clone()
        }
    }

    impl Drop for FakeServer {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::SeqCst);
        }
    }

    struct RawRequest {
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    fn read_request(stream: &mut std::net::TcpStream) -> Option<RawRequest> {
        let mut raw = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            match stream.read(&mut byte) {
                Ok(0) => return None,
                Ok(_) => {
                    raw.push(byte[0]);
                    if raw.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                Err(_) => return None,
            }
        }
        let head = String::from_utf8_lossy(&raw).into_owned();
        let mut length = 0usize;
        let mut headers = Vec::new();
        for line in head.lines().skip(1) {
            if let Some((name, value)) = line.split_once(':') {
                if name.trim().eq_ignore_ascii_case("Content-Length") {
                    length = value.trim().parse().unwrap_or(0);
                } else {
                    headers.push((name.trim().to_owned(), value.trim().to_owned()));
                }
            }
        }
        let mut body = vec![0u8; length];
        stream.read_exact(&mut body).ok()?;
        Some(RawRequest { headers, body })
    }

    fn answer(request: &RawRequest, partials: &mut HashMap<String, Vec<u8>>) -> String {
        let header = |name: &str| {
            request
                .headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.clone())
                .unwrap_or_default()
        };
        let id = header("X-Upload-Id");
        let file = partials.entry(id).or_default();
        file.extend_from_slice(&request.body);
        let received = file.len();
        // The completing chunk (the one carrying the digest) earns the ack; every chunk before it
        // earns the received count. Both name their own body's length, the way a real server does.
        let body = if request.headers.iter().any(|(key, _)| key.eq_ignore_ascii_case("X-Content-Sha256")) {
            let digest = header("X-Content-Sha256");
            format!("{{\"ok\":true,\"received\":{received},\"saved\":\"kept.jpg\",\"sha256\":\"{digest}\"}}")
        } else {
            format!("{{\"ok\":true,\"received\":{received}}}")
        };
        format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}", body.len())
    }

    fn item(id: u64, name: &str, path: &std::path::Path, remove: bool) -> SendItem {
        SendItem {
            id,
            name: name.to_owned(),
            path: path.to_path_buf(),
            remove_after_upload: remove,
        }
    }

    fn wait_done(queue: &mut SendQueue) -> SendOutcome {
        for _ in 0..4000 {
            if let Some(outcome) = queue.take_outcome() {
                return outcome;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        panic!("the run finished");
    }

    #[test]
    fn a_run_sends_every_item_and_applies_the_ack_both_ways() {
        let server = FakeServer::start();
        let dir = std::env::temp_dir().join("sortlens-send-ok");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("writable dir");
        let a = dir.join("a.jpg");
        let b = dir.join("b.jpg");
        std::fs::write(&a, vec![1, 2, 3, 4]).expect("writable");
        std::fs::write(&b, vec![5, 6]).expect("writable");
        let mut queue = SendQueue::spawn(
            server.addr(),
            "tok-1",
            vec![item(1, "a.jpg", &a, false), item(2, "b.jpg", &b, true)],
        );
        let outcome = wait_done(&mut queue);
        assert_eq!(outcome.sent, 2);
        assert_eq!(outcome.kept, 1, "the toggle rides on the item, both ways");
        assert_eq!(outcome.removed, 1);
        assert_eq!(
            outcome.to_remove,
            vec![2],
            "the remove decision names the device's own id, so the caller can carry it out"
        );
        assert!(outcome.failed.is_empty());
        assert!(!outcome.cancelled);
        assert_eq!(outcome.error, None);
        let report = queue.report();
        assert_eq!(report.total, 2);
        assert!(!report.running, "a finished run does not claim to be running");
    }

    #[test]
    fn a_file_that_cannot_be_read_is_one_failure_and_the_rest_still_land() {
        let server = FakeServer::start();
        let dir = std::env::temp_dir().join("sortlens-send-missing");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("writable dir");
        let good = dir.join("good.jpg");
        std::fs::write(&good, vec![7, 8]).expect("writable");
        let gone = dir.join("gone.jpg");
        let mut queue = SendQueue::spawn(
            server.addr(),
            "tok-1",
            vec![item(1, "gone.jpg", &gone, false), item(2, "good.jpg", &good, false)],
        );
        let outcome = wait_done(&mut queue);
        assert_eq!(outcome.sent, 1, "one missing file does not lose the run");
        assert_eq!(outcome.failed.len(), 1);
        assert_eq!(outcome.failed[0].0, 1, "the failure names the device's own id");
        assert!(outcome.failed[0].1.contains("could not be read"));
        assert_eq!(outcome.error, None);
        assert!(outcome.to_remove.is_empty(), "nothing landed, so nothing is to be removed");
    }

    #[test]
    fn an_unreachable_desktop_ends_the_run_with_one_reason() {
        // Port 1 on loopback is refused; nothing else may be listening there.
        let dir = std::env::temp_dir().join("sortlens-send-dead");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("writable dir");
        let a = dir.join("a.jpg");
        std::fs::write(&a, vec![1]).expect("writable");
        let mut queue = SendQueue::spawn("127.0.0.1:1", "tok-1", vec![item(1, "a.jpg", &a, false)]);
        let outcome = wait_done(&mut queue);
        assert_eq!(outcome.sent, 0);
        assert!(outcome.error.as_deref().unwrap_or("").contains("could not reach"), "{outcome:?}");
    }

    #[test]
    fn a_cancelled_run_stops_between_items_and_names_itself() {
        let server = FakeServer::start();
        let dir = std::env::temp_dir().join("sortlens-send-cancel");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("writable dir");
        let a = dir.join("a.jpg");
        let b = dir.join("b.jpg");
        std::fs::write(&a, vec![1]).expect("writable");
        std::fs::write(&b, vec![2]).expect("writable");
        let mut queue = SendQueue::spawn(
            server.addr(),
            "tok-1",
            vec![item(1, "a.jpg", &a, false), item(2, "b.jpg", &b, false)],
        );
        queue.cancel();
        let outcome = wait_done(&mut queue);
        assert!(outcome.cancelled, "the person's cancel is the run's own verdict");
        assert_eq!(outcome.sent, 0, "cancel lands before the first item when asked early");
        assert!(outcome.to_remove.is_empty());
        assert!(outcome.error.is_none());
    }

    #[test]
    fn progress_counts_items_as_they_finish() {
        let server = FakeServer::start();
        let dir = std::env::temp_dir().join("sortlens-send-progress");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("writable dir");
        let paths: Vec<_> = (0..3)
            .map(|n| {
                let path = dir.join(format!("p{n}.jpg"));
                std::fs::write(&path, vec![n as u8]).expect("writable");
                path
            })
            .collect();
        let items: Vec<SendItem> = paths
            .iter()
            .enumerate()
            .map(|(n, path)| item(n as u64, &format!("p{n}.jpg"), path, false))
            .collect();
        let mut queue = SendQueue::spawn(server.addr(), "tok-1", items);
        // The queue starts at zero and climbs to the whole run; never past it.
        let first = queue.report();
        assert!(first.total == 3, "the run's total is the queue's own count");
        let outcome = wait_done(&mut queue);
        assert_eq!(outcome.sent, 3);
        assert_eq!(queue.report().done, 3);
    }
}
