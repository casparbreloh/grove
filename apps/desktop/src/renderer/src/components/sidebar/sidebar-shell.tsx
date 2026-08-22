"use client";

import { PanelLeftIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import * as React from "react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

const SIDEBAR_WIDTH = "14rem";
const SIDEBAR_KEYBOARD_SHORTCUT = "b";

type SidebarVisibility = {
  open: boolean;
  toggleSidebar: () => void;
};

export function useSidebarVisibility(): SidebarVisibility {
  const [open, setOpen] = React.useState(true);
  const toggleSidebar = React.useCallback(() => setOpen((currentOpen) => !currentOpen), []);

  React.useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key === SIDEBAR_KEYBOARD_SHORTCUT && (event.metaKey || event.ctrlKey)) {
        event.preventDefault();
        toggleSidebar();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [toggleSidebar]);

  return { open, toggleSidebar };
}

export function SidebarShell({
  className,
  style,
  children,
  ...props
}: React.ComponentProps<"div">): React.ReactElement {
  return (
    <div
      className={cn("group/sidebar-shell flex min-h-svh w-full bg-transparent", className)}
      style={{ "--sidebar-width": SIDEBAR_WIDTH, ...style } as React.CSSProperties}
      {...props}
    >
      {children}
    </div>
  );
}

export function SidebarPanel({
  open,
  className,
  children,
  ...props
}: React.ComponentProps<"aside"> & { open: boolean }): React.ReactElement {
  return (
    <div className="peer text-foreground" data-state={open ? "expanded" : "collapsed"}>
      <div
        className={cn(
          "relative w-(--sidebar-width) bg-transparent transition-[width] duration-150 ease-linear",
          !open && "w-0",
        )}
      />
      <aside
        className={cn(
          "fixed inset-y-0 left-0 z-10 flex h-svh w-(--sidebar-width) transition-[left] duration-150 ease-linear",
          !open && "left-[calc(var(--sidebar-width)*-1)]",
          className,
        )}
        {...props}
      >
        <div className="flex h-full w-full flex-col bg-transparent">{children}</div>
      </aside>
    </div>
  );
}

export function SidebarToggle({
  onToggle,
  className,
  ...props
}: Omit<React.ComponentProps<typeof Button>, "onClick"> & {
  onToggle: () => void;
}): React.ReactElement {
  return (
    <Button
      aria-label="Toggle sidebar"
      className={cn("size-7", className)}
      onClick={onToggle}
      size="icon"
      variant="ghost"
      {...props}
    >
      <HugeiconsIcon icon={PanelLeftIcon} strokeWidth={2} />
    </Button>
  );
}

export function SidebarInset({
  className,
  ...props
}: React.ComponentProps<"main">): React.ReactElement {
  return (
    <main
      className={cn(
        "relative me-2 mb-2 flex w-full flex-1 flex-col rounded-xl bg-background shadow-sm/5 peer-data-[state=collapsed]:ms-2",
        className,
      )}
      {...props}
    />
  );
}

export function SidebarScrollArea({
  className,
  children,
}: Pick<React.ComponentProps<"div">, "children" | "className">): React.ReactElement {
  return (
    <ScrollArea className="min-h-0 flex-1" fill overscrollContain scrollFade>
      <div className={cn("flex h-full flex-col gap-1", className)}>{children}</div>
    </ScrollArea>
  );
}
