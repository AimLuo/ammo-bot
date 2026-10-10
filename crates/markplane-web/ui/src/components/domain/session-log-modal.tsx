"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import {
  Ban,
  ChevronRight,
  CircleCheck,
  CirclePause,
  CircleX,
  Clock,
  FileX,
  LoaderCircle,
  RefreshCw,
  Terminal,
  TriangleAlert,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from "@/components/ui/dialog";
import { MarkdownRenderer } from "@/components/domain/markdown-renderer";
import { cn } from "@/lib/utils";
import {
  fmtClock,
  fmtDuration,
  fmtTime,
  fmtTokens,
  sumUsage,
  toolSummary,
  usageTooltip,
} from "@/lib/session-log";
import {
  useRefreshSessionLog,
  useSessionTranscript,
} from "@/lib/hooks/use-session-transcript";
import type {
  Attempt,
  AttemptResult,
  Run,
  RunRecord,
  SessionTranscript,
  Stage,
  TranscriptMessage,
  Trigger,
} from "@/lib/types";

const STAGE_ORDER: Stage[] = ["implement", "fix", "merge"];
const PAGE = 60;

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
  model: string;
  workspace: string;
  attempts: { attempt: Attempt; index: number }[];
}

function groupSessions(run: Run): SessionGroup[] {
  const groups: SessionGroup[] = [];
  run.attempts.forEach((attempt, index) => {
    const sessionId = attempt.session_id ?? `pending-${index}`;
    let group = groups.find(
      (g) => g.sessionId === sessionId && g.stage === attempt.stage,
    );
    if (!group) {
      group = {
        sessionId,
        stage: attempt.stage,
        model: attempt.model,
        workspace: attempt.workspace,
        attempts: [],
      };
      groups.push(group);
    }
    group.attempts.push({ attempt, index });
  });
  return groups;
}

function sessionTitle(taskId: string, run: Run, g: SessionGroup): string {
  const sameStage = groupSessions(run).filter((x) => x.stage === g.stage);
  const pos = sameStage.findIndex((x) => x.sessionId === g.sessionId) + 1;
  const n = sameStage.length > 1 ? ` (${pos}/${sameStage.length})` : "";
  return `${taskId} ${g.stage} · Run #${run.seq}${n}`;
}

