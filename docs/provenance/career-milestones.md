# Provenance: career milestones

Addresses, files read, method and dead ends for [career-milestones.md](../specs/career-milestones.md) and
[career-milestones-data.md](../specs/career-milestones-data.md). The spec holds the rules; this file says where each one
came from. Ghidra auto names (`FUN_*`, `DAT_*`) are used here only. Image base `0x00400000`; program `/speed.exe`;
`D:\Need For Speed Most Wanted Black Edition\speed.exe` (6,029,312 bytes).

## Method

1. **Data.** A throwaway program (outside the repo) on top of `libs/blackbox-attrib` read `GLOBAL/ATTRIBUTES.BIN` and
   `GLOBAL/gameplay.bin`, listed the 30 `milestonetypes` collections, the `milestones`, `speedtraps` and `race_bin_NN`
   trees, and decompressed the Lua 5.0 bytecode blobs of the `gpcore` vault. `libs/blackbox-text` read
   `LANGUAGES/English.bin` and `Labels.bin` for the label indexes and texts.
2. **Field names.** The gameplay class stores field names as hashes (lookup2, initval `0xABCDEF00`). The Lua constants
   and a few exe strings gave candidate names; hashing them proved: `Bounty` `0x8E1904C7`, `MilestoneName` `0x704F72E8`,
   `BinIndex` `0x6CE23062`, `SpawnPoint` `0xDDF411F5`, `AutoStart` `0x883C65E3`, `ThreshholdValue` `0x4E90219D`,
   `ThreshholdSpeed` `0xC3710777`, `MilestoneBiggerIsBetter` `0x0896D043`, `CopSpawnPoints` `0xF124B151`,
   `Children` `0x916E0E78`, `CollectionName` `0x9CA1C8F9`, `MilestoneType` `0x16DF392A`, `RequiredRacesWon`
   `0xD617FEDC`, `RequiredBounty` `0xD3657D92`, `RequiredChallenges` `0x6DD4B98B`, `PursuitRace` `0x2B1F54F6`. Not found
   after trying a few thousand word combinations: `0x8445AF47`, `0x3B9BBFC2`, `0x006EC903`, `0xAB4A196F`, `0xDB89AB5C`. (The class definition is in `attributes.bin`; the readers print hashes for unnamed fields.)
3. **Lua.** A small Lua 5.0 bytecode lister (header: 4-byte floats, 32-bit instructions with opcode 6 bits, C 9 bits at
   bit 6, B 9 bits at bit 15, A 8 bits at bit 24) decoded the milestone, race, speed trap and rival handlers.
4. **Code.** The full decompile (25,799 functions) was searched by hash constants, strings and callers; Ghidra's
   cross references found direct callers that the decompile missed, because **some functions are not defined in Ghidra's
   analysis** (the escape routine at `0x00411070`, the pursuit tick around `0x00443BF0`, several virtual-table
   entries). Those were disassembled from the file with a standalone x86 disassembler (capstone) and read as
   instructions. Hash searches in the exe used the little-endian bytes of the hash.
5. **Install check.** `list-screens` of the repo's own binary listed the FEng packages (names, files, object and
   button counts).

## Addresses in `speed.exe`

