# Race restart and the pause menu's race flag (`scenes/world/race/restart.rs`)

- **Spec:** [race-restart](../specs/race-restart.md); the pause menu's items are in
  [career-parity-menus](career-parity-menus.md) ("Pause menu items"); the race status calls are in
  [race-mode](race-mode.md).
- **Restricted sources read:** `speed.exe` v1.3 in Ghidra (read only: nothing was renamed, retyped, created or changed in
  the project; the one byte dump at `0x0051D7C0` was read with the memory viewer, not disassembled into the listing).
  Machine code only; no code or bytes are copied. The decompiled class sources were not read.
- **Evidence tags:** [docs/README.md](../README.md#evidence-tags). **[exe]** = read from the executable.

## 1. The pause menu builder `FUN_0052b4d0` (re-read for the race flag)

The function takes the menu object and adds items. Two globals matter: the race status object `DAT_0091e000` (the same as
in [race-mode](race-mode.md) section 3) and the profile `DAT_0091cf90`.

| Test | Meaning | Result |
|---|---|---|
| `*(DAT_0091e000 + 0x1964) != 2` | quick-race play mode | the race menu, always: Resume Race, Restart, a Quit item, Tuning when the race is not kind 4 and not a challenge-series race, Options (or the shorter list when `DAT_00988e04 == 1`) |
| `== 2` and `*(DAT_0091e000 + 0x1960) == 0` | career, roaming | Resume Free Roam menu |
| `== 2` and `+0x1960 != 0` | career, a race was started | Resume Race, Restart, then below |

The career race menu (`+0x1960 != 0`), with `race = *(DAT_0091e000 + 0x1968)` and the bin stage byte at
`*(DAT_0091cf90 + 0x10) + 0xb0`:

- `bVar2 = race != 0 && FUN_005fc560(race)`: a boolean property of the race's definition (key `0x4393F69B`; its meaning is
  not named in the executable: read as "the event was started from the menus", because true takes the menu without Jump to Safe
  House and Quit to Free Roam).
- stage < 16, and (`FUN_0056dc00()` (the race is event `1.8.1`) or `bVar2`): Resume Race, Restart, Quit to Career Main Menu,
  Tuning, Options.
- stage < 16 otherwise: Resume Race, Restart, Jump to Safe House, Quit to Free Roam, Tuning, Options.
- stage >= 16: Resume Race, Restart, Quit to Career Main Menu, Options.

So **Restart is in every race variant**, and nothing in the menu builder separates a pursuit challenge from another race:
`FUN_005fc180(race)` (definition flags bit 15, "challenge-series race", or the property `0x1C650104`) only chooses the Quit
item of the quick-race menu (Quit to main menu `0xE950B7AF` for a challenge-series race, Quit to Quick Race Menu otherwise).
`FUN_0050e1b0()` is `race kind == 4` (`FUN_005faa20` returns the race kind; tollbooth is 4 in the type table of
[race-mode](race-mode.md) section 2). Free-roam speed traps do not pass through `StartRace`.

## 2. Who sets and clears `+0x1960`

| Routine | Address | Role | Effect on the race flag |
|---|---|---|---|
| `FUN_00602010` | `0x00602010` | called only by `StartRace`'s handler | `+0x1960 = 1`; then `FUN_005ffd80`, a loop over a global list of objects (virtual calls at +0x40 and +0x50), counts the racers that have a driver at `+0x19A0`, clears `+0x4490` |
| `FUN_0060dbd0` | `0x0060dbd0` | the Lua function `StartRace` (registered in `FUN_0061e750`) | calls `FUN_00602010`, sends `Game_StartRace` (`FUN_004c2340`), copies race parameters (`+0x454C`, `+0x4550`), tells each racer's simable interface whether the race kind is 2 (drag), applies the start speed |
| `FUN_005fe3e0` | `0x005fe3e0` | the Lua function `SetRoaming` | `+0x1960 = 0`, `+0x1964 = 2`, `+0x1968 = 0`, `+0x195C = 0`; also clears the rain override and runs some cleanup |
| `FUN_00603720` | `0x00603720` | constructor of the status object | `+0x1960` set 1 in the constructor, then `SetRoaming` runs once |
| `FUN_0060deb0` | `0x0060deb0` | the Lua function `AbandonRace` | `FUN_005e8ba0`, `SetNextCheckpointPos(0)`, `ClearCheckpoints`, in play mode 2 two calls on the world (`FUN_005eed90`, `FUN_005ebf90`), then each racer's simable interface method at slot 3 (`+0xC`) |

The Lua registration list `FUN_0061e750` gives the addresses of the restart-related natives: `RestoreStartPositions`
`0x0060AA50`, `SaveStartPositions` `0x00604E90`, `ResetCopsForRestart` `0x0060AB10`, `AbandonRace` `0x0060DEB0`,
`SetAllStaging` `0x0060DF70`, `StartRaceTimers` `0x00604EF0`, `NotifyCountdownDone` `0x0060E160` (the last four have no
function in the project, so their bodies were not decompiled).

