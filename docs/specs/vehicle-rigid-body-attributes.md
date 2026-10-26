# Vehicle rigid-body attributes

Sources and evidence tags: [rigid-body specification](vehicle-rigid-body.md).
These are data inputs, not a claim of complete runtime parity.

## AttribSys fields read

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
