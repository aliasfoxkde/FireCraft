//! The `batch` export surface (M8 task 8): directory iteration through
//! the injected `FileSystem`, the `--options` overlay semantics
//! (per-file stored settings win where the overlay leaves `None`),
//! per-file diagnostics, and the failure-exit posture.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use app_cli::ops;
use engine::platform::FileSystem as _;

/// An in-memory file system shaped like the shell's test double.
#[derive(Default)]
struct MemFs {
    files: std::cell::RefCell<std::collections::BTreeMap<String, Vec<u8>>>,
    /// Every written path, as reported by `read_dir`.
    dirs: std::cell::RefCell<Vec<String>>,
}

impl MemFs {
    fn add(&self, path: &str, bytes: &[u8]) {
        self.files
            .borrow_mut()
            .insert(path.to_owned(), bytes.to_vec());
        self.dirs.borrow_mut().push(path.to_owned());
    }

    fn contains(&self, path: &str) -> bool {
        self.files.borrow().contains_key(path)
    }
}

impl engine::platform::FileSystem for MemFs {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, engine::EngineError> {
        self.files
            .borrow()
            .get(path)
            .cloned()
            .ok_or_else(|| engine::EngineError::Io {
                path: path.to_owned(),
                detail: "no such file".to_owned(),
            })
    }

    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), engine::EngineError> {
        self.files
            .borrow_mut()
            .insert(path.to_owned(), bytes.to_vec());
        Ok(())
    }

    fn exists(&self, path: &str) -> bool {
        self.files.borrow().contains_key(path)
    }

    fn read_dir(&self, dir: &str) -> Result<Vec<String>, engine::EngineError> {
        let prefix = format!("{dir}/");
        let mut names: Vec<String> = self
            .dirs
            .borrow()
            .iter()
            .filter_map(|path| path.strip_prefix(&prefix))
            .filter(|rest| !rest.contains('/'))
            .map(ToOwned::to_owned)
            .collect();
        names.sort();
        Ok(names)
    }
}

/// A minimal document with a visible two-tone vector render. Content
/// is paths, not image tiles: the mkBT tile-bank writer is M7 debt, so
/// raster tiles do not survive the `.fw.png` round trip yet (render
/// fails loudly via `TileBankMiss`).
fn document_bytes(tag: &str) -> Vec<u8> {
    let mut document = doc::Document::default();
    document.info.canvas.width = 64.0;
    document.info.canvas.height = 64.0;
    document.info.name = Some(tag.to_owned());
    let red = path_square(&mut document.registry, 0.0, 0xFF_00_00_FF);
    let white = path_square(&mut document.registry, 32.0, 0xFF_FF_FF_FF);
    document.pages.push(doc::Page {
        info: doc::PageInfo {
            frame_count: 1,
            frame_count_duplicate: 1,
            canvas: document.info.canvas.clone(),
            ..page_info_default()
        },
        name: None,
        is_master: false,
        is_linked: false,
        page_id: None,
        last_export_dir: None,
        last_export_file: None,
        layers: doc::LayerList {
            layers: vec![doc::Layer {
                name: "Layer 1".to_owned(),
                display: true,
                cells: vec![doc::Cell {
                    locked: false,
                    visible: true,
                    elements: vec![red, white],
                }],
                ..doc::Layer::default()
            }],
        },
        grid: None,
        guides: doc::Guides::default(),
        export: None,
    });
    // Full PNG stream (signature + IHDR + mkTS + IEND), not a raw tree.
    app_cli::io::save(&document, &doc::fwmap::UnknownBag::default(), &[])
        .expect("fixture document saves")
}

/// A filled 32x32 square at (x, 0), opaque `fill` (ABGR).
fn path_square(registry: &mut doc::Registry, x: f64, fill: i64) -> doc::ObjectId {
    let corner = |dx: f64, dy: f64| doc::PathPoint {
        anchor: geom::Point::new(x + dx, dy),
        primary: None,
        secondary: None,
        is_curve: false,
    };
    registry.insert(doc::ObjectData::Path(doc::PathObject {
        contours: vec![doc::Contour {
            closed: true,
            point_count: 4,
            points: vec![
                corner(0.0, 0.0),
                corner(32.0, 0.0),
                corner(32.0, 32.0),
                corner(0.0, 32.0),
            ],
        }],
        primitive: None,
        even_odd: false,
        paint: Some(doc::PaintStyle {
            stroke_raw: 0,
            fill_raw: fill,
            fill: None,
        }),
        fill_start: None,
        fill_end: None,
        fill_third: None,
        brush_placement: None,
        fill_on_top: None,
        locked: false,
        visible: true,
        opacity: 1000,
        blend: 0,
        name: None,
        effects: Vec::new(),
    }))
}

