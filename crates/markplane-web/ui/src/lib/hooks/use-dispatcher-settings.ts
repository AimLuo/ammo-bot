"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { fetcher, patchJson } from "@/lib/api";
import type { AgentModel, DispatcherConcurrency } from "@/lib/types";

export function useDispatcherConcurrency() {
  return useQuery({
    queryKey: ["dispatcher-concurrency"],
    queryFn: () => fetcher<DispatcherConcurrency>("/api/dispatcher/concurrency"),
  });
}

export function useUpdateDispatcherConcurrency() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (concurrency: number) =>
      patchJson<DispatcherConcurrency>("/api/dispatcher/concurrency", {
        concurrency,
      }),
    onMutate: async (concurrency) => {
      await queryClient.cancelQueries({ queryKey: ["dispatcher-concurrency"] });
      const previous = queryClient.getQueryData<DispatcherConcurrency>([
        "dispatcher-concurrency",
      ]);
      if (previous) {
        queryClient.setQueryData<DispatcherConcurrency>(
          ["dispatcher-concurrency"],
          { ...previous, concurrency },
        );
      }
      return { previous };
    },
    onSuccess: () => {
      toast.success("Settings updated");
    },
    onError: (err, _vars, context) => {
      toast.error("Failed to update settings", { description: err.message });
      if (context?.previous) {
        queryClient.setQueryData(["dispatcher-concurrency"], context.previous);
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: ["dispatcher-concurrency"] });
    },
  });
}

export function useAgentModels() {
  return useQuery({
    queryKey: ["dispatcher-models"],
    queryFn: async () => {
      const data = await fetcher<{ models: AgentModel[] }>("/api/dispatcher/models");
      return data.models;
    },
    staleTime: 10 * 60 * 1000,
  });
}
