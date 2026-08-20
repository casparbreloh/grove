import { ArchiveIcon, Folder01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Button } from "@/components/ui/button";
import { SidebarMenuButton, SidebarMenuItem } from "@/components/ui/sidebar";
import type { Project, Task } from "@/lib/mock";

export function TaskItem({ project, task }: { project: Project | undefined; task: Task }) {
  return (
    <SidebarMenuItem>
      <SidebarMenuButton className="h-auto flex-col items-stretch gap-1 py-2 group-hover/menu-item:bg-sidebar-accent group-hover/menu-item:text-sidebar-accent-foreground">
        <span className="truncate">{task.title}</span>
        <div className="flex min-w-0 items-center gap-1 text-[11px]! text-muted-foreground">
          <HugeiconsIcon className="size-3!" icon={Folder01Icon} strokeWidth={2} />
          <span className="truncate">{project?.name}</span>
        </div>
      </SidebarMenuButton>
      <Button
        aria-label={`Archive ${task.title}`}
        className="pointer-events-none absolute right-0 bottom-0.5 text-muted-foreground opacity-0 group-hover/menu-item:pointer-events-auto group-hover/menu-item:opacity-100 hover:text-foreground focus-visible:pointer-events-auto focus-visible:opacity-100"
        data-sidebar="menu-action"
        size="icon"
        type="button"
        variant="ghost"
      >
        <HugeiconsIcon icon={ArchiveIcon} strokeWidth={2} />
      </Button>
    </SidebarMenuItem>
  );
}
