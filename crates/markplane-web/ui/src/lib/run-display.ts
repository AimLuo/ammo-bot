import type {
  Attempt,
  AttemptResult,
  Run,
  RunDisplayStatus,
  RunSummary,
  Stage,
  StatusCategory,
  Task,
  TaskWorkflow,
} from "@/lib/types";
import { categoryOf } from "@/lib/constants";
import { fmtDuration, sumUsage } from "@/lib/session-log";

/** Runs newest-first (highest seq first). */
export function runsNewestFirst(runs: Run[]): Run[] {
  return [...runs].sort((a, b) => b.seq - a.seq);
}

/** Whether the fix Stage should appear (REVIEW: issues path or fix attempts exist). */
export function showFixStage(run: Run): boolean {
  return run.attempts.some(
    (a) =>
      a.stage === "fix" ||
      (a.stage === "implement" &&
        !!a.verdict &&
        /\bREVIEW:\s*issues\b/i.test(a.verdict)),
  );
}

/** Stages to render for this Run's progress row. */
export function stagesForRun(run: Run): Stage[] {
  return showFixStage(run)
    ? ["implement", "fix", "merge"]
    : ["implement", "merge"];
}

export interface StageSummary {
  stage: Stage;
  /** Representative attempt (last non-resume_mismatch, else last). */
  attempt: Attempt | null;
  result: AttemptResult | null | undefined;
  model: string | null;
  startedAt: string | null;
  endedAt: string | null | undefined;
  durationLabel: string | null;
  inputTokens: number;
  outputTokens: number;
  unknownUsage: boolean;
  /** True when merge is waiting in the merge queue (no merge attempt yet). */
  queued: boolean;
  /** True when this Stage has not started and is not the active one. */
  pending: boolean;
}

function attemptsForStage(run: Run, stage: Stage): Attempt[] {
  return run.attempts.filter((a) => a.stage === stage);
}

function pickRepresentative(attempts: Attempt[]): Attempt | null {
  if (attempts.length === 0) return null;
  for (let i = attempts.length - 1; i >= 0; i--) {
    const a = attempts[i]!;
    if (a.result !== "resume_mismatch") return a;
  }
  return attempts[attempts.length - 1]!;
}

function stageOverallResult(attempts: Attempt[]): AttemptResult | null | undefined {
  const rep = pickRepresentative(attempts);
  return rep?.result;
}

/** Build per-Stage summaries for the progress row. */
export function stageSummaries(run: Run): StageSummary[] {
  const stages = stagesForRun(run);
  return stages.map((stage) => {
    const attempts = attemptsForStage(run, stage);
    const queued =
      stage === "merge" &&
      run.status === "merge-queued" &&
      attempts.length === 0;
    const pending =
      attempts.length === 0 &&
      !queued &&
      run.current_stage !== stage &&
      !(run.status === "running" && run.current_stage === stage);
    const rep = pickRepresentative(attempts);
    const usage = sumUsage(attempts);
    const startedAt = attempts[0]?.started_at ?? null;
    const lastEnded = [...attempts].reverse().find((a) => a.ended_at)?.ended_at;
    const inFlight =
      attempts.some((a) => a.result == null) ||
      (run.status === "running" && run.current_stage === stage);
    const endedAt = inFlight ? null : lastEnded;
    const durationLabel =
      startedAt != null ? fmtDuration(startedAt, endedAt) : null;

    return {
      stage,
      attempt: rep,
      result: stageOverallResult(attempts),
      model: rep?.model ?? null,
      startedAt,
      endedAt,
      durationLabel,
      inputTokens: usage.inputTokens,
      outputTokens: usage.outputTokens,
      unknownUsage: usage.unknown,
      queued,
      pending: pending && !inFlight,
    };
  });
}

/** Short SHA for display (7 chars when looks like hex). */
export function shortSha(sha: string): string {
  if (/^[0-9a-f]{7,40}$/i.test(sha)) return sha.slice(0, 7);
  return sha;
}

/**
 * Remnant / cleanup status line for a Run.
 * Counts of uncommitted files are not in the record — show reason labels instead.
 */
