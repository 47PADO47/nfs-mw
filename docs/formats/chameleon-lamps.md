# Chameleon car lamps

What the 2012 car bundles hold for lamps and how the 2012 game drives them. Container and resource formats:
[chameleon-bundle.md](chameleon-bundle.md), [chameleon-gfx.md](chameleon-gfx.md), [chameleon-vehicle.md](chameleon-vehicle.md).
Reader: `libs/chameleon-vehicle/src/lamps.rs`. Used by [`mods/nfsmw-2012-content`](../../mods/nfsmw-2012-content), which turns
it into the lamp markers and lamp surfaces of [the mod car lamp contract](../modding/car-lamps.md).

Tags: **[verified]** is checked on the 93 `VEH_<n>_HI.BNDL` of the install (v1.4.0.0) with the reader or a script;
**[exe]** is from `NFS13.exe` (Ghidra; function addresses are given, no code is reproduced); **[unconfirmed]** is a reading
of the data that the executable was not traced for.

## Where the lamps are

A car has no lamp mesh with a name. Its lamps are three kinds of data:

| Data | Where | Is |
|---|---|---|
| **Locator groups** | the `VehicleGraphicsSpec` main block | placements (rotation and position) of lamps, tail pipes and other parts, numbered by a group id |
| **Lamp records** | one Genesys object of type `VehicleVfxBehaviour` | coronas (flares seen from outside), lights (what a lamp throws on the road) and spot effects, each pointing at a group id and a light-state bit |
| **Lamp surfaces** | the body model's meshes | lit strips (`Refraction` shader), the lamp interiors and reflectors (`Emissive*` shaders), lamp covers (`Glass_Colourise`) |

## Locator groups [verified]

