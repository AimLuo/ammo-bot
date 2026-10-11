//! Dispatcher cleanup of recorded worktrees and `agent/<TASK-id>` branches.
//!
//! Only touches paths/branches registered on the latest Run of each ticket.
//! Auto cleanup is non-force; manual / re-pickup cleanup may force.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::Utc;
use markplane_core::{
    Project, Run, RunRecord, RunStatus, StatusCategory, Task, write_run_record,
};
use tokio::sync::broadcast;

use super::serve_dispatcher_resume::{run_orphan_pid, terminate_orphan_attempt};

/// Why automatic cleanup skipped a remnant.
pub const BLOCKED_UNCOMMITTED: &str = "uncommitted_changes";
pub const BLOCKED_UNMERGED: &str = "unmerged_commits";
pub const BLOCKED_ORPHAN: &str = "orphan_running";
pub const BLOCKED_NOT_WORKTREE: &str = "not_a_worktree";

pub const TRIGGER_AUTO: &str = "auto";
pub const TRIGGER_MANUAL: &str = "manual";
pub const TRIGGER_REPICKUP: &str = "repickup";

/// Snapshot for the Clean up confirmation dialog / GET preview.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CleanupPreview {
    pub worktree: String,
    pub worktree_present: bool,
    /// Path exists and is listed by `git worktree list` (safe to force-remove).
    pub worktree_registered: bool,
    pub uncommitted_files: u32,
    pub branch: String,
    pub branch_present: bool,
    /// Branch name starts with `agent/` (only those are deleted).
    pub branch_is_agent: bool,
    pub unmerged_commits: u32,
    pub branch_head: Option<String>,
    pub orphan_pid: Option<u32>,
    /// Whether the Clean up action is allowed for this Run + ticket.
    pub eligible: bool,
}

/// Whether automatic cleanup may attempt this latest Run.
pub fn auto_cleanup_eligible(
    run: &Run,
    ticket_category: Option<StatusCategory>,
) -> bool {
    if run.worktree_removed_at.is_some() && run.branch_removed_at.is_some() {
        return false;
    }
    match run.status {
        RunStatus::Succeeded => true,
        RunStatus::Failed | RunStatus::Stopped => matches!(
            ticket_category,
            None | Some(StatusCategory::Completed) | Some(StatusCategory::Cancelled)
        ),
        RunStatus::Running | RunStatus::MergeQueued => false,
    }
}

/// Whether the manual Clean up button / POST may run.
///
/// `display_status` is the server display status (or client-derived `stopping`).
pub fn manual_cleanup_eligible(
    run: &Run,
    display_status: &str,
    ticket_category: Option<StatusCategory>,
) -> bool {
    if run.worktree_removed_at.is_some() && run.branch_removed_at.is_some() {
        return false;
    }
    match display_status {
        "running" | "merge-queued" | "stopping" | "running elsewhere" | "interrupted" => {
            return false;
        }
        _ => {}
    }
    // failed while ticket still active → Continue, not Clean up
    if run.status == RunStatus::Failed
        && ticket_category == Some(StatusCategory::Active)
    {
        return false;
    }
    true
}

/// Build a confirmation preview for the latest Run.
pub fn cleanup_preview(
    project_dir: &Path,
    run: &Run,
    display_status: &str,
    ticket_category: Option<StatusCategory>,
) -> CleanupPreview {
    let orphan_pid = run_orphan_pid(run);
    let wt_path = PathBuf::from(&run.worktree);
    let registered = is_registered_worktree(project_dir, &wt_path);
    let worktree_present = run.worktree_removed_at.is_none() && (wt_path.exists() || registered);
    let uncommitted_files = if wt_path.is_dir() {
        count_porcelain_lines(&wt_path)
    } else {
        0
    };
    let branch_present = run.branch_removed_at.is_none()
        && branch_exists(project_dir, &run.branch);
    let branch_head = if branch_present {
        git_stdout(project_dir, &["rev-parse", &run.branch]).ok()
    } else {
        None
    };
    let unmerged_commits = if branch_present {
        count_unmerged_commits(project_dir, &run.branch)
    } else {
        0
    };
    let worktree_registered = worktree_present && registered;
    let branch_is_agent = run.branch.starts_with("agent/");
    // Spec: only when a recorded remnant is still there and deletable
    // (registered worktree, agent/* branch, and/or orphan to terminate).
    let has_deletable_remnant = worktree_registered
        || (branch_present && branch_is_agent)
        || orphan_pid.is_some();
    CleanupPreview {
        worktree: run.worktree.clone(),
        worktree_present,
        worktree_registered,
        uncommitted_files,
        branch: run.branch.clone(),
        branch_present,
        branch_is_agent,
        unmerged_commits,
        branch_head,
        orphan_pid,
        eligible: manual_cleanup_eligible(run, display_status, ticket_category)
            && has_deletable_remnant,
    }
}

