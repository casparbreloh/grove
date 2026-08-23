# Workspace sidebar colors

Workspace colors are quiet identity cues, not full themes. They tint the opaque
shadcn popover surface first, then the result receives the same 70% glass coverage
as the neutral sidebar. Text, controls, borders, and hover states continue to use
the shared semantic tokens.

Ocean is Grove's temporary default. The remaining colors are candidates for
future workspace identities; they do not need to appear in the temporary material
switcher until Workspaces own their appearance.

| Color | Light tint             | Dark tint              | Mix       |
| ----- | ---------------------- | ---------------------- | --------- |
| Ocean | `oklch(0.72 0.08 235)` | `oklch(0.50 0.08 235)` | 16% / 18% |
| Iris  | `oklch(0.72 0.08 275)` | `oklch(0.49 0.08 275)` | 16% / 18% |
| Plum  | `oklch(0.72 0.09 310)` | `oklch(0.48 0.09 310)` | 16% / 18% |
| Rose  | `oklch(0.72 0.08 20)`  | `oklch(0.50 0.08 20)`  | 16% / 18% |
| Amber | `oklch(0.76 0.08 85)`  | `oklch(0.54 0.08 85)`  | 16% / 18% |
| Moss  | `oklch(0.72 0.07 150)` | `oklch(0.50 0.07 150)` | 16% / 18% |
| Teal  | `oklch(0.72 0.07 190)` | `oklch(0.50 0.07 190)` | 16% / 18% |

The light and dark anchors deliberately keep chroma between 0.07 and 0.09. That
range remains recognizable through the translucent surface without competing with
content. Keep the final glass coverage at `--sidebar-glass-coverage`; changing a
workspace color should only require changing its tint anchor.
