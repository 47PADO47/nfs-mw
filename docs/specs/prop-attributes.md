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

## Duplicate collision collections

**[verified]** Within one bounds pack, the PC loader visits collections in file
order and keeps the first collection for each name hash. A later duplicate does
not replace it. The track catalog must preserve this precedence even when the
later record is a single broad box and the first contains collision children and
fragment nodes. Select by key and file order, not by node count or model name.

The 2026-10-12 original-game contact capture used a narrow object-channel pole
primitive from the first traffic-light collection, including its authored offset;
the later duplicate only contained the full assembly's broad root. Replacing the
first with the duplicate incorrectly makes that broad root the fallback collider
and loses fragment geometry. The constructor and lookup were also inspected in
the same PC executable. The installed track metadata contains a single bounds
pack; runtime precedence between separately loaded packs is a separate streaming
concern. This rule establishes duplicates within a pack.

## Authored static collision sources

**[decomp + verified]** A scenery instance without an object spawner does not
acquire an extra rigid collider by matching its model name to a bounds collection.
The inspected PC static-world path queries authored `CARP` collision instances
(triangles and barriers), plus grid-registered collision objects. At track
initialization, the latter also include the authored `TrackOBB` list
(`0x00034191`): each record supplies its own matrix and dimensions and is
registered as a fixed box with surface 0. This registration does not search
scenery names or add name suffixes.

The measured install has no `co` records in its 390 collision packs and one
`TrackOBB` record; these are asset measurements, not format restrictions.
Individual scenery without a spawner can still be solid through its authored
world triangles or barriers. Removing the inferred scenery-name boxes must
preserve those world contacts and the spawner-driven prop path. A bounds record's
existence alone does not establish that every matching visual placement has
collision. See [collision sources](../formats/collision.md#collision-objects-co-decomp).

The rewrite's standalone `TrackOBB` collision integration remains a baseline
gap. Scenery-name fallback boxes are not a substitute for that authored list.
This bounded registration audit does not establish the original collision of
every individual scenery placement.

## Contact and initialization rules

The PC behavior-change callback confirms: a virgin placement with nonzero
`DETACH_FORCE` attaches to the world; negative values attach with a zero break
threshold (permanent). A zero value starts unattached and asleep. Without an
explicit centre marker, the collision centre is the volume-weighted average of
object-colliding primitive offsets. Inertia still comes from the root dimensions.
Bounds object flag `0x04` becomes runtime primitive flag `0x02`; disabled bounds
remain in the mass calculation but are excluded from contacts. World-only bounds
must not be substituted for object primitives. Body selection, fallback shapes,
padding and material rules are in [world sequencers](world-sequencers.md).

The vehicle response override makes the vehicle's temporary contact moment
immobile when the other body is a movable smackable with `NO_CAR_EFFECT=true`.
This preserves the car's velocity while transferring motion to the prop. An
attached prop does not enable this override: the car can slow down while breaking
it loose. The next contact sees the detached state. A zero break threshold never
detaches an attached object. These are temporary contact flags, not a frozen car.

Separate objects before applying the 32-iteration contact response. If exactly
one is immobile, move only the other. If both are mobile, use the stored mass
ratio for car/prop contacts. For prop pairs use the normal-speed shares described
in [box contacts](box-object-contacts.md). When both moments are immobile, first
release any with a positive break threshold. Two unbreakable immobile moments
produce no separation or response. Shift the point with its incident surface and
translate temporary contact centres with their bodies. Compound primitive
visitation follows [box contacts](box-object-contacts.md); global body and pair
ordering remain separate parity work.

## Object and world contact order

**[decomp + verified]** The inspected PC rigid-body update runs begin-frame
callbacks and updates the broad phase for all bodies, then resolves object pairs,
then runs end-frame callbacks. Vehicle and smackable end-frame callbacks perform
world collision and drag before the final articulation pass. Car/prop collision
response therefore sees the incoming velocity before world barriers and other
static contacts remove it. Resolve car/prop contacts before the vehicle's wall
response when both occur in the same tick; do not compensate for an earlier wall
response by inventing additional impact speed or lowering the detach threshold.

This ordering does not prove that a nearby world barrier causes any particular
pole's failure to detach. That requires the actual contact geometry and a
same-tick reproduction. The original uses global body phases; preserving the
local car/prop-before-wall order alone does not establish global pair visitation
or complete physics-step parity. See the
[ordering provenance](../provenance/prop-world-contact-order.md).

## Runtime integration boundary

The first dynamic integration uses the existing body's fixed-step gravity/drag,
ground-plane response, measured box contact and render interpolation. Detached objects
remain rendered and collidable. Original ground-point query tolerances,
fragments, event sequencers, offscreen destruction and respawn are not established
by this stage. In particular, do not replace those missing lifecycle rules with
a six-second disappear/reappear timer.

## Ground support integration

The existing height-query integration must include every enabled ground mesh
vertex in the body frame, transformed by the current body pose. Disabled clouds
do not support the body. Ground box corners and cloud points are separate contact
sources; a sphere with ground flags must not be treated as a box. Keep at most
16 contacts, in primitive/cloud visitation order, then sort deepest first.
All support queries and impulse resolution run on the shared fixed clock.
This fixes missing support geometry; it does not certify the existing plane
solver or query tolerances against the original.
