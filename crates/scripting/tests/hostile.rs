//! The hostile-script battery: a script can fail in any way, but the
//! run reports instead of hanging or panicking.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use scripting::{Limits, Outcome, ScriptEngine};

fn tiny_limits() -> Limits {
    Limits {
        loop_iterations: 50_000,
        recursion: 64,
        stack_bytes: 1 << 18,
        host_calls: 16,
    }
}

#[test]
fn infinite_loop_hits_the_iteration_limit() {
    let mut engine = ScriptEngine::new(tiny_limits()).expect("engine");
    let report = engine.run("loop.jsf", "var x = 0; while (true) { x++; }");
    assert_eq!(report.outcome, Outcome::LimitExceeded, "{report:?}");
    let message = report.message.expect("message");
    assert!(message.contains("loop iteration"), "{message}");
}

#[test]
fn deep_recursion_hits_the_recursion_limit() {
    let mut engine = ScriptEngine::new(tiny_limits()).expect("engine");
    let report = engine.run("recur.jsf", "function f(n) { return f(n + 1) + 1; } f(0);");
    assert_eq!(report.outcome, Outcome::LimitExceeded, "{report:?}");
    let message = report.message.expect("message");
    assert!(message.contains("recursive"), "{message}");
}

#[test]
fn host_call_budget_exhausts_typed() {
    let mut engine = ScriptEngine::new(tiny_limits()).expect("engine");
    let report = engine.run(
        "budget.jsf",
        "while (fw.__budgetLeft() > 0) { fw.__log('tick'); } fw.__log('done');",
    );
    // The budget runs out mid-loop; the run ends as a limit, not a
    // completion, and 'done' never logs.
    assert_eq!(report.outcome, Outcome::LimitExceeded, "{report:?}");
    assert!(!report.notes.iter().any(|note| note == "done"));
    assert_eq!(report.host_calls, 16);
}

#[test]
fn not_implemented_names_throw_typed() {
    let mut engine = ScriptEngine::new(Limits::default()).expect("engine");
    for (source, needle) in [
        ("fw.getDocumentDOM();", "fw.getDocumentDOM"),
        ("fw.runScript('x.jsf');", "fw.runScript"),
        ("Files.read('/x');", "Files.read"),
        ("dom.addNewRectangle({});", "dom.addNewRectangle"),
        (
            "smartShape.GetCurrentContour();",
            "smartShape.GetCurrentContour",
        ),
    ] {
        let report = engine.run("ni.jsf", source);
        assert_eq!(report.outcome, Outcome::Threw, "{source}: {report:?}");
        let message = report.message.expect("message");
        assert!(
            message.contains("not implemented in FireCraft") && message.contains(needle),
            "{message}"
        );
    }
}

#[test]
fn syntax_error_reports_as_threw() {
    let mut engine = ScriptEngine::new(Limits::default()).expect("engine");
    let report = engine.run("bad.jsf", "var = ;");
    assert_eq!(report.outcome, Outcome::Threw, "{report:?}");
}

#[test]
fn a_failing_script_does_not_break_the_engine() {
    let mut engine = ScriptEngine::new(tiny_limits()).expect("engine");
    let report = engine.run("bad.jsf", "while (true) {}");
    assert_eq!(report.outcome, Outcome::LimitExceeded);
    // Same engine still runs good scripts.
    let report = engine.run("ok.jsf", "fw.__log('still alive');");
    assert!(report.completed(), "{:?}", report.message);
    assert_eq!(report.notes, vec!["still alive"]);
}
