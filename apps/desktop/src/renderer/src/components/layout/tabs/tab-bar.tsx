import { useDroppable } from "@dnd-kit/core";
import { horizontalListSortingStrategy, SortableContext, useSortable } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { Cancel01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";

import { Button } from "@/components/ui/button";
import {
  closeMockTab,
  getPaneTabs,
  selectMockTab,
  useMockGrove,
  type PaneId,
  type Tab,
  type TabPane,
} from "@/lib/mocks/grove";
import { cn } from "@/lib/utils";
import { NewTabMenu } from "./new-tab-menu";
import {
  focusTab,
  useHorizontalPaneLayout,
  type TabDragData,
  type TabDropData,
} from "./tab-drag-drop";

export function TopPaneHeaders() {
  const { tabLayout } = useMockGrove();
  const [horizontalLayout] = useHorizontalPaneLayout();
  const firstPane = tabLayout.panes[0];

  if (tabLayout.kind === "single") {
    return <PaneTabBar ariaLabel="Open tabs" pane={firstPane} />;
  }

  if (tabLayout.orientation === "vertical") {
    return <PaneTabBar ariaLabel="Top pane tabs" pane={firstPane} />;
  }

  const secondPane = tabLayout.panes[1];
  const splitKey = getSplitKey(tabLayout.panes);
  const layout = horizontalLayout?.splitKey === splitKey ? horizontalLayout.layout : undefined;
  const firstSize = layout?.[firstPane.paneId] ?? 50;
  const secondSize = layout?.[secondPane.paneId] ?? 50;

  return (
    <div
      className="grid h-full min-w-0"
      style={{ gridTemplateColumns: `minmax(0, ${firstSize}fr) 1px minmax(0, ${secondSize}fr)` }}
    >
      <PaneTabBar ariaLabel="Left pane tabs" pane={firstPane} />
      <div aria-hidden="true" className="bg-border" />
      <PaneTabBar ariaLabel="Right pane tabs" pane={secondPane} />
    </div>
  );
}

export function PaneTabBar({
  ariaLabel = "Open tabs",
  pane,
}: {
  ariaLabel?: string;
  pane: TabPane;
}) {
  const { tabs } = useMockGrove();
  const paneTabs = getPaneTabs(pane, tabs);

  return (
    <div
      className="flex h-10 min-w-0 items-center gap-1 [-webkit-app-region:drag] [&_button]:[-webkit-app-region:no-drag]"
      onFocusCapture={() => selectMockTab(pane.activeTabId)}
    >
      <nav
        aria-label={ariaLabel}
        className="no-scrollbar scroll-fade-x scroll-fade-6 flex h-full min-w-0 flex-1 items-center gap-1 overflow-x-auto"
      >
        <SortableContext items={[...pane.tabIds]} strategy={horizontalListSortingStrategy}>
          {paneTabs.map((tab) => (
            <SortableTab
              isActive={tab.tabId === pane.activeTabId}
              key={tab.tabId}
              paneId={pane.paneId}
              tab={tab}
              totalTabCount={tabs.length}
            />
          ))}
        </SortableContext>
        <PaneDropTail paneId={pane.paneId} />
      </nav>
      <NewTabMenu className="mr-2" paneId={pane.paneId} />
    </div>
  );
}

function SortableTab({
  isActive,
  paneId,
  tab,
  totalTabCount,
}: Readonly<{
  isActive: boolean;
  paneId: PaneId;
  tab: Tab;
  totalTabCount: number;
}>) {
  const { attributes, isDragging, listeners, setActivatorNodeRef, setNodeRef, transform } =
    useSortable({ id: tab.tabId, data: { kind: "tab", paneId } satisfies TabDragData });

  function closeTab() {
    const nextTabId = closeMockTab(tab.tabId);
    if (nextTabId) focusTab(nextTabId);
  }

  return (
    <div
      className={cn(
        "group/tab relative flex h-7 w-37.5 shrink-0 items-center transition-transform duration-200 ease-out motion-reduce:transition-none",
        isDragging && "opacity-30",
      )}
      ref={setNodeRef}
      role="presentation"
      style={{ transform: CSS.Transform.toString(transform) }}
    >
      <Button
        {...attributes}
        {...listeners}
        aria-controls={`${tab.tabId}-panel`}
        aria-pressed={isActive}
        className={cn(
          "h-full w-full cursor-default justify-start truncate pr-7 font-normal transition-colors duration-75 group-hover/tab:bg-accent group-hover/tab:text-accent-foreground focus-visible:ring-inset focus-visible:ring-offset-0 motion-reduce:transition-none",
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

function PaneDropTail({ paneId }: { paneId: PaneId }) {
  const { isOver, setNodeRef } = useDroppable({
    id: `pane:${paneId}`,
    data: { kind: "pane", paneId } satisfies TabDropData,
  });

  return (
    <div
      aria-hidden="true"
      className={cn("h-7 min-w-6 flex-1 rounded-md", isOver && "bg-accent")}
      ref={setNodeRef}
    />
  );
}

export function getSplitKey(panes: readonly TabPane[]) {
  return panes.map(({ paneId }) => paneId).join(":");
}
