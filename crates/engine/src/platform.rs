//! Platform services, injected: file system, dialogs, clipboard.
//!
//! The engine never touches `std::fs` or OS dialogs — surfaces provide
//! implementations. Desktop builds inject native ones; the wasm build
//! injects browser ones (File System Access API, download fallback);
//! tests inject [`MemoryFs`]. This is what keeps the engine wasm-clean
//! by construction.

/// File-system access the engine is allowed to use.
pub trait FileSystem {
    /// Reads a whole file.
    fn read_file(&self, path: &str) -> Result<Vec<u8>, crate::EngineError>;

    /// Writes (replaces) a whole file.
    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), crate::EngineError>;

    /// `true` when the path exists.
    fn exists(&self, path: &str) -> bool;
}

/// Open/save dialogs and user messages.
pub trait Dialogs {
    /// Asks for a file to open; `None` when the user cancels.
    /// `filters` are `(label, extensions)` pairs.
    fn open_path(&self, title: &str, filters: &[(String, Vec<String>)]) -> Option<String>;

    /// Asks for a save target; `None` when the user cancels.
    fn save_path(
        &self,
        title: &str,
        suggested_name: &str,
        filters: &[(String, Vec<String>)],
    ) -> Option<String>;

    /// Shows an informational message.
    fn message(&self, title: &str, text: &str);
}

/// Clipboard text access.
pub trait Clipboard {
    /// Puts text on the clipboard.
    fn write_text(&self, text: &str);

    /// Takes text off the clipboard, when it holds text.
    fn read_text(&self) -> Option<String>;
}

/// An in-memory file system for tests and offline demos — no `std::fs`
/// anywhere, so it is itself wasm-clean.
#[cfg(feature = "memory-fs")]
#[derive(Default)]
pub struct MemoryFs {
    files: std::cell::RefCell<std::collections::BTreeMap<String, Vec<u8>>>,
}

#[cfg(feature = "memory-fs")]
impl MemoryFs {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Directories that were "written" (paths of all files).
    #[must_use]
    pub fn paths(&self) -> Vec<String> {
        self.files.borrow().keys().cloned().collect()
    }
}

#[cfg(feature = "memory-fs")]
impl FileSystem for MemoryFs {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, crate::EngineError> {
        self.files
            .borrow()
            .get(path)
            .cloned()
            .ok_or_else(|| crate::EngineError::Io {
                path: path.to_owned(),
                detail: "no such file".to_owned(),
            })
    }

    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), crate::EngineError> {
        self.files
            .borrow_mut()
            .insert(path.to_owned(), bytes.to_vec());
        Ok(())
    }

    fn exists(&self, path: &str) -> bool {
        self.files.borrow().contains_key(path)
    }
}