The spec header names three tables (all offsets are in the spec's main block, 3,856 bytes for the Porsche):

| Offset | Type | Field |
|---|---|---|
| 0x14 | u32 | number of behaviour records (2 to 5) |
| 0x18 | u32 | offset of the behaviour table: 16-byte records `u32 object id (low bits), u32 type id (low bits), u32 hash, 0` |
| 0x1C | u32 | number of locator groups (14 to 35 in the 93 cars; the Porsche has 26) |
| 0x20 | u32 | offset of the group ids (`u32` each) |
| 0x24 | u32 | offset of the group entries (12 bytes each) |

A group entry is `u32 locator count, u32 offset of the locators, u32 (0 for all but two groups of the Porsche)`. A locator
is 32 bytes: a quaternion `x, y, z, w`, then a position `x, y, z` and a zero. Positions are in the engine's vehicle space
(x left, y up, z forward, metres, origin on the ground; the same space as the wheel hubs). The ids are the car's own object
numbers (`0x2181F`, `0x21835`, ...): they are not names, and only one id (`0x1401FC`, 62 of 93 cars) is shared across cars. The
bundle holds no text for them, so **a group's meaning comes from the records that point at it**, never from its id.

A locator's local +z axis is the way the part faces **[verified by position]**: the front lamps of the Porsche have
rotations near the identity (a few degrees about x), the rear ones are half a turn about y (`x, y, z, w = 0, 1, 0, 0`), and
the tail pipes half a turn about x. `Locator::forward` rotates (0, 0, 1) by the quaternion.

Porsche 911 Carrera S (`VEH_1085007_HI`), the groups the lamp records use, in engine space:

| Group | Locators | At |
|---|---|---|
| `0x21835` | 1 | (-0.69, 0.66, 2.26): the head lamp, front right; rotation 8 degrees about x |
| `0x21836` | 1 | (-0.69, 0.73, 1.72): a fender lamp (amber corona) |
| `0x2182E`, `0x2182F` | 1 each | (0, 0.72, 2.27): the front centre, where the road lights start |
| `0x21838`, `0x21839` | 1 each | (-0.63, 0.75, -2.01): the rear lamp, right |
| `0x21830` | 2 | (+-0.63, 0.75, -2.26): the rear road lights, rotated 130 degrees about x |
| `0x2181F` | 1 | (0, 0.96, -1.77): the centre high brake lamp |
| `0x21833` | 4 | (+-0.51, 0.27, -2.16), (+-0.41, 0.27, -2.19): the tail pipes ([chameleon-exhaust.md](chameleon-exhaust.md)) |

## The `VehicleVfxBehaviour` object [verified]

The object is named `<Make_Model>_Coronas` (a string at 0x30). Its structures are found through the type imports of its
import table ([chameleon-vehicle-stats.md](chameleon-vehicle-stats.md) describes the mechanism): 11 `Corona` records of
0x50 bytes from 0x50 (the Porsche), 6 `Light` records of 0x40 bytes after them, and 8 `SpotEffect` records of 0x20 bytes.
Over the 93 cars: 981 coronas (8 to 13 a car) and 549 lights (6 a car in most, 5 in a few, none in one). The field offsets
are the ones the type resources give (`Genesys.Gen.VehicleVfxBehaviour.Corona`, `.Light`, `.SpotEffect`: size, offset and a
hashed name per field, names not stored).

| Corona offset | Type | Field |
|---|---|---|
| +0x08 | u32 | hash of the type (`0x4A4E60CD`) |
| +0x10 | f32 x4 | colour factor: (1, 1, 1, 1) in most records; (1, 0.45, 0.13) for amber position lamps; dark blue-grey tints for xenon-looking lamps |
| +0x28 | 8 bytes | reference to a `Corona` object (flare and glow shapes, size and intensity curves, flare textures); the id is in the import table |
| +0x30 | u32 | the locator group id |
| +0x34, +0x38, +0x3C | f32 x3 | factors (1 to 2.4 seen), **[unconfirmed]** sizes |
| +0x40 | u16 | the **light-state bit** |
| +0x42 | u16 | mode: 0, 1 or 2 |
| +0x44 | u8 | 1 |

| Light offset | Type | Field |
|---|---|---|
| +0x10 | f32 x4 | colour factor (1, 1, 1, 1) |
| +0x28 | 8 bytes | reference to a `Light_Spot` object (cone angles, range, projected texture) |
| +0x30 | u32 | the locator group id |
| +0x34 | f32 | intensity factor (1; 0.5 and below for dim tail lights) |
| +0x38 | u16 | the **light-state bit** |
| +0x3A | u16 | mode (2 in about 90 percent of the lights, 0 or 1 in the others) |

A `SpotEffect` has a locator group id at +0x14, a state bit at +0x18 and a mirror flag at +0x1D; its reference at +0x0C is an
effect of `GLOBALEFFECTS.BNDL`. The ones with state bit 8 or 9 (nitrous) place the tail pipes:
[chameleon-exhaust.md](chameleon-exhaust.md).

### Mode **[unconfirmed]**

Mode 2 reads as the mirror image of the group's locators across the centre plane: the Porsche has two amber coronas at
one fender locator (-0.69), one with mode 1 and one with mode 2, that use the same two flare shapes, which is a left and a
right lamp. Groups that hold both sides (the rear road lights, +-0.63) have mode 2, which gives the same two places again. A
head lamp record with mode 2 and one locator would then give only the mirrored side, which does not fit the other
side's lamp having no record of its own in the Porsche; the data is not enough to say. The mod therefore takes each locator
and its mirror image, because the cars are symmetrical (`markers.rs`).

## The light-state word [exe]

The game keeps a 32-bit word of lamp states per vehicle (at +0x3C8 of the lamp controller) and builds it every frame in
`FUN_0057A5E0`:

| Bit | Set when |
|---|---|
| 0, 4, 6, 7 (`0xD1`) | the vehicle's headlights flag is on (byte +0x1A8C, the one `Player.GetHeadLightsOn` reads) |
| 1 | the second input float of the vehicle (+0x1930, the brake in its input block) is positive; also set together with bit 2 when the first (+0x192C, the throttle) is positive on some paths |
| 2 | set on the same code path (value 4, whenever that path's moving test passes, and as `6` with bit 1 when the first float is positive); **[unconfirmed]** as code, but the records that use it are the reverse lamps, see below |
| 3 | a second switch (byte +0x1C27) |
| 9 (`0x200`) | the gear byte (+0x195B) is 1, **[unconfirmed]**: the reverse gear |
| 8 | a flag of the same family (+0x1944 > 0 and the vehicle is the camera's, **[unconfirmed]**) |
| 18 | the vehicle is stopped (speed below 10 units) |

`FUN_005837E0` (the road lights) walks the vehicle's behaviours, picks the one of type `0x2AAB` (`VehicleVfxBehaviour`),
and for each `Light` record tests `state_word & (1 << record.bit)`; a lit record is projected as a spot from the locator
group's position. `FUN_00581640` collects the same records for the corona pass (`FUN_004970B0` queues them, at most 96 a
frame). So the bit of a record **is an index into this word** for lights **[exe]**, and the same field of the corona
records is read the same way **[unconfirmed]** (the pattern below fits it).

Bits of the records in 90 of the 93 cars (a script; the reader finds the other three too) **[verified]**; front is z > 0 of
the record's first locator:

| End | Bit | Coronas | Reading |
|---|---|---|---|
| front | 0 | 408 | head lamp, on with the headlights (`0xD1` includes 0) |
| front | 6 | 37 | position lamps; amber in 12 records (colour factor near (1, 0.45, 0.13)), white in the others (bit 6 is also in `0xD1`) |
| front | 3 | 2 | a second-switch lamp (fog) |
| rear | 1 | 230 | brake lamp |
| rear | 7 | 122 | tail lamp, on with the headlights |
| rear | 0 | 60 | tail lamp (on with the headlights) |
| rear | 2 | 125 coronas, 89 cars | a white or grey-white lamp: the **reverse lamp** (below) |
| rear | 3 | 6 | second-switch lamp |

The lights: front bits 6, 17 and 4 (three road lights), rear bits 7, 2 and 1 (the Porsche's rear light for bit 2 has
intensity 0.5, the others 1). Bit 17 is not set by `FUN_0057A5E0`: another writer exists.

### The reverse lamp is bit 2 **[verified by data]**

Every car but four (the Cobra 427, the Audi A1 Clubsport, the Huayra and one unnamed bundle) has rear coronas of bit 2, none
has one at the front, and none has a corona of bit 8 or 9. Their shape is one pattern: a white or light-grey tint (0.67 to
1), a size factor of 0.5 to 2 (100 in the 918 Spyder), mode 0, and a locator group that holds
- one lamp off the centre plane (54 cars: the 911 at (-0.63, 0.74, -2.00); the mod mirrors it, like every lamp),
- a left and a right lamp (32 cars: the Charger SRT8 at +-0.48 inside its tail cluster, the Corvettes low at +-0.50, height 0.33),
- or one lamp on the centre plane (3 cars): the **918 Spyder** (`VEH_2196000`) has a single lamp at (0, 0.31, -2.31), low at
  the rear face, turned about 10 degrees off the back axis. This is the white light seen at the bottom of the car in reverse.

The road lights agree: each car's rear `Light` record of bit 2 is the lamp the car throws backward and down while reversing
(the 918's group `0x21B4DB` points 49 degrees down, the Charger's has intensity 0.5), where the brake and tail records
(bits 1 and 7) share the group. The code path that sets bit 2 in `FUN_0057A5E0` was not traced to a gear test (the gear byte sets bit 9, which no lamp record uses), so the
reading rests on the data and on the 2012 game showing the 918's lamp in reverse.
The lit surfaces agree too: the blue channel of the lit pattern, whose triangles sit at the places of these records,
is the surface of the reverse lamp ([lamp circuits](#lamp-circuits-verified-by-data)); the 918's is a 30 cm bar of 15 LED
dots at the rear centre.

There is no turn signal record: the indicators of the 2012 game are not locator driven **[verified absence]**.

## Lamp surfaces

The Porsche body at the finest level (17 meshes, [verified] with `chameleon-vehicle`):

| Mesh | Shader technique | Is |
|---|---|---|
| 7 | `Refraction` (140 vertices, 8 flat pieces 1 to 2 cm thick) | the lit strips: a centre high brake bar, two front daytime strips, two rear strip pairs and a centre rear bar; each piece is one connected set of triangles |
| 10 | `Body_Textured_NormalMap_EmissiveFourChannel_NoDamage_NoEffects` (6,204 vertices over the whole car) | the lamp interiors, reflectors and grille parts with a lightmap whose four channels are four lamp circuits |
| 16 | `Body_Textured_NormalMap_Emissive_NoDamage_NoEffects` (diffuse map and a lights lightmap) | interior lights and the instrument lamps |
| 11 | `Glass_Colourise` (185 vertices, z -2.18 to -1.66) | the red rear lamp covers (a colour constant, no texture) |
| 5, 6, 15 | `Glass`, `Glass_Textured_Lightmap` | windows and the clear head lamp cover |

The `Refraction` shader names its colour constants `BrakeColour`, `EmissiveLuminance`, `HeadlightColour`, `ReversingColour`
and `TaillightColour` and has an `EmissiveTextureSampler`; `EmissiveFourChannel` names
`LightmappedLightsRed/Green/Blue/AlphaChannelColour` and has a `LightmapLightsTextureSampler`. Both read the same picture
in the same way: the **lit pattern** of [the lamp circuits](#lamp-circuits-verified-by-data) below. The converter reads the
pattern, not the colour constants.

## Lamp circuits [verified by data]

The lit pattern of a lamp material is a picture whose four channels are four lamp circuits, the same for both shaders
(a `Bc3` or `Bc1` picture of 256 to 1024 texels a side, read with the first texture coordinates):

| Channel | Circuit | Evidence |
|---|---|---|
| red | **brake** | lit texels sit on the rear lamps (**[unconfirmed]** against alpha: position cannot tell brake from tail); the red constant is the bright one (918: 0.43, 911: 909) and the `Refraction` shader's `BrakeColour` is the first of its four circuit colours |
| green | **head** | 13,140 of the ~13,800 lit triangles of `EmissiveFourChannel` over the 93 cars are within 0.5 m of a front record of bit 0 |
| blue | **reverse** | 824 of the ~1,000 lit triangles are within 0.5 m of a rear record of bit 2; in the `Refraction` lenses 436 of 564 |
| alpha | **tail** | rear lamps only, in dim areas (the 918 Spyder's alpha peaks at 45) and with the dim constant (918: 0.14, 911: 0.49); **[unconfirmed]** as the tail circuit, lit with the headlights (rear bits 7 and 0); a `Bc1` picture has no alpha channel |

The colour constants of `EmissiveFourChannel` sit in 16-byte slots in the alphabetical order of their names:
`LightmappedLightsAlphaChannelColour`, `Blue`, `Green`, `Red`, `PbrMaterialClearcoatFresnel`,
`PbrMaterialClearcoatSpecular`, `PbrMaterialDiffuseColour`, `mSelfIlluminationMultiplier` (8 slots, the last is 10 in the
Porsche cars). So the colour of a channel is the slot of its name, not the order red, green, blue, alpha: the Porsche 911's
red (brake) is (909, 5.7, 0), its blue (reverse) a white of 4.16, its green (head) a blue-white (0.07, 0.10, 0.12) and its
alpha (tail) a dim red (0.49, 0.008, 0); the 918 Spyder's are (0.43, 0, 0), (0.06, 0.08, 0.10), (0.09, 0.13, 0.14) and
(0.14, 0, 0). The values are HDR radiances the 2012 tone mapping brings down; they are not carried over. `Refraction`
orders its slots the same way (`BrakeColour`, `EmissiveLuminance`, `HeadlightColour`, `ParallaxConstants`, ...,
`ReversingColour`, `TaillightColour`, `mExternalGlassColour`), **[unconfirmed]** as the executable was not traced for it.
The game sets the state of the circuits each frame through `g_selfIlluminationMask`, a shader constant both shaders name; the
code that fills it was not traced, so the link from a circuit to a bit of the light-state word rests on the positions above.

### The 918 Spyder **[verified]**

The 918 has no `Refraction` mesh. Its lamps are in `EmissiveFourChannel` (mesh 6 of its finest level, 3,639 vertices and
3,420 triangles over the whole car, picture `0x0B22CE`), where the lit pattern shows dots and shapes on a black picture:

- red: two rows of 12 red LED dots, the rear brake lamps (bbox x +-0.77, y 0.70 to 0.78, z -2.16 to -1.95);
- green: two clusters of four dots, the front LED head lamps (x +-0.65 to 0.78, y 0.54 to 0.65, z 1.74);
- blue: one row of 15 dots, the **reverse bar**: 20 triangles at the rear centre, 0.30 m wide and 3 cm tall (x +-0.15,
  y 0.29 to 0.32, z -2.30), where the reverse record's locator sits (0, 0.31, -2.31);
- alpha: the dim areas of the tail lamp housings (x 0.42 to 0.85, y 0.64 to 0.75, z -2.15 to -1.87).

The number plate lamps are other meshes (`BodyPaint_Livery_Lightmap`, `Body_Textured_NormalMap_Lightmap` and `..._Licenseplate`)
with one grey picture (`0x0B22D7`) read with the fourth texture coordinate set (usage 8, index 3); they are not circuits of
the four channels and are not converted. The 2012 game draws the bar as 15 bright points with a large HDR multiplier and a
bloom, which looks like one lit bar; a 2005 flat texture colour on the same triangles is what the converter makes of it.

### How the converter cuts a lamp (`lamp_pieces.rs`)

A triangle is lit by the channel that is lit at most of the samples taken across it: at least every 2 texels along its
longest edge, at most 24 steps; a channel is lit above 35 percent of its peak in the picture (and above 48, alpha 16); a
triangle sampled 15 times or less is lit by one lit sample, a larger one by 3 percent of them. The picture is clamped at its
edge. A circuit lights the lamps at its own end only (head at the front, the others at the rear): a green texel on a rear
triangle is not read. Then:

- the `EmissiveFourChannel` triangles that a circuit lights become one surface per circuit and side
  (`LampSurface`: end, side by the triangle's place, circuit); the others stay the dark interior;
- a `Refraction` lens is cut into its connected pieces, and each piece again by the circuit of its triangles; a triangle the
  pattern does not light takes the main circuit of its piece, and a lens with no readable pattern takes the circuit of its
  end (head at the front, brake at the rear), as before.

Over the 93 cars the cut gives, as surfaces of the finest level: interior pieces of brake 149, head 154, reverse 42 and tail
122, and lens pieces of brake 213, head 248, reverse 124 and tail 127. The 918 Spyder gets brake, head, reverse (a centre
piece of 9 triangles and two of 4 at the corners of the bar) and tail pieces; the 911 gets brake and reverse lens pieces
at the rear corners (x +-0.45 to 0.74, y 0.72 to 0.75), head lens pieces at the front and interior pieces of all but reverse.

## Not decoded

- the `Corona`, `Light_Spot` and `Light_Base` objects (flare and glow curves, textures, cone data), the `SpotEffect` records;
- the meaning of mode, of bits 3, 8, 9 and 17 and of the corona factors; bit 2 as game code (it is the reverse lamp by data);
- the front bit 3 coronas (2 cars, fog lamps) and the rear ones (6), the amber position or turn lamps (bit 6, 12 records);
- the code that fills `g_selfIlluminationMask` (which bit drives which circuit), the `EmissiveLuminance` scale and the HDR
  multiplier, the single grey lightmaps (`Lightmap` techniques: number plate and some rear lamps) and the plain
  `Emissive` shaders (interior and dashboard lights);
- how the game turns the indicators.

## Tests

`CHAMELEON_GAME_DIR=<2012 install> cargo test -p chameleon-vehicle lamp -- --ignored` reads every car's groups and lamp
records (93 cars, 981 coronas, 549 lights; 92 cars have a head lamp record at the front and a brake record at the rear) and
checks every record points at a group the car has; it also loads every car, checks that the hinged-part classes line up with
the lamp pieces and that the 918 Spyder has its reverse bar (`lamp_pieces_tests.rs`). `MW2012_GAME_DIR=<2012 install> cargo test -p nfsmw-2012-content --test
lamps -- --ignored` converts the Porsche and checks its markers (head, brake, centre brake and reverse) and lamp textures.
