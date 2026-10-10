"use client";

import { useMemo, useState } from "react";
import { ProjectLink } from "@/components/project-link";
import { useSearchParams } from "next/navigation";
import { ArrowLeft } from "lucide-react";
import { useMap } from "@/lib/hooks/use-maps";
import { useConfig } from "@/lib/hooks/use-config";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { WikiLinkChip } from "@/components/domain/wiki-link-chip";
import { PageTransition } from "@/components/domain/page-transition";
import { MAP_STATUS_CONFIG } from "@/lib/constants";
import {
  TYPE_COLOR,
  STATE_COLOR,
  STATE_LABEL,
  ROUTE_LAYOUT,
  ticketState,
  layoutRouteGraph,
} from "@/lib/map-ticket-state";
import type { MapTicket, MapStatus, TicketState, TaskWorkflow } from "@/lib/types";

const DEFAULT_WORKFLOW: TaskWorkflow = {
  draft: ["draft"],
  backlog: ["backlog"],
  planned: ["planned"],
  active: ["in-progress"],
  completed: ["done"],
  cancelled: ["cancelled"],
};

function TypeBadge({ type }: { type: string }) {
  const color = TYPE_COLOR[type] ?? TYPE_COLOR.task;
  return (
    <span
      className="text-[10px] uppercase tracking-wide px-1.5 py-0.5 rounded font-medium"
      style={{
        color,
        backgroundColor: `color-mix(in oklch, ${color} 15%, transparent)`,
      }}
    >
      {type}
    </span>
  );
}

function StateDot({ state }: { state: TicketState }) {
  return (
    <span
      className="inline-block size-2 rounded-full shrink-0"
      style={{ backgroundColor: STATE_COLOR[state] }}
    />
  );
}

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

