# HUD settings

The `[custom_hud]` table of the config file holds the options of the Custom HUD (`ui_style = "custom"`): a preset, the
display size, the map, which information shows, notifications, accessibility, the camera and feedback. Code:
`crates/nfsmw/src/settings/hud/`. The options page in the menus comes later; until then the table is set in the config
file or from the console.

```toml
[custom_hud]
preset = "balanced"        # minimal, balanced or advanced
scale = 1.0                # 0.75 to 1.5
map_mode = "centered"      # off, compact, centered or expanded
camera_shake = 0.5         # 0 (none) to 1 (as it is)
```

## Keys and defaults

| Key | Values | Default | What it does |
|---|---|---|---|
| `preset` | `minimal`, `balanced`, `advanced` | `balanced` | Fills the options the player has not set (below). |
| `scale` | 0.75 to 1.5 | 1.0 | HUD scale. |
| `opacity` | 0.2 to 1 | 0.9 | HUD opacity. |
| `text_size` | 0.8 to 1.5 | 1.0 | Text size. |
| `map_mode` | `off`, `compact`, `centered`, `expanded` | `centered` | How the road map shows. |
| `map_opacity` | 0.1 to 0.9 | 0.35 | The map's opacity. |
| `map_zoom` | 0.5 to 2 | 1.0 | The map's zoom. |
| `advanced_telemetry` | `true`, `false` | `false` | Extra telemetry, only values the game computes. |
| `world_markers` | `true`, `false` | `true` | Markers over the road. |
| `race_info` | `true`, `false` | `true` | The race panel: place, lap, checkpoints, gaps. |
| `pursuit_info` | `true`, `false` | `true` | The pursuit panel: heat, units, bounty, cooldown. |
| `free_roam_extras` | `true`, `false` | `false` | Cash, bounty and the distance to the next event in free roam. |
| `notifications` | `all`, `important`, `off` | `all` | Which notifications show. |
| `high_contrast` | `true`, `false` | `false` | Stronger plates and outlines. |
| `palette` | `default`, `deuteranopia`, `tritanopia` | `default` | Colours for colour vision deficiencies. |
| `motion_reduction` | `true`, `false` | `false` | Calmer HUD animation. |
| `camera_shake` | 0 to 1 | 1.0 | Multiplies the contact shake of every camera and the original cameras' terrain and vehicle noise; 0 is a steady camera. |
| `fov_intensity` | 0 to 1 | 1.0 | Multiplies how far the Dynamic FOV camera widens with speed; 0 keeps its rest FOV. |
| `haptics` | `true`, `false` | `true` | Controller rumble for HUD events. |
| `hud_volume` | 0 to 1 | 0.8 | The volume of the HUD's own sounds. |

A number outside its range is clamped. A value of the wrong kind (a string for a number, an unknown name) is reported
in the log and only that key is ignored; the others still apply. A config file without the table changes nothing.

The map's orientation is not a key of the table: it is the existing `minimap` setting (`fixed`, `rotating`, `off`).
The speed unit is the existing `speed_units` setting.

## Presets

A preset fills only the options the player has not set: its values sit under the player's own, so an explicit value
always wins. Every preset covers the same six options:

| Option | Minimal | Balanced | Advanced |
|---|---|---|---|
| `advanced_telemetry` | `false` | `false` | `true` |
| `free_roam_extras` | `false` | `false` | `true` |
| `race_info` | `true` | `true` | `true` |
| `pursuit_info` | `true` | `true` | `true` |
| `notifications` | `important` | `all` | `all` |
| `map_mode` | `compact` | `centered` | `centered` |

Balanced is the defaults. Picking a preset from the console applies its values to those six options at once, and
leaves the other options as they are.

## The `hud` switch

The top-level `hud = false` still hides the HUD in every style. It is a separate key from the `[custom_hud]` table,
which holds only the Custom style's options.

## Console

`get custom_hud.<key>` and `set custom_hud.<key> <value>`, for example `set custom_hud.map_mode compact` or
`set custom_hud.camera_shake 0`. A switch with no value flips (`set custom_hud.haptics`). Console changes last for the
run. `get` lists every key. The camera multipliers apply on the next frame.
