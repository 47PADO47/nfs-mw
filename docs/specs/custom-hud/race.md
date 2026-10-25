# Custom HUD: the race layout

What the Custom HUD shows in a race: an intro card, the countdown, a compact status strip, a small leaderboard and the
hand-off to results. The telemetry cluster and the map are the same components as in free roam, not copies. Built in
PR 8 (slots `RaceStatus` 13, `Leaderboard` 14, `Banner` 16); data from PR 3a. Index: [README.md](README.md).

## 1. Data

| Value | Source | Missing |
|---|---|---|
| place, racers | `RaceHud::place`, `RaceHud::racers` (`hud/race.rs`) | |
| lap n/m | `RaceHud::lap` (circuit, knockout) | `None` for sprint, tollbooth, speed trap |
| progress 0 to 1 | `RaceHud::progress` | |
| clock | `RaceHud::clock` (time left under a limit, else since GO) | |
| countdown step | `RaceHud::count` | |
| race type (PR 3a) | `nfsmw_data::gameplay::RaceType` | |
| gate n/m (PR 3a) | checkpoints passed and in all | `None` without checkpoints |
| next checkpoint position (PR 3a) | the course's next gate, world position | `None` |
| remaining distance (PR 3a) | `RaceSession.route` through `blackbox_roads::Route::locate` | `None`: the field hides |
| rival gap (PR 3a) | `Racer.checkpoint_times` (`blackbox-race`) | `None`: hidden, see section 5 |
| speed-trap points | `blackbox_race::SpeedTraps` | only in speed-trap races |

## 2. Intro and countdown

- **Intro card** (`Banner` slot, top-left anchor so the start and the road stay clear): the event name, the race type,
  the laps for a circuit or knockout, the number of opponents and their names where the race has them. A field the race
  data lacks (a location name, for instance) is not shown. It slides in at `RaceIntro` and leaves when the countdown
  starts. **[ours]**
- **Countdown** (`Banner` slot, top-centre, upper third): large numeric digits `3`, `2`, `1`, then `GO` from the
  catalogue, one `pulse` per step, from `RaceHud::count`. Under motion reduction each step is a static change.
- The intro does not delay the race: it is a view of the race's own intro timing, not a timer of its own.

## 3. The status strip

Slot `RaceStatus` (13), top-centre. A compact strip, not a leaderboard:

```
 POS 2/6   ·   LAP 2/3   ▮▮▮▮▮▮▮▯▯▯▯   4.2 km   ·   VALDEZ  1.42 s AHEAD
```

| Field | Shown | Format |
|---|---|---|
| position | always | `POS` heading, numeric `2/6` |
| lap or checkpoint | lap races: `LAP 2/3`; sprint and checkpoint races: `CP 4/12` | from `RaceHud::lap` or gate n/m |
| progress bar | Balanced and Advanced | 12 segments from `RaceHud::progress` |
| remaining distance | when `Some` | `4.2 km` / `2.6 mi`, one decimal |
| nearest rival | when the gap is shown (section 5) | the rival's name and the gap |
| race figure | speed trap: points total; tollbooth: the clock's time left | only where the race computes it |

The strip arrives with a `slide` at `Racing`. A change of place pulses the position field once. The last lap shows the
catalogue's `FINAL LAP` label in place of `LAP`.

Drag races show the strip with position and distance only: no lap, no gap; the cluster's shift cue is the main
instrument.

## 4. Leaderboard

Slot `Leaderboard` (14), top-right. Hidden in Minimal. In Balanced, three rows: the racer ahead, the player, the racer
behind (two rows when the player leads or is last). In Advanced, up to eight rows. Each row: place, name, and for the
rows next to the player the gap where it is shown. Knocked-out and busted racers keep the labels the Original
leaderboard uses (`hud/leaders.rs`). **[ours]**

## 5. The rival gap

**Definition.** The gap to the nearest meaningful rival is the difference between the two racers' passage times at
the **latest checkpoint both have passed**, from `Racer.checkpoint_times` (one time per checkpoint passage, laps
counted on). With `k` that checkpoint:

```text
g = t_player(k) - t_rival(k)
```

**Nearest meaningful rival:** the racer one place ahead of the player; when the player leads, the racer one place
behind.

**Sign convention.** The number shown is never signed. The word carries the direction:

| `g` | Shown | Meaning |
|---|---|---|
| `g < 0` | `1.42 s AHEAD` | the player passed checkpoint `k` 1.42 s before the rival |
| `g > 0` | `0.85 s BEHIND` | the player passed checkpoint `k` 0.85 s after the rival |
| `abs(g) < 0.005` | `LEVEL` | the same time at the shown precision |

The value is `abs(g)` with two decimals and a space before `s`. `AHEAD` is drawn with an up-chevron, `BEHIND` with a
down-chevron, so the direction reads without colour. The text comes from `hud.ftl` with the value as an argument.

**When it is hidden:**

- before the first checkpoint both racers have passed (no shared split yet);
- in drag, speed-trap and tollbooth races (`RaceType::Drag`, `SpeedTrap`, `Tollbooth`): their ranking is not about the
  time at a shared checkpoint;
- when there is no other racer.

**Update.** The gap changes only when both racers have passed a new checkpoint (event-driven, no interpolation
between checkpoints). It is a split, not a live estimate: the race simulation has no continuous time gap, and the HUD
does not invent one ([not-shown.md](not-shown.md)). In Advanced the strip adds the checkpoint it was measured at
(`at CP 4`).

## 6. Checkpoint guidance

World-space checkpoint markers, the confirmation on passing and the recovery arrow for a missed checkpoint are in
[world-markers.md](world-markers.md). The map highlights the race route and the next checkpoint ([map.md](map.md)).
Missed checkpoint and wrong way are Medium notifications ([notifications.md](notifications.md)).

## 7. Results

At `RaceResults` the race strip and leaderboard leave and the existing results workflow takes over
([race-results.md](../race-results.md)): final position, time, rewards, bounty, unlocks and Blacklist progress are its
screens. In Custom the results screens use the Custom menu components when PR 11a/11b land; until then they are the
current ones. The HUD adds no unskippable animation.

The egui race card (`hud/race_card.rs`: the speed-trap points and the result words) is routed through the strip and the
banner when the style is Custom (PR 8). Original and Remastered keep it.

## 8. Tests (PR 3a and PR 8)

- The gap's sign: player earlier gives AHEAD, later gives BEHIND, equal gives LEVEL; the nearest rival when leading.
- The gap hidden before the first shared checkpoint, for drag, speed-trap and tollbooth races, and with no rival.
- The gap changes only on a new shared checkpoint.
- The strip's fields per race type; missing fields absent.
- A restarted race shows one strip and resets the gap.
