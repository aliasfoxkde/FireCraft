//! The Fireworks CS6 menu bar, transcribed as data.
//!
//! Ground truth: the CS6 `Fireworks Resources.dll` `RT_MENU` 128 tree and
//! the `English/Keyboard Shortcuts/Fireworks.xml` map, extracted per the
//! recipe in `docs/research/fw-ui-evidence.md` (raw dumps stay in the
//! scratch RE workspace; this table is the fresh, derived implementation).
//! Item ids are the resource command ids, kept so structure tests pin the
//! transcription and future work can map more of them.
//!
//! Two axes of honesty live here:
//! - an item present in CS6 but not wired in `FireCraft` yet is
//!   [`Action::Inert`] — the shell renders it disabled rather than
//!   pretending;
//! - displayed accelerator text matches the CS6 keymap even when the
//!   action is still [`Action::Inert`], so the bar reads like the
//!   original while the [`SHORTCUTS`] table drives only what works.

use crate::editor::Tool;

/// What a menu item (or keyboard shortcut) does in this shell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    /// Present in CS6, not yet wired — renders disabled.
    Inert,
    /// Shell-level behavior (file, view, edit plumbing).
    Shell(ShellAction),
    /// Engine command whose payload derives from the selection.
    Sel(SelAction),
    /// `object.align` with a mode string on the live selection.
    Align(&'static str),
    /// `object.distribute` with an axis string.
    Distribute(&'static str),
    /// `path.boolean` with an op string (needs exactly two live paths).
    Boolean(&'static str),
    /// `object.reorder` with the target index computed at dispatch.
    Arrange(Arrange),
    /// `text.setAttrs` size.
    TextSize(f64),
    /// `text.setAttrs` boolean attribute (bold/italic/underline).
    TextFlag(&'static str),
    /// `text.setAttrs` with the style flags cleared (Text ▸ Plain).
    TextPlain,
    /// `text.setAttrs` justification, `JSTi` 0..=3.
    TextJustify(i64),
    /// `text.setAttrs` size nudged one step from the current value
    /// (Text ▸ Smaller/Larger; CS6's step is the next preset down/up).
    TextStep(i64),
    /// `text.setFont` with a family name (Text ▸ Font; the same
    /// list the inspector's font combo offers).
    TextFont(&'static str),
    /// Filters ▸ Repeat Filter: re-applies the last filter preset the
    /// session applied (menu row or live-filters panel) to the current
    /// selection. CS6's `dom.applyFilter` repeat behavior.
    RepeatFilter,
    /// `text.setTextOnPathMode` with a `TAMi` 0..=3 mode.
    TextOnPathMode(i64),
    /// `filter.add` from the Filters menu: the preset's internal name,
    /// resolved through the same registry the Live Filters panel uses
    /// (`shell::filter_preset`). CS6 builds this menu at runtime from
    /// its filter registry — the static `RT_MENU` resource carries only
    /// Repeat Filter — and `FireCraft` mirrors exactly that.
    Filter(&'static str),
    /// A page-scoped document verb (canvas/rotation family); the
    /// payload derives from the active page, not the selection.
    Document(DocumentAction),
}

/// Page-scoped document verbs — Modify ▸ Canvas and friends. Each
/// maps to one registry command whose payload names the active page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentAction {
    /// `document.setDocumentCanvasSizeToDocumentExtents` — shrink the
    /// canvas to hug all page content (CS6's Trim Canvas).
    TrimCanvas,
    /// `document.setDocumentCanvasSizeToSelection` — canvas hugs the
    /// selection's extents (CS6's Fit Canvas).
    FitCanvas,
    /// `document.rotate` — rotate the page clockwise by degrees
    /// (90/180/270 only; CS6's 90° CCW is 270 CW).
    Rotate(i64),
}

/// Shell-level behaviors — plumbing that never goes through the engine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShellAction {
    New,
    Open,
    /// File ▸ Import… — place a raster (PNG/JPEG/GIF/BMP/TIFF/WebP)
    /// into the current document on the active layer.
    Import,
    Save,
    SaveAs,
    Export,
    ExportPreview,
    /// File ▸ Export Wizard…: the export dialog, opened by its own
    /// CS6 row (the same dialog as File ▸ Export).
    ExportWizard,
    /// File ▸ Export ▸ JPEG Compressed…: the export dialog opened on
    /// the JPEG format with its compression options.
    ExportJpeg,
    /// File ▸ File Info…: the document's facts (pages, layers,
    /// objects, canvas).
    FileInfo,
    /// File ▸ Export ▸ Fireworks PNG (format)…: the PNG format facts
    /// of the live document (canvas, color depth, object count).
    PngFormatInfo,
    /// File ▸ Close / Close All: discard the open document and start
    /// a fresh one.
    CloseDocument,
    /// File ▸ Save All: save the open document (the shell holds one).
    SaveAll,
    /// File ▸ Revert: reload the document from the file it was last
    /// saved to.
    Revert,
    /// Modify ▸ Transform ▸ Numeric Transform (Scale/Skew route here
    /// too): opens the numeric dialog over the live selection.
    NumericTransform,
    /// Set how the selection's transform handles behave: CS6's Free
    /// Transform (scale, the default) vs Distort (corner handles move
    /// independently, setting the quadrangle).
    TransformMode(TransformMode),
    /// The command palette (Ctrl+P): every registered engine command.
    Palette,
    Quit,
    Undo,
    Redo,
    Deselect,
    SelectAll,
    /// Select ▸ Select Inverse — everything in the current cell the
    /// selection does not cover.
    SelectInverse,
    /// Edit ▸ Cut — copy the selection to the clipboard, then delete it.
    Cut,
    /// Edit ▸ Copy — copy the selection to the clipboard.
    Copy,
    /// Edit ▸ Paste — paste the clipboard onto the active layer.
    Paste,
    /// Edit ▸ Paste as Mask — paste the clipboard as the selection's mask.
    PasteAsMask,
    /// Edit ▸ Paste Inside — paste the clipboard inside the selection.
    PasteInside,
    ZoomIn,
    ZoomOut,
    /// Set an absolute magnification (fraction: 1.0 = 100%).
    ZoomTo(f32),
    FitAll,
    About,
    /// Help ▸ Keyboard Shortcuts — the shortcut map panel (F1).
    ShowShortcuts,
    /// Edit ▸ Preferences… — the preferences dialog (UI scale and
    /// other persisted user settings).
    ShowPreferences,
    /// One-key tool select (the CS6 tool letters).
    PickTool(Tool),
    /// One-key tool *group* select: CS6 gives a group one shortcut
    /// and cycles through the group's tools on repeated presses. The
    /// tool named is the group's leader.
    PickToolGroup(Tool),
    /// View ▸ Grid ▸ Show Grid — the page overlay.
    ToggleGrid,
    /// View ▸ Grid ▸ Snap to Grid.
    ToggleSnapToGrid,
    /// View ▸ Grid ▸ Edit Grid... — the grid dialog.
    EditGrid,
    /// View ▸ Guides ▸ Show Guides.
    ToggleGuides,
    /// View ▸ Guides ▸ Lock Guides — a locked guide ignores drag-out
    /// and move drags cannot snap to it.
    ToggleGuidesLocked,
    /// View ▸ Guides ▸ Snap to Guides.
    ToggleSnapToGuides,
    /// View ▸ Guides ▸ Clear Guides — `page.removeAllGuides`.
    ClearGuides,
    /// View ▸ Fit Selection — zoom/pan to the selection bounds.
    FitSelection,
    /// View ▸ Smart Guides ▸ Show Smart Guides.
    ToggleSmartGuides,
    /// View ▸ Smart Guides ▸ Snap to Smart Guides.
    ToggleSnapToSmartGuides,
    /// View ▸ 9-Slice Scaling Guides ▸ Lock Guides.
    ToggleSliceLock,
    /// View ▸ Slice Guides.
    ToggleSliceGuides,
    /// View ▸ Slice Overlay.
    ToggleSliceOverlay,
    /// View ▸ Edges — object edge highlighting.
    ToggleEdges,
    /// View ▸ Rulers.
    ToggleRulers,
    /// View ▸ Tooltips.
    ToggleTooltips,
    /// View ▸ Show All — re-show everything Hide Selection folded.
    ShowAllHidden,
    /// View ▸ Gamma 1.8 — the 1.8 gamma preview toggle.
    ToggleGamma,
    /// Select ▸ Feather… — the feather-radius dialog over the pixel
    /// selection.
    FeatherDialog,
    /// Modify ▸ Canvas ▸ Canvas Size… — the absolute canvas-resize
    /// dialog with the CS6 anchor grid.
    CanvasSize,
    /// Modify ▸ Canvas ▸ Canvas Color… — the document matte color.
    CanvasColor,
    /// Modify ▸ Canvas ▸ Image Size… — scale the document into a
    /// target rect (`document.setImageSize`).
    ImageSize,
    /// Help ▸ Online Help — opens the project wiki in the browser.
    OnlineHelp,
    /// Help ▸ Manage Extensions… — the extension (script) manager.
    ManageExtensions,
    /// Window ▸ (panel name) — toggle one right-dock panel.
    TogglePanel(&'static str),
    /// Window ▸ Tools: fold/unfold the left tools dock.
    ToggleTools,
    /// A Window row whose CS6 semantics need multiple open documents
    /// (Duplicate Window, Cascade, Tile). The shell is single-document
    /// today; report that instead of sitting inert.
    SingleDocument,
    /// Window ▸ Hide Panels — fold/unfold the whole right dock.
    ToggleHidePanels,
    /// Window ▸ Toolbars ▸ Main — the context options bar.
    ToggleMainToolbar,
    /// The Tools panel's screen-mode control: cycle Standard Screen /
    /// Full Screen with Menus / Full Screen (fwref 1178–1185).
    CycleScreenMode,
    /// File ▸ Preview in Browser (primary/secondary/all pages):
    /// export the document to a PNG next to it (or in the temp dir for
    /// an unsaved document) and open it in the system browser.
    PreviewInBrowser {
        /// `true` = the user's secondary browser row; CS6 lets the OS
        /// default decide when no secondary is configured, and so does
        /// this build (both rows open the default handler today).
        secondary: bool,
        /// Preview All Pages: one PNG per page, all opened.
        all_pages: bool,
    },
    /// A File row whose CS6 backing service does not exist in this
    /// build (Bridge, TWAIN scanners, printer stack, HTML round-trip,
    /// batch processor, templates, e-mail). The message names the gap
    /// plainly instead of the row sitting inert.
    ReportGap(&'static str),
    /// Edit ▸ Insert ▸ Hotspot / Rectangular Slice / Polygon Slice /
    /// New Button…: insert a web object or button definition sized to
    /// the selection bounds (canvas center when nothing is selected).
    /// The polygon variant traces the selection's rectangle (CS6 asks
    /// for the vertices interactively — the drag tool does that).
    InsertWeb {
        /// What to insert.
        what: InsertWebKind,
    },
    /// Move the state (frame) pointer by a step — the keymap's `,`/`.`
    /// Frames rows (`cmd_Frame_Previous` / `cmd_Frame_Next`).
    SelectFrame(i64),
    /// Move the page pointer by a step — the keymap's PageUp/PageDown
    /// rows (`cmd_Page_Previous` / `cmd_Page_Next`).
    SelectPage(i64),
    /// Enter edit on the selected object: a text object opens the
    /// in-canvas text overlay; any other selection reports that the
    /// kind has no editor. The keymap's Text Editor / Edit Image
    /// Object rows.
    EditTextSelection,
}

/// What Edit ▸ Insert lands on the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InsertWebKind {
    /// A link-only hotspot area (`hotspot.create`).
    Hotspot,
    /// A bounding-box slice (`slice.create`).
    RectSlice,
    /// A polygon slice (`slice.create`, polygon shape).
    PolygonSlice,
    /// A multi-state button definition (`library.add`, button type).
    Button,
}

/// Engine commands with a selection- or document-derived payload.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelAction {
    /// `object.delete` on the selection.
    Delete,
    /// `path.simplify` at the CS6-default 1px tolerance.
    Simplify,
    /// `path.expandStroke` 2px butt/miter/4.
    ExpandStroke,
    /// `path.inset` at a signed distance (negative expands outward).
    Inset(f64),
    /// `library.add` — an empty graphic symbol named from the count.
    NewSymbol,
    /// `layer.add` on the current page.
    NewLayer,
    /// `page.add` appended.
    NewPage,
    /// `frame.add` — one empty state appended after the current one
    /// (CS6's Insert ▸ State).
    NewFrame,
    /// `symbol.breakApart` on the selection.
    BreakApart,
    /// `symbol.create` — fold the selection into a new graphic
    /// definition (CS6's Symbol ▸ Convert to Symbol).
    ConvertToSymbol,
    /// `symbol.create` with the `animation` type — CS6's
    /// Modify ▸ Animation ▸ Animate Selection.
    AnimateSelection,
    /// `symbol.tween` — CS6's Symbol ▸ Tween Instances (4 steps,
    /// distribute to frames).
    TweenInstances,
    /// `object.setVisible` false on the selection.
    Hide,
    /// `object.setLocked` true on the selection.
    LockSelection,
    /// `object.group` — fold the selection into one group.
    Group,
    /// `object.ungroup` — dissolve the selected group.
    Ungroup,
    /// `flatten.selection` — bake the selection into one bitmap.
    FlattenSelection,
    /// `flatten.mergeDown` — merge the selection into the bitmap below.
    MergeDown,
    /// `flatten.document` — collapse the page into one bitmap.
    FlattenLayers,
    /// `object.rotate` — rotate the selection clockwise by degrees.
    Rotate(f64),
    /// `object.reflect` — flip the selection.
    Reflect {
        /// Flip left-right.
        horizontal: bool,
        /// Flip top-bottom.
        vertical: bool,
    },
    /// `selection.selectParents` — collapse each fully-selected group.
    SelectParents,
    /// `selection.selectChildren` — expand each selected group.
    SelectChildren,
    /// Move the selection by whole pixels — the keymap's nudge arrows
    /// (1 px plain, 10 px big-nudge modifiers; fwref `cmd_Nudge*`,
    /// `cmd_BigNudge*`).
    Nudge {
        /// Horizontal pixels to move.
        dx: f64,
        /// Vertical pixels to move.
        dy: f64,
    },
    /// `bitmap.selectSimilar` — select matching pixels image-wide.
    SelectSimilar,
    /// `bitmap.feather` — feather the pixel selection.
    Feather(u8),
    /// `bitmap.adjustPixelSel` — grow/shrink/border/smooth the marquee.
    AdjustPixelSel {
        /// "expand" / "contract" / "border" / "smooth".
        what: &'static str,
        /// Pixels to adjust by.
        amount: i64,
    },
    /// `mask.pathToMarquee` — rasterize the selected path into the marquee.
    PathToMarquee,
    /// `mask.marqueeToPath` — trace the marquee into a path.
    MarqueeToPath,
    /// `selection.save` — save the pixel selection.
    SaveSelection,
    /// `selection.restore` — restore a saved pixel selection.
    RestoreSelection,
    /// `selection.deleteSaved` — delete a saved pixel selection.
    DeleteSavedSelection,
    /// `mask.add` — mask the selection (reveal/hide).
    AddMask {
        /// "reveal all" / "hide all" / "reveal selection" / "hide selection".
        mode: &'static str,
    },
    /// `mask.enable` — flip the selection's element mask between
    /// enabled and disabled. CS6's Modify ▸ Mask carries one
    /// enable/disable row whose visible label flips with the mask's
    /// state, so the shell sends the opposite of the current state.
    ToggleMask,
    /// `mask.link` — link/unlink the selection's mask.
    LinkMask(bool),
    /// `mask.remove` — apply or discard the selection's mask.
    RemoveMask {
        /// "apply" / "discard".
        what_if_image: &'static str,
    },
    /// `mask.saveJPEG` — stash the pixel selection as a JPEG mask.
    SaveJpegMask,
    /// `mask.restoreJPEG` — restore the stashed JPEG mask.
    RestoreJpegMask,
    /// `mask.clearJPEG` — drop the stashed JPEG mask.
    ClearJpegMask,
    /// `object.duplicate` — CS6 Edit ▸ Clone: the copy lands directly
    /// on top of the original (a following drag moves it).
    Clone,
    /// `object.duplicate` — CS6 Edit ▸ Duplicate (same command; CS6's
    /// variant differs only by the interactive placement drag).
    Duplicate,
    /// `text.detachPath` — remove the selected text's on-path guide.
    DetachFromPath,
    /// `text.attachPath` — lay the selected text along a selected
    /// path (`TAPv`): the selection must carry exactly one text and
    /// one path object; the path becomes the rail.
    AttachToPath,
    /// `path.splitPaths` — CS6 Modify ▸ Split: separate each selected
    /// path's contours into independent path objects.
    SplitPaths,
    /// Reset every selected object's `MTXv` to identity (CS6 Modify ▸
    /// Transform ▸ Remove Transformations).
    RemoveTransformations,
    /// `symbol.breakApart` on the selected symbol instance — CS6
    /// Modify ▸ Animation ▸ Remove Animation: dissolve the animation
    /// back to plain objects.
    RemoveAnimation,
    /// `text.reversePathTextDirection` — flip the selected text's
    /// travel direction along its rail.
    ReversePathText,
    /// `text.convertToPaths` — freeze the selected text into glyph
    /// outline paths (in place, same registry ids).
    ConvertToPaths,
    /// `bitmap.addImage` — CS6 Edit ▸ Insert ▸ Empty Bitmap: an empty
    /// image the size of the canvas on the active layer.
    EmptyBitmap,
    /// `bitmap.addImageViaCopy` — CS6 Edit ▸ Insert ▸ Bitmap Via
    /// Copy: the pixel selection of the selected bitmap becomes a new
    /// image above it.
    BitmapViaCopy,
    /// `bitmap.addImageViaCut` — the copy that also clears the
    /// selection from the source bitmap.
    BitmapViaCut,
    /// `document.setDocumentCanvasSizeToDocumentExtents` with `grow`
    /// true — CS6 Edit ▸ Crop Document: shrink the canvas to the
    /// content extents.
    CropDocument,
    /// `path.setFillEdgeMode` on every selected path — Modify ▸
    /// Alter Path ▸ Hard / Anti-Alias / Feather Fill.
    FillEdge {
        /// "hard" / "antialias" / "feather".
        mode: &'static str,
    },
    /// Enter the selected instance's definition — Modify ▸ Symbol ▸
    /// Edit Symbol / Edit in Place (a view-state change; the two rows
    /// differ only in CS6's presentation).
    EditSymbol,
}

/// The Modify ▸ Arrange items; dispatch resolves the numeric index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrange {
    Front,
    Forward,
    Backward,
    Back,
}

/// How the selection's transform handles behave (Modify ▸ Transform).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformMode {
    /// Free Transform: handles scale (the default).
    Scale,
    /// Distort: corner handles move independently (the quadrangle).
    Distort,
}

/// One command row: resource id, display label, accelerator text, action.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    /// CS6 resource command id (the `[35405]` column of the menu dump).
    pub id: u32,
    /// Label with Windows mnemonic `&` markers stripped.
    pub label: &'static str,
    /// Accelerator as CS6 displays it ("" when none).
    pub accel: &'static str,
    pub action: Action,
    /// Set when the visible label depends on live document state; the
    /// shell resolves it at render time.
    pub dynamic: Option<DynamicLabel>,
}

