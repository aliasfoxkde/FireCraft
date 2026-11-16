//! The element mask stack (decision 0092): a stack is nested mask
//! groups, and the level commands grow, reorder, and shrink it.
//!
//! Every command here must undo byte-exactly — a stack restructures
//! the document (objects move between containers), so a stack command
//! that does not round-trip would corrupt a document on undo.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use engine::CommandRegistry;
use engine::catalog::element_mask_stack;
use serde_json::json;

use support::sample_document;

fn builtins() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    engine::catalog::install(&mut registry).expect("builtins install");
    registry
}

/// Byte-level document equality (`serde_json`, deterministic order).
fn bytes(document: &doc::Document) -> Vec<u8> {
    serde_json::to_vec(document).expect("document bytes")
}

/// A document with one masked path: `ids[0]` carries an element mask,
/// `ids[2]` is a free path that can become a level's mask.
fn masked_document() -> (doc::Document, Vec<doc::ObjectId>) {
    let registry = builtins();
    let (mut document, ids) = sample_document();
    registry
        .execute(
            &mut document,
            "mask.add",
            json!({ "id": ids[0], "mode": "reveal all" }),
        )
        .expect("mask.add");
    (document, ids)
}

#[test]
fn a_mask_carries_one_level() {
    let (document, ids) = masked_document();
    let levels = element_mask_stack(&document, ids[0]);
    assert_eq!(levels.len(), 1, "the mask is one level");
    assert!(levels[0].enabled, "the level is enabled");
    assert!(levels[0].linked, "the level's mask is linked by default");
    assert_ne!(levels[0].mask, levels[0].group, "the mask is not the group");
}

#[test]
fn a_level_unlinks_its_own_mask() {
    let registry = builtins();
    let (mut document, ids) = masked_document();
    registry
        .execute(
            &mut document,
            "mask.addLevel",
            json!({ "id": ids[0], "source": ids[2] }),
        )
        .expect("mask.addLevel");
    let levels = element_mask_stack(&document, ids[0]);
    assert!(levels[0].linked, "the new level is linked");
    assert!(levels[1].linked, "the old level is linked");

    // The link flag rides the level's own mask group, so unlinking
    // one level leaves the other alone.
    registry
        .execute(
            &mut document,
            "mask.link",
            json!({ "id": levels[0].group, "linked": false }),
        )
        .expect("mask.link");
    let unlinked = element_mask_stack(&document, ids[0]);
    assert!(!unlinked[0].linked, "the top level is unlinked");
    assert!(unlinked[1].linked, "the level below is still linked");
}

#[test]
fn a_level_is_added_over_the_stack() {
    let registry = builtins();
    let (mut document, ids) = masked_document();
    let before = element_mask_stack(&document, ids[0]);
    assert_eq!(before.len(), 1);

    registry
        .execute(
            &mut document,
            "mask.addLevel",
            json!({ "id": ids[0], "source": ids[2], "mode": "subtract" }),
        )
        .expect("mask.addLevel");

    let levels = element_mask_stack(&document, ids[0]);
    assert_eq!(levels.len(), 2, "the stack grew");
    assert_eq!(levels[0].mask, ids[2], "the new level is on top");
    assert_eq!(
        levels[0].mode,
        doc::MaskCombineMode::Subtract,
        "the new mode"
    );
    assert_eq!(levels[1].mask, before[0].mask, "the old level is below");
}

#[test]
fn a_level_without_a_source_mints_a_reveal_all_mask() {
    let registry = builtins();
    let (mut document, ids) = masked_document();
    registry
        .execute(&mut document, "mask.addLevel", json!({ "id": ids[0] }))
        .expect("mask.addLevel");
    let levels = element_mask_stack(&document, ids[0]);
    assert_eq!(levels.len(), 2, "the stack grew");
    assert_eq!(
        levels[0].mode,
        doc::MaskCombineMode::Add,
        "the default mode"
    );
    assert!(levels[0].enabled, "the new level is enabled");
}

#[test]
fn a_level_moves_up_and_down() {
    let registry = builtins();
    let (mut document, ids) = masked_document();
    registry
        .execute(
            &mut document,
            "mask.addLevel",
            json!({ "id": ids[0], "source": ids[2] }),
        )
        .expect("mask.addLevel");
    let levels = element_mask_stack(&document, ids[0]);
    let top = levels[0];
    let bottom = levels[1];

    // The bottom level moves up: the two swap places.
    registry
        .execute(
            &mut document,
            "mask.moveLevel",
            json!({ "id": bottom.group, "delta": 1 }),
        )
        .expect("mask.moveLevel");
    let moved = element_mask_stack(&document, ids[0]);
    assert_eq!(moved[0].group, bottom.group, "the bottom level is on top");
    assert_eq!(moved[1].group, top.group, "the top level is below");
    assert_eq!(moved[0].mask, bottom.mask, "the level keeps its mask");
    assert_eq!(moved[0].mode, bottom.mode, "the level keeps its mode");

    // And back down.
    registry
        .execute(
            &mut document,
            "mask.moveLevel",
            json!({ "id": bottom.group, "delta": -1 }),
        )
        .expect("mask.moveLevel");
    let back = element_mask_stack(&document, ids[0]);
    assert_eq!(back[0].group, top.group, "the order is back");
    assert_eq!(back[1].group, bottom.group, "the order is back");
}