/// Auto-clean one Run in place. Returns whether the record should be persisted.
pub fn attempt_auto_cleanup(project_dir: &Path, run: &mut Run) -> bool {
    cleanup_run(project_dir, run, false, TRIGGER_AUTO, false)
}

/// Force-clean one Run (manual Clean up). `terminate_orphan` kills a live orphan first.
pub fn attempt_force_cleanup(
    project_dir: &Path,
    run: &mut Run,
    terminate_orphan: bool,
) -> Result<(), CleanupError> {
    if let Some(pid) = run_orphan_pid(run) {
        if !terminate_orphan {
            return Err(CleanupError::OrphanRunning { pid });
        }
        if let Some(last) = run.attempts.last() {
            terminate_orphan_attempt(last);
        }
    }
    cleanup_run(project_dir, run, true, TRIGGER_MANUAL, false);
    Ok(())
}

/// Before re-pickup: record old branch head SHA, then force-remove remnants.
pub fn force_cleanup_for_repickup(project_dir: &Path, run: &mut Run) -> bool {
    if run.worktree_removed_at.is_some() && run.branch_removed_at.is_some() {
        return false;
    }
    // Capture branch head before deletion when still present.
    if run.branch_removed_at.is_none()
        && branch_exists(project_dir, &run.branch)
        && run.branch_head_at_removal.is_none()
    {
        run.branch_head_at_removal =
            git_stdout(project_dir, &["rev-parse", &run.branch]).ok();
    }
    cleanup_run(project_dir, run, true, TRIGGER_REPICKUP, true)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanupError {
    OrphanRunning { pid: u32 },
    NotEligible,
}

impl CleanupError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::OrphanRunning { .. } => "orphan_running",
            Self::NotEligible => "not_cleanup_eligible",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::OrphanRunning { pid } => {
                format!("Previous agent process still running (pid {pid})")
            }
            Self::NotEligible => "Clean up is not available for this Run".into(),
        }
    }
}

/// Scan all latest Runs in a Project and auto-clean eligible ones.
pub fn auto_cleanup_project(
    project: &Project,
    project_dir: &Path,
    ws_tx: &broadcast::Sender<String>,
) {
    let Ok(records) = project.list_run_records() else {
        return;
    };
    let config = project.load_config().ok();
    for (task_id, mut record) in records {
        let Some(mut run) = record.runs.last().cloned() else {
            continue;
        };
        let ticket_cat = ticket_category(project, config.as_ref(), &task_id);
        if !auto_cleanup_eligible(&run, ticket_cat) {
            continue;
        }
        let before_blocked = run.cleanup_blocked.clone();
        if !attempt_auto_cleanup(project_dir, &mut run) {
            continue;
        }
        if run.cleanup_blocked != before_blocked {
            if let Some(reason) = &run.cleanup_blocked {
                eprintln!(
                    "dispatcher: cleanup skipped for {task_id}: {reason}"
                );
            }
        }
        update_latest(&mut record, &run);
        if write_run_record(project.root(), &task_id, &record).is_ok() {
            broadcast_run_changed(ws_tx, &task_id, &run);
        }
    }
}

fn ticket_category(
    project: &Project,
    config: Option<&markplane_core::Config>,
    task_id: &str,
) -> Option<StatusCategory> {
    let doc = project.read_item::<Task>(task_id).ok()?;
    let cfg = config?;
    cfg.workflows.task.category_of(&doc.frontmatter.status)
}

