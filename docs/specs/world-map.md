# World map (M)

The full-screen map the player opens from free roam with M. Epic E5. Status: the pure state is built and tested
(`crates/nfsmw/src/frontend/world_map/`); the screen, the picture and the GPS route are not.

## Keys

- **M** opens and closes the world map (`Action::Map`, `input/bindings.rs`). Before this change M was the radio's
  pause key. The radio now pauses and resumes on **N** (`Action::RadioToggle`, `audio/radio/input.rs`); `.` and `,`
  are still next and previous. The change is the only rebind: the media keys and the right stick are unchanged.
- The original M key binding (VK 0x4D) is not reversed here. The radio's M was this port's choice, not a reading of
  the executable, so moving it is not a fidelity change to a verified binding.

## Layout (from the first reference screenshot)

- The full picture fills the screen on the left. The right side has a **LOCATIONS** tab with a legend (Shop, Safe
  House, Car Lot), a **Toggle Icon Visibility** control, an **Activate GPS** control, **Zoom All** and **Back**.
- The filter tabs cycle the blip type. The map opens on **Locations**. The other tab is **Races** (the six race
  styles of `hud/blip_styles.rs`).
- The right-side list shows every blip of the selected type.

## Behaviour (implemented in `world_map/`)

- **Filter** (`filter.rs`): `Filter::Locations` (auto shop, safe house, car lot) and `Filter::Races` (the six race
  styles). `next` and `previous` wrap. The blip colours and icons are the HUD's own styles, by name.
- **Camera** (`camera.rs`): zoom is 1 for "Zoom All" (the whole picture) up to `MAX_ZOOM` (8). Each scroll line
  multiplies the zoom by 1.25. The view can never leave the picture: zooming out to 1 recentres, and a pan is
  clamped to the picture's edges. Pan is in view units, so a drag moves the picture with the pointer.
- **Attach** (`pick.rs`, `session.rs`): the cursor attaches to the nearest blip of the current filter within
  `PICK_RADIUS` (0.06 of a view width). A blip of another filter is never attached, even when nearer. Nothing is
  attached outside the radius. A frame with no cursor (a key press, or the pointer outside the map) keeps the current
  attachment, so Enter works from the keyboard; the next hover picks again.
- **GPS prompt** (`session.rs`): Enter or right click, with a blip attached, opens the prompt. Enter while the prompt
  is open gives `Outcome::ActivateGps(index)`, and the caller closes the map. Back cancels the prompt; back with no
  prompt closes the map. The attachment is frozen while the prompt is open, so it names the blip it asked about.
- **Tab switch** drops the prompt and the attachment.
- **Distance**: straight-line metres from the player to the attached blip (`distance_metres`), both in render space.

## GPS (stub)

`activate_gps` (`world_map/gps.rs`) logs and does nothing. The route is the pathfinder's GPS target
(`docs/specs/ai-pathfinder.md` section 7), not yet built. The map closes after the call either way.

## Not built, and why

- **The screen.** `WorldMapMain.fng` is not dumped, so its object, tab, list and cursor names are unknown. The
  screen logic, its catalog entry and its flow stage (a `Stage` beside `Paused` in `frontend/flow.rs`) are the next
  step. Until then `world_map` is `#[allow(dead_code)]` and nothing opens it.
- **The picture.** `TrackMaps.bin` (`docs/formats/world.md`, a 63.5 MB TPK, the world-map screen's picture) has no
  reader. `blackbox-tpk` is the existing TPK reader; a `&[u8]` loader for the picture belongs there or in a new lib,
  with no NFS:MW names. Its layout is not reversed in this change.
- **Reversal of the screen's zoom, pan, filter and list behaviour** from the executable (Ghidra, base 0x400000) is
  not done. The behaviour above is the design from the reference screenshot, not a verified reading.

## Projection

A blip's place on the picture is `blackbox_minimap::Calibration::to_map` of its world position. The picture is square,
0..1 on both axes. The HUD's minimap uses the same projection (`docs/specs/hud-minimap.md` section 3).
