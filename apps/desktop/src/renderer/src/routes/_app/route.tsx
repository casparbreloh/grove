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
import { Outlet, createFileRoute } from "@tanstack/react-router";
import { AppSidebar } from "@/components/sidebar/app-sidebar";
import { SidePane } from "@/components/side-pane";
import { tabRegistry } from "@/components/tabs/registry";
import { Button } from "@/components/ui/button";
import { PaneSplit, PaneSplitProvider, usePaneSplit } from "@/components/ui/pane-split";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SidebarInset, SidebarProvider, SidebarTrigger, useSidebar } from "@/components/ui/sidebar";

export const Route = createFileRoute("/_app")({ component: AppLayout });

function AppLayout() {
  return (
    <SidebarProvider className="desktop-shell h-svh overflow-hidden">
      <AppSidebar />
      <PaneSplitProvider>
        <PaneSplit
          defaultMainPaneSize="50%"
          keyboardStep="1rem"
          mainPane={
            <SidebarInset className="min-h-0 min-w-0 overflow-hidden">
              <DesktopHeader />
              <div className="flex min-h-0 flex-1 flex-col">
                <Outlet />
              </div>
            </SidebarInset>
          }
          minimumSize="20rem"
          sidePane={<SidePane />}
        />
        <DesktopControls />
        <SidePaneControls />
      </PaneSplitProvider>
    </SidebarProvider>
  );
}

function DesktopControls() {
  const appSidebarOpen = useAppSidebarOpen();

  return (
    <div className="fixed top-0 left-0 z-20 flex h-10 items-center gap-1 pr-1 pl-[calc(var(--desktop-header-safe-area-left)+0.5rem)] [-webkit-app-region:no-drag]">
      <SidebarTrigger size="icon" />
      <div className="flex items-center gap-1">
        <Button
          aria-label="Go back"
          onClick={() => window.history.back()}
          size="icon"
          variant="ghost"
        >
          <HugeiconsIcon icon={ArrowLeft02Icon} strokeWidth={2} />
        </Button>
        <Button
          aria-label="Go forward"
          onClick={() => window.history.forward()}
          size="icon"
          variant="ghost"
        >
          <HugeiconsIcon icon={ArrowRight02Icon} strokeWidth={2} />
        </Button>
      </div>
      {!appSidebarOpen && <NewTabMenu />}
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
          size="icon"
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
        size="icon"
        title="Toggle side pane"
        variant="ghost"
      >
        <HugeiconsIcon icon={LayoutRightIcon} strokeWidth={2} />
      </Button>
    </div>
  );
}

function DesktopHeader() {
  const appSidebarOpen = useAppSidebarOpen();

  return (
    <header className="relative flex h-10 shrink-0 items-center bg-background [-webkit-app-region:drag]">
      <span
        className="flex h-7 items-center gap-1.5 pr-3 text-xs/3.5 font-medium transition-[padding] duration-150 ease-linear data-[app-sidebar-open=false]:pl-[var(--desktop-header-controls-width)] data-[app-sidebar-open=true]:pl-3"
        data-app-sidebar-open={appSidebarOpen}
      >
        <HugeiconsIcon className="size-3.5" icon={Layers01Icon} strokeWidth={2} />
        <span>Chat</span>
      </span>
    </header>
  );
}

function useAppSidebarOpen() {
  const { isMobile, open, openMobile } = useSidebar();
  return isMobile ? openMobile : open;
}

function NewTabMenu() {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={<Button aria-label="New tab" size="icon" type="button" variant="ghost" />}
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
  );
}
