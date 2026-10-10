"use client";

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { apiPath } from "@/lib/project-prefix";
import { throwIfNotOk } from "@/lib/api-error";
import type { SessionTranscript } from "@/lib/types";

const API_BASE = typeof window !== "undefined" ? "" : "http://localhost:4200";

export async function fetchSessionTranscript(
  taskId: string,
  sessionId: string,
): Promise<SessionTranscript> {
  const res = await fetch(
    `${API_BASE}${apiPath(`/api/runs/${taskId}/sessions/${sessionId}`)}`,
  );
  await throwIfNotOk(res);
  const json = await res.json();
  return json.data as SessionTranscript;
}

export function sessionTranscriptQueryKey(taskId: string, sessionId: string) {
  return ["runs", "session", taskId, sessionId] as const;
}

/**
 * Load a session transcript. Does not auto-refresh on WebSocket events —
 * callers refetch via Refresh.
 */
export function useSessionTranscript(
  taskId: string,
  sessionId: string | null,
  options?: { enabled?: boolean },
) {
  return useQuery<SessionTranscript>({
    queryKey: sessionTranscriptQueryKey(taskId, sessionId ?? ""),
    queryFn: () => fetchSessionTranscript(taskId, sessionId!),
    enabled: !!taskId && !!sessionId && (options?.enabled ?? true),
    staleTime: Infinity,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  });
}

export function useRefreshSessionLog(taskId: string) {
  const queryClient = useQueryClient();
  return async (sessionId?: string | null) => {
    await queryClient.refetchQueries({ queryKey: ["runs", taskId] });
    if (sessionId) {
      await queryClient.refetchQueries({
        queryKey: sessionTranscriptQueryKey(taskId, sessionId),
      });
    }
  };
}
