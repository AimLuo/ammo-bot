//! Interrupt recognition helpers, orphan detection, and Resume planning.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use markplane_core::{Attempt, Run, RunStatus, Stage, agent_config_dir, session_meta_exists};

/// Grace period after SIGTERM before SIGKILL on serve shutdown / orphan kill.
pub const SHUTDOWN_KILL_GRACE: Duration = Duration::from_secs(5);

/// How to continue an interrupted Run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumePlan {
    /// Last attempt finished; apply Stage transitions without another agent call.
    AdvanceCompleted { stage: Stage },
    /// Re-enter merge queue (optionally at the front after a mid-merge interrupt).
    RequeueMerge { front: bool },
    /// Target already contains the agent branch — finish success without agent.
    FinishMergeSuccess,
    /// Run agent for `stage`, optionally `--resume` an existing session.
    RunAgent {
        stage: Stage,
        resume_session_id: Option<String>,
        /// Fresh session after interrupt (inject prior-halfdone / resume prompt).
        interrupted_fresh: bool,
    },
}

/// Plan the next Resume action from the latest Run record (no I/O besides session meta).
pub fn plan_resume(run: &Run, config_dir: &Path) -> Result<ResumePlan, String> {
    match run.status {
        RunStatus::Succeeded | RunStatus::Failed | RunStatus::Stopped => {
            return Err("run is not interrupted".into());
        }
        RunStatus::MergeQueued => {
            return Ok(ResumePlan::RequeueMerge { front: false });
        }
        RunStatus::Running => {}
    }

    let stage = run.current_stage.unwrap_or(Stage::Implement);
    let last = run.attempts.last();

    // Mid-merge with no completed result: maybe already landed.
    if stage == Stage::Merge {
        if let Some(a) = last {
            if a.stage == Stage::Merge && a.result.is_none() {
                // Caller checks ancestor; we signal requeue-front vs finish.
                return Ok(ResumePlan::RequeueMerge { front: true });
            }
            if a.stage == Stage::Merge && a.result.is_some() {
                return Ok(ResumePlan::AdvanceCompleted {
                    stage: Stage::Merge,
                });
            }
        } else {
            return Ok(ResumePlan::RequeueMerge { front: false });
        }
    }

    let Some(last) = last else {
        return Ok(ResumePlan::RunAgent {
            stage,
            resume_session_id: None,
            interrupted_fresh: true,
        });
    };

    if last.result.is_some() {
        return Ok(ResumePlan::AdvanceCompleted {
            stage: last.stage,
        });
    }

    // Incomplete attempt — try session resume.
    let resume_session_id = last
        .session_id
        .as_ref()
        .filter(|sid| !sid.is_empty())
        .filter(|sid| session_meta_exists(config_dir, &last.workspace, sid))
        .cloned();

    Ok(ResumePlan::RunAgent {
        stage: last.stage,
        resume_session_id,
        interrupted_fresh: true,
    })
}

/// Whether `pid` looks like a leftover agent for `worktree`.
///
/// Requires the process to be alive and its command line to contain both
/// `agent` and the worktree path (guards against PID reuse).
pub fn orphan_agent_pid(last: &Attempt, worktree: &str) -> Option<u32> {
    let pid = last.pid?;
    if last.result.is_some() {
        return None;
    }
    if !pid_alive(pid) {
        return None;
    }
    let cmdline = process_cmdline(pid)?;
    let wt = Path::new(worktree)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(worktree));
    let wt_str = wt.to_string_lossy();
    if cmdline.contains("agent")
        && (cmdline.contains(wt_str.as_ref()) || cmdline.contains(worktree))
    {
        Some(pid)
    } else {
        None
    }
}

/// Orphan PID for the latest attempt of a Run, if any.
pub fn run_orphan_pid(run: &Run) -> Option<u32> {
    let last = run.attempts.last()?;
    orphan_agent_pid(last, &run.worktree)
}

pub fn pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // signal 0: existence check
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

fn process_cmdline(pid: u32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let raw = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
        Some(String::from_utf8_lossy(&raw).replace('\0', " "))
    }
    #[cfg(target_os = "macos")]
    {
        let out = Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "command="])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        None
    }
}

