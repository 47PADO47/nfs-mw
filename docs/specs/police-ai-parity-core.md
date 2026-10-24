# Police integration corrections

This supplements `ai-pursuit.md`, `ai-pursuit-formations.md`, `ai-helicopter.md`,
`ai-pursuit-heat.md` and `cop-siren-audio.md`. Original behavior and numeric
tuning retain those documents' evidence tags and sources (the CC0 reconstruction
`dbalatoni13/nfsmw`, read for understanding, and installed data). No recording or
new runtime measurement establishes PC equivalence here.

## Helicopter sensing

The target position, facing and **full linear velocity** come from the same
physical body sample. A sliding or airborne target must not be replaced with
facing multiplied by longitudinal speed. The existing controller already accepts
three-dimensional velocity; this is an adapter correction.

Height-sheet queries expose elevation and an upward plane normal from the same
triangle. Preserve the existing first-covering-triangle selection. Convert map
coordinates `(u, v, height)` into physics `(-v, height, u)` before supplying the
normal. Degenerate triangles cannot provide a floor. This supplies the surface
normal required by `ai-helicopter.md` section 4.2; the controller's documented
uncertain side-push formula remains uncertain.

Flight control and body integration are separate from the helicopter's slower
behavior decision. `AIVehicleHelicopter::Update` updates goals and fuel; its
`OnDriving` consumes the retained destination and current rigid body. Integrate
the retained goal every physics step and interpolate adjacent physical poses
for drawing. Spatial sound reads the continuously integrated body. Do not hold the whole body still between AI
updates. Fuel and skid clocks consume the supplied elapsed seconds, not a
hard-coded assumed think duration. **[decomp]** from the original vehicle's
`Update`, `UpdateFuel` and `OnDriving`, read for understanding on 2026-10-23.
An airborne helicopter reads current pursuit-row sight distance, search radius
and skid-hit eligibility after heat/context changes. Fuel capacity is selected
on spawn; changing the row does not refill its remaining fuel.

## Support lifecycle

As specified in `ai-pursuit-formations.md` section 6, a heavy support contingent
leaves when the target is lost or collapse begins. Send the placed cars of an
active heavy request to the existing flee driver and reset the request. Leader
support follows its own timer. Pending requests retain their original retry
semantics and consume the option's duration while placement is blocked. This
includes heavy roadblock requests; loss of contact pauses their placement, not
their expiry. A pending heavy roadblock blocks another ground-support roll.
Activation restarts the duration. The support priority roll and ordinary selection
use independent random samples; do not condition ordinary selection on a failed
priority draw's value. Keep deterministic simulation ordering.

## Offences and observations

Resisting arrest is recorded when the chase enters cooldown, not on every failed
raw sight test inside the tracked-sight grace (the original state transition in
`ai-pursuit.md`). Do not invent missing infraction-manager thresholds.

Speech uses each cop's actual retained sight answer, not the global answer
combined with that car's distance. World observation and speech observation must
agree on whether a particular cop sees the target.

The pursuit meter's display gate also applies to a pursuit cop's sight test:
allow it while cooldown is not displayed, or after the mode timer exceeds
2.5 seconds. During the initial cooldown display gate, treat its target as
out of sight. Patrol and roadblock sight retain their own rules.
**[decomp]** `AIPursuit::PursuitMeterCanShowBusted` and
`AIVehicleCopCar::CanSeeTarget`.

Pursuit units now advance visibility clocks on their actual staggered eight-tick
AI tasks. Their successful-query age is -0.25 seconds; the pursuit aggregates the
minimum actual active-unit age after advancing its shared timer. It does not
repeatedly refresh sight from a held boolean. Patrol and roadblock rules remain
separate. See [unit visibility clocks](ai-pursuit-visibility-clocks.md) for the
constructor/patrol transitions, activity filtering and sensing-order boundary.

## Cadence

The PC scheduler clamps a task rate at one, adds that rate to a phase each
update, and runs the task when the phase reaches one. Fixed tasks receive the
simulation delta divided by their rate. Variable tasks receive accumulated
elapsed frame time, then clear that time bank. **[PC static verified]** This
corrects the old reconstruction's unsolved variable-task argument and the old
spec's mistaken 4 Hz recommendation. Rates are tick fractions, not seconds.

Cop rate 0.125 means eight updates; pursuit rate 0.25 means four updates. At
60 Hz these correspond to 7.5 and 15 decisions per second. Preserve the shared
deterministic physics timeline, run pursuit decisions every four simulation
ticks, and pass their accumulated seconds. Keep physics/busted integration and
contact collection at the simulation rate. A cooldown multiplication of 0.93
belongs to a pursuit decision, not every physics step. The original variable
task's dependence on presentation frame grouping is intentionally replaced with
deterministic simulation ticks; this is not a measured comparison at other frame
rates.
