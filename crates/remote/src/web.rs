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
}
