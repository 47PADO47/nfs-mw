# Vehicle rigid body (tick, integration, collision, reset)

How NFS: Most Wanted moves a car as one rigid body: the fixed simulation tick, how force and torque become
motion, damping and sleeping, what the car collides with and how contacts become velocity changes,
car-vs-car handling, and reset-to-road. Wheels, suspension, tires and engine are *not* here: they push
forces into this body through the `apply_*` helpers (§2; see the planned `vehicle-physics.md`).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; mostly
  the GameCube build), under `src/Speed/Indep/Src/`: `Physics/Behaviors/{RigidBody,RBVehicle}.{h,cpp}`,
  `Physics/Behaviors/ResetCar.cpp`, `Physics/Dynamics/{Collision,Inertia}.h`, `Physics/Bounds.h`,
  `Physics/VehicleBehaviors.h`, `Sim/Common/{Simulation,Util}.cpp`, `Misc/{Main.cpp,MWAttribUserTypes.h}`,
  `World/{WCollision,WCollider}.h`, `World/Common/{WCollisionMgr,WCollisionPack,WCollisionAssets,WWorldPos}.cpp`,
  `Libs/Support/Miscellaneous/CARP.h`, `Generated/AttribSys/Classes/{rigidbodyspecs,collisionreactions,pvehicle}.h`.
  Read for understanding; no code copied.
- **Not in the decompilation** (files empty or stripped): `Moment::React` (the impulse solver),
  `Geometry::FindIntersection` (box/sphere overlap), `Bounds::Collection::AddTo`, the code that builds a
  car's `RBComplexParams` (mass, dimension, tensor) and `IVehicle::SetVehicleOnGround`. Where needed this
  spec gives the interface plus a textbook rule, tagged **[unconfirmed]**, to be tuned against the game.
- **Data inputs:** AttribSys `rigidbodyspecs` (linked from `pvehicle`), `pvehicle` (`MASS`, `TENSOR_SCALE`),
  `collisionreactions` (via `aivehicle`), surface classes (`GROUND_FRICTION`, `WORLD_FRICTION`); chunk
  `0x0003B801` (`BCHUNK_W_COLLISION_ASSETS`, "CarpWCollisionPack") for world geometry. Layouts of the
  attribute classes: [attributes.md](../formats/attributes.md).

