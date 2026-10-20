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

Preserve single-precision construction order before iteration. A rotated vector
sums its z and y axis contributions before x. The inverse world tensor retains
the two original products: scale each orientation axis by its inverse principal
moment, then multiply by the transposed orientation. For an output component x,
sum contributions y, z, x; for output components y/z, sum x, z, y. Scaling a
different factor first or using one uniform sum order changes low bits. These
differences can change a near-tangent reaction branch and amplify in repeated
angular response. Synthetic original-helper measurements and captured-input
intermediate substitutions verify both constructions; no material retuning or
comparison-tolerance changes are involved.

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
The PC iterative projection sums the x and z products before the y product.
Retain this single-precision order separately from initial closing telemetry;
near-tangential momentum can cancel differently when a generic vector dot
operation sums x and y first. Original-machine-code execution on 2026-10-20
confirms synthetic near-tangent cancellation controls return no reaction.
The exact x87 instruction stack establishes this order; a prior interpretation
of the decompiler's reassociated expression as y/z before x was incorrect.

The normal impulse cancels the negative projection. Tangential momentum is the
remaining vector. Within `static friction * normal impulse`, friction cancels it
completely. Otherwise friction opposes it with magnitude
`kinetic friction * normal impulse`. Zero normal impulse reports no friction;
positive normal impulse with zero tangent reports static friction.

Iteration zero records initial normal/sliding velocity and friction state only.
An immobile body with a positive breaking threshold releases when the initial
normal impulse strictly exceeds it. Recompute reduced mass after release. Apply
no velocity increment on this iteration.

For iteration `i > 0`, form the full normal impulse vector before multiplying
both impulse vectors by `i / iteration_count`. Multiplying the normal by an
already scaled scalar changes rounding in repeated shallow contacts. For each
mobile body, use the appropriately signed impulse to compute:

- Linear increment: impulse divided by that body's effective mass.
- Angular increment: world inverse inertia times `arm cross impulse`, retaining
  the ordered vector transform above.

Temporary velocities receive normal and friction increments. Output increments
receive friction plus `(1 + body.elasticity)` times the normal increment. In
single precision, retain the original operation order: convert normal and
friction impulses to separate velocity increments before adding them, then add
`elasticity * normal_increment` to their sum for the output increment. Combining
impulses before conversion or distributing `(1 + elasticity)` changes rounding
in shallow, sliding contacts. This ordering was checked against captured original
moments and the PC increment helper on 2026-10-20. A
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

## Collision notifications

**[decomp]** The PC object-response dispatcher reports a scalar impact to each
body after a successful reaction. For a mobile moment this is normal impulse
divided by its own effective contact mass. For an immobile moment it instead
uses the other moment's effective mass. This notification can therefore be
positive while that body's velocities are unchanged, including a vehicle's
temporary `NO_CAR_EFFECT` override. Preserve both the notification and the
physical response: collision callbacks, damage filtering and audio retain it;
an immobile body still receives no physical velocity or steering correction.
The callback arm uses the original narrow-phase point relative to the separated
body origin. Its closing magnitude is the solver's initial normal component,
measured with the modified moments at the corrected solver point. The sliding
component remains separate and does not contribute to the damage eligibility
gate. Keep incoming linear velocity separately for inherited damage events.

Read-only PC dispatcher inspection on 2026-10-20 confirms both reciprocal-mass
branches and the helper's positive-mass guard. This is static executable evidence,
not a matched live camera or sound trace. A synthetic regression distinguishes
the stationary body's positive notification from its zero physical response,
including contact mass overrides and both body orders.

Collision geometry, swept contacts, world/ground response, damage, visuals, sounds,
scene timing, props and scripted set pieces require separate validation.
