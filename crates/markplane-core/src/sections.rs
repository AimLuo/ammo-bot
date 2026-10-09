//! Markdown section splitting by `##` headings.
//!
//! Used by Map validation (and later UI / `.context`) to read named sections
//! from item bodies without a full AST parse.

use std::collections::BTreeMap;

/// Canonical Map body section titles (wayfinder shape).
pub const REQUIRED_MAP_SECTIONS: &[&str] = &[
    "Destination",
    "Notes",
    "Decisions so far",
    "Not yet specified",
    "Out of scope",
];

/// Split a markdown body into `##` sections.
///
/// Keys are heading titles (text after `##`, trimmed). Values are the body
/// under that heading (trimmed), excluding the next `##` heading. Content
/// before the first `##` is ignored. Nested `###` headings stay inside their
/// parent `##` section.
pub fn split_markdown_sections(body: &str) -> BTreeMap<String, String> {
    let mut sections = BTreeMap::new();
    let mut current_title: Option<String> = None;
    let mut current_lines: Vec<&str> = Vec::new();

    let flush =
        |title: &Option<String>, lines: &mut Vec<&str>, out: &mut BTreeMap<String, String>| {
            if let Some(t) = title {
                let content = lines.join("\n").trim().to_string();
                out.insert(t.clone(), content);
                lines.clear();
            }
        };

    for line in body.lines() {
        let trimmed = line.trim_end();
        if let Some(rest) = trimmed.strip_prefix("## ") {
            // Reject ### (already handled: "## " won't match "### ")
            // Also reject "####" etc. — only exactly two hashes + space.
            if !trimmed.starts_with("###") {
                flush(&current_title, &mut current_lines, &mut sections);
                current_title = Some(rest.trim().to_string());
                continue;
            }
        }
        if current_title.is_some() {
            current_lines.push(line);
        }
    }
    flush(&current_title, &mut current_lines, &mut sections);

    sections
}

/// Titles from [`REQUIRED_MAP_SECTIONS`] that are missing from `body`.
pub fn missing_map_sections(body: &str) -> Vec<&'static str> {
    let present = split_markdown_sections(body);
    REQUIRED_MAP_SECTIONS
        .iter()
        .copied()
        .filter(|title| !present.contains_key(*title))
        .collect()
}

/// Set or replace a `## Answer` section. Inserts before `## Comments` when
/// present; otherwise appends at the end.
pub fn set_answer_section(body: &str, answer: &str) -> String {
    let answer = answer.trim_end();
    let section = if answer.is_empty() {
        "## Answer\n".to_string()
    } else {
        format!("## Answer\n\n{answer}\n")
    };

    if let Some(replaced) = replace_section_content(body, "Answer", answer) {
        return replaced;
    }

    let mut result = body.trim_end().to_string();
    let comments_heading = result.lines().position(|line| line.trim() == "## Comments");
    if let Some(idx) = comments_heading {
        let lines: Vec<&str> = result.lines().collect();
        let mut out = String::new();
        for (i, line) in lines.iter().enumerate() {
            if i == idx {
                if !out.is_empty() && !out.ends_with('\n') {
                    out.push('\n');
                }
                if !out.ends_with("\n\n") && !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&section);
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                out.push('\n');
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    } else {
        if !result.is_empty() {
            result.push_str("\n\n");
        }
        result.push_str(&section);
        result
    }
}

/// Append a line to a named `##` section (creates the section at end if missing).
pub fn append_line_to_section(body: &str, section_title: &str, line: &str) -> String {
    let line = line.trim_end();
    let heading = format!("## {section_title}");
    let lines: Vec<&str> = body.lines().collect();
    let mut heading_idx = None;
    for (i, l) in lines.iter().enumerate() {
        if l.trim() == heading {
            heading_idx = Some(i);
            break;
        }
    }

    let Some(start) = heading_idx else {
        let mut result = body.trim_end().to_string();
        if !result.is_empty() {
            result.push_str("\n\n");
        }
        result.push_str(&heading);
        result.push_str("\n\n");
        result.push_str(line);
        result.push('\n');
        return result;
    };

    let mut end = lines.len();
    for (i, l) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = l.trim_end();
        if trimmed.starts_with("## ") && !trimmed.starts_with("###") {
            end = i;
            break;
        }
    }

    let mut out = String::new();
    for (i, l) in lines.iter().enumerate() {
        if i == end {
            // Ensure a blank line before the new entry if section has content.
            if !out.ends_with("\n\n") {
                if out.ends_with('\n') {
                    out.push('\n');
                } else {
                    out.push_str("\n\n");
                }
            }
            out.push_str(line);
            out.push('\n');
            if end < lines.len() {
                out.push('\n');
            }
        }
        out.push_str(l);
        out.push('\n');
    }
    if end == lines.len() {
        if !out.ends_with("\n\n") {
            if out.ends_with('\n') {
                out.push('\n');
            } else {
                out.push_str("\n\n");
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Replace content under `## {title}`; returns None if the heading is absent.
fn replace_section_content(body: &str, title: &str, new_content: &str) -> Option<String> {
    let heading = format!("## {title}");
    let lines: Vec<&str> = body.lines().collect();
    let mut start = None;
    for (i, l) in lines.iter().enumerate() {
        if l.trim() == heading {
            start = Some(i);
            break;
        }
    }
    let start = start?;
    let mut end = lines.len();
    for (i, l) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = l.trim_end();
        if trimmed.starts_with("## ") && !trimmed.starts_with("###") {
            end = i;
            break;
        }
    }

    let mut out = String::new();
    for (i, l) in lines.iter().enumerate() {
        if i == start {
            out.push_str(&heading);
            out.push('\n');
            if !new_content.is_empty() {
                out.push('\n');
                out.push_str(new_content);
                out.push('\n');
            }
            if end < lines.len() {
                out.push('\n');
            }
            continue;
        }
        if i > start && i < end {
            continue;
        }
        out.push_str(l);
        out.push('\n');
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_markdown_sections_extracts_headings() {
        let body = r#"# Title

## Destination

Reach the summit.

## Notes

Some context.

## Decisions so far

- Chose path A

## Not yet specified

Fog ahead.

## Out of scope

Weather.
"#;
        let sections = split_markdown_sections(body);
        assert_eq!(sections.get("Destination").unwrap(), "Reach the summit.");
        assert_eq!(sections.get("Notes").unwrap(), "Some context.");
        assert_eq!(sections.get("Decisions so far").unwrap(), "- Chose path A");
        assert_eq!(sections.get("Not yet specified").unwrap(), "Fog ahead.");
        assert_eq!(sections.get("Out of scope").unwrap(), "Weather.");
    }

    #[test]
    fn split_markdown_sections_keeps_subheadings_inside_parent() {
        let body = "## Notes\n\nIntro\n\n### Detail\n\nMore\n";
        let sections = split_markdown_sections(body);
        assert_eq!(
            sections.get("Notes").unwrap(),
            "Intro\n\n### Detail\n\nMore"
        );
        assert!(!sections.contains_key("Detail"));
    }

    #[test]
    fn missing_map_sections_reports_absent_titles() {
        let body = "## Destination\n\nGo.\n\n## Notes\n\nHi.\n";
        let missing = missing_map_sections(body);
        assert_eq!(
            missing,
            vec!["Decisions so far", "Not yet specified", "Out of scope",]
        );
    }

    #[test]
    fn missing_map_sections_empty_when_complete() {
        let body = r#"
## Destination
x
## Notes
x
## Decisions so far
x
## Not yet specified
x
## Out of scope
x
"#;
        assert!(missing_map_sections(body).is_empty());
    }
}
