import type { MapTicket, StatusCategory, TaskWorkflow, TicketState } from "./types";
import { categoryOf } from "./constants";

export const TYPE_COLOR: Record<string, string> = {
  research: "oklch(0.65 0.15 240)",
  prototype: "oklch(0.62 0.19 300)",
  grilling: "oklch(0.72 0.16 70)",
  task: "oklch(0.60 0.02 260)",
};

export const STATE_COLOR: Record<TicketState, string> = {
  decided: "var(--status-done)",
  claimed: "var(--status-in-progress)",
  frontier: "var(--status-now)",
  blocked: "var(--status-blocked)",
  "out-of-scope": "var(--status-cancelled)",
};

export const STATE_LABEL: Record<TicketState, string> = {
  decided: "Decided",
  claimed: "Claimed",
  frontier: "Frontier",
  blocked: "Blocked",
  "out-of-scope": "Out of scope",
};

function isClosed(category: StatusCategory | undefined): boolean {
  return category === "completed" || category === "cancelled";
}

/** Ticket state from StatusCategory (matches Variant C prototype rules). */
export function ticketState(
  ticket: MapTicket,
  all: MapTicket[],
  workflow: TaskWorkflow
): TicketState {
  const cat = categoryOf(workflow, ticket.status);
  if (cat === "cancelled") return "out-of-scope";
  if (cat === "completed") return "decided";
  if (ticket.assignee) return "claimed";
  const blocked = ticket.depends_on.some((depId) => {
    const dep = all.find((t) => t.id === depId);
    if (!dep) return true;
    return !isClosed(categoryOf(workflow, dep.status));
  });
  return blocked ? "blocked" : "frontier";
}

const NODE_W = 210;
const NODE_H = 62;
const COL_GAP = 70;
const ROW_GAP = 18;

export const ROUTE_LAYOUT = { NODE_W, NODE_H, COL_GAP, ROW_GAP };

/** Left-to-right columns by dependency depth; cancelled tickets excluded. */
export function layoutRouteGraph(tickets: MapTicket[]) {
  const visible = tickets.filter((t) => t.status !== "cancelled");
  const depth = new Map<string, number>();

  const visit = (t: MapTicket): number => {
    if (depth.has(t.id)) return depth.get(t.id)!;
    const deps = t.depends_on
      .map((d) => visible.find((x) => x.id === d))
      .filter(Boolean) as MapTicket[];
    const d = deps.length ? 1 + Math.max(...deps.map(visit)) : 0;
    depth.set(t.id, d);
    return d;
  };

  visible.forEach(visit);

  const cols = new Map<number, MapTicket[]>();
  visible.forEach((t) => {
    const d = depth.get(t.id)!;
    cols.set(d, [...(cols.get(d) ?? []), t]);
  });

  const pos = new Map<string, { x: number; y: number }>();
  cols.forEach((ts, d) =>
    ts.forEach((t, i) =>
      pos.set(t.id, { x: d * (NODE_W + COL_GAP), y: i * (NODE_H + ROW_GAP) })
    )
  );

  const colKeys = [...cols.keys()];
  const width =
    colKeys.length > 0
      ? Math.max(...colKeys.map((d) => (d + 1) * (NODE_W + COL_GAP)))
      : NODE_W;
  const height =
    cols.size > 0
      ? Math.max(...[...cols.values()].map((ts) => ts.length * (NODE_H + ROW_GAP)))
      : NODE_H;

  return { pos, width, height };
}
