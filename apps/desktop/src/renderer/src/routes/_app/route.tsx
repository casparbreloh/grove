import { Outlet, createFileRoute } from "@tanstack/react-router";
import { AppControls } from "@/components/desktop/app-controls";
import { AppHeader } from "@/components/desktop/app-header";
import { SidePaneControls } from "@/components/desktop/side-pane-controls";
import { AppSidebar } from "@/components/sidebar/app-sidebar";
import { SidePane } from "@/components/side-pane";
import { PaneSplit, PaneSplitProvider } from "@/components/ui/pane-split";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";

export const Route = createFileRoute("/_app")({ component: AppLayout });

function AppLayout() {
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
