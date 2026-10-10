"use client";

import { useEffect, useState } from "react";

/** Page-wide wall clock for RunBadge elapsed minutes (one interval for all mounts). */
let sharedNow = Date.now();
const listeners = new Set<() => void>();
let intervalId: ReturnType<typeof setInterval> | null = null;
let intervalMsActive = 30_000;

function ensureTicker(intervalMs: number) {
  if (typeof window === "undefined") return;
  if (intervalId != null && intervalMsActive === intervalMs) return;
  if (intervalId != null) {
    clearInterval(intervalId);
    intervalId = null;
  }
  intervalMsActive = intervalMs;
  intervalId = setInterval(() => {
    sharedNow = Date.now();
    for (const listener of listeners) listener();
  }, intervalMs);
}

/** Shared wall-clock tick for RunBadge elapsed minutes (default 30s, one page timer). */
export function useNow(intervalMs = 30_000): number {
  const [now, setNow] = useState(() => sharedNow);
  useEffect(() => {
    ensureTicker(intervalMs);
    const listener = () => setNow(sharedNow);
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
      if (listeners.size === 0 && intervalId != null) {
        clearInterval(intervalId);
        intervalId = null;
      }
    };
  }, [intervalMs]);
  return now;
}
