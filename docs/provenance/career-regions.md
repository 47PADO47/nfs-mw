# Provenance: regions, barriers and the region unlock screen

Addresses, Ghidra names, method and dead ends for [career-regions.md](../specs/career-regions.md). Ghidra auto names
(`FUN_*`, `DAT_*`, `LAB_*`) are used here only. Image base `0x00400000`; program `/speed.exe` (Black Edition PC v1.3).

## Method

1. The full decompile of every function was searched by regex (strings, constants, field offsets, callers). The
   region screen was found from the string `Global\Rivals.bin` (three users) and from the factory table of the
   Blacklist stepper.
2. Several routines are **not defined as functions** in the project (vtable methods, async callbacks, script hooks,
   `LAB_*`). They were read from the file bytes: the Ghidra `read_memory` call plus a Capstone disassembly of the PE
   (a throwaway script). Jump targets of `jmp rel32` thunks were resolved by hand.
3. Name hashes were identified by computing `bStringHash` (seed `0xFFFFFFFF`, `h*33+c`) over candidate names and over
   the `RIVAL_NN...`, `TRACK_MAP_UNLOCK_n`, `MINI_MAP*` and label names, and the Jenkins lookup2 vault hash
   (`blackbox-attrib`) over the field names of `gameplay.bin` for attribute keys.
4. Data was read with throwaway programs outside the repo that depend on the repo's crates by path: `blackbox-feng`
   (the `.fng`), `blackbox-scenery` (instances of a streaming section), `blackbox-collision` (CARP collision packs),
   `ea-compress` (the `RIVALS.BIN` blobs, the minimap tiles), and small Python parsers for the scenery group,
   override and track path barrier chunks of `L2RA.BUN`.
5. No run-time tool was used. Every claim is a static reading or a check against the install; the spec tags say which.

