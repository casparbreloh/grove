import { Add01Icon, Cancel01Icon, TerminalIcon, TestTube01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ReactElement, ReactNode } from "react";
import { useCallback, useState } from "react";

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

function getSidePaneTabAfterClose(
  sidePaneTabs: readonly SidePaneTab[],
  closingSidePaneTabId: string,
  activeSidePaneTabId: string | undefined,
) {
  if (closingSidePaneTabId !== activeSidePaneTabId) return activeSidePaneTabId;
  const closingIndex = sidePaneTabs.findIndex(
    (sidePaneTab) => sidePaneTab.tabId === closingSidePaneTabId,
  );
  return sidePaneTabs[Math.min(closingIndex, sidePaneTabs.length - 2)]?.tabId;
}

export function SidePaneTabs() {
  const { closeSidePane, isSidePaneMaximized } = useSidePaneLayout();
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
          <SidePaneTabStrip
            activeSidePaneTabId={activeSidePaneTabId}
            onLastTabClose={closeSidePane}
            sidePaneTabs={sidePaneTabs}
          />
        )}
      </header>
      <div className="min-h-0 flex-1">
        {sidePaneTabs.map((sidePaneTab) => (
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
        ))}
      </div>
    </aside>
  );
}

function SidePaneTabStrip({
  activeSidePaneTabId,
  onLastTabClose,
  sidePaneTabs,
}: Readonly<{
  activeSidePaneTabId: string | undefined;
  onLastTabClose: (afterClose: () => void) => void;
  sidePaneTabs: readonly SidePaneTab[];
}>) {
  const [closingSidePaneTabId, setClosingSidePaneTabId] = useState<string>();

  const restoreTabFocus = useCallback((sidePaneTabId: string | undefined) => {
    window.requestAnimationFrame(() => {
      document.getElementById(sidePaneTabId ? `${sidePaneTabId}-tab` : "side-pane-toggle")?.focus();
    });
  }, []);

  function finishClosingSidePaneTab(sidePaneTabId: string, focusTabId: string | undefined) {
    if (closingSidePaneTabId !== sidePaneTabId) return;
    closeMockSidePaneTab(sidePaneTabId);
    setClosingSidePaneTabId(undefined);
    restoreTabFocus(focusTabId);
  }

  function closeSidePaneTab(sidePaneTabId: string) {
    if (closingSidePaneTabId) return;

    if (sidePaneTabs.length === 1) {
      onLastTabClose(() => closeMockSidePaneTab(sidePaneTabId));
      restoreTabFocus(undefined);
      return;
    }

    const focusTabId = getSidePaneTabAfterClose(sidePaneTabs, sidePaneTabId, activeSidePaneTabId);

    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      closeMockSidePaneTab(sidePaneTabId);
      restoreTabFocus(focusTabId);
      return;
    }
    setClosingSidePaneTabId(sidePaneTabId);
  }

  function selectSidePaneTab(sidePaneTabId: string) {
    selectMockSidePaneTab(sidePaneTabId);
    restoreTabFocus(sidePaneTabId);
  }

  return (
    <div className="flex h-full w-full min-w-0 items-center gap-1 [-webkit-app-region:no-drag]">
      <nav
        aria-label="Side pane tabs"
        className="scroll-fade-x scroll-fade-6 flex h-full w-max max-w-[calc(100%_-_2rem)] min-w-0 items-center gap-1 overflow-x-auto"
        aria-orientation="horizontal"
        role="tablist"
      >
        {sidePaneTabs.map((sidePaneTab) => (
          <SidePaneTabButton
            isActive={sidePaneTab.tabId === activeSidePaneTabId}
            isClosing={sidePaneTab.tabId === closingSidePaneTabId}
            key={sidePaneTab.tabId}
            onClose={() => closeSidePaneTab(sidePaneTab.tabId)}
            onCloseTransitionEnd={() =>
              finishClosingSidePaneTab(
                sidePaneTab.tabId,
                getSidePaneTabAfterClose(sidePaneTabs, sidePaneTab.tabId, activeSidePaneTabId),
              )
            }
            onSelect={() => selectSidePaneTab(sidePaneTab.tabId)}
            onSelectFromKeyboard={(key) => {
              const currentIndex = sidePaneTabs.findIndex((tab) => tab.tabId === sidePaneTab.tabId);
              const nextIndex =
                key === "Home"
                  ? 0
                  : key === "End"
                    ? sidePaneTabs.length - 1
                    : (currentIndex + (key === "ArrowRight" ? 1 : -1) + sidePaneTabs.length) %
                      sidePaneTabs.length;
              selectSidePaneTab(sidePaneTabs[nextIndex]?.tabId ?? sidePaneTab.tabId);
            }}
            sidePaneTab={sidePaneTab}
            shouldAnimateOnEnter={sidePaneTabs.length > 1}
          />
        ))}
      </nav>
      <SidePaneTabCreationMenu
        trigger={
          <Button aria-label="New side pane tab" size="icon-sm" type="button" variant="ghost" />
        }
      >
        <HugeiconsIcon icon={Add01Icon} strokeWidth={2} />
      </SidePaneTabCreationMenu>
    </div>
  );
}

