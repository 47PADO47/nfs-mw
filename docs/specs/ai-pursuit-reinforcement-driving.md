# Reinforcement road chase and steering continuity

Sources: original PC executable inspected locally; the CC0 reference
[AIActionRace](https://github.com/dbalatoni13/nfsmw/blob/main/src/Speed/Indep/Src/AI/Actions/AIActionRace.cpp),
and the project's existing driving integration specification. These sources
provide behavioral facts; no original code is copied.

## Road navigation inputs

**[decomp]** The ordinary road chase searches toward the target vehicle's
predicted road position. A formation slot is a separate tactical input. It must
not replace that road interception horizon when the cop is still using the
road chase. Close pursuit and an eligible explicit ram action retain their
own steering rules.

The accepted spawn navigator supplies the cop's initial road segment, travel
direction, lane and parameter. Pursuit entry must preserve this navigator;
reconstructing a cursor from only a world-space point can change its lane or
direction. This handoff is independent of the cursor advancement rule below.

## Drive cursor advancement

**[verified native]** The road chase keeps a steering cursor on the road ahead
of the physical car. Let `N` be the cursor displacement from the car in the
horizontal plane, `F` the horizontal components of the normalized road forward
vector, and `L` the current look-ahead radius.

Find the forward intersection between the tangent ray `N + d F` and the circle
of radius `L` around the car. Its quadratic coefficients are:

- `a = F dot F`
- `b = 2 * N dot F`
- `c = N dot N - L * L`

When the discriminant is nonnegative and `2a > 0.00001`, use the larger root
`d = (-b + sqrt(b*b - 4*a*c)) / (2*a)`. Advance the road cursor only when
`d > 0`. Keep the existing direction hint, road graph and trail recording.
No intersection or a degenerate forward vector means no advance.

This differs from subtracting the cursor's distance from `L`: a cursor behind
the car may require positive forward travel even when its current distance is
equal to or greater than `L`. The road cursor must not become a permanent
reverse steering target merely because the car passed it.

Original forward vectors can be spline derivatives, so normalize the full
three-dimensional vector before taking horizontal components. The road walker
then interprets the positive distance using its own lane geometry.

## Losing tracked sight

**[verified native]** For an attached pursuit, the traffic action is eligible
as soon as the pursuit reports the target out of sight. This eligibility test
does not require a fresh civilian spawn position. Starting the action changes
the existing drive navigator to traffic navigation and traffic lanes, resets
it at the physical car, cancels the chase path, and selects the nearest
selectable lane. The physical vehicle stays in place. The handoff does not
randomize its lane or call the civilian spawn-permission test.

A junction connector can support this existing car's search action even when
fresh traffic spawning there is forbidden. The adapter must not retain chase
steering merely because that connector fails the civilian spawn rules. A valid
search navigator remains required; this change does not admit new cars.

The current adapter still restricts search initialization to traffic-eligible
segments. Native action start preserves its existing navigator's segment
filters instead. A cop on a cop-only shortcut with no nearby traffic road can
therefore still fail search initialization. Resolving that boundary requires
separating the adapter's admission-only segment filters from its driving
filters; it is not covered by removing the civilian spawn gate.

## Acceptance and limits

### Shared primary target road

**[verified native]** A target vehicle keeps a current and future road walker.
Road interception reads that persistent state. It does not choose a new road
from a heading-weighted nearest-point query for each pursuing cop. Incremental
updates retain road identity at junctions; periodic physical-position validation
can rebuild a walker after the vehicle leaves its road.

The free-roam primary target adapter updates one tracker before any cop thinks.
It supplies a copied current-road gap, road direction and predicted seek point
to every cop assigned to that target. The seek prediction shares one cache;
the number or think order of pursuing cops must not advance it repeatedly.
The normal and jerk prediction horizons remain 7.8 and 3.12 seconds.

Outside a pursuit, the driven primary body establishes this vehicle's road
history. During a pursuit the adapter tracks only the published target body,
including its remembered stationary body after tracked sight expires. It must
not fall back to the hidden physical position when that pursuit target is
absent. An absent target publishes no road snapshot. A change of
primary player or replacement of that player's vehicle resets the tracker.
Tracker state, cache clocks, identity and copied cop observations participate
in the deterministic state hash. Presentation identities and memory addresses
must not determine the tracker lifetime.

Secondary co-op targets retain the existing local road adapter. Racing also
retains that adapter until the tracker can consume authored race-route
membership. These are explicit integration limits. Failure to track the
primary road uses the published target body; it must not silently restore the
heading-weighted road lookup that this correction replaces.

Synthetic regressions cover a same-direction reinforcement ahead of the target
with a formation slot behind it, and a road cursor left behind a moving car.
They require a forward road request rather than an invented turning delay.
An ahead cop can still slow to let the target approach under the existing
catch-up speed governor. The tests do not prohibit genuine opposing encounters.

A separate action-transition regression requires an existing cop to enter
search on a valid junction connector that rejects fresh civilian spawning,
while retaining its vehicle identity, position, velocity and forward vector.
The installed route test checks actual lost sight, search and reacquisition.

The correction does not establish full road-graph, collision-mask, ram,
formation-selection or helicopter parity. Installed road encounters and a
matched runtime comparison remain separate acceptance work.