/// A row whose visible label depends on live document state.
///
/// CS6's resource tables are static, but the reference application
/// swaps a row's label at runtime — the Modify ▸ Mask enable/disable
/// row carries the resource label "Disable Mask" and shows "Enable
/// Mask" while the selection's mask is off.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DynamicLabel {
    /// The mask enable/disable toggle: "Enable Mask" while the
    /// selection's element mask is disabled, "Disable Mask" otherwise.
    MaskToggle,
}

/// A menu-tree node: a separator, a command row, or a submenu.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Entry {
    Sep,
    Item(Item),
    /// Submenu (id 0 — popups carry no command id in the dump).
    Sub(&'static str, &'static [Entry]),
}

/// One top-level menu bar title with its tree.
#[derive(Clone, Copy)]
pub struct Menu {
    pub label: &'static str,
    pub entries: &'static [Entry],
}

const fn item(id: u32, label: &'static str, accel: &'static str, action: Action) -> Entry {
    Entry::Item(Item {
        id,
        label,
        accel,
        action,
        dynamic: None,
    })
}

/// A command row whose visible label the shell resolves from live
/// document state.
const fn item_dynamic(
    id: u32,
    label: &'static str,
    accel: &'static str,
    action: Action,
    dynamic: DynamicLabel,
) -> Entry {
    Entry::Item(Item {
        id,
        label,
        accel,
        action,
        dynamic: Some(dynamic),
    })
}

/// File — ids from `RT_MENU` 128; Ctrl+N/O/S/Shift+S/Shift+R from the
/// Fireworks.xml keymap, Ctrl+F4 as displayed by the resource itself.
pub static FILE: Menu = Menu {
    label: "File",
    entries: &[
        item(57600, "New", "Ctrl+N", Action::Shell(ShellAction::New)),
        item(
            37866,
            "New from Template...",
            "",
            Action::Shell(ShellAction::ReportGap(
                "Document templates are not available in this build.",
            )),
        ),
        item(57601, "Open...", "Ctrl+O", Action::Shell(ShellAction::Open)),
        item(
            35795,
            "Browse in Bridge...",
            "Ctrl+Shift+O",
            Action::Shell(ShellAction::ReportGap(
                "Adobe Bridge is not available in this build.",
            )),
        ),
        Entry::Sub(
            "Open Recent",
            &[item(
                57616,
                "Recent File",
                "",
                Action::Shell(ShellAction::ReportGap(
                    "The recent-files list is not kept in this build; use File ▸ Open.",
                )),
            )],
        ),
        Entry::Sub(
            "Scan",
            &[
                item(
                    35199,
                    "Twain Acquire...",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "TWAIN scanner acquisition is not available in this build.",
                    )),
                ),
                Entry::Sep,
                item(
                    35200,
                    "Twain Select...",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "TWAIN scanner selection is not available in this build.",
                    )),
                ),
            ],
        ),
        item(
            57602,
            "Close",
            "Ctrl+F4",
            Action::Shell(ShellAction::CloseDocument),
        ),
        item(
            35789,
            "Close All",
            "",
            Action::Shell(ShellAction::CloseDocument),
        ),
        Entry::Sep,
        item(57603, "Save", "Ctrl+S", Action::Shell(ShellAction::Save)),
        item(
            57604,
            "Save As...",
            "Ctrl+Shift+S",
            Action::Shell(ShellAction::SaveAs),
        ),
        item(35850, "Save All", "", Action::Shell(ShellAction::SaveAll)),
        item(
            37867,
            "Save As Template...",
            "",
            Action::Shell(ShellAction::ReportGap(
                "Document templates are not available in this build.",
            )),
        ),
        item(35036, "Revert", "", Action::Shell(ShellAction::Revert)),
        Entry::Sep,
        item(
            35012,
            "Import...",
            "Ctrl+R",
            Action::Shell(ShellAction::Import),
        ),
        item(
            35007,
            "Export...",
            "Ctrl+Shift+R",
            Action::Shell(ShellAction::Export),
        ),
        item(
            35404,
            "Image Preview...",
            "Ctrl+Shift+X",
            Action::Shell(ShellAction::ExportPreview),
        ),
        Entry::Sub(
            "Send to E-mail",
            &[
                item(
                    35587,
                    "Fireworks PNG (format)...",
                    "",
                    Action::Shell(ShellAction::PngFormatInfo),
                ),
                item(
                    35588,
                    "JPEG Compressed...",
                    "",
                    Action::Shell(ShellAction::ExportJpeg),
                ),
                item(
                    35420,
                    "Use Export Settings...",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "Saved export-settings presets are not available in this build.",
                    )),
                ),
            ],
        ),
        item(
            35416,
            "Update HTML...",
            "",
            Action::Shell(ShellAction::ReportGap(
                "HTML export and re-export are not available in this build.",
            )),
        ),
        Entry::Sep,
        item(
            35792,
            "File Info...",
            "Ctrl+Alt+Shift+F",
            Action::Shell(ShellAction::FileInfo),
        ),
        Entry::Sep,
        item(
            35181,
            "Export Wizard...",
            "",
            Action::Shell(ShellAction::ExportWizard),
        ),
        item(
            35302,
            "Batch Process...",
            "",
            Action::Shell(ShellAction::ReportGap(
                "Batch processing of multiple files is not available in this build.",
            )),
        ),
        Entry::Sub(
            "Preview in Browser",
            &[
                item(
                    35313,
                    "Preview in Primary Browser",
                    "F12",
                    Action::Shell(ShellAction::PreviewInBrowser {
                        secondary: false,
                        all_pages: false,
                    }),
                ),
                item(
                    35315,
                    "Preview in Secondary Browser",
                    "Shift+F12",
                    Action::Shell(ShellAction::PreviewInBrowser {
                        secondary: true,
                        all_pages: false,
                    }),
                ),
                Entry::Sep,
                item(
                    49003,
                    "Preview All Pages in Primary Browser",
                    "Ctrl+F12",
                    Action::Shell(ShellAction::PreviewInBrowser {
                        secondary: false,
                        all_pages: true,
                    }),
                ),
                Entry::Sep,
                item(
                    35314,
                    "Set Primary Browser...",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "Browser selection is not configurable in this build; the system default is used.",
                    )),
                ),
                item(
                    35316,
                    "Set Secondary Browser...",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "Browser selection is not configurable in this build; the system default is used.",
                    )),
                ),
            ],
        ),
        Entry::Sep,
        item(
            57605,
            "Page Setup...",
            "",
            Action::Shell(ShellAction::ReportGap(
                "Printing and page setup are not available in this build.",
            )),
        ),
        item(
            57607,
            "Print...",
            "Ctrl+P",
            Action::Shell(ShellAction::ReportGap(
                "Printing is not available in this build.",
            )),
        ),
        item(
            35452,
            "HTML Setup...",
            "",
            Action::Shell(ShellAction::ReportGap(
                "HTML export options are not available in this build.",
            )),
        ),
        Entry::Sep,
        item(57665, "Exit", "Alt+F4", Action::Shell(ShellAction::Quit)),
    ],
};

/// Edit — the Win32 MDI standard ids (57632–57644) keep their
/// conventional accelerators; Duplicate/Clone/Keyboard Shortcuts come
/// from the Fireworks.xml keymap.
pub static EDIT: Menu = Menu {
    label: "Edit",
    entries: &[
        item(57643, "Undo", "Ctrl+Z", Action::Shell(ShellAction::Undo)),
        item(
            57644,
            "Redo",
            "Ctrl+Shift+Z",
            Action::Shell(ShellAction::Redo),
        ),
        Entry::Sep,
        Entry::Sub(
            "Insert",
            &[
                item(
                    35191,
                    "New Button...",
                    "",
                    Action::Shell(ShellAction::InsertWeb {
                        what: InsertWebKind::Button,
                    }),
                ),
                item(
                    35195,
                    "New Symbol...",
                    "Ctrl+F8",
                    Action::Sel(SelAction::NewSymbol),
                ),
                Entry::Sep,
                item(
                    35179,
                    "Hotspot",
                    "Ctrl+Shift+U",
                    Action::Shell(ShellAction::InsertWeb {
                        what: InsertWebKind::Hotspot,
                    }),
                ),
                item(
                    35237,
                    "Rectangular Slice",
                    "",
                    Action::Shell(ShellAction::InsertWeb {
                        what: InsertWebKind::RectSlice,
                    }),
                ),
                item(
                    35786,
                    "Polygon Slice",
                    "Alt+Shift+P",
                    Action::Shell(ShellAction::InsertWeb {
                        what: InsertWebKind::PolygonSlice,
                    }),
                ),
                Entry::Sep,
                item(
                    35180,
                    "Empty Bitmap",
                    "Ctrl+Alt+Y",
                    Action::Sel(SelAction::EmptyBitmap),
                ),
                item(
                    35581,
                    "Bitmap Via Copy",
                    "",
                    Action::Sel(SelAction::BitmapViaCopy),
                ),
                item(
                    35582,
                    "Bitmap Via Cut",
                    "",
                    Action::Sel(SelAction::BitmapViaCut),
                ),
                Entry::Sep,
                item(35309, "Layer", "Shift+L", Action::Sel(SelAction::NewLayer)),
                item(35301, "State", "Shift+F", Action::Sel(SelAction::NewFrame)),
                item(35809, "Page", "", Action::Sel(SelAction::NewPage)),
            ],
        ),
        item(
            43402,
            "Find and Replace...",
            "",
            Action::Shell(ShellAction::ReportGap(
                "Find and Replace needs the text-search service, which this build does not ship",
            )),
        ),
        Entry::Sep,
        item(57635, "Cut", "Ctrl+X", Action::Shell(ShellAction::Cut)),
        item(57634, "Copy", "Ctrl+C", Action::Shell(ShellAction::Copy)),
        item(
            35411,
            "Copy as Vectors",
            "",
            Action::Shell(ShellAction::ReportGap(
                "Copy as Vectors needs the vector-clipboard serializer, which this build does not ship",
            )),
        ),
        item(
            35415,
            "Copy HTML Code...",
            "",
            Action::Shell(ShellAction::ReportGap(
                "Copy HTML Code needs the HTML clipboard serializer, which this build does not ship",
            )),
        ),
        item(57637, "Paste", "Ctrl+V", Action::Shell(ShellAction::Paste)),
        item(57632, "Clear", "Del", Action::Sel(SelAction::Delete)),
        Entry::Sep,
        item(
            35430,
            "Paste as Mask",
            "Ctrl+Alt+V",
            Action::Shell(ShellAction::PasteAsMask),
        ),
        item(
            35127,
            "Paste Inside",
            "Ctrl+Shift+V",
            Action::Shell(ShellAction::PasteInside),
        ),
        item(
            35033,
            "Paste Attributes",
            "Ctrl+Alt+Shift+V",
            Action::Shell(ShellAction::ReportGap(
                "Paste Attributes needs the attributes clipboard, which this build does not ship",
            )),
        ),
        Entry::Sep,
        item(
            35006,
            "Duplicate",
            "Ctrl+Alt+D",
            Action::Sel(SelAction::Duplicate),
        ),
        item(
            35003,
            "Clone",
            "Ctrl+Shift+C",
            Action::Sel(SelAction::Clone),
        ),
        Entry::Sep,
        item(
            35145,
            "Crop Selected Bitmap",
            "Ctrl+Alt+C",
            Action::Shell(ShellAction::ReportGap(
                "Crop Selected Bitmap needs the bitmap-crop verb, which this build does not ship",
            )),
        ),
        item(
            35346,
            "Crop Document",
            "",
            Action::Sel(SelAction::CropDocument),
        ),
        Entry::Sep,
        item(
            35034,
            "Preferences...",
            "",
            Action::Shell(ShellAction::ShowPreferences),
        ),
        item(
            35429,
            "Keyboard Shortcuts...",
            "",
            Action::Shell(ShellAction::ReportGap(
                "Keyboard Shortcuts needs the shortcut editor, which this build does not ship",
            )),
        ),
    ],
};

/// View — magnification steps are the CS6 list verbatim.
pub static VIEW: Menu = Menu {
    label: "View",
    entries: &[
        item(
            35063,
            "Zoom In",
            "Ctrl+=",
            Action::Shell(ShellAction::ZoomIn),
        ),
        item(
            35064,
            "Zoom Out",
            "Ctrl+-",
            Action::Shell(ShellAction::ZoomOut),
        ),
        Entry::Sub(
            "Magnification",
            &[
                item(35624, "6%", "", Action::Shell(ShellAction::ZoomTo(0.06))),
                item(35617, "12%", "", Action::Shell(ShellAction::ZoomTo(0.12))),
                item(35620, "25%", "", Action::Shell(ShellAction::ZoomTo(0.25))),
                item(35623, "50%", "", Action::Shell(ShellAction::ZoomTo(0.5))),
                item(35625, "66%", "", Action::Shell(ShellAction::ZoomTo(0.66))),
                item(35616, "100%", "", Action::Shell(ShellAction::ZoomTo(1.0))),
                item(35626, "150%", "", Action::Shell(ShellAction::ZoomTo(1.5))),
                item(35619, "200%", "", Action::Shell(ShellAction::ZoomTo(2.0))),
                item(35627, "300%", "", Action::Shell(ShellAction::ZoomTo(3.0))),
                item(35622, "400%", "", Action::Shell(ShellAction::ZoomTo(4.0))),
                item(35629, "800%", "", Action::Shell(ShellAction::ZoomTo(8.0))),
                item(35618, "1600%", "", Action::Shell(ShellAction::ZoomTo(16.0))),
                item(35621, "3200%", "", Action::Shell(ShellAction::ZoomTo(32.0))),
                item(35628, "6400%", "", Action::Shell(ShellAction::ZoomTo(64.0))),
            ],
        ),
        Entry::Sep,
        item(
            35066,
            "Fit Selection",
            "Ctrl+0",
            Action::Shell(ShellAction::FitSelection),
        ),
        item(
            35065,
            "Fit All",
            "Ctrl+Alt+0",
            Action::Shell(ShellAction::FitAll),
        ),
        Entry::Sep,
        item(
            35367,
            "Gamma 1.8",
            "",
            Action::Shell(ShellAction::ToggleGamma),
        ),
        Entry::Sep,
        item(
            35071,
            "Hide Selection",
            "Ctrl+M",
            Action::Sel(SelAction::Hide),
        ),
        item(
            35826,
            "Tooltips",
            "",
            Action::Shell(ShellAction::ToggleTooltips),
        ),
        item(
            35072,
            "Show All",
            "Ctrl+Shift+M",
            Action::Shell(ShellAction::ShowAllHidden),
        ),
        Entry::Sep,
        item(
            35130,
            "Rulers",
            "Ctrl+Alt+R",
            Action::Shell(ShellAction::ToggleRulers),
        ),
        Entry::Sub(
            "Grid",
            &[
                item(
                    35131,
                    "Show Grid",
                    "Ctrl+'",
                    Action::Shell(ShellAction::ToggleGrid),
                ),
                item(
                    35132,
                    "Snap to Grid",
                    "Ctrl+Shift+'",
                    Action::Shell(ShellAction::ToggleSnapToGrid),
                ),
                item(
                    35133,
                    "Edit Grid...",
                    "Ctrl+Alt+G",
                    Action::Shell(ShellAction::EditGrid),
                ),
            ],
        ),
        Entry::Sub(
            "Guides",
            &[
                item(
                    35134,
                    "Show Guides",
                    "Ctrl+;",
                    Action::Shell(ShellAction::ToggleGuides),
                ),
                item(
                    35135,
                    "Lock Guides",
                    "Ctrl+Alt+;",
                    Action::Shell(ShellAction::ToggleGuidesLocked),
                ),
                item(
                    35136,
                    "Snap to Guides",
                    "Ctrl+Shift+;",
                    Action::Shell(ShellAction::ToggleSnapToGuides),
                ),
                item(
                    49004,
                    "Clear Guides",
                    "",
                    Action::Shell(ShellAction::ClearGuides),
                ),
            ],
        ),
        Entry::Sub(
            "Smart Guides",
            &[
                item(
                    35815,
                    "Show Smart Guides",
                    "Ctrl+\\",
                    Action::Shell(ShellAction::ToggleSmartGuides),
                ),
                item(
                    35821,
                    "Snap to Smart Guides",
                    "Ctrl+Shift+\\",
                    Action::Shell(ShellAction::ToggleSnapToSmartGuides),
                ),
            ],
        ),
        Entry::Sub(
            "9-Slice Scaling Guides",
            &[item(
                35812,
                "Lock Guides",
                "",
                Action::Shell(ShellAction::ToggleSliceLock),
            )],
        ),
        item(
            35299,
            "Slice Guides",
            "Ctrl+Alt+Shift+;",
            Action::Shell(ShellAction::ToggleSliceGuides),
        ),
        item(
            35426,
            "Slice Overlay",
            "",
            Action::Shell(ShellAction::ToggleSliceOverlay),
        ),
        Entry::Sep,
        item(
            35088,
            "Edges",
            "Ctrl+H",
            Action::Shell(ShellAction::ToggleEdges),
        ),
    ],
};

