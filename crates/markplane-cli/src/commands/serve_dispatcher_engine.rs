//! Dispatcher engine: scan → pickup → implement → (fix) → merge → done,
//! plus Stage failure → human handoff and silent stop at Stage boundaries.
//!
//! Kept out of `serve.rs` on purpose.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use std::fs::TryLockError;
use markplane_core::{
    Attempt, AttemptResult, EffectiveDispatcherSettings, ImplementVerdict, MarkplaneDocument,
    Patch, Project, QueryFilter, Run, RunRecord, RunStatus, Stage, StatusCategory, StreamEvent,
    Task, Trigger, UpdateFields, agent_invocation_succeeded, classify_implement_verdict,
    filter_and_sort_pickup_candidates, implement_verdict_failure_reason, is_fix_done,
    is_merge_done, latest_agent_implement_review, parse_stream_json_line, read_run_record,
    round_robin_pickup, usage_from_events, verdict_from_events, write_run_record,
};
use serde_json::json;
use tokio::sync::{Mutex, broadcast};

use super::serve_dispatcher_handoff::{self, ticket_still_ours};
use super::serve_dispatcher_prompts::{self, PromptContext};

/// Scan interval when Scan switch is on.
pub const SCAN_INTERVAL: Duration = Duration::from_secs(15);

/// Consecutive Stage ends without `system/init` before Scan auto-closes.
pub const STARTUP_FAIL_THRESHOLD: u32 = 3;

/// How long to wait after SIGTERM before SIGKILL.
const KILL_GRACE: Duration = Duration::from_secs(10);

/// Per-serve bookkeeping for Scan auto-close (startup fail + lock lost).
#[derive(Debug, Default)]
pub struct ScanAutoCloseState {
    /// project_id → consecutive Stage ends that never saw `system/init`.
    startup_fail_streak: HashMap<String, u32>,
    /// project_id → human-readable reason (cleared on manual reopen).
    auto_close_reason: HashMap<String, String>,
    /// project_ids flipped off immediately; drained when serve applies close.
    immediate_off: HashSet<String>,
}

impl ScanAutoCloseState {
    pub fn reason(&self, project_id: &str) -> Option<&str> {
        self.auto_close_reason.get(project_id).map(String::as_str)
    }

    pub fn is_immediate_off(&self, project_id: &str) -> bool {
        self.immediate_off.contains(project_id)
    }

    pub fn clear_on_manual_open(&mut self, project_id: &str) {
        self.auto_close_reason.remove(project_id);
        self.startup_fail_streak.remove(project_id);
        self.immediate_off.remove(project_id);
    }

    /// Record a Stage end. Returns a close reason when the threshold is hit.
    pub fn note_stage_ended(&mut self, project_id: &str, saw_init: bool) -> Option<String> {
        if saw_init {
            self.startup_fail_streak.remove(project_id);
            return None;
        }
        let streak = self
            .startup_fail_streak
            .entry(project_id.to_string())
            .or_insert(0);
        *streak = streak.saturating_add(1);
        if *streak < STARTUP_FAIL_THRESHOLD {
            return None;
        }
        self.startup_fail_streak.remove(project_id);
        let reason = format!(
            "Scan closed: {STARTUP_FAIL_THRESHOLD} consecutive Stage starts failed before system/init"
        );
        self.mark_closed(project_id, reason)
    }

    /// Lock file identity mismatch → close reason.
    pub fn note_lock_lost(&mut self, project_id: &str) -> String {
        self.startup_fail_streak.remove(project_id);
        let reason =
            "Scan closed: Dispatcher lock file was deleted or replaced".to_string();
        self.mark_closed(project_id, reason)
            .expect("mark_closed always returns Some")
    }

    fn mark_closed(&mut self, project_id: &str, reason: String) -> Option<String> {
        self.auto_close_reason
            .insert(project_id.to_string(), reason.clone());
        self.immediate_off.insert(project_id.to_string());
        Some(reason)
    }

    pub fn clear_immediate_off(&mut self, project_id: &str) {
        self.immediate_off.remove(project_id);
    }
}

#[cfg(test)]
mod scan_auto_close_tests {
    use super::*;

    #[test]
    fn three_no_init_closes_and_init_resets_streak() {
        let mut s = ScanAutoCloseState::default();
        assert!(s.note_stage_ended("p1", false).is_none());
        assert!(s.note_stage_ended("p1", false).is_none());
        // Successful init clears streak.
        assert!(s.note_stage_ended("p1", true).is_none());
        assert!(s.note_stage_ended("p1", false).is_none());
        assert!(s.note_stage_ended("p1", false).is_none());
        let reason = s.note_stage_ended("p1", false).expect("third no-init closes");
        assert!(reason.contains("system/init"));
        assert_eq!(s.reason("p1"), Some(reason.as_str()));
        assert!(s.is_immediate_off("p1"));

        s.clear_on_manual_open("p1");
        assert!(s.reason("p1").is_none());
        assert!(!s.is_immediate_off("p1"));
    }

    #[test]
    fn lock_lost_sets_reason() {
        let mut s = ScanAutoCloseState::default();
        let reason = s.note_lock_lost("p1");
        assert!(reason.contains("lock"));
        assert_eq!(s.reason("p1"), Some(reason.as_str()));
    }
}

/// Slot + merge-queue bookkeeping shared across Projects in one serve.
#[derive(Default)]
pub struct EngineState {
    /// task_id → project_id for Runs holding a Slot.
    pub active_slots: HashMap<String, String>,
    /// Per worktree git-dir realpath: FIFO of task_ids waiting / holding merge.
    merge_queues: HashMap<PathBuf, Arc<MergeQueue>>,
    /// Tasks currently being processed (pickup through end) — skip re-pickup.
    in_flight: HashSet<String>,
    /// Next Project index for cross-Project round-robin (stable project_id order).
    rr_start: usize,
    /// Fan-out targets for `dispatcher_changed` (refreshed each tick / by serve).
    pub dispatcher_channels: Vec<broadcast::Sender<String>>,
}

