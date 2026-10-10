"use client";

import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { apiPath } from "@/lib/project-prefix";
import { throwIfNotOk } from "@/lib/api-error";
import type { RunRecord, RunSummary } from "@/lib/types";

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
