# The colour cubes of the 2012 world

What the 2012 game's `HAWAII/ENVIRONMENT.BNDL` and `POSTFX.BNDL` hold for colour grading, how the cubes are chosen through the
day, and which cube is which. Evidence tags as in [../README.md](../README.md): **[verified]** read from the install and checked,
**[exe]** from the executable's names, **[unconfirmed]** a reading. Reader: `libs/chameleon-postfx` (`environment.rs` for the
timelines, `lib.rs` for the `POSTFX` states). The host that draws them: [../specs/colour-grading.md](../specs/colour-grading.md).
Conversion: `mods/nfsmw-2012-content/src/grading.rs`.

## The cube resource (type `0x52`) **[verified]**

A cube is one block of 12,304 bytes: a 16-byte header, then 4,096 entries of 3 bytes (a cube of 16 entries a side). The header is the
edge as a 32-bit little endian number twice (`10 00 00 00 10 00 00 00`), then 8 zero bytes. All 21 cubes of the install (13 in
`ENVIRONMENT`, 8 in `POSTFX`) are 16 cubed; no cube of 32 exists, the "16 to 32" of the inventory is the host's limit, not the
data's.

**Order of the entries.** Entry `i = x + 16 y + 256 z`. In the identity cube (the `POSTFX` overdrive cube `0x78A0`, the default
state's `0x78F1` and the `ENVIRONMENT` night cube `0x3E93` are exactly it) the three bytes of the entry are `(17 z, 17 y, 17 x)`:
the byte that varies fastest is the third. The reader takes **x as the red input, y as green, z as blue, and the entry's bytes as
blue, green, red** **[unconfirmed]**. The other reading (x blue, bytes red, green, blue) gives the same identity; it differs from
the first by swapping red and blue in the input and the output. Evidence for the first, none of it from the executable:

- the sunrise, morning, evening and sunset cubes turn grey into a **warm** ramp (evening: 195, 194, 187 at the middle, 249, 245, 238
  near white) and noon's shadows are cool (49, 55, 61): warm highlights, cool shadows. The other reading turns the evening grey
  blue;
- a pure red input stays red (noon: 240, 47, 47) and a pure blue stays blue; the other reading keeps them too, so this only shows
  the cube is a grade and not a channel swap;
- the `POSTFX` heat cube comes out orange and the cooldown cube blue ([chameleon-nitrous.md](chameleon-nitrous.md)).

The sampling position (entry centres at `(i + 0.5) / 16`, so the first entry is hit at 0 and the last at 1) is the host's choice;
the executable's sampler was not read **[unconfirmed]**.

## The environment objects **[verified]**

`ENVIRONMENT.BNDL` is 5 MB, zlib blocks, and 290 resources: 13 cubes, 175 `Genesys.Gen.EnvironmentKeyframe`, 25
`Genesys.Gen.EnvironmentTimeline`, one `EnvironmentSettings`, 19 `LayerFog`, the textures and the Genesys types. The type names are
in the `GenesysType` blocks; the field names are hashed.

**Timeline.** `+0x10` is the offset of its keys (0x18), `+0x14` their count; a key is 28 bytes, its keyframe the import at `+0xC` of
the key and its hour a float at `+0x18` (the unit, hours of the day from 0 to 23, is a reading). 24 timelines have 21 or 22 keys at
the hours 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 17, 18, 19, 20, 21, 22, 23; one has 8 keys (5, 7, 11, 13, 15, 17, 20, 23).
Which timeline the game uses where (the districts, an event, the weather) is **not found**: nothing in the other 2,000 bundles imports
one (a scan of the `GenesysObject` imports of every bundle for six of the timelines), so the executable or a table it reads chooses.

**Keyframe** (1,216 or 1,232 bytes, 10 sub-structures of lighting, fog, sky, clouds, camera, vfx, depth of field, heat haze and
weather). Two imports matter here: the **colour cube at `+0x14`**, and at **`+0xC` another keyframe of the same type, the wet one**.
Of the 175 keyframes, 154 name a wet keyframe; the target is one of **7 shared wet keyframes** (1,216 bytes, with a cube and no
link of their own), so each wet look is one object. The other 21 keyframes (the 7 wet ones among them) have no link. The lighting
values of the wet keyframes were not compared.

## The 13 cubes **[verified]**; the roles **[unconfirmed]**

Used by the first timeline (`0x69CBB`, the lowest id and the first in the file) at the hours shown. Grey is the red, green, blue
the cube gives to the input grey at 4, 8 and 12 of 15.

| Cube | Role (reading) | Hours in `0x69CBB` | Grey at 4 / 8 / 12 of 15 |
|---|---|---|---|
| `0x003E93` | night | 0 to 3 | identity: 68, 136, 204 |
| `0x124514` | before dawn | 4 | 63, 64, 66 / 168, 174, 175 / 229, 235, 235 (cool, lifted) |
| `0x0F0864` | dawn | 5, 6 | 53, 54, 54 / 182, 180, 175 / 235, 232, 227 |
| `0x1244FC` | morning | 7 to 10 | 55, 58, 57 / 184, 182, 175 / 241, 235, 228 |
| `0x04E220` | noon | 12 to 14 | 49, 55, 61 / 165, 173, 173 / 236, 236, 231 (cool shadows) |
| `0x0F0862` | evening, night fall | 17 to 23 | 61, 65, 64 / 195, 194, 187 / 249, 245, 238 (warm) |
| `0x003E9E` | **wet** night, before dawn | the wet twin of `0x3E93` and `0x124514` | 62, 63, 65 / 174, 181, 182 / 233, 240, 240 |
| `0x09C4D5` | **wet** dawn | twin of `0xF0864` | 55, 58, 57 / 187, 185, 178 / 243, 238, 231 |
| `0x069F27` | **wet** morning | twin of `0x1244FC` | 55, 57, 56 / 181, 179, 172 / 239, 233, 226 |
| `0x09C4D4` | **wet** noon | twin of `0x4E220` | 48, 56, 64 / 171, 182, 182 / 246, 245, 240 (the coolest) |
| `0x124652` | **wet** evening | twin of `0xF0862` | 60, 64, 63 / 197, 195, 187 / 250, 246, 239 |
| `0x1244F2` | before dawn, another | hour 4 of three other timelines | 71, 68, 62 / 167, 164, 166 / 228, 229, 230 |
| `0x1243FB` | flat | the 8-key timeline `0x9EFCA`, all day | 69, 69, 69 / 161, 161, 161 / 236, 236, 236 (neutral S curve) |

The cubes are mild: they lift the shadows towards teal, warm the highlights and keep white at 253 to 255. The night cube does
nothing; the flat one is a neutral S curve.

**Dry and wet.** The twin pairs come from the keyframe link at `+0xC`: every dry cube of the table names the same wet cube
whichever timeline it is in (night and before dawn: `0x3E9E`; dawn: `0x9C4D5`; morning: `0x69F27`; noon: `0x9C4D4`; evening:
`0x124652`). Reading them as the rain look is a reading: they are the cooler, slightly lifted version of each dry cube, and the
game has a wet world (the rain, the road), but the executable's use of the link was not read **[unconfirmed]**. The keyframes of the
8-key timeline name a wet cube of the evening kind (`0xF0862`), which does not fit a rain look; that timeline is left unread.

**What the conversion makes of it.** The clear set is the dry cube of every key of timeline `0x69CBB`; the rain set is the wet cube
of each of the same keys. The game blends two keys by the time between their hours and the two sets by how wet the weather is
([../specs/colour-grading.md](../specs/colour-grading.md), "Selecting the grade"). The other 24 timelines were not converted: they
repeat the same cubes at other hours (the sunset and before-dawn variants) and differ in what the keyframes light, which the host
does not take.

## `POSTFX.BNDL`

The 8 cubes of `POSTFX` (the nitrous `0x789F`, the overdrive and default identities, the heat, cooldown, wreck, escape cubes) are
read by `ColourCube::grade` as a tint and a contrast for the nitrous screen ([chameleon-nitrous.md](chameleon-nitrous.md)); with a
cube pass in the host they could be played as cubes (`0x789F` for the nitrous). The nitrous variant still uses the fitted grade:
the pass is the world's, and the nitrous screen is a per-frame fade of the scene's own `FocusFilter`.

## Not decoded

- the executable's choice of timeline, and how it blends the keyframes (by hour, as a straight line, is the host's reading);
- the use of the wet link (a blend by the weather's wetness is the host's reading);
- the cubes' sample position and the first-entry order beyond the evidence above (a check in `NFS13.exe`, where the cube is
  uploaded as a volume texture, would settle the byte order);
- the lighting values of the keyframes (sun, fog, sky, clouds), which the host's time of day computes itself.
