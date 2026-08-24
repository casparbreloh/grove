# Pane split layout architecture

> Superseded for Grove's final animated overlay implementation by [Side-pane overlay behavior](./side-pane-overlay-behavior.md). This document is retained as the intermediate normal-flow sizing research that led to the dedicated `PaneSplit` seam.

Research date: 2026-08-24. Scope: a fixed, collapsible `16rem` app sidebar followed by a main-pane/side-pane split that starts at 50/50, lets the user resize the side pane, and keeps both panes at or above `20rem` without a React effect or `ResizeObserver` feedback loop.

## Recommendation

Make the main-pane/side-pane area a real two-pane layout in normal flow. Let CSS derive the **effective** side-pane width from one stored **user intent** value:

```css
.split {
  --pane-min: 20rem;
  --side-pane-intent: 50%;

  display: flex;
  min-inline-size: calc(var(--pane-min) + var(--pane-min));
}

.main-pane {
  flex: 1 1 0;
  min-inline-size: var(--pane-min);
}

.side-pane-slot {
  flex: 0 0 clamp(var(--pane-min), var(--side-pane-intent), calc(100% - var(--pane-min)));
  min-inline-size: 0;
  overflow: hidden;
}

.split[data-side-pane-open="false"] .side-pane-slot {
  flex-basis: 0;
}
```

The side pane should be the `.side-pane-slot` (or fill it), not a fixed-position pane paired with a separate phantom gap. A percentage width on the existing fixed container is relative to a different containing block than a percentage width on its in-flow gap, so a percentage-based split cannot reliably keep those two boxes identical. CSS defines percentages for `width` and related sizing properties relative to the element's containing block. [CSS Box Sizing Level 3, sizing properties and percentage sizing](https://www.w3.org/TR/css-sizing-3/#sizing-properties)

The app sidebar can retain shadcn's off-canvas fixed-container-plus-gap structure because it has one fixed `16rem` size and is not resizable. The main-pane/side-pane region should instead use a small dedicated split-layout primitive. This removes pane measurement and constraint policy from the general sidebar provider.

