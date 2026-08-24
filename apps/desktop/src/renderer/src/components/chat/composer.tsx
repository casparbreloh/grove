import { AuiIf, ComposerPrimitive } from "@assistant-ui/react";
import { ArrowUp02Icon, StopIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Button } from "@/components/ui/button";

export function Composer() {
  return (
    <ComposerPrimitive.Root className="rounded-3xl border bg-card p-2 shadow-xs focus-within:border-ring focus-within:ring-2 focus-within:ring-ring/20">
      <ComposerPrimitive.Input
        aria-label="Message input"
        autoFocus
        className="max-h-40 min-h-11 w-full resize-none bg-transparent px-2 py-2 text-sm outline-none"
        enterKeyHint="send"
        placeholder="Ask anything"
        rows={1}
      />
      <div className="flex justify-end pt-1">
        <AuiIf condition={(state) => !state.thread.isRunning}>
          <ComposerPrimitive.Send
            render={
              <Button
                aria-label="Send message"
                className="rounded-full"
                size="icon-lg"
                type="button"
              />
            }
          >
            <HugeiconsIcon icon={ArrowUp02Icon} strokeWidth={2} />
          </ComposerPrimitive.Send>
        </AuiIf>
        <AuiIf condition={(state) => state.thread.isRunning}>
          <ComposerPrimitive.Cancel
            render={
              <Button
                aria-label="Stop generating"
                className="rounded-full"
                size="icon-lg"
                type="button"
              />
            }
          >
            <HugeiconsIcon icon={StopIcon} strokeWidth={2} />
          </ComposerPrimitive.Cancel>
        </AuiIf>
      </div>
    </ComposerPrimitive.Root>
  );
}
