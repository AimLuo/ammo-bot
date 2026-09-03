import { join } from "node:path";
import { createBeadsCli } from "./beads-cli.ts";
import { createCursorAgent } from "./cursor-agent.ts";
import { createGitCli } from "./git-cli.ts";
import { mergeLock } from "./merge-lock.ts";
import { implementPrompt, mergePrompt, reviewPrompt } from "./prompts.ts";
import {
  AGENT_ACTOR,
  COMPLETION_SIGNAL,
  DEFAULT_TARGET_BRANCH,
  MAX_IMPLEMENT_ITERATIONS,
  type Agent,
  type Beads,
  type Git,
  type PipelineResult,
  type Project,
} from "./types.ts";

export function worktreePath(projectPath: string, beadId: string): string {
  return join(projectPath, ".worktrees", `ammo-issue-${beadId}`);
}

export function issueBranch(beadId: string): string {
  return `ammo/issue-${beadId}`;
}

function mergeBlockedReason(
  status: { clean: boolean; branch: string },
  targetBranch: string,
): string | null {
  if (!status.clean) {
    return "primary checkout has uncommitted changes";
  }
  if (status.branch !== targetBranch) {
    return `primary checkout is on ${status.branch}, not ${targetBranch}`;
  }
  return null;
}

function heldByAgent(bead: { assignee: string | null; status: string }): boolean {
  return bead.assignee === AGENT_ACTOR && bead.status === "in_progress";
}

export async function runPipeline(input: {
  project: Project;
  beadId: string;
  agent?: Agent;
  beads?: Beads;
  git?: Git;
}): Promise<PipelineResult> {
  const agent = input.agent ?? createCursorAgent();
  const beads = input.beads ?? createBeadsCli(input.project.path);
  const git = input.git ?? createGitCli();
  const targetBranch = input.project.targetBranch ?? DEFAULT_TARGET_BRANCH;
  const worktree = worktreePath(input.project.path, input.beadId);
  const branch = issueBranch(input.beadId);

  const before = await beads.get(input.beadId);
  const skipToMerge = heldByAgent(before);

  await beads.claim(input.beadId);
  const bead = await beads.get(input.beadId);
  await git.addWorktree({
    primaryPath: input.project.path,
    worktreePath: worktree,
    branch,
    targetBranch,
  });

  async function waitToResume(): Promise<boolean> {
    await beads.waitUntilNoHumanFlag(input.beadId);
    return heldByAgent(await beads.get(input.beadId));
  }

  const existingCommits = await git.listCommitsAhead({
    cwd: worktree,
    targetBranch,
  });
  if (!(skipToMerge && existingCommits.length > 0)) {
    for (let i = 0; i < MAX_IMPLEMENT_ITERATIONS; i++) {
      if (!(await waitToResume())) {
        return { outcome: "cancelled" };
      }
      const { output } = await agent.run({
        cwd: worktree,
        prompt: implementPrompt({
          beadId: input.beadId,
          title: bead.title,
          branch,
        }),
        role: "implement",
      });
      if (output.includes(COMPLETION_SIGNAL)) break;
    }

    const commits = await git.listCommitsAhead({
      cwd: worktree,
      targetBranch,
    });
    if (commits.length === 0) {
      await beads.unclaim(input.beadId);
      await git.removeWorktree({
        primaryPath: input.project.path,
        worktreePath: worktree,
        branch,
      });
      return { outcome: "cancelled" };
    }

    if (!(await waitToResume())) {
      return { outcome: "cancelled" };
    }
    await agent.run({
      cwd: worktree,
      prompt: reviewPrompt({ branch, targetBranch }),
      role: "review",
    });
  }

  const lockKey = `${input.project.path}::${targetBranch}`;
  for (;;) {
    if (!(await waitToResume())) {
      return { outcome: "cancelled" };
    }

    const blocked = mergeBlockedReason(
      await git.status({ cwd: input.project.path }),
      targetBranch,
    );
    if (blocked) {
      await beads.addHumanFlag(input.beadId, blocked);
      await beads.waitUntilNoHumanFlag(input.beadId);
      continue;
    }

    let waitForHuman = false;
    const release = await mergeLock.acquire(lockKey);
    try {
      const blockedUnderLock = mergeBlockedReason(
        await git.status({ cwd: input.project.path }),
        targetBranch,
      );
      if (blockedUnderLock) {
        await beads.addHumanFlag(input.beadId, blockedUnderLock);
        waitForHuman = true;
      } else {
        const { output } = await agent.run({
          cwd: input.project.path,
          prompt: mergePrompt({
            beadId: input.beadId,
            branch,
            targetBranch,
          }),
          role: "merge",
        });
        if (output.includes(COMPLETION_SIGNAL)) {
          await beads.close(input.beadId);
          await git.removeWorktree({
            primaryPath: input.project.path,
            worktreePath: worktree,
            branch,
          });
          return { outcome: "merged" };
        }
        await beads.addHumanFlag(
          input.beadId,
          "merger could not complete typecheck or tests",
        );
        waitForHuman = true;
      }
    } finally {
      release();
    }
    if (waitForHuman) {
      await beads.waitUntilNoHumanFlag(input.beadId);
    }
  }
}
