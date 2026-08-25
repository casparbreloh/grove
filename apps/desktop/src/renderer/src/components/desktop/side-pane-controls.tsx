import { CollapseIcon, ExpandIcon, LayoutRightIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import { Button } from "@/components/ui/button";
import { usePaneSplit } from "@/components/ui/pane-split";

export function SidePaneControls() {
  const { isSidePaneMaximized, isSidePaneOpen, toggleSidePane, toggleSidePaneMaximized } =
    usePaneSplit();

  return (
    <div className="fixed top-1.5 right-2 z-20 flex items-center gap-1 [-webkit-app-region:no-drag]">
      {isSidePaneOpen && (
        <Button
          aria-label={isSidePaneMaximized ? "Restore split view" : "Maximize side pane"}
          aria-pressed={isSidePaneMaximized}
          className="aria-pressed:bg-accent aria-pressed:text-accent-foreground"
          onClick={toggleSidePaneMaximized}
          size="icon-sm"
          title={isSidePaneMaximized ? "Restore split view" : "Maximize side pane"}
          variant="ghost"
        >
          <HugeiconsIcon icon={isSidePaneMaximized ? CollapseIcon : ExpandIcon} strokeWidth={2} />
        </Button>
      )}
      <Button
        aria-label="Toggle side pane"
        aria-pressed={isSidePaneOpen}
        className="aria-pressed:bg-accent aria-pressed:text-accent-foreground"
        onClick={toggleSidePane}
        size="icon-sm"
        title="Toggle side pane"
        variant="ghost"
      >
        <HugeiconsIcon icon={LayoutRightIcon} strokeWidth={2} />
      </Button>
    </div>
  );
}
