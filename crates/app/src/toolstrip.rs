//! CS6's Tools panel, as data.
//!
//! The order and the grouping here are CS6's, not `FireCraft`'s. The
//! evidence is the Fireworks CS6 help set and the book's tool table:
//!
//! * "The Tools panel is organized into six categories: Select,
//!   Bitmap, Vector, Web, Colors, and View." (CS6 Help, "The Tools
//!   panel")
//! * "A small triangle in the lower right corner of a tool in the
//!   Tools panel indicates that it is part of a tool group." and "To
//!   quickly select a hidden tool, repeatedly press the group
//!   keyboard shortcut until the tool you want appears." (CS6 Help,
//!   "Select a tool from a tool group") — so a group carries **one**
//!   shortcut, and the group's members are the tools it hides.
//! * The names and the one-key shortcuts are the book's Table 1-1,
//!   "Fireworks Tools", read off the printed pages.
//! * The group memberships the help set names explicitly: the Rubber
//!   Stamp pop-up holds Red-eye Removal and Replace Color; the Paint
//!   Bucket pop-up holds Gradient; the Pen pop-up holds Vector Path
//!   and Redraw Path; the Freeform pop-up holds Reshape Area; the
//!   Vector section's pop-up holds the Auto Shapes.
//!
//! `docs/research/fw-ui-evidence.md` records the evidence in full,
//! including the parts CS6's keymap file still withholds (the GUID
//! behind each shortcut).

use crate::editor::Tool;

/// One tool in the strip: CS6's name for it and the tool itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolEntry {
    /// The name CS6 gives the tool.
    pub label: &'static str,
    /// The tool itself.
    pub tool: Tool,
}

/// A CS6 tool group: the leader button and the tools it hides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolGroup {
    /// The group's one-key shortcut. CS6 gives a group one shortcut
    /// and cycles through the group on repeated presses; an empty
    /// string means the group has no documented shortcut.
    pub key: &'static str,
    /// The group's tools, leader first.
    pub tools: &'static [ToolEntry],
}

impl ToolGroup {
    /// The leader — the tool the button shows.
    #[must_use]
    pub fn leader(&self) -> ToolEntry {
        self.tools[0]
    }

    /// Whether the group hides any tool behind its leader.
    #[must_use]
    pub fn is_flyout(&self) -> bool {
        self.tools.len() > 1
    }
}

/// One CS6 category — a section of the Tools panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolCategory {
    /// CS6's name for the section.
    pub name: &'static str,
    /// The groups the section holds, in CS6's order.
    pub groups: &'static [ToolGroup],
    /// Whether the section carries CS6's Auto Shape pop-up. The
    /// Vector section is the only one that does.
    pub auto_shapes: bool,
}

