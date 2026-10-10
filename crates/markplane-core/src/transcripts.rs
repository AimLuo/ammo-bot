//! Agent session transcript files under Cursor's project data dir.
//!
//! Path: `${CURSOR_DATA_DIR:-~/.cursor}/projects/<slug>/agent-transcripts/<id>/<id>.jsonl`
//! where `slug` is derived from the attempt workspace (lossy).

use crate::runs::{Attempt, RunRecord};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Derive the Cursor project slug from an agent workspace path.
///
/// Non-`[A-Za-z0-9]` → `-`, collapse consecutive `-`, trim leading/trailing `-`.
pub fn workspace_transcript_slug(workspace: &str) -> String {
    let mut out = String::with_capacity(workspace.len());
    let mut prev_dash = false;
    for c in workspace.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Resolve `${CURSOR_DATA_DIR:-$HOME/.cursor}`.
pub fn cursor_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("CURSOR_DATA_DIR")
        && !dir.is_empty()
    {
        return PathBuf::from(dir);
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".cursor")
}

/// Preferred absolute path when the Run record stored one; else slug-derived path.
pub fn resolve_transcript_path(
    session_id: &str,
    workspace: &str,
    transcript_path: Option<&str>,
    cursor_data: &Path,
) -> PathBuf {
    if let Some(p) = transcript_path {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let slug = workspace_transcript_slug(workspace);
    cursor_data
        .join("projects")
        .join(slug)
        .join("agent-transcripts")
        .join(session_id)
        .join(format!("{session_id}.jsonl"))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TranscriptBlock {
    Text { text: String },
    ToolUse {
        name: String,
        #[serde(default)]
        input: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranscriptMessage {
    pub idx: usize,
    pub role: String,
    pub blocks: Vec<TranscriptBlock>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SessionTranscript {
    Ok {
        path: String,
        messages: Vec<TranscriptMessage>,
        /// `success` / `aborted` from trailing `turn_ended`, or null while still writing.
        ended: Option<String>,
    },
    Unavailable { path: String, reason: String },
}

/// Find an attempt that owns `session_id` (first match wins for path resolution).
pub fn find_session_attempt<'a>(
    record: &'a RunRecord,
    session_id: &str,
) -> Option<&'a Attempt> {
    record
        .runs
        .iter()
        .flat_map(|r| r.attempts.iter())
        .find(|a| a.session_id.as_deref() == Some(session_id))
}

/// Read and parse a session transcript for a ticket's Run record.
///
/// Missing / unreadable / truncated last line → `Unavailable` (not an error).
pub fn read_session_transcript(
    record: &RunRecord,
    session_id: &str,
    cursor_data: &Path,
) -> SessionTranscript {
    let Some(attempt) = find_session_attempt(record, session_id) else {
        return SessionTranscript::Unavailable {
            path: String::new(),
            reason: format!("Session {session_id} is not in the local Run record"),
        };
    };
    let path = resolve_transcript_path(
        session_id,
        &attempt.workspace,
        attempt.transcript_path.as_deref(),
        cursor_data,
    );
    read_transcript_file(&path)
}

/// Re-read the whole jsonl file and parse messages.
pub fn read_transcript_file(path: &Path) -> SessionTranscript {
    let path_str = path.display().to_string();
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return SessionTranscript::Unavailable {
                path: path_str,
                reason: "File not found. The agent config dir may have changed or ~/.cursor was cleaned.".into(),
            };
        }
        Err(e) => {
            return SessionTranscript::Unavailable {
                path: path_str,
                reason: format!("Could not read transcript: {e}"),
            };
        }
    };

    let text = match String::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => {
            return SessionTranscript::Unavailable {
                path: path_str,
                reason: format!("Transcript is not valid UTF-8: {e}"),
            };
        }
    };

    parse_transcript_jsonl(&path_str, &text)
}

fn parse_transcript_jsonl(path: &str, text: &str) -> SessionTranscript {
    let lines: Vec<&str> = text
        .split('\n')
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .collect();

    if let Some(last) = lines.last()
        && serde_json::from_str::<Value>(last).is_err()
    {
        return SessionTranscript::Unavailable {
            path: path.to_string(),
            reason: "Last line is truncated or not valid JSON.".into(),
        };
    }

    let mut messages = Vec::new();
    let mut ended: Option<String> = None;

    for line in &lines {
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if value.get("type").and_then(|t| t.as_str()) == Some("turn_ended") {
            ended = value
                .get("status")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());
            continue;
        }

        let role = match value.get("role").and_then(|r| r.as_str()) {
            Some("user") | Some("assistant") => {
                value.get("role").and_then(|r| r.as_str()).unwrap().to_string()
            }
            _ => continue,
        };

        let content = value
            .pointer("/message/content")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();

        let mut blocks = Vec::new();
        for item in content {
            let ty = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match ty {
                "text" => {
                    blocks.push(TranscriptBlock::Text {
                        text: item
                            .get("text")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
                "tool_use" => {
                    blocks.push(TranscriptBlock::ToolUse {
                        name: item
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("?")
                            .to_string(),
                        input: item.get("input").cloned().unwrap_or(Value::Object(Default::default())),
                    });
                }
                _ => {}
            }
        }

        let idx = messages.len();
        messages.push(TranscriptMessage { idx, role, blocks });
    }

    SessionTranscript::Ok {
        path: path.to_string(),
        messages,
        ended,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn slug_replaces_non_alnum_collapses_and_trims() {
        assert_eq!(
            workspace_transcript_slug(
                "/Users/chuck/agent-space/ammo-bot/.worktrees/TASK-abc12"
            ),
            "Users-chuck-agent-space-ammo-bot-worktrees-TASK-abc12"
        );
        assert_eq!(workspace_transcript_slug("---a--b---"), "a-b");
        assert_eq!(workspace_transcript_slug("/tmp/"), "tmp");
    }

    #[test]
    fn resolved_path_wins_over_slug() {
        let p = resolve_transcript_path(
            "sid",
            "/tmp/wt",
            Some("/custom/sid.jsonl"),
            Path::new("/cursor"),
        );
        assert_eq!(p, PathBuf::from("/custom/sid.jsonl"));
    }

    #[test]
    fn truncated_last_line_is_unavailable() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("t.jsonl");
        fs::write(
            &path,
            concat!(
                r#"{"role":"user","message":{"content":[{"type":"text","text":"a"}]}}"#,
                "\n",
                r#"{"role":"assistant","message":{"content":[{"type":"text","text":"b"#,
                "\n",
            ),
        )
        .unwrap();
        match read_transcript_file(&path) {
            SessionTranscript::Unavailable { reason, .. } => {
                assert!(reason.to_lowercase().contains("truncat"));
            }
            other => panic!("expected unavailable, got {other:?}"),
        }
    }
}
