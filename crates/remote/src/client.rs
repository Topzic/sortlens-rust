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

/// One fully built request: the transport's only job is to send it and bring the answer back.
///
/// The protocol is the crate's, not the transport's: the blocking [`Client`] and the wasm
/// page's [`crate::web::WebClient`] both build their calls with these functions, so a browser
/// cannot drift from what the native tests and the two-process harness prove.
#[derive(Debug, Clone)]
pub struct Request {
    /// The HTTP method.
    pub method: String,
    /// The path, any query already encoded.
    pub path: String,
    /// The request headers, in the order the call built them.
    pub headers: Vec<(String, String)>,
    /// The request body; empty on the GETs.
    pub body: Vec<u8>,
}

/// `GET /health`: the connection test takes no token, by design (SL-J07).
#[must_use]
pub fn health_request() -> Request {
    Request {
        method: "GET".to_string(),
        path: "/health".to_string(),
        headers: Vec::new(),
        body: Vec::new(),
    }
}

/// Interpret the answer to [`health_request`].
pub fn health_answer(response: Response) -> Result<Value, ClientError> {
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

/// What pairing left the client holding: the token every later call carries, and the device
/// name the server recorded.
#[derive(Debug, Clone)]
pub struct Pairing {
    /// The scoped bearer token. The code is a human carrier and is never kept.
    pub token: String,
    /// The device name the server recorded for this client.
    pub device: Option<String>,
}

/// `POST /pair`: trade a pairing code for a scoped token.
#[must_use]
pub fn pair_request(code: &str, device: &str) -> Request {
    Request {
        method: "POST".to_string(),
        path: "/pair".to_string(),
        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
        body: json!({ "code": code, "device": device }).to_string().into_bytes(),
    }
}

/// Interpret the answer to [`pair_request`].
pub fn pair_answer(response: Response) -> Result<Pairing, ClientError> {
    let body = response.json()?;
    if response.status != 200 {
        return Err(server_error(response.status, &body));
    }
    let token = body.get("token").and_then(Value::as_str).unwrap_or("");
    if token.is_empty() {
        return Err(ClientError::Server {
            status: 200,
            message: "the pairing answer carried no token".to_string(),
        });
    }
    Ok(Pairing {
        token: token.to_string(),
        device: body.get("device").and_then(Value::as_str).map(str::to_string),
    })
}

/// `POST /api`: one registry command through the allow-list, under the paired token's scopes.
#[must_use]
pub fn call_request(token: &str, method: &str, params: &Value) -> Request {
    Request {
        method: "POST".to_string(),
        path: "/api".to_string(),
        headers: vec![
            ("Authorization".to_string(), format!("Bearer {token}")),
            ("Content-Type".to_string(), "application/json".to_string()),
        ],
        body: json!({ "method": method, "params": params }).to_string().into_bytes(),
    }
}

/// Interpret the answer to [`call_request`]: the registry's result on success, the server's
/// own reason on a refusal.
pub fn call_answer(response: Response) -> Result<Value, ClientError> {
    let body = response.json()?;
    if response.status == 200 && body.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(body.get("result").cloned().unwrap_or(Value::Null))
    } else {
        Err(server_error(response.status, &body))
    }
}

/// The snapshot string inside a `library.snapshot` result — the state a wasm client opens
/// with [`sortlens_engine::Engine::open_memory`] and saves back between sessions.
pub fn snapshot_from_result(result: &Value) -> Result<String, ClientError> {
    result
        .get("snapshot")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| ClientError::Server {
            status: 200,
            message: "the snapshot answer carried no snapshot".to_string(),
        })
}

/// `GET /thumb/<id>/<edge>` under the paired token's `read` scope.
#[must_use]
pub fn thumbnail_request(token: &str, id: sortlens_core::PhotoId, edge: u32) -> Request {
    Request {
        method: "GET".to_string(),
        path: format!("/thumb/{}/{}", id, edge),
        headers: vec![("Authorization".to_string(), format!("Bearer {token}"))],
        body: Vec::new(),
    }
}

