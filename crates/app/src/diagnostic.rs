//! Minimal, user-consented diagnostic payload (`firecraft-diagnostic/v1`)
//! and PII/secret scrubber for zero-token GitHub Issue feedback.

use std::fmt::Write as _;

/// Schema identifier for FireCraft diagnostic payloads.
pub const DIAGNOSTIC_SCHEMA: &str = "firecraft-diagnostic/v1";

/// User-consented diagnostic payload for bug reports and feedback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticReport {
    /// Fixed schema version (`firecraft-diagnostic/v1`).
    pub schema: &'static str,
    /// Workspace/app version string.
    pub app_version: String,
    /// Target OS / architecture / runtime (`native` or `wasm32`).
    pub platform: String,
    /// Scrubbed user summary.
    pub summary: String,
    /// Scrubbed reproduction steps.
    pub repro_steps: String,
}

impl DiagnosticReport {
    /// Construct a new report, scrubbing PII and secret-like tokens from free text.
    #[must_use]
    pub fn new(
        app_version: impl Into<String>,
        platform: impl Into<String>,
        summary: &str,
        repro_steps: &str,
    ) -> Self {
        Self {
            schema: DIAGNOSTIC_SCHEMA,
            app_version: app_version.into(),
            platform: platform.into(),
            summary: scrub_pii(summary),
            repro_steps: scrub_pii(repro_steps),
        }
    }

    /// Render a markdown body suitable for clipboard copy or GitHub Issue URLs.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "**Schema:** `{}`", self.schema);
        let _ = writeln!(out, "**Version:** `{}`", self.app_version);
        let _ = writeln!(out, "**Platform:** `{}`\n", self.platform);
        let _ = writeln!(out, "### Summary\n{}\n", self.summary);
        if !self.repro_steps.is_empty() {
            let _ = writeln!(out, "### Reproduction Steps\n{}", self.repro_steps);
        }
        out
    }

    /// Build a zero-token prefilled GitHub Issue URL.
    #[must_use]
    pub fn prefilled_issue_url(&self) -> String {
        let title = url_encode(&format!("[Feedback] {}", truncate_line(&self.summary, 72)));
        let body = url_encode(&self.to_markdown());
        format!("https://github.com/aliasfoxkde/FireCraft/issues/new?title={title}&body={body}")
    }
}

/// Redact email addresses, local home directory paths, and common token prefixes.
#[must_use]
pub fn scrub_pii(input: &str) -> String {
    let mut out = Vec::with_capacity(input.len());
    for token in input.split_inclusive(char::is_whitespace) {
        let trimmed = token.trim_end_matches(char::is_whitespace);
        let ws = &token[trimmed.len()..];
        let scrubbed = if is_email_like(trimmed) {
            "[REDACTED_EMAIL]".to_owned()
        } else if is_token_like(trimmed) {
            "[REDACTED_TOKEN]".to_owned()
        } else {
            scrub_home_paths(trimmed)
        };
        out.push(scrubbed);
        if !ws.is_empty() {
            out.push(ws.to_owned());
        }
    }
    out.concat()
}

fn is_email_like(s: &str) -> bool {
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

fn is_token_like(s: &str) -> bool {
    s.starts_with("ghp_")
        || s.starts_with("gho_")
        || s.starts_with("github_pat_")
        || s.starts_with("sk-")
}

fn scrub_home_paths(s: &str) -> String {
    let mut res = s.to_owned();
    for prefix in ["/home/", "/Users/", "C:\\Users\\"] {
        if let Some(idx) = res.find(prefix) {
            let after = &res[idx + prefix.len()..];
            let sep = if prefix.contains('\\') { '\\' } else { '/' };
            let end = after.find(sep).map_or(res.len(), |off| idx + prefix.len() + off);
            res.replace_range(idx..end, "<HOME>");
        }
    }
    res
}

fn truncate_line(s: &str, max_chars: usize) -> String {
    let first = s.lines().next().unwrap_or("");
    first.chars().take(max_chars).collect()
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_pii_redacts_emails_tokens_and_home_paths() {
        let raw = "Contact alice@example.com at /home/alice/work with ghp_secret12345";
        let scrubbed = scrub_pii(raw);
        assert!(!scrubbed.contains("alice@example.com"));
        assert!(!scrubbed.contains("/home/alice"));
        assert!(!scrubbed.contains("ghp_secret12345"));
        assert!(scrubbed.contains("[REDACTED_EMAIL]"));
        assert!(scrubbed.contains("<HOME>/work"));
        assert!(scrubbed.contains("[REDACTED_TOKEN]"));
    }

    #[test]
    fn prefilled_issue_url_uses_canonical_repo() {
        let report = DiagnosticReport::new("1.0.0", "linux-x86_64", "Crash on export", "1. Open");
        let url = report.prefilled_issue_url();
        assert!(url.starts_with("https://github.com/aliasfoxkde/FireCraft/issues/new?title="));
        assert!(url.contains("firecraft-diagnostic%2Fv1"));
    }
}
