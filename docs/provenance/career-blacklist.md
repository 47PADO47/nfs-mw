# Provenance: career, Blacklist and story progression

Addresses, files, method and dead ends for [career-blacklist.md](../specs/career-blacklist.md). Ghidra auto names
(`FUN_*`, `LAB_*`, `DAT_*`) appear only here. Image base `0x00400000`; program `/speed.exe`
(6,029,312 bytes).

## Method

1. The script hook registry `FUN_0061e750` lists names and handler addresses. Many handlers are unnamed `LAB_*` code
   that Ghidra has not made into functions (`get_functions` fails there), so they were read with a linear
   disassembly (Python, capstone, pefile) straight from the file.
2. The career logic is mostly data: `GLOBAL/gameplay.bin` read with `libs/blackbox-attrib` (a throwaway Cargo
   project outside the repo), field names recovered by hashing candidate words (lookup2, init `0xABCDEF00`) taken from
   the Lua constants and the strings of the executable. The 259 Lua 5.0 chunks (vault `gpcore`, HUFF/JDLZ blobs) were
   decompressed with `ea-compress` and disassembled with a small Lua 5.0 reader (instruction layout: op 6 bits low,
   then C 9, B 9, A 8 at the top; `Bx` = bits 6..23; constants at index 250 and up in RK operands).
3. The save-side layout was checked by parsing the two files in `Documents\NFS Most Wanted` (read only).
4. A global scratch folder is shared with other agents; files with common names (`luadis.py`, `gp.pkl`) were
   overwritten by others once. Private tools live in a `blk` subfolder.

## Addresses in `speed.exe`

### Script hooks and race database

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Hook registry | `FUN_0061e750` | `0x0061E750` | strings -> handlers; table-bound ones via `FUN_0061ae80`/`FUN_0061aef0`/`FUN_0061afd0` |
| `UnlockRace` | `LAB_00612140` | `0x00612140` | race lookup `FUN_006022d0`, record `FUN_005dca40`, flags 4 and 8 |
| `IsRaceUnlocked` / `IsRaceCompleted` | `LAB_00605170` / `LAB_006051b0` | | `FUN_005dca70(db, key, 8 or 2)` |
| `GetNumChallengesPassed` / `GetNumRacesWon` | `LAB_00604a10` / `LAB_00604a30` | | `FUN_005e3980` (bin by index) then `FUN_005dd1c0` / `FUN_005dd1d0` (u16 +0x18, +0x1A) |
| `AwardCash` -> `AwardPoints` | `DAT_00612130` (a `jmp`), `FUN_0060e030` | | racer record `FUN_005dbed0(idx)` +0x120 via `FUN_005dc6d0` |
| `IntroduceRival` | `DAT_00605550` | `0x00605550` | one `ret` |
| `SetRaceCompleteForFE`, `StorePursuitRepForFE` | `DAT_00604c90`, `DAT_00604ca0` | | one `ret` each |
| `SetHasRapSheet` | `LAB_00605670` | | `flags |= 0x10` |
| `DoSafeHouseIntro` | `LAB_00605560` | | writes CurrentBin 0x10 |
| `DoSpecialSetup` | `LAB_00605830` | | career flag bit 20 |
| `JumpToCarLot`, `JumpToSafeHouse` | `FUN_00605250`, `FUN_006052b0` | | `Car_Select.fng`, `MainMenu_Sub.fng` via `FUN_0062fe40` |
| `NotifyRacePlacement` | `FUN_0060aa00` | | builds the message and sends it on subject `0x20D60DBF` |
| `NotifyRaceFinished` | `LAB_006119f0` | | records the finish (spec 4.2); calls `FUN_00602320`, `FUN_00600a20`, `FUN_0062aa90` |
| `ChallengeCompleted` | `FUN_00605140` | | broadcast of `MNotifyChallengePassed` (`FUN_006046c0`) |
| Record a finish | `FUN_00602320` | `0x00602320` | best result switch on race type; flag 2 (career) or 1 |
| Milestone completion | `FUN_006015b0` | `0x006015b0` | state 3 -> 4, `FUN_006010e0`, bounty `FUN_00612220`, `FUN_00605140` |
| Recompute bin counters | `FUN_006010e0` | `0x006010e0` | races won from record flag 2 over `WorldRaces`; challenges from the two tables |
| Bin by index | `FUN_005e3980` | `0x005e3980` | array at database +0x24, stride 0x1C, count +0x20, key `BinIndex` |

### Objects

