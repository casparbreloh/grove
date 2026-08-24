import {
  AssistantRuntimeProvider,
  AuiIf,
  ThreadPrimitive,
  useAuiEvent,
  useLocalRuntime,
} from "@assistant-ui/react";
import { Add01Icon, ArrowDown01Icon, Cancel01Icon, Folder01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useRef, useState } from "react";
import { Composer } from "@/components/chat/composer";
import { Message } from "@/components/chat/message";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { createMockProject, mockChatModel, selectMockDraftProject, useMockGrove } from "@/lib/mock";

export function Chat() {
  const runtime = useLocalRuntime(mockChatModel);

  return (
    <AssistantRuntimeProvider runtime={runtime}>
      <ChatViewport />
    </AssistantRuntimeProvider>
  );
}

function ChatViewport() {
  const { projects, draftProjectId } = useMockGrove();
  const selectedProject = projects.find(({ id }) => id === draftProjectId);
  const lastScrollTop = useRef(0);
  const userScrollIntent = useRef(false);
  const [reserveReleased, setReserveReleased] = useState(false);
  const [showNoProject, setShowNoProject] = useState(Boolean(draftProjectId));
  const [topPadding, setTopPadding] = useState(false);

  const handleNewProject = () => {
    const name = window.prompt("Project name");
    if (name?.trim()) createMockProject(name);
  };

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

          <ThreadPrimitive.ViewportFooter className="sticky bottom-0 mt-auto bg-gradient-to-t from-background via-background to-transparent pb-6 pt-8 sm:-mx-4">
            <AuiIf condition={(state) => state.thread.messages.length === 0}>
              <div className="mx-4 flex h-10 items-center rounded-t-2xl border border-b-0 bg-card px-2 text-muted-foreground shadow-xs">
                <DropdownMenu
                  onOpenChange={(open) => {
                    if (open) setShowNoProject(Boolean(draftProjectId));
                  }}
                >
                  <DropdownMenuTrigger
                    render={
                      <Button
                        aria-label="Choose project"
                        className={
                          selectedProject
                            ? "max-w-52 justify-start text-foreground"
                            : "max-w-52 justify-start text-muted-foreground"
                        }
                        variant="ghost"
                      />
                    }
                  >
                    <HugeiconsIcon data-icon="inline-start" icon={Folder01Icon} strokeWidth={2} />
                    <span className="truncate">{selectedProject?.name ?? "Choose project"}</span>
                    <HugeiconsIcon data-icon="inline-end" icon={ArrowDown01Icon} strokeWidth={2} />
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="start" className="w-56" side="top">
                    {projects.map((project) => (
                      <DropdownMenuItem
                        key={project.id}
                        onClick={() => selectMockDraftProject(project.id)}
                      >
                        <HugeiconsIcon icon={Folder01Icon} strokeWidth={2} />
                        {project.name}
                      </DropdownMenuItem>
                    ))}
                    <DropdownMenuSeparator />
                    <DropdownMenuItem onClick={handleNewProject}>
                      <HugeiconsIcon icon={Add01Icon} strokeWidth={2} />
                      New project
                    </DropdownMenuItem>
                    {showNoProject && (
                      <DropdownMenuItem onClick={() => selectMockDraftProject(undefined)}>
                        <HugeiconsIcon icon={Cancel01Icon} strokeWidth={2} />
                        No project
                      </DropdownMenuItem>
                    )}
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            </AuiIf>
            <div className="-mt-px">
              <Composer />
            </div>
          </ThreadPrimitive.ViewportFooter>
        </div>
      </ThreadPrimitive.Viewport>
    </ThreadPrimitive.Root>
  );
}
