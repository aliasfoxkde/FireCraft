//! The loopback JSON control channel (M9 task 7): one request, one
//! JSON reply, every mutation through [`engine::Session::apply`] — the
//! same registry door the GUI and every other surface use.
//!
//! Transport-free by design: the stdin loop in the binary's `serve`
//! verb and the tests drive the same [`handle`]. Native-only per
//! PLANNING §7.4 (the wasm shell embeds the engine directly); a
//! malformed request is a typed error reply, never a panic — that is
//! the channel's fuzz contract.
//!
//! Request shapes:
//!
//! ```json
//! {"command": "object.createPath", "args": {"kind": "rect", ...}}
//! ```

use engine::Session;

/// Handles one control request against `session` and returns the JSON
/// reply. Unknown fields are ignored; a missing/ill-typed `command`,
/// or a command the registry refuses, comes back as `ok: false` with
/// the reason — the session is untouched either way.
pub fn handle(session: &mut Session, request: &serde_json::Value) -> serde_json::Value {
    let Some(name) = request.get("command").and_then(serde_json::Value::as_str) else {
        return error_reply("request needs a string \"command\" field");
    };
    let args = request
        .get("args")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    match session.apply(name, args) {
        Ok(()) => serde_json::json!({ "ok": true, "journal": session.journal_len() }),
        Err(error) => error_reply(&error.to_string()),
    }
}

/// The typed failure reply.
fn error_reply(error: &str) -> serde_json::Value {
    serde_json::json!({ "ok": false, "error": error })
}
