# The race leaderboard of the HUD (`LeaderBoardGroup`) and the race information line

- **Spec:** [race-mode](../specs/race-mode.md) section 10.
- **Restricted sources read:** `speed.exe` v1.3 in Ghidra (read only: nothing was renamed, retyped or saved; functions were
  created in the open session to read code the project had not disassembled), `GLOBAL/InGameB.bun` (`HUD_SingleRace.fng`)
  through the repository's own readers, and `LANGUAGES/English.bin`. Evidence tags: [docs/README.md](../README.md#evidence-tags).
- **Started:** 2026-10-21, from a screenshot of the original during a sprint (four rows at the top right, the player's row
  highlighted with `-----`, the others `-609 FT`, a place number beside each).
- Earlier notes on the constructor: [race-results-screens](race-results-screens.md) section 3.

## 1. The element **[exe]**

| Address | What |
|---|---|
| `0x0059F0E0` | The constructor. Sets the vtables `0x008A237C` (the element: destructor `0x0059F390`, update `0x0058C160`) and `0x008A234C` (an interface at `this+0x30` for the code that feeds it, section 3), the update counter `+0x228 = 90`, the swap flag `+0x22C = 0`, the racer count `+0x38 = -1`, the player's row `+0x40 = -1`, and finds the objects. |
| `0x0058C160` | The update, once per HUD update (this is the vtable slot the decompiler did not resolve). |

Objects (`+0x230` the group, then four arrays of four): `LeaderBoardGroup` `0x5ED609A1`; the values `LBData_k`
`0x2AF89FD7 + k`; the places `LeaderText_k` `0x4183E0A1 + k`; the online headsets `OL_ICON_k` `0x51F5B673 + k`; the backings
`LBBacking_k` `0x4FDDD64C + k`, k from 0 to 3. In `HUD_SingleRace.fng` all of them are children of the group
(`0x000E1A9B`). The scripts, from the package **[install]**: the group `INIT`, `HIDE` (`0x16A259`); each backing `INIT` and
`HIGHLIGHT` (`0x249DB7B7`, 1200 ms); each string `INIT`, `ENTER` (`0x5079C8F8`, 160 ms) and `LEAVE` (`0x033113AC`, 160 ms);
each headset `INIT`, `HIDE`, `SHOW` (`0x1CA7C0`). `HUD_Drag.fng` and the split-screen HUD files have no such group, so a drag
race has no leaderboard.

## 2. The update **[exe]**

Per call, with `racers = +0x38`:

1. A flag of the state object the argument returns (`+5`) decides the group: set, it plays `INIT` on the group; clear, `HIDE`.
2. The counter `+0x228` is decremented. At 0 every row's `LBData` string plays `LEAVE`. Below 0, each row whose string is
   playing `LEAVE` and has run past its end (the script time is above its length) and is not already playing `ENTER` plays
   `ENTER`; if any row did, the counter is set back to 90 and the swap flag `+0x22C` is flipped. So every 90 updates the
   rows slide out, and then back in showing the other text. **The slide is the swap of the text, not a reaction to a change
   of place.** The update rate is not read; the rewrite takes 60 per second (a swap every 1.5 s) **[unconfirmed]**.
3. Rows 0 to `min(racers, 4) - 1`, only when `racers > 1`, set their text by the swap flag:
   - **flag 0:** `LBData_k` = `%s` of the racer's name (`entry+0x48`), `LeaderText_k` = `%$d` of an integer (`entry+0xB0`);
   - **flag 1:** `LBData_k` alone, in this order:
     1. the busted flag (`entry+0xBD`): label `0x532B5186`, `BUSTED`;
     2. the out flag (`entry+0xBE`): label `0x5D82DBA2`, `K.O.`;
     3. a speed-trap race (the race type routine `0x005FAA20` returns 5): `%$0.0f %s` of the racer's speed value (`entry+0x5C`,
        km/h) with `KM/H` `0x8569A25F` when the options byte `+0x3B` of the player options is set (metric), else the value times
        `0.27778` times `2.23694` with `MPH` `0x8569AB44`;
     4. the player's own row (`row == +0x40`): `-----`;
     5. any other row: the gap to the player. With `p` the row's percent (`entry+0x58`, a float) and `q` the player's:
        `p < q` prints `-%$0.0f %s`, else `+%$0.0f %s`, of `|p - q| * 0.01 * L`, times `3.28084` for imperial, with the unit
        `FT` `0x2E8496B1` (metric byte clear) or `M` `0xE2078322` (set). `L` is the race length, the float at `+0x4484` of the
        game object, which `0x005FEF30` builds: the sum of the straight lines between the points of the course (start,
        checkpoints, finish), for a lapped race the first lap plus `laps - 1` times a lap. So the gap is a share of the whole
        race turned into a distance, not a measured distance.
   - `%$` is the game's format flag for the locale's digit grouping ([career-rapsheet](../specs/career-rapsheet.md) section
     1.4), so a gap over 999 prints `-1,312 FT` in English.
   - The rows from `racers` to 3 (all four for one racer) have the four objects of the row hidden (`0x00514C70`).
