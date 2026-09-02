import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { DEFAULT_AGENT_MODEL } from "./constants.ts";
import type { Agent } from "./types.ts";

const execFileAsync = promisify(execFile);

export function createCursorAgent(
  model: string = DEFAULT_AGENT_MODEL,
): Agent {
  return {
    async run({ cwd, prompt }) {
      const { stdout } = await execFileAsync(
        "agent",
        [
          "--print",
          "--output-format",
          "stream-json",
          "--model",
          model,
          "--force",
          prompt,
        ],
        {
          cwd,
          maxBuffer: 32 * 1024 * 1024,
        },
      );
      return { output: stdout };
    },
  };
}