### Statistics and milestone core

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| Manager singleton pointer | `DAT_0091e00c` | `0x0091e00c` | size `0x2D8`; created by `FUN_005fa430`, constructed by `FUN_005f7d80` |
| Race status singleton | `DAT_0091e000` | `0x0091e000` | `+0x1960` play mode, `+0x1968` race parameters |
| Bin table singleton | `DAT_0091e004` | `0x0091e004` | `FUN_005e3980` finds a bin view by `BinIndex` (0x1C stride) |
| Update a statistic | `FUN_005eda20` | `0x005eda20` | name, value; progress broadcast; milestone loop |
| Increment a statistic | `FUN_005edb50` | `0x005edb50` | |
| Read current / best of a statistic | `FUN_005edbc0`, `FUN_005edc00`, `FUN_005edc60`, `FUN_005ee220` | | |
| Reset per-pursuit statistics | `FUN_005edd10` | `0x005edd10` | flag bit 1; one caller `0x00423e94` |
| Export statistics to save | `FUN_005edcb0` | `0x005edcb0` | 16 bytes per node |
| Statistics map find / insert | `FUN_005ec420` / `FUN_005f5300` | | tree map at manager `+0x98` |
| Build the statistics map | `FUN_005f62f0` | `0x005f62f0` | reads the 30 `milestonetypes` collections; bit 0 from `MilestoneType != 2`, bit 1 from `0xAB4A196F` |
| Milestone entry constructor | `FUN_005ee250` | `0x005ee250` | type key from `MilestoneName`, bin, threshold `0x8445AF47` |
| Build the milestone array | `FUN_005f4bf0` / reload `FUN_005f4d40` | `0x005f4bf0` | root collection `0x1d975142` (`milestones`), template `0xa3d34781` (`milestone`) |
| Tree walker | `FUN_005f48e0` | `0x005f48e0` | depth first over `Children`, matches the template in the parent chain |
| Build the speed trap array | `FUN_005f4e50` / reload `FUN_005f4fa0` | | root `0x49511906`, template `0xb05871d3`; entry init `FUN_005e5890` |
| Bounty-jump key list | `FUN_005f4ad0` | `0x005f4ad0` | root `0x3d48e303` (`bounty_jump`), template `0xa7bccf63` (`marker`) |
| Array init after loading the pack | `FUN_005f8080` | `0x005f8080` | also loads the `gpcore` vault |
| Threshold test | `FUN_005de4b0` | `0x005de4b0` | state 2 to 3; creates the at-stake event |
| Bank / revert one milestone | `FUN_006015b0` | `0x006015b0` | flag 1 bank (3 to 4, pay, messages), flag 0 revert (3 to 2) |
| Bank / revert all | `FUN_00602770` | `0x00602770` | callers `0x0043eb51` (flag 0) and `0x004110ec` (flag 1) |
| Force-complete (debug) | `FUN_00602a00` | `0x00602a00` | sets entry flag bit 1; no callers |
| Unlock a bin's milestones / traps | `FUN_005e46f0` / `FUN_005e9d60` | | sole caller of the first: `FUN_0062c2d0` at `0x0062c347` |
| Bin advance event | `FUN_0062c2d0` | `0x0062c2d0` | profile `+0xb0` byte; also called from `FUN_00561320` when it reads 16 |
| Iterate milestones / traps | `FUN_005e46d0`, `FUN_005dda70`, `FUN_005e9d40`, `FUN_005e4770` | | state or flag filter plus bin filter |
| Recount a bin | `FUN_006010e0` | `0x006010e0` | writes bin view `+0x18` and `+0x1A` |
| Complete a speed trap | `FUN_00601730` | `0x00601730` | flags `(f & ~2) | 4`, speed to `+0x10` |
| Speed trap triggered | `FUN_0060a7b0` | `0x0060a7b0` | script hook `NotifySpeedTrapTriggered` |
| Pay bounty | `FUN_00612220` | `0x00612220` | hook `AwardPlayerBounty`; also used by banking; divisor at `0x008f5a4c` (5 in the file) |
| Total bounty | `FUN_00590500` + `FUN_00580f40` | `0x00590500` | sum of record `+0x10` (getter at `0x0056edc0`) plus `stable+0x8CA0` |
| Career record lookup | `FUN_0056ecc0`, `FUN_0056f120` | | record index `<= 0x19`, stride `0x38`, base `+0x8728` |
| Challenge completed broadcast | `FUN_00605140` | `0x00605140` | hook `ChallengeCompleted`; message getter `FUN_006046c0` |
| Counters for scripts | raw code | `0x00604a10`, `0x00604a30` | tail jumps to `FUN_005dd1c0` (`+0x18`) and `FUN_005dd1d0` (`+0x1A`) |

### Producers

| Statistic | Ghidra function | Address of the call | Notes |
|---|---|---|---|
| `pursuit_length` | not a function | `0x00443ce7` | pursuit update; adds dt to `this+0xf4`, posts whole seconds with message `0x40a7c0` |
| `pursuit_evasion_time`, `pursuits_in_a_row` | not a function (starts `0x00411070`) | `0x004110d8`, `0x004110c2` | then `push 1; call 0x602770` at `0x4110ea` |
| `bounty_in_pursuit`, `cost_to_state(_in_pursuit)` | `FUN_006013f0` | `0x0060145b`, `0x006014a6`, `0x006014da` | also holds a no-effect loop over nine names (the per-pursuit set) at `0x008f5d4c` |
| `cops_destroyed` | `FUN_0043e8d0` | `0x0043ea3f` | also calls the revert at `0x0043eb51` |
| `cops_destroyed_in_pursuit` | `FUN_00418f30` | `0x004190a6` | |
| `cops_damaged` | `FUN_0040af40` | `0x0040afa6` | |
| `cops_in_pursuit`, `helis_in_pursuit` | `FUN_004190c0` | `0x00419132`, `0x0041914a` | callers `FUN_00423df0`, `FUN_0042b550` |
| `total_cops_in_pursuit` | `FUN_00423df0` | `0x0042404b` | also calls `FUN_005edd10` at `0x00423e94` |
| `roadblocks_dodged`, `tire_spikes_dodged` | `FUN_00439ee0` | `0x0043a06f`, `0x0043a144` | |
| `total_infractions` | `FUN_005fdda0` | `0x005fde64` | popcount of the infraction mask; callers include `FUN_004299b0` |
| `insurance_claims` | `FUN_004299b0` | `0x00429e0b` | |
| `speedtrap_speed` | `FUN_0060a7b0` | `0x0060a804` | |
| `heat_meter` | `FUN_006f13f0` | `0x006f2131` | |

