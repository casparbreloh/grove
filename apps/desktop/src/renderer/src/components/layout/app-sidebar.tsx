import {
  Compass01Icon,
  FilterMailIcon,
  Folder01Icon,
  Moon02Icon,
  PencilEdit02Icon,
  Sun03Icon,
  Tick02Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { useEffect, useRef } from "react";
import { useTheme } from "@/components/theme-provider";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import { filterMockTasksByProject, useMockGrove, type Project, type Task } from "@/lib/mock";

const allProjectsValue = "all-projects";
const taskTitleScrollDelayMs = 500;
const taskTitleScrollPixelsPerSecond = 30;

export function AppSidebar() {
  const { projects, tasks, taskProjectFilterId } = useMockGrove();
  const projectById = new Map(projects.map((project) => [project.id, project]));

  return (
    <Sidebar>
      <SidebarHeader className="shrink-0 gap-0 p-0">
        <div className="h-10 shrink-0 [-webkit-app-region:drag]" />
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu aria-label="Primary">
              <SidebarMenuItem>
                <SidebarMenuButton>
                  <HugeiconsIcon icon={PencilEdit02Icon} strokeWidth={2} />
                  <span>New Task</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
              <SidebarMenuItem>
                <SidebarMenuButton>
                  <HugeiconsIcon icon={Compass01Icon} strokeWidth={2} />
                  <span>Explore</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarHeader>
      <SidebarContent className="scroll-fade scroll-fade-6">
        <SidebarGroup aria-labelledby="tasks-heading">
          <div className="flex items-center">
            <SidebarGroupLabel className="min-w-0 flex-1" id="tasks-heading">
              Tasks
            </SidebarGroupLabel>
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <Button
                    aria-label="Filter tasks by project"
                    size="icon-sm"
                    type="button"
                    variant="ghost"
                  />
                }
              >
                <HugeiconsIcon icon={FilterMailIcon} strokeWidth={2} />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" className="w-52" side="bottom">
                <DropdownMenuRadioGroup
                  onValueChange={(value) =>
                    filterMockTasksByProject(value === allProjectsValue ? undefined : value)
                  }
                  value={taskProjectFilterId ?? allProjectsValue}
                >
                  <DropdownMenuRadioItem value={allProjectsValue}>
                    All projects
                  </DropdownMenuRadioItem>
                  {projects.map((project) => (
                    <DropdownMenuRadioItem key={project.id} value={project.id}>
                      {project.name}
                    </DropdownMenuRadioItem>
                  ))}
                </DropdownMenuRadioGroup>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          <SidebarGroupContent>
            <SidebarMenu>
              {tasks.map((task) => (
                <TaskItem key={task.id} project={projectById.get(task.projectId)} task={task} />
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <ThemeSwitcher />
      </SidebarFooter>
    </Sidebar>
  );
}

function TaskItem({ project, task }: { project: Project | undefined; task: Task }) {
  const titleViewportRef = useRef<HTMLSpanElement>(null);
  const titleTextRef = useRef<HTMLSpanElement>(null);
  const titleAnimationRef = useRef<Animation>(null);

  useEffect(() => () => titleAnimationRef.current?.cancel(), []);

  function revealTitle() {
    const titleViewport = titleViewportRef.current;
    const titleText = titleTextRef.current;
    if (!titleViewport || !titleText) return;

    const overflowWidth = titleText.scrollWidth - titleViewport.clientWidth;
    if (overflowWidth <= 0) return;

    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    titleAnimationRef.current = titleText.animate(
      { transform: ["translateX(0)", `translateX(-${overflowWidth}px)`] },
      {
        delay: taskTitleScrollDelayMs,
        duration: (overflowWidth / taskTitleScrollPixelsPerSecond) * 1000,
        easing: "linear",
        fill: "forwards",
      },
    );
  }

  function resetTitle() {
    titleAnimationRef.current?.cancel();
    titleTextRef.current?.style.removeProperty("transform");
  }

  return (
    <SidebarMenuItem
      className="group/task-item"
      onMouseEnter={revealTitle}
      onMouseLeave={resetTitle}
    >
      <SidebarMenuButton className="h-auto flex-col items-stretch gap-0 py-1.5 group-hover/task-item:bg-sidebar-accent group-hover/task-item:text-sidebar-accent-foreground">
        <span
          className="mask-r-from-[calc(100%-1rem)] overflow-hidden leading-snug whitespace-nowrap group-hover/task-item:mask-none"
          ref={titleViewportRef}
        >
          <span className="inline-block min-w-max" ref={titleTextRef}>
            {task.title}
          </span>
        </span>
        <span className="flex min-w-0 items-center gap-1 pr-16 text-xs text-muted-foreground">
          <HugeiconsIcon className="size-[var(--text-xs)]" icon={Folder01Icon} strokeWidth={2} />
          <span className="min-w-0 flex-1 truncate">{project?.name}</span>
        </span>
      </SidebarMenuButton>
      <Button
        aria-label={`Archive ${task.title}`}
        className="pointer-events-none absolute right-0 bottom-0.5 font-normal text-muted-foreground opacity-0 transition-opacity duration-75 group-hover/task-item:pointer-events-auto group-hover/task-item:opacity-100 hover:bg-transparent hover:text-foreground focus-visible:pointer-events-auto focus-visible:opacity-100"
        size="xs"
        type="button"
        variant="ghost"
      >
        <HugeiconsIcon icon={Tick02Icon} strokeWidth={2} />
        <span>Archive</span>
      </Button>
    </SidebarMenuItem>
  );
}

function ThemeSwitcher() {
  const { resolvedTheme, setTheme } = useTheme();

  return (
    <div aria-label="Color theme" className="grid grid-cols-2 gap-1" role="group">
      <Button
        aria-pressed={resolvedTheme === "light"}
        className="aria-pressed:bg-accent"
        onClick={() => setTheme("light")}
        size="sm"
        type="button"
        variant="ghost"
      >
        <HugeiconsIcon icon={Sun03Icon} strokeWidth={2} />
        Light
      </Button>
      <Button
        aria-pressed={resolvedTheme === "dark"}
        className="aria-pressed:bg-accent"
        onClick={() => setTheme("dark")}
        size="sm"
        type="button"
        variant="ghost"
      >
        <HugeiconsIcon icon={Moon02Icon} strokeWidth={2} />
        Dark
      </Button>
    </div>
  );
}
