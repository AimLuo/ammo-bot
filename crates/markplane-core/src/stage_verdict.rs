//! Stage verdict last-line judgment for the Dispatcher.
//!
//! Pure helpers so implement / fix / merge endings can be checked without
//! running agents.

/// Parsed implement Stage last line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImplementVerdict {
    Clean,
    Issues,
    /// Anything else, including `BLOCKED: <reason>`.
    Invalid(String),
}

/// Classify the implement Stage last line.
///
/// Legal endings are exactly `REVIEW: clean` and `REVIEW: issues`.
/// `BLOCKED: …` and any other text are invalid (hand off to a human; do not enter fix).
pub fn classify_implement_verdict(line: Option<&str>) -> ImplementVerdict {
    match line.map(str::trim) {
        Some("REVIEW: clean") => ImplementVerdict::Clean,
        Some("REVIEW: issues") => ImplementVerdict::Issues,
        Some(other) => ImplementVerdict::Invalid(other.to_string()),
        None => ImplementVerdict::Invalid(String::new()),
    }
}

/// Fix Stage succeeds only when the last line is exactly `FIX: done`.
pub fn is_fix_done(line: Option<&str>) -> bool {
    line.map(str::trim) == Some("FIX: done")
}

/// Merge Stage agent verdict succeeds only when the last line is exactly `MERGE: done`.
pub fn is_merge_done(line: Option<&str>) -> bool {
    line.map(str::trim) == Some("MERGE: done")
}

/// One-line failure reason for an illegal implement verdict (includes BLOCKED).
pub fn implement_verdict_failure_reason(verdict: &ImplementVerdict) -> Option<String> {
    match verdict {
        ImplementVerdict::Clean | ImplementVerdict::Issues => None,
        ImplementVerdict::Invalid(line) if line.is_empty() => {
            Some("implement ended without a legal REVIEW: clean|issues line".into())
        }
        ImplementVerdict::Invalid(line) if line.starts_with("BLOCKED:") => Some(format!(
            "implement blocked: {}",
            line.trim_start_matches("BLOCKED:").trim()
        )),
        ImplementVerdict::Invalid(line) => {
            Some(format!("implement ended with illegal verdict line: {line}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implement_accepts_clean_and_issues_only() {
        assert_eq!(
            classify_implement_verdict(Some("REVIEW: clean")),
            ImplementVerdict::Clean
        );
        assert_eq!(
            classify_implement_verdict(Some("REVIEW: issues")),
            ImplementVerdict::Issues
        );
        assert!(matches!(
            classify_implement_verdict(Some("BLOCKED: no spec")),
            ImplementVerdict::Invalid(_)
        ));
        assert!(matches!(
            classify_implement_verdict(Some("REVIEW: maybe")),
            ImplementVerdict::Invalid(_)
        ));
        assert!(matches!(
            classify_implement_verdict(None),
            ImplementVerdict::Invalid(_)
        ));
    }

    #[test]
    fn blocked_reason_is_surfaceable() {
        let v = classify_implement_verdict(Some("BLOCKED: missing API key"));
        let reason = implement_verdict_failure_reason(&v).unwrap();
        assert!(reason.contains("blocked"));
        assert!(reason.contains("missing API key"));
        assert!(implement_verdict_failure_reason(&ImplementVerdict::Clean).is_none());
    }

    #[test]
    fn fix_and_merge_require_exact_done() {
        assert!(is_fix_done(Some("FIX: done")));
        assert!(is_fix_done(Some("  FIX: done  \n")));
        assert!(!is_fix_done(Some("FIX: blocked")));
        assert!(!is_fix_done(Some("FIX:done")));
        assert!(!is_fix_done(None));
        assert!(is_merge_done(Some("MERGE: done")));
        assert!(!is_merge_done(Some("MERGE: failed")));
        assert!(!is_merge_done(None));
    }
}
