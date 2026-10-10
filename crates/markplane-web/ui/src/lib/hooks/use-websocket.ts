"use client";

import { useEffect, useRef } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { currentProjectId, withProject } from "@/lib/project-prefix";
import type { WsEvent } from "@/lib/types";

function websocketUrl(projectId: string): string {
  const prefixed = `/api/p/${encodeURIComponent(projectId)}/ws`;
  const configured = process.env.NEXT_PUBLIC_WS_URL;
  if (configured) {
    // Dev: NEXT_PUBLIC_WS_URL is ws://localhost:4200/ws — splice in the Project prefix.
    return configured.replace(/\/ws$/, prefixed);
  }
  const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${protocol}//${window.location.host}${prefixed}`;
}

export function useWebSocket() {
  const queryClient = useQueryClient();
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout>>(null);
  const hasConnectedBefore = useRef(false);

  useEffect(() => {
    function connect() {
      const projectId = currentProjectId();
      if (!projectId) {
        return;
      }
      const url = websocketUrl(projectId);
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        // Clear reconnect timer on successful connection
        if (reconnectTimer.current) {
          clearTimeout(reconnectTimer.current);
          reconnectTimer.current = null;
        }
        // Reconnect (not first open): refresh everything that may have drifted.
        if (hasConnectedBefore.current) {
          queryClient.invalidateQueries();
        }
        hasConnectedBefore.current = true;
      };

      ws.onmessage = (event) => {
        try {
          const msg: WsEvent = JSON.parse(event.data);

          switch (msg.type) {
            case "file_changed": {
              const queryPrefix = msg.entity + "s"; // "task" → "tasks"
              queryClient.invalidateQueries({ queryKey: [queryPrefix] });
              queryClient.invalidateQueries({ queryKey: ["summary"] });
              queryClient.invalidateQueries({ queryKey: ["graph"] });
              // Decision tickets live under maps; refresh route graphs on task edits.
              if (msg.entity === "task") {
                queryClient.invalidateQueries({ queryKey: ["maps"] });
              }
              break;
            }
            case "config_changed":
              queryClient.invalidateQueries({ queryKey: ["summary"] });
              queryClient.invalidateQueries({ queryKey: ["config"] });
              break;
            case "sync_complete":
              queryClient.invalidateQueries();
              break;
            case "doc_changed":
              queryClient.invalidateQueries({ queryKey: ["docs"] });
              break;
            case "dispatcher_changed":
              queryClient.invalidateQueries({ queryKey: ["dispatcher"] });
              queryClient.invalidateQueries({
                queryKey: ["dispatcher-concurrency"],
              });
              break;
            case "run_changed": {
              queryClient.invalidateQueries({ queryKey: ["runs"] });
              if (msg.status === "failed") {
                const stage = msg.stage || "stage";
                const taskId = msg.task_id;
                toast.error(`${taskId} run failed at ${stage}`, {
                  action: {
                    label: "Open",
                    onClick: () => {
                      window.location.assign(
                        withProject(
                          `/backlog?task=${encodeURIComponent(taskId)}`,
                        ),
                      );
                    },
                  },
                });
              }
              break;
            }
            case "connected":
              break;
          }
        } catch {
          // Ignore malformed messages
        }
      };

      ws.onclose = () => {
        wsRef.current = null;
        // Reconnect after 2 seconds
        reconnectTimer.current = setTimeout(connect, 2000);
      };

      ws.onerror = () => {
        ws.close();
      };
    }

    connect();

    return () => {
      if (reconnectTimer.current) {
        clearTimeout(reconnectTimer.current);
      }
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, [queryClient]);
}
