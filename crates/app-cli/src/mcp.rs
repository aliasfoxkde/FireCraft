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
//! - `firecraft.info` — the document summary (the `info` verb's JSON)
//!   as text.
//! - `firecraft.export` — `{path, format?, quality?}` renders the
//!   current document and writes it to `path` (same format set as the
//!   CLI `export` verb).
//! - `firecraft.undo` / `firecraft.redo` — step the session's undo
//!   stack; the step label comes back as text.
//! - `firecraft.listCommands` — every command id the session runs
//!   (the registry's ids plus the session-scoped ones — the
//!   `engine::catalog::all_ids()` authority), one per line.
//!
//! Resources: `document://current` is the `info` summary as
//! `application/json` — agents can read it without a tool round-trip.
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
                "capabilities": { "tools": {}, "resources": {} },
                "serverInfo": {
                    "name": "firecraft",
                    "version": env!("CARGO_PKG_VERSION"),
                },
            }),
        )),
        "ping" => Some(ok_reply(id, serde_json::json!({}))),
        "tools/list" => Some(ok_reply(id, tools_value())),
        "tools/call" => Some(tool_call(session, fs, id, request)),
        "resources/list" => Some(ok_reply(id, resources_value())),
        "resources/read" => Some(resource_read(session, id, request)),
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
    tools.push(serde_json::json!({
        "name": "firecraft.info",
        "description": "Summarize the open document (frames, canvas, pages, \
                        object counts by kind) as JSON text.",
        "inputSchema": { "type": "object", "properties": {} },
    }));
    tools.push(serde_json::json!({
        "name": "firecraft.export",
        "description": "Render the open document and write it to a file, \
                        same format set as the CLI export verb.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Output file path; \
                          the extension picks the codec." },
                "quality": { "type": "integer", "description": "1-100, codec \
                             formats only." },
            },
            "required": ["path"],
        },
    }));
    tools.push(serde_json::json!({
        "name": "firecraft.undo",
        "description": "Step the document back one undo level; returns the \
                        undone step label.",
        "inputSchema": { "type": "object", "properties": {} },
    }));
    tools.push(serde_json::json!({
        "name": "firecraft.redo",
        "description": "Step the document forward one redo level; returns the \
                        redone step label.",
        "inputSchema": { "type": "object", "properties": {} },
    }));
    tools.push(serde_json::json!({
        "name": "firecraft.listCommands",
        "description": "List every registry command id the session binds, \
                        one per line.",
        "inputSchema": { "type": "object", "properties": {} },
    }));
    serde_json::json!({ "tools": tools })
}

/// The resource table: the open document's summary, refreshable by
/// re-reading (MCP resources carry no change notification here).
fn resources_value() -> serde_json::Value {
    serde_json::json!({
        "resources": [{
            "uri": "document://current",
            "name": "Current document",
            "description": "FireCraft document summary (frames, canvas, \
                            pages, object counts by kind).",
            "mimeType": "application/json",
        }]
    })
}

/// `resources/read`: only `document://current` exists; anything else is
/// a typed tool-style error envelope with the resource's URI.
fn resource_read(
    session: &Session,
    id: serde_json::Value,
    request: &serde_json::Value,
) -> serde_json::Value {
    let Some(uri) = request
        .pointer("/params/uri")
        .and_then(serde_json::Value::as_str)
    else {
        return error_reply(id, INVALID_PARAMS, "resources/read needs params.uri");
    };
    if uri != "document://current" {
        return error_reply(id, INVALID_PARAMS, &format!("unknown resource: {uri}"));
    }
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "contents": [{
                "uri": uri,
                "mimeType": "application/json",
                "text": crate::info::summarize(session.document()).to_string(),
            }],
        },
    })
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
        "firecraft.info" => info_tool(session, id),
        "firecraft.export" => export_tool(session, &fs, id, &arguments),
        "firecraft.undo" => undo_tool(session, id, false),
        "firecraft.redo" => undo_tool(session, id, true),
        "firecraft.listCommands" => list_commands_tool(session, id),
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

/// `firecraft.info`: the `info` verb's summary as JSON text.
fn info_tool(session: &Session, id: serde_json::Value) -> serde_json::Value {
    tool_result(
        id,
        false,
        &crate::info::summarize(session.document()).to_string(),
    )
}

/// `firecraft.export`: renders and writes through the injected file
/// system, so tests inspect the write without touching disk. Quality
/// is codec-only (1..=100); the extension picks the format exactly as
/// the CLI `export` verb does.
fn export_tool(
    session: &Session,
    fs: &Rc<dyn FileSystem>,
    id: serde_json::Value,
    arguments: &serde_json::Value,
) -> serde_json::Value {
    let Some(path) = arguments.get("path").and_then(serde_json::Value::as_str) else {
        return tool_result(
            id,
            true,
            "firecraft.export needs a string \"path\" argument",
        );
    };
    let quality = arguments.get("quality").and_then(serde_json::Value::as_u64);
    if let Some(q) = quality
        && !(1..=100).contains(&q)
    {
        return tool_result(id, true, "firecraft.export: quality must be 1..=100");
    }
    let flags = crate::ops::ExportFlags {
        quality: quality.map(|q| u8::try_from(q).unwrap_or(100)),
        progressive: false,
        matte: ::io::Matte::White,
    };
    let bytes = match crate::ops::export_raster(session.document(), path, &flags) {
        Ok(bytes) => bytes,
        Err(error) => return tool_result(id, true, &format!("firecraft.export: {error}")),
    };
    match fs.write_file(path, &bytes) {
        Ok(()) => tool_result(
            id,
            false,
            &serde_json::json!({ "ok": true, "path": path, "bytes": bytes.len() }).to_string(),
        ),
        Err(error) => tool_result(id, true, &format!("firecraft.export: {error}")),
    }
}