## The screen

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Screen constructor | `FUN_0055b020` | `0x0055B020` | object size `0x70`; base constructor `FUN_00586290`; screen vtable `0x0089C3FC`; helper embedded at `+0x38` |
| Factory | `FUN_005714a0` | `0x005714A0` | `_malloc(0x70)` then the constructor; registered for `SafehouseRegionUnlock.fng` (the file spells `SafehouseRegionUnlock.fng`) |
| Screen vtable slots | data | `0x0089C3FC` | `+0` `0x005302A0` (destructor, calls `FUN_0051F630` then the helper's `FUN_0051F4B0`), `+4` `0x00548490` (message handler, not a defined function), `+8` `0x0050DE70` (stub, `ret 8`) |
| Message handler | undefined | `0x00548490` | `cmp [esp+4], 0x406415E3 (PAD_ACCEPT)`; if equal `ecx = [0x0091CA48]` and `call 0x005302C0`; `ret 0x10` |
| Rival texture helper constructor | `FUN_0055afa0` | `0x0055AFA0` | vtable `0x0089C3F8` (one slot `0x00530220`); sets key `-1`, loaded key `-1`, flag "loading" = 1, then asks for the pack with callback `0x00551040`; a second form (the flag argument set, the in-game variant) loads through another loader (`FUN_00731040`). Also used by `0x00561130`, `0x00561260`, `0x00561320` (rival bio), `0x007A4680`, `0x007B9A80` (markers) |
| Pack load callback | undefined | `0x00551040` | `mov ecx,[esp+4]; mov byte [ecx+0x18],0; jmp 0x00530240` |
| Bind the screen objects, set label and key | `FUN_005484b0` | `0x005484B0` | looks up objects `0xC1F62308`, `0xF5A2A087`, `0x2CBE1DD0` (type must be 1, image); label via `FUN_00525220(package, 0xD6C0E097, 0x29E4B193 or 0x2B0BCA2D)` when the stage byte is `0x0C` / `0x08`; key `= stage + 1`; hides the objects (`FUN_00514c70`, flags `|= 0x2400001`) |
| Start texture load | `FUN_00530240` | `0x00530240` | if loaded key != key: release old, mark loading, `FUN_005119d0` builds the names, `FUN_00508b10(names, count, callback 0x0052EDB0, helper, priority)` |
| Build the names | `FUN_005119d0` | `0x005119D0` | format strings at `0x0089B1D0` `RIVAL_%02d`, `0x0089B1DC` `RIVAL_%02d_IG`, `0x0089B1AC` `_GRAF`, `0x0089B1BC` `_IG_GRAF`, `0x0089B188` `_BG`, `0x0089B198` `_IG_BG`; sprintf-and-hash helper `FUN_00573140` |
| Texture callback | undefined | `0x0052EDB0` | thunk to `FUN_0051f520` |
| Finish (bind textures, show) | `FUN_0051f520` | `0x0051F520` | per present object: post `0xC0942E85` / `0x8C9D4547` / `0xD22B95D0` to the package (target `0xFFFFFFFC`), set texture field `+0x24`, flags `|= 0x2400000`, `FUN_00514cc0` shows it (clears bit 0) |
| Helper destructor | `FUN_0051f4b0` | `0x0051F4B0` | releases the pack (`FUN_00503380`, `FUN_00507e40` with `"Global\Rivals.bin"`) |
| Stepper | `FUN_005302c0` | `0x005302C0` | `ecx` = object at `0x0091CA48`; counter at `+4`; case 2: `SafeHouseRegionUnlock.fng` only if the stage byte (`[[0x0091CF90]+0x10]+0xB0`) is `0x08` or `0x0C`, else recursive call; pointer table `0x008F322C` (names), see below |
| Screen name table | data | `0x008F322C` | `SafeHouseRivalChallenge.fng`, `SafeHouseMarkers.fng`, `SafeHouseRegionUnlock.fng`, `MC_Main.fng`, `SafeHouseRivalBio.fng`, (two empty), `SafeHouseReputationOverview.fng`, `GenericDialog_MED.fng` |
| Enter-bin routine | `FUN_0062c2d0` | `0x0062C2D0` | loop `stage-1 .. target`: writes the stage byte, `FUN_005e46f0`, `FUN_005e9d60`, rewards; finally `FUN_005dbd70(race status)` which sets the "bin changed" byte at `+0x1998` |
| Pack name | data | `0x0089C3E4` | `"Global\\Rivals.bin"` referenced by `0x0055B020`, `0x0055AFA0`, `0x0051F4B0`, `0x0051F630` |

Package side (read with `blackbox-feng`, package `SafehouseRegionUnlock.fng`, `Global\FEng_Screens\Stripped\...`,
version 131072): 12 resources (`RIVAL_15_BG.tga` is the last, handle `0x87C0920A`), the objects named in the spec, the
package responses `0x13FD3296` -> Button(257, 0), `0x8CB81F09` -> Button(257, 1), `0x406415E3` -> post `0x97BDB465`,
post `0xADE14EC3`, sound `0x7B6B89D7`, Button(257, 0); target list of `0x97BDB465` = 21 objects; the event handler
object `0x07B7C74A` has scripts `0x0013C37B` (500 ticks, events `0xBDFD2DF9` -> `0xFFFFFFFC`, `0x8CB81F09` -> 0,
`0x63E5ED6F` -> `0xFFFFFFFB`) and `0xDE6EFF34` (600 ticks, `0xF786ED9D` -> sound at 0, then `0x8CB81F09` -> 0,
`0xE1FDE1D1` -> game, `0x406415E3` -> game, all at 600), started by `0xADE14EC3`. The Continue group answers
`0x7EABCA56` with the same list as PAD_ACCEPT.

## Barriers

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Script registry entries | in `FUN_0061e750` | `0x0061E750` | `DisableBarriers` -> `LAB_005dbf00`, `EnableBinBarriers` -> `LAB_005e3020`, `EnterGateZone` -> `LAB_00611fb0`, `ExitGateZone` -> `LAB_00611ff0`, `IsActiveMenuGate` -> `LAB_00604e40`, `AllowMenuGates` -> `LAB_00605650` |
| `DisableBarriers` body | undefined | `0x005DBF00` | `call 0x00659660; ecx=[0x009B38B4]; call 0x007653B0; jmp 0x007651A0` |
| `EnableBinBarriers` body | undefined | `0x005E3020` | `ecx = [ecx+0x196C]` (stored bin record); if non-null `jmp 0x005DD090` |
| Enable a bin record's list | `FUN_005dd090` | `0x005DD090` | loops the attribute `0xE244F26B` (`Barriers`), strips a leading `*`, calls `FUN_0064b410(name, starred)`; then `FUN_00782c60` and `FUN_007889d0(DAT_009b38b4)` |
| Enable one entry | `FUN_0064b410` | `0x0064B410` | `FUN_00724ff0` (prefix test), hash `FUN_00460bf0`, `FUN_00725060` (256-slot list at `0x009A3B50+0x6430`), `FUN_0072c5c0` (group), `FUN_0072ba80` (path barriers) |
| Prefix test | `FUN_00724ff0` | `0x00724FF0` | table `0x00903168`, five entries (pointer, int): `BARRIER_`, `BARRIERS_`, `PLAYER_BARRIERS_`, `SCENERY_GROUP_` (int 1), `FREE_ROAM` (int 0); compare `FUN_00460e30` (case-insensitive prefix) |
| Enable a group | `FUN_0072c5c0` | `0x0072C5C0` | finds the group node by hash in the list at `DAT_009AB24C`; `FUN_0072b9c0` writes override flags; state byte `0x00902160[group no]` = 1, `\|2` star, `\|4` if node `+0x11` |
| Write override flags | `FUN_0072b9c0` | `0x0072B9C0` | per entry (6-byte records at `DAT_009A3A64`): `flags = (flags & 0xFBEF) \| (star ? 0x400 : 0)`; `FUN_00728850` pushes them to the instance (`inst+0x18` low word), rebuilds the matrix through `FUN_00722c40` when `0x800000` is set and bit `0x400` changed |
| Enable path barriers of a name | `FUN_0072ba80` | `0x0072BA80` | table `0x009AA540` (count `+0x484`, array `+0x488`, 24-byte records); sets `+0x10` = 1 and `+0x12` = node `+0x11 != 0` for records whose hash at `+0x14` matches |
| Disable all | `FUN_00659660` | `0x00659660` | `FUN_00722b30(0x009AA540)` zeroes `+0x10`; zeroes 256 dwords at `0x009A9F80`; `FUN_0072c660` resets every active group (`FUN_0072ba30` sets bit `0x10` again); `FUN_0064b460` re-enables `SCENERY_GROUP_DOOR` |
| Road flag clears | `FUN_007653b0`, `FUN_007651a0` | `0x007653B0`, `0x007651A0` | over `DAT_009B3874` segments of 0x16 bytes at `DAT_009B38C0`: `+0xB &= 0xCF`; and `+0xB &= 0x7F`, `+0xA &= 0xFB`, `DAT_009B3829 = 0` |
| Recompute road flags | `FUN_007889d0` | `0x007889D0` | clears `+0xB & 0xCF`; exemption roads from the race start points (`FUN_005fcb20`, `FUN_005fb3b0`) or the fixed free-roam point (floats `-2511, 147.8, 1783`, direction `-0.7, 0, 0.7`); for each enabled barrier (`DAT_009AA9C4` count, `DAT_009AA9C8` array), queries the grid and sets `+0xB \|= 0x10` (drive-through byte 0) or `0x20` |
| Set current bin | `FUN_00602bb0` | `0x00602BB0` | `this` = race status (`DAT_0091E000`): if `+0x196C` non-null: `FUN_00659660`, `FUN_007653b0`, `FUN_007651a0`; `+0x196C = FUN_005e3980(DAT_0091E004, bin)`; `FUN_005dd090(record)`; then refreshes the race manager |
| Race status update | `FUN_00602c80` | `0x00602C80` | state `+0x1960` == 1 with `+0x19A5` -> `+0x1998 = 1`; state 0 with `+0x1998` -> `FUN_00602bb0(stage)` and clear |
| Race status constructor | `FUN_00603720` | `0x00603720` | stores the record of the stage at `+0x196C`; if `[DAT_0091E004+0x2C] == 0` calls `FUN_00602bb0(stage)`; creator `FUN_00603900` |
| Race status reset | `FUN_00603500` | `0x00603500` | ends with `if (event node == 0 && [+0x196C] != 0) FUN_005dd090(record)`; called from `FUN_006f9ea0` |
| Race start | `FUN_0060dbd0` | `0x0060DBD0` | calls `FUN_005fe330`: for each entry of the race's `Barriers` (`FUN_005fb420` name, `FUN_005fb4e0` star) `FUN_0064b410`, then `FUN_00782c60`, `FUN_007889d0`. **Before it** (`0x0060DD02` to `0x0060DD09`): `FUN_00460d20` (a plain byte-wise string compare, 0 when equal) of the literal `e3demo` (`0x008A06FC`) against `FUN_005FBA70` (the race's event id: the text field `0xA78403EC`, or a cached copy at `+4`); the clear-all routine `FUN_00659660` runs **only when they are equal**. Read 2026-10-22 from the decompiled routine and its disassembly; no race of the install has that event id (the text `e3demo` is not in `gameplay.bin`) |
| Race status constructor, event test | `FUN_00603720` | `0x00603720` | its tail: `if ([DAT_0091E004 + 0x2C] == 0) { FUN_00602bb0(this, stage); FUN_005fe3e0(this); }`. `DAT_0091E004` is the gameplay manager (`FUN_005e3980(DAT_0091E004, bin)` looks a bin record up in it); `+0x2C` is the manager's current event (`FUN_00603500` hands it to `FUN_005fd370` and applies the stage list only when it is 0). With an event current the stage's list is **not** applied at world creation; the `done` state applies it after the race. Which routine sets `+0x2C` before the world is built was not followed (inferred: the front end's hand-over of a career race) |
| Race/world callers of `FUN_0064b410` | `FUN_0062ef00` | `0x0062EF00` | an event object that enables or disables a named group (`FUN_0072c620` to disable); constructors `FUN_0062efd0`, `FUN_0062f030`; not traced further |
| Collision filter | `FUN_00783b80`, `FUN_00783e50` | `0x00783B80`, `0x00783E50` | instance (0x40 bytes) accepted if `u16 [+0x18] == 0 \|\| 0x00902160[[+0x18]] != 0`, `[+0x1C] != 0`, `(mask & u16 [+0x12]) == 0` |
| Group list load | `FUN_00728c00` | `0x00728C00` | chunk `0x34109`: walks records of size `(count*2 + 0x17) & ~3`, links them into the list at `DAT_009AB24C` |
| Path barrier table load | `FUN_007273c0`, `FUN_00722ab0` | | chunk `0x3414D`: array pointer `+0x488`, count `size / 0x18` (`+0x484`) |
| Override record unload | `FUN_007307e0` | `0x007307E0` | chunk `0x34108` handling |
| Lua use | stategraph `basicrace` / `done` / `stateenter` | | `DoSpecialFinalization`, `RaceStatus:DisableBarriers()`, `RaceStatus:EnableBinBarriers()`; the only occurrences in the 259 chunks (`lua_all.txt` of the earlier script pass) |

## Maps and regions

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Minimap file name | `FUN_0057bec0` | `0x0057BEC0` | roaming (`[race status+0x1960] == 0`): stage `< 9` -> `MINI_MAP`, `9..12` -> `MINI_MAP_UNLOCK_2`, `>= 13` -> `MINI_MAP_UNLOCK_1`; racing: `PursuitRace` false -> `MINI_MAP_%s` (event id), else `FUN_005faab0` (Region index 0 -> `_UNLOCK_1`, 1 -> `_UNLOCK_2`, else full) |
| `PursuitRace` / `Region` readers | `FUN_005fbe70`, `FUN_005faab0` | `0x005FBE70`, `0x005FAAB0` | attribute keys `0x2B1F54F6`, `0xCB01E454`; region table `0x008F5D30`: `college` 0, `coastal` 1, `city` 2 (three entries) |
| Race type reader | `FUN_005faa20` | `0x005FAA20` | attribute `0x0F6BCDE1` against the table at `0x008F5CD8` (`circuit` 1, `p2p` 0, `drag` 2, `knockout` 3) |
| World map screen constructor | `FUN_0055b8c0` | `0x0055B8C0` | region index: stage `>= 13` -> 1, `9..12` -> 2, else 0; passes it to `FUN_00546400` (stores `+0x18`), which loads `TRACKS\L2RA\TrackMaps.bin`; caller `FUN_005614d0` |
| Map texture name | `FUN_00510ac0`, `FUN_0051dbc0` | `0x00510AC0`, `0x0051DBC0` | `+0x0C` set -> `TRACK_MAP_<event>`, else `+0x18` nonzero -> `TRACK_MAP_UNLOCK_%d`, else `TRACK_MAP`; hashes `0x0F365871` (full, entry offset 10,112), `0xCDCF30EC`, `0xCDCF30ED` found in `TrackMaps.bin` |
| Quick race region filter | `FUN_007aac70` | `0x007AAC70` | keeps a race when `Region` index equals byte `+0x0D` of options record 11, or that byte is 3 |
| Loading tips by stage | `FUN_0056ada0` | `0x0056ADA0` | table `0x008F3D80` of (stage mask, ...) triples; mask `1 << stage` when career flag bit 0 is set. Not read further |
| Other stage-byte readers | | | `0x00601790`, `0x005A2390`, `0x0058A5F0`, `0x005529F0`, `0x0059F440`, `0x007AD7D0`, `0x007B3C00`, `0x00547E30`, `0x0052B4D0`, `0x007C2780`, `0x006F13F0` read the byte for unrelated purposes (milestone and rival lookups, icon variants); none opens or closes a region |

## Install data used

- `GLOBAL/gameplay.bin`: all `Barriers` lists (238 collections, 736 distinct names), `Region` of 218 races
  (73 / 85 / 60), the 15 `race_bin_challenge` lists. Dump files from the earlier script pass were reused.
- `TRACKS/L2RA.BUN`: scenery groups `0x34109` at file `0x69C50` (60,552 B, 754 groups), override records `0x34108` at
  `0x490B0` (134,040 B), `0x3414D` at `0x14B8C0` (33,024 B).
- `TRACKS/STREAML2RA.BUN`: scenery sections 425, 920, 932, 1605, 1801 (the gate pieces: 5, 7, 5, 12, 3 instances),
  135 and 902 (the door group); collision packs of the same sections (group numbers 200, 356, 389, 495, 496, 624).
- `TRACKS/L2RA/MINI_MAP.BIN`, `_UNLOCK_1.BIN`, `_UNLOCK_2.BIN`: mean colour-0 brightness per tile (all three decode).
- `GLOBAL/RIVALS.BIN`: key and entry chunks, all 90 blobs decompressed through `ea-compress` (JDLZ or HUFF) for the
  trailer sizes.
- `FRONTEND/FrontB.lzc`: package `SafehouseRegionUnlock.fng`.

## Dead ends and things not followed

- The caller chain above `FUN_006f9ea0` / `FUN_00603500` (the world-loaded path) and the event object
  `FUN_0062ef00` (a scripted group toggle; no gameplay collection was found that uses it for the regions).
- The meaning of the group byte at `+0x10`, of the 256-slot enabled list, and of the star (only the matrix rebuild in
  `FUN_00728850` is known).
- The message ids `0xBDFD2DF9`, `0x63E5ED6F`, `0xF786ED9D`, `0xC0942E85`, `0x8C9D4547`, `0xD22B95D0` were not found
  as names; they are not handled by the game-side message handler of this screen.
- No string "gate", "locked region" or similar warning exists in the language table; searching the language rows
  for barrier, restricted, closed, locked, area, bridge and boundary found only SMS and track names.
- The texture binding of the gate model (`SFX_TrackBarrierNeonLocked`, `SFX_TRACKBARRIERLOCK`) was found by string
  search only; the solid was not parsed.
- Who requests the arrival stepper after a rival win and who calls `FUN_0062c2d0` in normal play (the open item of
  the blacklist spec) was not reopened.