/// Interpret the answer to [`thumbnail_request`]: PNG bytes returned untouched — the bytes
/// are what a browser `<img>` or an egui texture decodes — and every refusal, which the
/// server answers in JSON whatever the success body was, as a typed error.
pub fn thumbnail_answer(response: Response) -> Result<Vec<u8>, ClientError> {
    if response.status == 200 && response.body.starts_with(b"\x89PNG") {
        return Ok(response.body);
    }
    Err(server_error(response.status, &response.json()?))
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
        health_answer(self.send(health_request())?)
    }

    /// Trade a pairing code for a scoped token. The code is a human carrier, never stored by
    /// the client; the token is, because every later call carries it.
    pub fn pair(&mut self, code: &str, device: &str) -> Result<(), ClientError> {
        let pairing = pair_answer(self.send(pair_request(code, device))?)?;
        self.token = Some(pairing.token);
        self.device = pairing.device;
        Ok(())
    }

    /// One registry command through `/api`, under the paired token's scopes.
    pub fn call(&mut self, method: &str, params: &Value) -> Result<Value, ClientError> {
        let token = self
            .token
            .clone()
            .ok_or_else(|| ClientError::Connection("the client is not paired".into()))?;
        let response = self.send(call_request(&token, method, params))?;
        call_answer(response)
    }

    /// The library as snapshot JSON — the state a wasm client opens with `Engine::open_memory`
    /// and saves back between sessions.
    pub fn snapshot(&mut self) -> Result<String, ClientError> {
        snapshot_from_result(&self.call("library.snapshot", &json!({}))?)
    }

    /// One photograph's thumbnail as PNG bytes: `GET /thumb/<id>/<edge>` under the paired
    /// token's `read` scope. The server bounds the edge; a refusal here names its range.
    /// The bytes are what a browser `<img>` or an egui `Image` decodes — a wasm page hands
    /// them straight to the element, and nothing on the wire is a second format.
    pub fn thumbnail(&mut self, id: sortlens_core::PhotoId, edge: u32) -> Result<Vec<u8>, ClientError> {
        let token = self
            .token
            .clone()
            .ok_or_else(|| ClientError::Connection("the client is not paired".into()))?;
        thumbnail_answer(self.send(thumbnail_request(&token, id, edge))?)
    }

    /// Send one photograph, and apply the keep-or-remove decision on the ack.
    ///
    /// A file at or under the chunk size goes in one request; anything larger goes as
    /// `Content-Range` chunks. A `409` naming the server's received count is not a failure —
    /// it is where the transfer resumes, which is what "including background refresh" means
    /// for SL-J05: the toggle survives the resume, because it lives on the item, not on the
    /// connection. The completing chunk carries `X-Content-Sha256`, and the server refuses
    /// the file if the bytes that arrived do not match, leaving the partial where it was.
    ///
    /// Every answer is decoded through [`upload_step`], the one decision both this blocking
    /// loop and the wasm page's async loop walk — the two cannot drift.
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
            return finish_upload(response, item.remove_after_upload);
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
            match upload_step(response, start) {
                UploadStep::Saved { saved, sha256 } => {
                    return Ok(UploadAck {
                        saved,
                        sha256,
                        local_copy: local_copy(item.remove_after_upload),
                    });
                }
                // The next pass starts after what the server received.
                UploadStep::Continue(received) => cursor = received,
                UploadStep::Fail(error) => return Err(error),
            }
        }
    }

    /// Send one fully built request through the transport.
    fn send(&mut self, request: Request) -> Result<Response, ClientError> {
        self.transport
            .request(&request.method, &request.path, &request.headers, &request.body)
    }
}

/// What one server answer to an upload chunk means, decoded once here for every client:
/// the blocking [`Client`]'s loop and the wasm page's async loop
/// ([`crate::web::WebClient::upload`]) both walk their transfers through this, so a browser
/// page and a phone build cannot drift from what the two-process harness proves.
pub(crate) enum UploadStep {
    /// The file landed: the published name, and the digest the server verified.
    Saved { saved: String, sha256: String },
    /// The transfer continues; the cursor moves to this count.
    Continue(u64),
    /// The server refused, and said why (or said nothing useful).
    Fail(ClientError),
}

