//! The M9 task 7 stopping condition: ONE command id through the GUI,
//! the CLI control channel, and the MCP server lands three byte-equal
//! documents — and the hostile-input contract holds everywhere
//! (malformed JSON is a typed reply, never a panic, and never a
//! touched journal).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::rc::Rc;

use app::editor::Editor;
use app_cli::{channel, mcp};
use engine::Session;
use engine::platform::{FileSystem, MemoryFs};

/// The one command id every surface drives in the parity battery.
const CREATE: &str = "object.createPath";

/// The identical payload each surface hands the registry.
fn create_args() -> serde_json::Value {
    serde_json::json!({
        "kind": "rect",
        "x": 10.0,
        "y": 20.0,
        "width": 80.0,
        "height": 40.0,
        "page": 0,
        "layer": 0,
        "cell": 0,
        "name": "plate",
    })
}

/// An `Rc` file system for the MCP dispatcher (runScript never runs in
/// this battery, but the handle signature carries it).
fn mcp_fs() -> Rc<dyn FileSystem> {
    Rc::new(MemoryFs::new())
}

#[test]
fn one_command_id_through_gui_cli_and_mcp() {
    // One shared base: the untitled seed (a page, a layer, a frame) —
    // a bare `Document::default()` has no pages to create into.
    let base = Editor::untitled().session.document().clone();

    // GUI surface: the editor's apply (the canvas path).
    let mut gui = Editor::from_document(base.clone());
    gui.apply(CREATE, create_args()).expect("GUI apply");

    // CLI surface: the `serve` verb's dispatcher, one JSON request.
    let mut cli = Session::new(base.clone());
    let reply = channel::handle(
        &mut cli,
        &serde_json::json!({ "command": CREATE, "args": create_args() }),
    );
    assert_eq!(reply["ok"], serde_json::json!(true), "channel: {reply}");

    // MCP surface: the `mcp` verb's dispatcher, tools/call
    // firecraft.apply with the same registry id.
    let mut via_mcp = Session::new(base);
    let reply = mcp::handle(
        &mut via_mcp,
        mcp_fs(),
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "firecraft.apply",
                "arguments": { "command": CREATE, "args": create_args() },
            },
        }),
    )
    .expect("tools/call answers");
    assert!(reply["error"].is_null(), "mcp: {reply}");
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .expect("text result");
    let result: serde_json::Value = serde_json::from_str(text).expect("tool payload is JSON");
    assert_eq!(result["ok"], serde_json::json!(true), "{text}");

    // Three surfaces, one registry: byte-equal documents, one journal
    // row each.
    let gui_doc = serde_json::to_vec(gui.session.document()).unwrap();
    assert_eq!(
        serde_json::to_vec(cli.document()).unwrap(),
        gui_doc,
        "the control channel diverged from the GUI"
    );
    assert_eq!(
        serde_json::to_vec(via_mcp.document()).unwrap(),
        gui_doc,
        "the MCP server diverged from the GUI"
    );
    assert_eq!(cli.journal_len(), 1);
    assert_eq!(via_mcp.journal_len(), 1);
}

/// The M11 task 5 battery: the web-layer ids through the same three
/// surfaces — `slice.create`, then `slice.setLink` naming the id the
/// create minted (identical everywhere because every surface starts
/// from the same base).
#[test]
fn web_command_ids_through_gui_cli_and_mcp() {
    let base = Editor::untitled().session.document().clone();
    let create = serde_json::json!({
        "shape": "rect",
        "x": 8.0, "y": 8.0, "width": 64.0, "height": 32.0,
        "page": 0,
        "name": "banner", "link": null, "alt": null,
    });

    // Scratch run: learn the id `slice.create` mints so the link
    // payload can name it (same slot on every surface).
    let mut scratch = Session::new(base.clone());
    let reply = channel::handle(
        &mut scratch,
        &serde_json::json!({ "command": "slice.create", "args": create }),
    );
    assert_eq!(reply["ok"], serde_json::json!(true), "channel: {reply}");
    let slice_id = scratch.document().pages[0]
        .layers
        .layers
        .iter()
        .find(|layer| layer.has_url && layer.is_special)
        .and_then(|layer| layer.cells[0].elements.first())
        .copied()
        .expect("the created slice is on the web layer");
    let link = serde_json::json!({
        "id": slice_id,
        "link": "https://example.test/",
        "alt": "banner",
    });

    // GUI surface: slice-create → link through Editor::apply.
    let mut gui = Editor::from_document(base.clone());
    gui.apply("slice.create", create.clone())
        .expect("GUI create");
    gui.apply("slice.setLink", link.clone()).expect("GUI link");

    // CLI surface: the control channel, both rows.
    let mut cli = Session::new(base.clone());
    for (command, args) in [("slice.create", &create), ("slice.setLink", &link)] {
        let reply = channel::handle(
            &mut cli,
            &serde_json::json!({ "command": command, "args": args }),
        );
        assert_eq!(reply["ok"], serde_json::json!(true), "channel: {reply}");
    }

    // MCP surface: firecraft.apply, both rows.
    let mut via_mcp = Session::new(base);
    for (command, args) in [("slice.create", &create), ("slice.setLink", &link)] {
        let reply = mcp::handle(
            &mut via_mcp,
            mcp_fs(),
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": 9,
                "method": "tools/call",
                "params": {
                    "name": "firecraft.apply",
                    "arguments": { "command": command, "args": args },
                },
            }),
        )
        .expect("tools/call answers");
        assert!(reply["error"].is_null(), "mcp: {reply}");
    }

    // Three surfaces, one registry: byte-equal documents.
    let gui_doc = serde_json::to_vec(gui.session.document()).unwrap();
    assert_eq!(
        serde_json::to_vec(cli.document()).unwrap(),
        gui_doc,
        "the control channel diverged from the GUI"
    );
    assert_eq!(
        serde_json::to_vec(via_mcp.document()).unwrap(),
        gui_doc,
        "the MCP server diverged from the GUI"
    );
}

