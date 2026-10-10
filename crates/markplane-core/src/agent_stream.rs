//! Pure helpers for parsing Cursor `agent --output-format stream-json` output
//! and extracting the Stage verdict last-line.

use serde_json::Value;

use crate::runs::Usage;

/// Parsed stream-json event (unknown types ignored by callers).
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    SystemInit {
        session_id: String,
    },
    Assistant {
        text: String,
    },
    Result {
        /// `success` / `error` / other subtype from the event.
        subtype: String,
        is_error: bool,
        usage: Option<Usage>,
        duration_api_ms: Option<u64>,
        duration_ms: Option<u64>,
    },
}

/// Parse one NDJSON line into a [`StreamEvent`]. Returns `None` for blank,
/// invalid JSON, or event types the Dispatcher ignores.
pub fn parse_stream_json_line(line: &str) -> Option<StreamEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let value: Value = serde_json::from_str(line).ok()?;
    let event_type = value.get("type")?.as_str()?;
    match event_type {
        "system" => {
            if value.get("subtype").and_then(|v| v.as_str()) != Some("init") {
                return None;
            }
            let session_id = value.get("session_id")?.as_str()?.to_string();
            Some(StreamEvent::SystemInit { session_id })
        }
        "assistant" => {
            // Skip model_call assistant events (tool-related).
            if value.get("model_call_id").is_some() {
                return None;
            }
            let text = assistant_text(&value);
            Some(StreamEvent::Assistant { text })
        }
        "result" => {
            let subtype = value
                .get("subtype")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let is_error = value
                .get("is_error")
                .and_then(|v| v.as_bool())
                .unwrap_or(subtype != "success");
            let usage = value.get("usage").and_then(parse_usage);
            let duration_api_ms = value
                .get("duration_api_ms")
                .and_then(|v| v.as_u64())
                .or_else(|| value.get("durationApiMs").and_then(|v| v.as_u64()));
            let duration_ms = value.get("duration_ms").and_then(|v| v.as_u64());
            Some(StreamEvent::Result {
                subtype,
                is_error,
                usage,
                duration_api_ms,
                duration_ms,
            })
        }
        _ => None,
    }
}

fn assistant_text(value: &Value) -> String {
    let Some(content) = value
        .pointer("/message/content")
        .and_then(|v| v.as_array())
    else {
        return String::new();
    };
    let mut out = String::new();
    for part in content {
        if part.get("type").and_then(|v| v.as_str()) == Some("text")
            && let Some(text) = part.get("text").and_then(|v| v.as_str())
        {
            out.push_str(text);
        }
    }
    out
}

fn parse_usage(value: &Value) -> Option<Usage> {
    Some(Usage {
        input_tokens: value.get("inputTokens").and_then(|v| v.as_u64()),
        output_tokens: value.get("outputTokens").and_then(|v| v.as_u64()),
        cache_read_tokens: value.get("cacheReadTokens").and_then(|v| v.as_u64()),
        cache_write_tokens: value.get("cacheWriteTokens").and_then(|v| v.as_u64()),
    })
}

/// Last non-empty line of text (trimmed). Used as the Stage verdict line.
pub fn last_nonempty_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).rfind(|l| !l.is_empty())
}

/// Walk parsed events and return the last assistant text's last non-empty line.
pub fn verdict_from_events(events: &[StreamEvent]) -> Option<String> {
    let mut last_text: Option<&str> = None;
    for event in events {
        if let StreamEvent::Assistant { text } = event {
            last_text = Some(text.as_str());
        }
    }
    last_text.and_then(last_nonempty_line).map(str::to_string)
}

/// Success = exit code 0 AND a `result` event with subtype `success` (and not error).
pub fn agent_invocation_succeeded(exit_code: i32, events: &[StreamEvent]) -> bool {
    if exit_code != 0 {
        return false;
    }
    events.iter().any(|e| {
        matches!(
            e,
            StreamEvent::Result {
                subtype,
                is_error: false,
                ..
            } if subtype == "success"
        )
    })
}

/// Collect usage / duration from the last `result` event, if any.
pub fn usage_from_events(events: &[StreamEvent]) -> (Option<Usage>, Option<u64>) {
    let mut usage = None;
    let mut duration_api_ms = None;
    for event in events {
        if let StreamEvent::Result {
            usage: u,
            duration_api_ms: d,
            ..
        } = event
        {
            usage = u.clone();
            duration_api_ms = *d;
        }
    }
    (usage, duration_api_ms)
}

/// First `system/init` session_id in the stream.
pub fn session_id_from_events(events: &[StreamEvent]) -> Option<String> {
    for event in events {
        if let StreamEvent::SystemInit { session_id } = event {
            return Some(session_id.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_init_assistant_and_result() {
        let init = r#"{"type":"system","subtype":"init","session_id":"sess-abc","model":"m"}"#;
        let assistant = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Working...\n\nREVIEW: clean\n"}]}}"#;
        let result = r#"{"type":"result","subtype":"success","is_error":false,"usage":{"inputTokens":10,"outputTokens":4},"duration_api_ms":1234}"#;

        assert_eq!(
            parse_stream_json_line(init),
            Some(StreamEvent::SystemInit {
                session_id: "sess-abc".into()
            })
        );
        let a = parse_stream_json_line(assistant).unwrap();
        assert!(matches!(a, StreamEvent::Assistant { .. }));
        let r = parse_stream_json_line(result).unwrap();
        match r {
            StreamEvent::Result {
                subtype,
                is_error,
                usage,
                duration_api_ms,
                ..
            } => {
                assert_eq!(subtype, "success");
                assert!(!is_error);
                assert_eq!(usage.unwrap().input_tokens, Some(10));
                assert_eq!(duration_api_ms, Some(1234));
            }
            _ => panic!("expected result"),
        }
    }

    #[test]
    fn ignores_unknown_and_model_call_assistant() {
        assert!(parse_stream_json_line(r#"{"type":"tool_call"}"#).is_none());
        assert!(
            parse_stream_json_line(
                r#"{"type":"assistant","model_call_id":"x","message":{"content":[{"type":"text","text":"nope"}]}}"#
            )
            .is_none()
        );
        assert!(parse_stream_json_line("not-json").is_none());
        assert!(parse_stream_json_line("").is_none());
    }

    #[test]
    fn last_nonempty_line_skips_trailing_blank() {
        assert_eq!(
            last_nonempty_line("a\n\nREVIEW: clean\n\n"),
            Some("REVIEW: clean")
        );
        assert_eq!(last_nonempty_line("   \n\n"), None);
    }

    #[test]
    fn verdict_uses_last_assistant_event_only() {
        let events = vec![
            StreamEvent::Assistant {
                text: "first\nREVIEW: issues\n".into(),
            },
            StreamEvent::Assistant {
                text: "second\n\nREVIEW: clean\n".into(),
            },
            StreamEvent::Result {
                subtype: "success".into(),
                is_error: false,
                usage: None,
                duration_api_ms: None,
                duration_ms: None,
            },
        ];
        assert_eq!(verdict_from_events(&events).as_deref(), Some("REVIEW: clean"));
        assert!(agent_invocation_succeeded(0, &events));
        assert!(!agent_invocation_succeeded(1, &events));
    }

    #[test]
    fn success_requires_result_success_subtype() {
        let events = vec![
            StreamEvent::Assistant {
                text: "REVIEW: clean".into(),
            },
            StreamEvent::Result {
                subtype: "error".into(),
                is_error: true,
                usage: None,
                duration_api_ms: None,
                duration_ms: None,
            },
        ];
        assert!(!agent_invocation_succeeded(0, &events));
    }
}
