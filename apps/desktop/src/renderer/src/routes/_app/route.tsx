import {
  ArrowLeft02Icon,
  ArrowRight02Icon,
  Layers01Icon,
  PencilEdit02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Outlet, createFileRoute } from "@tanstack/react-router";
import { AppSidebar } from "@/components/sidebar/app-sidebar";
import { AppRightSidebar } from "@/components/sidebar/app-right-sidebar";
import { tabRegistry } from "@/components/tabs/registry";
import { Button } from "@/components/ui/button";
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
      <SidebarProvider
        className="h-svh min-h-0 min-w-0 overflow-hidden bg-sidebar [--sidebar:var(--background)]"
        defaultOpen={false}
        side="right"
      >
        <SidebarInset className="min-h-0 min-w-0 overflow-hidden">
          <DesktopHeader />
          <div className="flex min-h-0 flex-1 flex-col">
            <Outlet />
          </div>
        </SidebarInset>
        <AppRightSidebar />
      </SidebarProvider>
    </SidebarProvider>
  );
}

function DesktopControls({ sidebarOpen }: { sidebarOpen: boolean }) {
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
      {!sidebarOpen && <NewTabMenu />}
    </div>
  );
}

function DesktopHeader() {
  const { isMobile, open, openMobile } = useSidebar();
  const sidebarOpen = isMobile ? openMobile : open;

  return (
    <header className="relative flex h-10 shrink-0 items-center bg-background [-webkit-app-region:drag]">
      <DesktopControls sidebarOpen={sidebarOpen} />
      <span
        className="flex h-7 items-center gap-1.5 pr-3 text-xs/3.5 font-medium transition-[padding] duration-150 ease-linear data-[sidebar-open=false]:pl-[var(--desktop-header-controls-width)] data-[sidebar-open=true]:pl-3"
        data-sidebar-open={sidebarOpen}
      >
        <HugeiconsIcon className="size-3.5" icon={Layers01Icon} strokeWidth={2} />
        <span>Chat</span>
      </span>
      <SidebarTrigger
        className="fixed top-1.5 right-2 z-20 [-webkit-app-region:no-drag]"
        side="right"
        size="icon"
      />
    </header>
  );
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
