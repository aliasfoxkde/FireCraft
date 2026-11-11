//! The Optimize panel: CS6's export-settings window over the page's
//! `EXPv` record, with a live preview of what the settings produce.
//!
//! The panel offers exactly the controls the export pipeline honors
//! (decision 0095). A setting the codecs cannot honor — PNG's Adam7
//! interlace — is shown as recorded, disabled, and explained, with a
//! one-click way to clear it, rather than offered as a knob that
//! breaks the export.
//!
//! The preview encodes the *current* page through the same resolution
//! the exporter uses (`io::expv`), so what the panel shows is what the
//! file gets — not a re-implementation of it.

use super::*;

/// The `FCLi` matte, ABGR raw: opaque white.
const MATTE_WHITE: i64 = 0xFFFF_FFFF;
/// The `FCLi` matte, ABGR raw: opaque black.
const MATTE_BLACK: i64 = 0xFF00_0000;

/// The Optimize panel's controls, as data.
///
/// The panel's shape and the fields it carries are CS6's: the format
/// select, the JPEG quality and progressive scans, the GIF loop and
/// dither, and the animated-GIF crop and frame-difference knobs. The
/// settings live in the page's `EXPv` record; this struct is the
/// panel's view of that record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OptimizeSettings {
    /// `FMTi` — 0 gif, 1 jpeg, 2 png.
    pub format: i64,
    /// `JPQi` — JPEG quality, 0..=100.
    pub jpeg_quality: i64,
    /// `JPPb` — progressive JPEG scans.
    pub jpeg_progressive: bool,
    /// `FCLi` — the matte, ABGR raw. `None` leaves the choice to the
    /// codec default.
    pub matte_raw: Option<i64>,
    /// `PLDi` — palette dither, 0..=100.
    pub dither: i64,
    /// `FLPi` — GIF frame loop: 0 infinite, -1 never, n repeat n.
    pub frame_loop: i64,
    /// `AACb` — crop animated frames to their ink bounding box.
    pub anim_crop: bool,
    /// `AADb` — animated inter-frame difference.
    pub anim_diff: bool,
    /// `GFIb` — interlaced output. Recorded, shown, and never edited:
    /// this build has no Adam7 pass encoder (decision 0095).
    pub interlaced: bool,
}

impl OptimizeSettings {
    /// The panel's controls read off a page's recorded settings.
    ///
    /// A page that records nothing reads as the codec defaults: PNG,
    /// quality 90, no progressive scans, no dither, an infinite loop.
    #[must_use]
    pub fn read(page: &doc::Page) -> Self {
        let export = page.export.clone().unwrap_or_default();
        Self {
            format: export.format.unwrap_or(2).clamp(0, 2),
            jpeg_quality: export.jpeg_quality.unwrap_or(90).clamp(0, 100),
            jpeg_progressive: export.jpeg_progressive.unwrap_or(false),
            matte_raw: export.matte_raw,
            dither: export.dither.unwrap_or(0).clamp(0, 100),
            frame_loop: export.frame_loop.unwrap_or(0),
            anim_crop: export.anim_crop.unwrap_or(false),
            anim_diff: export.anim_diff.unwrap_or(false),
            interlaced: export.interlaced.unwrap_or(false),
        }
    }

    /// The `page.setExport` payload these controls write.
    ///
    /// The knobs that do not apply to the chosen format are *cleared*
    /// rather than left behind: a page that records a GIF dither and
    /// then exports as JPEG should not keep a dither it cannot honor.
    #[must_use]
    pub fn payload(&self, page: usize) -> serde_json::Value {
        let jpeg = self.format == 1;
        let gif = self.format == 0;
        serde_json::json!({
            "page": page,
            "format": self.format,
            "jpeg_quality": jpeg.then_some(self.jpeg_quality),
            "jpeg_progressive": jpeg.then_some(self.jpeg_progressive),
            "matte_raw": self.matte_raw,
            "dither": gif.then_some(self.dither),
            "frame_loop": gif.then_some(self.frame_loop),
            "anim_crop": gif.then_some(self.anim_crop),
            "anim_diff": gif.then_some(self.anim_diff),
        })
    }

