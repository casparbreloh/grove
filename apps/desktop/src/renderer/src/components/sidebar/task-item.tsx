import { Folder01Icon, Tick02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Button } from "@/components/ui/button";
import { SidebarMenuButton, SidebarMenuItem } from "@/components/ui/sidebar";
import type { Project, Task } from "@/lib/mock";

export function TaskItem({ project, task }: { project: Project | undefined; task: Task }) {
  return (
    <SidebarMenuItem className="group/task-item">
      <SidebarMenuButton className="h-auto! flex-col items-stretch gap-0.5 px-2 py-1.5 font-normal text-foreground group-hover/task-item:bg-sidebar-accent group-hover/task-item:text-sidebar-accent-foreground">
        <span className="truncate">{task.title}</span>
        <div className="flex min-w-0 items-center gap-1 pr-16 text-[11px]! text-muted-foreground">
          <HugeiconsIcon className="size-3!" icon={Folder01Icon} strokeWidth={2} />
          <span className="truncate">{project?.name}</span>
        </div>
      </SidebarMenuButton>
      <Button
        aria-label={`Archive ${task.title}`}
        className="pointer-events-none absolute right-1.5 bottom-1 h-5 gap-1 px-1 text-[11px]! text-muted-foreground opacity-0 transition-opacity duration-75 group-hover/task-item:pointer-events-auto group-hover/task-item:opacity-100 hover:bg-transparent hover:text-foreground focus-visible:pointer-events-auto focus-visible:opacity-100 dark:hover:bg-transparent"
        size="xs"
        variant="ghost"
      >
        <HugeiconsIcon className="size-3!" icon={Tick02Icon} strokeWidth={2} />
        <span>Archive</span>
      </Button>
    </SidebarMenuItem>
  );
}
