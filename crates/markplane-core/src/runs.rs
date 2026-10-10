//! Run records stored under `.markplane/.runs/<TASK-id>.json`.
//!
//! Local-only, gitignored accounts of Dispatcher Runs for one Implementation
//! ticket. Written atomically by the Dispatcher; read by serve and the UI.

use crate::error::{MarkplaneError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Pipeline Stage order used by the session log UI.
pub const STAGE_ORDER: &[Stage] = &[Stage::Implement, Stage::Fix, Stage::Merge];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Implement,
    Fix,
    Merge,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Implement => "implement",
            Stage::Fix => "fix",
            Stage::Merge => "merge",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Initial,
    Resume,
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptResult {
    Ok,
    Failed,
    Timeout,
    Interrupted,
    ResumeMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunStatus {
    Running,
    MergeQueued,
    Succeeded,
    Failed,
    Stopped,
}

/// Token usage from an agent `result` event. Unknown fields are null.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    #[serde(rename = "inputTokens")]
    pub input_tokens: Option<u64>,
    #[serde(rename = "outputTokens")]
    pub output_tokens: Option<u64>,
    #[serde(rename = "cacheReadTokens")]
    pub cache_read_tokens: Option<u64>,
    #[serde(rename = "cacheWriteTokens")]
    pub cache_write_tokens: Option<u64>,
}

/// One agent process invocation within a Run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attempt {
    pub stage: Stage,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Agent working directory (used to derive transcript slug).
    pub workspace: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_group_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_config_dir: Option<String>,
    pub trigger: Trigger,
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<AttemptResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_api_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr_tail: Option<String>,
}

/// One Run of an Implementation ticket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Run {
    pub seq: u32,
    pub status: RunStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_stage: Option<Stage>,
    pub worktree: String,
    pub branch: String,
    pub base_sha: String,
    pub merge_target: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_sha: Option<String>,
    pub pushed: bool,
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_removed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_removed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_head_at_removal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cleanup_blocked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cleanup_trigger: Option<String>,
    #[serde(default)]
    pub attempts: Vec<Attempt>,
}

/// On-disk shape of `.markplane/.runs/<TASK-id>.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RunRecord {
    pub runs: Vec<Run>,
}

/// Path to `.markplane/.runs/<task_id>.json`.
pub fn run_record_path(markplane_root: &Path, task_id: &str) -> PathBuf {
    markplane_root.join(".runs").join(format!("{task_id}.json"))
}

/// Ensure `.runs/` exists with a self-contained `.gitignore` of `*`.
pub fn ensure_runs_dir(markplane_root: &Path) -> Result<PathBuf> {
    let runs_dir = markplane_root.join(".runs");
    fs::create_dir_all(&runs_dir)?;
    let ignore_path = runs_dir.join(".gitignore");
    if !ignore_path.exists() {
        fs::write(&ignore_path, "*\n")?;
    }
    Ok(runs_dir)
}

/// Read a Run record. Returns `Ok(None)` when the file is absent.
pub fn read_run_record(markplane_root: &Path, task_id: &str) -> Result<Option<RunRecord>> {
    let path = run_record_path(markplane_root, task_id);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path)?;
    let record: RunRecord = serde_json::from_slice(&bytes).map_err(|e| {
        MarkplaneError::Config(format!(
            "Invalid Run record {}: {e}",
            path.display()
        ))
    })?;
    Ok(Some(record))
}

/// Atomically write a Run record (tempfile + rename). Creates `.runs/` first.
pub fn write_run_record(markplane_root: &Path, task_id: &str, record: &RunRecord) -> Result<()> {
    ensure_runs_dir(markplane_root)?;
    let path = run_record_path(markplane_root, task_id);
    let bytes = serde_json::to_vec_pretty(record).map_err(|e| {
        MarkplaneError::Config(format!("Failed to serialize Run record: {e}"))
    })?;
    atomic_write(&path, &bytes)
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or_else(|| {
        MarkplaneError::Config(format!(
            "Cannot determine parent directory of {}",
            path.display()
        ))
    })?;
    let mut tmp = NamedTempFile::new_in(parent)?;
    tmp.write_all(content)?;
    tmp.persist(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_record() -> RunRecord {
        RunRecord {
            runs: vec![Run {
                seq: 1,
                status: RunStatus::Succeeded,
                current_stage: Some(Stage::Merge),
                worktree: "/tmp/wt".into(),
                branch: "agent/TASK-abc12".into(),
                base_sha: "deadbeef".into(),
                merge_target: "main".into(),
                merge_sha: Some("cafebabe".into()),
                pushed: false,
                started_at: "2026-10-10T09:00:00+08:00".into(),
                ended_at: Some("2026-10-10T10:00:00+08:00".into()),
                failure_reason: None,
                worktree_removed_at: None,
                branch_removed_at: None,
                branch_head_at_removal: None,
                cleanup_blocked: None,
                cleanup_trigger: None,
                attempts: vec![Attempt {
                    stage: Stage::Implement,
                    model: "claude-opus-4.5".into(),
                    timeout_secs: Some(3600),
                    session_id: Some("sess-1".into()),
                    workspace: "/tmp/wt".into(),
                    transcript_path: None,
                    pid: Some(1234),
                    process_group_id: Some(1234),
                    agent_config_dir: Some("/tmp/agent-config".into()),
                    trigger: Trigger::Initial,
                    started_at: "2026-10-10T09:00:01+08:00".into(),
                    ended_at: Some("2026-10-10T09:30:00+08:00".into()),
                    exit_code: Some(0),
                    verdict: Some("REVIEW: clean".into()),
                    result: Some(AttemptResult::Ok),
                    usage: Some(Usage {
                        input_tokens: Some(100),
                        output_tokens: Some(20),
                        cache_read_tokens: Some(80),
                        cache_write_tokens: Some(10),
                    }),
                    duration_api_ms: Some(12_000),
                    stderr_tail: None,
                }],
            }],
        }
    }

    #[test]
    fn read_returns_none_when_file_missing() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".markplane");
        fs::create_dir_all(&root).unwrap();
        assert!(read_run_record(&root, "TASK-abc12").unwrap().is_none());
    }

    #[test]
    fn write_creates_gitignore_and_round_trips() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".markplane");
        fs::create_dir_all(&root).unwrap();
        let record = sample_record();

        write_run_record(&root, "TASK-abc12", &record).unwrap();

        let ignore = root.join(".runs").join(".gitignore");
        assert_eq!(fs::read_to_string(&ignore).unwrap(), "*\n");

        let loaded = read_run_record(&root, "TASK-abc12").unwrap().unwrap();
        assert_eq!(loaded, record);
    }

    #[test]
    fn usage_serializes_camel_case_token_fields() {
        let usage = Usage {
            input_tokens: Some(1),
            output_tokens: Some(2),
            cache_read_tokens: None,
            cache_write_tokens: Some(3),
        };
        let json = serde_json::to_value(&usage).unwrap();
        assert_eq!(json["inputTokens"], 1);
        assert_eq!(json["outputTokens"], 2);
        assert!(json["cacheReadTokens"].is_null());
        assert_eq!(json["cacheWriteTokens"], 3);
    }

    #[test]
    fn stage_order_is_implement_fix_merge() {
        assert_eq!(
            STAGE_ORDER,
            &[Stage::Implement, Stage::Fix, Stage::Merge]
        );
    }
}
