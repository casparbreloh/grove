import * as React from "react";

type SidebarContextValue = {
  state: "expanded" | "collapsed";
  open: boolean;
  toggleSidebar: () => void;
};

const SidebarContext = React.createContext<SidebarContextValue | null>(null);

function useSidebar() {
  const context = React.useContext(SidebarContext);
  if (!context) throw new Error("useSidebar must be used within a SidebarProvider.");
  return context;
}

export { SidebarContext, useSidebar };
export type { SidebarContextValue };
