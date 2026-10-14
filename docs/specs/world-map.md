# World map (M)

The full-screen map the player opens from free roam with M. Epic E5. Status: M opens the map, which zooms, pans,
filters, lists and attaches the cursor to blips. Activating GPS is a stub. The package's names are in
`docs/provenance/world-map-screen.md`.

## Keys

- **M** opens the map from free roam and closes it again (`Action::Map`, `input/bindings.rs`). Before this, M was the
  radio's pause key. The radio now pauses and resumes on **N** (`Action::RadioToggle`); `.` and `,` are still next
  and previous. The media keys and the right stick are unchanged.
- In the map: **scroll** (or the pad's D-pad up and down) zooms; **Z** is Zoom All (the whole picture); a **drag** (the
  mouse's orbit drag, or the right stick) pans; **A / D** or the arrows change the tab; **Enter** or a **left click**
  on a blip asks "Activate GPS?", and a second Enter confirms; **right click** asks the same question; **Esc** or
  **M** cancels the question, or closes the map.

## Layout

The picture is a square on the left. On the right, a list of every blip of the selected type, with each one's name
and distance. The attached blip is highlighted in the list and has a card under the picture with its name and
distance. The title is WORLD MAP; the hints are along the bottom. The look follows the first reference screenshot,
drawn with egui, not with the FEng screen (below).

## Behaviour (`frontend/world_map/`)

- **Filter** (`filter.rs`): `Locations` (auto shop, safe house, car lot; the default) and `Races` (the six race
  styles). `next` and `previous` wrap.
- **Camera** (`camera.rs`): zoom 1 is Zoom All; each scroll line multiplies it by 1.25, up to 8. The view can never
  leave the picture. Pan is clamped to the picture's edges.
- **Attach** (`pick.rs`, `session.rs`): the cursor attaches to the nearest blip of the current filter within 0.06 of
  the picture's width. A frame with no cursor (a key press) keeps the attachment. The prompt keeps its blip even when
  the cursor moves away.
- **Blips** (`overlay.rs`): read from the world's HUD blips when the map opens, placed on the picture with the
  minimap's city calibration (`nfsmw_data::minimap::open_city_calibration`). The world is frozen while the map is
  open, and a paused scene reports no blips, so they are read before the freeze.
- **Icons and colours** (`control.rs`): the package's own `Minimap_Icon_*` textures; the interiors take the HUD's
  tints (auto shop yellow, car lot blue, safe house green). The races have no tint.
- **Picture** (`overlay.rs`, `view.rs`): `TRACK_MAP.tga`, the world-map screen's own texture (resource 17 of
  `WorldMapMain.fng`), so `TrackMaps.bin` is not needed for the base picture. Zoom and pan are a UV crop of it.
- **Flow** (`control.rs`, `flow.rs` `Stage::Map`, `plugin.rs`): M in free roam sets the stage to Map and pauses the
  world; the map's input runs each frame and its outcome closes the map and resumes free roam.
- **Drawing** (`devtools/mod.rs` egui pass calls `world_map::show`): the same pass that draws the garage and the
  radio card.

## GPS (stub)

`activate_gps` (`world_map/gps.rs`) logs and does nothing. The route is the pathfinder's GPS target
(`docs/specs/ai-pathfinder.md` section 7), not yet built. The map closes after the call either way.

## Not done, and why

- **The FEng screen is not opened.** `WorldMapMain.fng` is dumped and its names are in the provenance note. Its
  blip pools are fixed off-screen objects, and its list and info rows are off-screen too, so the port draws the map
  with egui (the same pass as the garage). A future FEng path would drive those objects; `frontend/ids.rs` and the
  catalog are not changed.
- **The list names.** A blip has no name in the data, so the list shows its style's name ("Auto shop", "Sprint"),
  not the place's.
- **The race icons.** The race styles use the generic event icon, except the speed trap and the circuit. The
  per-type race icon pools are not mapped.
- **The placement.** The picture's place and the blip positions use the city calibration the minimap uses. This is
  not checked against a rendered frame.
- **Behaviour not reversed.** The zoom, pan, filter and list behaviour is from the first reference screenshot, not
  from the executable (Ghidra).