impl EngineState {
    pub fn slots_used(&self) -> usize {
        self.active_slots.len()
    }

    pub fn project_has_active_runs(&self, project_id: &str) -> bool {
        self.active_slots.values().any(|p| p == project_id)
    }

    /// 1-based merge-queue position for `task_id`, or `None` if not queued.
    pub async fn merge_queue_position(&self, task_id: &str) -> Option<u32> {
        for queue in self.merge_queues.values() {
            let g = queue.inner.lock().await;
            if let Some(pos) = g.waiters.iter().position(|t| t == task_id) {
                return Some((pos + 1) as u32);
            }
        }
        None
    }

    pub fn broadcast_dispatcher_changed(&self) {
        let msg = r#"{"type":"dispatcher_changed"}"#.to_string();
        for tx in &self.dispatcher_channels {
            let _ = tx.send(msg.clone());
        }
    }

    fn merge_queue(&mut self, git_dir: PathBuf) -> Arc<MergeQueue> {
        self.merge_queues
            .entry(git_dir)
            .or_insert_with(|| Arc::new(MergeQueue::default()))
            .clone()
    }
}

#[derive(Default)]
struct MergeQueue {
    inner: Mutex<MergeQueueInner>,
}

#[derive(Default)]
struct MergeQueueInner {
    /// FIFO of waiters; front holds the merge lock when `busy`.
    waiters: VecDeque<String>,
    busy: bool,
}

impl MergeQueue {
    async fn enter(&self, task_id: &str) -> MergePermit<'_> {
        loop {
            {
                let mut g = self.inner.lock().await;
                if !g.waiters.iter().any(|t| t == task_id) {
                    g.waiters.push_back(task_id.to_string());
                }
                if g.waiters.front().map(|s| s.as_str()) == Some(task_id) && !g.busy {
                    g.busy = true;
                    return MergePermit {
                        queue: self,
                        task_id: task_id.to_string(),
                    };
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

struct MergePermit<'a> {
    queue: &'a MergeQueue,
    task_id: String,
}

impl Drop for MergePermit<'_> {
    fn drop(&mut self) {
        // Best-effort sync release; engine also clears via async path.
        // Use try_lock to avoid blocking Drop; if contended, the next enter loop heals.
        if let Ok(mut g) = self.queue.inner.try_lock() {
            g.busy = false;
            if g.waiters.front().map(|s| s.as_str()) == Some(self.task_id.as_str()) {
                g.waiters.pop_front();
            } else {
                g.waiters.retain(|t| t != &self.task_id);
            }
        }
    }
}

/// Inputs for one Project on a scan tick.
#[derive(Clone)]
pub struct ProjectTickInput {
    pub project_id: String,
    pub project: Project,
    pub project_dir: PathBuf,
    pub agent_bin: PathBuf,
    pub ws_tx: broadcast::Sender<String>,
    pub concurrency: u32,
    pub settings: EffectiveDispatcherSettings,
    pub scan_on: bool,
    /// When set (tests), overrides every Stage timeout in seconds.
    pub test_timeout_secs: Option<u64>,
    /// Serve URL written into `merge.json` (display only).
    pub serve_url: String,
    /// Shared Scan auto-close bookkeeping (startup fail streak / reasons).
    pub scan_auto_close: Arc<Mutex<ScanAutoCloseState>>,
    /// Notify serve to flip Scan off, log, and broadcast (`project_id`, reason).
    pub auto_close_tx: tokio::sync::mpsc::UnboundedSender<(String, String)>,
}

/// Explicit scan tick (tests call this; production uses a 15s interval).
pub async fn dispatcher_tick(engine: Arc<Mutex<EngineState>>, mut projects: Vec<ProjectTickInput>) {
    projects.retain(|p| p.scan_on);
    if projects.is_empty() {
        return;
    }
    // Stable order so rr_start indexes stay meaningful across ticks.
    projects.sort_by(|a, b| a.project_id.cmp(&b.project_id));

    let concurrency = projects[0].concurrency;
    let free = {
        let g = engine.lock().await;
        let used = g.slots_used() as u32;
        concurrency.saturating_sub(used) as usize
    };
    if free == 0 {
        return;
    }

    let mut queues: Vec<Vec<String>> = Vec::with_capacity(projects.len());
    {
        let g = engine.lock().await;
        for proj in &projects {
            let ids = match candidate_ids(proj) {
                Some(ids) => ids,
                None => {
                    queues.push(Vec::new());
                    continue;
                }
            };
            let available: Vec<String> = ids
                .into_iter()
                .filter(|id| !g.in_flight.contains(id) && !g.active_slots.contains_key(id))
                .collect();
            queues.push(available);
        }
    }

    let picks = {
        let mut g = engine.lock().await;
        let (picks, next) = round_robin_pickup(&queues, free, g.rr_start);
        g.rr_start = next;
        picks
    };

    if picks.is_empty() {
        return;
    }

    let mut started = false;
    for (proj_idx, task_id) in picks {
        let Some(proj) = projects.get(proj_idx).cloned() else {
            continue;
        };
        if !claim_slot(engine.clone(), &proj, &task_id).await {
            break;
        }
        started = true;
        spawn_pipeline(engine.clone(), proj, task_id);
    }
    if started {
        engine.lock().await.broadcast_dispatcher_changed();
    }
}

fn candidate_ids(proj: &ProjectTickInput) -> Option<Vec<String>> {
    let config = match proj.project.load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("dispatcher: load_config {}: {e}", proj.project_id);
            return None;
        }
    };
    let tasks = match proj.project.list_tasks(&QueryFilter::default()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("dispatcher: list_tasks {}: {e}", proj.project_id);
            return None;
        }
    };
    Some(
        filter_and_sort_pickup_candidates(&tasks, &config.workflows.task)
            .into_iter()
            .map(|d| d.frontmatter.id.clone())
            .collect(),
    )
}

