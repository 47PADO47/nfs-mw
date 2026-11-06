# Optional police limits

These are remake enhancements. Reference pursuit behavior remains the default.
The F8 Traffic panel groups them under **Police limits (enhancement)**.

- `enhanced`: off by default; controls whether any of the overrides below takes effect.
- `max_active_cops`: 8 to 256, initially 16 for the optional mode. Applies to all
  active police cars: patrols, pursuing cars, roadblocks and support together.
- `wave_percent`: 100 to 2000, initially 100. Above 100 the chase takes the heat
  rows' counts scaled by it, rounded, a non-zero count never below one: each wave
  car type's count (the helicopter stays one), `FullEngagementCopCount` (the cops to
  evade before the backup) and `NumPatrolCars` (the cool-down search). The
  reinforcement intervals (`TimeBetweenFirstFourCopSpawns`, `TimeBetweenCopSpawns`)
  are divided by it, never below the 0.2 s of the fast-spawn alternation, so the
  larger wave arrives in the reference wave's time: at the reference intervals cops
  are lost about as fast as they come, and a larger count alone barely showed on
  the road. A reinforcement tries one seek-walk spot and one fallback-disc spot a
  step at 100 %, as the original does, and one more of each for every further
  multiple of the scale, up to eight: each further seek spot lies another 40 m
  (the spawn spacing) along the walk's road, and the fallback samples the disc
  again. With one of each, the spacing from the cop placed last and the disc's
  points off the cop roads held a scaled wave to about one new cop a second.
  Spawn weights are unchanged.
  A change applies to a running chase at once: the chase keeps the reference rows
  and rebuilds its working rows from them, and the cops still to evade scale by the
  same ratio while those already evaded stay counted. A pursuit context change
  mid-chase takes the new context's rows at the chase's scale.
- `heat_cap`: zero preserves event/career limits; 1 to 10 overrides their maximum.
  The authored final pursuit remains fixed at six. Special heat rows are exposed
  as an explicit enhancement without claiming they are ordinary career tiers.

The same table holds `bounty_fixes`, which is not a limit and does not depend on
`enhanced`: on by default (also for an old file without it), it builds in the bounty
bug fixes of [ai-pursuit-heat.md](ai-pursuit-heat.md) section 5.5; off restores the
original's takedown bounty rules. The F8 panel shows it as the "Bounty fixes"
checkbox below the limits.

The vehicle budget police cars share with traffic and the player (never under 20) grows by the active-car limit's
excess over the reference, so the extra cops do not wait on traffic slots. Without that the budget held a 400 % wave
to about 13 cops on the road.

The active-car override does not alter the wave's composition or the finite
evasion/backup budget. Increasing it permits the existing heat row and support
requests to use available room. It does not promise 256 active cops at every heat:
the reference waves are 4, 5, 7, 8 and 10 cars at heats 1 to 5, so at 100% the
limit binds only at heat 5 and no value above about 12 changes a chase. More cops
come from `wave_percent`, which the active-car limit still caps.
Disabling the override restores the configured reference/INI cap, and restoring
event heat bounds clamps an active pursuit without resetting its statistics.

Settings apply through the existing traffic-settings propagation path and use
the `[pursuit_options]` TOML table. They participate in F8 Save for startup,
named presets, A/B restoration and Traffic Reset section. Old files default to
disabled; unknown fields survive a write. Invalid field types do not partly apply.

The development Heat buttons remain explicit forced starts rather than saved
heat-progression overrides. Changing only the active-car limit preserves their
forced range. An enabled, nonzero heat cap takes precedence over a development
start; the final pursuit's fixed heat six takes precedence over both. Leaving
an event clears the development override for the restored event/career bounds.
No original-game installation files are changed.
