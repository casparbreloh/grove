import { Sidebar, SidebarHeader, SidebarRail } from "@/components/ui/sidebar";

export function AppRightSidebar() {
  return (
    <Sidebar aria-label="Right sidebar" side="right">
      <SidebarHeader className="h-10 shrink-0 p-0 [-webkit-app-region:drag]" />
      <SidebarRail side="right" />
    </Sidebar>
  );
}
