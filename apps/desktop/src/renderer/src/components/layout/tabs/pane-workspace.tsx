import { useDndContext, useDroppable } from "@dnd-kit/core";
import { Fragment } from "react";
import { Group, Panel, Separator, type Layout } from "react-resizable-panels";

import {
  getPaneTabs,
  useMockGrove,
  type PaneId,
  type SplitEdge,
  type TabPane,
} from "@/lib/mocks/grove";
import { cn } from "@/lib/utils";
import { PaneTabBar, getSplitKey } from "./tab-bar";
import { TabContent } from "./tab-content";
import { useHorizontalPaneLayout, type TabDropData } from "./tab-drag-drop";

export function PaneWorkspace() {
  const { tabLayout } = useMockGrove();
  const [, setHorizontalLayout] = useHorizontalPaneLayout();

  if (tabLayout.kind === "single") {
    return <PaneContent pane={tabLayout.panes[0]} />;
  }

  const { orientation, panes } = tabLayout;
  const splitKey = getSplitKey(panes);
  const defaultLayout: Layout = {
    [panes[0].paneId]: 50,
    [panes[1].paneId]: 50,
  };

  return (
    <Group
      className="h-full min-h-0 min-w-0"
      defaultLayout={defaultLayout}
      id={`tab-panes-${orientation}`}
      onLayoutChange={
        orientation === "horizontal"
          ? (layout) => setHorizontalLayout({ splitKey, layout })
          : undefined
      }
      orientation={orientation}
    >
      {panes.map((pane, index) => (
        <Fragment key={pane.paneId}>
          {index > 0 && <PaneSeparator orientation={orientation} />}
          <Panel className="min-h-0 min-w-0" id={pane.paneId} minSize="20%">
            <div className="flex size-full min-h-0 min-w-0 flex-col">
              {orientation === "vertical" && index > 0 && (
                <div className="shrink-0 border-b">
                  <PaneTabBar ariaLabel="Bottom pane tabs" pane={pane} />
                </div>
              )}
              <PaneContent pane={pane} />
            </div>
          </Panel>
        </Fragment>
      ))}
    </Group>
  );
}

function PaneSeparator({ orientation }: { orientation: "horizontal" | "vertical" }) {
  return (
    <Separator
      aria-label="Resize tab panes"
      className={cn(
        "relative z-10 shrink-0 bg-border focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        orientation === "horizontal"
          ? "w-px cursor-col-resize after:absolute after:inset-y-0 after:-inset-x-1"
          : "h-px cursor-row-resize after:absolute after:inset-x-0 after:-inset-y-1",
      )}
    />
  );
}

function PaneContent({ pane }: { pane: TabPane }) {
  const { active } = useDndContext();
  const { tabLayout, tabs } = useMockGrove();
  const paneTabs = getPaneTabs(pane, tabs);
  const canSplit = tabLayout.kind === "single" && pane.tabIds.length > 1;

  return (
    <section className="relative min-h-0 min-w-0 flex-1" data-pane={pane.paneId}>
      {paneTabs.map((tab) => (
        <div
          aria-hidden={tab.tabId !== pane.activeTabId}
          className={tab.tabId === pane.activeTabId ? "h-full" : "hidden"}
          id={`${tab.tabId}-panel`}
          inert={tab.tabId !== pane.activeTabId}
          key={tab.tabId}
        >
          <TabContent tab={tab} />
        </div>
      ))}
      {canSplit && <SplitDropZones isVisible={active !== null} paneId={pane.paneId} />}
    </section>
  );
}

function SplitDropZones({ isVisible, paneId }: { isVisible: boolean; paneId: PaneId }) {
  return (
    <div
      aria-hidden={!isVisible}
      className="pointer-events-none absolute inset-3 z-20 grid grid-cols-4 grid-rows-4 gap-2 opacity-0 transition-opacity duration-75 motion-reduce:transition-none data-[visible=true]:pointer-events-auto data-[visible=true]:opacity-100"
      data-visible={isVisible}
    >
      <SplitDropZone className="col-start-1 row-span-4" edge="left" paneId={paneId} />
      <SplitDropZone className="col-start-4 row-span-4" edge="right" paneId={paneId} />
      <SplitDropZone className="col-span-2 col-start-2 row-start-1" edge="top" paneId={paneId} />
      <SplitDropZone className="col-span-2 col-start-2 row-start-4" edge="bottom" paneId={paneId} />
    </div>
  );
}

function SplitDropZone({
  className,
  edge,
  paneId,
}: {
  className: string;
  edge: SplitEdge;
  paneId: PaneId;
}) {
  const { isOver, setNodeRef } = useDroppable({
    id: `split:${paneId}:${edge}`,
    data: { kind: "split-edge", paneId, edge } satisfies TabDropData,
  });

  return (
    <div
      className={cn(
        "rounded-md border bg-background/40 backdrop-blur-sm transition-colors duration-75 motion-reduce:transition-none",
        isOver && "border-ring bg-accent/90",
        className,
      )}
      ref={setNodeRef}
    >
      <span className="sr-only">Split {edge}</span>
    </div>
  );
}
