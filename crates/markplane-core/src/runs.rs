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
    /// Last cleanup skip reason (`uncommitted_changes`, `unmerged_commits`,
    /// `orphan_running`, `not_a_worktree`); cleared after a successful cleanup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cleanup_blocked: Option<String>,
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

/// Whether this serve still holds the Run in memory, and lock contention.
///
/// Used with [`run_display_status`] to distinguish `interrupted` from
/// `running elsewhere` for non-terminal records after a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMemoryPresence {
    /// This serve tracks the Run (holds its Slot).
    InMemory,
    /// Not in this serve's memory; Dispatcher lock is free or held by us idle.
    Absent,
    /// Not in memory; another process holds the Dispatcher lock.
    HeldElsewhere,
}

/// Server-side display status for a Run (UI copies this string; client only
/// additionally derives `stopping`).
///
/// Returns one of: `running`, `merge-queued`, `interrupted`, `running elsewhere`,
/// `failed`, `stopped`, `succeeded`.
pub fn run_display_status(run: &Run, presence: RunMemoryPresence) -> &'static str {
    match run.status {
        RunStatus::Succeeded => "succeeded",
        RunStatus::Failed => "failed",
        RunStatus::Stopped => "stopped",
        RunStatus::Running | RunStatus::MergeQueued => match presence {
            RunMemoryPresence::InMemory => match run.status {
                RunStatus::MergeQueued => "merge-queued",
                _ => "running",
            },
            RunMemoryPresence::HeldElsewhere => "running elsewhere",
            RunMemoryPresence::Absent => "interrupted",
        },
    }
}

/// Start time of the current Stage (first attempt of `current_stage`), else Run start.
pub fn stage_started_at(run: &Run) -> String {
    let stage = match run.current_stage {
        Some(s) => s,
        None => return run.started_at.clone(),
    };
    run.attempts
        .iter()
        .find(|a| a.stage == stage)
        .map(|a| a.started_at.clone())
        .unwrap_or_else(|| run.started_at.clone())
}

/// Latest attempt for `current_stage` (model / timeout for RunBadge hover).
fn current_stage_attempt(run: &Run) -> Option<&Attempt> {
    let stage = run.current_stage?;
    run.attempts.iter().rev().find(|a| a.stage == stage)
}

/// Count of attempts for the current Stage.
fn current_stage_attempt_count(run: &Run) -> u32 {
    let Some(stage) = run.current_stage else {
        return 0;
    };
    run.attempts.iter().filter(|a| a.stage == stage).count() as u32
}

/// One-row summary of a ticket's latest Run for `GET /api/runs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunSummary {
    pub task_id: String,
    pub seq: u32,
    /// Display status from [`run_display_status`].
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_started_at: Option<String>,
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_position: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    /// Current Stage model (hover tip).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Current Stage timeout snapshot in seconds (hover tip).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    /// Attempts recorded for the current Stage (hover tip).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_attempt_count: Option<u32>,
}

/// Build a [`RunSummary`] for the latest Run in a record.
pub fn run_summary(
    task_id: &str,
    run: &Run,
    presence: RunMemoryPresence,
    queue_position: Option<u32>,
) -> RunSummary {
    let status = run_display_status(run, presence).to_string();
    let stage = run.current_stage.map(|s| s.as_str().to_string());
    let attempt = current_stage_attempt(run);
    let stage_attempt_count = current_stage_attempt_count(run);
    RunSummary {
        task_id: task_id.to_string(),
        seq: run.seq,
        status,
        stage,
        stage_started_at: Some(stage_started_at(run)),
        started_at: run.started_at.clone(),
        queue_position,
        failure_reason: run.failure_reason.clone(),
        ended_at: run.ended_at.clone(),
        model: attempt.map(|a| a.model.clone()),
        timeout_secs: attempt.and_then(|a| a.timeout_secs),
        stage_attempt_count: if stage_attempt_count > 0 {
            Some(stage_attempt_count)
        } else {
            None
        },
    }
}

