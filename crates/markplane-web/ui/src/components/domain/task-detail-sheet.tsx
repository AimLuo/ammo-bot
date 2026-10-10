"use client";

import { useState } from "react";
import { ProjectLink } from "@/components/project-link";
import { useTask, useTasks } from "@/lib/hooks/use-tasks";
import { useEpics } from "@/lib/hooks/use-epics";
import { usePlans } from "@/lib/hooks/use-plans";
import { useNotes } from "@/lib/hooks/use-notes";
import { useUpdateTask, useAddTaskComment, useArchiveItem, useUnarchiveItem } from "@/lib/hooks/use-mutations";
import { useRunSummaryMap } from "@/lib/hooks/use-runs";
import {
  activeRunConfirmMessage,
  hasActiveDispatcherRun,
} from "@/lib/run-display";
import { CreateDialog } from "./create-dialog";
import { PriorityIndicator } from "./priority-indicator";
import { MarkdownRenderer } from "./markdown-renderer";
import { MarkdownEditor } from "./markdown-editor";
import { InlineEdit } from "./inline-edit";
import { TagEditor } from "./tag-editor";
import { EntityCombobox } from "./entity-combobox";
import { EntityRefEditor } from "./entity-ref-editor";
import { FieldRow } from "./field-row";
import { RunSection } from "./run-section";
import { CommentComposer } from "./comment-composer";
import {
  Sheet,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { ResizableSheetContent } from "./resizable-sheet-content";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { STATUS_CONFIG, buildStatusConfig, allStatuses, categoryOf } from "@/lib/constants";
import { useConfig } from "@/lib/hooks/use-config";
import { Circle, Pencil, Archive, ArchiveRestore, ArrowUpRight, Bot } from "lucide-react";
import type { Priority, Effort, Comment } from "@/lib/types";
const ALL_PRIORITIES: Priority[] = [
  "critical",
  "high",
  "medium",
  "low",
  "someday",
];
const ALL_EFFORTS: Effort[] = ["xs", "small", "medium", "large", "xl"];

export function TaskDetailSheet({
  taskId,
  open,
  onOpenChange,
  archived,
}: {
  taskId: string | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  archived?: boolean;
}) {
  const [isEditingDescription, setIsEditingDescription] = useState(false);
  const [createPlanOpen, setCreatePlanOpen] = useState(false);
  const { data: task, isLoading } = useTask(taskId || "", {
    enabled: !isEditingDescription,
  });
  const updateTask = useUpdateTask();
  const addComment = useAddTaskComment();
  const archiveItem = useArchiveItem();
  const unarchiveItem = useUnarchiveItem();
  const { map: runMap } = useRunSummaryMap();
  const { data: epics } = useEpics();
  const { data: allTasks } = useTasks();
  const { data: plans } = usePlans();
  const { data: notes } = useNotes();
  const { data: config } = useConfig();
  const workflow = config?.workflows.task;
  const statusConfig = workflow ? buildStatusConfig(workflow) : null;
  const statusList = workflow ? allStatuses(workflow) : [];

  const taskCategory = task && workflow ? categoryOf(workflow, task.status) : undefined;
  const isClosedStatus = taskCategory === "completed" || taskCategory === "cancelled";
  const runSummary = task ? runMap.get(task.id) : undefined;

  const confirmActiveRun = (taskId: string): boolean => {
    if (!hasActiveDispatcherRun(runSummary)) return true;
    return window.confirm(activeRunConfirmMessage(taskId, runSummary!));
  };

  const epicOptions =
    epics?.map((e) => ({ id: e.id, title: e.title })) ?? [];
  const taskOptions =
    allTasks
      ?.filter((t) => t.id !== taskId)
      .map((t) => ({ id: t.id, title: t.title })) ?? [];
  const planOptions =
    plans?.map((p) => ({ id: p.id, title: p.title })) ?? [];
  const relatedOptions = [
    ...(allTasks?.filter((t) => t.id !== taskId).map((t) => ({ id: t.id, title: t.title })) ?? []),
    ...(epics?.map((e) => ({ id: e.id, title: e.title })) ?? []),
    ...(plans?.map((p) => ({ id: p.id, title: p.title })) ?? []),
    ...(notes?.map((n) => ({ id: n.id, title: n.title })) ?? []),
  ];

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <ResizableSheetContent>
        {isLoading || !task ? (
          <SheetHeader>
            <SheetTitle>
              <Skeleton className="h-6 w-48" />
            </SheetTitle>
            <div className="space-y-4 pt-4">
              <Skeleton className="h-4 w-32" />
              <Skeleton className="h-20 w-full" />
              <Skeleton className="h-40 w-full" />
            </div>
          </SheetHeader>
        ) : (
          <>
            <SheetHeader>
              <div className="flex items-center gap-2 pr-8">
                <span
                  className="font-mono text-sm"
                  style={{ color: "var(--entity-task)" }}
                >
                  {task.id}
                </span>
                <div className="flex-1" />
                {archived ? (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 px-2 gap-1 text-muted-foreground hover:text-foreground cursor-pointer"
                    onClick={() => {
                      unarchiveItem.mutate(task.id);
                      onOpenChange(false);
                    }}
                    disabled={unarchiveItem.isPending}
                  >
                    <ArchiveRestore className="size-3.5" />
                    <span className="text-xs">Restore</span>
                  </Button>
                ) : isClosedStatus && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 px-2 gap-1 text-muted-foreground hover:text-foreground cursor-pointer"
                    onClick={() => {
                      archiveItem.mutate(task.id);
                      onOpenChange(false);
                    }}
                    disabled={archiveItem.isPending}
                  >
                    <Archive className="size-3.5" />
                    <span className="text-xs">Archive</span>
                  </Button>
                )}
              </div>
              <SheetTitle className="text-left text-xl">
                <InlineEdit
                  value={task.title}
                  onSave={(title) =>
                    updateTask.mutate({ id: task.id, title })
                  }
                />
              </SheetTitle>
            </SheetHeader>

            <div className="space-y-4 px-4 pb-6">
              {/* Metadata */}
              <div className="space-y-0.5 text-sm">
                <FieldRow label="Status" editable>
                  <DropdownMenu>
                    <DropdownMenuTrigger className="cursor-pointer">
                      <span
                        className="inline-flex items-center gap-1.5"
                        style={{ color: `var(--status-${task.status})` }}
                      >
                        {(() => {
                          const cfg = statusConfig?.[task.status] ?? STATUS_CONFIG[task.status];
                          const Icon = cfg?.icon ?? Circle;
                          return <Icon className="size-3.5 text-current" />;
                        })()}
                        <span className="text-sm">
                          {statusConfig?.[task.status]?.label ?? STATUS_CONFIG[task.status]?.label ?? task.status}
                        </span>
                      </span>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent>
                      {statusList.map((s) => {
                        const cfg = statusConfig?.[s] ?? STATUS_CONFIG[s];
                        const Icon = cfg?.icon ?? Circle;
                        return (
                          <DropdownMenuItem
                            key={s}
                            onClick={() => {
                              if (s === task.status) return;
                              if (!confirmActiveRun(task.id)) return;
                              updateTask.mutate({ id: task.id, status: s });
                            }}
                          >
                            <Icon
                              className="mr-2 size-4 text-current"
                              style={{ color: `var(--status-${s})` }}
                            />
                            {cfg?.label ?? s}
                          </DropdownMenuItem>
                        );
                      })}
                    </DropdownMenuContent>
                  </DropdownMenu>
                </FieldRow>

                <FieldRow label="Priority" editable>
                  <DropdownMenu>
                    <DropdownMenuTrigger className="cursor-pointer">
                      <PriorityIndicator
                        priority={task.priority}
                        showLabel
                      />
                    </DropdownMenuTrigger>
                    <DropdownMenuContent>
                      {ALL_PRIORITIES.map((p) => (
                        <DropdownMenuItem
                          key={p}
                          onClick={() =>
                            updateTask.mutate({ id: task.id, priority: p })
                          }
                        >
                          <PriorityIndicator priority={p} showLabel />
                        </DropdownMenuItem>
                      ))}
                    </DropdownMenuContent>
                  </DropdownMenu>
                </FieldRow>

                <FieldRow label="Type" editable>
                  <DropdownMenu>
                    <DropdownMenuTrigger className="cursor-pointer">
                      <span className="text-sm uppercase">{task.type}</span>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent>
                      {(config?.task_types ?? ["feature", "bug", "enhancement", "chore", "research", "spike"]).map((t) => (
                        <DropdownMenuItem
                          key={t}
                          onClick={() =>
                            updateTask.mutate({ id: task.id, type: t })
                          }
                        >
                          <span className="uppercase text-xs">{t}</span>
                        </DropdownMenuItem>
                      ))}
                    </DropdownMenuContent>
                  </DropdownMenu>
                </FieldRow>

                <FieldRow label="Effort" editable>
                  <DropdownMenu>
                    <DropdownMenuTrigger className="cursor-pointer">
                      <span className="text-sm uppercase">{task.effort}</span>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent>
                      {ALL_EFFORTS.map((e) => (
                        <DropdownMenuItem
                          key={e}
                          onClick={() =>
                            updateTask.mutate({ id: task.id, effort: e })
                          }
                        >
                          <span className="uppercase text-xs">{e}</span>
                        </DropdownMenuItem>
                      ))}
                    </DropdownMenuContent>
                  </DropdownMenu>
                </FieldRow>

                <FieldRow label="Epic" editable>
                  <EntityCombobox
                    value={task.epic}
                    options={epicOptions}
                    onSelect={(id) =>
                      updateTask.mutate({ id: task.id, epic: id ?? "" })
                    }
                    placeholder="No epic"
                    emptyLabel="No epic"
                    linkValue
                  />
                </FieldRow>

                <FieldRow label="Plan" editable>
                  <EntityCombobox
                    value={task.plan}
                    options={planOptions}
                    onSelect={(id) =>
                      updateTask.mutate({ id: task.id, plan: id ?? "" })
                    }
                    placeholder="No plan"
                    emptyLabel="No plan"
                    linkValue
                    onCreateNew={() => setCreatePlanOpen(true)}
                    createNewLabel="Create new plan"
                  />
                </FieldRow>

                {task.map && (
                  <FieldRow label="Map">
                    <ProjectLink
                      href={`/maps?map=${task.map}&node=${task.id}`}
                      className="inline-flex items-center gap-1 font-mono text-xs px-1.5 py-0.5 rounded no-underline hover:opacity-80 transition-opacity"
                      style={{
                        backgroundColor:
                          "color-mix(in oklch, var(--entity-map) 15%, transparent)",
                        color: "var(--entity-map)",
                      }}
                    >
                      {task.map}
                      <ArrowUpRight className="size-3 opacity-70" />
                    </ProjectLink>
                  </FieldRow>
                )}

                <FieldRow label="Depends on" editable>
                  <EntityRefEditor
                    ids={task.depends_on}
                    options={taskOptions}
                    onAdd={(id) =>
                      updateTask.mutate({
                        id: task.id,
                        depends_on: [...task.depends_on, id],
                      })
                    }
                    onRemove={(id) =>
                      updateTask.mutate({
                        id: task.id,
                        depends_on: task.depends_on.filter((d) => d !== id),
                      })
                    }
                  />
                </FieldRow>

                <FieldRow label="Blocks" editable>
                  <EntityRefEditor
                    ids={task.blocks}
                    options={taskOptions}
                    onAdd={(id) =>
                      updateTask.mutate({
                        id: task.id,
                        blocks: [...task.blocks, id],
                      })
                    }
                    onRemove={(id) =>
                      updateTask.mutate({
                        id: task.id,
                        blocks: task.blocks.filter((b) => b !== id),
                      })
                    }
                  />
                </FieldRow>

                <FieldRow label="Related" editable>
                  <EntityRefEditor
                    ids={task.related}
                    options={relatedOptions}
                    onAdd={(id) =>
                      updateTask.mutate({
                        id: task.id,
                        related: [...task.related, id],
                      })
                    }
                    onRemove={(id) =>
                      updateTask.mutate({
                        id: task.id,
                        related: task.related.filter((r) => r !== id),
                      })
                    }
                  />
                </FieldRow>

                <FieldRow label="Assignee" editable>
                  <AssigneeEditor
                    value={task.assignee}
                    onSave={(assignee) => {
                      const next = assignee.trim() || "";
                      const prev = task.assignee ?? "";
                      if (next === prev) return;
                      if (!confirmActiveRun(task.id)) return;
                      updateTask.mutate({ id: task.id, assignee });
                    }}
                  />
                </FieldRow>

                <FieldRow label="Tags" editable>
                  <TagEditor
                    tags={task.tags}
                    onSave={(tags) =>
                      updateTask.mutate({ id: task.id, tags })
                    }
                  />
                </FieldRow>

                <FieldRow label="Created">
                  <span className="text-muted-foreground">{task.created}</span>
                </FieldRow>

                <FieldRow label="Updated">
                  <span className="text-muted-foreground">{task.updated}</span>
                </FieldRow>
              </div>

              <RunSection taskId={task.id} task={task} />

              <Separator />

              {/* Description (Comments section excluded) */}
              {isEditingDescription && !archived ? (
                <MarkdownEditor
                  content={task.description}
                  onSave={(description) => {
                    updateTask.mutate({ id: task.id, description });
                    setIsEditingDescription(false);
                  }}
                  onCancel={() => setIsEditingDescription(false)}
                  isLoading={updateTask.isPending}
                />
              ) : (
                <div className="group relative">
                  {!archived && (
                    <button
                      type="button"
                      onClick={() => setIsEditingDescription(true)}
                      className="sticky top-11 float-right ml-2 p-1 rounded opacity-0 group-hover:opacity-100 hover:bg-accent transition-opacity cursor-pointer z-10"
                    >
                      <Pencil className="size-4 text-primary/50 group-hover:text-primary" />
                    </button>
                  )}
                  {task.description.trim() ? (
                    <MarkdownRenderer content={task.description} />
                  ) : archived ? (
                    <p className="text-sm text-muted-foreground italic">No description</p>
                  ) : (
                    <button
                      type="button"
                      onClick={() => setIsEditingDescription(true)}
                      className="text-sm text-muted-foreground italic hover:text-foreground cursor-pointer"
                    >
                      Click to add description...
                    </button>
                  )}
                </div>
              )}

              <Separator />

              <div className="space-y-3">
                <h3 className="text-sm font-medium">
                  Comments ({task.comments.length})
                </h3>
                {task.comments.length === 0 ? (
                  <p className="text-sm text-muted-foreground">No comments yet</p>
                ) : (
                  <ul className="space-y-4">
                    {task.comments.map((comment, index) => (
                      <CommentItem key={`${comment.date}-${comment.author}-${index}`} comment={comment} />
                    ))}
                  </ul>
                )}
                {!archived && (
                  <CommentComposer
                    onSubmit={async (text) => {
                      await addComment.mutateAsync({ id: task.id, text });
                    }}
                  />
                )}
              </div>
            </div>
          </>
        )}

        {task && (
          <CreateDialog
            kind="plan"
            taskId={task.id}
            open={createPlanOpen}
            onOpenChange={setCreatePlanOpen}
          />
        )}
      </ResizableSheetContent>
    </Sheet>
  );
}

