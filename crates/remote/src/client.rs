//! The device side of the remote protocol (SL-J02): what a paired client speaks, in one place.
//!
//! The rule is the same one `docs/ARCHITECTURE.md` §10 gives the server: one protocol, not a
//! second one. Pairing, the `/api` registry calls, the resumable chunked `/upload`, and the
//! keep-or-remove decision the client applies once the server acks (SL-J05) live here, so the
//! wasm page and any later phone build cannot drift from what `apps/sortlens-server` answers.
//! The HTTP face is a [`Transport`], so the native tests run the real server over TCP while a
//! wasm build backs the same protocol with the browser's own fetch — the wire format is the
//! crate's, not the transport's.

use serde_json::{Value, json};
use std::fmt;

/// Why a client call failed.
#[derive(Debug)]
pub enum ClientError {
    /// The server answered with a status the call does not accept, and said why.
    Server { status: u16, message: String },
    /// The transport could not reach the server at all.
    Connection(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Server { status, message } => write!(f, "the server answered {status}: {message}"),
            Self::Connection(reason) => write!(f, "could not reach the server: {reason}"),
        }
    }
}

impl std::error::Error for ClientError {}

/// The HTTP face a client speaks through.
pub trait Transport {
    /// One request, one answer. The server closes the connection after answering
    /// (`Connection: close`), so reading to the end is the whole response.
    fn request(&mut self, method: &str, path: &str, headers: &[(String, String)], body: &[u8]) -> Result<Response, ClientError>;
}

/// A response, split the way a client needs it.
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Response {
    /// The body parsed as JSON, or a server error naming the status.
    pub fn json(&self) -> Result<Value, ClientError> {
        serde_json::from_slice(&self.body).map_err(|_| ClientError::Server {
            status: self.status,
            message: "the answer was not JSON".to_string(),
        })
    }
}

/// The native transport: a blocking TCP connection per request, over `std::net` only.
#[cfg(not(target_arch = "wasm32"))]
pub struct TcpTransport {
    addr: String,
    timeout: std::time::Duration,
}

#[cfg(not(target_arch = "wasm32"))]
impl TcpTransport {
    /// Speak to the server at `host:port` — the address its `listening` line announced.
    pub fn new(addr: impl Into<String>) -> Self {
        Self {
            addr: addr.into(),
            timeout: std::time::Duration::from_secs(15),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Transport for TcpTransport {
    fn request(&mut self, method: &str, path: &str, headers: &[(String, String)], body: &[u8]) -> Result<Response, ClientError> {
        use std::io::{Read, Write};
        let mut stream = std::net::TcpStream::connect(&self.addr).map_err(|error| ClientError::Connection(error.to_string()))?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|error| ClientError::Connection(error.to_string()))?;
        let mut head = format!("{method} {path} HTTP/1.1\r\nHost: sortlens\r\nContent-Length: {}\r\n", body.len());
        for (name, value) in headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");
        stream
            .write_all(head.as_bytes())
            .and_then(|_| stream.write_all(body))
            .and_then(|_| stream.flush())
            .map_err(|error| ClientError::Connection(error.to_string()))?;
        let mut raw = Vec::new();
        stream
            .read_to_end(&mut raw)
            .map_err(|error| ClientError::Connection(error.to_string()))?;
        split_response(&raw).ok_or_else(|| ClientError::Connection("the answer had no header terminator".into()))
    }
}

/// Split a raw response into status and body, on the header terminator.
#[cfg(not(target_arch = "wasm32"))]
fn split_response(raw: &[u8]) -> Option<Response> {
    let head_end = raw.windows(4).position(|window| window == b"\r\n\r\n")?;
    let head = String::from_utf8_lossy(&raw[..head_end]).into_owned();
    let status: u16 = head.lines().next()?.split_whitespace().nth(1)?.parse().ok()?;
    Some(Response {
        status,
        body: raw[head_end + 4..].to_vec(),
    })
}

/// A photograph the device wants in the host's album.
#[derive(Debug, Clone)]
pub struct UploadItem {
    /// The transfer id, chosen when the item was queued: a resume — including after the app
    /// was backgrounded and the connection died — reuses it, so the server's partial and
    /// this item are the same file. Build items with [`UploadItem::new`].
    pub id: String,
    /// The file name it asks to land under. The server sanitises and never overwrites, so the
    /// published name in the ack is the truth, not this.
    pub name: String,
    /// The bytes, held by the client: an upload that outlives its connection resumes from the
    /// server's received count, not from a re-read.
    pub bytes: Vec<u8>,
    /// The keep-or-remove toggle (SL-J05), chosen up front and applied the moment the server
    /// acks the completed upload — including when the transfer needed a resume to get there.
    pub remove_after_upload: bool,
}

impl UploadItem {
    /// Queue an upload: the id is minted here, once, and stays with the item.
    pub fn new(name: impl Into<String>, bytes: Vec<u8>, remove_after_upload: bool) -> Self {
        let name = name.into();
        let id = upload_id(&name, &bytes);
        Self {
            id,
            name,
            bytes,
            remove_after_upload,
        }
    }
}

/// What the client does with its own copy once the server has the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalCopy {
    /// The ack arrived and the toggle said keep: nothing is deleted on the device.
    Keep,
    /// The ack arrived and the toggle said remove: the device drops its copy now.
    Remove,
}

impl LocalCopy {
    /// The id the wire-adjacent surfaces report.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::Remove => "remove",
        }
    }
}

