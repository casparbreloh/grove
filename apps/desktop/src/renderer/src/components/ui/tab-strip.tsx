import { Cancel01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";

type TabStripItem = Readonly<{
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
    <div className="flex items-center">
      <Button
        aria-controls={`${tab.id}-panel`}
        aria-selected={isActive}
        id={`${tab.id}-tab`}
        onClick={() => onSelect(tab.id)}
        role="tab"
        size="sm"
        type="button"
        variant="ghost"
      >
        {tab.title}
      </Button>
      <Button
        aria-label={`Close ${tab.title}`}
        onClick={() => onClose(tab.id)}
        size="icon-sm"
        type="button"
        variant="ghost"
      >
        <HugeiconsIcon icon={Cancel01Icon} strokeWidth={2} />
      </Button>
    </div>
  );
}