`clamp(MIN, VALUE, MAX)` is normatively equivalent to `max(MIN, min(VALUE, MAX))`, so the expression above encodes both pane minima directly: the side pane cannot be less than `20rem`, and it cannot exceed the split width minus the main pane's `20rem`. Grove expresses this through Tailwind's arbitrary-value syntax: `basis-[clamp(...)]`. [CSS Values and Units Level 4, comparison functions](https://www.w3.org/TR/css-values-4/#comp-func)

Flex is sufficient; CSS Grid, container-relative units, and container queries are unnecessary for the normal two-pane case. Explicit `min-inline-size` is important because a flex item's automatic main-axis minimum can otherwise be content-based. [CSS Flexible Box Layout Level 1, automatic minimum size](https://www.w3.org/TR/css-flexbox-1/#min-size-auto)

## Why this fixes the delayed separator

The current implementation has two layout authorities:

1. CSS animates the app-sidebar gap from `16rem` to `0` (or back).
2. A `ResizeObserver` measures the resulting split width, updates React state, and React publishes a corrected side-pane width.

That second step is necessarily downstream of layout. The Resize Observer processing model first recalculates styles and updates layout, delivers observations, then can recalculate styles and update layout again in a notification loop. [Resize Observer specification, HTML event-loop integration](https://www.w3.org/TR/resize-observer/#html-event-loop)

The observer is not inherently slow. The problem is using an observation of layout to set state that controls that same layout. It adds a second authority, a renderer rerender, and potentially another layout pass. During an animation, that correction can visibly trail the already-running left-sidebar transition.

With the CSS-owned constraint, the side pane's preferred expression does not change when the app sidebar opens or closes. Only its containing block's used width changes. CSS sizing percentages resolve against that containing block, and the browser resolves the `clamp()` as part of the same layout that resolves the app-sidebar gap. [CSS Box Sizing Level 3, percentage sizing](https://www.w3.org/TR/css-sizing-3/#percentage-sizing) There is no observer notification, state correction, or second animation to catch up.

Keep the open/close transition on the right slot's `flex-basis`, but disable it while the rail is actively dragged. The left transition changes the available containing-block size; it does not change the authored `flex-basis` expression. CSS transitions are started by computed-value changes, while percentages used for box sizing resolve against the containing block. [CSS Transitions Level 1, style change events](https://www.w3.org/TR/css-transitions-1/#starting), [CSS Box Sizing Level 3, preferred size properties](https://www.w3.org/TR/css-sizing-3/#preferred-size-properties)

## Exact sizing model

For an open side pane, define:

- `A`: current inline size of the main-pane/side-pane split;
- `M`: the shared minimum, `20rem` converted by CSS;
- `I`: the user's side-pane intent, initially `50%`, then the last user-selected length;
- `R`: the effective side-pane width;
- `L`: the effective main-pane width.

The layout is:

```text
R = clamp(M, I, A - M)
L = A - R
```

For every `A >= 2M`, this guarantees `R >= M` and `L >= M`.

Important consequences:

- Initially `I = 50%`, so the panes are equal whenever the two `20rem` minima are satisfiable.
- If the user has put the main pane at `20rem` and opens the `16rem` app sidebar, `A` decreases by `16rem`; CSS reduces `R` by the same amount in the same layout frames, while `L` remains `20rem`.
- If a fixed user intent is already inside the new range, the side pane stays fixed and the main pane absorbs the available-space change.
- If a user resize puts the main pane at its `20rem` minimum, store the side-pane intent as `100%` rather than the current pixel width. The outer `clamp()` still enforces both minima, while the semantic maximum keeps the main pane at `20rem` when the app sidebar opens or closes.
- If a constraint temporarily clamps the intent, do **not** overwrite intent in response to the container resize. When space returns, restoring the user's chosen width is predictable; unlike the current implementation, the restoration follows the same CSS-driven outer animation rather than an observer-driven correction behind it.
- Commit a clamped effective value only at the end of a user resize. External layout changes must never commit a new intent.

## Rail interaction without resize effects

React only needs state for durable user intent and open/closed state. It does not need state for the continuously changing effective width.

On `pointerdown`:

1. Read the right slot's current width and the rail's starting `clientX` once.
2. Read the split width once for interaction clamping and accessible values.
3. Capture the pointer with `setPointerCapture(pointerId)`.

On `pointermove`, for the side pane:

```text
rawIntentPx = startRightWidthPx + startClientX - currentClientX
effectivePx = clamp(minPx, rawIntentPx, splitWidthPx - minPx)
```

Write `rawIntentPx` or `effectivePx` directly to `--side-pane-intent` on the split element. Writing the clamped value makes the DOM intent match what the user can see; writing the raw value provides a rubber-band return when the pointer passes a limit. Either is valid, but on `pointerup` commit the effective side-pane width—or the semantic `100%` maximum—to React/persistence. Do not set React state on every move.

Pointer capture is the platform mechanism intended to keep subsequent events targeted at the drag element after the pointer leaves its hit area. The Pointer Events specification also requires `touch-action` to declare direct-manipulation behavior; canceling pointer events alone does not suppress viewport panning. [Pointer Events, pointer capture](https://www.w3.org/TR/pointerevents/#pointer-capture), [Pointer Events, `touch-action`](https://www.w3.org/TR/pointerevents/#the-touch-action-css-property)

This event-driven path needs no `useEffect`, `useLayoutEffect`, window resize listener, or `ResizeObserver`. It performs one layout read at drag start, cheap arithmetic plus one style-property write per drag event, and one layout read/state commit at drag end. Avoid alternating a layout read and style write on every `pointermove`; that pattern can force synchronous layout. The width must actually participate in layout because pane contents need to reflow, so a transform-only drag is not an appropriate optimization.

For keyboard resizing, read the current slot and split widths in the `keydown` handler, apply the same clamp, then commit once. `Home` selects `M`; `End` selects `A - M`; arrow keys apply the configured step. No passive synchronization is needed.

## Accessibility without passive measurement

Keep the rail focusable with `role="separator"`, `aria-orientation="vertical"`, `aria-controls`, and an accessible name. The WAI-ARIA window-splitter pattern requires current, minimum, and maximum values and defines arrow-key behavior; `Home` and `End` are optional. [WAI-ARIA APG Window Splitter Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter/)

Use normalized percentage values (`0` to `100`) for ARIA rather than dynamic rem maxima. Refresh `aria-valuenow` from current DOM sizes on focus, pointer start/move, and keyboard interaction. Those are the moments assistive technology needs the interactive value, and they avoid a passive observer solely to mirror CSS into attributes. During a pointer drag, the start-of-interaction split size is sufficient unless Grove intentionally permits another layout toggle during the same drag.

## The real narrow-width boundary

Two `20rem` panes require at least `40rem` of split space. With the app sidebar open, the complete desktop layout requires at least `56rem` before window chrome/borders:

```text
app sidebar open:    16rem + 20rem + 20rem = 56rem
app sidebar closed:           20rem + 20rem = 40rem
```

No JavaScript algorithm, CSS Grid definition, or breakpoint can make two non-overlapping `20rem` panes fit in less than `40rem`. Below that size Grove must choose an explicit product policy:

- permit horizontal overflow;
- make the side pane an overlay/off-canvas pane;
- collapse one pane;
- or set an Electron window minimum that makes the hard constraints satisfiable.

If the side pane should become an overlay based on the _remaining split space_, a CSS container query is the clean breakpoint mechanism. Put `container-type: inline-size` on an outer split shell and style a descendant split below `40rem`; container size queries evaluate against that container's content box. [CSS Containment Level 3, container queries and inline-size feature](https://www.w3.org/TR/css-contain-3/#container-queries)

That responsive fallback is separate from the animation fix. Above `40rem`, `clamp()` already provides the complete constraint system without a breakpoint.

## Comparison with the official shadcn resizable stack

shadcn's official `Resizable` component is a wrapper around `react-resizable-panels` and is explicitly presented as an accessible, keyboard-capable horizontal composition of a group, panels, and a handle. [shadcn/ui Resizable documentation](https://ui.shadcn.com/docs/components/base/resizable)

`react-resizable-panels` v4 supports `rem` minima, a `50%` default, release-time `onLayoutChanged`, and `groupResizeBehavior="preserve-pixel-size"` for retaining a pane's pixel size as the group changes. It also supplies separator ARIA behavior. [Official `react-resizable-panels` API](https://github.com/bvaughn/react-resizable-panels/blob/main/README.md#panel)

It is a sound choice if Grove wants a general panel system. For this layout it is not the smallest choice:

- panels and separators must be direct DOM children of the group, so the side pane still has to be separate from shadcn Sidebar's fixed container plus gap;
- its group owns panel layout behavior, which duplicates functionality that this two-pane constraint expresses in one CSS declaration;
- Grove currently needs one resizable boundary, not a general nested-panel or persistence system.

If the library is chosen anyway, use a dedicated main-pane/side-pane group, set both `minSize="20rem"`, start both at `50%`, and persist only from `onLayoutChanged`. Do not wrap the side pane in a fixed/gap Sidebar.

## Suggested Grove seam

Keep the concerns separate:

- `SidebarProvider`: app-sidebar open state, trigger, mobile sheet behavior, and the fixed `16rem` app-sidebar contract;
- `PaneSplit`: main-pane/side-pane normal-flow layout, the `20rem` minimum token, user intent, rail input, and splitter accessibility;
- `SidePane`: the side pane's tab content, rendered inside the split slot rather than owning layout measurement.

The key invariant is: **CSS owns effective width; React owns only user intent.** No effect should copy measured layout back into sizing state.

## Verification cases

1. Right opens at an exact 50/50 split with left open and closed.
2. Main at `20rem` → open left: right shrinks synchronously and the separator has no trailing motion.
3. Main at `20rem` → close left: right expands synchronously and the separator has no trailing motion.
4. Right at `20rem` → toggle left: right stays `20rem` and main absorbs the change.
5. Resize the Electron window through the same cases; no width state changes should occur.
6. Drag and keyboard resize clamp both steady-state panes at `20rem`.
7. Releasing after a constrained drag commits the visible width, not an unreachable hidden width.
8. Right close/reopen restores the last user intent.
9. At less than `40rem` of split space, the chosen overflow/overlay/collapse policy activates explicitly.
10. React DevTools shows no component rerenders on passive container resizing and no provider-tree rerender per pointer move.
