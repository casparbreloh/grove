import {
  ArrowDown01Icon,
  FolderAddIcon,
  PencilEdit02Icon,
  Search01Icon,
} from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Button } from "@/components/ui/button";
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

export function AppSidebar() {
  return (
    <Sidebar className="top-10 h-[calc(100svh-2.5rem)]" variant="inset">
      <SidebarHeader className="h-8 flex-row items-center justify-between gap-0 p-0 pr-2">
        <Button className="px-2 text-sm" size="lg" type="button" variant="ghost">
          Grove
          <HugeiconsIcon icon={ArrowDown01Icon} strokeWidth={2} />
        </Button>
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
                  <span>New thread</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
        <SidebarGroup className="pl-0">
          <div className="flex items-center justify-between">
            <SidebarGroupLabel>Workspaces</SidebarGroupLabel>
            <Button
              aria-label="Add workspace"
              className="text-sidebar-foreground/70"
              size="icon"
              type="button"
              variant="ghost"
            >
              <HugeiconsIcon icon={FolderAddIcon} strokeWidth={2} />
            </Button>
          </div>
        </SidebarGroup>
      </SidebarContent>
    </Sidebar>
  );
}
