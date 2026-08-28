import {
  closestCenter,
  DndContext,
  DragOverlay,
  KeyboardSensor,
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

import { reorderMockTab, useMockGrove, type Tab } from "@/lib/mocks/grove";
import { focusTab } from "./focus-tab";

export type TabDragData = { kind: "tab"; tab: Tab };
export type TabDropData = { kind: "tab" } | { kind: "tab-list" };

type TabDragState =
  | { kind: "idle"; draggedTab?: never; input?: never }
  | {
      kind: "dragging";
      draggedTab: Tab;
      input: "keyboard" | "pointer";
    };

const TabDragStateContext = createContext<TabDragState | undefined>(undefined);

const tabCollisionDetection: CollisionDetection = (arguments_) => {
  const pointerY = arguments_.pointerCoordinates?.y;
  const isOverTabRow = [...arguments_.droppableRects.values()].some(
    (rect) => pointerY === undefined || (pointerY >= rect.top && pointerY <= rect.bottom),
  );

  return isOverTabRow ? closestCenter(arguments_) : [];
};

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
    });
  }

  function handleDragEnd(event: DragEndEvent) {
    const dragData = event.active.data.current as TabDragData | undefined;
    const dropData = event.over?.data.current as TabDropData | undefined;
    clearDrag();
    if (dragData?.kind !== "tab" || !dropData) return;

    const overTabId = dropData.kind === "tab" ? String(event.over?.id) : undefined;
    reorderMockTab(dragData.tab.tabId, overTabId);
    focusTab(dragData.tab.tabId);
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
    <div className="flex h-7 w-37.5 select-none items-center rounded-md bg-accent px-3 pr-7 text-sm text-accent-foreground shadow-sm">
      <span className="truncate">{tab.title}</span>
    </div>
  );
}