/// Select — bitmap-marquee operations are present but inert until the
/// bitmap selection model lands.
pub static SELECT: Menu = Menu {
    label: "Select",
    entries: &[
        item(
            35038,
            "Select All",
            "Ctrl+A",
            Action::Shell(ShellAction::SelectAll),
        ),
        item(
            35040,
            "Deselect",
            "Ctrl+D",
            Action::Shell(ShellAction::Deselect),
        ),
        item(
            35194,
            "Superselect",
            "Ctrl+↑",
            Action::Sel(SelAction::SelectParents),
        ),
        item(
            35193,
            "Subselect",
            "Ctrl+↓",
            Action::Sel(SelAction::SelectChildren),
        ),
        Entry::Sep,
        item(
            35041,
            "Select Similar",
            "",
            Action::Sel(SelAction::SelectSimilar),
        ),
        item(
            35039,
            "Select Inverse",
            "Ctrl+Shift+I",
            Action::Shell(ShellAction::SelectInverse),
        ),
        item(
            35089,
            "Feather...",
            "",
            Action::Shell(ShellAction::FeatherDialog),
        ),
        Entry::Sep,
        item(
            35405,
            "Expand Marquee...",
            "",
            Action::Sel(SelAction::AdjustPixelSel {
                what: "expand",
                amount: 2,
            }),
        ),
        item(
            35406,
            "Contract Marquee...",
            "",
            Action::Sel(SelAction::AdjustPixelSel {
                what: "contract",
                amount: 2,
            }),
        ),
        item(
            35407,
            "Border Marquee...",
            "",
            Action::Sel(SelAction::AdjustPixelSel {
                what: "border",
                amount: 2,
            }),
        ),
        item(
            35408,
            "Smooth Marquee...",
            "",
            Action::Sel(SelAction::AdjustPixelSel {
                what: "smooth",
                amount: 2,
            }),
        ),
        Entry::Sep,
        item(
            35780,
            "Convert Marquee to Path",
            "",
            Action::Sel(SelAction::MarqueeToPath),
        ),
        Entry::Sep,
        item(
            35467,
            "Save Bitmap Selection...",
            "",
            Action::Sel(SelAction::SaveSelection),
        ),
        item(
            35468,
            "Restore Bitmap Selection...",
            "",
            Action::Sel(SelAction::RestoreSelection),
        ),
        item(
            35783,
            "Delete Bitmap Selection...",
            "",
            Action::Sel(SelAction::DeleteSavedSelection),
        ),
    ],
};

/// Modify — the largest CS6 menu: canvas, symbol, mask, transform,
/// arrange, align, path surgery, grouping.
pub static MODIFY: Menu = Menu {
    label: "Modify",
    entries: &[
        Entry::Sub(
            "Canvas",
            &[
                item(
                    35226,
                    "Image Size...",
                    "",
                    Action::Shell(ShellAction::ImageSize),
                ),
                item(
                    35004,
                    "Canvas Size...",
                    "",
                    Action::Shell(ShellAction::CanvasSize),
                ),
                item(
                    35005,
                    "Canvas Color...",
                    "",
                    Action::Shell(ShellAction::CanvasColor),
                ),
                Entry::Sep,
                item(
                    35250,
                    "Trim Canvas",
                    "",
                    Action::Document(DocumentAction::TrimCanvas),
                ),
                item(
                    35574,
                    "Fit Canvas",
                    "",
                    Action::Document(DocumentAction::FitCanvas),
                ),
                Entry::Sep,
                item(
                    35348,
                    "Rotate 180°",
                    "",
                    Action::Document(DocumentAction::Rotate(180)),
                ),
                item(
                    35349,
                    "Rotate 90° CW",
                    "",
                    Action::Document(DocumentAction::Rotate(90)),
                ),
                item(
                    35350,
                    "Rotate 90° CCW",
                    "",
                    Action::Document(DocumentAction::Rotate(270)),
                ),
            ],
        ),
        Entry::Sep,
        Entry::Sub(
            "Animation",
            &[
                item(
                    35451,
                    "Animate Selection...",
                    "",
                    Action::Sel(SelAction::AnimateSelection),
                ),
                item(
                    35463,
                    "Settings...",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "animation settings need the animation dialog, which this build does not ship",
                    )),
                ),
                Entry::Sep,
                item(
                    35472,
                    "Remove Animation",
                    "",
                    Action::Sel(SelAction::RemoveAnimation),
                ),
            ],
        ),
        Entry::Sub(
            "Symbol",
            &[
                item(
                    35189,
                    "Convert to Symbol...",
                    "F8",
                    Action::Sel(SelAction::ConvertToSymbol),
                ),
                Entry::Sep,
                item(35334, "Edit Symbol", "", Action::Sel(SelAction::EditSymbol)),
                item(
                    35842,
                    "Edit in Place",
                    "",
                    Action::Sel(SelAction::EditSymbol),
                ),
                item(
                    35188,
                    "Tween Instances...",
                    "Ctrl+Alt+Shift+T",
                    Action::Sel(SelAction::TweenInstances),
                ),
                item(35190, "Break Apart", "", Action::Sel(SelAction::BreakApart)),
                item(
                    35807,
                    "Swap Symbol...",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "Swap Symbol needs an instance re-target verb, which this build does not ship",
                    )),
                ),
            ],
        ),
        Entry::Sub(
            "Pop-up Menu",
            &[
                item(
                    35445,
                    "Add Pop-up Menu...",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "Pop-up menus need the behavior structure editor, which this build does not ship",
                    )),
                ),
                item(
                    35592,
                    "Edit Pop-up Menu...",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "Pop-up menus need the behavior structure editor, which this build does not ship",
                    )),
                ),
                Entry::Sep,
                item(
                    35593,
                    "Delete Pop-up Menu",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "Pop-up menus need the behavior structure editor, which this build does not ship",
                    )),
                ),
            ],
        ),
        Entry::Sep,
        Entry::Sub(
            "Mask",
            &[
                item(
                    35431,
                    "Reveal All",
                    "",
                    Action::Sel(SelAction::AddMask { mode: "reveal all" }),
                ),
                item(
                    35432,
                    "Hide All",
                    "",
                    Action::Sel(SelAction::AddMask { mode: "hide all" }),
                ),
                Entry::Sep,
                item(
                    35430,
                    "Paste as Mask",
                    "Ctrl+Alt+V",
                    Action::Shell(ShellAction::PasteAsMask),
                ),
                item(
                    35128,
                    "Group as Mask",
                    "Ctrl+Shift+G",
                    Action::Shell(ShellAction::ReportGap(
                        "Group as Mask needs a mask-group verb, which this build does not ship",
                    )),
                ),
                Entry::Sep,
                item(
                    35433,
                    "Reveal Selection",
                    "",
                    Action::Sel(SelAction::AddMask {
                        mode: "reveal selection",
                    }),
                ),
                item(
                    35434,
                    "Hide Selection",
                    "",
                    Action::Sel(SelAction::AddMask {
                        mode: "hide selection",
                    }),
                ),
                Entry::Sep,
                item_dynamic(
                    35435,
                    "Disable Mask",
                    "Ctrl+Shift+K",
                    Action::Sel(SelAction::ToggleMask),
                    DynamicLabel::MaskToggle,
                ),
                item(
                    35436,
                    "Delete Mask",
                    "",
                    Action::Sel(SelAction::RemoveMask {
                        what_if_image: "discard",
                    }),
                ),
            ],
        ),
        Entry::Sub(
            "Selective JPEG",
            &[
                item(
                    35469,
                    "Save Selection as JPEG Mask",
                    "",
                    Action::Sel(SelAction::SaveJpegMask),
                ),
                item(
                    35470,
                    "Restore JPEG Mask as Selection",
                    "",
                    Action::Sel(SelAction::RestoreJpegMask),
                ),
                item(
                    35576,
                    "Settings...",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "JPEG mask settings need the selective-JPEG dialog, which this build does not ship",
                    )),
                ),
                Entry::Sep,
                item(
                    35471,
                    "Remove JPEG Mask",
                    "",
                    Action::Sel(SelAction::ClearJpegMask),
                ),
            ],
        ),
        Entry::Sep,
        item(
            35784,
            "Lock Selection",
            "",
            Action::Sel(SelAction::LockSelection),
        ),
        item(
            35009,
            "Flatten Selection",
            "Ctrl+Alt+Shift+Z",
            Action::Sel(SelAction::FlattenSelection),
        ),
        item(
            35583,
            "Merge Down",
            "Ctrl+Alt+E",
            Action::Sel(SelAction::MergeDown),
        ),
        item(
            35008,
            "Flatten Layers",
            "Ctrl+Alt+Shift+E",
            Action::Sel(SelAction::FlattenLayers),
        ),
        Entry::Sep,
        Entry::Sub(
            "Transform",
            &[
                item(
                    35178,
                    "Free Transform",
                    "Ctrl+T",
                    Action::Shell(ShellAction::TransformMode(TransformMode::Scale)),
                ),
                Entry::Sep,
                item(
                    35060,
                    "Scale",
                    "",
                    Action::Shell(ShellAction::NumericTransform),
                ),
                item(
                    35061,
                    "Skew",
                    "",
                    Action::Shell(ShellAction::NumericTransform),
                ),
                item(
                    35090,
                    "Distort",
                    "",
                    Action::Shell(ShellAction::TransformMode(TransformMode::Distort)),
                ),
                item(
                    35201,
                    "Numeric Transform...",
                    "Ctrl+Shift+T",
                    Action::Shell(ShellAction::NumericTransform),
                ),
                Entry::Sep,
                item(
                    35173,
                    "Rotate 180°",
                    "",
                    Action::Sel(SelAction::Rotate(180.0)),
                ),
                item(
                    35174,
                    "Rotate 90° CW",
                    "Ctrl+9",
                    Action::Sel(SelAction::Rotate(90.0)),
                ),
                item(
                    35175,
                    "Rotate 90° CCW",
                    "Ctrl+7",
                    Action::Sel(SelAction::Rotate(-90.0)),
                ),
                Entry::Sep,
                item(
                    35176,
                    "Flip Horizontal",
                    "",
                    Action::Sel(SelAction::Reflect {
                        horizontal: true,
                        vertical: false,
                    }),
                ),
                item(
                    35177,
                    "Flip Vertical",
                    "",
                    Action::Sel(SelAction::Reflect {
                        horizontal: false,
                        vertical: true,
                    }),
                ),
                Entry::Sep,
                item(
                    35152,
                    "Remove Transformations",
                    "",
                    Action::Sel(SelAction::RemoveTransformations),
                ),
            ],
        ),
        Entry::Sub(
            "Arrange",
            &[
                item(
                    35067,
                    "Bring to Front",
                    "Ctrl+F",
                    Action::Arrange(Arrange::Front),
                ),
                item(
                    35068,
                    "Bring Forward",
                    "Ctrl+Shift+F",
                    Action::Arrange(Arrange::Forward),
                ),
                item(
                    35070,
                    "Send Backward",
                    "Ctrl+Shift+B",
                    Action::Arrange(Arrange::Backward),
                ),
                item(
                    35069,
                    "Send to Back",
                    "Ctrl+B",
                    Action::Arrange(Arrange::Back),
                ),
            ],
        ),
        Entry::Sub(
            "Align",
            &[
                item(35073, "Left", "Ctrl+Alt+1", Action::Align("left")),
                item(
                    35075,
                    "Center Vertical",
                    "Ctrl+Alt+2",
                    Action::Align("centerX"),
                ),
                item(35074, "Right", "Ctrl+Alt+3", Action::Align("right")),
                Entry::Sep,
                item(35077, "Top", "Ctrl+Alt+4", Action::Align("top")),
                item(
                    35076,
                    "Center Horizontal",
                    "Ctrl+Alt+5",
                    Action::Align("centerY"),
                ),
                item(35078, "Bottom", "Ctrl+Alt+6", Action::Align("bottom")),
                Entry::Sep,
                item(
                    35080,
                    "Distribute Widths",
                    "Ctrl+Alt+7",
                    Action::Distribute("horizontal"),
                ),
                item(
                    35079,
                    "Distribute Heights",
                    "Ctrl+Alt+9",
                    Action::Distribute("vertical"),
                ),
            ],
        ),
        Entry::Sep,
        item(
            35781,
            "Convert Path to Marquee...",
            "",
            Action::Sel(SelAction::PathToMarquee),
        ),
        Entry::Sub(
            "Combine Paths",
            &[
                item(35094, "Join", "Ctrl+J", Action::Boolean("union")),
                item(
                    35095,
                    "Split",
                    "Ctrl+Shift+J",
                    Action::Sel(SelAction::SplitPaths),
                ),
                Entry::Sep,
                item(35211, "Union", "", Action::Boolean("union")),
                item(35212, "Intersect", "", Action::Boolean("intersect")),
                item(35217, "Punch", "", Action::Boolean("punch")),
                item(35213, "Crop", "", Action::Boolean("crop")),
            ],
        ),
        Entry::Sub(
            "Alter Path",
            &[
                item(35214, "Simplify...", "", Action::Sel(SelAction::Simplify)),
                item(
                    35215,
                    "Expand Stroke...",
                    "",
                    Action::Sel(SelAction::ExpandStroke),
                ),
                item(
                    35216,
                    "Inset Path...",
                    "",
                    Action::Sel(SelAction::Inset(2.0)),
                ),
                Entry::Sep,
                item(
                    35183,
                    "Hard Fill",
                    "",
                    Action::Sel(SelAction::FillEdge { mode: "hard" }),
                ),
                item(
                    35184,
                    "Anti-Alias Fill",
                    "",
                    Action::Sel(SelAction::FillEdge { mode: "antialias" }),
                ),
                item(
                    35185,
                    "Feather Fill",
                    "",
                    Action::Sel(SelAction::FillEdge { mode: "feather" }),
                ),
            ],
        ),
        item(
            35849,
            "Snap To Pixel",
            "Ctrl+K",
            Action::Shell(ShellAction::ReportGap(
                "Snap To Pixel needs a view snap mode, which this build does not ship",
            )),
        ),
        Entry::Sep,
        item(35010, "Group", "Ctrl+G", Action::Sel(SelAction::Group)),
        item(35062, "Ungroup", "Ctrl+U", Action::Sel(SelAction::Ungroup)),
    ],
};

