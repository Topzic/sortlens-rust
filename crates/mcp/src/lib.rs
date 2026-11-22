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

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented)]

use std::io::{BufRead, Write};

use serde_json::{Value, json};
use sortlens_engine::Engine;

/// Crate version, from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The protocol version this server implements.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// JSON-RPC error code: the method does not exist.
pub const METHOD_NOT_FOUND: i64 = -32601;
/// JSON-RPC error code: the request was not valid JSON-RPC.
pub const INVALID_REQUEST: i64 = -32600;
/// JSON-RPC error code: the parameters were wrong.
pub const INVALID_PARAMS: i64 = -32602;

/// One MCP server bound to an open library.
pub struct Mcp {
    engine: Engine,
}

impl Mcp {
    /// Wrap an engine.
    pub fn new(engine: Engine) -> Self {
        Self { engine }
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
        let request: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                return Some(error_response(Value::Null, INVALID_REQUEST, &format!("that is not JSON: {error}")));
            }
        };
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(Value::as_str).unwrap_or_default();
        let params = request.get("params").cloned().unwrap_or(Value::Null);

        let result = match method {
            "initialize" => Ok(self.initialize()),
            "notifications/initialized" | "initialized" => return None,
            "ping" => Ok(json!({})),
            "tools/list" => Ok(self.tools()),
            "tools/call" => self.call(&params),
            other => Err((METHOD_NOT_FOUND, format!("no such method: {other}"))),
        };

        match (id, result) {
            // No id: a notification, so nothing goes back even if it failed.
            (None, _) => None,
            (Some(id), Ok(result)) => Some(
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": result,
                })
                .to_string(),
            ),
            (Some(id), Err((code, message))) => Some(error_response(id, code, &message)),
        }
    }

    /// Serve stdio until the input ends.
    ///
    /// Returns the number of requests answered. A read or write error ends the loop: a client that
    /// has closed the pipe has nothing more to say, and there is nowhere left to report to.
    pub fn serve<R: BufRead, W: Write>(&mut self, input: R, mut output: W) -> std::io::Result<usize> {
        let mut answered = 0;
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Some(response) = self.handle(&line) {
                writeln!(output, "{response}")?;
                output.flush()?;
                answered += 1;
            }
        }
        Ok(answered)
    }

    fn initialize(&self) -> Value {
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "sortlens",
                "version": VERSION,
            },
            "instructions": "Every tool is a Sortlens command id. Call tools/list for the ready \
                             ones; a tool still listed as planned refuses and names its milestone.",
        })
    }

    /// The tool list, straight from the registry.
    fn tools(&self) -> Value {
        let mut tools = Vec::new();
        for command in self.engine.registry().all() {
            if !command.is_ready() {
                continue;
            }
            tools.push(json!({
                "name": command.id,
                "description": format!("{} ({})", command.label, command.menu),
                // Deliberately permissive: the engine validates every argument and says what was
                // wrong, which is more truthful than a schema that tries to restate 40 commands.
                "inputSchema": {
                    "type": "object",
                    "additionalProperties": true,
                },
            }));
        }
        json!({ "tools": tools })
    }

    /// Run one tool: `{ name, arguments }`.
    fn call(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or((INVALID_PARAMS, "a tool name is required".to_string()))?;
        let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
        match self.engine.run(name, &args) {
            Ok(value) => Ok(json!({
                "content": [{ "type": "text", "text": value.to_string() }],
                "isError": false,
            })),
            // A refused command is a tool result, not a protocol error: the agent should read the
            // reason ("arrives in M8", "read-only library") and decide what to do next.
            Err(error) => Ok(json!({
                "content": [{ "type": "text", "text": error.to_string() }],
                "isError": true,
            })),
        }
    }
}

fn error_response(id: Value, code: i64, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
    .to_string()
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

    #[test]
    fn the_tool_list_is_the_command_registry() {
        let (mut server, _temp) = server("tools");
        let response = call(&mut server, "tools/list", json!({}));
        let tools = response["result"]["tools"].as_array().expect("tools");
        assert!(tools.len() > 10, "{} tools", tools.len());
        let names: Vec<&str> = tools.iter().filter_map(|tool| tool["name"].as_str()).collect();
        assert!(names.contains(&"photo.rate"), "{names:?}");
        assert!(names.contains(&"library.stats"), "{names:?}");
        // Planned commands are not offered: a tool that can only refuse is noise.
        assert!(!names.contains(&"view.places"), "planned commands stay out: {names:?}");
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
        let planned = call(&mut server, "tools/call", json!({ "name": "view.places" }));
        assert_eq!(planned["result"]["isError"], true);
        let reason = planned["result"]["content"][0]["text"].as_str().expect("reason");
        assert!(reason.contains("M5") || reason.contains("planned"), "{reason}");

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

    #[test]
    fn a_notification_gets_no_answer_and_bad_input_gets_a_protocol_error() {
        let (mut server, _temp) = server("protocol");
        let line = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string();
        assert!(server.handle(&line).is_none(), "a notification must not be answered");

        let broken = server.handle("{not json").expect("a response");
        let parsed: Value = serde_json::from_str(&broken).expect("valid JSON");
        assert_eq!(parsed["error"]["code"], INVALID_REQUEST);
        assert_eq!(parsed["id"], Value::Null);

        let missing = call(&mut server, "no/such/method", json!({}));
        assert_eq!(missing["error"]["code"], METHOD_NOT_FOUND);
        assert_eq!(call(&mut server, "ping", json!({}))["result"], json!({}));
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