/// Decode one answer to an upload chunk, from the position the chunk claimed (`start`).
/// A `200` that names `saved` ends the transfer; a `200` or `409` that names a count past
/// where the client already is moves the cursor; everything else is the server's own
/// refusal, carried as a typed error.
pub(crate) fn upload_step(response: Response, start: u64) -> UploadStep {
    let body = match response.json() {
        Ok(body) => body,
        Err(error) => return UploadStep::Fail(error),
    };
    match response.status {
        200 => {
            if let Some(saved) = body.get("saved").and_then(Value::as_str) {
                UploadStep::Saved {
                    saved: saved.to_string(),
                    sha256: body.get("sha256").and_then(Value::as_str).unwrap_or_default().to_string(),
                }
            } else {
                let received = body.get("received").and_then(Value::as_u64).unwrap_or(0);
                if received > start {
                    UploadStep::Continue(received)
                } else {
                    UploadStep::Fail(ClientError::Server {
                        status: 200,
                        message: body
                            .get("error")
                            .and_then(Value::as_str)
                            .unwrap_or("the server acknowledged nothing")
                            .to_string(),
                    })
                }
            }
        }
        // A refusal that names a count past the cursor is an instruction to resume; one
        // that names no progress at all is a real failure.
        409 => match resume_from(&body) {
            Some(received) if received > start => UploadStep::Continue(received),
            _ => UploadStep::Fail(server_error(409, &body)),
        },
        _ => UploadStep::Fail(server_error(response.status, &body)),
    }
}

/// Interpret the server's answer to a completed (one-shot) upload, with the client's
/// keep-or-remove decision attached — the blocking client and the wasm page both use it.
pub(crate) fn finish_upload(response: Response, remove_after_upload: bool) -> Result<UploadAck, ClientError> {
    match upload_step(response, 0) {
        UploadStep::Saved { saved, sha256 } => Ok(UploadAck {
            saved,
            sha256,
            local_copy: local_copy(remove_after_upload),
        }),
        UploadStep::Fail(error) => Err(error),
        UploadStep::Continue(_) => Err(ClientError::Server {
            status: 200,
            message: "the one-shot upload answered as a partial".to_string(),
        }),
    }
}

