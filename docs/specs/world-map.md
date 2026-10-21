# World map (M)

The full-screen map the player opens from free roam with M. Epic E5. Status: M opens the map, which zooms, pans,
filters, lists and focuses the cursor on blips. The map is the package `WorldMapMain.fng`, drawn by the FEng runtime.
Confirming GPS raises the game's own `Dialog.fng` Yes/No; setting the route is a later step. Object names and GUIDs
are in `docs/provenance/world-map-screen.md`; the executable's behaviour is in `docs/provenance/world-map-ghidra.md`.

## Keys

- **M** opens the map from free roam and closes it again (`Action::Map`, `input/bindings.rs`). The radio pauses and
  resumes on **N** (`Action::RadioToggle`); `.` and `,` are still next and previous.
- In the map: **scroll** zooms while the cursor is over the picture, and scrolls the themed legend's list while it is
  off the picture (over the legend); **Z** is Zoom All (the whole picture; not a key the executable has, see below); a
  **drag** (the mouse's orbit drag, or the right stick) pans; **A / D** or the arrows change the tab. **Hovering** the
  cursor over a blip of the filter focuses it (the cursor snaps on); moving off unfocuses. A **left click**, **Enter**
  or a **right click** on the focused blip asks "Activate GPS?" in the game's `Dialog.fng` Yes/No; Yes closes the map,
  No returns to it with the blip still focused. **Esc** or **M** unfocuses, or closes the map.

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
- **Cursor and card**: the cursor follows the free mouse pointer as a cross and snaps onto the focused blip as a
  square (the executable's snap animation, replayed when the focus changes; `docs/provenance/world-map-ghidra.md`). The
  round glow inside the cursor group and the OS pointer are hidden on the map, so only the cross/square shows. The card
  under the picture shows the focused blip's style and distance.
- **Tab and zoom**: the tab's label is LOCATIONS or RACES; the zoom readout is "Zoom x1.0", "x2.0", "x3.3" or "x5.0".

## Behaviour (`frontend/world_map/`)

- **Filter** (`filter.rs`): `Locations` (auto shop, safe house, car lot; the default) and `Races` (the six race
  styles). `next` and `previous` wrap.
- **Camera** (`camera.rs`): four zoom levels, 1.0, 2.0, 3.3 and 5.0 (the executable's `Z`), one level per scroll line;
  a fine scroll adds up to a level. Zoom 1 is Zoom All. The view can never leave the picture. Pan is clamped to the
  picture's edges.
- **Focus** (`pick.rs`, `session.rs`): hovering the cursor over the picture focuses the nearest blip of the current
  filter within 0.06 of the picture's width (the executable's 20 canvas pixels, `PICK_RADIUS`); moving off a blip
  unfocuses. A frame with no cursor (a key press) keeps the focus, and switching the tab drops it (and resets the list
  scroll). The executable also detaches when a drag moves the cursor 20 pixels or more from the blip; the port keeps it.
- **List scroll** (`session.rs`, `legend.rs`): while the cursor is off the picture the scroll wheel moves the themed
  legend's list (`list_scroll`, clamped to the filter's blips less the `VISIBLE_ROWS` shown); over the picture it zooms.
  The map's own FEng list (seven rows) is not scrolled.
- **Blips** (`overlay.rs`): read from the world's HUD blips when the map opens, placed on the picture with the
  minimap's city calibration (`nfsmw_data::minimap::open_city_calibration`). The world is frozen while the map is
  open, and a paused scene reports no blips, so they are read before the freeze.
- **Icons and colours** (`control.rs`): the package's own `Minimap_Icon_*` textures, set per style; the interiors take
  the HUD's tints (auto shop yellow, car lot blue, safe house green). The races have no tint.
- **Flow** (`control.rs`, `flow.rs` `Stage::Map`, `plugin.rs`): M in free roam sets the stage to Map and pauses the
  world. Each frame the map's input runs the session, then the screen runs one step with the new frame. The session's
  outcome keeps the map open, closes it (resuming free roam), or opens the GPS `Dialog.fng` over the frozen map.

## GPS confirm

Confirming GPS on the focused blip raises the game's own `Dialog.fng` Yes/No over the frozen map (`control::ask_gps`
opens it with `ConfirmLogic`, `frontend/confirm.rs`; the dialog's input is routed while it is over the map,
`plugin.rs`). **Yes** resumes the game, which closes the map (`Command::Resume`); **No** pops the dialog and the map
goes on with the blip still focused. Setting the actual route is the pathfinder's GPS target
(`docs/specs/ai-pathfinder.md` section 7), built: Yes sets the route to the focused blip and closes the map (`docs/specs/gps-route.md`). The executable sets the
GPS target in its own GPS-target routine and asks through a separate dialog (`docs/provenance/world-map-ghidra.md`).

## Custom look (themeable UI)

Under the **Custom** theme the live `WorldMapMain.fng` screen is **recoloured in place**, not replaced, so it keeps
its fullscreen picture, its filter switch, its button prompts (`PC_key_*`, `R1`/`L1`) and all of its mouse behaviour
(drag, zoom, pin-click). `world_map/screen/restyle.rs` hides the decorative stripe and makes the one background layer
(`BasePoly.tga`, and the big side/centre panels) less transparent — a single dark scrim, no separate panel behind the
map content — so the frozen world still shows through; under Custom it also tints the panels dark and the static
text light. The screen then hides its own list and card, and the themed legend is drawn in their place.

