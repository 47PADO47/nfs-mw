# Player driving parity: implementation and remaining gaps

This branch improves selected player-driving components. It does **not** establish
complete handling parity with the original PC game. Earlier countdown and gauge
defects improved through local fixes; the user now reports that the build works
well. That qualitative acceptance does not establish exact numerical parity.

## Local gauge and countdown changes

A historical quick Golf GTI test found that the then-delivered build had
jittery needle motion and lacked launch-range color. The local candidate now
routes engine display RPM for staging/drag and existing audio visual RPM for
normal driving, interpolates physical gauge snapshots for rendering, and adds
the original staging-only needle RGB complement. It also uses physical speed
magnitude for the speed digits. See [the gauge spec](specs/player-gauge-presentation.md).
These are source-path corrections, not a closed live-jitter report. The user
reports that the latest local candidate feels good in playtesting; this is
qualitative acceptance, not a matched gauge or countdown trace.

The user reports that the gauge candidate seems better. This is qualitative
feedback, not complete countdown acceptance. The next local candidate also
blocks staged shift-button requests above first, processes shifts before
auto-reverse, and clears drag launch rules and drag RPM presentation at race
teardown. Brake-only reverse during staging is retained: original PC input
instructions confirm it. See [control provenance](provenance/countdown-controls.md).
GO remains after the final staged physics tick, matching retained callback
captures. Queued input across non-cinematic placement/restart transitions and
complete first-unstaged-frame behavior still need comparison.

## Countdown: improved in playtesting; exact comparison open

**Original report on 2026-10-26:** engine revs during the race countdown were
buggy and behaved differently from the original game. The latest user test
reports that the updated build works well; subtle differences remain difficult
to judge. The exact
input sequence and RPM trace have not yet been captured for the remake.
Two related drivetrain defects have since been corrected: engaging-clutch play
was skipped, and old clutch RPM difference survived engaged/disengaged states.
Their contribution to the reported symptom has not been measured in a complete
countdown retest. Do not classify the whole defect as fixed by component results.
See [the drivetrain comparison](specs/player-countdown-drivetrain.md).

Initial grid gear/clutch sequencing and the complete engine/displayed-RPM
producer chain remain comparison gaps. These are investigation
targets, not confirmed causes. Follow-up must compare throttle press, hold and
release during countdown and across GO, including simultaneous gas and brake,
at both offline physics rates. Compare engine RPM, displayed RPM, wheel speed,
gear/clutch state, pedal input and launch state against the original.

Acceptance requires stable countdown rev behavior and matching release behavior
with the same car, hardware, tuning and transmission. Numeric launch-strength
agreement alone cannot close this defect. No extra original-game recording is
required to publish this handoff.

## Local force-boundary candidate

The next local pass corrects six shared player-driving boundaries:
direction-aware AWD differential feedback, free-rate-based wheel write-back,
separate compressed-wheel flags and loaded count, signed-slip automatic upshift
gating, zero-threshold damper blow-off, and bottom-out recovery of the body and
all stored wheel positions even without loaded tires. No grip constants or
installed handling attributes are retuned. See
[force boundaries](specs/vehicle-force-boundaries.md) and
[provenance](provenance/player-force-boundaries.md).

The independent production replay had 99 differing records among 540 original
wheel-helper fixtures before correction and zero afterward. All 540 free/locked
reads and 270 write-back wheel-output cases match. Limiter wheel outputs are
excluded from this bridge. A separate joined suspension feedback comparison
matches all 72 read cases and the 36 write cases, keeping raw wheel speed separate
from reported road feedback. All 72 spring fixtures match in their declared FPU
modes, with digression disabled. These counts are component boundaries, not
complete vehicle ticks or trajectories.

The 12 original signed-slip/contact upshift cases also match the production
powertrain gate. The earlier 116 torque-loop boundary/sequence rows still match
their declared single-precision original outputs after these changes.

Earlier body integration checks used explicit synthetic states and short
sequences. Their pose residuals led to the joined follow-up below. The shared-phase
follow-up also clears frozen-body force/torque accumulators at body begin. Body
inertia and damping are not retuned to conceal residuals.

