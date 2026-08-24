# macOS vibrant-sidebar divider and hover tokens

Research date: 2026-08-24. Scope: Electron 43.4.0 and Grove's configured shadcn `base-mira` registry.

## Conclusion

The dark vertical line at Grove's desktop sidebar edge is the renderer's normal shadcn sidebar border, not a separator drawn by macOS vibrancy. The official sidebar puts a plain `border-r` on the normal left-sidebar container. In Grove, that utility receives its color from the global `--border` token; changing `--sidebar-border` therefore cannot change this line.

Electron's native material is still relevant to the perceived result because translucent CSS colors composite over it. A translucent black border that looks subtle over Grove's nearly white opaque main surface becomes visibly darker over a dimmer vibrant sidebar. The smallest source-aligned mitigation is to keep the official `border-r` and use shadcn's opaque light `--border` value. An opaque border is backdrop-independent, while the official dark theme can retain a translucent white border.

## Evidence

### Electron creates one full-window visual-effect view

For `vibrancy: "sidebar"`, Electron 43.4.0 maps the option to `NSVisualEffectMaterialSidebar`. It creates an `NSVisualEffectView` with the **entire content view's bounds**, uses behind-window blending, and inserts that view beneath all other window content. It does not position the native view at the renderer sidebar width or configure an internal vertical separator. [Electron 43.4.0 `NativeWindowMac::SetVibrancy`](https://github.com/electron/electron/blob/v43.4.0/shell/browser/native_window_mac.mm#L1397-L1513)

Electron's public API likewise describes vibrancy as an effect added to the browser window, and removing it requires `setVibrancy(null)`. There is no supported option for a sidebar-edge color or divider. [Electron `BrowserWindow.setVibrancy`](https://www.electronjs.org/docs/latest/api/browser-window#winsetvibrancytype-options-macos)

Apple describes `NSVisualEffectView` as a view that provides a material appearance and recommends choosing material by semantic purpose because its appearance can change with system state. The material is not a CSS surface token. [Apple `NSVisualEffectView`](https://developer.apple.com/documentation/appkit/nsvisualeffectview)

Therefore, a line exactly at Grove's DOM sidebar boundary is not an edge of Electron's `NSVisualEffectView`; the native view continues beneath the main content. This is an inference from Electron's view frame and hierarchy.

### The visible outer divider uses global `--border`

Running the official registry command from `apps/desktop` on the research date:

```sh
npx shadcn@latest view sidebar button
```

showed that the current `base-mira` normal sidebar container uses:

```text
group-data-[side=left]:border-r group-data-[side=right]:border-l
```

It does **not** add `border-sidebar-border` to that outer container. Grove's copied component has the same classes. shadcn's theming contract maps `border-border` to the global `border` token and defines `sidebar-border` separately for sidebar-specific borders and separators. [shadcn theming token reference](https://ui.shadcn.com/docs/theming#theme-tokens) The current sidebar does use `sidebar-border` explicitly for its rail hover, internal separator, submenu rule, and floating-sidebar ring. [Official shadcn Sidebar documentation](https://ui.shadcn.com/docs/components/base/sidebar)

Grove also applies `border-border` as the default border color to every element. Consequently, the outer `border-r` resolves through `--border`. This explains the earlier observation that setting `--sidebar-border` transparent left the dark line unchanged: that experiment changed a different token.

### The source-aligned light border is opaque

shadcn's current neutral theme uses:

```css
:root {
  --border: oklch(0.922 0 0);
}
.dark {
  --border: oklch(1 0 0 / 10%);
}
```

[Official shadcn default neutral theme](https://ui.shadcn.com/docs/theming#default-theme-css)

Grove's previous light value, `oklch(0 0 0 / 8%)`, was not equivalent on a translucent sidebar: it blended with whatever luminance the native material produced. The replacement, opaque `oklch(0.922 0 0)`, remains the same light gray over both the opaque composer and the vibrant sidebar. This is the minimal, token-only light-mode correction while preserving the official component structure. If the product later needs a deliberately sidebar-specific outer divider, the supported shadcn path is a small `border-sidebar-border` class override plus a `--sidebar-border` value; it is not an Electron vibrancy option.

## Hover behavior

The current official `base-mira` sources do not use one hover token everywhere:

- ghost `Button` uses `hover:bg-muted` and additionally halves that background in dark mode;
- `SidebarMenuButton` and `SidebarMenuAction` use `hover:bg-sidebar-accent`.

The official token meanings are `accent` for interactive hover/active surfaces and `sidebar-accent` for the corresponding sidebar states. [shadcn theming token reference](https://ui.shadcn.com/docs/theming#theme-tokens)

Grove's requested exact visual equality is therefore a deliberate, small deviation from the current registry. The simplest consistent arrangement is:

1. ordinary ghost controls use `accent`;
2. `--sidebar-accent` aliases `--accent`;
3. a compound task row paints the accent **once** on the whole row;
4. the nested Archive control does not paint a second background while the row owns the group hover.

The fourth point matters when the accent contains alpha. Painting the same translucent color on both parent and child composites two source layers and makes the child region darker or lighter than the row. The CSS compositing formula specifies that each additional source layer contributes again to the final color. [W3C Compositing and Blending, simple alpha compositing](https://www.w3.org/TR/compositing-1/#simplealphacompositing)

This grouping belongs in Grove's app-level `TaskItem`, not in the upstream shadcn primitives: restore the task button's group-hover accent and suppress only the nested Archive button's background on hover. That keeps Archive visible and interactive without producing a doubled hover layer.

## Minimal implementation direction

1. Restore the group-hover background on the task's `SidebarMenuButton`.
2. Keep the nested Archive ghost button background transparent while the task row is group-hovered.
3. Keep one semantic hover relationship: `--sidebar-accent: var(--accent)` and ordinary ghost buttons using `accent`.
4. Change only the light global border to shadcn's opaque `oklch(0.922 0 0)`; leave the official outer `border-r` intact.
5. Re-test by temporarily removing `border-r` or making global `--border` transparent. Changing `--sidebar-border` is not a valid diagnostic for this outer divider.

No main-process or native-window change is needed for these two symptoms. Theme synchronization between renderer CSS and AppKit vibrancy is a separate concern documented in [Coordinating a macOS vibrant sidebar with renderer themes](./macos-vibrant-sidebar-theme-coordination.md).
