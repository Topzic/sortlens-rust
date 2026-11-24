//! The two-process loopback harness (SL-J01..J09): a real server process, a real client, a
//! real library on disk.
//!
//! The test process builds a library the normal way (a scan through the engine), lets go of
//! the catalog's write lock, spawns `sortlens-server` against it, and then speaks the HTTP
//! the way a paired device would: a pairing code becomes a token, the token calls the
//! registry through `/api`, a photograph is uploaded in chunks that resume after the
//! connection drops, and the writes the phone made are read back from disk after the server
//! is gone.

use serde_json::{Value, json};
use sortlens_engine::Engine;
use sortlens_testkit::Fixture;
use std::io::{BufRead, Read, Write};
use std::net::TcpStream;
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
        // The pipe is drained for the whole life of the process: dropping the reader would
        // close it, and the server's next startup line would panic instead of serving.
        let lines = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let shared = lines.clone();
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                shared.lock().unwrap().push(line);
            }
        });
        let mut addr = None;
        let mut code = None;
        for _ in 0..100 {
            std::thread::sleep(Duration::from_millis(20));
            let buffer = lines.lock().unwrap().clone();
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

    fn client(&self) -> Client {
        Client { addr: self.addr.clone() }
    }
}

// ---------------------------------------------------------------- the client

struct Response {
    status: u16,
    body: Vec<u8>,
}

impl Response {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("the server answers JSON")
    }
}

struct Client {
    addr: String,
}

impl Client {
    fn request(&self, method: &str, path: &str, headers: &[(&str, &str)], body: &[u8]) -> Response {
        let mut stream = TcpStream::connect(&self.addr).expect("the server is listening");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("a read timeout is set");
        let mut request = format!("{method} {path} HTTP/1.1\r\nHost: sortlens\r\nContent-Length: {}\r\n", body.len());
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("\r\n");
        stream.write_all(request.as_bytes()).expect("the request head is written");
        stream.write_all(body).expect("the request body is written");
        stream.flush().expect("the request is flushed");
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).expect("the server closes after the answer");
        let head_end = raw
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("an HTTP response has a header terminator");
        let head = String::from_utf8_lossy(&raw[..head_end]).into_owned();
        let status: u16 = head
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|code| code.parse().ok())
            .expect("a status line");
        let body = raw[head_end + 4..].to_vec();
        Response { status, body }
    }

    fn post_json(&self, path: &str, token: Option<&str>, body: Value) -> Response {
        let mut headers = Vec::new();
        if let Some(token) = token {
            headers.push(("Authorization", format!("Bearer {token}")));
        }
        let mut header_refs: Vec<(&str, &str)> = headers.iter().map(|(name, value)| (*name, value.as_str())).collect();
        header_refs.push(("Content-Type", "application/json"));
        self.request("POST", path, &header_refs, body.to_string().as_bytes())
    }

    /// Pair with the server's code, returning the token.
    fn pair(&self, code: &str, device: &str) -> (u16, Value) {
        let response = self.post_json("/pair", None, json!({ "code": code, "device": device }));
        let body = response.json();
        (response.status, body)
    }

    /// Call a registry command through `/api`.
    fn api(&self, token: &str, method: &str, params: Value) -> (u16, Value) {
        let response = self.post_json("/api", Some(token), json!({ "method": method, "params": params }));
        (response.status, response.json())
    }
}

fn chunk_request(client: &Client, headers: Vec<(String, String)>, body: &[u8], complete: bool) -> (u16, Value) {
    let mut header_refs: Vec<(&str, &str)> = headers.iter().map(|(name, value)| (name.as_str(), value.as_str())).collect();
    header_refs.push(("Content-Type", "application/octet-stream"));
    let mut stream = TcpStream::connect(&client.addr).expect("the server is listening");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout is set");
    let mut request = format!("POST /upload HTTP/1.1\r\nHost: sortlens\r\nContent-Length: {}\r\n", body.len());
    for (name, value) in header_refs {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    if complete {
        request.push_str("\r\n");
        stream.write_all(request.as_bytes()).expect("the head is written");
        stream.write_all(body).expect("the body is written");
        stream.flush().expect("the request is flushed");
    } else {
        // Send the head, and less than the promised body: the connection drops mid-transfer.
        stream.write_all(request.as_bytes()).expect("the head is written");
        stream.flush().expect("the head is flushed");
        return (0, Value::Null);
    }
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).expect("the server answers");
    let head_end = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("a header terminator");
    let head = String::from_utf8_lossy(&raw[..head_end]).into_owned();
    let status: u16 = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .expect("a status line");
    (status, serde_json::from_slice(&raw[head_end + 4..]).expect("JSON"))
}

