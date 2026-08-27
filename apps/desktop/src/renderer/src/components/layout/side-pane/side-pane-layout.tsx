"use client";

import * as React from "react";
import { Group, Panel, Separator, usePanelRef } from "react-resizable-panels";

const DEFAULT_SIDE_PANE_LAYOUT = { "main-pane": 100, "side-pane": 0 } as const;
const DEFAULT_DOCKED_SIDE_PANE_WIDTH = "50%";
const MINIMUM_PANE_WIDTH = "20rem";
const RESIZE_TARGET_MINIMUM_SIZE = { coarse: 24, fine: 12 } as const;

type SidePaneLayoutProps = Readonly<{
  mainPaneContent: React.ReactNode;
  sidePaneContent: React.ReactNode;
}>;

type SidePaneLayoutContextValue = Readonly<{
  closeSidePane: (afterClose?: () => void) => void;
  isSidePaneMaximized: boolean;
  isSidePaneOpen: boolean;
  openSidePane: () => void;
  toggleSidePane: () => void;
  toggleSidePaneMaximized: () => void;
}>;

type SidePaneLayoutState =
  | Readonly<{ status: "closed" }>
  | Readonly<{ status: "open"; mode: "docked" | "maximized" }>;

type InternalSidePaneLayoutContextValue = SidePaneLayoutContextValue &
  Readonly<{
    completeSidePaneCloseTransition: () => void;
  }>;

const SidePaneLayoutContext = React.createContext<InternalSidePaneLayoutContextValue | null>(null);

function useSidePaneLayoutContext() {
  const sidePaneLayoutContext = React.useContext(SidePaneLayoutContext);
  if (!sidePaneLayoutContext) {
    throw new Error("useSidePaneLayout must be used within a SidePaneLayoutProvider.");
  }
  return sidePaneLayoutContext;
}

function useSidePaneLayout(): SidePaneLayoutContextValue {
  return useSidePaneLayoutContext();
}

function prefersReducedMotion() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

function SidePaneLayoutProvider({ children }: React.PropsWithChildren) {
  const [sidePaneLayoutState, setSidePaneLayoutState] = React.useState<SidePaneLayoutState>({
    status: "closed",
  });
  const isSidePaneOpen = sidePaneLayoutState.status === "open";
  const isSidePaneMaximized =
    sidePaneLayoutState.status === "open" && sidePaneLayoutState.mode === "maximized";
  const afterSidePaneCloseRef = React.useRef<(() => void) | undefined>(undefined);

  const openSidePane = React.useCallback(() => {
    afterSidePaneCloseRef.current = undefined;
    setSidePaneLayoutState((currentState) =>
      currentState.status === "open" ? currentState : { status: "open", mode: "docked" },
    );
  }, []);

  const closeSidePane = React.useCallback((afterClose?: () => void) => {
    afterSidePaneCloseRef.current = afterClose;
    const reducedMotion = prefersReducedMotion();
    setSidePaneLayoutState({ status: "closed" });
    if (reducedMotion) {
      afterSidePaneCloseRef.current?.();
      afterSidePaneCloseRef.current = undefined;
    }
  }, []);

  const toggleSidePane = React.useCallback(() => {
    afterSidePaneCloseRef.current = undefined;
    setSidePaneLayoutState((currentState) => {
      if (currentState.status === "closed") {
        return { status: "open", mode: "docked" };
      }
      return { status: "closed" };
    });
  }, []);

  const toggleSidePaneMaximized = React.useCallback(() => {
    setSidePaneLayoutState((currentState) => {
      if (currentState.status !== "open") return currentState;
      return {
        status: "open",
        mode: currentState.mode === "docked" ? "maximized" : "docked",
      };
    });
  }, []);

  const completeSidePaneCloseTransition = React.useCallback(() => {
    afterSidePaneCloseRef.current?.();
    afterSidePaneCloseRef.current = undefined;
  }, []);

  const sidePaneLayoutContextValue = React.useMemo<InternalSidePaneLayoutContextValue>(
    () => ({
      closeSidePane,
      completeSidePaneCloseTransition,
      isSidePaneMaximized,
      isSidePaneOpen,
      openSidePane,
      toggleSidePane,
      toggleSidePaneMaximized,
    }),
    [
      closeSidePane,
      completeSidePaneCloseTransition,
      isSidePaneMaximized,
      isSidePaneOpen,
      openSidePane,
      toggleSidePane,
      toggleSidePaneMaximized,
    ],
  );

  return (
    <SidePaneLayoutContext value={sidePaneLayoutContextValue}>{children}</SidePaneLayoutContext>
  );
}

