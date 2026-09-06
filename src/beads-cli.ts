import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { HUMAN_FLAG_POLL_MS } from "./constants.ts";
import { AGENT_ACTOR, type Bead, type BeadComment, type Beads } from "./types.ts";
import type { QueueBead, ScannerQueue } from "./scanner.ts";

const execFileAsync = promisify(execFile);

type BdIssue = {
  id?: string;
  title?: string;
  status?: string;
  assignee?: string;
  labels?: string[];
  issue_type?: string;
};

function asStatus(value: string | undefined): Bead["status"] {
  if (value === "in_progress" || value === "closed") return value;
  return "open";
}

function createBdRunner(projectPath: string) {
  const run = async (args: string[]): Promise<string> => {
    const { stdout } = await execFileAsync("bd", args, {
      cwd: projectPath,
      maxBuffer: 32 * 1024 * 1024,
      env: { ...process.env, BEADS_ACTOR: AGENT_ACTOR },
    });
    return stdout;
  };

  const runJson = async <T>(args: string[]): Promise<T> => {
    const stdout = await run([...args, "--json"]);
    return JSON.parse(stdout) as T;
  };

  return { run, runJson };
}

function asQueueBead(issue: BdIssue): QueueBead | null {
  if (!issue.id) return null;
  return {
    id: issue.id,
    issue_type: issue.issue_type ?? "task",
    labels: issue.labels ?? [],
    status: issue.status ?? "open",
  };
}

export function createScannerQueue(projectPath: string): ScannerQueue {
  const { run, runJson } = createBdRunner(projectPath);
  return {
    async listReady() {
      const stdout = await run(["ready", "--json"]);
      if (!stdout.trim()) return [];
      const raw = JSON.parse(stdout) as BdIssue[] | BdIssue | null;
      const issues = Array.isArray(raw) ? raw : raw ? [raw] : [];
      return issues.flatMap((issue) => {
        const item = asQueueBead(issue);
        return item ? [item] : [];
      });
    },
    async tryClaim(beadId) {
      try {
        await run(["update", beadId, "--claim"]);
      } catch {
        return false;
      }
      const raw = await runJson<BdIssue | BdIssue[]>(["show", beadId]);
      const issue = Array.isArray(raw) ? raw[0] : raw;
      return (
        issue?.status === "in_progress" && (issue.assignee || null) === AGENT_ACTOR
      );
    },
  };
}

export function createBeadsCli(repoPath: string): Beads {
  const { run, runJson } = createBdRunner(repoPath);

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
    async listComments(beadId) {
      const stdout = await run(["comments", beadId, "--json"]);
      if (!stdout.trim()) return [];
      const raw = JSON.parse(stdout) as unknown;
      const items = Array.isArray(raw) ? raw : raw ? [raw] : [];
      return items.flatMap((item): BeadComment[] => {
        if (!item || typeof item !== "object") return [];
        const rec = item as { author?: unknown; text?: unknown; body?: unknown };
        const text =
          typeof rec.text === "string"
            ? rec.text
            : typeof rec.body === "string"
              ? rec.body
              : "";
        if (!text) return [];
        return [
          {
            author: typeof rec.author === "string" ? rec.author : "",
            text,
          },
        ];
      });
    },
    async waitUntilNoHumanFlag(beadId) {
      for (;;) {
        const bead = await this.get(beadId);
        if (!bead.labels.includes("human")) return;
        await new Promise((resolve) => setTimeout(resolve, HUMAN_FLAG_POLL_MS));
      }
    },
  };
}
