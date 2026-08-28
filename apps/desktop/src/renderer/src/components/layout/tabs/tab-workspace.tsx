import { useDndContext, useDroppable } from "@dnd-kit/core";
import { Fragment } from "react";
import { Group, Panel, Separator, type Layout } from "react-resizable-panels";

import { useMockGrove, type SplitEdge, type Tab, type TabSplit } from "@/lib/mocks/grove";
import { cn } from "@/lib/utils";
import { TabContent } from "./tab-content";
import { useTabDragState, type TabDropData } from "./tab-drag-drop";

export function TabWorkspace() {
  const { activeTabId, tabs, tabSplit } = useMockGrove();

  return (
    <div className="min-h-0 flex-1">
      {tabs.map((tab) => (
        <div
          aria-hidden={tab.tabId !== activeTabId}
          className={tab.tabId === activeTabId ? "h-full" : "hidden"}
          id={`${tab.tabId}-panel`}
          inert={tab.tabId !== activeTabId}
          key={tab.tabId}
        >
          <TabSurface
            tab={tab}
            tabSplit={tabSplit?.ownerTabId === tab.tabId ? tabSplit : undefined}
          />
        </div>
      ))}
    </div>
  );
}

function TabSurface({ tab, tabSplit }: { tab: Tab; tabSplit: TabSplit | undefined }) {
  if (!tabSplit) return <SingleSurface tab={tab} />;

  const defaultLayout: Layout = {
    [`${tab.tabId}-surface-0`]: 50,
    [`${tab.tabId}-surface-1`]: 50,
  };

  return (
    <Group
      className="size-full min-h-0 min-w-0 p-2"
      defaultLayout={defaultLayout}
      id={`tab-split-${tab.tabId}`}
      orientation={tabSplit.orientation}
    >
      {tabSplit.surfaces.map((surface, index) => (
        <Fragment key={surface.tabId}>
          {index > 0 && <SplitSeparator orientation={tabSplit.orientation} />}
          <Panel className="min-h-0 min-w-0" id={`${tab.tabId}-surface-${index}`} minSize="20%">
            <div
              aria-label={surface.title}
              className="size-full overflow-hidden rounded-xl border bg-background/55 shadow-xs backdrop-blur-sm"
              role="region"
            >
              <TabContent tab={surface} />
            </div>
          </Panel>
        </Fragment>
      ))}
    </Group>
  );
}

function SingleSurface({ tab }: { tab: Tab }) {
  const { over } = useDndContext();
  const { draggedTab, input } = useTabDragState();
  const { tabSplit } = useMockGrove();
  const dropData = over?.data.current as TabDropData | undefined;
  const canSplit =
    draggedTab !== undefined && draggedTab.tabId !== tab.tabId && tabSplit === undefined;
  const previewEdge =
    canSplit && dropData?.kind === "split-edge" && dropData.targetTabId === tab.tabId
      ? dropData.edge
      : undefined;

  return (
    <section className="relative size-full min-h-0 min-w-0 overflow-hidden">
      <div
        className="size-full transition-[clip-path,transform] duration-200 [clip-path:inset(0_round_var(--radius-xl))] [transition-timing-function:cubic-bezier(0.77,0,0.175,1)] data-[drag-input=keyboard]:transition-none data-[preview=bottom]:-translate-y-1 data-[preview=bottom]:[clip-path:inset(0_0_calc(50%+0.25rem)_0_round_var(--radius-xl))] data-[preview=left]:translate-x-1 data-[preview=left]:[clip-path:inset(0_0_0_calc(50%+0.25rem)_round_var(--radius-xl))] data-[preview=right]:-translate-x-1 data-[preview=right]:[clip-path:inset(0_calc(50%+0.25rem)_0_0_round_var(--radius-xl))] data-[preview=top]:translate-y-1 data-[preview=top]:[clip-path:inset(calc(50%+0.25rem)_0_0_0_round_var(--radius-xl))] motion-reduce:transform-none motion-reduce:[transition-property:opacity]"
        data-drag-input={input ?? "none"}
        data-preview={previewEdge ?? "none"}
      >
        <TabContent tab={tab} />
      </div>
      {canSplit && <SplitDropTargets targetTabId={tab.tabId} />}
      {draggedTab && canSplit && (
        <SplitSourcePreview edge={previewEdge} input={input} tab={draggedTab} />
      )}
    </section>
  );
}

