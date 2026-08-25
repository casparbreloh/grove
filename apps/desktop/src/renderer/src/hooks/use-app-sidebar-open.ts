import { useSidebar } from "@/components/ui/sidebar";

export function useAppSidebarOpen() {
  const { isMobile, open, openMobile } = useSidebar();
  return isMobile ? openMobile : open;
}
