"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { fetcher, patchJson } from "@/lib/api";
import { ApiError } from "@/lib/api-error";
import { useProjectId } from "@/lib/project-prefix";
import type { DispatcherStatus, ScanSwitch } from "@/lib/types";

export function useDispatcher() {
  return useQuery({
    queryKey: ["dispatcher"],
    queryFn: () => fetcher<DispatcherStatus>("/api/dispatcher"),
  });
}

export function useScanSwitch() {
  const projectId = useProjectId();
  const dispatcher = useDispatcher();
  const project = dispatcher.data?.projects.find((p) => p.id === projectId);
  return {
    ...dispatcher,
    enabled: project?.scan_enabled ?? false,
    lock: project?.lock,
  };
}

export function useSetScanSwitch() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (enabled: boolean) =>
      patchJson<ScanSwitch>("/api/dispatcher/scan", { enabled }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["dispatcher"] });
    },
    onError: () => {
      queryClient.invalidateQueries({ queryKey: ["dispatcher"] });
    },
  });
}

export function scanSwitchErrorMessage(err: unknown): string {
  if (err instanceof ApiError) {
    return err.message;
  }
  if (err instanceof Error) {
    return err.message;
  }
  return "Failed to update Scan switch";
}
