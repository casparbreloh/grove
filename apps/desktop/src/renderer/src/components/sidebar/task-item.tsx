import { Folder01Icon, Tick02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Button } from "@/components/ui/button";
import { SidebarMenuButton, SidebarMenuItem } from "@/components/ui/sidebar";
import type { Project, Task } from "@/lib/mock";

export function TaskItem({ project, task }: { project: Project | undefined; task: Task }) {
  return (
    <SidebarMenuItem className="group/task-item rounded-md transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground">
      <SidebarMenuButton className="h-6 rounded-b-none py-1 font-normal text-foreground hover:bg-transparent active:bg-transparent group-hover/task-item:text-sidebar-accent-foreground">
        <span className="truncate">{task.title}</span>
      </SidebarMenuButton>
      <div className="flex h-6 min-w-0 items-center px-2 text-xs text-muted-foreground">
        <span className="flex min-w-0 items-center gap-1">
          <HugeiconsIcon className="size-3!" icon={Folder01Icon} strokeWidth={2} />
          <span className="truncate">{project?.name}</span>
        </span>
        <Button
          aria-label={`Archive ${task.title}`}
          className="pointer-events-none -mr-2 ml-auto font-normal text-muted-foreground opacity-0 transition-opacity duration-75 group-hover/task-item:pointer-events-auto group-hover/task-item:opacity-100 focus-visible:pointer-events-auto focus-visible:opacity-100"
          size="xs"
          type="button"
          variant="ghost"
        >
          <HugeiconsIcon icon={Tick02Icon} strokeWidth={2} />
          <span>Archive</span>
        </Button>
      </div>
    </SidebarMenuItem>
  );
}