export function remnantStatusLine(run: Run): string | null {
  const wtGone = !!run.worktree_removed_at;
  const brGone = !!run.branch_removed_at;
  if (wtGone && brGone) {
    return `Cleaned up ${fmtTimestamp(run.worktree_removed_at!)}`;
  }
  if (wtGone && !brGone) {
    const blocked = formatCleanupBlocked(run.cleanup_blocked);
    if (blocked?.startsWith("Branch")) return blocked;
    return "Worktree removed · branch kept";
  }
  if (!wtGone && brGone) {
    const blocked = formatCleanupBlocked(run.cleanup_blocked);
    if (blocked?.startsWith("Worktree")) return blocked;
    return "Branch removed · worktree kept";
  }

  const blocked = formatCleanupBlocked(run.cleanup_blocked);
  if (blocked) return blocked;

  const terminal =
    run.status === "failed" ||
    run.status === "stopped" ||
    run.status === "succeeded";
  if (terminal) {
    return "Worktree kept · branch kept";
  }
  return null;
}

function formatCleanupBlocked(reason: string | null | undefined): string | null {
  if (!reason) return null;
  switch (reason) {
    case "uncommitted_changes":
      return "Worktree kept · uncommitted changes";
    case "unmerged_commits":
      return "Branch kept · unmerged commits";
    case "orphan_running":
      return "Worktree kept · orphan still running";
    case "not_a_worktree":
      return "Worktree kept · not a worktree";
    default:
      return `Cleanup blocked · ${reason}`;
  }
}

