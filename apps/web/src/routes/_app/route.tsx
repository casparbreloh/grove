import { LayoutLeftIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Outlet, createFileRoute } from "@tanstack/react-router";
import { Button } from "@grove/ui/components/button";
import { SidebarProvider, useSidebar } from "@grove/ui/components/sidebar";

import { AppSidebar } from "../../components/app-sidebar";

export const Route = createFileRoute("/_app")({ component: AppLayout });

function AppLayout() {
  return (
    <SidebarProvider>
      <AppSidebar />
      <main className="flex min-h-svh min-w-0 flex-1 flex-col">
        <header className="flex h-14 shrink-0 items-center px-4">
          <SidebarToggle />
        </header>
        <div className="flex min-h-0 flex-1 flex-col">
          <Outlet />
        </div>
      </main>
    </SidebarProvider>
  );
}

function SidebarToggle() {
  const { toggleSidebar } = useSidebar();

  return (
    <Button aria-label="Toggle sidebar" onClick={toggleSidebar} size="icon-sm" variant="ghost">
      <HugeiconsIcon icon={LayoutLeftIcon} strokeWidth={2} />
    </Button>
  );
}