Evidence tags as in the [docs README](../README.md#evidence-tags). Nothing is **[verified]** yet: no
implementation exists to compare against the game.

## 0. Conventions

- Sim space: **+y up, +z forward, +x right** at identity; `bodyMatrix` rows are right, up, forward
  [decomp]. Metres, kilograms, seconds, radians. (Car-solid space in [car-assembly.md](car-assembly.md) is
  +x forward, +y left, +z up; the axis swap is a render concern.) Road-network code maps a sim position to
  a 2-D map point as `(z, -x)` [decomp].
- Row-vector matrices: `p' = p · M`. `bodyMatrix` has no translation; position is kept separately.
- `state`: 0 awake, 1 sleeping, 2 not modelled (frozen; no integration, no collisions) [decomp].

## 1. Tick rate and update order

- One **fixed step of 1/60 s (0.016666668)** [decomp: `Scheduler::Init`]. The game time scale multiplies
  it: `dT = (1/60)·speed`; nothing runs when the sim is not active or `dT <= epsilon`. There is no variable
  step; slow motion shrinks `dT`. Rigid bodies run at task rate 1: once per tick.
- Per tick, in order [decomp]: simple (non-car) rigid bodies; **rigid-body update (below)**; AI targets;
  world update (triggers, grid, event sequencer, input, surfaces); per-model frame processing; release of
  the rigid-body scratch memory. Vehicle behaviours (engine, suspension, `ResetCar`) are separate tasks in
  the "Physics" schedule; their forces added during a tick are consumed by the next rigid-body update
  [unconfirmed: exact relative order, set per vehicle by `pvehicle.BEHAVIOR_ORDER`].
- **Substepping:** the rigid body does one integration and one collision pass per tick; no inner substep
  loop [decomp]. Any suspension substepping belongs to the vehicle-physics spec.

### Rigid-body update (once per tick, all bodies)

```
for every body:   begin_frame(dT)      # integrate previous tick's forces, clear, add gravity
                  update broad-phase grid
sweep-and-prune broadphase over the grid -> for each overlapping pair: object_overlap(A, B, dT)
for every body:   end_frame(dT)        # world collision, drag, sleep test
resolve articulations (joints; unused for plain cars)
```

So **forces applied during tick N are integrated at the start of tick N+1** (semi-implicit Euler: velocity
first, then position from the *new* velocity), and car-vs-car contacts are resolved before world contacts.

## 2. State and integration

State per body (all [decomp]): `position` (m, of the body origin), `orientation` (unit quaternion),
`linearVel` (m/s), `angularVel` (rad/s, world frame), `mass` (kg), `oom = 1/mass`, `inertiaTensor`
(principal moments, kg·m², body frame, diagonal), `force`, `torque` (world frame, accumulators),
`radius` (bounding radius = `|dimension|`), `bodyMatrix`, and the cached **inverse world inertia**
`invWorldTensor = Rᵀ · diag(1/Ix, 1/Iy, 1/Iz) · R` where `R` is `bodyMatrix` (row-vector form); a moment
`<= epsilon` is left as-is rather than inverted. `mCOG` is the centre of gravity offset in body space
(m), read from `rigidbodyspecs.CG`; `dimension` is the half-extent box (min 0.001 per axis).

### Inertia from dimensions and mass

For a car, the principal moments are those of a solid box with *full* edge lengths `2·half` and the vehicle
mass, then scaled per axis [decomp: `Util_GenerateCarTensor`, `Dynamics::Inertia::Box`]:

```
w = 2*hx; h = 2*hy; l = 2*hz            # full lengths
Ix = m*(h^2 + l^2)/12
Iy = m*(w^2 + l^2)/12
Iz = m*(w^2 + h^2)/12
I  = (Ix,Iy,Iz) * pvehicle.TENSOR_SCALE.xyz           # per-axis tuning
```

`SetMass(newMass)` (when different and positive) rescales the tensor by `newMass / oldMass` and updates
`oom`. The initial `mass`, `dimension` and tensor reach the body as construction parameters; the code that
fills them (it reads `pvehicle.MASS`, `TENSOR_SCALE` and the car's collision bounds) is not in the
decompilation, so *which* bounds supply `dimension` is **[unconfirmed]**; the natural reading is the
half-extents of the car's root collision box.

### begin_frame (integration of last tick's force)

Run for every body whose state is not 2 [decomp]:

```
prepare each collision primitive: remember its world-space centre (needed for sweep velocity later)
if state == awake:
    cg0 = rotate(cog, bodyMatrix)                     # before rotating
    linearVel  += force * dT * oom
    position   += linearVel * dT                       # uses the already-updated velocity
    angularVel += (invWorldTensor applied to torque*dT)
    clamp linearVel  per axis to +-300 m/s
    clamp angularVel per axis to +-30 rad/s
    if |angularVel|^2 != 0:
        dq = quaternion(angularVel*dT, w=0) * orientation
        orientation = normalize(orientation + 0.5*dq)  # first-order quaternion step
        rebuild bodyMatrix from orientation
    recompute invWorldTensor from the new bodyMatrix
    cg1 = rotate(cog, bodyMatrix)
    position += cg0 - cg1                              # rotate about the COG, not the origin
else (sleeping): linearVel = angularVel = 0
force = torque = 0
if awake:
    statusPrev = status
    if not attached-to-world: force.y += GRAVITY * mass
```

`GRAVITY` is the signed value stored in `rigidbodyspecs` (negative for downward, ≈ -9.81 expected;
**[unconfirmed]** sign convention, since it is added directly to `force.y`).


### Applying forces and torques

Anything (engine, suspension, drag, scripted pushes) adds to the accumulators through four helpers
[decomp]; each call also wakes a sleeping body, sets the "moved" flag (`status 0x40`) and detaches the
body from the world if it was pinned:

```
apply_force(F)            : force  += F
apply_torque(T)           : torque += T
apply_force_at(F, p)      : r = p - (position + R*cog); torque += r x F; force += F
apply_torque_at(F, p)     : r likewise;                  torque += r x F   # no force
```

`Accelerate(a, dT)` adds `a*dT` straight to `linearVel` (awake bodies only). `Damp(k)` multiplies velocities
and the pending force/torque by `1 - k`. Point velocity of a world point `p`:
`v = linearVel + angularVel x (p - (position + R*cog))`.

## 3. Damping, drag, sleep

### Aerodynamic-style drag (end of tick)

Applied in `end_frame`, only if the body is not pinned to the world. Two independent terms; each runs only
if its vector in `rigidbodyspecs` has non-zero length (checked once at construction).

```
rho = 1.225                                     # constant in the code
# linear
v_local = bodyMatrix^T * linearVel
A = 4 * (hx^2, hy^2, hz^2)                       # per-axis frontal "area" from the half extents
Fdrag_local = -0.5 * rho * |v_local| * (v_local * A * DRAG.xyz)   # component-wise
apply_force(R * Fdrag_local)                     # at COG, so no torque
# angular
w_local = bodyMatrix^T * angularVel
S.x = 4*hx*sqrt(hy^2+hz^2); S.y = 4*hy*sqrt(hx^2+hz^2); S.z = 4*hz*sqrt(hx^2+hy^2)
Tdrag_local = -0.5 * rho * |w_local| * (w_local * S * DRAG_ANGULAR.xyz)
apply_torque(R * Tdrag_local)
```

The drag lands in the force/torque accumulators, so it is integrated next tick. `DRAG` = `(Cx,Cy,Cz)`
quadratic coefficients per body axis; the 4th component is unused here. `NATURAL_ANGULAR_DAMPING` exists in
the class but is not read by `RigidBody`/`RBVehicle`; if it is used at all it is by the suspension
**[unconfirmed]**.

### Sleep

At the end of each awake body's tick (after world collisions and drag):

```
sum = |linearVel| + |angularVel| * radius
sleep = sum < SLEEP_VELOCITY  and  contact_points > 2
```

where `contact_points` = ground contact points found this tick (§4.2) **plus, for vehicles, the number of
wheels on the ground**. A vehicle additionally refuses to sleep if it is animating, had an object (car) hit
this tick, or has wheels on the ground (unless it has been a destroyed, non-player car resting on at least
half its wheels for `>= 2 s`, `mDeadOnWheels`) [decomp]. A sleeping body is not integrated and has its
velocity zeroed; any `apply_*` call, or a hit by an awake body, wakes it (`state = 0`).
Sleeping bodies still refresh their ground height query each tick. A body created inactive starts asleep.

Clamping, drag and sleep constants (`300`, `30`, `1.225`, `0.5`, `4`) are hard-coded [decomp].

## 4. World collision

### 4.1 What the car collides against

The car is a list of **collision primitives**, each with `{name hash, half dimensions, offset from the body
origin, orientation quaternion, shape (box | sphere), material surface, flags}`. Flags: collide-with-world
(1), with-objects (2), with-ground (4), disabled (8), one-sided (16). Primitives come from the car's
`CollisionGeometry::Bounds` tree (below); if none can be added, a **default** is made from `dimension`:
`DEFAULT_COL_BOX == "SPHERE"` gives one sphere of radius `max(hx,hy,hz)` (flags world|objects), anything else
one box of the body dimension (flags world|objects|ground), the box grown by `COLLISION_BOX_PAD.xyz` [decomp].
Sphere primitives are kept at the head of the list, boxes after.

`Bounds` record (little-endian, 0x30 bytes in the original; all fields [decomp] from the header):
`orientation` 4×int16 / 32767 (quaternion), `position` 3×int16 / 1000 (m), `flags` u16, `half dimensions`
3×int16 / 1000, `numChildren` u8, `pcloud index` i8, `pivot` 3×int16 / 1000, `child index` i16, `radius`
f32, `surface` u32 hash (an AttribSys surface key), `name` u32 hash, a runtime pointer. `flags`: 1
disabled, 2 vs-world, 4 vs-objects, 8 vs-ground, 16 mesh-vs-ground, 32 internal, 64 box, 128 sphere,
256/512 conical/prismatic constraint, 1024/2048 female/male joint, 4096 male post, 8192 joint invert.
A bound with a point cloud (`PCloud`: count + 4-float vertices) becomes a **mesh** of ground-contact
points in the body frame. Primitives can be switched on and off by name hash (used for damage parts).

The **world** the body tests is gathered per body by a *collider* (a cylinder query) refreshed around the
body each tick with radius `max(primitive radius, 3 m)`; it caches three lists [decomp]:

| List | Origin | Used for |
|---|---|---|
| triangle strips of "collision instances" | static scenery/road, per section | ground height and normal under a point (§4.2) |
| barriers | edge segments of instance articles, in world space | walls, rails (§4.3) |
| oriented boxes ("objects") | `co` records, some dynamic | props, large solids (§4.4) |

#### World collision data (chunk 0x0003B801, one per streaming section) [decomp]

A header (section number, resolved flag, last load address, payload size), then a serialized group tree
with arrays `ci` (instances), `co` (objects) and `ca` (articles, indexed by an instance's render index):

- **Instance** (0x40 bytes): inverse-transform rows packed in three 4-vectors (row 0 + width; row 2 +
  length; inverse position + bounding radius), `height` f32, `flags` u16 (bit 0 y-axis not up, bit 1
  dynamic), iteration stamp u16, group u16, article index u16, runtime article pointer.
- **Object** (0x70 bytes): `posRadius` (centre xyz, bounding radius w), `dimensions` (half extents),
  `type` u8 (0 box, 1 cylinder), `shape` u8, `flags` u16 (bit 0 dynamic), render index u16, surface
  `{id u8, flags u8}`, 8 bytes pad, 4x4 matrix.
- **Article**: 0x10 header (`numStrips`, `stripsSize`, `numEdges`, `edgesSize` u16; resolved u8,
  `numSurfaces` u8, `surfacesSize` u16, intermediate index u16, flags i16), `numStrips` strip spheres
  (0x10: centre xyz f32, radius u16, offset-to-strip u16 (+0x10)), strip data, `numEdges` **barriers**
  (0x20: two 4-float points, `w` of the second = 1/xz-length; surface id at +0xC), `numSurfaces` u32
  surface name hashes (swapped for surface collections at load).
- **Strip**: packed vertices of 8 bytes (x, y, z int16 in 1/128 m relative to the instance origin, then
  `{surface id, flags}`); vertex 0's tail holds the vertex count, vertex 1's the strip flags (1 = faces
  up). Triangles are a normal strip; a triangle's surface is its third vertex's.

Barrier points are article-local; they are transformed by the instance matrix when a barrier list is built.
### 4.2 Ground (road surface) contact

Ground contact is a **point-in-height-field style query**, not a triangle-triangle solve.
For a car whose `INSTANCE_COLLISIONS_3D` is false (the normal case):

```
for each enabled ground-flagged BOX primitive:
    for each of its 8 corners c (+-1,+-1,+-1 scaled by half dims, rotated into the world):
        p = position + corner_arm
        tolerance = speedXZ*dT + max(0, (wx*(arm.z-cog.z) - (wz*(arm.x-cog.x) + vy))*dT) + depth
        tolerance = clamp(tolerance, 0.25, ceiling - p.y)       # ceiling = position.y + depth
        query road face under p within `tolerance`  ->  normal n, signed depth w
        if on a valid face and w > epsilon: record contact {lever=arm, normal, penetration=w, surface}
for each enabled MESH (point cloud): same per vertex
at most 16 contacts are kept; `depth` = |(R.col1) scaled per axis by half dims| (vertical extent)
```

`depth` is the length of the box's vertical extent vector; `speedXZ` is horizontal speed. The query
returns the face plane's normal (a face with `n.y >= 0.9999` is clamped to 0.9999; down-facing faces are
flipped) and `w = dot(facePoint - p, n)`, i.e. how far `p` is below the plane (positive = inside the
ground) [decomp: `WWorldPos::Update`]. The body also stores the **ground normal** (`mGroundNormal`) from
the query at the body origin each tick (used for "orient to ground" and by `ResetCar`).

Then, if any contacts were found:

1. `contacts_in_contact = count` (feeds sleep and `IsInGroundContact`);
2. contacts are sorted deepest first;
3. `resolve_ground(contacts)` tries contacts in that order and **stops at the first one that produces a
   reaction** (`React` returns true), writing back that reaction's linear and angular velocity;
4. the body is pushed out along the **deepest** contact's normal by its penetration depth.

Per contact: `plane.point = position + lever`, normal = contact normal, static friction `GROUND_FRICTION[0]·surface.GROUND_FRICTION`, kinetic `min(GROUND_FRICTION[1], GROUND_FRICTION[0])·surface.GROUND_FRICTION`, inertia scale `GROUND_MOMENT_SCALE`, restitution `e = |(n·right, n·up, n·forward) ⊙ GROUND_ELASTICITY.xyz|` (component-wise product, then length: the bounce depends on which side of the car hits). It runs the shared impulse routine (§5) with 16 sub-steps.


Eligibility: the body must be awake, `NO_GROUND_COLLISIONS` false, and have a non-empty instance list.
`RBVehicle` skips ground collision for **non-player** cars when *all* wheels (and more than 2) are on the
ground and the car had no car-hit this tick: AI cars rolling normally are held by the suspension alone.
The first resolved ground contact raises a collision event (type ground) with closing speed, sliding flag
and force (`|impulse|`), for sound, damage and effects.


### 4.3 Walls and barriers

Only for bodies that "moved" this tick (`status & 0x40`), `NO_WORLD_COLLISIONS` false, not asleep and with
at least one primitive. Per-vehicle throttles [decomp]: **player cars always collide**; non-player cars
use the generic rule; **traffic** cars that have not hit anything are only tested every 4th tick when slower
than 2 m/s (`v² < 4`) and every 2nd tick below 15 m/s (`v² < 225`).

```
if the body has > 1 primitive: first test one bounding sphere (radius = max primitive radius, swept by
    linearVel*dT) against the barriers; skip the per-primitive test if nothing is hit
for each enabled world-flagged primitive:
    build its world oriented box/sphere (+ sweep delta = movement since last tick)
    for each barrier near the body (sorted by distance):
        make a thin box from the barrier: width = xz length, height = |y1 - y0|, normal in xz,
            centre = midpoint
        intersect (box-vs-box / sphere-vs-box)
        unless the barrier's surface has flag 0x10 (two-sided) or the primitive is one-sided:
            ignore the hit if dot(barrier_normal, contact_normal) <= 0   # one-way walls
        on hit: push the body out by penetration along the normal, then resolve (below)
```

Push-out happens *before* the velocity reaction, and later primitives see the already-corrected position.

Wall reaction (static barrier, world velocity 0): static friction `WALL_FRICTION[0]·surface.WORLD_FRICTION`, kinetic `min(WALL_FRICTION[1], WALL_FRICTION[0])·surface.WORLD_FRICTION`, restitution `e` as above using `WALL_ELASTICITY`, inertia scale `WORLD_MOMENT_SCALE`; then the vehicle tweaks (below) and the impulse routine with 16 sub-steps. A non-zero force raises a collision event (type world).

#### Vehicle tweaks to wall and world-object reactions (`RBVehicle`, not the ground path) [decomp]

- `cg.y` of the reaction is forced to 0 (the impact is taken at the body's height of the origin, not the
  true COG), for every car.
- **Player head-on damping:** with `nose = -dot(n, forward)` and `nose >= -1` (always true), compute
  `closing = ramp(-dot(n, linearVel), 0, 5)` (0 at <=0 m/s, 1 at >=5 m/s) and `damp = ramp(nose, -1, 1)`.
  Elasticity is set to 0 and the **yaw inertia** (`Iy`) is multiplied by `1 + closing*damp*8`, so a car
  hitting a wall nose-first at speed spins much less. Constants 0, 5, 8, -1 are hard-coded.

### 4.4 Oriented world objects

World `co` objects (props, big solids) are tested per world-flagged primitive: build the object's oriented
box (its centre shifted up by its half height along its own up axis), intersect it with the primitive,
push the body out by the overlap, then run the impulse routine **against an effectively immovable
body** with mass 1e6, box inertia (`Inertia::Box(1e6, 2*dims)`), and the object's velocity (0 here), using
the car's `OBJ_FRICTION[0]`/`[1]` (static / kinetic = min of both) times the object's surface
`WORLD_FRICTION`, `WALL_ELASTICITY` for restitution, and the usual vehicle tweaks. The body is marked
"had object collision" (blocks sleep).

## 5. Contact resolution (impulse routine)

All collisions use one routine, `React(plane, steps)` for body-vs-world and `React(other, plane, steps)`
for body-vs-body. Its body is **not in the decompilation**; this section specifies the interface and the
expected behaviour **[unconfirmed]** (classical impulse with Coulomb friction), to be tuned by measurement.

Inputs per body ("moment"): orientation, mass, principal inertia (scaled per axis by the *inertia scale*
vector, so a scale `> 1` makes that axis harder to rotate), COG offset, linear and angular velocity,
position, `immobile` flag with an optional break force, `fixedCG` flag, restitution `e` (scalar).
Inputs per contact `plane`: point, unit normal (from the *other* body toward this one), static and kinetic
friction coefficients `(Us, Uk)`.

Expected solve for `steps` iterations: relative velocity at the contact, normal impulse `j = -(1+e)·vn / (1/mA + 1/mB + rotational terms)`, Coulomb friction clamped by `Us·j` (static) or `Uk·j` (dynamic) against the tangential velocity.


Immobile bodies take no velocity change and contribute infinite mass. Outputs used by the caller: new
linear/angular velocity per body, `force = |impulse|` (reported as the collision force; `impulse * 1/m` is
the delta-v reported per body), the closing velocity, the sliding velocity, and the friction state
(None / Static / Dynamic; Dynamic sets the event's `sliding` flag). The reaction returns false when the
bodies are not approaching.


## 6. Car vs car

### Broad and narrow phase

Each tick every body that can collide with objects (has primitives, `NO_OBJ_COLLISIONS` false, not state 2)
is in a **sweep-and-prune grid** (max 64 bodies) on the x and z axes by position and primitive radius; the
cheaper-to-sort axis (fewer overlaps) is used to produce candidate pairs [decomp]. For each pair:

1. quick reject if `|posA - posB|² > max(1, (rA + rB)²)` where `r` = the body's largest
   `|dimension| + |offset|` over its primitives;
2. both bodies must agree (`CanCollideWith`): two bodies that are both "animating" never collide; a car
   with `mObjectCollisionsEnabled == false` (cutscenes) never collides;
3. every object-flagged, enabled primitive of A is tested with every one of B (box/box, sphere/box,
   sphere/sphere), world box with sweep delta = centre movement since the last tick;
4. a hit yields `normal` (from B to A), `point` and `overlap`.

### Resolution

```
for a hit:
    if either car is invulnerable (see §7): skip entirely (no push, no reaction)
    friction: Us = sqrt(A.OBJ_FRICTION[0] * B.OBJ_FRICTION[0]); Uk = sqrt(A.OBJ_FRICTION[1]*B.OBJ_FRICTION[1])
              plane friction = (min(Us,Uk), Us)
    restitution per body: e_X = | n projected on X's body axes * X.OBJ_ELASTICITY.xyz |
    each body: ModifyCollision(other)  (below)
    separate: push apart along n by `overlap` (see below)
    React(A, B, plane, 32 steps)
    for each mobile body: take new velocities, wake it, mark moved
    raise a collision event (type object)
```

**Separate.** If exactly one body is immobile, the other moves the full overlap. If both are mobile: two
cars are split by **mass ratio** (`moveB = mA/(mA+mB)`, `moveA = 1 - moveB`, so the lighter car moves
more); two smackables use their closing speeds. Both immobile: nothing moves and no reaction. The contact
point is shifted with the body that penetrates.

**`ModifyCollision` (per body, `RigidBody` base)** [decomp]: inertia scale vector = `OBJ_MOMENT_SCALE.xyz`;
if `OBJ_MOMENT_SCALE.w > 0` the *effective mass* is multiplied by it; if `IMMOBILE_OBJECT_COLLISIONS`
the body is immobile against mobile partners; a world-attached body is immobile with its detach force as
break force; an anchored body gets `fixedCG`. `IsImmobile` = `IMMOBILE_OBJECT_COLLISIONS` or attached to
world.

**`ModifyCollision` for vehicles (`RBVehicle`)** [decomp]:

- A vehicle being animated (cutscene) is immobile.
- Other body is a **vehicle**:
  - this is the **player**: if both cars have `> 2` wheels on the ground, the reaction COG is lifted to
    `cg.y = contactPoint.y - position.y - 0.125` (so the hit torque pivots near the road, reducing flips);
  - this is an **AI car** and the other is a *racer or human driver* and the AI has `collisionreactions`
    set: pick a record by contact location (below), then
    `e = clamp(e + rec.Elasticity + 0, 0, 1)`; if `rec.MassScale > 0` multiply inertia and mass by it;
    `cg.y += rec.RollHeight; cg.z += rec.WeightBias`.
- Other body is a **smackable** that is not immobile and whose attributes set `NO_CAR_EFFECT`: the car is
  immobile (the prop is knocked aside, the car is unaffected).
- If the vehicle has a **collision mass** override (`SetCollisionMass`) and is mobile: inertia is rescaled
  by `collisionMass / mass` and mass replaced. A **collision COG** offset (`SetCollisionCOG`) is added to
  the reaction COG. (Both are set by gameplay code, e.g. the pursuit; values not in this spec.)

**Choosing the reaction record** (`collisionreactions` class; four records, each
`{Elasticity, RollHeight, WeightBias, MassScale, StunSpeed, StunTime}` floats):

```
rp = contact.point - position
front = dot(rp, forward) > 0
side  = |dot(normal, forward)| < 0.707       # contact normal more than 45 deg from the long axis
record = front ? (side ? FRONTSIDE_REACTION : FRONT_REACTION)
               : (side ? REARSIDE_REACTION  : REAR_REACTION)
```

The AI picks which `collisionreactions` instance to use from its `aivehicle` attributes: the
`PlayerCollisions` list entry whose `Goal` equals the current AI goal name, else `PlayerCollisionsDefault`;
only for driver classes traffic through racer. `StunSpeed`/`StunTime` are used by AI code, not here.

## 7. Invulnerability

`RBVehicle` carries a state (`none`, `from reset`, `from manual reset`) and a timer. While not `none`,
any penetration test against this car is refused (the other car sees no hit, no push-out) and the time of
the last refused hit is remembered. The state clears when `>= 1 s` has passed since the last refused hit
**and** the timer (decremented by `dT` each tick) has reached 0 [decomp]. Resets set the timer to 2 s.

## 8. Reset to road

`ResetCar` is a per-vehicle behaviour with a task run every frame (variable rate) [decomp]. Each check
(skipped while paused):

1. *Flipped timer:* if some wheels are off the ground, the body is modelled, `dot(up, groundNormal) < 0.5`
   **and** `up.y < 0.5`, add `dT` to `flippedOver`; else set it to 0.
2. If the car is in a track-path zone of type reset-to-point, reset to that zone's point and direction now.
3. Else **record a breadcrumb** ("cookie": position, forward vector, time, recorded flag; the trail keeps
   the 4 newest) when the newest cookie is `>= 2 m` away (or none), the surface under the car's own
   position is not the null surface, and every wheel is on the ground.
4. Else, if a cookie exists and (the driver is human, the car's own surface is null and at least 2 wheels'
   road surfaces are null, **or** `flippedOver > 4 s`), reset to the **oldest** cookie.

**Manual reset:** try a guided-reset zone first; else use the oldest cookie unless the car is still
invulnerable from a previous reset. A *recorded* cookie is first snapped to the nearest road: nearest lane
of the "reset" lane set toward the cookie's direction (race lanes during a race), accepted only if the lane
point is within 6 m in height of the cookie, the straight line to it crosses no enabled track barrier, the
world has ground there and the ground is within 3.5 m of the cookie's height; position := lane point,
direction := lane forward (reversed on race segments if opposite the race direction). A scripted
position/direction (`SetResetPosition`) replaces the trail.

**Placing:** the vehicle's `SetVehicleOnGround(position, direction)` (not in the decompilation) calls the
body's `PlaceObject`: state := awake, set orientation, clear force, torque and both velocities, set the
position, clear moved flags, clear and refresh the world collider. On success: a "vehicle reset" event,
cookies cleared, `flippedOver = 0`, invulnerability for 2 s (automatic or manual flavour).
Constants: flip time 4 s, cookie spacing 2 m, dots 0.5, altitude limits 6 m and 3.5 m, invulnerability 2 s.

## 9. AttribSys fields read

`rigidbodyspecs` (one record per vehicle class, reached through the vehicle's `rigidbodyspecs` link in
`pvehicle`). Vector4 fields use `.xyz` unless noted. "(b)" = body axes (x right, y up, z forward).

| Field | Type | Unit / meaning |
|---|---|---|
| `GRAVITY` | float | m/s²; added as `GRAVITY*mass` to `force.y` each tick |
| `CG` | vec4 | m, body-space COG offset |
| `DRAG` | vec4 | unitless quadratic coefficients per body axis (b); zero vector disables |
| `DRAG_ANGULAR` | vec4 | unitless per axis (b); zero disables |
| `NATURAL_ANGULAR_DAMPING` | float | not read by the rigid body |
| `SLEEP_VELOCITY` | float | m/s; compared with `|v| + |w|*radius` |
| `COLLISION_BOX_PAD` | vec4 | m, added to the default/box half dimensions |
| `DEFAULT_COL_BOX` | string key | `SPHERE` or box |
| `BASE_MATERIAL` | string key | surface of the default primitive |
| `NO_GROUND_COLLISIONS`, `NO_WORLD_COLLISIONS`, `NO_OBJ_COLLISIONS` | bool | disable that class |
| `INSTANCE_COLLISIONS_3D` | bool | selects the ground algorithm (§4.2) |
| `IMMOBILE_OBJECT_COLLISIONS` | bool | body never moves in car-vs-object hits |
| `GROUND_FRICTION[0..1]` | float | static, kinetic coefficients (kinetic clamped to <= static) |
| `GROUND_ELASTICITY` | vec4 | restitution per body axis (b) |
| `GROUND_MOMENT_SCALE` | vec4 | inertia multiplier per axis (b) in ground hits |
| `WALL_FRICTION[0..1]` | float | static, kinetic, vs barriers and world objects |
| `WALL_ELASTICITY` | vec4 | restitution per axis (b) vs barriers and world objects |
| `WORLD_MOMENT_SCALE` | vec4 | inertia multiplier per axis (b) in world hits |
| `OBJ_FRICTION[0..1]` | float | static, kinetic, car-vs-object |
| `OBJ_ELASTICITY` | vec4 | restitution per axis (b), car-vs-object |
| `OBJ_MOMENT_SCALE` | vec4 | `.xyz` inertia multiplier; `.w` mass multiplier if `> 0` |

`pvehicle`: `MASS` (kg), `TENSOR_SCALE` (vec4, per-axis multiplier of the box tensor), `rigidbodyspecs`
(link), `BEHAVIOR_MECHANIC_RIGIDBODY` / `_RESET` / `_SUSPENSION` (which behaviours to instantiate),
`BEHAVIOR_ORDER`. `collisionreactions`: `FRONT_REACTION`, `FRONTSIDE_REACTION`, `REAR_REACTION`,
`REARSIDE_REACTION` (each a six-float record). `aivehicle`: `PlayerCollisions`, `PlayerCollisionsDefault`.
Surface (`SimSurface` class, shared with tire grip): `GROUND_FRICTION`, `WORLD_FRICTION` (float
multipliers on the friction pair). Other game objects: `NO_CAR_EFFECT` (bool) on smackable attributes.
Track-path zone data (`RESET_TO_POINT`, `GUIDED_RESET`) comes from the track path chunk, see
[world.md](../formats/world.md).

## How to check it

- Drop a body (no wheels) on a flat road: it should settle with `contact_points > 2` and sleep once
  `|v| + |w|*r < SLEEP_VELOCITY`; free-fall time should match `GRAVITY`, speeds clamp at 300 m/s.
- Slide a car along and into a wall at several angles and speeds: compare rebound with `WALL_ELASTICITY`,
  the nose-first yaw damping (§4.3) and friction deceleration with `WALL_FRICTION`.
- Collide two cars of different mass: the lighter one gets most of the push-out and delta-v; compare AI
  versus player hits with the `collisionreactions` record chosen by contact side.
- Flip a car: auto reset after 4 s, position on a lane, 2 s invulnerability.
