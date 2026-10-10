"use client";

import { useState, type ReactNode } from "react";
import {
  Check,
  ChevronRight,
  CircleCheck,
  CirclePause,
  CircleX,
  Clock,
  Copy,
  LoaderCircle,
  Minus,
  ScrollText,
  TriangleAlert,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { RunBadge } from "@/components/domain/run-badge";
import { useRunRecord, useRunSummaryMap } from "@/lib/hooks/use-runs";
import { useConfig } from "@/lib/hooks/use-config";
import { cn } from "@/lib/utils";
import {
  fmtDuration,
  fmtTime,
  fmtTokens,
  sumUsage,
  usageTooltip,
} from "@/lib/session-log";
import {
  endingStage,
  remnantStatusLine,
  runsNewestFirst,
  shortSha,
  stageSummaries,
  summaryFromRun,
  type StageSummary,
} from "@/lib/run-display";
import type {
  Attempt,
  AttemptResult,
  Run,
  RunSummary,
  Task,
  TaskWorkflow,
} from "@/lib/types";
import { SessionLogModal } from "./session-log-modal";

/**
 * Full Run section for the Task detail panel (TASK-d7i23).
 * Shown when this machine has a Run record, or when assignee is `dispatcher`
 * with no local record (grey hint). Action slot: View sessions only for now.
 */
export function RunSection({
  taskId,
  task,
}: {
  taskId: string;
  /** Full task (status/assignee) for RunBadge stopping / visibility. */
  task: Pick<Task, "status" | "assignee">;
}) {
  const assignee = task.assignee;
  const { data: record, isLoading } = useRunRecord(taskId);
  const { map: runMap } = useRunSummaryMap();
  const { data: config } = useConfig();
  const workflow = config?.workflows.task;
  const [open, setOpen] = useState(false);

  const hasRecord = !!record && record.runs.length > 0;
  const dispatcherElsewhere = assignee === "dispatcher" && !hasRecord;

  if (isLoading) return null;
  if (!hasRecord && !dispatcherElsewhere) return null;

  const ordered = hasRecord ? runsNewestFirst(record!.runs) : [];
  const latest = ordered[0] ?? null;
  const earlier = ordered.slice(1);
  const listSummary = runMap.get(taskId);

  return (
    <div className="space-y-2">
      {latest ? (
        <LatestRunBlock
          taskId={taskId}
          task={task}
          run={latest}
          earlier={earlier}
          listSummary={listSummary}
          workflow={workflow}
          onViewSessions={() => setOpen(true)}
        />
      ) : (
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">Run</span>
          <span className="text-xs text-muted-foreground">
            No local Run record — it ran on another machine
          </span>
          <div className="flex-1" />
          <ViewSessionsButton onClick={() => setOpen(true)} />
        </div>
      )}
      <SessionLogModal
        taskId={taskId}
        record={hasRecord ? record : null}
        open={open}
        onOpenChange={setOpen}
      />
    </div>
  );
}

function ViewSessionsButton({ onClick }: { onClick: () => void }) {
  return (
    <Button
      variant="outline"
      size="sm"
      className="h-7 gap-1.5 text-xs"
      onClick={onClick}
    >
      <ScrollText className="size-3.5" />
      View sessions
    </Button>
  );
}

function ResultIcon({
  result,
  pending,
  queued,
}: {
  result?: AttemptResult | null;
  pending?: boolean;
  queued?: boolean;
}) {
  const c = "size-3.5 shrink-0";
  if (queued) return <Clock className={cn(c, "text-muted-foreground")} />;
  if (pending) return <Minus className={cn(c, "text-muted-foreground/50")} />;
  switch (result) {
    case "ok":
      return <CircleCheck className={cn(c, "text-emerald-500")} />;
    case "failed":
      return <CircleX className={cn(c, "text-destructive")} />;
    case "timeout":
      return <Clock className={cn(c, "text-destructive")} />;
    case "interrupted":
      return <CirclePause className={cn(c, "text-amber-500")} />;
    case "resume_mismatch":
      return <TriangleAlert className={cn(c, "text-amber-500")} />;
    case undefined:
    case null:
      return (
        <LoaderCircle className={cn(c, "animate-spin text-muted-foreground")} />
      );
    default:
      return <Minus className={cn(c, "text-muted-foreground/50")} />;
  }
}

function LatestRunBlock({
  taskId,
  task,
  run,
  earlier,
  listSummary,
  workflow,
  onViewSessions,
}: {
  taskId: string;
  task: Pick<Task, "status" | "assignee">;
  run: Run;
  earlier: Run[];
  listSummary?: RunSummary;
  workflow?: TaskWorkflow;
  onViewSessions: () => void;
}) {
  const stages = stageSummaries(run);
  const remnant = remnantStatusLine(run);
  const total = sumUsage(run.attempts);
  const badgeSummary = summaryFromRun(taskId, run, {
    status: listSummary?.status,
    queue_position: listSummary?.queue_position,
  });

  return (
    <div className="space-y-2">
      <div className="text-sm font-medium">Run</div>
      <div className="space-y-2 rounded-md border px-3 py-2.5">
      {/* Header */}
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <RunBadge summary={badgeSummary} task={task} workflow={workflow} />
        <span className="text-sm font-semibold">Run #{run.seq}</span>
        <span className="text-xs text-muted-foreground tabular-nums">
          {fmtTime(run.started_at)}
        </span>
        <span className="text-xs text-muted-foreground tabular-nums">
          {fmtDuration(run.started_at, run.ended_at)}
        </span>
        <div className="min-w-2 flex-1" />
        <ViewSessionsButton onClick={onViewSessions} />
      </div>

      {/* Stage progress */}
      <ol className="flex flex-wrap items-stretch gap-1">
        {stages.map((s, i) => (
          <li key={s.stage} className="flex min-w-0 items-stretch gap-1">
            {i > 0 && (
              <span
                aria-hidden
                className="flex items-center px-0.5 text-muted-foreground/40"
              >
                →
              </span>
            )}
            <StageStep summary={s} />
          </li>
        ))}
      </ol>

      {run.failure_reason ? (
        <p className="whitespace-pre-wrap text-xs text-destructive">
          {run.failure_reason}
        </p>
      ) : null}

      {remnant ? (
        <p className="text-xs text-muted-foreground">{remnant}</p>
      ) : null}

      <DetailsFold run={run} total={total} />

      {earlier.length > 0 ? (
        <EarlierRunsFold runs={earlier} count={earlier.length} />
      ) : null}
      </div>
    </div>
  );
}

function StageStep({ summary }: { summary: StageSummary }) {
  const {
    stage,
    result,
    model,
    durationLabel,
    inputTokens,
    outputTokens,
    unknownUsage,
    queued,
    pending,
    attempt,
  } = summary;
  const inFlight = !pending && !queued && result == null;

  return (
    <div
      className={cn(
        "min-w-[7.5rem] flex-1 rounded border px-2 py-1.5",
        pending && "opacity-50",
      )}
    >
      <div className="flex items-center gap-1.5">
        <ResultIcon result={result} pending={pending} queued={queued} />
        <span className="text-xs font-medium">{stage}</span>
      </div>
      {queued ? (
        <div className="mt-0.5 text-[11px] text-muted-foreground">queued</div>
      ) : pending ? (
        <div className="mt-0.5 text-[11px] text-muted-foreground">—</div>
      ) : (
        <>
          {model ? (
            <div
              className="mt-0.5 truncate font-mono text-[11px] text-muted-foreground"
              title={model}
            >
              {model}
            </div>
          ) : null}
          <div className="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-0.5 font-mono text-[11px] text-muted-foreground tabular-nums">
            {durationLabel ? <span>{durationLabel}</span> : null}
            {inFlight && !attempt?.usage ? (
              <span>…</span>
            ) : (
              <span
                title={usageTooltip(
                  attempt?.usage ?? null,
                  attempt?.duration_api_ms,
                )}
              >
                {unknownUsage ? "≥" : ""}
                {fmtTokens(inputTokens)} in / {fmtTokens(outputTokens)} out
              </span>
            )}
          </div>
        </>
      )}
    </div>
  );
}

function CopyPath({ path }: { path: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <button
      type="button"
      className="inline-flex max-w-full cursor-pointer items-center gap-1 rounded px-1 font-mono text-[11px] text-muted-foreground hover:bg-muted hover:text-foreground"
      title="Copy worktree path"
      onClick={() => {
        void navigator.clipboard?.writeText(path);
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1000);
      }}
    >
      <span className="truncate">{path}</span>
      {copied ? (
        <Check className="size-3 shrink-0 text-emerald-500" />
      ) : (
        <Copy className="size-3 shrink-0 opacity-60" />
      )}
    </button>
  );
}

