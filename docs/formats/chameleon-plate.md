# Chameleon licence plates: the text, its default and the glyph sheet

How NFS: Most Wanted (2012) puts a text on the licence plate of a car: where the text comes from, which defaults the install
holds, and how the plate mesh and its glyph sheet draw it. Evidence tags are the repository's
([README.md](../README.md#evidence-tags)): **[verified]** is checked on the 2012 install (v1.4.0.0, `NFS13.exe`) with the
readers of this repository, **[code]** is read from the decompiled game code in Ghidra, **[unconfirmed]** is a reading
that fits the data and has not been proved. Reader and tests: `libs/chameleon-vehicle/src/plate.rs`.

## Short answer

The plate text is **not a property of the car**. It is the *registration string of the player* (the player property
`LicenseRegString`, set in Autolog's "Edit Plate Registration" text entry) **[code]**. While the player has set none, the game
shows a **default that depends on the game pack the car belongs to**: five strings in all, held in the executable, not in
a bundle **[verified]**:

| Game pack | Number | `GamePackID` | Default text | Vehicle list entries | Bundles |
|---|---|---|---|---|---|
| the base game | 0 | `0x42E7` | `NEED4SPD` | 70 | 53 |
| Ultimate Speed Pack | 1 | `0x2045BB` | `ULTIMATE` | 5 | 5 |
| Terminal Velocity Pack | 2 | `0x21836A` | `VELOCITY` | 5 | 5 |
| Movie Legends Pack | 3 | `0x2184B4` | `MOVILGND` | 5 | 5 |
| NFS Heroes Pack | 4 | `0x2184B5` | `NFS HERO` | 25 | 25 |

Every one of the 110 entries of the vehicle list names one of the five packs, and so does each of the 93 bundles
**[verified]**. The pack names are the ones the text table gives the four DLC packs ("Ultimate Speed Pack", "Terminal Velocity Pack", "Movie
Legends Pack", "NFS Heroes Pack"); which number is which follows from the default texts and from the cars of each (the five hero
vehicles are number 4, five other cars each in the others) **[unconfirmed]** for numbers 2 and 3. The NFS Heroes entries are the five hero vehicles, each with its
four livery bundles ([chameleon-customization.md](chameleon-customization.md#liveries)); of the 67 cars the content mod
makes, that leaves 47 with `NEED4SPD` and 5 each with the others **[unconfirmed]**, since the count of folded vehicles per
pack was not checked.

The plate is drawn by a small mesh of **eight quads** whose texture is a **glyph sheet** of 36 cells; the text is chosen
by telling the shader which cell each quad shows. A car whose shader is not told shows the sheet as the mesh maps it,
`ABCDEFGH` **[verified]**. That is what a converted car showed before the content mod gave it a text.

## Where the default text is

`NFS13.exe` holds the five strings one after another, each in a record of 12 bytes: `NEED4SPD`, `ULTIMATE`, `VELOCITY`,
`MOVILGND`, `NFS HERO` (a space, not an underscore: the label a disassembler gives it says `NFS_HERO`, the bytes say
`NFS HERO`) **[verified]**. The reader finds the run by its first record (`default_texts`).

The function that answers the property `LicenseRegString` (`FUN_006bc0d0`, `Player` class, getter of the registration)
works like this **[code]**:

1. If the player's registration string is blank (only spaces), or equals a stored sentinel string, it is a default.
2. A default is chosen from the player's current vehicle: its vehicle list entry names a game pack object, and the pack's
   number (a 16-bit field of the pack) picks the string: 1 `ULTIMATE`, 2 `VELOCITY`, 3 `MOVILGND`, 4 `NFS HERO`, anything
   else (the base game's number is 0) `NEED4SPD`.
3. Otherwise the player's string is the text.

So in the single player game, where the player has typed nothing, a car loaded from the garage or the story shows the text
of its pack, and one the player customised shows that text on every car (the property is the player's, not the car's)
**[code]**.

The game packs are the objects of type `Genesys.Gen.GamePack` in `GAMELOGIC/GAMEPLAY.BNDL`, five structures in one object
(`0x33BA`) **[verified]**:

| Offset in a pack | Type | Field |
|---|---|---|
| +0x1C | u32 | the pack's id: the `GamePackID` of vehicle list entries (+0x2C of an entry) and the pack id of plate designs |
| +0x28 | u16 | the pack's number, 0 to 4 |

(The other fields are ids of the pack's text, image and unlock objects; they are not read.)

## The plate mesh

Every one of the 93 car bundles carries one material with the shader technique
`VehicleNFS13_Body_Textured_NormalMap_Lightmap_Licenseplate` (the shader declares the constant `VehicleLicencePlateOffsets`,
which the game sets from the text) **[verified]**. Its textures, in the material's order, are:

| Slot | Sampler | Texture |
|---|---|---|
| 0 | `NormalTextureSampler` | an 8 x 8 BC1 flat normal, the same in every car (`0x138806`) |
| 1 | `DiffuseTextureSampler` | the **glyph sheet**, 512 x 512 BC1, the same texture in all 93 cars (`0x138805` of `VEHICLETEX.BNDL`) |
| 2 | `EffectsTextureSampler` | the shared 2048 x 2048 scratch and dirt map |
| 3 | `LightmapLightsTextureSampler` | the car's own 128 x 128 lightmap |

The mesh is **32 vertices and 16 triangles: eight quads side by side**, one per character. At the finest level, 91 of the 93
cars have it; the vehicles `VEH_1097100` and `VEH_2196065` have the material and no mesh that draws it **[verified]**
(what those two are is not known). A plate is about 0.264 m wide and 0.06 m high; quad `n` (counted from the car's left,
as seen from behind) covers the texture columns of cell `n` of the first row of the sheet, with a margin of about 3 px
between cells:

| Quad | Left edge `u` | Right edge `u` |
|---|---|---|
| 0 | 0.0033 | 0.1080 |
| 1 | 0.1142 | 0.2190 |
| 7 | 0.7808 | 0.8853 |

and `v` from 0.010 to 0.243 **[verified]**: the cells are `1/9` wide and `1/4` high, the glyph sits inside the cell with that margin.

## The glyph sheet

The sheet is 9 columns by 4 rows of cells, 36 glyphs, black on a beige ground, in a condensed plate typeface. In reading order
the cells hold **`A` to `Z`, then a dash drawn at mid height, then `1` to `9`**; there is **no zero** and no other punctuation
**[verified]** by looking at the texture. The executable's own table of the same order is
`ABCDEFGHIJKLMNOPQRSTUVWXYZ 123456789`: the 27th character is a **space**, so cell 26 is the *blank* glyph, and the sheet
draws it as the dash **[verified]** table and picture, **[unconfirmed]** whether the shader blanks the quad instead of drawing the dash.

## How the game turns a text into cells

The routine that sets the registration of a plate (`FUN_00571cc0`) keeps, next to the string, an array of eight cell numbers,
all `26` (the blank) to begin with **[code]**. For each character of the string it:

1. turns a lower case letter into upper case;
2. turns `0` into `O` (the sheet has no zero);
3. looks the character up in `ABCDEFGHIJKLMNOPQRSTUVWXYZ 123456789`; the position is the cell. A character that is not there
   becomes a space, and its cell is 26;
4. stores the cell at position `(8 - length) / 2 + n` of the array: **a text shorter than eight is centred**, and the slots
   around it keep the blank cell (the dash).

The text kept for the plate is the string after these changes, so `NFS_HERO` typed by a player is kept as `NFS HERO`. The
routine does not limit the length; the plate has eight quads, so the text entry must (the limit the game's text entry puts
on the registration was not found: **[unconfirmed]**, eight is what the mesh can show).

The array is what the shader reads (`VehicleLicencePlateOffsets`): quad `n` draws cell `array[n]` of the sheet instead of
cell `n`. The reader's `cells(text)` and `registration(text)` do the same steps, and tests check them.

## The plate designs of Autolog (not on the car mesh)

The game also has **plate designs**: 666 backgrounds the player earns online and picks in Autolog (the property `LicensePlateId`),
one object of type `Genesys.Gen.Online.LicensePlate` in `GAMEPLAY.BNDL`. Each is a 512 x 256 interface picture of a plate over a
photograph, a name and a game pack **[verified]**; they are interface images, no car bundle holds one, and the 3D plate's
glyph quads take none of it. The records, the 469 distinct pictures, the unlock data and what is not stored (category, country,
glyph colour, a default) are in [chameleon-plate-designs.md](chameleon-plate-designs.md).

## What the content mod does with it

For each car with a plate mesh the mod writes the glyph sheet into the car's `TEXTURES.BIN` under the name `<FOLDER>_PLATE`,
and a `[customization.plate]` table in the car's manifest: the default text of its pack, the layout of the sheet (9 x 4,
the 36 glyphs, 8 slots, `0` drawn as `O`) and the sheet's name. The game draws the sheet again with the text the player
types ([car-customization.md](../modding/car-customization.md#license-plates)); it does not read the executable. How the
mod finds the text: `mods/nfsmw-2012-content/src/plate.rs`. The designs are one shared set written once for all cars
(`src/plate_designs.rs`, [chameleon-plate-designs.md](chameleon-plate-designs.md#what-the-content-mod-writes)); the car's table names
their index.

## Not verified

- A converted car has not been seen with a typed text in a window; the composition of the sheet is tested on pictures and on
  the real sheet, the drawing of it in the game was not looked at.
- The text length the 2012 text entry allows, and whether a blank slot shows the dash or nothing.
- Which two vehicles have no plate mesh, and whether any car puts the plate on a body that is not the one the mod converts.
