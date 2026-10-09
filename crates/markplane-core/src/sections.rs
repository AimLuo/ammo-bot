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
