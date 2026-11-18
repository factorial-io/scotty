# Factorial design system (vendored)

`tokens.css`, `components.css` and `rebrush.css` are copies from the
`factorial-design` skill of the Factorial Claude plugin, version 0.19.1
(copied 2026-11-18). They are not a package dependency.

Local changes, keep them when refreshing:

- `tokens.css`: the `@font-face` rules for SuisseIntl and SuisseIntlMono are
  removed. SuisseIntl is a licensed font and is not shipped in this public
  repository. The font stacks keep `SuisseIntl` / `SuisseIntlMono` first: the
  layout adds `@font-face` rules for them only when the server setting
  `api.suisse_fonts_url` is set (`src/lib/fonts.ts`). Otherwise the stacks fall
  through to Inter (`@fontsource/inter`) and JetBrains Mono
  (`@fontsource/jetbrains-mono`), both bundled. The `@font-face` comment lines
  in `components.css` are removed too.

To refresh, diff each file against the skill's `styles/` folder, copy what
changed, repeat the font removals, and check every screen in light
and dark mode.
