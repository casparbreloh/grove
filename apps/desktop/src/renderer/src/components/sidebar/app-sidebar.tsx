import { Compass01Icon, FilterMailIcon, PencilEdit02Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Button } from "@/components/ui/button";
import { Menu, MenuPopup, MenuRadioGroup, MenuRadioItem, MenuTrigger } from "@/components/ui/menu";
import { SidebarPanel, SidebarScrollArea } from "@/components/sidebar/sidebar-shell";
import { filterMockTasksByProject, useMockGrove } from "@/lib/mock";
import { TaskItem } from "./task-item";

const allProjectsValue = "all-projects";
const sidebarNavigationButtonClassName =
  "h-7 w-full justify-start gap-1.5 rounded-md px-2 text-xs font-normal text-foreground hover:bg-sidebar-hover hover:text-foreground";

export function AppSidebar({ open }: { open: boolean }) {
  const { projects, tasks, taskProjectFilterId } = useMockGrove();
  const projectById = new Map(projects.map((project) => [project.id, project]));

  return (
    <SidebarPanel
      className="top-(--desktop-header-height) h-[calc(100svh-var(--desktop-header-height))]"
      open={open}
    >
      <SidebarScrollArea>
        <nav aria-label="Primary" className="px-2 py-1">
          <ul className="flex min-w-0 flex-col gap-px">
            <li>
              <Button className={sidebarNavigationButtonClassName} variant="ghost">
                <HugeiconsIcon icon={PencilEdit02Icon} strokeWidth={2} />
                <span>New Task</span>
              </Button>
            </li>
            <li>
              <Button className={sidebarNavigationButtonClassName} variant="ghost">
                <HugeiconsIcon icon={Compass01Icon} strokeWidth={2} />
                <span>Explore</span>
              </Button>
            </li>
          </ul>
        </nav>

        <section
          aria-labelledby="tasks-heading"
          className="relative flex min-w-0 flex-col px-2 py-1"
        >
          <div className="flex items-center">
            <h2
              className="flex h-7 min-w-0 flex-1 items-center px-2 font-medium text-[11px] text-muted-foreground"
              id="tasks-heading"
            >
              Tasks
            </h2>
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
              <MenuPopup align="start" className="w-52" side="bottom">
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
          <ul className="flex min-w-0 flex-col gap-px text-xs">
            {tasks.map((task) => (
              <TaskItem key={task.id} project={projectById.get(task.projectId)} task={task} />
            ))}
          </ul>
        </section>
      </SidebarScrollArea>
    </SidebarPanel>
  );
}
