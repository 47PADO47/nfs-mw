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

### Hubs and the body's arches

The spec's hubs, the stats' axles (the Genesys vehicle info, [chameleon-vehicle-stats.md](chameleon-vehicle-stats.md)) and the
arches of the body agree on all but four cars (8 of the 93 bundles) **[verified]**: with the tyre (85% of its radius, 80% of
its width) put at the axle, at most a few vertices of the finest body level are inside it, and where a scan along the car finds
a clear stretch (the arch) near the hub, its middle is within about 2 cm of the hub and the info's axle (Porsche 911: front
hub 1.240, info 1.245, arch 1.245; open-wheel cars have no arch to find). The four that disagree:

| Bundle | Spec hubs z (front, rear) | Info axles z | Body's arch z | What it is |
|---|---|---|---|---|
| `BMW_1M_2012` (2196065) | 1.420, -1.240 | 1.035, -0.925 | 1.03, -0.93 | the hubs, info and physics box (4.3 m) are the 1M's; the body is Lightning McQueen's (3.5 m), so the tyres at the hubs have 66 and 54 body vertices inside, at the info's axles none |
| `Hennessey_VenomGTSpyder_2012` | 0.125, -1.505 | 1.300, -1.505 | front: none at the hub, 4 at the info's | the front hubs are behind the doors; the rear is right |
| `BMW_M3E46GTR_2001` (5 bundles) | 1.503, -1.367 | 1.464, -1.266 | 1.47, rear clear from -1.35 to -1.15 | the rear hubs are 10 cm behind the arch; the front hub is inside its arch (clear from 1.39 to 1.55) |
| `Lancia_Delta...` | 1.239, -1.240 | 1.324, -1.038 | none clear (the body holds liner vertices at both) | undecided: the spec's hubs are kept (its wheelbase, 2.48 m, is the real car's) |

So where the info's axle is 3 cm or more from the hub, the body has more than 16 vertices inside the tyre at the hub and at
most 8 at the info's axle, the info's axle is the one the arch is at (`chameleon_vehicle::prefer_other`; the real-install test
`only_the_cars_with_hubs_outside_their_arches_move_an_axle` finds exactly four axles: the 1M's two, the Venom's front and the
E46 GTR's rear, and not the Lancia's). The tyre's height, width and track are from the spec's hub and the wheel mesh in all cases (the 1M's wheel
meshes are 0.327 m in radius and its body is 0.74 as tall as the real car: the McQueen body has the real car's wheels).
**[unconfirmed]** which the 2012 game draws for the 1M: nothing of its code was read.

## The driver

One bundle draws a person: the police Explorer (`Ford_ExplorerInterceptor_2011`, 510072) has a body mesh (362 vertices,
389 triangles, finest level only) whose material technique is `VehicleNFS13_Body_Driver`, a 4x4-block colour swatch (8 x 8
pixels, 4 mips: skin and dark grey clothes) as its texture. It is a seated figure facing forward, hands on the wheel, the
feet on the pedals; **no other of the 93 bundles has a material of that technique** **[verified]**, and `CHARACTERS/ALL_CHARS.BNDL`
holds the cut-scene people, not a figure in a car. The figure is modelled around its car's steering wheel bone (`SteeringWheel`,
the skeleton of [chameleon-hinged-parts.md](chameleon-hinged-parts.md); at (0.456, 1.055, 0.552) in the Explorer): the hands are at
the bone, the head 0.49 above it, the soles 0.61 below it. 90 bundles have the bone (x 0.25 to 0.46 on the left, -0.35 and -0.37 on
the right-hand-drive DB5 and R34, 0.00 on the BAC Mono and the F1 LM, which seat in the middle); the SWAT van, the Zonda R and
the Huayra have none. `chameleon_vehicle::driver` carries the figure to another car by its bone and fits it to the room the car
has (the highest vertex of the body over the seat, 0.1 to 0.8 m behind the wheel and 0.25 m either side: the roof, or an
open car's roll hoop): the seat goes down by at most 0.12 m and then the figure shrinks to at least 85% about the bone, until its
head is 5 cm under that height; and it shrinks, not sinks, when the soles would be under 0.1 m from the ground. Low cars
(the Countach, the Diablo, the Cobra) get the smaller figures, the Explorer none. Every converted car has the
figure as its driver solid ([the mod](../modding/nfsmw-2012-content.md#the-driver)). **[unconfirmed]**: that the seat of
a sports car is where the Explorer's is relative to the wheel (the data has no seat position), and the look: nothing was
drawn in a running game.

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
[chameleon-hinged-parts.md](chameleon-hinged-parts.md)), the convex hulls and the physics body objects (the collision mesh is
read: [chameleon-collision.md](chameleon-collision.md)), vehicle sound, the engine and handling objects (the stats are read),
the effect objects the spot effects play ([chameleon-exhaust.md](chameleon-exhaust.md)); the lamp and exhaust locators are
read ([chameleon-lamps.md](chameleon-lamps.md)). Also not read: the rear-view
mirror and cockpit, and the low-detail bundles.
