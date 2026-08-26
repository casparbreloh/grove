import { Add01Icon, Cancel01Icon, TerminalIcon, TestTube01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useState } from "react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useAppSidebarOpen } from "@/hooks/use-app-sidebar-open";
import {
  closeMockSidePaneTab,
  createMockSidePaneTerminalTab,
  createMockSidePaneTestTab,
  selectMockSidePaneTab,
  type SidePaneTab,
  useMockSidePaneTabs,
} from "@/lib/mock-side-pane-state";
import { cn } from "@/lib/utils";
import { Terminal } from "../tabs/terminal";
import { useSidePaneLayout } from "./side-pane-layout";
import { useOpenSidePaneTab } from "./use-open-side-pane-tab";

const sidePaneTabDefinitions = [
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

export function SidePaneTabs() {
  const { isSidePaneMaximized } = useSidePaneLayout();
  const isAppSidebarOpen = useAppSidebarOpen();
  const { activeSidePaneTabId, sidePaneTabs } = useMockSidePaneTabs();
  const needsTitlebarSafeArea = isSidePaneMaximized && !isAppSidebarOpen;

  return (
    <aside
      aria-label="Side pane"
      className="flex size-full flex-col border-l bg-background text-foreground"
    >
      <header
        className={cn(
          "flex h-10 shrink-0 items-center pr-18 pl-1.5 [-webkit-app-region:drag]",
          needsTitlebarSafeArea && "pl-[calc(var(--desktop-header-safe-area-left)+0.375rem)]",
        )}
      >
        {sidePaneTabs.length > 0 && (
          <SidePaneTabStrip activeSidePaneTabId={activeSidePaneTabId} sidePaneTabs={sidePaneTabs} />
        )}
      </header>
      <div className="min-h-0 flex-1">
        {sidePaneTabs.length === 0 ? (
          <SidePaneTabPicker />
        ) : (
          sidePaneTabs.map((sidePaneTab) => (
            <div
              aria-hidden={sidePaneTab.tabId !== activeSidePaneTabId}
              aria-labelledby={`${sidePaneTab.tabId}-tab`}
              className={sidePaneTab.tabId === activeSidePaneTabId ? "h-full" : "hidden"}
              id={`${sidePaneTab.tabId}-panel`}
              inert={sidePaneTab.tabId !== activeSidePaneTabId}
              key={sidePaneTab.tabId}
              role="tabpanel"
            >
              {renderSidePaneTabContent(sidePaneTab)}
            </div>
          ))
        )}
      </div>
    </aside>
  );
}

function SidePaneTabStrip({
  activeSidePaneTabId,
  sidePaneTabs,
}: Readonly<{
  activeSidePaneTabId: string | undefined;
  sidePaneTabs: readonly SidePaneTab[];
}>) {
  const [closingSidePaneTabId, setClosingSidePaneTabId] = useState<string>();

  function finishClosingSidePaneTab(sidePaneTabId: string) {
    if (closingSidePaneTabId !== sidePaneTabId) return;
    closeMockSidePaneTab(sidePaneTabId);
    setClosingSidePaneTabId(undefined);
  }

  return (
    <div className="flex h-full w-full min-w-0 items-center gap-1 [-webkit-app-region:no-drag]">
      <nav
        aria-label="Side pane tabs"
        className="scroll-fade-x scroll-fade-6 flex h-full w-max max-w-[calc(100%_-_2rem)] min-w-0 items-center gap-1 overflow-x-auto"
        role="tablist"
      >
        {sidePaneTabs.map((sidePaneTab) => (
          <SidePaneTabButton
            isActive={sidePaneTab.tabId === activeSidePaneTabId}
            isClosing={sidePaneTab.tabId === closingSidePaneTabId}
            key={sidePaneTab.tabId}
            onClose={() =>
              setClosingSidePaneTabId((currentTabId) => currentTabId ?? sidePaneTab.tabId)
            }
            onCloseTransitionEnd={() => finishClosingSidePaneTab(sidePaneTab.tabId)}
            sidePaneTab={sidePaneTab}
          />
        ))}
      </nav>
      <SidePaneTabCreationMenu />
    </div>
  );
}

function SidePaneTabButton({
  isActive,
  isClosing,
  onClose,
  onCloseTransitionEnd,
  sidePaneTab,
}: Readonly<{
  isActive: boolean;
  isClosing: boolean;
  onClose: () => void;
  onCloseTransitionEnd: () => void;
  sidePaneTab: SidePaneTab;
}>) {
  const icon = getSidePaneTabIcon(sidePaneTab);

  return (
    <div
      className={cn(
        "group/tab relative flex h-7 w-50 min-w-25 shrink translate-x-0 items-center overflow-hidden opacity-100 transition-[width,min-width,opacity,translate] duration-100 ease-out starting:w-0 starting:min-w-0 starting:shrink-0 starting:-translate-x-1 starting:opacity-0",
        isClosing && "pointer-events-none w-0 min-w-0 shrink-0 -translate-x-1 opacity-0",
      )}
      onTransitionEnd={(event) => {
        if (event.currentTarget === event.target && isClosing) onCloseTransitionEnd();
      }}
    >
      <Button
        aria-controls={`${sidePaneTab.tabId}-panel`}
        aria-selected={isActive}
        className={cn(
          "h-full w-full justify-start pr-7 font-normal text-foreground group-hover/tab:bg-accent group-hover/tab:text-accent-foreground focus-visible:ring-inset focus-visible:ring-offset-0",
          isActive && "bg-accent text-accent-foreground",
        )}
        id={`${sidePaneTab.tabId}-tab`}
        onClick={() => selectMockSidePaneTab(sidePaneTab.tabId)}
        role="tab"
        type="button"
        variant="ghost"
      >
        {icon && <HugeiconsIcon icon={icon} strokeWidth={2} />}
        <span className="min-w-0 flex-1 truncate-fade">{sidePaneTab.title}</span>
      </Button>
      <Button
        aria-label={`Close ${sidePaneTab.title}`}
        className="pointer-events-none absolute top-1/2 right-0 -translate-y-1/2 text-muted-foreground opacity-0 transition-opacity duration-75 group-hover/tab:pointer-events-auto group-hover/tab:opacity-100 hover:bg-transparent hover:text-foreground focus-visible:pointer-events-auto focus-visible:opacity-100"
        onClick={(event) => {
          event.stopPropagation();
          onClose();
        }}
        size="icon-sm"
        type="button"
        variant="ghost"
      >
        <HugeiconsIcon icon={Cancel01Icon} strokeWidth={2} />
      </Button>
    </div>
  );
}

function SidePaneTabCreationMenu() {
  const openSidePaneTab = useOpenSidePaneTab();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button aria-label="New side pane tab" size="icon-sm" type="button" variant="ghost" />
        }
      >
        <HugeiconsIcon icon={Add01Icon} strokeWidth={2} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        {sidePaneTabDefinitions.map((sidePaneTabDefinition) => (
          <DropdownMenuItem
            key={sidePaneTabDefinition.kind}
            onClick={() => openSidePaneTab(sidePaneTabDefinition.create())}
          >
            <HugeiconsIcon icon={sidePaneTabDefinition.icon} strokeWidth={2} />
            {sidePaneTabDefinition.label}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function SidePaneTabPicker() {
  const openSidePaneTab = useOpenSidePaneTab();

  return (
    <div className="size-full overflow-y-auto p-6">
      <div className="flex min-h-full items-center justify-center">
        <div className="flex w-full max-w-64 flex-col gap-2">
          {sidePaneTabDefinitions.map((sidePaneTabDefinition) => (
            <Button
              className="w-full justify-start"
              key={sidePaneTabDefinition.kind}
              onClick={() => openSidePaneTab(sidePaneTabDefinition.create())}
              size="lg"
              type="button"
              variant="secondary"
            >
              <HugeiconsIcon icon={sidePaneTabDefinition.icon} strokeWidth={2} />
              {sidePaneTabDefinition.label}
            </Button>
          ))}
        </div>
      </div>
    </div>
  );
}

function getSidePaneTabIcon(sidePaneTab: SidePaneTab) {
  if (sidePaneTab.kind === "unavailable") return undefined;
  return sidePaneTabDefinitions.find(({ kind }) => kind === sidePaneTab.kind)?.icon;
}

function renderSidePaneTabContent(sidePaneTab: SidePaneTab) {
  if (sidePaneTab.kind === "unavailable") {
    return <UnavailableSidePaneTab title={sidePaneTab.title} />;
  }
  if (sidePaneTab.kind === "terminal") {
    return <Terminal terminalId={sidePaneTab.terminalId} />;
  }
  return <SidePaneTestTab />;
}

function UnavailableSidePaneTab({ title }: { title: string }) {
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

function SidePaneTestTab() {
  return <div className="size-full bg-background" />;
}
