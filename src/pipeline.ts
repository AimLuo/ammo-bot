import { readFile, stat } from "node:fs/promises";
import { join } from "node:path";
import { createBeadsCli } from "./beads-cli.ts";
import { HUMAN_FLAG_POLL_MS } from "./constants.ts";
import { createCursorAgent } from "./cursor-agent.ts";
import { createGitCli } from "./git-cli.ts";
import { mergeLock } from "./merge-lock.ts";
import { mergeSyncFrom, type MergeSync } from "./project-layout.ts";
import { implementPrompt, mergePrompt, reviewPrompt } from "./prompts.ts";
import {
  AGENT_ACTOR,
  COMPLETION_SIGNAL,
  DEFAULT_TARGET_BRANCH,
  MAX_IMPLEMENT_ITERATIONS,
  type Agent,
  type AgentRole,
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
  mergeSync?: MergeSync;
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

  async function commentsForPrompt() {
    return beads.listComments(input.beadId);
  }

  async function runAgent(args: {
    cwd: string;
    prompt: string;
    role: AgentRole;
  }): Promise<{ output: string }> {
    const ac = new AbortController();
    const watch = (async () => {
      while (!ac.signal.aborted) {
        const current = await beads.get(input.beadId);
        if (current.labels.includes("human")) {
          ac.abort();
          return;
        }
        await new Promise<void>((resolve) => {
          if (ac.signal.aborted) {
            resolve();
            return;
          }
          const timer = setTimeout(resolve, HUMAN_FLAG_POLL_MS);
          ac.signal.addEventListener(
            "abort",
            () => {
              clearTimeout(timer);
              resolve();
            },
            { once: true },
          );
        });
      }
    })();
    try {
      return await agent.run({ ...args, signal: ac.signal });
    } catch (err) {
      if (ac.signal.aborted) return { output: "" };
      throw err;
    } finally {
      ac.abort();
      await watch;
    }
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
      const { output } = await runAgent({
        cwd: worktree,
        prompt: implementPrompt({
          beadId: input.beadId,
          title: bead.title,
          branch,
          comments: await commentsForPrompt(),
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
    await runAgent({
      cwd: worktree,
      prompt: reviewPrompt({
        beadId: input.beadId,
        branch,
        targetBranch,
        comments: await commentsForPrompt(),
      }),
      role: "review",
    });
  }

  const lockKey = `${input.project.path}::${targetBranch}`;
  let rememberedSubtrees: MergeSync["subtrees"] | undefined;
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
        let mergeSync =
          input.mergeSync ??
          (await loadMergeSync({
            projectPath: input.project.path,
            git,
            targetBranch,
            branch,
          }));
        if (mergeSync.subtrees.length > 0) {
          rememberedSubtrees = mergeSync.subtrees;
        } else if (rememberedSubtrees?.length) {
          mergeSync = { ...mergeSync, subtrees: rememberedSubtrees };
        }
        const { output } = await runAgent({
          cwd: input.project.path,
          prompt: mergePrompt({
            beadId: input.beadId,
            branch,
            targetBranch,
            comments: await commentsForPrompt(),
            sync: mergeSync,
          }),
          role: "merge",
        });
        if (output.includes(COMPLETION_SIGNAL)) {
          if (
            await git.hasUnpushedCommits({
              cwd: input.project.path,
              targetBranch,
            })
          ) {
            await beads.addHumanFlag(
              input.beadId,
              "target branch has commits not on origin",
            );
            waitForHuman = true;
          } else {
            await beads.close(input.beadId);
            await git.removeWorktree({
              primaryPath: input.project.path,
              worktreePath: worktree,
              branch,
            });
            return { outcome: "merged" };
          }
        } else {
          await beads.addHumanFlag(
            input.beadId,
            "merger could not complete typecheck or tests",
          );
          waitForHuman = true;
        }
      }
    } finally {
      release();
    }
    if (waitForHuman) {
      await beads.waitUntilNoHumanFlag(input.beadId);
    }
  }
}

async function loadMergeSync(input: {
  projectPath: string;
  git: Git;
  targetBranch: string;
  branch: string;
}): Promise<MergeSync> {
  let agentsMd: string | null = null;
  try {
    agentsMd = await readFile(join(input.projectPath, "AGENTS.md"), "utf8");
  } catch {
    agentsMd = null;
  }
  let hasReposDir = false;
  try {
    hasReposDir = (await stat(join(input.projectPath, "repos"))).isDirectory();
  } catch {
    hasReposDir = false;
  }
  const remotes = await input.git.listRemotes({ cwd: input.projectPath });
  let diffPaths = await input.git.listChangedPaths({
    cwd: input.projectPath,
    from: input.targetBranch,
    to: input.branch,
    pathspec: "repos/",
  });
  if (diffPaths.length === 0) {
    diffPaths = await input.git.listChangedPaths({
      cwd: input.projectPath,
      from: `origin/${input.targetBranch}`,
      to: "HEAD",
      pathspec: "repos/",
    });
  }
  return mergeSyncFrom({
    agentsMd,
    hasReposDir,
    remotes,
    diffPaths,
  });
}
