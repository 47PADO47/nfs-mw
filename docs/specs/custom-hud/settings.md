# Custom HUD: the `[custom_hud]` settings table

The player options of the Custom HUD: the table, the three presets, and how they are read, merged and applied. Built
in PR 2 (`crates/nfsmw/src/settings/hud/`: `mod.rs` holds `HudSettings`, `HudLayer` and the ranges, `kinds.rs` the
choices, `preset.rs` the presets, `config.rs` the table, `console.rs` the console keys). The resolved options are
`Settings.hud_options`. The user-facing reference, with every key, its values and default, is
[hud-settings.md](../../hud-settings.md); this page gives the design around it. The options page is PR 11a
([menus.md](menus.md)). Index: [README.md](README.md).

## 1. The table

```toml
[custom_hud]
preset = "balanced"
scale = 1.0
opacity = 0.9
text_size = 1.0
map_mode = "centered"
map_opacity = 0.35
map_zoom = 1.0
advanced_telemetry = false
world_markers = true
race_info = true
pursuit_info = true
free_roam_extras = false
notifications = "all"
high_contrast = false
palette = "default"
motion_reduction = false
camera_shake = 1.0
fov_intensity = 1.0
haptics = false
hud_volume = 0.8
```

Ranges and defaults, from the constants of `settings/hud/mod.rs`:

| Key | Type | Range or values | Default | Preset covers it | Applies to |
|---|---|---|---|---|---|
| `preset` | choice | `minimal`, `balanced`, `advanced` | `balanced` | | Custom |
| `scale` | number | 0.75 to 1.5 | 1.0 | | Custom HUD |
| `opacity` | number | 0.2 to 1.0 | 0.9 | | Custom HUD |
| `text_size` | number | 0.8 to 1.5 | 1.0 | | Custom HUD and menus |
| `map_mode` | choice | `off`, `compact`, `centered`, `expanded` | `centered` | yes | Custom |
| `map_opacity` | number | 0.1 to 0.9 | 0.35 | | Custom |
| `map_zoom` | number | 0.5 to 2.0 | 1.0 | | Custom |
| `advanced_telemetry` | bool | | `false` | yes | Custom |
| `world_markers` | bool | | `true` | | Custom |
| `race_info` | bool | | `true` | yes | Custom |
| `pursuit_info` | bool | | `true` | yes | Custom |
| `free_roam_extras` | bool | | `false` | yes | Custom (cash, bounty, event distance) |
| `notifications` | choice | `all`, `important`, `off` | `all` | yes | Custom |
| `high_contrast` | bool | | `false` | | Custom HUD and menus |
| `palette` | choice | `default`, `deuteranopia`, `tritanopia` | `default` | | Custom HUD and menus |
| `motion_reduction` | bool | | `false` | | Custom HUD and menus |
| `camera_shake` | number | 0.0 to 1.0 | 1.0 | | every look (gameplay cameras) |
| `fov_intensity` | number | 0.0 to 1.0 | 1.0 | | every look (Dynamic FOV) |
| `haptics` | bool | | `false` | | every look |
| `hud_volume` | number | 0.0 to 1.0 | 0.8 | | every look (`Group::Hud`) |

`expanded` is an ordinary value of `map_mode`: a file that says `expanded` starts with the Expanded view. The
`MapExpand` action toggles it on demand and returns to the mode before it, or to `centered` when it started expanded
([map.md](map.md)). **[ours]**

**Settings reused, not duplicated:**

| Need | Existing setting |
|---|---|
| show the HUD at all | the top-level `hud = true` or `false` (with `NFSMW_HUD`, `--no-hud`, `set hud off`), unchanged and separate from the table |
| speed units | `speed_units` (`metric`, `imperial`) |
| map orientation (rotating or north-up) | `minimap` (`fixed`, `rotating`, `off`), read as in [map.md](map.md) section 3 |
| the accent family | `ui_theme` (`bevy`, `remastered`) |
| police dispatch volume | `speech_volume` |
| the look itself | `ui_style` (`custom`) |

## 2. Presets

A preset's layer sits under the player's own values (`HudSettings::from_layer`), so it fills only what the player has
not set, and an explicit value always wins. Every preset covers the same six options (`settings/hud/preset.rs`):

| Option | `minimal` | `balanced` (default) | `advanced` |
|---|---|---|---|
| `advanced_telemetry` | `false` | `false` | `true` |
| `free_roam_extras` | `false` | `false` | `true` |
| `race_info` | `true` | `true` | `true` |
| `pursuit_info` | `true` | `true` | `true` |
| `notifications` | `important` | `all` | `all` |
| `map_mode` | `compact` | `centered` | `centered` |

Balanced equals the defaults. Picking a preset at run time (`HudSettings::apply_preset`, the console's
`set custom_hud.preset`) sets those six options to the preset's values at once and leaves every other option as it
is. Minimal still shows speed, gear, nitrous, race position, the pursuit status and every critical and high warning:
the essential driving and safety information. World markers are not a preset option; they stay as the player set
them.

## 3. Reading, writing and applying

- **Layers:** file, then console. The table has no environment variable or command-line flag (`settings/env.rs` leaves
  it at its default).
- **File:** each key is read on its own (`settings/hud/config.rs`). A value of the wrong kind or an unknown name is
  reported in the log and only that key is ignored; a number outside its range is clamped by
  `HudSettings::from_layer`. A config file without the table changes nothing.
- **Write:** `config::merge_into` puts the changed keys into the `[custom_hud]` table through the existing per-key
  merge (`settings/write.rs`), keeping the rest of the file. A non-table value under the key is left alone.
- **Console:** `get custom_hud.<key>` and `set custom_hud.<key> <value>`, for example
  `set custom_hud.map_mode compact`. A switch with no value flips (`set custom_hud.haptics`). Console changes last for
  the run.
- **Live:** the options apply on the next frame without a restart. `camera_shake` and `fov_intensity` are copied into
  `settings/world_tuning.rs`, where the running world reads them:
  - `camera_shake` multiplies the contact shake of every camera (`viewer/camera/chase/shake.rs`,
    `viewer/camera/original/shake.rs`) and the original cameras' terrain and vehicle noise
    (`viewer/camera/original/noise.rs`); 0 is a steady camera;
  - `fov_intensity` multiplies how far the Dynamic FOV camera widens with speed (`viewer/camera/dynamic_fov.rs`);
    0 keeps its rest FOV.

  At 1.0 both leave the camera exactly as before.

## 4. Tests (PR 2)

`settings/hud/tests.rs`, `config_tests.rs` and `console_tests.rs`: parsing at and beyond the limits, wrong kinds ignored
key by key, the defaults equal to Balanced, each preset filling only unset options, an explicit value beating the
preset, `apply_preset`, the round trip through the merge, and the console's `get` and `set`; the camera multipliers in
the shake, noise and FOV tests.
