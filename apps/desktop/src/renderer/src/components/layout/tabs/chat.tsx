import {
  AssistantRuntimeProvider,
  AuiIf,
  ThreadPrimitive,
  useLocalRuntime,
} from "@assistant-ui/react";
import { Add01Icon, ArrowDown01Icon, Cancel01Icon, Folder01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState } from "react";
import { Composer } from "@/components/ai-elements/composer";
import { Message } from "@/components/ai-elements/message";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { createMockProject, mockChatModel, selectMockDraftProject, useMockGrove } from "@/lib/mock";

function createProjectFromPrompt() {
  const name = window.prompt("Project name");
  if (name?.trim()) createMockProject(name);
}

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
  const [showNoProject, setShowNoProject] = useState(Boolean(draftProjectId));

  return (
    <ThreadPrimitive.Root className="flex h-full flex-col bg-background">
      <ThreadPrimitive.Viewport className="flex flex-1 flex-col overflow-y-auto" turnAnchor="top">
        <div className="mx-auto flex w-full max-w-3xl flex-1 flex-col px-6 pt-8 sm:px-10">
          <AuiIf condition={(state) => state.thread.messages.length === 0}>
            <div className="flex flex-1 items-center justify-center pb-24">
              <h1 className="text-2xl font-semibold">What would you like to work on?</h1>
            </div>
          </AuiIf>

          <div className="flex flex-col gap-6 pb-8">
            <ThreadPrimitive.Messages>{() => <Message />}</ThreadPrimitive.Messages>
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
                        size="sm"
                        variant="ghost"
                      />
                    }
                  >
                    <HugeiconsIcon
                      className="size-[calc(var(--text-sm)+1px)]"
                      data-icon="inline-start"
                      icon={Folder01Icon}
                      strokeWidth={2}
                    />
                    <span className="truncate">{selectedProject?.name ?? "Choose project"}</span>
                    <HugeiconsIcon data-icon="inline-end" icon={ArrowDown01Icon} strokeWidth={2} />
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="start" className="w-56" side="top">
                    {projects.map((project) => (
                      <DropdownMenuItem
                        key={project.id}
                        onClick={() => selectMockDraftProject(project.id)}
                      >
                        <HugeiconsIcon
                          className="size-[calc(var(--text-sm)+1px)]"
                          icon={Folder01Icon}
                          strokeWidth={2}
                        />
                        {project.name}
                      </DropdownMenuItem>
                    ))}
                    <DropdownMenuSeparator />
                    <DropdownMenuItem onClick={createProjectFromPrompt}>
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
