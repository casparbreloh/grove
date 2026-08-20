import {
  ArrowDown01Icon,
  Compass01Icon,
  FilterMailIcon,
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
import { filterMockTasksByProject, selectMockWorkspace, useMockGrove } from "@/lib/mock";
import { TaskItem } from "./task-item";

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
            render={<Button className="min-w-0 text-sm" size="lg" type="button" variant="ghost" />}
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
            <SidebarMenu>
              {tasks.map((task) => (
                <TaskItem key={task.id} project={projectById.get(task.projectId)} task={task} />
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
    </Sidebar>
  );
}
