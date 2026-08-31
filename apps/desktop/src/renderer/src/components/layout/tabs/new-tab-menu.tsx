import { Add01Icon } from "@hugeicons/core-free-icons";
import type { IconSvgElement } from "@hugeicons/react";

import { NativeSelect, NativeSelectOption } from "@/components/ui/native-select";
import { createMockChatTab, createMockTerminalTab } from "@/lib/mocks/grove";
import { cn } from "@/lib/utils";

const newTabOptions = [
  { label: "Chat", create: createMockChatTab },
  { label: "Terminal", create: createMockTerminalTab },
] as const;

export function NewTabMenu({
  className,
  icon = Add01Icon,
}: {
  className?: string;
  icon?: IconSvgElement;
}) {
  return (
    <NativeSelect
      aria-label="New tab"
      className={cn("shrink-0 [-webkit-app-region:no-drag]", className)}
      icon={icon}
      onChange={(event) => {
        newTabOptions.find(({ label }) => label === event.currentTarget.value)?.create();
      }}
      size="sm"
      value=""
      variant="icon"
    >
      <NativeSelectOption disabled value="">
        New tab
      </NativeSelectOption>
      {newTabOptions.map((option) => (
        <NativeSelectOption key={option.label} value={option.label}>
          {option.label}
        </NativeSelectOption>
      ))}
    </NativeSelect>
  );
}
