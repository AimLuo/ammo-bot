import { categoryOf } from "@/lib/constants";
import type { Comment, Priority, Task, TaskWorkflow } from "@/lib/types";

const PRIORITY_RANK: Record<Priority, number> = {
  critical: 0,
  high: 1,
  medium: 2,
  low: 3,
  someday: 4,
};

/** Unarchived Tasks tagged ready-for-human whose status is not completed/cancelled. */
export function isNeedsYouTask(task: Task, workflow: TaskWorkflow): boolean {
  if (!task.tags.includes("ready-for-human")) return false;
  const category = categoryOf(workflow, task.status);
  if (category === "completed" || category === "cancelled") return false;
  return true;
}

export function filterNeedsYouBase(
  tasks: Task[],
  workflow: TaskWorkflow
): Task[] {
  return tasks.filter((t) => isNeedsYouTask(t, workflow));
}

/** Fixed sort: priority (critical→someday) → updated desc → ID. */
export function sortNeedsYouTasks(tasks: Task[]): Task[] {
  return [...tasks].sort((a, b) => {
    const pr = PRIORITY_RANK[a.priority] - PRIORITY_RANK[b.priority];
    if (pr !== 0) return pr;
    const updated = b.updated.localeCompare(a.updated);
    if (updated !== 0) return updated;
    return a.id.localeCompare(b.id);
  });
}

/** Latest comment preview: `author · date: first line` (or null if none). */
export function latestCommentPreview(comments: Comment[]): string | null {
  if (comments.length === 0) return null;
  const latest = comments[comments.length - 1];
  const firstLine = latest.body.split("\n")[0]?.trim() ?? "";
  return `${latest.author} · ${latest.date}: ${firstLine}`;
}
