"use client";

import { useEffect, useState } from "react";
import {
  ArrowUp,
  Folder,
  FolderCheck,
  Loader2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { ApiError } from "@/lib/api-error";
import {
  useAddProject,
  useFsListing,
  useInitProject,
} from "@/lib/hooks/use-projects";
import { projectHref, useProjectRouter } from "@/lib/project-prefix";

type AddProjectDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
};

export function AddProjectDialog({ open, onOpenChange }: AddProjectDialogProps) {
  const router = useProjectRouter();
  const [browsePath, setBrowsePath] = useState<string | null>(null);
  const [pathInput, setPathInput] = useState("");
  const [actionError, setActionError] = useState<string | null>(null);

  const listing = useFsListing(browsePath, open);
  const add = useAddProject();
  const init = useInitProject();
  const busy = add.isPending || init.isPending;

  useEffect(() => {
    if (!open) {
      setBrowsePath(null);
      setPathInput("");
      setActionError(null);
      return;
    }
  }, [open]);

  useEffect(() => {
    if (listing.data) {
      setPathInput(listing.data.path);
    }
  }, [listing.data]);

  async function goToPath(raw: string) {
    setActionError(null);
    setBrowsePath(raw.trim() || null);
  }

  async function handleAdd() {
    const path = listing.data?.path;
    if (!path) return;
    setActionError(null);
    try {
      const project = await add.mutateAsync(path);
      onOpenChange(false);
      router.push(projectHref(project.id));
    } catch (err) {
      if (err instanceof ApiError) {
        setActionError(err.message);
      } else {
        setActionError("Failed to add project");
      }
    }
  }

  async function handleInitAndAdd() {
    const path = listing.data?.path;
    if (!path) return;
    setActionError(null);
    try {
      const project = await init.mutateAsync(path);
      onOpenChange(false);
      router.push(projectHref(project.id));
    } catch (err) {
      if (err instanceof ApiError) {
        setActionError(err.message);
      } else {
        setActionError("Failed to initialize project");
      }
    }
  }

  const data = listing.data;
  const listError =
    listing.error instanceof Error
      ? listing.error.message
      : listing.error
        ? "Failed to list directory"
        : null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-xl gap-3">
        <DialogHeader>
          <DialogTitle>添加 Project</DialogTitle>
          <DialogDescription>
            从家目录浏览，或粘贴路径（支持 ~）后回车跳转。
          </DialogDescription>
        </DialogHeader>

        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            void goToPath(pathInput);
          }}
        >
          <Input
            value={pathInput}
            onChange={(e) => setPathInput(e.target.value)}
            placeholder="~/projects/my-app"
            className="font-mono text-sm"
            aria-label="Directory path"
            disabled={busy}
          />
          <Button type="submit" variant="secondary" disabled={busy || listing.isFetching}>
            Go
          </Button>
        </form>

        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={busy || !data?.parent}
            onClick={() => data?.parent && void goToPath(data.parent)}
          >
            <ArrowUp className="size-4" />
            上级
          </Button>
          {data?.home && data.path !== data.home && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={() => void goToPath(data.home)}
            >
              Home
            </Button>
          )}
          {listing.isFetching && (
            <Loader2 className="size-4 animate-spin text-muted-foreground" />
          )}
        </div>

        <div className="max-h-64 overflow-y-auto rounded-md border">
          {listError && (
            <p className="p-3 text-sm text-destructive">{listError}</p>
          )}
          {!listError && data && data.entries.length === 0 && (
            <p className="p-3 text-sm text-muted-foreground">No subdirectories</p>
          )}
          {!listError &&
            data?.entries.map((entry) => (
              <button
                key={entry.path}
                type="button"
                disabled={busy}
                className="flex w-full items-center gap-2 border-b px-3 py-2 text-left text-sm last:border-b-0 hover:bg-muted/60 disabled:opacity-50"
                onClick={() => void goToPath(entry.path)}
              >
                {entry.has_markplane ? (
                  <FolderCheck className="size-4 shrink-0 text-primary" />
                ) : (
                  <Folder className="size-4 shrink-0 text-muted-foreground" />
                )}
                <span className="truncate font-mono">{entry.name}</span>
                {entry.has_markplane && (
                  <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                    .markplane
                  </span>
                )}
              </button>
            ))}
        </div>

        {actionError ? (
          <p className="text-sm text-destructive">{actionError}</p>
        ) : data && !data.has_markplane ? (
          <p className="text-sm text-muted-foreground">
            当前目录尚未初始化 Markplane。可以初始化并添加，或继续浏览子目录。
          </p>
        ) : null}

        <DialogFooter className="gap-2 sm:gap-0">
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={busy}
          >
            Cancel
          </Button>
          {data?.has_markplane ? (
            <Button type="button" onClick={() => void handleAdd()} disabled={busy || !data}>
              {add.isPending && <Loader2 className="size-4 animate-spin" />}
              添加
            </Button>
          ) : (
            <Button
              type="button"
              onClick={() => void handleInitAndAdd()}
              disabled={busy || !data}
            >
              {init.isPending && <Loader2 className="size-4 animate-spin" />}
              初始化并添加
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