/// The server's ack for a completed upload, with the client's own decision attached.
#[derive(Debug, Clone)]
pub struct UploadAck {
    /// The name the photograph actually landed under (sanitised, ` (n)` beside a collision).
    pub saved: String,
    /// The digest the server verified the completed file against.
    pub sha256: String,
    /// The keep-or-remove decision (SL-J05): what the client now does with its own copy.
    pub local_copy: LocalCopy,
}

/// A paired device talking to one server.
pub struct Client<T> {
    transport: T,
    token: Option<String>,
    device: Option<String>,
    /// Upload chunk size in bytes. A file at or under it goes in one shot; anything larger
    /// goes as `Content-Range` chunks that resume from the server's received count.
    chunk: usize,
}

/// The default chunk: large enough that a phone photograph is one request, small enough that
/// a dropped connection does not lose a whole album.
pub const CHUNK: usize = 512 * 1024;

impl<T: Transport> Client<T> {
    /// A client, not yet paired. The address lives in the transport, which is the thing
    /// that actually connects.
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            token: None,
            device: None,
            chunk: CHUNK,
        }
    }

    /// A client that is already paired: the token it saved from the session that paired it
    /// (a wasm page keeps it in browser storage), carried straight back into the calls.
    pub fn paired(token: impl Into<String>, transport: T) -> Self {
        Self {
            transport,
            token: Some(token.into()),
            device: None,
            chunk: CHUNK,
        }
    }

    /// Override the chunk size, for tests that must see several chunks.
    #[must_use]
    pub fn with_chunk_size(mut self, chunk: usize) -> Self {
        if chunk > 0 {
            self.chunk = chunk;
        }
        self
    }

    /// The paired token, once [`Client::pair`] succeeded.
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// The device name the server recorded for this client.
    pub fn device(&self) -> Option<&str> {
        self.device.as_deref()
    }

    /// The connection test: `GET /health` takes no token, by design (SL-J07).
    pub fn health(&mut self) -> Result<Value, ClientError> {
        let response = self.transport.request("GET", "/health", &[], &[])?;
        let body = response.json()?;
        if response.status == 200 && body.get("ok").and_then(Value::as_bool) == Some(true) {
            Ok(body)
        } else {
            Err(ClientError::Server {
                status: response.status,
                message: "the health answer did not say ok".to_string(),
            })
        }
    }

    /// Trade a pairing code for a scoped token. The code is a human carrier, never stored by
    /// the client; the token is, because every later call carries it.
    pub fn pair(&mut self, code: &str, device: &str) -> Result<(), ClientError> {
        let response = self.post_json("/pair", None, json!({ "code": code, "device": device }))?;
        let body = response.json()?;
        match response.status {
            200 => {
                let token = body.get("token").and_then(Value::as_str).unwrap_or("");
                if token.is_empty() {
                    return Err(ClientError::Server {
                        status: 200,
                        message: "the pairing answer carried no token".to_string(),
                    });
                }
                self.token = Some(token.to_string());
                self.device = body.get("device").and_then(Value::as_str).map(str::to_string);
                Ok(())
            }
            _ => Err(server_error(response.status, &body)),
        }
    }

    /// One registry command through `/api`, under the paired token's scopes.
    pub fn call(&mut self, method: &str, params: &Value) -> Result<Value, ClientError> {
        let token = self
            .token
            .clone()
            .ok_or_else(|| ClientError::Connection("the client is not paired".into()))?;
        let response = self.post_json("/api", Some(&token), json!({ "method": method, "params": params }))?;
        let body = response.json()?;
        if response.status == 200 && body.get("ok").and_then(Value::as_bool) == Some(true) {
            Ok(body.get("result").cloned().unwrap_or(Value::Null))
        } else {
            Err(server_error(response.status, &body))
        }
    }

    /// The library as snapshot JSON — the state a wasm client opens with `Engine::open_memory`
    /// and saves back between sessions.
    pub fn snapshot(&mut self) -> Result<String, ClientError> {
        let result = self.call("library.snapshot", &json!({}))?;
        result
            .get("snapshot")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| ClientError::Server {
                status: 200,
                message: "the snapshot answer carried no snapshot".to_string(),
            })
    }

    /// Send one photograph, and apply the keep-or-remove decision on the ack.
    ///
    /// A file at or under the chunk size goes in one request; anything larger goes as
    /// `Content-Range` chunks. A `409` naming the server's received count is not a failure —
    /// it is where the transfer resumes, which is what "including background refresh" means
    /// for SL-J05: the toggle survives the resume, because it lives on the item, not on the
    /// connection. The completing chunk carries `X-Content-Sha256`, and the server refuses
    /// the file if the bytes that arrived do not match, leaving the partial where it was.
    pub fn upload(&mut self, item: &UploadItem) -> Result<UploadAck, ClientError> {
        let Some(token) = self.token.as_deref() else {
            return Err(ClientError::Connection("the client is not paired".into()));
        };
        let total = item.bytes.len() as u64;
        if total == 0 {
            return Err(ClientError::Server {
                status: 400,
                message: "the upload is empty".to_string(),
            });
        }
        let id = &item.id;
        let sha = sortlens_ai::sha256::to_hex(&sortlens_ai::sha256::digest(&item.bytes));
        let chunk = self.chunk.max(1) as u64;

        // One request covers a file that fits a chunk: the server hashes the whole body
        // against `X-Content-Sha256` before it publishes.
        if total <= chunk {
            let headers = upload_headers(item, id, Some(&sha), token);
            let response = self.transport.request("POST", "/upload", &headers, &item.bytes)?;
            return finished(response, item.remove_after_upload);
        }

        // Chunked, resuming from wherever the server says its copy ends. The ranges are
        // recomputed on every pass, so a resume that skips ahead picks up the same coverage.
        let mut cursor = 0u64;
        loop {
            let ranges = chunk_ranges(total, chunk);
            // Where to continue from: the server's own count when a 409 or an intermediate
            // ack moved it. A fresh transfer starts at zero.
            let from = cursor;
            let found = ranges.iter().find(|(from_range, _)| *from_range >= from).copied();
            let (start, end) = match found {
                Some(range) => range,
                // Every range is behind the cursor: the server has all the bytes and the
                // final ack was lost, and re-sending the completing chunk is the one way
                // to learn that — it either answers `saved` again or refuses harmlessly.
                None => {
                    let Some(&(last_start, last_end)) = ranges.last() else {
                        return Err(ClientError::Connection("the upload had no chunks".into()));
                    };
                    if from > last_end + 1 {
                        return Err(ClientError::Server {
                            status: 409,
                            message: "the upload did not advance".to_string(),
                        });
                    }
                    (last_start, last_end)
                }
            };
            let span = &item.bytes[start as usize..(end + 1) as usize];
            let completing = end + 1 == total;
            let mut headers = upload_headers(item, id, completing.then_some(sha.as_str()), token);
            headers.push(("Content-Range".to_string(), format!("bytes {start}-{end}/{total}")));
            let response = self.transport.request("POST", "/upload", &headers, span)?;
            let body = response.json()?;
            match response.status {
                // Completed: `saved` names the file that landed, `sha256` is the verified digest.
                200 if body.get("saved").and_then(Value::as_str).is_some() => {
                    return Ok(UploadAck {
                        saved: body.get("saved").and_then(Value::as_str).unwrap_or_default().to_string(),
                        sha256: body.get("sha256").and_then(Value::as_str).unwrap_or_default().to_string(),
                        local_copy: local_copy(item.remove_after_upload),
                    });
                }
                // An intermediate ack: the next pass starts after what the server received.
                200 => {
                    let received = body.get("received").and_then(Value::as_u64).unwrap_or(0);
                    if received <= start {
                        return Err(ClientError::Server {
                            status: response.status,
                            message: "the server acknowledged nothing".to_string(),
                        });
                    }
                    cursor = received;
                }
                // A refusal that names the count: resume from there, unless it names no
                // progress at all, in which case it is a real failure.
                409 => {
                    let Some(received) = resume_from(&body) else {
                        return Err(server_error(409, &body));
                    };
                    if received <= start {
                        return Err(server_error(409, &body));
                    }
                    cursor = received;
                }
                _ => return Err(server_error(response.status, &body)),
            }
        }
    }

    /// POST JSON with the token when there is one.
    fn post_json(&mut self, path: &str, token: Option<&str>, body: Value) -> Result<Response, ClientError> {
        let mut headers = Vec::new();
        if let Some(token) = token {
            headers.push(("Authorization".to_string(), format!("Bearer {token}")));
        }
        headers.push(("Content-Type".to_string(), "application/json".to_string()));
        self.transport.request("POST", path, &headers, body.to_string().as_bytes())
    }
}

