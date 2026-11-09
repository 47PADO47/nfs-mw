# Chameleon environment: the sky, the clouds and the moon

What `HAWAII/ENVIRONMENT.BNDL` of the NFS: Most Wanted (2012) install (v1.4.0.0, 4,989,442 bytes, zlib blocks) holds about the sky, and how
the sky variant of the 2005 game uses it ([sky-variants.md](../modding/sky-variants.md)). Reader:
[`libs/chameleon-environment`](../../libs/chameleon-environment). Container: [chameleon-bundle.md](chameleon-bundle.md), Genesys
objects: [chameleon-vehicle-stats.md](chameleon-vehicle-stats.md#genesys-types-and-objects-verified). Evidence tags as in
[README.md](../README.md): **[verified]** is a number or a link read from the file and checked by a test with the install,
**[looked at]** was read from the pictures, **[unconfirmed]** is a reading.

## The bundle **[verified]**

290 resources: 27 textures, 220 `GenesysObject`, 26 `GenesysType`, 13 colour cubes (the grading, not read here), 2 world paint maps, one
`Material` (the moon) and one sampler state. Resource names are hashes; the `GenesysType` resources spell the names of their types
(`Genesys.Gen.EnvironmentTimeline`, ...), which is how the reader finds a structure inside an object: by the **import** whose type is
the named one, never by a hash.

## Timelines and keyframes **[verified]**

| Object | Type | Holds |
|---|---|---|
| `EnvironmentSettings` (`0x69CAD`, 608 bytes) | `Genesys.Gen.EnvironmentSettings` | the sun and moon path, the day-night transition, the rear mirror and rendering settings, the handle of the moon material (`0x3EE1`) |
| 25 timelines | `Genesys.Gen.EnvironmentTimeline` | a list of keys: `{ keyframe handle at +0x0C, hour (f32) at +0x18 }`, 0x1C bytes each; the array offset is the `u32` at `+0x10` of the object and the count the one at `+0x14` |
| 175 keyframes (21 of 1,216 and 154 of 1,232 bytes) | `Genesys.Gen.EnvironmentKeyframe` | two handles (a Genesys object and a colour cube) and nine structures, one per type below |

The nine structures of a keyframe, named by their types: `Camera` (160 bytes), `Clouds` (96), `Fog` (128), `HeatHaze` (48), `LightRig`
(144), `Mini_DOF` (28), `Sky` (144), `Vfx` (36) and `Weather` (44). Each starts with two zero words and the type hash; the offset of
each in the object is the slot of the import whose id is its type.

The timeline's hours and the dome they name: **[verified]** by `the_installed_timelines_name_the_dome_slots_of_the_day`.

| Hours | Dome (texture key) | Timelines |
|---|---|---|
| 0 to 4 | `0x3F34` | 24 of 25 (the keys at 0 to 3 are one keyframe; 4 is the dawn keyframe) |
| 5 to 10 | `0x3F3C` | 24 of 25 |
| 12 to 14 | `0x3F46` | 23 of 25 (not `0x124583`, `0x9EFCA`) |
| 17 to 20 | `0x3F40` | 23 of 25 |
| 21 to 23 | `0x3F3E` | 24 of 25 |
| 12 to 20 | `0x3F3A` | one timeline (`0x124583`), the bright cumulus |
| 5 to 23 (8 keys) | `0x3F32` | one timeline (`0x9EFCA`) |
| (an extra key at 15, the same dome as at 14) | `0x3F46` | one of the 23, `0x1247A0` (22 keys) |

What chooses a timeline (weather, district, mode) is not in the file: the objects that name them are in other bundles. **The variant
reads `0x69CBB` (the first of the bundle) as the cloudy day and `0x124583` as the fair one** **[our choice]**.

### The `Clouds` structure

Offsets from the structure's start (its import slot). **[verified]** for what the values are, **[unconfirmed]** for what the engine
does with them:

| Offset | Type | Meaning |
|---|---|---|
| `+0x10` | 4 x f32 | the first colour (linear RGB, 1): 0 at night, 0.62 / 0.50 / 0.43 at 4 h, 1.74 grey at 7 to 10 h, 1.65 / 1.82 / 1.93 at noon (bluish), 0.94 / 0.84 / 0.80 at 17 to 20 h, 0.14 at 21 to 23 h. Read as the colour of the sunlit clouds |
| `+0x20` | 4 x f32 | the second colour: 0.02 at night, 0.25 / 0.32 / 0.38 at 4 h, 0.77 at 7 to 10 h, 1.72 at noon, 0.52 / 0.61 / 0.80 at 17 to 20 h, 0.47 in the evening. Read as the colour of the clouds in shade |
| `+0x30` | `CgsCore.UniqueId` | the key of the dome texture (class 1) |
| `+0x34` | `CgsCore.UniqueId` | the structure's own id |
| `+0x38` | 7 x f32 | 0.5 or 1.0, 0.186 or 0.21, 0.569 or 0.193, 5 or 3.5, 0 to 1, 0.3 to 0.65, 0: meaning not decoded |

The other eight structures were not decoded (their numbers are in the file; `Weather` holds six: 0.23 / 0.15 / 0 / 130 / 25 / 3.5 at
night and 0 / 0.25 / 0 / 130 / 25 / 3.5 by day).

The engine's cloud shader was not read. **The converter lights a dome with the mean of the two colours** (`Clouds::tint`) **[our
choice]**: it looks right on every dome (day, dusk, night) and is the simplest rule that is neither the sunlit nor the shaded colour
alone.

