# Placed prop attributes

Sources: [prop spawner format](../formats/prop-spawners.md), the user's PC 1.3
executable (same hash), and [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)
(CC0, read-only reference), `SmokeableInfo.hpp`, `Scenery.hpp`, and attribute class
headers. No decompiled code is included.

Resolve a spawner's collision key directly in the track bounds catalog and its
attribute key directly in the `smackable` class, including inherited fields.
There is no 150 kg cutoff for movable scenery. Names do not determine behavior.
The linked `rigidbodyspecs` collection supplies gravity, drag, sleep threshold,
ground and object friction, elasticity and contact inertia scales just as it does
for cars. `smackable` provides:

- `MASS`, in kg.
- `MOMENT`, a three-float vector. The constructor's inertia is twice the ordinary
  box inertia multiplied per axis by a positive MOMENT component (otherwise 1).
- `DETACH_FORCE`, used as a breakaway impulse threshold for attached objects.
- `NO_CAR_EFFECT`, which controls the vehicle-specific collision override.
- `RESPAWN_TIME`, `KILL_OFF_SCREEN`, `DROPOUT` and `ALLOW_OFF_WORLD`, which must be
  retained for the lifecycle rather than replaced with a fixed six-second timer.

Measured examples: cones have mass 100 kg; traffic lights 200 kg and detach
threshold 7500; trees 100 kg and threshold 20000; logs 1000 kg and
`NO_CAR_EFFECT=false`. These values are loaded from the install, never selected
by a scenery name or hardcoded in the runtime. The default linked prop body has
gravity -13, drag (2,2,2), angular drag (1,1,1), object friction (.35,.23), and
zero object restitution. Road speed/stop signs in A41 use `mediummetalpost`.

This spec currently establishes data interpretation and body construction only.
Attachment, no-car-effect response, hierarchy/fragments and lifecycle require
their own measured runtime rules before claiming original behavior.

## Contact and initialization rules

The PC behavior-change callback confirms: a virgin placement with nonzero
`DETACH_FORCE` attaches to the world; negative values attach with a zero break
threshold (permanent). A zero value starts unattached and asleep. Without an
explicit centre marker, the collision centre is the volume-weighted average of
object-colliding primitive offsets. Inertia still comes from the root dimensions.
Bounds object flag `0x04` becomes runtime primitive flag `0x02`; disabled bounds
are excluded. World-only bounds must not be substituted for object primitives.

The vehicle response override makes the vehicle's temporary contact moment
immobile when the other body is a movable smackable with `NO_CAR_EFFECT=true`.
This preserves the car's velocity while transferring motion to the prop. An
attached prop does not enable this override: the car can slow down while breaking
it loose. The next contact sees the detached state. A zero break threshold never
detaches an attached object. These are temporary contact flags, not a frozen car.

Separate objects before applying the 32-iteration contact response. If exactly
one is immobile, move only the other. If both are mobile, use the stored mass
ratio for car/prop contacts. Both immobile means no separation or response.
The original also shifts the contact point with whichever primitive penetrates
the other. The original geometry classification and full contact order remain
separate parity work.

## Runtime integration boundary

The first dynamic integration uses the existing body's fixed-step gravity/drag,
ground-plane response, box overlap and render interpolation. Detached objects
remain rendered and collidable. Geometry sweeps, original ground-point clouds,
fragments, event sequencers, offscreen destruction and respawn are not established
by this stage. In particular, do not replace those missing lifecycle rules with
a six-second disappear/reappear timer.