/// Text — size steps are the CS6 list; styles map onto the
/// `text.setAttrs` boolean attributes.
pub static TEXT: Menu = Menu {
    label: "Text",
    entries: &[
        Entry::Sub(
            "Font",
            // The families the renderer actually carries; the
            // inspector's combo offers the same list. CS6 builds this
            // menu from the system font inventory — a live enumeration
            // is the evidence gap recorded in CS6-PARITY row 52.
            &[
                item(
                    35343,
                    "Liberation Sans",
                    "",
                    Action::TextFont("Liberation Sans"),
                ),
                item(35344, "Arial", "", Action::TextFont("Arial")),
                item(35345, "Verdana", "", Action::TextFont("Verdana")),
                item(
                    35346,
                    "Times New Roman",
                    "",
                    Action::TextFont("Times New Roman"),
                ),
                item(35347, "Courier New", "", Action::TextFont("Courier New")),
            ],
        ),
        Entry::Sub(
            "Size",
            &[
                item(
                    35230,
                    "Other...",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "a custom font size needs the size-entry dialog, which this build does not ship",
                    )),
                ),
                item(35635, "Smaller", "", Action::TextStep(-1)),
                item(35636, "Larger", "", Action::TextStep(1)),
                Entry::Sep,
                item(35196, "8", "", Action::TextSize(8.0)),
                item(35197, "9", "", Action::TextSize(9.0)),
                item(35198, "10", "", Action::TextSize(10.0)),
                item(35047, "12", "", Action::TextSize(12.0)),
                item(35048, "14", "", Action::TextSize(14.0)),
                item(35049, "18", "", Action::TextSize(18.0)),
                item(35050, "24", "", Action::TextSize(24.0)),
                item(35051, "36", "", Action::TextSize(36.0)),
                item(35052, "48", "", Action::TextSize(48.0)),
                item(35053, "72", "", Action::TextSize(72.0)),
                item(35054, "96", "", Action::TextSize(96.0)),
                item(35141, "120", "", Action::TextSize(120.0)),
            ],
        ),
        Entry::Sub(
            "Style",
            &[
                item(35057, "Plain", "Ctrl+Alt+Shift+P", Action::TextPlain),
                Entry::Sep,
                item(
                    35055,
                    "Faux Bold",
                    "Ctrl+Alt+Shift+B",
                    Action::TextFlag("bold"),
                ),
                item(
                    35056,
                    "Faux Italic",
                    "Ctrl+Alt+Shift+I",
                    Action::TextFlag("italic"),
                ),
                item(
                    35320,
                    "Underline",
                    "Ctrl+Alt+Shift+U",
                    Action::TextFlag("underline"),
                ),
            ],
        ),
        Entry::Sub(
            "Align",
            &[
                item(35044, "Left", "Ctrl+Alt+Shift+L", Action::TextJustify(0)),
                item(
                    35042,
                    "Centered Horizontally",
                    "Ctrl+Alt+Shift+C",
                    Action::TextJustify(1),
                ),
                item(35045, "Right", "Ctrl+Alt+Shift+R", Action::TextJustify(2)),
                item(
                    35043,
                    "Justified",
                    "Ctrl+Alt+Shift+J",
                    Action::TextJustify(3),
                ),
                Entry::Sep,
                item(
                    35114,
                    "Top",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "vertical text alignment needs an engine verb the format does not yet document",
                    )),
                ),
                item(
                    35115,
                    "Centered Vertically",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "vertical text alignment needs an engine verb the format does not yet document",
                    )),
                ),
                item(
                    35116,
                    "Bottom",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "vertical text alignment needs an engine verb the format does not yet document",
                    )),
                ),
                item(
                    35117,
                    "Justified Vertically",
                    "",
                    Action::Shell(ShellAction::ReportGap(
                        "vertical text alignment needs an engine verb the format does not yet document",
                    )),
                ),
            ],
        ),
        Entry::Sep,
        item(
            49005,
            "Attach In Path",
            "",
            Action::Sel(SelAction::AttachToPath),
        ),
        item(
            35118,
            "Attach to Path",
            "Ctrl+Shift+Y",
            Action::Sel(SelAction::AttachToPath),
        ),
        item(
            35119,
            "Detach from Path",
            "",
            Action::Sel(SelAction::DetachFromPath),
        ),
        Entry::Sub(
            "Orientation",
            &[
                item(35120, "Rotate Around Path", "", Action::TextOnPathMode(0)),
                item(35121, "Vertical", "", Action::TextOnPathMode(1)),
                item(35122, "Skew Vertical", "", Action::TextOnPathMode(2)),
                item(35123, "Skew Horizontal", "", Action::TextOnPathMode(3)),
            ],
        ),
        item(
            35124,
            "Reverse Direction",
            "",
            Action::Sel(SelAction::ReversePathText),
        ),
        Entry::Sep,
        item(
            35125,
            "Convert to Paths",
            "Ctrl+Shift+P",
            Action::Sel(SelAction::ConvertToPaths),
        ),
        Entry::Sep,
        item(
            35591,
            "Check Spelling...",
            "",
            Action::Shell(ShellAction::ReportGap(
                "Check Spelling needs the spelling engine, which this build does not ship",
            )),
        ),
        item(
            35594,
            "Spelling Setup...",
            "",
            Action::Shell(ShellAction::ReportGap(
                "Spelling Setup needs the spelling engine, which this build does not ship",
            )),
        ),
    ],
};

/// Commands — the CS6 automation surface. `Run Script...` (35319)
/// and `Manage Extensions...` (35590) both dispatch to
/// `ShellAction::ManageExtensions`, whose shell side *is* the script
/// runner (`shell::open_extension_manager` → `run_script_file`).
/// Only `Manage Saved Commands...` (35386) is inert: CS6's
/// saved-command manager has no counterpart yet (recorded gap, and
/// the reason `inert_count_never_rises` pins the census at 2).
pub static COMMANDS: Menu = Menu {
    label: "Commands",
    entries: &[
        item(35386, "Manage Saved Commands...", "", Action::Inert),
        item(
            35590,
            "Manage Extensions...",
            "",
            Action::Shell(ShellAction::ManageExtensions),
        ),
        item(
            35319,
            "Run Script...",
            "",
            Action::Shell(ShellAction::ManageExtensions),
        ),
        Entry::Sep,
    ],
};

/// Filters — the static resource carries only Repeat Filter; CS6
/// builds the effect tree at runtime from its filter registry. The
/// submenus below are the same preset table that drives the Live
/// Filters panel (`shell::FILTER_FAMILIES`, pinned equal by test);
/// runtime-built items carry no resource id (the popups' own 0
/// convention), and the dispatch resolves each name through
/// `shell::filter_preset`.
pub static FILTERS: Menu = Menu {
    label: "Filters",
    entries: &[
        item(35035, "Repeat Filter", "", Action::RepeatFilter),
        Entry::Sep,
        Entry::Sub(
            "Adjust Color",
            &[
                item(0, "Invert", "", Action::Filter("invert")),
                item(
                    0,
                    "Brightness/Contrast",
                    "",
                    Action::Filter("brightnessContrast"),
                ),
                item(0, "Levels", "", Action::Filter("levels")),
                item(0, "Curves", "", Action::Filter("curves")),
                item(0, "Hue/Saturation", "", Action::Filter("hueSaturation")),
                item(0, "Auto Levels", "", Action::Filter("autoLevels")),
                item(0, "Color Fill", "", Action::Filter("colorFill")),
                item(0, "Convert to Alpha", "", Action::Filter("convertToAlpha")),
            ],
        ),
        Entry::Sub(
            "Blur",
            &[
                item(0, "Blur", "", Action::Filter("blur")),
                item(0, "Blur More", "", Action::Filter("blurMore")),
                item(0, "Gaussian Blur", "", Action::Filter("gaussianBlur")),
                item(0, "Motion Blur", "", Action::Filter("motionBlur")),
                item(0, "Radial Blur", "", Action::Filter("radialBlur")),
                item(0, "Zoom Blur", "", Action::Filter("zoomBlur")),
            ],
        ),
        Entry::Sub(
            "Sharpen",
            &[
                item(0, "Sharpen", "", Action::Filter("sharpen")),
                item(0, "Sharpen More", "", Action::Filter("sharpenMore")),
                item(0, "Unsharp Mask", "", Action::Filter("unsharpMask")),
            ],
        ),
        Entry::Sub(
            "Stylize",
            &[
                item(0, "Add Noise", "", Action::Filter("addNoise")),
                item(0, "Find Edges", "", Action::Filter("findEdges")),
                item(0, "Clouds", "", Action::Filter("clouds")),
                item(
                    0,
                    "Difference Clouds",
                    "",
                    Action::Filter("differenceClouds"),
                ),
            ],
        ),
        Entry::Sep,
        Entry::Sub(
            "Shadow and Glow",
            &[
                item(0, "Drop Shadow", "", Action::Filter("dropShadow")),
                item(0, "Inner Shadow", "", Action::Filter("innerShadow")),
                item(0, "Solid Shadow", "", Action::Filter("solidShadow")),
                item(0, "Inner Glow", "", Action::Filter("innerGlow")),
                item(0, "Glow", "", Action::Filter("glow")),
            ],
        ),
        Entry::Sub(
            "Bevel and Emboss",
            &[
                item(0, "Inner Bevel", "", Action::Filter("innerBevel")),
                item(0, "Outer Bevel", "", Action::Filter("outerBevel")),
                item(0, "Raised Emboss", "", Action::Filter("emboss")),
                item(0, "Inset Emboss", "", Action::Filter("pillowEmboss")),
            ],
        ),
        Entry::Sep,
    ],
};

/// Window — the CS6 panel roster. Every name here is the authoritative
/// panel list; docked panels toggle their right-dock section, tools
/// that have no panel yet report so instead of rendering inert.
pub static WINDOW: Menu = Menu {
    label: "Window",
    entries: &[
        item(
            57648,
            "Duplicate Window",
            "",
            Action::Shell(ShellAction::SingleDocument),
        ),
        item(
            35298,
            "Hide Panels",
            "Tab",
            Action::Shell(ShellAction::ToggleHidePanels),
        ),
        Entry::Sep,
        Entry::Sub("Extensions", &[item(35848, " ", "", Action::Inert)]),
        Entry::Sep,
        Entry::Sub(
            "Toolbars",
            &[item(
                35059,
                "Main",
                "",
                Action::Shell(ShellAction::ToggleMainToolbar),
            )],
        ),
        item(
            35058,
            "Tools",
            "Ctrl+Alt+T",
            Action::Shell(ShellAction::ToggleTools),
        ),
        item(
            35586,
            "Properties",
            "",
            Action::Shell(ShellAction::TogglePanel("Properties")),
        ),
        Entry::Sep,
        item(
            43409,
            "Optimize",
            "",
            Action::Shell(ShellAction::TogglePanel("Optimize")),
        ),
        item(
            43412,
            "Layers",
            "Ctrl+Alt+L",
            Action::Shell(ShellAction::TogglePanel("Layers")),
        ),
        Entry::Sep,
        item(
            45816,
            "Common Library",
            "",
            Action::Shell(ShellAction::TogglePanel("Common Library")),
        ),
        Entry::Sep,
        item(
            49125,
            "CSS Properties",
            "",
            Action::Shell(ShellAction::TogglePanel("CSS Properties")),
        ),
        Entry::Sep,
        item(
            35798,
            "Pages",
            "Ctrl+Alt+O",
            Action::Shell(ShellAction::TogglePanel("Pages")),
        ),
        item(
            43410,
            "States",
            "Ctrl+Alt+K",
            Action::Shell(ShellAction::TogglePanel("States")),
        ),
        item(
            43418,
            "History",
            "",
            Action::Shell(ShellAction::TogglePanel("History")),
        ),
        Entry::Sep,
        item(
            43422,
            "Auto Shapes",
            "",
            Action::Shell(ShellAction::TogglePanel("Auto Shapes")),
        ),
        item(
            43415,
            "Styles",
            "Ctrl+Alt+J",
            Action::Shell(ShellAction::TogglePanel("Styles")),
        ),
        item(
            43416,
            "Document Library",
            "",
            Action::Shell(ShellAction::TogglePanel("Document Library")),
        ),
        item(
            43414,
            "URL",
            "Ctrl+Alt+U",
            Action::Shell(ShellAction::TogglePanel("URL")),
        ),
        Entry::Sep,
        item(
            43405,
            "Color Mixer",
            "Ctrl+Alt+M",
            Action::Shell(ShellAction::TogglePanel("Color Mixer")),
        ),
        item(
            43408,
            "Swatches",
            "Ctrl+Alt+S",
            Action::Shell(ShellAction::TogglePanel("Swatches")),
        ),
        Entry::Sep,
        item(
            43404,
            "Info",
            "Ctrl+Alt+I",
            Action::Shell(ShellAction::TogglePanel("Info")),
        ),
        item(
            43413,
            "Behaviors",
            "Ctrl+Alt+H",
            Action::Shell(ShellAction::TogglePanel("Behaviors")),
        ),
        item(
            43402,
            "Find and Replace",
            "",
            Action::Shell(ShellAction::TogglePanel("Find and Replace")),
        ),
        // FireCraft's own web panels. CS6 has no Window-menu row for
        // either of them — CS6 manages slices in the Properties panel
        // — so these two ids are FireCraft-local, not CS6 resource
        // ids.
        item(
            43403,
            "Slices",
            "",
            Action::Shell(ShellAction::TogglePanel("Slices")),
        ),
        item(
            43406,
            "HTML Preview",
            "",
            Action::Shell(ShellAction::TogglePanel("HTML Preview")),
        ),
        Entry::Sep,
        item(
            57650,
            "Cascade",
            "",
            Action::Shell(ShellAction::SingleDocument),
        ),
        item(
            57651,
            "Tile Horizontally",
            "",
            Action::Shell(ShellAction::SingleDocument),
        ),
        item(
            57652,
            "Tile Vertically",
            "",
            Action::Shell(ShellAction::SingleDocument),
        ),
    ],
};

/// Help.
pub static HELP: Menu = Menu {
    label: "Help",
    entries: &[
        item(
            57667,
            "Online Help",
            "",
            Action::Shell(ShellAction::OnlineHelp),
        ),
        item(
            0,
            "Keyboard Shortcuts\tF1",
            "",
            Action::Shell(ShellAction::ShowShortcuts),
        ),
        Entry::Sep,
        item(
            35637,
            "Manage Extensions...",
            "",
            Action::Shell(ShellAction::ManageExtensions),
        ),
        Entry::Sep,
        item(
            57664,
            "About FireCraft...",
            "",
            Action::Shell(ShellAction::About),
        ),
    ],
};

/// The CS6 menu bar, in order: File, Edit, View, Select, Modify, Text,
/// Commands, Filters, Window, Help.
pub static MENUBAR: &[Menu] = &[
    FILE, EDIT, VIEW, SELECT, MODIFY, TEXT, COMMANDS, FILTERS, WINDOW, HELP,
];

/// One shortcut modifier triple.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

/// Ctrl.
pub const CTRL: Mods = Mods {
    ctrl: true,
    alt: false,
    shift: false,
};
/// Ctrl+Shift.
pub const CTRL_SHIFT: Mods = Mods {
    ctrl: true,
    alt: false,
    shift: true,
};
/// Ctrl+Alt.
pub const CTRL_ALT: Mods = Mods {
    ctrl: true,
    alt: true,
    shift: false,
};
/// Ctrl+Alt+Shift.
pub const CTRL_ALT_SHIFT: Mods = Mods {
    ctrl: true,
    alt: true,
    shift: true,
};
/// Alt.
pub const ALT: Mods = Mods {
    ctrl: false,
    alt: true,
    shift: false,
};
/// Shift only — the XML's mod 2 rows (Shift+L, Shift+F).
pub const SHIFT: Mods = Mods {
    ctrl: false,
    alt: false,
    shift: true,
};
/// Alt+Shift.
pub const ALT_SHIFT: Mods = Mods {
    ctrl: false,
    alt: true,
    shift: true,
};
/// No modifiers.
pub const NONE: Mods = Mods {
    ctrl: false,
    alt: false,
    shift: false,
};

/// One bound key.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shortcut {
    /// The section heading this binding is filed under in the help
    /// panel — the CS6 menu it belongs to.
    pub group: &'static str,
    pub mods: Mods,
    pub key: egui::Key,
    pub action: Action,
}

const fn key(group: &'static str, mods: Mods, k: egui::Key, action: Action) -> Shortcut {
    Shortcut {
        group,
        mods,
        key: k,
        action,
    }
}

/// Parse a CS6 accelerator string (`"Ctrl+Alt+Shift+F"`, `"F12"`,
/// `"Ctrl+]`) into the modifiers and key the binding table uses.
/// Returns `None` for text that is not an accelerator.
///
/// The accelerator vocabulary is the one the CS6 menu bar prints:
/// the modifiers `Ctrl`, `Alt` and `Shift` in any order, letters,
/// digits, `F1`–`F12`, and the handful of named or symbolic keys
/// CS6 uses (`Del`, `Tab`, `'` `]` `;` `\` `=` `-`, `↑` `↓`).
#[must_use]
pub fn parse_accel(text: &str) -> Option<(Mods, egui::Key)> {
    let mut mods = Mods {
        ctrl: false,
        alt: false,
        shift: false,
    };
    let mut key = None;
    for part in text.split('+') {
        match part {
            "Ctrl" => mods.ctrl = true,
            "Alt" => mods.alt = true,
            "Shift" => mods.shift = true,
            other => {
                // Two key tokens is not an accelerator.
                if key.is_some() {
                    return None;
                }
                key = Some(accel_key(other)?);
            }
        }
    }
    key.map(|k| (mods, k))
}

/// The key half of an accelerator, once the modifiers are split off.
fn accel_key(text: &str) -> Option<egui::Key> {
    match text {
        "Del" => return Some(egui::Key::Delete),
        "↑" => return Some(egui::Key::ArrowUp),
        "↓" => return Some(egui::Key::ArrowDown),
        _ => {}
    }
    // The printed text carries the backslash raw; accept the escaped
    // spelling too, so a hand-written accelerator still parses.
    if !text.is_empty() && text.chars().all(|c| c == '\\') {
        return Some(egui::Key::Backslash);
    }
    egui::Key::from_name(text)
}

