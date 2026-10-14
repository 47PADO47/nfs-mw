# World map screen: names from WorldMapMain.fng

Source: `nfsmw dump-screen WorldMapMain.fng` on the retail install (`Global\FEng_Screens\Stripped\WorldMapMain.fng`,
FEng version 0x20000). Object names are FEng name hashes (`blackbox_feng::fe_hash_upper`); strings are shown as
the dump prints them. **[verified]** from the dump, not from the executable's code.

## What the package holds

- Resource 17 is `TRACK_MAP.tga` (handle `0x0F365871`): the city's picture. The map's MultiImage (`0xF9984`, name
  `0x0F365871`, at (-111, 11), 340 by 340) draws it. So the full-city picture is in the screen's own package; the
  map does not need `TrackMaps.bin` to show it.
- Resources 11 and 12 are `R1.tga` and `L1.tga` (shoulder buttons). 13 is `Ingame_Icon_Pause_World_Map.tga`
  (the "WORLD MAP" header icon). 18 is `Visibility_WorldMap.tga` (the Toggle Icon Visibility icon).
- Resources 21 to 37 are the blip icons: `Minimap_Icon_Circuit`, `Car`, `Event`, `Event_Rival`, `Speedtrap`,
  `Safe_House`, `Chop_Shop`, `Car_Lot`, `Helicopter_Minimap_Icon`, `Hiding_Spot`, `Event_Cell`, `Roadblock`,
  `Pursuit_Breaker`, `Spike_Strips`. Plus `IconSelection_Glow` (`0x5FBBA846`) and the arrows.

## Objects the legend and the list use

- `LOCATIONS` string, name `0xD259525F`, label `0x9588BD38`, at (173, -15) in group `0xF94CE`.
- `WORLD MAP` string, name `0x863404B5` (the same hash as `PAUSE_HEADER` in `frontend/ids.rs`), label `0xD903F7C4`.
- Control hints: `Toggle` (`0x2991B217`), `Activate` (`0x9ACD367A`), `Exit` (`0x00153DB9`), `Back` (`0x001335F0`),
  `Done` (`0x51F0064F`), `Move Cursor` (`0x2945911D`), `Toggle Icon Visibility` (`0x244F2A79`), `Hide/Show Icon`
  (`0x29456CC8`), `ZOOM x 1.5` (`0xCB76CE5B`).
- Info card: `Item Title` (`0x9331FD4F`) and `Race Length` (`0xFEEEB39B`), with a card image
  (`0xF96A7`, 220 by 50) in group `0xF96AF`.
- Blip pools, parked off the picture (y 315 to 470, the FEng `y` axis points down): safe houses
  `0x3750C9EF`, `C9F0`, `C9F1`, `C9EE` (4); auto shops `0x79E5866A`, `866B`, `8668`, `8669`, `866F`, `866E`,
  `866D`, `866C` (8); car lots `0x008BA2D4`, `D5`, `D2`, `D3` (4); event icons `0x44075303`..`08` (6, column x 97),
  `0x5D8CA830`..`35` (6, x 157), speed traps `0xD4662C05`..`0A` (6, x 247).
- Cursor: `IconSelection_Glow` `0x5FBBA846` (guid `0xFA209`) in group `0xF93EF`, with four `Corner_Crosshair.tga`
  objects (`0xF93AF`, `B2`, `B5`, `B8`).
- The 6 `option_name` BUTTON strings (`0xB2980BA8`..`AD`, x -294) are the option rows; they sit off-screen (y 319 to
  460) on this layout.

## Messages

Package responses and messages (`0x97BDB465`, `0x696851AD`, `0x7EABCA56` = MOUSE_LEFT_RELEASED, `0x587C018B` =
LEAVE_SCREEN, `0x8CB81F09` = INPUT_ENABLE, `0x35F8620B` = INIT_COMPLETE) are as in `frontend/ids.rs`. The message
flow was not traced to the blip or zoom behaviour; the implementation does not use the package's messages.

## Coordinates (assumption)

The dump gives `pos (x, y, z)` with the screen centred on 0,0 and y pointing down (the parked pools sit at y > 240,
off the 480-unit canvas). The picture rectangle is therefore x -281 to 59, y -159 to 181 (centre -111, 11, size 340).
**[assumption]**: the pos is the centre and y points down. This was not checked against a rendered frame.

## How the port uses the package

The port opens the package as a screen and draws the map's state into these objects by GUID (`frontend/world_map/
screen/pools.rs`): the picture `0xF9984` (crop by `set_uv`), the blip pools (per style, below), the list rows (`option_name`
strings `0xF55E3`, `0xFA8B0`..`0xFA8BF`, with their icon groups `0xFC5E6`, `0xFD2EC`..`0xFD300`), the cursor group `0xF93EF`,
the card `0xF96AF` (title `0xF570E`, distance `0xF5710`, icon `0xFA3CE`), the tab label `0xFA3A9` and the zoom readout
`0xF5707`.

Pools the port uses, by blip style (the executable's own hashes; **[verified]** against the trace in
`world-map-ghidra.md`):

| Style | Pool objects (GUIDs, in the order blips take them) |
|---|---|
| Auto shop (8) | `0xFA072`, `0xFA073`, `0xFA070`, `0xFA071`, `0x122C16`, `0x122C1B`, `0x122C20`, `0x122C25` |
| Safe house (4) | `0xF54A4`, `0xF54A1`, `0xF54A2`, `0xF54A3` |
| Car lot (4) | `0xFA07C`, `0xFA07D`, `0xFA07A`, `0xFA07B` |
| Sprint (6) | `0xF50D5`, `0xF50E8`, `0xF50ED`, `0xF50F2`, `0xF50F7`, `0xF50FC` |
| Circuit (6) | `0xF5088`, `0xF5090`, `0xF5095`, `0xF509A`, `0xF509F`, `0xF50A4` |
| Drag (3) | `0xF5129`, `0xF5131`, `0xF5136` |
| Knockout (6) | `0xF51B9`, `0xF51C2`, `0xF51C7`, `0xF51CC`, `0xF51D1`, `0xF51D6` |
| Tollbooth (3) | `0xF51FA`, `0xF5201`, `0xF5206` |
| Speed trap (6) | `0x10502E`, `0x10502F`, `0x105030`, `0x105031`, `0x105032`, `0x105033` |

The other pools (speed trap cameras at x 247, Event_Rival `0xF523C`, Event_Cell `0xFCEF7`, the hiding spots, pursuit
breakers, roadblocks and spike strips) are not drawn by the port.

The executable's zoom, pan, filter, list, cursor and GPS behaviour is in `world-map-ghidra.md`. The port's behaviour
and its differences from the executable are in `docs/specs/world-map.md`.
