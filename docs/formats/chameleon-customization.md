# Chameleon customization: what NFS: Most Wanted (2012) offers for a car's looks

What the 2012 game stores per car for its looks (paint, liveries, rims, spoilers, vinyls, tint), where, and how it reads
it. Evidence tags are the repository's ([README.md](../README.md#evidence-tags)); **[verified]** here means checked on the
2012 install (v1.4.0.0, `NFS13.exe`) with the readers of this repository, **[code]** means read from the decompiled game
code in Ghidra (`NFS13.exe`), **[unconfirmed]** means a reading that fits the data and has not been proved.

## Short answer

Per car the 2012 game offers **a set of body paints** and, for five vehicles, **liveries**. It has **no vinyls, decals,
rims, spoilers, roof scoops, body kits, hoods or window tints to choose from**. A livery is a different vehicle list entry
(its own bundle) whose paint surface draws one picture, laid out for artwork, over the paint colour: the bundle is the same
car with that one texture replaced ([Liveries](#liveries)). Only the 36 bundles with a `_Livery` paint technique draw a
livery picture; the other bundles' paint surfaces take a flat colour (or, for the `TwoPaintMask` techniques, a mask).
Twelve vehicles also have **two-colour paint packs**: each pack is a pair of the car's paints, the second colour covering
the stripes, roofs and lower panels ([Two-colour paint](#two-colour-paint)).

| Item | In the 2012 game | Evidence |
|---|---|---|
| Body paint colours | 1 to 15 `VehiclePaint` records per car bundle (443 records in the 93 cars; 396 after dropping a colour and finish a car repeats; 90 distinct paint ids) | [verified] |
| Paint finishes | solid (`Type` 0, 214 records) and metallic (`Type` 1, 228 records); one record has a secondary (pearlescent) colour | [verified] counts, [unconfirmed] the names of the two types |
| Liveries | separate vehicle list entries with the same vehicle id and another `LiveryID`: 5 vehicles have 5 entries each (the car and 4 liveries); each has a bundle, which differs from the car's own in one texture (the paint picture) and the ids that name it; named `HERO` (the vehicle's own), `RACE`, `STREET`, `ABSTRACT`, `URBAN` by the vehicle's [paint pack](#paint-packs) | [verified] |
| Rims, tyres, brake discs, calipers | one model each per car, as the wheel entry of the graphics spec; no alternatives | [verified] |
| Spoiler | an animation of a part of the body (`<Car>_SpoilerAnimation`, driven by speed), not a choice | [verified] resource, [unconfirmed] which cars have it |
| Vinyls, decals, body kits, hoods, roof scoops, window tint | none: no resource, no UI binding, no string in `NFS13.exe` | [verified] searches, see below |
| Licence plate | the text is the player's (default by game pack, five strings in the executable), the design a global list (`LicensePlatesManager`); not per car. The body has a `Licenseplate` material and an eight-quad plate mesh: [chameleon-plate.md](chameleon-plate.md) | [verified] defaults, mesh and sheet; [code] rules |
| Performance mods | `PowerTrain`, `Nitrous`, `Body`, `Chassis`, `Tyre` upgrade lists per car (`CarUpgrade`); not looks | [code] |

## The paint record

The body-paint shaders (`VehicleNFS13_BodyPaint`, `VehicleNFS13_BodyPaint_Lightmap`) have **no colour in the material**: the
material holds two samplers (a crumple map and an effects map, both shared) and a lightmap, and the colour comes from
shader parameters the game sets at draw time: `PbrPaintDiffuseColour`, `PbrPaintSpecularColour`,
`PbrPaintSpecularSettings`, `PbrPaintClearcoatSettings`, `PbrPaintScratchColour`, `PbrPaintScratchSettings`,
`PbrPaintSecondaryDiffuseColour`, `PbrPaintSecondarySpecularColour` (names in the shader blocks and in `NFS13.exe`,
`g_paintColour`) **[verified]**. The plain paint surface therefore has no colour texture; its texture coordinates serve
the lightmap and the effects map. The `_Livery` techniques add one more sampler, `DiffuseTextureSampler`, for the livery
picture, and their coordinates are laid out for it ([Liveries](#liveries)).

The values come from a Genesys object of type `Genesys.Gen.VehiclePaint` **[verified]**. Every car bundle carries the
objects of the paints the car is offered in (the type resource spells the name; an object imports the type id). The main
block is 192 bytes:

| Offset | Type | Field | Evidence |
|---|---|---|---|
| 0x10 | f32 x4 | colour (a scratch or lacquer colour, 0.13, 0.11, 0.10 on most records) | [unconfirmed] meaning |
| 0x20 | f32 x4 | colour (0.69, 0.63, 0.47 on most records) | [unconfirmed] meaning |
| 0x30 | f32 x4 | **diffuse colour**, linear RGB, alpha 1: red 0.42, 0.02, 0.03 (sRGB 173, 43, 44) and the like | [verified] values; [code] read by the colour getters |
| 0x40 | f32 x4 | secondary (pearlescent) diffuse colour: zero on 442 of 443 records | [code] `GetPearlescentColour` reads it |
| 0x50 | f32 x4 | specular colour: 0.001 on 394 records, 0.279 on 29, 0.068 on 14 | [verified] |
| 0x60 | f32 x4 | second diffuse colour: near black on solid paints, a lighter flake colour on metallic ones | [verified] values; [code] read by the colour getters |
| 0x70 | u32 | id of the swatch image, `UI/IMAGES/STREAMED/<id>.BNDL` (the file name is the id in decimal) | [verified] |
| 0x74 | u32 | the paint's own id (the low bits of the resource id) | [verified] |
| 0x78 | u32 | `Name` (field name verified from the type; 0 on every record: the names are not here) | [verified] |
| 0x7C to 0x98 | f32 x8 | settings: 0.25, 0.034, 0, 0.002, 0.11, 0.25, 1.0, 0.57 on the first record | [unconfirmed] meaning |
| 0x9C | u16 | colour family, 0 to 7 | [unconfirmed] (0 black, 1 white or silver, 2 grey, 3 green, 4 red, 5 orange, 6 yellow or gold, 7 blue fits the records read) |
| 0x9E | u16 | `Type` (field name verified from the type): 0 solid, 1 metallic | [verified] values |
| 0xA0, 0xA1 | u8 | flags (1, 1 on the first record) | [unconfirmed] |

The offsets of the type's fields (0x10 to 0x60 as 16-byte slots, 0x70, 0x74, 0x78, 0x7C to 0x98 as 4-byte slots, 0x9C and
0x9E as 2-byte slots, 0xA0 and 0xA1 as bytes) are in the type resource itself. The game shows a swatch whose colour is the
mean of the float4 at 0x30 and the one at 0x60 (clamped to 0 to 1, times 255, no gamma), and the pearlescent swatch is the
float4 at 0x40 **[code: `FUN_00871650`, `FUN_008717b0`, the car select view model]**.

Painted reds, silvers, golds, oranges, greens and blues come out right when the float4 at 0x30 is encoded as sRGB
(`#AD2B2C`, `#C8CCC5`, `#D1A529`, `#C66624`, `#5BA330` on the first cars), so a converter for an older engine takes that
colour as the paint and the type as the finish **[verified]**.

## Where the paints are listed

- **The car's bundle** holds its `VehiclePaint` objects (1 to 15). Records repeat between cars: 90 distinct ids in 443
  records.
- **`GAMELOGIC/GAMEPLAY.BNDL`** holds the lists the game offers: types `Genesys.Gen.PaintPack` (six fields) and
  `Genesys.Gen.PaintPackGroup`, and one object (`0x17D1C0`, 23 KB) with the groups **[verified]**. A group is the list of colours
  or liveries the shop offers a vehicle, in the shop's order; its fields are in [Paint packs](#paint-packs).
- **`VEHICLES/VEHICLELIST.BNDL`** holds the vehicle list, one `VehicleList` resource (type `0x105`, 13,072 bytes): a 0x20-byte
  header (entry count 110 at +0x04, offset of the first entry 0x20 at +0x0C, offset of a trailing table at +0x10), then 110
  entries of 0x70 bytes: vehicle id (+0x00), `LiveryID` (+0x04), the game pack (`GamePackID`, +0x2C, [chameleon-plate.md](chameleon-plate.md)), the paint pack group (+0x34, `0x17D5xx` or `0x22Bxxx`) and
  the rollout numbers, and the upgrade records **[verified]**. (An earlier note here said 116 entries; the header and the
  trailing table say 110.) 93 of the 110 entries have a bundle in `VEHICLES/`; the other 17 are vehicles the install has no
  `VEH_<n>_HI.BNDL` for **[verified]**. `VehicleListEntry` is registered in `NFS13.exe` with the fields `LiveryID`, `GamePackID`,
  `Description`, `NameImage`, `ShortName`, `Image`, `OnlineImage`, `PaintPackGroup`, `TopSpeed`, `Acceleration`, `Power`,
  `PowerRPM`, `CarSelectSound`, `ManufacturerId` and `Rollout*` **[code: `FUN_0042ade0`]**.

## Liveries

**The list.** A `LiveryID` other than 0 makes another entry of the vehicle list for the same vehicle id. The number of an
entry's bundle is its `LiveryID`, or the vehicle id when the `LiveryID` is 0 or equal to it: `VEH_<n>_HI.BNDL` for all 93
bundles and no other file **[verified]**. 73 vehicle ids own the 93 bundles: 68 own one, and 5 own five (the entry with
`LiveryID` equal to the vehicle id, and four more) **[verified]**:

| Vehicle id | Car | Bundles (the vehicle's own first) |
|---|---|---|
| 393276 | BMW M3 E46 GTR 2001 | 393276, 2277210, 2277224, 2277238, 2277252 |
| 2076339 | Lamborghini Diablo SV 1995 | 2076339, 2277456, 2277467, 2277478, 2277489 |
| 2076399 | Porsche 911 GT2 2008 | 2076399, 2277332, 2277344, 2277356, 2277368 |
| 2076462 | Nissan 350Z 2006 | 2076462, 2255945, 2255956, 2255967, 2255978 |
| 2076524 | Nissan Skyline R34 GT-R 1999 | 2076524, 2255901, 2255912, 2255923, 2255934 |

The entries of one vehicle are identical in every other field of the list (the name and image fields are the same), so the
list holds no per-livery name; the names are in the vehicle's paint pack ([Paint packs](#paint-packs)): `HERO` (the vehicle's own
bundle) and four of `RACE`, `STREET`, `ABSTRACT` and `URBAN` **[verified]**. The game's rollout code selects among the entries
(`NumAvailableLiveries`, `AvailableLiveryId`, `AvailableLivery`) **[code]**, and `GLOBALCONFIG.BNDL` has a
`Genesys.Gen.Online.LiveryUnlockConfig` object with 20 `UnlockImageMap` entries (four per vehicle of five) **[verified]**
count, **[unconfirmed]** that they are the four liveries each vehicle unlocks online besides `HERO`. The game has no liveries
for any other vehicle: the Bugatti Veyron Super Sport and the McLaren MP4-12C and F1 LM have colours and no livery, and the
Veyron Grand Sport Vitesse has the one picture of [the other `_Livery` bundles](#liveries) **[verified]** (see
[Cars the list has twice](#cars-the-list-has-twice) for the vehicle ids that share a name).

**How a livery bundle differs from the vehicle's own.** The five groups were compared resource by resource (821, 777, 808,
756 and 779 resources per bundle) **[verified]**:

- Every resource the two bundles share by id is byte-identical, in all 20 liveries. The bundles differ only in a set of
  re-keyed resources that exist once in each: 1 texture, 1 to 2 materials, 3 renderables, 3 to 4 Genesys objects (the
  vehicle's behaviour objects), 1 model, 1 poly soup, 1 graphics spec and 2 vehicle sounds. Their ids are new (derived from
  the livery id). The renderables' vertex and index blocks, the poly soup and the sound blocks are byte-identical to the
  vehicle's own; the small main blocks of the other re-keyed resources differ in the ids they import and a few bytes more.
- **So a livery does not change the geometry, the lamps, the colours (the `VehiclePaint` records are shared, none is
  re-keyed), the collision, the stats or the sounds.** It changes exactly one thing: the one texture imported by the
  paint surface's material, a 2048 x 1024 BC1 picture with 12 mips (1,398,144 bytes) whose contents differ in all 25
  bundles (25 distinct pictures).
- Every paint surface of these five bodies uses the technique `VehicleNFS13_BodyPaint_Livery` (or its `_Lightmap` form for
  the small lightmapped panel of the 350Z and M3), and all of a car's paint surfaces draw the same picture.

**What the picture is.** `VehicleNFS13_BodyPaint_Livery` samples `DiffuseTextureSampler` at the surface's first texture
coordinates. Its pixel shader (disassembled from the DXBC in `SHADERS.BNDL` with `D3DDisassemble`) combines the paint
constant `PbrPaintDiffuseColour` and the sampled texel `t` as

```
base = lerp(PbrPaintDiffuseColour, t.rgb, t.a)
```

before the lighting, so the picture is laid over the paint colour by its alpha: where it is opaque it replaces the paint,
where it is transparent the paint shows **[verified: the `add` and `mad` of the texel against `cb1[4]`]**. The pictures use
both. The Skyline's own bundle has a BC1 picture with one-bit alpha and 64 percent transparent texels (white decals and dark
panels over the player's paint); the other 24 are fully opaque artwork (sponsor liveries with numbers and logos). Reading BC1
with one-bit alpha is therefore needed (`blackbox_tpk::decode_rgba8_cutout`). The pictures are authored for the bodies' own
texture coordinates, **so the paint surfaces of these cars are laid out for artwork** (the plain paint techniques are not).

**Other bundles with a `_Livery` technique.** 11 more bundles have a paint surface with a `_Livery` technique and are not in a
group: four draw a shared 1024 x 1024 BC3 picture that is not in their bundle (it is in `VEHICLETEX.BNDL`), one a 32 x 32
picture, and six have a picture of their own in the bundle (the two Ford F-150 SVT Raptor bundles and the Lamborghini Gallardo
for one panel next to plain paint surfaces; the Bentley, the Bugatti Veyron Grand Sport Vitesse and the Porsche 918 Spyder
for the body). A vehicle with one bundle has no alternative to choose from, but its picture is how the game dresses the car, so
the importer offers the picture of a single bundle as `Livery 1` when the `Livery` surfaces are most of the car's paint
(`liveries::own_picture`: the 918 Spyder, the Bentley, the Lancia Delta and the police
vehicles but the SWAT van), and the car wears it from the start (the shop can take it off). A picture on one panel next to plain paint (the Raptor,
the Gallardo) is laid out for that panel alone and a `TwoPaintMask` picture is a mask, so neither is offered; the Veyron GSV, whose picture repeats over its second-colour surfaces, offers its [two-colour packs](#two-colour-paint) instead. The techniques and textures are
**[verified]**; what the shared picture shows is **[unconfirmed]**.

## Paint packs

`GAMELOGIC/GAMEPLAY.BNDL` holds one Genesys object (`0x17D1C0`, 23 KB) whose root lists 67 `Genesys.Gen.PaintPackGroup`
structures, each with the `Genesys.Gen.PaintPack` structures of the choices the shop offers a vehicle, **in the order the shop
lists them** **[verified]**. A vehicle list entry names its group at +0x34 (0 for the police vehicles and for the second vehicle id of five cars: see
[Cars the list has twice](#cars-the-list-has-twice)). The structures sit inline in the object and the pointers between them are offsets in its main block (the import table
has a type import for each structure). Reader: `libs/chameleon-vehicle/src/paint_pack.rs`.

| Structure | Offset | Field | Evidence |
|---|---|---|---|
| group | +0x0C | the group's id, which the list entry names | [verified] |
| group | +0x14, +0x18 | count of packs, then that many offsets of packs | [verified] |
| pack | +0x0C | id of a `VehiclePaint` record of the vehicle's bundle (a plain colour), else 0 | [verified]: all 273 plain colour packs of 83 bundles name a record of their vehicle's bundle |
| pack | +0x10 | on a two-colour pack, the id of a `Genesys.Gen.ColourGroup` of the vehicle's bundle (`0x10E35F` and on, in sequence), a pair of paints ([Two-colour paint](#two-colour-paint)); 0 on the other packs | [verified]: 62 packs of 12 vehicles name a group of their bundle |
| pack | +0x14 | the pack's own number, in sequence within a group | [unconfirmed] |
| pack | +0x18 | an image id, the swatch | [unconfirmed] |
| pack | +0x1C | on a livery pack, the id of a Genesys object that is in the bundle of that livery and in no other bundle of the vehicle; else 0 | [verified]: 25 packs of 25 liveries |
| pack | +0x20 | the string id of the pack's name in the text table (`UI/LANGUAGE/0001.BNDL`) | [verified] |

Three kinds of vehicle, by their packs **[verified]**:

- **A plain colour pack per colour** (the fields at +0x0C set). The group has one pack per colour the shop offers, 3 to 9 of
  them, in the shop's order; the vehicle's bundle holds those paint records and sometimes a few more that no pack lists (the
  McLaren MP4-12C: 8 records, 7 packs, the first the orange `1F0577`). The pack's name is the colour's: `SILVER METALLIC`,
  `GREY GLOSS`, `ORANGE METALLIC`, `CARBON`.
- **A pack per livery** (the five vehicles of [Liveries](#liveries)): five packs, no colour, each with the livery object of one
  of the five bundles, so the bundle of a livery is known by the pack that names an object it holds. The names are the same
  five strings for the five vehicles, `HERO` (the vehicle's own bundle), `RACE`, `STREET`, `ABSTRACT` and `URBAN`, and their
  order differs between vehicles (the M3 GTR lists the bundles `2277252, 2277210, 2277224, 2277238, 393276`, the R34
  `2255934, 2255901, 2255912, 2255923, 2076524`, `HERO` last in both). The bundle holds one paint record, the colour behind
  the picture.
- **A two-colour pack** (twelve vehicles, [below](#two-colour-paint)): +0x10 is set. On nine of them (the Veyron Super Sport
  and Grand Sport Vitesse, the BMW M3 E92, the Challenger SRT8 392, the Ford GT, the Boss 302, the Cobra 427, the Caterham R500,
  the Ariel Atom) +0x0C is 0; the other three (the Shelby GT500, the Trans Am, the Charger R/T) name a plain colour there too,
  which is the first paint of the pair or a colour of the same name. The bundle holds more paint records than the group has
  packs (the Veyron SS: 12 and 5) because a pack is a pair of records.

The text table also names the swatches' colours, which the importer uses for the names of the two-colour liveries and does not
use for the paints: a saved garage selection stores the paint's name, and the importer's names (finish and hex colour) predate
this finding.

## Two-colour paint

**The colour groups.** Every car bundle holds a `Genesys.Gen.VehicleColourPalette` object (type id `0x2AA0`): a list of
`VehiclePaint` handles and, for 17 of the 93 bundles, a table of `Genesys.Gen.ColourGroup` structures **[verified]**. The
palette's header holds the offset of the list of paints (+0x10, with the count in the byte at +0x19), the offset of the table
of groups (+0x14), and the count of groups in the byte at +0x18 (the list of a car with no group is its paints). A group is
0x28 bytes, laid out inline like the other Genesys structures, with these fields (the type resource names them by hash, so
they are read by offset; `libs/chameleon-vehicle/src/colour_palette.rs`):

| Offset | Field | Evidence |
|---|---|---|
| +0x0C | handle of the **first paint**, a `VehiclePaint` of the bundle (the import table has a handle import at the slot; its resource key is the paint's own number, `VehiclePaint::id`) | [verified]: every handle of the 12 vehicles names a record of the bundle |
| +0x14 | handle of the **second paint** | [verified] |
| +0x1C | a third handle, never set | [verified] |
| +0x24 | the group's id, which the pack's +0x10 holds | [verified] |

A pack names its group by the id: Veyron Super Sport, `SILVER METALLIC`: group `0x10E35F`, paints `12462A` (`#646166`,
metallic) and `163181` (`#323232`, metallic). A palette holds a group or two that no pack names (the Veyron's `1631A6`, the
Caterham's `10E369`, the police vehicles' only group): they are not offered in the shop. The pack's name is the first
paint's in most cases (`BLUE METALLIC` is a blue first paint with a white second, `SILVER METALLIC` a silver one with a dark
grey second), but not always (the Atom's `RED GLOSS` is a black first paint and a red second), so **the pair is not "body
colour plus accent"**; which of the two is the colour the name says is not needed, because the surfaces say which they take.
The palette's own first paint (+0x10's list) is not always a pack's first (the Ford GT's is a paint no pack lists), so the
default of a new car is not read from it; that the game shows the first pack of a vehicle is **[unconfirmed]**.

**The Atom's pairs run the other way.** On every other vehicle the first paint changes from pack to pack and the second is a
fixed trim (a black roof, a dark grey lower body), or both change. All four of the Atom's packs have the same first paint,
`#0C0C0C`, and only the second one changes (`RED GLOSS` `#AD2B2C`, `YELLOW GLOSS`, `ORANGE GLOSS`, `GREEN GLOSS`), and its
big `TwoPaint` surface (5,239 triangles, the nose, the side panels and the tail) takes the first paint: read as the data says,
a pack is a black car with a few coloured parts (780 triangles: the chevron on the nose, the roll hoop trim) **[verified]**.
The shop sells the pack by the second colour, so the importer swaps such a pair (a vehicle whose packs share one first paint
and differ in the second) and the Atom wears the pack's colour on its body with the black as the trim. Whether the 2012 game
draws it black is **[unconfirmed]**. The swap alone left the car black: the Atom's panels (the nose, the floor and the
wheel arches: 5,071 and 3,168 triangles) are not paint at all but untextured `VehicleNFS13_Body` surfaces, whose material has
seven float4 parameters in name order (clearcoat fresnel, clearcoat specular, diffuse colour, roughness, scuff colour, scuff
settings, specular colour **[verified]** on the Atom's seven) and here a black diffuse colour (0.003 to 0.006), a dim specular
colour and a roughness of 0.1 to 0.5; its seats (roughness 0.75), chrome (specular 0.88) and tubular frame (fixed bronze) are
other values. The importer makes the black, smooth, non-metal surfaces of a vehicle that sells its second paint paint surfaces
(`body_panels.rs`, reader `libs/chameleon-vehicle/src/body_finish.rs`), so the panels wear the pack's colour and the player's
paint, and the black is left on the nose strip, the seats and the wings **[unconfirmed]** that the 2012 game paints them.
Importer: `mods/nfsmw-2012-content/src/two_colour.rs`.

**The surfaces.** The body-paint techniques with `TwoPaint` in their name draw the pair; the body of a car may mix them with
plain ones **[verified]** (technique of each paint mesh of the finest level, all 93 bundles):

| Technique | Takes | How |
|---|---|---|
| `BodyPaint`, `_Lightmap` | the first colour | `PbrPaintDiffuseColour` |
| `BodyPaint_TwoPaint` | the first or the second colour, per mesh | the material's `mPaintColourIndex` (a float in `gMaterialCB`, the only constant of the material: 0 or 1) selects it |
| `BodyPaint_TwoPaint_Livery` | the same, with a picture laid over by its alpha | the Veyron GSV's (index 1, a 128 x 128 BC3 picture whose coordinates repeat 20 times, probably a weave **[unconfirmed]**) and the police vehicles' |
| `BodyPaint_TwoPaintMask`, `_Lightmap` | both, by a texture | the `DiffuseTextureSampler` texture at the first texture coordinates: second colour where its green channel is 1 |

The pixel shaders (disassembled with `D3DDisassemble` from the DXBC in `SHADERS.BNDL`, programs `01010008D16ABE7E`,
`01010008CBE14B3A`) compute the paint colour as `PbrPaintDiffuseColour + PbrPaintSecondaryDiffuseColour * amount`, `amount`
being `mPaintColourIndex` or the mask's green; the specular colour likewise (`PbrPaintSpecularColour + PbrPaintSecondarySpecularColour * amount`).
The game must therefore feed the secondary constants with the *difference* of the pair's colours (a black second colour such
as the GSV's `#050505` roof would otherwise do nothing, and a bright one would overexpose) **[unconfirmed]**: the importer blends.
Masks are 64 x 128 to 2048 x 256 BC1 (the Caterham's BC3), unsigned-normalised, and binary in the green channel (nearly all
texels 0 or 255), and the surfaces' coordinates repeat (the Cobra's go from -1.9 to 3.5) **[verified]**. The shader of a
`TwoPaint` mesh never samples its first texture coordinates, and on four of the five cars measured they are no layout for a
picture (they run past +-25, overlap the other colour's mesh, or are one value), so a picture can only give such a mesh its
colour by pointing all its vertices at one place.

**The twelve vehicles** **[verified]** (file number, packs; surfaces of the finest level):

| Vehicle | Packs | Surfaces |
|---|---|---|
| Veyron 16.4 Super Sport (122692) | 5 | `TwoPaint`: 8,531 triangles first, 3,865 second; a 72-triangle plain lightmap panel |
| Veyron Grand Sport Vitesse (2076005) | 6 | `TwoPaint_Livery`, index 1: 4,241 triangles; plain: 1,508 |
| BMW M3 E92 (122682) | 6 | `TwoPaint`: 12,706 first, 320 second; plain lightmap panel 258 |
| Dodge Challenger SRT8 392 (122704) | 7 | `TwoPaintMask` 64 x 256 on all the paint |
| Ford GT (122714) | 5 | `TwoPaintMask` 1024 x 128 (10,867); plain 4,693 |
| Ford Mustang Boss 302 (122716) | 4 | `TwoPaintMask` 1024 x 512 (1,867); plain 5,973 |
| Shelby Cobra 427 (1085576) | 5 | `TwoPaintMask` 64 x 128 on all the paint |
| Caterham Superlight R500 (393001) | 4 | plain 4,535; `TwoPaintMask` 1024 x 256 BC3 (159); `TwoPaint` second (1,432) |
| Ariel Atom 500 V8 (1160082) | 4 | `TwoPaint`: 5,239 first, 780 second |
| Shelby Mustang GT500 1967 (2196334) | 4 | `TwoPaintMask` 2048 x 256 on all the paint |
| Pontiac Firebird Trans Am (2196399) | 5 | `TwoPaintMask` 1024 x 512 |
| Dodge Charger R/T 1970 (2196466) | 7 | `TwoPaint`: 5,893 first, 714 second |

Five more packs (the paint pack group `0x183F0F`) name colour groups of a vehicle the list has no bundle for. The police vehicles and the
Corvette Z06 have a group (`TwoPaint_Livery` surfaces) and no pack to name it. The McLaren MP4-12C and F1 LM have plain
colours only: their paint surfaces are plain `BodyPaint`, so they have no second colour to show **[verified]**.

## Cars the list has twice

The list names five cars under two vehicle ids, and one more under two with a different body. The second vehicle is the same
car with its group 0 and one paint record where the first holds the player's 4 to 10 **[verified]**; why it exists (the version
an event gives its AI car is a reading) is **[unconfirmed]**. Most resources are shared by id and byte-identical (783 of 821 for
the Porsche, 812 of 831 for the Mercedes) and the rest are re-keyed: the Genesys objects, the graphics spec and the vehicle sounds
of every pair, and for the Porsche also seven textures, ten renderables and four models, with the same meshes (9,586 and 118
triangles in both). The paint counts are:

| Car | Vehicle ids (file numbers) | Paint records |
|---|---|---|
| Porsche 911 Carrera S 2012 | 1085007, 1467186 | 9 and 1 |
| Mercedes-Benz SL 65 AMG Black Series 2009 | 122769, 1467179 | 4 and 1 |
| Maserati GranTurismo MC Stradale 2011 | 122757, 1467172 | 6 and 1 |
| Ford F-150 SVT Raptor 2011 | 300097, 1467164 | 6 and 1 |
| Caterham Superlight R500 2008 | 393001, 1467156 | 10 and 1 |
| Nissan Skyline R34 GT-R 1999 | 2076524 (with the four liveries), 2196131 | 1 and 6 |

The R34 pair differs more: 14,356 and 10,696 triangles in the body, 651 resources in common of 779 and 804, and different
values in the list's rollout and stat fields. The 2196131 bundle is the car in plain paint, the 2076524 group the car with
its liveries. No other object name is shared by two vehicles of the install (the police vehicles have none), which is how the importer
tells a car under two ids from two cars.

## What the game does not have

Searched for in the strings of `NFS13.exe` (Ghidra) and in every string of the non-texture resources of one car bundle (the
Porsche 911 Carrera S, `VEH_1085007_HI.BNDL`), all **[verified]** as absent (the other 92 bundles were checked for their paint
records only, not for strings):

- no resource, string or UI binding for vinyls, decals on cars (the `Decal_GBuffer` and `FloatingDecal` techniques are for
  the world), rims or wheel styles to choose (`rim` appears nowhere as a customization), body kits, hoods, roof scoops,
  window tint (a `Glass_Colourise` shader technique exists; what selects it is not known), spoilers or roof scoops as parts, or neon
  (strings for `spoiler`, `scoop`, `neon` and `wheel style` are absent from `NFS13.exe`);
- the car-select view model offers only the vehicle, its colour (`GetColourCount`, `GetCurrentColour`,
  `GetCurrentPearlescentColour`, `GetColourTexture`, `ColourHasTexture`, `GetColourID`, `GetNextColour`) and the five
  modification lists **[code: `FUN_00875370`]**;
- `ColourHasTexture` and `GetColourTexture` return a swatch image, not a paint texture.

The locator groups of the graphics spec (lamps, tail pipes) have numeric ids and no names
([chameleon-lamps.md](chameleon-lamps.md)), so spoiler or scoop mounts, if the 2012 bodies had any, cannot be read from
them either. The 2012 bodies do not need any: the game has no aftermarket part to mount.

## How the importer uses it

[`mods/nfsmw-2012-content`](../../mods/nfsmw-2012-content) turns the paint records of each car into the `[customization]`
table of the car's manifest ([car-customization.md](../modding/car-customization.md)): the float4 at 0x30 as the colour,
`Type` as the finish, repeated colours dropped, in the order of the bundle (396 paints in the 93 bundles). Reader:
`libs/chameleon-vehicle/src/paint.rs`. The real-install test `every_high_detail_car_has_paints` (`CHAMELEON_GAME_DIR`)
checks the 443 records.

**Liveries become variants of one car.** The importer reads the vehicle list (`src/vehicle_list.rs`) and groups the bundles by
vehicle id (`src/liveries.rs`, pure): the base is the entry without a livery (or with its own id as `LiveryID`), else the
first. A livery bundle is folded into the base when it is the same car: the same meshes at every level of the body (counts
of vertices and triangles) and a paint picture on both (every paint surface of the finest level drawing the same texture).
The base's picture and the livery pictures are written into the base car's `TEXTURES.BIN` as `<ID>_LIV1` to `<ID>_LIV5` (BC1
stays DXT1, cut to the texture size limit) and listed as `liveries = [{ name = "Hero", texture = "<ID>_LIV1" }, ...]` in
the car's `[customization]` table, in list order, the vehicle's own bundle first. A livery is named by the pack that names its
bundle ([Paint packs](#paint-packs), `src/variants.rs`), the text table's `HERO` as `Hero`; where a pack does not name it, or two
of the car's names would be alike, it is `Livery <n>`. The car wears its first livery as its stock look
(`default_livery = 0`), and the shop's `No livery` choice gives the plain paint, with every other livery beside it. A livery that
does not match its base, or a bundle the list does not mention, is converted as a car of its own. The shop lays a chosen
picture over the car's paint by its alpha, as the game's shader does.

**A car under two vehicle ids is one car** (`src/fold.rs`, pure; [Cars the list has twice](#cars-the-list-has-twice)). Vehicles
whose bundles have the same object name (a police vehicle never) are folded: the vehicle with liveries leads, else the one with
most paint records, else the earlier in the list; the others lend the lead their paint records, which are added after its own
and dropped where a colour and finish repeat. So the R34 GT-R is one car with its five liveries and the plain paint's six
colours as the paints to use without a livery. 93 bundles make 67 cars: 73 vehicle ids, less the 6 folded. The ids of the 67 are
the ones they had (ids are made from the object name in file order, and the lead is the earlier file of every pair); the 20
livery bundles and the 6 second vehicles had ids of their own before that no longer exist.

**Two-colour packs become liveries** (`src/two_colour.rs`; `libs/chameleon-vehicle/src/{colour_palette,paint_surface,two_colour}.rs`).
A car whose vehicle has packs that name a colour group (`PaintPackGroup::two_colour_packs`: the pair of the group's two
paints, their float4 at 0x30) gets one livery per such pack, in the shop's order, named by the pack's text (`SILVER METALLIC`
as `Silver Metallic`; a name two packs share is numbered, `Silver Metallic 2`; `Livery <n>` where the table has no name). The
picture of each is baked at conversion time into `<ID>_LIV<n>` (BC1) and the car's paint meshes get the texture coordinates
that pick their colour out of it, which is the same for every pack:

- A car with no mask (Veyron SS, M3 E92, Atom, Charger, and the GSV, whose weave picture is dropped) has a picture of 8 x 4
  texels, the first colour in the left block and the second in the right, and every vertex of a first-colour mesh is
  pointed at the middle of the left block, of a second-colour mesh at the middle of the right one. A mesh's coordinates are
  constant, so the picture is never minified and has one mip level.
- A car with a mask (Cobra, Challenger, Ford GT, Boss 302, Caterham, GT500, Trans Am) has a picture of the mask's size:
  each texel is the first colour where the mask's green channel is 0 and the second where it is 1, blended in linear light
  and encoded as sRGB (the end points of a BC1 block are the two colours, so a block takes the nearest of its four
  blends), with the full mip chain of the mask's size. The mask meshes keep their coordinates. The other paint meshes
  (plain, first colour) are pointed at a 4 x 4 block of the mask that is 0 (and the second-colour meshes at one that is 1);
  where the mask has no such block, the nearest is set to it (a few texels).
- The picture is opaque, so while a livery is on, the player's paint is hidden; the car wears its first pack as its stock
  look (`default_livery = 0`) and `No livery` gives the plain paints. The metallic finish of the game's paints is not baked
  in (flat colours), nor is the specular colour.

Which mesh takes which colour comes from the car's own bundle (`paint_surfaces`: the technique and, for `TwoPaint`, the
material's index; for a mask, the texture of its diffuse sampler), matched to the loaded paint meshes by order, technique and
triangle count at every level of the body (a car where it does not line up keeps its plain paints and the log says so). A
car with several masks or with a plain `_Livery` picture is not handled. 62 packs of 12 vehicles become liveries (the
install test `two_colour_packs_name_a_pair_of_their_cars_paints` counts them).

**The default paint is the pack's first colour.** The paints of a vehicle whose pack lists plain colours (`PaintPackGroup::paint_order`)
come first, in the pack's order, then the bundle's other paints in its own order; so the car starts in the colour the shop lists
first. That this is the colour the 2012 game shows the vehicle in is **[unconfirmed]**. The vehicles with two-colour packs keep
that order for their paints (the nine whose packs name no plain colour, the bundle's), and start in their first pack's livery.

## Open questions

- The meaning of the colours at 0x10 and 0x20, the eight settings at 0x7C and the flags at 0xA0 (clearcoat, scratch and
  specular settings of the shader parameters are the candidates).
- The fields of `PaintPack` at +0x14 and +0x18 (the pack's own number and an image id), and the third paint slot of a colour
  group (+0x1C, never set).
- Whether the game feeds the secondary paint constants with the difference of the pair's colours (the importer blends), and
  how it shades the second colour (the importer bakes flat colours; the weave of the GSV's `TwoPaint_Livery` surfaces is dropped).
- Why a palette keeps a group or two that no pack names (the Veyron's `1631A6`), and which paint is the palette's first.
- Whether the first pack of a group is the colour the 2012 game shows a vehicle in, and which paints of a bundle that no pack
  lists the game never offers (the importer offers them after the packs').
- The names of the paints: the `Name` field of the record is 0, and the pack's name (`SILVER METALLIC`, `ORANGE METALLIC`) is
  in the text table. The importer names a paint by finish and hex colour, so a saved selection keeps working.
- The language of the `Type` values (solid and metallic are readings of the data).
- What the shared picture of the other `_Livery` bundles is, and which of those pictures cover a whole body.
- What a `TwoPaintMask` picture looks like in the 2012 game (stripes, a roof, a lower panel in the second colour): the
  importer bakes it as the shader reads it (the Cobra, the Challenger, the Ford GT, the Boss 302, the GT500, the Trans Am and
  the Caterham have one), but the result has not been compared with the game's own screenshots.
