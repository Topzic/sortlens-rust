//! The consent-gated model downloader (SL-F02).
//!
//! The manifest in `sortlens-ai` pins what a model must hash to; this module is the only thing that
//! fetches one. It is deliberately **not** in `sortlens-ai`, whose `no_network` test keeps that
//! crate unable to open a socket.
//!
//! Three properties matter more than convenience:
//!
//! * **Consent first.** Nothing here runs until `Engine::set_models_consent(true)` has been called;
//!   the engine refuses and names the setting. The owner approved this one opt-in network call.
//! * **Resumable.** The bytes stream into `<store>/<digest>.<kind>.part`. A download that stops —
//!   cancelled, or the network dropped — leaves the part file where it is, and the next attempt
//!   asks the server for `Range: bytes=<len>-` and appends. A server that ignores the range is
//!   answered by a restart, not a corrupt model.
//! * **The digest is the contract.** A finished part is verified against the manifest before it is
//!   renamed into place under its digest name, and the renamed file is read back and verified
//!   again. A wrong file never becomes a model, and a model the store cannot verify is never
//!   reported as installed.

use serde::{Deserialize, Serialize};
use sortlens_ai::{Install, Model, Store};
use std::io::{ErrorKind, Read, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// How the read loop checks in: chunks of this size keep a 166 MB model out of RAM and give the
/// cancel flag a regular look.
const CHUNK: usize = 64 * 1024;

/// One HTTP fetch, opened at a byte offset so a download can resume.
pub trait Fetcher: Send + Sync {
    /// Open `url` from `offset`. A server that ignored the range is reported by `resumed: false`,
    /// with the body starting at zero — the caller restarts rather than splicing two files.
    fn fetch(&self, url: &str, offset: u64) -> std::result::Result<Fetched, String>;
}

/// What a fetch returned.
pub struct Fetched {
    /// The server honoured the range; the body starts at the requested offset.
    pub resumed: bool,
    /// The remaining bytes.
    pub reader: Box<dyn Read + Send>,
}

/// The real fetcher: plain HTTP(S) with a `Range` header, via `ureq` and rustls.
pub struct Http;

impl Fetcher for Http {
    fn fetch(&self, url: &str, offset: u64) -> std::result::Result<Fetched, String> {
        let builder = ureq::get(url);
        let builder = if offset > 0 {
            builder.header("Range", &format!("bytes={offset}-"))
        } else {
            builder
        };
        let response = builder.call().map_err(|error| format!("could not reach the server: {error}"))?;
        match response.status() {
            ureq::http::StatusCode::PARTIAL_CONTENT => {
                let (_, body) = response.into_parts();
                Ok(Fetched {
                    resumed: true,
                    reader: Box::new(body.into_reader()),
                })
            }
            ureq::http::StatusCode::OK => {
                // A fresh body. Legal for offset 0; for a resume it means the server ignored the
                // range, which the caller answers by starting over.
                let (_, body) = response.into_parts();
                Ok(Fetched {
                    resumed: false,
                    reader: Box::new(body.into_reader()),
                })
            }
            other => Err(format!("the server answered {other}")),
        }
    }
}

/// How a download ended.
#[derive(Debug)]
pub enum Outcome {
    /// The model is in the store under its digest name.
    Installed(Install),
    /// The user asked it to stop; the part file stays, so the next attempt resumes.
    Cancelled,
    /// It went wrong. A message worth reading, and whether a part file is waiting to resume.
    Failed(String),
}

impl Outcome {
    /// One line for a notice or an activity row.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Outcome::Installed(install) => install.headline(),
            Outcome::Cancelled => "stopped — running the install again resumes where it left off".into(),
            Outcome::Failed(message) => message.clone(),
        }
    }
}

