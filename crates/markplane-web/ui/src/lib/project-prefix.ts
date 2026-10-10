import { useCallback, useSyncExternalStore } from "react";

const noopSubscribe = () => () => {};

/**
 * Static export prerenders without a Project id, and React keeps the stale
 * prerendered href attribute after hydration. The server snapshot (null)
 * matches the prerender; the client snapshot re-renders with the real id.
 */
export function useProjectId(): string | null {
  return useSyncExternalStore(noopSubscribe, currentProjectId, () => null);
}

export function useWithProject(): (path: string) => string {
  const id = useProjectId();
  return useCallback(
    (path: string) =>
      !id || !path.startsWith("/") || path.startsWith("/p/")
        ? path
        : `/p/${encodeURIComponent(id)}${path}`,
    [id],
  );
}

export function currentProjectId(): string | null {
  if (typeof window === "undefined") return null;
  const m = window.location.pathname.match(/^\/p\/([^/]+)/);
  return m ? decodeURIComponent(m[1]) : null;
}

export function withProject(path: string): string {
  const id = currentProjectId();
  if (!id || !path.startsWith("/") || path.startsWith("/p/")) return path;
  return `/p/${encodeURIComponent(id)}${path}`;
}

const GLOBAL_API_PREFIXES = ["/api/docs", "/api/projects", "/api/fs"];

export function apiPath(path: string): string {
  if (GLOBAL_API_PREFIXES.some((p) => path === p || path.startsWith(`${p}/`))) {
    return path;
  }
  const id = currentProjectId();
  if (!id || !path.startsWith("/api/")) return path;
  return `/api/p/${encodeURIComponent(id)}/${path.slice("/api/".length)}`;
}

/** Strip /p/<id> so existing `pathname.startsWith("/backlog")` checks keep working. */
export function stripProject(pathname: string): string {
  return pathname.replace(/^\/p\/[^/]+/, "") || "/";
}
