# Custom HUD: world markers, as built

What PR 9 of the Custom HUD 2026 built for the world-space markers, and where it departs from the design
(`docs/specs/custom-hud/world-markers.md`). The code is `crates/nfsmw/src/ui/theme/custom/markers/` (the `WorldMarkers`
slot, 18) and `crates/nfsmw/src/scenes/world/hud_marks.rs` (the scene's half).

## Data path

- **Projection.** `HudState::camera` (`hud/state/camera.rs`), the gameplay lens's `view_proj` with the window as its
  viewport. A projected viewport pixel maps onto the canvas by its share of the viewport, over `layout::window`, so the
  markers use the whole window, not the 16:9 content box.
- **The scene's half** (`hud_marks.rs`, filled in `hud_frame` into `HudState::world_marks`):
  - the ground under each map place (`HudState::markers`) within 300 m of the car, nearest six, from
    `space::ground_below`: the map places are 2D, and a marker needs a height to project;
  - the pursuit breakers within 150 m of the car while a chase is on, nearest four (`BreakerIcons::near`, the icons the
    minimap shows, so a fired breaker is gone here too);
  - per place, breaker and the next gate, whether the static world hides it (below).
- **The rest comes from the state as it is:** the next gate and its number from `race_info`, the car's place and heading
  from `minimap`, the checkpoint pass and the wrong way from `events`.

## Occlusion

Built with the cops' own line-of-sight ray (`scenes/world/drive/sight.rs`, `sight_cast`): from the camera's eye to a point
2 m above the place, against the resident collision. A hit more than 1 m short of the end hides the marker. The eye is
recovered from the camera's matrix (the point it sends to clip `w` 0), so the scene builds the lens once.

Cost per HUD frame: at most six ground probes and eleven rays (six places, four breakers, one gate), all short of
300 m. A hidden checkpoint draws at 0.3 alpha without its label; hidden entrances, places and breakers are not drawn.
This settles the **[unconfirmed]** of the design's section 6.

## Choices made here **[ours]**

- **Recovery.** The next gate turns into a recovery marker (caution colour, a chevron over the diamond on screen, an edge
  arrow off it) for 5 s after a `WrongWay` event, and whenever it is more than 15 m behind the car's heading. Closer
  than that the car is driving through it.
- **The active gate's range.** It dims to 0.35 alpha and 0.6 size by 600 m but never disappears; it is the way to go.
- **Discovered locations** (safe houses, shops, car lots) show with `[custom_hud] free_roam_extras`, which the Advanced
  preset turns on; there is no separate "Advanced" switch.
- **Breaker sight distance** is 150 m. The design ties it to the cops' visibility, which has no single distance in the
  simulation.
- **Labels.** Steadied by a one-pole low-pass (50 ms) per marker id, held in a memory cell next to the feed
  (`CustomFeedCell`), and snapped to whole physical pixels. A move of more than 60 canvas units is taken at once (the
  marker changed place, which is not shake). The marks themselves follow the raw projection.
- **The checkpoint tick** is a ring in the success colour that grows from the diamond and fades over the `pulse`
  duration (0.4 s), or with motion reduction a still ring held as long, at the gate that was passed.

## Not built

- **Next checkpoints** (the two gates after the active one): `RaceInfo` carries the next gate only, and inventing the
  others is not allowed. They would need the race's gate list in the HUD state.
- **The direction to take near a junction** at the active checkpoint: the route's `next_turn` is drawn by the race and
  map components, not repeated on the marker.
- **Sighted-threat edge arrows** (cops and the helicopter outside the view, design section 3): not in this PR's brief;
  `PoliceIntel` holds what they need.
- **The interaction prompt** inside an event's entry zone stays the scene's own prompt.