/// The strip, in CS6's order.
pub static CATEGORIES: &[ToolCategory] = &[
    ToolCategory {
        name: "Select",
        auto_shapes: false,
        groups: &[
            ToolGroup {
                key: "V",
                tools: &[ToolEntry {
                    label: "Pointer",
                    tool: Tool::Pointer,
                }],
            },
            ToolGroup {
                key: "A",
                tools: &[ToolEntry {
                    label: "Subselection",
                    tool: Tool::Subselect,
                }],
            },
            // CS6 Help, "Transform a selected object...": "by using
            // the Scale, Skew, and Distort tools and menu commands".
            ToolGroup {
                key: "Q",
                tools: &[
                    ToolEntry {
                        label: "Scale",
                        tool: Tool::Scale,
                    },
                    ToolEntry {
                        label: "Skew",
                        tool: Tool::Skew,
                    },
                    ToolEntry {
                        label: "Distort",
                        tool: Tool::Distort,
                    },
                ],
            },
            ToolGroup {
                key: "C",
                tools: &[ToolEntry {
                    label: "Crop",
                    tool: Tool::Crop,
                }],
            },
        ],
    },
    ToolCategory {
        name: "Bitmap",
        auto_shapes: false,
        groups: &[
            ToolGroup {
                key: "M",
                tools: &[
                    ToolEntry {
                        label: "Marquee",
                        tool: Tool::Marquee,
                    },
                    ToolEntry {
                        label: "Oval Marquee",
                        tool: Tool::EllipticalMarquee,
                    },
                ],
            },
            ToolGroup {
                key: "L",
                tools: &[
                    ToolEntry {
                        label: "Lasso",
                        tool: Tool::Lasso,
                    },
                    ToolEntry {
                        label: "Polygon Lasso",
                        tool: Tool::PolygonalMarquee,
                    },
                ],
            },
            ToolGroup {
                key: "W",
                tools: &[ToolEntry {
                    label: "Magic Wand",
                    tool: Tool::MagicWand,
                }],
            },
            // The book's Table 1-1 gives both the Brush and the Pencil
            // the same key: "B (toggle with Pencil)".
            ToolGroup {
                key: "B",
                tools: &[
                    ToolEntry {
                        label: "Brush",
                        tool: Tool::Paintbrush,
                    },
                    ToolEntry {
                        label: "Pencil",
                        tool: Tool::Pencil,
                    },
                ],
            },
            ToolGroup {
                key: "E",
                tools: &[ToolEntry {
                    label: "Eraser",
                    tool: Tool::Eraser,
                }],
            },
            ToolGroup {
                key: "R",
                tools: &[
                    ToolEntry {
                        label: "Blur",
                        tool: Tool::Blur,
                    },
                    ToolEntry {
                        label: "Sharpen",
                        tool: Tool::Sharpen,
                    },
                    ToolEntry {
                        label: "Smudge",
                        tool: Tool::Smudge,
                    },
                ],
            },
            ToolGroup {
                key: "D",
                tools: &[
                    ToolEntry {
                        label: "Dodge",
                        tool: Tool::Dodge,
                    },
                    ToolEntry {
                        label: "Burn",
                        tool: Tool::Burn,
                    },
                ],
            },
            // CS6 Help: "If the tool isn't visible, hold down the
            // Rubber Stamp tool until a pop-menu appears" — the pop-up
            // holds Red-eye Removal and Replace Color.
            ToolGroup {
                key: "S",
                tools: &[
                    ToolEntry {
                        label: "Rubber Stamp",
                        tool: Tool::RubberStamp,
                    },
                    ToolEntry {
                        label: "Red-eye Removal",
                        tool: Tool::RedEyeRemoval,
                    },
                    ToolEntry {
                        label: "Replace Color",
                        tool: Tool::ReplaceColor,
                    },
                ],
            },
        ],
    },
    ToolCategory {
        name: "Vector",
        auto_shapes: true,
        groups: &[
            ToolGroup {
                key: "N",
                tools: &[ToolEntry {
                    label: "Line",
                    tool: Tool::Line,
                }],
            },
            // "From the Pen tool pop-up menu, select the Vector Path
            // tool" / "select the Redraw Path tool".
            ToolGroup {
                key: "P",
                tools: &[
                    ToolEntry {
                        label: "Pen",
                        tool: Tool::Pen,
                    },
                    ToolEntry {
                        label: "Vector Path",
                        tool: Tool::VectorPath,
                    },
                    ToolEntry {
                        label: "Redraw Path",
                        tool: Tool::RedrawPath,
                    },
                ],
            },
            ToolGroup {
                key: "U",
                tools: &[
                    ToolEntry {
                        label: "Rectangle",
                        tool: Tool::Rect,
                    },
                    ToolEntry {
                        label: "Ellipse",
                        tool: Tool::Ellipse,
                    },
                ],
            },
            ToolGroup {
                key: "T",
                tools: &[ToolEntry {
                    label: "Type",
                    tool: Tool::Text,
                }],
            },
            // "From the Freeform tool pop-up menu, select the Reshape
            // Area tool."
            ToolGroup {
                key: "O",
                tools: &[
                    ToolEntry {
                        label: "Freeform",
                        tool: Tool::Freeform,
                    },
                    ToolEntry {
                        label: "Reshape Area",
                        tool: Tool::ReshapeArea,
                    },
                ],
            },
            ToolGroup {
                key: "Y",
                tools: &[ToolEntry {
                    label: "Knife",
                    tool: Tool::Knife,
                }],
            },
        ],
    },
    ToolCategory {
        name: "Web",
        auto_shapes: false,
        groups: &[
            ToolGroup {
                key: "J",
                tools: &[
                    ToolEntry {
                        label: "Rectangle Hotspot",
                        tool: Tool::Hotspot,
                    },
                    ToolEntry {
                        label: "Circle Hotspot",
                        tool: Tool::CircleHotspot,
                    },
                    ToolEntry {
                        label: "Polygon Hotspot",
                        tool: Tool::PolygonHotspot,
                    },
                ],
            },
            ToolGroup {
                key: "K",
                tools: &[
                    ToolEntry {
                        label: "Slice",
                        tool: Tool::Slice,
                    },
                    ToolEntry {
                        label: "Polygon Slice",
                        tool: Tool::PolygonSlice,
                    },
                ],
            },
            ToolGroup {
                key: "2",
                tools: &[ToolEntry {
                    label: "Hide/Show Slices",
                    tool: Tool::HideShowSlices,
                }],
            },
        ],
    },
    ToolCategory {
        name: "Colors",
        auto_shapes: false,
        groups: &[
            ToolGroup {
                key: "I",
                tools: &[ToolEntry {
                    label: "Eyedropper",
                    tool: Tool::Eyedropper,
                }],
            },
            // "Click the Paint Bucket tool in the Tools panel and
            // select the Gradient tool from the pop-up menu."
            ToolGroup {
                key: "G",
                tools: &[
                    ToolEntry {
                        label: "Paint Bucket",
                        tool: Tool::PaintBucket,
                    },
                    ToolEntry {
                        label: "Gradient",
                        tool: Tool::Gradient,
                    },
                ],
            },
            ToolGroup {
                key: "",
                tools: &[ToolEntry {
                    label: "Default Stroke and Fill Colors",
                    tool: Tool::DefaultStrokeFill,
                }],
            },
            ToolGroup {
                key: "",
                tools: &[ToolEntry {
                    label: "Swap Stroke and Fill Colors",
                    tool: Tool::SwapStrokeFill,
                }],
            },
            ToolGroup {
                key: "",
                tools: &[ToolEntry {
                    label: "No Stroke or Fill",
                    tool: Tool::NoStrokeFill,
                }],
            },
        ],
    },
    ToolCategory {
        name: "View",
        auto_shapes: false,
        groups: &[
            ToolGroup {
                key: "H",
                tools: &[ToolEntry {
                    label: "Hand",
                    tool: Tool::Hand,
                }],
            },
            ToolGroup {
                key: "Z",
                tools: &[ToolEntry {
                    label: "Zoom",
                    tool: Tool::Zoom,
                }],
            },
            // CS6 Help: "In the Tools panel, select from one of three
            // view modes to control the layout of your workspace."
            ToolGroup {
                key: "",
                tools: &[ToolEntry {
                    label: "Screen Mode",
                    tool: Tool::ToggleScreenMode,
                }],
            },
        ],
    },
];