#[test]
fn mcp_handshake_and_tool_table() {
    let mut session = Session::new(doc::Document::default());
    let fs = mcp_fs();

    let init = mcp::handle(
        &mut session,
        fs.clone(),
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "battery", "version": "0" },
            },
        }),
    )
    .expect("initialize answers");
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "firecraft");

    // The client's initialized notification takes no reply at all.
    let notified = mcp::handle(
        &mut session,
        fs.clone(),
        &serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
    );
    assert!(notified.is_none(), "notifications take no reply");

    let ping = mcp::handle(
        &mut session,
        fs.clone(),
        &serde_json::json!({ "jsonrpc": "2.0", "id": 2, "method": "ping" }),
    )
    .expect("ping answers");
    assert!(ping["result"].is_object(), "ping: {ping}");

    let tools = mcp::handle(
        &mut session,
        fs,
        &serde_json::json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/list" }),
    )
    .expect("tools/list answers");
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .expect("tool array")
        .iter()
        .map(|tool| tool["name"].as_str().expect("tool name"))
        .collect();
    assert!(names.contains(&"firecraft.apply"), "tools: {names:?}");
    assert!(
        names.contains(&"firecraft.runScript"),
        "scripting builds ship runScript: {names:?}"
    );
}

#[test]
fn hostile_json_is_typed_never_panic() {
    // Requests no dispatcher should ever accept. Every reply is
    // well-formed, every journal stays empty.
    let garbage: Vec<serde_json::Value> = vec![
        serde_json::Value::Null,
        serde_json::json!([]),
        serde_json::json!(42),
        serde_json::json!("apply"),
        serde_json::json!({}),
        serde_json::json!({ "command": null }),
        serde_json::json!({ "command": 42 }),
        serde_json::json!({ "command": [] }),
        serde_json::json!({ "command": "no.such.command" }),
        serde_json::json!({ "command": CREATE }),
        serde_json::json!({ "command": CREATE, "args": "nope" }),
        serde_json::json!({ "command": CREATE, "args": { "kind": 42 } }),
        serde_json::json!({ "command": CREATE, "args": { "kind": "rect", "x": "1" } }),
    ];

    let mut cli = Session::new(doc::Document::default());
    for request in &garbage {
        let reply = channel::handle(&mut cli, request);
        assert_eq!(
            reply["ok"],
            serde_json::json!(false),
            "channel {request}: {reply}"
        );
        assert!(reply["error"].is_string(), "channel {request}: {reply}");
    }
    assert_eq!(
        cli.journal_len(),
        0,
        "hostile input must not touch the journal"
    );

    // The MCP dispatcher over the same garbage plus the protocol-level
    // shapes; every answer is a well-formed JSON-RPC envelope.
    let fs = mcp_fs();
    let mut protocol = garbage.clone();
    protocol.extend([
        serde_json::json!({ "id": 1 }),
        serde_json::json!({ "method": 42, "id": 1 }),
        serde_json::json!({ "method": "no/such", "id": 1 }),
        serde_json::json!({ "method": "tools/call", "id": 1 }),
        serde_json::json!({ "method": "tools/call", "id": 1, "params": {} }),
        serde_json::json!({ "method": "tools/call", "id": 1, "params": { "name": "nope" } }),
        serde_json::json!({ "method": "tools/call", "id": 1, "params": { "name": "firecraft.apply" } }),
        serde_json::json!({
            "method": "tools/call",
            "id": 1,
            "params": { "name": "firecraft.apply", "arguments": { "command": "no.such" } },
        }),
        serde_json::json!({ "method": "tools/call", "id": 1, "params": { "name": "firecraft.runScript" } }),
    ]);
    let mut via_mcp = Session::new(doc::Document::default());
    for request in &protocol {
        if let Some(reply) = mcp::handle(&mut via_mcp, fs.clone(), request) {
            assert_eq!(reply["jsonrpc"], "2.0", "mcp {request}: {reply}");
            let well_formed = reply["error"].is_object() ^ reply["result"].is_object();
            assert!(well_formed, "mcp {request}: {reply}");
        }
    }
    assert_eq!(
        via_mcp.journal_len(),
        0,
        "hostile input must not touch the journal"
    );

    // A frame that never parses at all gets the typed parse error.
    let parse = mcp::parse_error("expected value");
    assert_eq!(parse["error"]["code"], serde_json::json!(-32700));
    assert!(parse["id"].is_null());
}

