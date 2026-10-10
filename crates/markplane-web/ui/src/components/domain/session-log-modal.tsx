"use client";

import { useMemo, useState } from "react";
import { Ban, ChevronRight, TriangleAlert } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import type { Attempt, Run, RunRecord, Stage, Trigger } from "@/lib/types";

const STAGE_ORDER: Stage[] = ["implement", "fix", "merge"];

const TRIGGER_STYLE: Record<Trigger, string> = {
  initial: "bg-muted text-muted-foreground",
  resume: "bg-sky-500/15 text-sky-600 dark:text-sky-400",
  continue: "bg-violet-500/15 text-violet-600 dark:text-violet-400",
};

type Selection = {
  runSeq: number;
  sessionId: string;
  attemptIndex?: number;
};

interface SessionGroup {
  sessionId: string;
  stage: Stage;
  attempts: { attempt: Attempt; index: number }[];
}

function groupSessions(run: Run): SessionGroup[] {
  const groups: SessionGroup[] = [];
  run.attempts.forEach((attempt, index) => {
    const sessionId = attempt.session_id ?? `pending-${index}`;
    let group = groups.find((g) => g.sessionId === sessionId && g.stage === attempt.stage);
    if (!group) {
      group = {
        sessionId,
        stage: attempt.stage,
        attempts: [],
      };
      groups.push(group);
    }
    group.attempts.push({ attempt, index });
  });
  return groups;
}

function shortId(id: string): string {
  return id.length > 8 ? id.slice(0, 8) : id;
}

function TriggerBadge({ trigger }: { trigger: Trigger }) {
  return (
    <span
      className={cn(
        "rounded px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wide",
        TRIGGER_STYLE[trigger],
      )}
    >
      {trigger}
    </span>
  );
}

function MismatchFlag({ attempt }: { attempt: Attempt }) {
  if (attempt.result !== "resume_mismatch") return null;
  return (
    <span
      className="inline-flex items-center gap-1 rounded bg-amber-500/15 px-1.5 py-0.5 font-mono text-[10px] text-amber-600 dark:text-amber-400"
      title="Resume started a different session; it was killed"
    >
      <TriangleAlert className="size-3" />
      resume_mismatch
    </span>
  );
}

function EmptyRecordState({ taskId }: { taskId: string }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-3 p-10 text-center">
      <Ban className="size-8 text-muted-foreground/60" />
      <div className="text-sm font-medium">本机没有 Run 记录</div>
      <p className="max-w-md text-xs text-muted-foreground">
        Nothing in{" "}
        <code className="font-mono">.markplane/.runs/{taskId}.json</code> on
        this machine. The ticket never ran here, or it ran on another machine.
      </p>
    </div>
  );
}

function RunNavBlock({
  run,
  active,
  onSelect,
}: {
  run: Run;
  active: Selection | null;
  onSelect: (s: Selection) => void;
}) {
  const groups = groupSessions(run);

  return (
    <div className="mb-3">
      <div className="px-4 py-1.5">
        <span className="text-sm font-semibold">Run #{run.seq}</span>
      </div>
      {STAGE_ORDER.map((stage) => {
        const inStage = groups.filter((g) => g.stage === stage);
        if (inStage.length === 0) return null;
        return (
          <div key={stage} className="pl-4">
            <div className="px-2 pt-1 text-[10px] font-medium uppercase tracking-wider text-muted-foreground">
              {stage}
            </div>
            {inStage.map((group) => {
              const isActive =
                active?.runSeq === run.seq &&
                active.sessionId === group.sessionId;
              return (
                <div
                  key={`${group.stage}:${group.sessionId}`}
                  className={cn("mr-2 rounded-md", isActive && "bg-accent")}
                >
                  <button
                    type="button"
                    onClick={() =>
                      onSelect({
                        runSeq: run.seq,
                        sessionId: group.sessionId,
                      })
                    }
                    className="flex w-full cursor-pointer items-center gap-2 px-2 py-1 text-left text-xs hover:bg-accent/60"
                  >
                    <ChevronRight
                      className={cn(
                        "size-3 shrink-0 transition-transform",
                        isActive && "rotate-90",
                      )}
                    />
                    <span className="truncate">
                      {stage} · {shortId(group.sessionId)}
                    </span>
                  </button>
                  {group.attempts.map(({ attempt, index }) => (
                    <button
                      key={index}
                      type="button"
                      onClick={() =>
                        onSelect({
                          runSeq: run.seq,
                          sessionId: group.sessionId,
                          attemptIndex: index,
                        })
                      }
                      className={cn(
                        "flex w-full cursor-pointer items-center gap-1.5 py-0.5 pl-7 pr-2 text-left hover:bg-accent/60",
                        isActive &&
                          active?.attemptIndex === index &&
                          "bg-accent/80",
                      )}
                    >
                      <TriggerBadge trigger={attempt.trigger} />
                      <MismatchFlag attempt={attempt} />
                    </button>
                  ))}
                </div>
              );
            })}
          </div>
        );
      })}
    </div>
  );
}

export function SessionLogModal({
  taskId,
  record,
  open,
  onOpenChange,
}: {
  taskId: string;
  record: RunRecord | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const runs = useMemo(
    () => (record ? [...record.runs].reverse() : []),
    [record],
  );
  const [selection, setSelection] = useState<Selection | null>(null);

  const fallback: Selection | null = runs[0]
    ? (() => {
        const groups = groupSessions(runs[0]);
        const last = groups.at(-1);
        return last
          ? { runSeq: runs[0].seq, sessionId: last.sessionId }
          : null;
      })()
    : null;
  const active = selection ?? fallback;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="grid h-[85vh] max-w-[min(1200px,95vw)] grid-rows-[auto_1fr] gap-0 p-0 sm:max-w-[min(1200px,95vw)]">
        <header className="flex items-center gap-3 border-b px-5 py-3 pr-12">
          <DialogTitle className="text-base">Sessions · {taskId}</DialogTitle>
          <span className="font-mono text-[11px] text-muted-foreground">
            .markplane/.runs/{taskId}.json
          </span>
        </header>

        {runs.length === 0 ? (
          <EmptyRecordState taskId={taskId} />
        ) : (
          <div className="grid min-h-0 grid-cols-[320px_1fr]">
            <nav className="min-h-0 overflow-y-auto border-r bg-muted/20 py-2">
              {runs.map((run) => (
                <RunNavBlock
                  key={run.seq}
                  run={run}
                  active={active}
                  onSelect={setSelection}
                />
              ))}
            </nav>
            <section className="flex min-h-0 flex-col items-center justify-center gap-2 p-8 text-center">
              {active ? (
                <>
                  <p className="text-sm font-medium">Session selected</p>
                  <p className="max-w-sm text-xs text-muted-foreground">
                    Transcript for{" "}
                    <code className="font-mono">{shortId(active.sessionId)}</code>{" "}
                    will appear here in a later ticket.
                  </p>
                </>
              ) : (
                <p className="text-xs text-muted-foreground">
                  Select a session from the list.
                </p>
              )}
            </section>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
