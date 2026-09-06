import type { AgentRole } from "./types.ts";

const IMPLEMENT_AGENT_MODEL = "composer-2.5";
const REVIEW_AGENT_MODEL = "cursor-grok-4.6-high";
const MERGE_AGENT_MODEL = "cursor-grok-4.6-high";

export const AGENT_MODEL_BY_ROLE: Record<AgentRole, string> = {
  implement: IMPLEMENT_AGENT_MODEL,
  review: REVIEW_AGENT_MODEL,
  merge: MERGE_AGENT_MODEL,
};

/** How often pipeline re-reads the bead while waiting on, or watching for, a human flag. Shared by the in-flight watch and wait-until-cleared; 5s is the agreed abort/Resume latency. */
export const HUMAN_FLAG_POLL_MS = 5000;