function isMachineAuthor(author: string): boolean {
  return author === "dispatcher" || author.startsWith("agent-");
}

function CommentItem({ comment }: { comment: Comment }) {
  return (
    <li className="space-y-1">
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        {isMachineAuthor(comment.author) && (
          <Bot className="size-3.5 shrink-0" aria-label="Machine author" />
        )}
        <span className="font-medium text-foreground/80">{comment.author}</span>
        <span>·</span>
        <span>{comment.date}</span>
      </div>
      {comment.body.trim() ? (
        <MarkdownRenderer content={comment.body} />
      ) : null}
    </li>
  );
}

function AssigneeEditor({
  value,
  onSave,
}: {
  value: string | null;
  onSave: (value: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState(value ?? "");

  const handleSave = () => {
    onSave(draft.trim());
    setOpen(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleSave();
    } else if (e.key === "Escape") {
      setDraft(value ?? "");
      setOpen(false);
    }
  };

  return (
    <Popover
      open={open}
      onOpenChange={(o) => {
        setOpen(o);
        if (o) setDraft(value ?? "");
      }}
    >
      <PopoverTrigger className="cursor-pointer text-left">
        {value ? (
          <span className="text-sm">@{value}</span>
        ) : (
          <span className="text-sm text-muted-foreground italic">
            Unassigned
          </span>
        )}
      </PopoverTrigger>
      <PopoverContent className="w-52 p-3" align="start">
        <div className="space-y-2">
          <Input
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Assignee name..."
            className="h-7 text-sm"
            autoFocus
          />
          <p className="text-xs text-muted-foreground">
            Press Enter to save, leave empty to unassign
          </p>
        </div>
      </PopoverContent>
    </Popover>
  );
}