/// Download `model` into `store`, streaming, resumable and cancellable.
///
/// `progress` counts bytes on disk; `cancel` is checked between chunks. A failure keeps the part
/// file (that is what "resumable" means here), except for two cases where the part is useless —
/// the server sent more than the manifest promised, or the finished file is not the model — where
/// keeping it would only guarantee the same failure next time.
pub fn download(store: &Store, model: &Model, fetcher: &dyn Fetcher, progress: &AtomicU64, cancel: &AtomicBool) -> Outcome {
    let part = part_path(store, model);
    let mut start = std::fs::metadata(&part).map(|meta| meta.len()).unwrap_or(0);
    if start > model.bytes {
        // Longer than the model can ever be: not a resume point, not worth keeping.
        let _ = std::fs::remove_file(&part);
        start = 0;
    }
    progress.store(start, Ordering::Relaxed);

    if start == model.bytes {
        // The part already holds every byte: verify and adopt without a fetch.
        return adopt(store, model, &part, progress);
    }
    if model.url.is_empty() {
        return Outcome::Failed(format!("{} has no download source in the manifest", model.id));
    }
    if cancel.load(Ordering::Relaxed) {
        return Outcome::Cancelled;
    }

    let fetched = match fetcher.fetch(&model.url, start) {
        Ok(fetched) => fetched,
        Err(error) => {
            return Outcome::Failed(format!(
                "the download could not start at byte {start}: {error} — the part file is kept, so running it again resumes"
            ));
        }
    };
    if !fetched.resumed && start > 0 {
        // The server ignored the range and sent a fresh body: the part and the reader disagree
        // about what byte 0 means, so start over rather than splice.
        let _ = std::fs::remove_file(&part);
        start = 0;
        progress.store(0, Ordering::Relaxed);
    }

    let mut file = match std::fs::OpenOptions::new().create(true).append(true).open(&part) {
        Ok(file) => file,
        Err(error) => return Outcome::Failed(format!("could not write {}: {error}", part.display())),
    };
    let mut reader = fetched.reader;
    let mut buffer = vec![0u8; CHUNK];
    let mut downloaded = start;
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = craft_atomic::sync_file(&file);
            drop(file);
            return Outcome::Cancelled;
        }
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                if let Err(error) = file.write_all(&buffer[..read]) {
                    drop(file);
                    return Outcome::Failed(format!(
                        "the download stopped at {} of {}: {error} — the part file is kept, so running it again resumes",
                        sortlens_core::format_bytes(downloaded),
                        sortlens_core::format_bytes(model.bytes),
                    ));
                }
                downloaded += read as u64;
                progress.store(downloaded, Ordering::Relaxed);
                if downloaded > model.bytes {
                    drop(file);
                    let _ = std::fs::remove_file(&part);
                    return Outcome::Failed(format!(
                        "the server sent more than the manifest's {} — the part file was removed",
                        sortlens_core::format_bytes(model.bytes),
                    ));
                }
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => {
                let _ = craft_atomic::sync_file(&file);
                drop(file);
                return Outcome::Failed(format!(
                    "the download stopped at {} of {}: {error} — the part file is kept, so running it again resumes",
                    sortlens_core::format_bytes(downloaded),
                    sortlens_core::format_bytes(model.bytes),
                ));
            }
        }
    }
    let _ = craft_atomic::sync_file(&file);
    drop(file);

    if downloaded != model.bytes {
        let _ = std::fs::remove_file(&part);
        return Outcome::Failed(format!(
            "the server sent {} of {} bytes — the part file was removed, so the next attempt starts over",
            sortlens_core::format_bytes(downloaded),
            sortlens_core::format_bytes(model.bytes),
        ));
    }
    adopt(store, model, &part, progress)
}

/// The part file for one model, in the store.
fn part_path(store: &Store, model: &Model) -> std::path::PathBuf {
    store.root().join(format!("{}.part", model.file_name()))
}

/// Verify a finished part and rename it into place under its digest name.
fn adopt(store: &Store, model: &Model, part: &std::path::Path, progress: &AtomicU64) -> Outcome {
    if let Err(reason) = model.verify_file(part) {
        // A complete file that is not the model is not a resume point: keeping it would make every
        // retry end in the same refusal.
        let _ = std::fs::remove_file(part);
        progress.store(0, Ordering::Relaxed);
        return Outcome::Failed(format!(
            "the downloaded file is not the model: {} — the part file was removed, so the next attempt starts over",
            reason.detail()
        ));
    }
    let dest = store.root().join(model.file_name());
    // The rename is the whole installation: same directory, same volume, atomic. On Windows the
    // rename is retried, because an indexer or a scanner briefly holding the new file is normal.
    if let Err(error) = craft_atomic::retry_rename(craft_atomic::RenameRetry::platform(), || std::fs::rename(part, &dest)) {
        return Outcome::Failed(format!(
            "the model verified but could not be moved into place: {error} — the part file is kept"
        ));
    }
    // Read back what the rename landed: the same second look `Store::install` gives its writes.
    if let Err(reason) = model.verify_file(&dest) {
        return Outcome::Failed(format!("the copy on disk is not the model: {}", reason.detail()));
    }
    progress.store(model.bytes, Ordering::Relaxed);
    Outcome::Installed(Install {
        path: dest,
        bytes: model.bytes,
        sha256: model.sha256.clone(),
    })
}

