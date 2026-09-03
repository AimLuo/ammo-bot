import type { Project } from "./types.ts";

export type QueueBead = {
  id: string;
  issue_type: string;
  labels: string[];
  status: string;
};

export type ScannerQueue = {
  listReady(): Promise<QueueBead[]>;
  tryClaim(beadId: string): Promise<boolean>;
};

export type ScannerProject = {
  project: Project;
  queue: ScannerQueue;
};

export const MAX_IN_FLIGHT = 20;

function isWayfinderLabel(label: string): boolean {
  return label === "wayfinder" || label.startsWith("wayfinder:");
}

function inUnattendedQueue(bead: QueueBead): boolean {
  if (bead.issue_type === "epic") return false;
  if (bead.status === "in_progress") return false;
  if (bead.labels.some(isWayfinderLabel)) return false;
  if (bead.labels.includes("human")) return false;
  return true;
}

function flightKey(project: Project, beadId: string): string {
  return `${project.path}::${beadId}`;
}

export function createScanner(input: {
  runPipeline: (input: { project: Project; beadId: string }) => Promise<unknown>;
  maxInFlight?: number;
}) {
  const maxInFlight = input.maxInFlight ?? MAX_IN_FLIGHT;
  const inFlight = new Map<string, Promise<void>>();

  function track(key: string, work: Promise<unknown>) {
    const done = work.then(
      () => undefined,
      () => undefined,
    );
    inFlight.set(key, done);
    void done.finally(() => {
      inFlight.delete(key);
    });
  }

  return {
    inFlightCount() {
      return inFlight.size;
    },
    async whenIdle() {
      while (inFlight.size > 0) {
        await Promise.all([...inFlight.values()]);
      }
    },
    async tick(armed: ScannerProject[]) {
      for (const { project, queue } of armed) {
        if (inFlight.size >= maxInFlight) return;
        const ready = await queue.listReady();
        for (const item of ready) {
          if (inFlight.size >= maxInFlight) return;
          if (!inUnattendedQueue(item)) continue;
          const key = flightKey(project, item.id);
          if (inFlight.has(key)) continue;
          const won = await queue.tryClaim(item.id);
          if (!won) continue;
          track(key, input.runPipeline({ project, beadId: item.id }));
        }
      }
    },
  };
}
