import { ChatIcon, TerminalIcon, TestTube01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Button } from "@/components/ui/button";
import type { GroveTab } from "@/lib/mock";
import { createMockChatTab, createMockTerminalTab } from "@/lib/mock";
import type { SidePaneTab } from "@/lib/mock-side-pane";
import {
  createMockSidePaneTerminalTab,
  createMockSidePaneTestTab,
  openMockSidePaneTab,
} from "@/lib/mock-side-pane";
import { Chat } from "./chat";
import { Terminal } from "./terminal";
import { TestTab } from "./test";

export const tabRegistry = [
  { kind: "chat", label: "Chat", icon: ChatIcon, create: createMockChatTab },
  { kind: "terminal", label: "Terminal", icon: TerminalIcon, create: createMockTerminalTab },
] as const;

export function renderTab(tab: GroveTab) {
  if (tab.kind === "terminal") return <Terminal terminalId={tab.terminalId} />;
  return <Chat />;
}

export const sidePaneTabRegistry = [
  {
    kind: "terminal",
    label: "Terminal",
    icon: TerminalIcon,
    create: createMockSidePaneTerminalTab,
  },
  {
    kind: "test",
    label: "Test",
    icon: TestTube01Icon,
    create: createMockSidePaneTestTab,
  },
] as const;

export function getSidePaneTabIcon(tab: SidePaneTab) {
  if (tab.kind === "new" || tab.kind === "unavailable") return undefined;
  return sidePaneTabRegistry.find(({ kind }) => kind === tab.kind)?.icon;
}

export function renderSidePaneTab(tab: SidePaneTab) {
  if (tab.kind === "new") return <NewTabPicker />;
  if (tab.kind === "unavailable") return <UnavailableTab title={tab.title} />;
  if (tab.kind === "terminal") return <Terminal terminalId={tab.terminalId} />;
  return <TestTab />;
}

function NewTabPicker() {
  return (
    <div className="flex size-full items-center justify-center gap-1">
      {sidePaneTabRegistry.map((registration) => (
        <Button
          key={registration.kind}
          onClick={() => openMockSidePaneTab(registration.create())}
          type="button"
          variant="ghost"
        >
          <HugeiconsIcon icon={registration.icon} strokeWidth={2} />
          {registration.label}
        </Button>
      ))}
    </div>
  );
}

function UnavailableTab({ title }: { title: string }) {
  return (
    <div className="flex size-full items-center justify-center bg-background p-6 text-center">
      <div>
        <h1 className="text-sm font-medium">{title} is unavailable</h1>
        <p className="mt-1 text-xs text-muted-foreground">
          This tab type is not available in this version of Grove.
        </p>
      </div>
    </div>
  );
}
