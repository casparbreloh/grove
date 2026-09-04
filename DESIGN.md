# Grove renderer design

Grove is a calm, dense, desktop-native workbench. This file records visual principles; renderer CSS and existing components define the actual tokens, dimensions, themes, and implementation.

## Principles

- Make the current work—source, conversation, terminals, approvals, and state—the visual focus. Keep surrounding chrome quiet.
- Prefer clear hierarchy, compact geometry, and stable spatial relationships. Dense should feel efficient, never cramped or ambiguous.
- Keep Project, Task, Session, and Environment context easy to scan, especially around consequential actions.
- Use tonal surfaces, borders, typography, and spacing before shadows or decoration. Reserve elevation for elements that genuinely sit above the document.
- Use a grayscale palette in light and dark appearances. Separate the shell and content with quiet tonal differences; do not add sidebar color themes.
- Use `rounded-md` consistently for inset panels and regular controls. The composer may stay more rounded. Account for the inset gutter when visually matching the native macOS corner; the OS owns the outer window shape. Use tone and readable text for hierarchy before adding outlines.
- Measure shell spacing between visible edges, accounting for both margin and padding. Use Tailwind spacing utilities directly: `1.5` for outer gutters, `2` within sidebar rows, and `1` between adjacent header controls. On desktop, the inset owns the gap to sidebar rows; the mobile drawer supplies its own right padding.
- Use a small amount of glass only on floating surfaces such as the composer: high-opacity fill, restrained blur, and a soft shadow. Keep document surfaces opaque and provide a solid fallback for reduced transparency or unsupported blur.
- Reuse established Grove, shadcn/Base UI, and assistant-ui patterns. Prefer platform behavior when it serves the interaction.
- Preserve semantics, keyboard access, visible focus, contrast, usable hit targets, and reduced-motion behavior.
- Use restrained motion only to communicate state, continuity, or direct manipulation. Meaning and access must not depend on animation.
- Simplify supporting controls as space narrows without weakening access to the active resource.

## Avoid

- Marketing-page composition, ornamental card grids, oversized typography, and empty space used only to imply polish.
- Broad gradients, glow, decorative glass panels, Electron vibrancy, and translucent window chrome.
- One-off visual values, duplicate primitives, or unrelated visual languages within one view.
- Visual novelty that weakens clarity, accessibility, or platform expectations.
