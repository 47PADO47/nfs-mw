# Player driving parity: implementation and remaining gaps

This branch improves selected player-driving components. It does **not** establish
complete handling parity with the original PC game. The countdown rev behavior
is a known, user-reported defect in the playable build, despite passing component
tests. This report takes precedence over any interpretation that the launch
tests prove the complete race start works correctly.

## Known defect: countdown rev behavior

**Open, reported during playtesting on 2026-10-26:** engine revs during the race
countdown are buggy and behave differently from the original game. The exact
input sequence and RPM trace have not yet been captured for the remake.
Two related drivetrain defects have since been corrected: engaging-clutch play
was skipped, and old clutch RPM difference survived engaged/disengaged states.
Their contribution to the reported symptom has not been measured in a complete
countdown retest. Do not classify the whole defect as fixed by component results.
See [the drivetrain comparison](specs/player-countdown-drivetrain.md).

Initial grid gear/clutch sequencing and the ordering of engine, displayed-RPM,
staging and release producers remain comparison gaps. These are investigation
targets, not confirmed causes. Follow-up must compare throttle press, hold and
release during countdown and across GO, including simultaneous gas and brake,
at both offline physics rates. Compare engine RPM, displayed RPM, wheel speed,
gear/clutch state, pedal input and launch state against the original.

Acceptance requires stable countdown rev behavior and matching release behavior
with the same car, hardware, tuning and transmission. Numeric launch-strength
agreement alone cannot close this defect. No extra original-game recording is
required to publish this handoff.

## Implemented scope

- Human steering range/history corrections; AI range shaping remains bypassed.
- Explicit per-step simulation delta, including ground probes and consumers.
- Shared 60-Hz real-time physics by default, independent rendering, optional
  offline 144-Hz comparison mode, and endpoint-scaled Speedbreaker deltas.
  Network sessions retain 60 Hz. AI control algorithms are not retuned; shared
  cadence and elapsed-time propagation necessarily reach AI and props.
- Player launch band, strength, timer, torque/traction effects, staging restraint
  and localized perfect-launch notice. Countdown rev parity remains open above.
- Reference-tire nonzero-patch friction and locked-wheel sign-history fixes.
  Grip constants and the generic Regularized profile are not retuned.
- Installed comparison setup uses gameplay tire/jump/wall settings. Historical
  pedal reconstruction is explicitly diagnostic rather than exact-input proof.

See the [steering spec](specs/player-driving-parity.md),
[launch spec](specs/player-launch.md), [clock spec](specs/shared-physics-clock.md)
and [tire correction spec](specs/vehicle-tire-capture-corrections.md).

## Other unresolved or excluded behavior

- **Tire arithmetic:** Reference tires now use the original-style quantized angle
  helper and match every captured angle. Residual tire-force differences remain
  in other arithmetic. Exact per-call force equality is not claimed.
- **Complete driving:** matched-route trajectories, dynamic contact geometry,
  wall/chassis response, banks, landings and live Speedbreaker transitions need
  original/remake acceptance. Isolated endpoint agreement is narrower evidence.
- **Hardware and tuning:** captures cover a neutral-tuned Porsche 911 Turbo S
  with maximum upgrades plus Junkman and a neutral-tuned automatic Golf GTI with
  maximum ordinary upgrades, no Junkman, bodykit and spoiler. This is not a
  complete car/part/tuning matrix. Prefer shared attribute and component checks
  over requiring a manual recording for every combination.
- **Race presentation/context:** tachometer launch-band needle colors, initial
  grid sequencing, full drag heat/burnout context and complete drag race parity
  are not established by this work.
- **Flat tires:** handling parity was explicitly deferred. It is not added as
  an F5 option by this branch.

## Evidence and its limits

The selected original's executable code sections match the clean PC reference;
the executable difference is the large-address-aware header flag. Its installed
timing/input fixes still matter: the observed modded target ran at about 144 Hz.
The remake's default 60 Hz is an explicit policy choice, not proof that every
original installation has that cadence. See the linked provenance records.

Two original Golf sessions supplied 38,764 independently initialized tire calls.
After the fixes, wheel-lock and sign-history mismatches were eliminated in these
calls. With the angle correction, maximum remaining absolute longitudinal/lateral
force differences are 0.015625/0.0234375 N in one session and
0.1787109375/0.40234375 N in the other. Before that correction the corresponding
errors were 13.6015625/41.6399 N and 12.09375/57.7808 N. No
acceptance tolerance is invented for these residuals. This replay does not
reconstruct a complete vehicle trajectory.

The private engine comparison now supports independent boundaries and sequences.
Fifty-two synthetic one-step cases and four 16-tick sequences match the original
single-precision torque-loop outputs exactly for engine speed, transmission
speed, clutch RPM history, wheel torque and engine-braking state. Clutch and
wheel feedback are explicit boundary inputs. Suspension-side wheel-write effects,
stored angular acceleration and complete engine/countdown producers are outside
that comparison. Alternate original floating-point precision is recorded
separately; the live engine FPU control word remains unobserved.

A new installed-data regression exercises 244 directed cases across FWD, RWD and
AWD, nine distinct car/package pairs, 61 reachable tuning profiles, 60/144 Hz and
normal/quarter-speed deltas. It checks usable acceleration/braking and finite
state, not original trajectories. Existing broader car/part tests remain separate.

Local native-only validation before PR preparation passed 9,373 workspace tests
(868 ignored), seven Vulkan/DX12 smoke cases and 25 selected installed cases.
Clippy retained 12 known baseline diagnostics with zero new warnings; strict
warning-free Clippy is therefore not claimed. Deprecated Bevy rendering was
excluded. These counts describe the checked implementation, not countdown
playtest acceptance or every renderer/feature combination.

The packaged release passed install detection and free-roam rendering with both
physics settings. Release trace logging is compiled out, so those startup checks
do not independently measure runtime cadence. CPU regressions cover cadence,
render-frame grouping and original single-precision Speedbreaker endpoints.
There is no matched-route acceptance, manual countdown fix verification, or FPS
benchmark in this evidence. Private captures and installed assets are not shipped.
