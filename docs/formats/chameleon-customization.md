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

| Item | In the 2012 game | Evidence |
|---|---|---|
| Body paint colours | 1 to 15 `VehiclePaint` records per car bundle (443 records in the 93 cars; 396 after dropping a colour and finish a car repeats; 90 distinct paint ids) | [verified] |
| Paint finishes | solid (`Type` 0, 214 records) and metallic (`Type` 1, 228 records); one record has a secondary (pearlescent) colour | [verified] counts, [unconfirmed] the names of the two types |
| Liveries | separate vehicle list entries with the same vehicle id and another `LiveryID`: 5 vehicles have 5 entries each (the car and 4 liveries); each has a bundle, which differs from the car's own in one texture (the paint picture) and the ids that name it; named `HERO` (the vehicle's own), `RACE`, `STREET`, `ABSTRACT`, `URBAN` by the vehicle's [paint pack](#paint-packs) | [verified] |
| Rims, tyres, brake discs, calipers | one model each per car, as the wheel entry of the graphics spec; no alternatives | [verified] |
| Spoiler | an animation of a part of the body (`<Car>_SpoilerAnimation`, driven by speed), not a choice | [verified] resource, [unconfirmed] which cars have it |
| Vinyls, decals, body kits, hoods, roof scoops, window tint | none: no resource, no UI binding, no string in `NFS13.exe` | [verified] searches, see below |
| Licence plate | global list (`LicensePlatesManager`), a player choice, not per car; the body has a `Licenseplate` shader technique | [code] |
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
  entries of 0x70 bytes: vehicle id (+0x00), `LiveryID` (+0x04), the paint pack group (+0x34, `0x17D5xx` or `0x22Bxxx`) and
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
(`liveries::own_picture`: the 918 Spyder, the Bentley, the Lancia Delta, the Veyron Grand Sport Vitesse and the police
vehicles but the SWAT van), and the car wears it from the start (the shop can take it off). A picture on one panel next to plain paint (the Raptor,
the Gallardo) is laid out for that panel alone and a `TwoPaintMask` picture is a mask, so neither is offered. The techniques and textures are
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
| pack | +0x10 | on a two-colour pack, a number (`0x10E35F` and on, in sequence) that is the id of no resource of the install; 0 on a plain colour | [unconfirmed] meaning |
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
- **A two-colour pack** (nine vehicles: the Veyron Super Sport and Grand Sport Vitesse, the BMW M3 E92, the Challenger SRT8
  392, the Ford GT, the Boss 302, the Cobra 427, the Caterham R500, the Ariel Atom): +0x0C is 0 and +0x10 is set. The bundle
  holds more paint records than the group has packs (the Veyron SS: 12 and 5), so a pack is probably a pair of records; **how
  the pair is chosen is not found** [unconfirmed]. The importer takes these vehicles' paints in the order of the bundle.

The text table also names the swatches' colours, which the importer does not use: a saved garage selection stores the paint's
name, and the importer's names (finish and hex colour) predate this finding.

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

**The default paint is the pack's first colour.** The paints of a vehicle whose pack lists plain colours (`PaintPackGroup::paint_order`)
come first, in the pack's order, then the bundle's other paints in its own order; so the car starts in the colour the shop lists
first. That this is the colour the 2012 game shows the vehicle in is **[unconfirmed]**. The nine vehicles with two-colour packs
keep the order of the bundle.

## Open questions

- The meaning of the colours at 0x10 and 0x20, the eight settings at 0x7C and the flags at 0xA0 (clearcoat, scratch and
  specular settings of the shader parameters are the candidates).
- The fields of `PaintPack` at +0x10, +0x14 and +0x18, and how a two-colour pack picks its pair of paint records (nine
  vehicles); until then those vehicles are one colour all over, as the importer paints them.
- Whether the first pack of a group is the colour the 2012 game shows a vehicle in, and which paints of a bundle that no pack
  lists the game never offers (the importer offers them after the packs').
- The names of the paints: the `Name` field of the record is 0, and the pack's name (`SILVER METALLIC`, `ORANGE METALLIC`) is
  in the text table. The importer names a paint by finish and hex colour, so a saved selection keeps working.
- The language of the `Type` values (solid and metallic are readings of the data).
- What the shared picture of the other `_Livery` bundles is, and which of those pictures cover a whole body.
- What a `TwoPaintMask` picture draws (a stripe or a roof in the second colour): the Cobra, the Challenger, the Ford GT, the
  Boss 302, the GT500, the Trans Am and the Caterham have one, and none is offered as a livery.
