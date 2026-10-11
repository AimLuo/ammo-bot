"use client";

import {
  forwardRef,
  useImperativeHandle,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { Textarea } from "@/components/ui/textarea";
import { Button } from "@/components/ui/button";
import { toast } from "sonner";

export type CommentComposerHandle = {
  focus: () => void;
  scrollIntoView: () => void;
};

export const CommentComposer = forwardRef<
  CommentComposerHandle,
  {
    onSubmit: (text: string) => Promise<void>;
    /**
     * When set, shows "Comment & continue" using the same text.
     * Return `false` to keep the text (e.g. user cancelled a confirm).
     */
    onContinue?: (text: string) => Promise<void | false>;
    continueLabel?: string;
    continueDisabled?: boolean;
    continueTitle?: string;
    submitLabel?: string;
    extraActions?: ReactNode;
    disabled?: boolean;
  }
>(function CommentComposer(
  {
    onSubmit,
    onContinue,
    continueLabel = "Comment & continue",
    continueDisabled,
    continueTitle,
    submitLabel = "Comment",
    extraActions,
    disabled,
  },
  ref
) {
  const [text, setText] = useState("");
  const [pending, setPending] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const empty = text.trim().length === 0;
  const cannotSubmit = !!disabled || pending || empty;
  const cannotContinue =
    !!disabled || !!continueDisabled || pending || empty;

  useImperativeHandle(ref, () => ({
    focus: () => {
      textareaRef.current?.focus();
    },
    scrollIntoView: () => {
      rootRef.current?.scrollIntoView({ behavior: "smooth", block: "center" });
      textareaRef.current?.focus();
    },
  }));

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

  const submitContinue = async () => {
    if (!onContinue || cannotContinue) return;
    const toSend = text;
    setPending(true);
    try {
      const result = await onContinue(toSend);
      if (result !== false) setText("");
    } catch (err) {
      toast.error("Failed to continue", {
        description: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setPending(false);
    }
  };

  return (
    <div ref={rootRef} className="space-y-2">
      <Textarea
        ref={textareaRef}
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
        {onContinue ? (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="cursor-pointer"
            disabled={cannotContinue}
            title={continueTitle}
            onClick={() => void submitContinue()}
          >
            {continueLabel}
          </Button>
        ) : null}
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
});
