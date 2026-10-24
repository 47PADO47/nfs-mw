# Road pursuit nitrous and action entry

Sources: locally inspected original PC executable and the CC0 reference
`AIActionRace`, under the project's spec-first policy.

## Capability and previous input

**[verified native]** The road pursuit action has its own nitrous decision
state. It does not depend on the close-pursuit action's nitrous flag. Its
effective extra torque boost is the minimum of its own and the player's boost.
A missing nitrous part contributes zero. During pursuit its usable fraction is
one, and its bottle time is its own full-burn duration in seconds.
Both values include the current tuning: extra boost is multiplied by
`1 + 0.25 * tuning`, and bottle duration by `1 - 0.25 * tuning`.

Read the previous held nitrous control with positive remaining tank capacity.
When that control was active, multiply the normal potential acceleration by
`1 + matched_extra_boost` before adding the grade correction. Merely pressing
nitrous this think does not retroactively boost that think's acceleration.

## Availability and switching

**[verified native]** A road action may consider nitrous only at speeds at least
10 m/s and below its requested speed, with positive matched boost, usable
fraction, engine and remaining usable bottle capacity. Let skill be `s`:

- Availability is `usable_fraction * lerp(0.33, 1, s)`.
- Starting requires capacity above
  `availability * lerp(0.5, 0.3, s)`.
- Continuing requires capacity above `1 - availability`.
- Available burn time is excess capacity times its own bottle duration.

The action refuses nitrous while turning around, stopping or staging. It also
refuses a player-controlled owner. At skill one, both minimum rest and the
initial uninterrupted burn interval are four seconds. The signed timer begins
at zero; active decisions add the think interval and clamp to at least zero,
inactive decisions subtract it and clamp to at most zero.

Scale speed and acceleration deficits by `lerp(0.25, 1, s)`. Starting requires
the timer strictly below the negative rest interval, an unoccluded navigator,
speed deficit above 15 m/s, and acceleration deficit above half the potential
acceleration. Once active, keep nitrous through the initial burn interval.
After that interval, stop when speed deficit is below 5 m/s or acceleration
deficit is below 15 percent of potential acceleration. Availability and the
stop/turn gates apply throughout. Speed-trap skill adjustments belong to race
context and remain outside the free-roam police adapter.

## Action entry

**[verified native]** Entering the road action initializes previous speed and
the requested-speed governor from the car's current speed, sets previous
acceleration to zero, and resets its nitrous timer to zero. Returning from a
close action must therefore discard stale road-action speed history. It must
not interpret the complete close-action speed change as one road think.

## Checks and boundaries

Regressions must exercise the road pursuit action with a target outside close
pursuit range, including a usable bottle, exhausted or absent capability,
rest/burn thresholds and a close-to-road handoff. A physical fleet check must
also show the decision reaching actual vehicle controls. These checks do not
establish full catch-up, collision or all-heat pursuit parity.
