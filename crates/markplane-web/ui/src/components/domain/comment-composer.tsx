"use client";

import { useState, type ReactNode } from "react";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { toast } from "sonner";

export function CommentComposer({
  onSubmit,
  submitLabel = "Comment",
  extraActions,
  disabled,
}: {
  onSubmit: (text: string) => Promise<void>;
  submitLabel?: string;
  extraActions?: ReactNode;
  disabled?: boolean;
}) {
  const [text, setText] = useState("");
  const [pending, setPending] = useState(false);
  const empty = text.trim().length === 0;
  const cannotSubmit = !!disabled || pending || empty;

  const submit = async () => {
    if (cannotSubmit) return;
    const toSend = text;
    setPending(true);
    try {
      await onSubmit(toSend);
      setText("");
    } catch (err) {
      toast.error("Failed to post comment", {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setPending(false);
    }
  };

  return (
    <div className="space-y-2">
      <Textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        disabled={!!disabled || pending}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
            e.preventDefault();
            void submit();
          }
        }}
        placeholder="Write a comment..."
        className="min-h-20"
      />
      <div className="flex items-center gap-2">
        {extraActions}
        <div className="flex-1" />
        <Button
          type="button"
          size="sm"
          disabled={cannotSubmit}
          onClick={() => void submit()}
          className="cursor-pointer"
        >
          {submitLabel}
        </Button>
      </div>
    </div>
  );
}
