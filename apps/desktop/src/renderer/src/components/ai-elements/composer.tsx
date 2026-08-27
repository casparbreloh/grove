import { AuiIf, ComposerPrimitive } from "@assistant-ui/react";
import { ArrowUp02Icon, StopIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { RefObject } from "react";
import { Button } from "@/components/ui/button";

export function Composer({ inputRef }: { inputRef?: RefObject<HTMLTextAreaElement | null> }) {
  return (
    <ComposerPrimitive.Root className="relative flex w-full flex-col">
      <div className="flex w-full cursor-text flex-col gap-2 rounded-3xl border bg-card p-2 shadow-xs transition-colors focus-within:border-ring focus-within:ring-2 focus-within:ring-ring/20">
        <ComposerPrimitive.Input
          aria-label="Message input"
          autoFocus
          className="max-h-48 min-h-10 w-full resize-none bg-transparent px-2.5 py-1 text-sm/6 outline-none placeholder:text-muted-foreground/60"
          enterKeyHint="send"
          placeholder="Ask anything"
          ref={inputRef}
          rows={1}
        />
        <div className="flex items-center justify-end">
          <AuiIf condition={(state) => !state.thread.isRunning}>
            <ComposerPrimitive.Send
              render={
                <Button
                  aria-label="Send message"
                  className="rounded-full"
                  size="icon"
                  type="button"
                />
              }
            >
              <HugeiconsIcon className="size-4" icon={ArrowUp02Icon} strokeWidth={2} />
            </ComposerPrimitive.Send>
          </AuiIf>
          <AuiIf condition={(state) => state.thread.isRunning}>
            <ComposerPrimitive.Cancel
              render={
                <Button
                  aria-label="Stop generating"
                  className="rounded-full"
                  size="icon"
                  type="button"
                />
              }
            >
              <HugeiconsIcon className="size-3.5" icon={StopIcon} strokeWidth={2} />
            </ComposerPrimitive.Cancel>
          </AuiIf>
        </div>
      </div>
    </ComposerPrimitive.Root>
  );
}
