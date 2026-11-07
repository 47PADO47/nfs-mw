# Stock quick-race driving follow-up

This follows the stock-car On/Off captures. Evidence is recorded in
[provenance](../provenance/race-stock-pace.md). It complements
[driver speed/skill](ai-driver-speed-skill.md) and [catch-up](ai-racers-catchup.md).

## Driving inputs

In physical simulation, the race action's acceleration governor uses the actual
body forward vector's vertical component for road grade. In emulated movement,
the grade contribution is zero. Preserve the existing positive catch-up scale;
do not substitute the cheat value for slope or add a fixed speed boost.

For a computer racer using non-drag Simple suspension, the tire steering-drag
coefficient is `0.23 + (0.5 - 0.23) * cheat` **[PC static]**. The cheat is the
existing bounded per-racer value. Apply the coefficient after normal per-frame
tire reset, without changing installed tire specs. A temporary maneuver override
retains its existing precedence. Other driver classes and human cars retain
their existing host settings; this does not claim a complete Simple tire model.
The native special case excludes drag style. Carry that distinction from race
setup rather than applying the non-drag coefficient to drag opponents.

## Catch-up switch and phases

Native quick-race error history continues recording while catch-up is Off.
Simulated rivals then receive zero output. An emulated rival forces the built-in
Hard interpolation and receives its raw offset even when the selected switch is
Off **[PC static, recorded runtime]**. On returning to physical simulation with
Off selected, clear that offset. Do not discard the sampled history at a mode
transition. Career adaptive/override logic remains a separate approximation.
Update quick-race history and output once per racer decision, using the elapsed
decision interval. Retain that output between decisions. The recorded PC calls
occur every two physics ticks, including at the captured 144 Hz physics rate.
The host clears physical Off output on every tick, including a mode transition
between decisions; this must not add a history sample or reset the sample clock.
Keep career's existing update cadence until its integration is verified.

Initialize quick-race sample clocks independently over 0..1 seconds, as the
native driver does. The host uses a separate seeded stream so these draws do not
change grid, route, traffic or police rolls. This matches the phase policy, not
the original executable's exact random sequence. Keep career's existing phase
until its own integration is verified. Ordinary quick-race error conversion uses
the existing `RatingBook` package-aware normalized top-speed score. Reuse its
stock/max population and Junkman blend rather than inventing a second estimator.
Unavailable rating records retain the previous fallback of one.

## Diagnostic comparison

Use each capture's actual rival model identities and horizontal starting poses.
The On and Off captures contain different fields; do not compare their times as
if only the switch changed. Fixture overrides must not change production car
selection or grant rival hardware to the player. The user confirmed stock
Carrera GT, no Junkman and unchanged tuning in both captures.

Advance the recorded player on the race clock, retaining loading pauses in the
report. Cross the actual host finish gate and report checkpoint times, actual
finishes and projected times separately. The last recorded velocity may continue
for at most 0.25 seconds to cross the host's center-based finish trigger; retain
that bound and the native finish timestamp in the evidence. Do not inject finish
flags, gates or fabricated rival finish times.

Count severe per-frame speed losses as diagnostics, not automatically as crashes.
Report physical and emulated frame samples. A full-course completion pass alone does not
establish native pace, exact collision frequency or identical wheel dynamics.

The two new captures show player-engine staging ticks at approximately 1/144 s
and racer updates at approximately 1/72 s. Their comparison must also exercise
the existing supported 144 Hz host cadence. Keep a separate 60 Hz result; do not
attribute a cadence change to a driving-code improvement or silently change the
normal game default. The racer think period remains two physics ticks.
