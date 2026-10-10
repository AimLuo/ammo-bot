"use client";

import { ProjectLink } from "@/components/project-link";
import { useSearchParams } from "next/navigation";
import { useMaps } from "@/lib/hooks/use-maps";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { MAP_STATUS_CONFIG } from "@/lib/constants";
import { PageTransition } from "@/components/domain/page-transition";
import { EmptyState } from "@/components/domain/empty-state";
import { MapDetailContent } from "./map-detail-content";
import type { MapListItem, MapStatus } from "@/lib/types";

function MapStatusPill({ status }: { status: MapStatus }) {
  const config = MAP_STATUS_CONFIG[status];
  const color = config?.color ?? "var(--muted-foreground)";
  return (
    <span
      className="text-xs px-2 py-0.5 rounded"
      style={{
        color,
        backgroundColor: `color-mix(in oklch, ${color} 15%, transparent)`,
      }}
    >
      {config?.label ?? status}
    </span>
  );
}

function MapCard({ map }: { map: MapListItem }) {
  const total = Math.max(1, map.total_count);
  const pct = (map.decided_count / total) * 100;

  return (
    <ProjectLink
      href={`/maps?map=${map.id}`}
      className="block rounded-lg border bg-card p-4 hover:border-ring transition-colors no-underline text-inherit"
    >
      <div className="flex items-center justify-between mb-1">
        <span
          className="font-mono text-sm"
          style={{ color: "var(--entity-map)" }}
        >
          {map.id}
        </span>
        <MapStatusPill status={map.status} />
      </div>
      <h3 className="text-base font-medium leading-snug">{map.title}</h3>
      <div className="mt-3 h-1.5 rounded bg-muted overflow-hidden">
        <div
          className="h-full transition-[width]"
          style={{
            width: `${pct}%`,
            backgroundColor: "var(--status-done)",
          }}
        />
      </div>
      <div className="text-xs text-muted-foreground mt-1.5 flex items-center justify-between">
        <span>
          {map.decided_count}/{map.total_count} decided · {map.frontier_count}{" "}
          on frontier
        </span>
        <span>{map.updated}</span>
      </div>
    </ProjectLink>
  );
}

export function MapsContent() {
  const searchParams = useSearchParams();
  const selectedMapId = searchParams.get("map");

  if (selectedMapId) {
    return <MapDetailContent mapId={selectedMapId} />;
  }

  return <MapsList />;
}

function MapsList() {
  const { data, isLoading, error, refetch } = useMaps();
  const maps = data ?? [];

  const grouped: Record<MapStatus, MapListItem[]> = {
    active: maps.filter((m) => m.status === "active"),
    done: maps.filter((m) => m.status === "done"),
    abandoned: maps.filter((m) => m.status === "abandoned"),
  };

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center min-h-[400px] gap-4">
        <p className="text-muted-foreground">Failed to load maps.</p>
        <p className="text-sm text-destructive">{error.message}</p>
        <Button variant="outline" size="sm" onClick={() => refetch()}>
          Try again
        </Button>
      </div>
    );
  }

  return (
    <PageTransition>
      <div className="space-y-6">
        <h1 className="text-lg font-semibold">Maps</h1>

        {isLoading ? (
          <MapsSkeleton />
        ) : maps.length === 0 ? (
          <EmptyState
            title="No maps yet"
            description="Create a wayfinder map with markplane map &quot;title&quot;"
          />
        ) : (
          (["active", "done", "abandoned"] as MapStatus[]).map((status) => {
            const items = grouped[status];
            if (items.length === 0) return null;
            return (
              <div key={status} className="space-y-3">
                <h2 className="text-sm font-medium text-muted-foreground uppercase tracking-wide">
                  {MAP_STATUS_CONFIG[status]?.label ?? status} ({items.length})
                </h2>
                <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
                  {items.map((map) => (
                    <MapCard key={map.id} map={map} />
                  ))}
                </div>
              </div>
            );
          })
        )}
      </div>
    </PageTransition>
  );
}

export function MapsSkeleton() {
  return (
    <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
      {Array.from({ length: 3 }).map((_, i) => (
        <Skeleton key={i} className="h-28 w-full" />
      ))}
    </div>
  );
}
