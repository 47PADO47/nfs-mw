# GPS route in the drive

The GPS sets a route from the car to a point on the road. The route is followed by the road navigator
(`libs/blackbox-roads/src/gps.rs`, the path search in `path.rs`), drawn as the original's direction arrow over the car, and shown as a
blip on the minimap. Provenance of the algorithm: [ai-pathfinder.md](ai-pathfinder.md) section 7 (**[decomp]**), and
[world-map-ghidra.md](../provenance/world-map-ghidra.md) for the map side.

## Where it is set

| Source | How | Code |
|---|---|---|
| World map | M, click or Enter on a blip, then **Yes** on "Activate GPS?". The route goes to the blip's position. | `frontend/world_map/control.rs` (`ask_gps`), `Command::SetWaypoint` in `frontend/flow.rs` |
| Console | `gps <x> <y>` (render-space x, y as the map shows them); `gps off` clears | `scenes/world/commands.rs` |
| Mods | `Scene::set_waypoint(Some([x, y]))` (the trait method; the mod bridge does not expose it yet) | `viewer/mod.rs`, `scenes/world/scene.rs` |
| Drive to the first safe house | The drive's `cellusage` timer (`NavigatePlayerTo(target, MiniMapItem, 2000, true)`, [story-flow.md](story-flow.md) 2.5): the route goes to the activity's `gpstarget` box with a largest deviation of 2000 m | `scenes/world/career/drive_to_target.rs`, `WorldScene::route_to_within` |

The target keeps the car's height; the route search puts it on the nearest road segment. A point with no road, or with
no way to it, sets no route: the console says so, and the map closes on Yes without a route (no message yet).

## Behaviour

- **Engage.** The navigator is placed on the car's lane (centre lane), and the path to the target is searched with the
  GPS path type (`PathType::Gps`): one-way and barrier segments are not used. The route is tracking.
- **Every frame.** The navigator is re-placed on the car. If the car is on the route, the route is **tracking**. If not,
  it is **searching**: the navigator is placed on the route's point closest to the car. Deviation is the distance to that
  point; above `MAX_DEVIATION` (200 m) the route is **lost** and cleared.
- **Look-ahead.** `clamp(speed × 3 s, 20 m, 80 m)`, moved towards at 10 m/s. The arrow points at the navigator, advanced
  by the look-ahead along the route. The advance is walked on a copy of the navigator: a path navigator retires the
  route's segments it walks past, and walking the car's own navigator ate the route ahead of the car each frame, so the
  car fell off its route, was snapped onto what was left, and within seconds the arrow pointed straight at the target
  (fixed 2026-11-18; `gps_sample` traces before and after). While tracking, the segments behind the car are dropped, so
  the route (and the HUD's next turn) starts at the car's segment.
- **Finished.** Within 30 m of the target the route is finished and cleared.
- **Arrow.** The game's own `MARKER_DIRECTION_AID` model (`GLOBAL/InGameA.bun`), 1.6 m above the car and about 35 %
  of its size. It faces the chase camera, points straight up on screen when the route is ahead, and tilts in the screen
  plane towards the route's side (exponential rate 8 per second), as the drawing in section 7 of the pathfinder spec
  describes. The model is stored in physics axes: its point runs along -z, its thin axis is y **[verified]** (the
  installed model: thin along y, 0.31 m; symmetric across x, 2.2 m; along z from the point at -1.78 m to the shaft's end
  at 1.24 m). The rewrite lays it flat over the car with that point along the route (`gps/arrow.rs`). Until 2026-11-18 the
  rewrite turned model -x to the route, so the drawn arrow pointed a quarter turn to the right of the route.
  bobbing 0.12 m at 0.5 Hz, and pulsing ±10 % at 2.5 Hz while searching. Shown in the chase view only.
- **Minimap.** A GPS blip at the target (render space), shown while the route is set.
- **Trace.** An unattended career run records the route as it is set, its end and once a second the arrow against the
  route (`gps_route`, `gps_end`, `gps_sample` in [testing.md](../testing.md)).

## Verified in the executable (Ghidra, `speed.exe`)

- `0x0051FA10` sets the GPS target `DAT_0091CA4C` and sets bit `0x80` on the target's byte `+2`; `0x0051FA30` clears
  both. The map's GPS routine `0x0055B0C0` calls the setter (**[verified]**).
- The strings `MARKER_DIRECTION_AID`, `ShowGPS`, `EGPSFinished` and `EGPSLost` are in the executable.

## Not from the original, or not verified

- The arrow's **size and height** are set by eye from one screenshot, not measured against the original.
- The code that places the arrow in the original (the draw routine that uses `MARKER_DIRECTION_AID`) is not decompiled
  yet. The placement above follows the decomp notes in the pathfinder spec, checked against one screenshot.
- The minimap icon is the race **checkpoint** icon (`MMICON_CHECKPOINT`). The original's GPS icon is not among the
  minimap objects documented in [formats/minimap.md](../formats/minimap.md).
- "GPS finished" and "GPS lost" are logged, not shown on the HUD.
- The target height is the car's height, not a surveyed road height.
- The arrow's turn rate and pulse rate are from the spec's description of the drawing, not measured. The look-ahead,
  arrival and deviation numbers are the spec's **[decomp]** constants.

## Open items

- The arrow's placement: the pathfinder spec puts it in front of the camera, below the centre, turning at 0.125 to 1
  turns a second; the rewrite floats it flat above the car and turns it exponentially.
- Mod API: `map.set_waypoint` / `map.clear_waypoint` in [modding/api-world.md](../modding/api-world.md) are not bound.
- Route drawn on the minimap (only the target blip is shown).
