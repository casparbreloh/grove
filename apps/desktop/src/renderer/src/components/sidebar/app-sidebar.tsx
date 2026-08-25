import {
  Compass01Icon,
  FilterMailIcon,
  Moon02Icon,
  PencilEdit02Icon,
  Sun03Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
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
import { filterMockTasksByProject, useMockGrove } from "@/lib/mock";
import { TaskItem } from "./task-item";

const allProjectsValue = "all-projects";

export function AppSidebar() {
  const { projects, tasks, taskProjectFilterId } = useMockGrove();
  const projectById = new Map(projects.map((project) => [project.id, project]));

  return (
    <Sidebar>
      <SidebarHeader className="h-10 shrink-0 p-0 [-webkit-app-region:drag]" />
      <SidebarContent>
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
