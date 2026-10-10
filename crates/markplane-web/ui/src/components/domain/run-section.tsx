"use client";

import { useState } from "react";
import { ScrollText } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useRunRecord } from "@/lib/hooks/use-runs";
import { SessionLogModal } from "./session-log-modal";

/**
 * Minimal Run section for the Task detail panel.
 * Shown when this machine has a Run record, or when assignee is `dispatcher`
 * with no local record (grey hint). Action slot: View sessions only.
 */
export function RunSection({
  taskId,
  assignee,
}: {
  taskId: string;
  assignee: string | null;
}) {
  const { data: record, isLoading } = useRunRecord(taskId);
  const [open, setOpen] = useState(false);

  const hasRecord = !!record && record.runs.length > 0;
  const dispatcherElsewhere = assignee === "dispatcher" && !hasRecord;

  if (isLoading) return null;
  if (!hasRecord && !dispatcherElsewhere) return null;

  return (
    <div className="space-y-1">
      <div className="flex items-center gap-2">
        {hasRecord ? (
          <span className="text-sm font-medium">Run</span>
        ) : (
          <span className="text-xs text-muted-foreground">
            No local Run record — it ran on another machine
          </span>
        )}
        <div className="flex-1" />
        <Button
          variant="outline"
          size="sm"
          className="h-7 gap-1.5 text-xs"
          onClick={() => setOpen(true)}
        >
          <ScrollText className="size-3.5" />
          View sessions
        </Button>
      </div>
      <SessionLogModal
        taskId={taskId}
        record={hasRecord ? record : null}
        open={open}
        onOpenChange={setOpen}
      />
    </div>
  );
}
