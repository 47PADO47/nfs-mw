# Chameleon licence plate designs: the 666 backgrounds of Autolog

The 2012 game has **plate designs**: backgrounds the player earns and picks in Autolog ("Pick your License Plate"). This
page says what a design is in the install, what it covers on a car, and how the content mod turns them into a set the garage
offers. The text on the plate and the glyph sheet are in [chameleon-plate.md](chameleon-plate.md). Evidence tags are the
repository's ([README.md](../README.md#evidence-tags)): **[verified]** is checked on the 2012 install (v1.4.0.0, `NFS13.exe`)
with the readers of this repository, **[code]** is read from the decompiled game code in Ghidra, **[unconfirmed]** is a reading
that fits the data and has not been proved. Reader and tests: `libs/chameleon-vehicle/src/plate_design.rs`.

## Short answer

A design is **an interface picture, a name and a game pack**. It is not a mesh and not a texture of a car: no car bundle holds
one, and the plate mesh of a car (eight glyph quads, [chameleon-plate.md](chameleon-plate.md#the-plate-mesh)) takes none of
it (the one constant of the plate shader the executable names is `VehicleLicencePlateOffsets`, the eight cell numbers
**[code]**). The game shows a design on the plate pictures of its interface; the registration is drawn over it in the interface
font **[unconfirmed]**. The strings of the executable say so: the `LicensePlatesManager` object answers `PlateImages` ("backing image
gcid by index") and `PlayerPlateImages` ("backing images for each player"), the menu screens bind
`LicensePalette.LocalPlate.BackingImage` and `.Name`, and the text table calls the widget "LICENSE PLATE BACKING" **[code]**.

| Question | Answer |
|---|---|
| How many | 666 designs in one list, over **469 distinct pictures**: 197 designs share the picture of an earlier one ("ONE HIT WONDER", "BRING THE PAIN" and "OUT OF NOWHERE" are one picture) **[verified]** |
| What is stored | per design: a picture number, a name, a game pack **[verified]**; nothing else |
| The picture | `UI/IMAGES/STREAMED/<number>.BNDL`, one texture, **512 x 256, BC3, no mip levels** (131 KB); all 469 exist **[verified]** |
| Where on the picture | the plate is the **top left 337 x 170 pixels** of every picture, the rest is transparent **[verified]**; it has rounded corners, four screw holes and a stripe, over a photograph of cars |
| Name | a string id of the language tables; all 666 resolve in the English table ("START ME UP", "RUNNING IN", "TAKEDOWN") **[verified]**; many names repeat (the "TAKEDOWN MASTER" tiers) |
| Category | **none stored**. The game pack groups them: 448 base game, 47 Ultimate Speed, 63 Terminal Velocity, 51 Movie Legends, 57 NFS Heroes designs (the counts of `GamePackID` in the list: `0x42E7` 0x1C0, `0x2045BB` 0x2F, `0x21836A` 0x3F, `0x2184B4` 0x33, `0x2184B5` 0x39) **[verified]** |
| Country | **none**. No design has a country, region or style field; the pictures are art, not licence plates of countries |
| Glyph colour or font | **none**. One glyph sheet serves every design ([chameleon-plate.md](chameleon-plate.md#the-glyph-sheet)); the interface draws the registration over the picture in its own font **[unconfirmed]** |
| Unlock | the pack id (+0x18) says which pack owns the design: "Purchase {pack} to earn this license plate" **[code]**; the base game's are earned by milestones ("Collect all True Fan License Plates", "Unlock 'Supersonic'... License Plates (Multiplayer)", "Takedown an opponent with 'Ultimate Fan' as your License Plate") **[code]**. The table of which milestone gives which design (`PlateReleaseByIndex`) is not in `GAMEPLAY.BNDL` and was not found **[unconfirmed]** |
| Thumbnail | the picture itself (the interface scales it); there is no separate thumbnail |
| Default | the install sets none: the player's `LicensePlateId` is a field of the session's player slot (a gcid the message `SetLocalPlayerLicensePlate` stores by list index) and nothing in the data names a starting design **[code]**. The first record is "START ME UP" **[verified]**; whether it is the one a new player has is **[unconfirmed]** |

## The list

One object of type `Genesys.Gen.Online.LicensePlate` in `GAMELOGIC/GAMEPLAY.BNDL` (`0x148ECD`, 34,672 bytes) **[verified]**:

| Offset | Type | Field |
|---|---|---|
| +0x10 | u32 | offset of the array of record offsets (always 0x18) |
| +0x14 | u32 | number of designs, 666 |
| +0x18 | u32[666] | offset of each record in the object, one per design, 32 bytes apart from 0xA80 on |

A record is 32 bytes (`0xA80 + 32 n`), each its own structure type in the object's import table **[verified]**:

| Offset | Type | Field | Evidence |
|---|---|---|---|
| +0x00, +0x04 | u32 | zero | [verified] |
| +0x08 | u32 | the same word (`0xEB27EF41`) in all records: a type hash | [verified] |
| +0x0C | u32 | the picture number: `UI/IMAGES/STREAMED/<decimal number>.BNDL` | [verified] on all |
| +0x10 | u32 | an id with no text; the design's own id in the game's changer data, perhaps the value `LicensePlateId` takes | [unconfirmed] |
| +0x14 | u32 | the name's string id | [verified] on all |
| +0x18 | u32 | the id of the game pack: `GamePackID` of [chameleon-plate.md](chameleon-plate.md#short-answer) | [verified] |
| +0x1C | u32 | zero | [verified] |

The order of the list is the order of Autolog's: the nine base game designs "START ME UP" to "END OF THE ROAD?" first, then the
five of each pack in the order of the packs, then the milestone sets. The first design of the list is **picture 1863272,
"START ME UP"**.

## What a design covers on a car

The picture is a whole plate (2:1, with screws); the plate mesh of a car shows only the **glyph strip**, 0.264 m by 0.06 m (4.4
times wider than high). The repository uses a design as the **ground behind the glyphs of that strip**:

1. the plate is found by the alpha of the picture (`design_plate`: always `(0, 0, 337, 170)`);
2. the band that lies behind the glyphs is the central part of the plate in the aspect of the strip (`design_band`): 8 percent
   off each side, the height from the width (284 x 64 of the 337 x 170), centred a little below the middle, between the screws
   a choice of this repository; on the eight pictures looked at it clears the screw holes;
3. the band is stretched over the plate's eight slots, and the sheet's glyphs are drawn over it in a dark or light ink chosen by
   the picture (`plate_design.rs` of `nfsmw-data`: [car-customization.md](../modding/car-customization.md#plate-designs)).

This is the repository's own use, not the game's: the 2012 game does not draw a design on a car.

## What the content mod writes

`mods/nfsmw-2012-content/src/plate_designs.rs` writes the set **once** for all cars (`files/PLATES/MW2012/`):

| File | Holds |
|---|---|
| `DESIGNS.TOML` | the index: `pack = "DESIGNS.BIN"` and one `[[design]]` per picture (`id` the picture number, `name`, `picture` the texture name `PD<number>`, `group` the pack's name) |
| `DESIGNS.BIN` | a texture pack with one **256 x 64 DXT1** texture per picture: the band, cut and scaled, 8 KB each, no mip levels |

- **Size.** Whole, the 469 pictures are 61 MB (512 x 256 BC3, 131 KB each). The bands are 469 x 8,192 = **3.84 MB** of pixels
  plus the pack's tables, and the index is about 55 KB: **about 4 MB for all cars**, against 666 designs x 93 cars x a picture
  if it were written for each car. (Computed from the sizes; the written files were not measured: the mod was not built in the
  run that made them.)
- **Usable designs.** 469: each picture once, named after the first design that uses it; a name already taken gets its count
  (`TAKEDOWN MASTER 2`), so every name of the list is different. The other 197 designs have the same picture as one of these.
- **Default.** None (`DEFAULT_DESIGN` in `plate_designs.rs`): the 2012 data does not say which design a new player has, so a
  car's plate keeps the ground of its glyph sheet until a design is picked. A one-line change sets the first design for all cars.
- A car's `[customization.plate]` gets `designs = "PLATES/MW2012/DESIGNS.TOML"` when the install lists designs.

## Not verified

- The mod was not built in the run that wrote it: the files above, their size and the look of a design on a car (the glyphs
  over the band) are computed and tested on pictures, not seen in a window. `tests/plate_designs.rs` (`MW2012_GAME_DIR`,
  `--ignored`) writes a sample sheet to look at.
- Which design a new player has, and the table of which milestone unlocks which design.
- Whether the interface draws the registration over the picture exactly as the repository's ink does (colour, outline).
