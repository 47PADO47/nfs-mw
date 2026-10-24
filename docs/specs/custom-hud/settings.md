# Custom HUD: the `[hud]` settings table

Every player option of the Custom HUD, its type, range and default, the three presets, and how the table is read,
merged and written. Built in PR 2 (`crates/nfsmw/src/settings/hud.rs`, a `HudLayer` plus `from_layer`, following
`settings/xenon_effects.rs` and `settings/hd_content.rs`). The options page is PR 11a ([menus.md](menus.md)).
Index: [README.md](README.md).

## 1. The table

```toml
[hud]
enabled = true
preset = "balanced"
scale = 1.0
opacity = 1.0
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
hud_volume = 80
```

| Key | Type | Range or values | Default | Preset field | Applies to |
|---|---|---|---|---|---|
| `enabled` | bool | | `true` | | every look (the existing `hud` switch, section 4) |
| `preset` | enum | `minimal`, `balanced`, `advanced` | `balanced` | | Custom |
| `scale` | float | 0.75 to 1.5 | 1.0 | | Custom HUD |
| `opacity` | float | 0.3 to 1.0 | 1.0 | | Custom HUD |
| `text_size` | float | 0.8 to 1.5 | 1.0 | | Custom HUD and menus |
| `map_mode` | enum | `off`, `compact`, `centered`, `expanded` | `centered` | yes | Custom |
| `map_opacity` | float | 0.1 to 0.8 | 0.35 | | Custom |
| `map_zoom` | float | 0.5 to 2.0 | 1.0 | | Custom |
| `advanced_telemetry` | bool | | `false` | yes | Custom |
| `world_markers` | bool | | `true` | yes | Custom |
| `race_info` | bool | | `true` | yes | Custom |
| `pursuit_info` | bool | | `true` | yes | Custom |
| `free_roam_extras` | bool | | `false` | yes | Custom (cash, bounty, event distance) |
| `notifications` | enum | `all`, `important`, `off` | `all` | yes | Custom |
| `high_contrast` | bool | | `false` | | Custom HUD and menus |
| `palette` | enum | `default`, `deuteranopia`, `tritanopia` | `default` | | Custom HUD and menus |
| `motion_reduction` | bool | | `false` | | Custom HUD and menus |
| `camera_shake` | float | 0.0 to 1.0 | 1.0 | | every look (gameplay camera) |
| `fov_intensity` | float | 0.0 to 1.0 | 1.0 | | every look (gameplay camera) |
| `haptics` | bool | | `false` | | every look |
| `hud_volume` | integer percent | 0 to 100 | 80 | | every look (`Group::Hud`) |

`expanded` is accepted for `map_mode` but is a temporary view: the `MapExpand` action opens it and returns
([map.md](map.md)). A saved `expanded` loads as `centered`. **[ours]**

**Settings reused, not duplicated:**

| Need | Existing setting |
|---|---|
| speed units | `speed_units` (`metric`, `imperial`) |
| map orientation (rotating or north-up) | `minimap` (`fixed`, `rotating`, `off`), read as in [map.md](map.md) section 3 |
| the accent family | `ui_theme` (`bevy`, `remastered`) |
| police dispatch volume | `speech_volume` |
| the look itself | `ui_style` (`custom`) |

## 2. Presets

A preset only fills the fields marked "preset field" that the player has not set. A field set in any layer (file,
console, options page) wins over the preset. Changing the preset does not erase explicitly set fields; the options page
offers "reset to preset" (PR 11a), which clears them.

| Field | `minimal` | `balanced` (default) | `advanced` |
|---|---|---|---|
| `map_mode` | `compact` | `centered` | `centered` |
| `advanced_telemetry` | `false` | `false` | `true` |
| `world_markers` | `false` | `true` | `true` |
| `race_info` | `true` | `true` | `true` |
| `pursuit_info` | `true` | `true` | `true` |
| `free_roam_extras` | `false` | `false` | `true` |
| `notifications` | `important` | `all` | `all` |

The defaults in section 1 are the Balanced preset's. Minimal still shows speed, gear, nitrous, race position, the
pursuit status and every critical and high warning: the essential driving and safety information.

## 3. Reading, writing and applying

- **Layers:** the same as every setting (CLI over env over file over default, [ui-themes.md](../ui-themes.md) section
  11). The table has no new environment variable or command-line flag; the existing `NFSMW_HUD` and `--no-hud` keep
  driving `enabled`.
- **File:** the `[hud]` table. A value of the wrong kind is reported and ignored; a number outside its range is clamped
  and reported; an unknown key is reported. A bad key drops only that key, never the table. An old config without the
  table loads with the defaults.
- **Write:** through the existing per-key TOML merge (`settings/write.rs`), so writing one key keeps the player's other
  keys and comments.
- **Console:** `set hud.<key> <value>` and `get hud.<key>`, e.g. `set hud.map_mode compact`. `set hud off` keeps
  working (section 4).
- **Live:** every key applies on the next frame without a restart. `camera_shake` and `fov_intensity` are copied into
  `settings/world_tuning.rs`, where the running world reads them: `camera_shake` multiplies the shake of
  `viewer/camera/chase/shake.rs` and `viewer/camera/original/shake.rs`; `fov_intensity` multiplies the widening of
  `DynamicFov` (`viewer/camera/dynamic_fov.rs`). At 1.0 both are today's behaviour exactly.

## 4. The existing `hud` key

The config already has a top-level `hud = true | false` (draw the HUD while driving; `settings/file.rs`, `write.rs`,
`partial.rs`, `env.rs`). TOML cannot hold a key `hud` and a table `[hud]` at once, so PR 2 moves it into the table:

- The reader accepts both forms: a top-level boolean `hud` (legacy) is read as `[hud] enabled`. When a file somehow has
  neither, `enabled` defaults to `true`.
- The writer writes `[hud] enabled` and, when it writes the table into a file that still has the boolean, replaces the
  boolean with the table, carrying its value over.
- `NFSMW_HUD`, `--no-hud` and the console's `set hud off` keep their meaning and set `enabled`.
- An older build reading a new file sees a table where it expects a boolean; it reports that one key and keeps the
  others, as for any bad key.

Tests cover both forms, the rewrite of the boolean into the table, and that no other key moves. **[ours]**

## 5. Tests (PR 2)

- Parse every key at its limits and outside them (clamped); wrong kinds ignored; unknown keys reported.
- Defaults equal the Balanced preset; each preset fills only unset fields; an explicit field beats the preset.
- Round trip through `write::merge`: one key written keeps the rest of the file; the legacy boolean becomes the table.
- A bad key drops only that key.
- `set hud.*` and `get hud.*` in the console.
- `camera_shake` 0 removes the shake; 1 leaves it unchanged; `fov_intensity` likewise for the widening.
