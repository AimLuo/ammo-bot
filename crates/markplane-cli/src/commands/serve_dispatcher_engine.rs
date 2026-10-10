//! Dispatcher happy-path engine: scan → pickup → implement → merge → done.
//!
//! Kept out of `serve.rs` on purpose. Unhappy endings only record Run `failed`
//! + reason (human handoff is later tickets).

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use markplane_core::{
    Attempt, AttemptResult, EffectiveDispatcherSettings, MarkplaneDocument, Patch, Project,
    QueryFilter, Run, RunRecord, RunStatus, Stage, StatusCategory, Task, Trigger, UpdateFields,
    agent_invocation_succeeded, filter_and_sort_pickup_candidates, parse_stream_json_line,
    read_run_record, usage_from_events, verdict_from_events, write_run_record, StreamEvent,
};
use serde_json::json;
use tokio::sync::{Mutex, broadcast};

use super::serve_dispatcher_prompts::{self, PromptContext};

/// Scan interval when Scan switch is on.
pub const SCAN_INTERVAL: Duration = Duration::from_secs(15);

/// How long to wait after SIGTERM before SIGKILL.
const KILL_GRACE: Duration = Duration::from_secs(10);

/// Slot + merge-queue bookkeeping shared across Projects in one serve.
#[derive(Default)]
pub struct EngineState {
    /// task_id → project_id for Runs holding a Slot.
    pub active_slots: HashMap<String, String>,
    /// Per worktree git-dir realpath: FIFO of task_ids waiting / holding merge.
    merge_queues: HashMap<PathBuf, Arc<MergeQueue>>,
    /// Tasks currently being processed (pickup through end) — skip re-pickup.
    in_flight: HashSet<String>,
}

impl EngineState {
    pub fn slots_used(&self) -> usize {
        self.active_slots.len()
    }

