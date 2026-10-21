# NPC chassis state affecting contacts

Sources and limits are recorded in
[the provenance note](../provenance/npc-chassis-contact-state.md).
This covers a bounded portion of the original `SuspensionSimple` behavior.

## Centre of gravity

Before computing suspension forces, select an extra ride height from the previous
wheel support count: no wheels gives -10 inches, one wheel gives zero, and two or
more wheels gives +2 inches. Supply this extra height to the shared chassis
centre-of-gravity calculation, using installed ride height, roll centre, wheelbase
and front weight bias. No grounded wheels centres the longitudinal bias.

This is a centre-of-gravity offset, not an extra displacement of the car or its
wheel probes. It changes force and collision lever arms without changing stored
mass or inertia. The player's chassis retains its own ride tuning behavior.

## Steering range

The simple chassis starts with a 45-degree maximum steering angle. Each steering
update computes `blend = max(brake, handbrake, 1 - gas)` from bounded controls,
then linearly interpolates the maximum angle from 45 to 60 degrees. Apply steering
input and the installed tire steering scale before the existing Ackermann split.
The AI driver's suspension-range query returns the current maximum before the
tire scale, rather than a permanently fixed 45 degrees.

## First landing

On a transition from no grounded wheels to at least one grounded wheel, halve
local yaw velocity once. Preserve local pitch and roll velocity. Repeated grounded
steps do not repeatedly halve yaw. The player's chassis retains its separate
landing damping behavior.

## Integration and limits

The game selects these coefficients when configuring an AI car. The generic
vehicle library receives optional caller-supplied coefficients; absence retains
its existing chassis behavior. Police and patrol retain this selection
independently of their current pursuit goal. Ordinary traffic and trailers use
their separate [installed rolling mechanics](traffic-suspension-runtime.md).
The host's remaining AI-racer chassis selection is a separate parity boundary.

This does not implement the entire original simple suspension: tire slip and
traction rules, per-wheel integration, catch-up aerodynamics, sleep behavior,
wall steering differences and exact task scheduling remain separate gaps.
Recorded contact moments verify collision inputs/outputs; they do not by
themselves record all wheel states or establish a whole-vehicle trajectory match.
