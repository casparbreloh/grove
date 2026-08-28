# Electron macOS glass sidebar in 2026

Research date: 2026-08-28. Scope: Grove's Electron 43.4.0 desktop window, macOS 26 Liquid Glass, and the current renderer sidebar token.

## Conclusion

Grove should keep Electron's `vibrancy: "sidebar"` and `visualEffectState: "followWindow"` as its production-safe native material, but it should not describe that effect as macOS 26 Liquid Glass. Stock Electron 44.0.0 still implements the option with the older `NSVisualEffectView`; it does not expose `NSGlassEffectView`.

The blue regression is caused by the renderer tint, not by choosing the wrong Electron material. A translucent black layer darkens the blue wallpaper contribution without neutralizing its hue. The clean middle ground is to restore the pre-thread background-derived tint, make it modestly more opaque, and keep the token Shadcn-native:

```css
:root {
  --sidebar: color-mix(in oklch, var(--background) 62%, transparent);
}
```

This is a **62% neutral theme tint over 38% visible native material** in both light and dark mode. It is a renderer compositing percentage, not a setting for AppKit's internal blur or glass strength. If Grove temporarily retains its system/app-theme mismatch guards, those should also mix from `var(--background)` rather than from pure black or white; a stronger `72%` neutral tint is reasonable only for that mismatch fallback.

## What Electron actually renders

