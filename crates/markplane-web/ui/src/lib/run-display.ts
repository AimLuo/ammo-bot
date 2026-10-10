import type { Attempt, AttemptResult, Run, Stage } from "@/lib/types";
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

/** Compact status label used until TASK-gtgfz ships RunBadge. */
export function runStatusStubLabel(run: Run): string {
  const stageElapsed = fmtDuration(currentStageStartedAt(run), run.ended_at);
  switch (run.status) {
    case "running":
      return run.current_stage
        ? `${run.current_stage} · ${stageElapsed}`
        : "running";
    case "merge-queued":
      return `merge queue · ${stageElapsed}`;
    case "succeeded":
      return "succeeded";
    case "failed":
      return run.current_stage
        ? `Run failed · ${run.current_stage}`
        : "Run failed";
    case "stopped":
      return "Run stopped";
    default:
      return run.status;
  }
}
