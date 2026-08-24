import {
  ErrorPrimitive,
  MessagePartPrimitive,
  MessagePrimitive,
  useAuiState,
} from "@assistant-ui/react";
import { MarkdownText } from "@/components/chat/markdown-text";
import { cn } from "@/lib/utils";

type MessageProps = {
  topPadded: boolean;
};

export function Message({ topPadded }: MessageProps) {
  const role = useAuiState((state) => state.message.role);
  return role === "user" ? <UserMessage topPadded={topPadded} /> : <AssistantMessage />;
}

function UserMessage({ topPadded }: MessageProps) {
  const isLatestUserMessage = useAuiState((state) => {
    for (let index = state.thread.messages.length - 1; index >= 0; index -= 1) {
      const message = state.thread.messages[index];
      if (message?.role === "user") return message.id === state.message.id;
    }
    return false;
  });

  return (
    <MessagePrimitive.Root
      className={cn("ml-auto max-w-[85%] text-sm leading-6", {
        "pt-8": topPadded && isLatestUserMessage,
      })}
    >
      <div className="rounded-2xl bg-muted px-4 py-2">
        <MessagePrimitive.Parts components={{ Text: UserText }} />
      </div>
    </MessagePrimitive.Root>
  );
}

function UserText() {
  return <MessagePartPrimitive.Text />;
}

function AssistantMessage() {
  return (
    <MessagePrimitive.Root className="min-w-0 px-1 text-sm leading-6">
      <MessagePrimitive.Parts components={{ Text: MarkdownText }} />
      <MessagePrimitive.Error>
        <ErrorPrimitive.Root className="mt-3 rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-destructive">
          <ErrorPrimitive.Message />
        </ErrorPrimitive.Root>
      </MessagePrimitive.Error>
    </MessagePrimitive.Root>
  );
}
