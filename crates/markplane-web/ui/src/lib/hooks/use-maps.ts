"use client";

import { useQuery } from "@tanstack/react-query";
import { fetcher, fetchList } from "@/lib/api";
import type { MapDetail, MapListItem } from "@/lib/types";

export function useMaps() {
  return useQuery({
    queryKey: ["maps", "list"],
    queryFn: () => fetchList<MapListItem>("/api/maps"),
    select: (result) => result.data,
  });
}

export function useMap(id: string, options?: { enabled?: boolean }) {
  return useQuery<MapDetail>({
    queryKey: ["maps", "detail", id],
    queryFn: () => fetcher<MapDetail>(`/api/maps/${id}`),
    enabled: !!id && (options?.enabled ?? true),
  });
}

export function useArchivedMaps() {
  return useQuery({
    queryKey: ["archived", "maps"],
    queryFn: () => fetchList<MapListItem>("/api/maps?archived=true"),
    select: (result) => result.data,
  });
}
