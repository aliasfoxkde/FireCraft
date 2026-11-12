//! Phase 20.2: the Optimize panel's binding.
//!
//! The panel's controls are data ([`OptimizeSettings`]); this exercises
//! the path they travel: the panel's payload → the engine command →
//! the page's `EXPv` record → the export resolution the encoders read.
//! A control that does not reach the encoder is a control that lies.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app::editor::Editor;
use app::shell::OptimizeSettings;
use serde_json::json;

/// The Optimize settings of an untouched document.
fn settings(editor: &Editor) -> OptimizeSettings {
    OptimizeSettings::read(&editor.document().pages[0])
}

#[test]
fn an_untouched_page_reads_as_the_codec_defaults() {
    let editor = Editor::untitled();
    let read = settings(&editor);
    assert_eq!(read.format, 2);
    assert_eq!(read.jpeg_quality, 90);
    assert_eq!(read.dither, 0);
    assert_eq!(read.frame_loop, 0);
    assert!(!read.interlaced());
}

#[test]
fn the_jpeg_controls_reach_the_page_record() {
    let mut editor = Editor::untitled();
    let mut panel = settings(&editor);
    panel.format = 1;
    panel.jpeg_quality = 42;
    panel.jpeg_progressive = true;
    panel.matte_raw = Some(0xFFFF_FFFF);
    editor
        .apply("page.setExport", panel.payload(0))
        .expect("the panel's payload is a valid command");

    let read = settings(&editor);
    assert_eq!(read.format, 1);
    assert_eq!(read.jpeg_quality, 42);
    assert!(read.jpeg_progressive);
    assert_eq!(read.matte_raw, Some(0xFFFF_FFFF));
}

#[test]
fn the_gif_controls_reach_the_page_record() {
    let mut editor = Editor::untitled();
    let mut panel = settings(&editor);
    panel.format = 0;
    panel.dither = 30;
    panel.frame_loop = -1;
    panel.anim_crop = true;
    panel.anim_diff = true;
    editor
        .apply("page.setExport", panel.payload(0))
        .expect("the panel's payload is a valid command");

    let read = settings(&editor);
    assert_eq!(read.format, 0);
    assert_eq!(read.dither, 30);
    assert_eq!(read.frame_loop, -1);
    assert!(read.anim_crop);
    assert!(read.anim_diff);
}

#[test]
fn the_controls_reach_the_export_resolution() {
    let mut editor = Editor::untitled();
    let mut panel = settings(&editor);
    panel.format = 0;
    panel.dither = 30;
    panel.frame_loop = -1;
    panel.anim_crop = true;
    panel.anim_diff = true;
    editor
        .apply("page.setExport", panel.payload(0))
        .expect("the panel's payload is a valid command");

    // The GIF exporter reads the record through the same resolution the
    // panel's preview uses.
    let export = io::expv::for_page(editor.document(), 0);
    let options = io::expv::gif_options(export);
    assert_eq!(options.dither, 30);
    assert!(options.crop);
    assert!(options.diff);
    assert_eq!(options.repeat, io::gif::Loop::Never);
}

#[test]
fn the_jpeg_quality_reaches_the_export_resolution() {
    let mut editor = Editor::untitled();
    let mut panel = settings(&editor);
    panel.format = 1;
    panel.jpeg_quality = 42;
    editor
        .apply("page.setExport", panel.payload(0))
        .expect("the panel's payload is a valid command");

    let export = io::expv::for_page(editor.document(), 0);
    let options = io::expv::jpeg_options(export, io::expv::StatedJpeg::default());
    assert_eq!(options.quality, 42);
}

#[test]
fn a_format_change_clears_the_knobs_it_cannot_use() {
    let mut editor = Editor::untitled();
    let mut panel = settings(&editor);
    panel.format = 0;
    panel.dither = 30;
    editor
        .apply("page.setExport", panel.payload(0))
        .expect("the panel's payload is a valid command");
    assert_eq!(settings(&editor).dither, 30);

    // Switching to JPEG retires the GIF dither rather than leaving a
    // setting behind that the JPEG path cannot honor.
    let mut panel = settings(&editor);
    panel.format = 1;
    editor
        .apply("page.setExport", panel.payload(0))
        .expect("the panel's payload is a valid command");
    assert_eq!(settings(&editor).dither, 0);
    assert_eq!(settings(&editor).format, 1);
}

#[test]
fn an_optimize_edit_is_one_undoable_step() {
    let mut editor = Editor::untitled();
    let mut panel = settings(&editor);
    panel.format = 1;
    panel.jpeg_quality = 42;
    editor
        .apply("page.setExport", panel.payload(0))
        .expect("the panel's payload is a valid command");
    assert_eq!(settings(&editor).jpeg_quality, 42);

    editor.undo().expect("the edit undoes");
    assert_eq!(settings(&editor).jpeg_quality, 90);
}

#[test]
fn the_interlace_flag_is_only_cleared_on_request() {
    let mut editor = Editor::untitled();
    // A document that records interlaced output (CS6 wrote the flag).
    editor
        .apply("page.setExport", json!({ "page": 0, "interlaced": true }))
        .expect("the flag is a valid command");
    assert!(settings(&editor).interlaced());

    // Another edit leaves the flag alone: the panel never writes it as
    // a side effect.
    let mut panel = settings(&editor);
    panel.format = 1;
    editor
        .apply("page.setExport", panel.payload(0))
        .expect("the panel's payload is a valid command");
    assert!(settings(&editor).interlaced());

    editor
        .apply("page.setExport", json!({ "page": 0, "interlaced": false }))
        .expect("the flag is a valid command");
    assert!(!settings(&editor).interlaced());
}
