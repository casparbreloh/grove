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
      className="size-full min-h-0 min-w-0"
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
              className="size-full overflow-hidden rounded-xl bg-background shadow-sm"
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
  const previewOpen = previewEdge !== undefined;
  const previewOrientation =
    previewEdge === "top" || previewEdge === "bottom" ? "vertical" : "horizontal";
  const sourceFirst = previewEdge === "left" || previewEdge === "top";

  return (
    <section
      className="relative flex size-full min-h-0 min-w-0 overflow-hidden data-[open=true]:gap-2 data-[open=true]:bg-sidebar data-[orientation=vertical]:flex-col"
      data-open={previewOpen}
      data-orientation={previewOrientation}
    >
      <div
        className="relative min-h-0 min-w-0 basis-full shrink-0 overflow-hidden bg-background data-[open=true]:[flex-basis:calc(50%_-_0.25rem)] data-[open=true]:rounded-xl data-[open=true]:shadow-sm data-[source-first=true]:order-2"
        data-open={previewOpen}
        data-source-first={sourceFirst}
      >
        <TabContent tab={tab} />
      </div>
      {canSplit && <SplitDropTargets targetTabId={tab.tabId} />}
      {draggedTab && canSplit && (
        <SplitSourcePreview
          input={input}
          open={previewOpen}
          sourceFirst={sourceFirst}
          tab={draggedTab}
        />
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
  input,
  open,
  sourceFirst,
  tab,
}: {
  input: "pointer" | "keyboard" | undefined;
  open: boolean;
  sourceFirst: boolean;
  tab: Tab;
}) {
  return (
    <div
      aria-hidden="true"
      className="pointer-events-none z-10 flex min-h-0 min-w-0 basis-0 shrink-0 scale-[0.97] items-center justify-center overflow-hidden rounded-xl bg-transparent opacity-0 transition-[transform,opacity] duration-200 [transition-timing-function:cubic-bezier(0.23,1,0.32,1)] data-[drag-input=keyboard]:transition-none data-[open=true]:[flex-basis:calc(50%_-_0.25rem)] data-[open=true]:scale-100 data-[open=true]:border-2 data-[open=true]:border-foreground/15 data-[open=true]:bg-background/80 data-[open=true]:opacity-100 data-[open=true]:shadow-sm data-[open=true]:backdrop-blur-xl data-[source-first=true]:order-1 motion-reduce:scale-100 motion-reduce:transition-opacity contrast-more:border-foreground/40 contrast-more:bg-background [@media(prefers-reduced-transparency:reduce)]:bg-background [@media(prefers-reduced-transparency:reduce)]:backdrop-blur-none"
      data-drag-input={input ?? "none"}
      data-open={open}
      data-source-first={sourceFirst}
    >
      <span className="truncate px-4 text-sm font-medium text-foreground/70 contrast-more:text-foreground">
        {tab.title}
      </span>
    </div>
  );
}
