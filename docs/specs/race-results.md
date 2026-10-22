# Race results: the standings, statistics and stages pages

- **Provenance:** [race-results-screens](../provenance/race-results-screens.md) (the executable's page builders, the racer record,
  the labels). Evidence tags: [docs/README.md](../README.md#evidence-tags).
- **Code:** `libs/blackbox-race/src/stats.rs` (the per-racer totals), `scenes/world/race/report.rs` (the report),
  `scenes/world/ai/hit_count.rs` (traffic collisions), `frontend/post_race/` (pages, screen, flow), `frontend/confirm.rs`
  (the restart question), `frontend/photo.rs` (the win photo's buttons).
- Related: [race-mode](race-mode.md) section 11 (the flow after the finish), [race-restart](race-restart.md).

## 1. What the screen is

`PostRace_Results.fng` is one screen with three pages **[exe]**. The footer has Exit (Enter), a second button (key 2, the
pad's fourth button) that is labelled with the next page, and Restart (key 1).

| Page | Title | Racer filter | Footer's second button |
|---|---|---|---|
| Standings (first) | `STANDINGS` | none | `Statistics` |
| Statistics | `STATISTICS` | the racer's name between two arrows | `Stages` (sprint, drag), `Laps` (circuit, knockout), `Tollbooths`, `Speedtraps` |
| Stages | `STAGES` (`LAPS`, `TOLLBOOTHS`, `SPEEDTRAPS`) | as above | `Standings` |

Key 2 walks Standings, Statistics, Stages, Standings. The shoulder buttons (Page Up, Page Down, the pad's L1 and R1) step
the racer shown through **all** racers (the player first, then the opponents in grid order), wrapping; they do nothing on
the standings. The racer shown is kept while the pages change. The screen opens on the standings.

**Who opens it.** The post-race flow shows it after a race that was not won, and the win photo's **Race Stats** button
opens it on the standings, over the photo. A won race shows the photo first (`photo.rs`): its Continue answers Exit and its
Restart asks the restart question (section 5).

## 2. The rows

Times are the game's text: the hundredths are cut, never rounded; `30.36` under a minute, `1:35.19`, `2:52.51`, `h:mm:ss`
past an hour. Speeds follow the speed unit setting (miles per hour: `m/s * 2.23699`; kilometres per hour: that times
`1.60931`), rounded, with the unit's label: `123 MPH`. A value that does not exist says `--`. **[exe]**

**Standings** (all modes): columns `Position`, `Name`, `Time`; one row per racer by final place (10 at most). The time is the
finishing time, `--` for a racer that did not finish and `K.O.` for a racer that was knocked out. The speed trap race's last
column is `Speed`, the sum of the racer's trap speeds. The player's name is the loaded save's alias (a Quick Race runs with a
profile that is loaded but not played), `player` when none is loaded **[ours]**; the opponents' names are
[race-names](../provenance/race-names.md).

**Statistics** of the racer shown, a label on the left and the value on the right:

| Race | Rows |
|---|---|
| Sprint | Top Speed, Average Speed, Time Behind, Traffic Collisions |
| Circuit, knockout | Top Speed, Average Speed, Time Behind, Lap Variance, Traffic Collisions |
| Drag | `0 to 60 mph` (`0 to 100 km/h`), `Quarter Mile Time` (`400m Time`), Perfect Shifts, Time Behind, Traffic Collisions |
| Tollbooth | Top Speed, Average Speed, Traffic Collisions |
| Speed trap | Accumulated Speed, Speed Variance, Speed Behind |

- **Top Speed:** the highest speed the racer had while racing. **Average Speed:** the distance driven over the racer's race
  time (its finishing time; the race's time for a racer that did not finish).
- **Time Behind:** the racer's finishing time minus the winner's; `--` for the winner and for a racer with no time.
- **Lap Variance:** the longest lap minus the shortest; `--` under two laps.
- **Traffic Collisions:** the number of times the player's car ran into a traffic car while racing; a car that stays in
  contact counts once, and a new hit on it counts after a second without contact. The opponents show 0. **[ours]**: the
  original's counter was not found (provenance section 6).
- **Drag marks:** the race time when the top speed first reached 60 mph (26.8218 m/s) and when the distance first reached
  402.335 m. `Perfect Shifts` shows `--`: the rewrite keeps no shift rating.
- **Speed trap race:** the accumulated speed is the sum of the racer's trap speeds, the variance the greatest minus the least,
  and `Speed Behind` the winner's sum minus the racer's.

**Stages** of the racer shown, columns `Stage`, `Position`, `Time` (circuits `Lap`, tollbooths `Tollbooth` and `Time`, speed
traps `Speedtrap`, `Position`, `Speed`):

- **Sprint and drag:** five rows, `20%`, `40%`, `60%`, `80%`, `100%`: the place held and the race time when the racer crossed
  that share of the course. The last row is the final place and the finishing time. A stage the racer never reached is
  `--` in both cells.
- **Circuit and knockout:** one row per lap of the course: the lap number, the place held at the end of the lap (among the
  racers that had crossed that lap's line by then) and the lap's time; `--` for a lap not run.
- **Tollbooth:** one row per tollbooth reached with the race time there, and one more row for the finish. **[unconfirmed]** which
  value the original's last row shows.
- **Speed trap:** one row per trap: its number, the place the racer's speed there ranks among all racers, and the speed.

## 3. Recording the data (the world)

`blackbox_race::RacerStats`, kept by the engine per racer **[exe]** `0x005E3400`, `0x005E92E0`:

- `Race::set_speed` keeps the top speed and the 0 to 60 mark; `Race::move_racer` adds the distance moved (and the quarter
  mile mark) and, after the progress is updated, records at most one stage: the highest of 20, 40, 60 and 80 percent that the
  racer has passed and that has no mark yet, with the race time and the place the ranking gives at that moment. Nothing is
  recorded before GO or after the racer is done.
- The session counts the player's traffic collisions from `TrafficWorld::take_traffic_collisions` while the player races.
- The report (`RaceReport`) is built when the race-over message ends: per racer the place, time, laps, `RacerStats`, the
  average speed, the time behind, the collisions, the checkpoint times, the knocked-out flag and the speed trap speeds, and the
  race's type and number of laps.

## 4. The pad

The package has no game response for the second button, so the host turns the pad's newly pressed buttons into
`ScreenLogic::pad_pressed`. The screen (and the photo) act on the pad bits themselves, not only on the package's messages, so
they work with the keyboard, a controller and the mouse alike:

| Button | Keyboard | Pad | Does |
|---|---|---|---|
| Exit | Enter | A | leaves ([race-loss-retry](race-loss-retry.md) for a race not won) |
| Restart | 1 | X | asks OK or Cancel (`PAD_BUTTON1`; the package also answers `PAD_BUTTON5`, one press asks once) |
| Next page | 2 | Y | Standings, Statistics, Stages, Standings |
| Racer before / after | Page Up / Page Down | L1 / R1 | steps the racer of the statistics and stages |
| Back | Esc, Backspace | B | the page before; from the standings of a win, back to the photo it was opened over; nothing on the standings of a race not won |

A footer button reached with the D-pad and pressed with A (or clicked) reports `BUTTON_PRESSED` with its GUID
(`ScreenLogic::pressed`); the second button's press shows the next page. One press reaches the screen as a pad bit, a message and a
button press in the same frame, so each acts once (`done`). The package switches its inputs off when a button is pressed (it is
leaving); this screen stays up until the flow clears it, so it switches them on again every frame. The footer keeps the package's own
key pictures (no controller glyphs are drawn for it). **[ours]**

## 5. Restart asks first

Restart on the three pages and on the win photo opens the in-race question over the screen: header `ATTENTION` with the warning
triangle, the text "Are you sure you want to restart the race? All race progress will be lost." (`0x4D3399A8`) and the answers
`OK` and `Cancel`, Cancel focused. OK answers the race with a restart; Cancel (or Back) closes the dialog and the page stays.
The pause menu's Restart asks the same question and runs `RestartRace` on OK.

## 6. Where Exit goes

| The race was started from | Exit (and the photo's Continue) |
|---|---|
| The challenge series | the series screen (the world unloaded) |
| The front end's Quick Race menu (Quick Play, Custom Race, Quick Race) | the Quick Race menu, the world unloaded, as the pause menu's `Quit to Quick Race Menu` does |
| The career, the main menu's free roam, a console command | free roam goes on; a won career race chains on to its `PostRaceActivity` |

Restart always runs the same race again and goes on driving. A career race that was not won asks for a retry before Exit leaves
([race-loss-retry](race-loss-retry.md)).

## 7. Open points

- The traffic collision counter, `Perfect Shifts` and `N2O Used` are not kept as the original does (provenance section 6).
- Opponents that do not finish before the race ends have no time: their standings cell says `--`, their stages are dashes,
  and their statistics cover the part of the race they ran.
- The tollbooth and speed trap pages are built from the rules above and tested with synthetic reports, not run in a race.
