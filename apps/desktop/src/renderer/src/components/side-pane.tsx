import { Add01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect } from "react";
import { renderSidePaneTab, sidePaneTabRegistry } from "@/components/side-pane/registry";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { usePaneSplit } from "@/components/ui/pane-split";
import { TabStrip } from "@/components/ui/tab-strip";
import {
  closeMockSidePaneTab,
  ensureMockSidePaneTab,
  openMockSidePaneTab,
  selectMockSidePaneTab,
  useMockSidePane,
} from "@/lib/mock-side-pane";

export function SidePane() {
  const { isSidePaneMaximized, isSidePaneOpen } = usePaneSplit();
  const { activeTabId, tabs } = useMockSidePane();

  useEffect(() => {
    if (isSidePaneOpen) ensureMockSidePaneTab();
  }, [isSidePaneOpen]);

  return (
    <aside
      aria-label="Side pane"
      className="flex size-full flex-col border-l bg-background text-foreground"
    >
      <header
        className="flex h-10 shrink-0 items-center pr-20 pl-3 [-webkit-app-region:drag] data-[maximized=true]:pl-[calc(var(--desktop-header-safe-area-left)+0.5rem)]"
        data-maximized={isSidePaneMaximized}
      >
        <TabStrip
          activeTabId={activeTabId}
          label="Side pane tabs"
          onClose={closeMockSidePaneTab}
          onSelect={selectMockSidePaneTab}
          tabs={tabs.map(({ tabId, title }) => ({ id: tabId, title }))}
        >
          <NewSidePaneTabMenu />
        </TabStrip>
      </header>
      <div className="min-h-0 flex-1">
        {tabs.map((tab) => (
          <div
            aria-labelledby={`${tab.tabId}-tab`}
            className={tab.tabId === activeTabId ? "h-full" : "hidden"}
            id={`${tab.tabId}-panel`}
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
