//! The client half of the loopback harness (SL-J02, SL-J05): a real server process, and the
//! device side speaking through `sortlens_remote::client` — the code a wasm page will build
//! on, not a second protocol. Pairing, holding the library in memory from `library.snapshot`,
//! annotating through `/api`, an upload that dies mid-transfer and resumes from the server's
//! received count, the keep-or-remove decision applied on the ack, and the thumbnails a page
//! over the wire reads as PNG bytes.

use serde_json::json;
use sortlens_engine::{Engine, Filter, Sort};
use sortlens_remote::client::{Client, LocalCopy, TcpTransport, Transport, UploadItem};
use sortlens_testkit::Fixture;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

// ---------------------------------------------------------------- the library

struct Library {
    _fixture: Fixture,
    library: PathBuf,
    album: PathBuf,
}

impl Library {
    /// Two photographs, scanned by the engine, then the lock released.
    fn build(name: &str) -> Self {
        let fixture = Fixture::new(name);
        fixture.jpeg("Photos/a.jpg", [210, 60, 40], 48, 32);
        fixture.jpeg("Photos/b.jpg", [40, 90, 210], 40, 40);
        let library = fixture.root().join("library");
        let photos = fixture.root().join("Photos");
        let album = fixture.root().join("inbox");
        {
            let mut engine = Engine::open(&library).expect("library opens");
            engine
                .run("library.addFolder", &json!({ "path": photos.to_string_lossy() }))
                .expect("scan starts");
            loop {
                if engine.poll().is_some() {
                    continue;
                }
                if engine.scan_progress().is_some() {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
                break;
            }
            engine.close().expect("library closes");
        }
        Self {
            _fixture: fixture,
            library,
            album,
        }
    }
}

// ---------------------------------------------------------------- the server process

struct Server {
    child: Child,
    addr: String,
    code: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    fn spawn(library: &Library, extra: &[&str]) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sortlens-server"))
            .arg("--library")
            .arg(&library.library)
            .arg("--album")
            .arg(&library.album)
            .arg("--port")
            .arg("0")
            .args(extra)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("the server process starts");
        let stdout = child.stdout.take().expect("stdout is piped");
        let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let shared = lines.clone();
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            use std::io::BufRead;
            for line in reader.lines().map_while(Result::ok) {
                shared.lock().expect("lines lock").push(line);
            }
        });
        let mut addr = None;
        let mut code = None;
        for _ in 0..100 {
            std::thread::sleep(Duration::from_millis(20));
            let buffer = lines.lock().expect("lines lock").clone();
            for line in buffer {
                if let Some(rest) = line.strip_prefix("listening http://") {
                    addr = Some(rest.trim().to_string());
                }
                if let Some(rest) = line.strip_prefix("pairing ") {
                    code = Some(rest.trim().to_string());
                }
            }
            if addr.is_some() && code.is_some() {
                break;
            }
        }
        Server {
            child,
            addr: addr.expect("the server announces its port"),
            code: code.expect("the server prints a code"),
        }
    }

    /// The device-side client the crate under test provides, pointed at the real server.
    fn device(&self) -> Client<TcpTransport> {
        Client::new(TcpTransport::new(&self.addr))
    }
}

// ---------------------------------------------------------------- the tests

/// Pairing through the client, holding the library in memory from the wire's snapshot, and
/// annotating through the same client — the wasm page's whole day, end to end.
#[test]
fn a_device_pairs_holds_the_library_in_memory_and_annotates() {
    let library = Library::build("client-pair-hold");
    let server = Server::spawn(&library, &["--code", "369369"]);
    let mut device = server.device();

    // The connection test needs no token (SL-J07).
    let health = device.health().expect("health");
    assert_eq!(health["ok"], true);

    // A wrong code is refused with the server's reason.
    let error = device.pair("111111", "phone").expect_err("wrong code");
    assert!(error.to_string().contains("401"), "the refusal names the status: {error}");
    assert!(device.token().is_none(), "no token was stored");

    // The right code pairs, and the token is the client's from then on.
    device.pair(&server.code, "phone").expect("pair");
    assert!(device.token().is_some());
    assert_eq!(device.device(), Some("phone"));

    // The snapshot over the wire is exactly what the in-memory engine opens (SL-J02).
    let snapshot = device.snapshot().expect("snapshot");
    let held = Engine::open_memory(Some(&snapshot)).expect("the client holds the library");
    assert!(held.is_memory());
    assert_eq!(held.page(&Filter::default(), Sort::Natural, 0, 10).total, 2);

    // The client annotates through the server, and the change is visible from the wire.
    device
        .call("photo.rate", &json!({ "ids": [1], "stars": 5 }))
        .expect("rate through the wire");
    let page = device.call("library.page", &json!({ "limit": 10 })).expect("page");
    assert_eq!(page["total"], 2);
    let rated = page["photos"]
        .as_array()
        .expect("photos")
        .iter()
        .find(|photo| photo["id"] == 1)
        .expect("photo 1");
    assert_eq!(rated["rating"], 5, "the client's write is on the server: {page}");

    // A fresh pull of the snapshot carries the annotation to a reopened client.
    let again = device.snapshot().expect("snapshot again");
    let reopened = Engine::open_memory(Some(&again)).expect("reopen");
    assert_eq!(reopened.photo(sortlens_core::PhotoId(1)).expect("photo").rating, 5);
}

/// The snapshot is scoped: a token without `read` cannot hold the library.
#[test]
fn a_token_without_read_cannot_pull_the_snapshot() {
    let library = Library::build("client-narrow");
    let server = Server::spawn(&library, &["--code", "246810", "--allow", "annotate"]);
    let mut device = server.device();
    device.pair(&server.code, "phone").expect("pair");
    let error = device.snapshot().expect_err("no read scope");
    assert!(error.to_string().contains("read"), "the refusal names the missing scope: {error}");
    // The same scope covers the thumbnails: without `read` there is no wire to the pixels either.
    let error = device.thumbnail(sortlens_core::PhotoId(1), 64).expect_err("no read scope");
    assert!(error.to_string().contains("read"), "the refusal names the missing scope: {error}");
}

