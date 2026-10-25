# Custom HUD: the centred translucent map

The signature element: the game's own map picture drawn over the middle of the 3D view, translucent so the player sees
both the map and the world, with the game's own icons for the places and the cars. It is not a corner minimap. Built
in PR 7a (`libs/blackbox-ui` `Polyline`) and PR 7b (`crates/nfsmw/src/ui/theme/custom/map/`), and moved onto the
game's picture and icons afterwards (section 4). Index: [README.md](README.md).

## 1. Modes

`[custom_hud] map_mode = off | compact | centered | expanded`, default **`centered`** (the owner's decision,
[ADR 0013](../../decisions/0013-custom-hud.md) decision 3). Changes apply on the next frame, with no restart.

| Mode | What is drawn |
|---|---|
| `off` | No map and no turn cues. Event feedback (notifications, world markers) still shows. |
| `compact` | A small, fainter overlay (340 px wide at 1080p) and a **turn-cue chip** under the race strip (top-centre): an arrow for the next turn and its distance. The chip is hidden when there is no route. |
| `centered` | The player-centred overlay in the middle of the box, clipped to a soft ellipse (500 px wide), at `map_opacity`. |
| `expanded` | A larger, more opaque overlay (710 px wide) that shows farther, with the paths and the whole route. Opened on demand. |

**Turn-cue chip** (Compact): the arrow is one of straight, slight left or right, left or right, sharp left or right,
U-turn, from `next_turn` (PR 3b: the direction and the distance, derived from the junction angles on the GPS path). The
distance is rounded to 10 m (metric) or 50 ft below 1000 ft and 0.1 mi above (imperial), following `speed_units`.

**Expanded** is the on-demand view. The `MapExpand` action toggles it and returns to the previous mode; `MapMode`
from Expanded also returns to the previous mode. `expanded` is also a value of `map_mode`, so a config can start with
it; the way back is then `centered` ([settings.md](settings.md)). Its road opacity is
`max(map_opacity, min(map_opacity + 0.25, 0.85))`.

## 2. Geometry and placement

- The overlay's centre is the centre of the content box. Its clip is an ellipse with a horizontal radius of 15.5% of
  the box height and a vertical radius of 11.5% (Compact), 23% and 17% (Centered), or 33% and 24.5% (Expanded): 340,
  500 and 710 px wide at 1080p, the widths of the design system ([../../HUD_DESIGN_SYSTEM.md](../../HUD_DESIGN_SYSTEM.md)).
  The last 12% of the radius fades the picture to zero (soft edge). **[ours]**
- The view radius in the world is `base * span / map_zoom` with `base` = 200 m (Compact), 300 m (Centered) or 600 m
  (Expanded), and `span` = 2.4 when the game's picture is drawn (it is 6.5 m to a texel, so it needs a wider view than
  vector roads to stay crisp; about the scale the original minimap shows) and 1 for the roads.
  `[custom_hud] map_zoom` is 0.5 to 2.0, default 1.0. **[ours]**
- The player is at the centre. The overlay never captures input and never moves the camera.

## 3. Orientation

The orientation **reuses the existing `minimap` setting** (`MinimapMode`, `settings/minimap.rs`), so there is one
orientation switch for every HUD look:

