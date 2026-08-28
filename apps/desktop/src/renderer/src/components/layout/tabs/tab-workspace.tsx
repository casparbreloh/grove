import { useDndContext, useDraggable, useDroppable } from "@dnd-kit/core";
import { Fragment } from "react";
import { Group, Panel, Separator } from "react-resizable-panels";

import { Button } from "@/components/ui/button";
import { useMockGrove, type SplitEdge, type Tab, type TabSplit } from "@/lib/mocks/grove";
import { cn } from "@/lib/utils";
import { TabContent } from "./tab-content";
import { useTabDragState, type TabDragData, type TabDropData } from "./tab-drag-drop";

const sourcePaneMotion =
  "transition-transform duration-200 [transition-timing-function:cubic-bezier(0.77,0,0.175,1)] data-[drag-input=keyboard]:transition-none data-[open=false]:transition-none motion-reduce:transition-none";
const incomingPaneMotion =
  "[transition:transform_200ms_cubic-bezier(0.77,0,0.175,1),opacity_200ms_cubic-bezier(0.23,1,0.32,1)] data-[drag-input=keyboard]:transition-none data-[open=false]:transition-none motion-reduce:[transition:opacity_200ms_cubic-bezier(0.23,1,0.32,1)]";
const previewPaneGeometry =
  "data-[orientation=horizontal]:inset-y-0 data-[orientation=horizontal]:left-0 data-[orientation=horizontal]:w-[calc(50%_-_0.25rem)] data-[orientation=vertical]:inset-x-0 data-[orientation=vertical]:top-0 data-[orientation=vertical]:h-[calc(50%_-_0.25rem)] data-[far=true]:data-[orientation=horizontal]:translate-x-[calc(100%+0.5rem)] data-[far=true]:data-[orientation=vertical]:translate-y-[calc(100%+0.5rem)]";

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

  return (
    <Group
      className="size-full min-h-0 min-w-0"
      id={`tab-split-${tab.tabId}`}
      orientation={tabSplit.orientation}
    >
      {tabSplit.surfaces.map((surface, index) => (
        <Fragment key={surface.tabId}>
          {index > 0 && <SplitSeparator orientation={tabSplit.orientation} />}
          <Panel className="min-h-0 min-w-0" id={`${tab.tabId}-surface-${index}`} minSize="20rem">
            <SplitPane tab={surface} />
          </Panel>
        </Fragment>
      ))}
    </Group>
  );
}

function SingleSurface({ tab }: { tab: Tab }) {
  const { over } = useDndContext();
  const drag = useTabDragState();
  const { tabSplit } = useMockGrove();
  const dropData = over?.data.current as TabDropData | undefined;
  const canSplit =
    drag.kind === "dragging" &&
    drag.source === "tab-list" &&
    drag.draggedTab.tabId !== tab.tabId &&
    tabSplit === undefined;
  const previewEdge =
    canSplit && dropData?.kind === "split-edge" && dropData.targetTabId === tab.tabId
      ? dropData.edge
      : undefined;
  const previewOpen = previewEdge !== undefined;
  const orientation =
    previewEdge === "left" || previewEdge === "right"
      ? "horizontal"
      : previewEdge === "top" || previewEdge === "bottom"
        ? "vertical"
        : "none";
  const incomingFirst = previewEdge === "left" || previewEdge === "top";

  return (
    <section
      className="relative size-full overflow-hidden bg-background data-[open=true]:bg-sidebar"
      data-open={previewOpen}
    >
      <div
        className={cn(
          "absolute overflow-hidden bg-background data-[open=false]:inset-0 data-[open=true]:rounded-xl data-[open=true]:shadow-sm",
          sourcePaneMotion,
          previewPaneGeometry,
        )}
        data-drag-input={drag.input ?? "none"}
        data-far={incomingFirst}
        data-open={previewOpen}
        data-orientation={orientation}
      >
        <TabContent tab={tab} />
      </div>
      {canSplit && <SplitDropTargets targetTabId={tab.tabId} />}
      {drag.kind === "dragging" && canSplit && (
        <div
          aria-hidden="true"
          className={cn(
            "pointer-events-none absolute z-10 flex items-center justify-center overflow-hidden rounded-xl border-2 border-foreground/15 bg-background/80 opacity-0 shadow-sm backdrop-blur-xl data-[open=true]:opacity-100 contrast-more:border-foreground/40 contrast-more:bg-background [@media(prefers-reduced-transparency:reduce)]:bg-background [@media(prefers-reduced-transparency:reduce)]:backdrop-blur-none",
            incomingPaneMotion,
            previewPaneGeometry,
          )}
          data-drag-input={drag.input}
          data-far={!incomingFirst}
          data-open={previewOpen}
          data-orientation={orientation}
        >
          <span className="truncate px-4 text-sm font-medium text-foreground/70 contrast-more:text-foreground">
            {drag.draggedTab.title}
          </span>
        </div>
      )}
    </section>
  );
}

function SplitPane({ tab }: { tab: Tab }) {
  const { attributes, isDragging, listeners, setActivatorNodeRef, setNodeRef } = useDraggable({
    id: `split-pane:${tab.tabId}`,
    data: { kind: "tab", source: "split", tab } satisfies TabDragData,
  });

  return (
    <div
      aria-label={tab.title}
      className="flex size-full min-h-0 flex-col overflow-hidden rounded-xl bg-background shadow-sm"
      ref={setNodeRef}
      role="region"
    >
      <Button
        {...attributes}
        {...listeners}
        className={cn(
          "h-10 w-full shrink-0 touch-none cursor-grab select-none justify-start rounded-none px-3 font-normal active:cursor-grabbing",
          isDragging && "opacity-30",
        )}
        ref={setActivatorNodeRef}
        type="button"
        variant="ghost"
      >
        <span className="truncate">{tab.title}</span>
      </Button>
      <div className="min-h-0 flex-1 overflow-hidden">
        <TabContent tab={tab} />
      </div>
    </div>
  );
}

function SplitSeparator({ orientation }: { orientation: TabSplit["orientation"] }) {
  return (
    <Separator
      aria-label="Resize split"
      className={cn(
        "relative z-10 shrink-0 bg-transparent focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        orientation === "horizontal" ? "w-2 cursor-col-resize" : "h-2 cursor-row-resize",
      )}
    />
  );
}

function SplitDropTargets({ targetTabId }: { targetTabId: string }) {
  return (
    <div aria-hidden="true" className="pointer-events-none absolute inset-0 z-20">
      {(["left", "right", "top", "bottom"] as const).map((edge) => (
        <SplitDropTarget edge={edge} key={edge} targetTabId={targetTabId} />
      ))}
    </div>
  );
}

function SplitDropTarget({ edge, targetTabId }: { edge: SplitEdge; targetTabId: string }) {
  const { setNodeRef } = useDroppable({
    id: `split:${targetTabId}:${edge}`,
    data: { kind: "split-edge", targetTabId, edge } satisfies TabDropData,
  });

  return <div className="absolute inset-0" ref={setNodeRef} />;
}
