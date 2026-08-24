"use client";

import * as React from "react";
import { mergeProps } from "@base-ui/react/merge-props";
import { useRender } from "@base-ui/react/use-render";
import { cva, type VariantProps } from "class-variance-authority";

import { useIsMobile } from "@/hooks/use-media-query";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Separator } from "@/components/ui/separator";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Skeleton } from "@/components/ui/skeleton";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { HugeiconsIcon } from "@hugeicons/react";
import { LayoutLeftIcon, LayoutRightIcon } from "@hugeicons/core-free-icons";

const SIDEBAR_WIDTH_MOBILE = "18rem";
const SIDEBAR_WIDTH_ICON = "3rem";
const SIDEBAR_KEYBOARD_SHORTCUT = "b";
const SIDEBAR_RESIZE_THRESHOLD = 5;

type SidebarWidth = `${number}rem`;
type SidebarDefaultWidth = SidebarWidth | `${number}%`;

type SidebarWidthLimits =
  | Readonly<{ max: SidebarWidth; adjacentPaneMin?: never }>
  | Readonly<{ max?: never; adjacentPaneMin: SidebarWidth }>;

type SidebarWidthConfig = Readonly<{
  min: SidebarWidth;
  default: SidebarDefaultWidth;
  keyboardStep?: SidebarWidth;
}> &
  SidebarWidthLimits;

type ResolvedSidebarWidthConfig = Readonly<{
  min: number;
  max: number;
  keyboardStep: number;
}>;

type ResolvedSidebarWidthDefinition = Readonly<{
  min: number;
  default: Readonly<{ unit: "rem" | "percent"; value: number }>;
  max?: number;
  adjacentPaneMin?: number;
  keyboardStep: number;
}>;

const DEFAULT_SIDEBAR_WIDTH = {
  min: "16rem",
  default: "16rem",
  max: "16rem",
  keyboardStep: "1rem",
} satisfies SidebarWidthConfig;

type SidebarContextProps = {
  state: "expanded" | "collapsed";
  open: boolean;
  setOpen: (open: boolean) => void;
  openMobile: boolean;
  setOpenMobile: (open: boolean) => void;
  isMobile: boolean;
  toggleSidebar: () => void;
  width: number;
  widthConfig: ResolvedSidebarWidthConfig;
  setWidth: (width: number) => void;
  setDragging: (dragging: boolean) => void;
  wrapperRef: React.RefObject<HTMLDivElement | null>;
  sidebarId: string;
};

type SidebarSide = "left" | "right";

const SidebarContexts = {
  left: React.createContext<SidebarContextProps | null>(null),
  right: React.createContext<SidebarContextProps | null>(null),
} satisfies Record<SidebarSide, React.Context<SidebarContextProps | null>>;

function useSidebar(side: SidebarSide = "left") {
  const context = React.useContext(SidebarContexts[side]);
  if (!context) {
    throw new Error(`useSidebar("${side}") must be used within a ${side} SidebarProvider.`);
  }

  return context;
}

function parseRem(value: SidebarWidth) {
  return Number.parseFloat(value);
}

function resolveDefaultWidth(value: SidebarDefaultWidth) {
  return {
    unit: value.endsWith("%") ? ("percent" as const) : ("rem" as const),
    value: Number.parseFloat(value),
  };
}

function resolveWidthConfig(config: SidebarWidthConfig): ResolvedSidebarWidthDefinition {
  const resolved = {
    min: parseRem(config.min),
    default: resolveDefaultWidth(config.default),
    max: config.max ? parseRem(config.max) : undefined,
    adjacentPaneMin: config.adjacentPaneMin ? parseRem(config.adjacentPaneMin) : undefined,
    keyboardStep: parseRem(config.keyboardStep ?? "1rem"),
  };

  if (
    !Number.isFinite(resolved.min) ||
    !Number.isFinite(resolved.default.value) ||
    !Number.isFinite(resolved.max ?? resolved.adjacentPaneMin) ||
    (resolved.max === undefined) === (resolved.adjacentPaneMin === undefined) ||
    (resolved.max !== undefined && resolved.max < resolved.min) ||
    (resolved.adjacentPaneMin !== undefined && resolved.adjacentPaneMin <= 0) ||
    (resolved.default.unit === "rem" && resolved.default.value < resolved.min) ||
    (resolved.max !== undefined &&
      resolved.default.unit === "rem" &&
      resolved.default.value > resolved.max) ||
    (resolved.default.unit === "percent" &&
      (resolved.default.value <= 0 || resolved.default.value >= 100)) ||
    resolved.keyboardStep <= 0
  ) {
    throw new Error(
      "Sidebar widths need a valid minimum, default, and exactly one fixed maximum or adjacent-pane minimum.",
    );
  }

  return resolved;
}

