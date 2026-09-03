import { describe, expect, it } from "vitest";
import { createScanner } from "./scanner.ts";
import type { Project } from "./types.ts";
import type { QueueBead, ScannerQueue } from "./scanner.ts";

const PROJECT: Project = { path: "/projects/demo", targetBranch: "main" };

function bead(partial: Partial<QueueBead> & { id: string }): QueueBead {
  return {
    issue_type: "task",
    labels: [],
    status: "open",
    ...partial,
  };
}

function createQueue(ready: QueueBead[]) {
  const claimed: string[] = [];
  const lost = new Set<string>();
  const queue: ScannerQueue = {
    async listReady() {
      return ready.filter((b) => !claimed.includes(b.id));
    },
    async tryClaim(beadId) {
      if (lost.has(beadId)) return false;
      if (claimed.includes(beadId)) return false;
      claimed.push(beadId);
      return true;
    },
  };
  return { queue, claimed, lost };
}

describe("scanner", () => {
  it("does not claim when no project is Armed", async () => {
    const { queue, claimed } = createQueue([bead({ id: "ready-1" })]);
    const spawned: string[] = [];
    const scanner = createScanner({
      runPipeline: async ({ beadId }) => {
        spawned.push(beadId);
      },
    });

    await scanner.tick([]);

    expect(claimed).toEqual([]);
    expect(spawned).toEqual([]);
    expect(scanner.inFlightCount()).toBe(0);
  });

  it("when Armed, claims only the unattended queue and spawns a pipeline", async () => {
    const { queue, claimed } = createQueue([
      bead({ id: "epic-1", issue_type: "epic" }),
      bead({ id: "wayfinder-1", labels: ["wayfinder:research"] }),
      bead({ id: "in-flight", status: "in_progress" }),
      bead({ id: "ready-1" }),
    ]);
    const spawned: string[] = [];
    const scanner = createScanner({
      runPipeline: async ({ beadId }) => {
        spawned.push(beadId);
      },
    });

    await scanner.tick([{ project: PROJECT, queue }]);
    await scanner.whenIdle();

    expect(claimed).toEqual(["ready-1"]);
    expect(spawned).toEqual(["ready-1"]);
  });

  it("skips a bead when claim loses to a human", async () => {
    const { queue, claimed, lost } = createQueue([
      bead({ id: "raced-1" }),
      bead({ id: "ready-2" }),
    ]);
    lost.add("raced-1");
    const spawned: string[] = [];
    const scanner = createScanner({
      runPipeline: async ({ beadId }) => {
        spawned.push(beadId);
      },
    });

    await scanner.tick([{ project: PROJECT, queue }]);
    await scanner.whenIdle();

    expect(claimed).toEqual(["ready-2"]);
    expect(spawned).toEqual(["ready-2"]);
  });

  it("does not claim an 21st bead while 20 pipelines are in flight", async () => {
    const ready = Array.from({ length: 25 }, (_, i) => bead({ id: `bead-${i + 1}` }));
    const { queue, claimed } = createQueue(ready);
    let release!: () => void;
    const hold = new Promise<void>((resolve) => {
      release = resolve;
    });
    const scanner = createScanner({
      runPipeline: () => hold,
    });

    await scanner.tick([{ project: PROJECT, queue }]);

    try {
      expect(claimed).toHaveLength(20);
      expect(scanner.inFlightCount()).toBe(20);
    } finally {
      release();
      await scanner.whenIdle();
    }
  });

  it("claims again after an in-flight pipeline finishes", async () => {
    const ready = Array.from({ length: 21 }, (_, i) => bead({ id: `bead-${i + 1}` }));
    const { queue, claimed } = createQueue(ready);
    const releases: Array<() => void> = [];
    const scanner = createScanner({
      runPipeline: ({ beadId }) =>
        new Promise<void>((resolve) => {
          releases.push(() => {
            void beadId;
            resolve();
          });
        }),
    });

    await scanner.tick([{ project: PROJECT, queue }]);
    expect(claimed).toHaveLength(20);

    releases[0]!();
    while (scanner.inFlightCount() === 20) {
      await Promise.resolve();
    }
    await scanner.tick([{ project: PROJECT, queue }]);

    try {
      expect(claimed).toHaveLength(21);
    } finally {
      for (const r of releases) r();
      await scanner.whenIdle();
    }
  });

  it("does not claim new beads after Disarm, while in-flight pipelines finish", async () => {
    const { queue, claimed } = createQueue([
      bead({ id: "flying" }),
      bead({ id: "next" }),
    ]);
    let release!: () => void;
    const hold = new Promise<void>((resolve) => {
      release = resolve;
    });
    const spawned: string[] = [];
    const scanner = createScanner({
      maxInFlight: 1,
      runPipeline: async ({ beadId }) => {
        spawned.push(beadId);
        if (beadId === "flying") await hold;
      },
    });

    await scanner.tick([{ project: PROJECT, queue }]);
    expect(claimed).toEqual(["flying"]);
    expect(scanner.inFlightCount()).toBe(1);

    await scanner.tick([]);
    expect(claimed).toEqual(["flying"]);
    expect(spawned).toEqual(["flying"]);

    release();
    await scanner.whenIdle();
    await scanner.tick([]);
    expect(claimed).toEqual(["flying"]);
    expect(spawned).toEqual(["flying"]);
  });

  it("lets a finished bead complete without waiting for a sibling implement", async () => {
    const { queue } = createQueue([bead({ id: "slow" }), bead({ id: "fast" })]);
    const order: string[] = [];
    let releaseSlow!: () => void;
    const slowHold = new Promise<void>((resolve) => {
      releaseSlow = resolve;
    });
    const scanner = createScanner({
      runPipeline: async ({ beadId }) => {
        if (beadId === "slow") await slowHold;
        order.push(beadId);
      },
    });

    await scanner.tick([{ project: PROJECT, queue }]);
    await new Promise((r) => setTimeout(r, 20));

    try {
      expect(order).toEqual(["fast"]);
      expect(scanner.inFlightCount()).toBe(1);
    } finally {
      releaseSlow();
      await scanner.whenIdle();
    }
    expect(order).toEqual(["fast", "slow"]);
  });

  it("caps in-flight at 20 across all Armed projects", async () => {
    const other: Project = { path: "/projects/other", targetBranch: "main" };
    const a = createQueue(
      Array.from({ length: 15 }, (_, i) => bead({ id: `a-${i + 1}` })),
    );
    const b = createQueue(
      Array.from({ length: 15 }, (_, i) => bead({ id: `b-${i + 1}` })),
    );
    let release!: () => void;
    const hold = new Promise<void>((resolve) => {
      release = resolve;
    });
    const scanner = createScanner({
      runPipeline: () => hold,
    });

    await scanner.tick([
      { project: PROJECT, queue: a.queue },
      { project: other, queue: b.queue },
    ]);

    try {
      expect(a.claimed.length + b.claimed.length).toBe(20);
      expect(scanner.inFlightCount()).toBe(20);
    } finally {
      release();
      await scanner.whenIdle();
    }
  });
});
