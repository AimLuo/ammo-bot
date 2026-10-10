//! Failure → human handoff and silent-stop helpers for the Dispatcher engine.

use chrono::Utc;
use markplane_core::{
    MarkplaneDocument, Project, READY_FOR_AGENT_TAG, READY_FOR_HUMAN_TAG, Run, RunRecord,
    RunStatus, Stage, StatusCategory, Task, UpdateFields, write_run_record,
};
use serde_json::json;
use tokio::sync::broadcast;

/// Whether the Dispatcher still owns this ticket (active + assignee dispatcher).
pub fn ticket_still_ours(project: &Project, task_id: &str) -> Result<bool, String> {
    let config = project.load_config().map_err(|e| e.to_string())?;
    let doc: MarkplaneDocument<Task> = project.read_item(task_id).map_err(|e| e.to_string())?;
    Ok(
        config
            .workflows
            .task
            .category_of(&doc.frontmatter.status)
            == Some(StatusCategory::Active)
            && doc.frontmatter.assignee.as_deref() == Some("dispatcher"),
    )
}

/// Build the dispatcher-authored failure comment. First line is one sentence
/// with Stage and session id.
pub fn failure_comment_text(stage: Stage, session_id: Option<&str>, reason: &str) -> String {
    let sid = session_id.unwrap_or("unknown");
    format!(
        "{} stage failed in session {}: {}",
        stage.as_str(),
        sid,
        reason
    )
}

/// Mark Run failed and hand the ticket to a human (tags + comment). Keeps
/// status active and assignee unchanged. No-op on the ticket when ownership
/// was already taken away (silent stop path should be used instead).
pub async fn handoff_failure(
    project: &Project,
    ws_tx: &broadcast::Sender<String>,
    task_id: &str,
    record: &mut RunRecord,
    run: &mut Run,
    stage: Stage,
    reason: String,
) -> Result<(), String> {
    let session_id = run
        .attempts
        .iter()
        .rev()
        .find(|a| a.stage == stage)
        .and_then(|a| a.session_id.clone())
        .or_else(|| {
            run.attempts
                .last()
                .and_then(|a| a.session_id.clone())
        });

    run.status = RunStatus::Failed;
    run.current_stage = Some(stage);
    run.failure_reason = Some(reason.clone());
    run.ended_at = Some(Utc::now().to_rfc3339());
    if let Some(last) = record.runs.last_mut() {
        *last = run.clone();
    }
    write_run_record(project.root(), task_id, record).map_err(|e| e.to_string())?;
    broadcast_run_changed(ws_tx, task_id, run);

    if !ticket_still_ours(project, task_id)? {
        // Human already took the ticket — do not rewrite tags or comment.
        return Ok(());
    }

    let comment = failure_comment_text(stage, session_id.as_deref(), &reason);
    project
        .add_comment(task_id, "dispatcher", &comment)
        .map_err(|e| e.to_string())?;

    project
        .update_item(
            task_id,
            UpdateFields {
                remove_tags: vec![READY_FOR_AGENT_TAG.into()],
                add_tags: vec![READY_FOR_HUMAN_TAG.into()],
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Record Run as stopped without touching the ticket.
pub async fn silent_stop(
    project: &Project,
    ws_tx: &broadcast::Sender<String>,
    task_id: &str,
    record: &mut RunRecord,
    run: &mut Run,
) -> Result<(), String> {
    run.status = RunStatus::Stopped;
    run.ended_at = Some(Utc::now().to_rfc3339());
    run.current_stage = None;
    if let Some(last) = record.runs.last_mut() {
        *last = run.clone();
    }
    write_run_record(project.root(), task_id, record).map_err(|e| e.to_string())?;
    broadcast_run_changed(ws_tx, task_id, run);
    Ok(())
}

fn broadcast_run_changed(ws_tx: &broadcast::Sender<String>, task_id: &str, run: &Run) {
    let stage = run
        .current_stage
        .map(|s| s.as_str().to_string())
        .unwrap_or_default();
    let display = match run.status {
        RunStatus::Running => "running",
        RunStatus::MergeQueued => "merge-queued",
        RunStatus::Succeeded => "succeeded",
        RunStatus::Failed => "failed",
        RunStatus::Stopped => "stopped",
    };
    let msg = json!({
        "type": "run_changed",
        "task_id": task_id,
        "seq": run.seq,
        "status": display,
        "stage": stage,
    })
    .to_string();
    let _ = ws_tx.send(msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failure_comment_leads_with_stage_and_session() {
        let text = failure_comment_text(Stage::Implement, Some("sess-1"), "no new commits");
        let first = text.lines().next().unwrap();
        assert!(first.contains("implement"));
        assert!(first.contains("sess-1"));
        assert!(first.contains("no new commits"));
        assert_eq!(text.lines().count(), 1);
    }
}
