# Chameleon customization: what NFS: Most Wanted (2012) offers for a car's looks

What the 2012 game stores per car for its looks (paint, liveries, rims, spoilers, vinyls, tint), where, and how it reads
it. Evidence tags are the repository's ([README.md](../README.md#evidence-tags)); **[verified]** here means checked on the
2012 install (v1.4.0.0, `NFS13.exe`) with the readers of this repository, **[code]** means read from the decompiled game
code in Ghidra (`NFS13.exe`), **[unconfirmed]** means a reading that fits the data and has not been proved.

## Short answer

Per car the 2012 game offers **a set of body paints** and nothing else for the car's looks. It has **no vinyls, decals,
rims, spoilers, roof scoops, body kits, hoods or window tints to choose from**, and no paint texture or UV layout built
for artwork. What is called a livery is a different vehicle entry (its own bundle), not a layer on one car.

| Item | In the 2012 game | Evidence |
|---|---|---|
| Body paint colours | 1 to 15 `VehiclePaint` records per car bundle (443 records in the 93 cars; 396 after dropping a colour and finish a car repeats; 90 distinct paint ids) | [verified] |
| Paint finishes | solid (`Type` 0, 214 records) and metallic (`Type` 1, 228 records); one record has a secondary (pearlescent) colour | [verified] counts, [unconfirmed] the names of the two types |
| Liveries | separate vehicle list entries with the same vehicle id and another `LiveryID`: 5 vehicles have 5 entries each (the car and 4 liveries) | [verified] |
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
`g_paintColour`) **[verified]**. A paint surface therefore has no livery or colour texture; its texture coordinates serve
the lightmap and the effects map, and are laid out for those, not for artwork.

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
- **`VEHICLES/VEHICLELIST.BNDL`** holds the vehicle list (116 entries of 0x70 bytes after a 0x20 header): vehicle id (+0x00),
  `LiveryID` (+0x04), the paint pack group (+0x34, `0x17D5xx` or `0x22Bxxx`) and the rollout numbers, and the upgrade
  records **[verified]**. `VehicleListEntry` is registered in `NFS13.exe` with the fields `LiveryID`, `GamePackID`,
  `Description`, `NameImage`, `ShortName`, `Image`, `OnlineImage`, `PaintPackGroup`, `TopSpeed`, `Acceleration`, `Power`,
  `PowerRPM`, `CarSelectSound`, `ManufacturerId` and `Rollout*` **[code: `FUN_0042ade0`]**.

## Liveries

A `LiveryID` other than 0 makes another entry of the vehicle list for the same vehicle id: vehicle 393276 has the entry
with `LiveryID` 393276 and four more (2277210, 2277224, 2277238, 2277252); 2076339, 2076399, 2076462 and 2076524 likewise
**[verified in the list]**. The game's rollout code selects among them (`NumAvailableLiveries`, `AvailableLiveryId`, `AvailableLivery`)
**[code]**. They are the same car in other bundles with their own textures (the five Nissan Skyline R34 bundles) **[verified
by the bundle numbers]**: a livery is a different vehicle, so the importer treats each bundle as a car.

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

The car-space locators of the graphics spec (lamps, tail pipes) are present but their name table is not decoded
([chameleon-vehicle.md](chameleon-vehicle.md)), so spoiler or scoop mounts, if the 2012 bodies had any, cannot be read from
them either. The 2012 bodies do not need any: the game has no aftermarket part to mount.

## How the importer uses it

[`mods/nfsmw-2012-content`](../../mods/nfsmw-2012-content) turns the paint records of each car into the `[customization]`
table of the car's manifest ([car-customization.md](../modding/car-customization.md)): the float4 at 0x30 as the colour,
`Type` as the finish, repeated colours dropped, in the order of the bundle (396 paints in the 93 cars). Nothing else of
the table is filled, because the game has nothing else. Reader: `libs/chameleon-vehicle/src/paint.rs`. The real-install
test `every_high_detail_car_has_paints` (`CHAMELEON_GAME_DIR`) checks the 443 records.

## Open questions

- The meaning of the colours at 0x10 and 0x20, the eight settings at 0x7C and the flags at 0xA0 (clearcoat, scratch and
  specular settings of the shader parameters are the candidates).
- The fields of `PaintPack` and the paint pack group of each vehicle, and so which of a bundle's paints a given rollout
  offers. The importer offers all of the bundle's.
- The names of the paints: the `Name` field is 0, so the game must take them from a string table keyed by the colour id or
  the family; the importer names a paint by finish and hex colour.
- The language of the `Type` values (solid and metallic are readings of the data).
