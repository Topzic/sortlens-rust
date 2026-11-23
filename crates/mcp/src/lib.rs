//! The model-context protocol, over the same command registry the UI and the CLI use.
//!
//! One rule decides everything here: **a tool is a command id**. There is no second list of agent
//! actions to keep in sync, no hand-written schema per tool, and no way for an agent to reach
//! something the app cannot — the registry is the list, `Engine::run` is the call, and a command
//! still marked `Planned` refuses with the milestone that will implement it.
//!
//! Transport is newline-delimited JSON over stdio, one message per line, which is what MCP clients
//! speak when they launch a server as a subprocess. `handle` takes one line and returns one line, so
//! the whole protocol is testable without a pipe or a process.
//!
//! Envelope handling is `craft-jsonrpc`, not hand-rolled: it validates the `jsonrpc` version and the id,
//! separates requests from notifications, handles batches, and enforces a byte bound on every message. Session
//! handling — version negotiation, the initialize/initialized handshake, the tool envelopes — is `craft-mcp`.
//! This crate owns the transport and the method table, which is the division both state for themselves.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use std::io::{BufRead, Write};

use craft_jsonrpc::{Limits, ParseError, Request, RpcError};
use craft_mcp::{Policy, Session};
use serde_json::{Value, json};
use sortlens_engine::Engine;

pub mod examples;

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The protocol version this server speaks by default.
pub const PROTOCOL_VERSION: &str = craft_mcp::PROTOCOL_VERSION;

/// Versions offered during negotiation, newest first. A client asking for an older one is echoed back its own
/// version rather than being told it is wrong: the tool surface is the same either way.
pub const SUPPORTED_VERSIONS: &[&str] = &[PROTOCOL_VERSION, "2024-11-05"];

/// JSON-RPC error code: the input was not JSON.
pub const PARSE_ERROR: i64 = -32700;
/// JSON-RPC error code: the method does not exist.
pub const METHOD_NOT_FOUND: i64 = -32601;
/// JSON-RPC error code: the request was not valid JSON-RPC.
pub const INVALID_REQUEST: i64 = -32600;
/// JSON-RPC error code: the parameters were wrong.
pub const INVALID_PARAMS: i64 = -32602;

/// One MCP server bound to an open library.
pub struct Mcp {
    engine: Engine,
    session: Session,
}

impl Mcp {
    /// Wrap an engine.
    pub fn new(engine: Engine) -> Self {
        let session = Session::new("sortlens", VERSION, Policy::Compatible)
            .with_legacy_initialize(SUPPORTED_VERSIONS, json!({ "name": "sortlens", "version": VERSION }));
        let mut session = session;
        session.set_instructions(
            "Every tool is a Sortlens command id. Call tools/list for the ready ones; a tool still listed as \
             planned refuses and names its milestone."
                .to_string(),
        );
        Self { engine, session }
    }

    /// Whether the client has completed the handshake, which a host may want to log.
    pub fn is_initialized(&self) -> bool {
        self.session.is_initialized()
    }

    /// The engine, for tests and for a host that wants to keep using it.
    pub fn engine(&mut self) -> &mut Engine {
        &mut self.engine
    }

    /// Handle one line of stdio and return the line to write back, if any.
    ///
    /// A notification (no `id`) is answered with `None`: JSON-RPC forbids a response to one, and a
    /// client that gets an extra message where it expected none will fail to parse the stream.
    pub fn handle(&mut self, line: &str) -> Option<String> {
        self.handle_with(line, limits())
    }