/// The Fireworks.xml keymap bindings this shell implements. Tool
/// letters are the CS6 one-key tool selects; plain keys only fire when
/// no text field has keyboard focus.
///
/// The table is ordered by section: every entry carries the heading it
/// is filed under, and the help panel prints a heading whenever that
/// heading changes. Each binding matches the accelerator the CS6 menu
/// bar prints for the same row — `cs6_keymap_is_fully_bound` holds the
/// two in step.
pub static SHORTCUTS: &[Shortcut] = &[
    // File
    key("File", CTRL, egui::Key::N, Action::Shell(ShellAction::New)),
    key("File", CTRL, egui::Key::O, Action::Shell(ShellAction::Open)),
    key(
        "File",
        CTRL,
        egui::Key::F4,
        Action::Shell(ShellAction::CloseDocument),
    ),
    key("File", CTRL, egui::Key::S, Action::Shell(ShellAction::Save)),
    key(
        "File",
        CTRL_SHIFT,
        egui::Key::S,
        Action::Shell(ShellAction::SaveAs),
    ),
    key(
        "File",
        CTRL,
        egui::Key::R,
        Action::Shell(ShellAction::Import),
    ),
    key(
        "File",
        CTRL_SHIFT,
        egui::Key::R,
        Action::Shell(ShellAction::Export),
    ),
    key(
        "File",
        CTRL_SHIFT,
        egui::Key::X,
        Action::Shell(ShellAction::ExportPreview),
    ),
    key(
        "File",
        CTRL_ALT_SHIFT,
        egui::Key::F,
        Action::Shell(ShellAction::FileInfo),
    ),
    key(
        "File",
        NONE,
        egui::Key::F12,
        Action::Shell(ShellAction::PreviewInBrowser {
            secondary: false,
            all_pages: false,
        }),
    ),
    key(
        "File",
        SHIFT,
        egui::Key::F12,
        Action::Shell(ShellAction::PreviewInBrowser {
            secondary: true,
            all_pages: false,
        }),
    ),
    key(
        "File",
        CTRL,
        egui::Key::F12,
        Action::Shell(ShellAction::PreviewInBrowser {
            secondary: false,
            all_pages: true,
        }),
    ),
    key("File", ALT, egui::Key::F4, Action::Shell(ShellAction::Quit)),
    key("File", CTRL, egui::Key::Q, Action::Shell(ShellAction::Quit)),
    key(
        "File",
        CTRL_SHIFT,
        egui::Key::O,
        Action::Shell(ShellAction::ReportGap(
            "Adobe Bridge is not available in this build.",
        )),
    ),
    key(
        "File",
        CTRL,
        egui::Key::P,
        Action::Shell(ShellAction::Palette),
    ),
    // Edit
    key("Edit", CTRL, egui::Key::Z, Action::Shell(ShellAction::Undo)),
    key(
        "Edit",
        CTRL_SHIFT,
        egui::Key::Z,
        Action::Shell(ShellAction::Redo),
    ),
    key(
        "Edit",
        CTRL_ALT,
        egui::Key::D,
        Action::Sel(SelAction::Duplicate),
    ),
    key(
        "Edit",
        CTRL_SHIFT,
        egui::Key::C,
        Action::Sel(SelAction::Clone),
    ),
    key(
        "Edit",
        NONE,
        egui::Key::Delete,
        Action::Sel(SelAction::Delete),
    ),
    key(
        "Edit",
        CTRL,
        egui::Key::A,
        Action::Shell(ShellAction::SelectAll),
    ),
    key(
        "Edit",
        CTRL,
        egui::Key::D,
        Action::Shell(ShellAction::Deselect),
    ),
    key("Edit", CTRL, egui::Key::M, Action::Sel(SelAction::Hide)),
    key(
        "Edit",
        NONE,
        egui::Key::Backspace,
        Action::Sel(SelAction::Delete),
    ),
    // Edit ▸ insert
    key(
        "Edit ▸ insert",
        CTRL,
        egui::Key::F8,
        Action::Sel(SelAction::NewSymbol),
    ),
    key(
        "Edit ▸ insert",
        CTRL_SHIFT,
        egui::Key::U,
        Action::Shell(ShellAction::InsertWeb {
            what: InsertWebKind::Hotspot,
        }),
    ),
    key(
        "Edit ▸ insert",
        ALT_SHIFT,
        egui::Key::P,
        Action::Shell(ShellAction::InsertWeb {
            what: InsertWebKind::PolygonSlice,
        }),
    ),
    key(
        "Edit ▸ insert",
        CTRL_ALT,
        egui::Key::Y,
        Action::Sel(SelAction::EmptyBitmap),
    ),
    key(
        "Edit ▸ insert",
        SHIFT,
        egui::Key::L,
        Action::Sel(SelAction::NewLayer),
    ),
    key(
        "Edit ▸ insert",
        SHIFT,
        egui::Key::F,
        Action::Sel(SelAction::NewFrame),
    ),
    // Clipboard
    key(
        "Clipboard",
        CTRL,
        egui::Key::C,
        Action::Shell(ShellAction::Copy),
    ),
    key(
        "Clipboard",
        CTRL,
        egui::Key::X,
        Action::Shell(ShellAction::Cut),
    ),
    key(
        "Clipboard",
        CTRL,
        egui::Key::V,
        Action::Shell(ShellAction::Paste),
    ),
    key(
        "Clipboard",
        CTRL_ALT,
        egui::Key::V,
        Action::Shell(ShellAction::PasteAsMask),
    ),
    key(
        "Clipboard",
        CTRL_SHIFT,
        egui::Key::V,
        Action::Shell(ShellAction::PasteInside),
    ),
    // Select
    key(
        "Select",
        CTRL,
        egui::Key::ArrowUp,
        Action::Sel(SelAction::SelectParents),
    ),
    key(
        "Select",
        CTRL,
        egui::Key::ArrowDown,
        Action::Sel(SelAction::SelectChildren),
    ),
    key(
        "Select",
        CTRL_SHIFT,
        egui::Key::I,
        Action::Shell(ShellAction::SelectInverse),
    ),
    key(
        "Select",
        CTRL_ALT_SHIFT,
        egui::Key::H,
        Action::Sel(SelAction::SelectParents),
    ),
    // Modify ▸ Group
    key(
        "Modify ▸ Group",
        CTRL,
        egui::Key::G,
        Action::Sel(SelAction::Group),
    ),
    key(
        "Modify ▸ Group",
        CTRL,
        egui::Key::U,
        Action::Sel(SelAction::Ungroup),
    ),
    key(
        "Modify ▸ Group",
        CTRL_ALT,
        egui::Key::E,
        Action::Sel(SelAction::MergeDown),
    ),
    // Modify ▸ snap
    key(
        "Modify ▸ snap",
        CTRL,
        egui::Key::K,
        Action::Shell(ShellAction::ReportGap(
            "Snap To Pixel needs a view snap mode, which this build does not ship",
        )),
    ),
    // Modify ▸ Symbol
    key(
        "Modify ▸ Symbol",
        NONE,
        egui::Key::F8,
        Action::Sel(SelAction::ConvertToSymbol),
    ),
    key(
        "Modify ▸ Symbol",
        CTRL_ALT_SHIFT,
        egui::Key::T,
        Action::Sel(SelAction::TweenInstances),
    ),
    // Modify ▸ Mask
    key(
        "Modify ▸ Mask",
        CTRL_SHIFT,
        egui::Key::K,
        Action::Sel(SelAction::ToggleMask),
    ),
    // Modify ▸ selective JPEG
    key(
        "Modify ▸ selective JPEG",
        CTRL_ALT_SHIFT,
        egui::Key::Z,
        Action::Sel(SelAction::FlattenSelection),
    ),
    key(
        "Modify ▸ selective JPEG",
        CTRL_ALT_SHIFT,
        egui::Key::E,
        Action::Sel(SelAction::FlattenLayers),
    ),
    // Modify ▸ Transform
    key(
        "Modify ▸ Transform",
        CTRL,
        egui::Key::T,
        Action::Shell(ShellAction::TransformMode(TransformMode::Scale)),
    ),
    key(
        "Modify ▸ Transform",
        CTRL_SHIFT,
        egui::Key::T,
        Action::Shell(ShellAction::NumericTransform),
    ),
    key(
        "Modify ▸ Transform",
        CTRL,
        egui::Key::Num9,
        Action::Sel(SelAction::Rotate(90.0)),
    ),
    key(
        "Modify ▸ Transform",
        CTRL,
        egui::Key::Num7,
        Action::Sel(SelAction::Rotate(-90.0)),
    ),
    // Modify ▸ Arrange
    key(
        "Modify ▸ Arrange",
        CTRL,
        egui::Key::F,
        Action::Arrange(Arrange::Front),
    ),
    key(
        "Modify ▸ Arrange",
        CTRL_SHIFT,
        egui::Key::F,
        Action::Arrange(Arrange::Forward),
    ),
    key(
        "Modify ▸ Arrange",
        CTRL_SHIFT,
        egui::Key::B,
        Action::Arrange(Arrange::Backward),
    ),
    key(
        "Modify ▸ Arrange",
        CTRL,
        egui::Key::B,
        Action::Arrange(Arrange::Back),
    ),
    // Modify ▸ align
    key(
        "Modify ▸ align",
        CTRL_ALT,
        egui::Key::Num1,
        Action::Align("left"),
    ),
    key(
        "Modify ▸ align",
        CTRL_ALT,
        egui::Key::Num2,
        Action::Align("centerX"),
    ),
    key(
        "Modify ▸ align",
        CTRL_ALT,
        egui::Key::Num3,
        Action::Align("right"),
    ),
    key(
        "Modify ▸ align",
        CTRL_ALT,
        egui::Key::Num4,
        Action::Align("top"),
    ),
    key(
        "Modify ▸ align",
        CTRL_ALT,
        egui::Key::Num5,
        Action::Align("centerY"),
    ),
    key(
        "Modify ▸ align",
        CTRL_ALT,
        egui::Key::Num6,
        Action::Align("bottom"),
    ),
    // Modify ▸ distribute
    key(
        "Modify ▸ distribute",
        CTRL_ALT,
        egui::Key::Num7,
        Action::Distribute("horizontal"),
    ),
    key(
        "Modify ▸ distribute",
        CTRL_ALT,
        egui::Key::Num9,
        Action::Distribute("vertical"),
    ),
    // Modify ▸ Combine Paths
    key(
        "Modify ▸ Combine Paths",
        CTRL,
        egui::Key::J,
        Action::Boolean("union"),
    ),
    key(
        "Modify ▸ Combine Paths",
        CTRL_SHIFT,
        egui::Key::J,
        Action::Sel(SelAction::SplitPaths),
    ),
    // View
    key(
        "View",
        CTRL,
        egui::Key::Equals,
        Action::Shell(ShellAction::ZoomIn),
    ),
    key(
        "View",
        CTRL,
        egui::Key::Minus,
        Action::Shell(ShellAction::ZoomOut),
    ),
    key(
        "View",
        CTRL_ALT,
        egui::Key::Num0,
        Action::Shell(ShellAction::FitAll),
    ),
    key(
        "View",
        CTRL,
        egui::Key::Num0,
        Action::Shell(ShellAction::FitSelection),
    ),
    key(
        "View",
        CTRL,
        egui::Key::CloseBracket,
        Action::Shell(ShellAction::ToggleTooltips),
    ),
    key(
        "View",
        CTRL_SHIFT,
        egui::Key::M,
        Action::Shell(ShellAction::ShowAllHidden),
    ),
    key(
        "View",
        CTRL_ALT,
        egui::Key::R,
        Action::Shell(ShellAction::ToggleRulers),
    ),
    // View ▸ grid
    key(
        "View ▸ grid",
        CTRL,
        egui::Key::Quote,
        Action::Shell(ShellAction::ToggleGrid),
    ),
    key(
        "View ▸ grid",
        CTRL_SHIFT,
        egui::Key::Quote,
        Action::Shell(ShellAction::ToggleSnapToGrid),
    ),
    key(
        "View ▸ grid",
        CTRL_ALT,
        egui::Key::G,
        Action::Shell(ShellAction::EditGrid),
    ),
    // View ▸ guides
    key(
        "View ▸ guides",
        CTRL,
        egui::Key::Semicolon,
        Action::Shell(ShellAction::ToggleGuides),
    ),
    key(
        "View ▸ guides",
        CTRL_ALT,
        egui::Key::Semicolon,
        Action::Shell(ShellAction::ToggleGuidesLocked),
    ),
    key(
        "View ▸ guides",
        CTRL_SHIFT,
        egui::Key::Semicolon,
        Action::Shell(ShellAction::ToggleSnapToGuides),
    ),
    // CS6's keymap file opens a guides editor here (`cmd_ViewEditGuides`);
    // this build edits guides from the Properties panel instead.
    key(
        "View ▸ guides",
        CTRL_ALT_SHIFT,
        egui::Key::G,
        Action::Shell(ShellAction::ReportGap(
            "CS6's View Edit Guides dialog is not in this build",
        )),
    ),
    // View ▸ smart guides
    key(
        "View ▸ smart guides",
        CTRL,
        egui::Key::Backslash,
        Action::Shell(ShellAction::ToggleSmartGuides),
    ),
    key(
        "View ▸ smart guides",
        CTRL_SHIFT,
        egui::Key::Backslash,
        Action::Shell(ShellAction::ToggleSnapToSmartGuides),
    ),
    // View ▸ 9-slice guides
    key(
        "View ▸ 9-slice guides",
        CTRL_ALT_SHIFT,
        egui::Key::Semicolon,
        Action::Shell(ShellAction::ToggleSliceGuides),
    ),
    key(
        "View ▸ 9-slice guides",
        CTRL,
        egui::Key::H,
        Action::Shell(ShellAction::ToggleEdges),
    ),
    // Text styles
    key(
        "Text styles",
        CTRL_ALT_SHIFT,
        egui::Key::P,
        Action::TextPlain,
    ),
    key(
        "Text styles",
        CTRL_ALT_SHIFT,
        egui::Key::B,
        Action::TextFlag("bold"),
    ),
    key(
        "Text styles",
        CTRL_ALT_SHIFT,
        egui::Key::I,
        Action::TextFlag("italic"),
    ),
    key(
        "Text styles",
        CTRL_ALT_SHIFT,
        egui::Key::U,
        Action::TextFlag("underline"),
    ),
    // CS6's own keymap file gives each text style a function-key
    // alternate alongside the Ctrl+Alt+Shift chord (`cmd_TextStyle_*`).
    key("Text styles", NONE, egui::Key::F5, Action::TextPlain),
    key("Text styles", NONE, egui::Key::F6, Action::TextFlag("bold")),
    key(
        "Text styles",
        NONE,
        egui::Key::F7,
        Action::TextFlag("italic"),
    ),
    // Text ▸ justify
    key(
        "Text ▸ justify",
        CTRL_ALT_SHIFT,
        egui::Key::L,
        Action::TextJustify(0),
    ),
    key(
        "Text ▸ justify",
        CTRL_ALT_SHIFT,
        egui::Key::C,
        Action::TextJustify(1),
    ),
    key(
        "Text ▸ justify",
        CTRL_ALT_SHIFT,
        egui::Key::R,
        Action::TextJustify(2),
    ),
    key(
        "Text ▸ justify",
        CTRL_ALT_SHIFT,
        egui::Key::J,
        Action::TextJustify(3),
    ),
    // Text
    key(
        "Text",
        CTRL_SHIFT,
        egui::Key::Y,
        Action::Sel(SelAction::AttachToPath),
    ),
    key(
        "Text",
        CTRL_SHIFT,
        egui::Key::P,
        Action::Sel(SelAction::ConvertToPaths),
    ),
    // Tools (one-key selects)
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::V,
        Action::Shell(ShellAction::PickToolGroup(Tool::Pointer)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::A,
        Action::Shell(ShellAction::PickToolGroup(Tool::Subselect)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::Q,
        Action::Shell(ShellAction::PickToolGroup(Tool::Scale)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::C,
        Action::Shell(ShellAction::PickToolGroup(Tool::Crop)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::M,
        Action::Shell(ShellAction::PickToolGroup(Tool::Marquee)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::L,
        Action::Shell(ShellAction::PickToolGroup(Tool::Lasso)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::W,
        Action::Shell(ShellAction::PickToolGroup(Tool::MagicWand)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::B,
        Action::Shell(ShellAction::PickToolGroup(Tool::Paintbrush)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::E,
        Action::Shell(ShellAction::PickToolGroup(Tool::Eraser)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::R,
        Action::Shell(ShellAction::PickToolGroup(Tool::Blur)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::D,
        Action::Shell(ShellAction::PickToolGroup(Tool::Dodge)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::S,
        Action::Shell(ShellAction::PickToolGroup(Tool::RubberStamp)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::N,
        Action::Shell(ShellAction::PickToolGroup(Tool::Line)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::P,
        Action::Shell(ShellAction::PickToolGroup(Tool::Pen)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::U,
        Action::Shell(ShellAction::PickToolGroup(Tool::Rect)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::T,
        Action::Shell(ShellAction::PickToolGroup(Tool::Text)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::O,
        Action::Shell(ShellAction::PickToolGroup(Tool::Freeform)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::Y,
        Action::Shell(ShellAction::PickToolGroup(Tool::Knife)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::J,
        Action::Shell(ShellAction::PickToolGroup(Tool::Hotspot)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::K,
        Action::Shell(ShellAction::PickToolGroup(Tool::Slice)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::Num2,
        Action::Shell(ShellAction::PickToolGroup(Tool::HideShowSlices)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::I,
        Action::Shell(ShellAction::PickToolGroup(Tool::Eyedropper)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::G,
        Action::Shell(ShellAction::PickToolGroup(Tool::PaintBucket)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::H,
        Action::Shell(ShellAction::PickToolGroup(Tool::Hand)),
    ),
    key(
        "Tools (one-key selects)",
        NONE,
        egui::Key::Z,
        Action::Shell(ShellAction::PickToolGroup(Tool::Zoom)),
    ),
    // Window ▸ panel toggles
    key(
        "Window ▸ panel toggles",
        NONE,
        egui::Key::Tab,
        Action::Shell(ShellAction::ToggleHidePanels),
    ),
    // CS6's keymap file gives "Hide or show all panels" a second
    // accelerator (`cmd_HideOrShowAllPanels`).
    key(
        "Window ▸ panel toggles",
        CTRL_SHIFT,
        egui::Key::H,
        Action::Shell(ShellAction::ToggleHidePanels),
    ),
    key(
        "Window ▸ panel toggles",
        CTRL,
        egui::Key::F7,
        Action::Shell(ShellAction::TogglePanel("CSS Properties")),
    ),
    key(
        "Window ▸ panel toggles",
        CTRL_SHIFT,
        egui::Key::L,
        Action::Shell(ShellAction::TogglePanel("Common Library")),
    ),
    key(
        "Window ▸ panel toggles",
        CTRL_ALT,
        egui::Key::T,
        Action::Shell(ShellAction::ToggleTools),
    ),
    key(
        "Window ▸ panel toggles",
        CTRL_ALT,
        egui::Key::L,
        Action::Shell(ShellAction::TogglePanel("Layers")),
    ),
    key(
        "Window ▸ panel toggles",
        CTRL_ALT,
        egui::Key::O,
        Action::Shell(ShellAction::TogglePanel("Pages")),
    ),
    key(
        "Window ▸ panel toggles",
        CTRL_ALT,
        egui::Key::K,
        Action::Shell(ShellAction::TogglePanel("States")),
    ),
    key(
        "Window ▸ panel toggles",
        CTRL_ALT,
        egui::Key::J,
        Action::Shell(ShellAction::TogglePanel("Styles")),
    ),
    key(
        "Window ▸ panel toggles",
        CTRL_ALT,
        egui::Key::U,
        Action::Shell(ShellAction::TogglePanel("URL")),
    ),
    key(
        "Window ▸ panel toggles",
        CTRL_ALT,
        egui::Key::M,
        Action::Shell(ShellAction::TogglePanel("Color Mixer")),
    ),
    key(
        "Window ▸ panel toggles",
        CTRL_ALT,
        egui::Key::S,
        Action::Shell(ShellAction::TogglePanel("Swatches")),
    ),
    key(
        "Window ▸ panel toggles",
        CTRL_ALT,
        egui::Key::I,
        Action::Shell(ShellAction::TogglePanel("Info")),
    ),
    key(
        "Window ▸ panel toggles",
        CTRL_ALT,
        egui::Key::H,
        Action::Shell(ShellAction::TogglePanel("Behaviors")),
    ),
    // States navigation
    key(
        "States navigation",
        SHIFT,
        egui::Key::Comma,
        Action::Shell(ShellAction::SelectFrame(-1)),
    ),
    key(
        "States navigation",
        SHIFT,
        egui::Key::Period,
        Action::Shell(ShellAction::SelectFrame(1)),
    ),
    // Text ▸ Align
    key(
        "Text ▸ Align",
        CTRL_ALT_SHIFT,
        egui::Key::S,
        Action::TextFlag("stretch"),
    ),
    // Edit selection
    key(
        "Edit selection",
        CTRL_SHIFT,
        egui::Key::E,
        Action::Shell(ShellAction::EditTextSelection),
    ),
    // Pages navigation
    key(
        "Pages navigation",
        CTRL,
        egui::Key::PageUp,
        Action::Shell(ShellAction::SelectPage(-1)),
    ),
    key(
        "Pages navigation",
        CTRL,
        egui::Key::PageDown,
        Action::Shell(ShellAction::SelectPage(1)),
    ),
    // Bitmap edit
    key(
        "Bitmap edit",
        CTRL,
        egui::Key::E,
        Action::Shell(ShellAction::EditTextSelection),
    ),
    // Nudge
    key(
        "Nudge",
        NONE,
        egui::Key::ArrowLeft,
        Action::Sel(SelAction::Nudge { dx: -1.0, dy: 0.0 }),
    ),
    key(
        "Nudge",
        NONE,
        egui::Key::ArrowRight,
        Action::Sel(SelAction::Nudge { dx: 1.0, dy: 0.0 }),
    ),
    key(
        "Nudge",
        NONE,
        egui::Key::ArrowUp,
        Action::Sel(SelAction::Nudge { dx: 0.0, dy: -1.0 }),
    ),
    key(
        "Nudge",
        NONE,
        egui::Key::ArrowDown,
        Action::Sel(SelAction::Nudge { dx: 0.0, dy: 1.0 }),
    ),
    key(
        "Nudge",
        NONE,
        egui::Key::OpenBracket,
        Action::Sel(SelAction::Nudge { dx: -10.0, dy: 0.0 }),
    ),
    // CS6's big nudge is Shift with the arrow keys (`cmd_BigNudge*`);
    // the plain bracket above is FireCraft's own extra.
    key(
        "Nudge",
        SHIFT,
        egui::Key::ArrowLeft,
        Action::Sel(SelAction::Nudge { dx: -10.0, dy: 0.0 }),
    ),
    key(
        "Nudge",
        SHIFT,
        egui::Key::ArrowRight,
        Action::Sel(SelAction::Nudge { dx: 10.0, dy: 0.0 }),
    ),
    key(
        "Nudge",
        SHIFT,
        egui::Key::ArrowUp,
        Action::Sel(SelAction::Nudge { dx: 0.0, dy: -10.0 }),
    ),
    key(
        "Nudge",
        SHIFT,
        egui::Key::ArrowDown,
        Action::Sel(SelAction::Nudge { dx: 0.0, dy: 10.0 }),
    ),
    // Guides and grid
    key(
        "Guides and grid",
        CTRL,
        egui::Key::Comma,
        Action::Shell(ShellAction::ToggleGrid),
    ),
    key(
        "Guides and grid",
        CTRL,
        egui::Key::Backtick,
        Action::Shell(ShellAction::ToggleSmartGuides),
    ),
    key(
        "Guides and grid",
        CTRL_SHIFT,
        egui::Key::Comma,
        Action::Shell(ShellAction::ToggleSnapToGrid),
    ),
    key(
        "Guides and grid",
        CTRL,
        egui::Key::OpenBracket,
        Action::Shell(ShellAction::ToggleTooltips),
    ),
    // Fit and close
    key(
        "Fit and close",
        CTRL,
        egui::Key::W,
        Action::Shell(ShellAction::CloseDocument),
    ),
    // Paste variants
    key(
        "Paste variants",
        CTRL_ALT_SHIFT,
        egui::Key::V,
        Action::Shell(ShellAction::ReportGap(
            "Paste Attributes (no attribute clipboard in this build)",
        )),
    ),
    // Mask group and paths
    key(
        "Mask group and paths",
        CTRL_SHIFT,
        egui::Key::G,
        Action::Sel(SelAction::AddMask {
            mode: "reveal selection",
        }),
    ),
    // Insert
    key(
        "Insert",
        CTRL_ALT,
        egui::Key::C,
        Action::Sel(SelAction::CropDocument),
    ),
    // CS6's jQuery widget panel (`cmd_jquery`).
    key(
        "Insert",
        CTRL,
        egui::Key::F10,
        Action::Shell(ShellAction::ReportGap(
            "CS6's jQuery widget panel is not in this build",
        )),
    ),
    // Filters
    key(
        "Filters",
        CTRL_ALT_SHIFT,
        egui::Key::X,
        Action::RepeatFilter,
    ),
    // Layers and states
    key(
        "Layers and states",
        CTRL_SHIFT,
        egui::Key::D,
        Action::Shell(ShellAction::Deselect),
    ),
    // Selection history
    key(
        "Selection history",
        CTRL_SHIFT,
        egui::Key::ArrowLeft,
        Action::Shell(ShellAction::ReportGap(
            "Previous Selection (selection history not in this build)",
        )),
    ),
    key(
        "Selection history",
        CTRL_SHIFT,
        egui::Key::ArrowRight,
        Action::Shell(ShellAction::ReportGap(
            "Next Selection (selection history not in this build)",
        )),
    ),
    // Help
    key(
        "Help",
        NONE,
        egui::Key::F1,
        Action::Shell(ShellAction::ShowShortcuts),
    ),
];

impl Shortcut {
    /// Human-readable binding, e.g. `Ctrl+Shift+S` or `Delete` —
    /// the form the help panel shows.
    #[must_use]
    pub fn label(&self) -> String {
        let mut s = String::new();
        if self.mods.ctrl {
            s.push_str("Ctrl+");
        }
        if self.mods.alt {
            s.push_str("Alt+");
        }
        if self.mods.shift {
            s.push_str("Shift+");
        }
        s.push_str(&key_name(self.key));
        s
    }
}

/// Display name for an [`egui::Key`] as printed on the keyboard.
fn key_name(key: egui::Key) -> String {
    use egui::Key as K;
    match key {
        K::Equals => "=".into(),
        K::Minus => "-".into(),
        K::CloseBracket => "]".into(),
        K::Semicolon => ";".into(),
        K::Quote => "'".into(),
        K::Backslash => "\\".into(),
        K::Num0 => "0".into(),
        K::ArrowDown => "↓".into(),
        K::ArrowLeft => "←".into(),
        K::ArrowRight => "→".into(),
        K::ArrowUp => "↑".into(),
        other => format!("{other:?}"),
    }
}

/// Short description of what a bound action does, for the help panel.
#[must_use]
pub fn action_name(action: Action) -> String {
    match action {
        Action::Inert => "unwired".into(),
        Action::Shell(shell) => match shell {
            ShellAction::New => "New document".into(),
            ShellAction::Open => "Open".into(),
            ShellAction::Import => "Import image".into(),
            ShellAction::Save => "Save".into(),
            ShellAction::SaveAs => "Save As".into(),
            ShellAction::Export => "Export".into(),
            ShellAction::ExportPreview => "Export preview".into(),
            ShellAction::ExportWizard => "Export wizard".into(),
            ShellAction::ExportJpeg => "JPEG compressed export".into(),
            ShellAction::FileInfo => "File info".into(),
            ShellAction::PngFormatInfo => "Fireworks PNG format".into(),
            ShellAction::CloseDocument => "Close document".into(),
            ShellAction::SaveAll => "Save all".into(),
            ShellAction::Revert => "Revert".into(),
            ShellAction::NumericTransform => "Numeric Transform".into(),
            ShellAction::TransformMode(TransformMode::Scale) => "Free Transform".into(),
            ShellAction::TransformMode(TransformMode::Distort) => "Distort".into(),
            ShellAction::Palette => "Command palette".into(),
            ShellAction::Quit => "Quit".into(),
            ShellAction::Undo => "Undo".into(),
            ShellAction::Redo => "Redo".into(),
            ShellAction::Deselect => "Deselect".into(),
            ShellAction::SelectAll => "Select all".into(),
            ShellAction::SelectInverse => "Select inverse".into(),
            ShellAction::Copy => "Copy".into(),
            ShellAction::Cut => "Cut".into(),
            ShellAction::Paste => "Paste".into(),
            ShellAction::PasteAsMask => "Paste as mask".into(),
            ShellAction::PasteInside => "Paste inside".into(),
            ShellAction::ZoomIn => "Zoom in".into(),
            ShellAction::ZoomOut => "Zoom out".into(),
            ShellAction::ZoomTo(_) => "Zoom to".into(),
            ShellAction::FitAll => "Fit all".into(),
            ShellAction::About => "About".into(),
            ShellAction::ShowShortcuts => "Shortcut map".into(),
            ShellAction::ShowPreferences => "Preferences".into(),
            ShellAction::PickTool(tool) => format!("Tool: {tool:?}"),
            ShellAction::PickToolGroup(leader) => format!("Tool group: {leader:?}"),
            ShellAction::ToggleGrid => "Show/hide grid".into(),
            ShellAction::ToggleSnapToGrid => "Snap to grid".into(),
            ShellAction::EditGrid => "Edit grid".into(),
            ShellAction::ToggleGuides => "Show/hide guides".into(),
            ShellAction::ToggleGuidesLocked => "Lock guides".into(),
            ShellAction::ToggleSnapToGuides => "Snap to guides".into(),
            ShellAction::ClearGuides => "Clear guides".into(),
            ShellAction::FitSelection => "Fit selection".into(),
            ShellAction::ToggleSmartGuides => "Show/hide smart guides".into(),
            ShellAction::ToggleSnapToSmartGuides => "Snap to smart guides".into(),
            ShellAction::ToggleSliceLock => "Lock 9-slice guides".into(),
            ShellAction::ToggleSliceGuides => "Show/hide slice guides".into(),
            ShellAction::ToggleSliceOverlay => "Show/hide slice overlay".into(),
            ShellAction::ToggleEdges => "Show/hide edges".into(),
            ShellAction::ToggleRulers => "Show/hide rulers".into(),
            ShellAction::ToggleTooltips => "Show/hide tooltips".into(),
            ShellAction::ShowAllHidden => "Show all".into(),
            ShellAction::ToggleGamma => "Toggle gamma 1.8".into(),
            ShellAction::FeatherDialog => "Feather selection".into(),
            ShellAction::CanvasSize => "Canvas Size".into(),
            ShellAction::ImageSize => "Image Size".into(),
            ShellAction::CanvasColor => "Canvas Color".into(),
            ShellAction::OnlineHelp => "Online Help".into(),
            ShellAction::ManageExtensions => "Manage Extensions".into(),
            ShellAction::TogglePanel(name) => format!("{name} panel"),
            ShellAction::ToggleTools => "Tools panel".into(),
            ShellAction::SingleDocument => "Single-document window command".into(),
            ShellAction::ReportGap(_) => "Not available in this build".into(),
            ShellAction::PreviewInBrowser { .. } => "Preview in browser".into(),
            ShellAction::InsertWeb { what } => match what {
                InsertWebKind::Hotspot => "Insert hotspot".into(),
                InsertWebKind::RectSlice => "Insert rectangular slice".into(),
                InsertWebKind::PolygonSlice => "Insert polygon slice".into(),
                InsertWebKind::Button => "New button symbol".into(),
            },
            ShellAction::ToggleHidePanels => "Hide/show panels".into(),
            ShellAction::ToggleMainToolbar => "Main toolbar".into(),
            ShellAction::CycleScreenMode => "Screen mode".into(),
            ShellAction::SelectFrame(_) => "Go to previous/next state".into(),
            ShellAction::SelectPage(_) => "Go to previous/next page".into(),
            ShellAction::EditTextSelection => "Edit selection".into(),
        },
        Action::Sel(sel) => match sel {
            SelAction::Delete => "Delete selection".into(),
            SelAction::Clone => "Clone".into(),
            SelAction::Duplicate => "Duplicate".into(),
            SelAction::Simplify => "Simplify paths".into(),
            SelAction::ExpandStroke => "Expand stroke".into(),
            SelAction::Inset(_) => "Inset path".into(),
            SelAction::NewSymbol => "New symbol".into(),
            SelAction::NewLayer => "New layer".into(),
            SelAction::NewPage => "New page".into(),
            SelAction::NewFrame => "New state".into(),
            SelAction::BreakApart => "Break apart".into(),
            SelAction::Hide => "Hide selection".into(),
            SelAction::LockSelection => "Lock selection".into(),
            SelAction::Group => "Group".into(),
            SelAction::Ungroup => "Ungroup".into(),
            SelAction::FlattenSelection => "Flatten selection".into(),
            SelAction::MergeDown => "Merge down".into(),
            SelAction::FlattenLayers => "Flatten page".into(),
            SelAction::Rotate(deg) => format!("Rotate {deg}°"),
            SelAction::Reflect {
                horizontal,
                vertical,
            } => match (horizontal, vertical) {
                (true, false) => "Flip horizontal".into(),
                (false, true) => "Flip vertical".into(),
                (true, true) => "Flip both".into(),
                (false, false) => "Reflect (no-op)".into(),
            },
            SelAction::SelectParents => "Superselect".into(),
            SelAction::SelectChildren => "Subselect".into(),
            SelAction::Nudge { .. } => "Nudge selection".into(),
            SelAction::SelectSimilar => "Select similar".into(),
            SelAction::Feather(radius) => format!("Feather {radius}px"),
            SelAction::AdjustPixelSel { what, amount } => {
                let verb = match what {
                    "expand" => "Expand",
                    "contract" => "Contract",
                    "border" => "Border",
                    _ => "Smooth",
                };
                format!("{verb} marquee by {amount}px")
            }
            SelAction::PathToMarquee => "Convert path to marquee".into(),
            SelAction::MarqueeToPath => "Convert marquee to path".into(),
            SelAction::SaveSelection => "Save bitmap selection".into(),
            SelAction::RestoreSelection => "Restore bitmap selection".into(),
            SelAction::DeleteSavedSelection => "Delete bitmap selection".into(),
            SelAction::AddMask { mode } => format!("Mask: {mode}"),
            SelAction::ToggleMask => "Toggle element mask".into(),
            SelAction::LinkMask(true) => "Link mask".into(),
            SelAction::LinkMask(false) => "Unlink mask".into(),
            SelAction::RemoveMask { what_if_image } => format!("Delete mask ({what_if_image})"),
            SelAction::SaveJpegMask => "Save selection as JPEG mask".into(),
            SelAction::RestoreJpegMask => "Restore JPEG mask as selection".into(),
            SelAction::ClearJpegMask => "Remove JPEG mask".into(),
            SelAction::ConvertToSymbol => "Convert to symbol".into(),
            SelAction::AnimateSelection => "Animate selection".into(),
            SelAction::TweenInstances => "Tween instances".into(),
            SelAction::DetachFromPath => "Detach text from path".into(),
            SelAction::AttachToPath => "Attach text to path".into(),
            SelAction::SplitPaths => "Split paths".into(),
            SelAction::RemoveAnimation => "Remove animation".into(),
            SelAction::RemoveTransformations => "Remove transformations".into(),
            SelAction::EmptyBitmap => "Insert empty bitmap".into(),
            SelAction::BitmapViaCopy => "Bitmap via copy".into(),
            SelAction::BitmapViaCut => "Bitmap via cut".into(),
            SelAction::CropDocument => "Crop document".into(),
            SelAction::FillEdge { mode } => format!("Fill edge ({mode})"),
            SelAction::EditSymbol => "Edit symbol definition".into(),
            SelAction::ReversePathText => "Reverse text on path".into(),
            SelAction::ConvertToPaths => "Convert text to paths".into(),
        },
        Action::Align(mode) => format!("Align {mode}"),
        Action::Distribute(axis) => format!("Distribute {axis}"),
        Action::Boolean(op) => format!("Combine: {op}"),
        Action::Arrange(arrange) => match arrange {
            Arrange::Front => "Arrange to front".into(),
            Arrange::Forward => "Bring forward".into(),
            Arrange::Backward => "Send backward".into(),
            Arrange::Back => "Arrange to back".into(),
        },
        Action::TextSize(_) => "Text size".into(),
        Action::TextFlag(flag) => format!("Text {flag}"),
        Action::TextPlain => "Plain text".into(),
        Action::TextJustify(justification) => format!("Justify text ({justification})"),
        Action::TextStep(step) => {
            if step < 0 {
                "Smaller text".into()
            } else {
                "Larger text".into()
            }
        }
        Action::TextFont(family) => format!("Font: {family}"),
        Action::RepeatFilter => "Repeat the last filter".into(),
        Action::TextOnPathMode(mode) => format!("Text on-path mode ({mode})"),
        Action::Filter(preset) => format!("Filter: {preset}"),
        Action::Document(document) => match document {
            DocumentAction::TrimCanvas => "Trim canvas to content".into(),
            DocumentAction::FitCanvas => "Fit canvas to selection".into(),
            DocumentAction::Rotate(deg) => format!("Rotate document {deg}° CW"),
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// The highest inert-item count the docs have recorded. The
    /// `inert_count_never_rises` test refuses any regression above it;
    /// when the count falls, ratchet this down to the new number.
    const INERT_CEILING: usize = 2;

    fn walk(entries: &'static [Entry]) -> Vec<&'static Item> {
        let mut items = Vec::new();
        for entry in entries {
            match entry {
                Entry::Sep => {}
                Entry::Item(item) => items.push(item),
                Entry::Sub(_, sub) => items.extend(walk(sub)),
            }
        }
        items
    }

    /// The ten-menu CS6 bar, in order — the structural headline of M13.
    #[test]
    fn menu_bar_is_the_cs6_ten() {
        let labels: Vec<_> = MENUBAR.iter().map(|m| m.label).collect();
        assert_eq!(
            labels,
            [
                "File", "Edit", "View", "Select", "Modify", "Text", "Commands", "Filters",
                "Window", "Help"
            ]
        );
    }

    /// Command ids are unique — a transcription slip that would
    /// otherwise silently remap a CS6 command. Runtime-built items
    /// (id 0, the submenu popups' own convention) are exempt: the
    /// Filters effect tree has no resource ids by construction.
    #[test]
    fn command_ids_are_unique_per_menu() {
        for menu in MENUBAR {
            let mut ids: Vec<u32> = walk(menu.entries)
                .iter()
                .map(|i| i.id)
                .filter(|id| *id != 0)
                .collect();
            ids.sort_unstable();
            let before = ids.len();
            ids.dedup();
            assert_eq!(ids.len(), before, "duplicate ids in {}", menu.label);
        }
    }

    /// The Filters menu's runtime-built tree is the panel registry
    /// cast as menu items — same families, same presets, same order —
    /// and every name resolves. Repeat Filter keeps its resource id.
    #[test]
    fn filters_submenu_is_the_panel_registry() {
        assert_eq!(
            FILTERS.entries[0],
            item(35035, "Repeat Filter", "", Action::RepeatFilter)
        );
        let mut menu_names = Vec::new();
        for entry in FILTERS.entries {
            if let Entry::Sub(family, items) = entry {
                for item in *items {
                    let Entry::Item(item) = item else {
                        panic!("non-item in {family}");
                    };
                    let Action::Filter(name) = item.action else {
                        panic!("{} carries a non-filter action", item.label);
                    };
                    menu_names.push(name);
                }
            }
        }
        let panel_names: Vec<_> = crate::shell::FILTER_FAMILIES
            .iter()
            .flat_map(|(_, presets)| presets.iter().map(|p| p.name))
            .collect();
        assert_eq!(menu_names, panel_names, "menu and panel registries drift");
        for name in &menu_names {
            assert!(
                crate::shell::filter_preset(name).is_some(),
                "{name} does not resolve"
            );
        }
    }

    /// (total items, inert items) over the whole bar.
    fn census() -> (usize, usize) {
        let mut total = 0usize;
        let mut inert = 0usize;
        for menu in MENUBAR {
            for item in walk(menu.entries) {
                total += 1;
                if matches!(item.action, Action::Inert) {
                    inert += 1;
                }
            }
        }
        (total, inert)
    }

    /// Prints every inert row with its menu, id, and label — the
    /// burn-down worklist.
    #[test]
    fn inert_worklist() {
        for menu in MENUBAR {
            for item in walk(menu.entries) {
                if matches!(item.action, Action::Inert) {
                    eprintln!("INERT\t{}\t{}\t{}", menu.label, item.id, item.label);
                }
            }
        }
    }

    /// The reachability ceiling: the bar's inert count may fall but
    /// never rise. The docs quote this number, so it is derived here
    /// rather than maintained by hand — a new inert row is a
    /// regression the test refuses.
    #[test]
    fn inert_count_never_rises() {
        let (total, inert) = census();
        eprintln!("reachability: {inert} inert of {total} items");
        assert!(
            inert <= INERT_CEILING,
            "inert items rose to {inert} of {total} (ceiling {INERT_CEILING})"
        );
    }

    /// The binding table is read as sections: the help panel starts a
    /// new heading wherever the group changes, so a group that
    /// reappears later would be printed twice.
    #[test]
    fn shortcut_sections_are_contiguous() {
        let mut seen: Vec<&'static str> = Vec::new();
        for shortcut in SHORTCUTS {
            assert!(
                !shortcut.group.is_empty(),
                "a binding carries no section: {:?}",
                shortcut.action
            );
            if seen.last() != Some(&shortcut.group) {
                assert!(
                    !seen.contains(&shortcut.group),
                    "section {:?} is not contiguous",
                    shortcut.group
                );
                seen.push(shortcut.group);
            }
        }
    }

    /// The CS6 keymap parity gate.
    ///
    /// Every live menu row that prints an accelerator must have that
    /// accelerator bound to that row's action: the printed text is
    /// parsed by the same [`parse_accel`] the binding table is
    /// written against, so a menu row and the key that fires it
    /// cannot drift apart.
    ///
    /// Two kinds of row are display-only by design and are exempt:
    /// [`Action::Inert`] rows, which CS6 itself shipped unwired, and
    /// [`ShellAction::ReportGap`] rows, which name a capability this
    /// build does not ship.
    #[test]
    fn cs6_keymap_is_fully_bound() {
        for menu in MENUBAR {
            for item in walk(menu.entries) {
                if item.accel.is_empty() {
                    continue;
                }
                if matches!(item.action, Action::Inert) {
                    continue;
                }
                if matches!(item.action, Action::Shell(ShellAction::ReportGap(_))) {
                    continue;
                }
                let (mods, key) = parse_accel(item.accel).unwrap_or_else(|| {
                    panic!(
                        "{} ▸ {} prints the accelerator {:?}, which the keymap parser does not understand",
                        menu.label, item.label, item.accel
                    )
                });
                let bound = SHORTCUTS
                    .iter()
                    .find(|s| s.mods == mods && s.key == key)
                    .unwrap_or_else(|| {
                        let elsewhere: Vec<String> = SHORTCUTS
                            .iter()
                            .filter(|s| s.action == item.action)
                            .map(|s| s.label())
                            .collect();
                        panic!(
                            "{} ▸ {} prints {:?}, but no binding carries those modifiers and that key{}",
                            menu.label,
                            item.label,
                            item.accel,
                            if elsewhere.is_empty() {
                                String::new()
                            } else {
                                format!(" — the action is bound at {}", elsewhere.join(", "))
                            }
                        )
                    });
                assert_eq!(
                    bound.action,
                    item.action,
                    "{} ▸ {} prints {:?}, which is bound to {}, not to this row",
                    menu.label,
                    item.label,
                    item.accel,
                    action_name(bound.action)
                );
            }
        }
    }

    /// The item total is pinned exactly: it is the CS6 surface the
    /// transcription carries, and the docs quote it alongside the
    /// inert count. A change here means the surface changed —
    /// update the docs in the same change.
    ///
    /// The two web-panel rows (Slices, HTML Preview) are this build's
    /// own, so the total is two above the CS6 transcription.
    #[test]
    fn menu_item_total_is_pinned() {
        let (total, _) = census();
        assert_eq!(
            total, 296,
            "menu item total changed — update the docs that quote it"
        );
    }

    /// The Window menu's arrangement trio matches the CS6 resource:
    /// Cascade (57650), Tile Horizontally (57651, once missing from
    /// the transcription) and Tile Vertically (57652, once mislabeled
    /// "Tile"). All three report the single-document limitation
    /// instead of acting.
    #[test]
    fn the_window_arrangement_trio_matches_the_resource() {
        let window = MENUBAR
            .iter()
            .find(|menu| menu.label == "Window")
            .expect("the bar carries a Window menu");
        let find = |id: u32| {
            walk(window.entries)
                .into_iter()
                .find(|item| item.id == id)
                .unwrap_or_else(|| panic!("no Window row {id}"))
        };
        assert_eq!(find(57650).label, "Cascade");
        assert_eq!(find(57651).label, "Tile Horizontally");
        assert_eq!(find(57652).label, "Tile Vertically");
        for id in [57650, 57651, 57652] {
            assert!(matches!(
                find(id).action,
                Action::Shell(ShellAction::SingleDocument)
            ));
        }
    }

    /// Text ▸ Font: the resource ships the submenu as a single
    /// `placeholder - do not remove` popup (CS6 populates it at
    /// runtime from the system font inventory — `RT_MENU` 128, read
    /// directly from the resource DLL 2026-11-06). The transcription
    /// carries five renderer-known families through `text.setFont`,
    /// the same list the inspector's font combo offers.
    #[test]
    fn the_text_font_submenu_dispatches_set_font() {
        let text = MENUBAR
            .iter()
            .find(|menu| menu.label == "Text")
            .expect("the bar carries a Text menu");
        let font = text
            .entries
            .iter()
            .find_map(|entry| match entry {
                Entry::Sub("Font", items) => Some(items),
                _ => None,
            })
            .expect("a Font submenu");
        let families: Vec<&str> = walk(font)
            .into_iter()
            .map(|item| match item.action {
                Action::TextFont(family) => family,
                other => panic!("non-TextFont row in Font: {other:?}"),
            })
            .collect();
        assert_eq!(
            families,
            [
                "Liberation Sans",
                "Arial",
                "Verdana",
                "Times New Roman",
                "Courier New"
            ],
        );
    }

    /// Every item carries a label and (except the placeholder Font row
    /// copied verbatim from the resource) no Windows `&` mnemonics.
    #[test]
    fn labels_are_clean() {
        for menu in MENUBAR {
            for item in walk(menu.entries) {
                assert!(!item.label.is_empty(), "{}: empty label", menu.label);
                assert!(
                    !item.label.contains('&'),
                    "{}: {} keeps a mnemonic marker",
                    menu.label,
                    item.label
                );
            }
        }
    }

    /// No two separators in a row — CS6's tree has none; a stray pair
    /// would mean the parse desynced (the even-offset bug's tell).
    #[test]
    fn no_double_separators() {
        for menu in MENUBAR {
            let mut previous_sep = false;
            for entry in menu.entries {
                match entry {
                    Entry::Sep => {
                        assert!(!previous_sep, "{}: double separator", menu.label);
                        previous_sep = true;
                    }
                    _ => previous_sep = false,
                }
            }
        }
    }

    /// The shortcut table never shadows a binding with two actions and
    /// the load-bearing CS6 keys survive refactors.
    #[test]
    fn shortcuts_cover_the_core_map() {
        let mut flat: Vec<(Mods, egui::Key)> = SHORTCUTS.iter().map(|s| (s.mods, s.key)).collect();
        let before = flat.len();
        flat.sort_by_key(|(m, k)| (m.ctrl, m.alt, m.shift, *k as u32));
        flat.dedup();
        assert_eq!(flat.len(), before, "duplicate (mods, key) bindings");
        let find = |ctrl: bool, alt: bool, shift: bool, key: egui::Key| {
            SHORTCUTS
                .iter()
                .find(|s| {
                    s.mods.ctrl == ctrl
                        && s.mods.alt == alt
                        && s.mods.shift == shift
                        && s.key == key
                })
                .map(|s| &s.action)
        };
        assert_eq!(
            find(true, false, false, egui::Key::Z),
            Some(&Action::Shell(ShellAction::Undo))
        );
        assert_eq!(
            find(true, false, true, egui::Key::Z),
            Some(&Action::Shell(ShellAction::Redo))
        );
        assert_eq!(
            find(true, true, false, egui::Key::Num0),
            Some(&Action::Shell(ShellAction::FitAll))
        );
        assert_eq!(
            find(false, false, false, egui::Key::Delete),
            Some(&Action::Sel(SelAction::Delete))
        );
        assert_eq!(
            find(true, false, false, egui::Key::Num0),
            Some(&Action::Shell(ShellAction::FitSelection)),
            "Ctrl+0 is Fit Selection in CS6"
        );
        assert_eq!(
            find(true, true, false, egui::Key::Num0),
            Some(&Action::Shell(ShellAction::FitAll)),
            "Ctrl+Alt+0 is Fit All"
        );
    }

    /// The roster pinned by the Window menu is the CS6 panel set.
    #[test]
    fn window_menu_lists_the_cs6_panels() {
        let window = MENUBAR.iter().find(|m| m.label == "Window").unwrap();
        let labels: Vec<_> = walk(window.entries).iter().map(|i| i.label).collect();
        for expected in [
            "Tools",
            "Properties",
            "Optimize",
            "Layers",
            "Common Library",
            "Pages",
            "States",
            "History",
            "Auto Shapes",
            "Styles",
            "Document Library",
            "URL",
            "Color Mixer",
            "Swatches",
            "Info",
            "Behaviors",
            "Slices",
            "HTML Preview",
        ] {
            assert!(labels.contains(&expected), "Window menu missing {expected}");
        }
    }

    /// Every action in the table resolves to a help-panel description
    /// (no placeholder or panic path), and the F1 binding exists.
    #[test]
    fn every_shortcut_has_a_help_name() {
        for s in SHORTCUTS {
            let name = action_name(s.action);
            assert!(!name.is_empty(), "unlabeled action for {:?}", s.label());
        }
        assert!(
            SHORTCUTS
                .iter()
                .any(|s| s.action == Action::Shell(ShellAction::ShowShortcuts)),
            "F1 shortcut map binding missing"
        );
    }

    /// CS6's own keyboard-shortcut file, transcribed.
    ///
    /// The reference application ships an editable
    /// `Keyboard Shortcuts/Fireworks.xml`: one `<command>` per command
    /// and one `<shortcut text="M K"/>` per binding, where `M` is a
    /// modifier bitfield (1=Alt, 2=Shift, 4=Ctrl) and `K` is a Windows
    /// virtual-key code. Each row below is one of those entries, with
    /// the `FireCraft` action bound to that accelerator.
    ///
    /// The bitfield reading was validated against three independent
    /// anchors in the reference tree: the plain and the big nudges
    /// share the arrow keys and differ only in `Shift`, `cmd_
    /// PreviewSecondary` is Shift+F12, and `cmd_SelectCopyToPolySlice`
    /// is Alt+Shift+P.
    ///
    /// The command id is CS6's name for the command; the action is
    /// what this build answers that accelerator with. The two differ
    /// once, deliberately: CS6 calls Ctrl+PgUp `cmd_Page_Next` and
    /// Ctrl+PgDn `cmd_Page_Previous`, while this build follows the
    /// physical key and the tree's own Page menu rows — Ctrl+PgUp
    /// turns to the previous page. See `docs/research/fw-ui-evidence.md`.
    #[rustfmt::skip]
    static CS6_KEYMAP: &[(&str, u8, u32, Action)] = &[
    ("cmd_Frame_Previous", 2, 188, Action::Shell(ShellAction::SelectFrame(-1))),
    ("cmd_Frame_Next", 2, 190, Action::Shell(ShellAction::SelectFrame(1))),
    ("cmd_TextAlign_Justified", 7, 74, Action::TextJustify(3)),
    ("cmd_SelectNone", 4, 68, Action::Shell(ShellAction::Deselect)),
    ("cmd_TextEditor", 6, 69, Action::Shell(ShellAction::EditTextSelection)),
    ("cmd_SelectCopyToURL", 6, 85, Action::Shell(ShellAction::InsertWeb { what: InsertWebKind::Hotspot, })),
    ("cmd_AlignHorizontal", 5, 53, Action::Align("centerY")),
    ("cmd_Page_Next", 4, 33, Action::Shell(ShellAction::SelectPage(-1))),
    ("cmd_HideSelection", 4, 77, Action::Sel(SelAction::Hide)),
    ("cmd_Panels_URLMgr", 5, 85, Action::Shell(ShellAction::TogglePanel("URL"))),
    ("cmd_ViewRulers", 5, 82, Action::Shell(ShellAction::ToggleRulers)),
    ("cmd_Open", 4, 79, Action::Shell(ShellAction::Open)),
    ("cmd_EditImageObject", 4, 69, Action::Shell(ShellAction::EditTextSelection)),
    ("cmd_TextAlign_Stretch", 7, 83, Action::TextFlag("stretch")),
    ("cmd_ZoomIn", 4, 187, Action::Shell(ShellAction::ZoomIn)),
    ("cmd_Mask_EnableDisable", 6, 75, Action::Sel(SelAction::ToggleMask)),
    ("cmd_HideEdges", 4, 72, Action::Shell(ShellAction::ToggleEdges)),
    ("cmd_AlignBottom", 5, 54, Action::Align("bottom")),
    ("cmd_ArrangeFront", 4, 70, Action::Arrange(Arrange::Front)),
    ("cmd_TextAlign_Left", 7, 76, Action::TextJustify(0)),
    ("cmd_FlattenSelection", 7, 90, Action::Sel(SelAction::FlattenSelection)),
    ("cmd_Group", 4, 71, Action::Sel(SelAction::Group)),
    ("cmd_Split", 6, 74, Action::Sel(SelAction::SplitPaths)),
    ("cmd_TextStyle_Bold", 7, 66, Action::TextFlag("bold")),
    ("cmd_TextStyle_Bold", 0, 117, Action::TextFlag("bold")),
    ("cmd_ArrangeBack", 4, 66, Action::Arrange(Arrange::Back)),
    ("cmd_Toolbars_Art", 5, 84, Action::Shell(ShellAction::ToggleTools)),
    ("cmd_Panels_Styles", 5, 74, Action::Shell(ShellAction::TogglePanel("Styles"))),
    ("cmd_AlignTop", 5, 52, Action::Align("top")),
    ("cmd_Doc_FileInfo", 7, 70, Action::Shell(ShellAction::FileInfo)),
    ("cmd_New", 4, 78, Action::Shell(ShellAction::New)),
    ("cmd_NudgeRight", 0, 39, Action::Sel(SelAction::Nudge { dx: 1.0, dy: 0.0 })),
    ("cmd_Duplicate", 5, 68, Action::Sel(SelAction::Duplicate)),
    ("cmd_TextAlign_Right", 7, 82, Action::TextJustify(2)),
    ("cmd_Print", 4, 80, Action::Shell(ShellAction::Palette)),
    ("cmd_TweenInstances", 7, 84, Action::Sel(SelAction::TweenInstances)),
    ("cmd_ViewSliceGuides", 7, 186, Action::Shell(ShellAction::ToggleSliceGuides)),
    ("cmd_ViewGrid", 4, 222, Action::Shell(ShellAction::ToggleGrid)),
    ("cmd_NumericTransform", 6, 84, Action::Shell(ShellAction::NumericTransform)),
    ("cmd_Rotate270", 4, 55, Action::Sel(SelAction::Rotate(-90.0))),
    ("cmd_AlignVertical", 5, 50, Action::Align("centerX")),
    ("cmd_ExportPreview", 6, 88, Action::Shell(ShellAction::ExportPreview)),
    ("cmd_Undo", 4, 90, Action::Shell(ShellAction::Undo)),
    ("cmd_Export", 6, 82, Action::Shell(ShellAction::Export)),
    ("cmd_Clear", 0, 8, Action::Sel(SelAction::Delete)),
    ("cmd_Clear", 0, 46, Action::Sel(SelAction::Delete)),
    ("cmd_Save", 4, 83, Action::Shell(ShellAction::Save)),
    ("cmd_PasteAttributes", 7, 86, Action::Shell(ShellAction::ReportGap( "Paste Attributes (no attribute clipboard in this build)", ))),
    ("cmd_ZoomOut", 4, 189, Action::Shell(ShellAction::ZoomOut)),
    ("cmd_SaveAs", 6, 83, Action::Shell(ShellAction::SaveAs)),
    ("cmd_Redo", 6, 90, Action::Shell(ShellAction::Redo)),
    ("cmd_Join", 4, 74, Action::Boolean("union")),
    ("cmd_ViewSmartGuides", 4, 220, Action::Shell(ShellAction::ToggleSmartGuides)),
    ("cmd_SnapSmartGuides", 6, 220, Action::Shell(ShellAction::ToggleSnapToSmartGuides)),
    ("cmd_Tooltips", 4, 221, Action::Shell(ShellAction::ToggleTooltips)),
    ("cmd_BigNudgeDown", 2, 40, Action::Sel(SelAction::Nudge { dx: 0.0, dy: 10.0 })),
    ("cmd_ViewSnapGuides", 6, 186, Action::Shell(ShellAction::ToggleSnapToGuides)),
    ("cmd_NudgeUp", 0, 38, Action::Sel(SelAction::Nudge { dx: 0.0, dy: -1.0 })),
    ("cmd_PreviewSecondary", 2, 123, Action::Shell(ShellAction::PreviewInBrowser { secondary: true, all_pages: false, })),
    ("cmd_PreviewAllPagesInBrowser", 4, 123, Action::Shell(ShellAction::PreviewInBrowser { secondary: false, all_pages: true, })),
    ("cmd_FitSelection", 4, 48, Action::Shell(ShellAction::FitSelection)),
    ("cmd_PasteInside", 6, 86, Action::Shell(ShellAction::PasteInside)),
    ("cmd_ViewLockGuides", 5, 186, Action::Shell(ShellAction::ToggleGuidesLocked)),
    ("cmd_ViewGuides", 4, 186, Action::Shell(ShellAction::ToggleGuides)),
    ("cmd_ViewSnapGrid", 6, 222, Action::Shell(ShellAction::ToggleSnapToGrid)),
    ("cmd_HideOrShowAllPanels", 0, 9, Action::Shell(ShellAction::ToggleHidePanels)),
    ("cmd_HideOrShowAllPanels", 6, 72, Action::Shell(ShellAction::ToggleHidePanels)),
    ("cmd_SelectInverse", 6, 73, Action::Shell(ShellAction::SelectInverse)),
    ("cmd_ModifyCropObject", 5, 67, Action::Sel(SelAction::CropDocument)),
    ("cmd_TextStyle_Italic", 7, 73, Action::TextFlag("italic")),
    ("cmd_TextStyle_Italic", 0, 118, Action::TextFlag("italic")),
    ("cmd_FitInWindow", 5, 48, Action::Shell(ShellAction::FitAll)),
    ("cmd_Rotate90", 4, 57, Action::Sel(SelAction::Rotate(90.0))),
    ("cmd_Cut", 4, 88, Action::Shell(ShellAction::Cut)),
    ("cmd_ViewEditGuides", 7, 71, Action::Shell(ShellAction::ReportGap( "CS6's View Edit Guides dialog is not in this build", ))),
    ("cmd_Panels_Frames", 5, 75, Action::Shell(ShellAction::TogglePanel("States"))),
    ("cmd_TextAlign_Center", 7, 67, Action::TextJustify(1)),
    ("cmd_Panels_Layers", 5, 76, Action::Shell(ShellAction::TogglePanel("Layers"))),
    ("cmd_Panels_Swatches", 5, 83, Action::Shell(ShellAction::TogglePanel("Swatches"))),
    ("cmd_Quit", 4, 81, Action::Shell(ShellAction::Quit)),
    ("cmd_Copy", 4, 67, Action::Shell(ShellAction::Copy)),
    ("cmd_Page_Previous", 4, 34, Action::Shell(ShellAction::SelectPage(1))),
    ("cmd_ShowAll", 6, 77, Action::Shell(ShellAction::ShowAllHidden)),
    ("cmd_Panels_Behaviors", 5, 72, Action::Shell(ShellAction::TogglePanel("Behaviors"))),
    ("cmd_SelectChildren", 4, 40, Action::Sel(SelAction::SelectChildren)),
    ("cmd_ConvertToMaster", 0, 119, Action::Sel(SelAction::ConvertToSymbol)),
    ("cmd_ArrangeBackward", 6, 66, Action::Arrange(Arrange::Backward)),
    ("cmd_Close", 4, 87, Action::Shell(ShellAction::CloseDocument)),
    ("cmd_Clone", 6, 67, Action::Sel(SelAction::Clone)),
    ("cmd_Ungroup", 4, 85, Action::Sel(SelAction::Ungroup)),
    ("cmd_RepeatXtra", 7, 88, Action::RepeatFilter),
    ("cmd_Panels_Pages", 5, 79, Action::Shell(ShellAction::TogglePanel("Pages"))),
    ("cmd_cssproperties", 4, 118, Action::Shell(ShellAction::TogglePanel("CSS Properties"))),
    ("cmd_jquery", 4, 121, Action::Shell(ShellAction::ReportGap( "CS6's jQuery widget panel is not in this build", ))),
    ("cmd_SelectCopyToPolySlice", 3, 80, Action::Shell(ShellAction::InsertWeb { what: InsertWebKind::PolygonSlice, })),
    ("cmd_PreviewInBrowser", 0, 123, Action::Shell(ShellAction::PreviewInBrowser { secondary: false, all_pages: false, })),
    ("cmd_DistributeHorizontal", 5, 55, Action::Distribute("horizontal")),
    ("cmd_browse", 6, 79, Action::Shell(ShellAction::ReportGap( "Adobe Bridge is not available in this build.", ))),
    ("cmd_InsertLayer", 2, 76, Action::Sel(SelAction::NewLayer)),
    ("cmd_MergeDown", 5, 69, Action::Sel(SelAction::MergeDown)),
    ("cmd_TextStyle_Underline", 7, 85, Action::TextFlag("underline")),
    ("cmd_ViewEditGrid", 5, 71, Action::Shell(ShellAction::EditGrid)),
    ("cmd_commonlib_form_elements", 6, 76, Action::Shell(ShellAction::TogglePanel("Common Library"))),
    ("cmd_Transform", 4, 84, Action::Shell(ShellAction::TransformMode(TransformMode::Scale))),
    ("cmd_SelectAll", 4, 65, Action::Shell(ShellAction::SelectAll)),
    ("cmd_NewSymbol", 4, 119, Action::Sel(SelAction::NewSymbol)),
    ("cmd_TextStyle_Plain", 7, 80, Action::TextPlain),
    ("cmd_TextStyle_Plain", 0, 116, Action::TextPlain),
    ("cmd_ExitImageEdit", 6, 68, Action::Shell(ShellAction::Deselect)),
    ("cmd_InsertFrame", 2, 70, Action::Sel(SelAction::NewFrame)),
    ("cmd_FlattenDocument", 7, 69, Action::Sel(SelAction::FlattenLayers)),
    ("cmd_PanelInfo", 5, 73, Action::Shell(ShellAction::TogglePanel("Info"))),
    ("cmd_MaskGroup", 6, 71, Action::Sel(SelAction::AddMask { mode: "reveal selection", })),
    ("cmd_CreateImage", 5, 89, Action::Sel(SelAction::EmptyBitmap)),
    ("cmd_DistributeVertical", 5, 57, Action::Distribute("vertical")),
    ("cmd_ArrangeForward", 6, 70, Action::Arrange(Arrange::Forward)),
    ("cmd_BigNudgeUp", 2, 38, Action::Sel(SelAction::Nudge { dx: 0.0, dy: -10.0 })),
    ("cmd_Panels_ColorMixer", 5, 77, Action::Shell(ShellAction::TogglePanel("Color Mixer"))),
    ("cmd_AttachTextToPath", 6, 89, Action::Sel(SelAction::AttachToPath)),
    ("cmd_Import", 4, 82, Action::Shell(ShellAction::Import)),
    ("cmd_AlignRight", 5, 51, Action::Align("right")),
    ("cmd_BigNudgeRight", 2, 39, Action::Sel(SelAction::Nudge { dx: 10.0, dy: 0.0 })),
    ("cmd_SelectParent", 4, 38, Action::Sel(SelAction::SelectParents)),
    ("cmd_NudgeLeft", 0, 37, Action::Sel(SelAction::Nudge { dx: -1.0, dy: 0.0 })),
    ("cmd_NudgeDown", 0, 40, Action::Sel(SelAction::Nudge { dx: 0.0, dy: 1.0 })),
    ("cmd_AlignLeft", 5, 49, Action::Align("left")),
    ("cmd_ConvertToPaths", 6, 80, Action::Sel(SelAction::ConvertToPaths)),
    ("cmd_BigNudgeLeft", 2, 37, Action::Sel(SelAction::Nudge { dx: -10.0, dy: 0.0 })),
    ("cmd_Paste", 4, 86, Action::Shell(ShellAction::Paste)),
    ("cmd_PasteAsMask", 5, 86, Action::Shell(ShellAction::PasteAsMask)),
    ("cmd_Prev_Selection", 6, 37, Action::Shell(ShellAction::ReportGap( "Previous Selection (selection history not in this build)", ))),
    ("cmd_Next_Selection", 6, 39, Action::Shell(ShellAction::ReportGap( "Next Selection (selection history not in this build)", ))),
    ("cmd_SnapToPixel", 4, 75, Action::Shell(ShellAction::ReportGap( "Snap To Pixel needs a view snap mode, which this build does not ship", ))),
    ];

    /// The modifier combination a CS6 modifier bitfield names.
    fn cs6_mods(bits: u8) -> Mods {
        Mods {
            ctrl: bits & 4 != 0,
            alt: bits & 1 != 0,
            shift: bits & 2 != 0,
        }
    }

    /// The key a CS6 virtual-key code names.
    fn cs6_key(code: u32) -> Option<egui::Key> {
        let named = match code {
            8 => "Backspace",
            9 => "Tab",
            33 => "PageUp",
            34 => "PageDown",
            37 => "ArrowLeft",
            38 => "ArrowUp",
            39 => "ArrowRight",
            40 => "ArrowDown",
            46 => "Delete",
            186 => "Semicolon",
            187 => "Equals",
            188 => "Comma",
            189 => "Minus",
            190 => "Period",
            220 => "Backslash",
            221 => "CloseBracket",
            222 => "Quote",
            _ => {
                // A letter, a digit, or a function key: the virtual-key
                // code is the ASCII code.
                if (65..=90).contains(&code) || (48..=57).contains(&code) {
                    return egui::Key::from_name(&(code as u8 as char).to_string());
                }
                if (112..=123).contains(&code) {
                    return egui::Key::from_name(&format!("F{}", code - 111));
                }
                return None;
            }
        };
        egui::Key::from_name(named)
    }

    /// Every accelerator CS6's own keymap file declares is bound here,
    /// and bound to the action this build answers it with.
    #[test]
    fn cs6_keymap_entries_are_bound() {
        for &(command, mods, vk, action) in CS6_KEYMAP {
            let key =
                cs6_key(vk).unwrap_or_else(|| panic!("CS6 {command}: unknown virtual key {vk}"));
            let mods = cs6_mods(mods);
            let bound = SHORTCUTS
                .iter()
                .find(|s| s.mods == mods && s.key == key)
                .map(|s| s.action);
            assert_eq!(
                bound,
                Some(action),
                "CS6 {command} is not bound the way this build answers it"
            );
        }
    }

    /// The transcription covers CS6's whole keymap file: 133 shortcut
    /// entries across 128 command ids, with no two commands claiming
    /// the same accelerator.
    #[test]
    fn cs6_keymap_transcription_is_complete() {
        assert_eq!(CS6_KEYMAP.len(), 133, "CS6 shortcut entry count moved");
        let mut commands: Vec<&str> = CS6_KEYMAP.iter().map(|(command, ..)| *command).collect();
        commands.sort_unstable();
        commands.dedup();
        assert_eq!(commands.len(), 128, "CS6 command id count moved");
        let mut accelerators: Vec<(Mods, egui::Key)> = CS6_KEYMAP
            .iter()
            .map(|&(_, mods, vk, _)| {
                let key = cs6_key(vk).unwrap_or_else(|| panic!("unknown virtual key {vk}"));
                (cs6_mods(mods), key)
            })
            .collect();
        accelerators.sort_by_key(|(mods, key)| (mods.ctrl, mods.alt, mods.shift, *key as u32));
        let before = accelerators.len();
        accelerators.dedup();
        assert_eq!(
            accelerators.len(),
            before,
            "two CS6 commands claim the same accelerator"
        );
    }

    /// CS6's Print command is not shipped; Ctrl+P opens the command
    /// palette here. The divergence is named so the table is not read
    /// as "`FireCraft` prints on Ctrl+P".
    #[test]
    fn cs6_print_diverges_to_the_palette() {
        let entry = CS6_KEYMAP
            .iter()
            .find(|(command, ..)| *command == "cmd_Print")
            .expect("cmd_Print is transcribed");
        assert_eq!(entry.3, Action::Shell(ShellAction::Palette));
    }
}
