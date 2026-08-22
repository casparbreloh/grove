"use client";

import { PanelLeftIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import * as React from "react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Sheet,
  SheetDescription,
  SheetHeader,
  SheetPopup,
  SheetTitle,
} from "@/components/ui/sheet";
import { useMediaQuery } from "@/hooks/use-media-query";
import { cn } from "@/lib/utils";

const SIDEBAR_WIDTH = "14rem";
const SIDEBAR_WIDTH_MOBILE = "16rem";
const SIDEBAR_KEYBOARD_SHORTCUT = "b";

type SidebarShellContextValue = {
  isMobile: boolean;
  open: boolean;
  openMobile: boolean;
  setOpenMobile: React.Dispatch<React.SetStateAction<boolean>>;
  toggleSidebar: () => void;
};

const SidebarShellContext = React.createContext<SidebarShellContextValue | null>(null);

export function useSidebarShell(): SidebarShellContextValue {
  const context = React.useContext(SidebarShellContext);
  if (!context) {
    throw new Error("useSidebarShell must be used within SidebarShellProvider.");
  }

  return context;
}

export function SidebarShellProvider({
  className,
  style,
  children,
  ...props
}: React.ComponentProps<"div">): React.ReactElement {
  const isMobile = useMediaQuery("max-md");
  const [open, setOpen] = React.useState(true);
  const [openMobile, setOpenMobile] = React.useState(false);

  const toggleSidebar = React.useCallback(() => {
    if (isMobile) {
      setOpenMobile((currentOpen) => !currentOpen);
      return;
    }

    setOpen((currentOpen) => !currentOpen);
  }, [isMobile]);

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

  const contextValue = React.useMemo<SidebarShellContextValue>(
    () => ({ isMobile, open, openMobile, setOpenMobile, toggleSidebar }),
    [isMobile, open, openMobile, toggleSidebar],
  );

  return (
    <SidebarShellContext.Provider value={contextValue}>
      <div
        className={cn("group/sidebar-shell flex min-h-svh w-full bg-sidebar", className)}
        style={{ "--sidebar-width": SIDEBAR_WIDTH, ...style } as React.CSSProperties}
        {...props}
      >
        {children}
      </div>
    </SidebarShellContext.Provider>
  );
}

export function SidebarPanel({
  className,
  children,
}: Pick<React.ComponentProps<"aside">, "children" | "className">): React.ReactElement {
  const { isMobile, open, openMobile, setOpenMobile } = useSidebarShell();

  if (isMobile) {
    return (
      <Sheet onOpenChange={setOpenMobile} open={openMobile}>
        <SheetPopup
          className="w-(--sidebar-width) bg-sidebar p-0 text-sidebar-foreground [&>button]:hidden"
          side="left"
          style={{ "--sidebar-width": SIDEBAR_WIDTH_MOBILE } as React.CSSProperties}
        >
          <SheetHeader className="sr-only">
            <SheetTitle>Sidebar</SheetTitle>
            <SheetDescription>Displays the Grove sidebar.</SheetDescription>
          </SheetHeader>
          <div className="flex h-full w-full flex-col">{children}</div>
        </SheetPopup>
      </Sheet>
    );
  }

  return (
    <div
      className="peer hidden text-sidebar-foreground md:block"
      data-state={open ? "expanded" : "collapsed"}
    >
      <div
        className={cn(
          "relative w-(--sidebar-width) bg-transparent transition-[width] duration-200 ease-linear",
          !open && "w-0",
        )}
      />
      <aside
        className={cn(
          "fixed inset-y-0 left-0 z-10 hidden h-svh w-(--sidebar-width) transition-[left] duration-200 ease-linear md:flex",
          !open && "left-[calc(var(--sidebar-width)*-1)]",
          className,
        )}
      >
        <div className="flex h-full w-full flex-col bg-sidebar">{children}</div>
      </aside>
    </div>
  );
}

export function SidebarToggle({
  className,
  onClick,
  ...props
}: React.ComponentProps<typeof Button>): React.ReactElement {
  const { toggleSidebar } = useSidebarShell();

  return (
    <Button
      aria-label="Toggle sidebar"
      className={cn("size-7", className)}
      onClick={(event: React.MouseEvent<HTMLButtonElement>) => {
        onClick?.(event);
        toggleSidebar();
      }}
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
        "relative flex w-full flex-1 flex-col bg-background md:me-2 md:mb-2 md:rounded-xl md:shadow-sm/5 md:peer-data-[state=collapsed]:ms-2",
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
