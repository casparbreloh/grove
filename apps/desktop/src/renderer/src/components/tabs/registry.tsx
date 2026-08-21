import { BotIcon, FileDiffIcon, TerminalIcon } from "@hugeicons/core-free-icons";
import type { IconSvgElement } from "@hugeicons/react";
import { lazy, Suspense } from "react";
import type { ReactNode } from "react";
import { Agent } from "@/components/tabs/agent";
import { Terminal } from "@/components/tabs/terminal";
import type { GroveTab } from "@/lib/mock";
import { createMockAgentTab, createMockDiffTab, createMockTerminalTab } from "@/lib/mock";

const Diff = lazy(() =>
  import("@/components/tabs/diff").then((module) => ({ default: module.Diff })),
);

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
  {
    kind: "diff",
    label: "Diff",
    icon: FileDiffIcon,
    create: createMockDiffTab,
    render: (tab) => (
      <Suspense fallback={null}>
        <Diff diffId={tab.diffId} />
      </Suspense>
    ),
  } satisfies TabRegistration<"diff">,
] as const;

export function renderTab(tab: GroveTab) {
  switch (tab.kind) {
    case "agent":
      return tabRegistry[0].render();
    case "terminal":
      return tabRegistry[1].render(tab);
    case "diff":
      return tabRegistry[2].render(tab);
  }
}
