import { Folder01Icon, Tick02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { SidebarMenuAction, SidebarMenuButton, SidebarMenuItem } from "@/components/ui/sidebar";
import type { Project, Task } from "@/lib/mock";

export function TaskItem({ project, task }: { project: Project | undefined; task: Task }) {
  return (
    <SidebarMenuItem className="group/task-item">
      <SidebarMenuButton className="h-auto! flex-col items-stretch gap-0 py-1 font-normal text-foreground group-hover/task-item:bg-sidebar-accent group-hover/task-item:text-sidebar-accent-foreground">
        <span className="truncate">{task.title}</span>
        <div className="flex h-6 min-w-0 items-center gap-1 pr-18 text-xs text-muted-foreground">
          <HugeiconsIcon className="size-3!" icon={Folder01Icon} strokeWidth={2} />
          <span className="truncate">{project?.name}</span>
        </div>
      </SidebarMenuButton>
      <SidebarMenuAction
        aria-label={`Archive ${task.title}`}
        className="pointer-events-none top-auto right-0 bottom-1 aspect-auto h-6 w-auto gap-1 rounded-sm px-2 text-xs font-normal text-muted-foreground opacity-0 transition-opacity duration-75 group-hover/task-item:pointer-events-auto group-hover/task-item:opacity-100 hover:bg-transparent hover:text-foreground focus-visible:pointer-events-auto focus-visible:opacity-100"
        showOnHover
      >
        <HugeiconsIcon className="size-3!" icon={Tick02Icon} strokeWidth={2} />
        <span>Archive</span>
      </SidebarMenuAction>
    </SidebarMenuItem>
  );
}
