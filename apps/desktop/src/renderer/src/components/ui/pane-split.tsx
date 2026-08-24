"use client";

import * as React from "react";

import { cn } from "@/lib/utils";

const RESIZE_THRESHOLD = 5;
const DOCKED_MAIN_PANE_WIDTH =
  "clamp(var(--pane-minimum), var(--main-pane-intent), calc(100% - var(--pane-minimum)))";
type RemSize = `${number}rem`;
type PercentageSize = `${number}%`;
type PixelSize = `${number}px`;
type SizeIntent = PercentageSize | PixelSize;

type PaneSplitProps = Omit<React.ComponentProps<"div">, "children"> & {
  mainPane: React.ReactNode;
  sidePane: React.ReactNode;
  minimumSize?: RemSize;
  defaultMainPaneSize?: PercentageSize;
  keyboardStep?: RemSize;
};

type PaneSplitContextProps = Readonly<{
  isSidePaneOpen: boolean;
  isSidePaneMaximized: boolean;
  toggleSidePane: () => void;
  toggleSidePaneMaximized: () => void;
}>;

type ResizeBounds = Readonly<{
  minimum: number;
  maximum: number;
  splitWidth: number;
  pixelsPerRem: number;
}>;

const PaneSplitContext = React.createContext<PaneSplitContextProps | null>(null);

function usePaneSplit() {
  const context = React.useContext(PaneSplitContext);
  if (!context) {
    throw new Error("usePaneSplit must be used within a PaneSplitProvider.");
  }

  return context;
}

function PaneSplitProvider({
  defaultSidePaneOpen = false,
  children,
}: React.PropsWithChildren<{ defaultSidePaneOpen?: boolean }>) {
  const [isSidePaneOpen, setIsSidePaneOpen] = React.useState(defaultSidePaneOpen);
  const [isSidePaneMaximized, setIsSidePaneMaximized] = React.useState(false);

  const toggleSidePane = React.useCallback(() => {
    if (isSidePaneOpen) setIsSidePaneMaximized(false);
    setIsSidePaneOpen(!isSidePaneOpen);
  }, [isSidePaneOpen]);

  const toggleSidePaneMaximized = React.useCallback(() => {
    if (isSidePaneOpen) setIsSidePaneMaximized((isMaximized) => !isMaximized);
  }, [isSidePaneOpen]);

  const value = React.useMemo<PaneSplitContextProps>(
    () => ({
      isSidePaneOpen,
      isSidePaneMaximized,
      toggleSidePane,
      toggleSidePaneMaximized,
    }),
    [isSidePaneOpen, isSidePaneMaximized, toggleSidePane, toggleSidePaneMaximized],
  );

  return <PaneSplitContext value={value}>{children}</PaneSplitContext>;
}

function rootPixelsPerRem() {
  const value = Number.parseFloat(window.getComputedStyle(document.documentElement).fontSize);
  return Number.isFinite(value) ? value : 16;
}

function clamp(value: number, minimum: number, maximum: number) {
  return Math.min(maximum, Math.max(minimum, value));
}

