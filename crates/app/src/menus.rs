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
//! - an item present in CS6 but not wired in FireCraft yet is
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
    /// `filter.add` from the Filters menu: the preset's internal name,
    /// resolved through the same registry the Live Filters panel uses
    /// (`shell::filter_preset`). CS6 builds this menu at runtime from
    /// its filter registry — the static RT_MENU resource carries only
    /// Repeat Filter — and FireCraft mirrors exactly that.
    Filter(&'static str),
}

/// Shell-level behaviors — plumbing that never goes through the engine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShellAction {
    New,
    Open,
    Save,
    SaveAs,
    Export,
    ExportPreview,
    Quit,
    Undo,
    Redo,
    Deselect,
    SelectAll,
    ZoomIn,
    ZoomOut,
    /// Set an absolute magnification (fraction: 1.0 = 100%).
    ZoomTo(f32),
    FitAll,
    About,
    /// One-key tool select (the CS6 tool letters).
    PickTool(Tool),
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
    /// `symbol.breakApart` on the selection.
    BreakApart,
    /// `object.setVisible` false on the selection.
    Hide,
}

/// The Modify ▸ Arrange items; dispatch resolves the numeric index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrange {
    Front,
    Forward,
    Backward,
    Back,
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
    })
}

/// File — ids from RT_MENU 128; Ctrl+N/O/S/Shift+S/Shift+R from the
/// Fireworks.xml keymap, Ctrl+F4 as displayed by the resource itself.
pub static FILE: Menu = Menu {
    label: "File",
    entries: &[
        item(57600, "New", "Ctrl+N", Action::Shell(ShellAction::New)),
        item(37866, "New from Template...", "", Action::Inert),
        item(57601, "Open...", "Ctrl+O", Action::Shell(ShellAction::Open)),
        item(35795, "Browse in Bridge...", "", Action::Inert),
        Entry::Sub(
            "Open Recent",
            &[item(57616, "Recent File", "", Action::Inert)],
        ),
        Entry::Sub(
            "Scan",
            &[
                item(35199, "Twain Acquire...", "", Action::Inert),
                Entry::Sep,
                item(35200, "Twain Select...", "", Action::Inert),
            ],
        ),
        item(57602, "Close", "Ctrl+F4", Action::Inert),
        item(35789, "Close All", "", Action::Inert),
        Entry::Sep,
        item(57603, "Save", "Ctrl+S", Action::Shell(ShellAction::Save)),
        item(
            57604,
            "Save As...",
            "Ctrl+Shift+S",
            Action::Shell(ShellAction::SaveAs),
        ),
        item(35850, "Save All", "", Action::Inert),
        item(37867, "Save As Template...", "", Action::Inert),
        item(35036, "Revert", "", Action::Inert),
        Entry::Sep,
        item(35012, "Import...", "", Action::Inert),
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
                item(35587, "Fireworks PNG...", "", Action::Inert),
                item(35588, "JPEG Compressed...", "", Action::Inert),
                item(35420, "Use Export Settings...", "", Action::Inert),
            ],
        ),
        item(35416, "Update HTML...", "", Action::Inert),
        Entry::Sep,
        item(35792, "File Info...", "", Action::Inert),
        Entry::Sep,
        item(35181, "Export Wizard...", "", Action::Inert),
        item(35302, "Batch Process...", "", Action::Inert),
        Entry::Sub(
            "Preview in Browser",
            &[
                item(35313, "Preview in Primary Browser", "", Action::Inert),
                item(35315, "Preview in Secondary Browser", "", Action::Inert),
                Entry::Sep,
                item(
                    49003,
                    "Preview All Pages in Primary Browser",
                    "",
                    Action::Inert,
                ),
                Entry::Sep,
                item(35314, "Set Primary Browser...", "", Action::Inert),
                item(35316, "Set Secondary Browser...", "", Action::Inert),
            ],
        ),
        Entry::Sep,
        item(57605, "Page Setup...", "", Action::Inert),
        item(57607, "Print...", "Ctrl+P", Action::Inert),
        item(35452, "HTML Setup...", "", Action::Inert),
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
                item(35191, "New Button...", "", Action::Inert),
                item(
                    35195,
                    "New Symbol...",
                    "",
                    Action::Sel(SelAction::NewSymbol),
                ),
                Entry::Sep,
                item(35179, "Hotspot", "", Action::Inert),
                item(35237, "Rectangular Slice", "", Action::Inert),
                item(35786, "Polygon Slice", "", Action::Inert),
                Entry::Sep,
                item(35180, "Empty Bitmap", "", Action::Inert),
                item(35581, "Bitmap Via Copy", "", Action::Inert),
                item(35582, "Bitmap Via Cut", "", Action::Inert),
                Entry::Sep,
                item(35309, "Layer", "", Action::Sel(SelAction::NewLayer)),
                item(35301, "State", "", Action::Inert),
                item(35809, "Page", "", Action::Sel(SelAction::NewPage)),
            ],
        ),
        item(43402, "Find and Replace...", "", Action::Inert),
        Entry::Sep,
        item(57635, "Cut", "Ctrl+X", Action::Inert),
        item(57634, "Copy", "Ctrl+C", Action::Inert),
        item(35411, "Copy as Vectors", "", Action::Inert),
        item(35415, "Copy HTML Code...", "", Action::Inert),
        item(57637, "Paste", "Ctrl+V", Action::Inert),
        item(57632, "Clear", "Del", Action::Sel(SelAction::Delete)),
        Entry::Sep,
        item(35430, "Paste as Mask", "", Action::Inert),
        item(35127, "Paste Inside", "", Action::Inert),
        item(35033, "Paste Attributes", "", Action::Inert),
        Entry::Sep,
        item(35006, "Duplicate", "Ctrl+Alt+D", Action::Inert),
        item(35003, "Clone", "Ctrl+Shift+C", Action::Inert),
        Entry::Sep,
        item(35145, "Crop Selected Bitmap", "", Action::Inert),
        item(35346, "Crop Document", "", Action::Inert),
        Entry::Sep,
        item(35034, "Preferences...", "", Action::Inert),
        item(35429, "Keyboard Shortcuts...", "", Action::Inert),
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
        item(35066, "Fit Selection", "Ctrl+0", Action::Inert),
        item(
            35065,
            "Fit All",
            "Ctrl+Alt+0",
            Action::Shell(ShellAction::FitAll),
        ),
        Entry::Sep,
        item(35367, "Gamma 1.8", "", Action::Inert),
        Entry::Sep,
        item(
            35071,
            "Hide Selection",
            "Ctrl+M",
            Action::Sel(SelAction::Hide),
        ),
        item(35826, "Tooltips", "", Action::Inert),
        item(35072, "Show All", "", Action::Inert),
        Entry::Sep,
        item(35130, "Rulers", "", Action::Inert),
        Entry::Sub(
            "Grid",
            &[
                item(35131, "Show Grid", "", Action::Inert),
                item(35132, "Snap to Grid", "", Action::Inert),
                item(35133, "Edit Grid...", "", Action::Inert),
            ],
        ),
        Entry::Sub(
            "Guides",
            &[
                item(35134, "Show Guides", "", Action::Inert),
                item(35135, "Lock Guides", "", Action::Inert),
                item(35136, "Snap to Guides", "", Action::Inert),
                item(49004, "Clear Guides", "", Action::Inert),
            ],
        ),
        Entry::Sub(
            "Smart Guides",
            &[
                item(35815, "Show Smart Guides", "", Action::Inert),
                item(35821, "Snap to Smart Guides", "", Action::Inert),
            ],
        ),
        Entry::Sub(
            "9-Slice Scaling Guides",
            &[item(35812, "Lock Guides", "", Action::Inert)],
        ),
        item(35299, "Slice Guides", "", Action::Inert),
        item(35426, "Slice Overlay", "", Action::Inert),
        Entry::Sep,
        item(35088, "Edges", "", Action::Inert),
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
        item(35194, "Superselect", "", Action::Inert),
        item(35193, "Subselect", "", Action::Inert),
        Entry::Sep,
        item(35041, "Select Similar", "", Action::Inert),
        item(35039, "Select Inverse", "", Action::Inert),
        item(35089, "Feather...", "", Action::Inert),
        Entry::Sep,
        item(35405, "Expand Marquee...", "", Action::Inert),
        item(35406, "Contract Marquee...", "", Action::Inert),
        item(35407, "Border Marquee...", "", Action::Inert),
        item(35408, "Smooth Marquee...", "", Action::Inert),
        Entry::Sep,
        item(35780, "Convert Marquee to Path", "", Action::Inert),
        Entry::Sep,
        item(35467, "Save Bitmap Selection...", "", Action::Inert),
        item(35468, "Restore Bitmap Selection...", "", Action::Inert),
        item(35783, "Delete Bitmap Selection...", "", Action::Inert),
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
                item(35226, "Image Size...", "", Action::Inert),
                item(35004, "Canvas Size...", "", Action::Inert),
                item(35005, "Canvas Color...", "", Action::Inert),
                Entry::Sep,
                item(35250, "Trim Canvas", "", Action::Inert),
                item(35574, "Fit Canvas", "", Action::Inert),
                Entry::Sep,
                item(35348, "Rotate 180°", "", Action::Inert),
                item(35349, "Rotate 90° CW", "", Action::Inert),
                item(35350, "Rotate 90° CCW", "", Action::Inert),
            ],
        ),
        Entry::Sep,
        Entry::Sub(
            "Animation",
            &[
                item(35451, "Animate Selection...", "", Action::Inert),
                item(35463, "Settings...", "", Action::Inert),
                Entry::Sep,
                item(35472, "Remove Animation", "", Action::Inert),
            ],
        ),
        Entry::Sub(
            "Symbol",
            &[
                item(35189, "Convert to Symbol...", "", Action::Inert),
                Entry::Sep,
                item(35334, "Edit Symbol", "", Action::Inert),
                item(35842, "Edit in Place", "", Action::Inert),
                item(35188, "Tween Instances...", "", Action::Inert),
                item(35190, "Break Apart", "", Action::Sel(SelAction::BreakApart)),
                item(35807, "Swap Symbol...", "", Action::Inert),
            ],
        ),
        Entry::Sub(
            "Pop-up Menu",
            &[
                item(35445, "Add Pop-up Menu...", "", Action::Inert),
                item(35592, "Edit Pop-up Menu...", "", Action::Inert),
                Entry::Sep,
                item(35593, "Delete Pop-up Menu", "", Action::Inert),
            ],
        ),
        Entry::Sep,
        Entry::Sub(
            "Mask",
            &[
                item(35431, "Reveal All", "", Action::Inert),
                item(35432, "Hide All", "", Action::Inert),
                Entry::Sep,
                item(35430, "Paste as Mask", "", Action::Inert),
                item(35128, "Group as Mask", "", Action::Inert),
                Entry::Sep,
                item(35433, "Reveal Selection", "", Action::Inert),
                item(35434, "Hide Selection", "", Action::Inert),
                Entry::Sep,
                item(35435, "Disable Mask", "", Action::Inert),
                item(35436, "Delete Mask", "", Action::Inert),
            ],
        ),
        Entry::Sub(
            "Selective JPEG",
            &[
                item(35469, "Save Selection as JPEG Mask", "", Action::Inert),
                item(35470, "Restore JPEG Mask as Selection", "", Action::Inert),
                item(35576, "Settings...", "", Action::Inert),
                Entry::Sep,
                item(35471, "Remove JPEG Mask", "", Action::Inert),
            ],
        ),
        Entry::Sep,
        item(35784, "Lock Selection", "", Action::Inert),
        item(35009, "Flatten Selection", "", Action::Inert),
        item(35583, "Merge Down", "Ctrl+Alt+E", Action::Inert),
        item(35008, "Flatten Layers", "", Action::Inert),
        Entry::Sep,
        Entry::Sub(
            "Transform",
            &[
                item(35178, "Free Transform", "", Action::Inert),
                Entry::Sep,
                item(35060, "Scale", "", Action::Inert),
                item(35061, "Skew", "", Action::Inert),
                item(35090, "Distort", "", Action::Inert),
                item(35201, "Numeric Transform...", "", Action::Inert),
                Entry::Sep,
                item(35173, "Rotate 180°", "", Action::Inert),
                item(35174, "Rotate 90° CW", "", Action::Inert),
                item(35175, "Rotate 90° CCW", "", Action::Inert),
                Entry::Sep,
                item(35176, "Flip Horizontal", "", Action::Inert),
                item(35177, "Flip Vertical", "", Action::Inert),
                Entry::Sep,
                item(35152, "Remove Transformations", "", Action::Inert),
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
                item(35073, "Left", "", Action::Align("left")),
                item(35075, "Center Vertical", "", Action::Align("centerX")),
                item(35074, "Right", "", Action::Align("right")),
                Entry::Sep,
                item(35077, "Top", "", Action::Align("top")),
                item(35076, "Center Horizontal", "", Action::Align("centerY")),
                item(35078, "Bottom", "", Action::Align("bottom")),
                Entry::Sep,
                item(
                    35080,
                    "Distribute Widths",
                    "",
                    Action::Distribute("horizontal"),
                ),
                item(
                    35079,
                    "Distribute Heights",
                    "",
                    Action::Distribute("vertical"),
                ),
            ],
        ),
        Entry::Sep,
        item(35781, "Convert Path to Marquee...", "", Action::Inert),
        Entry::Sub(
            "Combine Paths",
            &[
                item(35094, "Join", "Ctrl+J", Action::Boolean("union")),
                item(35095, "Split", "Ctrl+Shift+J", Action::Inert),
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
                item(35183, "Hard Fill", "", Action::Inert),
                item(35184, "Anti-Alias Fill", "", Action::Inert),
                item(35185, "Feather Fill", "", Action::Inert),
            ],
        ),
        item(35849, "Snap To Pixel", "", Action::Inert),
        Entry::Sep,
        item(35010, "Group", "Ctrl+G", Action::Inert),
        item(35062, "Ungroup", "Ctrl+U", Action::Inert),
    ],
};

