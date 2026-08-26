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
import { useMockSidePaneTabs } from "@/lib/mock-side-pane-state";
import { AppSidebar } from "./app-sidebar";
import {
  SidePaneLayout,
  SidePaneLayoutProvider,
  useSidePaneLayout,
} from "./side-pane/side-pane-layout";
import { SidePaneTabCreationMenu, SidePaneTabs } from "./side-pane/side-pane-tabs";
import { mainPaneTabDefinitions } from "./tabs/main-pane-tabs";

export function AppLayout() {
  return (
    <SidebarProvider className="desktop-shell h-svh overflow-hidden">
      <AppSidebar />
      <SidePaneLayoutProvider>
        <SidePaneLayout mainPaneContent={<MainPane />} sidePaneContent={<SidePaneTabs />} />
        <AppTitlebarControls />
        <SidePaneTitlebarControls />
      </SidePaneLayoutProvider>
    </SidebarProvider>
  );
}

function MainPane() {
  return (
    <SidebarInset className="min-h-0 min-w-0 overflow-hidden">
      <MainPaneHeader />
      <div className="flex min-h-0 flex-1 flex-col">
        <Outlet />
      </div>
    </SidebarInset>
  );
}

function MainPaneHeader() {
  const appSidebarOpen = useAppSidebarOpen();

  return (
    <header className="relative flex h-10 shrink-0 items-center bg-background [-webkit-app-region:drag]">
      <span
        className="flex h-8 translate-x-0 items-center gap-2 pr-3 pl-3 text-sm font-medium transition-transform duration-150 ease-linear motion-reduce:transition-none data-[app-sidebar-open=false]:translate-x-[calc(var(--desktop-header-controls-width)-0.75rem)]"
        data-app-sidebar-open={appSidebarOpen}
      >
        <HugeiconsIcon className="size-[var(--icon-sm)]" icon={Layers01Icon} strokeWidth={2} />
        <span>Chat</span>
      </span>
    </header>
  );
}

function AppTitlebarControls() {
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
      <span className="group-data-[state=expanded]/sidebar-wrapper:hidden">
        <DropdownMenu>
          <DropdownMenuTrigger
            render={<Button aria-label="New tab" size="icon-sm" type="button" variant="ghost" />}
          >
            <HugeiconsIcon icon={PencilEdit02Icon} strokeWidth={2} />
          </DropdownMenuTrigger>
          <DropdownMenuContent align="start">
            {mainPaneTabDefinitions.map((mainPaneTabDefinition) => (
              <DropdownMenuItem
                key={mainPaneTabDefinition.kind}
                onClick={mainPaneTabDefinition.create}
              >
                <HugeiconsIcon icon={mainPaneTabDefinition.icon} strokeWidth={2} />
                {mainPaneTabDefinition.label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      </span>
    </div>
  );
}

function SidePaneTitlebarControls() {
  const { isSidePaneMaximized, isSidePaneOpen, toggleSidePane, toggleSidePaneMaximized } =
    useSidePaneLayout();
  const { sidePaneTabs } = useMockSidePaneTabs();
  const sidePaneToggle = (
    <Button
      aria-label={sidePaneTabs.length === 0 ? "New side pane tab" : "Toggle side pane"}
      aria-pressed={sidePaneTabs.length === 0 ? undefined : isSidePaneOpen}
      className="aria-pressed:bg-accent aria-pressed:text-accent-foreground"
      id="side-pane-toggle"
      onClick={sidePaneTabs.length === 0 ? undefined : toggleSidePane}
      size="icon-sm"
      title={sidePaneTabs.length === 0 ? "New side pane tab" : "Toggle side pane"}
      type="button"
      variant="ghost"
    />
  );

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
      {sidePaneTabs.length === 0 ? (
        <SidePaneTabCreationMenu trigger={sidePaneToggle}>
          <HugeiconsIcon icon={LayoutRightIcon} strokeWidth={2} />
        </SidePaneTabCreationMenu>
      ) : (
        <Button
          aria-label="Toggle side pane"
          aria-pressed={isSidePaneOpen}
          className="aria-pressed:bg-accent aria-pressed:text-accent-foreground"
          id="side-pane-toggle"
          onClick={toggleSidePane}
          size="icon-sm"
          title="Toggle side pane"
          type="button"
          variant="ghost"
        >
          <HugeiconsIcon icon={LayoutRightIcon} strokeWidth={2} />
        </Button>
      )}
    </div>
  );
}
