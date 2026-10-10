"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { deleteAction, fetcher, fetchList, patchJson, postJson } from "@/lib/api";

export type RegistryProject = {
  id: string;
  name: string;
  path: string;
  path_valid: boolean;
  last_opened: string;
};

export type FsEntry = {
  name: string;
  path: string;
  has_markplane: boolean;
};

export type FsListing = {
  path: string;
  parent: string | null;
  home: string;
  has_markplane: boolean;
  entries: FsEntry[];
};

export function useProjects() {
  return useQuery({
    queryKey: ["projects"],
    queryFn: () => fetchList<RegistryProject>("/api/projects"),
  });
}

export function useFsListing(path: string | null, enabled: boolean) {
  return useQuery({
    queryKey: ["fs", path ?? ""],
    queryFn: () => {
      const q =
        path != null && path !== ""
          ? `?path=${encodeURIComponent(path)}`
          : "";
      return fetcher<FsListing>(`/api/fs${q}`);
    },
    enabled,
  });
}

export function useAddProject() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (path: string) =>
      postJson<RegistryProject>("/api/projects", { path }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["projects"] });
    },
  });
}

export function useInitProject() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (path: string) =>
      postJson<RegistryProject>("/api/init", { path }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["projects"] });
    },
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
