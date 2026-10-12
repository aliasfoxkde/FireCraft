//! Script-file battery (M9 task 4): `.jsf` loading through
//! `engine::platform::FileSystem`, `fw.runScript` chains,
//! `fw.runCommands`, and the typed no-FS refusal. The "shipped
//! Commands/ script" leg uses an own-authored stand-in with the
//! shipped corpus's file shapes (UTF-8 BOM + CRLF + `fw.runScript`
//! chain) — committed code never references `source/`.
//! Quirk ledger: `docs/decisions/0012-es3-quirk-ledger.md`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::rc::Rc;

use doc::{Canvas, Cell, Document, Layer, LayerList, Page, PageInfo};
use engine::Session;
use engine::platform::{FileSystem, MemoryFs};
use scripting::{Limits, Outcome, ScriptEngine, run_in_session, run_with_file_system};

// --- fixtures ------------------------------------------------------------

fn page_info() -> PageInfo {
    PageInfo {
        version: None,
        revision: None,
        frame_count: 1,
        frame_count_duplicate: 1,
        request_id: 0,
        canvas: Canvas::default(),
        onion_before: 0,
        onion_after: 0,
        frame_durations: Vec::new(),
    }
}

/// 96x64 canvas, one page, two empty layers ("bottom", "top") — the
/// mutate battery's base document.
fn base_document() -> Document {
    let mut document = Document::default();
    document.info.canvas = Canvas {
        x: 0.0,
        y: 0.0,
        width: 96.0,
        height: 64.0,
        resolution: 72.0,
        resolution_unit: 0,
        background_raw: 0,
    };
    document.info.frame_count = 1;
    document.pages.push(Page {
        info: page_info(),
        name: Some("Page 1".to_owned()),
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: LayerList {
            layers: vec![
                Layer {
                    name: "bottom".to_owned(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements: Vec::new(),
                    }],
                    ..Layer::default()
                },
                Layer {
                    name: "top".to_owned(),
                    display: true,
                    cells: vec![Cell {
                        locked: false,
                        visible: true,
                        elements: Vec::new(),
                    }],
                    ..Layer::default()
                },
            ],
        },
        grid: None,
        guides: doc::Guides::default(),
        export: None,
    });
    document
}

fn bytes(document: &Document) -> Vec<u8> {
    serde_json::to_vec(document).expect("document serializes")
}

fn engine() -> ScriptEngine {
    ScriptEngine::new(Limits::default()).expect("engine")
}

fn fs() -> Rc<dyn FileSystem> {
    Rc::new(MemoryFs::new())
}

// --- Files.read/write/exists ----------------------------------------------

#[test]
fn files_round_trip_read_write_exists() {
    let fs = fs();
    let mut session = Session::new(base_document());
    let mut script = engine();
    let report = run_with_file_system(fs.clone(), || {
        run_in_session(
            &mut session,
            &mut script,
            "files.jsf",
            "fw.__log(Files.exists('Commands/notes.txt')); \
             Files.write('Commands/notes.txt', 'hi from a script'); \
             fw.__log(Files.read('Commands/notes.txt')); \
             fw.__log(Files.exists('Commands/notes.txt'));",
        )
    });
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(
        report.notes,
        vec!["false", "hi from a script", "true"],
        "exists/read/see each other's writes through the attached FS"
    );
    // The writes went to the injected FS, not anywhere else.
    assert_eq!(
        String::from_utf8(fs.read_file("Commands/notes.txt").expect("written")).expect("utf8"),
        "hi from a script"
    );
    // File access is not a document mutation: no journal rows.
    assert_eq!(session.journal_len(), 0);
}

// --- the stopping condition: a Commands/ script loads and runs -----------

#[test]
fn command_script_loads_from_virtual_fs_and_runs() {
    let fs = fs();
    // Shipped file shapes: UTF-8 BOM, CRLF line endings, a runScript
    // chain (assemble.jsf loads part.jsf from the same virtual FS).
    fs.write_file(
        "Commands/assemble.jsf",
        "\u{feff}// assemble: pull in the part\r\nfw.runScript('Commands/part.jsf');\r\n"
            .as_bytes(),
    )
    .unwrap();
    fs.write_file(
        "Commands/part.jsf",
        "\u{feff}var dom = fw.getDocumentDOM();\r\ndom.addNewRectangle(4, 6, 32, 16, 'part');\r\n"
            .as_bytes(),
    )
    .unwrap();

    // The host reads the top-level script itself (the CLI/run posture),
    // and the run resolves the chain through the attached FS.
    let main = String::from_utf8(fs.read_file("Commands/assemble.jsf").expect("main script"))
        .expect("utf8");
    let mut session = Session::new(base_document());
    let mut script = engine();
    let report = run_with_file_system(fs, || {
        run_in_session(&mut session, &mut script, "assemble.jsf", &main)
    });
    assert!(report.completed(), "{:?}", report.message);

    // The queued mutation from the loaded file ran: script ≡ direct.
    let scripted_bytes = bytes(session.document());
    let mut direct = Session::new(base_document());
    direct
        .apply(
            "object.createPath",
            serde_json::json!({
                "kind": "rect",
                "x": 4.0, "y": 6.0, "width": 32.0, "height": 16.0,
                "page": 0, "layer": 1, "cell": 0, "name": "part",
            }),
        )
        .expect("direct apply");
    assert_eq!(scripted_bytes, bytes(direct.document()));
    let replay = Session::reopen(base_document(), &session.journal_lines()).expect("reopens");
    assert_eq!(scripted_bytes, bytes(replay.document()));
}