function PaneSplit({
  mainPane,
  sidePane,
  minimumSize = "20rem",
  defaultMainPaneSize = "50%",
  keyboardStep = "1rem",
  className,
  style,
  ...props
}: PaneSplitProps) {
  const { isSidePaneOpen, isSidePaneMaximized, toggleSidePane } = usePaneSplit();
  const [mainPaneIntent, setMainPaneIntent] = React.useState<SizeIntent>(defaultMainPaneSize);
  const splitRef = React.useRef<HTMLDivElement>(null);
  const mainPaneRef = React.useRef<HTMLDivElement>(null);
  const railRef = React.useRef<HTMLButtonElement>(null);
  const sidePaneId = React.useId();
  const suppressClickRef = React.useRef(false);
  const dragRef = React.useRef<{
    pointerId: number;
    startX: number;
    startWidth: number;
    currentWidth: number;
    bounds: ResizeBounds;
    moved: boolean;
  } | null>(null);

  const minimumRem = Number.parseFloat(minimumSize);
  const defaultMainPanePercentage = Number.parseFloat(defaultMainPaneSize);
  const keyboardStepRem = Number.parseFloat(keyboardStep);
  const mainPaneWidth = isSidePaneOpen ? DOCKED_MAIN_PANE_WIDTH : "100%";
  const sidePaneInset = isSidePaneMaximized ? 0 : mainPaneWidth;

  if (!Number.isFinite(minimumRem) || minimumRem <= 0) {
    throw new Error("PaneSplit minimumSize must be a positive rem value.");
  }
  if (!Number.isFinite(keyboardStepRem) || keyboardStepRem <= 0) {
    throw new Error("PaneSplit keyboardStep must be a positive rem value.");
  }
  if (
    !Number.isFinite(defaultMainPanePercentage) ||
    defaultMainPanePercentage <= 0 ||
    defaultMainPanePercentage >= 100
  ) {
    throw new Error("PaneSplit defaultMainPaneSize must be between 0% and 100%.");
  }

  const readBounds = React.useCallback((): ResizeBounds => {
    const splitWidth = splitRef.current?.clientWidth ?? 0;
    const pixelsPerRem = rootPixelsPerRem();
    const minimum = minimumRem * pixelsPerRem;
    return {
      minimum,
      maximum: Math.max(minimum, splitWidth - minimum),
      splitWidth,
      pixelsPerRem,
    };
  }, [minimumRem]);

  const readMainPaneWidth = React.useCallback(
    () => mainPaneRef.current?.getBoundingClientRect().width ?? 0,
    [],
  );

  const syncAccessibleValue = React.useCallback((width: number, bounds: ResizeBounds) => {
    const percentage = bounds.splitWidth > 0 ? (width / bounds.splitWidth) * 100 : 0;
    const mainRem = width / bounds.pixelsPerRem;
    const sideRem = (bounds.splitWidth - width) / bounds.pixelsPerRem;
    railRef.current?.setAttribute("aria-valuenow", percentage.toFixed(1));
    railRef.current?.setAttribute(
      "aria-valuetext",
      `Main pane ${mainRem.toFixed(1)} rem, side pane ${sideRem.toFixed(1)} rem`,
    );
  }, []);

  const publishWidth = React.useCallback(
    (width: number, bounds: ResizeBounds) => {
      const effectiveWidth = clamp(width, bounds.minimum, bounds.maximum);
      splitRef.current?.style.setProperty("--main-pane-intent", `${effectiveWidth}px`);
      syncAccessibleValue(effectiveWidth, bounds);
      return effectiveWidth;
    },
    [syncAccessibleValue],
  );

  const setDragging = React.useCallback((dragging: boolean) => {
    if (!splitRef.current) return;
    if (dragging) splitRef.current.dataset.dragging = "true";
    else delete splitRef.current.dataset.dragging;
  }, []);

  const finishResize = React.useCallback(
    (pointerId: number, suppressClick: boolean) => {
      const drag = dragRef.current;
      if (!drag || drag.pointerId !== pointerId) return false;

      dragRef.current = null;
      setDragging(false);
      if (drag.moved) {
        setMainPaneIntent(`${drag.currentWidth}px`);
        if (suppressClick) {
          suppressClickRef.current = true;
          window.setTimeout(() => {
            suppressClickRef.current = false;
          });
        }
      } else {
        splitRef.current?.style.setProperty("--main-pane-intent", mainPaneIntent);
      }

      return true;
    },
    [mainPaneIntent, setDragging],
  );

  return (
    <div
      ref={splitRef}
      data-slot="pane-split"
      className={cn(
        "group/pane-split relative grid min-h-0 min-w-0 flex-1 grid-cols-1 grid-rows-1 bg-background",
        className,
      )}
      style={
        {
          ...style,
          "--pane-minimum": minimumSize,
          "--main-pane-intent": mainPaneIntent,
        } as React.CSSProperties
      }
      {...props}
    >
      <div
        ref={mainPaneRef}
        data-slot="main-pane"
        aria-hidden={isSidePaneMaximized}
        className="col-start-1 row-start-1 flex min-w-0 overflow-hidden transition-[width] duration-150 ease-linear group-data-[dragging=true]/pane-split:duration-0"
        inert={isSidePaneMaximized}
        style={{ width: mainPaneWidth }}
      >
        {mainPane}
      </div>
      <div
        id={sidePaneId}
        data-slot="side-pane"
        data-state={isSidePaneOpen ? "open" : "closed"}
        className="group/side-pane relative col-[1/-1] row-start-1 z-10 min-w-0 overflow-hidden bg-background transition-[margin-left] duration-150 ease-linear group-data-[dragging=true]/pane-split:duration-0"
        style={{ marginLeft: sidePaneInset }}
        aria-hidden={!isSidePaneOpen}
        inert={!isSidePaneOpen}
      >
        {sidePane}
        {isSidePaneOpen && !isSidePaneMaximized && (
          <button
            ref={railRef}
            aria-controls={sidePaneId}
            aria-label="Main and side pane divider"
            aria-orientation="vertical"
            aria-valuemax={100}
            aria-valuemin={0}
            aria-valuenow={50}
            aria-valuetext="Equal split"
            data-slot="pane-split-rail"
            role="separator"
            tabIndex={0}
            title="Resize panes or close side pane"
            className="absolute inset-y-0 left-0 z-20 w-4 -translate-x-1/2 touch-none cursor-col-resize select-none [-webkit-app-region:no-drag] after:absolute after:inset-y-0 after:left-1/2 after:w-px after:-translate-x-1/2 hover:after:bg-border focus-visible:outline-none focus-visible:after:bg-border"
            onClick={(event) => {
              if (suppressClickRef.current) {
                suppressClickRef.current = false;
                event.preventDefault();
                return;
              }
              toggleSidePane();
            }}
            onFocus={() => syncAccessibleValue(readMainPaneWidth(), readBounds())}
            onKeyDown={(event) => {
              const bounds = readBounds();
              const currentWidth = readMainPaneWidth();
              const step = keyboardStepRem * bounds.pixelsPerRem;
              let nextWidth: number | undefined;

              if (event.key === "Home") nextWidth = bounds.minimum;
              if (event.key === "End") nextWidth = bounds.maximum;
              if (event.key === "ArrowLeft") nextWidth = currentWidth - step;
              if (event.key === "ArrowRight") nextWidth = currentWidth + step;

              if (nextWidth !== undefined) {
                event.preventDefault();
                const effectiveWidth = publishWidth(nextWidth, bounds);
                setMainPaneIntent(`${effectiveWidth}px`);
              }
            }}
            onLostPointerCapture={(event) => finishResize(event.pointerId, true)}
            onPointerCancel={(event) => finishResize(event.pointerId, false)}
            onPointerDown={(event) => {
              if (event.button !== 0) return;

              const bounds = readBounds();
              const currentWidth = readMainPaneWidth();
              dragRef.current = {
                pointerId: event.pointerId,
                startX: event.clientX,
                startWidth: currentWidth,
                currentWidth,
                bounds,
                moved: false,
              };
              syncAccessibleValue(currentWidth, bounds);
              event.currentTarget.setPointerCapture(event.pointerId);
              setDragging(true);
            }}
            onPointerMove={(event) => {
              const drag = dragRef.current;
              if (!drag || drag.pointerId !== event.pointerId) return;

              const delta = event.clientX - drag.startX;
              event.preventDefault();
              drag.moved ||= Math.abs(delta) >= RESIZE_THRESHOLD;
              drag.currentWidth = publishWidth(drag.startWidth + delta, drag.bounds);
            }}
            onPointerUp={(event) => {
              if (!finishResize(event.pointerId, true)) return;
              if (event.currentTarget.hasPointerCapture(event.pointerId)) {
                event.currentTarget.releasePointerCapture(event.pointerId);
              }
            }}
          />
        )}
      </div>
    </div>
  );
}

export { PaneSplit, PaneSplitProvider, usePaneSplit };