/** Locale timestamp for "Cleaned up …". */
function fmtTimestamp(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

/** Start time for the current Stage's elapsed duration (RunBadge). */
export function currentStageStartedAt(run: Run): string {
  const stage = run.current_stage;
  if (!stage) return run.started_at;
  const first = run.attempts.find((a) => a.stage === stage);
  return first?.started_at ?? run.started_at;
}

/** Ending Stage for earlier-runs list (last attempt's stage, else current). */
export function endingStage(run: Run): Stage | null {
  const last = run.attempts[run.attempts.length - 1];
  return last?.stage ?? run.current_stage ?? null;
}

/** Whether the Dispatcher still considers this Run live (Slot-holding). */
export function isLiveRunStatus(status: string): boolean {
  return status === "running" || status === "merge-queued";
}

/**
 * Client-only stopping: Run is still live server-side, but the ticket left
 * active or assignee is no longer `dispatcher`.
 */
export function deriveDisplayStatus(
  summary: RunSummary,
  task: Pick<Task, "status" | "assignee">,
  workflow: TaskWorkflow | undefined,
): RunDisplayStatus {
  const base = summary.status as RunDisplayStatus;
  if (!isLiveRunStatus(summary.status)) return base;
  // Assignee can be checked without workflow; category needs it (avoid
  // flashing Stopping while config is still loading).
  if (task.assignee !== "dispatcher") return "stopping";
  if (!workflow) return base;
  if (categoryOf(workflow, task.status) !== "active") return "stopping";
  return base;
}

/** Whether RunBadge should render for this ticket + summary. */
export function shouldShowRunBadge(
  displayStatus: RunDisplayStatus,
  ticketCategory: StatusCategory | undefined,
): boolean {
  if (displayStatus === "succeeded") return false;
  if (
    displayStatus === "failed" ||
    displayStatus === "interrupted" ||
    displayStatus === "stopped"
  ) {
    return ticketCategory === "active";
  }
  return true;
}

/** Stage elapsed label (`<1m`, `12m`, …) using a wall-clock `now` ms. */
export function stageElapsedLabel(
  stageStartedAt: string | null | undefined,
  endedAt: string | null | undefined,
  nowMs: number,
): string {
  if (!stageStartedAt) return "<1m";
  const end = endedAt ? new Date(endedAt).getTime() : nowMs;
  const ms = end - new Date(stageStartedAt).getTime();
  const m = Math.round(ms / 60000);
  if (m < 1) return "<1m";
  return m >= 60 ? `${Math.floor(m / 60)}h ${m % 60}m` : `${m}m`;
}

/** Visible badge text for a derived display status. */
export function runBadgeLabel(
  displayStatus: RunDisplayStatus,
  summary: RunSummary,
  nowMs: number,
): string {
  const stage = summary.stage ?? undefined;
  const elapsed = stageElapsedLabel(
    summary.stage_started_at,
    summary.ended_at,
    nowMs,
  );
  switch (displayStatus) {
    case "running":
      return stage ? `${stage} · ${elapsed}` : `running · ${elapsed}`;
    case "merge-queued": {
      const n = summary.queue_position;
      const q = n != null ? `merge queue #${n}` : "merge queue";
      return `${q} · ${elapsed}`;
    }
    case "stopping":
      return stage ? `Stopping · ${stage}` : "Stopping";
    case "interrupted":
      return stage ? `Interrupted · ${stage}` : "Interrupted";
    case "failed":
      return stage ? `Run failed · ${stage}` : "Run failed";
    case "stopped":
      return "Run stopped";
    case "running elsewhere":
      return "running elsewhere";
    case "succeeded":
      return "succeeded";
    default:
      return displayStatus;
  }
}

/** Hover tip lines for RunBadge. */
export function runBadgeTitle(
  summary: RunSummary,
  nowMs: number,
): string {
  const total = stageElapsedLabel(summary.started_at, summary.ended_at, nowMs);
  const stageStart = summary.stage_started_at
    ? new Date(summary.stage_started_at).toLocaleString("en-US", {
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      })
    : "—";
  const model = summary.model ?? "—";
  const timeout =
    summary.timeout_secs != null
      ? `${Math.round(summary.timeout_secs / 60)}m timeout`
      : "—";
  const attempts =
    summary.stage_attempt_count != null
      ? `${summary.stage_attempt_count} call${summary.stage_attempt_count === 1 ? "" : "s"}`
      : "—";
  return [
    `Run #${summary.seq}`,
    `Total ${total}`,
    `Stage started ${stageStart}`,
    `${model} · ${timeout}`,
    attempts,
  ].join("\n");
}

/** Build a RunSummary-shaped view from a full Run (detail panel). */
export function summaryFromRun(
  taskId: string,
  run: Run,
  extras?: {
    status?: string;
    queue_position?: number | null;
  },
): RunSummary {
  const stage = run.current_stage ?? null;
  const stageAttempts = stage
    ? run.attempts.filter((a) => a.stage === stage)
    : [];
  const lastAttempt = stageAttempts[stageAttempts.length - 1];
  return {
    task_id: taskId,
    seq: run.seq,
    status: extras?.status ?? run.status,
    stage,
    stage_started_at: currentStageStartedAt(run),
    started_at: run.started_at,
    queue_position: extras?.queue_position ?? null,
    failure_reason: run.failure_reason ?? null,
    ended_at: run.ended_at ?? null,
    model: lastAttempt?.model ?? null,
    timeout_secs: lastAttempt?.timeout_secs ?? null,
    stage_attempt_count: stageAttempts.length > 0 ? stageAttempts.length : null,
  };
}

/** True when changing status/assignee should prompt before silent stop. */
export function hasActiveDispatcherRun(summary: RunSummary | undefined): boolean {
  return !!summary && isLiveRunStatus(summary.status);
}

export function activeRunConfirmMessage(
  taskId: string,
  summary: RunSummary,
): string {
  const stage = summary.stage ?? "current stage";
  return `Dispatcher is running ${taskId} (${stage}). It will stop after the current stage ends; the agent process is not killed and the worktree is kept. Continue?`;
}

/**
 * Whether the Run section should show "Clean up…".
 * Mirrors server `manual_cleanup_eligible`: not live/interrupted/elsewhere,
 * not failed-while-active, and at least one remnant remains.
 */
export function canCleanupRun(
  run: Run,
  displayStatus: string | undefined,
  task: Pick<Task, "status">,
  workflow?: TaskWorkflow,
): boolean {
  if (run.worktree_removed_at && run.branch_removed_at) return false;
  const status = displayStatus ?? run.status;
  if (
    status === "running" ||
    status === "merge-queued" ||
    status === "stopping" ||
    status === "running elsewhere" ||
    status === "interrupted"
  ) {
    return false;
  }
  const cat: StatusCategory | undefined = workflow
    ? categoryOf(workflow, task.status)
    : undefined;
  if (run.status === "failed" && cat === "active") return false;
  return true;
}

/** Confirm-dialog body for Clean up from a cleanup preview. */
export function cleanupConfirmMessage(preview: {
  worktree: string;
  worktree_present: boolean;
  uncommitted_files: number;
  branch: string;
  branch_present: boolean;
  unmerged_commits: number;
  branch_head?: string | null;
  orphan_pid?: number | null;
}): string {
  const lines: string[] = ["Force-delete Dispatcher remnants?"];
  if (preview.worktree_present) {
    lines.push(
      `Worktree: ${preview.worktree} (${preview.uncommitted_files} uncommitted file${preview.uncommitted_files === 1 ? "" : "s"})`,
    );
  }
  if (preview.branch_present) {
    const head = preview.branch_head
      ? preview.branch_head.slice(0, 12)
      : "unknown";
    lines.push(
      `Branch: ${preview.branch} (${preview.unmerged_commits} unmerged commit${preview.unmerged_commits === 1 ? "" : "s"}, head ${head})`,
    );
  }
  if (preview.orphan_pid != null) {
    lines.push(`Orphan agent pid ${preview.orphan_pid} will be terminated.`);
  }
  return lines.join("\n");
}
