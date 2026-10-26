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
| 2 | set on the same code path, **[unconfirmed]** meaning |
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
| rear | 2 | 86 | a grey-white record, **[unconfirmed]** (the pattern of a reverse lamp, but no code path was traced) |
| rear | 3 | 6 | second-switch lamp |

The lights: front bits 6, 17 and 4 (three road lights), rear bits 7, 2 and 1 (the Porsche's rear light for bit 2 has
intensity 0.5, the others 1). Bit 17 is not set by `FUN_0057A5E0`: another writer exists.

There is **no record for the reverse lamps** in any of the 93 cars, no record with bit 9, and no turn signal record: the
indicators of the 2012 game are not locator driven **[verified absence]**.

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
and `TaillightColour` and has an `EmissiveTextureSampler`; in the Porsche the first colour constant is (0.34, 0, 0) (the
brake red) and the emissive map is a two-channel mask with the strips' pattern. `EmissiveFourChannel` names
`LightmappedLightsRed/Green/Blue/AlphaChannelColour`, in the Porsche (0.49, 0.008, 0), (4.16, 4.16, 4.16),
(0.07, 0.10, 0.12) and a fourth that is not a colour; slot order follows the names, so the white one is the blue channel
**[unconfirmed]**. The converter does not read these constants. It reads each piece's place instead.

## Not decoded

- the `Corona`, `Light_Spot` and `Light_Base` objects (flare and glow curves, textures, cone data), the `SpotEffect` records;
- the meaning of mode, of bits 2, 8, 9 and 17 and of the corona factors;
- the lightmap channels of `EmissiveFourChannel` and the emissive map of `Refraction`, and which circuit (head, brake, reverse,
  signal) each belongs to;
- how the game turns the indicators.

## Tests

`CHAMELEON_GAME_DIR=<2012 install> cargo test -p chameleon-vehicle lamps -- --ignored` reads every car's groups and lamp
records (93 cars, 981 coronas, 549 lights; 92 cars have a head lamp record at the front and a brake record at the rear) and
checks every record points at a group the car has. `MW2012_GAME_DIR=<2012 install> cargo test -p nfsmw-2012-content --test
lamps -- --ignored` converts the Porsche and checks its markers and lamp textures.
