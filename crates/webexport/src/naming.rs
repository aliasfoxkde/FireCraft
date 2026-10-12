//! Slice auto-naming: the SLICES.XTT `CellName` scheme applied at
//! export time, never stored on the slice.
//!
//! A named slice exports under its cleaned name (one file for the
//! whole region). Anything else falls back to `n{doc}_{row}_{col}` —
//! the `_r_c` suffix appears only when the grid has more than one row
//! or column — with the document base name cleaned the same way.
//! Cleaning drops every character outside `[A-Za-z0-9_\-.:]` and
//! prefixes a leading digit/punctuation with `n`; a name that cleans
//! to nothing falls through to the auto scheme.

/// Strips characters the template refuses and guards a bad lead.
#[must_use]
pub fn clean(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
        .collect();
    if cleaned.is_empty() {
        return cleaned;
    }
    // "if it starts with a number, add N to the front" — the template
    // guards any leading digit/punctuation alike.
    if cleaned.starts_with(|c: char| !(c.is_ascii_alphabetic() || c == '_')) {
        format!("n{cleaned}")
    } else {
        cleaned
    }
}

/// The cell name for a piece: the slice's cleaned name when it has
/// one, else `n{doc}_{row}_{col}` (row/col are 1-based; the suffix is
/// dropped for a 1×1 grid).
#[must_use]
pub fn cell_name(
    slice_name: Option<&str>,
    doc_name: &str,
    row: usize,
    col: usize,
    multi: bool,
) -> String {
    if let Some(raw) = slice_name {
        let cleaned = clean(raw);
        if !cleaned.is_empty() {
            return cleaned;
        }
    }
    auto_name(doc_name, row, col, multi)
}

/// The auto scheme: `n{doc}` alone on a 1×1 grid, `n{doc}_{row}_{col}`
/// otherwise.
#[must_use]
pub fn auto_name(doc_name: &str, row: usize, col: usize, multi: bool) -> String {
    let base = {
        let cleaned = clean(doc_name);
        if cleaned.is_empty() {
            "doc".to_owned()
        } else {
            cleaned
        }
    };
    if multi {
        format!("n{base}_{}_{}", row + 1, col + 1)
    } else {
        format!("n{base}")
    }
}

/// Appends the format extension.
#[must_use]
pub fn with_extension(stem: &str, extension: &str) -> String {
    format!("{stem}.{extension}")
}

/// The file name without its extension — the `<map>`/`usemap` name
/// key (`m_{stem}`) and the `<img name>` value.
#[must_use]
pub fn stem_of(file: &str) -> &str {
    match file.rfind('.') {
        Some(at) => &file[..at],
        None => file,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_keeps_template_characters_only() {
        assert_eq!(clean("Button 1!"), "Button1");
        assert_eq!(clean("nav_bar-2.png"), "nav_bar-2.png");
        assert_eq!(clean("a:b"), "a:b");
    }

    #[test]
    fn clean_prefixes_a_bad_lead_and_drops_the_rest() {
        assert_eq!(clean("9slice"), "n9slice");
        assert_eq!(clean("###"), "");
    }

    #[test]
    fn auto_name_suffixes_only_multi_cell_grids() {
        assert_eq!(auto_name("doc", 0, 0, false), "ndoc");
        assert_eq!(auto_name("doc", 1, 2, true), "ndoc_2_3");
    }

    #[test]
    fn auto_name_cleans_the_base_and_falls_back() {
        assert_eq!(auto_name("My Doc!", 0, 0, true), "nMyDoc_1_1");
        assert_eq!(auto_name("###", 0, 0, false), "ndoc");
    }

    #[test]
    fn cell_name_prefers_the_slice_name() {
        assert_eq!(cell_name(Some("logo"), "doc", 0, 0, true), "logo");
        assert_eq!(cell_name(Some("###"), "doc", 0, 1, true), "ndoc_1_2");
        assert_eq!(cell_name(None, "doc", 0, 0, false), "ndoc");
    }

    #[test]
    fn stems_strip_the_extension_only() {
        assert_eq!(stem_of("a.b.png"), "a.b");
        assert_eq!(stem_of("noext"), "noext");
    }
}
