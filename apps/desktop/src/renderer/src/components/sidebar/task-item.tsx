import { Folder01Icon, Tick02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Button } from "@/components/ui/button";
import type { Project, Task } from "@/lib/mock";

export function TaskItem({ project, task }: { project: Project | undefined; task: Task }) {
  return (
    <li className="group/task-item relative">
      <Button
        className="h-auto! w-full flex-col items-stretch gap-0.5 overflow-hidden rounded-md px-2 py-1.5 text-left text-xs font-normal group-hover/task-item:bg-sidebar-accent group-hover/task-item:text-sidebar-accent-foreground"
        variant="ghost"
      >
        <span className="truncate">{task.title}</span>
        <div className="flex min-w-0 items-center gap-1 pr-16 text-[11px]! text-muted-foreground">
          <HugeiconsIcon className="size-3!" icon={Folder01Icon} strokeWidth={2} />
          <span className="truncate">{project?.name}</span>
        </div>
      </Button>
      <Button
        aria-label={`Archive ${task.title}`}
        className="absolute right-1.5 bottom-1 h-5 gap-1 px-1 text-[11px]! text-muted-foreground hover:bg-transparent hover:text-foreground dark:hover:bg-transparent"
        size="xs"
        variant="ghost"
      >
        <HugeiconsIcon className="size-3!" icon={Tick02Icon} strokeWidth={2} />
        <span>Archive</span>
      </Button>
    </li>
  );
}
