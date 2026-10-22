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

### F-01: the movie on the NIS path

The event table (hashes at `0x008A5418`, constructors at `0x008F61C8`, handlers at `0x008F5F40`) holds `EPlayRaceNIS`
(`0x66666DC8`, index 81; constructor `0x00623DB0`, handler `FUN_00641520`) and `EPlayRaceMovie` (`0xF4A034F9`,
index 14). The handler builds the event record in `FUN_00640f00`: marker, scene name, kind text, two words, then two
more texts (the race's `IntroMovie` is the first of these in the Lua call). It returns at once when the scene name is
empty or unknown; the kind text maps to 0 `Intro`, 1 `Arrest`, 2 `Outro`, 4 `Moment` (a moment also needs a cop car
near the player); then it gets the `NISActivity`, registers `car1`..`car8` (unless a moment), places the scene at the
marker (or the player car when there is none) and starts it with the scene name and the track word. Where the
activity plays the movie text (the "NIS Play Story" state label at `0x008B0754`, used by `FUN_006f7230`) is
**[unconfirmed]**; the rewrite plays the movie first, then the scene.

### F-03: `PlayTutorial`

Registered by the script registry `FUN_0061e750` (push of `0x006124E0` with the name at `0x008A4A70`). The routine at
`0x006124E0` (no function defined) returns the finished message at once unless the race status exists, a global at
`0x00926064` is 0, the play mode is 2 and a race is current. Then by race type (`FUN_005faa20`): 2 → settings `+0xAC`
bit 6 and `drag_tutorial`, 4 → bit 8 and `tollbooth_tutorial`, 5 → bit 7 and `speedtrap_tutorial`; a set bit or any
other type posts `0x20D60DBF`. Otherwise it sets the bit and starts the movie through `FUN_0062f820`.

### F-04: the message screen

`EShowMessageScreen` hashes to `0xDE59505A` (index 116 of the event table); its handler `FUN_00631020` builds the event
(`FUN_00625030`: the id at `+8`), and the event's teardown `FUN_00625090` opens `SixDaysLater.fng` through the screen
manager (`FUN_005257f0`) with the id as the argument.

### F-25: applying a reward

`FUN_0062aa90` (the reward request) reads the reward collection (class `0x5CEA9D46`, key = the argument): a non-zero
`0x0E0113FE` goes to the part unlock `FUN_00577680`; else a non-zero `0x06A077D5` goes to `FUN_005a62d0` on the marker
inventory (`0x0091D928`) with the flag "apply now" = no race status, or the current race's `FUN_005fbe00` (`0xF2FE50D7`)
false. `FUN_005a62d0`: the kind from `FUN_00577590(0x06A077D5 text, 0x3C2FDAAB text)`; apply now: `0x12` →
`FUN_00573140("BL%d")` then `FUN_005a41e0(stable + 0x414, preset)`, `0x13` → profile `+0xB4` += truncated
`0xAB0179F4`, other → `FUN_00576c50(kind)`; queued: `FUN_00576b80(kind, value)` (the pink slip queues its preset
handle, the cash its amount).

### F-24: the markers screen

Constructor `FUN_007b9a80` (vtable `0x008B7DF8`): reads the popup queue (`FUN_00576b50`, 6 slots of kind and value),
looks each kind up in a table at `0x00905EE0` (7 words per row, category hash in word 2) and appends the cards in the
order of four category hashes, then swaps three cards with random indices in 0..2 (`FUN_0045da30(3)`). Message handler
`FUN_007b3cd0` (created in this pass at `0x007B3CD0`, slot 1): on the select message `0x0C407210`, if two cards are
already revealed (`FUN_007a77c0`) it refuses; else it reveals the focused card and applies it (`0x12` →
`FUN_005a41e0(stable + 0x414, value)`, `0x13` → cash `+0xB4` += value, else `FUN_00576c50(kind, value)`); on
`0xE1FDE1D1` it clears the queue (`FUN_00576bd0`) and calls the arrival stepper.
