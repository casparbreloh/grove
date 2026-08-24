# Side-pane overlay behavior

Research date: 2026-08-24. Scope: a fixed app sidebar followed by a resizable main-pane/side-pane split, with an opaque side pane and a maximize action that covers the main pane without resizing, reflowing, or cropping it. The width model must keep both normal panes at or above `20rem` and must not use a resize effect or observer feedback loop.

## Recommendation

Keep the normal split and the maximized presentation as two independent layers:

1. The **split track** always owns normal main/side geometry. Its side slot retains its last normal `flex-basis` even while maximized.
2. The **side-pane surface** normally fills that slot. When maximized, only this surface becomes `position: absolute; inset: 0` against the split shell.
3. Do not change the main pane's basis, minimum, transform, width, or overflow when maximizing. Making it `inert`/`aria-hidden` while covered is an accessibility decision and does not require a layout change.

That gives the browser one simple invariant: maximizing never changes the split track. CSS absolute positioning explicitly takes a box out of flow so it has no impact on sibling size or position, and positions it against the nearest ancestor that establishes its containing block. [CSS Positioned Layout Level 3, positioning schemes and containing blocks](https://www.w3.org/TR/css-position-3/#position-property)

A suitable structure is:

```html
<div class="split-shell">
  <main class="main-pane">...</main>
  <div class="side-slot">
    <aside class="side-surface">...</aside>
  </div>
</div>
```

```css
.split-shell {
  --pane-min: 20rem;
  --side-intent: 50%;

  position: relative;
  isolation: isolate;
  display: flex;
  min-inline-size: calc(2 * var(--pane-min));
}

.main-pane {
  flex: 1 1 0;
  min-inline-size: var(--pane-min);
}

.side-slot {
  flex: 0 0 clamp(var(--pane-min), var(--side-intent), calc(100% - var(--pane-min)));
  min-inline-size: 0;
  overflow: hidden;
  background: var(--background);
}

.side-surface {
  position: relative;
  block-size: 100%;
  min-inline-size: var(--pane-min);
  background: var(--background);
}

.split-shell[data-maximized="true"] .side-slot {
  /* Keep normal slot geometry, but do not clip the escaping surface. */
  overflow: visible;
}

.split-shell[data-maximized="true"] .side-surface {
  position: absolute;
  inset: 0;
  z-index: 1;
  min-inline-size: 0;
}
```

The side slot must remain `position: static`; otherwise it becomes the absolute surface's containing block and the surface cannot cover the shell. The shell is the only positioned ancestor in this small subtree. If the resize rail needs an absolute containing block, place it inside the normally `position: relative` side surface, not on the slot.

This is directly expressible with Tailwind utilities: `relative isolate flex` on the shell; the existing `basis-[clamp(...)]` on the slot; `overflow-hidden bg-background` normally and `overflow-visible` when maximized; `relative size-full bg-background` on the normal surface; and `absolute inset-0 z-10 min-w-0` on that same surface while maximized. Tailwind officially supports both arbitrary `basis-[<value>]` values and the `absolute`/`relative` positioning utilities. [Tailwind flex-basis](https://tailwindcss.com/docs/flex-basis), [Tailwind position](https://tailwindcss.com/docs/position)

If maximize/restore is animated, animate only an overlay-owned visual property; never animate or replace the split track's main/side sizes.

### Grove's grid-overlay implementation

Grove expresses the same two-layer invariant with one full-workspace CSS Grid area. The Main Pane and Side Pane share that area. In the normal open state, the Main Pane's width and the Side Pane's left margin use the same clamped Main Pane width. Opening and closing animate those complementary properties between the clamped width and `100%`, so their boundary stays synchronized for the complete `150ms` transition. Maximizing leaves the Main Pane width untouched and animates only the Side Pane's margin to zero.

The Main Pane therefore keeps its normal layout width beneath the expanding Side Pane. CSS Grid explicitly allows items to overlap and uses `z-index` to control their stacking order. [MDN CSS Grid basic concepts, overlapping items](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Grid_layout/Basic_concepts#overlapping_items)

This variant also lets the Side Pane paint the opaque background throughout every transition. It avoids a separately translated surface and eliminates the transient unpainted gap at its source.

## Why the current maximize behavior crops the main pane

The current implementation changes the normal track when maximizing:

- the main flex item loses its `20rem` minimum;
- the side slot changes to `basis-full`;
- the normal side surface remains absolutely anchored inside that changing slot.

The main pane is therefore genuinely laid out toward zero width. Its descendants are not merely covered; their containing pane is shrinking, so text, tabs, and other content reflow or clip during the transition. An overlay cannot be obtained by maximizing the in-flow slot. The in-flow slot must stay unchanged and only its visual surface may escape it.

This distinction also prevents restore bugs. The normal side intent remains untouched during maximize, so restore requires only changing one boolean. It does not need to reconstruct a width or undo a temporary `basis-full` value.

## Opaque open/close behavior

`background-color` has an initial value of `transparent`, so every box exposed during an animation must explicitly paint a background if no underlying content should show through. [MDN `background-color` formal definition](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/background-color#formal_definition)

For this layout:

- paint the same known-opaque `--background` on both the side slot and side surface;
- verify that the token itself has no alpha component;
- do not animate `opacity`;
- do not combine a changing `flex-basis` with a percentage `translateX`, because the transform distance is recalculated from the element's changing width;
- reveal/hide the normal pane by clipping the fixed-minimum surface inside the animating slot.

The normal open/close path animates the Main Pane width and Side Pane left margin from the same endpoints. Both use `150ms linear` timing, matching the App Sidebar. During a rail drag, both transitions become zero-duration so the shared boundary follows the pointer directly. Tailwind's `overflow-hidden` utility clips pane contents to their live boxes. [Tailwind overflow](https://tailwindcss.com/docs/overflow)

## Preserving a `20rem` main pane when the app sidebar toggles

The simplest model for Grove is to store the Main Pane width directly:

```text
main = clamp(20rem, mainIntent, availableWidth - 20rem)
side = availableWidth - main
```

When the user drags the divider to `20rem`, `mainIntent` becomes `20rem`. Opening or closing the fixed App Sidebar changes `availableWidth`, but the same CSS layout pass keeps the Main Pane at `20rem` and gives the delta to the Side Pane. No `useEffect`, `ResizeObserver`, resize listener, percentage conversion, or edge test is needed.

The previous Side Pane-owned model needed a special semantic maximum:

- ordinary positions stored a pixel Side Pane width;
- putting the Main Pane at its minimum had to store `100%` rather than the current Side Pane pixel width.

That model can work with a one-CSS-pixel edge tolerance, but exact floating-point equality made visually identical releases persist different intent types. Storing the Main Pane intent removes that entire branch and directly represents the behavior Grove requires.

This follows the same high-level ownership rule used by VS Code's Agents window: its workbench grid disables proportional resizing and assigns one high-priority content part to absorb available-space changes while low-priority side parts retain user-set widths. [VS Code Agents window layout specification, layout priority model](https://github.com/microsoft/vscode/blob/main/src/vs/sessions/LAYOUT.md#23-layout-priority-model) Grove's CSS rule is much smaller, but the principle is the same: one part absorbs external layout deltas; there is no observer that measures and writes them back.

## What the primary implementations do—and do not prove

### OpenAI Codex

No inspectable Codex Desktop renderer implementation was found in OpenAI's public Codex repository. The official README describes the repository as Codex CLI and directs desktop users to `codex app`; the public top-level source areas expose the CLI/Rust/SDK rather than desktop renderer code. [OpenAI Codex README](https://github.com/openai/codex#readme), [OpenAI Codex repository tree](https://github.com/openai/codex)

Accordingly, this note does not claim that Codex uses a particular CSS mechanism. The requested visual behavior is a product reference, not source evidence.

### VS Code

VS Code provides two useful but distinct precedents:

- The normal Agents-window row is a non-proportional workbench grid in which the primary Sessions part absorbs visibility and window-size deltas; side parts preserve their user widths. [VS Code `Workbench.createWorkbenchLayout`](https://github.com/microsoft/vscode/blob/main/src/vs/sessions/browser/workbench.ts#L1540-L1545), [VS Code Agents layout](https://github.com/microsoft/vscode/blob/main/src/vs/sessions/LAYOUT.md#22-grid-tree)
- Its experimental docked detail panel reparents the auxiliary bar into the editor container and positions it absolutely on the right, with a dedicated controller for width and sash behavior. [VS Code `DockedAuxiliaryBarController`](https://github.com/microsoft/vscode/blob/main/src/vs/sessions/browser/dockedAuxiliaryBarController.ts)

VS Code's current maximize actions are **not** the requested overlay behavior. The Agents window snapshots size/visibility and hides other parts while maximizing the editor; the standard workbench similarly snapshots and hides sibling parts for a maximized auxiliary bar. [VS Code Agents `setEditorMaximized`](https://github.com/microsoft/vscode/blob/main/src/vs/sessions/browser/workbench.ts#L2756-L2811), [VS Code workbench `setAuxiliaryBarMaximized`](https://github.com/microsoft/vscode/blob/main/src/vs/workbench/browser/layout.ts#L2172-L2236) Copying that behavior would intentionally reflow/hide the main content, which the Grove requirement rejects. The reusable lessons are to keep normal size state reversible and to separate a docked surface from the outer workbench layout.

### shadcn and `react-resizable-panels`

shadcn's Resizable is a composition over `react-resizable-panels`: an in-flow group of panels and handles. [shadcn Resizable](https://ui.shadcn.com/docs/components/base/resizable), [`react-resizable-panels` official repository](https://github.com/bvaughn/react-resizable-panels)

It can own the normal split, but it does not remove the need for a separate maximized overlay layer. Putting `100%` into one resizable panel still changes the group layout and therefore shrinks/collapses its sibling. If Grove keeps its small custom split, the CSS above is simpler; if Grove adopts the library later, keep the group mounted at its normal layout and render/reposition the side surface over the group when maximized.

## Minimal state and event model

React needs only:

```text
sidePaneOpen: boolean
sidePaneMaximized: boolean
mainPaneIntent: `${number}px` | initial percentage
```

Transitions:

- open/close changes only `sidePaneOpen`;
- maximize/restore changes only `sidePaneMaximized`;
- resize changes `mainPaneIntent` only at interaction completion;
- app-sidebar and window resizing change no side-pane state.

During pointer movement, one start measurement plus direct writes to `--main-pane-intent` remains sufficient. On release, commit the Main Pane's pixel width. Maximization should neither read nor write geometry.

## Verification matrix

1. Open the side pane: no main content is visible through any part of the side slot or surface.
2. Close and reopen: the normal intent is unchanged.
3. Resize the main pane to `20rem` with the app sidebar open, then close the app sidebar: main remains `20rem`; the side pane absorbs all freed width.
4. Repeat App Sidebar open/close cycles: the Main Pane remains exactly `20rem`.
5. Toggle the app sidebar without a preceding user resize: no side-pane state update occurs.
6. Maximize: main-pane bounding box, scroll position, and content widths remain unchanged beneath the side surface.
7. Restore: the normal separator returns to the exact pre-maximize location without a measurement.
8. While maximized, the app sidebar stays visible and the side surface covers only the post-sidebar split shell.
9. Keyboard focus cannot enter the visually covered main pane while maximized.
10. React DevTools shows no rerender from passive window/app-sidebar resizing and no rerender per pointer move.
