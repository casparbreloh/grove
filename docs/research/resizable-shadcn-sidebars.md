# Resizable shadcn sidebars

Research date: 2026-08-24. Scope: OSS source implementations that preserve shadcn's `SidebarProvider` / `SidebarRail` composition, plus shadcn's official general-purpose resizable primitive as the accessibility and dependency baseline.

## Conclusion

The closest reusable implementation is the MIT-licensed [`lumpinif/shadcn-resizable-sidebar`](https://github.com/lumpinif/shadcn-resizable-sidebar). It is a thin fork of shadcn's Sidebar that adds no panel-resizing dependency: the provider owns the current width, publishes it as `--sidebar-width`, and the existing rail handles both click-to-toggle and pointer drag using React Pointer Events. [Dependency source](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/package.json#L13-L35) That architecture fits Grove's existing two side-specific providers substantially better than replacing the whole three-column layout with `react-resizable-panels`.

Grove should adopt the thin-provider pattern, but not copy the implementation verbatim. The rail should retain click-to-collapse alongside drag resize: use a small movement threshold to distinguish the two interactions and never toggle after a completed drag. Make the min/default/max widths provider configuration instead of file-wide constants, keep one width state and one wrapper ref in each side's existing context, update the provider-local CSS variable directly during pointer movement, and commit React state on pointer release. Unlike the reference fork, the resized rail should be keyboard-focusable and implement the WAI-ARIA variable-separator contract.

Recommended configuration:

```ts
type SidebarWidthLimits =
  | Readonly<{ max: `${number}rem`; adjacentPaneMin?: never }>
  | Readonly<{ max?: never; adjacentPaneMin: `${number}rem` }>;

type SidebarWidthConfig = Readonly<{
  min: `${number}rem`;
  default: `${number}rem` | `${number}%`;
  keyboardStep?: `${number}rem`;
}> &
  SidebarWidthLimits;

const SIDEBAR_WIDTHS = {
  left: { min: "14rem", default: "16rem", max: "18rem" },
  right: { min: "20rem", default: "50%", adjacentPaneMin: "20rem" },
} satisfies Record<SidebarSide, SidebarWidthConfig>;
```

Pass the appropriate object to each `SidebarProvider`. This makes the requested thresholds obvious at the layout call site, while the primitive remains reusable.

## Exact-match OSS implementation

### Width and state ownership

The reference fork adds `width`, `setWidth`, drag state, a wrapper ref, and a width-cookie name to `SidebarContext`. `SidebarProvider` seeds and clamps width, then writes it to `--sidebar-width` on the provider wrapper. The existing sidebar gap and fixed container continue consuming the same variable, so content spacing and the visible sidebar stay synchronized without changing shadcn's component composition. [Provider and context source](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/components/ui/sidebar.tsx#L37-L108), [provider CSS-variable source](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/components/ui/sidebar.tsx#L122-L268)

This is directly compatible with Grove's nested providers. Each Grove provider already selects a distinct left or right context and renders a wrapper that locally defines `--sidebar-width`; CSS custom-property scoping keeps the right width from changing the left sidebar. Width state should therefore remain next to each provider's existing open state, not be lifted into one shared layout controller.

The fork hard-codes one `MIN_SIDEBAR_WIDTH` and `MAX_SIDEBAR_WIDTH` pair for every sidebar. That does not meet Grove's asymmetric ranges. Moving the complete width triplet into a provider prop is the smallest generalization; it also lets the provider clamp a restored or controlled value before publishing it. The fork's parsing treats `rem` as exactly 16 pixels and treats any other suffix as pixels, so Grove should either intentionally accept only `rem` or use computed root font size rather than silently accepting arbitrary CSS lengths. [Clamp implementation](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/components/ui/sidebar.tsx#L37-L90)

### Pointer interaction and rendering performance

The reference rail replaces shadcn's `onClick` with `onPointerDown`, passes `direction="right"` for a left sidebar and `direction="left"` for a right sidebar, and adds `touch-none` plus `select-none`. The hook uses pointer capture, tracks the pointer id, prevents body text selection, and treats movement beyond five pixels as a drag; pointer release without a drag preserves the original rail click-to-toggle behavior. [Rail source](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/components/ui/sidebar.tsx#L429-L488), [pointer-start source](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/hooks/use-sidebar-resize.ts#L280-L361), [pointer-finish source](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/hooks/use-sidebar-resize.ts#L507-L580)

Width direction is side-dependent: a left sidebar grows by positive horizontal delta; a right sidebar grows by negative horizontal delta. The fork supports both window-edge position calculation and a delta-based `isNested` mode. Grove's providers are nested in the React/CSS tree, but both rails resize edge-attached sidebars, so delta calculation is the robust choice for both and avoids coupling width to `window.innerWidth`. [Direction calculation](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/hooks/use-sidebar-resize.ts#L199-L245)

For smooth dragging, the hook writes `--sidebar-width` directly to the provider wrapper during pointer movement and commits React state only when the pointer is released. It also sets `data-dragging` so width transitions become zero-duration during the drag. This avoids a context rerender of the entire sidebar tree on every pointer event and prevents the normal open/close transition from lagging behind the pointer. [Live CSS-variable write](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/hooks/use-sidebar-resize.ts#L247-L269), [clamp and commit path](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/hooks/use-sidebar-resize.ts#L474-L529), [transition suppression](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/components/ui/sidebar.tsx#L345-L395)

The fork also auto-collapses when dragged beyond a threshold and permits dragging back to expand. This is optional product behavior, not necessary for the requested min/max resizing. Grove should initially clamp at the configured minimum and retain explicit click/button toggling; auto-collapse can be added later if it proves desirable. [Auto-collapse source](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/hooks/use-sidebar-resize.ts#L363-L471)

### Persistence

The reference uses separate `${cookieKey}:state` and `${cookieKey}:width` cookies, commits width on pointer release, clamps cookie-provided width before it enters state, and requires distinct keys for multiple sidebars. [Persistence contract](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/README.md#L89-L141), [server restore source](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/components/providers/index.tsx#L10-L39)

Grove is an Electron renderer and is currently frontend-first, so copying Next.js cookie/SSR plumbing would be needless. For this slice, in-memory provider state is enough. When layout restoration becomes part of Grove-owned durable layout, persist `leftSidebarWidth` and `rightSidebarWidth` through that renderer mock/layout boundary and clamp restored values against the current config. Do not let persistence enter the sidebar primitive as a cookie-specific concern.

## Accessibility gap in the exact-match fork

The reference fork keeps the rail as a `<button aria-label="Toggle Sidebar">` with `tabIndex={-1}`. Its drag path has no keyboard resizing and exposes no current/min/max value. [Rail semantics](https://github.com/lumpinif/shadcn-resizable-sidebar/blob/ba42d920e9282ff0ba5bb860c55a78cd04ce6dda/components/ui/sidebar.tsx#L464-L485) That mirrors shadcn's original click-only rail, which is also removed from the tab order, but it is insufficient once the rail becomes a variable splitter. [Official shadcn rail](https://github.com/shadcn-ui/ui/blob/b9938d94635fca7a4560449713b0b1ba87d77bc6/apps/v4/registry/bases/base/ui/sidebar.tsx#L287-L310)

The WAI-ARIA window-splitter pattern calls for a focusable `separator` with `aria-valuenow`, `aria-valuemin`, `aria-valuemax`, an accessible name, and `aria-controls`. Left/Right arrows move a vertical splitter, Enter collapses/restores, and optional Home/End select the minimum/maximum. [W3C Window Splitter Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter/)

For Grove, the rail should therefore:

- be focusable on desktop and use `role="separator"` with `aria-orientation="vertical"`;
- identify the sidebar it controls and expose its current configured value range;
- resize by a configurable step with Left/Right Arrow, clamp with Home/End, and preserve click/Enter toggle behavior;
- retain a visually thin divider but a wider hit area, pointer capture, `touch-action: none`, and `-webkit-app-region: no-drag` for Electron;
- suppress width transitions only while dragging, then restore the existing open/close animation.

## Official shadcn alternative: `Resizable`

shadcn's official `Resizable` component wraps `react-resizable-panels` and is explicitly described as keyboard-accessible. Its horizontal composition is `ResizablePanelGroup` → `ResizablePanel` / `ResizableHandle` / `ResizablePanel`. [Official shadcn Resizable docs](https://ui.shadcn.com/docs/components/base/resizable)

The underlying v4 library accepts explicit `rem` values for `defaultSize`, `minSize`, and `maxSize`; offers `preserve-pixel-size`; supplies pointer and keyboard interaction, WAI-ARIA separator attributes, hit-target sizing, collapse/expand APIs, and a release-time `onLayoutChanged` callback suited to persistence. [Official library panel API](https://github.com/bvaughn/react-resizable-panels/blob/f9c422714a66e14f671a17f340a3560d8032fcdc/README.md#panel), [group persistence and hit-target API](https://github.com/bvaughn/react-resizable-panels/blob/f9c422714a66e14f671a17f340a3560d8032fcdc/README.md#group), [separator API](https://github.com/bvaughn/react-resizable-panels/blob/f9c422714a66e14f671a17f340a3560d8032fcdc/README.md#separator)

This is the stronger general panel system, but not the smallest fit here. Its panels and separators must be direct DOM children of a group, while Grove's current sidebar owns a fixed container plus a matching flex gap inside side-specific nested providers. Adopting it would replace rather than extend that layout contract, require a new dependency, and couple left/main/right sizing in a group or nested groups. The custom rail layer is much smaller and preserves Grove's current shadcn API; the accessibility behaviors above are the important parts to borrow from the official primitive.

## Implementation recommendation for Grove

1. Add a `width` configuration prop to `SidebarProvider` with a minimum, default, optional keyboard step, and either a fixed maximum or the adjacent pane's minimum. Clamp controlled/restored values against the resulting live range.
2. Extend each side-specific context with current width, a commit function, the immutable config, drag state, and a ref to its own provider wrapper.
3. Keep provider-local CSS variables as the layout output: `--sidebar-width` for the rail-controlled pane and `--sidebar-adjacent-pane-min-width` for a split sibling. During pointer drag, write the width directly to the correct wrapper; on release, commit the clamped value to React state.
4. Upgrade `SidebarRail` into a focusable vertical separator with side-aware pointer delta, keyboard resizing, ARIA values, a five-pixel click-versus-drag threshold, and Electron `no-drag` hit testing.
5. Do not add auto-collapse or persistence in this slice. Preserve the existing triggers and rail click toggle. Persist both widths later through Grove's restorable-layout model, not cookies.
6. Test both sides independently: exact default values, fixed and split-derived clamps, opposite drag directions, keyboard steps/Home/End, click toggle after a non-drag, no toggle after a drag, transition suppression, provider isolation, and right-trigger hit testing while open.

This preserves the requested dimensions—left `14/16/18rem`, followed by a 50/50 main/right split whose panes both have a `20rem` minimum—as data, while keeping the resizing mechanism generic and aligned with the existing shadcn sidebar structure.