async fn claim_slot(engine: Arc<Mutex<EngineState>>, proj: &ProjectTickInput, task_id: &str) -> bool {
    let mut g = engine.lock().await;
    if g.slots_used() as u32 >= proj.concurrency {
        return false;
    }
    if g.in_flight.contains(task_id) || g.active_slots.contains_key(task_id) {
        return false;
    }
    g.in_flight.insert(task_id.to_string());
    g.active_slots
        .insert(task_id.to_string(), proj.project_id.clone());
    true
}

fn spawn_pipeline(engine: Arc<Mutex<EngineState>>, proj: ProjectTickInput, task_id: String) {
    let engine_bg = engine.clone();
    let task_id_bg = task_id.clone();
    tokio::spawn(async move {
        if let Err(e) = run_pipeline(engine_bg.clone(), &proj, &task_id_bg).await {
            eprintln!("dispatcher: run {task_id_bg} error: {e}");
            let _ = fail_run_internal(&engine_bg, &proj, &task_id_bg, &format!("internal error: {e}")).await;
        }
        // Slot freed as soon as the Run ends (failed / stopped / succeeded).
        let mut g = engine_bg.lock().await;
        g.active_slots.remove(&task_id_bg);
        g.in_flight.remove(&task_id_bg);
        g.broadcast_dispatcher_changed();
    });
}

