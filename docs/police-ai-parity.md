# Police AI implementation coverage

For the accumulated driving follow-up, current failures and next milestones, see
[the police driving handoff](police-ai-driving-handoff.md).

This records the implementation passes dated 2026-10-23 and 2026-10-24. It supplements the
[pursuit system overview](pursuit-system.md). It is not a claim of complete
original-PC gameplay parity. Original recordings, listening comparisons and
matched route traces have not established that result.

## Implemented contracts

| Area | Contract and evidence |
|---|---|
| Scheduling and observation | [Core corrections](specs/police-ai-parity-core.md) and [unit visibility clocks](specs/ai-pursuit-visibility-clocks.md): PC fractional task rates, four-tick pursuit decisions, actual staggered cop-task sight checks, minimum last-seen ages, jerk observation, cooldown visibility gate and resisting-arrest transition. |
| Driving | [Driving integration](specs/ai-pursuit-driving-integration.md) and [reinforcement road interception](specs/ai-pursuit-reinforcement-driving.md): close-action arbitration, delayed target speed, predicted-road path search, forward cursor advancement, walls/separation, tactical steering and separate NOS input/engagement. |
| Primary target road state | [Persistent target tracking](specs/ai-road-target-tracker.md): shared current/future road walkers, bounded connected-road scoring and a seek cache for the published free-roam primary target. Vehicle/primary replacement resets its history; absent observations age clocks without exposing a snapshot. Generic and production-step regressions pass; the stronger recorded-target city stress remains failed. |
| Acceleration handoff | [AI estimates and physical response](specs/ai-pursuit-acceleration-handoff.md): analytical action-planning curves are separate from physical launch measurements and engine output. |
| Sustained pursuit speed | [Road-action NOS and reentry](specs/ai-pursuit-road-nitrous.md): installed/tuned matched boost, separate rest/burn hysteresis and fresh speed history on returning from close pursuit. [Acceptance](specs/ai-pursuit-catchup-acceptance.md) uses production-loaded actual heat fleets and sustained physical scenarios. |
| Moving-car avoidance | [Predicted encounter cuts](provenance/ai-road-nav-predictive-avoidance.md): cut the road corridor near the future encounter with a slower car, instead of beside its current position. The remaining side-choice and formation-exclusion details are separate parity limits. |
| Road geometry | [Trail geometry](specs/ai-road-nav-trail.md): independent endpoint bounds, a centre inside the unchanged safety margin, linear straight-road tapers and signed curvature averaging. Rolling road-based spawns derive their physical pose from that prepared corridor. Synthetic regressions distinguish each correction. |
| Close-action wall query | [Barrier query](provenance/ai-pursuit-wall-query.md): query authored barriers from the body position, excluding road faces. This removes a confirmed adapter mismatch without changing avoidance weights or speed limits. |
| Team tactics | [Formation runtime](provenance/ai-pursuit-formation-runtime.md): weighted assignment, stable attack members, readiness, progressive tightening, motion frame, heat-specific execution and collapse handoff. |
| Context and heat | [Context](specs/ai-pursuit-context.md): ordinary races select race tables during racing play; pursuit challenges and the final pursuit use world tables. Installed rows reach sight, search, bust, support and helicopter execution. |
| Dispatch and support | [Reinforcement lifecycle](specs/ai-pursuit-reinforcement-lifecycle.md), [scripted spawning](specs/ai-pursuit-scripted-spawning.md) and [support rules](specs/ai-pursuit-formations.md): accepted road-cursor handoff, stationary regular activation, lazy fallback draws, deficit-based join timing, authored requests, support expiry and release. |
| Helicopter | [Core corrections](specs/police-ai-parity-core.md): continuous flight integration, interpolated rendering, full target velocity, sheet normals and current-heat sensing without refueling. |
| Breaker and arrest | [Cinematic control](specs/pursuit-cinematic-control.md): temporary AI ownership, input restoration, placement holds and road cross-section arrest staging. |
| Statistics and feedback | [Cost attribution](specs/ai-pursuit-cost-attribution.md): deployments, direct traffic/property impacts and final-tick career totals. Radio uses per-unit retained sight and actual formation offsets; sirens receive actual speech focus. |
| Optional enhancements | [Limit options](specs/pursuit-limit-options.md): persisted F8 opt-in active-cop and heat caps. These controls do not redefine the reference baseline. |

The previous interpretation of task rates as seconds was incorrect. PC native
inspection confirms a fractional update accumulator: pursuit rate 0.25 means
one decision per four updates. Cop rate 0.125 means one per eight updates.
The deterministic host schedules these on physics ticks. The original variable
task scheduler depends on frame grouping; that difference remains explicit.

## Remaining parity limits

The latest v4 playtest still reports substantial problems. This branch is a
partial implementation for review, not completed police AI parity. The following
reported issues remain open; narrow regression passes do not close them:

- Police catch-up, driving and formation behavior remain unsatisfactory in play.
- Spike strips can sit below the road; roadblocks can overlap or fit poorly.
- Cooldown and radio gaps can be followed by unexpected roadblocks.
- Minimap visibility and white police icons can be wrong. Destroyed-cop text
  and the matching escaped-pursuit statistics screen are missing.
- Helicopter deployment was absent in the reported heat-five test. Starting
  heat five produced a burst of cars; backup timing and its display remain unclear.

