//! The slice/hotspot model behind the CS6 web panels: what a web
//! object is, the export formats the web export can produce, and the
//! behavior catalog the Behaviors panel offers.
//!
//! CS6's Behaviors panel is a two-axis editor — an *event* (On Click,
//! On Mouse Over, …) and an *action* (Swap Image, Roll Over, Link,
//! …). The engine's model is narrower: a behavior is a bare `BETi`
//! kind number plus, for the swap family, the name of the slice it
//! swaps to (`DTAv` `swapTarget`). This module holds the mapping from
//! CS6's pairs to the verbs that persist them, so the panel offers
//! exactly what the engine can round-trip and names the rest as
//! unsupported instead of pretending to save them.
//!
//! The engine's swap-image hook restores the image on mouse-out by
//! itself, so CS6's "Swap Image" and "Roll Over" actions differ in
//! CS6 only — on the wire both are the swap-image kind.

use doc::SliceObject;

/// The `BETi` kind of the popup-menu behavior (the only kind the
/// Fireworks node dictionary documents).
pub(crate) const POPUP_MENU_BEHAVIOR: i64 = 9;

/// The `BETi` kind the web export reads as "swap image".
pub(crate) const SWAP_IMAGE_BEHAVIOR: i64 = 2;

/// The `BETi` kind of the swap-image restore.
pub(crate) const SWAP_RESTORE_BEHAVIOR: i64 = 5;

/// The `BETi` kind of the nav-bar highlight (the export's
/// `MM_nbGroup('over', …)`).
pub(crate) const NAV_HIGHLIGHT_BEHAVIOR: i64 = 6;

/// The `BETi` kind of the nav-bar down state.
pub(crate) const NAV_DOWN_BEHAVIOR: i64 = 4;

/// The `BETi` kind of the nav-bar restore.
pub(crate) const NAV_RESTORE_BEHAVIOR: i64 = 7;

/// The `DTAv` key a swap-family behavior's target rides on.
pub(crate) const SWAP_TARGET_KEY: &str = "swapTarget";

/// What a web object is. CS6's three slice kinds: a foreground
/// slice, a background slice (the CSS-background family), and a
/// hotspot (a link-only map area).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SliceKind {
    /// A slice that rasterizes its region.
    Foreground,
    /// A slice exported as a CSS background.
    Background,
    /// A map-only area — no image, an `<area>`.
    Hotspot,
}

impl SliceKind {
    /// The CS6 label.
    pub(crate) fn label(self) -> &'static str {
        match self {
            SliceKind::Foreground => "Foreground slice",
            SliceKind::Background => "Background slice",
            SliceKind::Hotspot => "Hotspot",
        }
    }

    /// The `TSLi` value the engine stores (`slice.setType`); `None`
    /// is the hotspot's map-only shape.
    pub(crate) fn type_value(self) -> Option<i64> {
        match self {
            SliceKind::Foreground => Some(0),
            SliceKind::Background => Some(2),
            SliceKind::Hotspot => None,
        }
    }

    /// The kind a stored web object is: `TSLi` presence is the
    /// slice/hotspot distinction (decision 0017).
    pub(crate) fn of(slice: &SliceObject) -> Self {
        match slice.slice_type {
            Some(2) => SliceKind::Background,
            Some(_) => SliceKind::Foreground,
            None => SliceKind::Hotspot,
        }
    }

    /// The kinds the panel offers, in CS6's order.
    pub(crate) const ALL: [SliceKind; 3] = [
        SliceKind::Foreground,
        SliceKind::Background,
        SliceKind::Hotspot,
    ];
}

/// The hotspot shape (`URSi`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SliceShape {
    /// `URSi` 0.
    Rectangle,
    /// `URSi` 1.
    Ellipse,
    /// `URSi` 2.
    Polygon,
}

impl SliceShape {
    /// The CS6 label.
    pub(crate) fn label(self) -> &'static str {
        match self {
            SliceShape::Rectangle => "Rectangle",
            SliceShape::Ellipse => "Ellipse",
            SliceShape::Polygon => "Polygon",
        }
    }

    /// The shape a stored web object carries.
    pub(crate) fn of(slice: &SliceObject) -> Self {
        match slice.shape {
            1 => SliceShape::Ellipse,
            2 => SliceShape::Polygon,
            _ => SliceShape::Rectangle,
        }
    }

    /// The shape name `hotspot.setShape` takes.
    pub(crate) fn command_name(self) -> &'static str {
        match self {
            SliceShape::Rectangle => "rectangle",
            SliceShape::Ellipse => "oval",
            SliceShape::Polygon => "polyline",
        }
    }

    /// The shapes the panel offers, in CS6's order.
    pub(crate) const ALL: [SliceShape; 3] = [
        SliceShape::Rectangle,
        SliceShape::Ellipse,
        SliceShape::Polygon,
    ];
}

