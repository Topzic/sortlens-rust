//! The wasm client: the browser's own fetch behind the same protocol (SL-J02).
//!
//! The blocking [`Client`](crate::client::Client) needs a transport that waits, and a browser
//! never waits: its `fetch` is a promise. The wire, though, is the crate's — the request the
//! page builds and the answer it parses are the same functions the native tests and the
//! two-process harness run, so a page cannot drift from what the server answers.

#![forbid(unsafe_code)]

use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use crate::client::{self, ClientError, Response};
use serde_json::{Value, json};

/// The transport a wasm page speaks through: one browser `fetch` per request, same-origin
/// when the page is served by the server it pairs with (`/app`), or an absolute base URL
/// when it is not.
pub struct FetchTransport {
    base: String,
}

impl FetchTransport {
    /// A transport aimed at the host. A trailing `/` is dropped, so a base pasted with one
    /// still builds `/health` and not `//health`.
    #[must_use]
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into().trim_end_matches('/').to_string(),
        }
    }

    /// One request, one answer, through the browser's fetch.
    pub async fn request(&self, request: &client::Request) -> Result<Response, ClientError> {
        let window = web_sys::window().ok_or_else(|| ClientError::Connection("there is no window to fetch from".into()))?;
        let headers = web_sys::Headers::new().map_err(|_| ClientError::Connection("the request headers could not be built".into()))?;
        for (name, value) in &request.headers {
            headers
                .set(name, value)
                .map_err(|_| ClientError::Connection(format!("the {name} header could not be set")))?;
        }
        let init = web_sys::RequestInit::new();
        init.set_method(&request.method);
        init.set_headers(&headers);
        // A GET with a body — even an empty array — is refused by the browser's fetch.
        if !request.body.is_empty() {
            init.set_body(&Uint8Array::from(request.body.as_slice()).into());
        }
        let js_request = web_sys::Request::new_with_str_and_init(&format!("{}{}", self.base, request.path), &init)
            .map_err(|error| ClientError::Connection(format!("the request could not be built: {error:?}")))?;
        let js_response = JsFuture::from(window.fetch_with_request(&js_request))
            .await
            .map_err(|error| ClientError::Connection(format!("the fetch failed: {error:?}")))?;
        let js_response: web_sys::Response = js_response
            .dyn_into()
            .map_err(|_| ClientError::Connection("the answer was not a fetch response".into()))?;
        let status = js_response.status();
        let buffer = JsFuture::from(
            js_response
                .array_buffer()
                .map_err(|_| ClientError::Connection("the answer carried no body".into()))?,
        )
        .await
        .map_err(|error| ClientError::Connection(format!("the body could not be read: {error:?}")))?;
        Ok(Response {
            status,
            body: Uint8Array::new(&buffer).to_vec(),
        })
    }
}

/// A paired device speaking over the browser's fetch. The method set is the page's day:
/// pairing, registry calls, the snapshot [`sortlens_engine::Engine::open_memory`] reopens,
/// and the PNG thumbnails its grid paints. The upload surface joins it with the file picker.
pub struct WebClient {
    transport: FetchTransport,
    token: Option<String>,
    device: Option<String>,
}

