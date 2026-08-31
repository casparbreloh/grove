# Theme color foundations

## Recommendation

Keep Grove's main canvas neutral and let each named theme add only a quiet cast to the sidebar. Across themes, keep the sidebar's OKLCH lightness and chroma fixed and vary only hue. Let hue become obvious in compact interactive roles—selection, focus, and the theme icon—rather than across a large high-chroma surface.

This follows the common pattern across mature design systems: neutral grays establish the application frame, nearby surface tones create depth, and color is reserved for meaning and interaction.

## What the design systems agree on

- **Use neutrals as the foundation.** Spectrum uses fully desaturated grays so they coexist with any hue and uses nearby gray background layers for framing and depth. Carbon likewise makes neutral gray dominant, with subtle value shifts organizing distinct zones. [Spectrum color system](https://spectrum.adobe.com/page/color-system/), [Carbon color overview](https://carbondesignsystem.com/elements/color/overview/)
- **Keep adjacent app surfaces in the same quiet family.** Radix assigns steps 1–2 to app, canvas, card, and sidebar backgrounds. Carbon builds depth by alternating close neutral layers in light themes and making each elevated layer one step lighter in dark themes. Apple uses the same dark-mode direction: dim base surfaces recede and brighter elevated surfaces advance. [Radix scale use cases](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale), [Carbon color overview](https://carbondesignsystem.com/elements/color/overview/), [Apple Dark Mode](https://developer.apple.com/design/human-interface-guidelines/dark-mode)
- **Tinted neutrals can harmonize, but saturation is risky at application scale.** Radix says pure gray works with every hue and that a gray tinted toward the accent can create a more harmonious feel. It specifically warns that saturated gray backgrounds, especially in dark mode, can clash with colorful UI. [Radix palette composition](https://www.radix-ui.com/colors/docs/palette-composition/composing-a-palette)
- **Assign color by role, not raw swatch.** Primer separates base colors from mode-aware functional roles for backgrounds, borders, text, and interaction, and says base tokens should not be used directly. Carbon's themes preserve token roles while changing values. Grove's existing semantic tokens are the right abstraction. [Primer color usage](https://primer.style/product/getting-started/foundations/color-usage/), [Carbon themes](https://carbondesignsystem.com/elements/themes/overview/)
- **Use stronger color for interaction, not ambient framing.** Radix reserves steps 3–5 for component rest, hover, and selected fills; 6–8 for borders and focus; and its highest-chroma step for solid emphasis. Primer similarly distinguishes muted semantic backgrounds from emphasis colors. [Radix scale use cases](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale), [Primer color usage](https://primer.style/product/getting-started/foundations/color-usage/)
- **Dark mode is not a simple inversion.** Apple recommends appearance-specific semantic colors and notes that colors can seem brighter and more saturated in dark surroundings. Spectrum deliberately tunes dark-theme contrast independently from light theme. [Apple color](https://developer.apple.com/design/human-interface-guidelines/color), [Spectrum color system](https://spectrum.adobe.com/page/color-system/)
- **Perceptual color spaces help keep themes equally weighted.** CSS Color 4 defines OKLCH as perceptual lightness, chroma (distance from neutral gray), and hue. Stripe's first-party account describes using a perceptually uniform color space and a shared lightness curve to give different hues consistent visual weight. [CSS Color 4: Oklab and OKLCH](https://www.w3.org/TR/css-color-4/#ok-lab), [Stripe: Designing accessible color systems](https://stripe.com/blog/accessible-color-systems)

## Grove starting targets

The values below are a synthesis to test in Grove, not thresholds prescribed by the sources. `H` is the named theme's hue. The low chroma is intentional: at large surface area, even a small OKLCH chroma is legible.

```css
/* Light */
--background: oklch(0.98 0 0);
--sidebar: oklch(0.96 0.008 H);
--border: oklch(0.89 0.008 H);

/* Dark */
--background: oklch(0.22 0 0);
--sidebar: oklch(0.24 0.01 H);
--border: oklch(0.32 0.01 H);
```

Practical guardrails:

- Keep canvas/sidebar lightness about `0.015–0.025` apart. In light mode the sidebar should be slightly darker; in dark mode, slightly lighter. If the region needs more definition, strengthen the border before increasing surface chroma.
- Keep ambient surface chroma around `0.006–0.012`. Use `C: 0` for a genuinely neutral theme. Avoid the previous light sidebar `L: 0.91`: against Grove's `L: 0.98` canvas, that reads as a separate colored panel rather than a related surface.
- Use roughly `C: 0.015–0.03` for soft hover/selection fills, then reserve higher chroma for focus, selected indicators, theme icons, links, and semantic status. Keep normal sidebar text neutral.
- Preserve the same `L` and `C` values for Ocean, Sage, Sand, Clay, and Lilac; change only `H`. This makes theme choice about character rather than changing hierarchy or contrast.
- Prefer opaque colors. Apple's material guidance confirms that true glass derives its appearance from background blending; without external color bleed, the useful part to reproduce is its close tonal hierarchy, not translucency. [Apple materials](https://developer.apple.com/design/human-interface-guidelines/materials)

## Accessibility checks

OKLCH distance does not establish WCAG contrast. Test the rendered token pairs in both modes:

- Ordinary text needs at least `4.5:1`; large text needs `3:1`. [WCAG 2.2 text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html)
- Visual information required to identify a control or its state—including a focus indicator—needs `3:1` against adjacent colors. A merely decorative sidebar separator does not need to carry that burden. [WCAG 2.2 non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html)
- Do not use theme color as the only indication of state. Preserve shape, text, icon, or other non-color cues. [WCAG 2.2 use of color](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html)

## Suggested implementation order

1. Restore the pre-experiment layout and opaque window/sidebar behavior.
2. Fix the neutral canvas, content surface, and border hierarchy in light and dark mode.
3. Apply each theme hue to the same low-chroma sidebar recipe.
4. Add hue more clearly only to selection, focus, and the theme icon.
5. Compare all themes side by side in both appearances, then run contrast checks on text, controls, selected states, and focus rings.
