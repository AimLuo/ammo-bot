//! Comment & continue: failure-class routing and session resume planning.

use std::path::Path;

use markplane_core::{
    AttemptResult, Run, RunStatus, Stage, Trigger, agent_config_dir, session_meta_exists,
};

/// How to continue a failed or interrupted Run after a human comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinuePlan {
    /// Enter merge queue; finish success if the target already contains the branch.
    MergeQueue,
    /// Run agent for `stage`, optionally `--resume` an existing session.
    RunAgent {
        stage: Stage,
        resume_session_id: Option<String>,
    },
}

/// Failure criteria 1–6 from the Dispatcher epic, used only for continue routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinueFailureClass {
    /// Criteria 1 / 6 — retry the Stage that failed (nonzero exit / timeout).
    SameStage,
    /// Criteria 2 / 3 — implement (illegal REVIEW / no new commits).
    Implement,
    /// Criterion 4 — fix (not `FIX: done`).
    Fix,
    /// Criterion 5 — merge queue path (verdict / ancestor / dirty-tree style).
    MergeQueue,
}

/// Classify a failed Run's reason for continue Stage routing.
pub fn classify_continue_failure(run: &Run) -> ContinueFailureClass {
    let stage = run.current_stage.unwrap_or(Stage::Implement);
    let reason = run.failure_reason.as_deref().unwrap_or("");
    let last = run.attempts.last();

    // Interrupted (non-failed): continue the current Stage.
    if run.status != RunStatus::Failed {
        return ContinueFailureClass::SameStage;
    }

    // Criterion 6 — timeout.
    if last.is_some_and(|a| a.result == Some(AttemptResult::Timeout))
        || reason.contains("stage timeout")
    {
        return ContinueFailureClass::SameStage;
    }

    // Criterion 1 — agent nonzero exit (before verdict-based reasons).
    if reason.contains("agent failed") {
        return ContinueFailureClass::SameStage;
    }

    // Criterion 3.
    if reason.contains("no new commits") {
        return ContinueFailureClass::Implement;
    }

    // Criterion 2.
    if reason.contains("without a legal REVIEW")
        || reason.contains("illegal verdict")
        || reason.contains("implement blocked")
    {
        return ContinueFailureClass::Implement;
    }

    // Criterion 4.
    if reason.contains("without FIX: done") {
        return ContinueFailureClass::Fix;
    }

    // Criterion 5 — merge post-conditions / MERGE: failed (not agent exit/timeout).
    if stage == Stage::Merge
        && (reason.contains("without MERGE: done")
            || reason.contains("does not contain")
            || reason.contains("MERGE: failed"))
    {
        return ContinueFailureClass::MergeQueue;
    }

    // Default: same Stage (covers merge agent failures already handled above,
    // and any unclassified reason on the recorded Stage).
    match stage {
        Stage::Implement => ContinueFailureClass::Implement,
        Stage::Fix => ContinueFailureClass::Fix,
        Stage::Merge => ContinueFailureClass::MergeQueue,
    }
}

/// Plan the next continue action. `config_dir` is used to check session meta for `--resume`.
pub fn plan_continue(run: &Run, config_dir: &Path) -> Result<ContinuePlan, String> {
    match run.status {
        RunStatus::Succeeded | RunStatus::Stopped => {
            return Err("run is not continuable".into());
        }
        RunStatus::Failed | RunStatus::Running | RunStatus::MergeQueued => {}
    }

    let class = classify_continue_failure(run);
    let stage = match class {
        ContinueFailureClass::SameStage => run.current_stage.unwrap_or(Stage::Implement),
        ContinueFailureClass::Implement => Stage::Implement,
        ContinueFailureClass::Fix => Stage::Fix,
        ContinueFailureClass::MergeQueue => {
            return Ok(ContinuePlan::MergeQueue);
        }
    };

    // Prefer --resume when the last attempt for this Stage has a live session and
    // no completed result (interrupted) or when the failed attempt still has meta.
    let resume_session_id = resume_session_for_stage(run, stage, config_dir);

    Ok(ContinuePlan::RunAgent {
        stage,
        resume_session_id,
    })
}

fn resume_session_for_stage(run: &Run, stage: Stage, config_dir: &Path) -> Option<String> {
    let last = run
        .attempts
        .iter()
        .rev()
        .find(|a| a.stage == stage)?;
    let sid = last.session_id.as_ref().filter(|s| !s.is_empty())?;
    // Resume only when meta.json still exists (same rule as Resume).
    if !session_meta_exists(config_dir, &last.workspace, sid) {
        return None;
    }
    // For a completed failed attempt, still allow --resume so the continue
    // comment can be injected into the same conversation when meta remains.
    Some(sid.clone())
}

/// Count of `trigger: continue` attempts on a Run (for "continued n×" UI).
#[allow(dead_code)] // mirrored in the web UI; kept for server-side helpers/tests
pub fn continue_count(run: &Run) -> usize {
    run.attempts
        .iter()
        .filter(|a| a.trigger == Trigger::Continue)
        .count()
}

