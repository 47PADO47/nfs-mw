# Provenance: career flow, safe house, car lot and the save

Addresses, Ghidra names, files read, method and dead ends for [career-flow.md](../specs/career-flow.md),
[career-car-lot.md](../specs/career-car-lot.md) and [career-save.md](../formats/career-save.md). Ghidra auto names
(`FUN_*`, `DAT_*`, `LAB_*`) are used here only. Image base `0x00400000`; program `/speed.exe`;
`D:\Need For Speed Most Wanted Black Edition\speed.exe`.

## Method

1. A full decompile of all functions was searched by regex; the Ghidra MCP gave string xrefs, raw memory and disassembly.
2. Many screen handlers, script hooks and vtable methods are **not defined as functions** in the project (they show as `LAB_*`),
   so they were read by disassembling the bytes from the file with a small script (PE section map plus Capstone).
3. The saves (`NAME`, `mwo`, `stock`) were parsed with a script that checked each block boundary against the code's sizes
   (the controller block ends exactly where the MD5 starts, in all three).
4. `GLOBAL/FE_ATTRIB.bin`, `fe_attrib.bak` and `gameplay.bin` were read through a throwaway Rust program on top of
   `blackbox-attrib` (kept outside the repo).
5. Labels were resolved with `nfsmw strings 0x<hash>`.