async fn run_pipeline(
    engine: Arc<Mutex<EngineState>>,
    proj: &ProjectTickInput,
    task_id: &str,
) -> Result<(), String> {
    let config = proj
        .project
        .load_config()
        .map_err(|e| e.to_string())?;
    let active_status = config
        .workflows
        .task
        .statuses_in(StatusCategory::Active)
        .first()
        .cloned()
        .ok_or_else(|| "no active status in workflow".to_string())?;

    let doc: MarkplaneDocument<Task> = proj.project.read_item(task_id).map_err(|e| e.to_string())?;
    let title = doc.frontmatter.title.clone();

    let base_branch = git_stdout(&proj.project_dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let base_sha = git_stdout(&proj.project_dir, &["rev-parse", "HEAD"])?;
    let worktree = proj.project_dir.join(".worktrees").join(task_id);
    let branch = format!("agent/{task_id}");
    let git_dir = git_dir_realpath(&proj.project_dir)?;

    // Write Run record first, then mutate the ticket (status transition order).
    let started_at = Utc::now().to_rfc3339();
    let mut record = read_run_record(proj.project.root(), task_id)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let seq = record.runs.last().map(|r| r.seq + 1).unwrap_or(1);
    let mut run = Run {
        seq,
        status: RunStatus::Running,
        current_stage: Some(Stage::Implement),
        worktree: worktree.display().to_string(),
        branch: branch.clone(),
        base_sha: base_sha.clone(),
        merge_target: base_branch.clone(),
        merge_sha: None,
        pushed: false,
        started_at: started_at.clone(),
        ended_at: None,
        failure_reason: None,
        worktree_removed_at: None,
        branch_removed_at: None,
        branch_head_at_removal: None,
        cleanup_blocked: None,
        cleanup_trigger: None,
        attempts: vec![],
    };
    record.runs.push(run.clone());
    persist_run(&engine, proj, task_id, &record, &run, true).await?;

    proj.project
        .update_item(
            task_id,
            UpdateFields {
                status: Some(active_status),
                assignee: Patch::Set("dispatcher".into()),
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())?;

    // Worktree
    let _ = git_run(&proj.project_dir, &["worktree", "prune"]);
    if worktree.exists() {
        let _ = git_run(
            &proj.project_dir,
            &["worktree", "remove", "--force", &worktree.display().to_string()],
        );
        let _ = fs::remove_dir_all(&worktree);
    }
    let _ = git_run(&proj.project_dir, &["branch", "-D", &branch]);
    git_run(
        &proj.project_dir,
        &[
            "worktree",
            "add",
            "-b",
            &branch,
            &worktree.display().to_string(),
            &base_sha,
        ],
    )?;

    let ctx = PromptContext {
        task_id: task_id.to_string(),
        title: title.clone(),
        worktree: worktree.display().to_string(),
        branch: branch.clone(),
        base_branch: base_branch.clone(),
        base_sha: base_sha.clone(),
        main_tree: proj.project_dir.display().to_string(),
        target_branch: base_branch.clone(),
        author: String::new(),
        review_comment: String::new(),
    };

    // ── implement (silent-stop check at Stage boundary) ──
    if !ticket_still_ours(&proj.project, task_id)? {
        {
            serve_dispatcher_handoff::silent_stop(
            &proj.project,
            &proj.ws_tx,
            task_id,
            &mut record,
            &mut run,
        )
        .await?;
            engine.lock().await.broadcast_dispatcher_changed();
            return Ok(());
        }
    }

    run.status = RunStatus::Running;
    run.current_stage = Some(Stage::Implement);
    update_latest_run(&mut record, &run);
    persist_run(&engine, proj, task_id, &record, &run, true).await?;

    let impl_settings = proj.settings.stages.implement.clone();
    let impl_prompt = serve_dispatcher_prompts::render_implement_prompt(&ctx);
    let impl_result = run_agent(
        &engine,
        proj,
        task_id,
        &mut record,
        &mut run,
        Stage::Implement,
        &impl_settings.model,
        impl_settings.timeout_minutes,
        &worktree,
        &impl_prompt,
        "agent-implement",
    )
    .await?;

    // After a Stage ends: if human took over mid-Stage, stop without ticket writes.
    if !ticket_still_ours(&proj.project, task_id)? {
        {
            serve_dispatcher_handoff::silent_stop(
            &proj.project,
            &proj.ws_tx,
            task_id,
            &mut record,
            &mut run,
        )
        .await?;
            engine.lock().await.broadcast_dispatcher_changed();
            return Ok(());
        }
    }

    if let Some(reason) = stage_agent_failure_reason(Stage::Implement, &impl_result) {
        return fail_and_finish(&engine, proj, task_id, &mut record, &mut run, Stage::Implement, reason)
            .await;
    }

    let impl_verdict = classify_implement_verdict(impl_result.verdict.as_deref());
    if let Some(reason) = implement_verdict_failure_reason(&impl_verdict) {
        return fail_and_finish(&engine, proj, task_id, &mut record, &mut run, Stage::Implement, reason)
            .await;
    }

    if !branch_has_new_commits(&proj.project_dir, &base_sha, &branch)? {
        return fail_and_finish(
            &engine,
            proj,
            task_id,
            &mut record,
            &mut run,
            Stage::Implement,
            "implement produced no new commits".into(),
        )
        .await;
    }

    // ── one-shot fix after REVIEW: issues (no second review loop) ──
    if impl_verdict == ImplementVerdict::Issues {
        if !ticket_still_ours(&proj.project, task_id)? {
            {
                serve_dispatcher_handoff::silent_stop(
                &proj.project,
                &proj.ws_tx,
                task_id,
                &mut record,
                &mut run,
            )
            .await?;
                engine.lock().await.broadcast_dispatcher_changed();
                return Ok(());
            }
        }
        run.status = RunStatus::Running;
        run.current_stage = Some(Stage::Fix);
        update_latest_run(&mut record, &run);
        persist_run(&engine, proj, task_id, &record, &run, true).await?;

        // Snapshot fix Stage settings at Stage start; inline latest Review.
        let fix_settings = proj.settings.stages.fix.clone();
        let review_comment = {
            let doc: MarkplaneDocument<Task> =
                proj.project.read_item(task_id).map_err(|e| e.to_string())?;
            latest_agent_implement_review(&doc.body).unwrap_or_default()
        };
        let mut fix_ctx = ctx.clone();
        fix_ctx.review_comment = review_comment;
        let fix_prompt = serve_dispatcher_prompts::render_fix_prompt(&fix_ctx);
        let fix_result = run_agent(
            &engine,
            proj,
            task_id,
            &mut record,
            &mut run,
            Stage::Fix,
            &fix_settings.model,
            fix_settings.timeout_minutes,
            &worktree,
            &fix_prompt,
            "agent-fix",
        )
        .await?;

        if !ticket_still_ours(&proj.project, task_id)? {
            {
                serve_dispatcher_handoff::silent_stop(
                &proj.project,
                &proj.ws_tx,
                task_id,
                &mut record,
                &mut run,
            )
            .await?;
                engine.lock().await.broadcast_dispatcher_changed();
                return Ok(());
            }
        }

        if let Some(reason) = stage_agent_failure_reason(Stage::Fix, &fix_result) {
            return fail_and_finish(&engine, proj, task_id, &mut record, &mut run, Stage::Fix, reason).await;
        }
        if !is_fix_done(fix_result.verdict.as_deref()) {
            let line = fix_result.verdict.as_deref().unwrap_or("<none>");
            return fail_and_finish(
                &engine,
                proj,
                task_id,
                &mut record,
                &mut run,
                Stage::Fix,
                format!("fix ended without FIX: done (got {line})"),
            )
            .await;
        }
    }

    // ── merge queue ──
    run.status = RunStatus::MergeQueued;
    run.current_stage = Some(Stage::Merge);
    update_latest_run(&mut record, &run);
    persist_run(&engine, proj, task_id, &record, &run, true).await?;

    let queue = {
        let mut g = engine.lock().await;
        g.merge_queue(git_dir.clone())
    };
    let _permit = queue.enter(task_id).await;
    // Queue positions for other waiters may have advanced.
    engine.lock().await.broadcast_dispatcher_changed();

    // Cross-serve merge lock: stay at queue head and retry while another serve holds it.
    // Queue wait and lock wait are not timed; merge Stage timeout starts after acquire.
    let merge_lock = acquire_merge_file_lock(
        &git_dir,
        &MergeLockHolder {
            pid: std::process::id(),
            hostname: super::serve_dispatcher_lock::local_hostname(),
            serve_url: proj.serve_url.clone(),
            project_path: proj.project_dir.display().to_string(),
            task_id: task_id.to_string(),
            acquired_at: Utc::now().to_rfc3339(),
        },
    )
    .await?;
    // On drop: abort this Run's incomplete merge (if any), then release the OS lock.
    let _merge_guard = MergeCriticalGuard {
        _lock: merge_lock,
        project_dir: proj.project_dir.clone(),
        branch: branch.clone(),
    };

    // Reread ticket after merge queue + lock (silent stop).
    if !ticket_still_ours(&proj.project, task_id)? {
        serve_dispatcher_handoff::silent_stop(
            &proj.project,
            &proj.ws_tx,
            task_id,
            &mut record,
            &mut run,
        )
        .await?;
        engine.lock().await.broadcast_dispatcher_changed();
        return Ok(());
    }

    // Refresh target branch (may have moved).
    let target_branch = git_stdout(&proj.project_dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    run.merge_target = target_branch.clone();
    run.status = RunStatus::Running;
    run.current_stage = Some(Stage::Merge);
    update_latest_run(&mut record, &run);
    persist_run(&engine, proj, task_id, &record, &run, true).await?;

    let mut merge_ctx = ctx.clone();
    merge_ctx.target_branch = target_branch.clone();
    let merge_settings = proj.settings.stages.merge.clone();
    let merge_prompt = serve_dispatcher_prompts::render_merge_prompt(&merge_ctx);
    // Stage timeout starts here — after lock acquire, not during queue/lock wait.
    let merge_result = run_agent(
        &engine,
        proj,
        task_id,
        &mut record,
        &mut run,
        Stage::Merge,
        &merge_settings.model,
        merge_settings.timeout_minutes,
        &proj.project_dir,
        &merge_prompt,
        "agent-merge",
    )
    .await?;

    if !ticket_still_ours(&proj.project, task_id)? {
        let _ = abort_our_incomplete_merge(&proj.project_dir, &branch);
        serve_dispatcher_handoff::silent_stop(
            &proj.project,
            &proj.ws_tx,
            task_id,
            &mut record,
            &mut run,
        )
        .await?;
        engine.lock().await.broadcast_dispatcher_changed();
        return Ok(());
    }

    if let Some(reason) = stage_agent_failure_reason(Stage::Merge, &merge_result) {
        let reason = with_abort_note(&proj.project_dir, &branch, reason);
        return fail_and_finish(&engine, proj, task_id, &mut record, &mut run, Stage::Merge, reason)
            .await;
    }
    if !is_merge_done(merge_result.verdict.as_deref()) {
        let line = merge_result.verdict.as_deref().unwrap_or("<none>");
        let reason = with_abort_note(
            &proj.project_dir,
            &branch,
            format!("merge ended without MERGE: done (got {line})"),
        );
        return fail_and_finish(
            &engine,
            proj,
            task_id,
            &mut record,
            &mut run,
            Stage::Merge,
            reason,
        )
        .await;
    }

    if !branch_is_ancestor(&proj.project_dir, &branch)? {
        let reason = with_abort_note(
            &proj.project_dir,
            &branch,
            "merge target does not contain agent branch".into(),
        );
        return fail_and_finish(
            &engine,
            proj,
            task_id,
            &mut record,
            &mut run,
            Stage::Merge,
            reason,
        )
        .await;
    }

    let merge_sha = git_stdout(&proj.project_dir, &["rev-parse", "HEAD"])?;
    success_close(
        &engine,
        proj,
        task_id,
        &mut record,
        &mut run,
        &target_branch,
        &merge_sha,
    )
    .await?;
    Ok(())
}

/// Non-zero exit / timeout reasons from an agent invocation (verdict checked separately).
fn stage_agent_failure_reason(stage: Stage, outcome: &AgentOutcome) -> Option<String> {
    if outcome.timed_out {
        return Some(format!("{} stage timeout", stage.as_str()));
    }
    if !outcome.succeeded {
        return Some(outcome.failure_reason.clone().unwrap_or_else(|| {
            format!(
                "{} agent failed (exit {}, verdict={})",
                stage.as_str(),
                outcome.exit_code.unwrap_or(-1),
                outcome.verdict.as_deref().unwrap_or("<none>")
            )
        }));
    }
    None
}

struct AgentOutcome {
    succeeded: bool,
    timed_out: bool,
    exit_code: Option<i32>,
    verdict: Option<String>,
    failure_reason: Option<String>,
    saw_init: bool,
}

#[allow(clippy::too_many_arguments)]
async fn run_agent(
    engine: &Arc<Mutex<EngineState>>,
    proj: &ProjectTickInput,
    task_id: &str,
    record: &mut RunRecord,
    run: &mut Run,
    stage: Stage,
    model: &str,
    timeout_minutes: u32,
    cwd: &Path,
    prompt: &str,
    author: &str,
) -> Result<AgentOutcome, String> {
    let workspace = cwd
        .canonicalize()
        .unwrap_or_else(|_| cwd.to_path_buf())
        .display()
        .to_string();
    let timeout_secs = proj
        .test_timeout_secs
        .unwrap_or_else(|| (timeout_minutes as u64).saturating_mul(60));
    let started_at = Utc::now().to_rfc3339();
    let mut attempt = Attempt {
        stage,
        model: model.to_string(),
        timeout_secs: Some(timeout_secs),
        session_id: None,
        workspace: workspace.clone(),
        transcript_path: None,
        pid: None,
        process_group_id: None,
        agent_config_dir: None,
        trigger: Trigger::Initial,
        started_at,
        ended_at: None,
        exit_code: None,
        verdict: None,
        result: None,
        usage: None,
        duration_api_ms: None,
        stderr_tail: None,
    };
    run.attempts.push(attempt.clone());
    update_latest_run(record, run);
    persist_run(engine, proj, task_id, record, run, false).await?;

    let main_root = proj
        .project_dir
        .canonicalize()
        .unwrap_or_else(|_| proj.project_dir.clone());

    let mut cmd = Command::new(&proj.agent_bin);
    cmd.args([
        "-p",
        "--output-format",
        "stream-json",
        "--trust",
        "--force",
        "--approve-mcps",
        "--sandbox",
        "disabled",
        "--workspace",
        &workspace,
        "--model",
        model,
        prompt,
    ])
    .current_dir(cwd)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .env("MARKPLANE_ROOT", &main_root)
    .env("MARKPLANE_AUTHOR", author);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Own process group so we can signal the whole tree.
        cmd.process_group(0);
    }

    let mut child = cmd.spawn().map_err(|e| format!("spawn agent: {e}"))?;
    let pid = child.id();
    attempt.pid = Some(pid);
    attempt.process_group_id = Some(pid);
    if let Some(a) = run.attempts.last_mut() {
        a.pid = Some(pid);
        a.process_group_id = Some(pid);
    }
    update_latest_run(record, run);
    persist_run(engine, proj, task_id, record, run, false).await?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;

    let (sid_tx, mut sid_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let reader_handle = tokio::task::spawn_blocking(move || {
        let mut events = Vec::new();
        let mut session_id = None;
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if let Some(ev) = parse_stream_json_line(&line) {
                if let StreamEvent::SystemInit { session_id: sid } = &ev
                    && session_id.is_none()
                {
                    session_id = Some(sid.clone());
                    let _ = sid_tx.send(sid.clone());
                }
                events.push(ev);
            }
        }
        (events, session_id)
    });

    let stderr_handle = tokio::task::spawn_blocking(move || {
        let mut buf = String::new();
        let mut r = BufReader::new(stderr);
        let _ = r.read_to_string(&mut buf);
        tail_chars(&buf, 4000)
    });

    // Stage timeout watchdog on a plain OS thread (starts immediately).
    let timed_out = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let cancel_watchdog = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let timed_out = timed_out.clone();
        let cancel_watchdog = cancel_watchdog.clone();
        let grace = if proj.test_timeout_secs.is_some() {
            Duration::from_secs(1)
        } else {
            KILL_GRACE
        };
        let timeout = Duration::from_secs(timeout_secs.max(1));
        std::thread::Builder::new()
            .name("dispatcher-stage-watchdog".into())
            .spawn(move || {
                std::thread::sleep(timeout);
                if cancel_watchdog.load(std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                timed_out.store(true, std::sync::atomic::Ordering::SeqCst);
                #[cfg(unix)]
                {
                    unsafe {
                        libc::kill(-(pid as i32), libc::SIGTERM);
                        libc::kill(pid as i32, libc::SIGTERM);
                    }
                    std::thread::sleep(grace);
                    if cancel_watchdog.load(std::sync::atomic::Ordering::SeqCst) {
                        return;
                    }
                    unsafe {
                        libc::kill(-(pid as i32), libc::SIGKILL);
                        libc::kill(pid as i32, libc::SIGKILL);
                    }
                }
                #[cfg(not(unix))]
                {
                    let _ = (pid, grace);
                }
            })
            .map_err(|e| format!("watchdog spawn: {e}"))?;
    }

    // Poll try_wait on a blocking thread so timeout can prevail even if signals
    // are delayed; never sit forever in Child::wait.
    let timed_out_for_wait = timed_out.clone();
    let cancel_for_wait = cancel_watchdog.clone();
    let wait_handle = tokio::task::spawn_blocking(move || {
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    cancel_for_wait.store(true, std::sync::atomic::Ordering::SeqCst);
                    return Ok(status);
                }
                Ok(None) => {
                    if timed_out_for_wait.load(std::sync::atomic::Ordering::SeqCst) {
                        let _ = child.kill();
                        // Brief reap attempts after kill.
                        for _ in 0..40 {
                            match child.try_wait() {
                                Ok(Some(status)) => {
                                    cancel_for_wait
                                        .store(true, std::sync::atomic::Ordering::SeqCst);
                                    return Ok(status);
                                }
                                Ok(None) => std::thread::sleep(Duration::from_millis(50)),
                                Err(e) => return Err(e),
                            }
                        }
                        cancel_for_wait.store(true, std::sync::atomic::Ordering::SeqCst);
                        // Last resort: blocking wait (process should be dead).
                        return child.wait();
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(e),
            }
        }
    });

    // Persist session_id as it arrives; poll until the blocking wait finishes.
    let status = loop {
        while let Ok(sid) = sid_rx.try_recv() {
            if let Some(a) = run.attempts.last_mut()
                && a.session_id.is_none()
            {
                a.session_id = Some(sid);
                update_latest_run(record, run);
                persist_run(engine, proj, task_id, record, run, false).await?;
            }
        }
        if wait_handle.is_finished() {
            break wait_handle
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| format!("wait: {e}"))?;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    cancel_watchdog.store(true, std::sync::atomic::Ordering::SeqCst);

    if timed_out.load(std::sync::atomic::Ordering::SeqCst) {
        let _ = reader_handle.await;
        let stderr_tail = stderr_handle.await.unwrap_or_default();
        let saw_init = run
            .attempts
            .last()
            .and_then(|a| a.session_id.as_ref())
            .is_some();
        if let Some(a) = run.attempts.last_mut() {
            a.ended_at = Some(Utc::now().to_rfc3339());
            a.result = Some(AttemptResult::Timeout);
            a.stderr_tail = Some(stderr_tail);
            a.exit_code = Some(-1);
        }
        update_latest_run(record, run);
        persist_run(engine, proj, task_id, record, run, false).await?;
        let outcome = AgentOutcome {
            succeeded: false,
            timed_out: true,
            exit_code: Some(-1),
            verdict: None,
            failure_reason: Some(format!("{} stage timeout", stage.as_str())),
            saw_init,
        };
        maybe_auto_close_on_startup_fail(proj, outcome.saw_init).await;
        return Ok(outcome);
    }

    let (events, session_id) = reader_handle.await.map_err(|e| e.to_string())?;
    let stderr_tail = stderr_handle.await.unwrap_or_default();
    let exit_code = status.code().unwrap_or(-1);

    let verdict = verdict_from_events(&events);
    let (usage, duration_api_ms) = usage_from_events(&events);
    let succeeded = agent_invocation_succeeded(exit_code, &events);
    let result = if succeeded {
        AttemptResult::Ok
    } else {
        AttemptResult::Failed
    };
    let saw_init = session_id.is_some()
        || run
            .attempts
            .last()
            .and_then(|a| a.session_id.as_ref())
            .is_some();

    if let Some(a) = run.attempts.last_mut() {
        if a.session_id.is_none() {
            a.session_id = session_id;
        }
        a.ended_at = Some(Utc::now().to_rfc3339());
        a.exit_code = Some(exit_code);
        a.verdict = verdict.clone();
        a.result = Some(result);
        a.usage = usage;
        a.duration_api_ms = duration_api_ms;
        a.stderr_tail = Some(stderr_tail);
    }
    update_latest_run(record, run);
    persist_run(engine, proj, task_id, record, run, false).await?;

    let failure_reason = if !succeeded {
        Some(format!(
            "{} agent failed (exit {exit_code}, verdict={})",
            stage.as_str(),
            verdict.as_deref().unwrap_or("<none>")
        ))
    } else {
        None
    };

    let outcome = AgentOutcome {
        succeeded,
        timed_out: false,
        exit_code: Some(exit_code),
        verdict,
        failure_reason,
        saw_init,
    };
    maybe_auto_close_on_startup_fail(proj, outcome.saw_init).await;
    Ok(outcome)
}

async fn maybe_auto_close_on_startup_fail(proj: &ProjectTickInput, saw_init: bool) {
    let reason = {
        let mut g = proj.scan_auto_close.lock().await;
        g.note_stage_ended(&proj.project_id, saw_init)
    };
    let Some(reason) = reason else {
        return;
    };
    eprintln!(
        "dispatcher: auto-closing Scan for {}: {reason}",
        proj.project_id
    );
    // Serve listener flips Scan off then broadcasts; do not push early.
    let _ = proj.auto_close_tx.send((proj.project_id.clone(), reason));
}

async fn success_close(
    engine: &Arc<Mutex<EngineState>>,
    proj: &ProjectTickInput,
    task_id: &str,
    record: &mut RunRecord,
    run: &mut Run,
    target_branch: &str,
    merge_sha: &str,
) -> Result<(), String> {
    let config = proj.project.load_config().map_err(|e| e.to_string())?;
    let completed = config
        .workflows
        .task
        .statuses_in(StatusCategory::Completed)
        .first()
        .cloned()
        .ok_or_else(|| "no completed status".to_string())?;

    let doc: MarkplaneDocument<Task> = proj.project.read_item(task_id).map_err(|e| e.to_string())?;
    let already_done = config
        .workflows
        .task
        .category_of(&doc.frontmatter.status)
        == Some(StatusCategory::Completed);

    if !already_done {
        // Keep tags + assignee; only change status.
        proj.project
            .update_status(task_id, &completed)
            .map_err(|e| e.to_string())?;
    }

    // Idempotent: already closed successfully for this merge → do nothing.
    if run.status == RunStatus::Succeeded && run.merge_sha.as_deref() == Some(merge_sha) {
        return Ok(());
    }

    // Push merge commits first (ticket file may still be dirty), so the success
    // comment can record the real push outcome; then commit the ticket once and
    // push again to publish that commit when the first push succeeded.
    let mut pushed = run.pushed;
    if proj.settings.push_after_merge && !run.pushed {
        match git_run(&proj.project_dir, &["push", "origin", "HEAD"]) {
            Ok(()) => pushed = true,
            Err(e) => {
                eprintln!("dispatcher: push failed for {task_id}: {e}");
                pushed = false;
            }
        }
    }

    let doc: MarkplaneDocument<Task> = proj.project.read_item(task_id).map_err(|e| e.to_string())?;
    let already_commented = doc.body.contains(&format!("`{merge_sha}`"))
        && doc.body.contains("**Merged**");
    if !already_commented {
        let flag = if pushed { "yes" } else { "no" };
        let comment_text =
            format!("**Merged** into `{target_branch}` at `{merge_sha}` · pushed: {flag}");
        proj.project
            .add_comment(task_id, "dispatcher", &comment_text)
            .map_err(|e| e.to_string())?;
    }

    let item_path = proj.project.item_path(task_id).map_err(|e| e.to_string())?;
    let rel = item_path
        .strip_prefix(&proj.project_dir)
        .unwrap_or(&item_path)
        .to_path_buf();
    let rel_str = rel.to_string_lossy().replace('\\', "/");

    let status = git_stdout(&proj.project_dir, &["status", "--porcelain", "--", &rel_str])
        .unwrap_or_default();
    if !status.trim().is_empty() {
        git_run(&proj.project_dir, &["add", "--", &rel_str])?;
        let msg = format!("Mark ticket {task_id} done\n\nRefs: {task_id}");
        git_run(&proj.project_dir, &["commit", "-m", &msg])?;
        if pushed
            && let Err(e) = git_run(&proj.project_dir, &["push", "origin", "HEAD"])
        {
            eprintln!("dispatcher: ticket push failed for {task_id}: {e}");
        }
    }

    run.status = RunStatus::Succeeded;
    run.current_stage = Some(Stage::Merge);
    run.merge_sha = Some(merge_sha.to_string());
    run.pushed = pushed;
    run.ended_at = Some(Utc::now().to_rfc3339());
    run.failure_reason = None;
    update_latest_run(record, run);
    persist_run(engine, proj, task_id, record, run, true).await?;
    Ok(())
}

async fn fail_and_finish(
    engine: &Arc<Mutex<EngineState>>,
    proj: &ProjectTickInput,
    task_id: &str,
    record: &mut RunRecord,
    run: &mut Run,
    stage: Stage,
    reason: String,
) -> Result<(), String> {
    if !ticket_still_ours(&proj.project, task_id)? {
        serve_dispatcher_handoff::silent_stop(
            &proj.project,
            &proj.ws_tx,
            task_id,
            record,
            run,
        )
        .await?;
        engine.lock().await.broadcast_dispatcher_changed();
        return Ok(());
    }
    serve_dispatcher_handoff::handoff_failure(
        &proj.project,
        &proj.ws_tx,
        task_id,
        record,
        run,
        stage,
        reason,
    )
    .await?;
    engine.lock().await.broadcast_dispatcher_changed();
    Ok(())
}

async fn fail_run_internal(
    engine: &Arc<Mutex<EngineState>>,
    proj: &ProjectTickInput,
    task_id: &str,
    reason: &str,
) -> Result<(), String> {
    let mut record = read_run_record(proj.project.root(), task_id)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let mut run = match record.runs.last().cloned() {
        Some(r) => r,
        None => return Ok(()),
    };
    let stage = run.current_stage.unwrap_or(Stage::Implement);
    fail_and_finish(
        engine,
        proj,
        task_id,
        &mut record,
        &mut run,
        stage,
        reason.to_string(),
    )
    .await
}

fn update_latest_run(record: &mut RunRecord, run: &Run) {
    if let Some(last) = record.runs.last_mut() {
        *last = run.clone();
    }
}

async fn persist_run(
    engine: &Arc<Mutex<EngineState>>,
    proj: &ProjectTickInput,
    task_id: &str,
    record: &RunRecord,
    run: &Run,
    notify_dispatcher: bool,
) -> Result<(), String> {
    write_run_record(proj.project.root(), task_id, record).map_err(|e| e.to_string())?;
    broadcast_run_changed(&proj.ws_tx, task_id, run);
    if notify_dispatcher {
        engine.lock().await.broadcast_dispatcher_changed();
    }
    Ok(())
}

fn broadcast_run_changed(ws_tx: &broadcast::Sender<String>, task_id: &str, run: &Run) {
    let stage = run
        .current_stage
        .map(|s| s.as_str().to_string())
        .unwrap_or_else(|| "".into());
    let display = display_status(run);
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

fn display_status(run: &Run) -> &'static str {
    match run.status {
        RunStatus::Running => "running",
        RunStatus::MergeQueued => "merge-queued",
        RunStatus::Succeeded => "succeeded",
        RunStatus::Failed => "failed",
        RunStatus::Stopped => "stopped",
    }
}

fn git_run(cwd: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn git_stdout(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_dir_realpath(project_dir: &Path) -> Result<PathBuf, String> {
    let git_dir = git_stdout(project_dir, &["rev-parse", "--path-format=absolute", "--git-dir"])?;
    let p = PathBuf::from(git_dir);
    Ok(p.canonicalize().unwrap_or(p))
}

fn branch_has_new_commits(cwd: &Path, base_sha: &str, branch: &str) -> Result<bool, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["rev-list", "--count", &format!("{base_sha}..{branch}")])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Ok(false);
    }
    let n: u64 = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .unwrap_or(0);
    Ok(n > 0)
}

fn branch_is_ancestor(cwd: &Path, branch: &str) -> Result<bool, String> {
    let status = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["merge-base", "--is-ancestor", branch, "HEAD"])
        .status()
        .map_err(|e| e.to_string())?;
    Ok(status.success())
}

