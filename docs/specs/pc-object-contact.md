# PC object contact response

Sources read before implementation:

- The user's PC 1.3 executable, SHA-256
  `80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c`.
  Private read-only Ghidra inspection of the object response and math helpers;
  no decompiled text or executable bytes belong in this repository.
- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0, reference revision
  `80993f35224c61d2f1b135beba76c6f8234c7940`: `Physics/Dynamics/Collision.h`,
  `Physics/Behaviors/RigidBody.cpp`, `RBVehicle.cpp`, and generated attribute layouts.
  Used for names, layouts and comparison, never copied.
- [vehicle-rigid-body.md](vehicle-rigid-body.md), sections 5 and 6. The PC routine
  below replaces the previously unconfirmed textbook object solver.

Evidence: **[decomp]** for the algorithm; **[verified]** for synthetic input/output
observations from executing the original PC machine code in x86 emulation.
This does not establish agreement with a complete live driving scenario.

## Inputs

Each moment supplies orientation, principal inertia, inertia scale, mass, local
centre of gravity, position, linear and angular velocity, elasticity, an immobile
flag, a breaking impulse and a fixed-centre flag. A plane supplies a world contact
point, normal toward the first body, and kinetic/static friction. Object contacts
use 32 iterations; the world-contact entry point is separate.

Inertia is multiplied componentwise by its scale, inverted and rotated into world
space. The contact arm is the contact point minus the world centre of gravity.
The normal is normalized. Restitution belongs to each body separately.

Reduced mass is `a.mass * b.mass / (a.mass + b.mass)` when both are mobile;
when exactly one is immobile it is the mobile body's mass. Both immobile inputs
are treated as mobile by this low-level routine, although the higher-level
penetration handler normally rejects that pair.

## Iteration

Keep temporary velocities, initially the input velocities, and separate output
velocity increments, initially zero. Elasticity affects output increments, never
the temporary velocities used to decide the next impulse.

At each iteration, find relative contact velocity from temporary linear velocity
plus `angular velocity cross contact arm`. Multiply by reduced mass to obtain
relative momentum. If its projection on the normal is nonnegative, finish.

The normal impulse cancels the negative projection. Tangential momentum is the
remaining vector. Within `static friction * normal impulse`, friction cancels it
completely. Otherwise friction opposes it with magnitude
`kinetic friction * normal impulse`. Zero normal impulse reports no friction;
positive normal impulse with zero tangent reports static friction.

Iteration zero records initial normal/sliding velocity and friction state only.
An immobile body with a positive breaking threshold releases when the initial
normal impulse strictly exceeds it. Recompute reduced mass after release. Apply
no velocity increment on this iteration.

For iteration `i > 0`, multiply both impulses by `i / iteration_count`. For each
mobile body, use the appropriately signed impulse to compute:

- Linear increment: impulse divided by that body's effective mass.
- Angular increment: world inverse inertia times `arm cross impulse`.

Temporary velocities receive normal and friction increments. Output increments
receive friction plus `(1 + body.elasticity)` times the normal increment. A
fixed-centre body receives angular increments only. Sum the scaled normal impulse
magnitudes for the reported force. Finally add output increments to the original
velocities. Return no reaction if iteration zero was already separating, including
an exactly stationary pair.

This differs from using a rotational effective-mass denominator, averaging the
elasticities, or applying restitution to the iterative temporary velocities.

## Game data

Pair friction uses geometric means of `OBJ_FRICTION` values, with kinetic capped
at static. Per-body elasticity is the length of the local normal multiplied
componentwise by `OBJ_ELASTICITY.xyz`. `OBJ_MOMENT_SCALE.xyz` scales inertia; a
positive `.w` scales effective mass. Vehicle adjustments, reaction selection,
`NO_CAR_EFFECT`, attachment and immobility are specified in
[vehicle-rigid-body.md](vehicle-rigid-body.md). Effective collision parameters must
not change the stored vehicle mass or suspension centre of gravity.

The installed data gives these representative object-contact values **[verified]**:

| Vehicle | Elasticity xyz | Inertia scale xyz | Mass scale |
|---|---|---|---|
| M3 GTR | 0, 0, 0.05 | 4, 15, 3 | 1 |
| Traffic hatchback | 0, 0, 0.1 | 2, 1, 4 | 0.5 |
| Semi tractor | 0, 0, 0.05 | 2, 3, 2 | 1 |
| Midsize police car | 0.15, 0, 0.15 | 1.2, 1.2, 1 | 0.75 |

Their static/kinetic friction is 0.55/0.35. These are regression observations,
not replacement defaults: runtime parameters always come from the installation.

## Verification and limits

Reference cases cover equal/unequal mass, off-centre impact, static/sliding
friction, unequal elasticities, immobility, attachment release, fixed centre,
rotated inertia, separating and stationary pairs. Compare both bodies' velocities,
impulse, telemetry and release state with tolerances for x87 versus Rust `f32`.
Reference numbers are measured results of synthetic inputs, not extracted assets.

Collision geometry, swept contacts, world/ground response, damage, visuals, sounds,
scene timing, props and scripted set pieces require separate validation.
