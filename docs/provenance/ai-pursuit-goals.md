# Provenance: AI pursuit goals (pit, ram, box-in, rolling block, pull-over)

Spec: [docs/specs/ai-pursuit-goals.md](../specs/ai-pursuit-goals.md).

## Method

- **Tool:** Ghidra MCP on `/speed.exe` (PC build; image base `0x400000`; 26083 functions). Decompile by function address,
  disassembly not used. Strings by regex, xrefs by address (`get_xrefs_to`), memory by `read_memory`.
- **Hash checks:** a throwaway Python `lookup2` (initval `0xABCDEF00`, same as `libs/blackbox-attrib/src/hash.rs`) in the
  scratchpad, not in the repo. The test vector `pvehicle` = `0x4A97EC8F` matched first. Then candidate names were
  generated as prefix x suffix combinations (about 7700 names, then 5000 and 2800 in two later passes) and compared with
  the constants in the goal constructors and the action. Matches are listed in the spec, section 8.
- **Repo reads (not decomp):** `docs/specs/ai-pursuit-formations.md` (slot tables, finisher timers), `docs/specs/ai-pursuit-tactics.md`
  (action list, ram and pull-over math), `docs/formats/pursuit-data.md` (box and roll columns per heat),
  `docs/provenance/ai-driver-control.md` (the `AIGoalPit` hash), `docs/specs/ai-pursuit-cop-cars.md` (collision sets).
  The Rust port (`cop_aims.rs`, `SLOT_ARRIVAL`) was read for its inputs only and not used as evidence.

## Addresses

| Address | Name in Ghidra | What it is |
|---|---|---|
| `0x00442D00` | `FUN_00442d00` | cop car constructor: sets the goal kind `0x108` to 7 and calls the factory once |
| `0x00440330` | `FUN_00440330` | goal factory: `switch` on kind at `this+0x108`; one direct caller (the constructor) |
| `0x0043FE90` | `FUN_0043fe90` | pit goal constructor (kind 1); vtable `0x00892F70`; name string `AIGoalPit` at `0x00892F84` |
| `0x0043F1A0` | `FUN_0043f1a0` | box-in ram goal constructor (kind 2); vtable `0x00892F0C`; reads `BoxinTightness` and `BoxinDuration` |
| `0x0043F520` | `FUN_0043f520` | rolling block ram goal constructor (kind 3); vtable `0x00892F34`; reads `RollingBlockTightness` and `RollingBlockDuration`; slot table at `0x00890E5C` (the loop reads x at the pointer minus 4 bytes, y at the pointer, z at plus 4; stride 12 bytes) and kinds at `0x00892F20` |
| `0x0043F770` | `FUN_0043f770` | follow goal constructor (kinds 4 and 5); vtable `0x00892F48` |
| `0x0043FB00` | `FUN_0043fb00` | stagger follow goal constructor (kind 7); vtable `0x00892F5C` |
| `0x00440050` | `FUN_00440050` | herd goal constructor (kind 6); vtable `0x00892F90` |
| `0x00427C40` | `FUN_00427c40` | shared ram action update (box-in, rolling, pit and pull-over); name test on `AIGoalPullOver` at `0x00891628` |
| `0x004135F0` | `FUN_004135f0` | slot point builder (world offset from the record, detour for pull-over) |
| `0x00413AB0` | `FUN_00413ab0` | lead term helper (`steer = seek + v (1 + 0.7 dot / |v|^2)`); output written through a garbled return slot |
| `0x0043B9B0` | `FUN_0043b9b0` | action-list constructor (names `AIActionRam`, `AIActionPursuitOffRoad`, `AIActionRace`, `AIActionTooDamaged`, `AIActionGetUnstuck`, `AIActionAirborne`) |
| `0x0043BD10` | `FUN_0043bd10` | head-on ram action (for comparison; string `AIActionHeadOnRam` at `0x00892CC4`) |
| `0x00767400` | `FUN_00767400` | segment/circle solver used by the detour (not read in full) |
| `0x0040A5E0` | `FUN_0040a5e0` | performance limiter (the same one the tactics spec uses); not read in full here |

Strings: `AIGoalRam` `0x00892F00`, `AIGoalPit` `0x00892F84`, `AIActionRam` `0x00892C98`, `AIGoalPullOver` at
`0x00891628` (the bytes before it, `qtA`, are a packed-string prefix), `AIGoalStopShort` `0x00892FD8`.

Cross-references to the vtables: `0x892F0C` from `FUN_0043f1a0`; `0x892F34` from `FUN_0043f520`; `0x892F70` from `FUN_0043fe90`.
The strings `AIGoalRam` are also referenced from the data table at `0x00884560` (factory name table).

## Hashes (lookup2, initval `0xABCDEF00`)

| Name | Hash | Status |
|---|---|---|
| `pvehicle` | `0x4A97EC8F` | test vector, matches the repo's known value |
| `AIGoalPit` | `0xEC619CB2` | matches `docs/provenance/ai-driver-control.md` |
| `AIGoalRam` | `0x458F733B` | computed |
| `AIGoalPullOver` | `0x63472E0A` | computed |
| `BoxinTightness` | `0x67A15750` | matches the box-in constructor |
| `BoxinDuration` | `0x858B1097` | matches the box-in constructor |
| `RollingBlockTightness` | `0xC146FC03` | matches the rolling block constructor |
| `RollingBlockDuration` | `0x5C9F5F55` | matches the rolling block constructor |
| `CollapseAggression` | `0x594E1492` | matches the pull-over test in the action |
| `TopSpeedMultiplier` | `0xEC57E16B` | matches the action's ceiling |
| `AccelerationMultiplier` | `0xCC320329` | matches the action's acceleration |