fn cleanup_run(
    project_dir: &Path,
    run: &mut Run,
    force: bool,
    trigger: &str,
    preserve_branch_head: bool,
) -> bool {
    let mut changed = false;

    if !force {
        if let Some(_pid) = run_orphan_pid(run) {
            changed |= set_blocked(run, BLOCKED_ORPHAN);
            run.cleanup_trigger = Some(trigger.to_string());
            return changed;
        }
    }

    if run.worktree_removed_at.is_none() {
        changed |= cleanup_worktree(project_dir, run, force);
    }
    // A registered worktree still checks out `agent/<id>`, so `git branch -d`
    // cannot succeed until the worktree is gone. Skip the branch step only in
    // that case (or when an orphan is still running). `not_a_worktree` paths
    // are not checkouts, so the branch can still be removed independently.
    let skip_branch = !force
        && run.worktree_removed_at.is_none()
        && matches!(
            run.cleanup_blocked.as_deref(),
            Some(BLOCKED_UNCOMMITTED | BLOCKED_ORPHAN)
        );
    if !skip_branch && run.branch_removed_at.is_none() {
        let prev_blocked = run.cleanup_blocked.clone();
        let prefer_wt = run.cleanup_blocked.as_deref() == Some(BLOCKED_NOT_WORKTREE)
            && run.worktree_removed_at.is_none();
        changed |= cleanup_branch(project_dir, run, force, preserve_branch_head);
        if prefer_wt {
            run.cleanup_blocked = prev_blocked;
        }
    }

    if run.worktree_removed_at.is_some() && run.branch_removed_at.is_some() {
        if run.cleanup_blocked.is_some() {
            run.cleanup_blocked = None;
            changed = true;
        }
    }
    if changed {
        run.cleanup_trigger = Some(trigger.to_string());
    }
    changed
}

fn cleanup_worktree(project_dir: &Path, run: &mut Run, force: bool) -> bool {
    let wt = PathBuf::from(&run.worktree);
    let exists = wt.exists();
    let registered = is_registered_worktree(project_dir, &wt);

    if !exists {
        // Directory gone → prune stale registration.
        let _ = git_run(project_dir, &["worktree", "prune"]);
        run.worktree_removed_at = Some(Utc::now().to_rfc3339());
        if run.cleanup_blocked.as_deref() == Some(BLOCKED_NOT_WORKTREE)
            || run.cleanup_blocked.as_deref() == Some(BLOCKED_UNCOMMITTED)
            || run.cleanup_blocked.as_deref() == Some(BLOCKED_ORPHAN)
        {
            // clear worktree-related block if branch still blocked later
            if run.cleanup_blocked.as_deref() != Some(BLOCKED_UNMERGED) {
                run.cleanup_blocked = None;
            }
        }
        return true;
    }

    if !registered {
        // Present on disk but not a git worktree — never delete.
        return set_blocked(run, BLOCKED_NOT_WORKTREE);
    }

    // Only remove the recorded path (realpath match already done via registration).
    if !force {
        let dirty = count_porcelain_lines(&wt);
        if dirty > 0 {
            return set_blocked(run, BLOCKED_UNCOMMITTED);
        }
        match git_run(
            project_dir,
            &["worktree", "remove", &wt.display().to_string()],
        ) {
            Ok(()) => {
                run.worktree_removed_at = Some(Utc::now().to_rfc3339());
                if run.cleanup_blocked.as_deref() == Some(BLOCKED_UNCOMMITTED)
                    || run.cleanup_blocked.as_deref() == Some(BLOCKED_NOT_WORKTREE)
                    || run.cleanup_blocked.as_deref() == Some(BLOCKED_ORPHAN)
                {
                    run.cleanup_blocked = None;
                }
                true
            }
            Err(_) => {
                // git refused (often dirty) — treat as uncommitted
                set_blocked(run, BLOCKED_UNCOMMITTED)
            }
        }
    } else {
        let _ = git_run(
            project_dir,
            &[
                "worktree",
                "remove",
                "--force",
                &wt.display().to_string(),
            ],
        );
        if wt.exists() {
            let _ = fs::remove_dir_all(&wt);
        }
        let _ = git_run(project_dir, &["worktree", "prune"]);
        run.worktree_removed_at = Some(Utc::now().to_rfc3339());
        if run.cleanup_blocked.as_deref() == Some(BLOCKED_UNCOMMITTED)
            || run.cleanup_blocked.as_deref() == Some(BLOCKED_NOT_WORKTREE)
            || run.cleanup_blocked.as_deref() == Some(BLOCKED_ORPHAN)
        {
            run.cleanup_blocked = None;
        }
        true
    }
}

