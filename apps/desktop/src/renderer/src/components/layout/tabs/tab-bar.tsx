import { useDroppable } from "@dnd-kit/core";
import { horizontalListSortingStrategy, SortableContext, useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Cancel01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

import { Button } from "@/components/ui/button";
import { closeMockTab, selectMockTab, useMockGrove, type Tab } from "@/lib/mocks/grove";
import { cn } from "@/lib/utils";
import { focusTab } from "./focus-tab";
import { NewTabMenu } from "./new-tab-menu";
import { type TabDragData, type TabDropData, useTabDragState } from "./tab-drag-drop";

export function TabBar() {
  const { activeTabId, tabs } = useMockGrove();

  return (
    <div className="flex h-full min-w-0 flex-1 items-center gap-1 [-webkit-app-region:drag] [&_button]:[-webkit-app-region:no-drag]">
      <nav
        aria-label="Open tabs"
        className="no-scrollbar scroll-fade-x scroll-fade-6 relative flex h-full min-w-0 flex-[0_1_auto] items-center gap-1 overflow-x-auto"
      >
        <SortableContext
          items={tabs.map(({ tabId }) => tabId)}
          strategy={horizontalListSortingStrategy}
        >
          {tabs.map((tab) => (
            <SortableTab
              isActive={tab.tabId === activeTabId}
              key={tab.tabId}
              tab={tab}
              totalTabCount={tabs.length}
            />
          ))}
        </SortableContext>
        <TabListDropTail />
      </nav>
      <NewTabMenu className="mr-1.5" />
    </div>
  );
}

function SortableTab({
  isActive,
  tab,
  totalTabCount,
}: Readonly<{
  isActive: boolean;
  tab: Tab;
  totalTabCount: number;
}>) {
  const { input } = useTabDragState();
  const {
    attributes,
    isDragging,
    listeners,
    setActivatorNodeRef,
    setNodeRef,
    transform,
    transition,
  } = useSortable({
    id: tab.tabId,
    data: { kind: "tab", tab } satisfies TabDragData,
    transition: { duration: 150, easing: "cubic-bezier(0.77, 0, 0.175, 1)" },
  });

  function closeTab() {
    const nextTabId = closeMockTab(tab.tabId);
    if (nextTabId) focusTab(nextTabId);
  }

  return (
    <div
      className={cn(
        "group/tab relative flex h-7 w-37.5 shrink-0 items-center motion-reduce:transition-none!",
        isDragging && "opacity-30",
      )}
      ref={setNodeRef}
      role="presentation"
      style={{
        transform: CSS.Transform.toString(transform),
        transition: input === "keyboard" ? undefined : transition,
      }}
    >
      <Button
        {...attributes}
        {...listeners}
        aria-controls={`${tab.tabId}-panel`}
        aria-pressed={isActive}
        className={cn(
          "h-full w-full touch-none select-none justify-start truncate pr-7 font-normal transition-colors duration-75 group-hover/tab:bg-accent group-hover/tab:text-accent-foreground focus-visible:ring-inset focus-visible:ring-offset-0 motion-reduce:transition-none",
          isActive && "bg-accent text-accent-foreground",
        )}
        id={`${tab.tabId}-tab`}
        onClick={() => selectMockTab(tab.tabId)}
        ref={setActivatorNodeRef}
        type="button"
        variant="ghost"
      >
        {tab.title}
      </Button>
      {totalTabCount > 1 && (
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

function TabListDropTail() {
  const { setNodeRef } = useDroppable({
    id: "tab-list-tail",
    data: { kind: "tab-list" } satisfies TabDropData,
  });

  return (
    <div
      aria-hidden="true"
      className="pointer-events-none absolute inset-y-1.5 right-0 w-6"
      ref={setNodeRef}
    />
  );
}
