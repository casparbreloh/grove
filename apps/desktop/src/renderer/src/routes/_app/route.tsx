import { ArrowLeft02Icon, ArrowRight02Icon, PencilEdit02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Outlet, createFileRoute } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { SidebarInset, SidebarProvider, SidebarTrigger, useSidebar } from "@/components/ui/sidebar";

import { AppSidebar } from "../../components/app-sidebar";

export const Route = createFileRoute("/_app")({ component: AppLayout });

function AppLayout() {
  return (
    <SidebarProvider className="h-svh flex-col overflow-hidden bg-sidebar">
      <DesktopHeader />
      <div className="flex min-h-0 flex-1">
        <AppSidebar />
        <SidebarInset className="min-h-0 min-w-0 overflow-hidden">
          <div className="flex min-h-0 flex-1 flex-col">
            <Outlet />
          </div>
        </SidebarInset>
      </div>
    </SidebarProvider>
  );
}

function DesktopHeader() {
  const { open } = useSidebar();

  return (
    <header
      className="desktop-header grid h-10 shrink-0 bg-sidebar transition-[grid-template-columns] duration-150 ease-linear"
      style={{
        gridTemplateColumns: `${open ? "var(--sidebar-width)" : "calc(var(--desktop-header-controls-width) + var(--desktop-header-new-tab-width) + var(--desktop-header-safe-area-left))"} minmax(0, 1fr)`,
      }}
    >
      <div className="desktop-header-sidebar-controls flex items-center gap-1 px-2">
        <SidebarTrigger />
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
        {!open && (
          <Button aria-label="New tab" size="icon" variant="ghost">
            <HugeiconsIcon icon={PencilEdit02Icon} strokeWidth={2} />
          </Button>
        )}
      </div>
      <nav aria-label="Tabs" className="flex min-w-0 items-center gap-1 overflow-hidden">
        <Button className="w-36 justify-start" variant="secondary">
          Home
        </Button>
      </nav>
    </header>
  );
}
