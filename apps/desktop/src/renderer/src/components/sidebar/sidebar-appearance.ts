const sidebarAppearanceOptions = [
  {
    value: "opaque-soft",
    label: "Solid · Soft",
    swatch: "oklch(0.18 0 0)",
  },
  {
    value: "native-standard",
    label: "Native · Standard",
    swatch: "oklch(0.4 0 0)",
  },
  {
    value: "glass-neutral",
    label: "Glass · Neutral",
    swatch: "oklch(0.22 0 0)",
  },
  {
    value: "glass-plum",
    label: "Workspace · Plum",
    swatch: "oklch(0.5 0.11 310)",
  },
  {
    value: "glass-ocean",
    label: "Workspace · Ocean",
    swatch: "oklch(0.52 0.1 235)",
  },
  {
    value: "glass-moss",
    label: "Workspace · Moss",
    swatch: "oklch(0.52 0.09 150)",
  },
] as const;

type SidebarAppearance = (typeof sidebarAppearanceOptions)[number]["value"];

const defaultSidebarAppearance: SidebarAppearance = "glass-neutral";

export { defaultSidebarAppearance, sidebarAppearanceOptions };
export type { SidebarAppearance };
