# Vehicle wall surface friction

Vehicle wall/barrier response uses the installed rigid body WALL_FRICTION pair multiplied
by the contacted simsurface WORLD_FRICTION. Static friction is the first body coefficient;
kinetic friction is the lesser of both body coefficients. Multiply both by the surface
value after choosing the kinetic minimum. Preserve authored elasticity, inertia, contact
normal/point, damage and audio data. Surface multiplication is independent of tire grip
and body-ground friction.

The world-ground adapter supplies surface material metadata by opaque tag; generic ground
providers default the world multiplier to one. A prepared ground wrapper forwards it.
Resolve each actual world hit using its retained surface identity, including zero and
unknown tags through the existing installed unknown-surface fallback. Legacy synthetic
casts without world metadata keep multiplier one. This adds no arbitrary global friction
or special cop damping.

Tests must exercise an oblique real barrier query, compare the resulting body velocity
and spin to an independently constructed plane reaction with the authored coefficients,
and require both static and kinetic branches. Installed data checks include cop body
settings and all encountered barrier surface tags. A local analytic barrier check does
not establish complete world-collider geometry or a live highway wreck trajectory.

The origin-height reaction COG adjustment applies to every vehicle. The extra head-on
yaw-inertia damping and zero-restitution adjustment apply only to the player's vehicle.
NPC/cop/racer wall hits retain their authored elasticity and unmodified yaw inertia.
A generic response accepts an explicit damping-admission Boolean from the game role;
vehicle archetype, destruction and AI driving goals must not substitute for that role.
Test both roles against the same oblique material and assert the NPC retains its authored
parameters while the player keeps the existing response.
