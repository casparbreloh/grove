# renderer design

Keep Grove calm, dense, and desktop-native. Prefer semantic tokens and current shadcn/Base UI composition over bespoke components, wrappers, or one-off values.

## defaults

- Surfaces: `background` is canvas, `card` raised, `popover` overlay, and `sidebar` a translucent `background`-derived surface. Use semantic foreground, muted, accent, border, ring, and status tokens; never literal light/dark component colors.
- Type: `text-sm` is 13px for controls, reading, code, terminals, and tab titles. Native `text-xs` is 12px for metadata and compact controls. Use heading utilities only for hierarchy.
- Icons are text role + 2px: `--icon-sm` is 15px beside `text-sm`; `--icon-xs` is 14px beside `text-xs`. Primitives apply this to unqualified SVGs without changing hit areas. Oversized glyphs are adjusted at their use site; folder icons use text role + 1px.
- Use Tailwind spacing and existing radius tokens. Motion is 75–200ms, communicates state only, and has a `motion-reduce` path.

## composition

- Start with shadcn subcomponents, variants, `data-slot`, and state selectors. Keep Grove's compact Button/Input sizing, translucent dropdowns, stronger mobile-sheet scrim, and sidebar without cookie persistence or global shortcut.
- Add custom CSS only for platform layout or behavior primitives cannot express. Use shadcn `scroll-fade`, not hard-coded gradients or effects. Preserve `content-visibility`, native panel resizing, and CSS-driven pane animation.
- Tabs keep semantic roles, roving focus, arrow/Home/End navigation, reserved close-control space, a 150px preferred / 100px minimum width, and an always-reachable add button.
- Preserve focus rings, semantics, inert hidden panes, keyboard behavior, and reduced motion. Prefer `data-*` selectors and CSS transitions; use effects only for measured DOM behavior. Add dependencies only when installed primitives cannot cover the interaction; `react-resizable-panels` owns accessible splitting.

## shadcn updates

From `apps/desktop`, inspect `npx shadcn@latest diff --help`, then run `npx shadcn@latest add <component> --diff -y`. Review every difference as upstream improvement, intentional Grove deviation, or integration behavior; never overwrite a deviation blindly. Keep assistant-ui in `src/renderer/src/components/ai-elements`, adapt transitive primitives in `components/ui`, then check light/dark, focus, overflow, reduced motion, and the narrow relevant test.
