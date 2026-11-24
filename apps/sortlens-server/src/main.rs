//! `sortlens-server`: the library, served to a paired device.
//!
//! Opt-in by construction: the process only runs when someone starts it, it binds loopback
//! unless `--bind lan` is passed explicitly (SL-J09), and the first thing it prints is the
//! pairing code a device exchanges for a scoped bearer token (SL-J01/J03). The API is the
//! command registry behind an allow-list (`sortlens-remote`), not a second API, and uploads
//! are resumable and verified (SL-J04). No cloud, no account server, no telemetry.

#![forbid(unsafe_code)]

mod api;
mod http;
mod upload;

use api::Server;
use serde_json::json;
use sortlens_engine::Engine;
use sortlens_remote::pairing::{CODE_TTL_SECS, PairingGate};
use sortlens_remote::tokens::{Permissions, TokenStore};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const USAGE: &str = "\
sortlens-server --library DIR [options]

Options:
  --library DIR        the library to serve (required)
  --bind loopback|lan  loopback is the default; `lan` exposes the port on every interface
  --port N             the port to listen on (default: an ephemeral one, printed on start)
  --album DIR          where uploads land (default: <library>/inbox)
  --tokens FILE        the token store (default: <library>/remote-tokens.json)
  --code NNNNNN        pin the pairing code (default: generated and printed)
  --pair-ttl-secs N    how long a pairing code lives (default: 600)
  --allow LIST         scopes for newly paired devices, comma-separated
                       (default: read,upload,annotate,cull)
  --max-upload-mb N    the upload body cap in MiB (default: 512)
  --help               this text

Routes:
  GET  /health          the connection test (no token)
  POST /pair            {code, device} -> {token, permissions}
  POST /api             {method, params} with a bearer token -> the registry's answer
  POST /upload          a resumable chunk into the album folder
  GET  /upload/<id>     how much of an upload the host already holds
