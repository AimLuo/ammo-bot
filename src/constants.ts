export const DEFAULT_AGENT_MODEL = "composer-2";

/** How often pipeline re-reads the bead while waiting on, or watching for, a human flag. Shared by the in-flight watch and wait-until-cleared; 5s is the agreed abort/Resume latency. */
export const HUMAN_FLAG_POLL_MS = 5000;