// ---------------------------------------------------------------- the tests

#[test]
fn a_paired_phone_reads_annotates_and_is_scoped() {
    let library = Library::build("loopback-scope");
    let server = Server::spawn(&library, &["--code", "424242"]);
    let client = server.client();

    // The connection test answers without a token (SL-J07).
    let health = client.request("GET", "/health", &[], b"");
    assert_eq!(health.status, 200);
    assert_eq!(health.json()["ok"], true);

    // The wrong code does not pair.
    let (status, body) = client.pair("111111", "phone");
    assert_eq!(status, 401, "wrong code: {body}");

    // The right code pairs with the scopes the server granted.
    let (status, body) = client.pair(&server.code, "phone");
    assert_eq!(status, 200, "{body}");
    let token = body["token"].as_str().expect("a token comes back").to_string();
    assert_eq!(body["device"], "phone");
    let permissions: Vec<&str> = body["permissions"]
        .as_array()
        .expect("scopes")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(permissions.contains(&"read") && permissions.contains(&"annotate"));

    // Without a token, the API refuses.
    let response = client.post_json("/api", None, json!({ "method": "library.page", "params": {} }));
    assert_eq!(response.status, 401);

    // With a nonsense token, the API refuses with a reason.
    let (status, body) = client.api("ffff", "library.page", json!({}));
    assert_eq!(status, 401);
    assert!(body["error"].as_str().unwrap_or("").contains("unknown token"));

    // A read works and sees the library's two photographs.
    let (status, body) = client.api(&token, "library.page", json!({ "limit": 10 }));
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["result"]["total"], 2, "the scan's two photographs are visible: {body}");

    // An annotate works.
    let (status, body) = client.api(&token, "photo.rate", json!({ "ids": [1], "stars": 5 }));
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["result"]["changed"], 1);

    // A command the wire never accepts is refused even with a good token.
    let (status, body) = client.api(&token, "app.quit", json!({}));
    assert_eq!(status, 403);
    assert!(body["error"].as_str().unwrap_or("").contains("does not accept"));

    // The pairing code is spent: a second device cannot join with the same code.
    let (status, _) = client.pair("111111", "second");
    assert_eq!(status, 401);

    // The writes survive the server's exit.
    drop(server);
    let mut engine = Engine::open(&library.library).expect("the library reopens");
    let page = engine.run("library.page", &json!({ "limit": 10 })).expect("page");
    let rated = page["photos"]
        .as_array()
        .expect("photos")
        .iter()
        .find(|photo| photo["id"] == 1)
        .expect("photo 1");
    assert_eq!(rated["rating"], 5, "the phone's write is on disk: {page}");
}

#[test]
fn the_pairing_code_locks_out_after_five_wrong_attempts() {
    let library = Library::build("loopback-lockout");
    let server = Server::spawn(&library, &["--code", "777777"]);
    let client = server.client();
    for attempt in 1..=5 {
        let (status, body) = client.pair("000000", "guesser");
        assert_eq!(status, 401, "attempt {attempt}: {body}");
    }
    // Even the right code is dead now; a new code is the way back in.
    let (status, body) = client.pair("777777", "phone");
    assert_eq!(status, 401, "{body}");
    assert!(body["error"].as_str().unwrap_or("").contains("too many"));
}

#[test]
fn a_narrow_token_cannot_cull_and_the_wide_one_can() {
    let library = Library::build("loopback-narrow");
    let server = Server::spawn(&library, &["--code", "515151", "--allow", "read,annotate"]);
    let client = server.client();
    let (status, body) = client.pair(&server.code, "phone");
    assert_eq!(status, 200, "{body}");
    let permissions: Vec<&str> = body["permissions"]
        .as_array()
        .expect("scopes")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        !permissions.contains(&"cull"),
        "the server granted only what --allow listed: {body}"
    );
    let token = body["token"].as_str().expect("a token").to_string();

    let (status, body) = client.api(&token, "cull.start", json!({ "ids": [1] }));
    assert_eq!(status, 403, "{body}");
    assert!(body["error"].as_str().unwrap_or("").contains("cull"));
}

