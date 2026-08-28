import {
  closestCenter,
  DndContext,
  DragOverlay,
  KeyboardSensor,
  MeasuringStrategy,
  PointerSensor,
  useDndContext,
  useSensor,
  useSensors,
  type Announcements,
  type CollisionDetection,
  type DragEndEvent,
  type DragStartEvent,
  type Over,
  type UniqueIdentifier,
} from "@dnd-kit/core";
import { sortableKeyboardCoordinates } from "@dnd-kit/sortable";
import { createContext, useContext, useMemo, useState, type ReactNode } from "react";

import {
  moveMockSplitTabToTabList,
  reorderMockTab,
  splitMockTab,
  useMockGrove,
  type SplitEdge,
  type Tab,
} from "@/lib/mocks/grove";
import { focusTab } from "./focus-tab";

export type TabDragData = { kind: "tab"; source: "split" | "tab-list"; tab: Tab };
export type TabDropData =
  | { kind: "tab" }
  | { kind: "tab-list" }
  | { kind: "split-edge"; targetTabId: string; edge: SplitEdge };

type TabDragState =
  | { kind: "idle"; draggedTab?: never; input?: never }
  | {
      kind: "dragging";
      draggedTab: Tab;
      input: "keyboard" | "pointer";
      source: TabDragData["source"];
    };

const TabDragStateContext = createContext<TabDragState | undefined>(undefined);

const tabCollisionDetection: CollisionDetection = (arguments_) => {
  const collisions = closestCenter(arguments_).filter(
    (collision) => collision.id !== arguments_.active.id,
  );
  const coordinates = arguments_.pointerCoordinates ?? {
    x: arguments_.collisionRect.left + arguments_.collisionRect.width / 2,
    y: arguments_.collisionRect.top + arguments_.collisionRect.height / 2,
  };
  const containing = collisions.filter(({ id }) => {
    const rect = arguments_.droppableRects.get(id);
    return (
      rect !== undefined &&
      coordinates.x >= rect.left &&
      coordinates.x <= rect.right &&
      coordinates.y >= rect.top &&
      coordinates.y <= rect.bottom
    );
  });
  const splitCollisions = containing.filter(
    (collision) => collision.data?.droppableContainer.data.current?.kind === "split-edge",
  );
  const splitRect = splitCollisions[0]
    ? arguments_.droppableRects.get(splitCollisions[0].id)
    : undefined;

  if (splitRect) {
    const edge = getNearestEdge(coordinates, splitRect);
    const collision = splitCollisions.find(
      (candidate) => candidate.data?.droppableContainer.data.current?.edge === edge,
    );
    if (collision) return [collision];
  }

  return collisions;
};

function getNearestEdge(
  point: { x: number; y: number },
  rect: { top: number; right: number; bottom: number; left: number },
): SplitEdge {
  const distances: readonly [SplitEdge, number][] = [
    ["left", point.x - rect.left],
    ["right", rect.right - point.x],
    ["top", point.y - rect.top],
    ["bottom", rect.bottom - point.y],
  ];
  return distances.reduce((nearest, candidate) =>
    candidate[1] < nearest[1] ? candidate : nearest,
  )[0];
}

