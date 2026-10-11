//! Property test over the whole catalog: random calls against random
//! documents must never panic, failed calls must not mutate, and every
//! successful call must undo and redo exactly. This is the in-tree half
//! of `panic_hunt`; the standalone fuzz workspace drives the same
//! generator harder (`fuzz/src/bin/panic_hunt.rs`, target
//! `engine_commands`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use engine::Session;
use engine::fuzzdrive::{FuzzRng, random_call, random_document};

/// Byte-level document equality (`serde_json`, deterministic field
/// order).
fn bytes(document: &doc::Document) -> Vec<u8> {
    serde_json::to_vec(document).unwrap()
}

const SEEDS: [u64; 3] = [0x1, 0xDEAD_BEEF, 0x5EED_F00D];
const STEPS: usize = 64;

#[test]
fn random_sequences_never_panic_and_undo_restores() {
    for seed in SEEDS {
        let mut rng = FuzzRng::new(seed);
        let mut session = Session::new(random_document(&mut rng));
        for step in 0..STEPS {
            let before = bytes(session.document());
            let (name, payload) = random_call(&mut rng, session.document());
            match session.apply(&name, payload.clone()) {
                Err(error) => {
                    assert_eq!(
                        bytes(session.document()),
                        before,
                        "failed call mutated the document (seed {seed}, step {step}, \
                         {name} {payload}): {error}"
                    );
                }
                Ok(()) => {
                    let applied = bytes(session.document());
                    session.undo().unwrap_or_else(|error| {
                        panic!("undo broke (seed {seed}, step {step}): {error}")
                    });
                    assert_eq!(
                        bytes(session.document()),
                        before,
                        "undo diverged (seed {seed}, step {step}, {name} {payload})"
                    );
                    session.redo().unwrap_or_else(|error| {
                        panic!("redo broke (seed {seed}, step {step}): {error}")
                    });
                    assert_eq!(
                        bytes(session.document()),
                        applied,
                        "redo diverged (seed {seed}, step {step}, {name} {payload})"
                    );
                }
            }
        }
    }
}

#[test]
fn random_histories_replay_to_the_same_document() {
    for seed in [0x2u64, 0x1234_5678_9ABC_DEF0] {
        let mut rng = FuzzRng::new(seed);
        let document = random_document(&mut rng);
        let original = serde_json::to_vec(&document).unwrap();
        let mut session = Session::new(document);
        for _step in 0..48 {
            let (name, payload) = random_call(&mut rng, session.document());
            // Refused calls (corrupt payloads, stale targets) simply do
            // not journal; the replay only has to match what applied.
            let _ = session.apply(&name, payload);
        }
        let edited = bytes(session.document());
        let lines = session.journal_lines();

        // Restart: a fresh copy of the original + the journal replays
        // to the same document, and undo crosses the restart boundary
        // all the way back.
        let fresh: doc::Document = serde_json::from_slice(&original).unwrap();
        let mut restarted = Session::reopen(fresh, &lines)
            .unwrap_or_else(|error| panic!("replay broke (seed {seed}): {error}"));
        assert_eq!(
            bytes(restarted.document()),
            edited,
            "replay diverged (seed {seed})"
        );
        while restarted.undo_len() > 0 {
            restarted
                .undo()
                .unwrap_or_else(|error| panic!("replayed undo broke (seed {seed}): {error}"));
        }
        assert_eq!(
            serde_json::to_vec(restarted.document()).unwrap(),
            original,
            "replayed undo did not reach the original (seed {seed})"
        );
    }
}