/// A zeroed `PageInfo` (no `Default` — its scalars are required).
fn page_info_default() -> doc::PageInfo {
    doc::PageInfo {
        version: None,
        revision: None,
        frame_count: 1,
        frame_count_duplicate: 1,
        request_id: 0,
        canvas: doc::Canvas::default(),
        onion_before: 0,
        onion_after: 0,
        frame_durations: Vec::new(),
    }
}

#[test]
fn batch_exports_every_document_and_preserves_per_file_settings() {
    let fs = MemFs::default();
    fs.add("src/a.fw.png", &document_bytes("a"));
    fs.add("src/b.fw.png", &document_bytes("b"));
    // Non-documents stay untouched.
    fs.add("src/notes.txt", b"hello");
    fs.add("src/sub/c.fw.png", &document_bytes("c"));

    // The overlay sets JPEG quality only; both documents had no
    // recorded settings, so both take it.
    let overlay: doc::ExportOptions =
        serde_json::from_str(r#"{"jpeg_quality": 42}"#).expect("json");
    let written = ops::batch(&fs, "src", "jpg", Some(&overlay), Some("out")).expect("batch");
    assert_eq!(written.len(), 2, "only top-level .fw.png files");
    for entry in &written {
        assert!(entry.output.starts_with("out/"), "{}", entry.output);
        assert!(entry.output.ends_with(".jpg"), "{}", entry.output);
        assert!(fs.contains(&entry.output), "{} not written", entry.output);
        assert!(entry.bytes > 0);
    }

    // The quality took: 42-quality output is smaller than 95-quality.
    let high = doc::ExportOptions {
        jpeg_quality: Some(95),
        ..doc::ExportOptions::default()
    };
    let again = ops::batch(&fs, "src", "jpg", Some(&high), Some("out-high")).expect("batch");
    let low_file = fs.read_file(&written[0].output).expect("low-quality file");
    let high_file = fs.read_file(&again[0].output).expect("high-quality file");
    assert!(
        low_file.len() < high_file.len(),
        "q42 ({} bytes) should be smaller than q95 ({} bytes)",
        low_file.len(),
        high_file.len()
    );
}

#[test]
fn batch_overlay_leaves_recorded_settings_alone() {
    let fs = MemFs::default();
    // A document carrying its own quality 20.
    let bytes = document_bytes("recorded");
    let loaded = app_cli::io::open(&bytes).expect("opens");
    let mut document = loaded.document;
    document.pages[0].export = Some(doc::ExportOptions {
        jpeg_quality: Some(20),
        ..doc::ExportOptions::default()
    });
    let bytes =
        app_cli::io::save(&document, &doc::fwmap::UnknownBag::default(), &[]).expect("writes");
    fs.add("src/recorded.fw.png", &bytes);

    let overlay: doc::ExportOptions =
        serde_json::from_str(r#"{"jpeg_quality": 95}"#).expect("json");
    let written = ops::batch(&fs, "src", "jpg", Some(&overlay), None).expect("batch");
    assert_eq!(written.len(), 1);
    // Output lands beside the source when no out dir is given.
    assert_eq!(written[0].output, "src/recorded.jpg");
}

#[test]
fn batch_refuses_empty_dirs_and_aborts_on_first_failure() {
    let fs = MemFs::default();
    let error = ops::batch(&fs, "empty", "png", None, None).expect_err("no files");
    assert!(error.to_string().contains("no .fw.png"));

    // One broken file aborts the run; files before it stay written.
    fs.add("src/a.fw.png", &document_bytes("a"));
    fs.add("src/broken.fw.png", b"not a png at all");
    fs.add("src/z.fw.png", &document_bytes("z"));
    let error = ops::batch(&fs, "src", "png", None, Some("out"))
        .expect_err("the broken file fails the batch");
    assert!(error.to_string().contains("broken.fw.png"), "{error}");
    assert!(fs.contains("out/a.png"), "files before the failure persist");
    assert!(!fs.contains("out/z.png"), "the run stopped at the failure");
}

#[test]
fn batch_options_accept_a_file_path() {
    let fs = MemFs::default();
    fs.add("src/a.fw.png", &document_bytes("a"));
    std::fs::write("/var/tmp/firecraft-batch-options.json", r#"{"format": 2}"#)
        .expect("scratch write");
    let overlay =
        ops::parse_export_options("/var/tmp/firecraft-batch-options.json").expect("options file");
    assert_eq!(overlay.format, Some(2));
    let written = ops::batch(&fs, "src", "png", Some(&overlay), Some("out")).expect("batch");
    assert_eq!(written.len(), 1);
    let _ = std::fs::remove_file("/var/tmp/firecraft-batch-options.json");
}
