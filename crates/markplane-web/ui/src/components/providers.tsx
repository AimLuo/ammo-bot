"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ThemeProvider } from "next-themes";
import { TooltipProvider } from "@/components/ui/tooltip";
import { useState, type ReactNode } from "react";
import { usePathname } from "next/navigation";
import { useWebSocket } from "@/lib/hooks/use-websocket";
import { useKeyboardNav } from "@/lib/hooks/use-keyboard-nav";
import { projectIdFromPathname } from "@/lib/project-prefix";

function WebSocketProvider({ children }: { children: ReactNode }) {
  useWebSocket();
  return <>{children}</>;
}

function KeyboardNavProvider({ children }: { children: ReactNode }) {
  useKeyboardNav();
  return <>{children}</>;
}

function ScopedProviders({ children }: { children: ReactNode }) {
  const [queryClient] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 30_000,
            gcTime: 5 * 60_000,
            refetchOnWindowFocus: false,
            retry: 1,
          },
        },
      }),
  );

  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider attribute="class" defaultTheme="dark" enableSystem>
        <TooltipProvider>
          <WebSocketProvider>
            <KeyboardNavProvider>{children}</KeyboardNavProvider>
          </WebSocketProvider>
        </TooltipProvider>
      </ThemeProvider>
    </QueryClientProvider>
  );
}

/**
 * Remount the app providers (and clear the QueryClient) whenever the Project
 * in the URL changes, so Project A data never bleeds into Project B.
 */
export function Providers({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const scopeKey = projectIdFromPathname(pathname) ?? "__landing__";
  return <ScopedProviders key={scopeKey}>{children}</ScopedProviders>;
}