function DetailsFold({
  run,
  total,
}: {
  run: Run;
  total: ReturnType<typeof sumUsage>;
}) {
  const wtRemoved = !!run.worktree_removed_at;
  const brRemoved = !!run.branch_removed_at;

  return (
    <details className="group text-xs">
      <summary className="flex cursor-pointer list-none items-center gap-1 text-muted-foreground hover:text-foreground [&::-webkit-details-marker]:hidden">
        <ChevronRight className="size-3.5 transition-transform group-open:rotate-90" />
        Details
      </summary>
      <dl className="mt-2 space-y-1.5 border-t pt-2">
        <DetailRow label="Worktree">
          <span
            className={cn(
              "inline-flex min-w-0 max-w-full flex-wrap items-center gap-1",
              wtRemoved && "text-muted-foreground",
            )}
          >
            <CopyPath path={run.worktree} />
            {wtRemoved ? (
              <span className="shrink-0 text-muted-foreground">(removed)</span>
            ) : null}
          </span>
        </DetailRow>
        <DetailRow label="Branch">
          <span
            className={cn(
              "font-mono text-[11px]",
              brRemoved && "text-muted-foreground",
            )}
          >
            {run.branch}
            {brRemoved ? " (removed)" : ""}
          </span>
        </DetailRow>
        <DetailRow label="Base">
          <span className="font-mono text-[11px]" title={run.base_sha}>
            {shortSha(run.base_sha)}
          </span>
        </DetailRow>
        <DetailRow label="Merge target">
          <span className="font-mono text-[11px]">{run.merge_target}</span>
        </DetailRow>
        <DetailRow label="Merge SHA">
          <span className="font-mono text-[11px]">
            {run.merge_sha ? shortSha(run.merge_sha) : "—"}
          </span>
        </DetailRow>
        <DetailRow label="Pushed">
          <span>{run.pushed ? "yes" : "no"}</span>
        </DetailRow>

        <div className="pt-1">
          <div className="mb-1 text-[11px] font-medium text-muted-foreground">
            Usage
          </div>
          <ul className="space-y-1">
            {run.attempts.map((a, i) => (
              <AttemptUsageRow key={`${a.stage}-${i}`} attempt={a} index={i} />
            ))}
          </ul>
          <div
            className="mt-1.5 font-mono text-[11px] tabular-nums"
            title={
              total.unknown
                ? "Some invocations have unknown usage"
                : undefined
            }
          >
            Run total: {total.unknown ? "≥" : ""}
            {fmtTokens(total.inputTokens)} in / {fmtTokens(total.outputTokens)}{" "}
            out
          </div>
        </div>
      </dl>
    </details>
  );
}