/// The label CS6 gives `tool`.
#[must_use]
pub fn label_of(tool: Tool) -> &'static str {
    for category in CATEGORIES {
        for group in category.groups {
            for entry in group.tools {
                if entry.tool == tool {
                    return entry.label;
                }
            }
        }
    }
    "Auto Shape"
}

/// The group `tool` belongs to.
#[must_use]
pub fn group_of(tool: Tool) -> Option<&'static ToolGroup> {
    CATEGORIES
        .iter()
        .flat_map(|category| category.groups.iter())
        .find(|group| group.tools.iter().any(|entry| entry.tool == tool))
}

/// The shortcut that picks `tool`: its group's key, since CS6 gives a
/// group one shortcut and cycles through it.
#[must_use]
pub fn key_of(tool: Tool) -> &'static str {
    group_of(tool).map_or("", |group| group.key)
}

/// The tool's Alt-hold variant, or `None` when the tool has none.
/// CS6's help: "Hold down Alt (Windows) or Option (Mac OS) to change
/// from one tool behavior to the other" (fwref 3555, the Blur/
/// Sharpen/Smudge trio) and "To temporarily switch between the Dodge
/// tool and the Burn tool, hold down Alt (Windows)" (fwref 3580).
/// Both are pair-swaps within the group: Blur↔Sharpen and Dodge↔Burn.
/// Smudge has no second behavior to swap to, so it stays itself.
#[must_use]
pub fn alt_variant(tool: Tool) -> Option<Tool> {
    match tool {
        Tool::Blur => Some(Tool::Sharpen),
        Tool::Sharpen => Some(Tool::Blur),
        Tool::Dodge => Some(Tool::Burn),
        Tool::Burn => Some(Tool::Dodge),
        _ => None,
    }
}

