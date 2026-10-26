# Chameleon hinged parts: bonnet, bumpers, mirrors and doors of a 2012 car

What the car bundles of NFS: Most Wanted (2012) say about the parts of the body that hang on hinges and come loose when
the car is damaged, how this repo reads it ([`libs/chameleon-vehicle`](../../libs/chameleon-vehicle/src/hinged.rs),
`skeleton.rs`, `skin.rs`), and what is still unknown. Containers: [chameleon-bundle.md](chameleon-bundle.md); objects and
types: [chameleon-vehicle-stats.md](chameleon-vehicle-stats.md#genesys-types-and-objects-verified); the rest of the damage
data: [chameleon-collision.md](chameleon-collision.md#damage-and-impact). Evidence tags: [README](../README.md#evidence-tags);
**[exe]** is a string or table of `NFS13.exe` (v1.4.0.0).

Everything below was measured on the 93 `VEHICLES/VEH_<n>_HI.BNDL` of the install with the Rust readers and throwaway
scripts outside the repository, unless tagged otherwise. Axes are the file's: x left, y up, z forward, metres, the origin
on the ground between the wheels.

## Short answer

- A car's **damage behaviour** (`Genesys.Gen.VehicleDamageBehaviour`, 3,200 bytes on the 911) lists one `Bodypart` per
  hinged part. A `Bodypart` has the part's name, its centre, its mass, three angle ranges, the damage zone that drives it and
  a ladder of damage fractions **[verified]** (offsets below).
- The part's **name is the name of a bone** of the car's skeleton (`BonnetHinge`). The skeleton stores the bit-inverted CRC-32
  of each bone name, which is how the 7 names were found among the 37 bones of the 911 **[verified]**.
- The **body is skinned**. A vertex carries four bone indices and four weights. The triangles weighted to a hinge bone, or
  to a bone below it in the skeleton, are the part **[verified]**: the 911's bonnet is 418 triangles of the finest level,
  its front bumper 4,912, mirrors 709 each, rear bumper 3,090, doors 1,136 and 1,146.
- The ranges are small: bonnet -15 to 0 degrees, bumpers 2 to 5, mirrors -15 to 15, doors -1 to 1. A part swings a little on
  its hinge; **nothing in the data detaches a part** **[unconfirmed: no field was found that does]**.
- The same skeleton and skin carry the 2012 body's **crumple**: the behaviour imports a 19,504-byte `AnimationList` of 8
  animations over the 37 bones, one per damage zone **[unconfirmed]**. That is a deformation by bones, not by mesh swaps,
  and it is not read by this repo.

## `Bodypart` (`Genesys.Gen.VehicleDamageBehaviour.Bodypart`, 192 bytes)

Fields are addressed by offset in the structure, counting its 12-byte head ([chameleon-vehicle-stats.md](chameleon-vehicle-stats.md)
explains why: field names are hashed). The behaviour holds an array of them (offset at +0x24 of the behaviour, count in the
u16 at +0x30, 7 on the 911), then one `DamageThreshold` (24 bytes) per ladder step and one `DamageSequence` per zone.

| Offset | Type | Field | Evidence |
|---|---|---|---|
| +0x10, +0x20, +0x30 | f32 min, f32 max (in 16 bytes) | the angle range about the car's x, y and z axes | the 911's bonnet has x -15 to 0, which lifts its front edge about the cowl hinge (a rotation of -15 degrees about +x takes a point ahead of the hinge up); the mirrors fold backwards at +15 (left) and -15 (right) about y **[verified geometrically, unit degrees unconfirmed]** |
| +0x40 | f32 x3 | the part's centre | inside the bounding box of the triangles skinned to the part on all parts checked on the 911, for example the left mirror (0.91, 0.92, 0.34) in (0.80..0.99, 0.84..1.00, 0.25..0.48) **[verified]** |
| +0x50, +0x60, +0x70 | f32 x3 each | three triples, one per axis: (2, 0.15, 0.5), (1, 0.1, 1), (2, 0.1, 0.15) on the bonnet | spring and damping per axis **[unconfirmed]** |
| +0x80 | f32 x4 | (0, 0, 1, 0) on all parts of the 911 but the rear bumper, (-3, -5, 0.35, 0) | **[unconfirmed]** |
| +0x90 | string reference | the name: a u32 offset from the field itself, then the length with the terminator | **[verified]** |
| +0xA0, +0xA4 | u32 x2 | object ids; non-zero on the rear bumper only | **[unconfirmed]** |
| +0xA8 | f32 | mass in kg: 50 for bonnet and bumpers, 30 for mirrors and doors on the 911 | **[unconfirmed meaning]** |
| +0xAC | u32 | offset (from the start of the object) of the array of thresholds | **[verified]** |
| +0xB0 | u16 | the zone code (below) | **[verified]** by the headlight hinges |
| +0xB2 | u16 | the number of thresholds | **[verified]** |

A `DamageThreshold` has an object id at +0x0C (0 on the 911) and an f32 at +0x10, a damage fraction from 0 to 1. The 911's
ladders: bonnet 0.04, 0.05, 0.5; front and rear bumper 0.04, 0.05, 0.5, 0.75; mirrors 0.0, 0.09, 0.1, 0.19, 0.2; doors 0.4,
1.0. What a step does is not decoded: a part with more steps has more states, and the last one is the most damaged
**[unconfirmed]**.

### The zone code

The code is 1 for the front, 2 the rear, 3 the left, 4 the right, then 5 to 8 for the corners, in the order of the eight
damage bars the HUD reads (`GetFrontDamageBar`, `GetRearDamageBar`, `GetLeftDamageBar`, `GetRightDamageBar`,
`GetFrontLeftDamageBar`, `GetFrontRightDamageBar`, `GetRearLeftDamageBar`, `GetRearRightDamageBar` **[exe]**). Evidence:
across the 93 cars, `BonnetHinge`, `FrontBumperHinge` and `LightBarHinge` and `PushBumperHinge` are 1, `BootHinge`,
`RearBumperHinge` and `SpoilerHinge` are 2, left doors and mirrors 3, right ones 4, and `LeftHeadlightHinge` is 5 and
`RightHeadlightHinge` is 6, the front corners **[verified]**. The 2005 game's zones (`DZ_FRONT` to `DZ_RREAR`) are in the same
order, so code minus one is the index of the zone there.

## The skeleton (type 0xB2)

One per car bundle. Main block: u16 version (2), u16 bone count, u32 offset of the bone table (0x20), u32 offset of the hash
table (twice). A bone is 0x30 bytes: f32 x, y, z, pad; an f32 quaternion; then four i32: parent (-1 for the root), previous
sibling, next sibling, own index. The hashes are u32 each, one per bone **[verified]** (the offsets of the tables and the
tree on all 93 cars; the 911's tree has 37 bones).

A bone's hash is the **bit-inverted CRC-32** of its name: `Root`, `BonnetHinge`, `FrontBumperHinge`, `LeftMirrorHinge`,
`RightMirrorHinge`, `RearBumperHinge`, `FrontLeftDoorHinge`, `FrontRightDoorHinge` and `SteeringWheel` all matched **[verified]**.
The positions are absolute, in the same space as the mesh, not relative to the parent (the left mirror's bone (0.82, 0.85,
0.42) lies in its triangles' box) **[verified]**. The bones of the 911 are, by name or by where their triangles are: 1 front
bumper with 2 to 6 below it, 7 rear bumper with 8 to 12, 13 bonnet with 14 to 17 below it (the headlamps and the front
corners of the bonnet), 18 to 24 lamp and arch bones, 25 left door with its handle (26), mirror (27, own hinge) and window glass
(28), 29 to 32 the right side, 33 and 34 the rear spoiler and its glass, 35 the steering wheel, 36 the engine cover
**[verified by the skin, names unconfirmed except those above]**. Counts: 33 to 47 bones on 92 cars; the SWAT van has 1 and no
skin.

A bone's rest position is the **hinge point**: the bonnet's (-0.70, 0.88, 0.74) is on the cowl line and the door's (0.83,
0.63, 0.68) is at the front edge of the door, so each part turns about its own axis through the bone **[verified geometrically]**.

## The skin

The vertex layouts of the body use two elements that [chameleon-gfx.md](chameleon-gfx.md) lists as damage data:

| Usage | Format | Meaning |
|---|---|---|
| 13 | `R10G10B10A2_UINT`, 4 bytes | **four bone indices**, one byte each **[verified]**: values 0 to 36 on the 911, 255 for none |
| 14 | `R8G8B8A8_UNORM`, 4 bytes | **four weights**, one byte each, the weight of the index at the same position **[verified]**: (255, 0, 0, 0) is a rigid vertex, a bonnet vertex is (221, 27, 7, 0) over bones (13, 14, 16) |

The reader gives a triangle to the part with the largest summed weight over its three corners, counting a bone for the part
that owns it (its hinge bone and the bones below it, unless a deeper hinge owns them: the mirror on a door) and every other
bone for the body, and leaves it to the body when the body outweighs the parts (`skin::body_parts`).

Results on the 93 cars **[verified]**:

- **90 cars** have every part non-empty on the finest level. The two exceptions are `VEH_1097100` (a body of 52,521 triangles) and `VEH_2196065` (the *Cars* tie-in car, 5,672
  triangles): their vertices have the two skin elements but every weight is 0, so every triangle falls to the body. The SWAT
  van has a one-bone skeleton and no part.
- **Only the finest level is skinned** (the 911's second and third levels of the body give the parts 0 triangles, except the
  mirrors with 44 and 46), so a part cannot be cut out of a coarser level.
- The 911's parts hold 12,119 of the 30,908 triangles of the finest body level; an unsplit body is what the other levels draw.

## The parts of the 93 cars

Names found, with the number of cars and the zone code **[verified]**:

| Part | Cars | Zone | Part | Cars | Zone |
|---|---|---|---|---|---|
| `BonnetHinge` | 88 | 1 | `LeftMirrorHinge` | 89 (+1 `...1`) | 3 (4) |
| `FrontBumperHinge` | 85 | 1 | `RightMirrorHinge` | 82 | 4 |
| `BootHinge` | 76 | 2 | `FrontLeftDoorHinge` | 83 | 3 |
| `RearBumperHinge` | 84 (+1 `...1`) | 2 | `FrontRightDoorHinge` | 84 | 4 |
| `SpoilerHinge` | 19 | 2 | `RearLeftDoorHinge` / `RearRightDoorHinge` | 9 / 8 | 3 / 4 |
| `LightBarHinge`, `PushBumperHinge` | 4, 3 | 1 | `LeftHeadlightHinge` / `RightHeadlightHinge` | 2 / 2 | 5 / 6 |
| `LeftDoorHinge` / `RightDoorHinge` | 2 / 2 | 3 / 4 | `FrontLeftDoorWindow`, `FrontRightDoorWindow` | 2, 7 | 3, 4 |
| `ExhaustHinge`, `LeftExhaustHinge`, `RightExhaustHinge` | 2, 1, 1 | 4, 3, 4 | | | |

The 911 has no `BootHinge` (its engine is behind). Each car lists only the parts it has.

## What the repo does with it

- `chameleon_vehicle::hinged_parts(bundle)` reads the parts (name, bone, zone, centre, ranges, mass, ladder);
  `Skeleton::read` the bones; `body_parts(pool, spec, skeleton, parts)` the triangle classes of every level of the body,
  aligned with `Vehicle::body`.
- The 2012 content mod ([nfsmw-2012-content.md](../modding/nfsmw-2012-content.md#hinged-parts)) cuts those triangles out of
  the finest body solid into one solid per part and writes a `[[damage_parts]]` table per part
  ([car-damage-parts.md](../modding/car-damage-parts.md)). It condenses the three ranges into the one largest swing (the
  axis and end with the largest magnitude) and keeps the ladder as the stages.

## Not decoded, not verified

- **The consumer.** The fields above were read from data. `NFS13.exe` holds no names of the fields; the only string of the
  family is `Rear_Bumper_Hinge` at 0x00fae80c **[exe]**, and the table that registers the damage types (hash and type id
  pairs, 0x00f95bbc to 0x00f95be0) has no code reference, so no function that reads a `Bodypart` was found. The meaning of
  the ranges (degrees, per axis), the three triples and the ladder is inferred.
- Whether a part ever **detaches** in the 2012 game, and what each ladder step does.
- The 8 damage animations of the `AnimationList` (0x0C4DA5 on the 911), the `DamageSequence` effects (one `Handle` per zone)
  and the `SpotEffect` list (empty on the 911).
- Coarser body levels: no skin, so no part can be cut from them.
