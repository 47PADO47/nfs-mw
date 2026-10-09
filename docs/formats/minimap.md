# Minimap data (map tiles, calibration, HUD textures)

What the in-game minimap reads from the install: the map pictures cut into tiles, the per-track calibration
that places world coordinates on them, and the HUD textures and objects around it. How the game uses them
each frame is in [../specs/hud-minimap.md](../specs/hud-minimap.md). Evidence tags are explained in the
[docs README](../README.md#evidence-tags); **[verified]** means measured on the install in this repository's
tests or probes.

## The map files **[verified]**

`TRACKS/L2RA/` holds three whole-city maps and one map per event:

| File | Size | Used by the game for |
|---|---|---|
| `MINI_MAP.BIN` | 221,492 B | free roam once the career is far enough on (`CurrentBin < 9`), pursuits outside the other regions; the full city |
| `MINI_MAP_Unlock_1.BIN`, `MINI_MAP_Unlock_2.BIN` | | free roam earlier in the career (`CurrentBin >= 13` and `>= 9`): the same grid with the regions not unlocked yet left black (`Unlock_1` shows only the first, north-western, region and marks its gates with chevrons) **[decomp; the pictures looked at]** |
| `MINI_MAP_<event id>.BIN` (264 files) | | the minimap of one race; `_R` files are the reversed routes **[unconfirmed]** |

A file is a plain sequence of 64 top-level chunks `0x0003A100 CompTPKBlock` and nothing else. The payload of
each chunk in the original PC install is a bare JDLZ blob that inflates to a complete TPK of exactly one texture (17,152 bytes:
[textures.md](textures.md#linking-textures-to-models) has the container). The texture is:

| Property | Value |
|---|---|
| Size | 128 x 128, one mip level |
| Format | DXT3 (`Dxt3`), alpha 255 everywhere, blend type 2 (additive, see below) |
| Pack name | the file stem in upper case (`MINI_MAP`, `MINI_MAP_UNLOCK_1`), file name `Location2\RegionA\Pipeline\PC\Temp.tpk` |
| Texture name | `<pack name>_CHOP<n>`, `n` = 0..63; its name hash is `bStringHash` of that name (all 192 checked) |

**The tiles form an 8 x 8 grid, numbered along the rows from the top left** (`n = row * 8 + column`; the
chunk at position `n` of the file is tile `n`). Stitched, they give one 1024 x 1024 picture of the whole
city, seamless between tiles **[verified by looking at it]**: grey roads on dark ground, water in blue, the
highways leaving at the west edge. **Picture right is world +x and picture up is world +y** (the render space
of [world.md](world.md): x forward, y left, z up). The game draws each tile 128 pixels wide on the 640 x 480
HUD, so the whole picture is 1024 HUD units wide and one world metre is `1024 / width` HUD units (0.154 for
the whole city: a tile is 832 m wide).

The picture is additive: the tile texture's blend type is 2, so black is "nothing" and the map brightens what is
behind it. See the spec for how the HUD draws it.

### Replacement maps **[verified]**

A modded PC install has the same 64-chunk grid with higher-resolution tiles and mixed compression wrappers:
the full map contains 60 JDLZ and 4 HUFF blocks, `Unlock_1` has 33 JDLZ and 31 HUFF, and `Unlock_2` has 48 JDLZ
and 16 HUFF. The reader detects each block's wrapper separately using `ea-compress`; it also accepts the stored
RAWW wrapper supported by that library. The authored HUD tile size and world calibration are independent of
the replacement texture's pixel dimensions. Original map files remain unchanged.

## Calibration: `TrackInfos` **[verified]**

`GLOBAL/GlobalB.lzc` holds one chunk `0x00034201 TrackInfos`: 18 records of 0x120 (288) bytes with no header
(the payload starts with the first name). Layout (the fields the minimap needs are in bold; the rest is the
decompiled `TrackInfo` class and was not needed):

| Offset | Type | Field |
|---|---|---|
| 0x00 | char[32] | Name (`Most Wanted World`) |
| 0x20 | char[32] | TrackDirectory (`Location2\Track_2000`) |
| 0x8A | i16 | **TrackNumber** (2000 for the open city) |
| **0xAC** | f32 x 2 | **TrackMapCalibrationUpperLeft** (x, y): the world position of the picture's left edge and bottom edge (see below) |
| **0xB4** | f32 | **TrackMapCalibrationMapWidthMetres**: the world width of the whole picture, square |
| 0xB8 | u16 | TrackMapCalibrationRotation (0 in every record; not read by the HUD code) |
| 0xC0 | f32 | TrackMapZoomFactor (0 in every record) |
| 0x110 | f32 x 2, f32, u8 | TrackMapZoomTopLeft, ZoomWidth, StartZoomed (0 in every record) |

Track 2000 (`Most Wanted World`): origin `(-1224.8894, -1591.1449)`, width `6659.332` m. The other 17 records
(test tracks, the `27xx` speedway tracks, Carbon) share `(-1650.887, -1294.311)` and `6727.189`.

**The name "upper left" is wrong for the y axis.** The map position of a world point is
`u = (x - origin.x) / width`, `v = (origin.y - y) / width + 1` (v measured down the picture), so `origin.y` is
the world y of the picture's *bottom* edge and the top edge is `origin.y + width`. Checked by placing the
centres of all 90 drivable streaming sections of [maps.md](maps.md) on the stitched picture: every one lands on
land, mostly on roads, and the picture covers x from -1225 to 5434 and y from -1591 to 5068.
The picture's scale is therefore 6.50 m per picture pixel.

## HUD textures and objects **[verified]**

The package is `HUD_SingleRace.fng` (`GLOBAL/InGameB.bun`, [frontend.md](frontend.md)). Names are hashes
(`FEHashUpper`); only these are the minimap's:

| Name (hash) | What |
|---|---|
| `TRACK_MAP` (`0x0f365871`), guid `0xe2991` | root group at (-221, 139, 200): the four pieces below |
| `TRACK_MAP1` ... `TRACK_MAP4` (`0xf60166c2`..`c5`) | multi images, 128 x 128, at (-64,-64), (64,-64), (-64,64), (64,64) in the group, in that order; main texture `Track2000_map` (a placeholder, replaced at run time by a tile), mask texture 1 = `MINIMAP_MASK` (flag 1 = mask), the mask rectangle of each piece is (-0.5,-0.5)-(0.5,0.5), (0.5,-0.5)-(1.5,0.5), (-0.5,0.5)-(0.5,1.5), (0.5,0.5)-(1.5,1.5): the mask texture covers the 128 x 128 square around the centre of the group |
| `TRACKMAPTARGETRING` (`0x382d2fc9`) | group at (5, 132, 414) inside the group `0x157c5b37` at (-226, 7, 136): the backing disc `MiniMap_Backing_Color` (136 x 136, mirrored in x, alpha 200, depth behind everything) |
| `PLAYERCARINDICATOR` (`0xdd9ef5ff`) | image `Minimap_Icon_Car` at (-222, 139, 20), 14 x 16: the player's arrow |
| `PLAYERCARINDICATOR2` (`0x917db611`) | second player (split screen); not in this package's tree (the game finds nothing) |
| `MINIMAP_NORTH_INDICATOR`, `MAP_COLOR_TINT` | looked up by the game; absent from this package |
| `HELICOPTER_ICON_GROUP`, `MMICON_COPCAR_0..7`, `MMICON_AIRACER_0..7`, `MMICON_CHECKPOINT`, `Minimap_Icon_*` | the blips (cops, racers, event and shop icons); they need the race and pursuit state and are out of scope |

The group `0x157c5b37` and `TRACK_MAP` both answer the messages `FADEIN` and `FADEOUT` with a 600-tick colour
fade (scripts `0x5b0d9106` / `0xbcbfcc87`).

Textures (searched in the packs the HUD already uses, [textures.md](textures.md)):

| Texture | Where | Size, format | Content |
|---|---|---|---|
| `MINIMAP_MASK` (`0x40cdc515`) | `GLOBAL/InGameA.bun` | 256 x 256 DXT3, blend 2 | a disc of radius 126 px: alpha 204 (80 %) inside, 0 outside; colour black |
| `MINIMAP_ICON_CAR` | `GLOBAL/InGameA.bun`, `FRONTEND/FrontB.lzc` | 32 x 32 DXT3, blend 1 | a white arrow with a black outline **pointing up** |
| `MINIMAP_BACKING_COLOR` | `GLOBAL/GlobalB.lzc` | 64 x 64 DXT3, blend 1 | a white disc, tinted by the object's colour |

## Probing it

`cargo test --release -p nfsmw-data --test real_install minimap -- --ignored` reads the three files, checks the
64 tiles of each, the name hashes and the calibration of track 2000 (needs `NFSMW_GAME_DIR`).