function clampWidth(width: number, config: ResolvedSidebarWidthConfig) {
  return Math.min(config.max, Math.max(config.min, width));
}

function SidebarProvider({
  side = "left",
  defaultOpen = true,
  open: openProp,
  onOpenChange: setOpenProp,
  width: widthConfigProp = DEFAULT_SIDEBAR_WIDTH,
  className,
  style,
  children,
  ...props
}: React.ComponentProps<"div"> & {
  side?: SidebarSide;
  defaultOpen?: boolean;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  width?: SidebarWidthConfig;
}) {
  const SidebarContext = SidebarContexts[side];
  const isMobile = useIsMobile();
  const [openMobile, setOpenMobile] = React.useState(false);
  const wrapperRef = React.useRef<HTMLDivElement>(null);
  const sidebarId = React.useId();
  const widthDefinition = React.useMemo(
    () => resolveWidthConfig(widthConfigProp),
    [
      widthConfigProp.default,
      widthConfigProp.keyboardStep,
      widthConfigProp.adjacentPaneMin,
      widthConfigProp.max,
      widthConfigProp.min,
    ],
  );
  const [availableWidth, setAvailableWidth] = React.useState<number>();
  const initializedRelativeDefaultRef = React.useRef(false);
  const effectiveMax =
    widthDefinition.max ??
    Math.max(
      widthDefinition.min,
      (availableWidth ?? widthDefinition.min + (widthDefinition.adjacentPaneMin ?? 0)) -
        (widthDefinition.adjacentPaneMin ?? 0),
    );
  const widthConfig = React.useMemo<ResolvedSidebarWidthConfig>(
    () => ({
      min: widthDefinition.min,
      max: effectiveMax,
      keyboardStep: widthDefinition.keyboardStep,
    }),
    [effectiveMax, widthDefinition.keyboardStep, widthDefinition.min],
  );
  const [_width, _setWidth] = React.useState(
    widthDefinition.default.unit === "rem" ? widthDefinition.default.value : widthDefinition.min,
  );
  const width = clampWidth(_width, widthConfig);
  const setWidth = React.useCallback(
    (nextWidth: number) => _setWidth(clampWidth(nextWidth, widthConfig)),
    [widthConfig],
  );
  const [dragging, setDragging] = React.useState(false);

  React.useLayoutEffect(() => {
    if (widthDefinition.adjacentPaneMin === undefined && widthDefinition.default.unit === "rem") {
      return;
    }

    const wrapper = wrapperRef.current;
    if (!wrapper) return;

    const updateAvailableWidth = () => {
      const pixelsPerRem = Number.parseFloat(
        window.getComputedStyle(document.documentElement).fontSize,
      );
      const availableRem =
        wrapper.clientWidth / (Number.isFinite(pixelsPerRem) ? pixelsPerRem : 16);
      setAvailableWidth(availableRem);

      if (widthDefinition.default.unit === "percent" && !initializedRelativeDefaultRef.current) {
        initializedRelativeDefaultRef.current = true;
        _setWidth((availableRem * widthDefinition.default.value) / 100);
      }
    };

    updateAvailableWidth();
    const observer = new ResizeObserver(updateAvailableWidth);
    observer.observe(wrapper);
    return () => observer.disconnect();
  }, [widthDefinition]);

  const [_open, _setOpen] = React.useState(defaultOpen);
  const open = openProp ?? _open;
  const setOpen = React.useCallback(
    (value: boolean | ((value: boolean) => boolean)) => {
      const openState = typeof value === "function" ? value(open) : value;
      if (setOpenProp) {
        setOpenProp(openState);
      } else {
        _setOpen(openState);
      }
    },
    [setOpenProp, open],
  );

  const toggleSidebar = React.useCallback(() => {
    return isMobile ? setOpenMobile((open) => !open) : setOpen((open) => !open);
  }, [isMobile, setOpen, setOpenMobile]);

  React.useEffect(() => {
    if (side !== "left") return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === SIDEBAR_KEYBOARD_SHORTCUT && (event.metaKey || event.ctrlKey)) {
        event.preventDefault();
        toggleSidebar();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [side, toggleSidebar]);

  const state = open ? "expanded" : "collapsed";

  const contextValue = React.useMemo<SidebarContextProps>(
    () => ({
      state,
      open,
      setOpen,
      isMobile,
      openMobile,
      setOpenMobile,
      toggleSidebar,
      width,
      widthConfig,
      setWidth,
      setDragging,
      wrapperRef,
      sidebarId,
    }),
    [
      state,
      open,
      setOpen,
      isMobile,
      openMobile,
      setOpenMobile,
      toggleSidebar,
      width,
      widthConfig,
      setWidth,
      sidebarId,
    ],
  );

  return (
    <SidebarContext value={contextValue}>
      <div
        ref={wrapperRef}
        data-dragging={dragging}
        data-sidebar-provider={side}
        data-slot="sidebar-wrapper"
        style={
          {
            "--sidebar-adjacent-pane-min-width": widthDefinition.adjacentPaneMin
              ? `${widthDefinition.adjacentPaneMin}rem`
              : undefined,
            "--sidebar-width": `${width}rem`,
            "--sidebar-width-icon": SIDEBAR_WIDTH_ICON,
            ...style,
          } as React.CSSProperties
        }
        className={cn(
          "group/sidebar-wrapper flex min-h-svh w-full has-data-[variant=inset]:bg-sidebar",
          className,
        )}
        {...props}
      >
        {children}
      </div>
    </SidebarContext>
  );
}

function Sidebar({
  side = "left",
  variant = "sidebar",
  collapsible = "offcanvas",
  className,
  children,
  dir,
  ...props
}: React.ComponentProps<"div"> & {
  side?: SidebarSide;
  variant?: "sidebar" | "floating" | "inset";
  collapsible?: "offcanvas" | "icon" | "none";
}) {
  const { isMobile, state, openMobile, setOpenMobile, sidebarId } = useSidebar(side);

  if (collapsible === "none") {
    return (
      <div
        data-slot="sidebar"
        className={cn(
          "flex h-full w-(--sidebar-width) flex-col bg-sidebar text-sidebar-foreground",
          className,
        )}
        {...props}
      >
        {children}
      </div>
    );
  }

  if (isMobile) {
    return (
      <Sheet open={openMobile} onOpenChange={setOpenMobile} {...props}>
        <SheetContent
          dir={dir}
          data-sidebar="sidebar"
          data-slot="sidebar"
          data-mobile="true"
          className="w-(--sidebar-width) bg-sidebar p-0 text-sidebar-foreground [&>button]:hidden"
          style={
            {
              "--sidebar-width": SIDEBAR_WIDTH_MOBILE,
            } as React.CSSProperties
          }
          side={side}
        >
          <SheetHeader className="sr-only">
            <SheetTitle>Sidebar</SheetTitle>
            <SheetDescription>Displays the mobile sidebar.</SheetDescription>
          </SheetHeader>
          <div className="flex h-full w-full flex-col">{children}</div>
        </SheetContent>
      </Sheet>
    );
  }

  return (
    <div
      id={sidebarId}
      className="group peer hidden text-sidebar-foreground md:block"
      data-state={state}
      data-collapsible={state === "collapsed" ? collapsible : ""}
      data-variant={variant}
      data-side={side}
      data-slot="sidebar"
    >
      <div
        data-slot="sidebar-gap"
        className={cn(
          "relative w-(--sidebar-width) shrink-0 bg-transparent transition-[width] duration-150 ease-linear group-data-[dragging=true]/sidebar-wrapper:duration-0",
          "group-data-[collapsible=offcanvas]:w-0",
          "group-data-[side=right]:rotate-180",
          variant === "floating" || variant === "inset"
            ? "group-data-[collapsible=icon]:w-[calc(var(--sidebar-width-icon)+(--spacing(4)))]"
            : "group-data-[collapsible=icon]:w-(--sidebar-width-icon)",
        )}
      />
      <div
        data-slot="sidebar-container"
        data-side={side}
        className={cn(
          "fixed inset-y-0 z-10 hidden h-svh w-(--sidebar-width) transition-[left,right,width] duration-150 ease-linear group-data-[dragging=true]/sidebar-wrapper:duration-0 data-[side=left]:left-0 data-[side=left]:group-data-[collapsible=offcanvas]:left-[calc(var(--sidebar-width)*-1)] data-[side=right]:right-0 data-[side=right]:group-data-[collapsible=offcanvas]:right-[calc(var(--sidebar-width)*-1)] md:flex",
          variant === "floating" || variant === "inset"
            ? "p-2 group-data-[collapsible=icon]:w-[calc(var(--sidebar-width-icon)+(--spacing(4))+2px)]"
            : "group-data-[collapsible=icon]:w-(--sidebar-width-icon) group-data-[side=left]:border-r group-data-[side=right]:border-l",
          className,
        )}
        {...props}
      >
        <div
          data-sidebar="sidebar"
          data-slot="sidebar-inner"
          className="flex size-full flex-col bg-sidebar group-data-[variant=floating]:rounded-lg group-data-[variant=floating]:shadow-sm group-data-[variant=floating]:ring-1 group-data-[variant=floating]:ring-sidebar-border"
        >
          {children}
        </div>
      </div>
    </div>
  );
}

function SidebarTrigger({
  side = "left",
  className,
  onClick,
  ...props
}: React.ComponentProps<typeof Button> & { side?: SidebarSide }) {
  const { state, toggleSidebar } = useSidebar(side);

  return (
    <Button
      aria-pressed={state === "expanded"}
      data-sidebar="trigger"
      data-slot="sidebar-trigger"
      variant="ghost"
      size="icon-sm"
      className={cn(className)}
      onClick={(event) => {
        onClick?.(event);
        toggleSidebar();
      }}
      {...props}
    >
      <HugeiconsIcon icon={side === "left" ? LayoutLeftIcon : LayoutRightIcon} strokeWidth={2} />
      <span className="sr-only">Toggle {side} sidebar</span>
    </Button>
  );
}

function SidebarRail({
  side = "left",
  className,
  onLostPointerCapture,
  onClick,
  onKeyDown,
  onPointerCancel,
  onPointerDown,
  onPointerMove,
  onPointerUp,
  ...props
}: React.ComponentProps<"button"> & { side?: SidebarSide }) {
  const { sidebarId, setDragging, setWidth, state, toggleSidebar, width, widthConfig, wrapperRef } =
    useSidebar(side);
  const dragRef = React.useRef<{
    pointerId: number;
    startX: number;
    startWidth: number;
    currentWidth: number;
    pixelsPerRem: number;
    moved: boolean;
  } | null>(null);
  const railRef = React.useRef<HTMLButtonElement>(null);
  const suppressClickRef = React.useRef(false);

  const publishWidth = React.useCallback(
    (nextWidth: number) => {
      const clampedWidth = clampWidth(nextWidth, widthConfig);
      wrapperRef.current?.style.setProperty("--sidebar-width", `${clampedWidth}rem`);
      railRef.current?.setAttribute("aria-valuenow", String(clampedWidth));
      railRef.current?.setAttribute("aria-valuetext", `${clampedWidth} rem`);
      return clampedWidth;
    },
    [widthConfig, wrapperRef],
  );

  const finishResize = React.useCallback(
    (pointerId: number, suppressClick: boolean) => {
      const drag = dragRef.current;
      if (!drag || drag.pointerId !== pointerId) return false;

      dragRef.current = null;
      setDragging(false);
      if (drag.moved) {
        setWidth(drag.currentWidth);
        if (suppressClick) {
          suppressClickRef.current = true;
          window.setTimeout(() => {
            suppressClickRef.current = false;
          });
        }
      }

      return true;
    },
    [setDragging, setWidth],
  );

  return (
    <button
      ref={railRef}
      data-sidebar="rail"
      data-slot="sidebar-rail"
      aria-controls={sidebarId}
      aria-label={`${side} sidebar width`}
      aria-orientation="vertical"
      aria-valuemax={widthConfig.max}
      aria-valuemin={widthConfig.min}
      aria-valuenow={width}
      aria-valuetext={`${width} rem`}
      role="separator"
      tabIndex={0}
      title={`Resize or toggle ${side} sidebar`}
      onLostPointerCapture={(event) => {
        onLostPointerCapture?.(event);
        finishResize(event.pointerId, true);
      }}
      onClick={(event) => {
        onClick?.(event);
        if (event.defaultPrevented) return;

        if (suppressClickRef.current) {
          suppressClickRef.current = false;
          event.preventDefault();
          return;
        }

        toggleSidebar();
      }}
      onKeyDown={(event) => {
        onKeyDown?.(event);
        if (event.defaultPrevented) return;

        let nextWidth: number | undefined;
        if (event.key === "Home") nextWidth = widthConfig.min;
        if (event.key === "End") nextWidth = widthConfig.max;
        if (event.key === "ArrowLeft") {
          nextWidth = width + widthConfig.keyboardStep * (side === "left" ? -1 : 1);
        }
        if (event.key === "ArrowRight") {
          nextWidth = width + widthConfig.keyboardStep * (side === "left" ? 1 : -1);
        }

        if (nextWidth !== undefined) {
          event.preventDefault();
          setWidth(publishWidth(nextWidth));
        }
      }}
      onPointerDown={(event) => {
        onPointerDown?.(event);
        if (event.defaultPrevented || event.button !== 0 || state === "collapsed") return;

        const pixelsPerRem = Number.parseFloat(
          window.getComputedStyle(document.documentElement).fontSize,
        );
        dragRef.current = {
          pointerId: event.pointerId,
          startX: event.clientX,
          startWidth: width,
          currentWidth: width,
          pixelsPerRem: Number.isFinite(pixelsPerRem) ? pixelsPerRem : 16,
          moved: false,
        };
        event.currentTarget.setPointerCapture(event.pointerId);
        setDragging(true);
      }}
      onPointerMove={(event) => {
        onPointerMove?.(event);
        const drag = dragRef.current;
        if (!drag || drag.pointerId !== event.pointerId) return;

        const delta = event.clientX - drag.startX;
        if (!drag.moved && Math.abs(delta) < SIDEBAR_RESIZE_THRESHOLD) return;

        event.preventDefault();
        drag.moved = true;
        const direction = side === "left" ? 1 : -1;
        drag.currentWidth = publishWidth(drag.startWidth + (delta / drag.pixelsPerRem) * direction);
      }}
      onPointerUp={(event) => {
        onPointerUp?.(event);
        if (!finishResize(event.pointerId, true)) return;

        if (event.currentTarget.hasPointerCapture(event.pointerId)) {
          event.currentTarget.releasePointerCapture(event.pointerId);
        }
      }}
      onPointerCancel={(event) => {
        onPointerCancel?.(event);
        finishResize(event.pointerId, false);
      }}
      className={cn(
        "absolute inset-y-0 z-20 hidden w-4 touch-none select-none transition-all ease-linear [-webkit-app-region:no-drag] group-data-[side=left]:-right-4 group-data-[side=right]:left-0 after:absolute after:inset-y-0 after:start-1/2 after:w-[2px] hover:after:bg-sidebar-border focus-visible:after:bg-sidebar-border focus-visible:outline-none sm:flex ltr:-translate-x-1/2 rtl:-translate-x-1/2",
        "in-data-[side=left]:cursor-w-resize in-data-[side=right]:cursor-e-resize",
        "[[data-side=left][data-state=collapsed]_&]:cursor-e-resize [[data-side=right][data-state=collapsed]_&]:cursor-w-resize",
        "group-data-[collapsible=offcanvas]:translate-x-0 group-data-[collapsible=offcanvas]:after:left-full hover:group-data-[collapsible=offcanvas]:bg-sidebar",
        "[[data-side=left][data-collapsible=offcanvas]_&]:-right-2",
        "[[data-side=right][data-collapsible=offcanvas]_&]:-left-2",
        className,
      )}
      {...props}
    />
  );
}

function SidebarInset({ className, ...props }: React.ComponentProps<"main">) {
  return (
    <main
      data-slot="sidebar-inset"
      className={cn(
        "relative flex w-full flex-1 flex-col bg-background md:peer-data-[variant=inset]:m-2 md:peer-data-[variant=inset]:ml-0 md:peer-data-[variant=inset]:rounded-xl md:peer-data-[variant=inset]:shadow-sm md:peer-data-[variant=inset]:peer-data-[state=collapsed]:ml-2",
        className,
      )}
      {...props}
    />
  );
}

function SidebarInput({ className, ...props }: React.ComponentProps<typeof Input>) {
  return (
    <Input
      data-slot="sidebar-input"
      data-sidebar="input"
      className={cn("h-8 w-full border-input bg-muted/20 dark:bg-muted/30", className)}
      {...props}
    />
  );
}

function SidebarHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-header"
      data-sidebar="header"
      className={cn("flex flex-col gap-2 p-2", className)}
      {...props}
    />
  );
}

function SidebarFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-footer"
      data-sidebar="footer"
      className={cn("flex flex-col gap-2 p-2", className)}
      {...props}
    />
  );
}

function SidebarSeparator({ className, ...props }: React.ComponentProps<typeof Separator>) {
  return (
    <Separator
      data-slot="sidebar-separator"
      data-sidebar="separator"
      className={cn("mx-2 w-auto bg-sidebar-border", className)}
      {...props}
    />
  );
}

function SidebarContent({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-content"
      data-sidebar="content"
      className={cn(
        "no-scrollbar flex min-h-0 flex-1 flex-col gap-0 overflow-auto group-data-[collapsible=icon]:overflow-hidden",
        className,
      )}
      {...props}
    />
  );
}

function SidebarGroup({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-group"
      data-sidebar="group"
      className={cn("relative flex w-full min-w-0 flex-col px-2 py-1", className)}
      {...props}
    />
  );
}

function SidebarGroupLabel({
  className,
  render,
  ...props
}: useRender.ComponentProps<"div"> & React.ComponentProps<"div">) {
  return useRender({
    defaultTagName: "div",
    props: mergeProps<"div">(
      {
        className: cn(
          "flex h-8 shrink-0 items-center rounded-md px-2 text-xs text-sidebar-foreground/70 ring-sidebar-ring outline-hidden transition-[margin,opacity] duration-200 ease-linear group-data-[collapsible=icon]:-mt-8 group-data-[collapsible=icon]:opacity-0 focus-visible:ring-2 [&>svg]:size-4 [&>svg]:shrink-0",
          className,
        ),
      },
      props,
    ),
    render,
    state: {
      slot: "sidebar-group-label",
      sidebar: "group-label",
    },
  });
}

