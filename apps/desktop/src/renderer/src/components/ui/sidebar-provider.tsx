import * as React from "react";
import { SidebarContext, type SidebarContextValue } from "@/components/ui/sidebar-context";
import { cn } from "@/lib/utils";

const SIDEBAR_WIDTH = "14rem";
const SIDEBAR_KEYBOARD_SHORTCUT = "b";

function SidebarProvider({ className, style, children, ...props }: React.ComponentProps<"div">) {
  const [open, setOpen] = React.useState(true);
  const toggleSidebar = React.useCallback(() => setOpen((currentOpen) => !currentOpen), []);

  React.useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === SIDEBAR_KEYBOARD_SHORTCUT && (event.metaKey || event.ctrlKey)) {
        event.preventDefault();
        toggleSidebar();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [toggleSidebar]);

  const state: SidebarContextValue["state"] = open ? "expanded" : "collapsed";
  const contextValue = React.useMemo(
    () => ({ state, open, toggleSidebar }),
    [state, open, toggleSidebar],
  );

  return (
    <SidebarContext.Provider value={contextValue}>
      <div
        className={cn(
          "group/sidebar-wrapper relative isolate flex min-h-svh w-full has-data-[variant=inset]:bg-sidebar has-data-[variant=inset]:before:pointer-events-none has-data-[variant=inset]:before:absolute has-data-[variant=inset]:before:inset-0 has-data-[variant=inset]:before:-z-1",
          className,
        )}
        data-slot="sidebar-wrapper"
        style={{ "--sidebar-width": SIDEBAR_WIDTH, ...style } as React.CSSProperties}
        {...props}
      >
        {children}
      </div>
    </SidebarContext.Provider>
  );
}

export { SidebarProvider };