/// Whether the tool is one of CS6's panel *buttons* — the controls
/// that act when clicked instead of becoming the active tool: the
/// Colors section's three stroke/fill controls, the Hide/Show Slices
/// control and the screen-mode control.
#[must_use]
pub fn is_panel_button(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::DefaultStrokeFill
            | Tool::SwapStrokeFill
            | Tool::NoStrokeFill
            | Tool::HideShowSlices
            | Tool::ToggleScreenMode
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CS6's six categories, in CS6's order: "The Tools panel is
    /// organized into six categories: Select, Bitmap, Vector, Web,
    /// Colors, and View." (CS6 Help)
    #[test]
    fn the_categories_are_cs6_s_in_cs6_s_order() {
        let names: Vec<&str> = CATEGORIES.iter().map(|category| category.name).collect();
        assert_eq!(
            names,
            ["Select", "Bitmap", "Vector", "Web", "Colors", "View"]
        );
    }

    /// The book's Table 1-1 gives each tool its one-key shortcut.
    /// Every shortcut the book names belongs to exactly one group —
    /// a shortcut that picked two different tools would make the
    /// group's cycle ambiguous.
    #[test]
    fn the_shortcuts_are_unique_across_groups() {
        let mut keys: Vec<&str> = CATEGORIES
            .iter()
            .flat_map(|category| category.groups.iter())
            .map(|group| group.key)
            .filter(|key| !key.is_empty())
            .collect();
        let count = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), count, "a shortcut is shared by two groups");
    }

    /// A group carries one shortcut, and the shortcut names the
    /// group's leader — so the letter the book prints next to a tool
    /// is the group's letter, not the tool's own.
    #[test]
    fn a_groups_shortcut_names_its_leader() {
        for category in CATEGORIES {
            for group in category.groups {
                if group.key.is_empty() {
                    continue;
                }
                assert_eq!(key_of(group.leader().tool), group.key);
            }
        }
    }

    /// CS6's Alt-hold swap (help 3555, 3580) is a pair-swap inside
    /// the two retouch groups: Blur↔Sharpen and Dodge↔Burn, involution
    /// (holding Alt twice returns the tool), and every other tool —
    /// Smudge included — has no variant to swap to.
    #[test]
    fn the_alt_swap_is_an_involution_on_two_pairs() {
        assert_eq!(alt_variant(Tool::Blur), Some(Tool::Sharpen));
        assert_eq!(alt_variant(Tool::Sharpen), Some(Tool::Blur));
        assert_eq!(alt_variant(Tool::Dodge), Some(Tool::Burn));
        assert_eq!(alt_variant(Tool::Burn), Some(Tool::Dodge));
        for category in CATEGORIES {
            for group in category.groups {
                for entry in group.tools {
                    if !matches!(
                        entry.tool,
                        Tool::Blur | Tool::Sharpen | Tool::Dodge | Tool::Burn
                    ) {
                        assert_eq!(
                            alt_variant(entry.tool),
                            None,
                            "{} has no documented Alt variant",
                            entry.label
                        );
                    }
                }
            }
        }
    }

    /// The flyouts CS6's help set names by hand: the Rubber Stamp
    /// pop-up holds Red-eye Removal and Replace Color; the Paint
    /// Bucket pop-up holds Gradient; the Pen pop-up holds Vector Path
    /// and Redraw Path; the Freeform pop-up holds Reshape Area.
    #[test]
    fn the_named_flyouts_hold_the_tools_cs6_names() {
        let members = |leader: Tool| -> Vec<Tool> {
            group_of(leader)
                .map(|group| group.tools.iter().map(|entry| entry.tool).collect())
                .unwrap_or_default()
        };
        assert_eq!(
            members(Tool::RubberStamp),
            [Tool::RubberStamp, Tool::RedEyeRemoval, Tool::ReplaceColor]
        );
        assert_eq!(
            members(Tool::PaintBucket),
            [Tool::PaintBucket, Tool::Gradient]
        );
        assert_eq!(
            members(Tool::Pen),
            [Tool::Pen, Tool::VectorPath, Tool::RedrawPath]
        );
        assert_eq!(members(Tool::Freeform), [Tool::Freeform, Tool::ReshapeArea]);
    }

    /// CS6's modify trio — "Transform a selected object, group, or a
    /// pixel selection by using the Scale, Skew, and Distort tools and
    /// menu commands" (CS6 Help) — is one group.
    #[test]
    fn the_modify_trio_shares_one_group() {
        assert_eq!(
            group_of(Tool::Scale).map(|group| group.key),
            Some("Q"),
            "the book's Table 1-1 gives Scale the letter Q"
        );
        assert!(group_of(Tool::Skew).is_some_and(|group| group.key == "Q"));
        assert!(group_of(Tool::Distort).is_some_and(|group| group.key == "Q"));
    }

    /// The five panel buttons act when clicked; they never become the
    /// active tool, and none of them hides another tool behind it.
    /// The Hide/Show Slices control keeps the book's "2" key, but the
    /// key acts on the control instead of selecting it.
    #[test]
    fn the_panel_buttons_are_the_controls_not_tools() {
        let buttons = [
            Tool::DefaultStrokeFill,
            Tool::SwapStrokeFill,
            Tool::NoStrokeFill,
            Tool::HideShowSlices,
            Tool::ToggleScreenMode,
        ];
        for tool in buttons {
            assert!(is_panel_button(tool), "{tool:?} is a panel button");
            assert!(group_of(tool).is_some(), "{tool:?} is in the strip");
            let flyout = group_of(tool).is_some_and(ToolGroup::is_flyout);
            assert!(!flyout, "{tool:?} has no flyout");
        }
        for tool in [
            Tool::DefaultStrokeFill,
            Tool::SwapStrokeFill,
            Tool::NoStrokeFill,
            Tool::ToggleScreenMode,
        ] {
            let keyed = group_of(tool).is_some_and(|group| !group.key.is_empty());
            assert!(!keyed, "{tool:?} carries no shortcut");
        }
        assert_eq!(key_of(Tool::HideShowSlices), "2");
    }

    /// Every tool in the strip has the label CS6 gives it, and no
    /// tool appears twice.
    #[test]
    fn every_strip_tool_is_labelled_once() {
        let mut seen: Vec<Tool> = Vec::new();
        for category in CATEGORIES {
            for group in category.groups {
                for entry in group.tools {
                    assert!(
                        !seen.contains(&entry.tool),
                        "{:?} appears twice",
                        entry.tool
                    );
                    seen.push(entry.tool);
                    assert_eq!(label_of(entry.tool), entry.label);
                }
            }
        }
    }

    /// The Auto Shapes pop-up belongs to the Vector section and
    /// nowhere else.
    #[test]
    fn only_the_vector_section_carries_the_auto_shapes() {
        let carrying: Vec<&str> = CATEGORIES
            .iter()
            .filter(|category| category.auto_shapes)
            .map(|category| category.name)
            .collect();
        assert_eq!(carrying, ["Vector"]);
    }
}
