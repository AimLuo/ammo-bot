"use client";

import {
  AlertTriangle,
  CirclePause,
  Clock,
  LoaderCircle,
} from "lucide-react";
import { useNow } from "@/lib/hooks/use-now";
import {
  deriveDisplayStatus,
  runBadgeLabel,
  runBadgeTitle,
  shouldShowRunBadge,
} from "@/lib/run-display";
import { categoryOf } from "@/lib/constants";
import { cn } from "@/lib/utils";
import type { RunDisplayStatus, RunSummary, Task, TaskWorkflow } from "@/lib/types";

function BadgeIcon({ status }: { status: RunDisplayStatus }) {
  const c = "size-3 shrink-0";
  switch (status) {
    case "running":
      return (
        <LoaderCircle
          className={cn(c, "animate-spin motion-reduce:animate-none")}
          aria-hidden
        />
      );
    case "merge-queued":
      return <Clock className={c} aria-hidden />;
    case "interrupted":
      return <CirclePause className={c} aria-hidden />;
    case "failed":
      return <AlertTriangle className={c} aria-hidden />;
    default:
      return null;
  }
}

function toneClass(status: RunDisplayStatus): string {
  switch (status) {
    case "running":
    case "merge-queued":
      return "text-[var(--status-in-progress)]";
    case "interrupted":
      return "text-amber-600 dark:text-amber-400";
    case "failed":
      return "text-destructive";
    case "stopping":
    case "stopped":
    case "running elsewhere":
      return "text-muted-foreground";
    default:
      return "text-muted-foreground";
  }
}

/** Whether the task card should get a running accent border. */
export function runCardHighlight(
  summary: RunSummary | undefined,
  task: Pick<Task, "status" | "assignee">,
  workflow: TaskWorkflow | undefined,
): boolean {
  if (!summary) return false;
  const display = deriveDisplayStatus(summary, task, workflow);
  return display === "running" || display === "merge-queued";
}

export function RunBadge({
  summary,
  task,
  workflow,
  className,
}: {
  summary: RunSummary;
  task: Pick<Task, "status" | "assignee">;
  workflow?: TaskWorkflow;
  className?: string;
}) {
  const now = useNow();
  const display = deriveDisplayStatus(summary, task, workflow);
  const ticketCategory = workflow
    ? categoryOf(workflow, task.status)
    : undefined;
  if (!shouldShowRunBadge(display, ticketCategory)) return null;

  const label = runBadgeLabel(display, summary, now);
  const title = runBadgeTitle(summary, now);

  return (
    <span
      data-slot="run-badge"
      className={cn(
        "inline-flex items-center gap-1 text-xs font-medium tabular-nums",
        toneClass(display),
        className,
      )}
      title={title}
    >
      <BadgeIcon status={display} />
      {label}
    </span>
  );
}
