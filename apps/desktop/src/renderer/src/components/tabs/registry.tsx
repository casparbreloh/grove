import { ChatIcon, TerminalIcon } from "@hugeicons/core-free-icons";
import type { IconSvgElement } from "@hugeicons/react";
import type { ReactNode } from "react";
import { Chat } from "@/components/tabs/chat";
import { Terminal } from "@/components/tabs/terminal";
import type { GroveTab } from "@/lib/mock";
import { createMockChatTab, createMockTerminalTab } from "@/lib/mock";

type TabRegistration<K extends GroveTab["kind"]> = {
  kind: K;
  label: string;
  icon: IconSvgElement;
  create: () => void;
  render: (tab: Extract<GroveTab, { kind: K }>) => ReactNode;
};

type RegisteredTab = {
  kind: GroveTab["kind"];
  label: string;
  icon: IconSvgElement;
  create: () => void;
  render: (tab: GroveTab) => ReactNode;
};

function defineTab<K extends GroveTab["kind"]>(registration: TabRegistration<K>): RegisteredTab {
  return {
    ...registration,
    render: (tab) => {
      if (tab.kind !== registration.kind) return null;
      return registration.render(tab as Extract<GroveTab, { kind: K }>);
    },
  };
}

export const tabRegistry = [
  defineTab({
    kind: "chat",
    label: "Chat",
    icon: ChatIcon,
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
