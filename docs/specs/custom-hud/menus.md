# Custom HUD: the Custom menus

Custom covers the menus as well as the HUD (the owner's decision, [ADR 0013](../../decisions/0013-custom-hud.md)
decision 1). The menus are written from scratch with the Custom tokens and the OS fonts, inspired by Remastered's
vertical lists but sharing no code with them. Built in PR 11a and PR 11b (`crates/nfsmw/src/ui/theme/custom/menu/`).
Index: [README.md](README.md).

## 1. How the frontend feeds them

Today `frontend/logic.rs::Cx::vertical_look()` decides whether the frontend publishes its entries to the
`MenuListCell` (`ui/theme/menu_list.rs`) for a list look: it is true only when `ui_style` is Remastered and no mod theme
took over. It has 26 call sites. PR 11a generalises it to the **list looks**, Remastered **or** Custom, so the frontend
publishes the same `MenuListCell` entries for Custom. The FEng screens keep running underneath, as they do for
Remastered; only the drawing changes. The rename keeps one predicate, not two copies of the call sites.

## 2. Look

- **Layout:** a vertical list, left-aligned inside the content box ([layout.md](layout.md)), starting at 30% of the box
  height. The title (label-wide key, size 22) sits above it; a description line for the selected row sits at the
  bottom-left; the button prompts (from the live bindings, `ui/prompts.rs`) at the bottom-right.
- **Rows:** label key, size 16, sentence case, `text_secondary` when not selected. The selected row is `text`, on a
  `surface_raised` plate, with a 3-unit `accent` bar on its left. A disabled row is `text_secondary` at 0.5 alpha with
  its reason in the description line.
- **Pictures:** an entry that has a picture (the icon menus' own textures) shows it before its label in every HUD theme,
  not only where a palette asks for pictures; a list in which some entry has one keeps one label column, and an entry
  without a picture draws the HUD icon set's fallback pictogram (`menu/picture.rs`).
- **Values** (option rows): right-aligned in the numeric or label key; a cycling value has small chevrons either side;
  a slider is a 10-segment bar like the HUD's.
- **Motion:** the selection moves with a 150 ms ease; screens crossfade over `mode_change`. Motion reduction: no
  movement, instant selection, 150 ms crossfades.
- **Scale:** menus have their own scale (1.0) and take `[custom_hud] text_size`, `high_contrast` and `palette`, not
  `[custom_hud] scale` ([visual-identity.md](visual-identity.md) section 6).
- **Backdrop:** the existing menu backdrop ([menu-backdrop.md](../menu-backdrop.md)) with a `surface` scrim behind the
  list for contrast.

## 3. Screens

| Slot (id) | Custom component | PR |
|---|---|---|
| `MainMenu` (20) | `MainMenu`: the list of the main entries | 11a |
| `Categories` (21) | `Categories`: the option categories, in and out of the pause menu | 11a |
| `PauseMenu` (22) | `PauseMenu` | 11a |
| `SafeHouseMenu` (25) | `SafeHouseMenu` | 11a |
| `OptionTable` (23) | `OptionTable`: rows with values | 11a |
| `Dialog` (27) | `Dialog`: a centred plate with the message and its buttons | 11a |
| `ControlsList` (28) | `ControlsList`: actions and their live bindings | 11b |
| `Jukebox` (29) | `Jukebox` | 11b |
| `Credits` (30) | `Credits` | 11b |
| `Blacklist` (40) | `Blacklist` | 11b |
| `LanMenu` (31), `LanHud` (32) | `LanMenu`, `LanHud` | 11b |
| `WorldMapChrome` (26) | a Custom full-screen map: the game's world-map picture and icon textures (as Remastered draws them) with Custom chrome and the GPS route | 11b |
| `RadioCard` (10) | the radio card, rewritten in the new style | 11b |

`TrackSelect` (24) and the career screens other than the Blacklist (41 to 45) keep falling back to Original until a
later change claims them. With PR 11b the old Custom `map_legend.rs` (the legend over the recoloured FEng world map) is
retired.

## 4. The HUD options page

A new **HUD** category in the options (PR 11a), with one row per `[custom_hud]` key ([settings.md](settings.md)) and the reused
settings, in this order:

| Row | Setting |
|---|---|
| Preset | `preset`; picking one applies its six options at once (`HudSettings::apply_preset`) |
| HUD scale | `scale` |
| HUD opacity | `opacity` |
| Text size | `text_size` |
| Map mode | `map_mode` (`off`, `compact`, `centered`) |
| Map opacity | `map_opacity` |
| Map zoom | `map_zoom` |
| Map orientation | `minimap` (`fixed` shown as North up, `rotating` as Heading up) |
| Speed units | `speed_units` |
| Advanced telemetry | `advanced_telemetry` |
| World markers | `world_markers` |
| Race information | `race_info` |
| Pursuit information | `pursuit_info` |
| Free-roam extras | `free_roam_extras` |
| Notifications | `notifications` |
| High contrast | `high_contrast` |
| Colour palette | `palette` |
| Reduce motion | `motion_reduction` |
| Camera shake | `camera_shake` |
| Speed FOV intensity | `fov_intensity` |
| Controller vibration | `haptics` |
| HUD volume | `hud_volume` |

- Every row applies live through the existing options path (`frontend/options.rs`, `options_rows.rs`).
- A **preview line** at the bottom summarises the result in words, for example "Centered map, balanced information,
  high contrast". It updates as rows change.
- The page shows in every style, because the camera, haptics and volume rows apply to every look; the Custom-only rows
  say so in their description when another style is active.
- The speech volume row elsewhere is relabelled `Police dispatch volume` ([feedback.md](feedback.md) section 2.1).
- `view-screen … --options --category hud` renders the page offscreen for checks.

## 5. Tests (PR 11a, PR 11b)

- `vertical_look` (renamed) is true for Remastered and Custom without a mod theme, false otherwise; the 26 call sites
  still publish for Remastered exactly as before.
- Each Custom menu component builds from a fixed `MenuListCell` with golden node counts; the selected row's plate and
  bar; a disabled row.
- The HUD rows cycle and clamp their values and write the live settings.
