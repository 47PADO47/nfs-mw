# World map (M)

The full-screen map the player opens from free roam with M. Epic E5. Status: M opens the map, which zooms, pans,
filters, lists and attaches the cursor to blips. The map is the package `WorldMapMain.fng`, drawn by the FEng runtime.
Activating GPS is a stub. Object names and GUIDs are in `docs/provenance/world-map-screen.md`; the executable's
behaviour is in `docs/provenance/world-map-ghidra.md`.

## Keys

- **M** opens the map from free roam and closes it again (`Action::Map`, `input/bindings.rs`). The radio pauses and
  resumes on **N** (`Action::RadioToggle`); `.` and `,` are still next and previous.
- In the map: **scroll** (or the pad's D-pad up and down) zooms; **Z** is Zoom All (the whole picture; not a key the
  executable has, see below); a **drag** (the mouse's orbit drag, or the right stick) pans; **A / D** or the arrows
  change the tab; **Enter** or a **left click** on a blip asks "Activate GPS?", and a second Enter confirms; **right
  click** asks the same question; **Esc** or **M** cancels the question, or closes the map.

## Screen (`frontend/world_map/screen/`)

The map is a screen of the frontend, opened over the frozen world (`control::open` calls `Frontend::open_map`, stage
`Stage::Map`). The package is `WorldMapMain.fng`; its logic, `WorldMapLogic`, draws the map's frame into the package's
objects each tick. The frame (`frame.rs`: the session, the placed blips, the player) reaches the logic through
`Cx::world_map`, set by `Screens::set_world_map` (the same way the controls list gets its key names).

- **Picture**: the package's own map object (GUID `0xF9984`, `TRACK_MAP.tga`, centred at (-111, 11), 340 wide, FEng
  units). The camera's crop is its texture rectangle (`set_uv`), so zoom and pan move the texture.
- **Blips** (`screen/pools.rs`, `screen/place.rs`): each blip takes the next pool object of its style. The pools are the
  executable's own: auto shops (8), safe houses (4), car lots (4), and for the races sprint (6), circuit (6), drag (3),
  knockout (6), tollbooth (3), speed trap (6). A blip is drawn only when its style's pool is in view. A blip beyond its
  style's pool is listed and can be attached, but is not drawn. The executable drops it from the list too.
- **List**: seven rows (the package's `option_name` strings, moved onto the right of the picture), each with its icon.
  The window keeps the attached blip in view. The attached row is yellow. With no blip of the tab, the first row says so.
- **Cursor and card**: the package's `IconSelection_Glow` group moves to the attached blip. The card under the picture
  shows its style and distance. With the prompt open the card reads "Activate GPS?" and "Enter: yes   Esc: no".
- **Tab and zoom**: the tab's label is LOCATIONS or RACES; the zoom readout is "Zoom x1.0", "x2.0", "x3.3" or "x5.0".

## Behaviour (`frontend/world_map/`)

- **Filter** (`filter.rs`): `Locations` (auto shop, safe house, car lot; the default) and `Races` (the six race
  styles). `next` and `previous` wrap.
- **Camera** (`camera.rs`): four zoom levels, 1.0, 2.0, 3.3 and 5.0 (the executable's `Z`), one level per scroll line;
  a fine scroll adds up to a level. Zoom 1 is Zoom All. The view can never leave the picture. Pan is clamped to the
  picture's edges.
- **Attach** (`pick.rs`, `session.rs`): the cursor attaches to the nearest blip of the current filter within 0.06 of
  the picture's width (the executable's 20 canvas pixels, `PICK_RADIUS`). A frame with no cursor (a key press) keeps the
  attachment. The prompt keeps its blip even when the cursor moves away. The executable also detaches when a drag
  moves the cursor 20 pixels or more from the blip; the port keeps the attachment.
- **Blips** (`overlay.rs`): read from the world's HUD blips when the map opens, placed on the picture with the
  minimap's city calibration (`nfsmw_data::minimap::open_city_calibration`). The world is frozen while the map is
  open, and a paused scene reports no blips, so they are read before the freeze.
- **Icons and colours** (`control.rs`): the package's own `Minimap_Icon_*` textures, set per style; the interiors take
  the HUD's tints (auto shop yellow, car lot blue, safe house green). The races have no tint.
- **Flow** (`control.rs`, `flow.rs` `Stage::Map`, `plugin.rs`): M in free roam sets the stage to Map and pauses the
  world. Each frame the map's input runs the session, then the screen runs one step with the new frame. The map's
  outcome closes the screen and resumes free roam.

## GPS (stub)

`activate_gps` (`world_map/gps.rs`) logs and does nothing. The route is the pathfinder's GPS target
(`docs/specs/ai-pathfinder.md` section 7), not yet built. The map closes after the call either way. The executable sets
the GPS target in `FUN_0051FA10` and asks through a separate `InGameDialog.fng`; the port asks on the card.

## Differences from the executable

- **Zoom All** is the Z key. The executable has no Zoom-All message; it reaches zoom 1 by zooming out.
- **The prompt** is on the card, not a separate dialog.
- **The list** scrolls with the attached blip only (there is no list scroll key).
- **Attach** does not detach on a drag (see above).
- **Place names**: a blip has no name in the data, so the list shows its style's name ("Auto shop", "Sprint"), not
  the place's.
- **Race pools for Event_Rival and Event_Cell** are not drawn: the executable's type 7 and cell icons are not mapped.

## Not verified

- The picture's placement and the blips' positions use the city calibration the minimap uses. The FEng placement
  (centre of each object, y down) follows the presenter and the dump, but was not checked against a rendered frame in
  game.
- The package's start scripts were not traced; the logic writes its objects each tick, which wins over a script that
  only sets them once.
