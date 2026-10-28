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
| `_LO` | the same car for the distance: a body of five or six levels, among them a mid-detail one of 4,800 to 11,400 triangles that `_HI` lacks ([chameleon-traffic.md](chameleon-traffic.md#low-detail-bundles-_lo)) |
| `_TR` | the 11 traffic cars: body at five levels, wheels, lamps, collision soup, no stats, paints or driver ([chameleon-traffic.md](chameleon-traffic.md)) |

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

Every car but the BMW 1M shows a driver: a seated person, one of three shared characters (two civilians and the police officer), placed by a small object in the car's bundle. The
figures are not in the car bundles (only one bundle bakes a copy of the officer into its body, see the end of the section).

**The setup object.** Each of the 93 `VEH_*_HI` bundles holds one Genesys object with a structure of type
`Genesys.Gen.Vehicle_DriverSetup` (its field `SeatBeltBoneOffset` is a second type, one per belt bone) **[verified]**. Offsets
from the start of the structure; the field names are hashed, so these are by position:

| Offset | Type | Field |
|---|---|---|
| 0x10 | f32 x 3 | the seat: where the character's origin goes in the car's space (x left, y up, z forward), metres. (19, 19, 19) means no driver (only the BMW 1M, 2196065, whose body is Lightning McQueen's) |
| 0x24 | u32 | the key of the `CharacterSpec` the setup names: 0x3D8E in 61 bundles, 0x17785D in 32. **The five police bundles name 0x3D8E too, but the game seats the officer in them** ([the officer](#the-officer)) |
| 0x28 | u32 | the object's own key |
| 0x2C | u32 | the id of a **locator group** of the car's spec (one locator: the gear lever); 0 in 28 cars |
| 0x30 | u32 | the id of a locator group of the car's spec with **two locators: the left and right hand grips on the wheel** (the middle of the two is within 20 cm of the `SteeringWheel` bone and they are 31 to 50 cm apart in all 91 cars that have them) **[verified]** |
| 0x50 | u32 | the low word of the id of one of the clips 20 to 27 of the character's animation list: the pose of the hands and feet for this car's wheel **[verified]**, see [chameleon-driver-animation.md](chameleon-driver-animation.md) |
| 0x34, 0x38 | f32 | 0.05 and 0.095 (0x38 is 0.05 in three cars and 0.18 in the BAC Mono) **[unconfirmed]** |
| 0x3C | u32 | offset of the belt records (0x60) |
| 0x42 | u16 | the number of belt bones: 3 with character 0x3D8E, 4 with 0x17785D **[verified]** (the 4 are the racing harness: `HarnessTopRight`) |

The belt records (0x30 bytes each from 0x60) hold an offset (+0x10) and three angles in degrees (+0x20) per belt bone, zero for
most; they set the seat belt or harness up on the character's bones **[unconfirmed]**. In the 89 cars that have a steering wheel
bone and a seat, the seat's x is the bone's x to within 7 cm (0.00 in the BAC Mono, which seats in the middle, negative in the
right-hand-drive DB5 and R34) except in the Lancia Delta Integrale, whose bone is at +0.34 and whose seat at -0.34; its z is
0.21 to 0.51 m behind the bone; its y is -0.09 (the SWAT van's is +0.27) to -0.80 (the character's origin is at its seat, its
hips at +0.96).

**The characters.** `CHARACTERS/ALL_CHARS.BNDL` (the only file of the folder; the executable names it, `CharacterSpecResourceType`,
`HCCharacterSpec`) holds three `CharacterSpec` resources (type 0x209, class 1 ids 0x3D8D, 0x3D8E and 0x17785D) that share one
skeleton and one animation list. A spec is 96 bytes with three imports: the skeleton (+0x04), the animation list (+0x08) and the
model (+0x20) **[verified]**. The models have five levels of detail (switch distances 0, 200, 400, 600, 800, 1000 m, as stored
squared):

| Key | Model | Vertices per level | Diffuse (BC1) |
|---|---|---|---|
| 0x3D8E | 0xE6DEF161 | 4,458; 2,159; 1,341; 860; 433 | 0x177881, 2048 x 1024 |
| 0x17785D | 0xEE0B3878 | 4,632; 2,495; 1,347; 916; 472 | 0x17786F, 2048 x 1024 |
| 0x3D8D | 0xAB2342BC | 8,155; 2,919; 1,592; 1,184; 575 | 0x177878, 1024 x 1024: **the police officer** (no setup names it, [below](#the-officer)) |

One mesh per level, material technique `CharacterNew_Opaque_Textured_Normal_Spec_VertexAO` (samplers normal, diffuse, specular
in that order, so the diffuse is material texture 1; the normal map is 2048 x 2048, the specular 256 x 256), skinned
(`gBoneMatrices4Weights`) **[verified]**. `chameleon_vehicle::load_driver` reads one with the car loader's own model, material
and vertex readers. The positions are the **rest pose, which is the seated pose**: x within 0.27 of the middle, y 0.78 to 1.84,
z -0.44 (the back) to 1.00 (the feet) **[verified]**, with the hands out on the wheel at z 0.5. The skeleton has 46 bones, among
them `SeatbeltBottomRight`, `SeatbeltBottomLeft`, `SeatbeltTop`, `HarnessTopRight` (the executable spells these four), `Neck`,
`Head`, `LeftHand`, `RightHand` and `Hips` (found by their hashes). The animation list (201,584 bytes) holds 28 clips, decoded
in [chameleon-driver-animation.md](chameleon-driver-animation.md) (the skeleton's 46 bones are named there).

**Placement.** The car's driver is the character's rest-pose mesh moved by the seat, with no rotation and no scale
(`Driver::seated`). The geometry agrees **[verified]**: over the 92 cars with a seat, the head is under the highest point of the
body over the seat (the roof; an open car's windscreen or roll hoop) by 0.00 to 0.37 m in 81 cars, and above it by 0.03 to 0.32 m
in the other 11 (the open Cobra and BAC Mono by 0.27 and 0.32, the rest by 0.03 to 0.06); the soles are -0.02 to 1.05 m above the
ground (1.05 is a tall van, the Atom's are at 0.04); and nothing pokes out of the body sideways or lengthways. The test
`every_car_seats_a_character_inside_its_body` asserts the head under the body's top, the soles above the ground, and the
figure's box in the body's, for every car. That the game uses the seat as it is (no per-car rotation or scale) is
**[unconfirmed]**. The game animates the figure: it scrubs a steering clip and reaches the hands to the grips above
([chameleon-driver-animation.md](chameleon-driver-animation.md) **[exe]**). Nothing of the game was run.

**The officer.** The third character is a police officer (the diffuse map has two police badges, a tie and a dark uniform;
the other two wear jeans) **[verified]**. No setup and no other object of the install names its key, and it is not a setup
object's choice: the game picks it from a flag on the car's vehicle object. That object (a `Genesys.Gen.VehicleComponent`
structure) holds the handle of the driver setup at +0x2C, and a byte at **+0x8E** that is 1 in the five police bundles (the
same five that `police_marks` finds) and 0 in the other 88 **[verified]**; its word at +0x88 is 1 in every civilian car and 2 to 6 in
the police cars (SWAT van 2, Corvette 4, Explorer 5, Crown Victoria and Charger 6; read as a class or tier,
**[unconfirmed]**). The executable's choice reads the byte first **[exe]** (`NFS13.exe` `FUN_0059ea70`: byte set gives key
0x3D8D; clear gives the key stored in the setup at +0x24, and 0x3D8E if that is 0; its caller `FUN_00593ac0`, which sets a car up
for drawing, queues the key, and `FUN_00531cf0` asks the character bundle for the three keys). So every police car, the SWAT van too, has the one
officer: no car-by-car or class-by-class choice exists in the data. The setup keeps its own key (`named_character`),
the belt count (3: the belt goes with the officer) and the seat, grip group and pose clip of the car, so the officer is placed,
posed and reaches the wheel exactly as a civilian: the three characters import **the same skeleton (0x1777E6, 46 bones) and the same
animation list (0x1777E8, 28 clips)**, and the officer's skin names bones 0 to 45; the rest pose reproduces its mesh to 1 mm. The officer is
3 cm shorter than the civilians (head at y 1.803 against 1.835) and has the most detailed face (8,155 vertices at the finest
level). `driver_setup` applies the flag: `DriverSetup::character` is the key the game seats (0x3D8D, `COP_CHARACTER`, for a
police car), `named_character` the setup's own, `cop` the flag. The test `every_car_seats_a_character_inside_its_body` checks the
five.

**The baked driver.** One bundle, the police Explorer (`Ford_ExplorerInterceptor_2011`, 510072), also draws a 362-vertex
(389-triangle, finest level only) seated figure as a body mesh whose material technique ends in `Driver`
(`VehicleNFS13_Body_Driver`; a 4x4-block colour swatch of 8 x 8 pixels as its texture). It is the same posture as the seated
officer to within 0.2 m on every side of its box, and nearer to the officer than to the 0x3D8E civilian (its left edge is at x
0.2008 against the officer's 0.2009 and the civilian's 0.1908; its head at y 1.549 against 1.559 and 1.591) **[verified]**, so it is
a decimated baked copy of the officer, probably for the AI police car, which needs no animated figure **[unconfirmed]**. `remove_baked_driver` takes it out of the body so the car does
not draw two. The earlier reading of the format took this one figure for the only driver of the game and moved it to the other
cars by their steering wheel bone; that was wrong.

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

Animations of the car (steering wheel, the damage animations; the driver's are read), most damage data (the hinged parts and the skeleton are read:
[chameleon-hinged-parts.md](chameleon-hinged-parts.md)), the convex hulls and the physics body objects (the collision mesh is
read: [chameleon-collision.md](chameleon-collision.md)), vehicle sound, the engine and handling objects (the stats are read),
the effect objects the spot effects play ([chameleon-exhaust.md](chameleon-exhaust.md)); the lamp and exhaust locators are
read ([chameleon-lamps.md](chameleon-lamps.md)). Also not read: the rear-view
mirror and cockpit, and the low-detail bundles.
