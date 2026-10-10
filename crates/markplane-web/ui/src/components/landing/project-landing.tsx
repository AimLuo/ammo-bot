"use client";

import { useState } from "react";
import { FolderPlus, MoreHorizontal, Pencil, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import {
  useProjects,
  useRemoveProject,
  useRenameProject,
  type RegistryProject,
} from "@/lib/hooks/use-projects";
import { projectHref } from "@/lib/project-prefix";

function UnavailableDot() {
  return (
    <span
      className="inline-block size-2 shrink-0 rounded-full bg-destructive"
      title="Path unavailable"
      aria-label="Path unavailable"
    />
  );
}

function ProjectCard({
  project,
  onChanged,
}: {
  project: RegistryProject;
  onChanged: () => void;
}) {
  const [renaming, setRenaming] = useState(false);
  const [name, setName] = useState(project.name);
  const rename = useRenameProject();
  const remove = useRemoveProject();

  async function commitRename() {
    const trimmed = name.trim();
    if (!trimmed || trimmed === project.name) {
      setRenaming(false);
      setName(project.name);
      return;
    }
    await rename.mutateAsync({ id: project.id, name: trimmed });
    setRenaming(false);
    onChanged();
  }

  async function confirmRemove() {
    if (
      !window.confirm(
        `Remove “${project.name}” from the registry?\n\nProject files on disk will not be deleted.`,
      )
    ) {
      return;
    }
    await remove.mutateAsync(project.id);
    onChanged();
  }

  return (
    <div className="relative rounded-xl border bg-card text-card-foreground shadow-sm">
      <div className="px-4 py-3 pr-12">
        <div className="flex items-center gap-2 min-w-0">
          {!project.path_valid && <UnavailableDot />}
          {renaming ? (
            <Input
              autoFocus
              value={name}
              onChange={(e) => setName(e.target.value)}
              onBlur={() => void commitRename()}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  void commitRename();
                }
                if (e.key === "Escape") {
                  e.preventDefault();
                  setRenaming(false);
                  setName(project.name);
                }
              }}
              className="h-8"
            />
          ) : (
            <a
              href={projectHref(project.id)}
              className="font-medium truncate hover:underline"
            >
              {project.name}
            </a>
          )}
        </div>
        <a
          href={projectHref(project.id)}
          className="mt-1 block text-sm text-muted-foreground font-mono truncate hover:text-foreground"
        >
          {project.path}
        </a>
      </div>
      <div className="absolute right-2 top-2">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label={`Manage ${project.name}`}
            >
              <MoreHorizontal className="size-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuItem
              onSelect={() => {
                setName(project.name);
                setRenaming(true);
              }}
            >
              <Pencil className="size-4" />
              Rename
            </DropdownMenuItem>
            <DropdownMenuItem variant="destructive" onSelect={() => void confirmRemove()}>
              <Trash2 className="size-4" />
              Remove
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </div>
  );
}

function AddFirstPlaceholder() {
  return (
    <div
      className="flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed px-6 py-12 text-center"
      data-testid="add-first-project"
    >
      <FolderPlus className="size-8 text-muted-foreground/50" />
      <div className="space-y-1">
        <p className="text-sm font-medium">添加第一个 Project</p>
        <p className="text-xs text-muted-foreground max-w-sm">
          添加目录浏览将在后续提供。也可在 Project 目录里运行{" "}
          <code className="font-mono">markplane serve</code> 自动注册。
        </p>
      </div>
      <Button type="button" variant="outline" size="sm" disabled>
        添加 Project
      </Button>
    </div>
  );
}

export function ProjectLanding() {
  const { data, error, isLoading, refetch } = useProjects();
  const projects = data?.data ?? [];

  return (
    <div className="mx-auto max-w-2xl p-8">
      <h1 className="text-2xl font-semibold tracking-tight font-mono">Markplane</h1>
      <p className="mt-2 text-muted-foreground">Choose a Project</p>
      {error && (
        <p className="mt-4 text-destructive">
          {error instanceof Error ? error.message : "Failed to load projects"}
        </p>
      )}
      {isLoading && !error && (
        <p className="mt-4 text-muted-foreground">Loading…</p>
      )}
      {!isLoading && !error && projects.length === 0 && (
        <div className="mt-6">
          <AddFirstPlaceholder />
        </div>
      )}
      {!isLoading && projects.length > 0 && (
        <ul className="mt-6 space-y-3">
          {projects.map((p) => (
            <li key={p.id}>
              <ProjectCard project={p} onChanged={() => void refetch()} />
            </li>
          ))}
          <li>
            <div className="rounded-xl border border-dashed px-4 py-3 text-sm text-muted-foreground">
              添加 Project（即将提供）
            </div>
          </li>
        </ul>
      )}
    </div>
  );
}