function DetailRow({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="grid grid-cols-[7rem_minmax(0,1fr)] items-start gap-2">
      <dt className="text-[11px] text-muted-foreground">{label}</dt>
      <dd className="min-w-0">{children}</dd>
    </div>
  );
}

function AttemptUsageRow({
  attempt,
  index,
}: {
  attempt: Attempt;
  index: number;
}) {
  const u = attempt.usage;
  return (
    <li
      className="flex flex-wrap items-baseline gap-x-2 gap-y-0.5 font-mono text-[11px] text-muted-foreground tabular-nums"
      title={usageTooltip(u, attempt.duration_api_ms)}
    >
      <span className="text-foreground/80">
        #{index + 1} {attempt.stage}
      </span>
      <span>{attempt.model}</span>
      <span>
        {u
          ? `${fmtTokens(u.inputTokens ?? 0)} in / ${fmtTokens(u.outputTokens ?? 0)} out`
          : attempt.result == null
            ? "…"
            : "—"}
      </span>
    </li>
  );
}

function EarlierRunsFold({ runs, count }: { runs: Run[]; count: number }) {
  return (
    <details className="group text-xs">
      <summary className="flex cursor-pointer list-none items-center gap-1 text-muted-foreground hover:text-foreground [&::-webkit-details-marker]:hidden">
        <ChevronRight className="size-3.5 transition-transform group-open:rotate-90" />
        Earlier runs ({count})
      </summary>
      <ul className="mt-2 space-y-1 border-t pt-2">
        {runs.map((r) => {
          const stage = endingStage(r);
          return (
            <li
              key={r.seq}
              className="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[11px]"
            >
              <span className="font-medium">#{r.seq}</span>
              <span
                className={cn(
                  r.status === "failed" && "text-destructive",
                  r.status === "succeeded" &&
                    "text-emerald-600 dark:text-emerald-400",
                  r.status === "stopped" && "text-muted-foreground",
                )}
              >
                {r.status}
              </span>
              {stage ? (
                <span className="text-muted-foreground">{stage}</span>
              ) : null}
              <span className="tabular-nums text-muted-foreground">
                {fmtTime(r.ended_at ?? r.started_at)}
              </span>
            </li>
          );
        })}
      </ul>
    </details>
  );
}