function SidebarGroupAction({
  className,
  render,
  ...props
}: useRender.ComponentProps<"button"> & React.ComponentProps<"button">) {
  return useRender({
    defaultTagName: "button",
    props: mergeProps<"button">(
      {
        className: cn(
          "absolute top-3.5 right-3 flex aspect-square w-5 items-center justify-center rounded-md p-0 text-sidebar-foreground ring-sidebar-ring outline-hidden transition-transform group-data-[collapsible=icon]:hidden after:absolute after:-inset-2 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 md:after:hidden [&>svg]:size-4 [&>svg]:shrink-0",
          className,
        ),
      },
      props,
    ),
    render,
    state: {
      slot: "sidebar-group-action",
      sidebar: "group-action",
    },
  });
}

function SidebarGroupContent({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-group-content"
      data-sidebar="group-content"
      className={cn("w-full text-xs", className)}
      {...props}
    />
  );
}

function SidebarMenu({ className, ...props }: React.ComponentProps<"ul">) {
  return (
    <ul
      data-slot="sidebar-menu"
      data-sidebar="menu"
      className={cn("flex w-full min-w-0 flex-col gap-px", className)}
      {...props}
    />
  );
}

function SidebarMenuItem({ className, ...props }: React.ComponentProps<"li">) {
  return (
    <li
      data-slot="sidebar-menu-item"
      data-sidebar="menu-item"
      className={cn("group/menu-item relative", className)}
      {...props}
    />
  );
}

