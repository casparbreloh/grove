import { Loading03Icon } from "@hugeicons/core-free-icons";
import { HugeiconsIcon, type HugeiconsIconProps } from "@hugeicons/react";
import { cn } from "@/lib/utils";

export function Spinner({
  className,
  ...props
}: Omit<HugeiconsIconProps, "icon">): React.ReactElement {
  return (
    <HugeiconsIcon
      aria-label="Loading"
      className={cn("animate-spin", className)}
      icon={Loading03Icon}
      role="status"
      strokeWidth={2}
      {...props}
    />
  );
}