/** Message range each live invocation contributed (k-th user prompt → k-th attempt). */
function invocationRanges(
  g: SessionGroup,
  messages: TranscriptMessage[],
): Map<number, [number, number]> {
  const starts = messages.filter((m) => m.role === "user").map((m) => m.idx);
  const live = g.attempts.filter((a) => a.attempt.result !== "resume_mismatch");
  const out = new Map<number, [number, number]>();
  live.forEach((a, k) => {
    if (starts[k] === undefined) return;
    out.set(a.index, [starts[k]!, starts[k + 1] ?? messages.length]);
  });
  return out;
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

function ResultIcon({ result }: { result?: AttemptResult | null }) {
  const c = "size-3.5 shrink-0";
  switch (result) {
    case "ok":
      return <CircleCheck className={cn(c, "text-emerald-500")} />;
    case "failed":
      return <CircleX className={cn(c, "text-destructive")} />;
    case "timeout":
      return <Clock className={cn(c, "text-destructive")} />;
    case "interrupted":
      return <CirclePause className={cn(c, "text-amber-500")} />;
    case "resume_mismatch":
      return <TriangleAlert className={cn(c, "text-amber-500")} />;
    case undefined:
    case null:
      return (
        <LoaderCircle className={cn(c, "animate-spin text-muted-foreground")} />
      );
    default:
      return null;
  }
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

function UsageInline({
  attempt,
  className,
}: {
  attempt: Attempt;
  className?: string;
}) {
  const u = attempt.usage;
  return (
    <span
      className={cn(
        "font-mono text-[11px] text-muted-foreground tabular-nums",
        className,
      )}
      title={usageTooltip(u, attempt.duration_api_ms)}
    >
      {u
        ? `${fmtTokens(u.inputTokens ?? 0)} in · ${fmtTokens(u.outputTokens ?? 0)} out`
        : attempt.result == null
          ? "…"
          : "—"}
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

function TranscriptUnavailable({
  transcript,
}: {
  transcript: Extract<SessionTranscript, { status: "unavailable" }>;
}) {
  return (
    <div className="flex flex-col items-start gap-2 rounded-md border border-dashed p-4 text-xs">
      <div className="flex items-center gap-2 text-sm font-medium">
        <FileX className="size-4 text-muted-foreground" />
        会话记录不可用
      </div>
      <p className="text-muted-foreground">{transcript.reason}</p>
      {transcript.path ? (
        <code className="break-all font-mono text-[11px] text-muted-foreground">
          {transcript.path}
        </code>
      ) : null}
    </div>
  );
}

function RunNavBlock({
  taskId,
  run,
  active,
  onSelect,
}: {
  taskId: string;
  run: Run;
  active: Selection | null;
  onSelect: (s: Selection) => void;
}) {
  const groups = groupSessions(run);
  const total = sumUsage(run.attempts);

  return (
    <div className="mb-3">
      <div className="flex items-baseline gap-2 px-4 py-1.5">
        <span className="text-sm font-semibold">Run #{run.seq}</span>
        <span
          className="ml-auto font-mono text-[11px] text-muted-foreground"
          title="Run total (≥ when some invocations have unknown usage)"
        >
          {total.unknown ? "≥" : ""}
          {fmtTokens(total.inputTokens)} / {fmtTokens(total.outputTokens)}
        </span>
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
              const sessionTotal = sumUsage(
                group.attempts.map((a) => a.attempt),
              );
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
                      {sessionTitle(taskId, run, group)}
                    </span>
                    <span
                      className="ml-auto shrink-0 font-mono text-[10px] text-muted-foreground"
                      title="Session total"
                    >
                      {sessionTotal.unknown ? "≥" : ""}
                      {fmtTokens(sessionTotal.inputTokens)}/
                      {fmtTokens(sessionTotal.outputTokens)}
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
                      <ResultIcon result={attempt.result} />
                      <TriggerBadge trigger={attempt.trigger} />
                      <MismatchFlag attempt={attempt} />
                      <span className="text-[11px] text-muted-foreground">
                        {fmtDuration(attempt.started_at, attempt.ended_at)}
                      </span>
                      <UsageInline attempt={attempt} className="ml-auto" />
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

/** One expandable line for all tool_use blocks in an assistant turn. */
function ToolTurnRow({
  tools,
}: {
  tools: { name: string; input: Record<string, unknown> }[];
}) {
  const [open, setOpen] = useState(false);
  if (tools.length === 0) return null;
  const summary =
    tools.length === 1
      ? `${tools[0]!.name} ${toolSummary(tools[0]!.input)}`.trim()
      : `${tools.length} tools · ${tools.map((t) => t.name).join(", ")}`;
  return (
    <div className="font-mono text-[11px]">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex w-full cursor-pointer items-center gap-1.5 text-left text-muted-foreground hover:text-foreground"
      >
        <Terminal className="size-3 shrink-0" />
        <span className="truncate font-semibold text-foreground/80">
          {summary}
        </span>
      </button>
      {open && (
        <div className="mt-1 space-y-2">
          {tools.map((t, i) => (
            <div key={i}>
              <div className="text-foreground/80">{t.name}</div>
              <pre className="max-h-60 overflow-auto rounded bg-muted p-2 whitespace-pre-wrap break-all">
                {JSON.stringify(t.input, null, 2)}
              </pre>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function MessageRow({
  message,
  show,
}: {
  message: TranscriptMessage;
  show: { user: boolean; text: boolean; tools: boolean };
}) {
  const [open, setOpen] = useState(false);
  if (message.role === "user") {
    if (!show.user) return null;
    const text = message.blocks
      .map((b) => (b.type === "text" ? b.text : ""))
      .join("\n");
    const lines = text.split("\n");
    return (
      <div className="grid grid-cols-[64px_1fr] gap-2 rounded-md bg-sky-500/5 py-2 pr-2">
        <span className="pl-2 text-[10px] font-medium uppercase text-sky-600 dark:text-sky-400">
          prompt
        </span>
        <div className="min-w-0">
          <pre className="whitespace-pre-wrap break-words font-sans text-xs">
            {open ? text : lines.slice(0, 6).join("\n")}
          </pre>
          {lines.length > 6 && (
            <button
              type="button"
              className="mt-1 cursor-pointer text-[11px] text-muted-foreground underline"
              onClick={() => setOpen(!open)}
            >
              {open ? "collapse" : `+${lines.length - 6} lines`}
            </button>
          )}
        </div>
      </div>
    );
  }
  const textBlocks = message.blocks.filter(
    (b): b is Extract<typeof b, { type: "text" }> => b.type === "text",
  );
  const tools = message.blocks
    .filter(
      (b): b is Extract<typeof b, { type: "tool_use" }> => b.type === "tool_use",
    )
    .map((b) => ({ name: b.name, input: b.input ?? {} }));

  return (
    <div className="grid grid-cols-[64px_1fr] gap-2">
      <span className="pt-0.5 text-[10px] font-medium uppercase text-muted-foreground">
        agent
      </span>
      <div className="min-w-0 space-y-1">
        {show.text &&
          textBlocks.map((b, i) =>
            b.text.trim() ? (
              <div key={i} className="text-sm [&_.prose]:text-sm">
                <MarkdownRenderer content={b.text} />
              </div>
            ) : null,
          )}
        {show.tools && <ToolTurnRow tools={tools} />}
      </div>
    </div>
  );
}

function TranscriptPane({
  group,
  messages,
  focus,
  show,
}: {
  group: SessionGroup;
  messages: TranscriptMessage[];
  focus?: number;
  show: { user: boolean; text: boolean; tools: boolean };
}) {
  const ranges = useMemo(
    () => invocationRanges(group, messages),
    [group, messages],
  );
  const startOf = useMemo(() => {
    const m = new Map<number, number>();
    ranges.forEach(([start], attemptIndex) => m.set(start, attemptIndex));
    return m;
  }, [ranges]);
  const focusStart = focus !== undefined ? ranges.get(focus)?.[0] : undefined;

  const [from, setFrom] = useState(() => Math.max(0, messages.length - PAGE));
  const effectiveFrom =
    focusStart !== undefined ? Math.min(from, focusStart) : from;
  const dividerRefs = useRef(new Map<number, HTMLDivElement>());
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (focusStart !== undefined) {
      dividerRefs.current.get(focusStart)?.scrollIntoView({ block: "start" });
    } else {
      bottomRef.current?.scrollIntoView({ block: "end" });
    }
  }, [focusStart]);

  const attemptByIndex = new Map(
    group.attempts.map((a) => [a.index, a.attempt]),
  );

  return (
    <div className="space-y-1.5">
      {effectiveFrom > 0 && (
        <div className="flex items-center gap-2 pb-2 text-xs">
          <button
            type="button"
            className="cursor-pointer rounded border px-2 py-1 hover:bg-muted"
            onClick={() => setFrom(Math.max(0, effectiveFrom - PAGE))}
          >
            Show {Math.min(PAGE, effectiveFrom)} earlier
          </button>
          <button
            type="button"
            className="cursor-pointer text-muted-foreground underline"
            onClick={() => setFrom(0)}
          >
            show all ({effectiveFrom} hidden)
          </button>
        </div>
      )}
      {messages.slice(effectiveFrom).map((m) => {
        const attemptIndex = startOf.get(m.idx);
        const attempt =
          attemptIndex !== undefined
            ? attemptByIndex.get(attemptIndex)
            : undefined;
        return (
          <div key={m.idx}>
            {attempt && (
              <div
                ref={(el) => {
                  if (el) dividerRefs.current.set(m.idx, el);
                }}
                className={cn(
                  "my-3 flex scroll-mt-2 items-center gap-2 text-[11px] text-muted-foreground",
                  attemptIndex === focus && "text-foreground",
                )}
              >
                <div className="h-px flex-1 bg-border" />
                <ResultIcon result={attempt.result} />
                <TriggerBadge trigger={attempt.trigger} />
                <span>{fmtTime(attempt.started_at)}</span>
                {attempt.verdict && (
                  <span className="font-mono">{attempt.verdict}</span>
                )}
                <UsageInline attempt={attempt} />
                <div className="h-px flex-1 bg-border" />
              </div>
            )}
            <MessageRow message={m} show={show} />
          </div>
        );
      })}
      <div ref={bottomRef} />
    </div>
  );
}

function SessionPane({
  taskId,
  run,
  group,
  focus,
  open,
}: {
  taskId: string;
  run: Run;
  group: SessionGroup;
  focus?: number;
  open: boolean;
}) {
  const { data: transcript, isFetching } = useSessionTranscript(
    taskId,
    group.sessionId,
    { enabled: open },
  );
  const total = sumUsage(group.attempts.map((a) => a.attempt));
  const removed =
    !!run.worktree_removed_at && group.workspace === run.worktree;
  const [show, setShow] = useState({ user: true, text: true, tools: true });

  return (
    <div className="flex h-full min-h-0 flex-col">
      <div className="space-y-1 border-b px-5 py-3 text-xs">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">
            {sessionTitle(taskId, run, group)}
          </span>
          <code className="font-mono text-[11px] text-muted-foreground">
            {group.sessionId}
          </code>
          {isFetching && (
            <LoaderCircle className="size-3.5 animate-spin text-muted-foreground" />
          )}
        </div>
        <div className="flex flex-wrap gap-x-4 gap-y-1 text-muted-foreground">
          <span>model {group.model}</span>
          <span>
            cwd{" "}
            <span className={cn("font-mono", removed && "line-through")}>
              {group.workspace}
            </span>
            {removed && " (removed)"}
          </span>
          <span
            title={[
              `input ${total.inputTokens.toLocaleString()}`,
              `output ${total.outputTokens.toLocaleString()}`,
              `cache read ${total.cacheReadTokens.toLocaleString()}`,
              `cache write ${total.cacheWriteTokens.toLocaleString()}`,
            ].join("\n")}
          >
            Σ {total.unknown ? "≥" : ""}
            {fmtTokens(total.inputTokens)} in · {fmtTokens(total.outputTokens)}{" "}
            out
          </span>
          {transcript?.status === "ok" && (
            <span>
              {transcript.messages.length} messages ·{" "}
              {transcript.ended ? (
                `turn ${transcript.ended}`
              ) : (
                <span className="text-muted-foreground">still being written</span>
              )}
            </span>
          )}
        </div>
        {transcript?.status === "ok" && (
          <div className="flex items-center gap-1 pt-1">
            {(
              [
                ["user", "Prompts"],
                ["text", "Agent text"],
                ["tools", "Tool calls"],
              ] as const
            ).map(([k, label]) => (
              <button
                key={k}
                type="button"
                onClick={() => setShow((s) => ({ ...s, [k]: !s[k] }))}
                className={cn(
                  "cursor-pointer rounded-full border px-2 py-0.5 text-[11px]",
                  show[k]
                    ? "bg-foreground text-background"
                    : "text-muted-foreground",
                )}
              >
                {label}
              </button>
            ))}
          </div>
        )}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-5 py-3">
        {!transcript ? (
          <p className="text-xs text-muted-foreground">Loading transcript…</p>
        ) : transcript.status === "unavailable" ? (
          <TranscriptUnavailable transcript={transcript} />
        ) : (
          <TranscriptPane
            group={group}
            messages={transcript.messages}
            focus={focus}
            show={show}
          />
        )}
      </div>
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
  const [refreshing, setRefreshing] = useState(false);
  const [readAt, setReadAt] = useState(() => new Date());
  const refresh = useRefreshSessionLog(taskId);

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
  const activeRun = runs.find((r) => r.seq === active?.runSeq);
  const activeGroup = activeRun
    ? groupSessions(activeRun).find((g) => g.sessionId === active?.sessionId)
    : undefined;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="grid h-[85vh] max-w-[min(1200px,95vw)] grid-rows-[auto_1fr] gap-0 p-0 sm:max-w-[min(1200px,95vw)]">
        <header className="flex items-center gap-3 border-b px-5 py-3 pr-12">
          <DialogTitle className="text-base">Sessions · {taskId}</DialogTitle>
          <span className="font-mono text-[11px] text-muted-foreground">
            .markplane/.runs/{taskId}.json
          </span>
          <div className="flex-1" />
          <button
            type="button"
            disabled={refreshing}
            title="Re-read the Run record and transcripts (this dialog does not auto-update)"
            onClick={async () => {
              setRefreshing(true);
              try {
                await refresh(active?.sessionId);
                setReadAt(new Date());
              } finally {
                setRefreshing(false);
              }
            }}
            className="inline-flex cursor-pointer items-center gap-1.5 rounded-md border px-2 py-1 text-xs hover:bg-muted disabled:opacity-60"
          >
            <RefreshCw
              className={cn("size-3.5", refreshing && "animate-spin")}
            />
            <span>Refresh</span>
            <span className="text-muted-foreground">
              read {fmtClock(readAt)}
            </span>
          </button>
        </header>

        {runs.length === 0 ? (
          <EmptyRecordState taskId={taskId} />
        ) : (
          <div className="grid min-h-0 grid-cols-[320px_1fr]">
            <nav className="min-h-0 overflow-y-auto border-r bg-muted/20 py-2">
              {runs.map((run) => (
                <RunNavBlock
                  key={run.seq}
                  taskId={taskId}
                  run={run}
                  active={active}
                  onSelect={setSelection}
                />
              ))}
            </nav>
            <section className="min-h-0 overflow-hidden">
              {activeRun && activeGroup ? (
                <SessionPane
                  key={`${activeRun.seq}:${activeGroup.sessionId}`}
                  taskId={taskId}
                  run={activeRun}
                  group={activeGroup}
                  focus={active?.attemptIndex}
                  open={open}
                />
              ) : (
                <div className="flex h-full items-center justify-center p-8 text-xs text-muted-foreground">
                  Select a session from the list.
                </div>
              )}
            </section>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
