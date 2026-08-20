import {
  ArchiveIcon,
  ArrowDown01Icon,
  Compass01Icon,
  FilterMailIcon,
  Folder01Icon,
  PencilEdit02Icon,
  Search01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
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
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import type { Project, Task } from "@/lib/grove";
import { filterMockTasksByProject, selectMockWorkspace, useMockGrove } from "@/lib/mock-grove";

const allProjectsValue = "all-projects";

export function AppSidebar() {
  const { activeWorkspaceId, workspaces, projects, tasks, taskProjectFilterId } = useMockGrove();
  const activeWorkspace = workspaces.find(({ id }) => id === activeWorkspaceId);
  const projectById = new Map(projects.map((project) => [project.id, project]));

  return (
    <Sidebar className="top-10 h-[calc(100svh-2.5rem)]" variant="inset">
      <SidebarHeader className="h-8 flex-row items-center justify-between gap-0 p-0 pr-2">
        <DropdownMenu>
          <DropdownMenuTrigger
            render={
              <Button className="min-w-0 px-2 text-sm" size="lg" type="button" variant="ghost" />
            }
          >
            <span className="truncate">{activeWorkspace?.name ?? "Grove"}</span>
            <HugeiconsIcon data-icon="inline-end" icon={ArrowDown01Icon} strokeWidth={2} />
          </DropdownMenuTrigger>
          <DropdownMenuContent className="w-52">
            <DropdownMenuRadioGroup
              onValueChange={(value) => selectMockWorkspace(value)}
              value={activeWorkspaceId}
            >
              {workspaces.map((workspace) => (
                <DropdownMenuRadioItem key={workspace.id} value={workspace.id}>
                  {workspace.name}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
          </DropdownMenuContent>
        </DropdownMenu>
        <Button aria-label="Search" size="icon" type="button" variant="ghost">
          <HugeiconsIcon icon={Search01Icon} strokeWidth={2} />
        </Button>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup className="pl-0">
          <SidebarGroupContent>
            <SidebarMenu>
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

        <SidebarGroup className="pl-0">
          <div className="flex items-center">
            <SidebarGroupLabel className="min-w-0 flex-1">Tasks</SidebarGroupLabel>
            <DropdownMenu>
              <DropdownMenuTrigger
                render={
                  <Button
                    aria-label="Filter tasks by project"
                    size="icon"
                    type="button"
                    variant="ghost"
                  />
                }
              >
                <HugeiconsIcon icon={FilterMailIcon} strokeWidth={2} />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-52">
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
            <TaskMenu projectById={projectById} tasks={tasks} />
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
    </Sidebar>
  );
}

function TaskMenu({
  projectById,
  tasks,
}: {
  projectById: ReadonlyMap<string, Project>;
  tasks: readonly Task[];
}) {
  return (
    <SidebarMenu>
      {tasks.map((task) => (
        <TaskItem key={task.id} project={projectById.get(task.projectId)} task={task} />
      ))}
    </SidebarMenu>
  );
}

function TaskItem({ project, task }: { project: Project | undefined; task: Task }) {
  return (
    <SidebarMenuItem>
      <SidebarMenuButton className="h-auto flex-col items-stretch gap-1 py-2 group-hover/menu-item:bg-sidebar-accent group-hover/menu-item:text-sidebar-accent-foreground">
        <span className="truncate">{task.title}</span>
        <div className="flex min-w-0 items-center gap-1 text-[0.6875rem] text-muted-foreground">
          <HugeiconsIcon className="size-3!" icon={Folder01Icon} strokeWidth={2} />
          <span className="truncate">{project?.name}</span>
        </div>
      </SidebarMenuButton>
      <Button
        aria-label={`Archive ${task.title}`}
        className="pointer-events-none absolute right-0 bottom-0.5 text-muted-foreground opacity-0 group-hover/menu-item:pointer-events-auto group-hover/menu-item:opacity-100 hover:text-foreground focus-visible:pointer-events-auto focus-visible:opacity-100"
        size="icon"
        type="button"
        variant="ghost"
      >
        <HugeiconsIcon className="size-3.5" icon={ArchiveIcon} strokeWidth={2} />
      </Button>
    </SidebarMenuItem>
  );
}
