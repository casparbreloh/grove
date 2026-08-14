import { ArrowUp02Icon, Attachment01Icon, File01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { createFileRoute } from "@tanstack/react-router";
import { useLayoutEffect, useRef, useState } from "react";
import {
  Attachment,
  AttachmentContent,
  AttachmentDescription,
  AttachmentMedia,
  AttachmentTitle,
} from "@/components/ui/attachment";
import { Bubble, BubbleContent } from "@/components/ui/bubble";
import { Button } from "@/components/ui/button";
import { Message, MessageContent } from "@/components/ui/message";
import {
  MessageScroller,
  MessageScrollerButton,
  MessageScrollerContent,
  MessageScrollerItem,
  MessageScrollerProvider,
  MessageScrollerViewport,
} from "@/components/ui/message-scroller";
import { Textarea } from "@/components/ui/textarea";

export const Route = createFileRoute("/_app/")({ component: App });

type ChatMessage = {
  id: string;
  role: "assistant" | "user";
  content: string;
  attachment?: {
    name: string;
    description: string;
  };
};

const initialMessages: ChatMessage[] = [
  {
    id: "welcome",
    role: "assistant",
    content: "Hey! What would you like to work on?",
  },
  {
    id: "request",
    role: "user",
    content: "Can you review the product vision and suggest a good next step?",
    attachment: {
      name: "VISION.md",
      description: "Project context",
    },
  },
  {
    id: "first-step",
    role: "assistant",
    content:
      "I’d start with the first complete chat loop: send a message, stream the response, and make stopping reliable.",
  },
  {
    id: "why-first",
    role: "user",
    content: "Why prioritize that over session history?",
  },
  {
    id: "foundation",
    role: "assistant",
    content:
      "Because it validates the core interaction first. History becomes much easier to shape once a real conversation can start, update, and stop cleanly.",
  },
  {
    id: "scope",
    role: "user",
    content: "What is the smallest useful version?",
  },
  {
    id: "scope-answer",
    role: "assistant",
    content:
      "One conversation view, one provider, streaming text, a stop action, and a clear error state. No branching, search, or rich tools yet.",
  },
  {
    id: "tools",
    role: "user",
    content: "Should tool calls be part of the first pass?",
  },
  {
    id: "tools-answer",
    role: "assistant",
    content:
      "Only enough to reserve the shape. Keep tool events in the message model, but render a simple status row until the chat loop is dependable.",
  },
  {
    id: "architecture",
    role: "user",
    content: "Where should the provider integration live?",
  },
  {
    id: "architecture-answer",
    role: "assistant",
    content:
      "In the main process behind a narrow typed IPC boundary. The renderer should only receive normalized messages, events, and status updates.",
  },
  {
    id: "first-slice",
    role: "user",
    content: "Can you turn that into a first implementation slice?",
  },
  {
    id: "plan",
    role: "assistant",
    content:
      "Start with the renderer state and transcript, add a typed send and cancel contract in preload, then connect one main-process adapter that emits normalized streaming events.",
  },
  {
    id: "verification",
    role: "assistant",
    content:
      "Verify the happy path, cancellation during a stream, provider failure, and navigating away mid-response. That is enough confidence to build history next.",
  },
];

function App() {
  const [messages, setMessages] = useState(initialMessages);
  const [draft, setDraft] = useState("");
  const [composerHeight, setComposerHeight] = useState(0);
  const composerRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const composer = composerRef.current;

    if (!composer) return;

    const observer = new ResizeObserver(() => {
      setComposerHeight(composer.getBoundingClientRect().height);
    });

    observer.observe(composer);
    return () => observer.disconnect();
  }, []);

  function sendMessage() {
    const content = draft.trim();

    if (!content) return;

    setMessages((current) => [...current, { id: crypto.randomUUID(), role: "user", content }]);
    setDraft("");
  }

  return (
    <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden bg-background">
      <MessageScrollerProvider autoScroll defaultScrollPosition="end">
        <MessageScroller className="flex-1">
          <MessageScrollerViewport className="chat-scroll-viewport">
            <MessageScrollerContent
              className="mx-auto w-full max-w-3xl px-6 pt-8 sm:px-10"
              style={{ paddingBottom: composerHeight + 8 }}
            >
              {messages.map((message) => (
                <MessageScrollerItem
                  key={message.id}
                  messageId={message.id}
                  scrollAnchor={message.role === "user"}
                >
                  <Message align={message.role === "user" ? "end" : "start"}>
                    <MessageContent>
                      {message.attachment && (
                        <Attachment size="sm">
                          <AttachmentMedia>
                            <HugeiconsIcon icon={File01Icon} strokeWidth={2} />
                          </AttachmentMedia>
                          <AttachmentContent>
                            <AttachmentTitle>{message.attachment.name}</AttachmentTitle>
                            <AttachmentDescription>
                              {message.attachment.description}
                            </AttachmentDescription>
                          </AttachmentContent>
                        </Attachment>
                      )}
                      <Bubble variant={message.role === "user" ? "secondary" : "ghost"}>
                        <BubbleContent className="text-sm leading-6">
                          {message.content}
                        </BubbleContent>
                      </Bubble>
                    </MessageContent>
                  </Message>
                </MessageScrollerItem>
              ))}
            </MessageScrollerContent>
          </MessageScrollerViewport>
          <MessageScrollerButton style={{ bottom: Math.max(composerHeight - 40, 8) }} />
        </MessageScroller>
      </MessageScrollerProvider>

      <div
        ref={composerRef}
        className="pointer-events-none absolute inset-x-0 bottom-0 z-10 px-6 pb-6 pt-3 sm:px-10"
      >
        <div
          aria-hidden="true"
          className="absolute inset-y-0 left-6 right-6 mx-auto max-w-3xl bg-gradient-to-t from-background via-background to-transparent forced-colors:bg-[Canvas] sm:left-10 sm:right-10"
        />
        <form
          className="pointer-events-auto relative mx-auto max-w-3xl rounded-xl border bg-card p-2 shadow-sm focus-within:border-ring focus-within:ring-2 focus-within:ring-ring/20"
          onSubmit={(event) => {
            event.preventDefault();
            sendMessage();
          }}
        >
          <Textarea
            aria-label="Message"
            className="max-h-40 min-h-14 overflow-y-auto overscroll-y-none border-0 bg-transparent px-2 py-2 shadow-none focus-visible:border-transparent focus-visible:ring-0 dark:bg-transparent"
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
                event.preventDefault();
                sendMessage();
              }
            }}
            placeholder="Ask anything"
            value={draft}
          />
          <div className="flex items-center justify-between pt-1">
            <Button aria-label="Attach a file" size="icon" type="button" variant="ghost">
              <HugeiconsIcon icon={Attachment01Icon} strokeWidth={2} />
            </Button>
            <Button aria-label="Send message" disabled={!draft.trim()} size="icon" type="submit">
              <HugeiconsIcon icon={ArrowUp02Icon} strokeWidth={2} />
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
