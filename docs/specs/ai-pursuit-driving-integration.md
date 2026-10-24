# Pursuit driving integration

This supplements `ai-pursuit-tactics.md` and `ai-pursuit-goals.md`.
Evidence: the source reconstruction of the original game, independently expressed
as behavior below. PC differences remain possible unless separately identified.
See `../provenance/ai-pursuit-driving.md`.

## Speed reaction history

**[decomp]** The delay history contains 16 samples at a fixed 0.125-second
resolution, independent of the cop action's update cadence. A new input linearly
interpolates between the previous input and the new one at every sample boundary
crossed by the elapsed time. It retains the unconsumed fractional sample time.

A query inside that fractional interval interpolates between its newest stored
sample and the latest input. Older queries interpolate adjacent stored samples.
Delays beyond the retained window use the oldest stored sample. Starting the
action fills the entire history with the target's current speed.

The action scheduler's `0.125` rate is a separate quantity: it selects one update
per eight simulation ticks. Do not equate that rate with the history resolution.

## Drivability

**[decomp]** A ray longer than 200 metres is blocked. Raise each ray endpoint by
0.5 metres. Ignore a hit when its squared distance from the destination is at most
0.5 square metres. The drive-nav ray begins at the cop's position predicted
0.65 seconds along its current velocity; the target ray begins at its position.
The collision query's exact original masks still need PC confirmation.

Ram goals require both rays, with no close-action range or relative-speed gate.
When blocked, retain road navigation and the road action's speed. An eligible ram
goal consumes its computed steering target and speed; nitrous stays off.
An arrived pull-over brakes with the handbrake and steering centred.

## Separation

**[decomp]** For horizontal half-dimensions `a=min(width,length)` and
`b=max(width,length)`, the capsule axis half-length is `b-a*a/b` and its radius is
`sqrt(a*a+(a*a/b)^2)`. Its axis follows the longer body dimension.
The geometric push comes from the closest points of the two capsule axes.
Predictive weighting uses the closest time and squared distance between vehicle
centres, rather than between the future capsule axes. Clamp each vehicle's
contribution to 160 km/h before adding the contributions.

The world avoidable record exposes horizontal heading and dimensions. The adapter
retains vertical position and velocity but cannot reconstruct pitched or rolled
capsule axes. Collision response remains a separate system.

## Road chase speed and turn-around

**[decomp]** Catch-up speed uses the displacement from the raw target to the
cop. Its seek direction runs from the raw target to the last path-search goal.
A formation slot or predicted road horizon must not replace the raw target in
these two measurements.

Above 40 km/h, a pursuit path destination more than 0.3 of its displacement
against the current velocity causes a stop request. Steering then follows the
cop's future road while it slows, before attempting the reverse destination.
The future-road distance is the greater of one second of speed and two metres.
The remake obtains this future road from its existing lane navigator along the
cop's heading. Exact original future-road state
still requires comparison.

## Implementation and acceptance boundary

The close action uses target performance, heat reaction delays, wall avoidance,
vehicle separation, road-edge affinity, curvature limiting and the nitrous delay.
The path action uses the actual road steering vectors, full cop skill and grade.
Goal limiters and commands update with the cop's think, rather than every physics
step. Formation changes reset the goal limiter to the current cop speed.

Road adaptation uses the available lane navigator. Exact persistent drive-to
cursor transitions, lane snapping, target-road centres and trail exclusions still
need comparison with the original. The current adapter must not be described as
verified full route-level parity. Original collision masks, accumulated airborne
timer behavior, spawn speed handoff and generic far-chase nitrous remain open.