| Object | Global / ctor | Layout facts |
|---|---|---|
| Race database | `DAT_0091e004`, ctor `FUN_006039c0` (size 0x40) | +0 capacity of record table, +0x0C race parameter array (stride 0x14), +0x20/+0x24 bins (stride 0x1C), +0x28 class, +0x2C/+0x30 current engaged race and mode, +0x3C record table (stride 0x10) |
| Race status | `DAT_0091e000`, ctor `FUN_00603720` | see spec 7.2 |
| Script manager | `DAT_0091e00c`, ctor `FUN_005f7d80` (size 0x2D8) | +0xA4/+0xA8 milestone table (stride 0x14), +0xAC/+0xB0 challenge table, +0x108/+0x10C respawn marker hashes, +0x118 flag, +0x264 flag bits (64 bytes), +0x130 state-table list |
| Career settings | embedded in the profile at +0xA8 | +0 car, +4 flags, +8 CurrentBin, +0xC cash; init `FUN_005a3950` (new career), reset `FUN_0057f4d0` |
| Bin advance | `FUN_0062c2d0` (vtable `0x008A7AB4`) | called from `FUN_00561320`, `FUN_0062c470` (event handler, table slot `0x008F6174`), `FUN_0062c4c0` (Lua `EEnterBin`, slot `0x008F6684`) |
| Post-bin FE sequence | `FUN_005302c0` | step counter; table of screen names at `0x008F3220` |
| Post-race FE return | `FUN_00623f80` | career branch opens `SafeHouseRaceSheet.fng` or `GameWon.fng` |
| Rival challenge screen | `FUN_0052ee50` | uses bin getters `FUN_005dd160/180/1a0/1c0/1d0` |
| Final race test | `FUN_0056dc00` | hash of `"1.8.1"` |

### Save routines

| Role | Ghidra name | Address |
|---|---|---|
| Load "Game" block | `FUN_005f81d0` | `0x005F81D0` |
| Save "Game" block | `FUN_005f7410` | `0x005F7410` |
| Load profile body (block, career settings, stable, records) | `FUN_005ac950` | `0x005AC950` |
| Save race records | `FUN_0056d810` | `0x0056D810` (count capped at 300) |
| Save/restore bin counters | `FUN_005e35d0`, `FUN_005e98d0` | |
| Milestone / challenge restore | `FUN_005ddbd0`, `FUN_005ddcb0` | match on word 1 |

### Event and Lua function tables

Event names are hashed with the same lookup2; tables at `0x008A5418` (161 sorted hashes), `0x008F61C8` (event
constructors) and `0x008F6450` (Lua-callable wrappers). `EEnterBin` is index 141: constructor `0x00621BB0` is a bare
`ret`; Lua wrapper `0x0062C4C0`. The Lua global lookup for names that start with `E` is `FUN_0061b970`.

## Data findings (not in the spec text)

- `EventID` is attribute `0xA78403EC`; `CashValue` `0xD8BAA07B`; `Reputation` `0x477EC5AA`; unlock-race list
  `0xFC8995C8` (67 collections); reward list `0x50104D90` (16); `BossRace` `0xFF5EE5D6`; `InitiallyUnlocked`
  `0xEA855EAF`; `Boss` `0xA5F39DC7`; `BossReputation` `0x1D33241A` (1000 in the template, 0 in the bins, 10000 in OPM).
- Final-race hashes: event `19.8.69` is `challenge_34_pursuit_length`; `19.8.31` is `challenge_burgerking`.
- Boss races have rewards `reward_pink_slip` (15), `reward_out_of_jail_free` (13), `reward_backroom` (16),
  `reward_vinyl`, `reward_cash_bonus`, `reward_add_impound_box`, `reward_release_car_from_impound`, part rewards.
- 55 milestone collections and 39 speed-trap entries match the save counts (55 and 39) exactly.

## Dead ends

- The caller of the bin advance. Searched: direct calls, data references, the event hash in every install file
  (raw and chunk-unwrapped), the Lua constants (737 strings), state-name hashes in code. Nothing found.
- `Global\Rivals.bin` is a texture pack (portraits `RIVAL_%02d_*`), not career data.
- `reputation_unlock` and `completion_unlock` graphs exist in `gpcore` but no collection instances them in this install.
- Names of attributes `0xB39ED8C3`, `0x39509746`, `0x8CB01ABF`, `0xA4E6FCFD`, `0xE10FB7A3`, `0xA07AE814`,
  `0x1823B89E`, `0x8F186AC4`, `0xE8C24416`: not found by hashing words.
- The `CareerModeHasBeenCompletedAtLeastOnce` store.