/// List all on-disk Run records as `(task_id, record)` pairs.
///
/// Skips non-`.json` entries and unreadable files. Sorted by task_id.
pub fn list_run_records(markplane_root: &Path) -> Result<Vec<(String, RunRecord)>> {
    let runs_dir = markplane_root.join(".runs");
    if !runs_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&runs_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".json") {
            continue;
        }
        let task_id = name.trim_end_matches(".json").to_string();
        match read_run_record(markplane_root, &task_id)? {
            Some(record) if !record.runs.is_empty() => out.push((task_id, record)),
            _ => {}
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
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

    fn bare_run(status: RunStatus, stage: Option<Stage>) -> Run {
        Run {
            seq: 1,
            status,
            current_stage: stage,
            worktree: "/tmp/wt".into(),
            branch: "agent/TASK-x".into(),
            base_sha: "abc".into(),
            merge_target: "main".into(),
            merge_sha: None,
            pushed: false,
            started_at: "2026-10-10T09:00:00+08:00".into(),
            ended_at: None,
            failure_reason: None,
            worktree_removed_at: None,
            branch_removed_at: None,
            branch_head_at_removal: None,
            cleanup_blocked: None,
            cleanup_trigger: None,
            attempts: vec![],
        }
    }

    #[test]
    fn display_status_terminal_ignores_presence() {
        let failed = bare_run(RunStatus::Failed, Some(Stage::Merge));
        assert_eq!(
            run_display_status(&failed, RunMemoryPresence::InMemory),
            "failed"
        );
        assert_eq!(
            run_display_status(&failed, RunMemoryPresence::Absent),
            "failed"
        );
        assert_eq!(
            run_display_status(&failed, RunMemoryPresence::HeldElsewhere),
            "failed"
        );

        let stopped = bare_run(RunStatus::Stopped, Some(Stage::Implement));
        assert_eq!(
            run_display_status(&stopped, RunMemoryPresence::Absent),
            "stopped"
        );

        let ok = bare_run(RunStatus::Succeeded, Some(Stage::Merge));
        assert_eq!(
            run_display_status(&ok, RunMemoryPresence::InMemory),
            "succeeded"
        );
    }

    #[test]
    fn display_status_non_terminal_by_presence() {
        let running = bare_run(RunStatus::Running, Some(Stage::Fix));
        assert_eq!(
            run_display_status(&running, RunMemoryPresence::InMemory),
            "running"
        );
        assert_eq!(
            run_display_status(&running, RunMemoryPresence::Absent),
            "interrupted"
        );
        assert_eq!(
            run_display_status(&running, RunMemoryPresence::HeldElsewhere),
            "running elsewhere"
        );

        let queued = bare_run(RunStatus::MergeQueued, Some(Stage::Merge));
        assert_eq!(
            run_display_status(&queued, RunMemoryPresence::InMemory),
            "merge-queued"
        );
        assert_eq!(
            run_display_status(&queued, RunMemoryPresence::Absent),
            "interrupted"
        );
        assert_eq!(
            run_display_status(&queued, RunMemoryPresence::HeldElsewhere),
            "running elsewhere"
        );
    }

    #[test]
    fn list_run_records_returns_sorted_task_ids() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join(".markplane");
        fs::create_dir_all(&root).unwrap();

        write_run_record(&root, "TASK-bbb22", &sample_record()).unwrap();
        write_run_record(&root, "TASK-aaa11", &sample_record()).unwrap();

        let listed = list_run_records(&root).unwrap();
        assert_eq!(
            listed
                .iter()
                .map(|(id, _)| id.as_str())
                .collect::<Vec<_>>(),
            vec!["TASK-aaa11", "TASK-bbb22"]
        );
    }

    #[test]
    fn run_summary_includes_stage_timing_and_model() {
        let mut run = bare_run(RunStatus::Running, Some(Stage::Implement));
        run.attempts.push(Attempt {
            stage: Stage::Implement,
            model: "cursor-grok-4.5-high".into(),
            timeout_secs: Some(3600),
            session_id: Some("s1".into()),
            workspace: "/tmp/wt".into(),
            transcript_path: None,
            pid: None,
            process_group_id: None,
            agent_config_dir: None,
            trigger: Trigger::Initial,
            started_at: "2026-10-10T09:05:00+08:00".into(),
            ended_at: None,
            exit_code: None,
            verdict: None,
            result: None,
            usage: None,
            duration_api_ms: None,
            stderr_tail: None,
        });
        let summary = run_summary("TASK-abc12", &run, RunMemoryPresence::InMemory, None);
        assert_eq!(summary.status, "running");
        assert_eq!(summary.stage.as_deref(), Some("implement"));
        assert_eq!(
            summary.stage_started_at.as_deref(),
            Some("2026-10-10T09:05:00+08:00")
        );
        assert_eq!(summary.model.as_deref(), Some("cursor-grok-4.5-high"));
        assert_eq!(summary.timeout_secs, Some(3600));
        assert_eq!(summary.stage_attempt_count, Some(1));
    }
}