#[test]
fn a_level_is_removed() {
    let registry = builtins();
    let (mut document, ids) = masked_document();
    registry
        .execute(
            &mut document,
            "mask.addLevel",
            json!({ "id": ids[0], "source": ids[2] }),
        )
        .expect("mask.addLevel");
    let levels = element_mask_stack(&document, ids[0]);

    registry
        .execute(
            &mut document,
            "mask.removeLevel",
            json!({ "id": levels[0].group }),
        )
        .expect("mask.removeLevel");
    let left = element_mask_stack(&document, ids[0]);
    assert_eq!(left.len(), 1, "the top level is gone");
    assert_eq!(left[0].group, levels[1].group, "the level below remains");
}

#[test]
fn every_level_command_undoes_byte_exactly() {
    let registry = builtins();
    let (mut document, ids) = masked_document();

    let before = bytes(&document);
    let added = registry
        .execute(
            &mut document,
            "mask.addLevel",
            json!({ "id": ids[0], "source": ids[2] }),
        )
        .expect("mask.addLevel");
    let after_add = bytes(&document);
    assert_ne!(after_add, before, "the stack grew");

    let levels = element_mask_stack(&document, ids[0]);
    let moved = registry
        .execute(
            &mut document,
            "mask.moveLevel",
            json!({ "id": levels[1].group, "delta": 1 }),
        )
        .expect("mask.moveLevel");
    let after_move = bytes(&document);

    let removed = registry
        .execute(
            &mut document,
            "mask.removeLevel",
            json!({ "id": levels[0].group }),
        )
        .expect("mask.removeLevel");

    // Undo in reverse: the removal, the move, the addition.
    removed.apply_inverse(&mut document).expect("undo remove");
    assert_eq!(bytes(&document), after_move, "undo of the removal");
    moved.apply_inverse(&mut document).expect("undo move");
    assert_eq!(bytes(&document), after_add, "undo of the move");
    added.apply_inverse(&mut document).expect("undo add");
    assert_eq!(bytes(&document), before, "undo of the addition");
}

#[test]
fn a_disabled_level_keeps_its_place_and_its_mode_stays_editable() {
    let registry = builtins();
    let (mut document, ids) = masked_document();
    registry
        .execute(
            &mut document,
            "mask.addLevel",
            json!({ "id": ids[0], "source": ids[2] }),
        )
        .expect("mask.addLevel");
    let levels = element_mask_stack(&document, ids[0]);
    let top = levels[0];

    // The top level is disabled: it stays in the stack, in place, but
    // contributes no coverage.
    registry
        .execute(
            &mut document,
            "mask.enable",
            json!({ "id": top.group, "enable": false }),
        )
        .expect("mask.enable");
    let disabled = element_mask_stack(&document, ids[0]);
    assert_eq!(disabled.len(), 2, "the stack keeps both levels");
    assert_eq!(disabled[0].group, top.group, "the level keeps its place");
    assert!(!disabled[0].enabled, "the level is disabled");

    // A disabled level's combine mode is still editable — the level
    // keeps its mode across the enable/disable round trip.
    registry
        .execute(
            &mut document,
            "mask.setStackMode",
            json!({ "id": top.group, "mode": "intersect" }),
        )
        .expect("mask.setStackMode");
    let edited = element_mask_stack(&document, ids[0]);
    assert_eq!(
        edited[0].mode,
        doc::MaskCombineMode::Intersect,
        "the disabled level's mode"
    );
    assert!(!edited[0].enabled, "the level is still disabled");

    // And re-enabling keeps the mode.
    registry
        .execute(
            &mut document,
            "mask.enable",
            json!({ "id": top.group, "enable": true }),
        )
        .expect("mask.enable");
    let enabled = element_mask_stack(&document, ids[0]);
    assert!(enabled[0].enabled, "the level is enabled again");
    assert_eq!(
        enabled[0].mode,
        doc::MaskCombineMode::Intersect,
        "the mode survives the round trip"
    );
}

#[test]
fn a_refused_level_command_leaves_the_document_alone() {
    let registry = builtins();
    let (mut document, ids) = masked_document();
    let before = bytes(&document);

    // An object that carries no mask has no stack to extend.
    let refused = registry.execute(
        &mut document,
        "mask.addLevel",
        json!({ "id": ids[4], "source": ids[2] }),
    );
    assert!(refused.is_err(), "an unmasked object has no stack");
    assert_eq!(bytes(&document), before, "the refusal changed nothing");

    // A source that is already part of the stack cannot join it twice.
    let levels = element_mask_stack(&document, ids[0]);
    let refused = registry.execute(
        &mut document,
        "mask.addLevel",
        json!({ "id": ids[0], "source": levels[0].mask }),
    );
    assert!(refused.is_err(), "the source is already in the stack");
    assert_eq!(bytes(&document), before, "the refusal changed nothing");
}