export function MapDetailContent({ mapId }: { mapId: string }) {
  const searchParams = useSearchParams();
  // Deep-link from Task detail: /maps?map=…&node=TASK-…
  const highlightNode = searchParams.get("node");
  const { data: map, isLoading, error, refetch } = useMap(mapId);
  const { data: config } = useConfig();
  const workflow = config?.workflows.task ?? DEFAULT_WORKFLOW;
  const [focusOverride, setFocusOverride] = useState<string | null>(null);
  const [syncedNode, setSyncedNode] = useState(highlightNode);
  if (highlightNode !== syncedNode) {
    setSyncedNode(highlightNode);
    setFocusOverride(null);
  }
  const focus = focusOverride ?? highlightNode;

  const rows = useMemo(() => {
    if (!map) return [];
    return map.tickets.map((ticket) => ({
      ticket,
      state: ticketState(ticket, map.tickets, workflow),
    }));
  }, [map, workflow]);

  const layoutTickets = useMemo(
    () => rows.filter((r) => r.state !== "out-of-scope").map((r) => r.ticket),
    [rows]
  );

  const { pos, width, height } = useMemo(
    () => layoutRouteGraph(layoutTickets),
    [layoutTickets]
  );

  const focused = rows.find((r) => r.ticket.id === focus);
  const { NODE_W, NODE_H, COL_GAP } = ROUTE_LAYOUT;

  const titleOf = (id: string) =>
    map?.tickets.find((t) => t.id === id)?.title ?? id;

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center min-h-[400px] gap-4">
        <p className="text-muted-foreground">Failed to load map.</p>
        <p className="text-sm text-destructive">{error.message}</p>
        <div className="flex gap-2">
          <Button variant="outline" size="sm" asChild>
            <ProjectLink href="/maps">Back to Maps</ProjectLink>
          </Button>
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            Try again
          </Button>
        </div>
      </div>
    );
  }

  if (isLoading || !map) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-8 w-64" />
        <Skeleton className="h-[480px] w-full" />
      </div>
    );
  }

  const outOfScopeTickets = rows.filter((r) => r.state === "out-of-scope");

  return (
    <PageTransition>
      <div className="space-y-4">
        <div className="flex items-center gap-3 flex-wrap">
          <Button variant="ghost" size="sm" className="gap-1 -ml-2" asChild>
            <ProjectLink href="/maps">
              <ArrowLeft className="size-4" />
              Maps
            </ProjectLink>
          </Button>
          <span
            className="font-mono text-sm"
            style={{ color: "var(--entity-map)" }}
          >
            {map.id}
          </span>
          <MapStatusPill status={map.status} />
          <h1 className="text-lg font-semibold flex-1 min-w-0 truncate">
            {map.title}
          </h1>
          <div className="flex gap-3 text-xs text-muted-foreground">
            {(["decided", "claimed", "frontier", "blocked"] as TicketState[]).map(
              (s) => (
                <span key={s} className="flex items-center gap-1">
                  <StateDot state={s} />
                  {STATE_LABEL[s]}
                </span>
              )
            )}
          </div>
        </div>

        <div className="flex gap-4 items-start">
          <div className="flex-1 overflow-auto rounded-lg border bg-card p-4 min-w-0">
            <div className="flex gap-4 items-stretch min-w-max">
              <div className="w-28 shrink-0 rounded-md bg-muted/50 p-2 text-xs text-muted-foreground flex items-center justify-center">
                Start
              </div>
              <div
                className="relative"
                style={{ width, height: Math.max(height, NODE_H) }}
              >
                <svg
                  className="absolute inset-0 pointer-events-none"
                  width={width}
                  height={Math.max(height, NODE_H)}
                >
                  {layoutTickets.flatMap((t) =>
                    t.depends_on.map((d) => {
                      const a = pos.get(d);
                      const b = pos.get(t.id);
                      if (!a || !b) return null;
                      const x1 = a.x + NODE_W;
                      const y1 = a.y + NODE_H / 2;
                      const x2 = b.x;
                      const y2 = b.y + NODE_H / 2;
                      const hot = focus === t.id || focus === d;
                      return (
                        <path
                          key={`${d}-${t.id}`}
                          d={`M${x1},${y1} C${x1 + COL_GAP / 2},${y1} ${x2 - COL_GAP / 2},${y2} ${x2},${y2}`}
                          fill="none"
                          stroke={hot ? "var(--foreground)" : "var(--border)"}
                          strokeWidth={hot ? 2 : 1.5}
                        />
                      );
                    })
                  )}
                </svg>
                {rows
                  .filter((r) => pos.has(r.ticket.id))
                  .map(({ ticket, state }) => {
                    const p = pos.get(ticket.id)!;
                    return (
                      <TicketNode
                        key={ticket.id}
                        ticket={ticket}
                        state={state}
                        focused={focus === ticket.id}
                        x={p.x}
                        y={p.y}
                        onFocus={() => setFocusOverride(ticket.id)}
                      />
                    );
                  })}
              </div>
              <div
                className="w-48 shrink-0 rounded-md border-2 border-dashed p-2 text-xs flex flex-col gap-2"
                style={{ borderColor: "var(--status-now)" }}
              >
                <div className="font-medium">Destination</div>
                <div className="text-muted-foreground line-clamp-[10] whitespace-pre-wrap">
                  {map.destination || "—"}
                </div>
                {map.specs.length > 0 && (
                  <div className="flex flex-wrap gap-1">
                    {map.specs.map((id) => (
                      <WikiLinkChip key={id} id={id} />
                    ))}
                  </div>
                )}
                {map.not_yet_specified.length > 0 && (
                  <div className="mt-auto text-muted-foreground italic">
                    ☁ {map.not_yet_specified.length} patches of fog before here
                  </div>
                )}
              </div>
            </div>
          </div>

          <aside className="w-80 shrink-0 space-y-4">
            {focused ? (
              <div className="rounded-lg border bg-card p-3 space-y-2">
                <div className="flex items-center gap-2">
                  <StateDot state={focused.state} />
                  <span className="text-sm font-medium">
                    {STATE_LABEL[focused.state]}
                  </span>
                  <span className="font-mono text-xs text-muted-foreground">
                    {focused.ticket.id}
                  </span>
                </div>
                <div className="text-base font-medium">{focused.ticket.title}</div>
                <TypeBadge type={focused.ticket.type} />
                {focused.state === "claimed" && focused.ticket.assignee && (
                  <div className="text-xs text-muted-foreground">
                    @{focused.ticket.assignee}
                  </div>
                )}
                <div className="text-xs text-muted-foreground uppercase tracking-wide pt-2">
                  Question
                </div>
                <p className="text-sm whitespace-pre-wrap">
                  {focused.ticket.question || "—"}
                </p>
                {focused.ticket.answer && (
                  <>
                    <div className="text-xs text-muted-foreground uppercase tracking-wide pt-2">
                      Answer
                    </div>
                    <p className="text-sm whitespace-pre-wrap">
                      {focused.ticket.answer}
                    </p>
                  </>
                )}
                {focused.ticket.depends_on.length > 0 && (
                  <div className="text-xs text-muted-foreground pt-2">
                    Blocked by:{" "}
                    {focused.ticket.depends_on.map((d) => titleOf(d)).join(", ")}
                  </div>
                )}
              </div>
            ) : (
              <div className="rounded-lg border border-dashed p-3 text-sm text-muted-foreground">
                Click a ticket to see its question and answer.
              </div>
            )}
            <div className="rounded-lg border p-3">
              <div className="text-sm font-medium mb-1">Not yet specified</div>
              {map.not_yet_specified.length === 0 ? (
                <p className="text-xs text-muted-foreground">None</p>
              ) : (
                <ul className="list-disc ml-4 text-xs text-muted-foreground space-y-1">
                  {map.not_yet_specified.map((f) => (
                    <li key={f}>{f}</li>
                  ))}
                </ul>
              )}
            </div>
            <div className="rounded-lg border p-3 opacity-70">
              <div className="text-sm font-medium mb-1">Out of scope</div>
              {map.out_of_scope.length === 0 && outOfScopeTickets.length === 0 ? (
                <p className="text-xs text-muted-foreground">None</p>
              ) : (
                <ul className="list-disc ml-4 text-xs text-muted-foreground space-y-1">
                  {map.out_of_scope.map((f) => (
                    <li key={f}>{f}</li>
                  ))}
                  {outOfScopeTickets.map(({ ticket }) => (
                    <li key={ticket.id}>
                      {ticket.title}{" "}
                      <span className="font-mono">{ticket.id}</span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          </aside>
        </div>
      </div>
    </PageTransition>
  );
}

function TicketNode({
  ticket,
  state,
  focused,
  x,
  y,
  onFocus,
}: {
  ticket: MapTicket;
  state: TicketState;
  focused: boolean;
  x: number;
  y: number;
  onFocus: () => void;
}) {
  const { NODE_W, NODE_H } = ROUTE_LAYOUT;
  return (
    <button
      type="button"
      onClick={onFocus}
      className={`absolute rounded-md border-2 bg-background px-2 py-1.5 text-left cursor-pointer ${
        focused ? "ring-2 ring-ring" : ""
      }`}
      style={{
        left: x,
        top: y,
        width: NODE_W,
        height: NODE_H,
        borderColor: STATE_COLOR[state],
        opacity: state === "decided" ? 0.65 : 1,
        borderStyle: state === "blocked" ? "dashed" : "solid",
      }}
    >
      <div className="flex items-center gap-1.5">
        <TypeBadge type={ticket.type} />
        {state === "claimed" && ticket.assignee && (
          <span className="text-[10px] text-muted-foreground">
            @{ticket.assignee}
          </span>
        )}
      </div>
      <div className="text-xs leading-tight mt-1 line-clamp-2">{ticket.title}</div>
    </button>
  );
}