    /// Whether the settings ask for interlaced output.
    #[must_use]
    pub fn interlaced(&self) -> bool {
        self.interlaced
    }
}

impl Shell {
    /// The Optimize panel's format names in `FMTi` order (0 gif,
    /// 1 jpeg, 2 png — the dictionary's documented domain).
    pub(crate) const FMT_NAMES: [&str; 3] = ["GIF", "JPEG", "PNG"];

    /// Window ▸ Optimize: the page's export settings (`EXPv`) as CS6's
    /// panel edits them, with a live preview of the encoded result.
    ///
    /// Every change goes through `page.setExport`, so the model, the
    /// journal and the undo stack stay in step — and the preview reads
    /// the same record the exporter reads.
    pub(super) fn optimize_panel(&mut self, ui: &mut egui::Ui) {
        let page = self.editor.page;
        let Some(recorded) = self.editor.document().pages.get(page) else {
            return;
        };
        let mut settings = OptimizeSettings::read(recorded);
        let mut changed = false;
        egui::Grid::new("optimize").num_columns(2).show(ui, |ui| {
            ui.label("format");
            egui::ComboBox::new("optimize_format", "")
                .selected_text(Shell::FMT_NAMES[settings.format.clamp(0, 2) as usize])
                .show_ui(ui, |ui| {
                    for (id, name) in Shell::FMT_NAMES.iter().enumerate() {
                        ui.selectable_value(&mut settings.format, id as i64, *name);
                    }
                });
            let jpeg = settings.format == 1;
            let gif = settings.format == 0;
            if jpeg {
                ui.label("quality");
                changed |= ui
                    .add(egui::Slider::new(&mut settings.jpeg_quality, 0..=100i64).text("%"))
                    .changed();
                ui.label("progressive");
                changed |= ui.checkbox(&mut settings.jpeg_progressive, "").changed();
                ui.label("matte");
                egui::ComboBox::new("optimize_matte", "")
                    .selected_text(matte_name(settings.matte_raw))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut settings.matte_raw, None, "codec default");
                        ui.selectable_value(&mut settings.matte_raw, Some(MATTE_WHITE), "white");
                        ui.selectable_value(&mut settings.matte_raw, Some(MATTE_BLACK), "black");
                    });
            }
            if gif {
                ui.label("loop");
                egui::ComboBox::new("optimize_loop", "")
                    .selected_text(match settings.frame_loop {
                        0 => "infinite".to_owned(),
                        -1 => "never".to_owned(),
                        n => format!("×{n}"),
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut settings.frame_loop, 0, "infinite");
                        ui.selectable_value(&mut settings.frame_loop, -1, "never");
                        ui.selectable_value(&mut settings.frame_loop, 3, "×3");
                    });
                ui.label("dither");
                changed |= ui
                    .add(egui::Slider::new(&mut settings.dither, 0..=100i64).text("%"))
                    .changed();
                ui.label("crop frames");
                changed |= ui.checkbox(&mut settings.anim_crop, "").changed();
                ui.label("frame difference");
                changed |= ui.checkbox(&mut settings.anim_diff, "").changed();
            }
            // The interlace row: CS6 offers it, this build cannot write
            // it. A recorded flag blocks the PNG export, so the row
            // says so and offers the way out.
            ui.label("interlaced");
            ui.add_enabled(
                false,
                egui::Checkbox::new(&mut settings.interlaced, "not supported"),
            )
            .on_hover_text(
                "PNG and GIF interlacing need an Adam7 pass encoder, which this build does not \
                 have. A recorded flag blocks the PNG export.",
            );
        });
        if settings.interlaced {
            ui.label("This page records interlaced output. Clear it to export a PNG.");
            if ui.button("Clear interlaced flag").clicked()
                && self.run_command(
                    "page.setExport",
                    serde_json::json!({ "page": page, "interlaced": false }),
                )
            {
                self.status = "interlaced flag cleared.".to_owned();
            }
        }
        if changed {
            let payload = settings.payload(page);
            if self.run_command("page.setExport", payload) {
                self.status = "Optimize settings applied to the page.".to_owned();
            }
        }

        // The live preview: the page encoded through the same settings
        // the export path reads.
        ui.separator();
        match self.optimize_preview(ui.ctx(), &settings) {
            Ok((texture, label)) => {
                ui.label(format!("preview (page {}, {label})", page + 1));
                ui.add(egui::Image::new((texture.id(), Vec2::splat(128.0))));
            }
            Err(error) => {
                ui.label(format!("preview failed: {error}"));
            }
        }
    }

    /// The Optimize preview: the encoded page as a texture, plus the
    /// byte size the export would produce.
    ///
    /// The preview is cached against a stamp of the settings, the
    /// current page and frame, and the document's journal state, so it
    /// regenerates exactly when the encoded bytes could have moved.
    fn optimize_preview(
        &mut self,
        ctx: &egui::Context,
        settings: &OptimizeSettings,
    ) -> Result<(egui::TextureHandle, String), String> {
        let stamp = self.optimize_preview_stamp(settings);
        if let Some((made, texture, label)) = &self.optimize_preview
            && *made == stamp
        {
            return Ok((texture.clone(), label.clone()));
        }
        let bytes = self.encode_for_preview(settings)?;
        let decoded = io::raster::decode(&bytes).map_err(|error| error.to_string())?;
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [decoded.width as usize, decoded.height as usize],
            &decoded.rgba,
        );
        let texture = ctx.load_texture("optimize-preview", image, egui::TextureOptions::LINEAR);
        let label = format!("{} bytes", bytes.len());
        self.optimize_preview = Some((stamp, texture.clone(), label.clone()));
        Ok((texture, label))
    }

    /// The stamp the preview is cached against.
    fn optimize_preview_stamp(&self, settings: &OptimizeSettings) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (settings, self.editor.page, self.editor.frame, self.stamp()).hash(&mut hasher);
        hasher.finish()
    }

    /// Encodes the current page the way the export path would, with
    /// the panel's settings in force.
    ///
    /// The GIF path is the animated exporter (every state of the
    /// page); the JPEG and PNG paths are the single state's render.
    fn encode_for_preview(&self, settings: &OptimizeSettings) -> Result<Vec<u8>, String> {
        let document = self.editor.document();
        let page = self.editor.page;
        let export = io::expv::for_page(document, page)
            .cloned()
            .unwrap_or_default();
        if settings.format == 0 {
            let options = io::expv::gif_options(Some(&export));
            return io::gif::encode_animated(document, page, &options)
                .map_err(|error| error.to_string());
        }
        let render = compose::render(document, page, self.editor.frame)
            .map_err(|error| error.to_string())?;
        let straight = compose::straight_alpha(&render.pixels);
        let plane = io::Plane::new(u32::from(render.width), u32::from(render.height), &straight)
            .map_err(|error| error.to_string())?;
        if settings.format == 1 {
            let options = io::expv::jpeg_options(Some(&export), io::expv::StatedJpeg::default());
            return io::jpeg::encode(&plane, &options).map_err(|error| error.to_string());
        }
        let options = io::expv::png_options(Some(&export));
        io::png::encode_with(&plane, &options).map_err(|error| error.to_string())
    }
}

