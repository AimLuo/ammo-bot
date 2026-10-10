//! Comment author defaults, body sanitization, and Comments-section parsing.

/// A single dated comment under `## Comments`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    pub date: String,
    pub author: String,
    pub body: String,
}

/// Body split into description (everything except the Comments section) and
/// parsed comments. Text before the first `### {date} {author}` is kept on
/// disk but not listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedBody {
    pub description: String,
    pub comments: Vec<Comment>,
}

/// Split an item body into description and comments.
///
/// The `## Comments` section runs from that heading (outside fences) to the
/// next H2 or end of file. Headings inside fenced code blocks do not start or
/// end sections, and do not start comments.
pub fn parse_item_body(body: &str) -> ParsedBody {
    let Some((before, comments_raw, after)) = split_comments_section(body) else {
        return ParsedBody {
            description: body.to_string(),
            comments: Vec::new(),
        };
    };
    let description = join_non_comment(before, after);
    let comments = parse_comments_list(comments_raw);
    ParsedBody {
        description,
        comments,
    }
}

/// Replace the non-comment part of `body`, appending the on-disk Comments
/// section unchanged at the end (when present).
pub fn with_description(body: &str, description: &str) -> String {
    let description = description.trim_end();
    match split_comments_section(body) {
        Some((_, comments_raw, _)) => {
            let mut out = description.to_string();
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(comments_raw.trim_end());
            out.push('\n');
            out
        }
        None => {
            if description.is_empty() {
                String::new()
            } else if description.ends_with('\n') {
                description.to_string()
            } else {
                format!("{description}\n")
            }
        }
    }
}

/// Split `body` into (before, comments_section_including_heading, after).
/// Returns `None` when there is no `## Comments` outside fences.
fn split_comments_section(body: &str) -> Option<(&str, &str, &str)> {
    let bytes = body.as_bytes();
    let mut in_fence = false;
    let mut line_start = 0usize;
    let mut comments_start: Option<usize> = None;
    let mut after_start: Option<usize> = None;
    let mut i = 0usize;

    while i <= bytes.len() {
        let is_eof = i == bytes.len();
        let is_eol = is_eof || bytes[i] == b'\n';
        if !is_eol {
            i += 1;
            continue;
        }

        let line_end = i;
        let line = &body[line_start..line_end];
        let trimmed = line.trim_end();
        let trimmed_start = trimmed.trim_start();

        if trimmed_start.starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence {
            if comments_start.is_none() && trimmed_start == "## Comments" {
                comments_start = Some(line_start);
            } else if comments_start.is_some()
                && after_start.is_none()
                && is_h2_heading(trimmed_start)
            {
                after_start = Some(line_start);
            }
        }

        if is_eof {
            break;
        }
        i += 1;
        line_start = i;
    }

    let start = comments_start?;
    let end = after_start.unwrap_or(body.len());
    Some((&body[..start], &body[start..end], &body[end..]))
}

fn join_non_comment(before: &str, after: &str) -> String {
    let before = before.trim_end();
    let after = after.trim_start_matches('\n').trim_end();
    if before.is_empty() {
        after.to_string()
    } else if after.is_empty() {
        before.to_string()
    } else {
        format!("{before}\n\n{after}")
    }
}

fn parse_comments_list(comments_section: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    let mut in_fence = false;
    let mut current: Option<(String, String, Vec<String>)> = None;

    let flush = |current: &mut Option<(String, String, Vec<String>)>, out: &mut Vec<Comment>| {
        if let Some((date, author, lines)) = current.take() {
            let body = trim_comment_body(&lines);
            out.push(Comment { date, author, body });
        }
    };

    for line in comments_section.lines() {
        let trimmed_start = line.trim_start();
        if trimmed_start.starts_with("```") {
            in_fence = !in_fence;
            if let Some((_, _, ref mut lines)) = current {
                lines.push(line.to_string());
            }
            continue;
        }
        if !in_fence
            && let Some((date, author)) = parse_comment_heading(trimmed_start)
        {
            flush(&mut current, &mut comments);
            current = Some((date, author, Vec::new()));
            continue;
        }
        if let Some((_, _, ref mut lines)) = current {
            lines.push(line.to_string());
        }
        // Preamble before the first comment heading is ignored for the list.
    }
    flush(&mut current, &mut comments);
    comments
}