/// The expansion battery: info / listCommands / undo / redo / export
/// tools and the `document://current` resource, all over one session
/// that starts from the untitled seed and mutates exactly once.
#[test]
fn mcp_expanded_tools_and_resource() {
    let base = Editor::untitled().session.document().clone();
    let mut session = Session::new(base);
    let fs = Rc::new(MemoryFs::new());

    let call = |session: &mut Session, name: &str, arguments: serde_json::Value| {
        mcp::handle(
            session,
            Rc::new(MemoryFs::new()) as Rc<dyn FileSystem>,
            &serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": { "name": name, "arguments": arguments },
            }),
        )
        .expect("tools/call answers")
    };
    let text_of = |reply: &serde_json::Value| -> serde_json::Value {
        let text = reply["result"]["content"][0]["text"]
            .as_str()
            .expect("text");
        serde_json::from_str(text).expect("tool payload is JSON")
    };

    // Mutate, then info sees the object; listCommands carries the id.
    let reply = call(
        &mut session,
        "firecraft.apply",
        serde_json::json!(
        { "command": CREATE, "args": create_args() }),
    );
    assert!(
        reply["result"]["isError"] == serde_json::json!(false),
        "{reply}"
    );

    let reply = call(&mut session, "firecraft.info", serde_json::json!({}));
    let summary = text_of(&reply);
    assert_eq!(
        summary["objects"]["live"],
        serde_json::json!(1),
        "{summary}"
    );

    let reply = call(
        &mut session,
        "firecraft.listCommands",
        serde_json::json!({}),
    );
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(text.split('\n').any(|line| line == CREATE), "{text}");

    // undo steps back to zero objects, redo restores them.
    let reply = call(&mut session, "firecraft.undo", serde_json::json!({}));
    let result = text_of(&reply);
    assert_eq!(result["ok"], serde_json::json!(true), "{result}");
    assert_eq!(session.undo_len(), 0, "the undo stack is empty");
    assert_eq!(session.document().registry.len(), 0);
    let reply = call(&mut session, "firecraft.redo", serde_json::json!({}));
    let result = text_of(&reply);
    assert_eq!(result["ok"], serde_json::json!(true), "{result}");
    assert_eq!(session.document().registry.len(), 1);
    // Undo again (legal — redo put the step back), then a further undo
    // past the now-empty stack is a tool-level failure, not a panic.
    let reply = call(&mut session, "firecraft.undo", serde_json::json!({}));
    assert_eq!(
        reply["result"]["isError"],
        serde_json::json!(false),
        "{reply}"
    );
    let reply = call(&mut session, "firecraft.undo", serde_json::json!({}));
    assert_eq!(
        reply["result"]["isError"],
        serde_json::json!(true),
        "{reply}"
    );

    // export renders and writes through the injected file system.
    let export_fs: Rc<dyn FileSystem> = Rc::new(MemoryFs::new());
    let reply = mcp::handle(
        &mut session,
        export_fs.clone(),
        &serde_json::json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": { "name": "firecraft.export",
                        "arguments": { "path": "test://out.png" } },
        }),
    )
    .expect("tools/call answers");
    assert!(
        reply["result"]["isError"] == serde_json::json!(false),
        "{reply}"
    );
    // Parameter validation surfaces as tool-level errors.
    let reply = call(
        &mut session,
        "firecraft.export",
        serde_json::json!({ "path": "x.png", "quality": 0 }),
    );
    assert_eq!(
        reply["result"]["isError"],
        serde_json::json!(true),
        "{reply}"
    );

    // The resource: document://current reads back the same summary.
    let resources = mcp::handle(
        &mut session,
        fs,
        &serde_json::json!({ "jsonrpc": "2.0", "id": 3, "method": "resources/list" }),
    )
    .expect("resources/list answers");
    assert_eq!(
        resources["result"]["resources"][0]["uri"],
        "document://current"
    );
    let read = mcp::handle(
        &mut session,
        Rc::new(MemoryFs::new()) as Rc<dyn FileSystem>,
        &serde_json::json!({
            "jsonrpc": "2.0", "id": 4, "method": "resources/read",
            "params": { "uri": "document://current" },
        }),
    )
    .expect("resources/read answers");
    let text = read["result"]["contents"][0]["text"]
        .as_str()
        .expect("text");
    let summary: serde_json::Value = serde_json::from_str(text).expect("summary is JSON");
    // The session is back at zero objects after the extra undo above;
    // the point is the resource reflects the *live* session state.
    assert_eq!(summary["objects"]["live"], serde_json::json!(0));
    // Unknown resources are typed errors.
    let read = mcp::handle(
        &mut session,
        Rc::new(MemoryFs::new()) as Rc<dyn FileSystem>,
        &serde_json::json!({
            "jsonrpc": "2.0", "id": 5, "method": "resources/read",
            "params": { "uri": "document://nope" },
        }),
    )
    .expect("resources/read answers");
    assert!(read["error"].is_object(), "{read}");
}