/// One row of Settings → AI: what the model is, and what is on disk for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRow {
    /// Stable id, as the manifest spells it.
    pub id: String,
    /// What a person sees.
    pub label: String,
    /// Kind · licence · size.
    pub summary: String,
    /// What the store found, in words.
    pub status: String,
    /// Whether the store holds a verified copy.
    pub installed: bool,
    /// Whether this model is downloading right now.
    pub downloading: bool,
}

/// A download running on a worker thread.
pub struct Download {
    /// The model id, so the row can be marked as the one in flight.
    pub id: String,
    /// How many bytes have landed.
    pub progress: Arc<AtomicU64>,
    /// The manifest's promised size.
    pub total: u64,
    /// Ask it to stop.
    pub cancel: Arc<AtomicBool>,
    /// The worker's answer.
    pub rx: std::sync::mpsc::Receiver<Outcome>,
}

impl Download {
    /// (done, total), for a progress bar.
    #[must_use]
    pub fn share(&self) -> (u64, u64) {
        (self.progress.load(Ordering::Relaxed), self.total)
    }

    /// Ask the worker to stop; it notices between chunks and keeps the part file.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Start `model` downloading into `store` on a worker thread.
///
/// # Errors
/// Only the spawn can fail; everything the download itself can do wrong comes back through
/// [`Outcome`] on the channel.
pub fn begin_download(store: &Store, model: &Model, fetcher: Arc<dyn Fetcher>) -> std::result::Result<Download, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let progress = Arc::new(AtomicU64::new(0));
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_progress = Arc::clone(&progress);
    let worker_cancel = Arc::clone(&cancel);
    let worker_store = Store::open(store.root()).map_err(|error| format!("the model store could not be opened: {error}"))?;
    let worker_model = model.clone();
    std::thread::Builder::new()
        .name("sortlens-model".into())
        .spawn(move || {
            let outcome = download(&worker_store, &worker_model, fetcher.as_ref(), &worker_progress, &worker_cancel);
            let _ = tx.send(outcome);
        })
        .map_err(|error| format!("could not start the download worker: {error}"))?;
    Ok(Download {
        id: model.id.clone(),
        progress,
        total: model.bytes,
        cancel,
        rx,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use sortlens_ai::{ModelKind, sha256};
    use std::net::TcpListener;
    use std::sync::mpsc::channel;
    use std::time::Duration;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sortlens-engine-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn model_of(bytes: &[u8], url: &str) -> Model {
        Model {
            id: "test-model".into(),
            label: "Test model".into(),
            kind: ModelKind::Detector,
            licence: "Apache-2.0".into(),
            sha256: sha256::to_hex(&sha256::digest(bytes)),
            bytes: bytes.len() as u64,
            url: url.into(),
            input_edge: None,
        }
    }

    /// A loopback HTTP server. No TLS, no network: 127.0.0.1 on an ephemeral port.
    struct Server {
        url: String,
        shutdown: std::sync::mpsc::Sender<()>,
        /// Requests seen: (range header value or None, whether the connection was cut mid-body).
        requests: std::sync::mpsc::Receiver<(Option<String>, bool)>,
    }

    /// How the server answers.
    enum Serve {
        /// Honour the range like a file host does.
        Ranges,
        /// Always answer 200 with the whole body, like a host that cannot resume.
        Whole,
        /// Send `Content-Length` for the whole body but close the connection after `cut` bytes.
        Truncate(usize),
        /// Stream the body a kilobyte at a time with a pause between, so a mid-copy cancel is
        /// observable: the reader blocks between chunks.
        Trickle(u64),
    }

    impl Server {
        fn serve(body: Vec<u8>, behaviour: Serve) -> Server {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
            let port = listener.local_addr().expect("addr").port();
            let (stop_tx, stop_rx) = channel::<()>();
            let (seen_tx, seen_rx) = channel::<(Option<String>, bool)>();
            std::thread::Builder::new()
                .name("test-http".into())
                .spawn(move || {
                    let stop = stop_rx;
                    let _ = listener.set_nonblocking(false);
                    for stream in listener.incoming() {
                        if stop.try_recv().is_ok() {
                            return;
                        }
                        let Ok(mut stream) = stream else { continue };
                        stream
                            .set_read_timeout(Some(Duration::from_secs(5)))
                            .expect("timeout");
                        let mut request = String::new();
                        let mut buffer = [0u8; 1024];
                        loop {
                            match stream.read(&mut buffer) {
                                Ok(0) | Err(_) => break,
                                Ok(read) => {
                                    request.push_str(&String::from_utf8_lossy(&buffer[..read]));
                                    if request.contains("\r\n\r\n") {
                                        break;
                                    }
                                }
                            }
                        }
                        let range = request
                            .lines()
                            .find(|line| line.to_lowercase().starts_with("range:"))
                            .map(|line| line.split_once(':').map(|(_, rest)| rest.trim().to_string()).unwrap_or_default());
                        let start = range
                            .as_ref()
                            .and_then(|value| value.strip_prefix("bytes=").and_then(|rest| rest.split('-').next()))
                            .and_then(|value| value.parse::<usize>().ok())
                            .unwrap_or(0);
                        match &behaviour {
                            Serve::Ranges if start > 0 => {
                                let head = format!(
                                    "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    body.len() - 1,
                                    body.len(),
                                    body.len() - start
                                );
                                let _ = stream.write_all(head.as_bytes());
                                let _ = stream.write_all(&body[start.min(body.len())..]);
                            }
                            Serve::Truncate(cut) => {
                                let head = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    body.len()
                                );
                                let _ = stream.write_all(head.as_bytes());
                                let _ = stream.write_all(&body[..(*cut).min(body.len())]);
                                // The connection closes early, which the reader sees as an error.
                            }
                            Serve::Trickle(step_ms) => {
                                let head = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    body.len()
                                );
                                let _ = stream.write_all(head.as_bytes());
                                for chunk in body.chunks(1024) {
                                    let _ = stream.write_all(chunk);
                                    std::thread::sleep(Duration::from_millis(*step_ms));
                                }
                            }
                            _ => {
                                let head = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    body.len()
                                );
                                let _ = stream.write_all(head.as_bytes());
                                let _ = stream.write_all(&body);
                            }
                        }
                        let _ = seen_tx.send((range, matches!(behaviour, Serve::Truncate(_))));
                    }
                })
                .expect("spawn server");
            Server {
                url: format!("http://127.0.0.1:{port}/model.bin"),
                shutdown: stop_tx,
                requests: seen_rx,
            }
        }