### Events, messages, screens

| Role | Ghidra name | Address | Notes |
|---|---|---|---|
| `MNotifyMilestoneProgress` type, build, post, Lua table | `FUN_005dd5f0`, `FUN_005dd670`, `FUN_006357a0`, `FUN_00626040`, `FUN_00632d20` | | strings at `0x008a393c`; payload name pointer `+0x10`, value `+0x14` |
| `MNotifyMilestoneReached` | `FUN_005de370`, `FUN_006358a0`, `FUN_00626090`, `FUN_00632da0` | | string `0x008a3990` |
| `MNotifyChallengePassed` | `FUN_006046c0` | `0x006046c0` | |
| `EReportMilestoneAtStake` | constructor `FUN_00641c20`, virtual table `0x008a7dc4` | | name getter at `0x006247b0` returns `0x008a7314` |
| `EShowMilestones` | constructor `FUN_00641fe0`, table `0x008a7dcc` | | name getter at `0x00625120` returns `0x008a74e8`; pushes `PostRace_MilestoneRewards.fng` |
| Third event of the same group | `FUN_00642140`, table `0x008a7dd4` | | pushes `PostRace_Results.fng` or `PostRace_Pursuit.fng` |
| Post-race flow | `FUN_0060ac20` | `0x0060ac20` | pursuit race: `EShowMilestones(0)` on a win, the race-bust request on a loss |
| Autosave event | `FUN_00620920`, table `0x008a6820` | | table is followed by the string `EAutoSave` (inferred name) |
| Pursuit summary data | `DAT_0091d3d8` | `0x0091d3d8` | list count `+0x28`, up to 32 pointers from `+0x2c`; append `FUN_0056a500`, clear `FUN_0056a540`, fill `FUN_0056a440` |
| Text label index of an entry | `FUN_005e55a0` | | reads `0xDB89AB5C` of the type collection (`FUN_005e2650` finds it by class hash `0xE4C3D904`) |
| Bounty of an entry | `FUN_005e54f0`, `FUN_005e56c0` | | |
| Type to label / object maps | `FUN_0056dea0`, `FUN_0056dff0`, `FUN_0056a670` | | by type key; time types are `0x033fa23a`, `0x5392e4fd` |
| Row builders | `FUN_005a2390` (pause list), `FUN_005a28e0` (rewards rows), `FUN_0051f310` (rap sheet row), `FUN_00552440` (rap sheet list), `FUN_005481c0` (pin) | | row datum virtual table `0x0089c3a4` |
| HUD board | constructor `FUN_0059f440`, update `FUN_0059f7c0` | | slot stride `0x18`, count at `+0x40`; reads profile `+0xb0` |
| Rival rap sheet | `FUN_0052ee50` | `0x0052ee50` | three requirement rows |
| Hook table | `FUN_0061e750` | `0x0061e750` | the script names used here |
| Profile buffer writer | `FUN_0057f7b0` | `0x0057f7b0` | writes the 16 KiB block first |
| Save / load block | `FUN_005f7410` / `FUN_005f81d0` | | helpers `FUN_005ddbd0`, `FUN_005ddcb0`, `FUN_005f6430`, `FUN_005e98d0` (bins), `FUN_005e35d0` (save bins), `FUN_005e4c40`, `FUN_005f50c0` |
| Reset | `FUN_005f7730` | `0x005f7730` | callers `FUN_005ab9c0`, constructor, loader |

## Script handlers read (vault `gpcore`)

`stategraph_milestone_challenge_initial_handler_{stateenter,notifymilestoneprogress,notifymilestonereached}`;
`stategraph_basicrace_racing_handler_{notifymilestoneprogress,notifymilestoneprogress_2,perpescaped,perpescaped_2}`;
`stategraph_basicrace_prerace_handler_stateenter_{8,9}`; `stategraph_speedtrap_{activity,group}_monitor_handler_triggerenter`;
`stategraph_race_bin_check_rival_handler_stateenter`; `stategraph_race_bin_rival_locked_handler_notifychallengepassed` (12).

SMS triggers of the 12 `notifychallengepassed` handlers (variant number: bin challenge counts that send, message numbers):
2: 1 sends 124, 3 sends 137; 3: 2 sends 19, 3 sends 22; 4: 1 sends 35, 2 sends 25, 3 sends 26; 5: 1 sends 33; 6: 2 sends 38;
7: 2 sends 132; 8: 3 sends 61; 9: 2 sends 133; 10: 2 sends 100; 11: 3 sends 101; 12: 4 sends 102; 13: 2 sends 43. Which bin
owns each handler was not decoded. The rival check enters `rival_unlocked` only when the three requirements all hold.

