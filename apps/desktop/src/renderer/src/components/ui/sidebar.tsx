"use client";

import { PanelLeftIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { cva, type VariantProps } from "class-variance-authority";
import * as React from "react";
import { Button } from "@/components/ui/button";
import { useSidebar } from "@/components/ui/sidebar-context";
import { cn } from "@/lib/utils";

function Sidebar({
  variant = "sidebar",
  className,
  children,
  ...props
}: React.ComponentProps<"aside"> & {
  variant?: "sidebar" | "inset";
}) {
  const { state } = useSidebar();

  return (
    <div
      className="group peer text-sidebar-foreground"
      data-collapsible={state === "collapsed" ? "offcanvas" : ""}
      data-slot="sidebar"
      data-state={state}
      data-variant={variant}
    >
      <div
        className="relative w-(--sidebar-width) bg-transparent transition-[width] duration-150 ease-linear group-data-[collapsible=offcanvas]:w-0"
        data-slot="sidebar-gap"
      />
      <aside
        aria-hidden={state === "collapsed"}
        className={cn(
          "fixed inset-y-0 left-0 z-10 flex h-svh w-(--sidebar-width) transition-[left,width] duration-150 ease-linear group-data-[collapsible=offcanvas]:left-[calc(var(--sidebar-width)*-1)]",
          className,
        )}
        data-slot="sidebar-container"
        inert={state === "collapsed"}
        {...props}
      >
        <div
          className="flex size-full flex-col bg-sidebar group-data-[variant=inset]:bg-transparent"
          data-sidebar="sidebar"
          data-slot="sidebar-inner"
        >
          {children}
        </div>
      </aside>
    </div>
  );
}

function SidebarTrigger({ className, onClick, ...props }: React.ComponentProps<typeof Button>) {
  const { toggleSidebar } = useSidebar();

  return (
    <Button
      className={className}
      data-sidebar="trigger"
      data-slot="sidebar-trigger"
      onClick={(event) => {
        onClick?.(event);
        toggleSidebar();
      }}
      size="icon"
      variant="sidebar"
      {...props}
    >
      <HugeiconsIcon icon={PanelLeftIcon} strokeWidth={2} />
      <span className="sr-only">Toggle sidebar</span>
    </Button>
  );
}

function SidebarInset({ className, ...props }: React.ComponentProps<"main">) {
  return (
    <main
      className={cn(
        "relative flex w-full flex-1 flex-col bg-background peer-data-[variant=inset]:me-2 peer-data-[variant=inset]:mb-2 peer-data-[variant=inset]:rounded-xl peer-data-[variant=inset]:shadow-sm peer-data-[variant=inset]:peer-data-[state=collapsed]:ms-2",
        className,
      )}
      data-slot="sidebar-inset"
      {...props}
    />
  );
}

function SidebarContent({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      className={cn("flex min-h-0 flex-1 flex-col gap-0 overflow-auto", className)}
      data-sidebar="content"
      data-slot="sidebar-content"
      {...props}
    />
  );
}

function SidebarGroup({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      className={cn("relative flex w-full min-w-0 flex-col px-2 py-1", className)}
      data-sidebar="group"
      data-slot="sidebar-group"
      {...props}
    />
  );
}

function SidebarGroupLabel({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      className={cn(
        "flex h-7 shrink-0 items-center rounded-md px-2 text-xs text-muted-foreground outline-hidden focus-visible:ring-2 focus-visible:ring-sidebar-ring",
        className,
      )}
      data-sidebar="group-label"
      data-slot="sidebar-group-label"
      {...props}
    />
  );
}

function SidebarGroupContent({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      className={cn("w-full text-xs", className)}
      data-sidebar="group-content"
      data-slot="sidebar-group-content"
      {...props}
    />
  );
}

function SidebarMenu({ className, ...props }: React.ComponentProps<"ul">) {
  return (
    <ul
      className={cn("flex w-full min-w-0 flex-col gap-px", className)}
      data-sidebar="menu"
      data-slot="sidebar-menu"
      {...props}
    />
  );
}

function SidebarMenuItem({ className, ...props }: React.ComponentProps<"li">) {
  return (
    <li
      className={cn("group/menu-item relative", className)}
      data-sidebar="menu-item"
      data-slot="sidebar-menu-item"
      {...props}
    />
  );
}

const sidebarMenuButtonVariants = cva(
  "peer/menu-button group/menu-button flex w-full items-center gap-2 overflow-hidden rounded-[calc(var(--radius-sm)+2px)] p-2 text-left text-xs ring-sidebar-ring outline-hidden transition-[width,height,padding] hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 active:bg-sidebar-accent active:text-sidebar-accent-foreground disabled:pointer-events-none disabled:opacity-50 aria-disabled:pointer-events-none aria-disabled:opacity-50 [&_svg]:size-4 [&_svg]:shrink-0 [&>span:last-child]:truncate",
  {
    variants: {
      size: {
        default: "h-8 text-xs",
        sm: "h-7 text-xs",
      },
    },
    defaultVariants: {
      size: "default",
    },
  },
);

function SidebarMenuButton({
  className,
  size,
  ...props
}: React.ComponentProps<"button"> & VariantProps<typeof sidebarMenuButtonVariants>) {
  return (
    <button
      className={cn(sidebarMenuButtonVariants({ size }), className)}
      data-sidebar="menu-button"
      data-size={size}
      data-slot="sidebar-menu-button"
      type="button"
      {...props}
    />
  );
}

export {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarTrigger,
};
