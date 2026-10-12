# World event sequencing and vehicle fragments

Sources: [event format](../formats/event-sequences.md), the original PC 1.3
executable, and the CC0 [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw)
reference: `EventSequencer.cpp`, `DamageVehicle.cpp`, `DrawVehicle.cpp`,
`DrawCar.cpp`, `SimModel.cpp`, `CarPartNames.cpp` and generated event layouts.
This spec precedes the implementation; unresolved behavior is listed explicitly.

## Sequencer

An engine has independent named systems. Each system starts in its initial
state (falling back to its default state if missing). A matching stimulus picks
an end state and an action. Fire its `start` tag, then timed events at time zero.
Advance action time on the shared physics clock; each event fires once when its
time is crossed. On reaching a nonnegative duration, fire `end`, clear the
action, enter its end state and process the oldest queued stimulus. Negative
duration means no automatic end. Render interpolation must not advance events.

The PC function at `0x645A40` resolves an error in the unfinished reference:
an **idle** system invokes a stimulus immediately; a busy system may queue it.
Queue modes are disable (0), shallow (1), allow (2), flush (3), abort (4).
Shallow permits one waiting item; allow permits four. When queuing, check the
response against the projected state at the end of the queued actions, using
the default state if needed. Unknown stimuli do not consume queue capacity.
Flush clears queued items; abort stops the active action before invoking.
An explicit event dispatch queue preserves list order and avoids recursively
borrowing systems when a process-stimulus event starts a different system.

Filters require expression evaluation. Until implemented, filtered states must
return an explicit unsupported result. Unknown event IDs remain observable to
the host, not treated as successfully implemented gameplay effects.

`EFireEventList` (`0xA3CFBFEC`) contains an event-list record tag at byte 4,
delayed flag at byte 8 and verbose flag at byte 12. An undelayed invocation
expands the named list at that position in the current list and retains the
current dynamic context. This is how TrailerB's combined release action invokes
its two smaller lists. Missing list references and delayed invocations require
an explicit error until their deferred context is implemented.

## Damage triggers

An impact's closing speed must be at least 1 m/s. Ignore a hit from a smackable
whose NO_CAR_EFFECT flag is set. Resolve the impacted vehicle's `damagespecs`.
Its FORCE times 1000 is the material strength. Object collisions use twice the
body's collision delta-speed times its mass. Classify the contact using its
local arm, local normal and half dimensions:

- Above/below half the longitudinal extent: front/rear. A corner is selected
  when the arm is beyond half the width and the normal points inward with a
  lateral component exceeding 0.8.
- Otherwise top/bottom when the vertical arm exceeds half height and the normal
  points inward with vertical magnitude exceeding 0.8.
- Otherwise right/left according to the sign of local arm.x.

Compute `floor(impulse / strength * zone.VisualScale)`, capped at six. Send
`COLLISION`, then vehicle/cop/player stimuli when applicable, then `IMPACT_1`
through the computed level to that zone's system. Keep the pre-impact velocity
as the event's dynamic velocity. The original also tracks hit points, visual
damage, shock, suppression distance and repair; those remain separate work. Visible
damage now accumulates per zone from these impacts ([vehicle-damage.md](vehicle-damage.md));
hit points, shock, suppression distance and repair are still open.

## Fragments

Resolve part hash to the car-part slot and collision hash to the bounds subtree.
For `ATTACHMENT0..9` the model slots are 52..61. A fragment needs both its mesh
and its collision subtree; missing references produce no fabricated part.
Use the event's smackable attributes and its inherited-velocity factor. Spawn
and draw from the subtree's frame, preserving the mesh's original world pose.
Hide only the parent part requested by the event, retaining it on failed spawn.
Keep detached bodies collidable, renderable and owned after their truck moves.

The PC DrawVehicle routine (`0x6B0500`) permits a spawn while the global vehicle
fragment count is below 121 and the model count is below 2297. The unfinished
reference's 61/434 limits belong to a different build and are not the PC limits. Its
offscreen lifetime comes from KILL_OFF_SCREEN, with the timer cleared on seeing
the fragment again. The exact fragment initialization, explosion response,
constraint limits, child cleanup and event ordering need PC routine validation
before claiming complete parity. Parsing a sequence does not prove its effects.

## Bounds and hitch

Named female/male joint bounds identify the tractor and trailer anchors. The PC
CreateJoint routine (`0x6718F0`) searches immediate children of each body's root
for the requested name and female/male flag, then uses their stored `position`
vectors directly as the local anchors. Do not derive them from absolute pivots:
separate quantization can produce different values. The tractor requests
`5THWHEEL` on both bodies. Missing either node means no joint can be created.
Do not infer anchors from vehicle length or a constant height.
Conical/prismatic child bounds carry constraint data. Their construction and
response are specified in [trailer constraints](trailer-constraints.md).

Internal hierarchy nodes belong to independently spawned parts. A spawned
fragment explicitly selects its own subtree; the rules measured below decide
which shapes belong to its body.

## PC construction measurements

The executable's Collection::AddTo (`0x672050`) and AddNode (`0x66A890`) were
executed in isolated x86 memory with synthetic bounds and a recording collision
interface. They select **immediate children**, not a flattened descendant list.
An internal child (`0x20`) is skipped, including its descendants. Other children
contribute their own primitive/mesh flags; disabled flags are retained for later
collision filtering. Child primitive centres use `position`, not `pivot`, and
their stored quaternion is relative to the selected root body's frame.

If no child contributed geometry, AddTo tries the selected root itself. Its
primitive has zero offset and identity rotation because the body's transform
already includes that root's transform. If that also has no collision flags,
RigidBody::CreateGeometries (`0x6A6E30`) creates a body-sized primitive, enabling
world/object/ground collision. This matters for cones whose root flags are only
`0x40`: they must still collide. A rigid-body DEFAULT_COL_BOX value of SPHERE
selects a sphere using the largest half extent, with world/object channels only;
otherwise the fallback is a box. COLLISION_BOX_PAD expands every collision box
(including explicit bounds boxes), but never a sphere. A bounds surface of zero
uses the body's BASE_MATERIAL. Sphere orientation is identity.

Scenery's placement transform (`0x66B520`) places the body's centre at the
spawner position plus its rotated root pivot. Its orientation is the spawner's
orientation. Rendering retains the original visual matrix through the inverse
initial body transform. Root dimensions have a minimum half extent of 0.025 m
in the smackable constructor. A fallback-only body has a zero local centre of
gravity. Explicit primitive offsets determine the volume-weighted centre.
Spheres use the stored radius, not half dimensions, and are inserted before
boxes (in reverse sphere insertion order). Mesh-only bounds can suppress a
fallback even when they add no object-colliding primitive.

The fragment event handler at `0x63B320` confirms that the same inheritance
factor multiplies both captured linear and angular velocity. It constructs the
child frame from the parent model transform and the selected node's local
orientation/position. Process-stimulus events (`0x646570`) do not forward the
previous damage callback's context, so the cargo action captures its model's
current velocity. The renderer applies the inverse fragment frame to keep the
detached mesh aligned at birth. Queue hashes map to disable `0xCC7F660A`, shallow
`0x669EC6BC`, allow `0`/`0x102ABF91`, flush `0x3C56741D`, abort `0x936F462E`.
