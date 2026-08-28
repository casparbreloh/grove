import {
  closestCenter,
  DndContext,
  DragOverlay,
  KeyboardSensor,
  MeasuringStrategy,
  PointerSensor,
  useSensor,
  useSensors,
  type CollisionDetection,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { sortableKeyboardCoordinates } from "@dnd-kit/sortable";
import { createContext, useContext, useState, type ReactNode } from "react";
import type { Layout } from "react-resizable-panels";

import {
  moveMockTab,
  splitMockTab,
  useMockGrove,
  type PaneId,
  type SplitEdge,
  type Tab,
} from "@/lib/mocks/grove";

export type TabDragData = { kind: "tab"; paneId: PaneId };
export type TabDropData =
  | { kind: "tab"; paneId: PaneId }
  | { kind: "pane"; paneId: PaneId }
  | { kind: "split-edge"; paneId: PaneId; edge: SplitEdge };

type HorizontalLayoutState = { splitKey: string; layout: Layout } | undefined;

const HorizontalLayoutContext = createContext<
  readonly [HorizontalLayoutState, (state: HorizontalLayoutState) => void] | undefined
>(undefined);

const tabCollisionDetection: CollisionDetection = (arguments_) => {
  const collisions = closestCenter(arguments_).filter(
    (collision) => collision.id !== arguments_.active.id,
  );
  const { pointerCoordinates } = arguments_;
  if (pointerCoordinates) {
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
  }
  return collisions;
};

export function focusTab(tabId: string) {
  window.requestAnimationFrame(() => document.getElementById(`${tabId}-tab`)?.focus());
}

export function TabDragDropProvider({ children }: { children: ReactNode }) {
  const { tabs } = useMockGrove();
  const [draggedTabId, setDraggedTabId] = useState<string>();
  const horizontalLayout = useState<HorizontalLayoutState>();
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
      keyboardCodes: { start: ["KeyD"], cancel: ["Escape"], end: ["KeyD"] },
    }),
  );
  const draggedTab = tabs.find(({ tabId }) => tabId === draggedTabId);

  function handleDragStart(event: DragStartEvent) {
    setDraggedTabId(String(event.active.id));
  }

  function handleDragEnd(event: DragEndEvent) {
    const tabId = String(event.active.id);
    const dropData = event.over?.data.current as TabDropData | undefined;
    setDraggedTabId(undefined);
    if (!dropData) return;

    switch (dropData.kind) {
      case "split-edge":
        splitMockTab(tabId, dropData.paneId, dropData.edge);
        break;
      case "tab":
        moveMockTab(tabId, dropData.paneId, String(event.over?.id));
        break;
      case "pane":
        moveMockTab(tabId, dropData.paneId);
        break;
    }
    focusTab(tabId);
  }

  return (
    <HorizontalLayoutContext value={horizontalLayout}>
      <DndContext
        accessibility={{
          screenReaderInstructions: {
            draggable:
              "To pick up a tab, press D. While dragging, use the arrow keys to move it. Press D again to drop, or Escape to cancel.",
          },
        }}
        collisionDetection={tabCollisionDetection}
        measuring={{ droppable: { strategy: MeasuringStrategy.Always } }}
        onDragCancel={() => setDraggedTabId(undefined)}
        onDragEnd={handleDragEnd}
        onDragStart={handleDragStart}
        sensors={sensors}
      >
        {children}
        <DragOverlay dropAnimation={null}>
          {draggedTab ? <TabDragPreview tab={draggedTab} /> : null}
        </DragOverlay>
      </DndContext>
    </HorizontalLayoutContext>
  );
}

export function useHorizontalPaneLayout() {
  const context = useContext(HorizontalLayoutContext);
  if (!context) throw new Error("useHorizontalPaneLayout must be used within TabDragDropProvider");
  return context;
}

function TabDragPreview({ tab }: { tab: Tab }) {
  return (
    <div className="flex h-7 w-37.5 items-center rounded-md border bg-popover px-3 text-sm text-popover-foreground shadow-md">
      <span className="truncate">{tab.title}</span>
    </div>
  );
}
