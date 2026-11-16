//! The keymap sets the shell offers.
//!
//! CS6's own keymap — the `Fireworks.xml` resource transcribed in
//! [`crate::menus`] — is the default and stays byte-identical in
//! behaviour. The other sets bind the *shared verbs* the way the
//! products whose muscle memory `FireCraft` borrows bind them, so a
//! user arriving from Illustrator or Photoshop is not fighting the
//! shell on every keystroke.
//!
//! A set is a set of **overrides**, not a second copy of the table:
//! each override replaces the CS6 binding of one verb. A verb the set
//! says nothing about keeps the CS6 binding, so no set is a
//! regression against the default. The resolved table is what the
//! shell dispatches from and what the published table prints.

use crate::editor::Tool;
use crate::menus::{
    Action, Arrange, CTRL, CTRL_ALT, CTRL_SHIFT, Mods, NONE, SHIFT, SelAction, ShellAction,
    Shortcut,
};

/// The keymap sets `FireCraft` ships.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum KeymapSet {
    /// CS6's own keymap — the `Fireworks.xml` resource. The default.
    #[default]
    Cs6,
    /// Adobe Illustrator's defaults.
    Illustrator,
    /// Adobe Photoshop's defaults.
    Photoshop,
    /// Adobe `FreeHand`'s defaults.
    FreeHand,
    /// The Web Standard conventions (the browser's and the platform's).
    WebStandard,
}

/// One set's binding for one shared verb.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Binding {
    /// The verb the binding fires.
    pub action: Action,
    /// The modifiers the set uses.
    pub mods: Mods,
    /// The key the set uses.
    pub key: egui::Key,
    /// The vendor's own name for the command, as the source's table
    /// spells it. The published table prints it so a reader can find
    /// the row in the vendor's own reference.
    pub vendor_command: &'static str,
}

/// A binding in the shape the tables are written in.
const fn bind(action: Action, mods: Mods, key: egui::Key, vendor_command: &'static str) -> Binding {
    Binding {
        action,
        mods,
        key,
        vendor_command,
    }
}

impl KeymapSet {
    /// The sets the shell offers, in the order the preferences panel
    /// lists them.
    pub const ALL: [KeymapSet; 5] = [
        KeymapSet::Cs6,
        KeymapSet::Illustrator,
        KeymapSet::Photoshop,
        KeymapSet::FreeHand,
        KeymapSet::WebStandard,
    ];