/// The export formats the web export can actually produce. CS6's
/// export dialog also offers SVG and animated GIF; the web export
/// writes neither, so the panel does not offer them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExportFormat {
    /// `FMTi` 2 — the default: the exported pixels are the
    /// composer's.
    Png,
    /// `FMTi` 1 — flattened over the slice's matte.
    Jpeg,
    /// `FMTi` 0 — a single-frame GIF.
    Gif,
}

impl ExportFormat {
    /// The `FMTi` value.
    pub(crate) fn code(self) -> i64 {
        match self {
            ExportFormat::Png => 2,
            ExportFormat::Jpeg => 1,
            ExportFormat::Gif => 0,
        }
    }

    /// The CS6 label.
    pub(crate) fn label(self) -> &'static str {
        match self {
            ExportFormat::Png => "PNG",
            ExportFormat::Jpeg => "JPEG",
            ExportFormat::Gif => "GIF",
        }
    }

    /// The format a web object exports with: its own `EXPv` override,
    /// else PNG.
    pub(crate) fn of(slice: &SliceObject) -> Self {
        match slice.export.as_ref().and_then(|export| export.format) {
            Some(0) => ExportFormat::Gif,
            Some(1) => ExportFormat::Jpeg,
            _ => ExportFormat::Png,
        }
    }

    /// Whether the web object carries its own format override (as
    /// opposed to following the page's export settings).
    pub(crate) fn is_override(slice: &SliceObject) -> bool {
        slice
            .export
            .as_ref()
            .is_some_and(|export| export.format.is_some())
    }

    /// The formats the panel offers, in CS6's order.
    pub(crate) const ALL: [ExportFormat; 3] =
        [ExportFormat::Png, ExportFormat::Jpeg, ExportFormat::Gif];
}

/// CS6's Behaviors panel event axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BehaviorEvent {
    /// On Click.
    Click,
    /// On Mouse Over.
    MouseOver,
    /// On Mouse Out.
    MouseOut,
    /// On Roll Over.
    RollOver,
    /// On Roll Out.
    RollOut,
    /// On Mouse Down.
    MouseDown,
    /// On Mouse Up.
    MouseUp,
    /// On Focus.
    Focus,
    /// On Blur.
    Blur,
    /// On Key Press.
    KeyPress,
    /// On Key Release.
    KeyRelease,
}

impl BehaviorEvent {
    /// The CS6 label.
    pub(crate) fn label(self) -> &'static str {
        match self {
            BehaviorEvent::Click => "On Click",
            BehaviorEvent::MouseOver => "On Mouse Over",
            BehaviorEvent::MouseOut => "On Mouse Out",
            BehaviorEvent::RollOver => "On Roll Over",
            BehaviorEvent::RollOut => "On Roll Out",
            BehaviorEvent::MouseDown => "On Mouse Down",
            BehaviorEvent::MouseUp => "On Mouse Up",
            BehaviorEvent::Focus => "On Focus",
            BehaviorEvent::Blur => "On Blur",
            BehaviorEvent::KeyPress => "On Key Press",
            BehaviorEvent::KeyRelease => "On Key Release",
        }
    }

    /// The events the panel offers, in CS6's order.
    pub(crate) const ALL: [BehaviorEvent; 11] = [
        BehaviorEvent::Click,
        BehaviorEvent::MouseOver,
        BehaviorEvent::MouseOut,
        BehaviorEvent::RollOver,
        BehaviorEvent::RollOut,
        BehaviorEvent::MouseDown,
        BehaviorEvent::MouseUp,
        BehaviorEvent::Focus,
        BehaviorEvent::Blur,
        BehaviorEvent::KeyPress,
        BehaviorEvent::KeyRelease,
    ];
}

/// CS6's Behaviors panel action axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BehaviorAction {
    /// Swap Image.
    SwapImage,
    /// Swap Image Restore.
    SwapImageRestore,
    /// Roll Over.
    RollOver,
    /// Link.
    Link,
    /// Set Pop-up.
    SetPopUp,
    /// Named Anchor.
    NamedAnchor,
    /// Go To Web Page.
    GoToWebPage,
    /// Set Image.
    SetImage,
    /// Status message.
    StatusMessage,
    /// Sound.
    Sound,
}