A complete matched tick needs the original pre-tick engine/clutch state,
all tire history, suspension constants and compression, pending body force and
torque, contact planes/surfaces, and exact scheduler order. Earlier recordings
did not supply every field. The expanded capture below fills specific gaps;
the boundary replayer still reports missing inputs rather than reconstructing
them and labeling them exact. The latest local candidate has received a positive
user playtest; the specific numerical comparisons remain separate.

## Expanded freeway capture and replay

A later passive Golf GTI run retains 22,194 updates and 5,551 detailed checkpoint
sets with exact engine-entry wheel state, raw engine/clutch history, suspension
layouts, steering history arrays, supporting planes and body pending forces.
Capture errors and dropped events are zero. Dense bursts contain eight of every
32 updates; they are not an uninterrupted full-route initial-state recording.

This replay identified additional corrections, specified in
[live-drive replay](specs/player-live-drive-replay.md) with
[PC provenance](provenance/player-live-drive-replay.md):

- Consume cached shift potential before advancing shift/clutch timers; refresh
  it after the engine callback. Preserve original neutral exit and sport timing.
- Normalize transmission speed in the original operation order so a strict
  redline crossing cuts torque instead of slipping below the threshold.
- Evaluate the Reference rear-tire pilot factor in the original SI domain.
- Preserve original torque-vector and wheel torque-arm arithmetic grouping.

In 5,125 NOS-inactive engine callback diagnostics, gear, shift timer, raw wheel
outputs, induction spool/boost and braking flags match exactly. Maximum torque
error falls from 11,012.8721 Nm to 0.06201172 Nm; engine/transmission speeds retain
up to two ULP of error. Raw input-interface bytes were not captured, although
same-frame steering gas agrees with effective original throttle in every full
engine record. Complete NOS input-phase acceptance is excluded from this mode.

All 5,551 body boundaries match position, linear velocity, pending force and
torque. The ordered transform also makes angular velocity exact in every case.
The joined follow-up below also corrects the captured quaternion/cache residuals.

Across the three Golf recordings, 60,128 loaded-tire calls retain exact slip
angles, slip values, brake-lock decisions and sign history. The new freeway
run's maximum longitudinal/lateral force errors fall from 1.265625/1.0625 N to
0.0078125/0.015625 N. The maximum across all three runs is 0.015625/0.03125 N.
No acceptance tolerance or altered grip constant is introduced.

The suspension assembly comparison uses recorded tire forces as explicit inputs;
it does not validate the tire producer a second time. Its maximum load residual
is 0.02832 N. Drift phase/value match 4,857 contiguous recorded pairs; drift
torque must be included before judging total-body yaw torque. Complete collision
primitive/world-query closure, audio warm state, full route trajectories and
countdown acceptance remain separate. These corrections remain local for testing.

## Joined-stage follow-up

The private harness now joins production body integration, engine torque and
wheel write-back, differential distribution, suspension and tire forces.
Calculated wheel speed and torque survive the later tire-setup boundary.
Recorded tire setup, steering angles and supporting planes remain explicit
external inputs; the harness does not claim to run a complete world tick.
It checks 4,710 all-loaded, NOS-idle frames and rejects unknown NOS inputs,
free-tire frames and unobserved body changes instead of resetting over them.

This exposed two more arithmetic corrections: ordered wheel geometry transforms
and scalar quaternion integration/normalization. After correction, all 5,551
independent body boundaries match every recorded output exactly, including
orientation, rotation cache and inverse inertia. In the joined engine-to-wheel
diagnostic, maximum total wheel-force component error is 1.33887 N; small load,
torque and tire-force residuals remain. This is a measured residual, not a
selected acceptance tolerance or proof of route parity.

NOS now uses its original subsystem-specific speed conversion. All 1,045
recorded positive-recharge boundaries match the production callback exactly
with explicit recharge permission. Across 5,551 state rows, four explicit
button/permission hypotheses include an exact match in every row; 4,345 are
ambiguous. This does not reconstruct the original buttons. The clock audit
matches all 22,194 recorded delta products, including quarter-speed samples.

The countdown audit matches launch strength in all 744 staged samples and
confirms the retained launch timer order across GO. Exact countdown engine
warm state is still absent. World closure also remains incomplete: 116 dense
frames change pose before wheel forces, and 86 of 4,857 contiguous handoffs add
pending force/torque afterward. Collision inputs and complete tick exits were
not captured. These gaps require targeted evidence, not a manual car/part matrix.

## Collision and terrain follow-up