export function TabDragDropProvider({ children }: { children: ReactNode }) {
  const { tabs } = useMockGrove();
  const [drag, setDrag] = useState<TabDragState>({ kind: "idle" });
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
      keyboardCodes: { start: ["KeyD"], cancel: ["Escape"], end: ["KeyD"] },
    }),
  );
  const draggedTab = drag.kind === "dragging" ? drag.draggedTab : undefined;
  const announcements = useMemo<Announcements>(
    () => ({
      onDragStart: ({ active }) => `Picked up ${getDragTitle(tabs, active)}.`,
      onDragOver: ({ active, over }) =>
        over
          ? `${getDragTitle(tabs, active)} is over ${getDropDescription(tabs, over)}.`
          : `${getDragTitle(tabs, active)} is no longer over a drop target.`,
      onDragEnd: ({ active, over }) =>
        over
          ? `Dropped ${getDragTitle(tabs, active)} on ${getDropDescription(tabs, over)}.`
          : `Dropped ${getDragTitle(tabs, active)} in its original position.`,
      onDragCancel: ({ active }) => `Cancelled dragging ${getDragTitle(tabs, active)}.`,
    }),
    [tabs],
  );

  function clearDrag() {
    setDrag({ kind: "idle" });
  }

  function handleDragStart(event: DragStartEvent) {
    const data = event.active.data.current as TabDragData | undefined;
    if (data?.kind !== "tab") return;
    setDrag({
      draggedTab: data.tab,
      kind: "dragging",
      input: event.activatorEvent instanceof KeyboardEvent ? "keyboard" : "pointer",
      source: data.source,
    });
  }

  function handleDragEnd(event: DragEndEvent) {
    const dragData = event.active.data.current as TabDragData | undefined;
    const dropData = event.over?.data.current as TabDropData | undefined;
    clearDrag();
    if (dragData?.kind !== "tab" || !dropData) return;

    const { source, tab } = dragData;

    switch (dropData.kind) {
      case "split-edge": {
        if (source !== "tab-list") return;
        const ownerTabId = splitMockTab(tab.tabId, dropData.targetTabId, dropData.edge);
        if (ownerTabId) focusTab(ownerTabId);
        return;
      }
      case "tab": {
        const overTabId = String(event.over?.id);
        if (source === "split") moveMockSplitTabToTabList(tab.tabId, overTabId);
        else reorderMockTab(tab.tabId, overTabId);
        break;
      }
      case "tab-list": {
        if (source === "split") moveMockSplitTabToTabList(tab.tabId);
        else reorderMockTab(tab.tabId);
        break;
      }
    }
    focusTab(tab.tabId);
  }

  return (
    <TabDragStateContext value={drag}>
      <DndContext
        accessibility={{
          announcements,
          screenReaderInstructions: {
            draggable:
              "To pick up a focused tab, press D. Use the arrow keys to move it. Press D again to drop, or Escape to cancel.",
          },
        }}
        collisionDetection={tabCollisionDetection}
        measuring={{ droppable: { strategy: MeasuringStrategy.Always } }}
        onDragCancel={clearDrag}
        onDragEnd={handleDragEnd}
        onDragStart={handleDragStart}
        sensors={sensors}
      >
        {children}
        <DragOverlay adjustScale={false} dropAnimation={null}>
          {draggedTab ? <TabDragPreview tab={draggedTab} /> : null}
        </DragOverlay>
      </DndContext>
    </TabDragStateContext>
  );
}

function getTabTitle(tabs: readonly Tab[], tabId: UniqueIdentifier) {
  return tabs.find((tab) => tab.tabId === String(tabId))?.title ?? "tab";
}

function getDragTitle(
  tabs: readonly Tab[],
  active: { data: { current?: Record<string, unknown> }; id: UniqueIdentifier },
) {
  const data = active.data.current as TabDragData | undefined;
  return data?.kind === "tab" ? data.tab.title : getTabTitle(tabs, active.id);
}

function getDropDescription(tabs: readonly Tab[], over: Over) {
  const dropData = over.data.current as TabDropData | undefined;
  if (dropData?.kind === "split-edge") {
    return `the ${dropData.edge} half of ${getTabTitle(tabs, dropData.targetTabId)}`;
  }
  if (dropData?.kind === "tab") return getTabTitle(tabs, over.id);
  return "the end of the tab list";
}

export function useTabDragState() {
  const context = useContext(TabDragStateContext);
  if (!context) throw new Error("useTabDragState must be used within TabDragDropProvider");
  return context;
}

function TabDragPreview({ tab }: { tab: Tab }) {
  const { over } = useDndContext();
  const dropData = over?.data.current as TabDropData | undefined;

  return (
    <div
      className="flex h-7 w-37.5 select-none items-center rounded-md bg-accent px-3 pr-7 text-sm text-accent-foreground shadow-sm data-[split=true]:opacity-0"
      data-split={dropData?.kind === "split-edge"}
    >
      <span className="truncate">{tab.title}</span>
    </div>
  );
}
