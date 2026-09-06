import { describe, expect, it } from "vitest";
import { HUMAN_FLAG_POLL_MS } from "./constants.ts";
import {
  issueBranch,
  runPipeline,
  worktreePath as projectWorktreePath,
} from "./pipeline.ts";
import {
  AGENT_ACTOR,
  COMPLETION_SIGNAL,
  type Agent,
  type AgentRole,
  type Beads,
  type Bead,
  type BeadComment,
  type Git,
} from "./types.ts";

const PRIMARY = "/projects/demo";
const TARGET = "main";

function worktreePath(beadId: string) {
  return projectWorktreePath(PRIMARY, beadId);
}

type ImplementStep = {
  output?: string;
  commit?: boolean;
  delayMs?: number;
  raiseHumanFlag?: boolean;
};

type MergeConcurrency = { current: number; max: number };

function createHarness(options: {
  beadId?: string;
  implement?: ImplementStep[];
  review?: { output?: string; delayMs?: number; raiseHumanFlag?: boolean };
  merge?: { output?: string | string[]; delayMs?: number };
  primary?: { clean?: boolean; branch?: string };
  mergeConcurrency?: MergeConcurrency;
  claimedByAgent?: boolean;
  seedCommits?: string[];
  startWithHumanFlag?: boolean;
} = {}) {
  const beadId = options.beadId ?? "bead-1";
  const mergeConcurrency = options.mergeConcurrency ?? {
    current: 0,
    max: 0,
  };
  const bead: Bead = {
    id: beadId,
    title: "Do the thing",
    status: options.claimedByAgent ? "in_progress" : "open",
    assignee: options.claimedByAgent ? AGENT_ACTOR : null,
    labels: options.startWithHumanFlag ? ["human"] : [],
  };

  function raiseFlag() {
    if (!bead.labels.includes("human")) bead.labels.push("human");
  }

  const agentCalls: {
    role: AgentRole;
    cwd: string;
    at: number;
    prompt: string;
  }[] = [];
  const thread: BeadComment[] = [];
  const commitsAhead = new Map<string, string[]>();
  const worktrees = new Set<string>();
  const removedWorktrees: string[] = [];
  const removedBranches: string[] = [];
  let primaryClean = options.primary?.clean ?? true;
  let primaryBranch = options.primary?.branch ?? TARGET;
  let implementIndex = 0;
  let mergeIndex = 0;
  let humanFlagWaiters: Array<() => void> = [];
  let humanFlagCount = 0;
  const humanFlagListeners: Array<() => void> = [];
  let waitingForHumanFlag = 0;
  let waitStartResolvers: Array<() => void> = [];
  let agentStarted = 0;
  let agentStartResolvers: Array<() => void> = [];
  let mergeStarted!: () => void;
  const whenMergeStarted = new Promise<void>((resolve) => {
    mergeStarted = resolve;
  });

  const implementSteps: ImplementStep[] =
    options.implement ?? [{ output: COMPLETION_SIGNAL }];

  function sleep(ms: number, signal?: AbortSignal) {
    return new Promise<void>((resolve) => {
      if (signal?.aborted) {
        resolve();
        return;
      }
      const timer = setTimeout(resolve, ms);
      signal?.addEventListener(
        "abort",
        () => {
          clearTimeout(timer);
          resolve();
        },
        { once: true },
      );
    });
  }

  const agent: Agent = {
    async run({ cwd, role, prompt, signal }) {
      agentCalls.push({ role, cwd, at: Date.now(), prompt });
      agentStarted += 1;
      const started = agentStartResolvers;
      agentStartResolvers = [];
      for (const resolve of started) resolve();
      if (role === "implement") {
        const step = implementSteps[implementIndex] ?? { output: "" };
        implementIndex += 1;
        if (step.delayMs) {
          await sleep(step.delayMs, signal);
        }
        if (signal?.aborted) return { output: "" };
        if (step.commit) {
          const existing = commitsAhead.get(cwd) ?? [];
          commitsAhead.set(cwd, [...existing, `sha-${existing.length + 1}`]);
        }
        if (step.raiseHumanFlag) raiseFlag();
        return { output: step.output ?? "" };
      }
      if (role === "review") {
        const reviewDelay = options.review?.delayMs;
        if (reviewDelay) {
          await sleep(reviewDelay, signal);
        }
        if (signal?.aborted) return { output: "" };
        if (options.review?.raiseHumanFlag) raiseFlag();
        return { output: options.review?.output ?? COMPLETION_SIGNAL };
      }
      mergeConcurrency.current += 1;
      mergeConcurrency.max = Math.max(
        mergeConcurrency.max,
        mergeConcurrency.current,
      );
      mergeStarted();
      const mergeDelay = options.merge?.delayMs;
      if (mergeDelay) {
        await sleep(mergeDelay, signal);
      }
      mergeConcurrency.current -= 1;
      if (signal?.aborted) return { output: "" };
      const outputs = options.merge?.output;
      const scripted = Array.isArray(outputs)
        ? outputs
        : [outputs ?? COMPLETION_SIGNAL];
      const output = scripted[mergeIndex] ?? scripted[scripted.length - 1] ?? "";
      mergeIndex += 1;
      return { output };
    },
  };

  const beads: Beads & {
    comments: string[];
  } = {
    comments: [],
    async claim(id) {
      if (id !== bead.id) throw new Error(`unknown bead ${id}`);
      bead.status = "in_progress";
      bead.assignee = AGENT_ACTOR;
    },
    async unclaim(id) {
      if (id !== bead.id) throw new Error(`unknown bead ${id}`);
      bead.status = "open";
      bead.assignee = null;
    },
    async close(id) {
      if (id !== bead.id) throw new Error(`unknown bead ${id}`);
      bead.status = "closed";
    },
    async addHumanFlag(id, comment) {
      if (id !== bead.id) throw new Error(`unknown bead ${id}`);
      if (!bead.labels.includes("human")) bead.labels.push("human");
      this.comments.push(comment);
      thread.push({ author: AGENT_ACTOR, text: comment });
      humanFlagCount += 1;
      for (const listener of humanFlagListeners) listener();
    },
    async get(id) {
      if (id !== bead.id) throw new Error(`unknown bead ${id}`);
      return { ...bead, labels: [...bead.labels] };
    },
    async listComments(id) {
      if (id !== bead.id) throw new Error(`unknown bead ${id}`);
      return thread.map((c) => ({ ...c }));
    },
    async waitUntilNoHumanFlag(id) {
      if (id !== bead.id) throw new Error(`unknown bead ${id}`);
      if (!bead.labels.includes("human")) return;
      waitingForHumanFlag += 1;
      const started = waitStartResolvers;
      waitStartResolvers = [];
      for (const resolve of started) resolve();
      await new Promise<void>((resolve) => {
        humanFlagWaiters.push(resolve);
      });
      waitingForHumanFlag -= 1;
    },
  };

  const git: Git & {
    setPrimary(next: { clean?: boolean; branch?: string }): void;
  } = {
    async addWorktree({ worktreePath, branch }) {
      worktrees.add(worktreePath);
      if (!commitsAhead.has(worktreePath)) {
        commitsAhead.set(worktreePath, options.seedCommits ?? []);
      }
      void branch;
    },
    async removeWorktree({ worktreePath, branch }) {
      worktrees.delete(worktreePath);
      removedWorktrees.push(worktreePath);
      removedBranches.push(branch);
    },
    async listCommitsAhead({ cwd }) {
      return commitsAhead.get(cwd) ?? [];
    },
    async status({ cwd }) {
      if (cwd === PRIMARY) {
        return { clean: primaryClean, branch: primaryBranch };
      }
      return { clean: true, branch: issueBranch(beadId) };
    },
    setPrimary(next) {
      if (next.clean !== undefined) primaryClean = next.clean;
      if (next.branch !== undefined) primaryBranch = next.branch;
    },
  };

  function clearHumanFlag() {
    bead.labels = bead.labels.filter((l) => l !== "human");
    const waiters = humanFlagWaiters;
    humanFlagWaiters = [];
    for (const w of waiters) w();
  }

  async function waitForHumanFlagCount(n: number) {
    while (humanFlagCount < n) {
      await new Promise<void>((resolve) => {
        humanFlagListeners.push(resolve);
      });
    }
  }

  function whenWaitingForHumanFlag() {
    if (waitingForHumanFlag > 0) return Promise.resolve();
    return new Promise<void>((resolve) => {
      waitStartResolvers.push(resolve);
    });
  }

  return {
    beadId,
    bead,
    agentCalls,
    worktrees,
    removedWorktrees,
    removedBranches,
    get maxConcurrentMerges() {
      return mergeConcurrency.max;
    },
    whenMergeStarted,
    waitForHumanFlagCount,
    whenWaitingForHumanFlag,
    whenAgentStarted() {
      if (agentStarted > 0) return Promise.resolve();
      return new Promise<void>((resolve) => {
        agentStartResolvers.push(resolve);
      });
    },
    raiseHumanFlag() {
      raiseFlag();
    },
    clearHumanFlag,
    addComment(author: string, text: string) {
      thread.push({ author, text });
    },
    setAssignee(assignee: string | null) {
      bead.assignee = assignee;
    },
    comments: beads.comments,
    git,
    run() {
      return runPipeline({
        project: { path: PRIMARY, targetBranch: TARGET },
        beadId,
        agent,
        beads,
        git,
      });
    },
  };
}