    /// The set's name, as the preferences panel and the published
    /// table show it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            KeymapSet::Cs6 => "Fireworks CS6",
            KeymapSet::Illustrator => "Illustrator",
            KeymapSet::Photoshop => "Photoshop",
            KeymapSet::FreeHand => "FreeHand",
            KeymapSet::WebStandard => "Web Standard",
        }
    }

    /// Where the set's bindings come from — the authority a reader of
    /// the published table needs.
    #[must_use]
    pub fn source(self) -> &'static str {
        match self {
            KeymapSet::Cs6 => "the CS6 `Fireworks.xml` keymap resource",
            KeymapSet::Illustrator => {
                "Adobe's *Default keyboard shortcuts in Adobe Illustrator* \
                 (helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html)"
            }
            KeymapSet::Photoshop => {
                "Adobe's *Default keyboard shortcuts in Adobe Photoshop* \
                 (helpx.adobe.com/photoshop/using/default-keyboard-shortcuts.html)"
            }
            KeymapSet::FreeHand => {
                "Macromedia's *FreeHand MX Keyboard Shortcuts* \
                 (`Command` read as `Ctrl`, `Option` read as `Alt`)"
            }
            KeymapSet::WebStandard => {
                "the Web Standard conventions — the browser and platform \
                 keyboard conventions"
            }
        }
    }

    /// The set's own bindings for the shared verbs.
    ///
    /// A set says nothing about the verbs it does not list: those keep
    /// the CS6 binding.
    #[must_use]
    pub fn overrides(self) -> &'static [Binding] {
        match self {
            KeymapSet::Cs6 => &[],
            KeymapSet::Illustrator => ILLUSTRATOR,
            KeymapSet::Photoshop => PHOTOSHOP,
            KeymapSet::FreeHand => FREEHAND,
            KeymapSet::WebStandard => WEB_STANDARD,
        }
    }

    /// The set's binding for one verb, if the set overrides it.
    #[must_use]
    pub fn binding(self, action: Action) -> Option<Binding> {
        self.overrides()
            .iter()
            .copied()
            .find(|binding| binding.action == action)
    }

    /// The bindings in force for this set: the CS6 table with the
    /// set's overrides applied.
    ///
    /// An overridden verb carries exactly one binding in the resolved
    /// table — the set's. The other set's binding for that verb is
    /// gone, so a verb cannot fire twice from one keystroke.
    #[must_use]
    pub fn bindings(self) -> Vec<Shortcut> {
        let mut table: Vec<Shortcut> = Vec::with_capacity(crate::menus::SHORTCUTS.len());
        let mut replaced: Vec<Action> = Vec::new();
        for shortcut in crate::menus::SHORTCUTS {
            match self.binding(shortcut.action) {
                Some(binding) => {
                    if replaced.contains(&shortcut.action) {
                        continue;
                    }
                    replaced.push(shortcut.action);
                    table.push(Shortcut {
                        group: shortcut.group,
                        mods: binding.mods,
                        key: binding.key,
                        action: shortcut.action,
                    });
                }
                None => table.push(*shortcut),
            }
        }
        table
    }

    /// The overrides that name a verb the CS6 table does not bind.
    ///
    /// Such an override cannot take effect — the CS6 table is where
    /// the shell learns that a verb exists — so it is reported rather
    /// than silently dropped. The set's own test asserts the list is
    /// empty for every set.
    #[must_use]
    pub fn unbound_overrides(self) -> Vec<Action> {
        self.overrides()
            .iter()
            .map(|binding| binding.action)
            .filter(|action| {
                !crate::menus::SHORTCUTS
                    .iter()
                    .any(|shortcut| shortcut.action == *action)
            })
            .collect()
    }
}

/// Adobe Illustrator's defaults, for the shared verbs.
///
/// Source: Adobe's *Default keyboard shortcuts in Adobe Illustrator* —
/// <https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html>
/// (Windows column). The vendor's own command name for each row is
/// kept beside the binding so a reader can find the row in that
/// reference.
///
/// Adobe's *ungroup* (`Ctrl+Shift+G`) and *bring forward* /
/// *bring backward* (`Ctrl+]` / `Ctrl+[`) are **not** carried over:
/// CS6 already binds those keys to *mask: reveal selection* and
/// *show/hide tooltips*, and one keystroke must fire one verb.
static ILLUSTRATOR: &[Binding] = &[
    bind(
        Action::Shell(ShellAction::Deselect),
        CTRL_SHIFT,
        egui::Key::A,
        "Deselect all artwork",
    ),
    bind(
        Action::Arrange(Arrange::Front),
        CTRL_SHIFT,
        egui::Key::CloseBracket,
        "Bring the selection to the front",
    ),
    bind(
        Action::Arrange(Arrange::Back),
        CTRL_SHIFT,
        egui::Key::OpenBracket,
        "Send the selection to the back",
    ),
];

/// Adobe Photoshop's defaults, for the shared verbs.
///
/// Source: Adobe's *Default keyboard shortcuts in Adobe Photoshop* —
/// <https://helpx.adobe.com/photoshop/using/default-keyboard-shortcuts.html>
/// (Windows column).
///
/// Adobe's *ungroup* (`Ctrl+Shift+G`) and *move target layer
/// down/up* (`Ctrl+]` / `Ctrl+[`) are **not** carried over: CS6
/// already binds those keys to *mask: reveal selection* and
/// *show/hide tooltips*, and one keystroke must fire one verb.
static PHOTOSHOP: &[Binding] = &[
    bind(
        Action::Sel(SelAction::NewLayer),
        CTRL_SHIFT,
        egui::Key::N,
        "New layer",
    ),
    bind(
        Action::Arrange(Arrange::Front),
        CTRL_SHIFT,
        egui::Key::CloseBracket,
        "Move layer to bottom or top",
    ),
    bind(
        Action::Arrange(Arrange::Back),
        CTRL_SHIFT,
        egui::Key::OpenBracket,
        "Move layer to bottom or top",
    ),
];

