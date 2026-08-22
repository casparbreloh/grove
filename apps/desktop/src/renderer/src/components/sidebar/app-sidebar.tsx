import { Compass01Icon, FilterMailIcon, PencilEdit02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Button } from "@/components/ui/button";
import { Menu, MenuPopup, MenuRadioGroup, MenuRadioItem, MenuTrigger } from "@/components/ui/menu";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
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
    <Sidebar
      className="top-(--desktop-header-height) h-[calc(100svh-var(--desktop-header-height))]"
      collapsible="offcanvas"
      variant="inset"
    >
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
            <Menu>
              <MenuTrigger
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
              </MenuTrigger>
              <MenuPopup align="end" className="w-52">
                <MenuRadioGroup
                  onValueChange={(value) =>
                    filterMockTasksByProject(value === allProjectsValue ? undefined : value)
                  }
                  value={taskProjectFilterId ?? allProjectsValue}
                >
                  <MenuRadioItem value={allProjectsValue}>All projects</MenuRadioItem>
                  {projects.map((project) => (
                    <MenuRadioItem key={project.id} value={project.id}>
                      {project.name}
                    </MenuRadioItem>
                  ))}
                </MenuRadioGroup>
              </MenuPopup>
            </Menu>
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