impl BehaviorAction {
    /// The CS6 label.
    pub(crate) fn label(self) -> &'static str {
        match self {
            BehaviorAction::SwapImage => "Swap Image",
            BehaviorAction::SwapImageRestore => "Swap Image Restore",
            BehaviorAction::RollOver => "Roll Over",
            BehaviorAction::Link => "Link",
            BehaviorAction::SetPopUp => "Set Pop-up",
            BehaviorAction::NamedAnchor => "Named Anchor",
            BehaviorAction::GoToWebPage => "Go To Web Page",
            BehaviorAction::SetImage => "Set Image",
            BehaviorAction::StatusMessage => "Status message",
            BehaviorAction::Sound => "Sound",
        }
    }

    /// The actions the panel offers, in CS6's order.
    pub(crate) const ALL: [BehaviorAction; 10] = [
        BehaviorAction::SwapImage,
        BehaviorAction::SwapImageRestore,
        BehaviorAction::RollOver,
        BehaviorAction::Link,
        BehaviorAction::SetPopUp,
        BehaviorAction::NamedAnchor,
        BehaviorAction::GoToWebPage,
        BehaviorAction::SetImage,
        BehaviorAction::StatusMessage,
        BehaviorAction::Sound,
    ];
}

/// How one CS6 event/action pair persists in the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BehaviorPlan {
    /// A `BETi` kind appended through `slice.addBehavior`. `target`
    /// says the behavior names another slice, which rides the `DTAv`
    /// `swapTarget` key.
    Kind {
        /// The `BETi` kind.
        kind: i64,
        /// Whether the behavior needs a swap target.
        target: bool,
    },
    /// The object's anchor link (`slice.setLink`).
    Link,
    /// CS6 offers the pair; the engine cannot persist it. The text
    /// says why, so the panel can say it instead of dropping it.
    Unsupported(&'static str),
}

impl BehaviorPlan {
    /// Whether the pair can be applied.
    pub(crate) fn is_supported(self) -> bool {
        !matches!(self, BehaviorPlan::Unsupported(_))
    }
}

/// The engine's mapping of one CS6 event/action pair.
///
/// The swap family needs the target slice's name, so those plans ask
/// for a target; the link family writes the object's anchor URL.
pub(crate) fn plan(event: BehaviorEvent, action: BehaviorAction) -> BehaviorPlan {
    use BehaviorAction::{
        GoToWebPage, Link, NamedAnchor, RollOver, SetImage, SetPopUp, Sound, StatusMessage,
        SwapImage, SwapImageRestore,
    };
    use BehaviorEvent::{
        Blur, Click, Focus, KeyPress, KeyRelease, MouseDown, MouseOut, MouseOver, MouseUp, RollOut,
        RollOver as EventRollOver,
    };
    match (event, action) {
        // The swap family: the target rides `swapTarget`, and the
        // exporter restores the image on mouse-out by itself.
        (MouseOver, SwapImage | SetImage) => BehaviorPlan::Kind {
            kind: SWAP_IMAGE_BEHAVIOR,
            target: true,
        },
        (MouseOver, RollOver) => BehaviorPlan::Kind {
            kind: SWAP_IMAGE_BEHAVIOR,
            target: true,
        },
        (MouseOut | MouseUp, SwapImageRestore) => BehaviorPlan::Kind {
            kind: SWAP_RESTORE_BEHAVIOR,
            target: false,
        },
        // The nav-bar family: highlight, down, restore.
        (EventRollOver, RollOver) => BehaviorPlan::Kind {
            kind: NAV_HIGHLIGHT_BEHAVIOR,
            target: true,
        },
        (MouseDown, RollOver) => BehaviorPlan::Kind {
            kind: NAV_DOWN_BEHAVIOR,
            target: true,
        },
        (RollOut, RollOver) => BehaviorPlan::Kind {
            kind: NAV_RESTORE_BEHAVIOR,
            target: false,
        },
        // The pop-up menu: the engine records the kind; the exporter
        // writes no hook for it (a documented gap).
        (Click, SetPopUp) => BehaviorPlan::Kind {
            kind: POPUP_MENU_BEHAVIOR,
            target: false,
        },
        // The anchor link: one URL per web object.
        (Click, Link | GoToWebPage | NamedAnchor) => BehaviorPlan::Link,
        // What the engine has no room for.
        (_, StatusMessage) => {
            BehaviorPlan::Unsupported("the engine records no status-message behavior")
        }
        (_, Sound) => BehaviorPlan::Unsupported("the engine records no sound behavior"),
        (_, Link | GoToWebPage | NamedAnchor) => {
            BehaviorPlan::Unsupported("the engine stores one anchor link per object, on click")
        }
        (KeyPress | KeyRelease, _) => {
            BehaviorPlan::Unsupported("the engine records no key-press behavior")
        }
        (Focus | Blur, _) => BehaviorPlan::Unsupported("the engine records no focus behavior"),
        // The swap family swaps on mouse over and restores on mouse
        // out; CS6's other events for it have no engine counterpart.
        (_, SwapImage | SetImage | SwapImageRestore | RollOver) => BehaviorPlan::Unsupported(
            "the engine's swap behaviors fire on mouse over and mouse out",
        ),
        _ => BehaviorPlan::Unsupported("the engine has no behavior for this event and action"),
    }
}

