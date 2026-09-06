export const AGENT_ACTOR = "ammo-bot";

export const COMPLETION_SIGNAL = "<promise>COMPLETE</promise>";

export const DEFAULT_TARGET_BRANCH = "main";

export const MAX_IMPLEMENT_ITERATIONS = 100;

export type AgentRole = "implement" | "review" | "merge";

export type Project = {
  path: string;
  targetBranch?: string;
};

export type Bead = {
  id: string;
  title: string;
  status: "open" | "in_progress" | "closed";
  assignee: string | null;
  labels: string[];
};

export type BeadComment = {
  author: string;
  text: string;
};

export type Agent = {
  run(input: {
    cwd: string;
    prompt: string;
    role: AgentRole;
    signal?: AbortSignal;
  }): Promise<{ output: string }>;
};

export type Beads = {
  claim(beadId: string): Promise<void>;
  unclaim(beadId: string): Promise<void>;
  close(beadId: string): Promise<void>;
  addHumanFlag(beadId: string, comment: string): Promise<void>;
  get(beadId: string): Promise<Bead>;
  listComments(beadId: string): Promise<BeadComment[]>;
  waitUntilNoHumanFlag(beadId: string): Promise<void>;
};

export type Git = {
  addWorktree(input: {
    primaryPath: string;
    worktreePath: string;
    branch: string;
    targetBranch: string;
  }): Promise<void>;
  removeWorktree(input: {
    primaryPath: string;
    worktreePath: string;
    branch: string;
  }): Promise<void>;
  listCommitsAhead(input: {
    cwd: string;
    targetBranch: string;
  }): Promise<string[]>;
  status(input: { cwd: string }): Promise<{ clean: boolean; branch: string }>;
};

export type PipelineResult =
  | { outcome: "cancelled" }
  | { outcome: "merged" };