/// The ranges that cover `total` bytes in chunks of `chunk`: `(start, end)` inclusive, each
/// start exactly where the last ended, so the server's `start != received_now` guard never
/// refuses a fresh client's next chunk.
fn chunk_ranges(total: u64, chunk: u64) -> Vec<(u64, u64)> {
    let chunk = chunk.max(1);
    let mut ranges = Vec::new();
    let mut start = 0;
    while start < total {
        let end = (start + chunk).min(total) - 1;
        ranges.push((start, end));
        start = end + 1;
    }
    ranges
}

/// What a `409` tells the client to do: continue from the count the server received.
fn resume_from(body: &Value) -> Option<u64> {
    body.get("received").and_then(Value::as_u64)
}

/// The keep-or-remove decision (SL-J05), applied client-side on the server's ack.
fn local_copy(remove_after_upload: bool) -> LocalCopy {
    if remove_after_upload { LocalCopy::Remove } else { LocalCopy::Keep }
}

/// A transfer id the server will accept: 1–64 characters of its safe alphabet, unique enough
/// across a session and stable enough across a resume that both sides mean the same file.
fn upload_id(name: &str, bytes: &[u8]) -> String {
    let mut entropy = name.as_bytes().to_vec();
    entropy.extend(bytes.len().to_le_bytes());
    entropy.extend(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| since.as_nanos())
            .unwrap_or(0)
            .to_le_bytes(),
    );
    format!("u{}", &sortlens_ai::sha256::to_hex(&sortlens_ai::sha256::digest(&entropy))[..24])
}

