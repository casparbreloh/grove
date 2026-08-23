# Workspace sidebar colors

Workspace colors are quiet identity cues, not full themes. These are resolved
sidebar surface values with a shared 70% alpha. Text, controls, borders, and hover
states continue to use the shared semantic tokens.

Ocean is Grove's temporary default. The remaining colors are candidates for
future workspace identities and remain documentation-only until Workspaces own
their appearance.

| Color | Light surface                  | Dark surface                   |
| ----- | ------------------------------ | ------------------------------ |
| Ocean | `oklch(0.955 0.013 235 / 70%)` | `oklch(0.262 0.014 235 / 70%)` |
| Iris  | `oklch(0.955 0.013 275 / 70%)` | `oklch(0.260 0.014 275 / 70%)` |
| Plum  | `oklch(0.955 0.014 310 / 70%)` | `oklch(0.259 0.016 310 / 70%)` |
| Rose  | `oklch(0.955 0.013 20 / 70%)`  | `oklch(0.262 0.014 20 / 70%)`  |
| Amber | `oklch(0.962 0.013 85 / 70%)`  | `oklch(0.269 0.014 85 / 70%)`  |
| Moss  | `oklch(0.955 0.011 150 / 70%)` | `oklch(0.262 0.013 150 / 70%)` |
| Teal  | `oklch(0.955 0.011 190 / 70%)` | `oklch(0.262 0.013 190 / 70%)` |

The low final chroma remains recognizable through the translucent surface without
competing with content. When Workspace appearance becomes real state, its theming
layer should resolve the one semantic `--sidebar` output rather than adding a CSS
token for every color.
