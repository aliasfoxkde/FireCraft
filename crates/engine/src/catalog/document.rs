//! Document commands: canvas color, resolution.
//!
//! CS6's `dom.setDocumentCanvas*`/`setDocumentResolution` verbs are
//! document-scoped: one setting in the UI, carried on the wire by both
//! the `MKBv` root canvas and every page's `PDCv` canvas copy. The
//! commands write both so the round-trip stays consistent (decision
//! 0023).

use doc::Document;

use crate::command::TypedCommand;
use crate::delta::Delta;
use crate::error::EngineError;
use crate::registry::CommandRegistry;

/// Registers the document commands.
pub(crate) fn install(registry: &mut CommandRegistry) -> Result<(), EngineError> {
    registry.register(SetDocumentCanvasColor)?;
    registry.register(SetDocumentResolution)
}

/// `document.setCanvasColor` — CS6's
/// `dom.setDocumentCanvasColor(color)`: sets the canvas matte to one
/// color across the document (`BGCi` on the `MKBv` root canvas and
/// every page's copy). The raw is ABGR like every paint slot
/// (`compose` paints `R = raw & 0xFF`); the scripting layer's color
/// string maps to this raw before the command runs.
struct SetDocumentCanvasColor;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetDocumentCanvasColorPayload {
    /// Canvas color, ABGR layout (0x00RRGGBB when fully opaque).
    color_raw: i64,
}

impl TypedCommand for SetDocumentCanvasColor {
    type Payload = SetDocumentCanvasColorPayload;

    fn name(&self) -> &'static str {
        "document.setCanvasColor"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "document.setCanvasColor";
        if !(0..=0xFF_FF_FF_FF).contains(&payload.color_raw) {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!("color_raw {} outside 0..=0xFFFFFFFF", payload.color_raw),
            });
        }
        // Capture every `before` before the first mutation (undo law).
        let mut deltas = vec![Delta::Info {
            before: Box::new(document.info.clone()),
        }];
        let mut pages = Vec::with_capacity(document.pages.len());
        for page in &document.pages {
            pages.push(page.info.clone());
        }
        document.info.canvas.background_raw = payload.color_raw;
        for (index, page) in document.pages.iter_mut().enumerate() {
            page.info.canvas.background_raw = payload.color_raw;
            deltas.push(Delta::PageInfo {
                page: index,
                before: Box::new(pages[index].clone()),
            });
        }
        Ok(Delta::Group(deltas))
    }
}

/// Resolution unit (CS6's `units` string; `RSUi` on the wire).
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ResolutionUnit {
    /// `RSUi` 0 — pixels per inch (the default).
    Inch,
    /// `RSUi` 1 — pixels per centimeter.
    Cm,
}

impl ResolutionUnit {
    fn raw(self) -> i64 {
        match self {
            ResolutionUnit::Inch => 0,
            ResolutionUnit::Cm => 1,
        }
    }
}

/// `document.setResolution` — CS6's
/// `dom.setDocumentResolution({pixelsPerUnit, units})`: sets the
/// resolution and its unit across the document (`RESf`/`RSUi` on the
/// `MKBv` root canvas and every page's copy).
struct SetDocumentResolution;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct SetDocumentResolutionPayload {
    /// Pixels per unit; the UI's document resolution.
    resolution: f64,
    /// Unit the resolution is expressed in.
    unit: ResolutionUnit,
}

impl TypedCommand for SetDocumentResolution {
    type Payload = SetDocumentResolutionPayload;

    fn name(&self) -> &'static str {
        "document.setResolution"
    }

    fn apply(
        &self,
        document: &mut Document,
        payload: &Self::Payload,
    ) -> Result<Delta, EngineError> {
        const NAME: &str = "document.setResolution";
        if !payload.resolution.is_finite() || payload.resolution <= 0.0 {
            return Err(EngineError::InvalidState {
                name: NAME.to_owned(),
                detail: format!(
                    "resolution {} must be finite and positive",
                    payload.resolution
                ),
            });
        }
        // Capture every `before` before the first mutation (undo law).
        let mut deltas = vec![Delta::Info {
            before: Box::new(document.info.clone()),
        }];
        let mut pages = Vec::with_capacity(document.pages.len());
        for page in &document.pages {
            pages.push(page.info.clone());
        }
        document.info.canvas.resolution = payload.resolution;
        document.info.canvas.resolution_unit = payload.unit.raw();
        for (index, page) in document.pages.iter_mut().enumerate() {
            page.info.canvas.resolution = payload.resolution;
            page.info.canvas.resolution_unit = payload.unit.raw();
            deltas.push(Delta::PageInfo {
                page: index,
                before: Box::new(pages[index].clone()),
            });
        }
        Ok(Delta::Group(deltas))
    }
}

