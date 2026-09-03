"use client";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/api-client";
import { toastError } from "@/components/error-toast";

export function useProjects() {
  return useQuery({ queryKey: ["projects"], queryFn: api.projects.list });
}

export function useSetArmed() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, armed }: { id: string; armed: boolean }) =>
      api.projects.setArmed(id, armed),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["projects"] });
    },
    onError: (err) => toastError(err),
  });
}
