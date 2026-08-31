import { ArrowDown01Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type IconSvgElement } from "@hugeicons/react";
import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

type NativeSelectProps = Omit<ComponentProps<"select">, "size"> & {
  icon?: IconSvgElement;
  iconClassName?: string;
  size?: "sm" | "default";
  variant?: "default" | "leading-icon" | "icon" | "sidebar-icon";
};

function NativeSelect({
  className,
  icon,
  iconClassName,
  size = "default",
  variant = "default",
  ...props
}: NativeSelectProps) {
  const iconOnly = variant === "icon" || variant === "sidebar-icon";
  const leadingIcon = variant === "leading-icon";

  return (
    <div
      className={cn(
        "group/native-select relative w-fit rounded-md has-[select:disabled]:opacity-50",
        variant === "leading-icon" && "has-[select:enabled]:hover:bg-accent",
        variant === "icon" && "has-[select:enabled]:hover:bg-accent",
        variant === "sidebar-icon" && "has-[select:enabled]:hover:bg-sidebar-accent",
        className,
      )}
      data-size={size}
      data-slot="native-select-wrapper"
    >
      <select
        className={cn(
          "h-8 w-full min-w-0 appearance-none rounded-md border border-input bg-transparent py-1 pr-7 pl-2.5 text-sm outline-none select-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/30 disabled:pointer-events-none disabled:cursor-not-allowed data-[size=sm]:h-7 data-[size=sm]:py-0.5 dark:bg-input/30",
          iconOnly &&
            "size-7 border-transparent p-0 text-transparent focus-visible:text-transparent dark:bg-transparent",
          leadingIcon && "w-auto max-w-full [field-sizing:content] pl-7",
        )}
        data-size={size}
        data-slot="native-select"
        {...props}
      />
      {leadingIcon && icon && (
        <HugeiconsIcon
          aria-hidden="true"
          className={cn(
            "pointer-events-none absolute top-1/2 left-2 size-[var(--icon-sm)] -translate-y-1/2",
            iconClassName,
          )}
          data-slot="native-select-leading-icon"
          icon={icon}
          strokeWidth={2}
        />
      )}
      <HugeiconsIcon
        aria-hidden="true"
        className={cn(
          "pointer-events-none absolute top-1/2 -translate-y-1/2",
          iconOnly
            ? cn(
                "left-1/2 -translate-x-1/2",
                variant === "sidebar-icon" ? "size-[var(--icon-xs)]" : "size-[var(--icon-sm)]",
              )
            : "right-2 size-[var(--icon-xs)] text-muted-foreground",
          !leadingIcon && iconClassName,
        )}
        data-slot="native-select-icon"
        icon={iconOnly ? (icon ?? ArrowDown01Icon) : ArrowDown01Icon}
        strokeWidth={2}
      />
    </div>
  );
}

function NativeSelectOption({ className, ...props }: ComponentProps<"option">) {
  return (
    <option
      className={cn("bg-[Canvas] text-[CanvasText]", className)}
      data-slot="native-select-option"
      {...props}
    />
  );
}

function NativeSelectOptGroup({ className, ...props }: ComponentProps<"optgroup">) {
  return (
    <optgroup
      className={cn("bg-[Canvas] text-[CanvasText]", className)}
      data-slot="native-select-optgroup"
      {...props}
    />
  );
}

export { NativeSelect, NativeSelectOptGroup, NativeSelectOption };
