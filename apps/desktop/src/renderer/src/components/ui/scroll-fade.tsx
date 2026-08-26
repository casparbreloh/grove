import { cn } from "@/lib/utils";

const edgeStyles = {
  top: "inset-x-0 top-0 h-6 bg-linear-to-b",
  right: "inset-y-0 right-0 w-6 bg-linear-to-l",
  bottom: "inset-x-0 bottom-0 h-6 bg-linear-to-t",
  left: "inset-y-0 left-0 w-6 bg-linear-to-r",
} as const;

const surfaceStyles = {
  background: "from-background to-background/0",
  sidebar: "from-sidebar to-sidebar/0",
} as const;

export function ScrollFade({
  className,
  edge,
  isVisible,
  surface = "background",
}: Readonly<{
  className?: string;
  edge: keyof typeof edgeStyles;
  isVisible: boolean;
  surface?: keyof typeof surfaceStyles;
}>) {
  return (
    <div
      aria-hidden="true"
      className={cn(
        "pointer-events-none absolute z-10 opacity-0 transition-opacity duration-100",
        edgeStyles[edge],
        surfaceStyles[surface],
        isVisible && "opacity-100",
        className,
      )}
    />
  );
}
