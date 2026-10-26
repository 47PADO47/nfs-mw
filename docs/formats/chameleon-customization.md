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
| Liveries | separate vehicle list entries with the same vehicle id and another `LiveryID`: 5 vehicles have 5 entries each (the car and 4 liveries); each has a bundle, which differs from the car's own in one texture (the paint picture) and the ids that name it | [verified] |
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
  `Genesys.Gen.PaintPackGroup`, and one object (`0x17D1C0`, 23 KB) with the groups **[verified]**. A pack entry names a
  `VehiclePaint` object, a paint id, the swatch image and a second image (`0x124631`, `0x1632A0`, `0x1B8BA1`, `0x1B8CCC` in
  the first entry) **[verified]**; the field meanings are **[unconfirmed]**.
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
game has no per-livery name in the list. Its text tables have the word "LIVERIES" (a shop category) and the sentence that a
bodyshop does not remove the livery, and no livery names **[verified]** (`UI/LANGUAGE/0001.BNDL`). The game's rollout code
selects among the entries (`NumAvailableLiveries`, `AvailableLiveryId`, `AvailableLivery`) **[code]**. Other cars share a
name with an unrelated vehicle id (the Skyline R34 is also vehicle 2196131, a different car with no livery technique): the
vehicle id, not the car's name, is what groups bundles.

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
for the body). A vehicle with one bundle has no alternative to choose from, so the importer offers none of these: they stay
flat paint, and whether the car's own picture should be its default look is not decided. The techniques and textures are
**[verified]**; what the shared picture shows is **[unconfirmed]**.

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
stays DXT1, cut to the texture size limit) and listed as `liveries = [{ name = "Livery 1", texture = "<ID>_LIV1" }, ...]` in
the car's `[customization]` table, in list order, the vehicle's own bundle first. One car is made per vehicle: 93 bundles make
73 cars. A livery that does not match its base, or a bundle the list does not mention, is converted as a car of its own, as
before. The shop lays a chosen picture over the car's paint by its alpha, as the game's shader does. The ids of the 73 base
cars are the ones they had (ids are made from the object name in file order, and the livery bundles come last in file order);
the 20 livery bundles had ids of their own before (the base's id with a counter) that no longer exist.

## Open questions

- The meaning of the colours at 0x10 and 0x20, the eight settings at 0x7C and the flags at 0xA0 (clearcoat, scratch and
  specular settings of the shader parameters are the candidates).
- The fields of `PaintPack` and the paint pack group of each vehicle, and so which of a bundle's paints a given rollout
  offers. The importer offers all of the bundle's.
- The names of the paints: the `Name` field is 0, so the game must take them from a string table keyed by the colour id or
  the family; the importer names a paint by finish and hex colour.
- The language of the `Type` values (solid and metallic are readings of the data).
- What the shared picture of the other `_Livery` bundles is, and which of those pictures cover a whole body.
- The names the game gives its liveries: none is in the vehicle list or the text tables, so the importer numbers them.
