import { ColorsIcon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon } from "@hugeicons/react";
import {
  sidebarAppearanceOptions,
  type SidebarAppearance,
} from "@/components/sidebar/sidebar-appearance";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { sidebarControlClassName } from "@/components/ui/sidebar";

function SidebarAppearanceMenu({
  value,
  onValueChange,
}: {
  value: SidebarAppearance;
  onValueChange: (value: SidebarAppearance) => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger
        render={
          <Button
            aria-label="Choose sidebar appearance"
            className={sidebarControlClassName}
            size="icon"
            type="button"
            variant="ghost"
          >
            <HugeiconsIcon icon={ColorsIcon} strokeWidth={2} />
          </Button>
        }
      />
      <DropdownMenuContent align="end" className="w-48" side="bottom">
        <DropdownMenuRadioGroup
          onValueChange={(nextValue) => onValueChange(nextValue as SidebarAppearance)}
          value={value}
        >
          <DropdownMenuLabel>Sidebar material</DropdownMenuLabel>
          {sidebarAppearanceOptions.map((option) => (
            <DropdownMenuRadioItem key={option.value} value={option.value}>
              <span
                aria-hidden="true"
                className="size-2.5 rounded-full ring-1 ring-foreground/15"
                style={{ background: option.swatch }}
              />
              {option.label}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export { SidebarAppearanceMenu };
