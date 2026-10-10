"use client";

import { useCallback, useMemo, useState } from "react";
import { ChevronsUpDown } from "lucide-react";
import { useConfig } from "@/lib/hooks/use-config";
import { useUpdateConfig } from "@/lib/hooks/use-mutations";
import {
  useAgentModels,
  useDispatcherConcurrency,
  useUpdateDispatcherConcurrency,
} from "@/lib/hooks/use-dispatcher-settings";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { PageTransition } from "@/components/domain/page-transition";
import type { AgentModel, DispatcherConfig, ProjectConfig, Stage } from "@/lib/types";
import { cn } from "@/lib/utils";

const STAGES: Stage[] = ["implement", "fix", "merge"];

function isValidModel(value: string): boolean {
  const trimmed = value.trim();
  return (
    trimmed.length > 0 &&
    trimmed.length <= 200 &&
    !/\s/.test(trimmed)
  );
}

function ModelCombobox({
  id,
  value,
  models,
  onCommit,
}: {
  id: string;
  value: string;
  models: AgentModel[];
  onCommit: (model: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState(value);
  const known = useMemo(
    () => new Set(models.map((m) => m.slug)),
    [models],
  );
  const warnUnknown = draft.trim().length > 0 && !known.has(draft.trim()) && models.length > 0;
  const invalid = draft.trim().length > 0 && !isValidModel(draft);

  const commit = useCallback(
    (next: string) => {
      setDraft(next);
      if (next === value) return;
      if (!isValidModel(next)) return;
      onCommit(next.trim());
    },
    [onCommit, value],
  );

  return (
    <div className="space-y-1.5">
      <div className="flex gap-2">
        <Input
          id={id}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={() => commit(draft)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              (e.target as HTMLInputElement).blur();
            }
          }}
          placeholder="model slug"
          aria-invalid={invalid || undefined}
          className="flex-1"
        />
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverTrigger asChild>
            <Button
              type="button"
              variant="outline"
              size="icon"
              className="shrink-0"
              aria-label="Browse models"
            >
              <ChevronsUpDown className="size-4" />
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-80 p-0" align="end">
            <Command>
              <CommandInput placeholder="Search models…" />
              <CommandList>
                <CommandEmpty>No models found.</CommandEmpty>
                <CommandGroup>
                  {models.map((model) => (
                    <CommandItem
                      key={model.slug}
                      value={`${model.slug} ${model.label}`}
                      onSelect={() => {
                        commit(model.slug);
                        setOpen(false);
                      }}
                    >
                      <span className="truncate font-medium">{model.slug}</span>
                      <span className="text-muted-foreground truncate text-xs">
                        {model.label}
                      </span>
                    </CommandItem>
                  ))}
                </CommandGroup>
              </CommandList>
            </Command>
          </PopoverContent>
        </Popover>
      </div>
      {invalid && (
        <p className="text-destructive text-xs">
          Model must be non-empty, without whitespace, and at most 200 characters.
        </p>
      )}
      {!invalid && warnUnknown && (
        <p className="text-muted-foreground text-xs">
          Not in this machine&apos;s model list — saved anyway.
        </p>
      )}
    </div>
  );
}

