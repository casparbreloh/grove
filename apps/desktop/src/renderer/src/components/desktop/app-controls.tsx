import { ArrowLeft02Icon, ArrowRight02Icon, PencilEdit02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { tabRegistry } from "@/components/tabs/registry";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { useAppSidebarOpen } from "@/hooks/use-app-sidebar-open";

export function AppControls() {
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