#[cfg(test)]
mod document_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use crate::catalog::testdoc;
    use crate::command::Command;

    #[test]
    fn canvas_color_is_document_wide_and_undoes() {
        let (mut document, _) = testdoc::sample_document();
        document.info.canvas.background_raw = 0x00_FF_FF;
        document.pages[0].info.canvas.background_raw = 0x11_22_33;
        document.pages[1].info.canvas.background_raw = 0x44_55_66;

        let delta = SetDocumentCanvasColor
            .apply_json(&mut document, serde_json::json!({"color_raw": 0x00_00_FF}))
            .expect("setCanvasColor applies");
        assert_eq!(document.info.canvas.background_raw, 0x00_00_FF);
        for page in &document.pages {
            assert_eq!(page.info.canvas.background_raw, 0x00_00_FF);
        }

        delta.apply_inverse(&mut document).expect("inverse applies");
        assert_eq!(document.info.canvas.background_raw, 0x00_FF_FF);
        assert_eq!(document.pages[0].info.canvas.background_raw, 0x11_22_33);
        assert_eq!(document.pages[1].info.canvas.background_raw, 0x44_55_66);
    }

    #[test]
    fn canvas_color_rejects_out_of_range_raw() {
        let (mut document, _) = testdoc::sample_document();
        let err = SetDocumentCanvasColor
            .apply_json(&mut document, serde_json::json!({"color_raw": -1}))
            .expect_err("negative raw is refused");
        assert!(matches!(
            err,
            crate::error::EngineError::InvalidState { .. }
        ));
    }

    #[test]
    fn resolution_sets_unit_and_undoes() {
        let (mut document, _) = testdoc::sample_document();
        assert_eq!(document.info.canvas.resolution, 72.0);
        assert_eq!(document.info.canvas.resolution_unit, 0);

        SetDocumentResolution
            .apply_json(
                &mut document,
                serde_json::json!({"resolution": 96.0, "unit": "inch"}),
            )
            .expect("setResolution applies");
        assert_eq!(document.info.canvas.resolution, 96.0);
        assert_eq!(document.info.canvas.resolution_unit, 0);
        assert_eq!(document.pages[1].info.canvas.resolution, 96.0);

        let delta = SetDocumentResolution
            .apply_json(
                &mut document,
                serde_json::json!({"resolution": 28.35, "unit": "cm"}),
            )
            .expect("cm applies");
        assert_eq!(document.info.canvas.resolution_unit, 1);

        delta.apply_inverse(&mut document).expect("inverse applies");
        assert_eq!(document.info.canvas.resolution, 96.0);
        assert_eq!(document.info.canvas.resolution_unit, 0);
    }

    #[test]
    fn resolution_rejects_non_positive() {
        let (mut document, _) = testdoc::sample_document();
        for bad in [0.0, -72.0] {
            let err = SetDocumentResolution
                .apply_json(
                    &mut document,
                    serde_json::json!({"resolution": bad, "unit": "inch"}),
                )
                .expect_err("non-positive resolution is refused");
            assert!(matches!(
                err,
                crate::error::EngineError::InvalidState { .. }
            ));
        }
        // NaN/inf never reach the command: they serialize to JSON null
        // and the payload layer refuses first (same outcome).
        let err = SetDocumentResolution
            .apply_json(
                &mut document,
                serde_json::json!({"resolution": f64::NAN, "unit": "inch"}),
            )
            .expect_err("null resolution is refused");
        assert!(matches!(err, crate::error::EngineError::BadPayload { .. }));
    }

    #[test]
    fn page_info_is_captured_per_page_not_shared() {
        // Each page's inverse must restore its own pre-command info:
        // a shared `before` would stamp page 0's state onto page 1.
        let (mut document, _) = testdoc::sample_document();
        document.pages[0].info.onion_before = 3;
        document.pages[1].info.onion_after = 4;
        let delta = SetDocumentCanvasColor
            .apply_json(&mut document, serde_json::json!({"color_raw": 1}))
            .expect("applies");
        delta.apply_inverse(&mut document).expect("inverse applies");
        assert_eq!(document.pages[0].info.onion_before, 3);
        assert_eq!(document.pages[1].info.onion_after, 4);
    }
}
