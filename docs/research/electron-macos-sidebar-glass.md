# Grove sidebar appearance experiment

Date: 2026-08-23

Status: implementation guidance for a temporary renderer switcher, not a product decision

## Conclusion

Grove initially compared six useful sidebar treatment families without adding a native bridge.
After the first visual pass, the temporary selector was narrowed to six concrete entries:

1. one softly separated opaque grayscale surface;
2. one standard native treatment midway between the initial airy and muted covers;
3. the current shadcn-like neutral glass surface;
4. that same surface with restrained Plum, Ocean, and Moss Workspace color touches.

The native option must be described honestly as the native sidebar material with a neutral
renderer tint overlay. Electron exposes no sidebar-vibrancy intensity control. This experiment
can keep the existing native backdrop enabled and change only the CSS surface above it; the
opaque variant simply covers it. That avoids temporary renderer-to-main IPC and preserves
Grove's renderer boundary.

This effect is native macOS vibrancy, but it is not macOS Tahoe's true Liquid Glass sidebar. Electron 43.4.0 creates a legacy `NSVisualEffectView`; Tahoe's native sidebar glass comes from an AppKit `NSSplitViewController` configured with sidebar behavior. Apple explicitly says that an old `NSVisualEffectView` sidebar material prevents the new glass from showing through. [Apple, “Build an AppKit app with the new design”](https://developer.apple.com/videos/play/wwdc2025/310/?time=205)

## What macOS actually provides

### Standard visual-effect materials

`NSVisualEffectView` combines translucency, background blur, and vibrancy. Apple says the material and blending mode determine the result, the result can change with system settings, and materials must be chosen for their semantic use rather than their sampled color. `sidebar` is the material intended for a window sidebar. [Apple `NSVisualEffectView`](https://developer.apple.com/documentation/appkit/nsvisualeffectview)

AppKit offers two distinct sampling models:

- `behindWindow` samples the desktop or other windows behind the app window.
- `withinWindow` samples content behind the view inside the same window.

Apple's own sidebar illustration uses behind-window blending, while toolbars normally use within-window blending. [Apple `NSVisualEffectView.BlendingMode`](https://developer.apple.com/documentation/appkit/nsvisualeffectview/blendingmode-swift.enum), [Apple `blendingMode`](https://developer.apple.com/documentation/appkit/nsvisualeffectview/blendingmode-swift.property)

The visual-effect state is about window activity, not translucency strength:

- `followsWindowActiveState` adapts when the window gains or loses focus;
- `active` forces the active appearance;
- `inactive` forces the inactive appearance.

The default is `followsWindowActiveState`. It is the correct Grove setting because an unfocused window should visually recede. [Apple `NSVisualEffectView.State`](https://developer.apple.com/documentation/appkit/nsvisualeffectview/state-swift.enum), [Apple `state`](https://developer.apple.com/documentation/appkit/nsvisualeffectview/state-swift.property)

The standard material does not expose a continuous “intensity” property. `isEmphasized` is only a boolean and Apple describes it as emphasizing some materials, for example to convey first-responder status; it is not an opacity control. [Apple `isEmphasized`](https://developer.apple.com/documentation/appkit/nsvisualeffectview/isemphasized)

Apple recommends grayscale foreground content on vibrancy because AppKit can improve its contrast while only subtly changing its hue. It recommends semantic label colors instead of custom grayscale swatches for native views. Grove's foreground is HTML rather than AppKit content, so AppKit does not automatically provide that vibrant label treatment; Grove still owns and must test the renderer's text contrast. [Apple `NSVisualEffectView`](https://developer.apple.com/documentation/appkit/nsvisualeffectview)

### Tahoe Liquid Glass is a different path

On macOS Tahoe, AppKit automatically presents a split item with sidebar behavior on appropriate Liquid Glass. Apple says to remove legacy sidebar `NSVisualEffectView`s because they block that glass. Custom Liquid Glass uses `NSGlassEffectView`, places the real content in its `contentView`, and can apply corner-radius and tint-color customization. [Apple, “Build an AppKit app with the new design”](https://developer.apple.com/videos/play/wwdc2025/310/?time=205), [Apple `NSGlassEffectView`](https://developer.apple.com/documentation/appkit/nsglasseffectview)

Apple's design guidance treats Liquid Glass as a functional navigation/control layer above opaque content, not as a content background. It recommends the more opaque regular variant for text-heavy structures such as sidebars and popovers, uses clear glass only over visually rich media, and says color should be applied sparingly. Larger components such as sidebars intentionally appear more opaque to preserve legibility. [Apple HIG, Materials](https://developer.apple.com/design/human-interface-guidelines/materials), [Apple HIG, Color](https://developer.apple.com/design/human-interface-guidelines/color)

This supports Grove's current structure: keep the main inset opaque and use translucency only for the surrounding navigation shell.

## What Electron 43.4.0 exposes

Grove pins Electron 43.4.0 in [`aube-workspace.yaml`](../../aube-workspace.yaml). Electron's public API exposes:

- constructor options `vibrancy` and `visualEffectState` on macOS;
- runtime `win.setVibrancy(type | null)`;
- optional fade-in/fade-out duration for enabling or removing vibrancy;
- runtime `win.setBackgroundColor(...)`, including colors with alpha.

The fade duration does not interpolate between material types. Electron explicitly says animation between vibrancy types is unsupported. [Electron `BaseWindow` options](https://www.electronjs.org/docs/latest/api/base-window), [Electron `BrowserWindow.setVibrancy`](https://www.electronjs.org/docs/latest/api/browser-window#winsetvibrancytype-options-macos), [Electron `BrowserWindow.setBackgroundColor`](https://www.electronjs.org/docs/latest/api/browser-window#winsetbackgroundcolorbackgroundcolor)

Electron 43.4.0's implementation maps `sidebar` to `NSVisualEffectMaterialSidebar`, creates one visual-effect view at full-window size, forces `NSVisualEffectBlendingModeBehindWindow`, and inserts it below every other view. Transparent renderer pixels reveal the effect; opaque renderer pixels cover it. The implementation applies the constructor's visual-effect state when it creates the native view. [Electron 43.4.0 `native_window_mac.mm`](https://github.com/electron/electron/blob/v43.4.0/shell/browser/native_window_mac.mm#L1397-L1516)

Grove already uses the supported shape of this API in [`apps/desktop/src/main/index.ts`](../../apps/desktop/src/main/index.ts): `sidebar`, `followWindow`, a transparent window background while vibrancy is enabled, and an opaque fallback when reduced transparency is requested.

### Public-API limits

Electron's public API does **not** expose:

- a blur radius or vibrancy-intensity value;
- `NSVisualEffectView.isEmphasized`;
- a runtime setter for `visualEffectState`;
- a choice between behind-window and within-window blending;
- a view-local native vibrancy region;
- `NSGlassEffectView`, `NSGlassEffectContainerView`, or native `NSSplitViewItem` sidebar behavior;
- a native tint color for its macOS vibrancy view.

Changing `BrowserWindow` opacity is not a substitute because it dims the entire window, including text and content. Selecting `menu`, `popover`, or another material merely because it looks lighter or darker would also contradict Apple's semantic-material guidance.

CSS cannot recreate behind-window blur. Electron documents that CSS blur applies to web contents only and cannot blur other applications behind the window. A CSS `backdrop-filter` can be useful when actual renderer content lies behind an element, but it cannot replace AppKit's desktop sampling here. [Electron, transparent-window limitations](https://www.electronjs.org/docs/latest/tutorial/custom-window-styles#limitations)

Therefore the honest native comparison is one semantic `sidebar` material with different neutral or colored CSS overlays. The overlay changes how much desktop color remains visible; it does not change the native blur kernel or create a second AppKit material intensity.

## Arc and Dia: sourced behavior versus design inference

### Sourced facts

Arc defines Spaces as separate browsing contexts, and each Space owns a Theme and Icon. A person can choose a color or gradient per Space, while Arc's Light/Dark/Automatic appearance applies globally across all Spaces. [Arc Help, Spaces](https://resources.arc.net/hc/en-us/articles/19228064149143-Spaces-Distinct-Browsing-Areas), [Arc, “Paint with the internet”](https://start.arc.net/paint-the-internet)

Arc's first-party release history shows that this is broader than coloring one panel: Space theme colors expanded into the command bar, previews, menus, fields, settings, selections, and split-view controls. Arc also exposed theme Intensity and Graininess controls. [Arc for macOS 2022 release notes](https://resources.arc.net/hc/en-us/articles/20498417809815-Arc-for-macOS-2022-Release-Notes), [Arc for Windows release notes](https://resources.arc.net/hc/en-us/articles/22513842649623-Arc-for-Windows-2023-2024-Release-Notes)

Dia uses color more as contextual wayfinding. Its official notes say Tab Groups can have colors, new group colors can derive from a site's favicon or URL-bar theme color, and profile color helps identify the current context. Dia later described refining profile theme colors so windows feel cohesive “without shouting for attention.” It also now lets the top browser band adopt the current site's color. [Dia, “Organize your Tabs”](https://www.diabrowser.com/release-notes/1-9-0-tab-groups), [Dia, “No loose ends”](https://www.diabrowser.com/release-notes/1-10-1-year-end-release), [Dia 1.16.0](https://www.diabrowser.com/changelog/1-16-0), [Dia, “A closer look”](https://www.diabrowser.com/release-notes/1-28-0-look-closer)

### Design inference

The following is an interpretation of those first-party descriptions, not a claim about Arc or Dia implementation internals:

- Arc treats color as user-authored identity for a whole Space and permits a more expressive, chrome-wide theme.
- Dia uses a quieter hierarchy: profile color for context, group color for wayfinding, site-derived color to connect chrome to content, and stronger color only for brief moments of emphasis.
- Grove's Workspace maps more closely to Arc's Space at the product-semantic level, but Grove's compact tool UI benefits from Dia's more restrained application. A low-chroma wash should identify a Workspace without recoloring ordinary text, icons, focus rings, or status colors.

No first-party Arc or Dia source found here publishes blur radii, opacity values, OKLCH tokens, or material-layer recipes. Values inferred from screenshots would be visual estimates and must not be represented as facts.

## Recommended temporary variants

Keep the existing main inset surface unchanged for the experiment. Put a `data-sidebar-appearance` value on the desktop shell and make all six options resolve the same small set of sidebar tokens. The header's leading region and the sidebar must consume the same surface token so they remain one visual plane.

| ID                | Switcher label    | Mechanism                                              | Suggested starting surface                                                 | What it tests                                                          |
| ----------------- | ----------------- | ------------------------------------------------------ | -------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `opaque-soft`     | Gray · soft       | Opaque CSS                                             | Light `oklch(0.97 0 0)`; dark `oklch(0.18 0 0)`                            | Small separation from the main inset without wallpaper variability     |
| `native-standard` | Native · standard | Electron `sidebar` plus CSS tint                       | `color-mix(in oklch, var(--popover) 35%, transparent)`                     | A balanced midpoint between wallpaper contribution and stable contrast |
| `glass-neutral`   | Glass · neutral   | Same native substrate plus shadcn-like neutral surface | Current 70% popover/30% transparent surface                                | The current restrained “translucent preset” feeling                    |
| `glass-workspace` | Glass · Workspace | Tinted shadcn-like glass                               | Tint the opaque popover, then apply the same 70% coverage as neutral glass | Arc-like identity with Dia-like restraint                              |

The implementation expands `glass-workspace` into Plum, Ocean, and Moss entries so the
experiment can judge whether the treatment works across hues, rather than accidentally choosing
one color that flatters the surrounding desktop.

These values are starting points for side-by-side visual testing, not platform constants. Preserve the current mostly grayscale OKLCH scheme:

- keep `foreground`, `muted-foreground`, border, hover, and focus tokens independent of the Workspace tint;
- keep the Workspace tint low-chroma (roughly `C = 0.025–0.05`) and alter hue between mock Workspaces;
- keep hover/selected deltas based on neutral white or black alpha so interaction strength remains perceptually consistent across colors;
- do not use tint as the only Workspace identifier; retain text or an icon because both Apple and Electron expose “differentiate without color” accessibility preferences. [Apple HIG, Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility/), [Electron `nativeTheme.shouldDifferentiateWithoutColor`](https://www.electronjs.org/docs/latest/api/native-theme#nativethemeshoulddifferentiatewithoutcolor-macos-readonly)

The initial 20% and 50% native covers established the useful range; the current 35% cover is its
midpoint. It is a renderer tint over one native material, not a separate vibrancy intensity.

## Switcher implementation guidance

For this temporary comparison:

1. Keep `vibrancy: "sidebar"` and `visualEffectState: "followWindow"` in the main process whenever macOS transparency is permitted.
2. Keep the content inset opaque. Only the sidebar and its continuous header region should use the experimental surface.
3. Place a compact temporary switcher in the trailing desktop header. Hold its state in the renderer and set `data-sidebar-appearance` on the shell. Do not persist it yet.
4. Do not add IPC only to toggle the experiment. Opaque modes can cover the native material, and all translucent modes share the same native substrate.
5. On non-macOS platforms, fall back from the native option to the opaque neutral treatment; do not claim native macOS vibrancy there.
6. Treat the reinstalled non-translucent shadcn dropdown/popover components as a separate decision. Popups should use the preset's ordinary opaque `popover` surface, not inherit sidebar glass.

This experiment should remain one shallow presentation module: a finite appearance ID, the temporary control, and theme-token mappings. It should not become a new design-system primitive or a durable Workspace setting until a winning treatment is selected.

## Accessibility and fallback behavior

Apple says that when Reduce Transparency is enabled, apps should avoid semitransparent backgrounds and use opaque windows. Electron exposes the preference as `nativeTheme.prefersReducedTransparency`. [Apple `accessibilityDisplayShouldReduceTransparency`](https://developer.apple.com/documentation/appkit/nsworkspace/accessibilitydisplayshouldreducetransparency), [Electron `nativeTheme.prefersReducedTransparency`](https://www.electronjs.org/docs/latest/api/native-theme#nativethemeprefersreducedtransparency-readonly)

When reduced transparency is active:

- the main process should keep removing vibrancy and restoring an opaque window background, as Grove already does;
- every selected translucent variant should resolve to `opaque-soft` in CSS;
- the switcher may keep the person's selected experimental ID, but it should visually indicate that the OS accessibility fallback is active if the distinction matters during testing.

Electron's `nativeTheme.updated` event is only documented as firing when something in the underlying native theme changes and says this normally means dark appearance, high contrast, or inverted colors. It does not explicitly promise a notification for Reduce Transparency. Grove's existing listener re-checks `prefersReducedTransparency`, which is the narrowest public-Electron approach, but the macOS toggle must be tested manually. A native bridge could instead observe Apple's documented `accessibilityDisplayOptionsDidChangeNotification` if the Electron event does not fire. [Electron `nativeTheme.updated`](https://www.electronjs.org/docs/latest/api/native-theme#event-updated), [Apple `accessibilityDisplayOptionsDidChangeNotification`](https://developer.apple.com/documentation/appkit/nsworkspace/accessibilitydisplayoptionsdidchangenotification)

Also test Increase Contrast independently and together with Reduce Transparency. Apple specifically recommends testing both settings in Dark Mode, and its accessibility guidance uses 4.5:1 as the minimum reference for ordinary small text. [Apple HIG, Dark Mode](https://developer.apple.com/design/human-interface-guidelines/dark-mode), [Apple HIG, Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility/)

## QA matrix

Compare each variant under all of the following before selecting one:

- dark and light appearance;
- active and inactive window states;
- bright, dark, saturated, and grayscale desktop backgrounds;
- Reduce Transparency and Increase Contrast, separately and together;
- sidebar open/close animation and the collapsed leading-header region;
- focus rings, muted project metadata, hover fills, selected rows, and destructive/status colors;
- external display and built-in display if available;
- non-macOS opaque fallback.

Judge identity separately from legibility: first ask whether a person can tell Workspaces apart, then whether all sidebar content remains as calm and readable as the neutral version.

## What Grove cannot claim without a native bridge

With Electron's current public API, Grove cannot honestly claim:

- two native sidebar blur or translucency intensities;
- a renderer-local native material that samples only under the sidebar;
- within-window AppKit blending under the sidebar;
- Tahoe's adaptive Liquid Glass sidebar geometry, refraction, grouping, or native tint system;
- Arc's or Dia's exact material implementation.

Exact Tahoe sidebar glass would require restructuring the window around native AppKit split-view behavior or adding a focused native bridge for `NSGlassEffectView`/related APIs. That is materially larger than a temporary visual comparison and conflicts with Grove's current preference for the smallest renderer-first slice.
