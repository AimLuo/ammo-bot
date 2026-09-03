import { createScannerQueue } from "./beads-cli.ts";
import { runPipeline } from "./pipeline.ts";
import { createScanner } from "./scanner.ts";
import type { Project } from "./types.ts";

const DEFAULT_INTERVAL_MS = 3000;

export function startScannerLoop(input: {
  listArmed: () => Project[];
}): { stop: () => void } {
  const scanner = createScanner({
    runPipeline,
  });
  let stopped = false;

  const loop = async () => {
    while (!stopped) {
      const armed = input.listArmed().map((project) => ({
        project,
        queue: createScannerQueue(project.path),
      }));
      try {
        await scanner.tick(armed);
      } catch (err) {
        console.error("[scanner]", err);
      }
      if (stopped) return;
      await new Promise((resolve) => setTimeout(resolve, DEFAULT_INTERVAL_MS));
    }
  };

  void loop();
  return {
    stop() {
      stopped = true;
    },
  };
}
