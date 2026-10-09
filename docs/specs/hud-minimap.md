# The HUD minimap (free roam)

How the original game keeps the minimap of the in-game HUD up to date: which map it loads, how a world position
becomes a place on the map, how the view is scrolled and turned around the player, and the (missing) speed
zoom. The data it reads is in [../formats/minimap.md](../formats/minimap.md); the objects it drives are in the
package `HUD_SingleRace.fng` ([feng-runtime.md](feng-runtime.md) section 8).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), under
  `src/Speed/Indep/Src/`: `Frontend/HUD/{FeMinimap.cpp, feMinimap.hpp, FeMinimapStreamer.cpp,
  FeMinimapStreamer.hpp, FeHudElement.cpp, FEPkg_Hud.cpp}`, `World/TrackInfo.hpp`,
  `Frontend/FEngInterfaces/FEngInterfaceFEObjects.cpp`, `bWare/Src/bMath.cpp`, and
  `Frontend/Database/FEDatabase.cpp`. The decompiled `Minimap::SetupMinimap` and `Update` are marked unsolved
  (they do not match the binary byte for byte), and the globals `MinimapMaxSpeed` and `MinimapPivotX/Y/DispX`
  have no value in the sources. So `speed.exe` v1.3 was disassembled locally (`Minimap::Update`, `SetupMinimap`,
  `UpdateTrackMapArt`, `UpdateElementArt`, `ConvertPos`, `GetVehicleVectors`, `bATan`) to check every formula
  below and to read the globals from its data section. No code was copied.
