# AI racers: the route, the grid and the targets

How an opponent finds its way round a race: the route from the start through the checkpoints, the grid it starts on,
and what it steers at. Rules for the cars are in [ai-driver-control.md](ai-driver-control.md); the ranking is in
[race-mode.md](race-mode.md) section 4. Evidence tags as in [docs/README.md](../README.md#evidence-tags).
Sources: `gameplay.bin` **[verified]**, `speed.exe` v1.3 ([provenance/race-mode.md](../provenance/race-mode.md))
**[exe]**, and the path finder spec [ai-pathfinder.md](ai-pathfinder.md) **[decomp]**.

## 1. The checkpoint list

The game hands the race's checkpoints to the status one by one (`ClearCheckpoints`, `AddCheckpoint`) and tells the
route which one is next (`SetNextCheckpointPos`) **[exe]**. The list is `Checkpoint[]` in order, then the finish
line. The next checkpoint is also the target of the player's GPS arrow and the minimap checkpoint blip
(`MMICON_CHECKPOINT`, [hud-minimap.md](hud-minimap.md) section 7).

Positions are the centres of the box triggers ([formats/gameplay-races.md](../formats/gameplay-races.md) section 3.1),
converted from map to physics space.

## 2. Route and length

- An opponent's route is the road path from its position to the next checkpoint centre, searched on the road
  graph with the racer path type of [ai-pathfinder.md](ai-pathfinder.md) (the same search as the cops, with the
  GPS-style look-ahead) **[decomp]**, and searched again when the car reaches the checkpoint or leaves the path.
- `Shortcuts[]` markers mark places where a racer may cut: when a racer's route passes within a marker's radius,
  it takes the shortcut with a probability between `ShortcutMinChance` and `ShortcutMaxChance` (values above 1 are
  percentages) **[verified]** data, **[unconfirmed]** rule. The rewrite does not take shortcuts yet.
- The route length is the sum of the straight lines start, checkpoint 1 ... finish, measured on the road
  (`RaceLength` is the editor's stored value, 0 for a few races) **[unconfirmed]**. It is only used for percent
  complete and for the catch-up error, so a straight-line sum is used when the road path is not yet known.
- A racer's progress along the lap is `1 - distance_to_next / leg_length` of its current leg added to the legs
  already done, over the sum of legs **[unconfirmed]**.

## 3. The grid

`racestart` is the front of the grid: a position and a heading. The player takes it. The opponents line up behind
and beside it **[unconfirmed]** — the rewrite uses two columns 3.5 m apart and 7.5 m between rows, the player
first, then the opponents in order, each aimed along the heading. `ForceStartPosition` (10 opponent slots) names a
marker the racer uses instead. A rolling start puts the cars on the same slots at `InitialPlayerSpeed`.

The grid is checked against the road: each slot is moved to the nearest road ground, as the traffic spawn does; a
slot with no ground is skipped.

## 4. What the car steers at

The racer think is the cop think without a quarry ([ai-driver-speed-skill.md](ai-driver-speed-skill.md) and the
`ai/cop.rs` think in the rewrite): a look-ahead point on the path to the target (30 m at rest up to 100 m at 100 m/s),
the curvature of the trail, the requested speed from the curvature, the skill and the governor. Differences for a
racer **[decomp]**:

- `race_running` is true, so the pursuit speed cap does not apply; the maximum is the car's own limit.
- While staging (before GO) the speed limit is `lerp(2.5, 3.0, skill)` mph and the car holds on the grid.
- The nitrous rules of section 9 of the speed spec apply.
- Opponents steer round other traffic and other racers through the trail occlusion; they do not avoid the player
  on purpose.

## 5. Reaching a checkpoint

An opponent reaches checkpoint `n` when its position crosses the box ([race-mode.md](race-mode.md) section 3); it
is reported with the same notification as the player's, so the rank list treats all racers alike. After the last
checkpoint it drives to the finish line, then (for a circuit) on to checkpoint 1 for the next lap. After the finish
an opponent keeps driving at cruising speed along the road and is removed when the race is over.

## Open questions

1. How the game builds its racer route (the path type and the shortcut rule are read from the data only).
2. The grid spacing and order of the original.