## 3. The restart routines

Read as raw bytes where Ghidra has no function there (`0x0060AA50`, `0x0060AB10`, `0x00604E90`):

- **`ResetCopsForRestart`** (`0x0060AB10`): loads the object at `DAT_0092C67C`, and if it is not null calls its virtual method
  at slot 2 with the argument 1. The object is the cop manager by the Lua name; slot 2 with 1 is "reset" **[unconfirmed]**.
- **`RestoreStartPositions`** (`0x0060AA50`): loads the object at `DAT_0092C674` and, if it is not null, calls its slot 1
  with 0 (not identified; probably the traffic or the AI vehicles). Then, for each racer `0 .. count-1`
  (`FUN_005dbef0` = the racer count, `FUN_005dbed0(i)` = the record), it calls two routines on the record:
  - **`FUN_005e96d0`**: if the record has a simable (`*record` is its id, looked up in the table at `DAT_0092FEB4`), packs eight
    dwords of the record, `+0x170 .. +0x18C` (the saved start pose written by `SaveStartPositions`), and calls the simable
    interface's method at slot 1 with that block and a flag 1: **put the racer's car back at its start pose**.
  - **`FUN_005e8300`**: **resets the racer record**: zeroes `+0x10 .. +0x18` (progress and percent fields), the seven flag
    bytes `+0x1C .. +0x22` (finished, out, and the like), `+0x24 .. +0x30`, `+0x34 .. +0x70` (checkpoint and lap counters),
    `+0xF8 .. +0x128` (lap and split times), and the 16 slots at `+0xA8 .. +0xE4` / `+0xB8 ..` (0 and -1: the checkpoint
    times and the time-bonus list); the three timers at `+0x14C/+0x150`, `+0x158/+0x15C` and `+0x164/+0x168` (a stopwatch
    each: a running stopwatch is folded to its elapsed value, then cleared and restarted from the clock `FUN_006e8de0`);
    `+0x12C .. +0x148`.
- **`SaveStartPositions`** (`0x00604E90`): for each racer it calls one routine (call target not resolved: the byte offset
  computed from the call is `0x0060053F`, where the project has no function) that stores the car's current pose into
  `+0x170 .. +0x18C`. It is called at the start of a race, so the saved pose is the grid pose **[unconfirmed]** (the call site
  was not read).

Together: a restart in the original resets the cops, puts every car back on its grid pose and zeroes every racer's progress,
checkpoint, lap, finish and timer state. The rewrite's table in the spec follows this.

## 4. The pause item `pm_RestartRace`

The names `pm_RestartRace` and the others are in the string table around `0x0089C294` and only referenced from unwind records
(see [career-parity-menus](career-parity-menus.md)). The vtable of the Restart item class is at `0x0089C25C`: `{0x0052B270,
0x0051D7C0}`. The first is the deleting destructor (`FUN_0052b270` -> `FUN_0052b290`, `thunk_FUN_007c7250`). The second is
the message handler; the project has no function at `0x0051D7C0`, so its bytes were read:

- it compares the second argument with the message id `0x0C407210`; on a match it takes the first argument, pushes the label
  `0x4D3399A8` ("Are you sure you want to restart the race? All race progress will be lost."), the style numbers `1`,
  `0xB4623F67` (twice), `0xE1A57D51`, `0x1A294DAD`, `0x417B2601`, `1` and a string at `0x0089736C`, stores `0xFBDF2EE3` in the
  global `DAT_0091CA2C`, and calls `FUN_00598C00`: **the confirmation dialog** (`FUN_00598C00` loads the localised string for the
  label, formats it into a 512-byte buffer and calls `FUN_00598AE0`);
- the dialog's answer is a game message; the code that compares `0xE1A57D51` sits at `0x00550181`, inside no defined function.

The Restart item in the menu builder is the class built with `FUN_0051D790(texture 0xB295A6B6, label 0xF893AFA1)` (it sets this
vtable); in the race variants of career it is `FUN_0051D790` too. The Resume Race item of the career race menu uses
`FUN_0051D730` (a different class: same picture, label `0xDED357E7`).

## 5. Dead ends

- Where the accepted confirmation goes (which message reaches which Lua state, and whether `restartrace` of the race graph
  ([career-scripts-events](../specs/career-scripts-events.md) section 1.1) is the receiver) was not followed: the code at
  `0x00550181` is not a function in the project and the dialog's callback table was not found. The rewrite restarts directly.
- The classes behind `DAT_0092C674` and `DAT_0092C67C` are known only by the offsets of the calls.
- `FUN_005fc560`'s property key `0x4393F69B` has no name in the executable's strings that was found.
- `SetAllStaging`, `StartRaceTimers` and `NotifyCountdownDone` have no function in the project, so the countdown after a
  restart is taken from [race-mode](../specs/race-mode.md) section 2, not from the code.
