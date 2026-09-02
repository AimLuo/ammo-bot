import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { AGENT_ACTOR, type Bead, type Beads } from "./types.ts";

const execFileAsync = promisify(execFile);

type BdIssue = {
  id?: string;
  title?: string;
  status?: string;
  assignee?: string;
  labels?: string[];
};

function asStatus(value: string | undefined): Bead["status"] {
  if (value === "in_progress" || value === "closed") return value;
  return "open";
}

export function createBeadsCli(repoPath: string): Beads {
  const run = async (args: string[]): Promise<string> => {
    const { stdout } = await execFileAsync("bd", args, {
      cwd: repoPath,
      maxBuffer: 32 * 1024 * 1024,
      env: { ...process.env, BEADS_ACTOR: AGENT_ACTOR },
    });
    return stdout;
  };

  const runJson = async <T>(args: string[]): Promise<T> => {
    const stdout = await run([...args, "--json"]);
    return JSON.parse(stdout) as T;
  };

  return {
    async claim(beadId) {
      await run(["update", beadId, "--claim"]);
    },
    async unclaim(beadId) {
      await run(["update", beadId, "--status", "open", "--assignee", ""]);
    },
    async close(beadId) {
      try {
        await run(["close", beadId]);
      } catch (err) {
        const bead = await this.get(beadId);
        if (bead.status === "closed") return;
        throw err;
      }
    },
    async addHumanFlag(beadId, comment) {
      await run(["update", beadId, "--add-label", "human"]);
      await run(["comment", beadId, comment]);
    },
    async get(beadId) {
      const raw = await runJson<BdIssue | BdIssue[]>(["show", beadId]);
      const issue = Array.isArray(raw) ? raw[0] : raw;
      if (!issue?.id) {
        throw new Error(`bd show ${beadId} returned no issue`);
      }
      return {
        id: issue.id,
        title: issue.title ?? "",
        status: asStatus(issue.status),
        assignee: issue.assignee || null,
        labels: issue.labels ?? [],
      };
    },
    async waitUntilNoHumanFlag(beadId) {
      for (;;) {
        const bead = await this.get(beadId);
        if (!bead.labels.includes("human")) return;
        await new Promise((resolve) => setTimeout(resolve, 500));
      }
    },
  };
}
