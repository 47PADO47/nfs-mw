# Race mode (`libs/blackbox-race`, `crates/nfsmw/src/scenes/world/race`)

- **Specs:** [race-mode](../specs/race-mode.md), [ai-racers](../specs/ai-racers.md),
  [ai-racers-route](../specs/ai-racers-route.md), [ai-racers-catchup](../specs/ai-racers-catchup.md); data in
  [formats/gameplay-races.md](../formats/gameplay-races.md).
- **Restricted sources read:** `speed.exe` v1.3 in Ghidra (read only; no function was renamed or changed). The
  decompiled class sources (`GRaceStatus.cpp` and friends) were **not** read in this pass: fetching them was refused
  by the environment, so the specs rely on the executable and on the existing specs of the repository.
- **What was read, with addresses (no code copied):**
  - `FUN_0061e750`: the script-binding table. Names every `GRaceStatus` call listed in the race-mode spec section 1,
    and the pointers they run: `NotifyCheckpointReached` `0x006117f0`, `NotifyLapFinished` `0x00611850`,
    `NotifyRaceFinished` `0x006119f0`, `StartRace` `0x0060dbd0`, `KnockoutRacer` `0x00611440`, `SetRacerGoal`
    `0x00611700`, `NotifySpeedTrapTriggered` `0x0060a7b0`, `AwardBonusTime` `0x005fe600`, `SetNextCheckpointPos`
    `0x005fe660`, `CalculateRanking` `0x006058b0`.
  - `0x005faa20`: the race type is `EventIconType` looked up in an 11-entry table (as the format doc says).
  - `0x005facd0`, `0x005fc6e0`: the opponent count and `KnockoutsPerLap` getters used by the knockout rule.
  - `0x00600660`, `0x00602200`: per-racer checkpoint counter (advances only to the next index), lap number and
    times, the checkpoint counter reset on a lap.
  - `0x005fde80`, `0x005e31d0`: the ranking sort and its comparison (finished, then eliminated flags, then laps,
    then progress; points when the race ranks by points).
  - `0x005fe090`, `0x005fc260`: time left = `TimeLimit` + bonus − elapsed, floored at 0.
  - `0x0060a7b0`: a speed-trap notification multiplies the speed by 0.27778 and stores it per trap.
  - `0x006119f0` (disassembly): a finished racer's progress is set to 100 and the racers are re-ranked.
  - Strings: `HUD_Drag.fng`, `HUD_Drag_Player1.fng`, `HUD_Drag_Player2.fng`, `EngineDragster`, `DamageDragster`,
    `InputPlayerDrag`, `MMICON_AIRACER_%d`, `MMICON_CHECKPOINT`, `RACERNAME_%03d`.
- **Not read:** the Lua state graphs, the countdown timing, the post-race screens, `ComputeCatchUpSkill`.
- **Choices where the sources are silent** are marked **[unconfirmed]** in the specs and next to the code.
- **Implemented:** 2026-10-20, from the specs, without the executable's code open.
