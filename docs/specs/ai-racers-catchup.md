# AI racers: the catch-up (rubber band)

How the opponents are kept near the player. The driver side (the glue error, how skill and the cheat scale the
car) is in [ai-driver-speed-skill.md](ai-driver-speed-skill.md) sections 8.2 and 8.3; this page adds the race
status side, the part that page lists as open question 3. Evidence tags as in
[docs/README.md](../README.md#evidence-tags).

## 1. What is known

- Races carry the switches and gains `CatchUp` (True everywhere), `CatchUpOverride` (False everywhere),
  `CatchUpSkill`, `CatchUpSpread`, `CatchUpIntegral` (5e-5) and `CatchUpDerivative` (0.001, 0.005 in four races)
  **[verified]**. With `CatchUpOverride` False the game uses built-in tables, so the fields are not used
  ([formats/gameplay-races.md](../formats/gameplay-races.md) section 3.3).
- Once a second the driver of each racing, non-staging racer records
  `glue_error = (race_length / 100) * (average_player_percent - own_percent) * performance_ratio` into a
  10-sample history, with `performance_ratio = 1 + (1 - player_top_speed_rating) * 0.5` **[decomp]**.
- The status turns that history into `glue_output` and a signed `glue_skill` in `ComputeCatchUpSkill`; the driver
  adds `glue_skill` to the base skill, scaled by a table over the base skill: a positive glue by `[0.33, 0.66, 1.0]`
  (0.5 in a speed-trap race), a negative one by `[1.0, 1.0, 0.66]` (0.5 in a speed-trap race) **[decomp]**.
- The cheat is `clamp(base_skill + glue_skill - 1, 0, 1) * 0.5`, at most 0.5, and it scales acceleration (1 to 1.5),
  drive torque and nitrous rates (section 8.3 of the speed spec).

## 2. `ComputeCatchUpSkill` (the rewrite's version)

The function is not in the sources read and its body was not located in the executable **[unconfirmed]**. The
rewrite uses a plain PID on the glue error, using the gains the data names, which fits the field names:

```
integral   = clamp(integral + glue_error * dt, -I_MAX, I_MAX)
derivative = (glue_error - previous_error) / dt
glue_skill = clamp(KP * glue_error + CatchUpIntegral * integral + CatchUpDerivative * derivative, -1, 1)
```

- `KP` is `CatchUpSkill` (default 0.2) per `CatchUpSpread` (default 1000) metres of error, that is
  `KP = CatchUpSkill / CatchUpSpread * 100`; the error is in route percent times the route length over 100 (metres
  behind the average player when positive) **[unconfirmed]**.
- The history keeps 10 samples, one per second; the PID takes the newest and the mean of the history for the
  integral part.
- With the race's `CatchUp` False, the glue skill is 0.
- A racer is not glued while staging, after it has finished or is out, or when the player has not yet left the grid.

This keeps the field close: an opponent behind the player gets a positive glue and a mild boost, one far ahead
gets a negative glue and slows.

## 3. What the rewrite does (status)

`libs/blackbox-race/src/catchup.rs` holds the controller; `race/flow.rs` (`race_glue`) runs it once a second for every
opponent still racing while the player has not finished, with the player rating taken as 1, and scales the result by the
base skill ([ai-driver-speed-skill.md](ai-driver-speed-skill.md) section 8.2, `race/setup.rs` `scale_glue`). The skill
the driver uses is `clamp(base + glue, 0, 1)` (`RacerState::skill`). The race's `CatchUp` switch turns it off. The
`CatchUpSkill`, `CatchUpSpread`, `CatchUpIntegral` and `CatchUpDerivative` values are not read from the data yet (the
defaults of the `basicrace` template are used, which are the values of 235 of the 255 races).

## 4. Open questions

1. The original `ComputeCatchUpSkill` and the built-in tables used when `CatchUpOverride` is False.
2. The unit of the error in the gains.