/// Default config dir helper for callers that don't inject one.
#[allow(dead_code)]
pub fn plan_continue_default(run: &Run) -> Result<ContinuePlan, String> {
    plan_continue(run, &agent_config_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use markplane_core::{Attempt, Trigger, Usage};

    fn attempt(stage: Stage, result: Option<AttemptResult>, sid: Option<&str>) -> Attempt {
        Attempt {
            stage,
            model: "fake".into(),
            timeout_secs: Some(60),
            session_id: sid.map(str::to_string),
            workspace: "/tmp/wt".into(),
            transcript_path: None,
            pid: None,
            process_group_id: None,
            agent_config_dir: None,
            trigger: Trigger::Initial,
            started_at: "2026-10-10T09:00:00+08:00".into(),
            ended_at: Some("2026-10-10T09:30:00+08:00".into()),
            exit_code: Some(1),
            verdict: None,
            result,
            usage: None::<Usage>,
            duration_api_ms: None,
            stderr_tail: None,
        }
    }

    fn failed_run(stage: Stage, reason: &str, a: Attempt) -> Run {
        Run {
            seq: 1,
            status: RunStatus::Failed,
            current_stage: Some(stage),
            worktree: "/tmp/wt".into(),
            branch: "agent/TASK-x".into(),
            base_sha: "abc".into(),
            merge_target: "main".into(),
            merge_sha: None,
            pushed: false,
            started_at: "2026-10-10T09:00:00+08:00".into(),
            ended_at: Some("2026-10-10T09:30:00+08:00".into()),
            failure_reason: Some(reason.into()),
            worktree_removed_at: None,
            branch_removed_at: None,
            branch_head_at_removal: None,
            cleanup_blocked: None,
            cleanup_trigger: None,
            attempts: vec![a],
        }
    }

    #[test]
    fn classifies_timeout_as_same_stage() {
        let mut a = attempt(Stage::Fix, Some(AttemptResult::Timeout), Some("s1"));
        a.exit_code = None;
        let run = failed_run(Stage::Fix, "fix stage timeout", a);
        assert_eq!(
            classify_continue_failure(&run),
            ContinueFailureClass::SameStage
        );
        let plan = plan_continue(&run, Path::new("/tmp/no-meta")).unwrap();
        assert_eq!(
            plan,
            ContinuePlan::RunAgent {
                stage: Stage::Fix,
                resume_session_id: None,
            }
        );
    }

    #[test]
    fn classifies_agent_exit_as_same_stage() {
        let run = failed_run(
            Stage::Implement,
            "implement agent failed (exit 1, verdict=<none>)",
            attempt(Stage::Implement, Some(AttemptResult::Failed), Some("s1")),
        );
        assert_eq!(
            classify_continue_failure(&run),
            ContinueFailureClass::SameStage
        );
    }

    #[test]
    fn classifies_illegal_review_and_no_commits_as_implement() {
        let a = attempt(Stage::Implement, Some(AttemptResult::Ok), Some("s1"));
        let r1 = failed_run(
            Stage::Implement,
            "implement ended without a legal REVIEW: clean|issues line",
            a.clone(),
        );
        assert_eq!(
            classify_continue_failure(&r1),
            ContinueFailureClass::Implement
        );
        let r2 = failed_run(
            Stage::Implement,
            "implement produced no new commits",
            a,
        );
        assert_eq!(
            classify_continue_failure(&r2),
            ContinueFailureClass::Implement
        );
    }

    #[test]
    fn classifies_fix_verdict_as_fix() {
        let run = failed_run(
            Stage::Fix,
            "fix ended without FIX: done (got FIX: blocked)",
            attempt(Stage::Fix, Some(AttemptResult::Ok), Some("s1")),
        );
        assert_eq!(classify_continue_failure(&run), ContinueFailureClass::Fix);
    }

    #[test]
    fn classifies_merge_postcondition_as_merge_queue() {
        let run = failed_run(
            Stage::Merge,
            "merge ended without MERGE: done (got MERGE: failed)",
            attempt(Stage::Merge, Some(AttemptResult::Ok), Some("s1")),
        );
        assert_eq!(
            classify_continue_failure(&run),
            ContinueFailureClass::MergeQueue
        );
        assert_eq!(
            plan_continue(&run, Path::new("/tmp")).unwrap(),
            ContinuePlan::MergeQueue
        );
    }

    #[test]
    fn continue_count_counts_trigger() {
        let mut run = failed_run(
            Stage::Implement,
            "x",
            attempt(Stage::Implement, Some(AttemptResult::Failed), None),
        );
        assert_eq!(continue_count(&run), 0);
        let mut c = attempt(Stage::Implement, Some(AttemptResult::Ok), None);
        c.trigger = Trigger::Continue;
        run.attempts.push(c);
        assert_eq!(continue_count(&run), 1);
    }
}
