# Sustained pursuit speed acceptance

This is an engineering acceptance contract for the independent remake. It does
not define an original-game acceleration curve or promise that every cop catches
every player. Original behavior sources remain the existing driving and vehicle
performance specifications.

## Test the actual fleet and production model handoff

Load the installed heat rows without replacing their requested vehicle types.
Load those vehicles through the production model worker, including inherited
geometry, installed physics, AI attributes and analytical action performance.
Record both the requested vehicle and any geometry donor. A geometry alias must
not silently substitute that donor's behavior or performance for the requested
vehicle's installed records.

## Separate pursuit conditions

Exercise an established, rolling police unit behind the target separately from
an empty fleet populated by automatic reinforcement placement. A new stationary
unit ahead of the player may slow to allow an encounter. That behavior does not
establish the ability to catch a player who has passed it.

Sustained straight-road acceptance uses installed production vehicle physics and
a synthetic supported corridor. The target follows a constant-speed kinematic
route without physical player-car collisions, so collisions and manual steering
do not hide the action-to-body speed response. Initial conditions and requested
target speed are part of each result. They are engineering scenarios, not values
fitted to mixed original-game capture samples.

Keep established high-speed pursuit separate from adverse entry. A trailing unit
already near the target's speed must close and retain moving engagement. A unit
starting substantially slower can first lose ground, then begin closing. Native
player-performance matching and visibility timeouts do not guarantee it catches
a constant-speed target within an arbitrary forty-second deadline. Retain that
stress result explicitly, including lost contact. Check physical nitrous burn,
speed above the target and actual recovery from the largest deficit separately;
those facts must not be reported as successful sustained engagement.

Record unit identity, model, signed separation, actual and requested speed,
steering-target direction, controls, NOS capability/input/burning, gear, RPM,
wheel support, search/flee state and visibility age. Retain samples after the
player passes a reinforcement. Check that a moving pursuer closes a supported
initial deficit and remains within engagement range over a sustained interval.
Report loss of the original unit, replacement pressure and cooldown transitions.

## Acceptance boundary

A straight corridor isolates speed selection, controls and vehicle response.
Actual installed roads remain necessary to check curvature, obstacle avoidance,
streaming and route decisions at freeway speeds. A short low-speed curved-road
test or a manually placed fixed-model cop is insufficient to establish actual
high-heat fleet competitiveness.

An optional external target trace can supply timed physics positions, velocities
and headings to an installed-city test. Validate finite, increasing samples and
road/collision compatibility before scoring. Interpolate the target without
snapping its path to the remake's road. Record the input identity privately.
Retain one rolling pursuer's identity throughout the run, so fresh deployments
cannot hide its loss. This tests the remake following a supplied route; it is
not a matched original-versus-remake experiment unless both vehicles, settings
and initial states are independently established.

Passing these cases does not establish tire-model equivalence, all-speed catch-up,
complete formations or full original-game pursuit parity. Preserve user-reported
failures until a relevant sustained scenario and playable verification cover them.
