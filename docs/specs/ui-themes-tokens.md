# UI themes: tokens

The contract for theme tokens and the `bb.ui` 1.1 `set_tokens`. It extends [ui-themes.md](ui-themes.md) section 15.

A theme declares its look once, as `blackbox_ui::Tokens` (`libs/blackbox-ui/src/tokens.rs`): `primary`, `secondary`,
`background`, `border` and `muted` colours, and a `radius` and `padding` in FEng units. `Theme::tokens()` returns them and
defaults to a neutral set.

- **Delivery.** `UiThemes` merges the active theme's tokens into the skin each component reads, under the `theme.*` keys
  (`token_keys`). A component names the key, never the theme, so one component draws in any palette. The merged skin is
  rebuilt only when the active theme changes (`UiThemes::refresh_skin`).
- **Custom HUD.** The radio card, speedometer, tachometer, gear and meters read `theme.*` for their colours. The radio
  card and the tachometer read `theme.padding` and `theme.radius`. The status colours (redline, shift light, the three
  meter fills) stay fixed.
- **Palettes.** `ui/theme/custom/palette.rs`: `bevy()` (the speedometer and panel colours) and `remastered()` (the gold
  accent of the remastered HUD). The settings choose one (`UiTheme`, row "HUD theme"); `palette::for_ui_theme` maps it.
  The palette applies to the token-driven HUD (`ThemeId::Custom`) only, so the Remastered HUD does not read it yet.
- **Mods.** `bb.ui` 1.1 `set_tokens`. A mod theme that declares tokens draws the token look for the slots it does not
  name (`ui/theme/modded` `ModHud`). See [api-ui.md](../modding/api-ui.md).