function SidePaneLayout({ mainPaneContent, sidePaneContent }: SidePaneLayoutProps) {
  const { completeSidePaneCloseTransition, isSidePaneMaximized, isSidePaneOpen } =
    useSidePaneLayoutContext();
  const sidePanePanelRef = usePanelRef();
  const dockedSidePaneWidthRef = React.useRef(50);
  const hasOpenedSidePaneRef = React.useRef(false);
  const wasSidePaneMaximizedRef = React.useRef(false);

  React.useLayoutEffect(() => {
    const sidePanePanelHandle = sidePanePanelRef.current;
    if (!sidePanePanelHandle) return;

    const wasMaximized = wasSidePaneMaximizedRef.current;

    if (!isSidePaneOpen) {
      if (wasMaximized) {
        sidePanePanelHandle.resize(`${dockedSidePaneWidthRef.current}%`);
      }
      sidePanePanelHandle.collapse();
    } else if (isSidePaneMaximized) {
      if (!wasMaximized) {
        dockedSidePaneWidthRef.current = sidePanePanelHandle.getSize().asPercentage;
      }
      sidePanePanelHandle.resize("100%");
    } else if (wasMaximized) {
      sidePanePanelHandle.resize(`${dockedSidePaneWidthRef.current}%`);
    } else if (sidePanePanelHandle.isCollapsed()) {
      if (hasOpenedSidePaneRef.current) {
        sidePanePanelHandle.expand();
      } else {
        sidePanePanelHandle.resize(DEFAULT_DOCKED_SIDE_PANE_WIDTH);
        hasOpenedSidePaneRef.current = true;
      }
    }

    wasSidePaneMaximizedRef.current = isSidePaneMaximized;
  }, [isSidePaneMaximized, isSidePaneOpen, sidePanePanelRef]);

  return (
    <Group
      data-slot="side-pane-layout"
      className="group/side-pane-layout relative min-h-0 min-w-0 flex-1 bg-background motion-reduce:[&>[data-panel]]:transition-none [&>[data-panel]]:transition-[flex-grow] [&>[data-panel]]:duration-150 [&>[data-panel]]:ease-linear has-[[data-separator=active]]:[&>[data-panel]]:duration-0 has-[[data-separator=focus]]:[&>[data-panel]]:duration-0"
      defaultLayout={DEFAULT_SIDE_PANE_LAYOUT}
      disabled={!isSidePaneOpen || isSidePaneMaximized}
      id="side-pane-layout"
      orientation="horizontal"
      resizeTargetMinimumSize={RESIZE_TARGET_MINIMUM_SIZE}
    >
      <Panel
        aria-hidden={isSidePaneMaximized}
        groupResizeBehavior="preserve-relative-size"
        id="main-pane"
        inert={isSidePaneMaximized}
        minSize={isSidePaneMaximized ? "0%" : MINIMUM_PANE_WIDTH}
        style={{ display: "flex", minWidth: 0, overflow: "hidden" }}
      >
        {mainPaneContent}
      </Panel>
      <Separator
        aria-label="Main pane and side pane separator"
        className="relative z-20 w-px cursor-col-resize bg-transparent outline-none after:absolute after:inset-y-0 after:left-0 after:w-px data-[pane-resize-disabled=true]:pointer-events-none data-[pane-resize-disabled=true]:w-0 data-[pane-resize-disabled=true]:opacity-0 data-[separator=active]:after:bg-border data-[separator=focus]:after:bg-border data-[separator=hover]:after:bg-border"
        data-pane-resize-disabled={!isSidePaneOpen || isSidePaneMaximized}
        disabled={!isSidePaneOpen || isSidePaneMaximized}
        id="side-pane-separator"
        title="Resize main pane and side pane"
      />
      <Panel
        aria-hidden={!isSidePaneOpen}
        className="overflow-hidden bg-background"
        collapsedSize="0%"
        collapsible
        groupResizeBehavior="preserve-relative-size"
        id="side-pane"
        inert={!isSidePaneOpen}
        minSize={MINIMUM_PANE_WIDTH}
        onTransitionEnd={(event) => {
          if (
            !isSidePaneOpen &&
            event.currentTarget === event.target &&
            event.propertyName === "flex-grow"
          ) {
            completeSidePaneCloseTransition();
          }
        }}
        panelRef={sidePanePanelRef}
      >
        <div
          className="group/side-pane-surface size-full"
          data-maximized={isSidePaneMaximized}
          data-slot="side-pane-surface"
        >
          {sidePaneContent}
        </div>
      </Panel>
    </Group>
  );
}

export { SidePaneLayout, SidePaneLayoutProvider, useSidePaneLayout };