function SidePaneTabButton({
  isActive,
  isClosing,
  onClose,
  onCloseTransitionEnd,
  onSelect,
  onSelectFromKeyboard,
  sidePaneTab,
  shouldAnimateOnEnter,
}: Readonly<{
  isActive: boolean;
  isClosing: boolean;
  onClose: () => void;
  onCloseTransitionEnd: () => void;
  onSelect: () => void;
  onSelectFromKeyboard: (key: "ArrowLeft" | "ArrowRight" | "End" | "Home") => void;
  sidePaneTab: SidePaneTab;
  shouldAnimateOnEnter: boolean;
}>) {
  const icon = getSidePaneTabIcon(sidePaneTab);

  return (
    <div
      className={cn(
        "group/tab relative flex h-7 w-37.5 min-w-25 shrink translate-x-0 items-center overflow-hidden opacity-100 transition-[width,min-width,opacity,translate] duration-100 ease-out motion-reduce:transition-none",
        shouldAnimateOnEnter &&
          "starting:w-0 starting:min-w-0 starting:shrink-0 starting:-translate-x-1 starting:opacity-0",
        isClosing && "pointer-events-none w-0 min-w-0 shrink-0 -translate-x-1 opacity-0",
      )}
      inert={isClosing}
      onTransitionEnd={(event) => {
        if (event.currentTarget === event.target && isClosing) onCloseTransitionEnd();
      }}
    >
      <Button
        aria-controls={`${sidePaneTab.tabId}-panel`}
        aria-selected={isActive}
        className={cn(
          "h-full w-full justify-start font-normal text-foreground transition-colors duration-75 group-hover/tab:bg-accent group-hover/tab:pr-7 group-hover/tab:text-accent-foreground group-focus-within/tab:pr-7 focus-visible:ring-inset focus-visible:ring-offset-0",
          isActive && "bg-accent pr-7 text-accent-foreground",
        )}
        id={`${sidePaneTab.tabId}-tab`}
        onClick={onSelect}
        onKeyDown={(event) => {
          switch (event.key) {
            case "ArrowLeft":
            case "ArrowRight":
            case "End":
            case "Home":
              event.preventDefault();
              onSelectFromKeyboard(event.key);
          }
        }}
        role="tab"
        tabIndex={isActive && !isClosing ? 0 : -1}
        type="button"
        variant="ghost"
      >
        {icon && <HugeiconsIcon className="size-[var(--text-sm)]" icon={icon} strokeWidth={2} />}
        <span className="mask-r-from-[calc(100%-1rem)] flex min-w-0 flex-1 overflow-hidden text-sm whitespace-nowrap">
          <span className="min-w-max">{sidePaneTab.title}</span>
        </span>
      </Button>
      <Button
        aria-label={`Close ${sidePaneTab.title}`}
        className={cn(
          "pointer-events-none absolute top-1/2 right-0 -translate-y-1/2 text-muted-foreground opacity-0 transition-none group-hover/tab:pointer-events-auto group-hover/tab:opacity-100 group-focus-within/tab:pointer-events-auto group-focus-within/tab:opacity-100 hover:bg-transparent hover:text-foreground focus-visible:pointer-events-auto focus-visible:opacity-100",
          isActive && "pointer-events-auto opacity-100",
        )}
        onClick={(event) => {
          event.stopPropagation();
          onClose();
        }}
        size="icon-sm"
        tabIndex={isActive && !isClosing ? 0 : -1}
        type="button"
        variant="ghost"
      >
        <HugeiconsIcon className="size-[var(--text-xs)]" icon={Cancel01Icon} strokeWidth={2} />
      </Button>
    </div>
  );
}

export function SidePaneTabCreationMenu({
  children,
  trigger,
}: Readonly<{
  children: ReactNode;
  trigger: ReactElement;
}>) {
  const openSidePaneTab = useOpenSidePaneTab();

  return (
    <DropdownMenu>
      <DropdownMenuTrigger render={trigger}>{children}</DropdownMenuTrigger>
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