/// Adobe `FreeHand`'s defaults, for the shared verbs.
///
/// Source: Macromedia's `FreeHand MX Keyboard Shortcuts` — the
/// product's own shortcut sheet. Macromedia shipped one sheet per
/// platform; this is the Mac one, read with `Command` as `Ctrl` and
/// `Option` as `Alt` (the two products differ only in those two
/// modifier names on the same keystrokes).
static FREEHAND: &[Binding] = &[
    bind(
        Action::Shell(ShellAction::Redo),
        CTRL,
        egui::Key::Y,
        "Edit: Redo",
    ),
    bind(
        Action::Shell(ShellAction::FitAll),
        CTRL,
        egui::Key::Num0,
        "View: Fit All",
    ),
    bind(
        Action::Shell(ShellAction::FitSelection),
        CTRL_ALT,
        egui::Key::Num0,
        "View: Fit Selection",
    ),
    bind(
        Action::Shell(ShellAction::ToggleHidePanels),
        NONE,
        egui::Key::F4,
        "View: Panels",
    ),
    bind(
        Action::Shell(ShellAction::PickToolGroup(Tool::Pointer)),
        SHIFT,
        egui::Key::F10,
        "Tools / Commands: Tools: Select",
    ),
    bind(
        Action::Shell(ShellAction::PickToolGroup(Tool::Rect)),
        SHIFT,
        egui::Key::F1,
        "Tools / Commands: Tools: Rectangle",
    ),
    bind(
        Action::Shell(ShellAction::PickToolGroup(Tool::Pen)),
        SHIFT,
        egui::Key::F6,
        "Tools / Commands: Tools: Pen",
    ),
    bind(
        Action::Shell(ShellAction::PickToolGroup(Tool::Text)),
        CTRL_SHIFT,
        egui::Key::F9,
        "Tools / Commands: Tools: Text",
    ),
    bind(
        Action::Shell(ShellAction::PickToolGroup(Tool::Line)),
        SHIFT,
        egui::Key::F4,
        "Tools / Commands: Tools: Line",
    ),
    bind(
        Action::Shell(ShellAction::PickToolGroup(Tool::Freeform)),
        SHIFT,
        egui::Key::F5,
        "Tools / Commands: Tools: Freeform",
    ),
    bind(
        Action::Shell(ShellAction::PickToolGroup(Tool::Knife)),
        SHIFT,
        egui::Key::F7,
        "Tools / Commands: Tools: Knife",
    ),
    bind(
        Action::Shell(ShellAction::PickToolGroup(Tool::Scale)),
        NONE,
        egui::Key::F10,
        "Tools / Commands: Tools: Scale",
    ),
    bind(
        Action::TextFlag("italic"),
        CTRL,
        egui::Key::I,
        "Text: Font Style: Italic",
    ),
];