    /// The same, with explicit bounds — a test uses this to prove the message bound bites without writing
    /// eight megabytes.
    pub fn handle_with(&mut self, line: &str, limits: Limits) -> Option<String> {
        let value = match craft_jsonrpc::parse_json(line.as_bytes(), limits) {
            Ok(value) => value,
            // An oversized message is refused *before* it is parsed, which is the point of the bound: the
            // limit protects the allocation, so it cannot be a check on the parsed result.
            Err(ParseError::MessageTooLarge) => {
                return Some(
                    craft_jsonrpc::error_response(
                        Value::Null,
                        RpcError::new(PARSE_ERROR, format!("message over {} bytes", limits.max_message_bytes)),
                    )
                    .to_string(),
                );
            }
            Err(ParseError::BatchTooLarge) => {
                return Some(
                    craft_jsonrpc::error_response(
                        Value::Null,
                        RpcError::new(INVALID_REQUEST, format!("batch over {} requests", limits.max_batch_items)),
                    )
                    .to_string(),
                );
            }
            Err(ParseError::InvalidJson) => return Some(craft_jsonrpc::parse_error_response().to_string()),
        };
        let server = std::cell::RefCell::new(&mut *self);
        let response = craft_jsonrpc::process(
            &value,
            limits,
            |request| {
                let mut server = server.borrow_mut();
                server.dispatch(request)
            },
            |notification| {
                // Notifications never reach `dispatch`: JSON-RPC answers none of them, so `craft-jsonrpc` hands
                // them to this callback instead.
                server.borrow_mut().notify(notification);
            },
        );
        match response {
            Ok(value) => value.map(|value| value.to_string()),
            Err(error) => Some(craft_jsonrpc::error_response(Value::Null, RpcError::new(INVALID_REQUEST, error.to_string())).to_string()),
        }
    }

    /// One validated request, dispatched to a method.
    ///
    /// A notification (`notifications/initialized`) is handled by being ignored: JSON-RPC forbids answering
    /// one, and `craft-jsonrpc` never asks for a response when the id is absent.
    /// A notification from the client. The only one this protocol defines is `initialized`, which the session
    /// records; anything else is ignored rather than answered.
    fn notify(&mut self, request: Request<'_>) {
        self.session.notification(request);
    }

    fn dispatch(&mut self, request: Request<'_>) -> Result<Value, RpcError> {
        // The handshake belongs to the session: version negotiation, `ping`, and the initialized
        // notification are the same in every MCP server, and getting them subtly wrong is how a client
        // decides not to talk to you.
        if let Some(answer) = self.session.request(request, false) {
            return answer;
        }
        let params = request.params.cloned().unwrap_or(Value::Null);
        match request.method {
            // `initialized` is a notification, so nothing is answered; the session has recorded it.
            "notifications/initialized" | "initialized" => {
                self.session.notification(request);
                Ok(Value::Null)
            }
            "tools/list" => Ok(craft_mcp::tool_list(self.tools())),
            "tools/call" => {
                let (name, arguments) = craft_mcp::tool_call(&params).map_err(|error| RpcError::new(error.code, error.message))?;
                Ok(self.invoke(name, &arguments))
            }
            other => Err(RpcError::new(METHOD_NOT_FOUND, format!("no such method: {other}"))),
        }
    }

    /// Serve stdio until the input ends.
    ///
    /// Returns the number of requests answered. A read or write error ends the loop: a client that
    /// has closed the pipe has nothing more to say, and there is nowhere left to report to.
    pub fn serve<R: BufRead, W: Write>(&mut self, input: R, output: W) -> std::io::Result<usize> {
        self.serve_with(input, output, limits())
    }

    /// The same, with explicit bounds. A message over the bound is answered with an error and the rest of that
    /// line is discarded, so one bad frame does not end the session — and does not allocate the line either.
    pub fn serve_with<R: BufRead, W: Write>(&mut self, mut input: R, mut output: W, limits: Limits) -> std::io::Result<usize> {
        let mut answered = 0;
        loop {
            match craft_jsonrpc::read_line(&mut input, limits.max_message_bytes) {
                Ok(None) => break,
                Ok(Some(bytes)) => {
                    if bytes.iter().all(u8::is_ascii_whitespace) {
                        continue;
                    }
                    let line = String::from_utf8_lossy(&bytes).into_owned();
                    if let Some(response) = self.handle_with(&line, limits) {
                        writeln!(output, "{response}")?;
                        output.flush()?;
                        answered += 1;
                    }
                }
                Err(craft_jsonrpc::LineError::TooLong) => {
                    // Discard the rest of the line without holding it, then say what happened and keep going.
                    discard_line(&mut input)?;
                    let response = craft_jsonrpc::error_response(
                        Value::Null,
                        RpcError::new(PARSE_ERROR, format!("message over {} bytes", limits.max_message_bytes)),
                    );
                    writeln!(output, "{response}")?;
                    output.flush()?;
                    answered += 1;
                }
                Err(craft_jsonrpc::LineError::Io(error)) => return Err(error),
            }
        }
        Ok(answered)
    }

