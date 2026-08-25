import { Add01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect } from "react";
import {
  getSidePaneTabIcon,
  renderSidePaneTab,
  sidePaneTabRegistry,
} from "@/components/side-pane/registry";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { usePaneSplit } from "@/components/ui/pane-split";
import { useSidebar } from "@/components/ui/sidebar";
import { TabStrip } from "@/components/ui/tab-strip";
import {
  closeMockSidePaneTab,
  ensureMockSidePaneTab,
  openMockSidePaneTab,
  selectMockSidePaneTab,
  useMockSidePane,
} from "@/lib/mock-side-pane";
import { cn } from "@/lib/utils";

export function SidePane() {
  const { isSidePaneMaximized, isSidePaneOpen } = usePaneSplit();
  const { isMobile, open: isAppSidebarOpen, openMobile: isMobileSidebarOpen } = useSidebar();
  const { activeTabId, tabs } = useMockSidePane();
  const activeTab = tabs.find(({ tabId }) => tabId === activeTabId);
  const needsTitlebarSafeArea =
    isSidePaneMaximized && !(isMobile ? isMobileSidebarOpen : isAppSidebarOpen);

  useEffect(() => {
    if (isSidePaneOpen) ensureMockSidePaneTab();
  }, [isSidePaneOpen]);

  return (
    <aside
      aria-label="Side pane"
      className="flex size-full flex-col border-l bg-background text-foreground"
    >
      <header
        className={cn(
          "flex h-10 shrink-0 items-center pr-20 pl-1.5 [-webkit-app-region:drag]",
          needsTitlebarSafeArea && "pl-[calc(var(--desktop-header-safe-area-left)+0.375rem)]",
        )}
      >
        {activeTab !== undefined && activeTab.kind !== "new" && (
          <TabStrip
            activeTabId={activeTabId}
            label="Side pane tabs"
            onClose={closeMockSidePaneTab}
            onSelect={selectMockSidePaneTab}
            tabs={tabs.map((tab) => ({
              id: tab.tabId,
              icon: getSidePaneTabIcon(tab),
              title: tab.title,
            }))}
          >
            <NewSidePaneTabMenu />
          </TabStrip>
        )}
      </header>
      <div className="relative min-h-0 flex-1">
        {tabs.map((tab) => (
          <div
            aria-hidden={tab.tabId !== activeTabId}
            aria-label={tab.kind === "new" ? tab.title : undefined}
            aria-labelledby={tab.kind === "new" ? undefined : `${tab.tabId}-tab`}
            className={
              tab.tabId === activeTabId
                ? "absolute inset-0 size-full"
                : "invisible pointer-events-none absolute inset-0 size-full"
            }
            id={`${tab.tabId}-panel`}
            inert={tab.tabId !== activeTabId}
            key={tab.tabId}
            role="tabpanel"
          >
            {renderSidePaneTab(tab)}
          </div>
        ))}
      </div>
    </aside>
  );
}

function NewSidePaneTabMenu() {
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
        {sidePaneTabRegistry.map((registration) => (
          <DropdownMenuItem
            key={registration.kind}
            onClick={() => openMockSidePaneTab(registration.create())}
          >
            <HugeiconsIcon icon={registration.icon} strokeWidth={2} />
            {registration.label}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