The field hashes are the keys of the install's AttribSys rows. The match here is the evidence that the names are right. The
install's row values were not re-read for this task; the per-heat numbers come from the earlier data read in
`docs/formats/pursuit-data.md`.

## Findings in order

1. The factory `switch` gives the goal kinds at once. Kind 1 is pit, kinds 2 and 3 are the ram class (box-in, rolling),
   kinds 4 and 5 are follow, kind 6 is herd, kind 7 is stagger follow (the default of a new cop).
2. The three record tables (box-in, rolling, pit) match the formation spec's slot tables exactly, except one value: the
   rolling block's inner right slot is `z = 14` in memory (the formation spec says `10`). The rolling slots are read at
   `0x00890E5C`, five triplets, the loop ends at `0x00890E98`. The four floats after that (`-2.0`, `4.0` at `0x00890E94` and
   `0x00890E98`) are outside the loop.
3. The record's fourth field is the formation priority (`1` to `4`), and the record's name field is the goal name hash.
4. The formation fields the factory sets are `[+4]` = max cops, `[+8]` = min cops, `[+0xC]` = has finisher. Pit: 1, 1, yes.
   Box-in: 4, 2, yes. Rolling: 4, 2, yes. Follow: 6, base 1, no. Herd: 3, base 1, no. Stagger: 6, base 1, no.
5. The pull-over action (`FUN_00427c40`) reads `CollapseAggression` from the target's pursuit level (default 0, forced to 1
   on a jerk). The arrival test uses `|seek|` before separation, with threshold `(1 - a) 0.2 + 0.5` km/h.
6. The ram action has no nitrous factor and no grade term in its own body; the limiter may have them (open).

## Lessons

- **Factory first.** A `switch` in a factory names every goal kind and its constructor in one place. Read it before
  chasing names; each constructor then gives its table and its attributes with no guesswork.
- **Brute-force attribute names.** An unnamed attribute hash can be matched by generating names from a small vocabulary
  (prefix x suffix, with and without underscores, in lower and upper case). The tightness and duration names matched
  (the rolling-block pair in the second pass, the box-in pair in the third). Keep the candidate lists in a scratch file, not the repo.
- **Garbled helpers.** Ghidra mislabels the output of a helper that takes a struct by pointer: `unaff_retaddr`,
  `unaff_EDI` and `extraout_ECX` appear where the real argument is. Check the constructor and the arithmetic instead
  of the helper's output parameters. Mark the affected constant **[unconfirmed]**.
- **Float tables.** A loop that reads `p[-1]`, `p[0]`, `p[1]` and steps by 3 floats means the x of the first entry is
  four bytes before the loop's start. Read the bytes with `read_memory` and decode the loop bound to get the entry count.
- **Vtable from a constructor is not always the object's main vtable.** `0x8928EC`, the first vtable the cop constructor
  writes, is a three-slot secondary table followed by RTTI strings. Look for the main vtable by a different route (the
  object's first write, or the class's typeinfo).

## Dead ends

- **Goal-kind setter.** `FUN_00440330` has one direct caller, the cop constructor. The kind at `0x108` is written by a
  setter that calls the factory through a vtable slot, and that call site is not found. Cross-references to the factory
  do not show it. Next: find the cop class's vtable (not `0x8928EC`) and the slot that holds `0x00440330`.
- **Pull-over factory case.** No factory case has the name `AIGoalPullOver`. The pull-over goal is probably a ram-class
  goal given the name at runtime (the collapse code), but no caller that passes that name was found.
- **Speech trigger.** The `intenttoram` collection (`AnytimeEvents_IntentToRam` at `0x008B1464`) is referenced from `0x00901E5C`,
  which is data (a name table), not code. The caller that requests the event was not found.
- **Tangent length.** The pull-over detour's factor `2.1` (tactics spec) is not in the code read. The function
  `FUN_004135f0` scales one vector by a float argument the decompiler labels `unaff_EDI`, and the caller's value for it
  was not traced. The factor is left **[unconfirmed]**.
- **The `0x108` kind of a pursuit cop** may also be set from the formation's "in position" code, which would be the same
  setter. Not checked.

## Next leads

See the spec's "Next leads" list (six items). The first two decide the question "when is a pit or pull-over goal set":
the goal-kind setter and the pull-over factory path.

## Stationary target correction (23 October 2026)

Read the reconstructed [AIActionRam.cpp](https://github.com/dbalatoni13/nfsmw/blob/main/src/Speed/Indep/Src/AI/Actions/AIActionRam.cpp)
and [AISteer.cpp](https://github.com/dbalatoni13/nfsmw/blob/main/src/Speed/Indep/Src/AI/Common/AISteer.cpp).
The slot builder selects heading below 5 km/h. The separate `UpdateSeek` reads
actual target velocity for its projection; `Pursuit` adds that velocity for one
second. This corrects the earlier conflation of slot orientation and prediction.
The pull-over uses the same action and checks the complete seek before separation
for arrival, so the earlier lead-offset-only workaround is removed.

Evidence is static reconstructed GameCube behavior, not a recorded PC trajectory.
Regressions distinguish a stationary target's heading from its predictive velocity,
cover a slowly moving target below the slot-frame threshold, and check pull-over
arrival with the complete predictive seek. No original implementation is copied.