    /// The tool list, straight from the registry.
    fn tools(&self) -> Value {
        let mut tools = Vec::new();
        for command in self.engine.registry().all() {
            if !command.is_ready() {
                continue;
            }
            // The description carries the worked example, because an agent choosing a tool needs
            // the argument shapes as much as the label. The examples are executed in a test, so a
            // name here is a name the engine really accepts.
            let args = examples::arguments(command.id);
            let description = match &args {
                Some(details) => match details["note"].as_str() {
                    Some(note) => format!("{} ({}). Arguments: {note}", command.label, command.menu),
                    None => format!("{} ({})", command.label, command.menu),
                },
                None => format!("{} ({})", command.label, command.menu),
            };
            tools.push(json!({
                "name": command.id,
                "description": description,
                "arguments": args,
                // Deliberately permissive: the engine validates every argument and says what was
                // wrong, which is more truthful than a schema that tries to restate 40 commands.
                "inputSchema": {
                    "type": "object",
                    "additionalProperties": true,
                },
            }));
        }
        Value::Array(tools)
    }

    /// Run one tool. A refused command is a tool *result*, not a protocol error: the agent should read the
    /// reason ("arrives in M8", "read-only library") and decide what to do next rather than retry a frame.
    fn invoke(&mut self, name: &str, arguments: &Value) -> Value {
        match self.engine.run(name, arguments) {
            Ok(value) => craft_mcp::text_tool_result(&value.to_string(), false),
            Err(error) => craft_mcp::text_tool_result(&error.to_string(), true),
        }
    }
}

/// The bounds this server serves under.
///
/// Eight megabytes is more than any tool call and far less than any library: the limit exists so that a client
/// cannot turn one line into an allocation, not to be generous.
pub const MAX_MESSAGE_BYTES: usize = 8 * 1024 * 1024;

/// How many requests one batch may hold. MCP clients send one at a time; a batch is accepted because JSON-RPC
/// allows it, and bounded because a batch is also a way to ask for a thousand things at once.
pub const MAX_BATCH_ITEMS: usize = 64;

/// The bounds above, as `craft-jsonrpc` wants them.
pub fn limits() -> Limits {
    Limits {
        max_message_bytes: MAX_MESSAGE_BYTES,
        max_batch_items: MAX_BATCH_ITEMS,
        // MCP clients use string or number ids, and a null id is discouraged but not fatal: refusing it would
        // reject a client that works with every other server for no benefit to anyone.
        integer_ids_only: false,
        allow_null_ids: true,
    }
}

