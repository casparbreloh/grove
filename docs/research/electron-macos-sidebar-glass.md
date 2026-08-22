# Electron macOS sidebar glass

Date: 2026-08-22

## Recommendation

Use Electron's native macOS `sidebar` vibrancy as a restrained, platform-specific backdrop, not as an emulation of Apple's exact Liquid Glass API. Keep Grove's content pane opaque, expose the native backdrop only through transparent sidebar/header surfaces, and use the ordinary `foreground` and `muted-foreground` tokens for renderer text. Keep one narrow component token for the sidebar hover fill (for example `--sidebar-hover`) because that value is specific to legibility over a variable native backdrop.

This is a small change in Grove because the main window already sets `vibrancy: "sidebar"`, `visualEffectState: "followWindow"`, `titleBarStyle: "hiddenInset"`, and a macOS-only traffic-light position in [`apps/desktop/src/main/index.ts`](../../apps/desktop/src/main/index.ts). The native effect is currently hidden by two layers:

1. The window explicitly sets the opaque `backgroundColor: "#171717"`.
2. The renderer paints `bg-sidebar` on the shell, header, and sidebar panel.

On macOS, conditionally omit the opaque window background when vibrancy is active; keep an opaque, theme-matching background on other platforms and whenever reduced transparency is preferred. In the renderer, make only the sidebar-side shell/header/panel surfaces transparent. Do **not** make the content inset transparent.

## Why this is the supported Electron path

- Electron 43.4.0 is pinned in [`aube-workspace.yaml`](../../aube-workspace.yaml). Its public window options include macOS-only `vibrancy` and `visualEffectState`; `followWindow` makes the material track active/inactive window state and is already the default. [`sidebar` is the semantically appropriate material](https://www.electronjs.org/docs/latest/api/base-window#new-basewindowoptions), rather than choosing a material by a fixed sampled color.
- Electron's 43.4.0 implementation maps `sidebar` to `NSVisualEffectMaterialSidebar`, creates a full-window `NSVisualEffectView` with `BehindWindow` blending, and inserts it underneath all other views. The web UI decides where it is visible by leaving only those pixels transparent. [Electron 43.4.0 source](https://github.com/electron/electron/blob/v43.4.0/shell/browser/native_window_mac.mm#L1397-L1513)
- Vibrancy itself makes the native window translucent. If no explicit `backgroundColor` is supplied, Electron chooses a transparent background; an explicit opaque color overrides that default. [Electron 43.4.0 source](https://github.com/electron/electron/blob/v43.4.0/shell/browser/native_window.cc#L209-L223), [translucency check](https://github.com/electron/electron/blob/v43.4.0/shell/browser/native_window.cc#L758-L774)
- `win.setVibrancy("sidebar")` can enable the effect at runtime and `win.setVibrancy(null)` removes it. The optional duration only fades the effect in or out; Electron cannot animate between material types. For Grove's snappy interaction style, use no vibrancy animation. [Electron BrowserWindow API](https://www.electronjs.org/docs/latest/api/browser-window#winsetvibrancytype-options-macos)
- `visualEffectState` is documented only as a constructor option; Electron exposes no runtime setter. Keep `followWindow` and do not force the active appearance while the window is inactive. [Electron BaseWindow options](https://www.electronjs.org/docs/latest/api/base-window#new-basewindowoptions)

## Window and title-bar constraints

- Keep all vibrancy and traffic-light configuration behind `process.platform === "darwin"`; these are macOS-only APIs. Preserve a normal opaque surface on Windows and Linux.
- `hiddenInset` keeps native traffic lights and shifts them inward. `trafficLightPosition` provides the precise placement Grove already uses. [Electron custom-title-bar guide](https://www.electronjs.org/docs/latest/tutorial/custom-title-bar#custom-traffic-lights-macos)
- Keep `titleBarOverlay: true` in Grove because the renderer uses the resulting `env(titlebar-area-*)` CSS variables to protect the traffic-light area. The overlay requires a non-default title-bar style. [Electron custom-title-bar guide](https://www.electronjs.org/docs/latest/tutorial/custom-title-bar#custom-window-controls)
- Do not add `transparent: true`. It is a different full-window feature and brings avoidable constraints: unreliable resizing on some platforms, no macOS native shadow, no transparency while DevTools is open, and no ability for CSS blur to blur other applications behind the window. Native vibrancy already supplies the supported behind-window blur. [Electron transparent-window constraints](https://www.electronjs.org/docs/latest/tutorial/custom-window-styles#transparent-windows)

## Accessibility

Electron exposes the OS preference as `nativeTheme.prefersReducedTransparency`. Apple says that when it is true, apps should avoid semitransparent backgrounds and use opaque windows instead. [Electron `nativeTheme`](https://www.electronjs.org/docs/latest/api/native-theme#nativethemeprefersreducedtransparency-readonly), [Apple accessibility guidance](https://developer.apple.com/documentation/appkit/nsworkspace/accessibilitydisplayshouldreducetransparency)

At window creation, skip vibrancy when that preference is true. For a runtime response, re-check it on Electron's `nativeTheme.updated` event and switch between `win.setVibrancy(null)` plus an opaque theme-matching background, and `win.setVibrancy("sidebar")` plus a clear background. Electron describes `updated` as a general native-theme change signal but does not explicitly promise that reduced-transparency changes emit it, so this needs a manual macOS Accessibility toggle test. If that test fails, observe Apple's `accessibilityDisplayOptionsDidChangeNotification`, which Apple documents for this setting. [Electron `nativeTheme.updated`](https://www.electronjs.org/docs/latest/api/native-theme#event-updated), [Apple notification](https://developer.apple.com/documentation/appkit/nsworkspace/accessibilitydisplayoptionsdidchangenotification)

The native backdrop does not make HTML text into AppKit-vibrant controls. Grove still owns text contrast. Use `text-foreground` for primary task/navigation labels, `text-muted-foreground` for project metadata and secondary actions, and test both appearances against varied desktop content with Increase Contrast and Reduce Transparency enabled.

## Liquid Glass boundary

This produces a native, adaptive frosted sidebar in the direction of Apple's glass design, but it is not exact Liquid Glass. Apple's current custom AppKit API is `NSGlassEffectView`, with content placed inside its `contentView`; Electron 43.4.0's public API and implementation still use `NSVisualEffectView` and expose no `NSGlassEffectView` option. Exact Liquid Glass behavior would therefore require native AppKit code or a future Electron API and is outside this renderer-first slice. [Apple `NSGlassEffectView`](https://developer.apple.com/documentation/appkit/nsglasseffectview), [Apple adoption guidance](https://developer.apple.com/documentation/technologyoverviews/adopting-liquid-glass), [Electron implementation](https://github.com/electron/electron/blob/v43.4.0/shell/browser/native_window_mac.mm#L1471-L1513)

## Suggested narrow slice

1. Replace sidebar-specific text utilities with the regular foreground/muted-foreground utilities.
2. Retain only a `--sidebar-hover` token, with light/dark values tuned over the native material.
3. Make the sidebar-side renderer surfaces transparent and leave `SidebarInset` opaque.
4. Remove the explicit opaque macOS `backgroundColor` only when vibrancy is enabled; keep the existing non-macOS fallback.
5. Honor reduced transparency with an opaque fallback.
6. QA active/inactive windows, light/dark appearances, wallpaper/content variation, sidebar animation, traffic lights, DevTools, and the Reduce Transparency toggle before considering a native `NSGlassEffectView` bridge.