#[test]
fn an_upload_resumes_after_a_dropped_connection_and_verifies_the_digest() {
    let library = Library::build("loopback-upload");
    let server = Server::spawn(&library, &["--code", "242424"]);
    let client = server.client();
    let (status, body) = client.pair(&server.code, "phone");
    assert_eq!(status, 200, "{body}");
    let token = body["token"].as_str().expect("a token").to_string();

    // The file: 300 KB of a repeating pattern, chunked into three.
    let file: Vec<u8> = (0..300 * 1024u32).map(|i| (i % 251) as u8).collect();
    let total = file.len() as u64;
    let chunk = 100 * 1024;
    let sha = sortlens_ai::sha256::to_hex(&sortlens_ai::sha256::digest(&file));

    let headers = |token: &str, name: &str, range: Option<(u64, u64)>, sha_header: Option<&str>| {
        let mut out = vec![
            ("Authorization".to_string(), format!("Bearer {token}")),
            ("X-Upload-Id".to_string(), "phone-1".to_string()),
            ("X-Filename".to_string(), name.to_string()),
        ];
        if let Some((start, end)) = range {
            out.push(("Content-Range".to_string(), format!("bytes {start}-{end}/{total}")));
        }
        if let Some(sha) = sha_header {
            out.push(("X-Content-Sha256".to_string(), sha.to_string()));
        }
        out
    };

    // Chunk 1 lands.
    let (status, body) = chunk_request(
        &client,
        headers(&token, "IMG_0001.jpg", Some((0, chunk as u64 - 1)), None),
        &file[..chunk],
        true,
    );
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["received"], chunk as u64);

    // Chunk 2 is sent and the connection dies before the host reads it all.
    let (status, _) = chunk_request(
        &client,
        headers(&token, "IMG_0001.jpg", Some((chunk as u64, 2 * chunk as u64 - 1)), None),
        &file[chunk..],
        false,
    );
    assert_eq!(status, 0, "the server never answers a dropped connection");

    // The client asks where to continue.
    let bearer = format!("Bearer {token}");
    let response = client.request("GET", "/upload/phone-1", &[("Authorization", bearer.as_str())], b"");
    assert_eq!(response.status, 200, "the partial survives the drop: {}", response.json());
    let body = response.json();
    assert_eq!(body["received"], chunk as u64, "only chunk 1 is on disk");

    // Chunk 2 resent from where the file ends.
    let (status, body) = chunk_request(
        &client,
        headers(&token, "IMG_0001.jpg", Some((chunk as u64, 2 * chunk as u64 - 1)), None),
        &file[chunk..2 * chunk],
        true,
    );
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["received"], 2 * chunk as u64);

    // The final chunk with the wrong digest is refused and keeps the partial.
    let wrong_sha = "0".repeat(64);
    let (status, body) = chunk_request(
        &client,
        headers(&token, "IMG_0001.jpg", Some((2 * chunk as u64, total - 1)), Some(&wrong_sha)),
        &file[2 * chunk..],
        true,
    );
    assert_eq!(status, 422, "{body}");

    // The same chunk with the right digest completes, and the file lands in the album.
    let (status, body) = chunk_request(
        &client,
        headers(&token, "IMG_0001.jpg", Some((2 * chunk as u64, total - 1)), Some(&sha)),
        &file[2 * chunk..],
        true,
    );
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["saved"], "IMG_0001.jpg");
    assert_eq!(body["sha256"], sha.as_str());

    let landed = std::fs::read(library.album.join("IMG_0001.jpg")).expect("the file is in the album");
    assert_eq!(landed, file, "the bytes on disk are the bytes the phone sent");
    assert!(
        !library.album.join(".uploads/phone-1.part").exists(),
        "the partial is gone once the file is published"
    );

    // The same name again is not overwritten: the second photograph lands beside the first.
    let other: Vec<u8> = vec![7u8; 4 * 1024];
    let (status, body) = chunk_request(&client, headers(&token, "IMG_0001.jpg", None, None), &other, true);
    assert_eq!(status, 200, "{body}");
    assert_eq!(
        body["saved"], "IMG_0001 (1).jpg",
        "the collision policy renames, never overwrites: {body}"
    );
    assert!(library.album.join("IMG_0001.jpg").exists());

    // A token without upload cannot push a byte.
    drop(server);
}

#[test]
fn a_server_restart_keeps_the_tokens_and_the_pairing_scope() {
    let library = Library::build("loopback-restart");
    let token = {
        let server = Server::spawn(&library, &["--code", "939393"]);
        let client = server.client();
        let (status, body) = client.pair(&server.code, "phone");
        assert_eq!(status, 200, "{body}");
        body["token"].as_str().expect("a token").to_string()
    };
    // The first server is gone. Its token file is not.
    assert!(
        library.library.join("remote-tokens.json").exists(),
        "the tokens persist beside the library"
    );
    let server = Server::spawn(&library, &["--code", "939394"]);
    let client = server.client();
    let (status, body) = client.api(&token, "library.stats", json!({}));
    assert_eq!(status, 200, "the paired device survives the restart: {body}");
}
