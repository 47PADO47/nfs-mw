# AI racers: the opponents of a race

Who the opponents are, which car each drives, how skilled it is, and how it is created and removed. The route
and ranking are in [ai-racers-route.md](ai-racers-route.md), the rubber band in
[ai-racers-catchup.md](ai-racers-catchup.md). The driver layer (PID control, speed limit, skill) is already
specified in [ai-driver-control.md](ai-driver-control.md), [ai-driver-control-pid.md](ai-driver-control-pid.md)
and [ai-driver-speed-skill.md](ai-driver-speed-skill.md); this page only adds what the race decides.
Evidence tags as in [docs/README.md](../README.md#evidence-tags). Sources: the data in `GLOBAL/gameplay.bin`
**[verified]**, `speed.exe` v1.3 strings and the `GRaceStatus` calls
([provenance/race-mode.md](../provenance/race-mode.md)) **[exe]**, and the specs above **[decomp]**.

## 1. Who races

- A race lists its opponents in `Opponents` (0 to 3 characters; 108 races have none, 45 boss races have one, 101
  street races have three) **[verified]**. The racer list of a race is the player plus these.
- A quick race replaces a boss opponent by the generic `character_smart` and may set the opponent count, the
  lap count, the difficulty and the traffic ([formats/gameplay-races.md](../formats/gameplay-races.md) section 5).
  The rewrite's quick race asks for 3 opponents, as the street races have.
- The player's racer is human; every other is a driver of class racer (`DRIVER_TYPE_AI_RACER`, goal
  `AIGoalRacer`) **[exe]** strings, **[decomp]** goal.

## 2. The opponent's car

- A character carries a `PresetRide` (a front-end preset car) or a `CarType`/`CarTypeLowMem` (a `pvehicle`)
  **[verified]**. The street rivals have neither; the game picks a stock car for them
  ([formats/gameplay-races.md](../formats/gameplay-races.md) section 4).
- **The rewrite's pick** when the character names no usable car: a stock car of the install in a fixed order that
  starts from the race's hash, so that the same race gives the same cars **[unconfirmed]**. Cars named by
  `PresetRide` resolve through the front-end preset data when the car is installed; otherwise the pick above is used.
- **The opponent's name** is fixed in a career race when the data fixes one (a Blacklist rival, the prologue's opponents,
  [racer-names.md](racer-names.md)); every other racer's is drawn from the language table's `RACERNAME_000` to `RACERNAME_149`
  while no other racer of the race has it, as the original does **[exe]** ([provenance/race-names.md](../provenance/race-names.md));
  mods add names with `[racers]`. The HUD leaderboard and the results screens show it.
- **Performance matching.** An opponent's top speed, grip and acceleration are limited to the player car's own,
  scaled by the character's `MinimumAIPerformance` ([ai-driver-speed-skill.md](ai-driver-speed-skill.md) section 2):
  0 means always matched down to the weakest player rating, 1 means no matching **[decomp]**. The rewrite uses the
  cop matching of `ai/cop.rs` (`matched_limits`) with the bias taken from `MinimumAIPerformance`.

## 3. Skill

`base_skill` ([ai-driver-speed-skill.md](ai-driver-speed-skill.md) section 8.1) **[decomp]**:

- quick race: `[0.15, 0.40, 1.00]` for easy, medium, hard (`[0.15, 0.40, 0.80]` with the catch-up setting off);
  the difficulty comes from `DifficultyLevel`: below 34 easy, 34 to 66 medium, 67 and above hard **[exe]**
  (the quick race writes 33, 66 or 100);
- career: `SkillLevel / 100` of the character (the street rivals' and bosses' values are in
  [formats/gameplay-races.md](../formats/gameplay-races.md) section 4), adjusted by the adaptive difficulty
  (taken as 0 here **[unconfirmed]**).

## 4. Creating the field

On entering a race ([race-mode.md](race-mode.md) section 2) the world:

1. removes the free-roam traffic and any pursuit near the start, or turns them to the race's `TrafficLevel`
   percent (0 for drag) and `CopsInRace` **[unconfirmed]** — the rewrite sets the traffic target to
   `TrafficLevel`/100 of the setting and bars the cops unless the race is a pursuit race or challenge (`race-mode.md`, police in
   races; the rewrite ignores `CopsInRace`) **[ours]**;
2. places the player on the start marker and the opponents on the grid, [ai-racers-route.md](ai-racers-route.md)
   section 3, each in staging (held) unless the start is rolling;
3. gives each opponent a racer goal with the race's route, `AIGoalRacer` with `SetRacerGoal` **[exe]**.

Opponents are not removed by the traffic's distance rules while the race runs, and never respawn; a racer that is
wrecked or stuck for long is recovered by the stuck rules of [ai-driver-control.md](ai-driver-control.md)
section 7. When the race ends, the opponents drive on for a while and are removed with the rest ([race-mode.md](race-mode.md)
section 11).

## 5. Minimap

Racers appear on the minimap with the `MMICON_AIRACER_0..7` icons **[exe]** string; the HUD style `racer` draws
one per racer (`hud/blip_styles.rs`). The rewrite fills one blip per opponent from the race state each frame,
with the car's heading. Racers that have been knocked out or have finished are not drawn **[unconfirmed]**.

## 6. What the rewrite does (status)

- The opponents come from the race's `Opponents` (or the generic `character_smart` for a quick race or a race with none
  when a count is asked for): `race/setup.rs` (`opponents_of`), with the car from the preset ride (`CarTables::preset`),
  the `CarType`, or a stable pick from the cars the car lot sells (`stock_car`). A car whose model does not load (the
  generic character's `trueno` has no physics in this install) is replaced by the next stock car.
- Skill: `base_skill` of section 3; matching: `racer.rs` (`matched_limits`) with the character's `MinimumAIPerformance`
  as the bias between the weaker of the two cars and the racer's own numbers.
- Spawn on the grid: `ai/racers.rs`; the cars are held with the brakes until GO (rolling starts are not held and start
  at `InitialPlayerSpeed`, unit assumed m/s). They are exempt from the traffic's removal rules and count in its budget.
- Far racers are emulated: [ai-racers-route.md](ai-racers-route.md) section 6.

## Open questions

1. How the game picks the stock cars of the street rivals.
2. The adaptive difficulty value.
3. The exact traffic and pursuit changes at the start of a race (the Lua graphs).