/// `firecraft.undo` / `firecraft.redo`: one stack step per call; the
/// step label comes back, failures (empty stack) are `isError`.
fn undo_tool(session: &mut Session, id: serde_json::Value, redo: bool) -> serde_json::Value {
    let outcome = if redo { session.redo() } else { session.undo() };
    match outcome {
        Ok(label) => tool_result(
            id,
            false,
            &serde_json::json!({ "ok": true, "label": label }).to_string(),
        ),
        Err(error) => tool_result(id, true, &error.to_string()),
    }
}

/// `firecraft.listCommands`: every command id the session runs, one
/// per line (sorted — discovery wants scan order, not insertion order).
///
/// The list is the registry's ids plus the session-scoped ones
/// (`history.rewind`): a command `firecraft.apply` can run belongs in
/// the discovery list, or an agent cannot find it.
fn list_commands_tool(session: &Session, id: serde_json::Value) -> serde_json::Value {
    let mut names = session.command_names();
    names.extend(engine::catalog::session_ids());
    names.sort_unstable();
    tool_result(id, false, &names.join("\n"))
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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

    use super::*;
    use engine::platform::MemoryFs;

    fn session() -> Session {
        let mut document = doc::Document::default();
        document.info.canvas.width = 64.0;
        document.info.canvas.height = 64.0;
        Session::new(document)
    }

    fn fs() -> Rc<dyn FileSystem> {
        Rc::new(MemoryFs::new())
    }

    fn request(
        session: &mut Session,
        fs: Rc<dyn FileSystem>,
        method: &str,
        params: serde_json::Value,
    ) -> serde_json::Value {
        handle(
            session,
            fs,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 17,
                "method": method,
                "params": params,
            }),
        )
        .expect("requests with an id return a reply")
    }

    fn call_tool(
        session: &mut Session,
        fs: Rc<dyn FileSystem>,
        name: &str,
        arguments: serde_json::Value,
    ) -> serde_json::Value {
        request(
            session,
            fs,
            "tools/call",
            serde_json::json!({ "name": name, "arguments": arguments }),
        )
    }

    fn result_text(reply: &serde_json::Value) -> &str {
        reply["result"]["content"][0]["text"]
            .as_str()
            .expect("tool replies carry text content")
    }

    #[test]
    fn protocol_dispatch_handles_initialize_ping_notifications_and_errors() {
        let mut session = session();
        let fs = fs();

        let initialized = request(
            &mut session,
            fs.clone(),
            "initialize",
            serde_json::json!({}),
        );
        assert_eq!(initialized["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(initialized["result"]["serverInfo"]["name"], "firecraft");

        let ping = request(&mut session, fs.clone(), "ping", serde_json::json!({}));
        assert_eq!(ping["result"], serde_json::json!({}));

        let notification = serde_json::json!({ "method": "notifications/initialized" });
        assert_eq!(handle(&mut session, fs.clone(), &notification), None);

        let unknown = request(&mut session, fs.clone(), "made/up", serde_json::json!({}));
        assert_eq!(unknown["error"]["code"], METHOD_NOT_FOUND);
        assert!(
            unknown["error"]["message"]
                .as_str()
                .unwrap()
                .contains("made/up")
        );

        let no_method = handle(&mut session, fs.clone(), &serde_json::json!({ "id": 9 }));
        assert_eq!(no_method.unwrap()["error"]["code"], INVALID_PARAMS);
        let bad_method = handle(
            &mut session,
            fs,
            &serde_json::json!({ "id": 10, "method": 4 }),
        );
        assert_eq!(bad_method.unwrap()["error"]["code"], INVALID_PARAMS);
        assert_eq!(parse_error("bad JSON")["error"]["code"], PARSE_ERROR);
    }

    #[test]
    fn tool_and_resource_lists_are_discoverable_over_json_rpc() {
        let mut session = session();
        let fs = fs();

        let tools = request(
            &mut session,
            fs.clone(),
            "tools/list",
            serde_json::json!({}),
        );
        let names: Vec<_> = tools["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        assert!(names.contains(&"firecraft.apply"));
        assert!(names.contains(&"firecraft.info"));
        assert!(names.contains(&"firecraft.listCommands"));

        let resources = request(&mut session, fs, "resources/list", serde_json::json!({}));
        assert_eq!(
            resources["result"]["resources"][0]["uri"],
            "document://current"
        );
    }

    #[test]
    fn current_document_resource_and_info_tool_share_real_document_summary() {
        let mut session = session();
        let fs = fs();
        let canvas_width = session.document().info.canvas.width;

        let resource = request(
            &mut session,
            fs.clone(),
            "resources/read",
            serde_json::json!({ "uri": "document://current" }),
        );
        let summary: serde_json::Value =
            serde_json::from_str(resource["result"]["contents"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(summary["canvas"]["width"], canvas_width);

        let missing_uri = request(
            &mut session,
            fs.clone(),
            "resources/read",
            serde_json::json!({}),
        );
        assert_eq!(missing_uri["error"]["code"], INVALID_PARAMS);
        let unknown_uri = request(
            &mut session,
            fs.clone(),
            "resources/read",
            serde_json::json!({ "uri": "document://missing" }),
        );
        assert_eq!(unknown_uri["error"]["code"], INVALID_PARAMS);

        let info = call_tool(&mut session, fs, "firecraft.info", serde_json::json!({}));
        let info_summary: serde_json::Value = serde_json::from_str(result_text(&info)).unwrap();
        assert_eq!(info_summary, summary);
        assert_eq!(info["result"]["isError"], false);
    }

    #[test]
    fn apply_undo_redo_and_list_commands_use_the_session_registry() {
        let mut session = session();
        let fs = fs();

        let no_name = request(
            &mut session,
            fs.clone(),
            "tools/call",
            serde_json::json!({}),
        );
        assert_eq!(no_name["error"]["code"], INVALID_PARAMS);
        let unknown = call_tool(
            &mut session,
            fs.clone(),
            "firecraft.unknown",
            serde_json::json!({}),
        );
        assert_eq!(unknown["error"]["code"], INVALID_PARAMS);

        let missing_command = call_tool(
            &mut session,
            fs.clone(),
            "firecraft.apply",
            serde_json::json!({}),
        );
        assert_eq!(missing_command["result"]["isError"], true);

        let pages_before = session.document().pages.len();
        let added = call_tool(
            &mut session,
            fs.clone(),
            "firecraft.apply",
            serde_json::json!({ "command": "page.add", "args": {} }),
        );
        assert_eq!(added["result"]["isError"], false);
        assert_eq!(session.document().pages.len(), pages_before + 1);
        assert!(session.journal_len() > 0);

        let commands = call_tool(
            &mut session,
            fs.clone(),
            "firecraft.listCommands",
            serde_json::json!({}),
        );
        let command_list = result_text(&commands);
        assert!(command_list.lines().any(|line| line == "history.rewind"));
        assert!(command_list.lines().any(|line| line == "page.add"));
        assert!(
            command_list
                .lines()
                .zip(command_list.lines().skip(1))
                .all(|(a, b)| a <= b)
        );

        let undone = call_tool(
            &mut session,
            fs.clone(),
            "firecraft.undo",
            serde_json::json!({}),
        );
        assert_eq!(undone["result"]["isError"], false);
        assert_eq!(session.document().pages.len(), pages_before);
        let redone = call_tool(&mut session, fs, "firecraft.redo", serde_json::json!({}));
        assert_eq!(redone["result"]["isError"], false);
        assert_eq!(session.document().pages.len(), pages_before + 1);
    }

    #[test]
    fn export_tool_validates_inputs_and_writes_a_real_png_to_the_injected_filesystem() {
        let mut session = session();
        let memory_fs = Rc::new(MemoryFs::new());
        let fs: Rc<dyn FileSystem> = memory_fs.clone();

        let page = call_tool(
            &mut session,
            fs.clone(),
            "firecraft.apply",
            serde_json::json!({ "command": "page.add", "args": {} }),
        );
        assert_eq!(page["result"]["isError"], false);

        let missing_path = call_tool(
            &mut session,
            fs.clone(),
            "firecraft.export",
            serde_json::json!({}),
        );
        assert_eq!(missing_path["result"]["isError"], true);

        let bad_quality = call_tool(
            &mut session,
            fs.clone(),
            "firecraft.export",
            serde_json::json!({ "path": "design.jpg", "quality": 101 }),
        );
        assert!(result_text(&bad_quality).contains("quality must be 1..=100"));

        let bad_format = call_tool(
            &mut session,
            fs.clone(),
            "firecraft.export",
            serde_json::json!({ "path": "design.unknown" }),
        );
        assert_eq!(bad_format["result"]["isError"], true);

        let exported = call_tool(
            &mut session,
            fs,
            "firecraft.export",
            serde_json::json!({ "path": "design.png" }),
        );
        assert_eq!(
            exported["result"]["isError"],
            false,
            "{}",
            result_text(&exported)
        );
        let bytes = memory_fs.read_file("design.png").unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[cfg(feature = "scripting")]
    #[test]
    fn script_tool_reports_a_missing_path_as_a_tool_error() {
        let mut session = session();
        let reply = call_tool(
            &mut session,
            fs(),
            "firecraft.runScript",
            serde_json::json!({}),
        );
        assert_eq!(reply["result"]["isError"], true);
        assert!(result_text(&reply).contains("needs a string \"path\""));
    }
}