impl WebClient {
    /// A client, not yet paired, aimed at the host.
    #[must_use]
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            transport: FetchTransport::new(base),
            token: None,
            device: None,
        }
    }

    /// A client that is already paired: the token the page saved from its last session,
    /// carried straight back into the calls.
    #[must_use]
    pub fn paired(token: impl Into<String>, base: impl Into<String>) -> Self {
        Self {
            transport: FetchTransport::new(base),
            token: Some(token.into()),
            device: None,
        }
    }

    /// The paired token, once [`WebClient::pair`] succeeded or the session restored it.
    #[must_use]
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// The device name the server recorded for this page.
    #[must_use]
    pub fn device(&self) -> Option<&str> {
        self.device.as_deref()
    }

    /// Pair with the host's current code. The code is a human carrier, never stored.
    pub async fn pair(&mut self, code: &str, device: &str) -> Result<(), ClientError> {
        let pairing = client::pair_answer(self.transport.request(&client::pair_request(code, device)).await?)?;
        self.token = Some(pairing.token);
        self.device = pairing.device;
        Ok(())
    }

    /// One registry command through `/api`, under the paired token's scopes.
    pub async fn call(&mut self, method: &str, params: &Value) -> Result<Value, ClientError> {
        let token = self
            .token
            .clone()
            .ok_or_else(|| ClientError::Connection("the client is not paired".into()))?;
        let response = self.transport.request(&client::call_request(&token, method, params)).await?;
        client::call_answer(response)
    }

    /// The library as snapshot JSON — the state the page opens with
    /// [`sortlens_engine::Engine::open_memory`] and saves back between sessions.
    pub async fn snapshot(&mut self) -> Result<String, ClientError> {
        client::snapshot_from_result(&self.call("library.snapshot", &json!({})).await?)
    }

    /// One photograph's thumbnail as PNG bytes: `GET /thumb/<id>/<edge>` under the paired
    /// token's `read` scope, the bytes straight to the texture, never re-wrapped.
    pub async fn thumbnail(&mut self, id: sortlens_core::PhotoId, edge: u32) -> Result<Vec<u8>, ClientError> {
        let token = self
            .token
            .clone()
            .ok_or_else(|| ClientError::Connection("the client is not paired".into()))?;
        let response = self.transport.request(&client::thumbnail_request(&token, id, edge)).await?;
        client::thumbnail_answer(response)
    }

    /// Send one photograph through the resumable chunk loop — the same wire the blocking
    /// [`Client::upload`](client::Client::upload) walks, decoded through the same shared
    /// [`client::upload_step`], so a page and a phone cannot drift. One request at or under
    /// [`client::CHUNK`]; above it, `Content-Range` chunks that resume from the server's
    /// received count; the completing chunk carries `X-Content-Sha256`; the ack carries the
    /// keep-or-remove decision (SL-J05) that rides on the item.
    pub async fn upload(&mut self, item: &client::UploadItem) -> Result<client::UploadAck, ClientError> {
        let Some(token) = self.token.clone() else {
            return Err(ClientError::Connection("the client is not paired".into()));
        };
        let total = item.bytes.len() as u64;
        if total == 0 {
            return Err(ClientError::Server {
                status: 400,
                message: "the upload is empty".to_string(),
            });
        }
        let id = item.id.clone();
        let sha = sortlens_ai::sha256::to_hex(&sortlens_ai::sha256::digest(&item.bytes));
        let chunk = client::CHUNK as u64;

        // One request covers a file that fits a chunk: the server hashes the whole body
        // against `X-Content-Sha256` before it publishes.
        if total <= chunk {
            let headers = client::upload_headers(item, &id, Some(&sha), &token);
            let response = self
                .transport
                .request(&client::Request {
                    method: "POST".to_string(),
                    path: "/upload".to_string(),
                    headers,
                    body: item.bytes.clone(),
                })
                .await?;
            return client::finish_upload(response, item.remove_after_upload);
        }

        // Chunked, resuming from wherever the server says its copy ends — the same
        // cursor logic the blocking client walks, over an await instead of a block.
        let mut cursor = 0u64;
        loop {
            let ranges = client::chunk_ranges(total, chunk);
            let found = ranges.iter().find(|(from_range, _)| *from_range >= cursor).copied();
            let (start, end) = match found {
                Some(range) => range,
                // Every range is behind the cursor: the server has all the bytes and the
                // final ack was lost, and re-sending the completing chunk is the one way
                // to learn that — it either answers `saved` again or refuses harmlessly.
                None => {
                    let Some(&(last_start, last_end)) = ranges.last() else {
                        return Err(ClientError::Connection("the upload had no chunks".into()));
                    };
                    if cursor > last_end + 1 {
                        return Err(ClientError::Server {
                            status: 409,
                            message: "the upload did not advance".to_string(),
                        });
                    }
                    (last_start, last_end)
                }
            };
            let span = item.bytes[start as usize..(end + 1) as usize].to_vec();
            let completing = end + 1 == total;
            let mut headers = client::upload_headers(item, &id, completing.then_some(sha.as_str()), &token);
            headers.push(("Content-Range".to_string(), format!("bytes {start}-{end}/{total}")));
            let response = self
                .transport
                .request(&client::Request {
                    method: "POST".to_string(),
                    path: "/upload".to_string(),
                    headers,
                    body: span,
                })
                .await?;
            match client::upload_step(response, start) {
                client::UploadStep::Saved { saved, sha256 } => {
                    return Ok(client::UploadAck {
                        saved,
                        sha256,
                        local_copy: client::local_copy(item.remove_after_upload),
                    });
                }
                client::UploadStep::Continue(received) => cursor = received,
                client::UploadStep::Fail(error) => return Err(error),
            }
        }
    }
}