/// The matte's name, as the panel shows it.
fn matte_name(raw: Option<i64>) -> &'static str {
    match raw {
        None => "codec default",
        Some(MATTE_WHITE) => "white",
        Some(MATTE_BLACK) => "black",
        Some(_) => "custom",
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    /// A page with the given recorded export settings.
    fn page_with(export: Option<doc::ExportOptions>) -> doc::Page {
        let mut document = doc::Document::default();
        document.pages.push(doc::Page {
            info: doc::PageInfo {
                version: None,
                revision: None,
                frame_count: 1,
                frame_count_duplicate: 1,
                request_id: 0,
                canvas: doc::Canvas::default(),
                onion_before: 0,
                onion_after: 0,
                frame_durations: Vec::new(),
            },
            name: None,
            is_master: false,
            is_linked: false,
            page_id: None,
            last_export_dir: None,
            last_export_file: None,
            layers: doc::LayerList::default(),
            grid: None,
            guides: doc::Guides::default(),
            export,
        });
        document.pages.remove(0)
    }

    #[test]
    fn an_unrecorded_page_reads_as_the_codec_defaults() {
        let settings = OptimizeSettings::read(&page_with(None));
        assert_eq!(settings.format, 2);
        assert_eq!(settings.jpeg_quality, 90);
        assert!(!settings.jpeg_progressive);
        assert_eq!(settings.matte_raw, None);
        assert_eq!(settings.dither, 0);
        assert_eq!(settings.frame_loop, 0);
        assert!(!settings.anim_crop);
        assert!(!settings.anim_diff);
        assert!(!settings.interlaced);
    }

    #[test]
    fn a_recorded_page_reads_back_its_own_settings() {
        let page = page_with(Some(doc::ExportOptions {
            format: Some(1),
            jpeg_quality: Some(42),
            jpeg_progressive: Some(true),
            matte_raw: Some(MATTE_BLACK),
            dither: Some(30),
            frame_loop: Some(-1),
            anim_crop: Some(true),
            anim_diff: Some(true),
            interlaced: Some(true),
            ..doc::ExportOptions::default()
        }));
        let settings = OptimizeSettings::read(&page);
        assert_eq!(settings.format, 1);
        assert_eq!(settings.jpeg_quality, 42);
        assert!(settings.jpeg_progressive);
        assert_eq!(settings.matte_raw, Some(MATTE_BLACK));
        assert_eq!(settings.dither, 30);
        assert_eq!(settings.frame_loop, -1);
        assert!(settings.anim_crop);
        assert!(settings.anim_diff);
        assert!(settings.interlaced());
    }

    #[test]
    fn the_payload_carries_the_settings_the_format_uses() {
        let settings = OptimizeSettings {
            format: 1,
            jpeg_quality: 55,
            jpeg_progressive: true,
            matte_raw: Some(MATTE_WHITE),
            dither: 20,
            frame_loop: 3,
            anim_crop: true,
            anim_diff: false,
            interlaced: false,
        };
        let payload = settings.payload(2);
        assert_eq!(payload["page"], 2);
        assert_eq!(payload["format"], 1);
        assert_eq!(payload["jpeg_quality"], 55);
        assert_eq!(payload["jpeg_progressive"], true);
        assert_eq!(payload["matte_raw"], MATTE_WHITE);
        // The GIF knobs are cleared while the format is JPEG.
        assert!(payload["dither"].is_null());
        assert!(payload["frame_loop"].is_null());
        assert!(payload["anim_crop"].is_null());
    }

    #[test]
    fn the_payload_carries_the_gif_knobs_for_a_gif() {
        let settings = OptimizeSettings {
            format: 0,
            jpeg_quality: 55,
            jpeg_progressive: true,
            matte_raw: None,
            dither: 20,
            frame_loop: 3,
            anim_crop: true,
            anim_diff: true,
            interlaced: false,
        };
        let payload = settings.payload(0);
        assert_eq!(payload["dither"], 20);
        assert_eq!(payload["frame_loop"], 3);
        assert_eq!(payload["anim_crop"], true);
        assert_eq!(payload["anim_diff"], true);
        // The JPEG knobs are cleared while the format is GIF.
        assert!(payload["jpeg_quality"].is_null());
        assert!(payload["jpeg_progressive"].is_null());
    }

    #[test]
    fn the_payload_never_writes_the_interlace_flag() {
        // The flag is cleared through its own command, never as a
        // side effect of another edit.
        let settings = OptimizeSettings {
            format: 2,
            jpeg_quality: 90,
            jpeg_progressive: false,
            matte_raw: None,
            dither: 0,
            frame_loop: 0,
            anim_crop: false,
            anim_diff: false,
            interlaced: true,
        };
        let payload = settings.payload(0);
        assert!(payload.get("interlaced").is_none());
    }

    #[test]
    fn the_matte_names_read_back() {
        assert_eq!(matte_name(None), "codec default");
        assert_eq!(matte_name(Some(MATTE_WHITE)), "white");
        assert_eq!(matte_name(Some(MATTE_BLACK)), "black");
        assert_eq!(matte_name(Some(0x0012_3456)), "custom");
    }
}