const sidebarMenuButtonVariants = cva(
  "peer/menu-button group/menu-button flex w-full items-center gap-2 overflow-hidden rounded-[calc(var(--radius-sm)+2px)] p-2 text-left text-xs ring-sidebar-ring outline-hidden transition-[width,height,padding] group-has-data-[sidebar=menu-action]/menu-item:pr-8 group-data-[collapsible=icon]:size-8! group-data-[collapsible=icon]:p-2! hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 active:bg-sidebar-accent active:text-sidebar-accent-foreground disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 data-open:hover:bg-sidebar-accent data-open:hover:text-sidebar-accent-foreground data-active:bg-sidebar-accent data-active:font-medium data-active:text-sidebar-accent-foreground [&_svg]:size-4 [&_svg]:shrink-0 [&>span:last-child]:truncate",
  {
    variants: {
      variant: {
        default: "hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
        outline:
          "bg-background shadow-[0_0_0_1px_var(--sidebar-border)] hover:bg-sidebar-accent hover:text-sidebar-accent-foreground hover:shadow-[0_0_0_1px_var(--sidebar-accent)]",
      },
      size: {
        default: "h-8 text-xs",
        sm: "h-7 text-xs",
        lg: "h-12 text-xs group-data-[collapsible=icon]:p-0!",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

function SidebarMenuButton({
  render,
  isActive = false,
  variant = "default",
  size = "default",
  tooltip,
  className,
  ...props
}: useRender.ComponentProps<"button"> &
  React.ComponentProps<"button"> & {
    isActive?: boolean;
    tooltip?: string | React.ComponentProps<typeof TooltipContent>;
  } & VariantProps<typeof sidebarMenuButtonVariants>) {
  const { isMobile, state } = useSidebar();
  const comp = useRender({
    defaultTagName: "button",
    props: mergeProps<"button">(
      {
        className: cn(sidebarMenuButtonVariants({ variant, size }), className),
      },
      props,
    ),
    render: !tooltip ? render : <TooltipTrigger render={render} />,
    state: {
      slot: "sidebar-menu-button",
      sidebar: "menu-button",
      size,
      active: isActive,
    },
  });

  if (!tooltip) {
    return comp;
  }

  if (typeof tooltip === "string") {
    tooltip = {
      children: tooltip,
    };
  }

  return (
    <Tooltip>
      {comp}
      <TooltipContent
        side="right"
        align="center"
        hidden={state !== "collapsed" || isMobile}
        {...tooltip}
      />
    </Tooltip>
  );
}

function SidebarMenuAction({
  className,
  render,
  showOnHover = false,
  ...props
}: useRender.ComponentProps<"button"> &
  React.ComponentProps<"button"> & {
    showOnHover?: boolean;
  }) {
  return useRender({
    defaultTagName: "button",
    props: mergeProps<"button">(
      {
        className: cn(
          "absolute top-1.5 right-1 flex aspect-square w-5 items-center justify-center rounded-[calc(var(--radius-sm)-2px)] p-0 text-sidebar-foreground ring-sidebar-ring outline-hidden transition-transform group-data-[collapsible=icon]:hidden peer-hover/menu-button:text-sidebar-accent-foreground peer-data-[size=default]/menu-button:top-1.5 peer-data-[size=lg]/menu-button:top-2.5 peer-data-[size=sm]/menu-button:top-1 after:absolute after:-inset-2 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 md:after:hidden [&>svg]:size-4 [&>svg]:shrink-0",
          showOnHover &&
            "group-focus-within/menu-item:opacity-100 group-hover/menu-item:opacity-100 peer-data-active/menu-button:text-sidebar-accent-foreground aria-expanded:opacity-100 md:opacity-0",
          className,
        ),
      },
      props,
    ),
    render,
    state: {
      slot: "sidebar-menu-action",
      sidebar: "menu-action",
    },
  });
}

function SidebarMenuBadge({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sidebar-menu-badge"
      data-sidebar="menu-badge"
      className={cn(
        "pointer-events-none absolute right-1 flex h-5 min-w-5 items-center justify-center rounded-[calc(var(--radius-sm)-2px)] px-1 text-xs font-medium text-sidebar-foreground tabular-nums select-none group-data-[collapsible=icon]:hidden peer-hover/menu-button:text-sidebar-accent-foreground peer-data-[size=default]/menu-button:top-1.5 peer-data-[size=lg]/menu-button:top-2.5 peer-data-[size=sm]/menu-button:top-1 peer-data-active/menu-button:text-sidebar-accent-foreground",
        className,
      )}
      {...props}
    />
  );
}

function SidebarMenuSkeleton({
  className,
  showIcon = false,
  ...props
}: React.ComponentProps<"div"> & {
  showIcon?: boolean;
}) {
  const [width] = React.useState(() => {
    return `${Math.floor(Math.random() * 40) + 50}%`;
  });

  return (
    <div
      data-slot="sidebar-menu-skeleton"
      data-sidebar="menu-skeleton"
      className={cn("flex h-8 items-center gap-2 rounded-md px-2", className)}
      {...props}
    >
      {showIcon && <Skeleton className="size-4 rounded-md" data-sidebar="menu-skeleton-icon" />}
      <Skeleton
        className="h-4 max-w-(--skeleton-width) flex-1"
        data-sidebar="menu-skeleton-text"
        style={
          {
            "--skeleton-width": width,
          } as React.CSSProperties
        }
      />
    </div>
  );
}

function SidebarMenuSub({ className, ...props }: React.ComponentProps<"ul">) {
  return (
    <ul
      data-slot="sidebar-menu-sub"
      data-sidebar="menu-sub"
      className={cn(
        "mx-3.5 flex min-w-0 translate-x-px flex-col gap-1 border-l border-sidebar-border px-2.5 py-0.5 group-data-[collapsible=icon]:hidden",
        className,
      )}
      {...props}
    />
  );
}

function SidebarMenuSubItem({ className, ...props }: React.ComponentProps<"li">) {
  return (
    <li
      data-slot="sidebar-menu-sub-item"
      data-sidebar="menu-sub-item"
      className={cn("group/menu-sub-item relative", className)}
      {...props}
    />
  );
}

function SidebarMenuSubButton({
  render,
  size = "md",
  isActive = false,
  className,
  ...props
}: useRender.ComponentProps<"a"> &
  React.ComponentProps<"a"> & {
    size?: "sm" | "md";
    isActive?: boolean;
  }) {
  return useRender({
    defaultTagName: "a",
    props: mergeProps<"a">(
      {
        className: cn(
          "flex h-7 min-w-0 -translate-x-px items-center gap-2 overflow-hidden rounded-md px-2 text-sidebar-foreground ring-sidebar-ring outline-hidden group-data-[collapsible=icon]:hidden hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 active:bg-sidebar-accent active:text-sidebar-accent-foreground disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 data-[size=md]:text-xs data-[size=sm]:text-xs data-active:bg-sidebar-accent data-active:text-sidebar-accent-foreground [&>span:last-child]:truncate [&>svg]:size-4 [&>svg]:shrink-0 [&>svg]:text-sidebar-accent-foreground",
          className,
        ),
      },
      props,
    ),
    render,
    state: {
      slot: "sidebar-menu-sub-button",
      sidebar: "menu-sub-button",
      size,
      active: isActive,
    },
  });
}

export {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupAction,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInput,
  SidebarInset,
  SidebarMenu,
  SidebarMenuAction,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSkeleton,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  SidebarProvider,
  SidebarRail,
  SidebarSeparator,
  SidebarTrigger,
  useSidebar,
};

export type { SidebarWidthConfig };
