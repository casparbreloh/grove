import { Layers01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useAppSidebarOpen } from "@/hooks/use-app-sidebar-open";

export function AppHeader() {
  const appSidebarOpen = useAppSidebarOpen();

  return (
    <header className="relative flex h-10 shrink-0 items-center bg-background [-webkit-app-region:drag]">
      <span
        className="flex h-8 translate-x-0 items-center gap-2 pr-3 pl-3 text-sm font-medium transition-transform duration-150 ease-linear data-[app-sidebar-open=false]:translate-x-[calc(var(--desktop-header-controls-width)-0.75rem)]"
        data-app-sidebar-open={appSidebarOpen}
      >
        <HugeiconsIcon className="size-4" icon={Layers01Icon} strokeWidth={2} />
        <span>Chat</span>
      </span>
    </header>
  );
}
