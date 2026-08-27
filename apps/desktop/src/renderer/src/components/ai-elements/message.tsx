import { ErrorPrimitive, MessagePrimitive, useAuiState } from "@assistant-ui/react";
import { MarkdownText } from "@/components/chat/markdown-text";

export function Message() {
  const role = useAuiState((state) => state.message.role);
  return role === "user" ? <UserMessage /> : <AssistantMessage />;
}

function UserMessage() {
  return (
    <MessagePrimitive.Root className="fade-in slide-in-from-bottom-1 animate-in flex justify-end px-2 duration-150 motion-reduce:animate-none [contain-intrinsic-size:auto_200px] [content-visibility:auto]">
      <div className="max-w-[85%] rounded-xl bg-muted px-4 py-2 text-sm/relaxed text-foreground wrap-break-word empty:hidden">
        <MessagePrimitive.Parts />
      </div>
    </MessagePrimitive.Root>
  );
}

function AssistantMessage() {
  return (
    <MessagePrimitive.Root className="fade-in slide-in-from-bottom-1 animate-in px-2 text-sm/relaxed duration-150 motion-reduce:animate-none [contain-intrinsic-size:auto_200px] [content-visibility:auto]">
      <div className="text-foreground wrap-break-word">
        <MessagePrimitive.Parts components={{ Text: MarkdownText }} />
        <MessagePrimitive.Error>
          <ErrorPrimitive.Root className="mt-2 rounded-md border border-destructive bg-destructive/10 p-3 text-sm text-destructive dark:bg-destructive/5">
            <ErrorPrimitive.Message className="line-clamp-2" />
          </ErrorPrimitive.Root>
        </MessagePrimitive.Error>
      </div>
    </MessagePrimitive.Root>
  );
}
