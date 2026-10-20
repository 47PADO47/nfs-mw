# Provenance: the minimap during a race

Addresses, Ghidra names, method and dead ends for [race-minimap.md](../specs/race-minimap.md). Ghidra auto names
(`FUN_*`, `DAT_*`) are used here only. Image base `0x00400000`; program `/speed.exe` (Black Edition PC v1.3). The
free-roam and pursuit-region rows of the file-name routine were first recorded in
[career-regions.md](career-regions.md) (section "Maps and regions"); this record re-read the routine completely and
followed it to its callers.

## Method

1. The routine that builds the file name was decompiled again in full (read-only queries; nothing was renamed, created or
   saved in the Ghidra project).
2. Its helpers were decompiled: the event id reader, the upper-casing routine, the file test, and the loader that calls it.
3. Its three entry points are not defined as functions in the project (a thunk, a callback thunk and a tail jump into
   it); they were read by disassembling the bytes. The cross-reference tools of the MCP were not reachable in this
   session, so callers come from the callers list of the decompiler tool and from disassembly.
4. Strings were searched (`MINI_MAP`, `TRACK_MAP`, `MINIMAP`, `GPS`, `ROUTE`).
5. The install was checked with the repository's own readers: every `TRACKS/L2RA/MINI_MAP_*.BIN` (tile count and name
   hashes), and every race of `gameplay.bin` against the file its event id names (a real-install test and a throwaway
   dump of the races; no data was copied into the repository).
6. The tiles of `MINI_MAP`, `MINI_MAP_10_2_1`, `MINI_MAP_10_4_2_R` and `MINI_MAP_19_8_32` were decoded and stitched into
   one picture each (a throwaway test, not kept) and looked at. A race (`10.2.1`) was run offscreen with
   `view-world --drive --exec "race start 10.2.1" --screenshot` and the minimap compared with the free-roam one.

## The routine

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Minimap file name | `FUN_0057bec0` | `0x0057BEC0` | cdecl, five arguments: HUD layout (global `DAT_0091CF20`; `2` returns false at once), a 32-byte name buffer, its size, a 64-byte path buffer, its size. Reads the race status `DAT_0091E000`: `+0x1960` racing state, `+0x1968` current race; and the career stage byte `*(*(DAT_0091CF90+0x10)+0xB0)`. Racing state 0: stage `< 9` -> `MINI_MAP` (string `0x008A0DE0`), `< 0xD` -> `MINI_MAP_UNLOCK_2` (`0x008A0DEC`), else `MINI_MAP_UNLOCK_1` (`0x008A0E00`). Racing and a race is set: `PursuitRace` false -> `FUN_005fba70(race)` then `MINI_MAP_%s` (`0x008A0DD4`); true -> `FUN_005faab0(race)`: 0 `_UNLOCK_1`, 1 `_UNLOCK_2`, else `MINI_MAP`. Then every `.` of the name becomes `_` (a byte loop, `0x2E` -> `0x5F`), `FUN_00460f50` upper-cases it, and `TRACKS\L2RA\%s.BIN` (`0x008A0DC0`) is formatted into the path buffer. Returns whether `FUN_0065f7a0(path)` is non-zero. No branch tests the game mode: career, quick race and the console all give a race at `+0x1968` |
| Event id of a race | `FUN_005fba70` | `0x005FBA70` | fastcall on the race: if the pointer at `+4` is non-null the string at `(+4)+4` (an override); otherwise it makes sure the two linked records at `+0xC` and `+0x10` are loaded (`FUN_005fa780`) and reads the attribute `0xA78403EC` of the record at `+8` (`FUN_00454810`), with a default from `FUN_006269b0`. The attribute is `Race::event` in `nfsmw-data` and holds `.r` for reversed races. 16 callers (race UI, scoring, the start routine, the `"19.8.31"` and `"16.1.0"` special cases) |
| Upper-case a string | `FUN_00460f50` | `0x00460F50` | in place, `a`-`z` only; also used for the `MW-%d-%.4sXXX` profile name |
| Formatted copy | `FUN_004647e0` | `0x004647E0` | `sprintf`-like; the event id string reaches the `%s` through the register the previous call left it in (the decompiler shows no argument) |
| File test | `FUN_0065f7a0` | `0x0065F7A0` | looks the path up (`FUN_0065f680`), returns the entry number plus one, or 0 when no such file; also used for `NFS\ZDIR.BIN`, `GLOBAL\INGAMEC.BUN`, `TRACKS/L2RA_Sunset.BUN` |
| `PursuitRace`, `Region` readers | `FUN_005fbe70`, `FUN_005faab0` | `0x005FBE70`, `0x005FAAB0` | keys `0x2B1F54F6`, `0xCB01E454`; see the regions record |

