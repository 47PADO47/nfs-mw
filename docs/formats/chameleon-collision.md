# Chameleon collision, physics shapes and damage data of a car

What a car of NFS: Most Wanted (2012) keeps for collision, physics shapes, damage and impact, and what this repo reads
of it. Container and resource types: [chameleon-bundle.md](chameleon-bundle.md); graphics, wheels and names:
[chameleon-vehicle.md](chameleon-vehicle.md). Reader: [`libs/chameleon-vehicle/src/collision.rs`](../../libs/chameleon-vehicle/src/collision.rs).
How the mod uses it: [car-collision.md](../modding/car-collision.md). Evidence tags: [README](../README.md#evidence-tags).

Everything below was measured on the 93 `VEHICLES/VEH_<n>_HI.BNDL` of the 2012 install (v1.4.0.0, `NFS13.exe`) with the Rust
reader and with throwaway scripts outside the repository, unless tagged otherwise. Positions are in the engine's car space:
x left, y up, z forward, metres, the origin on the ground between the wheels (the same space as the wheel hubs).

## What a car holds

| Data | Resource | Per car | Read here |
|---|---|---|---|
| Collision mesh | `PolySoupList` (0x60), imported by the graphics spec | 1 (92 of 93 cars; one is empty) | yes, [below](#the-collision-mesh-polysouplist-0x60) |
| Convex hulls of the chassis | `ConvexHull` (0x303) | 5 (92 cars) or 4 (1 car), 8 to 20 vertices each | format read, not used |
| Physics body definition | `GenesysObject` (0x15) of type `RaceCarPhysicalDefinition` | 1 | partly decoded, not read |
| Damage behaviour | `GenesysObject` of type `VehicleDamageBehaviour` and `Vehicle_Gameplay.DamageStats` | 1 each | the hinged parts, [decoded](chameleon-hinged-parts.md); the rest names and counts only |
| Skeleton, hinge animations | `Skeleton` (0xB2), `Animation` (0xB3), `HSMData` (0x501) | 1 each | no |

Not found in any car bundle: a deformable or crumple mesh, damage mesh swaps (no `DAMAGE0_*` style parts), impact point
locators, bumper or crumple zone volumes, a collision box other than the hulls and the soup. The 2012 body is not
deformed by mesh swaps: damage is a set of numbers per zone, hinged parts ([chameleon-hinged-parts.md](chameleon-hinged-parts.md))
and, probably, bone animations of the skinned body (an `AnimationList` of 8 animations over the car's bones, one per zone
**[unconfirmed]**) **[verified]** by the type list of the bundles (the `GenesysType` resources name every kind of object a car
can hold).

## The collision mesh (`PolySoupList`, 0x60)

The graphics spec (type 0x106) imports it (at offset 0x8 of its main block in the 911; the reader looks for the import
whose type is 0x60 instead). It is the shape the 2012 world collides a car's body with **[verified]** by its extents,
which are the body's: the 911 Carrera S has x -0.90 to 0.885, y 0.17 to 1.31, z -2.27 to 2.23, and the bundle's body
model fits it to within a few centimetres on all 92 cars (the visible body sticks out of it by up to 0.22 m: mirrors,
spoilers). A car's soup is 166 to 256 triangles.

Main block (all little endian):

| Offset | Type | Field |
|---|---|---|
| 0x00 | f32 x3, pad | minimum corner of the whole soup |
| 0x10 | f32 x3, pad | maximum corner |
| 0x20 | u32 | offset of the table of chunk offsets (0x30) |
| 0x24 | u32 | offset of the table of per-chunk boxes (SIMD layout, two chunks to a block; not needed) |
| 0x28 | u32 | chunk count (4 to 7) |
| 0x2C | u32 | size of the block |
| table | u32 x count | offset of each chunk |

The header box of one car (`VEH_435169`) reaches z = -500: it is computed from a stray vertex no triangle uses, see below.
An empty soup (one car, the SWAT van, `VEH_1467242`) is a 48-byte block with chunk count 0.

A **chunk** is at most 255 vertices and 255 triangles:

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | offset of the triangle table |
| 0x04 | u32 | offset of the vertex table |
| 0x08 | u16 | size of the chunk |
| 0x0A | u8 x4 | axis flags x, y, z, 0: non-zero = the axis is centred (see below) |
| 0x0E | u8 | triangle count |
| 0x0F | u8 | vertex count |

A **vertex** is three `u16` (6 bytes). The value of an axis is `q * 500 / 32768` metres, minus 500 when the chunk's flag for that
axis is set. That is a 1000 m range at 1.5 cm steps; an axis that crosses zero is stored centred (flag set), an axis that
stays positive (the heights, usually the front chunk's z) is not. **[verified]**: decoded this way, the extents of every
car's chunks equal the header box to 0.0 m (92 of 92), and the first three chunks of the 911 equal the stored per-chunk
boxes exactly.

A **triangle** is 12 bytes: 4 bytes of data (0 in all cars; probably a surface or flags word **[unconfirmed]**), the three
vertex indices as bytes, and 5 bytes of 0xFF (links to the neighbouring triangles, unused here **[unconfirmed]**).

Reader: `chameleon_vehicle::load_poly_soup(pool, spec)` gives `PolySoup { vertices, triangles }`. It keeps only the vertices
a triangle uses (the stray vertex of `VEH_435169` is dropped), checks every index against the chunk and every offset
against the block, and returns `None` for the empty soup.

## Convex hulls (`ConvexHull`, 0x303)

Five hulls per car (the 911: 12, 12, 9, 8 and 20 vertices), 4 on one car, named by `ConvexHullResourceType` in the
executable **[exe]**. The block is a half-edge hull: `u32` vertex count, edge count (vertices + faces - 2) and face count at
0x00, 0x04, 0x08; a table of edge and face indices; the vertices as `f32 x, y, z, 0` (16 bytes each) at the offset that the
`u32` at 0x30 holds, **relative to its own address** (0x30 + value), with their count in the `u32` at 0x34; and, at 0x44 + the `u32`
there, rows of `f32 nx, ny, nz, d` planes (the count of rows is not determined **[unconfirmed]**). The hulls are in their
own frame, centred on the origin.

They are placed by the physics body definition (next section) and are an **inner** chassis: the 911's five hulls are 1.22 m
wide at most (half width 0.609) where the collision mesh is 1.79 m, and across the 92 cars the widest hull half width is 0.52
to 1.24 m against 0.84 to 1.10 m for the mesh. The wheels stand outside them (hub x 0.77, tyre inner face 0.65). So they are
not the contact shape of the body; they are the volume the rigid-body dynamics uses **[unconfirmed]**. This repo does not use them.

## The physics body definition (`RaceCarPhysicalDefinition`)

One `GenesysObject` per car, 3,360 bytes in the 911, imports the five hulls and is built from the types
`RaceCarPhysicalDefinition`, `.PhysicalDefinition`, `.RigidBody` (volume lists `BoxVolume`, `SphereVolume`, `CylinderVolume`,
`CapsuleVolume`, `ConvexHullVolume`) and `ConvexHullConectivityData` **[verified]** (type names in the bundle). Its rigid bodies,
named by strings in the object (`this.ChassisRigidBody`, `FrontLeft.WheelRigidBody`, ..., `RearRight.WheelRigidBody`), are
laid out as records of 0xC0 bytes (wheels) and 0x60 bytes per hull volume. What was read on the 911, and matches the rest of the
bundle:

- the chassis body has a position (0, 0.45, -0.27), a centre of mass or bounds centre (0.002, 0.269, 0.247) and a box
  half size (0.91, 0.55, 2.21) **[unconfirmed meaning]**, and a mass of 1295 kg at offset 0x138 of the object (field mapping
  from the type description is not decoded);
- each wheel body sits at the hub of the graphics spec (0.769, 0.338, 1.24 front; 0.758, 0.34, -1.21 rear) with a size of
  (0.34, 0.34, 0.123): radius, radius, half width **[verified]** against the spec's hubs;
- each hull volume is an identity rotation, a translation and the hull's import. The 911's translations are
  (0.007, 0.259, 0.265), (0.004, 0.130, -1.467), (-0.003, 0.097, 1.627), (0.004, 0.643, -0.088) and (0.004, 0.111, -0.088)
  for the hulls of 20, 12, 12, 9 and 8 vertices. Added to the chassis position they put the 20-vertex hull at
  (0.007, 0.709, -0.005) with y from 0.15 to 1.27, which is the soup's height range (0.17 to 1.31) **[verified numerically on
  the 911; the offset is the chassis position: unconfirmed]**.

None of this is read by the mod. Handling numbers (mass, engine, tyres, suspension) are in other objects of the bundle and
are the business of the car's stats, not of its collision.

## Damage and impact

The bundle has a `VehicleDamageBehaviour` object (3,200 bytes in the 911) with the type `Bodypart`, `Bodypart.DamageThreshold`,
`SpotEffect` and `DamageSequence`. The fields of a part (centre, angle ranges, mass, zone, ladder, the bone it is skinned to)
are decoded in [chameleon-hinged-parts.md](chameleon-hinged-parts.md). In the 911 it lists seven hinged parts by name: `BonnetHinge`, `FrontBumperHinge`,
`LeftMirrorHinge`, `RightMirrorHinge`, `RearBumperHinge`, `FrontLeftDoorHinge`, `FrontRightDoorHinge`, 25 damage thresholds and
eight damage sequences (empty on all 93 cars). The crumple and breakage clips of the zones are in [chameleon-damage.md](chameleon-damage.md). The eight are tied by name to the eight damage bars the HUD reads from the car:
`GetFrontDamageBar`, `GetRearDamageBar`, `GetLeftDamageBar`, `GetRightDamageBar`, `GetFrontLeftDamageBar`,
`GetFrontRightDamageBar`, `GetRearLeftDamageBar`, `GetRearRightDamageBar` (the string `GetFrontDamageBar` is in the executable
**[exe]**; the objects of each are in the bundle). So the 2012 damage zones are **front, rear, left, right and the four
corners**: the 2005 game's `DZ_FRONT` ... `DZ_RREAR` without roof and underbody. The record `Vehicle_Gameplay.DamageStats`
(three instances, each holding the number 126.6 and byte-sized values from 15 to 100) and `WheelDamage` (one per wheel:
-15, 1, 1, 0.25, 30, 7) are present; what the numbers are is **not decoded**. A `Skeleton` and an animation list drive the hinge animations (the list is [decoded](chameleon-damage.md)).

So the 2012 game keeps no impact points as geometry: where a hit lands is decided by zone at run time, as the 2005 game does
(a contact point against the half dimensions of the body, [vehicle-damage.md](../specs/vehicle-damage.md)), and a part comes
loose on its hinge when the zone's damage passes a threshold of its ladder. The converted cars therefore take **zones from their own root box**
(it is the extent of the collision mesh) and the damage numbers of [car-collision.md](../modding/car-collision.md#damage).

## Using it

The mod fits the 2005 bounds set to the soup's vertices and triangle centres plus the eight corners of each tyre's box
(`mods/nfsmw-2012-content/src/collision.rs`); the one car without a soup is fitted to its visible body mesh. Measured on
the 93 cars: root half width 0.85 to 1.10 m (1.66 for the SWAT van with its mirrors), half height 0.52 to 0.93 m, half length
1.57 to 2.79 m. The visible body sticks out of the root box by 0.00 to about 0.22 m on the 92 cars with a soup.

## Not read, not verified

- Whether the soup's 4 data bytes per triangle are a surface type; the 5 link bytes; the second table of per-chunk boxes.
- The field layout of the `Genesys` physics, damage and wheel objects (their type descriptions name hashed fields, not strings);
  everything marked unconfirmed above.
- How the 2012 executable consumes soup and hulls (a function that registers `PolySoupList` at `0x00a1f300`, and the `PolySoup`
  collision batch string at `0x01033a5c` with its debug-draw consumer, were found; the narrow phase was not read).
- Whether the `_LO` files hold a different collision shape from the `_HI` ones (not compared). Each of the 11 `_TR` traffic bundles has one poly soup of 186 to 268 triangles, which the content mod fits like a player car's ([chameleon-traffic.md](chameleon-traffic.md)).
