import {
  closestCenter,
  DndContext,
  DragOverlay,
  KeyboardSensor,
  MeasuringStrategy,
  PointerSensor,
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
  reorderMockTab,
  splitMockTab,
  useMockGrove,
  type SplitEdge,
  type Tab,
} from "@/lib/mocks/grove";
import { focusTab } from "./focus-tab";

export type TabDragData = { kind: "tab" };
export type TabDropData =
  | { kind: "tab" }
  | { kind: "tab-list" }
  | { kind: "split-edge"; targetTabId: string; edge: SplitEdge };

type TabDragState =
  | { kind: "idle"; draggedTab?: never; input?: never }
  | { kind: "dragging"; draggedTab: Tab; input: "pointer" | "keyboard" };

type ActiveTabDrag =
  | { kind: "idle" }
  | { kind: "dragging"; tabId: string; input: "pointer" | "keyboard" };

const TabDragStateContext = createContext<TabDragState | undefined>(undefined);

const tabCollisionDetection: CollisionDetection = (arguments_) => {
  const collisions = closestCenter(arguments_).filter(
    (collision) => collision.id !== arguments_.active.id,
  );
  const { pointerCoordinates } = arguments_;
  if (!pointerCoordinates) return collisions;

  return collisions
    .filter(({ id }) => {
      const rect = arguments_.droppableRects.get(id);
      return (
        rect !== undefined &&
        pointerCoordinates.x >= rect.left &&
        pointerCoordinates.x <= rect.right &&
        pointerCoordinates.y >= rect.top &&
        pointerCoordinates.y <= rect.bottom
      );
    })
    .sort((left, right) => {
      const priority = (kind: unknown) => (kind === "tab" ? 2 : kind === "split-edge" ? 1 : 0);
      return (
        priority(right.data?.droppableContainer.data.current?.kind) -
        priority(left.data?.droppableContainer.data.current?.kind)
      );
    });
};

export function TabDragDropProvider({ children }: { children: ReactNode }) {
  const { tabs } = useMockGrove();
  const [drag, setDrag] = useState<ActiveTabDrag>({ kind: "idle" });
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
      keyboardCodes: { start: ["KeyD"], cancel: ["Escape"], end: ["KeyD"] },
    }),
  );
  const draggedTab =
    drag.kind === "dragging" ? tabs.find(({ tabId }) => tabId === drag.tabId) : undefined;
  const dragState = useMemo<TabDragState>(
    () =>
      drag.kind === "dragging" && draggedTab
        ? { kind: "dragging", draggedTab, input: drag.input }
        : { kind: "idle" },
    [drag, draggedTab],
  );
  const announcements = useMemo<Announcements>(
    () => ({
      onDragStart: ({ active }) => `Picked up ${getTabTitle(tabs, active.id)}.`,
      onDragOver: ({ active, over }) =>
        over
          ? `${getTabTitle(tabs, active.id)} is over ${getDropDescription(tabs, over)}.`
          : `${getTabTitle(tabs, active.id)} is no longer over a drop target.`,
      onDragEnd: ({ active, over }) =>
        over
          ? `Dropped ${getTabTitle(tabs, active.id)} on ${getDropDescription(tabs, over)}.`
          : `Dropped ${getTabTitle(tabs, active.id)} in its original position.`,
      onDragCancel: ({ active }) => `Cancelled dragging ${getTabTitle(tabs, active.id)}.`,
    }),
    [tabs],
  );

  function clearDrag() {
    setDrag({ kind: "idle" });
  }

  function handleDragStart(event: DragStartEvent) {
    setDrag({
      kind: "dragging",
      tabId: String(event.active.id),
      input: event.activatorEvent instanceof KeyboardEvent ? "keyboard" : "pointer",
    });
  }

  function handleDragEnd(event: DragEndEvent) {
    const tabId = String(event.active.id);
    const dropData = event.over?.data.current as TabDropData | undefined;
    clearDrag();
    if (!dropData) return;

    switch (dropData.kind) {
      case "split-edge": {
        const ownerTabId = splitMockTab(tabId, dropData.targetTabId, dropData.edge);
        if (ownerTabId) focusTab(ownerTabId);
        return;
      }
      case "tab":
        reorderMockTab(tabId, String(event.over?.id));
        break;
      case "tab-list":
        reorderMockTab(tabId);
        break;
    }
    focusTab(tabId);
  }

  return (
    <TabDragStateContext value={dragState}>
      <DndContext
        accessibility={{
          announcements,
          screenReaderInstructions: {
            draggable:
              "To pick up a tab, press D. Use the arrow keys to reorder it or move it into the active tab. Press D again to drop, or Escape to cancel.",
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
        <DragOverlay dropAnimation={null}>
          {draggedTab ? <TabDragPreview tab={draggedTab} /> : null}
        </DragOverlay>
      </DndContext>
    </TabDragStateContext>
  );
}

function getTabTitle(tabs: readonly Tab[], tabId: UniqueIdentifier) {
  return tabs.find((tab) => tab.tabId === String(tabId))?.title ?? "tab";
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
  return (
    <div className="flex h-7 w-37.5 items-center rounded-md border bg-popover px-3 text-sm text-popover-foreground shadow-md">
      <span className="truncate">{tab.title}</span>
    </div>
  );
}