## The textures **[verified]** (formats) **[looked at]** (what they show)

| Key | Size, format | What it is |
|---|---|---|
| `0x3F31` | 256 x 256 BC1 | tileable cloud noise (white and black blotches); not used |
| `0x3F32`, `0x3F34` | 2048 x 256 BC3 | cloud domes of low clouds at night |
| `0x3F38` | 1024 x 512 BC3 | a thick storm mass, brown and dark; named by no timeline; not used |
| `0x3F3A` | 2048 x 512 BC3 | bright cumulus, white on blue: the fair midday |
| `0x3F3C` | 2048 x 512 BC3 | wisps with the sun painted at the horizon in the middle (u 0.499): the morning |
| `0x3F3E` | 2048 x 512 BC3 | dark pink and grey clouds after sunset: the evening |
| `0x3F40` | 2048 x 512 BC3 | the afternoon: warm sky glow and dark clouds |
| `0x3F46` | 2048 x 512 BC3 | a thin grey veil over the whole sky: the noon |
| `0x3F33`, `0x3F3B`, `0x3F3D`, `0x3F3F`, `0x3F41`, `0x3F47` | 256 x 64 BC3 | the same pictures as the domes at 1/8 size (the key of a dome plus one); not used |
| `0x3F35`, `0x3F39` | 256 x 64 RGBA16F | sky colour gradients, a glow at the horizon in the middle: dusk and dark (`0x3F35`, up to 0.96) and day with the sun (`0x3F39`, up to 2.13); not used |
| `0x3F4A` | 64 x 64 BC3, 7 mips | the moon: a lit disc about 3/4 of the width, alpha 1 inside it, 0 outside. The `Material` `0x3EE1` imports it |
| `0x3F36`, `0x3F37`, `0x3F42` to `0x3F45`, `0x3F48`, `0x3F49`, `0x3F4B` | 8 x 8 or 4 x 4 | placeholders |

A dome is an **equirectangular picture of the upper hemisphere**: the full width is 360 degrees, the top row the zenith and the bottom
row the horizon (clouds pile up at the bottom, the bottom rows of day domes are opaque haze; the sun of `0x3F3C` sits on the bottom
rows). The aspect is 4:1 for the 2048 x 512 pictures and 8:1 for the 2048 x 256 ones, which are stretched over the same sky.
The colour is **not premultiplied** by the alpha (the pixels where the alpha is 0 hold the sky behind) **[looked at]**.

The converter reads no star picture: the 2012 install's sky has none in this bundle (the night stars of the variant are the game's
procedural ones). The sun glare of the 2012 game is not in this bundle either.

## What the variant does with it

`libs/chameleon-environment` reads the timelines and the textures; `mods/nfsmw-2012-content/src/sky/` turns them into a profile
([sky-variants.md](../modding/sky-variants.md)): three weathers (fair: `0x124583`, cloudy: `0x69CBB`, overcast: `0x69CBB` with the
cloud cover multiplied by 1.8 and the light dimmed to 0.7 **[our choice]**), six domes (`night`, `morning`, `noon`, `afternoon`,
`evening`, `fair`), the moon. The suns painted in `0x3F3C` and `0x3F40` are at u 0.5 (`glow = 0.5`) **[looked at]**; the game turns the
dome so that the painted sun is under its own.

## Cost

The bundle is 5 MB; the conversion reads its table and the blocks of seven textures (about 4.5 MB of the compressed file) and writes six
domes (0.5 + 5 x 1.0 MB at the install's size, a quarter of that at `sky_size = 1024`) and the 64 x 64 moon: **5,638 KiB in 518 ms at 2048, 1,414 KiB in 476 ms at 1024** **[measured]** (`the_installed_sky_is_converted_to_a_profile_the_game_reads_with_every_picture`). Runtime: see
[sky-variants.md](../modding/sky-variants.md#cost).

## Not decoded

The sun and moon path (`SunMoonPath`, ten floats), the fog, the light rigs, the camera, heat haze and weather structures of a
keyframe; which timeline the game picks for which weather; the colour cubes (the grading: another page); the engine's cloud shader.
