"use client";

import { useMemo } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { apiPath } from "@/lib/project-prefix";
import { throwIfNotOk } from "@/lib/api-error";
import { fetcher, postJson } from "@/lib/api";
import { toast } from "sonner";
import type {
  CleanupOneResponse,
  CleanupPreview,
  ResumeAllResponse,
  ResumeOneResponse,
  RunRecord,
  RunSummary,
} from "@/lib/types";

const API_BASE = typeof window !== "undefined" ? "" : "http://localhost:4200";

/**
 * Fetch the local Run record for a task.
 * The API returns the full record object, or an empty array when absent.
 */
export async function fetchRunRecord(taskId: string): Promise<RunRecord | null> {
  const res = await fetch(`${API_BASE}${apiPath(`/api/runs/${taskId}`)}`);
  await throwIfNotOk(res);
  const json = await res.json();
  const data = json.data;
  if (Array.isArray(data)) {
    return null;
  }
  if (data && typeof data === "object" && Array.isArray(data.runs)) {
    return data as RunRecord;
  }
  return null;
}

export function useRunRecord(taskId: string, options?: { enabled?: boolean }) {
  return useQuery<RunRecord | null>({
    queryKey: ["runs", taskId],
    queryFn: () => fetchRunRecord(taskId),
    enabled: !!taskId && (options?.enabled ?? true),
  });
}

/** Fetch latest-Run summaries for every ticket with a local record. */
export async function fetchRunSummaries(): Promise<RunSummary[]> {
  const res = await fetch(`${API_BASE}${apiPath("/api/runs")}`);
  await throwIfNotOk(res);
  const json = await res.json();
  const data = json.data;
  if (!Array.isArray(data)) return [];
  return data as RunSummary[];
}

export function useRunSummaries(options?: { enabled?: boolean }) {
  return useQuery<RunSummary[]>({
    queryKey: ["runs"],
    queryFn: fetchRunSummaries,
    enabled: options?.enabled ?? true,
  });
}

/** Map of task_id → latest Run summary. */
export function useRunSummaryMap() {
  const query = useRunSummaries();
  const map = useMemo(() => {
    const m = new Map<string, RunSummary>();
    for (const row of query.data ?? []) {
      m.set(row.task_id, row);
    }
    return m;
  }, [query.data]);
  return { ...query, map };
}

/** Interrupted Runs in the current Project (for "Resume N interrupted"). */
export function useInterruptedCount() {
  const { data } = useRunSummaries();
  return useMemo(
    () => (data ?? []).filter((r) => r.status === "interrupted").length,
    [data]
  );
}

export function useResumeRun(taskId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (terminateOrphan?: boolean) =>
      postJson<ResumeOneResponse>(`/api/runs/${taskId}/resume`, {
        terminate_orphan: !!terminateOrphan,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["runs"] });
      void qc.invalidateQueries({ queryKey: ["runs", taskId] });
      void qc.invalidateQueries({ queryKey: ["dispatcher"] });
      toast.success(`Resumed ${taskId}`);
    },
    onError: (err: Error) => {
      toast.error(err.message || "Resume failed");
    },
  });
}

export function useResumeAllInterrupted() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () =>
      postJson<ResumeAllResponse>("/api/runs/resume", {}),
    onSuccess: (data) => {
      void qc.invalidateQueries({ queryKey: ["runs"] });
      void qc.invalidateQueries({ queryKey: ["dispatcher"] });
      const n = data.resumed.length;
      const left = data.remaining.length;
      if (data.stopped_early) {
        toast.message(
          `Resumed ${n}; ${left} still interrupted (no free slot)`
        );
      } else {
        toast.success(`Resumed ${n} interrupted run${n === 1 ? "" : "s"}`);
      }
    },
    onError: (err: Error) => {
      toast.error(err.message || "Resume failed");
    },
  });
}

export type ContinueOneResponse = {
  task_id: string;
  status: string;
};

/** Comment & continue a failed or interrupted Run. */
export function useContinueRun(taskId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (args: { text: string; terminateOrphan?: boolean }) =>
      postJson<ContinueOneResponse>(`/api/runs/${taskId}/continue`, {
        text: args.text,
        terminate_orphan: !!args.terminateOrphan,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["runs"] });
      void qc.invalidateQueries({ queryKey: ["runs", taskId] });
      void qc.invalidateQueries({ queryKey: ["dispatcher"] });
      void qc.invalidateQueries({ queryKey: ["tasks"] });
      void qc.invalidateQueries({ queryKey: ["task", taskId] });
      toast.success(`Continued ${taskId}`);
    },
  });
}

/** Fetch Clean up confirmation preview. */
export async function fetchCleanupPreview(
  taskId: string,
): Promise<CleanupPreview> {
  return fetcher<CleanupPreview>(`/api/runs/${taskId}/cleanup`);
}

/** Force-clean recorded worktree / agent branch for a Run. */
export function useCleanupRun(taskId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (terminateOrphan?: boolean) =>
      postJson<CleanupOneResponse>(`/api/runs/${taskId}/cleanup`, {
        terminate_orphan: !!terminateOrphan,
      }),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ["runs"] });
      void qc.invalidateQueries({ queryKey: ["runs", taskId] });
      void qc.invalidateQueries({ queryKey: ["dispatcher"] });
      toast.success(`Cleaned up ${taskId}`);
    },
    onError: (err: Error) => {
      toast.error(err.message || "Clean up failed");
    },
  });
}
