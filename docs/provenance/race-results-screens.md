# The post-race standings, statistics and stages pages, and the race leaderboard

- **Specs:** [race-mode](../specs/race-mode.md) section 11, [race-results](../specs/race-results.md).
- **Restricted sources read:** `speed.exe` v1.3 in Ghidra (read only: nothing was renamed, retyped or changed in the
  project), `GLOBAL/InGameB.bun` (`PostRace_Results.fng`, `HUD_SingleRace.fng`) through the repository's own readers, and
  the English language table (**[verified]**). Evidence tags: [docs/README.md](../README.md#evidence-tags).
- **Started:** 2026-10-21 from three screenshots of the original (standings, statistics and stages of a sprint).

## 1. One screen, three pages **[exe]**

`PostRace_Results.fng` is the screen of all three pages. Its class sets the same objects for each page and keeps the mode of
the race at `+0x1E0` (0 sprint, 1 circuit, 2 drag, 3 knockout, 4 cash grab, 5 speed trap; the numbering follows which
builders each mode calls, not a name table **[unconfirmed]**). Three setup routines, one per page, are called as the page
changes:

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
