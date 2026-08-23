import { BotIcon, TerminalIcon } from "@hugeicons/core-free-icons";
import type { IconSvgElement } from "@hugeicons/react";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ReactNode } from "react";
import { Agent } from "@/components/tabs/agent";
import { Terminal } from "@/components/tabs/terminal";
import { Button } from "@/components/ui/button";
import type { CreatableGroveTab, GroveTab } from "@/lib/mock";
import { createMockAgentTab, createMockTerminalTab } from "@/lib/mock";

type TabRegistration<K extends CreatableGroveTab["kind"]> = {
  kind: K;
  label: string;
  description: string;
  icon: IconSvgElement;
  create: (replaceTabId?: string) => void;
  render: (tab: Extract<GroveTab, { kind: K }>) => ReactNode;
};

type RegisteredTab = {
  kind: CreatableGroveTab["kind"];
  label: string;
  description: string;
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
    kind: "agent",
    label: "Agent",
    description: "Work with a coding agent in this task.",
    icon: BotIcon,
    create: createMockAgentTab,
    render: () => <Agent />,
  }),
  defineTab({
    kind: "terminal",
    label: "Terminal",
    description: "Open a shell for this task's environment.",
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
      <div className="w-full max-w-lg">
        <div className="mb-5 text-center">
          <h1 className="text-base font-medium">Open a tab</h1>
          <p className="mt-1 text-xs text-muted-foreground">Choose a view for this task.</p>
        </div>
        <div className="grid grid-cols-2 gap-3">
          {tabRegistry.map((registration) => (
            <Button
              className="h-auto min-h-24 items-start justify-start gap-3 bg-card p-4 text-left whitespace-normal"
              key={registration.kind}
              onClick={() => registration.create(tabId)}
              variant="outline"
            >
              <span className="flex size-8 shrink-0 items-center justify-center rounded-md bg-muted text-foreground">
                <HugeiconsIcon icon={registration.icon} strokeWidth={2} />
              </span>
              <span className="min-w-0 pt-0.5">
                <span className="block text-xs font-medium text-foreground">
                  {registration.label}
                </span>
                <span className="mt-1 block text-xs/relaxed font-normal text-muted-foreground">
                  {registration.description}
                </span>
              </span>
            </Button>
          ))}
        </div>
      </div>
    </div>
  );
}