- **Evidence tags** as in the [docs README](../README.md#evidence-tags). **[decomp]** = the decompiled source,
  **[binary]** = confirmed in the disassembly of `speed.exe` 1.3, **[verified]** = measured on the install.

## 1. Where it is shown

- The minimap is a HUD feature (bit `0x10000`). The game turns it on only for the open city (track number 2000,
  free roam and the races inside it) and not in an online race; the player's setting turns it off too
  (section 6). **[decomp]**
- Its objects are `TRACK_MAP` (the group with the four map pieces), `TRACKMAPTARGETRING` (the backing disc) and
  `PLAYERCARINDICATOR` (the arrow); the blip images are described in section 7. Showing the feature makes all of
  them visible; each frame then only moves them. **[decomp]**
- Host validation: the rewrite keeps the map hidden if its calibration is non-finite or has a non-positive
  width, if a named tile is missing, or if a piece has no readable mask. This prevents invalid positions and
  unclipped squares when an install is incomplete or modified; the speedometer and other HUD elements remain
  usable. This is a robustness rule of the rewrite, not a claim about the original's error handling.

## 2. Which map

For the player's own HUD (not the drag HUD) the game picks the file from the situation: in free roam by the
career's current blacklist rank (`>= 13`: `MINI_MAP_Unlock_1`, `>= 9`: `MINI_MAP_Unlock_2`, else `MINI_MAP`); in a
race `MINI_MAP_<event id>`; in a pursuit race the same three by region. The file name's stem, upper case, is the
texture header (`MINI_MAP`, `MINI_MAP_UNLOCK_1`, ...). If the file does not exist there is no minimap. The 64 tiles
are held compressed and a tile is inflated when a piece needs it. **[decomp]**

## 3. The projection

Per track the game keeps an origin `(ox, oy)` and a width `W` in metres ([minimap.md](../formats/minimap.md#calibration-trackinfos)).
A world position `(x, y)` (render space: x forward, y left; the game builds it from its physics position
`p` as `(p.z, -p.x)`) lands on the map, in units of the whole picture, at

```
mx = (x - ox) / W
my = (oy - y) / W + 1            # 0 at the top edge, 1 at the bottom edge
```

**[binary]** (`ConvertPos`). A heading is turned the same way: the car's forward vector `f` gives the
world direction `(dx, dy) = (f.z, -f.x)`, and the *bearing*

```
bearing = atan2(dx, dy)  in degrees, 0..360        # from world +y (up the map) turning towards +x (right)
```

(the game's `bATan(dir.y, dir.x)` takes its arguments as `(x, y)`, so the roles are swapped) **[binary]**. A car
heading up the picture has bearing 0, heading right 90. All positions on the HUD are in HUD units (640 x 480,
origin at the centre, y down); the picture is 1024 units wide.

## 4. Each frame (`Minimap::Update`)

1. **Mode.** `rotate_with_player = 1`, except that it is 0 in split screen or when the setting for this situation
   says "fixed" (section 6). **[decomp, binary]**
2. **Player.** `pos` and `dir` as above; `speed` = absolute speed of the vehicle, m/s. `bearing` is the
   `mPolyRotation` of the game; `target = ConvertPos(pos)` is the player on the map (units of the picture).
3. **Zoom.** `speed` is clamped to `[0, MinimapMaxSpeed]` (`MinimapMaxSpeed` = 100 m/s in the binary's data), then
   `zoom = 1 - speed / MinimapMaxSpeed`, and `zoom` is set to 1 if it is below 1. **The result is always exactly 1:
   there is no speed zoom in the retail game** **[binary]** (`1 - s/100` is at most 1, and the code raises
   anything below 1 to 1; the compare and the store were read in the disassembly). The rest of the algorithm multiplies by `zoom`, so a build
   that fixed the formula would zoom; the Rust code keeps the rule and its factor so this stays visible. Note
   that `SetupMinimap` runs before the zoom is recomputed, so it uses the value of the previous frame.
4. **Tiles (`SetupMinimap`).** With `n = 8` tiles per side: `(a, b) = (mx * n, my * n)` for the player,
   `X = trunc(a)`, `Y = trunc(b)`, `fx = a - X`, `fy = b - Y`. The four tiles around the player are the two
   columns and two rows nearest to it:

   ```
   columns: fx < 0.5 -> X-1, X        else X, X+1
   rows:    fy < 0.5 -> Y-1, Y        else Y, Y+1
   tile numbers (top-left, top-right, bottom-left, bottom-right) = row * 8 + column
   offset (ox_t, oy_t): fx if fx < 0.5 else fx - 1, the same for y     # the player relative to the centre of the 2 x 2 block, in tiles, -0.5 .. 0.5
   ```

   **[binary]** A tile number below 0 or above 63 is looked up all the same (the numbers do not wrap by
   column, so near the left or right edge of the picture the neighbouring row's tile is used, and past the last
   row nothing exists); the picture has no content there (open sea or the edge of the world). The four pieces
   `TRACK_MAP1..4` get the textures `<header>_CHOP<number>`.
5. **Scrolling.** `(dx, dy) = (ox_t, oy_t) * zoom * 128` HUD units (128 = the width of a piece). The group
   `TRACK_MAP` is placed at `default - (dx, dy)` with its pivot at `(dx, dy) + (MinimapPivotX, MinimapPivotY)`
   (both globals are 0 in a 4:3 HUD), `default` being its position in the package (-221, 139). A point of the
   group at `p` appears at `default + R(p - (dx, dy))`, so the player's spot is at `default` whatever the
   rotation `R`. **[binary]**
6. **Mask.** Each piece's mask texture (the disc) is attached to the piece's own coordinates; to keep the disc
   fixed on the screen while the group moves, the mask rectangle of every piece is its authored rectangle
   shifted by `(-ox_t * zoom, -oy_t * zoom)` (in mask units: 1 = the mask's 128 units). The piece's own texture
   rectangle (the part of the tile that is shown) is the whole tile for `zoom = 1`; in general piece 0 shows
   `(zoom-1 .. 1)` in both axes, piece 1 `u: 0 .. 2-zoom, v: zoom-1 .. 1`, piece 2 `u: zoom-1 .. 1, v: 0 .. 2-zoom`,
   piece 3 `0 .. 2-zoom` in both. **[binary]**
7. **Turning (`UpdateTrackMapArt`).** Fixed mode: the arrow `PLAYERCARINDICATOR` is rotated by `bearing` (clockwise
   on the screen, as FEng's positive z rotation), the group by 0, the north marker by 0. Rotating mode: the arrow
   by 0, the group (and the north marker, which this package does not have) by `-bearing` about the pivot of
   step 5, so the heading points up the screen. In fixed mode `bearing` is used as above and after
   `UpdateTrackMapArt` `mPolyRotation` is set to 0 for the blips. **[binary]**

What the screen shows is the disc of the mask (alpha 80 %, 128 units across, so 832 m at the whole-city scale)
of the picture around the player, over the backing disc, with the arrow in its centre. The backing
(`MiniMap_Backing_Color`) is part of the package and needs no code.

## 5. Drawing the picture

Each piece is a multi image: its own texture (the tile, additive: black adds nothing) through its first extra
texture (the mask, flag "mask"). The rule the rewrite uses is the one already used for the gauges
([feng-runtime.md](feng-runtime.md) section 6): the piece's alpha times the mask's alpha. A piece's mask
rectangle may extend past the mask texture (it covers `-0.5 .. 1.5`); outside the texture the mask is 0.
The edge of the disc of the 256 x 256 mask texture is at radius 126 of 128 texels, which is 63 HUD units when
the mask covers 128 units. **[verified]** (the extents); the blend of the original's mask in platform code is not in the
decompilation **[inferred]**.

## 6. Settings

Two bytes of the gameplay settings choose the minimap per situation: `ExploringMiniMapMode` (free roam, default 0)
and `RacingMiniMapMode` (in a race, default 1). Value 0 = fixed (north up, section 4 step 7), 1 = rotating,
2 = no minimap (the HUD feature is not even enabled). The options menu cycles through the three. Free roam
therefore starts with the **fixed** minimap. **[decomp]** The rewrite has one setting, `minimap = fixed | rotating |
off` (config file, `NFSMW_MINIMAP`, console `set minimap`), fixed by default.

## 7. Blips (not implemented: a hook)

Every other object on the minimap is placed by `UpdateElementArt(world position, world direction, object, pulse)`
(cop cars and the helicopter, racers, the second player, the checkpoint, event and shop icons from the gameplay
icon list). It needs the AI, pursuit and event state of milestone 7. The placement rule **[binary]**:

```
m   = ConvertPos(position)
e   = (m - target) * zoom                    # in picture units, relative to the player
r   = rotate e by bearing:  (e.x*cos b + e.y*sin b,  e.y*cos b - e.x*sin b)      # b = bearing in rotating mode, 0 in fixed mode
d   = |r|
if d > 0.06:  r *= 0.06 / d;  alpha = 1 if d <= 0.125 else 1 - (d - 0.125) * 9.5238 (0 beyond 0.23); alpha = 1 when pulsing; d = 0.06
if d <= 0.06: screen = centre + r * 1024;  rotation = bearing(direction) - b;  alpha as above;  visible
else:         hidden
```

`centre` is the position of the `TRACK_MAP` group. An element farther than 0.06 (61 units, inside the disc)
is pinned to the edge of the circle and fades out between 0.125 and 0.23; elements beyond 0.23 are hidden. The
Rust library has this function ready and tested; nothing calls it yet.

## 8. Widescreen

`AdjustForWidescreen(true)` (the game calls it when the HUD changes to a wide aspect ratio) moves the whole
minimap 120 units to the left: the pieces' x, the arrow's x and the blip centre by -120 (and sets `MinimapPivotX`
to -120, `MinimapDispX` to -0.9375), `false` moves it back. In this rewrite the HUD is not widescreen aware yet
([architecture.md](../architecture.md#the-hud)); the minimap stays where the 4:3 layout puts it.

## What is not known or not done

- The game's physics position is the rigid body's; the rewrite uses the car model's origin (a few decimetres
  off, invisible at 6.5 m per picture pixel).
- Which of the three whole-city files a free roam run should use depends on career progress that this rewrite
  does not have; it always uses the full `MINI_MAP`.
- Tile numbers outside 0..63 draw nothing. A column past the left or right edge still gives a neighbouring
  row's in-range tile, as described in section 4; the four tile numbers are not clamped by column.
- The look has not been compared with a capture of the original.