## Save serialization

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| FEDatabase global | `DAT_0091cf90` | `0x0091CF90` | `+8` default stable hash, `+0xC` current player, `+0x10/+0x14` profiles, `+0x18` race-type index, `+0x1C` 11 x 0x18 selection table, `+0x128` snapshot buffer, `+0x12C` context flags, `+0x134` race settings |
| FEDatabase constructor | `FUN_005992e0` | `0x005992E0` | allocates the 0x9CB8 profile, sets `DAT_008f8718` from the file `.\GLOBAL\INGAMEC.BUN` |
| Profile save | `FUN_0058fd10` | `0x0058FD10` | `UserProfile::SaveToBuffer`; the order of section 2 of the format page |
| Profile load | `FUN_005acb70` | `0x005ACB70` | `LoadFromBuffer(profile, buf, size, flag, player)`; hash check at the third step |
| Settings save | `FUN_0057f7b0` | `0x0057F7B0` | writes the 0x4000 blob first (`FUN_005f7410`), then the fields, then the three tail tables |
| Settings load | `FUN_005ac950` | `0x005AC950` | flags merged `(old | stored) & ~0x800` |
| Gameplay blob save / load | `FUN_005f7410` / `FUN_005f81d0` | `0x005F7410` / `0x005F81D0` | MD5, tag `Game`, version 8 |
| Node list save / load | `FUN_0056d810` | `0x0056D810` | 300-entry cap, fixed 4,804 bytes |
| Hint table save | `FUN_0056d8c0` | `0x0056D8C0` | 57 x 8 bytes at `DAT_0091cd10` |
| Pending popup queue | `DAT_0091d928` | `0x0091D928` | 63 x 12; add `FUN_00576c50`, check `FUN_00576f10`, reset `FUN_00576bd0` |
| MD5 writer / checker | `FUN_0057f920` / `FUN_0057f9f0` | `0x0057F920` / `0x0057F9F0` | init constants `0x67452301...`; helpers `FUN_0065b6e0`, `FUN_0065b760` |
| Controller table save / load | `FUN_0056e180` / `FUN_0056e200` | `0x0056E180` / `0x0056E200` | 19 x 0x30 at `DAT_0091f41c`, device hash from `FUN_00634da0` |
| Race settings save / load | `FUN_0057ffb0` / `FUN_00580080` | `0x0057FFB0` / `0x00580080` | 54 bytes from `FEDatabase + 0x134` |
| Buffer size | `FUN_0057fd90` | `0x0057FD90` | `(0x30 + 0x825) x 0x13 + 0x59F9` = 63,560 |
| Unsaved-change test | `FUN_00599570` | `0x00599570` | serializes again and compares with the snapshot at `+0x128` |
| Snapshot take / restore | `FUN_00599500` / `FUN_005ace40` | `0x00599500` / `0x005ACE40` | restore calls `LoadFromBuffer` |
| FEDatabase init | `FUN_005ab900` | `0x005AB900` | computes the default stable checksum with `FUN_0045ca60` |
| Settings reset | `FUN_0057f4d0` | `0x0057F4D0` | stage 16, SMS table init |
| Container header | `FUN_007f4104` | `0x007F4104` | magic `0x4D433032` |
| Container checksum | `FUN_007f73e1` | `0x007F73E1` | table at `0x00908248`; polynomial `0x04C11DB7` |
| Container write | `FUN_007f4f11` | `0x007F4F11` | state machine; open flags `0x8301`; writes header, descriptor, profile |
| Container read / validate | `FUN_007f5255` | `0x007F5255` | the buffer checksum failure is overwritten at `LAB_007f5690` |
| Save / load entry | `FUN_005188f0` / `FUN_005189f0` | `0x005188F0` / `0x005189F0` | profile serialize + hand to the writer |
| Save root | `FUN_006cbf00` | `0x006CBF00` | `SHGetFolderPath(0x8005)` + `\` + string at `0x00901748` |
| Underground 2 folder check | `FUN_00518b40` | `0x00518B40` | then `FUN_00518570` pops the dialog and calls `FUN_0056d7c0` (+10,000) |

Corrections and additions found by the format work are listed at the end.

## Stable and records

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Stable constructor | `FUN_005902e0` | `0x005902E0` | 200 handles = -1, flags 0 |
| Default stable build | `FUN_00590d40` | `0x00590D40` | slot 0 start car; presets; one record per visible vehicle |
| Preset ride creation | `FUN_00590a20` | `0x00590A20` | flags `0xF0010`; handle = name hash |
| Record flag init | `FUN_005815b0` | `0x005815B0` | class bits from frontend field `+0x38`; `0x20` or `0x1` |
| Career record init / alloc | `FUN_00580d60` | `0x00580D60` | strike limit 3 at `+2` |
| Add career car | `FUN_00599d20` | `0x00599D20` | refuses at more than 24 owned (25 slots); clones by handle with flags `0xF0002` |
| Add car by preset (reward) | `FUN_005a41e0` | `0x005A41E0` | sets settings flag bit 20 for the E3 BMW |
| Reward application | `FUN_005a62d0` | `0x005A62D0` | maps reward kind to the popup types; cash reward adds to `+0xB4` |
| Reward kind table | `0x008F547C` | | `{kind, part, id}` triples |
| Owned-count helpers | `FUN_005810e0`, `FUN_005812c0`, `FUN_005906f0`, `FUN_005811a0` | | owned cars, mask counts, customs, cars not impounded |
| Record visitor | `FUN_00580f40` | `0x00580F40` | sums a per-record value over owned cars (tail totals) |
| Strike/countdown tick | `FUN_00580990`, `FUN_00580a40` | | per race |
| Frontend field accessors | `FUN_00581730` (price), `FUN_005816b0` (vehicle type) | | `+0x4C` price |
| Availability | `FUN_0058a9f0`, `FUN_0058a0d0`, `FUN_0058a5f0` | `0x0058A9F0`.. | mode mask 1, 2, 4; `DAT_00926124` unlock all |
| Customization part table | `FUN_00747bb0` / `FUN_00747c00` at `DAT_009b26a8` | | part id to u16 index |

## Flow

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Front-end update | `FUN_00562ff0` | `0x00562FF0` | picks new career or resume at context bit 8 |
| New career | `FUN_005a3950` | `0x005A3950` | `M3GTRCAREERSTART`, node `16.1.0` |
| Resume career | `FUN_0057f570` | `0x0057F570` | nodes `16.2.1`, `1.2.3`, `1.8.1` |
| Main menu build | `FUN_00544e50` | `0x00544E50` | button handlers `0x0051AC80` (Career), `0x0051AD20`, `0x0051AD70`, `0x0051ADE0`, `0x0051AE50`, `0x0051AEA0`, `0x0051AEF0` |
| Screen factory table | `0x008F4318` | | `{name, factory}` pairs; `MainMenu_Sub.fng` -> `FUN_00570660` |
| Safe house screen | `FUN_00549280`, `FUN_00532220` | | buttons; handlers `0x0051FF10`, `0x00532080`, `0x005320E0`, `0x00532150`, `0x005321C0` (vtables `0x0089C428..`) |
| Screen request object | `FUN_0062fe40` / `FUN_00623f80` | | constructor and executor |
| Platform names | `FUN_00516e40`, `FUN_00516e90` | | scene files and tags |
| Zone enter | `FUN_0058c640` | | `safehouse` / `carlot` / `customshop` |
| Game mode selection | `FUN_00662950` | `0x00662950` | `CareerGame` / `QuickGame` / `OnlineGame` -> `FUN_006fc3a0` |
| State names | strings near `0x008A8CDC` | | `BeginGameFlowLoadTrack`, `BeginWorldLoad`, ... |
| Script hook registry | `FUN_0061e750` | `0x0061E750` | hook bodies at `0x00605560` (DoSafeHouseIntro), `0x00605610` (IsCareerMode), `0x00605660` (AllowEngageSafehouse), `0x00605670` (SetHasRapSheet), `0x00612490` (SkipCareerIntro) are undefined in Ghidra |
| Career start point | `FUN_005e4cf0`, `FUN_005e4df0` | | `career_start_%s`; fields `0x9F743A0E` (position), `0x5A6A57C6` (heading) |
| Blacklist arrival stepper | `FUN_005302c0` | `0x005302C0` | table at `0x008F322C` |
| Autosave test | `FUN_005184d0` | `0x005184D0` | |
| Autosave event | `FUN_00620920` / `FUN_00620930` | | type `0x10`; posted by `FUN_005fe3e0` (SetRoaming) and `FUN_00602010` (race end) |
| Autosave run | `FUN_00563d10` | `0x00563D10` | calls `FUN_00562ab0` |
| Car Select class | `FUN_007c2ba0`, message handler `0x007C1770` | | purchase and sale |
| Car list builder | `FUN_007bf7b0` | | mode `local_c`: 2 career, 1 quick race, 7 online |
| Shop checkout | `FUN_007bab60` | `0x007BAB60` | spend and refund; `FUN_0056d7a0` spend (saturates at 0) |
| My Cars | `FUN_007b9740` | | customs, limit 0x14 |
| Statistics accessors | `FUN_005a3c30`, `FUN_005a3f10` | | category names and counter offsets |

## Dead ends and caveats

- The names `CStartNewCareer`, `CResumeCareer`, `CLoadCareer`, `MainCareer`, `uiCareerManager`, `uiCareerCrib` are only
  referenced from exception-unwind tables (scope names of stack objects); they do not identify handlers.
- `DoSafeHouseIntro`, `IsCareerMode`, `SkipCareerIntro` and the other hook bodies were decoded from raw bytes; their
  exact side effects on the gameplay manager (+0x108, +0x10C, +0x110) are read, not observed.
- The meaning of the 55 + 39 twenty-byte tables of the gameplay blob was not found (they are copied from two arrays of the
  manager at `+0xA8` and `+0xB0`).
- The car-lot purchase handler (`0x007C1770`) is one big undefined function; the dialog text hashes were looked up with
  `nfsmw strings`. Its sale path divides the price by two.
- Lua handlers (`stategraph_menu_gate_*`, `drive_to_target_*careerintrodone`, `race_bin_*`) were only listed by name from
  the plain strings of `gameplay.bin`; the bytecode was not decompiled.
- The save in `mwo` has a different default stable hash, a different car key set and 0xFFFFFFFF as the controller device
  hash. It would not load in an install with the stock car data; this is consistent with the load rule.
- `FUN_005ac7d0` / `FUN_005ac6f0` are called around every settings read; they are no-op stream hooks as far as the
  offsets of the real saves show (all offsets match without a displacement).

## Corrections for `docs/formats/saves.md` (not edited)

1. Bytes 0x10..0x1B are three CRC-32 checksums (descriptor, profile, header), not "unknown 16 bytes"; 0x1C..0x23 is the
   descriptor `{0x10D, size}`, not just "the size again". The file is header + descriptor + profile.
2. The MD5 range is correct; the profile "version" field is the first 16 bytes of the profile buffer.
3. Profile order: after the settings come the default stable hash, name, **playlist, options**, stable, completed byte,
   then the statistics block, **11 quick-race car handles**, a 54-byte race settings block and the **controller bindings**
   (916 bytes), then the MD5. The `saves.md` list omits the last two blocks.
4. "cars (array)" at 0xE2ED are the 25 career records, not the 200-slot car table (which starts at 0x5BC5).
5. "high scores" is the career statistics block; the "pursuit records" at 0xF2BA are the five most recent pursuits.
6. `TheCareerSettings` is not a small struct: it holds the 0x4000-byte gameplay blob, cash, current car, stage, flags,
   the SMS table, the node list, hint counters and the popup queue (23,033 bytes).
