import { ArrowLeft01Icon, ArrowRight01Icon, LayoutLeftIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Outlet, createFileRoute } from "@tanstack/react-router";
import { Button } from "@grove/ui/components/button";
import { SidebarInset, SidebarProvider, useSidebar } from "@grove/ui/components/sidebar";

import { AppSidebar } from "../../components/app-sidebar";

export const Route = createFileRoute("/_app")({ component: AppLayout });

function AppLayout() {
  return (
    <SidebarProvider className="flex-col overflow-hidden bg-sidebar [&_[data-slot=sidebar-container]]:duration-150 [&_[data-slot=sidebar-gap]]:duration-150">
      <DesktopHeader />
      <div className="flex min-h-0 flex-1">
        <AppSidebar />
        <SidebarInset className="min-w-0 md:peer-data-[variant=inset]:mt-0">
          <div className="flex min-h-0 flex-1 flex-col">
            <Outlet />
          </div>
        </SidebarInset>
      </div>
    </SidebarProvider>
  );
}

function DesktopHeader() {
  const { open, toggleSidebar } = useSidebar();

  return (
    <header
      className="grid h-10 shrink-0 bg-sidebar transition-[grid-template-columns] duration-150 ease-linear"
      style={{ gridTemplateColumns: `${open ? "var(--sidebar-width)" : "6.5rem"} minmax(0, 1fr)` }}
    >
      <div className="flex items-center justify-between px-2">
        <Button aria-label="Toggle sidebar" onClick={toggleSidebar} size="icon" variant="ghost">
          <HugeiconsIcon icon={LayoutLeftIcon} strokeWidth={2} />
        </Button>
        <div className="flex items-center gap-1">
          <Button
            aria-label="Go back"
            onClick={() => window.history.back()}
            size="icon"
            variant="ghost"
          >
            <HugeiconsIcon icon={ArrowLeft01Icon} strokeWidth={2} />
          </Button>
          <Button
            aria-label="Go forward"
            onClick={() => window.history.forward()}
            size="icon"
            variant="ghost"
          >
            <HugeiconsIcon icon={ArrowRight01Icon} strokeWidth={2} />
          </Button>
        </div>
      </div>
      <nav aria-label="Tabs" className="flex min-w-0 items-center gap-1 overflow-hidden">
        <Button className="w-36 justify-start" variant="secondary">
          Home
        </Button>
      </nav>
    </header>
  );
}