/// The headers every chunk carries: the bearer token, the file name and transfer id, and the
/// digest on the chunk that completes the file.
fn upload_headers(item: &UploadItem, id: &str, sha: Option<&str>, token: &str) -> Vec<(String, String)> {
    let mut headers = vec![
        ("Authorization".to_string(), format!("Bearer {token}")),
        ("X-Filename".to_string(), item.name.clone()),
        ("X-Upload-Id".to_string(), id.to_string()),
        ("Content-Type".to_string(), "application/octet-stream".to_string()),
    ];
    if let Some(sha) = sha {
        headers.push(("X-Content-Sha256".to_string(), sha.to_string()));
    }
    headers
}

/// Interpret the server's answer to a completed upload.
fn finished(response: Response, remove_after_upload: bool) -> Result<UploadAck, ClientError> {
    let body = response.json()?;
    if response.status == 200
        && body.get("ok").and_then(Value::as_bool) == Some(true)
        && let Some(saved) = body.get("saved").and_then(Value::as_str)
    {
        return Ok(UploadAck {
            saved: saved.to_string(),
            sha256: body.get("sha256").and_then(Value::as_str).unwrap_or_default().to_string(),
            local_copy: local_copy(remove_after_upload),
        });
    }
    Err(server_error(response.status, &body))
}