";

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{USAGE}");
        return std::process::ExitCode::SUCCESS;
    }
    match run(args) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("sortlens-server: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut library: Option<PathBuf> = None;
    let mut bind_lan = false;
    let mut port = 0u16;
    let mut album: Option<PathBuf> = None;
    let mut tokens: Option<PathBuf> = None;
    let mut code: Option<String> = None;
    let mut ttl = CODE_TTL_SECS;
    let mut grants: Option<Permissions> = None;
    let mut max_upload = 512 * 1024 * 1024;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |what: &str| -> Result<String, String> { iter.next().cloned().ok_or_else(|| format!("{what} needs a value")) };
        match arg.as_str() {
            "--library" => library = Some(PathBuf::from(value("--library")?)),
            "--bind" => match value("--bind")?.as_str() {
                "loopback" => bind_lan = false,
                "lan" => bind_lan = true,
                other => return Err(format!("--bind is loopback or lan, not {other}")),
            },
            "--port" => port = value("--port")?.parse::<u16>().map_err(|_| "--port is a number".to_string())?,
            "--album" => album = Some(PathBuf::from(value("--album")?)),
            "--tokens" => tokens = Some(PathBuf::from(value("--tokens")?)),
            "--code" => {
                let pinned = value("--code")?;
                sortlens_remote::pairing::validate_pinned(&pinned).map_err(|error| error.to_string())?;
                code = Some(pinned);
            }
            "--pair-ttl-secs" => {
                ttl = value("--pair-ttl-secs")?
                    .parse::<u64>()
                    .map_err(|_| "--pair-ttl-secs is a number".to_string())?;
            }
            "--allow" => {
                let list = value("--allow")?;
                let mut parsed = Vec::new();
                for scope in list.split(',') {
                    let scope = scope.trim();
                    if scope.is_empty() {
                        continue;
                    }
                    parsed.push(
                        sortlens_remote::tokens::Permission::parse(scope)
                            .ok_or_else(|| format!("--allow knows read, upload, annotate, cull — not {scope}"))?,
                    );
                }
                grants = Some(Permissions::from_list(&parsed));
            }
            "--max-upload-mb" => {
                max_upload = value("--max-upload-mb")?
                    .parse::<u64>()
                    .map_err(|_| "--max-upload-mb is a number".to_string())?
                    .saturating_mul(1024 * 1024);
            }
            other => return Err(format!("unknown argument {other}; --help lists them")),
        }
    }
    let library = library.ok_or("--library DIR is required")?;
    let tokens = tokens.unwrap_or_else(|| library.join("remote-tokens.json"));
    let album = album.unwrap_or_else(|| library.join("inbox"));

    sortlens_remote::allowlist::audit().map_err(|error| error.to_string())?;
    let engine = Engine::open(&library).map_err(|error| format!("could not open {}: {error}", library.display()))?;
    let token_store = TokenStore::load(&tokens).map_err(|error| error.to_string())?;
    let now = unix_now();
    let gate = match &code {
        Some(pinned) => PairingGate::new(pinned.clone(), now, ttl),
        None => PairingGate::new(sortlens_remote::pairing::generate_code(entropy().as_bytes()), now, ttl),
    };
    let config = api::ServerConfig {
        album: album.clone(),
        grants: grants.unwrap_or_else(Permissions::all),
        max_upload,
        pairing_file: None,
    };
    let mut server = Server::new(engine, token_store, gate, config);

    let host = if bind_lan { "0.0.0.0" } else { "127.0.0.1" };
    let listener = TcpListener::bind((host, port)).map_err(|error| format!("could not bind {host}:{port}: {error}"))?;
    let port = listener.local_addr().map_err(|error| error.to_string())?.port();
    println!("sortlens-server {}", sortlens_remote::VERSION);
    println!("listening http://{host}:{port}");
    println!("album {}", album.display());
    println!("tokens {}", tokens.display());
    println!("pairing {}", server.pairing_code());
    println!("paired devices: {}", if server_is_empty(&server) { "none yet" } else { "some" });
    use std::io::Write;
    let _ = std::io::stdout().flush();

    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!("a connection failed: {error}");
                continue;
            }
        };
        let request = http::read_request(&mut stream, server.max_upload);
        let response = match request {
            Ok(request) => server.handle(&request, unix_now()),
            Err(http::ReadError::Bad(reason)) => http::Response::json(400, json!({ "ok": false, "error": reason })),
            Err(http::ReadError::TooLarge) => {
                http::Response::json(413, json!({ "ok": false, "error": "the head or body is past the limit" }))
            }
            Err(http::ReadError::Closed) => continue,
            Err(http::ReadError::Io(error)) => {
                eprintln!("a socket failed: {error}");
                continue;
            }
        };
        if response.write_to(&mut stream).is_err() {
            continue;
        }
        // The engine owns the scan worker; between connections is where its finished work is
        // collected, so a phone's rescan actually lands while the process serves.
        pump(&mut server);
        // The pairing code keeps its ten minutes even while nothing is connected.
        if server.pairing_expired(unix_now()) {
            let fresh = sortlens_remote::pairing::generate_code(entropy().as_bytes());
            server.rotate_pairing(fresh, unix_now());
            println!("pairing {}", server.pairing_code());
            let _ = std::io::stdout().flush();
        }
    }
    Ok(())
}

fn server_is_empty(server: &Server) -> bool {
    server.paired_devices() == 0
}

/// Drive the engine's finished background work.
fn pump(server: &mut Server) {
    let mut quiet = 0u32;
    while quiet < 20 {
        if server.engine_mut().poll().is_some() {
            quiet = 0;
            continue;
        }
        if server.engine_mut().scan_progress().is_some() {
            std::thread::sleep(std::time::Duration::from_millis(5));
            continue;
        }
        quiet += 1;
    }
}

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn entropy() -> String {
    format!(
        "{}-{:?}-{:?}",
        unix_now(),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    )
}
