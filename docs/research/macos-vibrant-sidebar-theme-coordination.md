# Coordinating a macOS vibrant sidebar with renderer themes

## Question

How should Grove coordinate Electron's native macOS sidebar vibrancy with a renderer-controlled light/dark theme, especially before renderer-to-main theme IPC exists?

## Findings

### The native material follows AppKit appearance, not renderer CSS

Electron implements `vibrancy: "sidebar"` by creating an `NSVisualEffectView`, mapping the value to `NSVisualEffectMaterialSidebar`, and setting its material, behind-window blending mode, and activity state. It does not assign an appearance directly to that view. [Electron macOS window implementation](https://github.com/electron/electron/blob/cff8e1dfdc0af58b7b905ccfb01c76ca27ecf361/shell/browser/native_window_mac.mm#L1403-L1519)

AppKit appearances inherit down the native hierarchy: windows inherit their application's appearance, and views inherit the nearest ancestor or window appearance. [Apple `NSAppearance`](https://developer.apple.com/documentation/appkit/nsappearance)

Therefore, Grove's native sidebar material follows the effective `NSApplication`/`NSWindow` appearance. A `.dark` class or `color-scheme` declaration inside Chromium does not change that native appearance.

`visualEffectState` is unrelated to light/dark mode. It selects whether the effect follows window activity or always looks active or inactive. [Electron `BaseWindow` options](https://www.electronjs.org/docs/latest/api/base-window#new-basewindowoptions)

### `nativeTheme.themeSource` is Electron's synchronization point

Electron documents `nativeTheme.themeSource` as the `system | light | dark` override that changes macOS-rendered UI and propagates the corresponding `prefers-color-scheme` value to renderers. It recommends the conventional three-state mapping of system, dark, and light. [Electron `nativeTheme.themeSource`](https://www.electronjs.org/docs/latest/api/native-theme#nativethemethemesource)

The implementation confirms the native behavior: forced dark assigns `NSAppearanceNameDarkAqua`, forced light assigns `NSAppearanceNameAqua`, and system clears the override; Electron then assigns this appearance to `NSApplication`. [Electron native-theme macOS implementation](https://github.com/electron/electron/blob/cff8e1dfdc0af58b7b905ccfb01c76ca27ecf361/shell/browser/api/electron_api_native_theme_mac.mm#L11-L27)

The durable implementation should therefore make the main process own the selected theme, set `nativeTheme.themeSource`, and expose a narrow validated theme API to the renderer. That one setting aligns AppKit vibrancy, Chromium's preference, and other native UI.

### Renderer `color-scheme` cannot retheme native vibrancy

The CSS `color-scheme` property controls an element's used color scheme and browser-rendered details such as system colors, form controls, scrollbars, and the root canvas. Its defined effects do not include host-platform views outside the document. [CSS Color Adjustment, effects of the used color scheme](https://www.w3.org/TR/css-color-adjust-1/#color-scheme-effect)

`next-themes` applies the selected theme attribute/class to `document.documentElement`; with `enableColorScheme`, it also writes `document.documentElement.style.colorScheme`. [next-themes implementation](https://github.com/pacocoursey/next-themes/blob/a7eeabc39cfb37d74ea3d82eac674d0e1851b1cb/next-themes/src/index.tsx#L52-L85) Its own documentation describes `enableColorScheme` as affecting browser built-in UI such as inputs and buttons. [next-themes `ThemeProvider`](https://github.com/pacocoursey/next-themes/blob/a7eeabc39cfb37d74ea3d82eac674d0e1851b1cb/next-themes/README.md#themeprovider)

Consequently, CSS cannot force that material from Dark Aqua to Aqua or vice versa. A partially transparent light overlay over a dark native material remains visually muddy and environment-dependent.

In a live Grove/Electron compositor check, DevTools reported the sidebar element and its `--sidebar` token as fully opaque white while the captured native sidebar still rendered dark gray. For Grove's current window configuration, even an opaque renderer surface is therefore not a reliable mismatch fallback while the native visual-effect view remains enabled.

### Use the semantic sidebar material and respect accessibility

Apple says to select a visual-effect material by intended use rather than by its apparent color, because material appearance can change with the system environment. It specifically recommends the sidebar material for sidebar backgrounds. [Apple `NSVisualEffectView`](https://developer.apple.com/documentation/appkit/nsvisualeffectview)

For Reduce Transparency, Apple says not to use semitransparent backgrounds and gives opaque windows as the expected alternative. [Apple `accessibilityDisplayShouldReduceTransparency`](https://developer.apple.com/documentation/appkit/nsworkspace/accessibilitydisplayshouldreducetransparency)

Electron exposes the native preference as `nativeTheme.prefersReducedTransparency`. [Electron `nativeTheme.prefersReducedTransparency`](https://www.electronjs.org/docs/latest/api/native-theme#nativethemeprefersreducedtransparency-readonly) CSS also defines `@media (prefers-reduced-transparency: reduce)` for renderer content. [Media Queries Level 5](https://www.w3.org/TR/mediaqueries-5/#prefers-reduced-transparency)

Both layers matter: main should remove or avoid native vibrancy when reduction is requested, while renderer CSS should provide an opaque sidebar surface. Making only the web layer opaque does not remove the underlying `NSVisualEffectView`.

## Recommended Grove behavior

### Durable behavior

1. Keep Electron's semantic `sidebar` material and a transparent renderer background over it.
2. Make main-process `nativeTheme.themeSource` the source of truth for `system`, `light`, and `dark`.
3. Add only a narrow validated theme command/state surface when Grove is ready for preload and IPC work.
4. Disable native vibrancy and use an opaque renderer surface when Reduce Transparency is enabled.
5. Use one semantic hover token across ordinary and sidebar ghost controls, but give that token light- and dark-specific values. On translucent surfaces, prefer a subtle theme-specific alpha overlay; identical CSS tokens express consistency even though backdrop compositing means the final pixels can differ.

### Frontend-first limitation without new IPC

There is no reliable renderer-only fallback that preserves explicit light/dark selection and native translucency in Grove's current window configuration. Until a narrow native theme API is allowed, Grove must choose between:

- following the system theme so the renderer and native material stay aligned, or
- retaining explicit renderer themes and accepting that the native sidebar can mismatch macOS.

Making mismatched modes correct requires main-process coordination: set `nativeTheme.themeSource` to the selected theme, or disable native vibrancy and provide an opaque renderer surface. Full translucent light/dark switching independent of macOS is not achievable correctly with CSS alone.
