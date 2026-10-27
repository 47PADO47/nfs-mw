# UI themes: tokens

The contract for theme tokens and the `bb.ui` 1.1 `set_tokens`. It extends [ui-themes.md](ui-themes.md) section 15.

A theme declares its look once, as `blackbox_ui::Tokens` (`libs/blackbox-ui/src/tokens.rs`): `primary`, `secondary`,
`background`, `border` and `muted` colours, and a `radius` and `padding` in FEng units. `Theme::tokens()` returns them and
defaults to a neutral set.

- **Delivery.** `UiThemes` merges the active theme's tokens into the skin each component reads, under the `theme.*` keys
  (`token_keys`). A component names the key, never the theme, so one component draws in any palette. The merged skin is
  rebuilt only when the active theme changes (`UiThemes::refresh_skin`).
- **Custom HUD.** The radio card reads `theme.*` for its colours and `theme.padding`. The Custom HUD 2026 (ADR 0013)
  has its own tokens under `custom.*` (`ui/theme/custom/tokens/`): `text`, `text_secondary`, `surface`,
  `surface_raised`, `rule`, `accent`, `nav`, `success`, `caution`, `danger`, `critical`, `critical_text`, `outline`
  (high contrast only) and the three fonts `custom.font.numeric`, `custom.font.label` and `custom.font.label_wide`.
  `UiThemes::refresh_skin` merges them after the theme's tokens, for the `[custom_hud]` palette and high contrast; the
  `accent` is the active tokens' `secondary`, so `ui_theme` keeps choosing the accent family. Opacity is applied by each
  component, not in the skin.
- **Palettes.** `ui/theme/custom/palette.rs`: `bevy()` (the speedometer and panel colours) and `remastered()` (the gold
  accent of the remastered HUD). The settings choose one (`UiTheme`, row "HUD theme"); `palette::for_ui_theme` maps it.
  The palette applies to the Custom and the Remastered HUDs: `UiThemes::refresh_skin` (`ui/theme/mod.rs`) takes
  `palette::for_ui_theme` as the tokens when the active theme is `ThemeId::Custom` or `ThemeId::Remastered`, and merges
  `remastered::palette_skin` over them, so the Remastered dial's redline tint, gear panel, font sizes and menu colours
  follow the chosen palette. Every other theme brings its own tokens.
- **The Custom rewrite.** The Custom HUD of [ADR 0013](../decisions/0013-custom-hud.md) keeps `UiTheme` as its accent
  family and adds its own `custom.*` tokens ([custom-hud/visual-identity.md](custom-hud/visual-identity.md)).
- **Mods.** `bb.ui` 1.1 `set_tokens`. A mod theme that declares tokens draws the token look for the slots it does not
  name (`ui/theme/modded` `ModHud`). See [api-ui.md](../modding/api-ui.md).

## Colour roles

Beyond the five core colours a theme can declare **colour roles** (`blackbox_ui::Roles`, a field of `Tokens`;
`libs/blackbox-ui/src/tokens/roles.rs`). They are generic: nothing in `blackbox-ui` names the game.

| Role | Skin key | Meaning | Derived from the core colours when unset |
|---|---|---|---|
| `line` | `theme.line` | the main linework of a plan or map (main roads) | `primary` |
| `line_minor` | `theme.line_minor` | lesser linework (side roads) | `muted` |
| `line_faint` | `theme.line_faint` | the faintest linework (paths) | `border` |
| `guide` | `theme.guide` | the path that guides the user over the linework (a route) | `secondary` |
| `success` | `theme.success` | a good or safe state | green `[125, 206, 160]` |
| `caution` | `theme.caution` | a warning | orange `[232, 135, 58]` |
| `danger` | `theme.danger` | a threat | red `[240, 68, 82]` |
| `info` | `theme.info` | a neutral second signal | blue `[90, 150, 255]` |
| `scrim` | `theme.scrim` | dims everything behind a modal layer | `background` at 40% brightness, alpha 165 |
| `overlay` | `theme.overlay` | the strong plate that holds content over the scrim | `background`, alpha at least 224 |

- **Resolution.** Each role is `Option<[u8; 4]>`; `Tokens::skin()` writes all ten to the skin, the declared colour or the
  derived one, so a component can always read them. `Tokens::colour_in(skin, key)` reads a colour with the neutral
  default's value when a skin predates the key. Adding the field is backwards compatible: `Roles::default()` is all unset,
  and a theme written before the roles existed gets a set derived from its own palette.
- **Built-in themes.** Original declares none (the neutral defaults derive). The Custom palettes (`custom/palette.rs`, the
  Bevy and Remastered `ui_theme`) declare them from the Custom brief's colours, with the signal yellow (Bevy) or the gold
  (Remastered) as `guide`, and a graphite scrim and overlay.
- **Mod themes.** The `bb.ui` ABI carries only the five core colours, so a mod theme's roles all derive from them
  (`modding/backends/ui.rs`), and its map, route and backdrop match its palette. Declaring roles from a mod needs a new
  ABI table version (`set_tokens` extended in `bb.ui` 1.2); that is not done.
- **Accessibility.** `UiThemes::refresh_skin` passes the roles through `custom/tokens/roles.rs` for the player's
  `[custom_hud]` look: a colour-vision palette replaces `guide`, `success`, `caution` and `danger` with its own hues, and
  high contrast whitens the linework, darkens `scrim` and `overlay` and brightens the meaning hues. State is never carried
  by colour alone.
- **Users.** The Custom map and the full-screen map ([custom-hud/map.md](custom-hud/map.md) section 4c) draw only in these
  keys and the core `theme.primary` and `theme.secondary`; a test scans their sources for fixed colours.