    pub fn project_has_active_runs(&self, project_id: &str) -> bool {
        self.active_slots.values().any(|p| p == project_id)
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
pub struct ProjectTickInput {
    pub project_id: String,
    pub project: Project,
    pub project_dir: PathBuf,
    pub agent_bin: PathBuf,
    pub ws_tx: broadcast::Sender<String>,
    pub concurrency: u32,
    pub settings: EffectiveDispatcherSettings,
    pub scan_on: bool,
}

/// Explicit scan tick (tests call this; production uses a 15s interval).
pub async fn dispatcher_tick(engine: Arc<Mutex<EngineState>>, projects: Vec<ProjectTickInput>) {
    // One pickup per Project per tick (within global concurrency).
    for proj in projects {
        if !proj.scan_on {
            continue;
        }
        try_pickup_one(engine.clone(), proj).await;
    }
}

async fn try_pickup_one(engine: Arc<Mutex<EngineState>>, proj: ProjectTickInput) {
    let free = {
        let g = engine.lock().await;
        let used = g.slots_used() as u32;
        if used >= proj.concurrency {
            return;
        }
        proj.concurrency - used
    };
    if free == 0 {
        return;
    }

    let config = match proj.project.load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("dispatcher: load_config {}: {e}", proj.project_id);
            return;
        }
    };
    let tasks = match proj.project.list_tasks(&QueryFilter::default()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("dispatcher: list_tasks {}: {e}", proj.project_id);
            return;
        }
    };
    let candidates = filter_and_sort_pickup_candidates(&tasks, &config.workflows.task);
    let task_id = {
        let g = engine.lock().await;
        candidates
            .iter()
            .map(|d| d.frontmatter.id.clone())
            .find(|id| !g.in_flight.contains(id) && !g.active_slots.contains_key(id))
    };
    let Some(task_id) = task_id else {
        return;
    };

    {
        let mut g = engine.lock().await;
        if g.slots_used() as u32 >= proj.concurrency {
            return;
        }
        g.in_flight.insert(task_id.clone());
        g.active_slots
            .insert(task_id.clone(), proj.project_id.clone());
    }

    let engine_bg = engine.clone();
    let task_id_bg = task_id.clone();
    tokio::spawn(async move {
        if let Err(e) = run_pipeline(engine_bg.clone(), &proj, &task_id_bg).await {
            eprintln!("dispatcher: run {task_id_bg} error: {e}");
            let _ = fail_run(
                &proj.project,
                &proj.ws_tx,
                &task_id_bg,
                &format!("internal error: {e}"),
                Some(Stage::Implement),
            )
            .await;
        }
        let mut g = engine_bg.lock().await;
        g.active_slots.remove(&task_id_bg);
        g.in_flight.remove(&task_id_bg);
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
    persist_run(proj, task_id, &record, &run).await?;

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
    };

    // ── implement ──
    let impl_settings = proj.settings.stages.implement.clone();
    let impl_prompt = serve_dispatcher_prompts::render_implement_prompt(&ctx);
    let impl_result = run_agent(
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

    if !impl_result.succeeded {
        return fail_and_finish(
            proj,
            task_id,
            &mut record,
            &mut run,
            Stage::Implement,
            impl_result
                .failure_reason
                .unwrap_or_else(|| "implement stage failed".into()),
        )
        .await;
    }
    let verdict = impl_result.verdict.unwrap_or_default();
    if verdict != "REVIEW: clean" {
        return fail_and_finish(
            proj,
            task_id,
            &mut record,
            &mut run,
            Stage::Implement,
            format!("unexpected implement verdict: {verdict}"),
        )
        .await;
    }
    // Require new commits on the branch.
    if !branch_has_new_commits(&proj.project_dir, &base_sha, &branch)? {
        return fail_and_finish(
            proj,
            task_id,
            &mut record,
            &mut run,
            Stage::Implement,
            "implement produced no new commits".into(),
        )
        .await;
    }

    // ── merge queue ──
    run.status = RunStatus::MergeQueued;
    run.current_stage = Some(Stage::Merge);
    update_latest_run(&mut record, &run);
    persist_run(proj, task_id, &record, &run).await?;

    let queue = {
        let mut g = engine.lock().await;
        g.merge_queue(git_dir.clone())
    };
    let _permit = queue.enter(task_id).await;

    // Cross-serve merge file lock (best-effort; hold while merging).
    let _merge_lock = acquire_merge_file_lock(&git_dir)?;

    // Reread ticket before merge.
    let doc: MarkplaneDocument<Task> = proj.project.read_item(task_id).map_err(|e| e.to_string())?;
    let still_ours = config
        .workflows
        .task
        .category_of(&doc.frontmatter.status)
        == Some(StatusCategory::Active)
        && doc.frontmatter.assignee.as_deref() == Some("dispatcher");
    if !still_ours {
        run.status = RunStatus::Stopped;
        run.ended_at = Some(Utc::now().to_rfc3339());
        run.current_stage = None;
        update_latest_run(&mut record, &run);
        persist_run(proj, task_id, &record, &run).await?;
        return Ok(());
    }

    // Refresh target branch (may have moved).
    let target_branch = git_stdout(&proj.project_dir, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    run.merge_target = target_branch.clone();
    run.status = RunStatus::Running;
    run.current_stage = Some(Stage::Merge);
    update_latest_run(&mut record, &run);
    persist_run(proj, task_id, &record, &run).await?;

    let mut merge_ctx = ctx.clone();
    merge_ctx.target_branch = target_branch.clone();
    let merge_settings = proj.settings.stages.merge.clone();
    let merge_prompt = serve_dispatcher_prompts::render_merge_prompt(&merge_ctx);
    let merge_result = run_agent(
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

    if !merge_result.succeeded || merge_result.verdict.as_deref() != Some("MERGE: done") {
        // Abort leftover merge if any.
        let _ = git_run(&proj.project_dir, &["merge", "--abort"]);
        return fail_and_finish(
            proj,
            task_id,
            &mut record,
            &mut run,
            Stage::Merge,
            merge_result
                .failure_reason
                .or(merge_result.verdict)
                .unwrap_or_else(|| "merge stage failed".into()),
        )
        .await;
    }

    if !branch_is_ancestor(&proj.project_dir, &branch)? {
        let _ = git_run(&proj.project_dir, &["merge", "--abort"]);
        return fail_and_finish(
            proj,
            task_id,
            &mut record,
            &mut run,
            Stage::Merge,
            "merge target does not contain agent branch".into(),
        )
        .await;
    }

    let merge_sha = git_stdout(&proj.project_dir, &["rev-parse", "HEAD"])?;
    success_close(
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

struct AgentOutcome {
    succeeded: bool,
    verdict: Option<String>,
    failure_reason: Option<String>,
}

#[allow(clippy::too_many_arguments)]
async fn run_agent(
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
    let timeout_secs = (timeout_minutes as u64).saturating_mul(60);
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
    persist_run(proj, task_id, record, run).await?;

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
    persist_run(proj, task_id, record, run).await?;

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

    let wait_handle = tokio::task::spawn_blocking(move || child.wait());

    // Persist session_id as soon as system/init arrives.
    let sid_project_root = proj.project.root().to_path_buf();
    let sid_task_id = task_id.to_string();
    let sid_ws = proj.ws_tx.clone();
    let sid_writer = tokio::spawn(async move {
        if let Some(sid) = sid_rx.recv().await
            && let Ok(Some(mut rec)) = read_run_record(&sid_project_root, &sid_task_id)
            && let Some(run) = rec.runs.last_mut()
            && let Some(attempt) = run.attempts.last_mut()
            && attempt.session_id.is_none()
        {
            attempt.session_id = Some(sid);
            let snap = run.clone();
            let _ = write_run_record(&sid_project_root, &sid_task_id, &rec);
            broadcast_run_changed(&sid_ws, &sid_task_id, &snap);
        }
    });

    let timed = tokio::time::timeout(
        Duration::from_secs(timeout_secs.max(1)),
        wait_handle,
    )
    .await;

    let (exit_code, events, session_id, stderr_tail) = match timed {
        Ok(Ok(Ok(status))) => {
            let (events, session_id) = reader_handle.await.map_err(|e| e.to_string())?;
            let stderr_tail = stderr_handle.await.unwrap_or_default();
            (status.code().unwrap_or(-1), events, session_id, stderr_tail)
        }
        Ok(Ok(Err(e))) => return Err(format!("wait: {e}")),
        Ok(Err(e)) => return Err(e.to_string()),
        Err(_) => {
            #[cfg(unix)]
            {
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGTERM);
                }
                tokio::time::sleep(KILL_GRACE).await;
                unsafe {
                    libc::kill(-(pid as i32), libc::SIGKILL);
                }
            }
            #[cfg(not(unix))]
            {
                let _ = pid;
            }
            let _ = reader_handle.await;
            let stderr_tail = stderr_handle.await.unwrap_or_default();
            let _ = sid_writer.await;
            if let Some(a) = run.attempts.last_mut() {
                a.ended_at = Some(Utc::now().to_rfc3339());
                a.result = Some(AttemptResult::Timeout);
                a.stderr_tail = Some(stderr_tail);
                a.exit_code = Some(-1);
            }
            update_latest_run(record, run);
            persist_run(proj, task_id, record, run).await?;
            return Ok(AgentOutcome {
                succeeded: false,
                verdict: None,
                failure_reason: Some(format!("{stage:?} timed out").to_lowercase()),
            });
        }
    };
    let _ = sid_writer.await;

    let verdict = verdict_from_events(&events);
    let (usage, duration_api_ms) = usage_from_events(&events);
    let succeeded = agent_invocation_succeeded(exit_code, &events);
    let result = if succeeded {
        AttemptResult::Ok
    } else {
        AttemptResult::Failed
    };

    if let Some(a) = run.attempts.last_mut() {
        a.session_id = session_id.or(a.session_id.clone());
        a.ended_at = Some(Utc::now().to_rfc3339());
        a.exit_code = Some(exit_code);
        a.verdict = verdict.clone();
        a.result = Some(result);
        a.usage = usage;
        a.duration_api_ms = duration_api_ms;
        a.stderr_tail = Some(stderr_tail);
    }
    update_latest_run(record, run);
    persist_run(proj, task_id, record, run).await?;

    let failure_reason = if !succeeded {
        Some(format!(
            "{stage:?} agent failed (exit {exit_code}, verdict={})",
            verdict.as_deref().unwrap_or("<none>")
        ))
    } else {
        None
    };

    Ok(AgentOutcome {
        succeeded,
        verdict,
        failure_reason,
    })
}

async fn success_close(
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

    // Comment records whether we *will* push (setting), then commit ticket, then push once.
    let want_push = proj.settings.push_after_merge && !run.pushed;
    let doc: MarkplaneDocument<Task> = proj.project.read_item(task_id).map_err(|e| e.to_string())?;
    let already_commented = doc.body.contains(&format!("`{merge_sha}`"))
        && doc.body.contains("**Merged**");
    if !already_commented {
        let flag = if want_push { "yes" } else { "no" };
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
    }

    let mut pushed = run.pushed;
    if want_push {
        match git_run(&proj.project_dir, &["push", "origin", "HEAD"]) {
            Ok(()) => pushed = true,
            Err(e) => {
                eprintln!("dispatcher: push failed for {task_id}: {e}");
                pushed = false;
            }
        }
    }

    run.status = RunStatus::Succeeded;
    run.current_stage = Some(Stage::Merge);
    run.merge_sha = Some(merge_sha.to_string());
    run.pushed = pushed;
    run.ended_at = Some(Utc::now().to_rfc3339());
    run.failure_reason = None;
    update_latest_run(record, run);
    persist_run(proj, task_id, record, run).await?;
    Ok(())
}

async fn fail_and_finish(
    proj: &ProjectTickInput,
    task_id: &str,
    record: &mut RunRecord,
    run: &mut Run,
    stage: Stage,
    reason: String,
) -> Result<(), String> {
    run.status = RunStatus::Failed;
    run.current_stage = Some(stage);
    run.failure_reason = Some(reason);
    run.ended_at = Some(Utc::now().to_rfc3339());
    update_latest_run(record, run);
    persist_run(proj, task_id, record, run).await?;
    Ok(())
}

async fn fail_run(
    project: &Project,
    ws_tx: &broadcast::Sender<String>,
    task_id: &str,
    reason: &str,
    stage: Option<Stage>,
) -> Result<(), String> {
    let mut record = read_run_record(project.root(), task_id)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    if let Some(run) = record.runs.last_mut() {
        run.status = RunStatus::Failed;
        run.failure_reason = Some(reason.to_string());
        run.ended_at = Some(Utc::now().to_rfc3339());
        if stage.is_some() {
            run.current_stage = stage;
        }
        let snap = run.clone();
        write_run_record(project.root(), task_id, &record).map_err(|e| e.to_string())?;
        broadcast_run_changed(ws_tx, task_id, &snap);
    }
    Ok(())
}

fn update_latest_run(record: &mut RunRecord, run: &Run) {
    if let Some(last) = record.runs.last_mut() {
        *last = run.clone();
    }
}

async fn persist_run(
    proj: &ProjectTickInput,
    task_id: &str,
    record: &RunRecord,
    run: &Run,
) -> Result<(), String> {
    write_run_record(proj.project.root(), task_id, record).map_err(|e| e.to_string())?;
    broadcast_run_changed(&proj.ws_tx, task_id, run);
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

struct MergeFileLock {
    _file: File,
}

fn acquire_merge_file_lock(git_dir: &Path) -> Result<MergeFileLock, String> {
    let dir = git_dir.join("markplane");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("merge.lock");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| format!("open merge.lock: {e}"))?;
    // Retry a few times for contended happy-path tests.
    for _ in 0..200 {
        match file.try_lock() {
            Ok(()) => {
                let _ = file.set_len(0);
                let meta = json!({
                    "pid": std::process::id(),
                    "acquired_at": Utc::now().to_rfc3339(),
                });
                let meta_path = dir.join("merge.json");
                let _ = fs::write(meta_path, format!("{}\n", meta));
                return Ok(MergeFileLock { _file: file });
            }
            Err(std::fs::TryLockError::WouldBlock) => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(std::fs::TryLockError::Error(e)) => {
                return Err(format!("merge lock: {e}"));
            }
        }
    }
    Err("timed out waiting for merge.lock".into())
}

