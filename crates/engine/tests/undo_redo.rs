//! Undo/redo property: for randomized command sequences over the
//! catalog, undo replays exactly to the original document and redo
//! re-applies exactly to the final one; a failed command leaves the
//! document byte-identical. Document equality is `serde_json` bytes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use support::{DocumentBytes, sample_document};

use engine::{CommandRegistry, Delta, EngineError};
use serde_json::json;

/// Deterministic PRNG (xorshift64*): the sequence is reproducible from
/// the seed, no dependency needed.
struct Xorshift(u64);

impl Xorshift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

#[test]
fn undo_redo_round_trip_on_random_sequences() {
    for seed in [0x5EED_0001u64, 0x5EED_0002, 0x5EED_0003] {
        let mut rng = Xorshift(seed);
        let registry = builtins();
        let (mut document, ids) = sample_document();
        let original = DocumentBytes::of(&document);

        // A mix of valid and deliberately invalid calls; failures must
        // not disturb the document. Applied calls keep their deltas —
        // undo is delta replay, not re-execution.
        let mut applied: Vec<(String, serde_json::Value, Delta)> = Vec::new();
        for step in 0..48 {
            let (name, payload) = random_call(&mut rng, &ids, step, document.pages.len());
            let before = DocumentBytes::of(&document);
            match registry.execute(&mut document, &name, payload.clone()) {
                Ok(delta) => applied.push((name, payload, delta)),
                Err(error) => {
                    assert!(
                        matches!(
                            error,
                            EngineError::MissingTarget { .. } | EngineError::InvalidState { .. }
                        ),
                        "unexpected error from {name}: {error}"
                    );
                    assert_eq!(
                        DocumentBytes::of(&document),
                        before,
                        "failed call mutated the document"
                    );
                }
            }
        }
        let after_all = DocumentBytes::of(&document);

        // Undo everything in reverse: back to the original, byte for
        // byte.
        for (name, _, delta) in applied.iter().rev() {
            delta
                .apply_inverse(&mut document)
                .unwrap_or_else(|error| panic!("undo of {name} failed: {error}"));
        }
        assert_eq!(
            DocumentBytes::of(&document),
            original,
            "undo restores the original document"
        );

        // Redo (re-apply the payloads, in order): back to after_all.
        for (name, payload, _) in &applied {
            registry
                .execute(&mut document, name, payload.clone())
                .unwrap_or_else(|error| panic!("redo of {name} failed: {error}"));
        }
        assert_eq!(
            DocumentBytes::of(&document),
            after_all,
            "redo restores the edited document"
        );
    }
}

#[test]
fn apply_undo_apply_matches_apply_once() {
    let mut rng = Xorshift(0x5EED_0004);
    let registry = builtins();
    let (_, ids) = sample_document();
    for step in 0..32 {
        let (name, payload) = random_call(&mut rng, &ids, step, 2);
        // Path A: apply once.
        let (mut one, _) = sample_document();
        let applied_once = registry.execute(&mut one, &name, payload.clone()).is_ok();
        // Path B: apply, undo, apply.
        let (mut two, _) = sample_document();
        let first = registry.execute(&mut two, &name, payload.clone());
        if let Ok(delta) = first {
            delta.apply_inverse(&mut two).expect("undo");
        }
        let second = registry.execute(&mut two, &name, payload);
        assert_eq!(
            applied_once,
            second.is_ok(),
            "re-appliablity differs for {name}"
        );
        if applied_once {
            assert_eq!(
                DocumentBytes::of(&two),
                DocumentBytes::of(&one),
                "apply-undo-apply diverges from apply-once for {name}"
            );
        }
    }
}

fn builtins() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    engine::catalog::install(&mut registry).expect("builtins install");
    registry
}

/// Draws a (catalog id, payload) pair — three quarters of the time
/// against a live target, otherwise out of range or a stale id.
fn random_call(
    rng: &mut Xorshift,
    ids: &[doc::ObjectId],
    step: usize,
    page_count: usize,
) -> (String, serde_json::Value) {
    let bad = rng.next() % 4 == 0;
    let id = ids[rng.below(ids.len())];
    let stale = doc::ObjectId::from_parts(id.index() as u32, id.generation().wrapping_add(7));
    match step % 3 {
        0 => (
            "object.setOpacity".into(),
            json!({
                "id": if bad { stale } else { id },
                "opacity": 100 + rng.below(900) as i64,
            }),
        ),
        1 => (
            "cell.setVisible".into(),
            json!({
                "page": rng.below(page_count + usize::from(bad)),
                "layer": rng.below(2),
                "cell": rng.below(2),
                "visible": rng.next() % 2 == 0,
            }),
        ),
        _ => (
            "layer.rename".into(),
            json!({
                "page": rng.below(page_count),
                "layer": if bad { 9 } else { rng.below(2) },
                "name": format!("renamed-{}", rng.below(1000)),
            }),
        ),
    }
}
