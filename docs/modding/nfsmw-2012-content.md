# The NFS: Most Wanted (2012) cars mod

`mw2012-cars` adds the cars of **NFS: Most Wanted (2012)** to this game: 93 more cars to choose from, with their own
bodies, interiors, glass, lamps, wheels and textures. The 2012 game runs on a different engine (Criterion's Chameleon,
not Black Box), so the cars are converted, not copied. The mod is a native mod like [`hello`](examples.md): the source
is [`examples/mods/mw2012-cars`](../../examples/mods/mw2012-cars), the conversion is
[`crates/nfsmw-import2012`](../../crates/nfsmw-import2012) and the readers behind it are the three `libs/chameleon-*`
crates (formats: [chameleon-bundle](../formats/chameleon-bundle.md), [chameleon-gfx](../formats/chameleon-gfx.md),
[chameleon-vehicle](../formats/chameleon-vehicle.md)).

**Nothing from either game is in the repository or in a release.** The cars are made on your machine, while the game
starts, from your own installs. Without a 2012 install the mod adds nothing and the game is unaffected.

## Using it

1. Build the mod (`cargo build --release -p mw2012-cars-mod`) and put `examples/mods/mw2012-cars/mod.toml` and the built
   library (`mw2012_cars_mod.dll`, `libmw2012_cars_mod.so` or `libmw2012_cars_mod.dylib` from `target/release/`) in one
   folder under the game's mods folder, for example `%APPDATA%\nfsmw\config\mods\mw2012.cars\` (see
   [Using mods](README.md#using-mods)).
2. Start the game once. The mod writes a commented `config.toml` next to its `mod.toml` and logs where it is.
3. Put the folder of your 2012 install in it and start the game again:

   ```toml
   source = 'D:\Need For Speed Most Wanted 2012'
   ```

That is all. The first start with a `source` converts the cars while the game starts (about ten seconds and 360 MB for
all 93, written into the mod's own `files/` and `cars/` folders); the cars are then in the game like any content mod's.
Later starts convert nothing unless the settings or the files of either install changed (the mod keeps a fingerprint
in `.import-stamp`) and take under a second. A change in `config.toml` takes effect at the next start. To remove the
cars, delete the mod folder or set `mods = off` / use `--no-mods`.

The cars are clones: each takes the physics, sound, parts and wheel layout of a stock racing car it fits best (the mod
picks the one whose wheelbase, track and tyre radius need the least stretching). Paint is the game's own: the body paint
surfaces use the placeholder texture and light material the stock cars use, so the colour comes from the clone's paint
system. Only the default paint was looked at; repainting in the garage is untested.

### Settings (`config.toml` in the mod's folder)

Every key is optional. An unknown key is an error and no car is added, so a typo is not silently ignored.

| Key | Meaning |
|---|---|
| `source` | The 2012 install folder. Not set: nothing is converted |
| `cars` | `"all"` (default) or a list of ids or name fragments (`["Porsche", "LAMGALLARD11"]`) |
| `exclude` | Ids or name fragments to leave out |
| `max_texture_size` | Longest texture side, a power of two from 16 to 4096 (default 1024); bigger maps lose their top mips |
| `role` | `"player"` (default), `"cop"` or `"both"`: who can use the cars |

The ids are made from the car's name (`POR911CARR12` for the 2012 Porsche 911 Carrera S); the mod's log lists every car
it converts or leaves out and why.

## What is converted

| 2012 | In the game |
|---|---|
| Body, interior, lamps, mirrors (finest of 3 levels, 5,000 to 36,000 triangles) | `<CLONE>_KIT00_BODY_A..D`, in car space (+x forward, +y left, +z up), squeezed along each axis until the wheels land on the clone's: hub height and wheelbase at the hubs, track at the rim face (the game puts the rim face at the stock track) |
| Tyre, brake disc, rim and caliper | one `<CLONE>_KIT00_FRONT_TIRE_A..D` solid, rim face at y = 0, as the game's wheels are modelled |
| Diffuse maps (BC1, BC3) | the car's `TEXTURES.BIN` as DXT1 and DXT5, cut to `max_texture_size` |
| Paint surfaces | the game's `<CAR>_SKIN1` placeholder with light material `CARSKIN`, which the game swaps for the paint |
| Glass and lamp lenses | a translucent tint texture with light material `WINDSHIELD` |
| Untextured surfaces | a flat colour texture from the material's diffuse colour (dark grey when it has none) |
| Normals | recomputed from the triangles |

Levels of detail: the game draws nothing for a level a car lacks, so A and B are the finest body, C and D the coarser
ones. The vertex limit of one solid (60,000) can force a coarser level.

## How a mod does this

The game reads a mod's `files/` and `cars/` folders **after** the mod's `load` has run (`open_install`), so a mod can
generate content in `load`. For that the host table tells every mod where it lives: `ctx.mod_dir()` is the mod's own
folder (where `mod.toml`, its `config.toml` and the generated folders are) and `ctx.game_dir()` is the game's install,
which the mod needs to read the stock cars' wheel layouts (see [api.md](api.md#lifecycle)). Both are appended fields of
the host table, so mods built before them keep loading.

## Limits

- **Looks only.** Handling, engine sound, damage and the HUD are the clone's. The 2012 damage model, headlight beams,
  brake discs (they are part of the wheel), spoiler and mirror animation, and the lamp locators are not converted.
- **Normal, specular and effect maps are dropped**: the 2005 car shader has its own lighting.
- **Names in the garage.** The mod gives every car a display name (`Porsche 911 Carrera S 2012`) and brand, but the
  garage does not use mod names yet (see [api-content.md](api-content.md#cars)); it shows the clone's.
- **Livery duplicates.** The 2012 game ships some cars several times as liveries (five Skyline R34 bundles, for
  example); all are imported, with a number added to the id.
- **Traffic and low-detail bundles** (`_TR`, `_LO`) are not imported.
- A few cars stretch visibly: the SWAT van and the Venom GT need 1.4 times along one axis on the nearest stock car.
- The conversion runs on the game's main thread while it starts, so the very first start with a `source` waits for it.

## Tests

`cargo test -p nfsmw-import2012` runs the unit tests (settings, ids, fitting, textures, folder and fingerprint safety).
With `MW2012_GAME_DIR=<2012 install>` and `NFSMW_GAME_DIR=<2005 install>` set, `cargo test -p nfsmw-import2012 --
--ignored` converts a car the way the mod does and reads it back with the game's readers, and
`CHAMELEON_GAME_DIR=<2012 install> cargo test -p chameleon-bundle -p chameleon-vehicle -- --ignored` loads every car of
the install. To look at a car without opening a window:

```
nfsmw view-car POR911CARR12 --screenshot out.png --muted
```
