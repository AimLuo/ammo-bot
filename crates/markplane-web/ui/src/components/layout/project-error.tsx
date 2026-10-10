"use client";

import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";

export function ProjectErrorPage({
  code,
  message,
  path,
}: {
  code: "project_not_found" | "project_unavailable";
  message: string;
  path?: string | null;
}) {
  const title =
    code === "project_not_found" ? "Project not found" : "Project unavailable";

  return (
    <div className="flex min-h-[60vh] flex-col items-center justify-center gap-4 p-8 text-center">
      <AlertTriangle className="size-10 text-destructive" />
      <div className="space-y-2 max-w-md">
        <h1 className="text-xl font-semibold tracking-tight">{title}</h1>
        <p className="text-sm text-muted-foreground">{message}</p>
        {path && (
          <p className="text-xs font-mono text-muted-foreground break-all">{path}</p>
        )}
      </div>
      <Button asChild>
        <a href="/">返回 Project 列表</a>
      </Button>
    </div>
  );
}