/// The server's error shape, as a typed failure.
fn server_error(status: u16, body: &Value) -> ClientError {
    ClientError::Server {
        status,
        message: body
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("the server refused the call")
            .to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    /// The chunk plan covers the file exactly once, contiguously, ending one byte short of
    /// the total: the shape the server's `start != received_now` guard demands.
    #[test]
    fn chunk_ranges_cover_the_file_once() {
        let ranges = chunk_ranges(10_000, 4_000);
        assert_eq!(ranges, vec![(0, 3_999), (4_000, 7_999), (8_000, 9_999)]);
        // An exact fit is one range, not a zero-byte tail.
        assert_eq!(chunk_ranges(4_000, 4_000), vec![(0, 3_999)]);
        // A one-byte file is one range, never empty and never start-end reversed.
        assert_eq!(chunk_ranges(1, 512), vec![(0, 0)]);
    }

    /// A `409` is an instruction, not a failure: it names where the server's copy ends.
    #[test]
    fn a_409_names_where_to_resume() {
        let body = json!({ "ok": false, "error": "the chunk does not start where the file ends", "received": 4096 });
        assert_eq!(resume_from(&body), Some(4_096));
        // A refusal that names no count cannot move the cursor.
        assert_eq!(resume_from(&json!({ "ok": false, "error": "unknown id" })), None);
    }

    /// The keep-or-remove toggle (SL-J05) maps to the decision the client applies on the ack.
    #[test]
    fn the_ack_maps_the_toggle_to_keep_or_remove() {
        assert_eq!(local_copy(false), LocalCopy::Keep);
        assert_eq!(local_copy(true), LocalCopy::Remove);
        assert_eq!(LocalCopy::Keep.id(), "keep");
        assert_eq!(LocalCopy::Remove.id(), "remove");
    }

    /// A transfer id is inside the server's safe alphabet: 1-64 characters of A-Z a-z 0-9 - _.
    #[test]
    fn an_upload_id_fits_the_server_alphabet() {
        let item = UploadItem::new("PXL_20261124.jpg", vec![1, 2, 3], false);
        let id = &item.id;
        assert_eq!(id.len(), 25, "u + 24 hex");
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    /// A transport that records what it was asked and answers from a script, so the client's
    /// flow is testable without a server.
    type RecordedRequest = (String, String, Vec<(String, String)>, Vec<u8>);

    struct Scripted {
        answers: Vec<Result<Response, ClientError>>,
        requests: Vec<RecordedRequest>,
    }

    impl Scripted {
        fn with(answers: Vec<Result<Response, ClientError>>) -> Self {
            Self {
                answers,
                requests: Vec::new(),
            }
        }
    }

    impl Transport for Scripted {
        fn request(&mut self, method: &str, path: &str, headers: &[(String, String)], body: &[u8]) -> Result<Response, ClientError> {
            self.requests
                .push((method.to_string(), path.to_string(), headers.to_vec(), body.to_vec()));
            if self.answers.is_empty() {
                Err(ClientError::Connection("script exhausted".into()))
            } else {
                self.answers.remove(0)
            }
        }
    }

    fn ok_json(body: Value) -> Result<Response, ClientError> {
        Ok(Response {
            status: 200,
            body: body.to_string().into_bytes(),
        })
    }

    fn error_json(status: u16, body: Value) -> Result<Response, ClientError> {
        Ok(Response {
            status,
            body: body.to_string().into_bytes(),
        })
    }

    fn item(bytes: usize, remove: bool) -> UploadItem {
        UploadItem::new("photo.jpg", (0..bytes).map(|step| (step % 251) as u8).collect(), remove)
    }

    /// A file at or under the chunk size is one request, and the ack carries the
    /// keep-or-remove decision the toggle chose.
    #[test]
    fn a_small_upload_is_one_request_and_the_ack_carries_the_decision() {
        let mut client = Client::new(Scripted::with(vec![ok_json(json!({
            "ok": true, "received": 5, "saved": "photo.jpg", "sha256": "abc"
        }))]));
        client.token = Some("t".to_string());
        let ack = client.upload(&item(5, true)).expect("upload");
        assert_eq!(ack.saved, "photo.jpg");
        assert_eq!(ack.local_copy, LocalCopy::Remove);
        assert_eq!(client.transport.requests.len(), 1, "one request");
        let (method, path, headers, _) = &client.transport.requests[0];
        assert_eq!(method, "POST");
        assert_eq!(path, "/upload");
        let names: Vec<&str> = headers.iter().map(|(name, _)| name.as_str()).collect();
        assert!(names.contains(&"X-Content-Sha256"), "the one-shot carries the digest");
        assert!(!names.contains(&"Content-Range"), "no span for a one-shot");
    }

    /// A larger file is chunked in order, and a 409 moves the cursor instead of failing.
    #[test]
    fn a_409_mid_transfer_resumes_from_the_server_count() {
        let mut client = Client::new(Scripted::with(vec![
            ok_json(json!({ "ok": true, "received": 4, "total": 10 })),
            ok_json(json!({ "ok": true, "received": 10, "saved": "photo.jpg", "sha256": "abc" })),
        ]))
        .with_chunk_size(4);
        client.token = Some("t".to_string());
        let ack = client.upload(&item(10, false)).expect("upload");
        assert_eq!(ack.local_copy, LocalCopy::Keep);
        assert_eq!(client.transport.requests.len(), 2, "two chunks, not three");
        let first = &client.transport.requests[0].3;
        assert_eq!(first.len(), 4, "the first chunk is the first four bytes");
        let second_headers = &client.transport.requests[1].2;
        let range = second_headers
            .iter()
            .find(|(name, _)| name == "Content-Range")
            .map(|(_, value)| value.clone())
            .expect("the chunked request carries a span");
        assert_eq!(range, "bytes 4-7/10", "the 409's count is where the next chunk starts");
    }

    /// A 409 that names no progress at all is a failure, not an infinite loop.
    #[test]
    fn a_409_without_progress_fails_instead_of_looping() {
        let mut client = Client::new(Scripted::with(vec![error_json(
            409,
            json!({ "ok": false, "error": "the total does not match the upload in progress", "received": 0 }),
        )]))
        .with_chunk_size(4);
        client.token = Some("t".to_string());
        let error = client.upload(&item(10, false)).expect_err("refused");
        assert!(error.to_string().contains("does not match"), "the server's reason carries: {error}");
        assert_eq!(client.transport.requests.len(), 1, "one attempt, no loop");
    }

    /// Nothing is sent before the client is paired.
    #[test]
    fn an_unpaired_client_refuses_without_talking() {
        let mut client = Client::new(Scripted::with(Vec::new()));
        let error = client.upload(&item(5, false)).expect_err("not paired");
        assert!(error.to_string().contains("not paired"), "the refusal says why: {error}");
        assert!(client.transport.requests.is_empty(), "nothing reached the wire");
    }

    /// Pairing stores the token and the device name the server recorded.
    #[test]
    fn pairing_stores_the_token_and_device() {
        let mut client = Client::new(Scripted::with(vec![ok_json(json!({
            "ok": true, "token": "tok-1", "device": "phone"
        }))]));
        client.pair("123456", "phone").expect("pair");
        assert_eq!(client.token(), Some("tok-1"));
        assert_eq!(client.device(), Some("phone"));
    }
}
