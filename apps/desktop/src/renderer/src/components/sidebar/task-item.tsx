import { Folder01Icon, Tick02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { SidebarMenuAction, SidebarMenuButton, SidebarMenuItem } from "@/components/ui/sidebar";
import type { Project, Task } from "@/lib/mock";

export function TaskItem({ project, task }: { project: Project | undefined; task: Task }) {
  return (
    <SidebarMenuItem>
      <SidebarMenuButton className="h-auto flex-col items-stretch gap-0.5 py-1.5 group-hover/menu-item:bg-sidebar-accent group-hover/menu-item:text-sidebar-accent-foreground">
        <span className="truncate">{task.title}</span>
        <div className="flex min-w-0 items-center gap-1 pr-16 text-[11px]! text-muted-foreground">
          <HugeiconsIcon className="size-3!" icon={Folder01Icon} strokeWidth={2} />
          <span className="truncate">{project?.name}</span>
        </div>
      </SidebarMenuButton>
      <SidebarMenuAction
        aria-label={`Archive ${task.title}`}
        className="top-auto! right-2 bottom-1.5! aspect-auto h-4 w-auto gap-1 rounded-none p-0 text-[11px] text-muted-foreground hover:bg-transparent hover:text-foreground"
        showOnHover
        type="button"
      >
        <HugeiconsIcon className="size-3!" icon={Tick02Icon} strokeWidth={2} />
        <span>Archive</span>
      </SidebarMenuAction>
    </SidebarMenuItem>
  );
}
