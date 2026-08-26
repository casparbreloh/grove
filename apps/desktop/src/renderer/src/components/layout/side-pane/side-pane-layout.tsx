"use client";

import * as React from "react";
import {
  Group,
  Panel,
  Separator,
  usePanelCallbackRef,
  type Layout,
  type LayoutChangedMeta,
  type PanelSize,
} from "react-resizable-panels";

const DEFAULT_SIDE_PANE_LAYOUT = { "main-pane": 100, "side-pane": 0 } as const;
const DEFAULT_DOCKED_SIDE_PANE_WIDTH = "50%";
const MINIMUM_PANE_WIDTH = "20rem";

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
  | Readonly<{ status: "open"; mode: "docked" | "maximized" }>
  | Readonly<{ status: "closing"; mode: "maximized" }>;

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

function beginSidePaneClose(
  currentState: SidePaneLayoutState,
  prefersReducedMotion: boolean,
): SidePaneLayoutState {
  if (currentState.status !== "open") return currentState;
  return currentState.mode === "maximized" && !prefersReducedMotion
    ? { status: "closing", mode: "maximized" }
    : { status: "closed" };
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
    sidePaneLayoutState.status !== "closed" && sidePaneLayoutState.mode === "maximized";
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
    setSidePaneLayoutState((currentState) => beginSidePaneClose(currentState, reducedMotion));
    if (reducedMotion) {
      afterSidePaneCloseRef.current?.();
      afterSidePaneCloseRef.current = undefined;
    }
  }, []);

  const toggleSidePane = React.useCallback(() => {
    afterSidePaneCloseRef.current = undefined;
    setSidePaneLayoutState((currentState) => {
      if (currentState.status === "closed" || currentState.status === "closing") {
        return { status: "open", mode: "docked" };
      }
      return beginSidePaneClose(currentState, prefersReducedMotion());
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
    setSidePaneLayoutState((currentState) =>
      currentState.status === "closing" ? { status: "closed" } : currentState,
    );
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
  const [sidePaneSizingPanelHandle, setSidePaneSizingPanelHandle] = usePanelCallbackRef();
  const sidePaneSeparatorRef = React.useRef<HTMLDivElement>(null);
  const sidePaneLayoutRef = React.useRef<HTMLDivElement>(null);
  const dockedSidePaneWidthRef = React.useRef(DEFAULT_DOCKED_SIDE_PANE_WIDTH);

  React.useLayoutEffect(() => {
    if (!sidePaneSizingPanelHandle) return;
    if (isSidePaneOpen) sidePaneSizingPanelHandle.resize(dockedSidePaneWidthRef.current);
    else sidePaneSizingPanelHandle.collapse();
  }, [isSidePaneOpen, sidePaneSizingPanelHandle]);

  const rememberDockedSidePaneWidth = React.useCallback((panelSize: PanelSize) => {
    if (panelSize.asPercentage > 0) {
      const width = `${panelSize.asPercentage}%`;
      dockedSidePaneWidthRef.current = width;
      sidePaneLayoutRef.current?.style.setProperty("--docked-side-pane-width", width);
    }
  }, []);

  // onResize follows active pointer movement; onLayoutChanged captures the settled keyboard size
  // without replacing the remembered width during programmatic open and close transitions.
  const handleSidePaneSizingPanelResize = React.useCallback(
    (panelSize: PanelSize) => {
      if (isSidePaneOpen && sidePaneSeparatorRef.current?.dataset.separator === "active") {
        rememberDockedSidePaneWidth(panelSize);
      }
    },
    [isSidePaneOpen, rememberDockedSidePaneWidth],
  );

  const handleSidePanePanelLayoutChanged = React.useCallback(
    (_layout: Layout, metadata: LayoutChangedMeta) => {
      if (metadata.isUserInteraction && sidePaneSizingPanelHandle) {
        rememberDockedSidePaneWidth(sidePaneSizingPanelHandle.getSize());
      }
    },
    [rememberDockedSidePaneWidth, sidePaneSizingPanelHandle],
  );

  return (
    <div
      ref={sidePaneLayoutRef}
      data-slot="side-pane-layout"
      className="group/side-pane-layout relative min-h-0 min-w-0 flex-1 overflow-hidden bg-background"
      // SAFETY: React forwards this custom property unchanged; it is scoped to this layout.
      style={
        {
          "--docked-side-pane-width": DEFAULT_DOCKED_SIDE_PANE_WIDTH,
        } as React.CSSProperties
      }
    >
      <Group
        className="size-full motion-reduce:[&>[data-panel]]:transition-none [&>[data-panel]]:transition-[flex-grow] [&>[data-panel]]:duration-150 [&>[data-panel]]:ease-linear has-[[data-separator=active]]:[&>[data-panel]]:duration-0 has-[[data-separator=focus]]:[&>[data-panel]]:duration-0"
        defaultLayout={DEFAULT_SIDE_PANE_LAYOUT}
        disabled={!isSidePaneOpen || isSidePaneMaximized}
        id="side-pane-layout"
        onLayoutChanged={handleSidePanePanelLayoutChanged}
        orientation="horizontal"
      >
        <Panel
          aria-hidden={isSidePaneMaximized}
          groupResizeBehavior="preserve-relative-size"
          id="main-pane"
          inert={isSidePaneMaximized}
          minSize={MINIMUM_PANE_WIDTH}
          style={{ display: "flex", minWidth: 0, overflow: "hidden" }}
        >
          {mainPaneContent}
        </Panel>
        <Separator
          aria-label="Main pane and side pane separator"
          className="relative z-20 w-px cursor-col-resize bg-transparent outline-none after:absolute after:inset-y-0 after:left-0 after:w-px data-[pane-resize-disabled=true]:pointer-events-none data-[pane-resize-disabled=true]:w-0 data-[pane-resize-disabled=true]:opacity-0 data-[separator=active]:after:bg-border data-[separator=focus]:after:bg-border data-[separator=hover]:after:bg-border"
          data-pane-resize-disabled={!isSidePaneOpen || isSidePaneMaximized}
          disabled={!isSidePaneOpen || isSidePaneMaximized}
          elementRef={sidePaneSeparatorRef}
          id="side-pane-separator"
          title="Resize main pane and side pane"
        />
        <Panel
          aria-hidden
          collapsedSize="0%"
          collapsible
          groupResizeBehavior="preserve-relative-size"
          id="side-pane"
          inert
          minSize={MINIMUM_PANE_WIDTH}
          onResize={handleSidePaneSizingPanelResize}
          panelRef={setSidePaneSizingPanelHandle}
        />
      </Group>
      <div
        data-slot="side-pane-surface"
        data-state={isSidePaneOpen ? "open" : "closed"}
        className="absolute inset-y-0 right-0 z-10 min-w-0 overflow-hidden bg-background opacity-100 transition-[opacity,translate,width] duration-150 ease-linear motion-reduce:transition-none data-[state=closed]:translate-x-full data-[state=closed]:opacity-0 group-has-[[data-separator=active]]/side-pane-layout:duration-0 group-has-[[data-separator=focus]]/side-pane-layout:duration-0"
        style={{ width: isSidePaneMaximized ? "100%" : "var(--docked-side-pane-width)" }}
        aria-hidden={!isSidePaneOpen}
        inert={!isSidePaneOpen}
        onTransitionEnd={(event) => {
          if (event.currentTarget === event.target && event.propertyName === "opacity") {
            completeSidePaneCloseTransition();
          }
        }}
      >
        {sidePaneContent}
      </div>
    </div>
  );
}

export { SidePaneLayout, SidePaneLayoutProvider, useSidePaneLayout };
