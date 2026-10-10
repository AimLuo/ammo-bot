"use client";

import type { ReactNode } from "react";
import { usePathname } from "next/navigation";
import { useQuery } from "@tanstack/react-query";
import { ApiError } from "@/lib/api-error";
import { fetcher } from "@/lib/api";
import { projectIdFromPathname } from "@/lib/project-prefix";
import { ProjectErrorPage } from "@/components/layout/project-error";

/**
 * When a Project id is in the URL, probe the API once so 404/409 become a
 * full-page recovery UI instead of a broken shell.
 */
export function ProjectGate({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const projectId = projectIdFromPathname(pathname);

  const probe = useQuery({
    queryKey: ["project-gate", projectId],
    queryFn: () => fetcher<{ name: string }>("/api/summary"),
    enabled: !!projectId,
    retry: false,
    staleTime: 30_000,
  });

  if (!projectId) {
    return <>{children}</>;
  }

  if (probe.isPending) {
    return (
      <div className="flex min-h-[40vh] items-center justify-center p-8 text-muted-foreground">
        Loading project…
      </div>
    );
  }

  if (probe.error instanceof ApiError && probe.error.isProjectNotFound) {
    return (
      <ProjectErrorPage
        code="project_not_found"
        message={probe.error.message}
        path={probe.error.path}
      />
    );
  }

  if (probe.error instanceof ApiError && probe.error.isProjectUnavailable) {
    return (
      <ProjectErrorPage
        code="project_unavailable"
        message={probe.error.message}
        path={probe.error.path}
      />
    );
  }

  return <>{children}</>;
}