function SplitSeparator({ orientation }: { orientation: TabSplit["orientation"] }) {
  return (
    <Separator
      aria-label="Resize split"
      className={cn(
        "relative z-10 shrink-0 bg-transparent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        "after:absolute after:rounded-full after:bg-border",
        orientation === "horizontal"
          ? "w-2 cursor-col-resize after:inset-y-2 after:left-1/2 after:w-px"
          : "h-2 cursor-row-resize after:inset-x-2 after:top-1/2 after:h-px",
      )}
    />
  );
}

function SplitDropTargets({ targetTabId }: { targetTabId: string }) {
  return (
    <div
      aria-hidden="true"
      className="pointer-events-none absolute inset-2 z-20 grid grid-cols-4 grid-rows-4"
    >
      <SplitDropTarget className="col-start-1 row-span-4" edge="left" targetTabId={targetTabId} />
      <SplitDropTarget className="col-start-4 row-span-4" edge="right" targetTabId={targetTabId} />
      <SplitDropTarget
        className="col-span-2 col-start-2 row-start-1"
        edge="top"
        targetTabId={targetTabId}
      />
      <SplitDropTarget
        className="col-span-2 col-start-2 row-start-4"
        edge="bottom"
        targetTabId={targetTabId}
      />
    </div>
  );
}

function SplitDropTarget({
  className,
  edge,
  targetTabId,
}: {
  className: string;
  edge: SplitEdge;
  targetTabId: string;
}) {
  const { setNodeRef } = useDroppable({
    id: `split:${targetTabId}:${edge}`,
    data: { kind: "split-edge", targetTabId, edge } satisfies TabDropData,
  });

  return <div className={className} ref={setNodeRef} />;
}

function SplitSourcePreview({
  edge,
  input,
  tab,
}: {
  edge: SplitEdge | undefined;
  input: "pointer" | "keyboard" | undefined;
  tab: Tab;
}) {
  return (
    <div
      aria-hidden="true"
      className="pointer-events-none absolute inset-0 z-10 bg-card/80 opacity-0 shadow-sm ring-1 ring-border backdrop-blur-md transition-[clip-path,opacity] duration-200 [clip-path:inset(50%_round_var(--radius-xl))] [transition-timing-function:cubic-bezier(0.77,0,0.175,1)] data-[drag-input=keyboard]:transition-none data-[edge=bottom]:opacity-100 data-[edge=bottom]:[clip-path:inset(calc(50%+0.25rem)_0_0_0_round_var(--radius-xl))] data-[edge=left]:opacity-100 data-[edge=left]:[clip-path:inset(0_calc(50%+0.25rem)_0_0_round_var(--radius-xl))] data-[edge=right]:opacity-100 data-[edge=right]:[clip-path:inset(0_0_0_calc(50%+0.25rem)_round_var(--radius-xl))] data-[edge=top]:opacity-100 data-[edge=top]:[clip-path:inset(0_0_calc(50%+0.25rem)_0_round_var(--radius-xl))] motion-reduce:[transition-property:opacity]"
      data-drag-input={input ?? "none"}
      data-edge={edge ?? "none"}
    >
      <span
        className="absolute -translate-x-1/2 -translate-y-1/2 rounded-md border bg-popover/90 px-3 py-1.5 text-sm text-popover-foreground shadow-sm data-[edge=bottom]:top-3/4 data-[edge=bottom]:left-1/2 data-[edge=left]:top-1/2 data-[edge=left]:left-1/4 data-[edge=right]:top-1/2 data-[edge=right]:left-3/4 data-[edge=top]:top-1/4 data-[edge=top]:left-1/2"
        data-edge={edge ?? "none"}
      >
        {tab.title}
      </span>
    </div>
  );
}