- **The flag + legend.** `ElementId::WorldMapChrome` (26) resolves to `MapLegend` (`ui/theme/custom/map_legend.rs`)
  under Custom and `Empty` under Original. Providing it also tells the frontend to recolour: the plugin reads
  `UiThemes::draws_world_map_chrome`, passes `themed` into `control::step` (which sets `WorldMapFrame.chrome_themed`, so
  the screen recolours and hides its list), fills the legend from the open map (`world_map/legend.rs`, the same data the
  FEng list shows), and presents the legend over the map. Under Original the flag is off and the screen is untouched.
- **The legend panel** shows up to `VISIBLE_ROWS` of the filter's blips (icon, name, distance), the focused row in the
  accent colour. When the filter has more, the wheel (cursor off the picture) scrolls the list and a slim scrollbar in
  the right gutter shows the position.

## Remastered look (themeable UI)

Under the **Remastered** theme the whole map is drawn by the theme (`ui/theme/remastered/map/`); the FEng screen
`WorldMapMain.fng` still runs underneath, for its logic and the dialog stack, but is not presented. The session, the input
and the GPS prompt are the same as every other look's.

- **The flag.** `ElementId::WorldMapChrome` (26) resolves to `WorldMapView` under Remastered, to `MapLegend` under Custom and
  to `Empty` under Original. `UiThemes::world_map_chrome` turns that into a `Chrome` (`Original`, `Recoloured`, `Drawn`),
  which the plugin passes to `control::input` and `control::step` and stores in `WorldMapFrame::chrome`.
- **The picture** is the career stage's map texture (`WorldMapFrame::texture`, from `MapVariant::world_map_texture`). It is
  square and is drawn as high as the window (`Picture::FULL`, 480 units), centred, so Zoom All shows the whole map and the
  rest of the window is dark ground; zoom and pan move and scale the same square (`camera.to_view`). A dark veil lies over
  it and its left and right edges fade into the ground.
- **Blips** are the game's own `Minimap_Icon_*` textures at the HUD's tints, every blip of the current tab (no pool cap),
  with the player's car turned to its bearing.
- **Cursor.** A cross at the pointer; over a blip the session focuses it and the cursor becomes four corner brackets that close
  from 46 to 27 units over 0.18 s (`WorldMapFrame::snap_age`, from `WorldMapOverlay::tick`), with a label of the blip's style and
  distance beside it. A click, Enter or a right click asks for GPS, as before. The system pointer is hidden over the whole
  window (`map_pointer_on_picture`).
- **Chrome.** Title "MAP" (bold) with the tab under it, a compass and the zoom readout top right, a legend at the right (the
  player and each icon of the tab, with how many are on the map; the focused blip's icon is lit; **L** hides it), and the
  footer hints of the menus: Back, Legend, Filter. The whole map fades in over 0.3 s and the legend slides in after it.
- **Input.** Over the window the wheel zooms and a drag pans (`MapInput::full_screen`: the cursor is on the map wherever it is);
  A/D or the arrows switch the tab.
- **Screenshots.** `--ui-script "wait 8;key M;wait 2;pointer -18 38"` opens the map in a `play --drive` run and puts the pointer at
  a point of the screen (FEng units); `key L` hides the legend.

## In a race **[ours]**

- **Picture.** While a race with an event map runs, the picture is the event's own `TRACK_MAP_<event id>` of `TrackMaps.bin`
  (the same pictures the challenge series screen uses), which has the route baked in as a white band. It is read on a worker
  thread when the map opens (`WorldMapOverlay::show_event`); until it is in the assets the stage's map stays. A race with no
  such picture keeps the stage's map. The picture is dropped when the map closes.
- **Moving cars.** The racers and the police cars (the HUD's `racer` and `cop` blips, read when the map opens) are drawn on
  every tab, as the car arrow turned to where they point: a racer orange, a cop white, or blue and red in a chase
  (`WorldMapFrame::live`). They are not listed, cannot be focused and have no GPS. The package has no objects for them, so the
  original-look screen gives each a pool object that no blip of the tab uses; the Remastered map draws them itself.

## Differences from the executable

- **Zoom All** is the Z key. The executable has no Zoom-All message; it reaches zoom 1 by zooming out.
- **The GPS confirm** reuses the shared `Dialog.fng` Yes/No, not the executable's separate GPS dialog.
- **The map's own FEng list** (Original look) scrolls to keep the focused blip in view; the themed legend scrolls by
  the wheel instead.
- **Focus** does not detach on a drag (see above).
- **Place names**: a blip has no name in the data, so the list shows its style's name ("Auto shop", "Sprint"), not
  the place's.
- **Race pools for Event_Rival and Event_Cell** are not drawn: the executable's type 7 and cell icons are not mapped.

## Not verified

- The picture's placement and the blips' positions use the city calibration the minimap uses. The FEng placement
  (centre of each object, y down) follows the presenter and the dump, but was not checked against a rendered frame in
  game.
- The package's start scripts were not traced; the logic writes its objects each tick, which wins over a script that
  only sets them once.
