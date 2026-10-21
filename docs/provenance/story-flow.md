# Story flow: provenance

Records of the restricted sources behind [specs/story-flow.md](../specs/story-flow.md) and
[story-flow-2.md](../specs/story-flow-2.md). Process: [licensing.md § Spec-first](../licensing.md#spec-first).

## Method

The two spec files were first assembled from the repo's existing career specs (no new source). The facts they listed as
missing (story-flow-2 section 6) were then looked up: script-side facts in the Lua graphs of `GLOBAL/gameplay.bin`, and
native facts in `speed.exe` 1.3 with Ghidra (read for behaviour only; no decompiled code enters the repo).

## Ghidra passes (speed.exe 1.3)

### F-10: what a finish writes

`NotifyRaceFinished` (`LAB_006119f0`, no function defined at that address) disassembled from `0x006119F0` to `0x00611CC0`:

| Address | Reading |
|---|---|
| `0x00611A34` .. `0x00611A4F` | racer index (`GetRacerIndex`, `0x0060DAE0`) minus 1, then the racer's record (`FUN_005dbed0`) |
| `0x00611A6E` | a virtual test on the vehicle (`+0x24`); false skips everything (taken as "is a human racer", **[unconfirmed]** name) |
| `0x00611B11` | `record + 0x10` (the place, see [race-results-screens.md](race-results-screens.md)) compared with 1; anything else jumps past the win block |
| `0x00611B4C` .. `0x00611B5F` | the race-status processed byte `+0x4554`: the win block runs once per race |
| `0x00611B78` | `FUN_005dca70(node hash, 2)`: the node already carries `0x2` (won before) |
| `0x00611B84` .. `0x00611BB9` | play mode `+0x1964` = 2 (career): profile cash `+0xB4` += the race's cash value (`FUN_005fb920`, truncated), on every win |
| `0x00611BC1` .. `0x00611C10` | not won before: each entry of the race's reward list goes to the reward request `0x0062AA90` |
| `0x00611C25` .. `0x00611CA2` | not won before: each race of the unlock list (`0xFC8995C8`) gets `0x4`, and `0x8` unless `0xA4E6FCFD` |
| `0x00611CB4` | `FUN_00602320(1)`: the node record, `0x2` in career or `0x1` otherwise ([career-save-screens.md](career-save-screens.md#node-state-bits)) |

### F-14: the special-setup hook

`DoSpecialSetup` (`0x00605830`, no function defined there; disassembled to `0x0060588F`): node lookup by hash
(`FUN_006022d0`), the node attribute test `FUN_005fc560` (`0x4393F69B`), then the profile (`DAT_0091cf90 + 0x10`) flags
word `+0xAC` shifted by 20 and tested; when the bit is clear, `FUN_005a41e0(stable + 0x414, 0x03A94520)` finds the
preset's stable record and its first word goes to the current car (`+0xA8`); returns 1. Any other path returns 0.

### F-18, F-19, F-20: the arrival stepper

| Function | Address | Reading |
|---|---|---|
| stepper | `FUN_005302c0` | counter `+4` incremented on entry; step 5 formats `blacklist_%02d` with the stage byte `[[0x0091CF90]+0x10]+0xB0` (push at `0x005302F1`) and opens `FEAnyMovie.fng` / `WS_FEAnyMovie.fng`; step 6: stage 15 clears context bit 17, sets context bit 0 and settings bit 5, opens `SafeHouseReputationOverview.fng`, counter -1; stage 13 with settings bit 4 clear: counter 5, sets bit 4, plays the movie named at `0x0089D8B0` (`storyfmv_rap30`); otherwise `FUN_0056c5b0` |
| front-end entry | `FUN_00562ff0`, call at `0x005630E2` | calls the stepper when the request flags (`DAT_0091cbac`) carry `0x400000` |
| rival bio, Accept | code at `0x00551D1A` (vtable `0x0089F0A8`, slot 1 `0x00551C80`, no function defined) | on `PAD_ACCEPT` (`0x406415E3`) with context bit 17: a counter of -1 becomes 4 (`0x00551D3D`), then the stepper is called |
| movie screen teardown | `FUN_00622600` | calls the stepper when the counter is 6 |
