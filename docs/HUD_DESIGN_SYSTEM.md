# HUD design system: Rockport / Signal System

The visual source of truth for the Custom HUD, as adapted to this project. It states the look and the rules; the
behaviour of each part is in [specs/custom-hud/](specs/custom-hud/README.md), the references in
[HUD_REFERENCE_BOARD.md](HUD_REFERENCE_BOARD.md), the user-facing options in [hud-settings.md](hud-settings.md).

## 1. Intent

A fast, understated street-racing HUD, for the **Custom** UI style (`ui_style = custom`):

- a **centred, translucent map**, drawn from the game's own map picture and icons, as the signature element;
- a wide, horizontal speed readout with a small gear and a restrained nitrous bar, never a round speedometer;
- layouts that follow the mode: quiet in free roam, precise in a race, urgent in a pursuit;
- dark translucent surfaces, thin lines, one accent.

It reads the game's real data only. A value the game does not compute is not drawn
([specs/custom-hud/not-shown.md](specs/custom-hud/not-shown.md)).

## 2. Colour

One graphite base and one accent. The tokens are the `custom.*` skin keys
(`ui/theme/custom/tokens/colours.rs`); alpha is applied in code.

| Token | Hex | Use |
|---|---|---|
| `text` | `#F4F6F8` | main values |
| `text_secondary` | `#B4BEC8` | supporting values |
| `surface` | `#10161D`, 0.60 | the translucent plate behind text |
| `surface_raised` | `#171F28`, 0.75 | urgent information, a selected row |
| `rule` | `#7C8995`, 0.35 | separators, empty tracks |
| `nav` (and the accent) | `#F4B942` | the route, the player's action, the active choice |
| `success` | `#7DCEA0` | escaped, confirmed |
| `caution` | `#E8873A` | a general warning |
| `danger` | `#F04452` | the police red, the red line |
| `critical` | `#FF5A55` | immediate danger, on a solid plate |

Deviations from the first draft of this page: `caution` is a deeper orange than the signal yellow, so a warning is not
read as the route; the police blue has no token, it is the cop icon's light bar. High contrast, the colour-vision
palettes and the opacity limits are in [specs/custom-hud/visual-identity.md](specs/custom-hud/visual-identity.md).

## 3. Type

The face is chosen by role (`numeric`, `label`, `label wide`) and resolved from the **fonts installed on the player's
system** (`ui/system_fonts/`), with the existing raster font as the fallback
([specs/custom-hud/typography.md](specs/custom-hud/typography.md)). No font is bundled: the owner decided against it
(ADR 0013, decision 2), and the repository's leak check refuses binary files. Rajdhani and IBM Plex are what the
roles would use if they are installed; shipping a font family with the game is a decision to take (it needs its
licence file and a change to the leak check).

| Role | Size at 1080p | Used for |
|---|---|---|
| Speed digits | 37 units, 83 px | the speed |
| Gear | 19 units, 43 px | the gear |
| Strip values | 16 units, 36 px | lap, distance, gap |
| Headings and the unit | 10 units, 22 px | `POS`, `LAP`, `KM/H` |

A unit is 2.25 px at 1080p. Numbers are tabular, so a value does not shift as it changes.

## 4. Shape and layout

- Squared plates with a thin top rule; no rounded cards, no bevels, no glow. Bars are flat.
- The HUD is laid out in a 16:9 **content box** inside a safe area, so 21:9 and 32:9 keep the groups together
  ([specs/custom-hud/layout.md](specs/custom-hud/layout.md)). Scale is 0.75 to 1.5.
- Free roam: the cluster bottom right, the map in the middle, notifications on the left. A race adds the strip at the
  top centre and the leaderboard top right. A pursuit adds the panel top right and the edge arrows.

## 5. The map

The map is the one element drawn over the middle of the view. Its modes and sizes (1080p):

| Mode | Overlay | Also |
|---|---|---|
| `off` | none | |
| `compact` | 340 px wide, faint | the turn-cue chip at the top centre |
| `centered` (default) | 500 px wide | |
| `expanded` | 710 px wide, more opaque | the whole route |

- **The picture.** The tiles of the game's minimap file, as the install has them (the stage's map, or a race's own
  map). The tiles are additive: black is transparent, so only roads and water show over the world. The overlay fades
  to nothing at its elliptical edge. It has no backdrop yet, so its thin lines are faint over a bright street.
- **The icons.** The checkpoints, racers, race starts, shops, safe houses, cops and the helicopter, and the player's
  arrow, are the game's own icons (`HUD_SingleRace.fng`). A kind with no game icon (a roadblock, the last-known place)
  is a thin shape. The route is signal yellow.
- **Orientation.** Heading-up or north-up, by the `minimap` setting; the arrow stays upright when the map turns.
- **Fair knowledge.** Police are marked only as the player can know them
  ([specs/custom-hud/fair-knowledge.md](specs/custom-hud/fair-knowledge.md)).
- **Without tiles.** An install with no minimap files draws the road network as strokes instead.

Details: [specs/custom-hud/map.md](specs/custom-hud/map.md) and
[specs/custom-hud-map-impl.md](specs/custom-hud-map-impl.md).

## 6. Motion and notifications

Motion tokens: 100 to 140 ms for small changes, 160 to 240 ms for the map and components, 260 to 360 ms for mode
changes (`ui/theme/custom/tokens/motion.rs`). Ease-out in, ease-in out, no bounce. One major notification at a time,
by priority; the rest queue briefly or drop ([specs/custom-hud/notifications.md](specs/custom-hud/notifications.md)).
`motion_reduction` removes movement and holds the cop's light bar on one colour.

## 7. Accessibility and options

Every option the design asks for is a `[custom_hud]` key ([hud-settings.md](hud-settings.md)): `scale`, `text_size`,
`opacity`, `map_mode`, `map_opacity`, `map_zoom`, `high_contrast`, `palette`, `motion_reduction`, and the switches for
`world_markers`, `race_info`, `pursuit_info`, `advanced_telemetry`, `free_roam_extras` and `notifications`. The map's
orientation is `minimap`, the unit `speed_units`. No state is carried by colour alone.

## 8. Previewing

`nfsmw play --ui-style custom --hud-scenario <name> --screenshot out.png --muted` renders a fixed state offscreen
([specs/custom-hud/dev-scenarios.md](specs/custom-hud/dev-scenarios.md)); `tools/hud_shots.ps1` renders the matrix:
free roam, race, pursuit, search, cooldown, escape, and the map at `map-compact`, `map-centered` and `map-expanded`.
