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
