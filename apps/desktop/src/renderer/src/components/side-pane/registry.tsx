import { SquareIcon, TerminalIcon } from "@hugeicons/core-free-icons";
import type { IconSvgElement } from "@hugeicons/react";
import type { ReactNode } from "react";
import { TestTab } from "@/components/side-pane/test-tab";
import { Terminal } from "@/components/tabs/terminal";
import type { CreatableSidePaneTab, SidePaneTab } from "@/lib/mock-side-pane";
import { createMockSidePaneTerminalTab, createMockSidePaneTestTab } from "@/lib/mock-side-pane";

type SidePaneTabRegistration<K extends CreatableSidePaneTab["kind"]> = Readonly<{
  create: () => Extract<CreatableSidePaneTab, { kind: K }>;
  icon: IconSvgElement;
  kind: K;
  label: string;
  render: (tab: Extract<CreatableSidePaneTab, { kind: K }>) => ReactNode;
}>;

type RegisteredSidePaneTab = Readonly<{
  create: () => CreatableSidePaneTab;
  icon: IconSvgElement;
  kind: CreatableSidePaneTab["kind"];
  label: string;
  render: (tab: CreatableSidePaneTab) => ReactNode;
}>;

function defineSidePaneTab<K extends CreatableSidePaneTab["kind"]>(
  registration: SidePaneTabRegistration<K>,
): RegisteredSidePaneTab {
  return {
    ...registration,
    render: (tab) => {
      if (tab.kind !== registration.kind) return null;
      return registration.render(tab as Extract<CreatableSidePaneTab, { kind: K }>);
    },
  };
}

export const sidePaneTabRegistry = [
  defineSidePaneTab({
    kind: "terminal",
    label: "Terminal",
    icon: TerminalIcon,
    create: createMockSidePaneTerminalTab,
    render: (tab) => <Terminal terminalId={tab.terminalId} />,
  }),
  defineSidePaneTab({
    kind: "test",
    label: "Test",
    icon: SquareIcon,
    create: createMockSidePaneTestTab,
    render: () => <TestTab />,
  }),
] as const;

export function renderSidePaneTab(tab: SidePaneTab) {
  if (tab.kind === "new") {
    return (
      <div className="flex size-full items-center justify-center p-6 text-center text-xs text-muted-foreground">
        Choose a view from the new tab menu.
      </div>
    );
  }

  if (tab.kind === "unavailable") {
    return <UnavailableTab title={tab.title} />;
  }

  const registration = sidePaneTabRegistry.find(({ kind }) => kind === tab.kind);
  return registration ? registration.render(tab) : <UnavailableTab title={tab.title} />;
}

function UnavailableTab({ title }: { title: string }) {
  return (
    <div className="flex size-full items-center justify-center p-6 text-center">
      <div>
        <h1 className="text-sm font-medium">{title} is unavailable</h1>
        <p className="mt-1 text-xs text-muted-foreground">
          This tab type is not available in this version of Grove.
        </p>
      </div>
    </div>
  );
}