fn trim_comment_body(lines: &[String]) -> String {
    let mut start = 0;
    let mut end = lines.len();
    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    lines[start..end].join("\n")
}

fn parse_comment_heading(trimmed: &str) -> Option<(String, String)> {
    let rest = trimmed.strip_prefix("### ")?;
    let mut parts = rest.splitn(2, char::is_whitespace);
    let date = parts.next()?.to_string();
    if !is_iso_date(&date) {
        return None;
    }
    let author = parts.next()?.trim().to_string();
    if author.is_empty() {
        return None;
    }
    Some((date, author))
}

fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 {
        return false;
    }
    // YYYY-MM-DD
    b[0..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
}

fn is_h2_heading(trimmed: &str) -> bool {
    trimmed.starts_with("## ") && !trimmed.starts_with("###")
}

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

    #[test]
    fn parse_item_body_splits_description_and_comments() {
        let body = "# Task\n\nDo the thing.\n\n## Comments\n\n### 2026-10-10 alice\n\nFirst\n\n### 2026-10-11 bob smith\n\nSecond\n";
        let parsed = parse_item_body(body);
        assert_eq!(parsed.description, "# Task\n\nDo the thing.");
        assert_eq!(
            parsed.comments,
            vec![
                Comment {
                    date: "2026-10-10".into(),
                    author: "alice".into(),
                    body: "First".into(),
                },
                Comment {
                    date: "2026-10-11".into(),
                    author: "bob smith".into(),
                    body: "Second".into(),
                },
            ]
        );
    }

    #[test]
    fn parse_item_body_keeps_preamble_off_the_list() {
        let body = "# T\n\n## Comments\n\nPreamble note\n\n### 2026-10-10 alice\n\nHi\n";
        let parsed = parse_item_body(body);
        assert_eq!(parsed.comments.len(), 1);
        assert_eq!(parsed.comments[0].author, "alice");
        assert_eq!(parsed.comments[0].body, "Hi");
    }

    #[test]
    fn parse_item_body_ignores_headings_inside_fences() {
        let body = "# T\n\n```\n## Comments\n### 2026-10-10 fake\n```\n\n## Comments\n\n### 2026-10-10 alice\n\nReal\n```\n### 2099-01-01 no\n```\n";
        let parsed = parse_item_body(body);
        assert_eq!(parsed.description, "# T\n\n```\n## Comments\n### 2026-10-10 fake\n```");
        assert_eq!(parsed.comments.len(), 1);
        assert_eq!(parsed.comments[0].author, "alice");
        assert!(parsed.comments[0].body.contains("### 2099-01-01 no"));
    }

    #[test]
    fn with_description_preserves_on_disk_comments_section() {
        let body = "# Old\n\nDesc\n\n## Comments\n\nPreamble\n\n### 2026-10-10 alice\n\nKeep me\n";
        let updated = with_description(body, "# New\n\nUpdated desc");
        assert_eq!(
            updated,
            "# New\n\nUpdated desc\n\n## Comments\n\nPreamble\n\n### 2026-10-10 alice\n\nKeep me\n"
        );
        // Simulate a concurrent append then description write
        let after_append = crate::append_comment_to_body(&body, "2026-10-10", "agent", "new");
        let preserved = with_description(&after_append, "# New\n\nOnly desc");
        assert!(preserved.contains("### 2026-10-10 alice"));
        assert!(preserved.contains("Keep me"));
        assert!(preserved.contains("### 2026-10-10 agent"));
        assert!(preserved.contains("new"));
        assert!(preserved.starts_with("# New\n\nOnly desc"));
    }

    #[test]
    fn with_description_without_comments_writes_description_only() {
        assert_eq!(with_description("# Old\n", "# New\n\nBody"), "# New\n\nBody\n");
    }
}
