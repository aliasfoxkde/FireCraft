//! Browser platform services behind the [`engine::platform`] traits.
//!
//! The browser has no path-addressable file system, so the two traits
//! split duties with the async world outside:
//!
//! - **Save** is fully synchronous: [`WebFs::write_file`] downloads
//!   the bytes (Blob → object URL → anchor click) — works everywhere,
//!   including inside the WebOS iframe.
//! - **Open** is async by nature (the user must pick a file), which
//!   the synchronous [`Dialogs`] trait cannot await. So
//!   [`WebDialogs::open_path`] triggers the hidden `<input
//!   type="file">` and returns `None` — the shell reads that as
//!   "cancelled" and stays silent — while the input's `change` handler
//!   (wired in [`crate`]) performs the real load through
//!   `Shell::load_document_bytes`, the same read path as desktop.

use engine::platform::{Dialogs, FileSystem};
use wasm_bindgen::JsCast as _;

/// File system for the browser: writes download, reads refuse (open
/// flows through the picker instead).
pub struct WebFs;

impl FileSystem for WebFs {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, engine::EngineError> {
        Err(engine::EngineError::Io {
            path: path.to_owned(),
            detail: "browser build: file reads flow through the open picker, not paths".to_owned(),
        })
    }

    fn write_file(&self, path: &str, bytes: &[u8]) -> Result<(), engine::EngineError> {
        if download(path, bytes).is_none() {
            return Err(engine::EngineError::Io {
                path: path.to_owned(),
                detail: "download failed (no window/document?)".to_owned(),
            });
        }
        Ok(())
    }

    fn exists(&self, _path: &str) -> bool {
        false
    }
}

/// Downloads `bytes` as `name` via a temporary object-URL anchor.
/// Returns `None` when the DOM is unreachable.
fn download(name: &str, bytes: &[u8]) -> Option<()> {
    let window = web_sys::window()?;
    let document = window.document()?;
    let array = js_sys::Uint8Array::from(bytes);
    let blob =
        web_sys::Blob::new_with_u8_array_sequence(&js_sys::Array::of1(&array.into())).ok()?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).ok()?;
    let anchor = document
        .create_element("a")
        .ok()?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .ok()?;
    anchor.set_href(&url);
    anchor.set_download(name);
    let body = document.body()?;
    let _ = body.append_child(&anchor);
    anchor.click();
    let _ = body.remove_child(&anchor);
    let _ = web_sys::Url::revoke_object_url(&url);
    Some(())
}

/// Dialogs for the browser: open triggers the hidden file input (and
/// reports "cancelled" so the shell stays quiet until the real load),
/// save names are taken as-is, messages surface as alerts.
pub struct WebDialogs;

impl Dialogs for WebDialogs {
    fn open_path(&self, _title: &str, _filters: &[(String, Vec<String>)]) -> Option<String> {
        let window = web_sys::window()?;
        let document = window.document()?;
        let input = document
            .get_element_by_id("firecraft-file-input")?
            .dyn_into::<web_sys::HtmlInputElement>()
            .ok()?;
        // Runs inside the menu-click's user activation, so the picker
        // is allowed to open.
        input.click();
        None
    }

    fn save_path(
        &self,
        _title: &str,
        suggested_name: &str,
        _filters: &[(String, Vec<String>)],
    ) -> Option<String> {
        Some(suggested_name.to_owned())
    }

    fn message(&self, title: &str, text: &str) {
        if let Some(window) = web_sys::window() {
            let _ = window.alert_with_message(format!("{title}: {text}").as_str());
        }
    }
}
