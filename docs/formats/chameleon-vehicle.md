# Chameleon vehicle bundles

What a car is made of in NFS: Most Wanted (2012), and how `libs/chameleon-vehicle` assembles it. Containers and
graphics resources are in [chameleon-bundle.md](chameleon-bundle.md) and [chameleon-gfx.md](chameleon-gfx.md). All of it
**[verified]** on the install by loading the 93 `VEH_<n>_HI.BNDL` files, unless tagged.

## One car

Every `VEH_<n>_HI.BNDL` has exactly one `VehicleGraphicsSpec` (type 0x106), and 4 to 11 models: the body and, for
each wheel, the tyre, brake disc, rim and caliper. The engine reaches them only through the spec. Ids in the file
names (`122672`, `1085007`, ...) are the car's number in `VEHICLELIST.BNDL`.

| Bundle | Detail |
|---|---|
| `_HI` | the body at three levels (the Porsche: 17, 10 and 1 meshes), wheel models at two or three; 5,000 to 36,000 triangles on the finest body level |
| `_LO` | the same car cut down for traffic and distance |
| `_TR` | traffic cars (not imported) |

Several numbers are the same car in other liveries: the Nissan Skyline R34 appears five times, the 350Z five, the
BMW M3 E46 GTR five, the Porsche 911 GT2 five, the Lamborghini Diablo SV five (one vehicle each, the vehicle list names the bundles; they differ in one texture, see [chameleon-customization.md](chameleon-customization.md#liveries)). One slot (`BMW1M12`, number 2196065)
holds Lightning McQueen from the *Cars* tie-in, with a 5,700-triangle body.

## Graphics spec (type 0x106)

Main block, 3,856 bytes for the Porsche. The imports are the skeleton (0xB2), a poly soup (0x60), the body model, the
wheel models and the brake textures.

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | offset of the body model's pointer slot (0x30) |
| 0x0C | u32 | offset of the wheel table (0x40) |
| 0x13 | u8 | number of wheels (4) |

A wheel record is 0x90 bytes:

| Offset | Type | Field |
|---|---|---|
| +0x00 | f32[3] | hub position in the car's frame, metres: x left, y up, z forward (0.768, 0.339, 1.239 for the Porsche's front left) |
| +0x78 | u32 | offset of four model pointer slots: tyre (radius 0.35), brake disc (0.30), rim (0.27, centred 0.1 outwards) and caliper (0.17) |
| +0x82 | char[] | the name: `FrontLeft`, `FrontRight`, `RearLeft`, `RearRight` |

The left and right wheels import the same four models. The models are authored as a **left** wheel (the rim face
towards +x); the right wheel is the mirror image, applied at draw time **[unconfirmed: the mirroring itself]**. The
hubs sit at the height of the tyre radius, so the car's origin is on the ground.

The spec also holds the table of **locator groups** (lamp, tail pipe and other placements, from 0x4A0 in the Porsche) and a
table of behaviour records at 0x280; the lamps are in [chameleon-lamps.md](chameleon-lamps.md).

## The car's name

The bundle has no name table. The car's name is in the Genesys objects (`GenesysObject`, type 0x15), spelled
`Make_Model_Year` and followed by the object's role: `Porsche_911CarreraS_2012_SteeringWheel_Behaviour`,
`Porsche_911CarreraS_2012_DamageBehaviour`, `Porsche_911CarreraS_2012_SpoilerAnimation`. `vehicle_name` collects every
word of that shape from the objects and takes the most frequent. 91 of the 93 cars have a name with a year; two have
no object that carries one (`VEH_122706` and `VEH_122713`). A few cars have names without a year
(`AstonMartin_V12_Vantage`), which the reader also accepts when the name is followed by `Steering`, `Spoiler`,
`Damage`, `Behaviour`, `Passby` or `Headlights`.

## What a surface is

`chameleon-vehicle` reduces each mesh's shader technique to a `SurfaceKind`: `Paint` (`BodyPaint*`), `Glass`
(`Glass*`, `Refraction`), `Tyre`, `Wheel`, `Light` (`*Emissive*`), `Body` (other `Body*`) or `Other`. The diffuse
texture is the one the shader's `DiffuseTextureSampler` (or `ColourSampler`) takes; the colour is the material's
diffuse colour; `Alpha1bit` in the technique means the diffuse alpha cuts the surface out.

## Not read

Animations (steering wheel, spoiler, the damage animations), most damage data (the hinged parts and the skeleton are read:
[chameleon-hinged-parts.md](chameleon-hinged-parts.md)), the convex hulls and the physics
body objects (the collision mesh is read: [chameleon-collision.md](chameleon-collision.md)), vehicle sound, lamp and exhaust locators, the engine and handling objects, the rear-view
Skeleton and animations (steering wheel, spoiler, bonnet and mirror hinges), damage data, the convex hulls and the physics
body objects (the collision mesh is read: [chameleon-collision.md](chameleon-collision.md)), vehicle sound, the lamp and exhaust locators are read ([chameleon-lamps.md](chameleon-lamps.md), [chameleon-exhaust.md](chameleon-exhaust.md)), the engine and handling objects, the rear-view
mirror and cockpit, and the low-detail bundles.