fn cleanup_branch(
    project_dir: &Path,
    run: &mut Run,
    force: bool,
    preserve_branch_head: bool,
) -> bool {
    // Never delete non-agent branches, even if a Run record names one.
    if !run.branch.starts_with("agent/") {
        if !branch_exists(project_dir, &run.branch) {
            run.branch_removed_at = Some(Utc::now().to_rfc3339());
            return true;
        }
        return false;
    }

    if !branch_exists(project_dir, &run.branch) {
        run.branch_removed_at = Some(Utc::now().to_rfc3339());
        if run.cleanup_blocked.as_deref() == Some(BLOCKED_UNMERGED) {
            run.cleanup_blocked = None;
        }
        return true;
    }

    let _ = preserve_branch_head; // caller may pre-set branch_head_at_removal
    if run.branch_head_at_removal.is_none() {
        run.branch_head_at_removal =
            git_stdout(project_dir, &["rev-parse", &run.branch]).ok();
    }

    let delete_flag = if force { "-D" } else { "-d" };
    match git_run(project_dir, &["branch", delete_flag, &run.branch]) {
        Ok(()) => {
            run.branch_removed_at = Some(Utc::now().to_rfc3339());
            if run.cleanup_blocked.as_deref() == Some(BLOCKED_UNMERGED) {
                run.cleanup_blocked = None;
            }
            true
        }
        Err(_) => {
            if force {
                // Already tried -D; leave as-is if still present.
                if !branch_exists(project_dir, &run.branch) {
                    run.branch_removed_at = Some(Utc::now().to_rfc3339());
                    return true;
                }
                false
            } else {
                set_blocked(run, BLOCKED_UNMERGED)
            }
        }
    }
}

fn set_blocked(run: &mut Run, reason: &str) -> bool {
    if run.cleanup_blocked.as_deref() == Some(reason) {
        return false;
    }
    run.cleanup_blocked = Some(reason.to_string());
    true
}

fn update_latest(record: &mut RunRecord, run: &Run) {
    if let Some(last) = record.runs.last_mut() {
        *last = run.clone();
    }
}

fn broadcast_run_changed(ws_tx: &broadcast::Sender<String>, task_id: &str, run: &Run) {
    let stage = run
        .current_stage
        .map(|s| s.as_str().to_string())
        .unwrap_or_default();
    let display =
        markplane_core::run_display_status(run, markplane_core::RunMemoryPresence::Absent);
    let msg = serde_json::json!({
        "type": "run_changed",
        "task_id": task_id,
        "seq": run.seq,
        "status": display,
        "stage": stage,
    })
    .to_string();
    let _ = ws_tx.send(msg);
}

fn is_registered_worktree(project_dir: &Path, worktree: &Path) -> bool {
    let Ok(list) = git_stdout(project_dir, &["worktree", "list", "--porcelain"]) else {
        return false;
    };
    let want = canonicalize(worktree);
    for line in list.lines() {
        let Some(path) = line.strip_prefix("worktree ") else {
            continue;
        };
        if canonicalize(Path::new(path)) == want {
            return true;
        }
    }
    false
}

fn canonicalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn branch_exists(project_dir: &Path, branch: &str) -> bool {
    git_run(
        project_dir,
        &["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")],
    )
    .is_ok()
}

fn count_porcelain_lines(worktree: &Path) -> u32 {
    let Ok(out) = git_stdout(worktree, &["status", "--porcelain"]) else {
        return 0;
    };
    out.lines().filter(|l| !l.is_empty()).count() as u32
}