The sustained-speed follow-up retains a failed adverse-entry stress: a unit
starting 100 m behind at 59.5 m/s can burn nitrous and recover some ground from
an 85 m/s target, but still loses contact within forty seconds. Established
high-speed pursuit and this slower entry are separate scenarios. The diagnostic
does not establish successful engagement, and the wider fast-driving playtest
report remains open until relevant integrated and playable evidence covers it.

The unchanged recorded-target city stress also remains failed. Its original
player model, performance and nitrous inputs were not captured, while the
fixture supplies stock BMW performance for a prescribed faster trajectory.
Original initial-unit observation history does not establish its same-unit
survival requirement. Keep this stress result separate from
[physical BMW city acceptance](specs/ai-pursuit-physical-target-acceptance.md),
whose target controls and capability are known. Its first automatic pilot run
left the declared route tube and was invalid; no police pass follows from it.
The geometric-preview revision also exceeded the route tolerance. Applying the
production AI input mapping to its physical racer chassis establishes a valid
target, but the unchanged police initially still fail catch-up. The follow-up
[road steering and reinforcement prediction](specs/ai-pursuit-reinforcement-prediction.md)
corrects bend-apex vector signs, continuous pursuit road offsets, heading lookup
on one-way roads and consumption of the target's existing seek cache. Integrated
city acceptance remains separate from these rule corrections. The corrected
physical BMW fixture is valid and the initial cop closes the declared gap, but
still fails late moving engagement. Passing CPU and focused road tests does not
close that sustained city failure.

The first playable package still exhibited fast heat escalation and poor ground
driving in user playtests. The follow-up corrects sandbox heat growth and its
ordinary cap, raw-target catch-up measurements, braking before reversing a path,
and road-edge affinity from the near-car inset trail. Regression coverage now
includes physical lane changes and a multi-unit installed curved-road encounter.
These corrections do not close the user's wider driving report or establish
matched original-game route behavior.

A subsequent private PC capture covers user-driven heats 1-5. It distinguishes
ordinary reinforcements from opposing heavy support and stationary roadblocks.
It motivated corrections to far-road formation-slot routing, stale cursor
advancement, spawn-cursor reconstruction and AI performance estimation. New
regressions start with an empty fleet and exercise automatic placement through
physical driving. Original capture data remains private. Verification of these
contracts is separate from a matched full original/remake pursuit acceptance.
The final real-road scenario also exposed an invalid civilian-spawn gate in the
existing cop's search transition. Changing driving actions now accepts a valid
junction navigator without applying fresh vehicle-spawn admission rules.

- Ground driving still needs matched original/remake route acceptance. Shared primary
  free-roam target tracking is implemented, but the recorded-target city stress
  remains failed. Secondary targets and races retain the previous local road adapter;
  authored race membership and complete native junction/offset blending remain
  boundaries. The [road follow-up](specs/ai-pursuit-road-followup.md) adds continuous
  selectable target offsets and corrects head-on SUV action entry, road fallback,
  support-pair spacing/search and spawn-to-driving junction filters. These rule
  corrections do not close the sustained city engagement failure.
  The [obstacle-selection follow-up](specs/ai-pursuit-obstacle-selection.md)
  prevents distant world bodies filling the 32-body local avoidance list and
  corrects roadblock lookup/traversal through junctions. Pursuit status now reports
  support delays, blockers and helicopter eligibility. User-reported collisions,
  roadblocks appearing behind the player and missing support remain integrated
  acceptance gaps; these corrections do not establish their complete resolution.
  Primary replacement resets tracker identity, but co-op sight and remembered
  target ownership still use the existing shared pursuit memory. Query masks,
  pitched capsule geometry, support spawn handoffs and cumulative airborne
  behavior remain open.
  Live simplified police tire response is still distinct from the current racer
  tire solver. The flat-road diagnostic checks available acceleration; it does
  not establish tire-model equivalence. Support and converted-roadblock joins
  still need complete shared attachment-count/timer integration.
- The pursuit speed cap still approximates the target's steering direction with
  its current-to-future road direction. The original road action reads the
  target's drive navigator relative to its body. A matching physical player
  navigation source has not been established; the new shared road tracker does
  not claim to close that separate governor-input boundary.
- Pursuit sight now follows each cop's actual staggered task and last-seen age.
  Exact original ray/physics ordering, collision query masks and inactive-to-active
  streaming transitions still need original-PC comparison. Patrols retain their
  existing shared check schedule.
- Helicopter skid collision-abort callbacks, some PC gains and side-normal
  steering need original-PC corroboration and collision integration. Smooth
  simulation and interpolation tests do not establish visual flight parity.
- Roadblock pool-first acquisition, partial occupancy behavior, uncertain ram
  gates and full search-route coverage remain open. Special heat rows are not
  assumed to be ordinary career tiers.
- Breaker autopilot uses the existing race driver with entry-speed fallback;
  full original goal arbitration and exact arrest transform axes remain open.
- Indirect damage causality and original callback coalescing remain open.
  Radio screen visibility, mixing/filtering and matched listening remain open.
- Full offence-to-bust/payment/impound and offence-to-escape/restored-world
  acceptance still require integrated scenarios. Prior user playtest failures
  remain open unless a relevant integrated check establishes their resolution.

Synthetic regressions exercise implementation contracts. Installed regressions
exercise owned data, geometry and runtime adapters. Neither kind of test alone
establishes original-game behavior across all heat levels and contexts.
