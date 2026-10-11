//! FireCraft editor shell, windowless core + thin egui face.
//!
//! The rule that makes surfaces equivalent: shell code never touches a
//! `Document` except through [`editor::Editor`], which only mutates
//! through [`engine::Session::apply`] — the same registry the CLI and
//! journal replay drive. Tests exercise the core without a window;
//! the window is glue.

pub mod canvas;
pub mod editor;
pub mod shell;
