"use client";

import { useQuery } from "@tanstack/react-query";
import { apiPath } from "@/lib/project-prefix";
import { throwIfNotOk } from "@/lib/api-error";
import type { RunRecord } from "@/lib/types";

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
    queryKey: ["runs", "detail", taskId],
    queryFn: () => fetchRunRecord(taskId),
    enabled: !!taskId && (options?.enabled ?? true),
  });
}