## Install files

| File | Used for |
|---|---|
| `GLOBAL/attributes.bin` | class `milestonetypes` (30 collections), class definitions |
| `GLOBAL/gameplay.bin` | `milestones` tree (55 leaves), `speedtraps` tree (39), `race_bin_NN` requirement fields, scripts |
| `LANGUAGES/English.bin`, `Labels.bin` | the label indexes 0 to 9 and the HUD and rap sheet strings |
| `GLOBAL/InGameB.bun` | `InGameMilestones.fng`, `PostRace_MilestoneRewards.fng`, `PostRace_Pursuit.fng`, `PostRace_Results.fng` |
| `FRONTEND/FrontB.lzc` | `SafeHouseMilestones.fng`, `SafehouseMilestones_HELP.fng` |

## Dead ends and lessons

- Searching the decompile for a hash found only the accessors of five fields; `0x3B9BBFC2` and `0x006EC903` have no hit
  in the whole file (byte search), so they are dead data or computed from strings.
- `pursuit_length` has no call with its name as a string. It lives in code Ghidra did not define as a function; a
  byte search for the string address found it.
- The escape routine `0x00411070` was found only through Ghidra's cross references to the commit function.
- No code subscribes to the progress message by its static type (only senders call its type getter), so the HUD board
  slot builder was not found that way; the board is probably filled by another path (open).
- `AwardPlayerBounty` divides by a memory value that is 5 in the file; the creator `0x005fa430` of the manager runs an
  obfuscated integrity computation over the table that holds the pointer, so the value may be changed or irrelevant at
  run time. Not resolved.
- `MilestoneCompleteHeatAdjust` (hash `0xA6CB6EFF`) has no hit in the exe; the name is not in the strings either.
- Names of the two events `EAutoSave` and `EShowMilestones`: `EShowMilestones` and `EReportMilestoneAtStake` are proven by
  the name getters stored in their virtual tables; `EAutoSave` is only adjacent to the table.

## Next leads

- The rap sheet "launch a milestone" path (`REP_SHEET_MILESTONE_CONFIRM`): the label hash was not found in code; look for
  the FEng message handler of the milestone page.
- The HUD board slot filler (`Hud_MilestoneBoard`, vtable `0x008a2420`).
- The cop manager functions around `0x00411070` and `0x00443BF0`: define them in a scratch Ghidra copy to read virtual call
  targets (`+0x8c`, `+0x18`, `+0x134`).
- The 8-slot block and the 64-byte flag set of the save block.

## 2026-10-18: gameplay blob offsets (save routines)

Goal: the offsets that the spec's section 7 listed in order without numbers. Result is in
[career-save.md](../formats/career-save.md) section 3.1a; the spec rules did not change.

- **Routines read:** save `0x005F7410` (writer), load `0x005F81D0` (reader), slot block `0x005E4C40`, statistic nodes
  `0x005EDCB0`, bin counter block `0x005E35D0`. All in `/speed.exe`. Ghidra had two `speed.exe` programs open, so calls
  need the program path `/speed.exe`.
- **Saves read:** `NAME\NAME`, `mwo\mwo`, `stock\stock` (read only) with a throwaway parser under the session scratch
  directory, outside the repo. Blob at file 0x34. The MD5 over blob bytes 0x10 to 0x3FFF matched in all three saves.
- **Hash port:** the engine name hash from `libs/blackbox-hash` was reimplemented in the throwaway script only.
- **Checks passed:** tag `Game`, version 8; header counts 2/17, 8, 30, 55, 39, 512, 164, 0; milestone and speed trap
  structure as in the save doc; the used part ends where the older text said (0xC64 and 0xD54).
- **Corrected:** header 0x34 is the bin counter block size, not a zone block; 0x38 is the bounty-jump key count.
- **Conflict left open:** bounty-jump list location in the manager (spec 2.1 says +0xB4, the writer reads +0x2A8 and
  +0x2AC). No effect on the saves (K = 0).
- **Not resolved:** the two object keys in the saves do not equal the name hashes of `race_bin_15` and `speedtraps`. The
  byte names of the object records are unknown. The meaning of bin id field `0x6CE23062` (values up to 21) is open.
  Milestone type keys were checked only against the statistic node keys, not against `MilestoneName` hashes.

Next leads: find the hash of the object keys (maybe a seed or a different name form); decode field `0x6CE23062` in
the bin view; write a check of milestone type keys against the names in `gameplay.bin`.
