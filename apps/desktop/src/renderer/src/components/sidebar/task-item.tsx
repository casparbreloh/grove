import { ArchiveIcon, Folder01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { SidebarMenuAction, SidebarMenuButton, SidebarMenuItem } from "@/components/ui/sidebar";
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
      <SidebarMenuAction
        aria-label={`Archive ${task.title}`}
        className="top-auto right-1 bottom-1 text-muted-foreground hover:text-foreground"
        showOnHover
        type="button"
      >
        <HugeiconsIcon icon={ArchiveIcon} strokeWidth={2} />
      </SidebarMenuAction>
    </SidebarMenuItem>
  );
}