/// The Web Standard conventions, for the shared verbs.
///
/// Source: the Web Standard conventions — the browser and platform
/// keyboard conventions every web user already has in their fingers
/// (the editing and clipboard conventions the Chrome, Edge and
/// Firefox help pages document).
static WEB_STANDARD: &[Binding] = &[
    bind(Action::Shell(ShellAction::Redo), CTRL, egui::Key::Y, "Redo"),
    bind(Action::TextFlag("italic"), CTRL, egui::Key::I, "Italic"),
];

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    /// The CS6 binding for one verb.
    fn cs6(action: Action) -> Option<(Mods, egui::Key)> {
        crate::menus::SHORTCUTS
            .iter()
            .find(|shortcut| shortcut.action == action)
            .map(|shortcut| (shortcut.mods, shortcut.key))
    }

    #[test]
    fn the_cs6_set_is_the_default() {
        assert_eq!(KeymapSet::default(), KeymapSet::Cs6);
        assert_eq!(KeymapSet::ALL[0], KeymapSet::Cs6);
    }

    #[test]
    fn the_cs6_set_resolves_to_the_cs6_table() {
        let resolved = KeymapSet::Cs6.bindings();
        assert_eq!(resolved.len(), crate::menus::SHORTCUTS.len());
        for (resolved, cs6) in resolved.iter().zip(crate::menus::SHORTCUTS) {
            assert_eq!(resolved.action, cs6.action);
            assert_eq!(resolved.mods, cs6.mods);
            assert_eq!(resolved.key, cs6.key);
            assert_eq!(resolved.group, cs6.group);
        }
    }

    #[test]
    fn every_override_names_a_bound_verb() {
        for set in KeymapSet::ALL {
            assert!(
                set.unbound_overrides().is_empty(),
                "{set:?} names verbs the CS6 table does not bind: {:?}",
                set.unbound_overrides()
            );
        }
    }

    #[test]
    fn every_set_binds_every_verb_the_cs6_table_binds() {
        for set in KeymapSet::ALL {
            let cs6: Vec<Action> = crate::menus::SHORTCUTS
                .iter()
                .map(|shortcut| shortcut.action)
                .collect();
            let resolved: Vec<Action> = set.bindings().iter().map(|s| s.action).collect();
            for action in cs6 {
                assert!(
                    resolved.contains(&action),
                    "{set:?} dropped the binding for {action:?}"
                );
            }
        }
    }

    #[test]
    fn an_overridden_verb_carries_exactly_one_binding() {
        for set in KeymapSet::ALL {
            for binding in set.overrides() {
                let count = set
                    .bindings()
                    .iter()
                    .filter(|shortcut| shortcut.action == binding.action)
                    .count();
                assert_eq!(
                    count,
                    1,
                    "{set:?} binds {action:?} {count} times",
                    action = binding.action
                );
            }
        }
    }

    #[test]
    fn switching_sets_changes_the_resolved_binding() {
        // Every alternative set rebinds at least one verb, and every
        // rebinding differs from the CS6 binding it replaces. A set
        // that emptied itself would fail here rather than quietly
        // become the default.
        for set in KeymapSet::ALL {
            if set == KeymapSet::Cs6 {
                continue;
            }
            assert!(
                !set.overrides().is_empty(),
                "{set:?} rebinds nothing — it is not an alternative set"
            );
            for binding in set.overrides() {
                let cs6 = cs6(binding.action).expect("the CS6 table binds the verb");
                let resolved = set
                    .bindings()
                    .into_iter()
                    .find(|shortcut| shortcut.action == binding.action)
                    .expect("the set resolves the verb");
                assert_eq!(
                    (resolved.mods, resolved.key),
                    (binding.mods, binding.key),
                    "{set:?} resolves {action:?} to another key",
                    action = binding.action
                );
                assert_ne!(
                    (resolved.mods, resolved.key),
                    cs6,
                    "{set:?} leaves {action:?} on the CS6 key",
                    action = binding.action
                );
            }
        }
    }

    #[test]
    fn a_set_binds_no_keystroke_twice() {
        // The shell dispatches every binding whose key and modifiers
        // match, so one keystroke bound to two verbs would fire both.
        // The CS6 table holds this invariant already; a set must not
        // break it by parking an override on a key the table uses.
        for set in KeymapSet::ALL {
            let mut seen: Vec<(Mods, egui::Key)> = Vec::new();
            for shortcut in set.bindings() {
                assert!(
                    !seen.contains(&(shortcut.mods, shortcut.key)),
                    "{set:?} binds {:?} twice — one keystroke would fire two verbs",
                    shortcut.key
                );
                seen.push((shortcut.mods, shortcut.key));
            }
        }
    }

    #[test]
    fn the_cs6_binding_of_a_shared_verb_is_unchanged() {
        // The CS6 table is the default and stays byte-identical: the
        // resolution of the default set reproduces it exactly.
        assert_eq!(
            cs6(Action::Shell(crate::menus::ShellAction::Save)),
            Some((CTRL, egui::Key::S))
        );
        assert_eq!(
            cs6(Action::Shell(crate::menus::ShellAction::SaveAs)),
            Some((CTRL_SHIFT, egui::Key::S))
        );
    }
}
