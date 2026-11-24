//! The routes: health, pairing, the allow-listed command call, and the upload protocol.

use crate::http::{Request, Response};
use crate::upload;
use serde_json::{Value, json};
use sortlens_core::PhotoId;
use sortlens_engine::Engine;
use sortlens_remote::allowlist;
use sortlens_remote::pairing::{PairError, PairingGate};
use sortlens_remote::tokens::{Permission, Permissions, TokenStore};
use std::path::PathBuf;
use std::sync::Mutex;

/// The smallest edge a thumbnail request may ask for. Below it a browser is upscaling a
/// few pixels over a grid cell, which the client can do itself from a bigger answer.
const THUMB_MIN_EDGE: u32 = 16;
/// The largest edge: the detail tier the desktop's own detail view decodes at. A larger
/// request would mean decoding the primary image to serve a grid cell.
const THUMB_MAX_EDGE: u32 = sortlens_thumbs::DETAIL;

/// The whole server: one engine, one token store, one pairing gate.
pub struct Server {
    engine: Engine,
    tokens: TokenStore,
    gate: Mutex<PairingGate>,
    /// Where uploads land.
    pub album: PathBuf,
    /// The scopes a newly paired device gets.
    pub grants: Permissions,
    /// The body cap the HTTP reader is given.
    pub max_upload: u64,
    /// Where the current pairing code is written, if the owner asked for the file.
    pub pairing_file: Option<PathBuf>,
}

/// The parts of a server that come from the command line.
pub struct ServerConfig {
    /// Where uploads land.
    pub album: PathBuf,
    /// The scopes a newly paired device gets.
    pub grants: Permissions,
    /// The body cap the HTTP reader is given.
    pub max_upload: u64,
    /// Where the current pairing code is written, if the owner asked for the file.
    pub pairing_file: Option<PathBuf>,
}

impl Server {
    /// Assemble from the parts `main` parsed.
    pub fn new(engine: Engine, tokens: TokenStore, gate: PairingGate, config: ServerConfig) -> Self {
        Self {
            engine,
            tokens,
            gate: Mutex::new(gate),
            album: config.album,
            grants: config.grants,
            max_upload: config.max_upload,
            pairing_file: config.pairing_file,
        }
    }

    /// A fresh pairing code, printed and persisted.
    pub fn rotate_pairing(&mut self, code: String, now_unix: u64) {
        let gate = self.gate.get_mut().unwrap_or_else(|poisoned| poisoned.into_inner());
        gate.rotate(code, now_unix);
        self.write_pairing_file();
    }