// --- refusals --------------------------------------------------------------

#[test]
fn scripts_refuse_to_run_without_an_attached_fs() {
    let mut session = Session::new(base_document());
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "nofs.jsf",
        "fw.runScript('Commands/anything.jsf');",
    );
    assert_eq!(report.outcome, Outcome::Threw);
    assert!(
        report
            .message
            .as_ref()
            .expect("message")
            .contains("no file system is attached"),
        "typed refusal, never a silent default: {:?}",
        report.message
    );
    assert_eq!(session.journal_len(), 0);
}

// --- fw.runCommands ---------------------------------------------------------

#[test]
fn run_commands_queues_each_string_argument() {
    // `page.add` takes no required arguments, so the empty payload
    // drains successfully — the corpus's no-argument command shape.
    let mut session = Session::new(base_document());
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "cmds.jsf",
        "fw.runCommands('page.add');",
    );
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(session.document().pages.len(), 2);
    assert_eq!(session.journal_len(), 1);

    // A command with required arguments fails typed at drain: the name
    // is visible, nothing applied (it was the first intent).
    let base = base_document();
    let mut session = Session::new(base.clone());
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "bad-cmd.jsf",
        "fw.runCommands('text.createText');",
    );
    assert_eq!(report.outcome, Outcome::Threw);
    assert!(
        report
            .message
            .as_ref()
            .expect("message")
            .contains("text.createText"),
        "the failing command id is reported: {:?}",
        report.message
    );
    assert_eq!(bytes(session.document()), bytes(&base));
    assert_eq!(session.journal_len(), 0);

    // Non-string arguments refuse in-run (before any drain).
    let mut session = Session::new(base);
    let mut script = engine();
    let report = run_in_session(
        &mut session,
        &mut script,
        "bad-arg.jsf",
        "fw.runCommands('page.add', 42);",
    );
    assert_eq!(report.outcome, Outcome::Threw);
    assert!(
        report
            .message
            .expect("message")
            .contains("must be a command name string")
    );
    assert_eq!(session.journal_len(), 0);
}

// --- sandbox bounds ----------------------------------------------------------

#[test]
fn run_script_recursion_is_bounded() {
    let fs = fs();
    fs.write_file("Commands/a.jsf", b"fw.runScript('Commands/b.jsf');")
        .unwrap();
    fs.write_file("Commands/b.jsf", b"fw.runScript('Commands/a.jsf');")
        .unwrap();
    // Tight recursion limit so the interpreter bound — not the real
    // machine stack — is what fires.
    let mut script = ScriptEngine::new(Limits {
        recursion: 64,
        ..Limits::default()
    })
    .expect("engine");
    let mut session = Session::new(base_document());
    let base = base_document();
    let report = run_with_file_system(fs, || {
        run_in_session(
            &mut session,
            &mut script,
            "a.jsf",
            "fw.runScript('Commands/a.jsf');",
        )
    });
    assert_eq!(report.outcome, Outcome::LimitExceeded, "{report:?}");
    assert_eq!(bytes(session.document()), bytes(&base));
    assert_eq!(session.journal_len(), 0);
}

#[test]
fn preprocessor_directives_are_not_processed() {
    // Quirk ledger 0012: the shipped corpus contains no `#target`/
    // `#include` lines, so the loader does not pre-process `#` lines —
    // one in a script is a syntax error (typed Threw), never a
    // silently ignored directive.
    let fs = fs();
    fs.write_file(
        "Commands/directive.jsf",
        b"#target fireworks\nfw.__log('never');",
    )
    .unwrap();
    let mut session = Session::new(base_document());
    let mut script = engine();
    let report = run_with_file_system(fs, || {
        run_in_session(
            &mut session,
            &mut script,
            "directive.jsf",
            "fw.runScript('Commands/directive.jsf');",
        )
    });
    assert_eq!(report.outcome, Outcome::Threw, "{report:?}");
    assert_eq!(session.journal_len(), 0);
}