| `minimap` | Custom map |
|---|---|
| `fixed` (default) | north-up; the player arrow turns |
| `rotating` | heading-up; the map turns under a fixed arrow |
| `off` | north-up (the map's visibility is `map_mode` alone) |

The `MapOrientation` action flips `minimap` between `fixed` and `rotating`; from `off` it sets `rotating`. The change
is the same setting Original and Remastered read, saved through the existing write path. **[ours]**

## 4. Layers

Back to front, all clipped to the ellipse:

1. **The game's map picture**, at `map_opacity`: the tiles of the minimap file the HUD shows (the stage's map, a
   race's own map), cut into cells that turn and scale with the view and fade at the ellipse's edge (section 4a). Under
   it, a faint wash of the `surface` colour (alpha 0.16, 0.4 with high contrast) keeps it readable over a bright street.
   An install with no minimap tiles draws the roads instead: main roads two units wide, side roads one unit, with a
   dark under-stroke one unit wider under high contrast.
2. **Routes:** the GPS route (`Gps` path, PR 3b) in `nav`, three units; in a race the race route in `accent`, three
   units, with the stretch already driven at half alpha.
3. **Places:** checkpoints (the next one emphasised), discovered events, safe houses and shops from the existing blip
   data (`HudState::blips`, `hud/blip_style.rs`), drawn as the game's own icons (section 4a).
4. **Pursuit intel** under the fair-knowledge rules ([fair-knowledge.md](fair-knowledge.md)): sighted cops and the
   helicopter as solid marks, last-known positions as hollow marks fading with their age.
5. **The player arrow**, the game's own arrow, at full HUD opacity, always on top.

### 4a. The picture and the icons

- **The picture.** `hud::MapPicture` (the HUD plugin builds one when the minimap file changes) holds the 64 tile
  texture keys and the calibration ([../../formats/minimap.md](../../formats/minimap.md)). The map cuts the picture
  into blocks of an eighth of a tile (104 m in the city): a block well inside the ellipse is one quad, one the soft edge
  crosses is cut into cells about 14 units wide, each tinted by the edge's fade at its middle. The cells turn with the
  heading-up view. The tiles are additive textures, so black is transparent: only the roads and the water show over the
  world, and the tint (the `text` colour at the picture's opacity) sets how bright they are.
- **The icons.** The marks use the textures of the HUD package's own objects, found in the frame's HUD tree by name:
  `MMICON_CHECKPOINT` (the checkpoints and the route's end, in the route's colour), `MMICON_AIRACER_<n>` (a racer, turned
  to its heading), `MMICON_<TYPE>_0` (a discovered race start, by its type), `SHOP_0` and `CAR_LOT_0` (the game's yellow
  and blue where the object is plain white), `SAFEHOUSEINDICATOR_0` (green), `MMICON_COPCAR_0` (a sighted cop, with the
  light bar alternating blue and red, held blue with motion reduction) and the helicopter's group. The player's arrow is
  `PLAYERCARINDICATOR` (`Minimap_Icon_Car`). A kind with no icon of the game's (the last-known place, a roadblock) or a
  package without the object draws the shape instead.

Line widths are in HUD units, so they are the same apparent thickness at every resolution.

## 5. Road data and caching

`custom/map/roads.rs` builds the drawable roads from the game's road network (`blackbox_roads::RoadNetwork`):

- **Simplification.** Each segment becomes a 2D polyline: a straight segment is its two ends; a curved one
  (`segment::flags::CURVED`) is sampled with `blackbox_roads::curve` until the chord error is below 1.5 m, then thinned
  with Douglas-Peucker at the same tolerance. Done once per network load.
- **Classes.** `shortcut` where the network flags one (`SHORTCUT`, set at run time by race setup); otherwise no
  shortcut class exists. Main against side road is chosen from the road's lanes (`blackbox_roads` `lane` and `road`)
  in PR 7b; the exact rule is written here then. **[unconfirmed]**
- **Culling grid.** A uniform grid of 128 m cells over the network; each cell lists the polylines crossing it.
- **The local set** is the polylines of the cells under the view circle (radius plus a 25% margin). It is rebuilt only
  when the car has moved more than 25% of the view radius from the set's centre, or the zoom has changed by more than
  10%. It is cached between frames, never rebuilt every frame.
- **Per frame** the map only transforms the cached local set (translate, rotate in heading-up) and builds the
  `Polyline` quads. The route and the marks are rebuilt from the frame's state because they change.

## 6. The `Polyline` component (PR 7a)

Engine-generic, in `libs/blackbox-ui/src/components/polyline.rs`, with no game names:

- Input: points in layout units, a width, a colour, a join (mitre with a mitre limit, or bevel), an optional dash
  pattern, and a clip (circle, ellipse or rectangle) with a soft-edge width.
- Output: one image quad per segment, rotated to the segment, plus a join quad or triangle at each corner. The width is
  constant in screen units. Segments outside the clip are dropped; segments in the soft edge get their alpha scaled by
  the distance to the clip edge.
- Unit tests: quad corners for a straight, an L and a zigzag line; mitre limit falling back to bevel; clip drop and
  edge fade.

## 7. Motion

Zoom steps glide on a critically damped spring (`map_zoom`, 250 ms); mode changes crossfade over `map_mode` (300 ms).
Heading-up rotation follows the car's heading with no extra smoothing. Motion reduction turns zoom and mode changes
into 150 ms crossfades ([visual-identity.md](visual-identity.md) section 7).

## 8. Input actions

New actions (PR 7b) in `input/action.rs`, appended to `Action::ALL`, with names for the `[bindings]` table:

| Action | Config name | Keyboard default | Pad default | Effect |
|---|---|---|---|---|
| `MapMode` | `map_mode` | Tab | D-pad Down | cycle Off, Compact, Centered (Expanded is not in the cycle) |
| `MapExpand` | `map_expand` | T | none | toggle Expanded and back |
| `MapZoomIn` | `map_zoom_in` | I | none | `map_zoom` × 1.25, up to 2.0 |
| `MapZoomOut` | `map_zoom_out` | K | none | `map_zoom` ÷ 1.25, down to 0.5 |
| `MapOrientation` | `map_orientation` | O | D-pad Right | flip `minimap` fixed / rotating |
| `ToggleHud` | `toggle_hud` | F1 | none | hide or show the whole Custom HUD for the session |

Rules:

- **Never driving inputs.** The actions sit in the driving context but are read as button presses on their own; no
  driving action reads them, so opening the map never accelerates, brakes or steers. They are ignored while a menu or
  the console captures input (the existing `capture_menu` / `capture_ui` gating).
- **Free defaults.** In the driving layout the keys Tab, T, I, K, O and F1 and the pad's D-pad Down and D-pad Right are
  unbound today (`input/bindings.rs`: D-pad Up is Interact, D-pad Left the world map). D-pad Down is also the free
  camera's zoom and a menu direction, which are read in other contexts. A test in PR 7b checks that no default of a new
  action collides with a driving action.
- **Remappable** through `[bindings]` and the controls screen; the controls list shows them, and prompts use the live
  binding (`ui/prompts.rs`).
- **Persistence.** Zoom and mode changes made with the keys change the live `Settings.hud_options` for the run, like
  the console's `set custom_hud.*`. Whether they are also written to the config is decided in PR 7b. **[unconfirmed]**
- `ToggleHud` is a session toggle, not saved: a HUD hidden by mistake comes back on the next start.

## 9. Performance

Frame time with the map on against off, measured with the existing bench, `--muted`, while driving: **to be measured**
in PR 7b and recorded here.

| Case | p50 | p99 |
|---|---|---|
| Custom, map `off` | to be measured | to be measured |
| Custom, map `centered` | to be measured | to be measured |
| Custom, map `expanded` | to be measured | to be measured |

## 10. Tests (PR 7b)

- Mode cycling and Expanded's return to the previous mode.
- Orientation from each `minimap` value; the action's flip.
- The local-set rebuild threshold (movement and zoom) and that an unchanged frame does not rebuild.
- Default bindings collide with no driving action.
- Fair knowledge on the map: no cop mark without a sighting ([fair-knowledge.md](fair-knowledge.md)).
