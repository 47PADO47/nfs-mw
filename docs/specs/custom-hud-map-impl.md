# Custom HUD map: implementation notes

How PR 7b builds the centred translucent map of the Custom HUD 2026 (the design is `docs/specs/custom-hud/map.md` and
`fair-knowledge.md`; ADR 0013). This page records what the road data has, the rules the code follows, and what is
not done yet.

Code:

- `crates/nfsmw/src/hud/map_roads/`: the roads as simplified polylines in a uniform grid, built on the scene side.
- `crates/nfsmw/src/ui/theme/custom/map/`: the map component (modes, memory, view, picture, icons, roads, marks, draw,
  chip).
- `crates/nfsmw/src/hud/map_picture.rs`: the game's map tiles and calibration for the picture (section 9).
- `crates/nfsmw/src/hud/map_keys.rs`: the map's keys applied to the live settings.
- `crates/nfsmw/src/input/bindings.rs` (`hud_map`): the default bindings.

## 1. Road data and its classes

The map reads the traffic's road network (`TrafficWorld::road_graph()`, `blackbox_roads::RoadNetwork`). What it
has, from `docs/formats/road-network.md` (counts of this install) and the code:

| Data | What it says | Used for |
|---|---|---|
| Profile zones (`RNpf`) | lane kinds per node: traffic (1,634 zones), sidewalk, shoulder, median (160), barrier (75), road (471) | main against side |
| `NO_TRAFFIC` (1,356 segments) | traffic never drives the segment | the path class |
| `DECISION` (3,463) | a connector inside a junction, from each way in to each way out | through connectors only |
| `CURVED` (5,892) | a Bézier segment | sampling |
| `SHORTCUT` (0) | runtime only, set by race setup | nothing: see below |
| `RNrd` roads | a chain of segments; `scale` 1.0 everywhere, shortcut number 0xFF everywhere, a speech id | nothing |

**Classes** (`hud/map_roads/classify.rs`):

- **Path:** a segment flagged `NO_TRAFFIC`. Drawn by the Expanded map only, in the rule tone.
- **Main:** at least 4 traffic lanes (both ways together, the larger of the two end profiles), or a median or barrier
  zone (a divided road). Two units wide, primary text tone.
- **Side:** everything else. One unit wide, secondary text tone.

The lane threshold is **[ours]**, chosen so a street with one lane each way is a side road and one with two each way
(or a divided road) is a main road. How the install's roads split under it has not been counted or checked on screen
yet. **[unconfirmed]**

**No shortcut class.** `SHORTCUT` is zero in the file, and the rewrite's race setup never sets it (no code reads or
writes the flag outside `blackbox-roads`), so there is nothing to class by. If race setup starts setting it, a
`Shortcut` class drawn dashed is the follow-up.

**Junctions.** Drawing every `DECISION` connector fills a junction with a fan of curves. Only the connectors whose
direction turns less than 35° between their two ends are kept: they close the gap between the roads at the junction
and draw a plain crossing. The turning ones are dropped.

## 2. Simplification and the grid

- Each kept segment becomes a 2D polyline in render space (x forward, y left, metres, as `MapPosition`): a straight
  segment is its two ends; a curved one is cut in halves (at least into quarters, at most 1/256) until each piece's
  middle is within 1.5 m of its chord, then thinned with Douglas-Peucker at 1.5 m. The polyline stays within 3 m of
  the curve (each step allows its tolerance).
- One polyline per segment; segments are not chained into longer roads, so a bend between two segments is a butt
  joint. At map widths of one or two units it does not show.
- The grid has cells of 128 m over the roads' bounds; each cell lists every road whose pieces' boxes cover it.

## 3. Handover and the cache rule

- **Built once per network.** `scenes/world/open.rs` builds `MapRoads` from the traffic's network when the world
  opens, behind an `Arc` (`MapRoadsHandle`). Every HUD frame copies the handle into `HudState::map_roads`
  (`scenes/world/hud.rs`): a reference count, never a copy of the roads. Two handles compare equal only when they
  share the same roads, so comparing HUD states never walks them.
- **The car's place.** `HudState::minimap` is `None` when the `minimap` setting is off, so the HUD plugin copies it
  to `HudState::map_car` in `sync` before applying the setting. The Custom map follows the car whatever `minimap`
  says; `minimap`'s orientation still turns it.
