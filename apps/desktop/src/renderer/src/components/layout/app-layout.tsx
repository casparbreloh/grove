import { PencilEdit02Icon } from "@hugeicons/core-free-icons";
import { Outlet } from "@tanstack/react-router";

import { SidebarInset, SidebarProvider, SidebarTrigger } from "@/components/ui/sidebar";
import { AppSidebar } from "./app-sidebar";
import { NewTabMenu } from "./tabs/new-tab-menu";
import { TopPaneHeaders } from "./tabs/tab-bar";
import { TabDragDropProvider } from "./tabs/tab-drag-drop";

export function AppLayout() {
  return (
    <SidebarProvider className="relative isolate h-svh flex-col overflow-hidden before:pointer-events-none before:absolute before:inset-0 before:-z-1 before:backdrop-blur-2xl before:backdrop-saturate-150 [--header-height:2.5rem] [--titlebar-safe-area:min(4.5rem,env(titlebar-area-x,0px))]">
      <TabDragDropProvider>
        <DesktopHeader />
        <div className="flex min-h-0 flex-1">
          <AppSidebar />
          <SidebarInset className="min-h-0 min-w-0 overflow-hidden">
            <Outlet />
          </SidebarInset>
        </div>
      </TabDragDropProvider>
    </SidebarProvider>
  );
}

function DesktopHeader() {
  return (
    <header className="grid h-(--header-height) shrink-0 grid-cols-[calc(var(--sidebar-width)+0.5rem)_minmax(0,1fr)] transition-[grid-template-columns] duration-150 ease-linear group-data-[state=collapsed]/sidebar-wrapper:grid-cols-[calc(var(--titlebar-safe-area)+4.5rem)_minmax(0,1fr)] motion-reduce:transition-none [-webkit-app-region:drag] [&_button]:[-webkit-app-region:no-drag]">
      <div className="flex items-center gap-1 pr-1 pl-[calc(var(--titlebar-safe-area)+0.5rem)]">
        <SidebarTrigger size="icon-sm" />
        <NewTabMenu
          className="hidden group-data-[state=collapsed]/sidebar-wrapper:flex"
          icon={PencilEdit02Icon}
        />
      </div>
      <TopPaneHeaders />
    </header>
  );
}