The next local pass separates immovable-plane arithmetic from object-contact
arithmetic and preserves the plane's scalar reciprocal-multiply normalization.
Before correction, 27 of 54 original single-precision synthetic plane results
differed, including eight reaction decisions. After correction all 54 match
reaction acceptance and final linear/angular velocities. Alternate original
precision retains documented differences; the live FPU word remains unobserved.

A targeted Carrera GT run, user-declared maximum upgrades plus Junkman,
provides 95 complete plane calls and 7,872 complete terrain-cache queries.
All 95 plane acceptance/velocity results match production; these particular
live cases also matched the former solver and serve as regression controls.
The new synthetic controls establish the corrected near-tangent behavior.

Recorded candidate/cache replay now matches every compared field in all 7,872
queries after correcting signed plane separation to Z/Y before X. Selected
face, surface, normal and validity already matched; the correction removes
1,806 small separation residuals. Candidate acquisition, streaming and scenery
group ownership remain external to this boundary.

The recording retains 1,967 complete phase sets; its byte limit interrupted the
final outer frame, which is excluded from complete-tick claims. Actual tuning
included aerodynamics +1, with other values at zero or float round-trip noise.
Comparisons use captured state, not an assumed neutral setup.

The initial observer crashed at a new instruction-level pedal probe. An owned
32-bit reproducer confirms register corruption at that epilogue pattern. The
revised observer removed all such probes and completed without errors/drops,
but the user reported severe FPS loss. The data supports independent collision
boundaries and stage order, not normal-FPS timing or route acceptance. Recorder
overhead and failed capture evidence remain private; no original files change.

## Shared collision and driving response candidate

The local implementation now integrates player, guest, AI and trailer bodies
before the shared car-pair pass. All body world/ground contacts and drag finish
before engine and suspension response. Standalone simulation and prediction
compose the same phases. Collision-altered speeds now reach that tick's engine
and tire work, while pending forces still integrate on the following tick.
See [the phase contract](specs/shared-vehicle-phases.md).

The retained recording confirms the phase sequence and equal deltas in all
1,967 complete sampled frames. This supports the implementation's order, not a
claim that its whole tick matches numerically. The follow-up below corrects the
prop/articulation barriers and downstream consumers. Pedal-free Reference
suspension wake was subsequently corrected below. Complete chassis timing and
matched-route acceptance remain open. AI decisions
and handling attributes are not retuned.

## Downstream readers and chassis follow-up

The local follow-up uses completed tick state consistently for NOS trails,
effects, gauges and camera/audio observers. It moves property/pursuit observation
and traffic sound frames after current contacts and driving response. Incoming
contact snapshots remain the source for damage severity and collision causality.
Prediction restores body/wheel terrain histories rather than discarding them;
overlapping-road regression controls demonstrate that a cold search can change
the selected support and generated forces. See [the state audit](driving-state-audit.md).

Speedbreaker resolves frame input/depletion before choosing both dilation and
vehicle assists. Passive recharge uses each completed tick's speed and delta.
The existing frame-level activation/depletion policy is preserved; exact OG
exhaustion timing within a long frame remains unverified.

Prop pairs now follow all body integration. Prop world/ground response precedes
drag and sleep. Trailer articulations resolve after all body finishes and before
engine/suspension response. Existing solver tuning, trailer-release checking and
AI decisions are preserved. Unified mixed-body pair traversal and exact release
callback priority remain open.

Reference low-speed creep now preserves original additive force/torque response,
full-stop subtraction rounding and epsilon zero. Ten synthetic controls and all
1,967 complete recorded calls match production body-vector outputs exactly with
explicit cached rotation inputs, including seven active calls and 1,960 no-op
calls. Regularized stabilization is unchanged. See
[creep response](specs/vehicle-creep-response.md). Ordered passive transforms,
alignment dot/normalization and correction accumulation also remove all measured
jump-helper residuals: all 1,967 retained calls now match production force,
torque and timer outputs exactly, including 72 active and 1,895 no-op calls.
Exact helper comparison does not
establish complete chassis timing or route acceptance. Reference intact sleepers now run engine and suspension without a pedal gate;
retired sleepers retain engine response but pause suspension. Sleep preserves
vectors until the next sleeping begin, and Frozen force methods ignore writes.
Original instruction checks establish these local rules; live overturned-car
settling, destroyed-player task admission and tire-heat consumers remain open.
The higher-gear displayed-RPM smoother also retains fast decay while the clutch
is fully open after a shift. This does not resolve first-gear countdown parity.

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

