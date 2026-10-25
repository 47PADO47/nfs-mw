# Shared road prediction for ordinary reinforcements

Sources: the original PC `AICopManager::SpawnPursuitIVehicle` and
`AIVehicle::GetSeekAheadPosition`, inspected locally; the CC0 reconstruction
[AICopManager.cpp](https://github.com/dbalatoni13/nfsmw/blob/main/src/Speed/Indep/Src/AI/Activities/AICopManager.cpp).
Read for behavior only. No original code or capture data is copied.

## Contract

**[native verified]** Ordinary reinforcement placement obtains the seek point
from its target vehicle. Road pursuit reads that same target-owned prediction.
The prediction continues the persistent current road and is cached, rather than
locating a fresh road independently for each spawn. Its distance is
`min(500, speed * horizon + 8)` metres. The horizon is 7.8 seconds normally and
3.12 seconds for a jerk pursuit. The existing target tracker owns this rule.

The game adapter must consume the published primary-target snapshot without
advancing its clocks, replacing its history or drawing navigation random values.
An unavailable free-roam snapshot retains the previous fallback adapter. Racing
and secondary target ownership are separate boundaries; do not read hidden live
target state to produce a new free-roam observation during search.

The remaining placement walk, reversal, fallback disc, exclusion checks, spawn
speed, authored poses and support interceptors retain their own contracts.
Using shared prediction does not authorize making all cops face the same way.

## Continuous driving offsets and direction

**[native verified]** Position-and-heading road initialization selects travel
direction from the heading's dot product with the stored tangent. It does not
force the stored direction on a one-way segment. The lane-index traffic spawn
overload has its own one-way rule; traffic admission also rejects wrong-way entry.
Do not apply the traffic spawn rule to a pursuit driver's road lookup.

Non-traffic road cursors retain the signed lateral distance from the centre line
instead of snapping it to a lane index. Traffic initialization explicitly snaps
that distance. The PC initialization and position-to-offset routines distinguish
these cases. An explicit generic continuous-offset mode can preserve existing
lane-index callers while correcting the pursuit adapter.

Use the same requested offset at both endpoint profiles, subject to the existing
trail safety bounds. Retain it across plain segment joins. Reversal negates the
travel-frame offset so the physical side is preserved; explicit lane placement
returns to indexed-lane geometry. Include the offset in deterministic state.
Close-action lane selection remains an explicit snap, not permission to snap all
road initialization. Complete native lane-change blending remains separate work.

The road action begins with a fresh drive cursor when returning from close pursuit,
as well as a fresh speed governor. Rebuild it from the current physical body and
heading; do not carry a tactical slot's cursor into the next road chase. The first
road action of an admitted reinforcement retains its accepted placement cursor.

## Physical target fixture

**[ours, adapter correction]** A physical BMW driven by the AI PID must consume
AI steering input. Use the production cinematic-control input mapping, retaining
the player's racer chassis, installed physics and collisions. Gamepad steering
range and rate shaping belong to human controls and must not be applied again
to this wheel-angle request. Keep all declared route and pursuit gates unchanged.
The previous invalid pilot runs remain invalid; no result is retroactively passed.

## Occluded road steering

**[native verified]** The bend apex construction uses unit vectors from the car
to the apex and from the navigation point to the apex. Their normalized sum is
the projection axis. Project the latter vector times car-to-apex distance onto
that axis, cap it at the corridor width, then add the projected axis to the apex.
Reversing both input vectors preserves the projection length but reverses the
final displacement. It therefore produces a different steering point.

While finding the sweep start, add each passed cookie's positive out-of-bounds
contribution to the existing look distance, capped at the traffic/driver limit.
Do not restart from the minimum at every cookie. Interpolate the current section
using the current cookie's tangent for both longitudinal distances.
