import {
  Add01Icon,
  ArrowLeft02Icon,
  ArrowRight02Icon,
  Cancel01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Outlet, createFileRoute } from "@tanstack/react-router";
import { AppSidebar } from "@/components/sidebar/app-sidebar";
import { tabRegistry } from "@/components/tabs/registry";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SidebarInset, SidebarProvider, SidebarTrigger, useSidebar } from "@/components/ui/sidebar";
import type { GroveTab } from "@/lib/mock";
import { closeMockTab, selectMockTab, useMockGrove } from "@/lib/mock";
import { cn } from "@/lib/utils";

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
  const { activeTabId, tabs } = useMockGrove();

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
        {!open && <NewTabMenu />}
      </div>
      <nav
        aria-label="Tabs"
        className="flex min-w-0 items-center gap-1 overflow-x-auto"
        role="tablist"
      >
        {tabs.map((tab) => (
          <TabItem
            canClose={tabs.length > 1}
            isActive={tab.tabId === activeTabId}
            key={tab.tabId}
            tab={tab}
          />
        ))}
        <NewTabMenu />
      </nav>
    </header>
  );
}

function NewTabMenu() {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button aria-label="New tab" size="icon" type="button" variant="ghost">
            <HugeiconsIcon icon={Add01Icon} strokeWidth={2} />
          </Button>
        }
      />
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

function TabItem({
  tab,
  isActive,
  canClose,
}: {
  tab: GroveTab;
  isActive: boolean;
  canClose: boolean;
}) {
  return (
    <div className="group/tab relative w-36 shrink-0">
      <Button
        aria-selected={isActive}
        className={cn(
          "w-full justify-start truncate group-hover/tab:pr-7",
          isActive
            ? "bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)] hover:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)] group-hover/tab:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]"
            : "group-hover/tab:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]",
        )}
        onClick={() => selectMockTab(tab.tabId)}
        role="tab"
        variant="secondary"
      >
        {tab.title}
      </Button>
      {canClose && (
        <Button
          aria-label={`Close ${tab.title}`}
          className="pointer-events-none absolute top-0 right-0 text-muted-foreground opacity-0 group-hover/tab:pointer-events-auto group-hover/tab:opacity-100 hover:bg-transparent hover:text-foreground focus-visible:pointer-events-auto focus-visible:opacity-100 dark:hover:bg-transparent"
          onClick={() => closeMockTab(tab.tabId)}
          size="icon"
          type="button"
          variant="ghost"
        >
          <HugeiconsIcon icon={Cancel01Icon} strokeWidth={2} />
        </Button>
      )}
    </div>
  );
}