describe("runPipeline", () => {
  it("cancels claim without review, merge, or close when implement produces no commits", async () => {
    const h = createHarness({
      implement: [{ output: COMPLETION_SIGNAL }],
    });

    const result = await h.run();

    expect(result).toEqual({ outcome: "cancelled" });
    expect(h.bead.status).toBe("open");
    expect(h.bead.assignee).toBeNull();
    expect(h.agentCalls.map((c) => c.role)).toEqual(["implement"]);
    expect(h.agentCalls[0]?.cwd).toBe(worktreePath(h.beadId));
    expect(h.bead.labels).not.toContain("human");
    expect(h.removedWorktrees).toContain(worktreePath(h.beadId));
    expect(h.removedBranches).toContain(issueBranch(h.beadId));
  });

  it("runs implement iterations until the completion signal, then reviews once and merges", async () => {
    const h = createHarness({
      implement: [
        { output: "still working", commit: true },
        { output: COMPLETION_SIGNAL, commit: true },
      ],
    });

    const result = await h.run();

    expect(result).toEqual({ outcome: "merged" });
    expect(h.bead.status).toBe("closed");
    expect(h.agentCalls.map((c) => c.role)).toEqual([
      "implement",
      "implement",
      "review",
      "merge",
    ]);
    expect(h.agentCalls[0]?.cwd).toBe(worktreePath(h.beadId));
    expect(h.agentCalls[2]?.cwd).toBe(worktreePath(h.beadId));
    expect(h.agentCalls[3]?.cwd).toBe(PRIMARY);
    expect(h.removedWorktrees).toContain(worktreePath(h.beadId));
    expect(h.removedBranches).toContain(issueBranch(h.beadId));
  });

  it("stops implement after 100 iterations without a completion signal", async () => {
    const h = createHarness({
      implement: Array.from({ length: 100 }, (_, i) => ({
        commit: i === 0,
      })),
    });

    const result = await h.run();

    expect(result).toEqual({ outcome: "merged" });
    expect(h.agentCalls.filter((c) => c.role === "implement")).toHaveLength(
      100,
    );
    expect(h.agentCalls.filter((c) => c.role === "review")).toHaveLength(1);
    expect(h.agentCalls.filter((c) => c.role === "merge")).toHaveLength(1);
  });

  it("runs two mergers one at a time", async () => {
    const mergeConcurrency = { current: 0, max: 0 };
    const a = createHarness({
      beadId: "bead-a",
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
      merge: { delayMs: 40 },
      mergeConcurrency,
    });
    const b = createHarness({
      beadId: "bead-b",
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
      merge: { delayMs: 40 },
      mergeConcurrency,
    });

    const [ra, rb] = await Promise.all([a.run(), b.run()]);

    expect(ra).toEqual({ outcome: "merged" });
    expect(rb).toEqual({ outcome: "merged" });
    expect(mergeConcurrency.max).toBe(1);
  });

  it("lets implement and review proceed while another pipeline holds the merge lock", async () => {
    const slow = createHarness({
      beadId: "slow-merge",
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
      merge: { delayMs: 80 },
    });
    const fast = createHarness({
      beadId: "fast-implement",
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
    });

    const slowRun = slow.run();
    await slow.whenMergeStarted;
    const started = Date.now();
    const fastRun = fast.run();
    const [slowResult, fastResult] = await Promise.all([slowRun, fastRun]);

    expect(slowResult).toEqual({ outcome: "merged" });
    expect(fastResult).toEqual({ outcome: "merged" });
    const fastReview = fast.agentCalls.find((c) => c.role === "review");
    const fastMerge = fast.agentCalls.find((c) => c.role === "merge");
    expect(fastReview).toBeDefined();
    expect(fastMerge).toBeDefined();
    expect(fastReview!.at - started).toBeLessThan(80);
  });

  it("flags human and keeps in_progress when primary checkout is dirty, then Resumes after the flag is cleared", async () => {
    const h = createHarness({
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
      primary: { clean: false },
    });

    const running = h.run();
    await h.waitForHumanFlagCount(1);

    expect(h.bead.status).toBe("in_progress");
    expect(h.bead.assignee).toBe(AGENT_ACTOR);
    expect(h.bead.labels).toContain("human");
    expect(h.comments.length).toBeGreaterThan(0);
    expect(h.agentCalls.map((c) => c.role)).toEqual(["implement", "review"]);
    expect(h.removedWorktrees).toEqual([]);

    h.git.setPrimary({ clean: true });
    h.clearHumanFlag();
    const result = await running;

    expect(result).toEqual({ outcome: "merged" });
    expect(h.bead.status).toBe("closed");
    expect(h.agentCalls.map((c) => c.role)).toEqual([
      "implement",
      "review",
      "merge",
    ]);
    expect(h.removedWorktrees).toContain(worktreePath(h.beadId));
  });

  it("flags human when primary checkout is not on the target branch", async () => {
    const h = createHarness({
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
      primary: { branch: "feature" },
    });

    const running = h.run();
    await h.waitForHumanFlagCount(1);

    expect(h.bead.status).toBe("in_progress");
    expect(h.bead.labels).toContain("human");
    expect(h.agentCalls.map((c) => c.role)).toEqual(["implement", "review"]);

    h.git.setPrimary({ branch: TARGET });
    h.clearHumanFlag();
    await running;
    expect(h.bead.status).toBe("closed");
  });

  it("flags human again when Resume still cannot proceed", async () => {
    const h = createHarness({
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
      primary: { clean: false },
    });

    const running = h.run();
    await h.waitForHumanFlagCount(1);
    h.clearHumanFlag();
    await h.waitForHumanFlagCount(2);

    expect(h.bead.status).toBe("in_progress");
    expect(h.bead.labels).toContain("human");
    expect(h.agentCalls.map((c) => c.role)).toEqual(["implement", "review"]);
    expect(h.removedWorktrees).toEqual([]);

    h.git.setPrimary({ clean: true });
    h.clearHumanFlag();
    await running;
    expect(h.bead.status).toBe("closed");
  });

  it("flags human and leaves the worktree when the merger iteration does not complete", async () => {
    const h = createHarness({
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
      merge: { output: ["tests still failing", COMPLETION_SIGNAL] },
    });

    const running = h.run();
    await h.waitForHumanFlagCount(1);

    expect(h.bead.status).toBe("in_progress");
    expect(h.bead.labels).toContain("human");
    expect(h.removedWorktrees).toEqual([]);
    expect(h.agentCalls.filter((c) => c.role === "merge")).toHaveLength(1);

    h.clearHumanFlag();
    await running;
    expect(h.bead.status).toBe("closed");
    expect(h.removedWorktrees).toContain(worktreePath(h.beadId));
  });

  it("skips implement and review when ammo-bot already holds the claim and the worktree has commits", async () => {
    const h = createHarness({
      claimedByAgent: true,
      seedCommits: ["sha-existing"],
    });

    const result = await h.run();

    expect(result).toEqual({ outcome: "merged" });
    expect(h.agentCalls.map((c) => c.role)).toEqual(["merge"]);
    expect(h.bead.status).toBe("closed");
  });

  it("does not merge after the human flag is cleared if ammo-bot no longer holds the claim", async () => {
    const h = createHarness({
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
      primary: { clean: false },
    });

    const running = h.run();
    await h.waitForHumanFlagCount(1);
    h.setAssignee("ada");
    h.clearHumanFlag();
    const result = await running;

    expect(result).toEqual({ outcome: "cancelled" });
    expect(h.agentCalls.map((c) => c.role)).toEqual(["implement", "review"]);
    expect(h.bead.status).toBe("in_progress");
    expect(h.removedWorktrees).toEqual([]);
  });

  it("does not call implement while a human flag is present, then Resumes after it is cleared", async () => {
    const h = createHarness({
      startWithHumanFlag: true,
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
    });

    const running = h.run();
    await h.whenWaitingForHumanFlag();

    expect(h.agentCalls).toEqual([]);
    expect(h.bead.status).toBe("in_progress");
    expect(h.bead.assignee).toBe(AGENT_ACTOR);
    expect(h.worktrees.has(worktreePath(h.beadId))).toBe(true);

    h.clearHumanFlag();
    const result = await running;

    expect(result).toEqual({ outcome: "merged" });
    expect(h.agentCalls.map((c) => c.role)).toEqual([
      "implement",
      "review",
      "merge",
    ]);
  });

  it("Resumes implement with bead comments after the human flag is cleared", async () => {
    const h = createHarness({
      implement: [
        { output: "asking", commit: true, raiseHumanFlag: true },
        { output: COMPLETION_SIGNAL, commit: true },
      ],
    });

    const running = h.run();
    await h.whenWaitingForHumanFlag();
    h.addComment("ada", "use Clerk for sign-in");
    h.clearHumanFlag();
    await running;

    const implementPrompts = h.agentCalls
      .filter((c) => c.role === "implement")
      .map((c) => c.prompt);
    expect(implementPrompts[0]).toContain("--add-label human");
    expect(implementPrompts[1]).toContain("ada");
    expect(implementPrompts[1]).toContain("use Clerk for sign-in");
    expect(h.agentCalls.map((c) => c.role)).toEqual([
      "implement",
      "implement",
      "review",
      "merge",
    ]);
  });

  it("Resumes review with bead comments after the human flag is cleared", async () => {
    const h = createHarness({
      implement: [
        { output: COMPLETION_SIGNAL, commit: true, raiseHumanFlag: true },
      ],
    });

    const running = h.run();
    await h.whenWaitingForHumanFlag();
    h.addComment("ada", "keep the public API unchanged");
    h.clearHumanFlag();
    await running;

    const reviewPrompt = h.agentCalls.find((c) => c.role === "review")?.prompt;
    expect(reviewPrompt).toContain("--add-label human");
    expect(reviewPrompt).toContain("ada");
    expect(reviewPrompt).toContain("keep the public API unchanged");
  });

  it("Resumes the merger with bead comments after the human flag is cleared", async () => {
    const h = createHarness({
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
      review: { raiseHumanFlag: true },
    });

    const running = h.run();
    await h.whenWaitingForHumanFlag();
    h.addComment("ada", "resolve the lockfile in favor of main");
    h.clearHumanFlag();
    await running;

    const mergePrompt = h.agentCalls.find((c) => c.role === "merge")?.prompt;
    expect(mergePrompt).toContain("--add-label human");
    expect(mergePrompt).toContain("ada");
    expect(mergePrompt).toContain("resolve the lockfile in favor of main");
  });

  it("stops the current implement iteration as soon as a human flag is added", async () => {
    const h = createHarness({
      implement: [
        { delayMs: HUMAN_FLAG_POLL_MS + 500, output: COMPLETION_SIGNAL, commit: true },
        { output: COMPLETION_SIGNAL, commit: true },
      ],
    });

    const running = h.run();
    await h.whenAgentStarted();
    h.raiseHumanFlag();
    await h.whenWaitingForHumanFlag();

    expect(h.agentCalls.map((c) => c.role)).toEqual(["implement"]);
    expect(h.bead.status).toBe("in_progress");
    expect(h.removedWorktrees).toEqual([]);

    h.clearHumanFlag();
    await running;

    expect(h.agentCalls.map((c) => c.role)).toEqual([
      "implement",
      "implement",
      "review",
      "merge",
    ]);
  }, HUMAN_FLAG_POLL_MS + 10_000);

  it("does not start the next implement iteration while a human flag is present", async () => {
    const h = createHarness({
      implement: [
        { output: "asking", commit: true, raiseHumanFlag: true },
        { output: COMPLETION_SIGNAL, commit: true },
      ],
    });

    const running = h.run();
    await h.whenWaitingForHumanFlag();

    expect(h.agentCalls.map((c) => c.role)).toEqual(["implement"]);
    expect(h.bead.status).toBe("in_progress");
    expect(h.removedWorktrees).toEqual([]);

    h.clearHumanFlag();
    await running;

    expect(h.agentCalls.map((c) => c.role)).toEqual([
      "implement",
      "implement",
      "review",
      "merge",
    ]);
  });

  it("does not call review while a human flag is present, then Resumes after it is cleared", async () => {
    const h = createHarness({
      implement: [
        { output: COMPLETION_SIGNAL, commit: true, raiseHumanFlag: true },
      ],
    });

    const running = h.run();
    await h.whenWaitingForHumanFlag();

    expect(h.agentCalls.map((c) => c.role)).toEqual(["implement"]);
    expect(h.removedWorktrees).toEqual([]);

    h.clearHumanFlag();
    await running;

    expect(h.agentCalls.map((c) => c.role)).toEqual([
      "implement",
      "review",
      "merge",
    ]);
  });

  it("does not call the merger while a human flag is present even if primary checkout is clean", async () => {
    const h = createHarness({
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
      review: { raiseHumanFlag: true },
    });

    const running = h.run();
    await h.whenWaitingForHumanFlag();

    expect(h.agentCalls.map((c) => c.role)).toEqual(["implement", "review"]);
    expect(h.bead.status).toBe("in_progress");
    expect(h.removedWorktrees).toEqual([]);

    h.clearHumanFlag();
    const result = await running;

    expect(result).toEqual({ outcome: "merged" });
    expect(h.agentCalls.map((c) => c.role)).toEqual([
      "implement",
      "review",
      "merge",
    ]);
  });

  it("cancels claim without review or merge when implement produces no commits even if a human flag is present", async () => {
    const h = createHarness({
      implement: [{ output: COMPLETION_SIGNAL, raiseHumanFlag: true }],
    });

    const result = await h.run();

    expect(result).toEqual({ outcome: "cancelled" });
    expect(h.bead.status).toBe("open");
    expect(h.bead.assignee).toBeNull();
    expect(h.bead.labels).toContain("human");
    expect(h.agentCalls.map((c) => c.role)).toEqual(["implement"]);
    expect(h.removedWorktrees).toContain(worktreePath(h.beadId));
    expect(h.removedBranches).toContain(issueBranch(h.beadId));
  });

  it("does not merge after Resume if ammo-bot no longer holds the claim", async () => {
    const h = createHarness({
      implement: [{ output: COMPLETION_SIGNAL, commit: true }],
      review: { raiseHumanFlag: true },
    });

    const running = h.run();
    await h.whenWaitingForHumanFlag();
    h.setAssignee("ada");
    h.clearHumanFlag();
    const result = await running;

    expect(result).toEqual({ outcome: "cancelled" });
    expect(h.agentCalls.map((c) => c.role)).toEqual(["implement", "review"]);
    expect(h.bead.status).toBe("in_progress");
    expect(h.removedWorktrees).toEqual([]);
  });
});
