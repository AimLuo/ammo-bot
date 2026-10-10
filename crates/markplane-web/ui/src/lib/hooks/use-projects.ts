"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { deleteAction, fetchList, patchJson } from "@/lib/api";

export type RegistryProject = {
  id: string;
  name: string;
  path: string;
  path_valid: boolean;
  last_opened: string;
};

export function useProjects() {
  return useQuery({
    queryKey: ["projects"],
    queryFn: () => fetchList<RegistryProject>("/api/projects"),
  });
}

export function useRemoveProject() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => deleteAction<RegistryProject>(`/api/projects/${encodeURIComponent(id)}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["projects"] });
    },
  });
}

export function useRenameProject() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, name }: { id: string; name: string }) =>
      patchJson<RegistryProject>(`/api/projects/${encodeURIComponent(id)}`, { name }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["projects"] });
    },
  });
}
