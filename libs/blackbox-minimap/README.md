# blackbox-minimap

The minimap of EA Black Box games (Need for Speed: Most Wanted): a reader for the map files (an 8 x 8 grid of
JDLZ-compressed one-texture TPKs), the projection from world metres to places on the map picture, and the
placement of the view around the player: which four tiles are shown, how far the group of pieces scrolls, what
the mask and the pieces' texture rectangles follow, how the picture or the arrow turns, and where a blip
(another car, an event icon) goes. **No rendering, no file access:** a host passes bytes in and draws.

- `TileSet::parse` reads a `MINI_MAP*.BIN` file into its 64 tile textures (`blackbox-tpk` `Texture`s); `tile_name`
  gives the name the HUD package refers to a tile by.
- `Calibration::to_map` is the game's world-to-map conversion; `bearing_degrees` turns a world direction into a
  compass bearing.
- `View::new` works out one frame; its methods give the scroll, the mask shift, the pieces' rectangles and the
  rotations for either orientation (`Orientation::North` or `Heading`). `speed_zoom` is the zoom rule (which is
  always 1 in the retail game).
- `place_blip` places another object on the minimap (unused until the game has other cars to show).

Formats: `docs/formats/minimap.md`. Behaviour: `docs/specs/hud-minimap.md`.

License: MIT OR Apache-2.0.