fn count_unmerged_commits(project_dir: &Path, branch: &str) -> u32 {
    // Commits on branch not reachable from current HEAD.
    match git_stdout(project_dir, &["rev-list", "--count", &format!("HEAD..{branch}")]) {
        Ok(s) => s.trim().parse().unwrap_or(0),
        Err(_) => 0,
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

#[cfg(test)]
mod tests {
    use super::*;
    use markplane_core::{Stage, ensure_runs_dir};
    use tempfile::TempDir;

    fn init_repo() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        run(&root, &["git", "init", "-b", "main"]);
        run(
            &root,
            &["git", "config", "user.email", "test@example.com"],
        );
        run(&root, &["git", "config", "user.name", "Test"]);
        fs::write(root.join("README"), "hi").unwrap();
        run(&root, &["git", "add", "."]);
        run(&root, &["git", "commit", "-m", "init"]);
        (tmp, root)
    }

    fn run(cwd: &Path, args: &[&str]) {
        let st = Command::new(args[0])
            .args(&args[1..])
            .current_dir(cwd)
            .status()
            .unwrap();
        assert!(st.success(), "{args:?}");
    }

    fn sample_run(worktree: &Path, branch: &str) -> Run {
        Run {
            seq: 1,
            status: RunStatus::Succeeded,
            current_stage: Some(Stage::Merge),
            worktree: worktree.display().to_string(),
            branch: branch.to_string(),
            base_sha: "abc".into(),
            merge_target: "main".into(),
            merge_sha: Some("def".into()),
            pushed: false,
            started_at: "2026-10-10T10:00:00+08:00".into(),
            ended_at: Some("2026-10-10T11:00:00+08:00".into()),
            failure_reason: None,
            worktree_removed_at: None,
            branch_removed_at: None,
            branch_head_at_removal: None,
            cleanup_blocked: None,
            cleanup_trigger: None,
            attempts: vec![],
        }
    }

    fn add_agent_worktree(root: &Path, task_id: &str) -> (PathBuf, String) {
        let branch = format!("agent/{task_id}");
        let wt = root.join(".worktrees").join(task_id);
        fs::create_dir_all(wt.parent().unwrap()).unwrap();
        run(
            root,
            &[
                "git",
                "worktree",
                "add",
                "-b",
                &branch,
                wt.to_str().unwrap(),
                "HEAD",
            ],
        );
        (wt, branch)
    }

    #[test]
    fn auto_removes_clean_succeeded_worktree_and_merged_branch() {
        let (_tmp, root) = init_repo();
        let (wt, branch) = add_agent_worktree(&root, "TASK-clean");
        // Merge branch into main so -d works.
        run(&root, &["git", "merge", "--no-ff", "-m", "merge agent", &branch]);
        let mut run_rec = sample_run(&wt, &branch);
        assert!(attempt_auto_cleanup(&root, &mut run_rec));
        assert!(run_rec.worktree_removed_at.is_some());
        assert!(run_rec.branch_removed_at.is_some());
        assert!(run_rec.branch_head_at_removal.is_some());
        assert_eq!(run_rec.cleanup_trigger.as_deref(), Some(TRIGGER_AUTO));
        assert!(run_rec.cleanup_blocked.is_none());
        assert!(!wt.exists());
        assert!(!branch_exists(&root, &branch));
    }

    #[test]
    fn auto_skips_uncommitted_changes() {
        let (_tmp, root) = init_repo();
        let (wt, branch) = add_agent_worktree(&root, "TASK-dirty");
        fs::write(wt.join("scratch.txt"), "dirty").unwrap();
        let mut run_rec = sample_run(&wt, &branch);
        assert!(attempt_auto_cleanup(&root, &mut run_rec));
        assert!(run_rec.worktree_removed_at.is_none());
        // Branch stays: still checked out in the dirty worktree.
        assert!(run_rec.branch_removed_at.is_none());
        assert_eq!(
            run_rec.cleanup_blocked.as_deref(),
            Some(BLOCKED_UNCOMMITTED)
        );
        assert!(wt.exists());
        assert!(branch_exists(&root, &branch));
    }

    #[test]
    fn auto_removes_branch_when_path_is_not_a_worktree() {
        let (_tmp, root) = init_repo();
        let (wt, branch) = add_agent_worktree(&root, "TASK-mix");
        run(
            &root,
            &["git", "merge", "--no-ff", "-m", "merge agent", &branch],
        );
        // Detach the worktree registration but leave the directory + branch.
        run(
            &root,
            &["git", "worktree", "remove", "--force", wt.to_str().unwrap()],
        );
        fs::create_dir_all(&wt).unwrap();
        fs::write(wt.join("x"), "y").unwrap();
        let mut run_rec = sample_run(&wt, &branch);
        assert!(attempt_auto_cleanup(&root, &mut run_rec));
        assert!(run_rec.worktree_removed_at.is_none());
        assert_eq!(
            run_rec.cleanup_blocked.as_deref(),
            Some(BLOCKED_NOT_WORKTREE)
        );
        assert!(
            run_rec.branch_removed_at.is_some(),
            "merged agent branch removed even when path is not a worktree"
        );
        assert!(!branch_exists(&root, &branch));
    }

    #[test]
    fn auto_skips_unmerged_branch_after_worktree_removed() {
        let (_tmp, root) = init_repo();
        let (wt, branch) = add_agent_worktree(&root, "TASK-unmerged");
        // Commit on agent branch only — not merged into main.
        fs::write(wt.join("feat.txt"), "x").unwrap();
        run(&wt, &["git", "add", "."]);
        run(&wt, &["git", "commit", "-m", "feat"]);
        let mut run_rec = sample_run(&wt, &branch);
        assert!(attempt_auto_cleanup(&root, &mut run_rec));
        assert!(run_rec.worktree_removed_at.is_some());
        assert!(run_rec.branch_removed_at.is_none());
        assert_eq!(
            run_rec.cleanup_blocked.as_deref(),
            Some(BLOCKED_UNMERGED)
        );
        assert!(branch_exists(&root, &branch));
    }

    #[test]
    fn auto_skips_directory_that_is_not_a_worktree() {
        let (_tmp, root) = init_repo();
        let wt = root.join(".worktrees").join("TASK-fake");
        fs::create_dir_all(&wt).unwrap();
        fs::write(wt.join("x"), "y").unwrap();
        let mut run_rec = sample_run(&wt, "agent/TASK-fake");
        assert!(attempt_auto_cleanup(&root, &mut run_rec));
        assert!(run_rec.worktree_removed_at.is_none());
        assert_eq!(
            run_rec.cleanup_blocked.as_deref(),
            Some(BLOCKED_NOT_WORKTREE)
        );
        assert!(wt.exists());
    }

    #[test]
    fn force_removes_dirty_worktree_and_unmerged_branch() {
        let (_tmp, root) = init_repo();
        let (wt, branch) = add_agent_worktree(&root, "TASK-force");
        fs::write(wt.join("scratch.txt"), "dirty").unwrap();
        fs::write(wt.join("feat.txt"), "x").unwrap();
        run(&wt, &["git", "add", "feat.txt"]);
        run(&wt, &["git", "commit", "-m", "feat"]);
        let mut run_rec = sample_run(&wt, &branch);
        attempt_force_cleanup(&root, &mut run_rec, false).unwrap();
        assert!(run_rec.worktree_removed_at.is_some());
        assert!(run_rec.branch_removed_at.is_some());
        assert_eq!(run_rec.cleanup_trigger.as_deref(), Some(TRIGGER_MANUAL));
        assert!(!wt.exists());
        assert!(!branch_exists(&root, &branch));
    }

    #[test]
    fn never_deletes_unregistered_path_even_on_force() {
        let (_tmp, root) = init_repo();
        let handmade = root.join("handmade-wt");
        fs::create_dir_all(&handmade).unwrap();
        fs::write(handmade.join("keep"), "me").unwrap();
        // Record points at handmade path — not a registered worktree.
        let mut run_rec = sample_run(&handmade, "feature/mine");
        attempt_force_cleanup(&root, &mut run_rec, false).unwrap();
        // Force still refuses not_a_worktree for safety (spec: never touch unregistered).
        // Actually re-read: "绝不碰未登记的 worktree" — registered means in Run record
        // AND matched via git worktree list. Directory not in git worktree list → not_a_worktree, don't delete.
        assert!(handmade.exists());
        assert_eq!(
            run_rec.cleanup_blocked.as_deref(),
            Some(BLOCKED_NOT_WORKTREE)
        );
    }

    #[test]
    fn repickup_records_branch_head_before_force_delete() {
        let (_tmp, root) = init_repo();
        let (wt, branch) = add_agent_worktree(&root, "TASK-retry");
        fs::write(wt.join("feat.txt"), "x").unwrap();
        run(&wt, &["git", "add", "."]);
        run(&wt, &["git", "commit", "-m", "feat"]);
        let head = git_stdout(&root, &["rev-parse", &branch]).unwrap();
        let mut run_rec = sample_run(&wt, &branch);
        run_rec.status = RunStatus::Failed;
        assert!(force_cleanup_for_repickup(&root, &mut run_rec));
        assert_eq!(run_rec.branch_head_at_removal.as_deref(), Some(head.as_str()));
        assert!(run_rec.worktree_removed_at.is_some());
        assert!(run_rec.branch_removed_at.is_some());
        assert_eq!(run_rec.cleanup_trigger.as_deref(), Some(TRIGGER_REPICKUP));
    }

    #[test]
    fn missing_worktree_dir_prunes_and_marks_removed() {
        let (_tmp, root) = init_repo();
        let (wt, branch) = add_agent_worktree(&root, "TASK-gone");
        run(
            &root,
            &["git", "merge", "--no-ff", "-m", "merge", &branch],
        );
        // Remove directory without git worktree remove.
        fs::remove_dir_all(&wt).unwrap();
        let mut run_rec = sample_run(&wt, &branch);
        assert!(attempt_auto_cleanup(&root, &mut run_rec));
        assert!(run_rec.worktree_removed_at.is_some());
        assert!(run_rec.branch_removed_at.is_some());
    }

    #[test]
    fn eligibility_helpers() {
        let run = sample_run(Path::new("/tmp/wt"), "agent/TASK-x");
        assert!(auto_cleanup_eligible(&run, Some(StatusCategory::Completed)));
        let mut failed = run.clone();
        failed.status = RunStatus::Failed;
        assert!(!auto_cleanup_eligible(
            &failed,
            Some(StatusCategory::Active)
        ));
        assert!(auto_cleanup_eligible(
            &failed,
            Some(StatusCategory::Completed)
        ));
        assert!(auto_cleanup_eligible(&failed, None)); // ticket file gone

        assert!(!manual_cleanup_eligible(
            &failed,
            "failed",
            Some(StatusCategory::Active)
        ));
        assert!(manual_cleanup_eligible(
            &failed,
            "failed",
            Some(StatusCategory::Completed)
        ));
        assert!(manual_cleanup_eligible(
            &run,
            "succeeded",
            Some(StatusCategory::Completed)
        ));
        assert!(!manual_cleanup_eligible(
            &run,
            "interrupted",
            Some(StatusCategory::Active)
        ));
    }

    #[test]
    fn preview_counts_uncommitted_and_unmerged() {
        let (_tmp, root) = init_repo();
        let (wt, branch) = add_agent_worktree(&root, "TASK-prev");
        fs::write(wt.join("a.txt"), "1").unwrap();
        fs::write(wt.join("b.txt"), "2").unwrap();
        fs::write(wt.join("feat.txt"), "x").unwrap();
        run(&wt, &["git", "add", "feat.txt"]);
        run(&wt, &["git", "commit", "-m", "feat"]);
        let run_rec = sample_run(&wt, &branch);
        let preview = cleanup_preview(
            &root,
            &run_rec,
            "succeeded",
            Some(StatusCategory::Completed),
        );
        assert!(preview.eligible);
        assert_eq!(preview.uncommitted_files, 2);
        assert!(preview.unmerged_commits >= 1);
        assert!(preview.branch_head.is_some());
        assert!(preview.worktree_present);
        assert!(preview.worktree_registered);
        assert!(preview.branch_present);
        assert!(preview.branch_is_agent);
    }

    #[test]
    fn preview_not_eligible_when_nothing_deletable() {
        let (_tmp, root) = init_repo();
        let handmade = root.join("handmade");
        fs::create_dir_all(&handmade).unwrap();
        let run_rec = sample_run(&handmade, "feature/mine");
        let preview = cleanup_preview(
            &root,
            &run_rec,
            "failed",
            Some(StatusCategory::Completed),
        );
        assert!(!preview.worktree_registered);
        assert!(!preview.branch_present);
        assert!(!preview.eligible);
    }

    #[test]
    fn ensure_runs_dir_smoke() {
        let (_tmp, root) = init_repo();
        let mp = root.join(".markplane");
        fs::create_dir_all(&mp).unwrap();
        ensure_runs_dir(&mp).unwrap();
    }
}
