# renderer design

Keep Grove calm, dense, and desktop-native. Prefer semantic tokens and current shadcn/Base UI composition over bespoke components, wrappers, or one-off values.

## defaults

- Surfaces: `background` is canvas, `card` raised, `popover` overlay, and `sidebar` a translucent `background`-derived surface. Use semantic foreground, muted, accent, border, ring, and status tokens; never literal light/dark component colors.
- Type: `text-sm` is 13px for controls, reading, code, terminals, and tab titles. Native `text-xs` is 12px for metadata and compact controls. Use heading utilities only for hierarchy.
- Icons are text role + 2px: `--icon-sm` is 15px beside `text-sm`; `--icon-xs` is 14px beside `text-xs`. Primitives apply this to unqualified SVGs without changing hit areas. The task-item project folder is the sole +1px optical exception.
- Use Tailwind spacing and existing radius tokens. Control height and padding come from explicit size variants, not typography. Motion is 75–200ms, communicates state only, and has a `motion-reduce` path.

## composition

- Start with shadcn subcomponents, variants, `data-slot`, and state selectors. Keep Grove's compact Button/Input sizing, translucent dropdowns, stronger mobile-sheet scrim, and sidebar without cookie persistence or global shortcut.
- Add custom CSS only for platform layout or behavior primitives cannot express. Use shadcn `scroll-fade`, not hard-coded gradients or effects. Preserve `content-visibility` and CSS-driven motion.
- Tabs use native button focus order, reserved close-control space, a fixed 150px width, direct horizontal overflow, and an always-reachable add button. Tab and Shift+Tab move through each tab and its close control; Enter and Space activate the focused control.
- Preserve focus rings, semantics, inert hidden panes, keyboard behavior, and reduced motion. Let JavaScript change state; render visibility and motion with `data-*` selectors and CSS transitions. Use effects only for measured DOM behavior.

## shadcn updates

From `apps/desktop`, inspect `npx shadcn@latest diff --help`, then run `npx shadcn@latest add <component> --diff -y`. Review every difference as upstream improvement, intentional Grove deviation, or integration behavior; never overwrite a deviation blindly. Keep assistant-ui in `src/renderer/src/components/ai-elements`, adapt transitive primitives in `components/ui`, then check light/dark, focus, overflow, reduced motion, and the narrow relevant test.
