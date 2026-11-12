//! `frame.setDuration` (decision 0081): the frame delay the animation
//! timeline needs. The page's duration record is positional, undoable
//! through the session, and refuses a target outside the page or a
//! delay the GIF delay field cannot carry.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use doc::DEFAULT_FRAME_DURATION;
use engine::Session;
use serde_json::json;

use support::sample_document;

fn session() -> Session {
    let (document, _) = sample_document();
    Session::new(document)
}

#[test]
fn a_frame_delay_is_recorded_and_undone() {
    let mut session = session();

    session
        .apply(
            "frame.setDuration",
            json!({ "page": 0, "frame": 1, "duration": 25 }),
        )
        .expect("apply");
    assert_eq!(
        session.document().pages[0].info.frame_durations,
        vec![DEFAULT_FRAME_DURATION, 25],
        "the record is positional and the untouched frame keeps the default"
    );

    session.undo().expect("undo");
    assert!(
        session.document().pages[0].info.frame_durations.is_empty(),
        "undo restores the page's own record"
    );

    session.redo().expect("redo");
    assert_eq!(
        session.document().pages[0].info.frame_durations,
        vec![DEFAULT_FRAME_DURATION, 25],
        "redo re-applies the delay"
    );
}

#[test]
fn a_delay_outside_the_page_is_refused() {
    let mut session = session();
    let error = session
        .apply(
            "frame.setDuration",
            json!({ "page": 0, "frame": 9, "duration": 10 }),
        )
        .expect_err("frame 9 is outside the page");
    assert!(
        matches!(error, engine::EngineError::MissingTarget { .. }),
        "{error}"
    );
    let error = session
        .apply(
            "frame.setDuration",
            json!({ "page": 9, "frame": 0, "duration": 10 }),
        )
        .expect_err("page 9 does not exist");
    assert!(
        matches!(error, engine::EngineError::MissingTarget { .. }),
        "{error}"
    );
    assert!(
        session.document().pages[0].info.frame_durations.is_empty(),
        "a refused call mutates nothing"
    );
}

#[test]
fn a_delay_outside_the_gif_range_is_refused() {
    let mut session = session();
    for duration in [-1, 65_536] {
        let error = session
            .apply(
                "frame.setDuration",
                json!({ "page": 0, "frame": 0, "duration": duration }),
            )
            .expect_err("the GIF delay field cannot carry this");
        assert!(
            matches!(error, engine::EngineError::BadPayload { .. }),
            "{error}"
        );
    }
    assert!(
        session.document().pages[0].info.frame_durations.is_empty(),
        "a refused call mutates nothing"
    );
}

#[test]
fn a_page_with_an_impossible_frame_count_is_refused() {
    let (mut document, _) = sample_document();
    // A corrupt document can claim more frames than the format allows;
    // the duration record must not be sized from it.
    document.pages[0].info.frame_count = 1_000_000;
    let mut session = Session::new(document);
    let error = session
        .apply(
            "frame.setDuration",
            json!({ "page": 0, "frame": 0, "duration": 10 }),
        )
        .expect_err("the page claims too many frames");
    assert!(
        matches!(error, engine::EngineError::InvalidState { .. }),
        "{error}"
    );
    assert!(
        session.document().pages[0].info.frame_durations.is_empty(),
        "a refused call mutates nothing"
    );
}