- **The local set** (`custom/map/roads.rs`) is the roads of the grid cells under a circle round the car, 1.4 times the
  view radius. It is queried again only when the car has moved more than 25% of the view radius from where the set
  was taken, the view radius has changed by more than 10%, or the network is another one. The margin covers both
  thresholds, so no road in view is missing between two queries. A frame with no change only projects the cached
  roads and builds their strokes. Off and Compact do not touch the set.
- **The race route** (`hud/map_roads/race_route.rs`). `RaceSession::route` (the opponents' `blackbox_roads::Route`)
  runs through every gate of every lap. When the session starts (`scenes/world/race/start.rs`), the first lap's legs
  (checkpoints plus the finish) are sampled like the roads (each step's piece of its segment's centre line, a missing
  leg as the straight line between its goals), thinned at 1.5 m, and each kept point keeps its distance along the
  route. A lap race (circuit, knockout) is closed: the line runs on to its first point. The result sits in the session
  as a `RaceRouteHandle` (an `Arc`), and `hud_frame` copies the handle into `HudState::race_route` while a session
  exists, `None` outside a race. The player's place on the lap is the route's length less `race_info.remaining_m`,
  modulo the lap's length, so splitting the line costs a binary search per frame. A pursuit challenge has no route
  and draws none.
- **Memory between frames.** The theme makes a fresh component every frame, so the local set, the zoom spring and the
  mode change live in a `MapMemoryCell` the `CustomFeedCell` holds (`custom/map/memory.rs`).

## 4. Modes, orientation and motion

| Mode | Draws |
|---|---|
| Off | nothing |
| Compact | a small overlay, an ellipse of 15.5% by 11.5% of the box height for 200 m, at 0.7 of `map_opacity`; and the turn-cue chip under the top centre: an arrow (straight, left, right, U-turn) and the distance, rounded to 10 m, or to 50 ft below 1000 ft and 0.1 mi above |
| Centered | the overlay: an ellipse of 23% by 17% of the box height for 300 m, at `map_opacity` |
| Expanded | 33% by 24.5% for 600 m, at `max(map_opacity, min(map_opacity + 0.25, 0.85))`, plus paths (roads only) and the stretch of the route already driven (at half the route's alpha) |

The metres are for the roads; with the game's picture the view is 2.4 times wider (section 9).

- The view radius is the base over `map_zoom`; it is what the ellipse's horizontal half-axis shows. The last 12% of
  the mean radius fades the roads, the route and the marks to nothing.
- Orientation: `minimap` rotating is heading-up (the map turns under an upright arrow); fixed and off are north-up
  (the arrow turns).
- A mode change blends the old mode's overlay into the new one's over `Motion::map_mode` (300 ms, eased): the ellipse
  and the scale glide, the overlay and the chip fade. A zoom step glides on a critically damped spring settling in
  `Motion::map_zoom` (250 ms).
- Motion reduction: the shape, the scale and the zoom take the new value at once and the overlay fades back in over
  150 ms. This stands for the spec's crossfade: the old view is not drawn under the new one.

## 5. Layers and marks

Back to front, depths far behind the HUD's other elements (`DEFAULT_Z` is 1): roads (60), route (59), marks (58), the
player's arrow (57). Line widths are in HUD units times the HUD scale, the same on screen at every zoom.

- **Route:** `HudState::route.path`, nav (signal yellow), three units, the stretch ahead of the car at the roads' opacity plus 0.4.
- **Race route:** `HudState::race_route` (section 3), nav yellow, three units, under the GPS route. The stretch ahead of
  the player is at the route's alpha and the stretch already driven this lap at half of it; without the player's
  place on the route (no `race_info.remaining_m`) the whole lap is bright. Centered and Expanded both draw it.
- **Race:** the next gate (`race_info.next_gate`) as a nav ring held inside the ellipse's edge when out of view; the
  checkpoint, speed-trap and racer blips as dots.
- **Places:** the discovered `HudState::markers`: race starts (accent diamond), safe houses (success square), shops
  and car lots (text dot). The route's end is a nav diamond, held in view like the next gate.
- **Police**, by the fair-knowledge rules, from `HudState::police_intel` only (the cop blips are never read): sighted
  cops as solid danger dots; the helicopter only while it has the player in sight; the police's last-known place as a
  hollow danger ring in Search and Cooldown only, fading from 0.8 to 0 over 20 s of age; roadblocks only once
  dispatch has reported them.
- Colours are the `custom.*` tokens, so the palette and high contrast apply; with high contrast the main roads get a
  dark under-stroke one unit wider.

## 6. Keys

`MapMode` (Tab, D-pad down) cycles off, compact, centered, expanded, then off. `MapExpand` (T) opens Expanded and goes
back to the mode it was opened from (Centered when Expanded was reached by the cycle). `MapZoomIn` / `MapZoomOut`
(I, K) multiply or divide `map_zoom` by 1.25 within 0.5 to 2. `MapOrientation` (O, D-pad right) flips `minimap` between
fixed and rotating (off becomes rotating). `ToggleHud` (F1) flips the `hud` setting.

- The design page leaves Expanded out of the `MapMode` cycle; the PR 7b brief puts it in, so the pad (which has no
  `MapExpand` default) can reach it. The code follows the brief.
- **Persistence: session only.** The keys change the live `Settings`, like the console's `set`; nothing is written to
  the config file. The saved values stay those of `[custom_hud]`, `minimap` and `hud`.
- The keys work while the scene shows a HUD and never while a menu or the console has the input. No default shares
  an input with a driving action (`input/map_bindings_tests.rs`).

## 7. Performance

Frame time with the map on against off, measured with the existing bench, `--muted`, while driving: **perf: to be
measured.** The cost to watch is the stroke count: one quad per road piece in view (plus the fade band's pieces), a
few hundred roads in the Centered view, more in Expanded.

| Case | p50 | p99 |
|---|---|---|
| Custom, map `off` | to be measured | to be measured |
| Custom, map `centered` | to be measured | to be measured |
| Custom, map `expanded` | to be measured | to be measured |

## 8. The full-screen world map (PR 11c)

The Custom theme's full-screen world map (`ui/theme/custom/full_map/`, described in `ui-themes-custom.md` section 11)
draws with this map's pieces rather than its own:

- **Roads:** the same `MapRoadsHandle`, carried from the HUD state into the frontend's map frame at open (a reference
  count), the same `LocalSet` cache rule (here over the window's half diagonal, so a pan or a zoom step queries the grid
  again and a still map does not), and `road_lines` with `road_look`, at opacity 0.85 with paths.
- **Projection:** a north-up `MapView` built from the city's calibration and the frontend's camera, its centre the
  screen's centre; no ellipse, a rectangle clip of the window with no soft edge.
- **Route and marks:** `route_lines` for the GPS route, `draw_marks` for the current tab's places and the route's end
  (pinned inside the window's inscribed ellipse like the overlay's), `mark_parts` for the legend's and the card's
  shapes, `ring` for the focused place, and `arrow` for the players.

The refactor that shared them (`mark_parts`, `ring`, `road_look`) changes nothing the HUD map draws; its tests are
unchanged.

## 9. The game's picture and icons

The map draws the game's own minimap picture and icons rather than strokes of the road network
([map.md](custom-hud/map.md) section 4a for the look).

- **Handover.** `hud::MapPicture` holds the texture keys of the 64 tiles of the minimap file in use (`<stem>_CHOP<n>`)
  and its calibration. The HUD plugin builds one when the file changes (`MinimapBinding::picture`) and copies the
  handle into `HudState::map_picture` every frame; `None` when the install has no minimap, and the map draws the roads.
  The file is the one the Original and Remastered minimaps show: the stage's, a pursuit race's region, or a race's own
  event map (with its route baked in).
- **Cells.** `map/picture.rs` finds the blocks under the circle that holds the ellipse (the picture is 64 blocks
  wide), keeps those the ellipse reaches, draws the ones well inside as one `Image` each and cuts the ones the soft edge
  crosses into up to 12 x 12 cells, each tinted by the fade at its middle. The cells are one component (one box per
  frame). A block is an upright square of the picture turned by the view's heading-up turn.
- **Why cells and not the game's mask.** The game fades its minimap through a mask texture that moves per object; the
  presenter keeps a slot per masked object, which does not fit a thousand cells. A tint per cell does the same on an
  additive texture.
- **Icons.** `map/icons.rs` finds the object by its name hash in the frame's HUD tree (`FrameCell`, the source the
  Original minimap reads) and takes its texture, its size on the HUD and its colour. `draw.rs` places it at the mark's
  projected place, turned to the mark's heading when the HUD state has one (the racers); an icon that is missing draws
  the shape. A mod theme that declares tokens draws the picture and the shapes: it has no frame cell.
- **Cost.** About a thousand quads in the Centered view (the ellipse's area over the cell, and a block for the interior).
  Measured: **to be measured** (see section 7).