/// SIGTERM the process group, wait up to `grace`, then SIGKILL.
pub fn kill_process_group(pgid: u32, grace: Duration) {
    #[cfg(unix)]
    {
        unsafe {
            // Prefer the specific pid first (pgid may equal pid for process-group leaders).
            libc::kill(pgid as i32, libc::SIGTERM);
            libc::kill(-(pgid as i32), libc::SIGTERM);
        }
        let deadline = std::time::Instant::now() + grace;
        while std::time::Instant::now() < deadline {
            if !pid_alive(pgid) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        unsafe {
            libc::kill(pgid as i32, libc::SIGKILL);
            libc::kill(-(pgid as i32), libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (pgid, grace);
    }
}

/// Kill an orphan from the last attempt (process group preferred).
pub fn terminate_orphan_attempt(last: &Attempt) {
    let pgid = last.process_group_id.or(last.pid);
    if let Some(pgid) = pgid {
        kill_process_group(pgid, SHUTDOWN_KILL_GRACE);
    }
}

/// Default config dir recorded on attempts (resolved at spawn time).
pub fn current_agent_config_dir() -> String {
    agent_config_dir().display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use markplane_core::{AttemptResult, Trigger, Usage};
    use std::fs;
    use tempfile::TempDir;

    fn attempt(stage: Stage, result: Option<AttemptResult>, sid: Option<&str>) -> Attempt {
        Attempt {
            stage,
            model: "m".into(),
            timeout_secs: Some(60),
            session_id: sid.map(str::to_string),
            workspace: "/tmp/wt".into(),
            transcript_path: None,
            pid: None,
            process_group_id: None,
            agent_config_dir: None,
            trigger: Trigger::Initial,
            started_at: "t0".into(),
            ended_at: result.map(|_| "t1".into()),
            exit_code: result.map(|_| 0),
            verdict: None,
            result,
            usage: None::<Usage>,
            duration_api_ms: None,
            stderr_tail: None,
        }
    }

    fn run_with(status: RunStatus, stage: Stage, attempts: Vec<Attempt>) -> Run {
        Run {
            seq: 1,
            status,
            current_stage: Some(stage),
            worktree: "/tmp/wt".into(),
            branch: "agent/TASK-x".into(),
            base_sha: "aaa".into(),
            merge_target: "main".into(),
            merge_sha: None,
            pushed: false,
            started_at: "t0".into(),
            ended_at: None,
            failure_reason: None,
            worktree_removed_at: None,
            branch_removed_at: None,
            branch_head_at_removal: None,
            cleanup_blocked: None,
            cleanup_trigger: None,
            attempts,
        }
    }

    #[test]
    fn plan_merge_queued_requeues() {
        let run = run_with(RunStatus::MergeQueued, Stage::Merge, vec![]);
        assert_eq!(
            plan_resume(&run, Path::new("/tmp")).unwrap(),
            ResumePlan::RequeueMerge { front: false }
        );
    }

    #[test]
    fn plan_completed_implement_advances() {
        let mut a = attempt(Stage::Implement, Some(AttemptResult::Ok), Some("s1"));
        a.verdict = Some("REVIEW: clean".into());
        let run = run_with(RunStatus::Running, Stage::Implement, vec![a]);
        assert_eq!(
            plan_resume(&run, Path::new("/tmp")).unwrap(),
            ResumePlan::AdvanceCompleted {
                stage: Stage::Implement
            }
        );
    }

    #[test]
    fn plan_incomplete_with_meta_resumes_session() {
        let tmp = TempDir::new().unwrap();
        let workspace = "/tmp/wt";
        let sid = "sess-1";
        let meta = markplane_core::session_meta_path(tmp.path(), workspace, sid);
        fs::create_dir_all(meta.parent().unwrap()).unwrap();
        fs::write(&meta, "{}").unwrap();
        let a = attempt(Stage::Fix, None, Some(sid));
        let run = run_with(RunStatus::Running, Stage::Fix, vec![a]);
        assert_eq!(
            plan_resume(&run, tmp.path()).unwrap(),
            ResumePlan::RunAgent {
                stage: Stage::Fix,
                resume_session_id: Some(sid.into()),
                interrupted_fresh: true,
            }
        );
    }

    #[test]
    fn plan_incomplete_without_meta_fresh() {
        let a = attempt(Stage::Implement, None, Some("missing"));
        let run = run_with(RunStatus::Running, Stage::Implement, vec![a]);
        assert_eq!(
            plan_resume(&run, Path::new("/tmp/no-such-config")).unwrap(),
            ResumePlan::RunAgent {
                stage: Stage::Implement,
                resume_session_id: None,
                interrupted_fresh: true,
            }
        );
    }
}
