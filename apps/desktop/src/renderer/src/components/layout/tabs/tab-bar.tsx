import { Cancel01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

import { Button } from "@/components/ui/button";
import { closeMockTab, selectMockTab, useMockGrove, type Tab } from "@/lib/mocks/grove";
import { cn } from "@/lib/utils";
import { NewTabMenu } from "./new-tab-menu";

function focusTab(tabId: string) {
  window.requestAnimationFrame(() => document.getElementById(`${tabId}-tab`)?.focus());
}

export function TabBar() {
  const { activeTabId, tabs } = useMockGrove();

  return (
    <div className="flex h-full min-w-0 items-center gap-1">
      <nav
        aria-label="Open tabs"
        className="no-scrollbar scroll-fade-x scroll-fade-6 flex h-full min-w-0 items-center gap-1 overflow-x-auto"
      >
        {tabs.map((tab) => (
          <TabItem isActive={tab.tabId === activeTabId} key={tab.tabId} tab={tab} tabs={tabs} />
        ))}
      </nav>
      <NewTabMenu className="mr-2" />
    </div>
  );
}

function TabItem({
  isActive,
  tab,
  tabs,
}: Readonly<{
  isActive: boolean;
  tab: Tab;
  tabs: readonly Tab[];
}>) {
  function closeTab() {
    const nextTabId = closeMockTab(tab.tabId);
    if (nextTabId) focusTab(nextTabId);
  }

  return (
    <div className="group/tab relative flex h-7 w-37.5 shrink-0 items-center" role="presentation">
      <Button
        aria-controls={`${tab.tabId}-panel`}
        aria-pressed={isActive}
        className={cn(
          "h-full w-full cursor-default justify-start truncate pr-7 font-normal transition-colors duration-75 group-hover/tab:bg-accent group-hover/tab:text-accent-foreground focus-visible:ring-inset focus-visible:ring-offset-0 motion-reduce:transition-none",
          isActive && "bg-accent text-accent-foreground",
        )}
        id={`${tab.tabId}-tab`}
        onClick={() => selectMockTab(tab.tabId)}
        type="button"
        variant="ghost"
      >
        {tab.title}
      </Button>
      {tabs.length > 1 && (
        <Button
          aria-label={`Close ${tab.title}`}
          className={cn(
            "pointer-events-none absolute top-1/2 right-0 -translate-y-1/2 text-muted-foreground opacity-0 transition-none group-hover/tab:pointer-events-auto group-hover/tab:opacity-100 group-focus-within/tab:pointer-events-auto group-focus-within/tab:opacity-100 hover:bg-transparent hover:text-foreground focus-visible:pointer-events-auto focus-visible:opacity-100",
            isActive && "pointer-events-auto opacity-100",
          )}
          onClick={closeTab}
          size="icon-sm"
          type="button"
          variant="ghost"
        >
          <HugeiconsIcon className="size-[var(--icon-xs)]" icon={Cancel01Icon} strokeWidth={2} />
        </Button>
      )}
    </div>
  );
}