/// Read and throw away the rest of an over-long line, without holding it in memory.
fn discard_line<R: BufRead>(input: &mut R) -> std::io::Result<()> {
    loop {
        let bytes = input.fill_buf()?;
        if bytes.is_empty() {
            return Ok(());
        }
        match bytes.iter().position(|&byte| byte == b'\n') {
            Some(newline) => {
                input.consume(newline + 1);
                return Ok(());
            }
            None => {
                let len = bytes.len();
                input.consume(len);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    fn server(name: &str) -> (Mcp, tempdir::Temp) {
        let temp = tempdir::Temp::new(name);
        let engine = Engine::open(&temp.0).expect("open");
        (Mcp::new(engine), temp)
    }

    /// A tiny scratch directory, so the crate keeps its no-dependency promise.
    mod tempdir {
        use std::path::PathBuf;

        pub struct Temp(pub PathBuf);

        impl Temp {
            pub fn new(name: &str) -> Self {
                let path = std::env::temp_dir().join(format!("sortlens-mcp-{name}-{}", std::process::id()));
                let _ = std::fs::remove_dir_all(&path);
                std::fs::create_dir_all(&path).expect("dir");
                Self(path)
            }
        }

        impl Drop for Temp {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    fn call(server: &mut Mcp, method: &str, params: Value) -> Value {
        let request = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
        let line = server.handle(&request.to_string()).expect("a response");
        serde_json::from_str(&line).expect("valid JSON")
    }

    #[test]
    fn initialize_names_the_protocol_and_the_server() {
        let (mut server, _temp) = server("init");
        let response = call(&mut server, "initialize", json!({}));
        assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(response["result"]["serverInfo"]["name"], "sortlens");
        assert!(response["result"]["capabilities"]["tools"].is_object());
    }

    /// Negotiation: a client asking for a version this server knows is answered with its own version, and one
    /// asking for a version nobody has heard of gets the newest this server speaks.
    #[test]
    fn the_protocol_version_is_negotiated_not_announced() {
        let (mut server, _temp) = server("negotiate");
        let older = call(&mut server, "initialize", json!({ "protocolVersion": "2024-11-05" }));
        assert_eq!(older["result"]["protocolVersion"], "2024-11-05");
        let unknown = call(&mut server, "initialize", json!({ "protocolVersion": "1999-01-01" }));
        assert_eq!(unknown["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(unknown["result"]["serverInfo"]["name"], "sortlens");
        assert!(
            unknown["result"]["instructions"]
                .as_str()
                .unwrap_or_default()
                .contains("command id"),
            "the session carries the instructions that say what a tool is"
        );
    }

    /// The initialized notification completes the handshake, and sending it is not the same as being answered.
    #[test]
    fn the_initialized_notification_completes_the_handshake() {
        let (mut server, _temp) = server("handshake");
        assert!(!server.is_initialized());
        let line = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string();
        assert!(server.handle(&line).is_none(), "a notification is never answered");
        assert!(server.is_initialized(), "but the session noticed it");
    }

    /// Tool arguments must be an object: a tool call with a string where the arguments go is a bad request, not
    /// an empty call, and saying so is more useful than running the command with nothing.
    #[test]
    fn tool_arguments_must_be_an_object() {
        let (mut server, _temp) = server("arguments");
        let answer = call(&mut server, "tools/call", json!({ "name": "library.stats", "arguments": "all" }));
        assert_eq!(answer["error"]["code"], INVALID_PARAMS);
        let fine = call(&mut server, "tools/call", json!({ "name": "library.stats", "arguments": {} }));
        assert!(fine["result"]["content"][0]["text"].is_string());
    }

    #[test]
    fn the_tool_list_is_the_command_registry() {
        let (mut server, _temp) = server("tools");
        let response = call(&mut server, "tools/list", json!({}));
        let tools = response["result"]["tools"].as_array().expect("tools");
        assert!(tools.len() > 10, "{} tools", tools.len());
        let names: Vec<&str> = tools.iter().filter_map(|tool| tool["name"].as_str()).collect();
        assert!(names.contains(&"photo.rate"), "{names:?}");
        assert!(names.contains(&"library.stats"), "{names:?}");
        // Planned commands are not offered: a tool that can only refuse is noise. Places is not one of
        // them any more — the map shipped, and the registry row that said otherwise was stale.
        assert!(names.contains(&"view.places"), "the map is reachable: {names:?}");
        assert!(!names.contains(&"library.import"), "planned commands stay out: {names:?}");
        // Every tool carries the description an agent needs to choose it.
        for tool in tools {
            assert!(tool["description"].as_str().is_some_and(|text| !text.is_empty()));
            assert_eq!(tool["inputSchema"]["type"], "object");
        }
    }

    #[test]
    fn a_tool_call_returns_a_result_and_a_refusal_is_a_result_too() {
        let (mut server, _temp) = server("call");
        let stats = call(&mut server, "tools/call", json!({ "name": "library.stats", "arguments": {} }));
        assert_eq!(stats["result"]["isError"], false);
        let text = stats["result"]["content"][0]["text"].as_str().expect("text");
        assert!(text.contains("total"), "{text}");

        // A command that is not ready is an error result with a readable reason, not a crash.
        let planned = call(&mut server, "tools/call", json!({ "name": "library.import" }));
        assert_eq!(planned["result"]["isError"], true);
        let reason = planned["result"]["content"][0]["text"].as_str().expect("reason");
        assert!(reason.contains("M1") || reason.contains("planned"), "{reason}");

        // So is an unknown tool, and a missing name is a protocol-level error.
        let unknown = call(&mut server, "tools/call", json!({ "name": "no.suchCommand" }));
        assert_eq!(unknown["result"]["isError"], true);
        let nameless = call(&mut server, "tools/call", json!({}));
        assert_eq!(nameless["error"]["code"], INVALID_PARAMS);
    }

    #[test]
    fn an_agent_can_cull_through_the_same_op_log_the_app_writes() {
        let (mut server, temp) = server("cull");
        let photos = temp.0.join("photos");
        std::fs::create_dir_all(&photos).expect("dir");
        std::fs::write(photos.join("a.jpg"), b"aaa").expect("write");
        std::fs::write(photos.join("b.jpg"), b"bbb").expect("write");

        call(
            &mut server,
            "tools/call",
            json!({ "name": "library.addFolder", "arguments": { "path": photos.to_string_lossy() } }),
        );
        // The scan runs on a thread; poll the way the app does until it lands.
        for _ in 0..200 {
            if server.engine().poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let listing = call(&mut server, "tools/call", json!({ "name": "library.stats", "arguments": {} }));
        assert_eq!(listing["result"]["isError"], false, "{listing}");
        let text = listing["result"]["content"][0]["text"].as_str().expect("text");
        assert!(text.contains("total"), "{text}");

        let rated = call(
            &mut server,
            "tools/call",
            json!({ "name": "photo.rate", "arguments": { "ids": [1], "stars": 5 } }),
        );
        assert_eq!(rated["result"]["isError"], false, "{rated}");
        assert!(server.engine().photo(sortlens_core::PhotoId(1)).expect("photo").rating == 5);

        // The write is in the op log, so the app's undo and history see it too.
        let history = call(&mut server, "tools/call", json!({ "name": "library.history" }));
        let text = history["result"]["content"][0]["text"].as_str().expect("text");
        assert!(text.contains("rate"), "{text}");
    }

    /// Every published example is run against a real fixture library.
    ///
    /// This is the test that makes `examples.rs` trustworthy: an argument name the engine does not
    /// accept comes back as `isError`, and the failure message names the command and the reason, so
    /// the fix is to correct the example rather than to relax the test.
    #[test]
    fn every_published_example_actually_runs() {
        let (mut server, temp) = server("examples");
        let photos = temp.0.join("photos");
        std::fs::create_dir_all(photos.join("day2")).expect("dir");
        for name in ["a.jpg", "b.jpg", "c.jpg"] {
            std::fs::write(photos.join(name), name.as_bytes()).expect("write");
        }
        std::fs::write(photos.join("day2/d.jpg"), b"dddd").expect("write");
        call(
            &mut server,
            "tools/call",
            json!({ "name": "library.addFolder", "arguments": { "path": photos.to_string_lossy() } }),
        );
        for _ in 0..400 {
            if server.engine().poll().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        // Some examples need state another example creates; the table is ordered for that. These two
        // need a path only the caller can choose, so they are set up here instead.
        let archive = temp.0.join("library.backup");
        let restored = temp.0.join("restored");
        let moved = temp.0.join("moved");
        std::fs::create_dir_all(&restored).expect("dir");
        std::fs::create_dir_all(&moved).expect("dir");

        let mut failures = Vec::new();
        for example in examples::EXAMPLES {
            let mut args: Value = serde_json::from_str(example.args).expect("json");
            if example.command == "library.restoreBackup" {
                call(
                    &mut server,
                    "tools/call",
                    json!({
                        "name": "library.backup",
                        "arguments": { "file": archive.to_string_lossy() },
                    }),
                );
                args = json!({
                    "file": archive.to_string_lossy(),
                    "dest": restored.to_string_lossy(),
                });
            }
            if example.command == "edit.movePicks" {
                args = json!({ "dest": moved.to_string_lossy(), "ids": [3] });
            }
            if example.command == "library.subfolders" {
                args = json!({ "root": photos.to_string_lossy(), "prefix": "" });
            }
            // Two examples refer to state an earlier example would have to create, so they are
            // created here and the id is read back rather than guessed.
            if example.command == "edit.undoMove" {
                let receipt = call(
                    &mut server,
                    "tools/call",
                    json!({
                        "name": "edit.movePicks",
                        "arguments": { "dest": moved.to_string_lossy(), "ids": [3] },
                    }),
                );
                let text = receipt["result"]["content"][0]["text"].as_str().unwrap_or_default();
                let id = serde_json::from_str::<Value>(text)
                    .ok()
                    .and_then(|value| value["receipt"].as_u64())
                    .unwrap_or(1);
                args = json!({ "id": id });
            }
            // Each of these needs a batch of its own: one is restored and one is forgotten, and a
            // shared batch would let the second call pass by accident.
            if example.command == "library.restoreBatch" {
                args = json!({ "id": applied_batch(&mut server, 1) });
            }
            if example.command == "library.forgetBatch" {
                args = json!({ "id": applied_batch(&mut server, 2) });
            }
            let result = call(&mut server, "tools/call", json!({ "name": example.command, "arguments": args }));
            if result["result"]["isError"] == true {
                let reason = result["result"]["content"][0]["text"].as_str().unwrap_or_default();
                failures.push(format!("{}: {reason}", example.command));
            }
        }
        assert!(
            failures.is_empty(),
            "{} published example(s) do not run:\n{}",
            failures.len(),
            failures.join("\n")
        );

        // The commands that need a caller are documented with the argument they need rather than
        // silently absent, and their published description carries that note.
        let listed = call(&mut server, "tools/list", json!({}));
        let tools = listed["result"]["tools"].as_array().expect("tools");
        for (id, note) in examples::NEEDS_CALLER {
            assert!(!note.is_empty(), "{id} has no note");
            let tool = tools
                .iter()
                .find(|tool| tool["name"] == *id)
                .unwrap_or_else(|| panic!("{id} is not in the tool list"));
            let description = tool["description"].as_str().expect("description");
            assert!(description.contains(note), "{id} does not publish its argument note: {description}");
        }
    }

    /// Stage and apply one reject on `photo` so the bin really has a batch, and return the newest
    /// batch id. It reads the id back rather than assuming one: an earlier example may have applied
    /// a batch already, and a guessed id would make the test lie about what it verified.
    fn applied_batch(server: &mut Mcp, photo: u64) -> u64 {
        for (name, args) in [
            ("cull.clear", json!({})),
            ("cull.start", json!({ "ids": [photo] })),
            ("cull.decide", json!({ "decision": "reject" })),
            ("cull.apply", json!({ "mode": "tag" })),
        ] {
            let result = call(server, "tools/call", json!({ "name": name, "arguments": args }));
            assert_ne!(
                result["result"]["isError"],
                true,
                "{name} failed: {}",
                result["result"]["content"][0]["text"].as_str().unwrap_or_default()
            );
        }
        let deleted = call(server, "tools/call", json!({ "name": "library.deleted", "arguments": {} }));
        let text = deleted["result"]["content"][0]["text"].as_str().unwrap_or_default();
        serde_json::from_str::<Value>(text)
            .ok()
            .and_then(|value| {
                value["batches"]
                    .as_array()
                    .and_then(|batches| batches.iter().filter_map(|batch| batch["id"].as_u64()).max())
            })
            .expect("a batch id after applying")
    }

    #[test]
    fn a_notification_gets_no_answer_and_bad_input_gets_a_protocol_error() {
        let (mut server, _temp) = server("protocol");
        let line = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string();
        assert!(server.handle(&line).is_none(), "a notification must not be answered");

        // Not JSON is a *parse* error, which JSON-RPC numbers -32700; the old hand-rolled server called it
        // an invalid request, and a client cannot fix a malformed frame it is told is a bad request.
        let broken = server.handle("{not json").expect("a response");
        let parsed: Value = serde_json::from_str(&broken).expect("valid JSON");
        assert_eq!(parsed["error"]["code"], PARSE_ERROR);
        assert_eq!(parsed["id"], Value::Null);

        let missing = call(&mut server, "no/such/method", json!({}));
        assert_eq!(missing["error"]["code"], METHOD_NOT_FOUND);
        assert_eq!(call(&mut server, "ping", json!({}))["result"], json!({}));
    }

    /// An envelope without `jsonrpc: "2.0"` is refused: the version is not decoration, and a server that
    /// accepts anything teaches clients to send anything.
    #[test]
    fn an_envelope_without_the_version_is_refused() {
        let (mut server, _temp) = server("envelope");
        let unversioned = json!({ "id": 1, "method": "ping" }).to_string();
        let answer = server.handle(&unversioned).expect("a response");
        let parsed: Value = serde_json::from_str(&answer).expect("valid JSON");
        assert_eq!(parsed["error"]["code"], INVALID_REQUEST);
        assert_eq!(
            parsed["id"],
            Value::Null,
            "there is no id to echo: the frame was refused before dispatch"
        );
    }

    /// A batch is answered as a batch, and a batch over the item bound is refused whole.
    #[test]
    fn a_batch_is_answered_as_a_batch_within_the_bound() {
        let (mut server, _temp) = server("batch");
        let batch = json!([
            { "jsonrpc": "2.0", "id": 1, "method": "ping" },
            { "jsonrpc": "2.0", "method": "notifications/initialized" },
            { "jsonrpc": "2.0", "id": 2, "method": "tools/list" },
        ])
        .to_string();
        let answer = server.handle(&batch).expect("a response");
        let parsed: Value = serde_json::from_str(&answer).expect("valid JSON");
        let replies = parsed.as_array().expect("a batch reply");
        assert_eq!(replies.len(), 2, "the notification is not answered");
        assert_eq!(replies[0]["id"], 1);
        assert_eq!(replies[1]["id"], 2);

        // Three requests against a bound of two: refused before any of them runs.
        let two = Limits {
            max_batch_items: 2,
            ..limits()
        };
        let answer = server.handle_with(&batch, two).expect("a response");
        let parsed: Value = serde_json::from_str(&answer).expect("valid JSON");
        assert_eq!(parsed["error"]["code"], INVALID_REQUEST);
    }

    /// The message bound protects the allocation: an oversized line is refused before it is parsed, and
    /// serving continues after it rather than dying on one bad frame.
    #[test]
    fn an_oversized_message_is_refused_and_the_session_survives() {
        let (mut server, _temp) = server("oversize");
        let small = Limits {
            max_message_bytes: 64,
            ..limits()
        };
        let oversized = json!({ "jsonrpc": "2.0", "id": 9, "method": "ping", "params": { "pad": "x".repeat(200) } });
        let input = format!("{oversized}\n{}\n", json!({ "jsonrpc": "2.0", "id": 10, "method": "ping" }));
        let mut output = Vec::new();
        let answered = server.serve_with(input.as_bytes(), &mut output, small).expect("serve");
        assert_eq!(answered, 2, "the bad frame is answered and so is the good one");
        let text = String::from_utf8(output).expect("utf8");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        let first: Value = serde_json::from_str(lines[0]).expect("valid JSON");
        assert_eq!(first["error"]["code"], PARSE_ERROR);
        assert!(first["error"]["message"].as_str().unwrap_or_default().contains("64 bytes"));
        let second: Value = serde_json::from_str(lines[1]).expect("valid JSON");
        assert_eq!(second["id"], 10, "the session kept going");
        assert_eq!(second["result"], json!({}));
    }

    #[test]
    fn serving_a_stream_answers_each_request_on_its_own_line() {
        let (mut server, _temp) = server("stream");
        let input = format!(
            "{}\n\n{}\n",
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }),
            json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }),
        );
        let mut output = Vec::new();
        let answered = server.serve(input.as_bytes(), &mut output).expect("serve");
        assert_eq!(answered, 2, "the blank line is not a request");
        let text = String::from_utf8(output).expect("utf8");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("protocolVersion"));
        assert!(lines[1].contains("tools"));
        for line in lines {
            assert!(serde_json::from_str::<Value>(line).is_ok(), "one message per line");
        }
    }
}
