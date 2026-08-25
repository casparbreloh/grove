import { Cancel01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type TabStripItem = Readonly<{
  icon?: IconSvgElement;
  id: string;
  title: string;
}>;

type TabStripProps = Readonly<{
  activeTabId: string | undefined;
  children?: ReactNode;
  label: string;
  onClose: (tabId: string) => void;
  onSelect: (tabId: string) => void;
  tabs: readonly TabStripItem[];
}>;

export function TabStrip({ activeTabId, children, label, onClose, onSelect, tabs }: TabStripProps) {
  return (
    <nav
      aria-label={label}
      className="flex h-full min-w-0 items-center gap-1 overflow-x-auto [-webkit-app-region:no-drag]"
      role="tablist"
    >
      {tabs.map((tab) => (
        <TabStripButton
          isActive={tab.id === activeTabId}
          key={tab.id}
          onClose={onClose}
          onSelect={onSelect}
          tab={tab}
        />
      ))}
      {children}
    </nav>
  );
}

function TabStripButton({
  isActive,
  onClose,
  onSelect,
  tab,
}: Readonly<{
  isActive: boolean;
  onClose: (tabId: string) => void;
  onSelect: (tabId: string) => void;
  tab: TabStripItem;
}>) {
  return (
    <div className="group/tab relative flex h-7 w-36 shrink-0 items-center">
      <Button
        aria-controls={`${tab.id}-panel`}
        aria-selected={isActive}
        className={cn(
          "h-full w-full justify-start pr-7 group-hover/tab:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)] focus-visible:ring-inset focus-visible:ring-offset-0",
          isActive && "bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]",
        )}
        id={`${tab.id}-tab`}
        onClick={() => onSelect(tab.id)}
        role="tab"
        type="button"
        variant="secondary"
      >
        {tab.icon && <HugeiconsIcon icon={tab.icon} strokeWidth={2} />}
        <span className="truncate">{tab.title}</span>
      </Button>
      <Button
        aria-label={`Close ${tab.title}`}
        className="pointer-events-none absolute top-1/2 right-0 -translate-y-1/2 text-muted-foreground opacity-0 group-hover/tab:pointer-events-auto group-hover/tab:opacity-100 hover:bg-transparent hover:text-foreground focus-visible:pointer-events-auto focus-visible:opacity-100 dark:hover:bg-transparent"
        onClick={(event) => {
          event.stopPropagation();
          onClose(tab.id);
        }}
        size="icon-sm"
        type="button"
        variant="ghost"
      >
        <HugeiconsIcon icon={Cancel01Icon} strokeWidth={2} />
      </Button>
    </div>
  );
}