    /// Record the current code in the pairing file, so the owner can find it without reading
    /// a terminal.
    fn write_pairing_file(&self) {
        let Some(path) = &self.pairing_file else { return };
        let gate = self.gate.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let text = format!("{}\n", gate.code());
        let tmp = path.with_extension("txt.tmp");
        let result = std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, path));
        if let Err(error) = result {
            eprintln!("could not write the pairing file: {error}");
        }
    }

    /// The pairing code, for the startup line and the tests.
    #[must_use]
    pub fn pairing_code(&self) -> String {
        self.gate.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).code().to_string()
    }

    /// Whether the current code is past its life.
    #[must_use]
    pub fn pairing_expired(&self, now_unix: u64) -> bool {
        self.gate.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).expired(now_unix)
    }

    /// How many devices are paired.
    #[must_use]
    pub fn paired_devices(&self) -> usize {
        self.tokens.len()
    }

    /// The engine, for the serve loop's background pump.
    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    /// Serve one request.
    pub fn handle(&mut self, request: &Request, now_unix: u64) -> Response {
        match (request.method.as_str(), request.path.as_str()) {
            ("GET", "/health") => self.health(),
            ("POST", "/pair") => self.pair(request, now_unix),
            ("POST", "/api") => self.api(request),
            ("POST", "/upload") => self.upload(request),
            ("GET", path) if path.starts_with("/upload/") => self.upload_status(request, &path["/upload/".len()..]),
            ("GET", path) if path.starts_with("/thumb/") => self.thumb(request, &path["/thumb/".len()..]),
            ("GET", "/") => Response::json(
                200,
                json!({ "ok": true, "name": "sortlens-server", "version": sortlens_remote::VERSION, "routes": ["/health", "/pair", "/api", "/upload", "/thumb"] }),
            ),
            ("GET", _) | ("POST", _) => Response::json(404, json!({ "ok": false, "error": "no such route" })),
            _ => Response::json(405, json!({ "ok": false, "error": "method not allowed" })),
        }
    }

    fn health(&mut self) -> Response {
        let scanning = self.engine.scan_progress().is_some();
        Response::json(
            200,
            json!({ "ok": true, "name": "sortlens-server", "version": sortlens_remote::VERSION, "scanning": scanning }),
        )
    }

    fn pair(&mut self, request: &Request, now_unix: u64) -> Response {
        let args: Value = match serde_json::from_slice(&request.body) {
            Ok(args) => args,
            Err(error) => return bad_request(format!("the body is not a JSON object: {error}")),
        };
        let code = args.get("code").and_then(Value::as_str).unwrap_or("");
        let device = args.get("device").and_then(Value::as_str).unwrap_or("device");
        let redemption = {
            let mut gate = self.gate.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            gate.redeem(code, now_unix)
        };
        if let Err(error) = redemption {
            let status = match error {
                PairError::Expired => 410,
                PairError::Wrong { .. } | PairError::Locked => 401,
            };
            let left = if let PairError::Wrong { left } = error {
                json!(left)
            } else {
                Value::Null
            };
            return Response::json(status, json!({ "ok": false, "error": error.message(), "attempts_left": left }));
        }

        // The token draws from the clock, the pair of peers and a counter; the code itself is
        // only a human carrier and is never part of the token.
        let entropy = format!(
            "{now_unix}-{}-{:?}-{}-{}",
            std::process::id(),
            request.bearer().map(str::len),
            self.tokens.len(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
        );
        let token = sortlens_remote::pairing::generate_token(entropy.as_bytes());
        match self.tokens.issue(token.clone(), device, self.grants, now_unix) {
            Ok(issued) => Response::json(
                200,
                json!({
                    "ok": true,
                    "token": token,
                    "device": issued.device,
                    "permissions": issued.permissions.to_list().iter().map(|p| p.id()).collect::<Vec<_>>(),
                }),
            ),
            Err(error) => Response::json(500, json!({ "ok": false, "error": error.to_string() })),
        }
    }

    fn api(&mut self, request: &Request) -> Response {
        let Some(token) = request.bearer() else {
            return Response::json(401, json!({ "ok": false, "error": "no bearer token; pair first" }));
        };
        let Some(device) = self.tokens.find(token) else {
            return Response::json(401, json!({ "ok": false, "error": "unknown token" }));
        };
        let device = device.clone();
        let args: Value = match serde_json::from_slice(&request.body) {
            Ok(args) => args,
            Err(error) => return bad_request(format!("the body is not a JSON object: {error}")),
        };
        let method = args.get("method").and_then(Value::as_str).unwrap_or("");
        let params = args.get("params").cloned().unwrap_or(json!({}));
        let Some(permission) = allowlist::permission_for(method) else {
            // A method the wire does not accept says so without leaking whether it exists.
            return Response::json(403, json!({ "ok": false, "error": "the remote API does not accept this command" }));
        };
        if !device.permissions.covers(permission) {
            return Response::json(
                403,
                json!({ "ok": false, "error": format!("this token lacks the {} permission", permission.id()) }),
            );
        }
        println!("remote {} {} ({})", device.device, method, permission.id());
        match self.engine.run(method, &params) {
            Ok(result) => Response::json(200, json!({ "ok": true, "result": result })),
            Err(error) => {
                let status = match &error {
                    sortlens_core::Error::StaleRevision { .. } => 409,
                    sortlens_core::Error::NoSuchPhoto(_) | sortlens_core::Error::NoSuchCollection(_) => 404,
                    _ => 200,
                };
                Response::json(status, json!({ "ok": false, "error": error.to_string() }))
            }
        }
    }

    fn upload(&mut self, request: &Request) -> Response {
        let device = match self.authorise(request, Permission::Upload) {
            Ok(device) => device,
            Err(response) => return response,
        };
        let range = upload::parse_range(request.header("content-range"));
        let id = request.header("x-upload-id").unwrap_or("");
        let name = request.header("x-filename").unwrap_or("");
        let sha = request.header("x-content-sha256");
        let chunk = upload::Chunk {
            id,
            raw_name: name,
            range,
            expected_sha: sha,
            body: &request.body,
            max_upload: self.max_upload,
        };
        let outcome = upload::ingest(&self.album.clone(), &chunk);
        println!("remote {} upload {} -> {}", device.device, id, outcome.status);
        Response::json(outcome.status, outcome.body)
    }

    fn upload_status(&mut self, request: &Request, id: &str) -> Response {
        if let Err(response) = self.authorise(request, Permission::Upload) {
            return response;
        }
        let outcome = upload::status(&self.album.clone(), id);
        Response::json(outcome.status, outcome.body)
    }

    /// One photograph's thumbnail as PNG bytes: `GET /thumb/<photo-id>/<edge>` under the
    /// paired token's `read` scope. A page over the wire cannot read the photo files, so
    /// the grid on a browser page needs this the way the desktop needs its cache. The edge
    /// is bounded: below [`THUMB_MIN_EDGE`] the answer would be upscaled nonsense, above
    /// [`THUMB_MAX_EDGE`] it would mean decoding a primary image to serve a grid cell.
    /// The bytes are the same lossless PNG the export path writes, and a photo whose file
    /// cannot be read answers 404 — a page must show a hole, not a lie.
    fn thumb(&mut self, request: &Request, rest: &str) -> Response {
        let device = match self.authorise(request, Permission::Read) {
            Ok(device) => device,
            Err(response) => return response,
        };
        let Some((id_text, edge_text)) = rest.split_once('/') else {
            return Response::json(400, json!({ "ok": false, "error": "the path is /thumb/<photo-id>/<edge>" }));
        };
        let Ok(id_value) = id_text.parse::<u64>() else {
            return Response::json(400, json!({ "ok": false, "error": "the photo id is not a number" }));
        };
        let Ok(edge) = edge_text.parse::<u32>() else {
            return Response::json(400, json!({ "ok": false, "error": "the edge is not a number" }));
        };
        if !(THUMB_MIN_EDGE..=THUMB_MAX_EDGE).contains(&edge) {
            return Response::json(
                400,
                json!({ "ok": false, "error": format!("the edge is {THUMB_MIN_EDGE} to {THUMB_MAX_EDGE} pixels") }),
            );
        }
        let id = PhotoId(id_value);
        let Some(photo) = self.engine.photo(id) else {
            return Response::json(404, json!({ "ok": false, "error": "no such photo" }));
        };
        let path = std::path::Path::new(&photo.root).join(&photo.rel);
        let decoded = match sortlens_thumbs::decode_file(&path, edge) {
            Ok(decoded) => decoded,
            Err(_) => {
                return Response::json(404, json!({ "ok": false, "error": "the photo's file could not be read" }));
            }
        };
        let Ok(image) =
            craft_image_codecs::Image::from_u8(decoded.width, decoded.height, craft_image_codecs::ChannelLayout::Rgba, decoded.rgba)
        else {
            return Response::json(500, json!({ "ok": false, "error": "the decoded thumbnail could not be wrapped" }));
        };
        let png = match craft_image_codecs::encode(
            &image,
            craft_image_codecs::Format::Png,
            &craft_image_codecs::EncodeOptions::default(),
        ) {
            Ok(png) => png,
            Err(_) => {
                return Response::json(500, json!({ "ok": false, "error": "the thumbnail could not be encoded" }));
            }
        };
        println!("remote {} thumb {} {}px", device.device, id, edge);
        Response::bytes(200, "image/png", png)
    }

    /// The device behind a bearer token with `permission` granted, or the reply to send.
    fn authorise(&mut self, request: &Request, permission: Permission) -> Result<sortlens_remote::tokens::DeviceToken, Response> {
        let Some(token) = request.bearer() else {
            return Err(Response::json(401, json!({ "ok": false, "error": "no bearer token; pair first" })));
        };
        let Some(device) = self.tokens.find(token) else {
            return Err(Response::json(401, json!({ "ok": false, "error": "unknown token" })));
        };
        if !device.permissions.covers(permission) {
            return Err(Response::json(
                403,
                json!({ "ok": false, "error": format!("this token lacks the {} permission", permission.id()) }),
            ));
        }
        Ok(device.clone())
    }
}

fn bad_request(message: String) -> Response {
    Response::json(400, json!({ "ok": false, "error": message }))
}
