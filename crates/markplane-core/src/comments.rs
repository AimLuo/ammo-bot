//! Comment author defaults and body sanitization for `## Comments`.

/// Default author for appended comments.
///
/// Resolution order: `MARKPLANE_AUTHOR` → `$USER` → `$USERNAME` → `me`.
/// Empty values are treated as unset and fall through.
pub fn default_comment_author() -> String {
    for key in ["MARKPLANE_AUTHOR", "USER", "USERNAME"] {
        if let Ok(value) = std::env::var(key)
            && !value.is_empty()
        {
            return value;
        }
    }
    "me".to_string()
}

/// Demote ATX H1–H3 headings outside fenced code blocks to H4.
///
/// Lines whose trimmed form starts with `` ``` `` toggle fence state. Inside a
/// fence, heading-like lines are left unchanged. Outside, `#` / `##` / `###`
/// headings (1–3 hashes followed by a space or end of line) become `####`.
pub fn demote_comment_headings(text: &str) -> String {
    let mut in_fence = false;
    let mut out = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            out.push(line.to_string());
            continue;
        }
        if !in_fence
            && let Some(demoted) = demote_heading_line(line)
        {
            out.push(demoted);
            continue;
        }
        out.push(line.to_string());
    }

    let mut result = out.join("\n");
    if text.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn demote_heading_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();
    let indent = &line[..indent_len];

    let mut hash_count = 0usize;
    for b in trimmed.bytes() {
        if b == b'#' {
            hash_count += 1;
        } else {
            break;
        }
    }
    if !(1..=3).contains(&hash_count) {
        return None;
    }
    let rest = &trimmed[hash_count..];
    if !(rest.is_empty() || rest.starts_with(' ')) {
        return None;
    }
    Some(format!("{indent}####{rest}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvGuard {
        keys: Vec<(&'static str, Option<std::ffi::OsString>)>,
    }

    impl EnvGuard {
        fn apply(pairs: &[(&'static str, Option<&str>)]) -> Self {
            let keys: Vec<_> = pairs
                .iter()
                .map(|(k, _)| (*k, std::env::var_os(k)))
                .collect();
            // SAFETY: serialized by env_lock; restored in Drop.
            unsafe {
                for (key, value) in pairs {
                    match value {
                        Some(v) => std::env::set_var(key, v),
                        None => std::env::remove_var(key),
                    }
                }
            }
            Self { keys }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                for (key, prev) in &self.keys {
                    match prev {
                        Some(v) => std::env::set_var(key, v),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    #[test]
    fn default_comment_author_prefers_markplane_author() {
        let _lock = env_lock().lock().unwrap();
        let _guard = EnvGuard::apply(&[
            ("MARKPLANE_AUTHOR", Some("agent-implement")),
            ("USER", Some("alice")),
            ("USERNAME", Some("bob")),
        ]);
        assert_eq!(default_comment_author(), "agent-implement");
    }

    #[test]
    fn default_comment_author_falls_back_to_user() {
        let _lock = env_lock().lock().unwrap();
        let _guard = EnvGuard::apply(&[
            ("MARKPLANE_AUTHOR", None),
            ("USER", Some("alice")),
            ("USERNAME", Some("bob")),
        ]);
        assert_eq!(default_comment_author(), "alice");
    }

    #[test]
    fn default_comment_author_falls_back_to_username() {
        let _lock = env_lock().lock().unwrap();
        let _guard = EnvGuard::apply(&[
            ("MARKPLANE_AUTHOR", None),
            ("USER", None),
            ("USERNAME", Some("bob")),
        ]);
        assert_eq!(default_comment_author(), "bob");
    }

    #[test]
    fn default_comment_author_falls_back_to_me() {
        let _lock = env_lock().lock().unwrap();
        let _guard = EnvGuard::apply(&[
            ("MARKPLANE_AUTHOR", None),
            ("USER", None),
            ("USERNAME", None),
        ]);
        assert_eq!(default_comment_author(), "me");
    }

    #[test]
    fn default_comment_author_skips_empty_values() {
        let _lock = env_lock().lock().unwrap();
        let _guard = EnvGuard::apply(&[
            ("MARKPLANE_AUTHOR", Some("")),
            ("USER", Some("")),
            ("USERNAME", Some("bob")),
        ]);
        assert_eq!(default_comment_author(), "bob");
    }

    #[test]
    fn demote_comment_headings_demotes_h1_h2_h3_outside_fences() {
        let input = "# One\n## Two\n### Three\n#### Four\nplain\n";
        let expected = "#### One\n#### Two\n#### Three\n#### Four\nplain\n";
        assert_eq!(demote_comment_headings(input), expected);
    }

    #[test]
    fn demote_comment_headings_preserves_headings_inside_fences() {
        let input = "### Outside\n```\n# Inside\n## Also\n```\n## After\n";
        let expected = "#### Outside\n```\n# Inside\n## Also\n```\n#### After\n";
        assert_eq!(demote_comment_headings(input), expected);
    }

    #[test]
    fn demote_comment_headings_preserves_language_fences() {
        let input = "```rust\n# not a heading\n```\n# real\n";
        let expected = "```rust\n# not a heading\n```\n#### real\n";
        assert_eq!(demote_comment_headings(input), expected);
    }

    #[test]
    fn demote_comment_headings_leaves_non_heading_hash_lines() {
        let input = "#not-heading\n###also\n# ok\n";
        let expected = "#not-heading\n###also\n#### ok\n";
        assert_eq!(demote_comment_headings(input), expected);
    }

    #[test]
    fn append_comment_to_body_demotes_headings_in_new_entry() {
        let body = "# Task\n\nDesc.";
        let result = crate::append_comment_to_body(
            body,
            "2026-10-10",
            "agent-implement",
            "# Bad\n```\n## keep\n```\n## Also",
        );
        assert!(result.contains("#### Bad"));
        assert!(result.contains("```\n## keep\n```"));
        assert!(result.contains("#### Also"));
        assert!(!result.contains("\n# Bad"));
        assert!(result.contains("### 2026-10-10 agent-implement"));
    }
}
