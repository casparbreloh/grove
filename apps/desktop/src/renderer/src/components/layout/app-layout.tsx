import {
  ArrowLeft02Icon,
  ArrowRight02Icon,
  CollapseIcon,
  ExpandIcon,
  Layers01Icon,
  LayoutRightIcon,
  PencilEdit02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Outlet } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import { useAppSidebarOpen } from "@/hooks/use-app-sidebar-open";
import { AppSidebar } from "./app-sidebar";
import { PaneSplit, PaneSplitProvider, usePaneSplit } from "./pane-split";
import { SidePane } from "./side-pane";
import { tabRegistry } from "./tabs/registry";

export function AppLayout() {
  return (
    <SidebarProvider className="desktop-shell h-svh overflow-hidden">
      <AppSidebar />
      <PaneSplitProvider>
        <PaneSplit
          mainPane={
            <SidebarInset className="min-h-0 min-w-0 overflow-hidden">
              <AppHeader />
              <div className="flex min-h-0 flex-1 flex-col">
                <Outlet />
              </div>
            </SidebarInset>
          }
          sidePane={<SidePane />}
        />
        <AppControls />
        <SidePaneControls />
      </PaneSplitProvider>
    </SidebarProvider>
  );
}

function AppHeader() {
  const appSidebarOpen = useAppSidebarOpen();

  return (
    <header className="relative flex h-10 shrink-0 items-center bg-background [-webkit-app-region:drag]">
      <span
        className="flex h-8 translate-x-0 items-center gap-2 pr-3 pl-3 text-sm font-medium transition-transform duration-150 ease-linear data-[app-sidebar-open=false]:translate-x-[calc(var(--desktop-header-controls-width)-0.75rem)]"
        data-app-sidebar-open={appSidebarOpen}
      >
        <HugeiconsIcon className="size-4" icon={Layers01Icon} strokeWidth={2} />
        <span>Chat</span>
      </span>
    </header>
  );
}

function AppControls() {
  const appSidebarOpen = useAppSidebarOpen();

  return (
    <div className="fixed top-0 left-0 z-20 flex h-10 items-center gap-1 pr-1 pl-[calc(var(--desktop-header-safe-area-left)+0.5rem)] [-webkit-app-region:no-drag]">
      <SidebarTrigger size="icon-sm" />
      <div className="flex items-center gap-1">
        <Button
          aria-label="Go back"
          onClick={() => window.history.back()}
          size="icon-sm"
          variant="ghost"
        >
          <HugeiconsIcon icon={ArrowLeft02Icon} strokeWidth={2} />
        </Button>
        <Button
          aria-label="Go forward"
          onClick={() => window.history.forward()}
          size="icon-sm"
          variant="ghost"
        >
          <HugeiconsIcon icon={ArrowRight02Icon} strokeWidth={2} />
        </Button>
      </div>
      {!appSidebarOpen && (
        <DropdownMenu>
          <DropdownMenuTrigger
            render={<Button aria-label="New tab" size="icon-sm" type="button" variant="ghost" />}
          >
            <HugeiconsIcon icon={PencilEdit02Icon} strokeWidth={2} />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            {tabRegistry.map((registration) => (
              <DropdownMenuItem key={registration.kind} onClick={registration.create}>
                <HugeiconsIcon icon={registration.icon} strokeWidth={2} />
                {registration.label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  );
}

function SidePaneControls() {
  const { isSidePaneMaximized, isSidePaneOpen, toggleSidePane, toggleSidePaneMaximized } =
    usePaneSplit();

  return (
    <div className="fixed top-1.5 right-2 z-20 flex items-center gap-1 [-webkit-app-region:no-drag]">
      {isSidePaneOpen && (
        <Button
          aria-label={isSidePaneMaximized ? "Restore split view" : "Maximize side pane"}
          aria-pressed={isSidePaneMaximized}
          className="aria-pressed:bg-accent aria-pressed:text-accent-foreground"
          onClick={toggleSidePaneMaximized}
          size="icon-sm"
          title={isSidePaneMaximized ? "Restore split view" : "Maximize side pane"}
          variant="ghost"
        >
          <HugeiconsIcon icon={isSidePaneMaximized ? CollapseIcon : ExpandIcon} strokeWidth={2} />
        </Button>
      )}
      <Button
        aria-label="Toggle side pane"
        aria-pressed={isSidePaneOpen}
        className="aria-pressed:bg-accent aria-pressed:text-accent-foreground"
        onClick={toggleSidePane}
        size="icon-sm"
        title="Toggle side pane"
        variant="ghost"
      >
        <HugeiconsIcon icon={LayoutRightIcon} strokeWidth={2} />
      </Button>
    </div>
  );
}
