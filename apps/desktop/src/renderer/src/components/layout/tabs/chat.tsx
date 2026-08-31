import {
  AssistantRuntimeProvider,
  AuiIf,
  ThreadPrimitive,
  useLocalRuntime,
} from "@assistant-ui/react";
import { Folder01Icon } from "@hugeicons/core-free-icons";
import { useRef } from "react";
import { Composer } from "@/components/ai-elements/composer";
import { Message } from "@/components/ai-elements/message";
import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { mockChatModel } from "@/lib/mocks/chat-model";
import { createMockProject, selectMockDraftProject, useMockGrove } from "@/lib/mocks/grove";

const newProjectValue = "action:create-project";
const noProjectValue = "action:no-project";

function createProjectFromPrompt() {
  const name = window.prompt("Project name");
  if (!name?.trim()) return false;
  createMockProject(name);
  return true;
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
  const composerInputRef = useRef<HTMLTextAreaElement>(null);

  return (
    <ThreadPrimitive.Root className="flex h-full flex-col bg-background">
      <ThreadPrimitive.Viewport
        className="no-scrollbar flex flex-1 flex-col overflow-y-auto"
        turnAnchor="top"
      >
        <div className="mx-auto flex w-full max-w-3xl flex-1 flex-col px-6 pt-8 sm:px-10">
          <AuiIf condition={(state) => state.thread.isEmpty}>
            <div className="flex flex-1 items-center justify-center pb-24">
              <h1 className="text-2xl font-semibold">What would you like to work on?</h1>
            </div>
          </AuiIf>

          <div className="flex flex-col gap-6 pb-8">
            <ThreadPrimitive.Messages>{() => <Message />}</ThreadPrimitive.Messages>
          </div>

          <ThreadPrimitive.ViewportFooter className="sticky bottom-0 mt-auto bg-gradient-to-t from-background via-background to-transparent pb-6 pt-8 sm:-mx-4">
            <AuiIf condition={(state) => state.thread.isEmpty}>
              <div className="mx-4 flex h-10 items-center rounded-t-2xl border border-b-0 bg-card px-2 text-muted-foreground shadow-xs">
                <NativeSelect
                  aria-label="Choose project"
                  className="max-w-52 [&_select]:truncate [&_select]:border-transparent [&_select]:bg-transparent [&_select]:dark:bg-transparent"
                  icon={Folder01Icon}
                  onChange={(event) => {
                    const projectId = event.currentTarget.value;
                    if (projectId === newProjectValue) {
                      if (!createProjectFromPrompt())
                        event.currentTarget.value = draftProjectId ?? noProjectValue;
                    } else if (projectId === noProjectValue) {
                      selectMockDraftProject(undefined);
                    } else {
                      selectMockDraftProject(projectId);
                    }
                    requestAnimationFrame(() => composerInputRef.current?.focus());
                  }}
                  size="sm"
                  value={draftProjectId ?? noProjectValue}
                  variant="leading-icon"
                >
                  {projects.map((project) => (
                    <NativeSelectOption key={project.id} value={project.id}>
                      {project.name}
                    </NativeSelectOption>
                  ))}
                  <NativeSelectOption value={noProjectValue}>No project</NativeSelectOption>
                  <NativeSelectOption value={newProjectValue}>New project…</NativeSelectOption>
                </NativeSelect>
              </div>
            </AuiIf>
            <div className="-mt-px">
              <Composer inputRef={composerInputRef} />
            </div>
          </ThreadPrimitive.ViewportFooter>
        </div>
      </ThreadPrimitive.Viewport>
    </ThreadPrimitive.Root>
  );
}