Electron documents `vibrancy` as a macOS `BaseWindow` option and includes `sidebar` among the semantic material values. It documents `followWindow` as the default activity behavior: active when the window is active and inactive when it is not. [`BaseWindow` options](https://www.electronjs.org/docs/latest/api/base-window#new-basewindowoptions)

Electron 44's implementation maps `sidebar` to `NSVisualEffectMaterialSidebar`, constructs an `NSVisualEffectView`, sets `NSVisualEffectBlendingModeBehindWindow`, and inserts it beneath the web content. The native effect covers the full content-view bounds; the renderer's opaque main surface and translucent `--sidebar` surface determine where it is visible. [`NativeWindowMac::SetVibrancy`](https://github.com/electron/electron/blob/v44.0.0/shell/browser/native_window_mac.mm#L1406-L1522)

When a `BrowserWindow` has vibrancy and no explicit web-content background color, Electron propagates a transparent background to the `WebContents`. Grove's choice to omit an opaque constructor background while vibrancy is enabled is therefore source-aligned. [`BrowserWindow` constructor implementation](https://github.com/electron/electron/blob/v44.0.0/shell/browser/api/electron_api_browser_window.cc#L31-L47)

`visualEffectState: "followWindow"` should remain. Forcing `active` makes an unfocused window retain the active material and loses normal macOS focus feedback; it does not make the effect less colorful or more opaque.

## Electron versus macOS 26 Liquid Glass

Apple's current AppKit API for custom Liquid Glass is `NSGlassEffectView`, with `regular` and `clear` styles, an optional tint, and `NSGlassEffectContainerView` for efficiently coordinating nearby glass shapes. [`NSGlassEffectView`](https://developer.apple.com/documentation/appkit/nsglasseffectview) and [`NSGlassEffectContainerView`](https://developer.apple.com/documentation/appkit/nsglasseffectcontainerview)

Apple's macOS 26 guidance is explicit that a legacy `NSVisualEffectView` inside an AppKit sidebar prevents the new sidebar glass from showing through. Standard AppKit split-view sidebars receive the new material through `NSSplitViewController`; custom glass belongs in `NSGlassEffectView`. [WWDC25: Build an AppKit app with the new design](https://developer.apple.com/videos/play/wwdc2025/310/?time=1050)

Electron 44.0.0 was released on 2026-08-25, but its public window API still only exposes `vibrancy`, and its macOS implementation still uses `NSVisualEffectView`. Its source contains no `NSGlassEffectView` integration. [Electron 44.0.0 release](https://releases.electronjs.org/release/v44.0.0), [`BrowserWindow.setVibrancy`](https://www.electronjs.org/docs/latest/api/browser-window#winsetvibrancytype-options-macos), and [Electron 44 macOS implementation](https://github.com/electron/electron/blob/v44.0.0/shell/browser/native_window_mac.mm#L1406-L1522)

Consequently:

- upgrading Grove from Electron 43.4.0 to 44 is sensible maintenance, but it does not unlock Liquid Glass;
- true Tahoe Liquid Glass would require a carefully maintained native AppKit bridge that embeds Electron's content in `NSGlassEffectView`;
- that bridge is a separate platform-integration project, not a CSS refinement, and it should not be introduced through a reverse-engineered package merely to tune this sidebar.

For the current vertical slice, Electron's semantic `sidebar` vibrancy is still the correct supported fallback. It matches the component's purpose and remains more native than a renderer-only `backdrop-filter` approximation.

## Why black made the blue stronger

Behind-window vibrancy intentionally samples the desktop. Wallpaper color therefore participates in the native result. Apple similarly explains that Liquid Glass takes on color from the content beneath it, and that large text-bearing surfaces such as sidebars appear more opaque to preserve legibility. [Apple HIG: Color](https://developer.apple.com/design/human-interface-guidelines/color) and [Apple HIG: Materials](https://developer.apple.com/design/human-interface-guidelines/materials)

Normal source-over compositing is defined per color channel. With a black source layer, the source contribution is zero, so the backdrop channels are mainly scaled by the remaining transparency. The blue channel's relationship to the red and green channels is not pulled toward a neutral gray; the result is simply darker and can read as more intensely blue. [W3C Compositing and Blending Level 1](https://www.w3.org/TR/compositing-1/#simplealphacompositing)

Mixing from Grove's neutral `--background` instead adds an achromatic source contribution. Increasing that tint from the old 55% to 62% reduces wallpaper chroma while preserving enough native material to read as glass. Pure black and pure white are both inferior defaults because they ignore the theme's actual neutral surface level.

Do not add a second CSS `backdrop-filter`, blur, or saturation pass over Electron's native view. Electron already performs the behind-window material sampling; another filter makes the surface foggier, less predictable, and more expensive without turning it into Liquid Glass.

## Apple-aligned material weight

Apple recommends the regular Liquid Glass variant for components where background content can affect legibility, including sidebars and popovers; `clear` is intended for sparse controls over visually rich media. It also says larger elements such as sidebars deliberately look more opaque. [Apple HIG: Materials](https://developer.apple.com/design/human-interface-guidelines/materials) and [Meet Liquid Glass](https://developer.apple.com/videos/play/wwdc2025/219/)

That makes Grove's goal a **frosted, neutral structural surface**, not maximum wallpaper visibility. The 62% background-derived tint is a reasonable Electron approximation of that material weight. The native blur supplies depth; the neutral renderer tint supplies the opacity and color discipline Electron's legacy material cannot expose as a parameter.

## Accessibility and theme coordination

Electron exposes `nativeTheme.prefersReducedTransparency` and emits `updated` when native appearance preferences change. It also documents `nativeTheme.themeSource` as the synchronization point for system, light, and dark appearance across Chromium and macOS-rendered UI. [`nativeTheme`](https://www.electronjs.org/docs/latest/api/native-theme)

Grove's main process already follows the correct reduced-transparency lifecycle:

1. avoid or remove vibrancy when reduction is requested;
2. use an opaque native window background;
3. listen for `nativeTheme.updated` so the behavior changes without restarting;
4. provide a solid renderer `--sidebar` fallback through `prefers-reduced-transparency`.

Keep that behavior. Apple says Reduced Transparency makes native Liquid Glass frostier and obscures more background content; for Grove's legacy Electron path, fully removing native vibrancy and rendering a solid semantic surface is the reliable equivalent. [Meet Liquid Glass](https://developer.apple.com/videos/play/wwdc2025/219/#:~:text=Reduced%20Transparency) and [`nativeTheme.prefersReducedTransparency`](https://www.electronjs.org/docs/latest/api/native-theme#nativethemeprefersreducedtransparency-readonly)

Grove currently lets the renderer choose light or dark independently through `next-themes`, while the native material follows AppKit. The durable solution remains a narrow theme command that sets `nativeTheme.themeSource`; CSS media-query compensation is only a fallback. This coordination issue is covered separately in [Coordinating a macOS vibrant sidebar with renderer themes](./macos-vibrant-sidebar-theme-coordination.md).

## Recommended implementation

1. Keep `vibrancy: "sidebar"` and `visualEffectState: "followWindow"`.
2. Keep the native effect transparent at the window/WebContents layer and keep Grove's main content opaque.
3. Replace the light 28%-white and dark 45%-black sidebar declarations with `color-mix(in oklch, var(--background) 62%, transparent)`.
4. If mismatch guards remain, derive them from `var(--background)` too; use 72% only to mask the wrong native appearance.
5. Keep `--sidebar: var(--secondary)` under `prefers-reduced-transparency` and keep disabling native vibrancy in main.
6. Do not add CSS blur/saturation or a third-party native glass dependency for this refinement.
7. Track true `NSGlassEffectView` support as a separate native-integration decision, and reassess when Electron exposes a supported API.