/// The label of one stored behavior kind. The kinds the web export
/// reads are named; anything else shows its raw number rather than a
/// guess.
pub(crate) fn behavior_label(kind: i64) -> String {
    match kind {
        SWAP_IMAGE_BEHAVIOR => "Swap Image".to_owned(),
        SWAP_RESTORE_BEHAVIOR => "Swap Image Restore".to_owned(),
        NAV_HIGHLIGHT_BEHAVIOR => "Roll Over (nav bar)".to_owned(),
        NAV_DOWN_BEHAVIOR => "Button Down (nav bar)".to_owned(),
        NAV_RESTORE_BEHAVIOR => "Roll Over Restore (nav bar)".to_owned(),
        POPUP_MENU_BEHAVIOR => "Set Pop-up".to_owned(),
        other => format!("Behavior {other}"),
    }
}

/// A web object's display name: its object name, then its internal
/// name.
pub(crate) fn slice_name(slice: &SliceObject) -> String {
    slice
        .name
        .clone()
        .or_else(|| slice.internal_name.clone())
        .unwrap_or_else(|| "(unnamed slice)".to_owned())
}

/// The name of the slice a swap-family behavior points at, when the
/// web object carries one.
pub(crate) fn swap_target_of(slice: &SliceObject) -> Option<String> {
    slice
        .custom_data
        .get(SWAP_TARGET_KEY)
        .filter(|target| !target.is_empty())
        .cloned()
}

/// Whether a behavior kind names another slice through the
/// `swapTarget` key — the swap and nav-bar families do, the restore
/// and pop-up kinds do not.
pub(crate) fn is_targeted(kind: i64) -> bool {
    matches!(
        kind,
        SWAP_IMAGE_BEHAVIOR | NAV_HIGHLIGHT_BEHAVIOR | NAV_DOWN_BEHAVIOR
    )
}

/// One row of the Slices panel: a web object with the facts CS6's
/// panel shows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SliceRow {
    /// The web object.
    pub(crate) id: doc::ObjectId,
    /// Its display name.
    pub(crate) name: String,
    /// Foreground slice, background slice, or hotspot.
    pub(crate) kind: SliceKind,
    /// Its shape.
    pub(crate) shape: SliceShape,
    /// The format it exports with.
    pub(crate) format: ExportFormat,
    /// Whether the format is the object's own override.
    pub(crate) overridden: bool,
    /// The JPEG quality of its export override.
    pub(crate) jpeg_quality: Option<i64>,
    /// Its anchor link.
    pub(crate) link: Option<String>,
    /// Its alt text.
    pub(crate) alt: Option<String>,
    /// Its behavior kinds, in order.
    pub(crate) behaviors: Vec<i64>,
    /// The slice its swap-family behaviors name.
    pub(crate) swap_target: Option<String>,
    /// The frame's width, in document units.
    pub(crate) width: f64,
    /// The frame's height, in document units.
    pub(crate) height: f64,
}

impl SliceRow {
    /// The row's detail line: the shape, the export format, and the
    /// behaviors it carries — CS6's secondary column.
    pub(crate) fn detail(&self) -> String {
        let format = if self.overridden {
            self.format.label().to_owned()
        } else {
            format!("{} (page default)", self.format.label())
        };
        format!(
            "{} · {format} · {}",
            self.shape.label(),
            self.behavior_count()
        )
    }