/// Text — size steps are the CS6 list; styles map onto the
/// `text.setAttrs` boolean attributes.
pub static TEXT: Menu = Menu {
    label: "Text",
    entries: &[
        Entry::Sub(
            "Font",
            &[item(
                35343,
                "placeholder - do not remove",
                "",
                Action::Inert,
            )],
        ),
        Entry::Sub(
            "Size",
            &[
                item(35230, "Other...", "", Action::Inert),
                item(35635, "Smaller", "", Action::Inert),
                item(35636, "Larger", "", Action::Inert),
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
                item(35057, "Plain", "", Action::Inert),
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
                item(35044, "Left", "", Action::Inert),
                item(35042, "Centered Horizontally", "", Action::Inert),
                item(35045, "Right", "", Action::Inert),
                item(35043, "Justified", "", Action::Inert),
                Entry::Sep,
                item(35114, "Top", "", Action::Inert),
                item(35115, "Centered Vertically", "", Action::Inert),
                item(35116, "Bottom", "", Action::Inert),
                item(35117, "Justified Vertically", "", Action::Inert),
            ],
        ),
        Entry::Sep,
        item(49005, "Attach In Path", "", Action::Inert),
        item(35118, "Attach to Path", "", Action::Inert),
        item(35119, "Detach from Path", "", Action::Inert),
        Entry::Sub(
            "Orientation",
            &[
                item(35120, "Rotate Around Path", "", Action::Inert),
                item(35121, "Vertical", "", Action::Inert),
                item(35122, "Skew Vertical", "", Action::Inert),
                item(35123, "Skew Horizontal", "", Action::Inert),
            ],
        ),
        item(35124, "Reverse Direction", "", Action::Inert),
        Entry::Sep,
        item(35125, "Convert to Paths", "", Action::Inert),
        Entry::Sep,
        item(35591, "Check Spelling...", "", Action::Inert),
        item(35594, "Spelling Setup...", "", Action::Inert),
    ],
};

