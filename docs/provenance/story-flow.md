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