fn tail_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let start = s
        .char_indices()
        .rev()
        .nth(max - 1)
        .map(|(i, _)| i)
        .unwrap_or(0);
    s[start..].to_string()
}

/// Holder metadata beside the empty `merge.lock` (display only).
#[derive(Debug, Clone, serde::Serialize)]
struct MergeLockHolder {
    pid: u32,
    hostname: String,
    serve_url: String,
    project_path: String,
    task_id: String,
    acquired_at: String,
}

/// OS advisory merge lock under `<git dir>/markplane/merge.lock`.
struct MergeFileLock {
    _file: File,
}

/// Holds the merge lock and aborts this Run's incomplete merge on drop.
struct MergeCriticalGuard {
    _lock: MergeFileLock,
    project_dir: PathBuf,
    branch: String,
}

impl Drop for MergeCriticalGuard {
    fn drop(&mut self) {
        // Safety net if a path returned without an explicit recover step.
        let _ = abort_our_incomplete_merge(&self.project_dir, &self.branch);
    }
}

/// Acquire the worktree merge lock (non-blocking try_lock, retry while contended).
///
/// The lock file stays empty and is never deleted. Holder info is written
/// atomically to sibling `merge.json` for display only. Caller must already be
/// at the head of the in-process Merge queue; this wait is not timed.
async fn acquire_merge_file_lock(
    git_dir: &Path,
    holder: &MergeLockHolder,
) -> Result<MergeFileLock, String> {
    let dir = git_dir.join("markplane");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("merge.lock");
    let meta_path = dir.join("merge.json");
    loop {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| format!("open merge.lock: {e}"))?;
        match file.try_lock() {
            Ok(()) => {
                // Keep the lock file empty; never recycle/delete it.
                let _ = file.set_len(0);
                write_merge_holder_atomic(&meta_path, holder)
                    .map_err(|e| format!("write merge.json: {e}"))?;
                return Ok(MergeFileLock { _file: file });
            }
            Err(TryLockError::WouldBlock) => {
                drop(file);
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(TryLockError::Error(e)) => {
                return Err(format!("merge lock: {e}"));
            }
        }
    }
}

