"use client";

import type { ReactNode } from "react";
import { usePathname } from "next/navigation";
import { AppSidebar } from "@/components/layout/app-sidebar";
import { CommandPaletteWrapper } from "@/components/layout/command-palette-wrapper";
import { GlobalCreateDialog } from "@/components/layout/global-create-dialog";
import { ErrorBoundary } from "@/components/domain/error-boundary";
import { MobileHeader } from "@/components/layout/mobile-header";
import { ProjectGate } from "@/components/layout/project-gate";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { projectIdFromPathname } from "@/lib/project-prefix";

export function AppShell({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const projectId = projectIdFromPathname(pathname);
  const onLanding = !projectId && (pathname === "/" || pathname === "");

  if (onLanding) {
    return (
      <main className="min-h-svh">
        <ErrorBoundary>{children}</ErrorBoundary>
      </main>
    );
  }

  return (
    <SidebarProvider>
      <AppSidebar />
      <SidebarInset>
        <MobileHeader />
        <main className="flex-1">
          <ErrorBoundary>
            <ProjectGate>{children}</ProjectGate>
          </ErrorBoundary>
        </main>
      </SidebarInset>
      <CommandPaletteWrapper />
      <GlobalCreateDialog />
    </SidebarProvider>
  );
}
