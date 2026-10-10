"use client";

import { useEffect, useState, type ReactNode } from "react";
import { usePathname } from "next/navigation";
import { ApiError } from "@/lib/api-error";
import { fetcher } from "@/lib/api";
import { projectIdFromPathname } from "@/lib/project-prefix";
import { ProjectErrorPage } from "@/components/layout/project-error";

type GateState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "ok" }
  | {
      status: "error";
      code: "project_not_found" | "project_unavailable";
      message: string;
      path?: string | null;
    };

/**
 * When a Project id is in the URL, probe the API once so 404/409 become a
 * full-page recovery UI instead of a broken shell.
 */
export function ProjectGate({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const projectId = projectIdFromPathname(pathname);
  const [state, setState] = useState<GateState>({ status: "idle" });

  useEffect(() => {
    if (!projectId) {
      setState({ status: "idle" });
      return;
    }

    let cancelled = false;
    setState({ status: "loading" });

    fetcher<{ name: string }>("/api/summary")
      .then(() => {
        if (!cancelled) setState({ status: "ok" });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        if (err instanceof ApiError && err.isProjectNotFound) {
          setState({
            status: "error",
            code: "project_not_found",
            message: err.message,
            path: err.path,
          });
          return;
        }
        if (err instanceof ApiError && err.isProjectUnavailable) {
          setState({
            status: "error",
            code: "project_unavailable",
            message: err.message,
            path: err.path,
          });
          return;
        }
        // Other errors: let the normal page/error boundary handle them.
        setState({ status: "ok" });
      });

    return () => {
      cancelled = true;
    };
  }, [projectId]);

  if (!projectId) {
    return <>{children}</>;
  }

  if (state.status === "loading" || state.status === "idle") {
    return (
      <div className="flex min-h-[40vh] items-center justify-center p-8 text-muted-foreground">
        Loading project…
      </div>
    );
  }

  if (state.status === "error") {
    return (
      <ProjectErrorPage
        code={state.code}
        message={state.message}
        path={state.path}
      />
    );
  }

  return <>{children}</>;
}
