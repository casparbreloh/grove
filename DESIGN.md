# renderer design

## principles

Keep Grove calm, dense, and native to the desktop. The renderer projects Grove state; it does not own persistence, agents, environments, or filesystem behavior. Prefer a small set of semantic tokens and shadcn/Base UI composition over bespoke components or one-off values.

## tokens

- Surfaces: `background` is the app canvas, `card` is a raised opaque surface, `popover` is an overlay, and `sidebar` is a translucent `background`-derived surface. Use semantic foreground, muted, accent, border, ring, destructive, info, success, and warning tokens; do not introduce literal light/dark colors in components.
- Type: `text-ui-micro` is 12/16 metadata, `text-ui-small` is 13/19 controls and compact UI, `text-ui` is 15/24 reading and composition, `text-section-title` is 24/32, and `text-page-title` is 56/61. `text-ui-code` is 13/19 in `font-mono` for code and terminal content. Apply roles at primitive defaults before local overrides.
- Icons: `--icon-micro` (12px), `--icon-small` (14px), `--icon-ui` (16px), and `--icon-section` (20px). A control icon normally follows its text role; use a larger icon only when it improves optical balance.
- Spacing and radius follow Tailwind’s scale. Keep standard controls compact; use the existing radius tokens rather than literal radii. Motion is quick (75–200ms), only clarifies a state change, and always has a `motion-reduce` path.

## composition

Start with current shadcn primitives, their documented subcomponents, variants, `data-slot`, and state selectors. Grove intentionally keeps compact desktop Button/Input sizing, translucent blurred dropdowns, a stronger mobile-sheet scrim, and a sidebar without cookie persistence or a global keyboard shortcut. Those deviations serve the desktop shell; keep them when updating upstream components.

Custom CSS is justified only for platform layout variables or behavior a primitive cannot express clearly. Use shadcn's `scroll-fade` utility for adaptive fade affordances; it is surface-independent and requires no listeners or effects. Never emulate it with black or white surface gradients. Preserve message `content-visibility`, native panel resizing, and CSS-driven pane animation rather than moving high-frequency work into React state.

Tabs use semantic tab roles, roving focus, arrow/Home/End navigation, and a visible close control with reserved title space. Keep the 150px preferred / 100px minimum side-pane tab contract and the always-reachable add button.

## accessibility and performance

Keep focus-visible rings, native semantic controls, keyboard behavior, inert hidden panes, and reduced-motion behavior intact. Do not hide focusable closing content. Use `data-*` selectors and CSS transitions before effects; reserve effects for measured DOM work such as title-overflow reveal. Do not add a dependency when Tailwind, Base UI, or an installed primitive is clearer; `react-resizable-panels` remains because it owns accessible splitter interaction.

## shadcn update checklist

From `apps/desktop`, inspect `npx shadcn@latest diff --help`, then use `npx shadcn@latest add <component> --diff -y` for every affected primitive. Compare each difference before accepting it: classify it as upstream improvement, intentional Grove deviation, or integration behavior. Never overwrite a Grove deviation blindly. Keep assistant-ui elements in `src/renderer/src/components/ai-elements` and review any downloaded shadcn transitive primitives in `components/ui`. Apply the semantic tokens above, preserve the intentional deviations listed here, run the narrow type/lint check, and visually check light/dark, focus, overflow, and reduced-motion states.
