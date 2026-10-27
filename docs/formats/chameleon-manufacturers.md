# Chameleon manufacturers: the brands and logos of NFS: Most Wanted (2012)

Where the 2012 game keeps its car brands (the manufacturer table of the vehicle list), their names, and the logo
pictures of the car select. Measured on the 2012 install (v1.4.0.0) with the bundle reader
([chameleon-bundle.md](chameleon-bundle.md)), by reading every texture under `UI/IMAGES/` and by `NFS13.exe` in Ghidra.
Evidence tags: [README.md](../README.md#evidence-tags). Readers: `chameleon-vehicle::manufacturers` (the table),
`chameleon-vehicle::localised_text` (a name), `chameleon-gfx::Texture::to_rgba8` (the picture). The mod that uses them is
[nfsmw-2012-content](../modding/nfsmw-2012-content.md) (`brands.rs`).

## Where they are

| What | Where |
|---|---|
| The manufacturer table | `VEHICLES/VEHICLELIST.BNDL`, the `VehicleList` resource (type 0x105), after the vehicle entries **[verified]** |
| The brand's name | a string id in `UI/LANGUAGE/0001.BNDL` (English; the table is [chameleon-language.md](chameleon-language.md)), upper case: `ASTON MARTIN`, `MERCEDES-BENZ` **[verified]** |
| The logo picture | one bundle per picture, `UI/IMAGES/STREAMED/<id>.BNDL` with `<id>` in decimal, holding one `Texture` whose resource id is `0x01000000_00000000 + id` (the class byte 1: a numeric id) **[verified]** |

## The table **[verified]**

The list resource's main block starts with a 0x20-byte header:

| Offset | Type | Field |
|---|---|---|
| 0x04 | u32 | number of vehicle entries (110) |
| 0x08 | u32 | number of manufacturers (36, `0x24`) |
| 0x0C | u32 | offset of the vehicle entries (0x20) |
| 0x10 | u32 | offset of the manufacturer table (0x3040: it follows the entries) |

The table is one record of five u32 per manufacturer, and ends with the block:

| Offset | Field | Evidence |
|---|---|---|
| +0x00 | the manufacturer id (`0x1EDBD` to `0x1EDD7`, `0x5FB40`, `0x5FF28`, `0xA2324`, `0x10C098`, `0x11B4E1` to `0x11B4E7`, `0x17AD23`, `0x1FAE26`, `0x2183ED`) | the vehicle entries name it **[verified]** |
| +0x04 | string id of the name | **[verified]**: all 36 read in `0001.BNDL` |
| +0x08 | a second id (one of `0x1EDD8` to `0x1EDDE`, `0x17AD25`) | not a string of `0001.BNDL`; it takes 7 values, so it may be a region or country: **[unconfirmed]** |
| +0x0C | the logo picture's numeric id, 0 for the two records with no logo | **[verified]**: 34 of 36 name a bundle of `UI/IMAGES/STREAMED/` |
| +0x10 | a second picture id | no resource of the install has it: **[unconfirmed]** |

**The link from a vehicle.** Each vehicle entry of the list (0x70 bytes, see [chameleon-customization.md](chameleon-customization.md#where-the-paints-are-listed))
holds `ManufacturerId` at +0x14 **[code: `FUN_0042ade0`, the registration of `VehicleListEntry`]**. All 110 entries name a
manufacturer of the table **[verified]**: 11 `TRAFFIC` entries, 1 `OTHER`, the rest cars (Nissan 12, Porsche 11, Ford 9,
Lamborghini 9, BMW 7, and so on). The 36 records:

ALFA ROMEO, ASTON MARTIN, AUDI, BENTLEY, BMW, BUGATTI, CHEVROLET, DODGE, FORD, JAGUAR, KOENIGSEGG, LAMBORGHINI,
MASERATI, MCLAREN, MERCEDES-BENZ, MITSUBISHI, NISSAN, PAGANI, PORSCHE, SHELBY, SUBARU, TRAFFIC and OTHER (no logo),
HUMMER, CATERHAM, JEEP, ARIEL, SRT, LEXUS, BAC, LANCIA, LAND ROVER, TESLA, MARUSSIA, HENNESSEY, PONTIAC.

## The pictures **[verified]**

Every logo is a bundle with one texture, no mips. Sizes and formats (all measured):

| Logos | Size | Format |
|---|---|---|
| 29 of the 34 | 256 x 128 | BC3 (alpha) |
| Dodge, Pontiac | 256 x 256 | BC3 |
| Hummer, Jeep | 128 x 128 | BC3 |
| Hennessey | 128 x 128 | BC1 with punch-through alpha |

The artwork is the brand's real logo in colour (chrome, metal and enamel renderings), centred in the frame with a
transparent surround: the visible part is 86 to 254 pixels wide. The logos of the wide marques (Hummer, Dodge, McLaren) are
only 15 to 45 pixels high inside their frame, so a user of the pictures should cut to the alpha before scaling.

Other pictures under `UI/IMAGES/STREAMED/` that look like logos (Mazda, Toyota, Gumpert, Carbon Motors, a McLaren and
Mercedes pair, each 128 x 128) are not named by the table: **[unconfirmed]** what uses them. The rest of the folder is the
paint swatches (64 x 32), the car pictures (512 x 256 and 512 x 128), the loading screens (256 x 128 in
`STOREIMAGES.BNDL`) and HUD icons (64 x 64).

## What a converter does with them

`nfsmw-2012-content` matches the make of each converted car to a manufacturer by name (ignoring case, spaces and
punctuation; `Mercedes` matches `MERCEDES-BENZ`), takes the logo's visible part, scales it to fit a transparent 300 x 150
picture with a margin, and writes it as an 8-bit RGBA PNG with a manufacturer manifest. A make that the table does not
name (`NFS`, one car of the install) gets none.

## Open questions

- The meaning of the second id (+0x08) and the second picture (+0x10) of a record.
- What the unnamed 128 x 128 brand pictures (Mazda, Toyota, Gumpert, Carbon Motors) are used for.
- The other language tables (`0002` to `0014`): the names were read in English only.
