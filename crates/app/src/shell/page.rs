//! Page and CSS slug helpers shared across document export and panels.

/// Convert an object or slice name into a clean CSS class identifier.
pub fn slugify(name: &str) -> String {
    let s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    if s.is_empty() { "slice".to_string() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_formats_names() {
        assert_eq!(slugify("Hero CTA 1"), "hero-cta-1");
        assert_eq!(slugify(""), "slice");
    }
}
