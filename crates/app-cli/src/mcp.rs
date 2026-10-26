//! Minimal MCP (Model Context Protocol) server core (M9 task 7):
//! JSON-RPC 2.0 dispatch over newline-delimited messages, exposing the
//! SAME registry ids every other surface drives.
//!
//! Tools:
//!
//! - `firecraft.apply` — `{command, args}` through
//!   [`engine::Session::apply`]; tool-level failures come back as
//!   `isError` results (the registry's typed refusal), not protocol
//!   errors.
//! - `firecraft.runScript` — `{path}` run through the scripting host
//!   with the command queue drained into the same session (native
//!   builds with the `scripting` feature only).
//!
//! Transport lives in the binary's `mcp` verb (stdio loop); this
//! module is the pure dispatch so tests drive exactly what runs.
//! Malformed frames are typed JSON-RPC errors, never panics.

use std::rc::Rc;

use engine::Session;
use engine::platform::FileSystem;

/// The MCP protocol version this server speaks (the dated spec this
/// implementation targets).
const PROTOCOL_VERSION: &str = "2025-06-18";

/// JSON-RPC error codes this server uses.
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const PARSE_ERROR: i64 = -32700;

/// Handles one incoming JSON-RPC message against `session`; `None`
/// for notifications (which take no reply). `fs` backs the runScript
/// tool.
pub fn handle(
    session: &mut Session,
    fs: Rc<dyn FileSystem>,
    request: &serde_json::Value,
) -> Option<serde_json::Value> {
    let id = request
        .get("id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let Some(method) = request.get("method").and_then(serde_json::Value::as_str) else {
        return Some(error_reply(
            id,
            INVALID_PARAMS,
            "request needs a string \"method\"",
        ));
    };
    // Notifications take no reply, whatever their method.
    if method.starts_with("notifications/") {
        return None;
    }
    match method {
        "initialize" => Some(ok_reply(
            id,
            serde_json::json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "firecraft",
                    "version": env!("CARGO_PKG_VERSION"),
                },
            }),
        )),
        "ping" => Some(ok_reply(id, serde_json::json!({}))),
        "tools/list" => Some(ok_reply(id, tools_value())),
        "tools/call" => Some(tool_call(session, fs, id, request)),
        other => Some(error_reply(
            id,
            METHOD_NOT_FOUND,
            &format!("unknown method: {other}"),
        )),
    }
}

/// A frame that failed JSON parsing gets this typed reply (`id` null —
/// there is nothing to echo).
#[must_use]
pub fn parse_error(detail: &str) -> serde_json::Value {
    error_reply(serde_json::Value::Null, PARSE_ERROR, detail)
}

/// The tool table: fixed schemas over the same registry ids.
fn tools_value() -> serde_json::Value {
    #[allow(unused_mut)] // the second tool is feature-gated
    let mut tools = vec![serde_json::json!({
        "name": "firecraft.apply",
        "description": "Apply one FireCraft registry command to the open document.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "Registry command id." },
                "args": { "type": "object", "description": "Command payload." },
            },
            "required": ["command"],
        },
    })];
    #[cfg(feature = "scripting")]
    tools.push(serde_json::json!({
        "name": "firecraft.runScript",
        "description": "Run a .jsf script against the open document; queued \
                        mutations drain through the same registry.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Script file path." },
            },
            "required": ["path"],
        },
    }));
    serde_json::json!({ "tools": tools })
}

/// Dispatches `tools/call`. `fs` is only consumed by the feature-gated
/// runScript tool; without `scripting` it rides along unused.
fn tool_call(
    session: &mut Session,
    #[cfg_attr(not(feature = "scripting"), allow(unused_variables))] fs: Rc<dyn FileSystem>,
    id: serde_json::Value,
    request: &serde_json::Value,
) -> serde_json::Value {
    let Some(name) = request
        .pointer("/params/name")
        .and_then(serde_json::Value::as_str)
    else {
        return error_reply(id, INVALID_PARAMS, "tools/call needs params.name");
    };
    let arguments = request
        .pointer("/params/arguments")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    match name {
        "firecraft.apply" => apply_tool(session, id, &arguments),
        #[cfg(feature = "scripting")]
        "firecraft.runScript" => run_script_tool(session, fs, id, &arguments),
        other => error_reply(id, INVALID_PARAMS, &format!("unknown tool: {other}")),
    }
}

/// `firecraft.apply`: the registry door, one JSON reply per call.
fn apply_tool(
    session: &mut Session,
    id: serde_json::Value,
    arguments: &serde_json::Value,
) -> serde_json::Value {
    let Some(command) = arguments.get("command").and_then(serde_json::Value::as_str) else {
        return tool_result(
            id,
            true,
            "firecraft.apply needs a string \"command\" argument",
        );
    };
    let args = arguments
        .get("args")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    match session.apply(command, args) {
        Ok(()) => tool_result(
            id,
            false,
            &serde_json::json!({ "ok": true, "journal": session.journal_len() }).to_string(),
        ),
        Err(error) => tool_result(id, true, &error.to_string()),
    }
}

/// `firecraft.runScript`: the script runs against the session through
/// the injected file system; queued intents drain only on completion,
/// so a failing script leaves the document untouched.
#[cfg(feature = "scripting")]
fn run_script_tool(
    session: &mut Session,
    fs: Rc<dyn FileSystem>,
    id: serde_json::Value,
    arguments: &serde_json::Value,
) -> serde_json::Value {
    let Some(path) = arguments.get("path").and_then(serde_json::Value::as_str) else {
        return tool_result(
            id,
            true,
            "firecraft.runScript needs a string \"path\" argument",
        );
    };
    let source = match fs.read_file(path) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => return tool_result(id, true, &format!("runScript: {error}")),
    };
    let mut engine = match scripting::ScriptEngine::new(scripting::Limits::default()) {
        Ok(engine) => engine,
        Err(error) => return tool_result(id, true, &format!("runScript: {error}")),
    };
    let script_name = path.to_owned();
    let report = scripting::run_with_file_system(fs, || {
        scripting::run_in_session(session, &mut engine, &script_name, &source)
    });
    if report.completed() {
        tool_result(
            id,
            false,
            &serde_json::json!({
                "ok": true,
                "journal": session.journal_len(),
                "logs": report.notes,
            })
            .to_string(),
        )
    } else {
        tool_result(
            id,
            true,
            &format!(
                "{}: {}",
                match report.outcome {
                    scripting::Outcome::LimitExceeded => "limit exceeded",
                    _ => "script failed",
                },
                report.message.as_deref().unwrap_or("no message")
            ),
        )
    }
}

/// A successful tool result wrapping `text` content.
// The envelope takes the id by value: each call site moves its id into
// exactly one reply, so a reference would only force a clone at every
// caller.
#[allow(clippy::needless_pass_by_value)]
fn tool_result(id: serde_json::Value, is_error: bool, text: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{ "type": "text", "text": text }],
            "isError": is_error,
        },
    })
}

/// A successful JSON-RPC result.
// Same envelope-ownership rule as `tool_result`.
#[allow(clippy::needless_pass_by_value)]
fn ok_reply(id: serde_json::Value, result: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// A JSON-RPC error envelope.
// Same envelope-ownership rule as `tool_result`.
#[allow(clippy::needless_pass_by_value)]
fn error_reply(id: serde_json::Value, code: i64, message: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}
