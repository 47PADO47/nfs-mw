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

## Not reversed

- The zoom, pan, filter and list behaviour in the executable (Ghidra) was not traced. The implementation's behaviour
  is from the first reference screenshot.
- The screen is not opened by the port (see `docs/specs/world-map.md`): the overlay is drawn with egui, because the
  list and the info rows are off-screen objects and the blip pools are fixed.
