# The post-race standings, statistics and stages pages, and the race leaderboard

- **Specs:** [race-mode](../specs/race-mode.md) section 11, [race-results](../specs/race-results.md).
- **Restricted sources read:** `speed.exe` v1.3 in Ghidra (read only: nothing was renamed, retyped or changed in the
  project), `GLOBAL/InGameB.bun` (`PostRace_Results.fng`, `HUD_SingleRace.fng`) through the repository's own readers, and
  the English language table (**[verified]**). Evidence tags: [docs/README.md](../README.md#evidence-tags).
- **Started:** 2026-10-21 from three screenshots of the original (standings, statistics and stages of a sprint).

## 1. One screen, three pages **[exe]**

`PostRace_Results.fng` is the screen of all three pages. Its class sets the same objects for each page and keeps the mode of
the race at `+0x1E0` (0 sprint, 1 circuit, 2 drag, 3 knockout, 4 tollbooth, 5 speed trap). The numbers are the race
type's (`RaceType` of `nfsmw-data`, the index of `EventIconType`), and mode 4's footer says `Tollbooths` and mode 5's
`Speedtraps` **[verified]** by the language table (an earlier note called mode 4 cash grab: that was a guess).
The page is `+0x1E4` (0 standings, 1 statistics, 2 stages), the racer count `+0x1CC`, the index of the winner `+0x1D0`
(the racer that has finished with rank 1, found by `0x005A19C0`, which also rebuilds the page) and the racer shown
`+0x1D4`. Three setup routines, one per page, are called as the page changes:

| Routine | Page | Title (`0x2D691760`) | Footer button text (`+0x1F0` string) |
|---|---|---|---|
| `0x005964A0` | Standings | `STANDINGS` `0xFF115FFF` | `Statistics` `0xD0B8AA33` |
| `0x005970E0` | Statistics | `STATISTICS` `0x4E706980` | `Stages` `0x1135F776` (sprint, drag); other modes use `0x7B8F45DF`, `0xAC23368C`, `0xAEF51E9D` |
| `0x005972E0` | Stages | `STAGES` `0x34BA50FF` (sprint, drag) | `Standings` `0x8159A0B2` |

So key 2 walks Standings, Statistics, Stages and back to Standings; each page's footer names the next page. The footer's other
buttons are Exit (Enter) and Restart (key 1). Standings has no racer filter. Statistics and Stages show one racer between the
two arrows (the shoulder buttons step through the racers); the page rebuilds for the racer shown.

The page headers are the three strings `0x586AB4A6`, `0x44AC8987`, `0x30EE5E68` of the package, set by label:

| Page, mode | Column 1 | Column 2 | Column 3 |
|---|---|---|---|
| Standings (modes 0 to 3) | `Position` `0x96B05F47` | `Name` `0xCE678AD3` | `Time` `0xB67DA102` |
| Standings (mode 5) | `Position` | `Name` | `0x7540FB04` (the speed) |
| Stages (modes 0, 2) | `Stage` `0xE8B7D527` | `Position` | `Time` |
| Stages (modes 1, 3) | the lap label `0x0000BF9C` | `Position` | `Time` |

The rows are built from row classes (`0x005951E0` standings, `0x00595670` stage rows of a sprint, `0x00595510` lap rows of a
circuit, `0x005958E0` and `0x005957C0` the cash grab and speed trap rows), each filling three strings of a `LINE%d_GROUP`.
Statistics of mode 0 are four rows, from `0x005969A0`, `0x00596A60`, `0x00596B20`, `0x00596C50`: `Top Speed` `0x0EF34382`,
`Average Speed` `0x57F4140A`, `Time Behind` `0xAB44ED8B`, `Traffic Collisions` `0x094BFDFC`. The last two are left out
when two flags of the game's settings object (`+0x12C` bits 3 and 6) are set (an online or special game **[unconfirmed]**).

## 2. What the screenshots show **[verified against the screenshots]**

- Standings of a sprint with four racers: the place, the name and the finishing time `m:ss.cc` (`2:52.51`). The player is first
  under the profile's alias (`stock`), the others under their drawn names ([race-names](race-names.md)).
- Statistics for the player: top speed `123 MPH`, average speed `79 MPH`, time behind `--` (the winner has none) and traffic
  collisions `3`. Speeds follow the speed unit setting (the code multiplies m/s by `2.23699` for imperial).
- Stages for the player in the same sprint: five rows `20%`, `40%`, `60%`, `80%`, `100%` with the place held when the racer
  crossed that share of the course and the race time then (`30.36`, `59.92`, `1:35.19`, `2:20.29`, `2:52.51`).
- Restart opens the in-race dialog with the header `ATTENTION`, a warning triangle, the text "Are you sure you want to
  restart the race? All race progress will be lost." (`0x4D3399A8`) and the answers `OK` and `Cancel`, Cancel focused.
- Quit on the main menu asks with the header `selection`, a question mark and Yes and No ("Are you sure you want to quit?
  Any unsaved changes will be lost.", `0x075CAC3B`), No focused.

## 3. The race leaderboard of the HUD **[exe]** **[install]**

`HUD_SingleRace.fng` holds the group `LeaderBoardGroup` (`0x5ED609A1`). The HUD element that owns it (constructor
`0x0059F0E0`) finds, for k from 1 to 4, `LBData_k` (the value string, `0x2AF89FD7` + k - 1), `LeaderText_k` (the place number,
`0x4183E0A1` + k - 1), `OL_ICON_k` (the online headset) and `LBBacking_k` (the row's backing, `0x4FDDD64C` + k - 1). Four
rows, in rank order. In the screenshot the player's row is the highlighted one with `-----` and the others show their gap in
feet (`-609 FT`). The strings carry two scripts each, `ENTER` `0x5079C8F8` and `LEAVE` `0x033113AC` (160 ms, position and scale tracks): the slide
of the rows when their text swaps between the names and the gaps. The update routine was read afterwards (it is vtable slot
1 of `0x008A237C`, at `0x0058C160`): [race-leaderboard](race-leaderboard.md) has the rules, the gap's sign and unit and what
feeds the rows.

## 4. The racer record and what the pages read **[exe]**

The race status object (`DAT_0091E000`) holds the racers from `+0x18`, `0x194` bytes each (`0x005DBED0`). The fields the
pages read (offsets in the record):

| Field | Meaning | Written by |
|---|---|---|
| `+0x08` | the name, drawn into the standings row | the race setup |
| `+0x10` | the rank now (1 for first); at the end, the final place | the ranking |
| `+0x18` | percent complete, 0 to 100: the laps done and the way along the lap over the whole course, clamped | `0x005E92E0`, every frame |
| `+0x1C`, `+0x1D`, `+0x1E` | out of the race: knocked out (`K.O.` `0x5D82DBA2`), totaled (`0xB7B75185`), blown engine (`0x01E66364`) | the race |
| `+0x21` | finished | the race |
| `+0xFC` | metres driven: the speed times the frame time, summed | `0x005E92E0`, every frame |
| `+0x100` | the top speed, m/s | `0x005E92E0` |
| `+0x118` | traffic collisions (an integer, zeroed at the start; **the code that adds to it was not found**) | **[unconfirmed]** |
| `+0x124`, `+0x128` | the drag marks: the race time when the top speed first reached 26.8218 m/s (60 mph) and when `+0xFC` first reached 402.335 m (a quarter mile) | `0x005E92E0` |
| `+0x12C` to `+0x138` | the race time at 20, 40, 60 and 80 percent | `0x005E3400` |
| `+0x13C` to `+0x148` | the rank at those moments | `0x005E3400` |
| `+0x14C` | the racer's timer: start, accumulated seconds, running flag; the value is `now - start + accumulated` while it runs | the race |

`0x005E3400` records one stage per call (a frame): it tests 80, 60, 40, then 20 percent and takes the first one whose
percent is reached and whose slot is still zero, then stores the timer's value and the rank now. A racer that jumps over
two shares fills the higher one first and the lower one the next frame.

The average speed is `+0xFC` over the racer's timer (`0x005E3390`; 0 for no time). The statistic rows are built only when
the race's own clock (`0x005E3060`) is above zero and `0x005DC7D0` allows it (it refuses a racer that has not finished and is
not out while the race still runs; after the race it always allows). Otherwise the row says `--` (`0x0FC1BF40`).

Values: speeds are `m/s * 2.23699` for miles per hour, and `* 1.60931` more for kilometres per hour when the player's unit
flag (`+0x3B` of the profile's settings) is 1, followed by the unit label (`MPH` `0x8569AB44`, `KM/H` `0x8569A25F`) and
printed with `%$0.0f`. Time behind is `|winner's timer - the racer's timer|`, and `--` for the winner (or when there is no winner).

The rows of the statistics page, by mode: sprint (0) `Top Speed`, `Average Speed`, `Time Behind`, `Traffic Collisions`;
circuit and knockout (1, 3) the same with `Lap Variance` `0x4121E8C4` before the collisions (`0x00596BC0`: the longest lap
minus the shortest, only when the racer has done more than one lap); drag (2) `0 to 60 mph` `0xCCBC22B3` (`0 to 100 km/h`
`0xB8CF16FC`) from `+0x124`, `Quarter Mile Time` `0x49FD5DCB` (`400m Time` `0x1C6F2A82`) from `+0x128`, `Perfect Shifts`
`0x680AC597` from `+0x114`, `Time Behind`, `Traffic Collisions`; tollbooth (4) `Top Speed`, `Average Speed`, `Traffic
Collisions`; speed trap (5) `Accumulated Speed` `0xD57E02AB` (`+0x120`, km/h in the record), `Speed Variance` `0x6EEABE8C`
(the greatest minus the least trap speed, both above zero) and `Speed Behind` `0x2E54B7ED`. The two flags of the settings
object (`+0x12C` bits 3 and 6) swap `Time Behind` and `Traffic Collisions` for `N2O Used` `0x114E759F` (`+0x108`, pounds to
kilograms): left out here.

The standings row (`0x005953F0`): the name, then in the time cell the time of the racer's timer when it has finished, or
`K.O.` / `Totaled` / `Blown Engine` by the flags, or `--` when the racer has not finished; the place cell is `+0x10`. The
time cell of a speed trap race is the accumulated speed.

## 5. The stage rows and the time text **[exe]**

The sprint and drag stage row (`0x00595670`, fill `0x005956F0`) prints `(index + 1) * 20` and `%`, the rank and the
time. Rows 1 to 4 come from the record's slots, the fifth from the final rank and the racer's timer, which is 0 (so `--`)
when the racer did not finish. A time of 0 prints `--` in both the rank and the time cell. The circuit row
(`0x00595510`, fill `0x00595560`) is the lap number, the place by cumulative time through that lap among the racers with
a positive time (`0x005DC320`: one more than the racers that were faster) and the lap's own time (`0x005DC280`); a racer
with the out flag `+0x1C` and a place below 2 prints `K.O.`. The tollbooth row (`0x005958E0`) has no middle cell: the
tollbooth's number and its time, and one more row after them. The speed trap row (`0x005957C0`) is the trap's number, the
racer's place by speed at it and the speed.

Times (`0x006547A0`, `0x00654710`, `0x006541B0`) count 4000 ticks to the second (the timer's float times 4000 plus a half,
cut to an integer) and print the hundredths cut, not rounded: `30.36` under a minute (no leading zero), `1:35.19` with
minutes, `h:mm:ss` past an hour. The standings and the stages use it; a pursuit result's `mm:ss.cc` is another text.

## 6. Dead ends and what was not read

- The code that counts a racer's traffic collisions (`+0x118`) was not found: the field is zeroed at the start and read by
  the page. The rewrite counts the player's contacts with traffic cars instead (`race-results.md`). **[unconfirmed]**
- `+0x114` (`Perfect Shifts`) and `+0x108` (`N2O Used`) were read only as far as the table above; the rewrite keeps neither.
- The second button has no game response in the package (`PAD_BUTTON4` answers only with a package message and a sound),
  so the host takes the page change from the pad itself (`ScreenLogic::pad_pressed`).
- The restart question's text is not an immediate of the executable (`0x4D3399A8` is hashed at run time): `OK`
  `0x417B2601` and `Cancel` `0x1A294DAD` are the pair of the game's alerts, found beside `Yes` and `No`. **[inferred]**