/// The ranges that cover `total` bytes in chunks of `chunk`: `(start, end)` inclusive, each
/// start exactly where the last ended, so the server's `start != received_now` guard never
/// refuses a fresh client's next chunk.
pub(crate) fn chunk_ranges(total: u64, chunk: u64) -> Vec<(u64, u64)> {
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
pub(crate) fn resume_from(body: &Value) -> Option<u64> {
    body.get("received").and_then(Value::as_u64)
}

/// The keep-or-remove decision (SL-J05), applied client-side on the server's ack.
pub(crate) fn local_copy(remove_after_upload: bool) -> LocalCopy {
    if remove_after_upload { LocalCopy::Remove } else { LocalCopy::Keep }
}

/// A transfer id the server will accept: 1–64 characters of its safe alphabet, unique enough
/// across a session and stable enough across a resume that both sides mean the same file.
fn upload_id(name: &str, bytes: &[u8]) -> String {
    let mut entropy = name.as_bytes().to_vec();
    entropy.extend(bytes.len().to_le_bytes());
    entropy.extend(clock_entropy().to_le_bytes());
    format!("u{}", &sortlens_ai::sha256::to_hex(&sortlens_ai::sha256::digest(&entropy))[..24])
}

/// The clock part of the upload id's entropy. A browser page cannot call `SystemTime::now`
/// — wasm32 has no clock, and it *panics* rather than answering — so the wasm build asks
/// the browser's own clock, the same way the page already speaks to the browser.
#[cfg(not(target_arch = "wasm32"))]
fn clock_entropy() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
fn clock_entropy() -> u64 {
    js_sys::Date::now() as u64
}

/// The headers every chunk carries: the bearer token, the file name and transfer id, and the
/// digest on the chunk that completes the file.
pub(crate) fn upload_headers(item: &UploadItem, id: &str, sha: Option<&str>, token: &str) -> Vec<(String, String)> {
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

    /// A snapshot call returns the exact string the answer carried — the state
    /// `Engine::open_memory` reopens, untouched.
    #[test]
    fn a_snapshot_call_returns_the_wire_state() {
        let mut client = Client::new(Scripted::with(vec![ok_json(json!({
            "ok": true, "result": { "snapshot": "{\"version\":3}" }
        }))]));
        client.token = Some("t".to_string());
        let snapshot = client.snapshot().expect("snapshot");
        assert_eq!(snapshot, "{\"version\":3}");
    }

    /// The wire builders are the protocol: what a wasm page builds is what the blocking
    /// client builds, and the answers parse the same way. Pairing first.
    #[test]
    fn the_pair_wire_builds_the_request_and_parses_the_answer() {
        let request = pair_request("369369", "tablet");
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/pair");
        let body: Value = serde_json::from_slice(&request.body).expect("JSON body");
        assert_eq!(body["code"], "369369");
        assert_eq!(body["device"], "tablet");
        let pairing =
            pair_answer(ok_json(json!({ "ok": true, "token": "tok-1", "device": "tablet" })).expect("response")).expect("pairing");
        assert_eq!(pairing.token, "tok-1");
        assert_eq!(pairing.device.as_deref(), Some("tablet"));
        // A refusal is the server's own reason; a 200 without a token is not a pairing.
        let error = pair_answer(error_json(401, json!({ "ok": false, "error": "wrong code" })).expect("response")).expect_err("refused");
        assert!(error.to_string().contains("wrong code"), "{error}");
        let error = pair_answer(ok_json(json!({ "ok": true })).expect("response")).expect_err("no token");
        assert!(error.to_string().contains("no token"), "{error}");
    }

    /// The `/api` wire: the token rides on the request, the result is what the answer
    /// carried, and a refusal is the server's own reason.
    #[test]
    fn the_call_wire_carries_the_token_and_parses_the_result() {
        let request = call_request("tok-1", "library.snapshot", &json!({}));
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/api");
        assert!(
            request
                .headers
                .iter()
                .any(|(name, value)| name == "Authorization" && value == "Bearer tok-1")
        );
        let body: Value = serde_json::from_slice(&request.body).expect("JSON body");
        assert_eq!(body["method"], "library.snapshot");
        let result = call_answer(ok_json(json!({ "ok": true, "result": { "n": 2 } })).expect("response")).expect("result");
        assert_eq!(result["n"], 2);
        let error = call_answer(error_json(403, json!({ "ok": false, "error": "lacks read" })).expect("response")).expect_err("refused");
        assert!(error.to_string().contains("lacks read"), "{error}");
    }

    /// The snapshot is the string inside the `library.snapshot` result, and a result without
    /// one is refused, not swallowed.
    #[test]
    fn the_snapshot_is_the_string_inside_the_result() {
        let snapshot = snapshot_from_result(&json!({ "snapshot": "{\"v\":1}" })).expect("snapshot");
        assert_eq!(snapshot, "{\"v\":1}");
        let error = snapshot_from_result(&json!({ "n": 2 })).expect_err("no snapshot");
        assert!(error.to_string().contains("no snapshot"), "{error}");
    }

    /// The thumbnail wire: PNG bytes pass through untouched, and a 200 that is not a PNG —
    /// the shape of a JSON refusal over a photo that is gone — is a typed error, not pixels.
    #[test]
    fn the_thumbnail_wire_answers_png_bytes_or_a_typed_refusal() {
        let png: Vec<u8> = b"\x89PNG\r\n\x1a\npixels".to_vec();
        let got = thumbnail_answer(Response {
            status: 200,
            body: png.clone(),
        })
        .expect("thumbnail");
        assert_eq!(got, png, "the bytes are not re-wrapped");
        let request = thumbnail_request("tok-1", sortlens_core::PhotoId(7), 256);
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/thumb/7/256");
        assert!(request.body.is_empty());
        assert!(
            request
                .headers
                .iter()
                .any(|(name, value)| name == "Authorization" && value == "Bearer tok-1")
        );
        let error = thumbnail_answer(Response {
            status: 200,
            body: json!({ "ok": false, "error": "no such photo" }).to_string().into_bytes(),
        })
        .expect_err("not pixels");
        assert!(error.to_string().contains("no such photo"), "{error}");
    }

    /// The shared step decoder is the one decision both clients walk: a completing answer
    /// is `Saved` with the published name and digest, an intermediate answer moves the
    /// cursor, and every refusal is the server's own reason.
    #[test]
    fn upload_step_decides_saved_continue_and_fail() {
        let saved = upload_step(
            ok_json(json!({ "ok": true, "received": 5, "saved": "photo.jpg", "sha256": "abc" })).expect("response"),
            0,
        );
        match saved {
            UploadStep::Saved { saved, sha256 } => {
                assert_eq!(saved, "photo.jpg");
                assert_eq!(sha256, "abc");
            }
            _ => panic!("a completing answer is Saved"),
        }
        let step = upload_step(ok_json(json!({ "ok": true, "received": 4_096 })).expect("response"), 0);
        assert_eq!(
            match step {
                UploadStep::Continue(n) => n,
                _ => panic!("intermediate is Continue"),
            },
            4_096
        );
        // A count at or behind the cursor is not progress; the server's reason carries.
        let error = upload_step(ok_json(json!({ "ok": true, "received": 0 })).expect("response"), 0);
        assert!(matches!(error, UploadStep::Fail(_)));
        let step = upload_step(
            error_json(
                409,
                json!({ "ok": false, "error": "the chunk does not start where the file ends", "received": 8_192 }),
            )
            .expect("response"),
            4_096,
        );
        assert_eq!(
            match step {
                UploadStep::Continue(n) => n,
                _ => panic!("a 409 with progress resumes"),
            },
            8_192
        );
        let error = upload_step(error_json(409, json!({ "ok": false, "error": "unknown id" })).expect("response"), 0);
        match error {
            UploadStep::Fail(ClientError::Server { status, message }) => {
                assert_eq!(status, 409);
                assert_eq!(message, "unknown id");
            }
            _ => panic!("a refusal without progress fails with the server's reason"),
        }
        let error = upload_step(
            error_json(413, json!({ "ok": false, "error": "the upload is past the limit" })).expect("response"),
            0,
        );
        assert!(matches!(error, UploadStep::Fail(_)));
    }

    /// A one-shot upload whose answer is not a completion is the server's own refusal.
    #[test]
    fn a_one_shot_answer_is_a_completion_or_the_server_s_reason() {
        let ack = finish_upload(
            ok_json(json!({ "ok": true, "received": 3, "saved": "kept.jpg", "sha256": "abc" })).expect("response"),
            false,
        )
        .expect("completed");
        assert_eq!(ack.saved, "kept.jpg");
        assert_eq!(ack.local_copy, LocalCopy::Keep);
        let error = finish_upload(ok_json(json!({ "ok": true, "received": 3 })).expect("response"), false).expect_err("no saved");
        assert!(error.to_string().contains("partial"), "{error}");
        let error = finish_upload(
            error_json(413, json!({ "ok": false, "error": "the upload is past the limit" })).expect("response"),
            false,
        )
        .expect_err("refused");
        assert!(error.to_string().contains("past the limit"), "{error}");
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

    /// A thumbnail request is `GET /thumb/<id>/<edge>` carrying the paired token, and the
    /// answer's PNG bytes come back untouched.
    #[test]
    fn a_thumbnail_request_carries_the_token_and_returns_the_png() {
        let png: Vec<u8> = b"\x89PNG\r\n\x1a\nrest".to_vec();
        let mut client = Client::new(Scripted::with(vec![Ok(Response {
            status: 200,
            body: png.clone(),
        })]));
        client.token = Some("tok-1".to_string());
        let got = client.thumbnail(sortlens_core::PhotoId(7), 128).expect("thumbnail");
        assert_eq!(got, png, "the bytes are not re-wrapped");
        let (method, path, headers, body) = &client.transport.requests[0];
        assert_eq!(method, "GET");
        assert_eq!(path, "/thumb/7/128");
        assert!(
            headers
                .iter()
                .any(|(name, value)| name == "Authorization" && value == "Bearer tok-1"),
            "the paired token rides on the request"
        );
        assert!(body.is_empty(), "a GET carries no body");
    }

    /// Nothing is fetched before the client is paired, and a JSON refusal is a typed error.
    #[test]
    fn an_unpaired_thumbnail_refuses_and_a_json_refusal_names_the_server() {
        let mut client = Client::new(Scripted::with(Vec::new()));
        let error = client.thumbnail(sortlens_core::PhotoId(1), 128).expect_err("not paired");
        assert!(error.to_string().contains("not paired"), "the refusal says why: {error}");
        assert!(client.transport.requests.is_empty(), "nothing reached the wire");

        let mut client = Client::new(Scripted::with(vec![error_json(
            404,
            json!({ "ok": false, "error": "no such photo" }),
        )]));
        client.token = Some("t".to_string());
        let error = client.thumbnail(sortlens_core::PhotoId(999), 128).expect_err("refused");
        assert!(
            error.to_string().contains("404") && error.to_string().contains("no such photo"),
            "{error}"
        );
    }
}