        /// Fetch through the real HTTP client, against the loopback server.
        fn fetcher(&self) -> Http {
            Http
        }
    }

    impl Drop for Server {
        fn drop(&mut self) {
            let _ = self.shutdown.send(());
        }
    }

    fn run_download(store: &Store, model: &Model, server: &Server) -> (Outcome, Arc<AtomicU64>) {
        let progress = Arc::new(AtomicU64::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let outcome = download(store, model, &server.fetcher(), &progress, &cancel);
        (outcome, progress)
    }

    #[test]
    fn a_download_is_verified_and_lands_under_its_digest() {
        let dir = temp("download-ok");
        let bytes: Vec<u8> = (0..80_000u32).map(|index| (index % 251) as u8).collect();
        let server = Server::serve(bytes.clone(), Serve::Ranges);
        let store = Store::open(dir.join("models")).expect("store");
        let model = model_of(&bytes, &server.url);
        let (outcome, progress) = run_download(&store, &model, &server);
        let install = match outcome {
            Outcome::Installed(install) => install,
            other => panic!("expected an install, got {:?}", other.message()),
        };
        assert_eq!(install.bytes as usize, bytes.len());
        assert_eq!(install.sha256, model.sha256);
        assert!(install.path.ends_with(model.file_name()));
        assert!(store.has(&model), "the store verifies the renamed file");
        assert_eq!(progress.load(Ordering::Relaxed), bytes.len() as u64);
        // The part file does not survive a completed install.
        assert!(!part_path(&store, &model).exists());
        assert_eq!(
            store.remove(&model).expect("remove"),
            bytes.len() as u64,
            "and it is removable like any install"
        );
    }

    /// The heart of SL-F02: an interrupted download leaves a part, and the next attempt resumes
    /// from the byte the server last sent, asking for a range.
    #[test]
    fn an_interrupted_download_resumes_from_the_part() {
        let dir = temp("download-resume");
        let bytes: Vec<u8> = (0..120_000u32).map(|index| (index % 251) as u8).collect();
        let cut = 40_000;
        let server = Server::serve(bytes.clone(), Serve::Truncate(cut));
        let store = Store::open(dir.join("models")).expect("store");
        let model = model_of(&bytes, &server.url);

        let (outcome, progress) = run_download(&store, &model, &server);
        match outcome {
            Outcome::Failed(message) => assert!(message.contains("resumes"), "{message}"),
            other => panic!("expected a failure, got {:?}", other.message()),
        }
        let part = part_path(&store, &model);
        assert!(part.exists(), "the part is kept");
        assert_eq!(std::fs::metadata(&part).expect("part").len(), cut as u64);
        assert_eq!(progress.load(Ordering::Relaxed), cut as u64);
        assert!(!store.has(&model), "nothing is installed yet");

        // The second server sees the Range request the resume makes.
        let second = Server::serve(bytes.clone(), Serve::Ranges);
        let model = model_of(&bytes, &second.url);
        let (outcome, _) = run_download(&store, &model, &second);
        assert!(
            matches!(outcome, Outcome::Installed(_)),
            "the resume finishes: {:?}",
            outcome.message()
        );
        assert!(store.has(&model));
        let (range, _) = second.requests.try_recv().expect("a request was seen");
        assert_eq!(range.as_deref(), Some("bytes=40000-"), "the resume asks for the range");
        drop(server);
    }

    #[test]
    fn a_server_that_ignores_the_range_is_answered_by_a_restart() {
        let dir = temp("download-whole");
        let bytes: Vec<u8> = (0..60_000u32).map(|index| (index % 251) as u8).collect();
        // First attempt is truncated, as if the connection dropped.
        let truncating = Server::serve(bytes.clone(), Serve::Truncate(20_000));
        let store = Store::open(dir.join("models")).expect("store");
        let model = model_of(&bytes, &truncating.url);
        let (outcome, _) = run_download(&store, &model, &truncating);
        assert!(matches!(outcome, Outcome::Failed(_)), "{:?}", outcome.message());
        drop(truncating);

        // The resume is answered by a server that sends 200 and the whole body anyway.
        let whole = Server::serve(bytes.clone(), Serve::Whole);
        let whole_url = whole.url.clone();
        let model = model_of(&bytes, &whole_url);
        let (outcome, _) = run_download(&store, &model, &whole);
        assert!(
            matches!(outcome, Outcome::Installed(_)),
            "a 200 answer restarts the download: {:?}",
            outcome.message()
        );
        assert!(store.has(&model));
    }

    #[test]
    fn wrong_bytes_are_refused_and_the_part_is_removed() {
        let dir = temp("download-wrong");
        let bytes: Vec<u8> = (0..50_000u32).map(|index| (index % 251) as u8).collect();
        let server = Server::serve(bytes.clone(), Serve::Ranges);
        let store = Store::open(dir.join("models")).expect("store");
        // The manifest describes different bytes than the server serves.
        let mut model = model_of(&bytes, &server.url);
        let other = sha256::to_hex(&sha256::digest(b"some other weights"));
        model.sha256 = other.clone();
        let (outcome, progress) = run_download(&store, &model, &server);
        match outcome {
            Outcome::Failed(message) => {
                assert!(message.contains("not the model"), "{message}");
                assert!(message.contains("removed"), "a wrong part is not kept: {message}");
            }
            other => panic!("expected a refusal, got {:?}", other.message()),
        }
        assert!(!part_path(&store, &model).exists());
        assert_eq!(progress.load(Ordering::Relaxed), 0, "progress resets with the part");
        assert!(!store.has(&model));
    }

    /// A cancel between chunks keeps the part file exactly where progress got to, so the next
    /// attempt resumes.
    #[test]
    fn a_cancel_mid_copy_keeps_the_part() {
        let dir = temp("download-cancel");
        let bytes: Vec<u8> = (0..120_000u32).map(|index| (index % 251) as u8).collect();
        let server = Server::serve(bytes.clone(), Serve::Trickle(5));
        let store = Store::open(dir.join("models")).expect("store");
        let model = model_of(&bytes, &server.url);
        let progress = Arc::new(AtomicU64::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        // Let a few chunks land, then cancel; the worker is the same thread as here, so the flag
        // is set from a helper thread while `download` is blocked in `read`.
        let flag = Arc::clone(&cancel);
        let watched = Arc::clone(&progress);
        let setter = std::thread::spawn(move || {
            loop {
                if watched.load(Ordering::Relaxed) > 2048 {
                    flag.store(true, Ordering::Relaxed);
                    return;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        });
        let outcome = download(&store, &model, &server.fetcher(), &progress, &cancel);
        setter.join().expect("setter");
        assert!(matches!(outcome, Outcome::Cancelled), "{:?}", outcome.message());
        let part = part_path(&store, &model);
        assert!(part.exists(), "the part survives the cancel");
        let kept = std::fs::metadata(&part).expect("part").len();
        assert!(kept > 0 && kept < bytes.len() as u64, "progress stopped mid-copy: {kept}");
        assert_eq!(progress.load(Ordering::Relaxed), kept);
        assert!(!store.has(&model));

        // And the resume finishes the job from that byte.
        let second = Server::serve(bytes.clone(), Serve::Ranges);
        let model = model_of(&bytes, &second.url);
        let (outcome, _) = run_download(&store, &model, &second);
        assert!(matches!(outcome, Outcome::Installed(_)), "{:?}", outcome.message());
        let (range, _) = second.requests.try_recv().expect("a request was seen");
        assert_eq!(range.as_deref(), Some(format!("bytes={kept}-")).as_deref());
    }

    #[test]
    fn a_stale_part_that_is_too_long_is_not_trusted() {
        let dir = temp("download-stale");
        let bytes = b"real model bytes";
        let server = Server::serve(bytes.to_vec(), Serve::Ranges);
        let store = Store::open(dir.join("models")).expect("store");
        let model = model_of(bytes, &server.url);
        let part = part_path(&store, &model);
        std::fs::write(&part, vec![9u8; bytes.len() + 100]).expect("write a stale part");
        let progress = Arc::new(AtomicU64::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let outcome = download(&store, &model, &server.fetcher(), &progress, &cancel);
        assert!(matches!(outcome, Outcome::Installed(_)), "{:?}", outcome.message());
        assert!(store.has(&model));
    }

    #[test]
    fn a_manifest_without_a_url_is_refused_before_anything_is_fetched() {
        let dir = temp("download-nourl");
        let store = Store::open(dir.join("models")).expect("store");
        let model = model_of(b"weights", "");
        let progress = Arc::new(AtomicU64::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let outcome = download(&store, &model, &Http, &progress, &cancel);
        match outcome {
            Outcome::Failed(message) => assert!(message.contains("no download source"), "{message}"),
            other => panic!("expected a refusal, got {:?}", other.message()),
        }
    }

    /// The worker path: a download on its own thread, polled like the engine polls it, finishing
    /// through the channel with the store holding the model.
    #[test]
    fn the_worker_polls_to_a_finished_install() {
        let dir = temp("download-worker");
        let bytes: Vec<u8> = (0..70_000u32).map(|index| (index % 251) as u8).collect();
        let server = Server::serve(bytes.clone(), Serve::Ranges);
        let store = Store::open(dir.join("models")).expect("store");
        let model = model_of(&bytes, &server.url);
        let started = begin_download(&store, &model, Arc::new(Http)).expect("start");
        assert_eq!(started.share(), (0, bytes.len() as u64));
        let mut outcome = None;
        for _ in 0..600 {
            if let Ok(result) = started.rx.try_recv() {
                outcome = Some(result);
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let outcome = outcome.expect("the worker answered");
        assert!(matches!(outcome, Outcome::Installed(_)), "{:?}", outcome.message());
        assert!(store.has(&model));
        assert_eq!(started.share().1, bytes.len() as u64);
    }
}