4. Each backing: the player's row plays `HIGHLIGHT`, every other row `INIT`, if it is not playing it yet.
5. A flag at `+0x44`, set by the feeding code when the lead changes, calls `0x0057AB10`: a line `"%s\n+%s"` or `"%s\n-%s"` with
   the label `0x7771A159` (`SPLIT TIME`) for the generic message group, scripts `0xA19BB14C` and `0x5230FAF6`. That is the
   split-time line, not the board (the rewrite has the generic message group for its own lines).

## 3. What feeds it **[exe]**

The interface at `this+0x30` (vtable `0x008A234C`, slots in order): count (`+0x38`), `+0x3C` (the number of laps), the
player's row (`+0x40`), name (`entry+0x48`, rows 0 to 3 only), the integer at `entry+0xB0`, a float at `entry+0xB4`, a lap
record (`0x0058C590`), the percent (`0x005689C0`, stored at `entry+0x58`; it also keeps, per racer, the percent snapshots of the
stages page: every 25 percent for types 0 and 4, every 50 for 1 and 3, none for the drag), the busted flag, the out flag.
Entries are 0x78 bytes. The feeder is the big HUD update `0x006F13F0`: for the rows 0 to 3 it finds the racer whose rank
(`record+0x10`) is `row + 1`, so **the rows are the four best placed racers in place order**, calls the player's-row slot for
the racer that is the player's, and passes the racer's name (`record+0x4C` of the driver), percent (`0x006004D0`), busted
(`record+0x1F`) and out (`record+0x1C`) bytes. The count is the number of racers of the race (`0x005DBEF0`). This runs while
the racing state of the race status object (`+0x1960`) is 1.

Not read: what is in `entry+0xB0` and `entry+0xB4` (the rewrite shows the row's place number, which is what the screenshot
shows); the state object of step 1 (the rewrite keeps the group shown for the whole race); the player's row when the player is
not among the four (the slot is only written when the player is found, so the original keeps its last value; the rewrite
highlights nothing and measures the gaps from the player all the same) **[unconfirmed]**.

## 4. The language strings **[verified]**

`English.bin`: `0x2E8496B1` `FT`, `0xE2078322` `M`, `0x8569A25F` `KM/H`, `0x8569AB44` `MPH`, `0x532B5186` `BUSTED`, `0x5D82DBA2`
`K.O.`, `0x7771A159` `SPLIT TIME`.

## 5. The race information element (progress, laps, place, clock) **[exe]**

The element that owns `RaceInformationGroup`, `CurrentLapTime`, `Complete_Text`, `POSITION_GROUP`, `Milestone_TollBooth` and the
strings `0x1CDDD8D0`, `0x18A7ACD2` (the tollbooth's two numbers), `0x9C183BF8` (place), `0x3BBD6268` (racers) and `0x9CB5C95D` (the
progress number) is built at `0x0059EE10` (vtable `0x008A22F4`, update `0x0057A900`, the feeding interface `0x008A22CC`). The
update reads the race type (`0x005FAA20`: 0 sprint, 1 circuit, 2 drag, 3 knockout, 4 tollbooth, 5 speed trap):

| Part | Rule |
|---|---|
| Place group (`POSITION_GROUP`) | Not a tollbooth race: plays `INIT`, and the strings `0x9C183BF8` and `0x3BBD6268` take `%d` of the place (`+0x40`) and the racer count (`+0x38`). A tollbooth race (type 4): the place group plays `HIDE`, `Milestone_TollBooth` plays `ENTER`, and the strings `0x1CDDD8D0` and `0x18A7ACD2` take `%d` of the integers at `+0x54` and `+0x58` (not read what they count). |
| Progress, types 0, 4, 5 | The caption `Complete_Text` (`0x328F1B7C`) takes the label `0x59BB1918`, `COMPLETE`, and `0x9CB5C95D` takes `%d %%` of the racer's percent (`+0x50`, a float cut off to an integer): `99 %`, with a space. |
| Laps, types 1, 2, 3 (and with no race) | The caption takes the label `0x0000BF9C`, `LAP`, and `0x9CB5C95D` takes `%d/%d` of the lap now (`+0x44`) and the laps in all (`+0x3C`): `2/4`. |
| Clock | A flag at `+0x4C` plays `0x733C8147` on `CurrentLapTime` (not read); otherwise the time `+0x48` is formatted as the clock text. |

The rewrite before this read printed `99% ` and put the laps of a lapped race in the tollbooth milestone group; the
progress group now follows the table. Not done: the tollbooth branch (its two integers are unread, so the place group stays
and `Milestone_TollBooth` stays hidden), the clock flag, and the lap text for a drag race (the drag HUD shows the progress
text as before).
