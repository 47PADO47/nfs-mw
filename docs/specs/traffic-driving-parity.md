# Traffic driving and event context

This specification separates original traffic rules from the removed custom
`traffic_avoidance` option. It does not establish complete driving parity.

Sources: read-only PC 1.3 executable analysis and the CC0-1.0
[reference reconstruction](https://github.com/dbalatoni13/nfsmw), especially
`AI/Common/AIVehicle.cpp`, `AI/Actions/AIActionTraffic.cpp` and
`AI/Activities/AITrafficManager.cpp`. See [provenance](../provenance/traffic-driving-parity.md).
No translated game code is included.

## Driving updates

**[PC static]** An ordinary traffic driver's steering step refreshes the
occluded position along its existing road trail before steering. It does this
on the physical driving step, independently of the slower traffic decision.
It does not advance the road cursor or choose a junction exit during this refresh.
The stopped condition (zero requested speed and horizontal speed below 1 m/s)
returns before the refresh. Only traffic driver class 1 takes this path;
patrols, pursuit drivers, racers and human control retain their own controllers.

The remake must refresh the steering target and current blockage from the
fixed-tick snapshot between decisions. It must retain the requested speed,
decision schedule and random stream. Its deterministic 60 Hz clock remains
an adapter choice rather than an original variable-frame scheduling claim.

**[PC static and isolated original execution]** Ordinary traffic decisions
read rigid-body speed: the length of the full three-dimensional velocity.
Forward projection is a separate input to the pedal controller. A sideways,
backwards or vertical component must therefore count toward traffic look-ahead,
following distance and the ceiling on the requested speed.

The decision stores its look-ahead distance as the obstacle-search radius.
Between decisions, the search uses the larger of that retained radius and
twice the current rigid-body speed. A steering refresh must not recompute the
retained radius from a new blockage or speed. The reference reconstruction
initializes the radius to 20 m before the first decision. Retained radius is
simulation state and belongs in the deterministic state hash.

## Following distance

**[PC static and isolated original execution]** The table has endpoints 3 m
and 50 m over inputs 0 through 80. Its stored index multiplier is 0.0125,
not 0.05. The existing calculation using input divided by 80 is correct.
At input 15 the original lookup returns 11.8125 m; at 40 it returns 26.5 m.
The input is speed times max(effective mass times 0.0005, 1); tractor
effective mass is doubled for this rule without changing physical mass.

## Event density and patterns

**[PC static]** Free roam requests density 1. A race supplies its traffic
percentage, clamped to 0 through 100. Pursuit scaling and user density scaling
apply to that context density. Density changes the original spawn-rate and
offscreen-time/distance lookups; it must not be implemented by multiplying
the active vehicle budget. Zero density prevents new ambient traffic.

A nonempty race `TrafficPattern` overrides the neighbourhood pattern. Without
an override, zone selection applies; without a matching zone the previous
pattern remains. Changing the selected pattern resets its type timers.
An unresolved override does not silently substitute neighbourhood traffic.
The existing scripted drag-traffic adapter stays separate from ambient traffic.

## Removed custom behavior and limits

The three-second blocked-lane change, timed junction yielding, blocked-car
horn and sixty-second blocked timer were custom rules. Remove their setting
and runtime wiring. Preserve ordinary trail obstacle avoidance and original
junction exit/lane selection. Old saved keys must not reactivate the extension.
The separate autonomous sound-side horn producer remains active; it does not
need a blocked-car event from the driving extension.

## Native lane transitions and observation limits

**[PC static and isolated original execution]** The navigator supports a
distance-based lane-offset transition. A positive distance interpolates the
offset as the navigator advances; a zero distance applies the offset directly.
The examined direct callers all request zero distance. They include spawn
placement and road tracking, so a call alone does not establish overtaking.

**[Modded PC runtime]** Bounded free-roam captures show ordinary traffic
changing its lane index at segment transitions. Road metadata identifies
junction destination selection and, in one case, lane renumbering with no
change to the lateral offset. Preserve these behaviors. A changed lane index
alone must not be treated as a physical move into an adjacent lane.

The stationary-blocker recording contains stationary traffic with accident
state zero, but does not establish the complete stop-and-restart response.
No positive-distance lane-change request was observed. This bounded result
does not prove that original traffic never overtakes. Loaded mods and
unexamined indirect paths remain possible influences.

Mid-road overtaking triggers still require matched original-game observation.
Native instance reuse, full vehicle-budget accounting, task timing, powertrain
behavior and full route comparisons remain open. The remake's minimum cruise
request and pedal thresholds must be assessed together before removing its
restart workaround.

The [traffic queue correction](traffic-queue-parity.md) establishes native
blocker eligibility through slow closing motion. It keeps restart behavior and
physics-mode selection as separate parity questions.