export function DispatcherSection() {
  const { data: config, isLoading, dataUpdatedAt } = useConfig();
  const {
    data: concurrencyData,
    isLoading: concurrencyLoading,
    dataUpdatedAt: concurrencyUpdatedAt,
  } = useDispatcherConcurrency();

  if (isLoading || !config || concurrencyLoading || concurrencyData == null) {
    return (
      <div className="space-y-6">
        <Card>
          <CardContent className="p-6">
            <div className="h-40 animate-pulse rounded bg-muted" />
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-6">
            <div className="h-24 animate-pulse rounded bg-muted" />
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <DispatcherSectionForm
      key={`${dataUpdatedAt}-${concurrencyUpdatedAt}`}
      config={config}
      concurrency={concurrencyData.concurrency}
      inUse={concurrencyData.in_use}
    />
  );
}

function DispatcherSectionForm({
  config,
  concurrency,
  inUse,
}: {
  config: ProjectConfig;
  concurrency: number;
  inUse: number;
}) {
  const updateConfig = useUpdateConfig();
  const updateConcurrency = useUpdateDispatcherConcurrency();
  const { data: models = [] } = useAgentModels();
  const dispatcher: DispatcherConfig = config.dispatcher;

  const [timeoutDrafts, setTimeoutDrafts] = useState(() => ({
    implement: String(dispatcher.stages.implement.timeout_minutes),
    fix: String(dispatcher.stages.fix.timeout_minutes),
    merge: String(dispatcher.stages.merge.timeout_minutes),
  }));
  const [concurrencyDraft, setConcurrencyDraft] = useState(String(concurrency));
  const [timeoutErrors, setTimeoutErrors] = useState<Partial<Record<Stage, string>>>({});
  const [concurrencyError, setConcurrencyError] = useState<string | null>(null);

  const saveTimeout = useCallback(
    (stage: Stage, raw: string) => {
      const num = parseInt(raw, 10);
      if (Number.isNaN(num) || num < 1 || num > 1440) {
        setTimeoutErrors((prev) => ({
          ...prev,
          [stage]: "Timeout must be between 1 and 1440 minutes.",
        }));
        return;
      }
      setTimeoutErrors((prev) => {
        const next = { ...prev };
        delete next[stage];
        return next;
      });
      if (num === dispatcher.stages[stage].timeout_minutes) return;
      updateConfig.mutate({
        dispatcher: { stages: { [stage]: { timeout_minutes: num } } },
      });
    },
    [dispatcher.stages, updateConfig],
  );

  const saveConcurrency = useCallback(
    (raw: string) => {
      const num = parseInt(raw, 10);
      if (Number.isNaN(num) || num < 1 || num > 100) {
        setConcurrencyError("Concurrency must be between 1 and 100.");
        return;
      }
      setConcurrencyError(null);
      if (num === concurrency) return;
      updateConcurrency.mutate(num);
    },
    [concurrency, updateConcurrency],
  );

  return (
    <PageTransition>
      <div className="space-y-6">
        <p className="text-muted-foreground text-sm">
          Agents run with full permissions as the serve user — no sandbox. Only
          enable scanning on trusted repositories. See the Dispatcher docs for
          security guidance.
        </p>

        <Card>
          <CardHeader>
            <CardTitle>This Project (shared via config.yaml)</CardTitle>
            <CardDescription>
              Push policy and Stage models/timeouts travel with the repository.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-6">
            <div className="flex items-center justify-between gap-4">
              <div className="space-y-0.5">
                <Label htmlFor="push-after-merge">Push after merge</Label>
                <p className="text-muted-foreground text-xs">
                  When on, successful merges are pushed to the remote.
                </p>
              </div>
              <Switch
                id="push-after-merge"
                checked={dispatcher.push_after_merge}
                onCheckedChange={(checked) =>
                  updateConfig.mutate({
                    dispatcher: { push_after_merge: checked },
                  })
                }
              />
            </div>

            {STAGES.map((stage) => (
              <div
                key={stage}
                className="grid gap-4 border-t pt-4 sm:grid-cols-2"
              >
                <div className="space-y-2 sm:col-span-2">
                  <Label className="capitalize">{stage}</Label>
                </div>
                <div className="space-y-2">
                  <Label htmlFor={`model-${stage}`}>Model</Label>
                  <ModelCombobox
                    id={`model-${stage}`}
                    value={dispatcher.stages[stage].model}
                    models={models}
                    onCommit={(model) =>
                      updateConfig.mutate({
                        dispatcher: { stages: { [stage]: { model } } },
                      })
                    }
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor={`timeout-${stage}`}>Timeout (minutes)</Label>
                  <Input
                    id={`timeout-${stage}`}
                    inputMode="numeric"
                    value={timeoutDrafts[stage]}
                    onChange={(e) =>
                      setTimeoutDrafts((prev) => ({
                        ...prev,
                        [stage]: e.target.value,
                      }))
                    }
                    onBlur={() => saveTimeout(stage, timeoutDrafts[stage])}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.preventDefault();
                        (e.target as HTMLInputElement).blur();
                      }
                    }}
                    aria-invalid={timeoutErrors[stage] ? true : undefined}
                  />
                  {timeoutErrors[stage] && (
                    <p className="text-destructive text-xs">{timeoutErrors[stage]}</p>
                  )}
                </div>
              </div>
            ))}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>This machine (all Projects)</CardTitle>
            <CardDescription>
              Global concurrency for every Project served on this machine.
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            <div className="flex flex-wrap items-end gap-4">
              <div className="space-y-2">
                <Label htmlFor="concurrency">Concurrency limit</Label>
                <Input
                  id="concurrency"
                  inputMode="numeric"
                  className={cn("w-32")}
                  value={concurrencyDraft}
                  onChange={(e) => setConcurrencyDraft(e.target.value)}
                  onBlur={() => saveConcurrency(concurrencyDraft)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      (e.target as HTMLInputElement).blur();
                    }
                  }}
                  aria-invalid={concurrencyError ? true : undefined}
                />
              </div>
              <p className="text-muted-foreground pb-2 text-sm">
                In use {inUse}
              </p>
            </div>
            {concurrencyError && (
              <p className="text-destructive text-xs">{concurrencyError}</p>
            )}
          </CardContent>
        </Card>
      </div>
    </PageTransition>
  );
}
