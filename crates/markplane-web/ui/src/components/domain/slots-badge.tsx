"use client";

import { useMemo } from "react";
import { Badge } from "@/components/ui/badge";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { useDispatcher } from "@/lib/hooks/use-dispatcher";
import { useProjectId, useProjectRouter } from "@/lib/project-prefix";
import { cn } from "@/lib/utils";
import type { DispatcherActiveRun, DispatcherProjectStatus } from "@/lib/types";

function runStatusLabel(run: DispatcherActiveRun): string {
  if (run.status === "merge-queued") {
    const n = run.queue_position;
    return n != null ? `merge queue #${n}` : "merge queue";
  }
  if (run.status === "running" && run.stage) {
    return run.stage;
  }
  return run.status;
}

export function SlotsBadge() {
  const { data, isLoading } = useDispatcher();
  const projectId = useProjectId();
  const router = useProjectRouter();

  const used = data?.slots_used ?? 0;
  const limit = data?.concurrency ?? 20;
  const full = used >= limit && limit > 0;

  const grouped = useMemo(() => {
    if (!data) return [] as DispatcherProjectStatus[];
    const projects = [...data.projects];
    projects.sort((a, b) => {
      if (a.id === projectId) return -1;
      if (b.id === projectId) return 1;
      return a.name.localeCompare(b.name);
    });
    return projects.filter((p) => (p.active_runs ?? []).length > 0);
  }, [data, projectId]);

  const hasAny = (data?.slots_used ?? 0) > 0;

  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          className="inline-flex focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded-full"
          aria-label={`Slots ${used} of ${limit}`}
        >
          <Badge
            variant={full ? "destructive" : "outline"}
            className={cn(
              "cursor-pointer font-normal tabular-nums",
              !full && "text-muted-foreground",
              isLoading && "opacity-60"
            )}
          >
            Slots {used}/{limit}
          </Badge>
        </button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80 p-3">
        <p className="text-muted-foreground mb-3 text-xs tabular-nums">
          running {data?.running ?? 0} · merge queue {data?.merge_queued ?? 0} ·
          limit {limit}
        </p>
        {!hasAny ? (
          <p className="text-muted-foreground text-sm">No active runs</p>
        ) : (
          <div className="space-y-3">
            {grouped.map((project) => (
              <div key={project.id}>
                <div className="mb-1 flex items-baseline justify-between gap-2">
                  <span className="truncate text-sm font-medium">
                    {project.name}
                  </span>
                  <span className="text-muted-foreground shrink-0 text-[10px] uppercase tracking-wide">
                    Scan {project.scan_enabled ? "on" : "off"}
                  </span>
                </div>
                <ul className="space-y-0.5">
                  {(project.active_runs ?? []).map((run) => (
                    <li key={run.task_id}>
                      <button
                        type="button"
                        className="hover:bg-accent flex w-full items-center gap-2 rounded-sm px-1.5 py-1 text-left text-sm"
                        onClick={() => {
                          router.push(
                            `/p/${encodeURIComponent(project.id)}/backlog?task=${encodeURIComponent(run.task_id)}`
                          );
                        }}
                      >
                        <span className="text-muted-foreground shrink-0 font-mono text-xs">
                          {run.task_id}
                        </span>
                        <span className="min-w-0 flex-1 truncate">
                          {run.title}
                        </span>
                        <span className="text-muted-foreground shrink-0 text-xs">
                          {runStatusLabel(run)}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>
        )}
      </PopoverContent>
    </Popover>
  );
}