## Who calls it, and when

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| HUD loader | `FUN_005a0710` | `0x005A0710` | fastcall, a state machine on the counter `DAT_0091CF14` (incremented on entry). Step 1: builds the name with `FUN_0057bec0(DAT_0091CF20, ...)`; if it returns true, formats the name into the object at `DAT_0091CEFC+8`, starts an asynchronous load of the file with `FUN_00661760(path, 4, ...)` (the global `DAT_0091CF1C` keeps the handle), hashes the name (`FUN_00460bf0`) and registers the callback `0x00567120`; if it returns false it enters itself again, so the next step runs and no minimap tiles are loaded. Step 2: `FUN_0058c8b0(layout, buffer)` picks the HUD package (`GLOBAL\HUDS_Custom_%2.2d.bin` for layout 1) and the HUD index. Step 3: sets `DAT_008F511C = 2` and finishes. The racing state is read when step 1 runs, once, not per frame |
| Entry stubs of the loader | undefined | `0x005A64F0`, `0x00567120`, `0x0058CA0E` | `mov ecx, [esp+4]; jmp 0x005A0710` (twice) and a tail jump at the end of a routine at `0x0058C930` that sets up the HUD's elements by hash (`0xC3383B63`, ...) before it. The routines above them are not defined as functions and were not traced to the screen that starts the HUD |
| Race start | `FUN_0060dbd0` | `0x0060DBD0` | calls `FUN_00602010` (sets `race status +0x1960 = 1`, counts the objects, special-cases the event `16.1.0` by string compare), fires `Game_StartRace` (`FUN_004c2340`), applies `Barriers` (`FUN_005fe330`). Referenced as data from `FUN_0061E750` (`0x0061E9EC`). It does not load a HUD itself |
| Quick race list | `FUN_007aac70` | `0x007AAC70` | (regions record) the quick race picks among the same race records, so its race is a normal race at `+0x1968` |

## Strings looked at

| String | Address | Result |
|---|---|---|
| `MINI_MAP`, `MINI_MAP_%s`, `MINI_MAP_UNLOCK_1`, `_2` | `0x008A0DE0`, `0x008A0DD4`, `0x008A0E00`, `0x008A0DEC` | all four are used by `FUN_0057bec0` only |
| `TRACK_MAP`, `TRACK_MAP_UNLOCK_%d`, `TRACK_MAP_`, `TRACK_MAP%d` | `0x0089B148`, `0x0089B154`, `0x0089B168`, `0x008A2034` | the world map picture names and the HUD pieces; not the minimap tiles |
| `MiniMap Chop`, `MiniMap Chop embedded` | `0x008A04DC`, `0x008A04C4` | names of the loaded tile resource; not followed |
| `GORacingMiniMap`, `GOExploringMiniMap` | `0x0089D0FC`, `0x0089D10C` | not followed (see dead ends) |
| `ShowGPS`, `EGPSFinished`, `EGPSLost` | `0x008A4AEC`, `0x008A6CCC`, `0x008A6CE4` | the GPS; unrelated to the tiles ([gps-route.md](../specs/gps-route.md)) |
| `ROUTE` | none | no string; nothing named as a route overlay |

## Install data used

- `TRACKS/L2RA/MINI_MAP_*.BIN`: 264 event files (55 `_R`), 3 whole-city files. Every file reads (JDLZ or HUFF per block)
  with 64 tiles and every tile's name hash equal to the hash of `<stem>_CHOP<n>`.
- `GLOBAL/gameplay.bin`: 255 race records, each with the event attribute; 42 ids end in `.r`; 37 have `PursuitRace`
  true; 212 of the 218 others have their file.
- The stitched pictures: the white route band, green start bar and yellow checkpoint dashes are in the tiles; tiles
  away from the route are black.

## Dead ends and things not followed

- Which screen or message starts the HUD loader and whether a race end loads the HUD again (the stubs at the end of the
  table are not defined functions and the cross-reference tools were not reachable). The spec says the rule is "file at
  HUD load", and the rewrite follows the race instead of reloading.
- What the four minimap pieces show when the file is missing (the loader skips the tile load).
- What `GORacingMiniMap` and `GOExploringMiniMap` do (possibly object or message names that change the minimap's
  look between a race and free roam); their users were not found.
- Whether the HUD asks for a minimap in the layouts other than 1 and 2.
