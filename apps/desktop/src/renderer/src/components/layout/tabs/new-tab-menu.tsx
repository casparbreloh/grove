import { Add01Icon, ChatIcon, TerminalIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { createMockChatTab, createMockTerminalTab } from "@/lib/mocks/grove";

const newTabOptions = [
  { label: "Chat", icon: ChatIcon, create: createMockChatTab },
  { label: "Terminal", icon: TerminalIcon, create: createMockTerminalTab },
] as const;

export function NewTabMenu({
  className,
  icon = Add01Icon,
}: {
  className?: string;
  icon?: IconSvgElement;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            aria-label="New tab"
            className={className}
            size="icon-sm"
            type="button"
            variant="ghost"
          />
        }
      >
        <HugeiconsIcon icon={icon} strokeWidth={2} />
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start">
        {newTabOptions.map((option) => (
          <DropdownMenuItem key={option.label} onClick={option.create}>
            <HugeiconsIcon icon={option.icon} strokeWidth={2} />
            {option.label}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
