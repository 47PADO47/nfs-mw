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
  percentages) **[verified]** data, **[unconfirmed]** rule.
- **The rewrite's route [ours].** The whole route is built once when the race starts: the road from the grid through
  every gate of every lap, with the race's barrier lines closing the roads they cross (`blackbox_roads::build_route`,
  `ai/race_route.rs`). Each leg tries a route search with the lines, then the same with the lines lifted within 60 m of
  its ends, then a plain racer search with no lines; a leg whose road does not pass through its gate (its centre line
  sampled every 2 m) does not count, and a leg with none is missing. On the install 2,207 legs of 217 courses give
  2,106, 55, 3 and 43 missing (`race_routes`, a real-install test). The network's barrier flags are clear again
  afterwards, as before.
- **Shortcuts in the rewrite [ours].** A marker's shortcut is the road from the start of the route's nearest leg
  through the marker to the leg's end, kept when it is at most 1.25 times the leg (661 of the 971 markers of the
  courses). Each racer rolls once per shortcut on the grid, with the chance `lerp(min, max, skill)`, from a seed of the
  race and the racer, and drives its own copy of the route with the legs it rolled for replaced (`race/shortcuts.rs`).
- **Following the route [ours].** When a racer searches again (every second, at a dead end, and when it reaches a
  gate) its navigator follows its route instead of the shortest road (`ai/racer_route.rs`): from the navigator's own
  segment when that is on the current or the next leg; else from the nearest point of those legs when the car is within
  12 m of it on the ground and 3 m up or down (so not on a bridge over the route); else along a racer path to the point
  80 m further along the route. A leg with no road keeps the plain search to the gate. A racer on its route is not
  snapped back to the road nearest it when its look-ahead falls far behind (that cut corners and dropped its trail).
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

## 6. What the rewrite does differently from a plain path

- **Aim past the gate.** A path ends at its goal and a car at the end of a path stops. An opponent therefore drives to a
  point 40 m beyond the gate's centre along the gate's heading, and the gate is crossed on the way (`race/flow.rs`,
  `next_aim`) **[unconfirmed]**. The player's GPS blip and the checkpoint blip use the gate's centre.
- **Far racers keep driving [ours].** The world streams its collision around every AI car
  (`ai/policy.rs`, `residency/traffic_support.rs`), so a racer far from the player is simulated like a near one and
  follows its route. Only a racer whose surroundings are not loaded (it outran the streaming) switches to the stand-in of
  [ai-simulation-lod.md](ai-simulation-lod.md) section 3, after 0.25 s off the loaded world: the car is moved towards its
  drive target at the stand-in speed of section 3.2 (braking at 30 m/s², speeding up with `lerp(0.5, 1, skill)` of its
  acceleration, at most `lerp(0.75, 1, skill)` of its top speed), level and with no ground, at the think rate's targets
  (`ai/car/stand_in.rs`, `CarSim::place_kinematic`). Its gates are crossed by that travel, as a driven car's are. After
  0.25 s back on the loaded world it is put on the ground where it stands, rolling at its stand-in speed. A stand-in is
  never stuck. The straight-line emulation this replaces (a frozen car whose race place slid to each gate's centre, then
  was put back on the road when the player came near) counted gates the car never drove to and could finish a race off
  the route.
- **No gate for a teleport [ours].** The race engine counts no gate for a move longer than 60 m in one tick
  (`RaceConfig::max_move`): a reset, a fall back onto the road or any placement only moves the racer.
- **Stuck racers** reverse with the same recovery as a cop (section 7.1 of [ai-driver-control.md](ai-driver-control.md)).

## Open questions

1. How the game builds its racer route (the path type and the shortcut rule are read from the data only).
2. The grid spacing and order of the original.
