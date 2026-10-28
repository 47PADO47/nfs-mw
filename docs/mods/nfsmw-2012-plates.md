# 2012 content mod: licence plates

Part of [the content mod's README](../../mods/nfsmw-2012-content/README.md).

Every car with a plate mesh (91 of the 93 bundles; the plate is eight quads and a 36-glyph sheet) gets its **proper default
text** and a plate the player can retype. In the 2012 game the text is the player's registration, and until the player sets one
it is a default of the car's **game pack**: `NEED4SPD` for the base game, `ULTIMATE`, `VELOCITY`, `MOVILGND` and `NFS HERO` for
the four DLC packs. The five strings are in `NFS13.exe`, the pack of a car is the `GamePackID` of its vehicle list entry
(+0x2C), and `GAMEPLAY.BNDL` gives the packs' numbers ([chameleon-plate.md](../formats/chameleon-plate.md)). The mod
reads them (`src/plate.rs`, `src/vehicle_list.rs`) and writes, for each car, the glyph sheet as `<ID>_PLATE` in `TEXTURES.BIN`
and a `[customization.plate]` table with the text, the 9 x 4 layout, 8 slots and `0` drawn as `O`. Without the executable
the cars keep the sheet as drawn (`ABCDEFGH`) and can still be retyped. In the garage, **Visual, License Plate** is a keyboard
prompt (letters and digits the sheet has, up to 8; empty goes back to the car's text); the text is kept per car and profile in
`plates.toml` beside the save, which the original game's saves never see ([car-customization.md](../modding/car-customization.md#license-plates)).
`nfsmw view-car <ID> --part plate=TEXT` shows a text. The garage lists License Plate and Plate Design only for a car that has the
plate table (and designs), never for a stock car. The F6 garage (Appearance, License plate) and the console commands `plate` and
`plate-design` change the plate of the car being driven for the session only. Not checked in a window: the drawing of a typed text on the car.

**Plate designs.** The 2012 game's 666 Autolog designs (469 pictures of 512 x 256: a name, a game pack and a picture each; no category, country
or glyph colour in the data) are written **once** for all cars, as `files/PLATES/MW2012/DESIGNS.TOML` (the index) and `DESIGNS.BIN`
(a texture pack: each picture cut to the band behind the plate's glyph strip, 256 x 64 DXT1, 8 KB, about 4 MB in all), and every
car's `[customization.plate]` names the index (`designs = ...`). **Visual, Plate Design** in the garage steps through them with the
car showing each; the pick is kept per car in `plates.toml` (`[plate_designs]`). No design is the default (the 2012 data does not
say which one a new player has; `DEFAULT_DESIGN` in `src/plate_designs.rs` sets one for all cars), so a plate keeps its sheet's own
ground until one is picked. `nfsmw view-car <ID> --part plate-design=ID` shows one (`none` the plain plate). See
[chameleon-plate-designs.md](../formats/chameleon-plate-designs.md) and [car-customization.md](../modding/car-customization.md#plate-designs)
(`src/plate_designs.rs`). Not checked in a window: how a design looks on the car.

Tests: the test `plate` and the test `plate_designs` of the mod (`MW2012_GAME_DIR`, `cargo test -p nfsmw-2012-content --test
plate_designs -- --ignored`) convert one car with the real install and check the manifest, the index, the pack (469 pictures)
and that the game's plate code draws a design; the second also writes `plate-designs-sample.png` (a plate with three designs
and the plain one) into its temporary folder, to look at. The cutting and encoding have unit tests of their own.
