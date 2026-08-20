import { BotIcon, TerminalIcon } from "@hugeicons/core-free-icons";
import type { IconSvgElement } from "@hugeicons/react";
import type { ReactNode } from "react";
import { Agent } from "@/components/tabs/agent";
import { Terminal } from "@/components/tabs/terminal";
import type { GroveTab } from "@/lib/mock";
import { createMockAgentTab, createMockTerminalTab } from "@/lib/mock";

type TabRegistration<K extends GroveTab["kind"]> = {
  kind: K;
  label: string;
  icon: IconSvgElement;
  create: () => void;
  render: (tab: Extract<GroveTab, { kind: K }>) => ReactNode;
};

export const tabRegistry = [
  {
    kind: "agent",
    label: "Agent",
    icon: BotIcon,
    create: createMockAgentTab,
    render: () => <Agent />,
  } satisfies TabRegistration<"agent">,
  {
    kind: "terminal",
    label: "Terminal",
    icon: TerminalIcon,
    create: createMockTerminalTab,
    render: (tab) => <Terminal terminalId={tab.terminalId} />,
  } satisfies TabRegistration<"terminal">,
] as const;

export function renderTab(tab: GroveTab) {
  switch (tab.kind) {
    case "agent":
      return tabRegistry[0].render();
    case "terminal":
      return tabRegistry[1].render(tab);
  }
}
