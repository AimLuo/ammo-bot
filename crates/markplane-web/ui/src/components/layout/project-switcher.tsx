"use client";

import { ChevronsUpDown, FolderKanban, LayoutGrid } from "lucide-react";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar";
import { useProjects } from "@/lib/hooks/use-projects";
import { projectHref, projectIdFromPathname } from "@/lib/project-prefix";
import { usePathname } from "next/navigation";

export function ProjectSwitcher() {
  const pathname = usePathname();
  const currentId = projectIdFromPathname(pathname);
  const { data } = useProjects();
  const projects = data?.data ?? [];
  const { isMobile } = useSidebar();

  const current = projects.find((p) => p.id === currentId);
  const label = current?.name ?? (currentId ? currentId : "所有 Project");

  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <SidebarMenuButton
              size="lg"
              className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
              tooltip={label}
            >
              <div className="flex aspect-square size-8 items-center justify-center rounded-md bg-sidebar-primary text-sidebar-primary-foreground">
                <FolderKanban className="size-4" />
              </div>
              <div className="grid flex-1 text-left text-sm leading-tight">
                <span className="truncate font-medium">{label}</span>
                <span className="truncate text-xs text-muted-foreground">
                  {current?.path ?? "Project list"}
                </span>
              </div>
              <ChevronsUpDown className="ml-auto size-4" />
            </SidebarMenuButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            className="w-(--radix-dropdown-menu-trigger-width) min-w-56 rounded-lg"
            align="start"
            side={isMobile ? "bottom" : "right"}
            sideOffset={4}
          >
            <DropdownMenuLabel className="text-xs text-muted-foreground">
              Projects
            </DropdownMenuLabel>
            <DropdownMenuItem asChild>
              <a href="/">
                <LayoutGrid className="size-4" />
                所有 Project
              </a>
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            {projects.map((p) => (
              <DropdownMenuItem key={p.id} asChild>
                <a href={projectHref(p.id)}>
                  <span className="flex items-center gap-2 min-w-0">
                    {!p.path_valid && (
                      <span
                        className="inline-block size-2 shrink-0 rounded-full bg-destructive"
                        aria-hidden
                      />
                    )}
                    <span className="truncate">{p.name}</span>
                  </span>
                </a>
              </DropdownMenuItem>
            ))}
            {projects.length === 0 && (
              <DropdownMenuItem disabled>No projects registered</DropdownMenuItem>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
    </SidebarMenu>
  );
}