/// Commands — the CS6 automation surface; scripts land with the
/// automation milestone, so all three are inert here.
pub static COMMANDS: Menu = Menu {
    label: "Commands",
    entries: &[
        item(35386, "Manage Saved Commands...", "", Action::Inert),
        item(35590, "Manage Extensions...", "", Action::Inert),
        item(35319, "Run Script...", "", Action::Inert),
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
        item(35035, "Repeat Filter", "", Action::Inert),
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
/// panel list; toggles are inert while the layout is fixed.
pub static WINDOW: Menu = Menu {
    label: "Window",
    entries: &[
        item(57648, "Duplicate Window", "", Action::Inert),
        item(35298, "Hide Panels", "", Action::Inert),
        Entry::Sep,
        Entry::Sub("Extensions", &[item(35848, " ", "", Action::Inert)]),
        Entry::Sep,
        Entry::Sub("Toolbars", &[item(35059, "Main", "", Action::Inert)]),
        item(35058, "Tools", "", Action::Inert),
        item(35586, "Properties", "", Action::Inert),
        Entry::Sep,
        item(43409, "Optimize", "", Action::Inert),
        item(43412, "Layers", "", Action::Inert),
        Entry::Sep,
        item(45816, "Common Library", "", Action::Inert),
        Entry::Sep,
        item(49125, "CSS Properties", "", Action::Inert),
        Entry::Sep,
        item(35798, "Pages", "", Action::Inert),
        item(43410, "States", "", Action::Inert),
        item(43418, "History", "", Action::Inert),
        Entry::Sep,
        item(43422, "Auto Shapes", "", Action::Inert),
        item(43415, "Styles", "", Action::Inert),
        item(43416, "Document Library", "", Action::Inert),
        item(43414, "URL", "", Action::Inert),
        Entry::Sep,
        item(43405, "Color Mixer", "", Action::Inert),
        item(43408, "Swatches", "", Action::Inert),
        Entry::Sep,
        item(43404, "Info", "", Action::Inert),
        item(43413, "Behaviors", "", Action::Inert),
        item(43402, "Find and Replace", "", Action::Inert),
        Entry::Sep,
        item(57650, "Cascade", "", Action::Inert),
        item(57652, "Tile", "", Action::Inert),
    ],
};

/// Help.
pub static HELP: Menu = Menu {
    label: "Help",
    entries: &[
        item(57667, "Fireworks Help", "", Action::Inert),
        item(35323, "Fireworks Support Center", "", Action::Inert),
        Entry::Sep,
        item(
            7028,
            "Adobe Product Improvement Program...",
            "",
            Action::Inert,
        ),
        Entry::Sep,
        item(7013, "Fireworks Exchange", "", Action::Inert),
        item(35637, "Manage Extensions...", "", Action::Inert),
        Entry::Sep,
        item(35580, "Adobe Online Forums", "", Action::Inert),
        Entry::Sep,
        item(
            57664,
            "About Fireworks...",
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
/// No modifiers.
pub const NONE: Mods = Mods {
    ctrl: false,
    alt: false,
    shift: false,
};

/// One bound key.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shortcut {
    pub mods: Mods,
    pub key: egui::Key,
    pub action: Action,
}

const fn key(mods: Mods, k: egui::Key, action: Action) -> Shortcut {
    Shortcut {
        mods,
        key: k,
        action,
    }
}

/// The Fireworks.xml keymap bindings this shell implements. Tool
/// letters are the CS6 one-key tool selects; plain keys only fire when
/// no text field has keyboard focus.
pub static SHORTCUTS: &[Shortcut] = &[
    // File
    key(CTRL, egui::Key::N, Action::Shell(ShellAction::New)),
    key(CTRL, egui::Key::O, Action::Shell(ShellAction::Open)),
    key(CTRL, egui::Key::S, Action::Shell(ShellAction::Save)),
    key(CTRL_SHIFT, egui::Key::S, Action::Shell(ShellAction::SaveAs)),
    key(CTRL_SHIFT, egui::Key::R, Action::Shell(ShellAction::Export)),
    key(
        CTRL_SHIFT,
        egui::Key::X,
        Action::Shell(ShellAction::ExportPreview),
    ),
    key(CTRL, egui::Key::Q, Action::Shell(ShellAction::Quit)),
    key(
        Mods {
            ctrl: false,
            alt: true,
            shift: false,
        },
        egui::Key::F4,
        Action::Shell(ShellAction::Quit),
    ),
    // Edit
    key(CTRL, egui::Key::Z, Action::Shell(ShellAction::Undo)),
    key(CTRL_SHIFT, egui::Key::Z, Action::Shell(ShellAction::Redo)),
    key(NONE, egui::Key::Delete, Action::Sel(SelAction::Delete)),
    key(NONE, egui::Key::Backspace, Action::Sel(SelAction::Delete)),
    key(CTRL, egui::Key::A, Action::Shell(ShellAction::SelectAll)),
    key(CTRL, egui::Key::D, Action::Shell(ShellAction::Deselect)),
    key(CTRL, egui::Key::M, Action::Sel(SelAction::Hide)),
    // View
    key(CTRL, egui::Key::Equals, Action::Shell(ShellAction::ZoomIn)),
    key(CTRL, egui::Key::Minus, Action::Shell(ShellAction::ZoomOut)),
    key(
        CTRL_ALT,
        egui::Key::Num0,
        Action::Shell(ShellAction::FitAll),
    ),
    // Modify ▸ Arrange (Fireworks.xml: 4 70 / 6 70 / 6 66 / 4 66)
    key(CTRL, egui::Key::F, Action::Arrange(Arrange::Front)),
    key(CTRL_SHIFT, egui::Key::F, Action::Arrange(Arrange::Forward)),
    key(CTRL_SHIFT, egui::Key::B, Action::Arrange(Arrange::Backward)),
    key(CTRL, egui::Key::B, Action::Arrange(Arrange::Back)),
    // Modify ▸ Combine Paths
    key(CTRL, egui::Key::J, Action::Boolean("union")),
    // Text styles
    key(CTRL_ALT_SHIFT, egui::Key::B, Action::TextFlag("bold")),
    key(CTRL_ALT_SHIFT, egui::Key::I, Action::TextFlag("italic")),
    key(CTRL_ALT_SHIFT, egui::Key::U, Action::TextFlag("underline")),
    // Tools (one-key selects)
    key(
        NONE,
        egui::Key::V,
        Action::Shell(ShellAction::PickTool(Tool::Pointer)),
    ),
    key(
        NONE,
        egui::Key::R,
        Action::Shell(ShellAction::PickTool(Tool::Rect)),
    ),
    key(
        NONE,
        egui::Key::E,
        Action::Shell(ShellAction::PickTool(Tool::Ellipse)),
    ),
    key(
        NONE,
        egui::Key::L,
        Action::Shell(ShellAction::PickTool(Tool::Line)),
    ),
    key(
        NONE,
        egui::Key::T,
        Action::Shell(ShellAction::PickTool(Tool::Text)),
    ),
    key(
        NONE,
        egui::Key::Z,
        Action::Shell(ShellAction::PickTool(Tool::Zoom)),
    ),
    key(
        NONE,
        egui::Key::H,
        Action::Shell(ShellAction::PickTool(Tool::Hand)),
    ),
];

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

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
            item(35035, "Repeat Filter", "", Action::Inert)
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
            None,
            "Ctrl+0 is Fit Selection in CS6 — not wired, must not bind Fit All"
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
        ] {
            assert!(labels.contains(&expected), "Window menu missing {expected}");
        }
    }
}
