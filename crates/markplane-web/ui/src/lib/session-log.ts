import type { Attempt, Usage } from "@/lib/types";

export function fmtTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(n >= 10_000_000 ? 0 : 1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(n >= 100_000 ? 0 : 1)}k`;
  return String(n);
}

export function fmtTime(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

export function fmtClock(d: Date): string {
  return d.toLocaleTimeString("en-US", { hour12: false });
}

export function fmtDuration(from: string, to: string | null | undefined): string {
  const ms = (to ? new Date(to) : new Date()).getTime() - new Date(from).getTime();
  const m = Math.round(ms / 60000);
  if (m < 1) return "<1m";
  return m >= 60 ? `${Math.floor(m / 60)}h ${m % 60}m` : `${m}m`;
}

/** Sum attempt usage; `unknown` when any finished attempt lacks usage (show ≥). */
export function sumUsage(attempts: Attempt[]) {
  let unknown = false;
  let inputTokens = 0;
  let outputTokens = 0;
  let cacheReadTokens = 0;
  let cacheWriteTokens = 0;
  for (const a of attempts) {
    if (!a.usage) {
      if (a.result != null) unknown = true;
      continue;
    }
    inputTokens += a.usage.inputTokens ?? 0;
    outputTokens += a.usage.outputTokens ?? 0;
    cacheReadTokens += a.usage.cacheReadTokens ?? 0;
    cacheWriteTokens += a.usage.cacheWriteTokens ?? 0;
    if (
      a.usage.inputTokens == null ||
      a.usage.outputTokens == null
    ) {
      unknown = true;
    }
  }
  return { inputTokens, outputTokens, cacheReadTokens, cacheWriteTokens, unknown };
}

export function usageTooltip(
  usage: Usage | null | undefined,
  durationApiMs?: number | null,
): string {
  if (!usage) return "Usage unknown: the agent ended without a result event";
  const lines = [
    `input ${(usage.inputTokens ?? 0).toLocaleString()}`,
    `output ${(usage.outputTokens ?? 0).toLocaleString()}`,
    `cache read ${(usage.cacheReadTokens ?? 0).toLocaleString()}`,
    `cache write ${(usage.cacheWriteTokens ?? 0).toLocaleString()}`,
  ];
  if (durationApiMs != null) {
    lines.push(`api time ${Math.round(durationApiMs / 1000)}s`);
  }
  return lines.join("\n");
}

export function toolSummary(input: Record<string, unknown>): string {
  for (const key of [
    "command",
    "path",
    "pattern",
    "glob_pattern",
    "description",
    "query",
    "toolName",
  ]) {
    const v = input[key];
    if (typeof v === "string" && v) return v.split("\n")[0] ?? "";
  }
  const first = Object.values(input).find((v) => typeof v === "string") as
    | string
    | undefined;
  return first?.split("\n")[0] ?? "";
}