    /// How many behaviors the web object carries, in CS6's wording.
    pub(crate) fn behavior_count(&self) -> String {
        match self.behaviors.len() {
            0 => "no behaviors".to_owned(),
            1 => "1 behavior".to_owned(),
            count => format!("{count} behaviors"),
        }
    }

    /// The frame's size, in CS6's wording.
    pub(crate) fn dimensions(&self) -> String {
        format!("{} × {}", self.width.round(), self.height.round())
    }
}

/// Every web object in the document, in registry order.
pub(crate) fn slice_rows(document: &doc::Document) -> Vec<SliceRow> {
    document
        .registry
        .iter()
        .filter_map(|(id, data)| match data {
            doc::ObjectData::Slice(slice) => Some(SliceRow {
                id,
                name: slice_name(slice),
                kind: SliceKind::of(slice),
                shape: SliceShape::of(slice),
                format: ExportFormat::of(slice),
                overridden: ExportFormat::is_override(slice),
                jpeg_quality: slice.export.as_ref().and_then(|export| export.jpeg_quality),
                link: slice.link.clone(),
                alt: slice.alt.clone(),
                behaviors: slice
                    .behaviors
                    .iter()
                    .map(|behavior| behavior.kind)
                    .collect(),
                swap_target: swap_target_of(slice),
                width: slice.frame.x1 - slice.frame.x0,
                height: slice.frame.y1 - slice.frame.y0,
            }),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// A web object with the given `TSLi` — the slice/hotspot
    /// distinction is `TSLi` presence (decision 0017).
    fn web_object(slice_type: Option<i64>) -> SliceObject {
        SliceObject {
            slice_type,
            ..SliceObject::default()
        }
    }

    /// The kinds map to the engine's `TSLi` values, and a hotspot is
    /// the map-only shape.
    #[test]
    fn the_kinds_map_onto_the_engines_type_values() {
        assert_eq!(SliceKind::Foreground.type_value(), Some(0));
        assert_eq!(SliceKind::Background.type_value(), Some(2));
        assert_eq!(SliceKind::Hotspot.type_value(), None);
        assert_eq!(SliceKind::of(&web_object(Some(0))), SliceKind::Foreground);
        assert_eq!(SliceKind::of(&web_object(Some(2))), SliceKind::Background);
        assert_eq!(SliceKind::of(&web_object(None)), SliceKind::Hotspot);
    }

    /// The export formats the panel offers are the ones the web
    /// export can write; the stored override picks one.
    #[test]
    fn the_export_formats_follow_the_slice_override() {
        let plain = web_object(Some(0));
        assert_eq!(ExportFormat::of(&plain), ExportFormat::Png);
        assert!(!ExportFormat::is_override(&plain));
        let jpeg = SliceObject {
            export: Some(doc::ExportOptions {
                format: Some(1),
                ..doc::ExportOptions::default()
            }),
            ..web_object(Some(0))
        };
        assert_eq!(ExportFormat::of(&jpeg), ExportFormat::Jpeg);
        assert!(ExportFormat::is_override(&jpeg));
        assert_eq!(ExportFormat::Jpeg.code(), 1);
    }

    /// The swap family maps onto the kinds the web export reads.
    #[test]
    fn the_swap_family_maps_onto_the_export_kinds() {
        assert_eq!(
            plan(BehaviorEvent::MouseOver, BehaviorAction::SwapImage),
            BehaviorPlan::Kind {
                kind: SWAP_IMAGE_BEHAVIOR,
                target: true
            }
        );
        assert_eq!(
            plan(BehaviorEvent::MouseOut, BehaviorAction::SwapImageRestore),
            BehaviorPlan::Kind {
                kind: SWAP_RESTORE_BEHAVIOR,
                target: false
            }
        );
        assert_eq!(
            plan(BehaviorEvent::RollOver, BehaviorAction::RollOver),
            BehaviorPlan::Kind {
                kind: NAV_HIGHLIGHT_BEHAVIOR,
                target: true
            }
        );
        assert_eq!(
            plan(BehaviorEvent::MouseDown, BehaviorAction::RollOver),
            BehaviorPlan::Kind {
                kind: NAV_DOWN_BEHAVIOR,
                target: true
            }
        );
        assert_eq!(
            plan(BehaviorEvent::RollOut, BehaviorAction::RollOver),
            BehaviorPlan::Kind {
                kind: NAV_RESTORE_BEHAVIOR,
                target: false
            }
        );
    }

    /// The link family writes the object's anchor URL; the pop-up
    /// menu records its kind.
    #[test]
    fn the_link_family_writes_the_anchor() {
        assert_eq!(
            plan(BehaviorEvent::Click, BehaviorAction::Link),
            BehaviorPlan::Link
        );
        assert_eq!(
            plan(BehaviorEvent::Click, BehaviorAction::GoToWebPage),
            BehaviorPlan::Link
        );
        assert_eq!(
            plan(BehaviorEvent::Click, BehaviorAction::NamedAnchor),
            BehaviorPlan::Link
        );
        assert_eq!(
            plan(BehaviorEvent::Click, BehaviorAction::SetPopUp),
            BehaviorPlan::Kind {
                kind: POPUP_MENU_BEHAVIOR,
                target: false
            }
        );
    }

    /// The pairs the engine cannot hold are named, not dropped.
    #[test]
    fn the_unsupported_pairs_say_why() {
        assert!(!plan(BehaviorEvent::Click, BehaviorAction::Sound).is_supported());
        assert!(!plan(BehaviorEvent::KeyPress, BehaviorAction::GoToWebPage).is_supported());
        assert!(!plan(BehaviorEvent::Focus, BehaviorAction::Link).is_supported());
        assert!(plan(BehaviorEvent::MouseOver, BehaviorAction::SwapImage).is_supported());
    }

    /// The swap-family kinds name their target through the export
    /// key; the restore and pop-up kinds do not.
    #[test]
    fn the_targeted_kinds_are_the_swap_family() {
        assert!(is_targeted(SWAP_IMAGE_BEHAVIOR));
        assert!(is_targeted(NAV_HIGHLIGHT_BEHAVIOR));
        assert!(is_targeted(NAV_DOWN_BEHAVIOR));
        assert!(!is_targeted(SWAP_RESTORE_BEHAVIOR));
        assert!(!is_targeted(NAV_RESTORE_BEHAVIOR));
        assert!(!is_targeted(POPUP_MENU_BEHAVIOR));
    }

    /// The swap target rides the `DTAv` export key.
    #[test]
    fn the_swap_target_rides_the_export_key() {
        let plain = web_object(Some(0));
        assert_eq!(swap_target_of(&plain), None);
        let named = SliceObject {
            custom_data: std::collections::BTreeMap::from([(
                SWAP_TARGET_KEY.to_owned(),
                "plate".to_owned(),
            )]),
            ..web_object(Some(0))
        };
        assert_eq!(swap_target_of(&named).as_deref(), Some("plate"));
    }

    /// The rows carry the facts CS6's panel shows: the export format
    /// override, the behavior stack, and the frame's size.
    #[test]
    fn the_rows_carry_the_export_settings_and_the_behavior_stack() {
        let slice = SliceObject {
            slice_type: Some(0),
            shape: 1,
            export: Some(doc::ExportOptions {
                format: Some(1),
                jpeg_quality: Some(60),
                ..doc::ExportOptions::default()
            }),
            behaviors: vec![doc::SliceBehavior {
                kind: SWAP_IMAGE_BEHAVIOR,
            }],
            frame: geom::Rect::new(10.0, 20.0, 90.0, 60.0),
            ..SliceObject::default()
        };
        let mut document = doc::Document::default();
        let id = document.registry.insert(doc::ObjectData::Slice(slice));
        let rows = slice_rows(&document);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.id, id);
        assert_eq!(row.kind, SliceKind::Foreground);
        assert_eq!(row.shape, SliceShape::Ellipse);
        assert_eq!(row.format, ExportFormat::Jpeg);
        assert!(row.overridden);
        assert_eq!(row.jpeg_quality, Some(60));
        assert_eq!(row.behaviors, vec![SWAP_IMAGE_BEHAVIOR]);
        assert_eq!(row.dimensions(), "80 × 40");
        assert_eq!(row.detail(), "Ellipse · JPEG · 1 behavior");
    }

    /// The stored kinds are named; an unknown kind shows its number.
    #[test]
    fn the_behavior_labels_name_the_documented_kinds() {
        assert_eq!(behavior_label(SWAP_IMAGE_BEHAVIOR), "Swap Image");
        assert_eq!(behavior_label(POPUP_MENU_BEHAVIOR), "Set Pop-up");
        assert_eq!(behavior_label(7), "Roll Over Restore (nav bar)");
        assert_eq!(behavior_label(42), "Behavior 42");
    }
}