fn write_merge_holder_atomic(path: &Path, holder: &MergeLockHolder) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = dir.join(".merge.json.tmp");
    {
        let mut f = File::create(&tmp)?;
        let json = serde_json::to_string_pretty(holder)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        f.write_all(json.as_bytes())?;
        f.write_all(b"\n")?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

/// If `MERGE_HEAD` matches this Run's agent branch tip, `git merge --abort`.
/// Returns `Err` with a loggable message when abort itself fails.
/// Never stash/reset/clean.
fn abort_our_incomplete_merge(project_dir: &Path, branch: &str) -> Result<(), String> {
    let merge_head = Command::new("git")
        .arg("-C")
        .arg(project_dir)
        .args(["rev-parse", "-q", "--verify", "MERGE_HEAD"])
        .output()
        .ok();
    let Some(out) = merge_head else {
        return Ok(());
    };
    if !out.status.success() {
        return Ok(());
    }
    let merge_sha = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if merge_sha.is_empty() {
        return Ok(());
    }
    let Ok(branch_tip) = git_stdout(project_dir, &["rev-parse", branch]) else {
        return Ok(());
    };
    if merge_sha != branch_tip.trim() {
        return Ok(());
    }
    if let Err(e) = git_run(project_dir, &["merge", "--abort"]) {
        let msg = format!("git merge --abort failed after merge of {branch}: {e}");
        eprintln!("dispatcher: {msg}");
        return Err(msg);
    }
    Ok(())
}

/// Recover this Run's incomplete merge before writing the handoff comment.
fn with_abort_note(project_dir: &Path, branch: &str, reason: String) -> String {
    match abort_our_incomplete_merge(project_dir, branch) {
        Ok(()) => reason,
        Err(abort_err) => format!("{reason}; {abort_err}"),
    }
}

