import { execFile } from "node:child_process";
import { DEFAULT_AGENT_MODEL } from "./constants.ts";
import type { Agent } from "./types.ts";

function runAgent(
  args: string[],
  opts: { cwd: string; signal?: AbortSignal },
): Promise<{ output: string }> {
  return new Promise((resolve, reject) => {
    execFile(
      "agent",
      args,
      {
        cwd: opts.cwd,
        maxBuffer: 32 * 1024 * 1024,
        signal: opts.signal,
      },
      (err, stdout) => {
        if (opts.signal?.aborted) {
          resolve({ output: "" });
          return;
        }
        if (err) {
          reject(err);
          return;
        }
        resolve({ output: stdout });
      },
    );
  });
}

export function createCursorAgent(
  model: string = DEFAULT_AGENT_MODEL,
): Agent {
  return {
    async run({ cwd, prompt, signal }) {
      return runAgent(
        [
          "--print",
          "--output-format",
          "stream-json",
          "--model",
          model,
          "--force",
          prompt,
        ],
        { cwd, signal },
      );
    },
  };
}