/// A paired device reads thumbnails over the wire: PNG bytes that decode to the photograph's
/// own pixels, the refusals named, and a hole where the file has gone.
#[test]
fn a_paired_device_reads_thumbnails_over_the_wire() {
    let library = Library::build("client-thumbs");
    let server = Server::spawn(&library, &["--code", "147147"]);
    let mut device = server.device();
    device.pair(&server.code, "phone").expect("pair");

    // Photo 1 is the red 48×32 fixture; the answer is a PNG that decodes to its own pixels,
    // never upscaled past the file.
    let png = device.thumbnail(sortlens_core::PhotoId(1), 64).expect("thumbnail");
    assert!(
        png.starts_with(b"\x89PNG"),
        "the wire carries a PNG, not a private format: {:02x?}",
        &png[..8.min(png.len())]
    );
    let decoded = sortlens_thumbs::decode_image(&png, 4096).expect("the PNG decodes");
    assert_eq!((decoded.width, decoded.height), (48, 32), "no upscaling past the file's own pixels");
    for channel in 0..3 {
        let got = decoded.rgba[channel] as i32;
        let want = [210, 60, 40][channel];
        assert!((got - want).abs() <= 2, "channel {channel}: {got} vs the fixture's {want}");
    }

    // The route bounds the edge, and says so.
    let error = device.thumbnail(sortlens_core::PhotoId(1), 8).expect_err("edge below the floor");
    assert!(error.to_string().contains("16"), "the floor is named: {error}");
    let error = device
        .thumbnail(sortlens_core::PhotoId(1), 4096)
        .expect_err("edge above the ceiling");
    assert!(error.to_string().contains("1024"), "the ceiling is named: {error}");

    // A photo the catalog does not hold is a 404, not a panic.
    let error = device.thumbnail(sortlens_core::PhotoId(999), 64).expect_err("unknown photo");
    assert!(error.to_string().contains("no such photo"), "{error}");

    // A photo whose file has gone is a hole the page can paint, not a 500.
    let gone = library._fixture.root().join("Photos/a.jpg");
    std::fs::remove_file(&gone).expect("the fixture is removed");
    let error = device.thumbnail(sortlens_core::PhotoId(1), 64).expect_err("file gone");
    assert!(error.to_string().contains("could not be read"), "a missing file is named: {error}");
}

/// An upload that dies mid-transfer resumes from the server's received count, and the
/// keep-or-remove decision (SL-J05) rides on the item, not the connection: both outcomes
/// land with the ack, including after the resume.
#[test]
fn an_upload_resumes_and_the_ack_decides_the_local_copy() {
    let library = Library::build("client-upload");
    let server = Server::spawn(&library, &["--code", "135713"]);
    let mut device = server.device().with_chunk_size(1024);
    device.pair(&server.code, "phone").expect("pair");

    // A photograph bigger than one chunk, so the transfer has several requests to lose.
    let bytes: Vec<u8> = (0..3 * 1024).map(|step| (step % 251) as u8).collect();
    let item = UploadItem::new("PXL_today.jpg", bytes.clone(), true);
    let token = device.token().expect("paired").to_string();

    // The connection dies after the first chunk: the transport fails from there on.
    let mut dropping = Client::paired(token.clone(), DropAfter(1, TcpTransport::new(&server.addr))).with_chunk_size(1024);
    let error = dropping.upload(&item).expect_err("the connection dropped");
    assert!(
        error.to_string().contains("reach the server"),
        "a dropped transfer is a connection error: {error}"
    );

    // The retry — the same queued item, same id — resumes from the count the server has,
    // and the ack carries the keep-or-remove decision the toggle chose.
    let ack = device.upload(&item).expect("the retry completes");
    assert_eq!(ack.saved, "PXL_today.jpg");
    assert_eq!(ack.local_copy, LocalCopy::Remove, "the toggle said remove, the ack says so");
    assert_eq!(ack.sha256.len(), 64, "the digest the server verified is hex");

    // The file is in the album, byte for byte, and the partial is gone.
    let landed = library.album.join("PXL_today.jpg");
    let on_disk = std::fs::read(&landed).expect("the upload landed in the album");
    assert_eq!(on_disk, bytes, "every byte arrived");
    assert!(
        !library.album.join(".uploads").join(format!("{}.part", item.id)).exists(),
        "the partial was published, not left behind"
    );

    // The same toggle, set to keep, is the default and says so.
    let kept_item = UploadItem::new("PXL_kept.jpg", bytes.clone(), false);
    let ack = device.upload(&kept_item).expect("the kept upload");
    assert_eq!(ack.local_copy, LocalCopy::Keep);
    assert_eq!(ack.saved, "PXL_kept.jpg");
    assert!(library.album.join("PXL_kept.jpg").exists());
    let _ = kept_item;
    let _ = kept_item;
}

/// A wrapper that lets the first N requests through and then reports the connection dead,
/// so the resume test loses a transfer without staging a real network fault.
struct DropAfter(usize, TcpTransport);
impl Transport for DropAfter {
    fn request(
        &mut self,
        method: &str,
        path: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Result<sortlens_remote::client::Response, sortlens_remote::client::ClientError> {
        if self.0 == 0 {
            return Err(sortlens_remote::client::ClientError::Connection(
                "connection reset mid-transfer".into(),
            ));
        }
        self.0 -= 1;
        self.1.request(method, path, headers, body)
    }
}
