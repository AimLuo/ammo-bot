import { execFile } from "node:child_process";
import { access, mkdir } from "node:fs/promises";
import { dirname } from "node:path";
import { promisify } from "node:util";
import type { Git } from "./types.ts";

const execFileAsync = promisify(execFile);

async function pathExists(path: string): Promise<boolean> {
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function git(args: string[], cwd: string): Promise<string> {
  const { stdout } = await execFileAsync("git", args, {
    cwd,
    encoding: "utf8",
    maxBuffer: 16 * 1024 * 1024,
  });
  return stdout;
}

async function gitOk(args: string[], cwd: string): Promise<boolean> {
  try {
    await git(args, cwd);
    return true;
  } catch {
    return false;
  }
}

export function createGitCli(): Git {
  return {
    async addWorktree({ primaryPath, worktreePath, branch, targetBranch }) {
      await mkdir(dirname(worktreePath), { recursive: true });
      if (await pathExists(worktreePath)) {
        return;
      }
      const branchExists = await gitOk(
        ["show-ref", "--verify", "--quiet", `refs/heads/${branch}`],
        primaryPath,
      );
      if (branchExists) {
        await git(["worktree", "add", worktreePath, branch], primaryPath);
        return;
      }
      await git(
        ["worktree", "add", "-b", branch, worktreePath, targetBranch],
        primaryPath,
      );
    },
    async removeWorktree({ primaryPath, worktreePath, branch }) {
      try {
        await git(["worktree", "remove", "--force", worktreePath], primaryPath);
      } catch {
        // worktree may already be gone
      }
      try {
        await git(["branch", "-D", branch], primaryPath);
      } catch {
        // named branch may already be gone
      }
    },
    async listCommitsAhead({ cwd, targetBranch }) {
      const stdout = await git(["log", "--format=%H", `${targetBranch}..HEAD`], cwd);
      return stdout
        .split("\n")
        .map((line) => line.trim())
        .filter(Boolean);
    },
    async status({ cwd }) {
      const porcelain = await git(["status", "--porcelain"], cwd);
      const branch = (
        await git(["rev-parse", "--abbrev-ref", "HEAD"], cwd)
      ).trim();
      return { clean: porcelain.trim() === "", branch };
    },
  };
}