## Captured engine scalar follow-up

A compact original-PC capture now supplies 1,074 complete engine boundaries,
including countdown press/release, two launches, a restart and automatic shifts.
Actual tuning is neutral and every recorded simulation delta is 1/60 second.
Original-instruction replay reproduces the entire saved engine object after
every callback. This validates the captured boundary reconstruction, not a
continuous race or every outside method/event consumer.

Production displayed-RPM smoothing matches all 1,074 recorded outputs exactly
when actual engine speed is supplied as an explicit boundary. The broader
engine comparison found two scalar-order corrections: multiply the gear ratio
before wheel speed in transmission feedback, and preserve boost-gauge target
products and direct target selection. See [scalar-order provenance](provenance/engine-scalar-order.md).
After correction, every modeled common engine-state output matches exactly in
all 1,074 independently seeded callbacks: engine/transmission/display RPM,
torque, throttle, gear, clutch, induction, NOS state, shift timers, thresholds
and potential. Stored angular acceleration, cached maximum horsepower, damage
flags and original shift-status storage are not represented by that comparison.
This is a boundary replay, not an uninterrupted whole-car simulation.
These are shared arithmetic changes, with no handling-attribute retuning.
Complete world/tire sequencing and continuous countdown/race acceptance remain
open; an exact smoother alone does not close the reported countdown issue.

## Continuous comparison follow-up

The engine diagnostic now carries its own modeled state through 1,073 captured
callbacks, with two initializations around the recorded restart gap. Every
modeled before/after engine output matches. Wheel, body, contact and launch
inputs remain external; this is an engine sequence result, not a whole-car pass.

A separate short vehicle diagnostic seeds mechanical state once and evaluates
carried wheel-face caches at simulated query points. Processed controls,
steering angles and captured terrain candidates remain explicit external inputs.
The observed zero mechanical body-response port requires an exactly matching
pre-world body checkpoint. The comparison stops when that guard differs; it
does not reset state or invent a numerical tolerance.

This exposes shared arithmetic corrections for non-unit gear efficiency,
speed-dependent grip/traction tables, progressive springs, compression speed,
lateral-force families, reference tire feedback and traction scaling, terrain
query tolerance and aerodynamic product order. See the
[force arithmetic contract](specs/vehicle-scalar-order.md) and
[provenance](provenance/vehicle-scalar-order.md). Handling attributes and AI
decisions are not retuned. The carried diagnostic now completes eight responses
with all mapped checkpoints exact, including all seven body-begin and
body-finish checkpoints, all 32 query tolerances and tire input/history fields,
and all 96 per-wheel world-force components. The separately instrumented tire
comparison uses a diagnostic build. The external ports and unmapped state mean
this is not a complete vehicle or world parity pass.

Longer routes, complete world contacts, raw-input shaping, live floating-point
precision and runtime tire descriptor selection remain outside its verified
boundary. A later M3 GTR steering-only diagnostic closely matches the recorded
steering angle, but supplies recorded body motion and rear slip as inputs. It
does not compare the car's resulting path.

## Retained M3 response follow-up

The 2026-10-28 follow-up uses 717 complete response boundaries from the separately
audited prefix of the interrupted M3 recording. The full recording remains
invalid. Each response supplies recorded body, tire and world state; only
steering history carries between responses. This is not a trajectory test.

Two arithmetic corrections preserve the original stored steering-history sum
and Z/Y-before-X wheel-load projection. No handling attributes are retuned.
Original-instruction synthetic probes check the isolated arithmetic contracts.
In the retained response comparison, exact steering angles increase from
385 to 500 of 717, and the maximum angle residual falls from 0.000286103 to
0.0000114441 degrees. Exact wheel loads increase from 2,411 to 2,579 of 2,868.
All compared engine outputs, terrain query positions and selected planes still
match. Query tolerances and some load, tire-force and body-response fields
retain differences. Maximum body force/torque component residuals remain
1.23828125 N and 1.10888672 Nm. These are measurements, not acceptance limits.

Timestamp expiry, runtime input/descriptor selection, live floating-point
precision and between-tick world contacts remain open.
