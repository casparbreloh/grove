import { BubbleChatIcon, TerminalIcon } from "@hugeicons/core-free-icons";
import type { IconSvgElement } from "@hugeicons/react";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ReactNode } from "react";
import { Chat } from "@/components/tabs/chat";
import { Terminal } from "@/components/tabs/terminal";
import { Button } from "@/components/ui/button";
import type { CreatableGroveTab, GroveTab } from "@/lib/mock";
import { createMockChatTab, createMockTerminalTab } from "@/lib/mock";

type TabRegistration<K extends CreatableGroveTab["kind"]> = {
  kind: K;
  label: string;
  icon: IconSvgElement;
  create: (replaceTabId?: string) => void;
  render: (tab: Extract<GroveTab, { kind: K }>) => ReactNode;
};

type RegisteredTab = {
  kind: CreatableGroveTab["kind"];
  label: string;
  icon: IconSvgElement;
  create: (replaceTabId?: string) => void;
  render: (tab: CreatableGroveTab) => ReactNode;
};

function defineTab<K extends CreatableGroveTab["kind"]>(
  registration: TabRegistration<K>,
): RegisteredTab {
  return {
    ...registration,
    render: (tab) => {
      if (tab.kind !== registration.kind) return null;
      return registration.render(tab as Extract<CreatableGroveTab, { kind: K }>);
    },
  };
}

export const tabRegistry = [
  defineTab({
    kind: "chat",
    label: "Chat",
    icon: BubbleChatIcon,
    create: createMockChatTab,
    render: () => <Chat />,
  }),
  defineTab({
    kind: "terminal",
    label: "Terminal",
    icon: TerminalIcon,
    create: createMockTerminalTab,
    render: (tab) => <Terminal terminalId={tab.terminalId} />,
  }),
] as const;

export function renderTab(tab: GroveTab) {
  if (tab.kind === "new") return <TabPicker tabId={tab.tabId} />;
  const registration = tabRegistry.find(({ kind }) => kind === tab.kind);
  return registration ? registration.render(tab) : <UnavailableTab title={tab.title} />;
}

function UnavailableTab({ title }: { title: string }) {
  return (
    <div className="flex h-full items-center justify-center bg-background p-6 text-center">
      <div>
        <h1 className="text-sm font-medium">{title} is unavailable</h1>
        <p className="mt-1 text-xs text-muted-foreground">
          This tab type is not available in this version of Grove.
        </p>
      </div>
    </div>
  );
}

function TabPicker({ tabId }: { tabId: string }) {
  return (
    <div className="flex h-full items-center justify-center bg-background p-6">
      <div className="flex w-full max-w-2xl flex-col gap-2">
        {tabRegistry.map((registration) => (
          <Button
            className="h-12 w-full justify-start gap-3 px-4 text-sm font-normal"
            key={registration.kind}
            onClick={() => registration.create(tabId)}
            variant="secondary"
          >
            <HugeiconsIcon
              className="text-muted-foreground"
              icon={registration.icon}
              strokeWidth={2}
            />
            {registration.label}
          </Button>
        ))}
      </div>
    </div>
  );
}
