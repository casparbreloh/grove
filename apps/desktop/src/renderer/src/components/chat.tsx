import {
  AssistantRuntimeProvider,
  AuiIf,
  ThreadPrimitive,
  type ChatModelAdapter,
  useAuiEvent,
  useLocalRuntime,
} from "@assistant-ui/react";
import { useRef, useState } from "react";
import type { ChatEvent } from "../../../shared/chat-ipc";
import { Composer } from "@/components/assistant-ui/composer";
import { Message } from "@/components/assistant-ui/message";

const modelAdapter: ChatModelAdapter = {
  async *run({ messages, abortSignal }) {
    const userMessage = messages.findLast((message) => message.role === "user");
    const prompt = userMessage?.content
      .filter((part) => part.type === "text")
      .map((part) => part.text)
      .join("\n\n")
      .trim();
    if (!prompt) throw new Error("A message is required");

    const events: ChatEvent[] = [];
    let text = "";
    let aborted = abortSignal.aborted;
    let cancellation: Promise<void> | undefined;
    let settled = false;
    let failure: unknown;
    let wake: (() => void) | undefined;

    const notify = () => {
      wake?.();
      wake = undefined;
    };
    const unsubscribe = window.grove.chat.onEvent((event) => {
      events.push(event);
      notify();
    });
    const cancel = () => {
      aborted = true;
      notify();
      cancellation ??= window.grove.chat.cancel();
    };

    abortSignal.addEventListener("abort", cancel, { once: true });
    if (aborted) {
      unsubscribe();
      abortSignal.removeEventListener("abort", cancel);
      return;
    }

    const sending = window.grove.chat
      .send(prompt)
      .catch((error: unknown) => {
        failure = error;
      })
      .finally(() => {
        settled = true;
        notify();
      });

    try {
      while (!aborted && (!settled || events.length > 0)) {
        if (events.length === 0) {
          await new Promise<void>((resolve) => {
            wake = resolve;
          });
          continue;
        }

        const event = events.shift();
        if (!event) continue;

        text += event.delta;
        yield { content: [{ type: "text", text }] };
      }

      if (!aborted) {
        await sending;
        if (failure) throw failure;
      }
    } finally {
      unsubscribe();
      abortSignal.removeEventListener("abort", cancel);
      await cancellation;
    }
  },
};

export function Chat() {
  const runtime = useLocalRuntime(modelAdapter);

  return (
    <AssistantRuntimeProvider runtime={runtime}>
      <ChatViewport />
    </AssistantRuntimeProvider>
  );
}

function ChatViewport() {
  const lastScrollTop = useRef(0);
  const userScrollIntent = useRef(false);
  const [reserveReleased, setReserveReleased] = useState(false);
  const [topPadding, setTopPadding] = useState(false);

  useAuiEvent("thread.runStart", () => {
    userScrollIntent.current = false;
    setReserveReleased(false);
    setTopPadding(true);
  });
  useAuiEvent("threadListItem.switchedTo", () => {
    userScrollIntent.current = false;
    setReserveReleased(false);
    setTopPadding(false);
  });

  const markUserScroll = () => {
    if (!reserveReleased) userScrollIntent.current = true;
  };

  const releaseReserve = (element: HTMLDivElement) => {
    const scrollingUp = element.scrollTop < lastScrollTop.current;
    lastScrollTop.current = element.scrollTop;
    if (!scrollingUp || reserveReleased || !userScrollIntent.current) return;

    const reserve = element.querySelector<HTMLElement>("[data-aui-top-anchor-reserve]");
    if (!reserve) return;

    const gap = Number.parseFloat(getComputedStyle(reserve.parentElement ?? reserve).rowGap) || 0;
    const naturalBottom = element.scrollHeight - element.clientHeight - reserve.offsetHeight - gap;
    if (element.scrollTop > naturalBottom) return;

    userScrollIntent.current = false;
    setReserveReleased(true);
  };

  return (
    <ThreadPrimitive.Root className="flex h-full flex-col bg-background">
      <ThreadPrimitive.Viewport
        className="chat-viewport flex flex-1 flex-col overflow-y-auto"
        data-top-anchor-released={reserveReleased || undefined}
        onKeyDown={(event) => {
          const target = event.target as HTMLElement;
          if (
            !target.matches("input, textarea, [contenteditable='true']") &&
            ["ArrowUp", "PageUp", "Home"].includes(event.key)
          )
            markUserScroll();
        }}
        onPointerDown={(event) => {
          if (event.target === event.currentTarget) markUserScroll();
        }}
        onScroll={(event) => releaseReserve(event.currentTarget)}
        onTouchStart={markUserScroll}
        onWheel={(event) => {
          if (event.deltaY < 0) markUserScroll();
        }}
        turnAnchor="top"
      >
        <div className="mx-auto flex w-full max-w-3xl flex-1 flex-col px-6 pt-8 sm:px-10">
          <AuiIf condition={(state) => state.thread.messages.length === 0}>
            <div className="flex flex-1 items-center justify-center pb-24">
              <h1 className="text-2xl font-semibold">What would you like to work on?</h1>
            </div>
          </AuiIf>

          <div className="flex flex-col gap-6 pb-8">
            <ThreadPrimitive.Messages>
              {() => <Message topPadded={topPadding} />}
            </ThreadPrimitive.Messages>
          </div>

          <ThreadPrimitive.ViewportFooter className="sticky bottom-0 mt-auto bg-gradient-to-t from-background via-background to-transparent pb-6 pt-8">
            <Composer />
          </ThreadPrimitive.ViewportFooter>
        </div>
      </ThreadPrimitive.Viewport>
    </ThreadPrimitive.Root>
  );
}
