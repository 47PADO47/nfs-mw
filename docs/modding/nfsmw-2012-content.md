# The NFS: Most Wanted (2012) content mod

`nfsmw-2012-content` adds the cars of **NFS: Most Wanted (2012)** to this game: 93 more cars to choose from, with their
own bodies, interiors, glass, lamps, wheels and textures. The 2012 game runs on a different engine (Criterion's
Chameleon, not Black Box), so the cars are converted, not copied. It is a native mod like [`hello`](examples.md), the only
mod of the repository that is built with the workspace: [`mods/nfsmw-2012-content`](../../mods/nfsmw-2012-content). The
readers behind it are the three `libs/chameleon-*` crates (formats: [chameleon-bundle](../formats/chameleon-bundle.md),
[chameleon-gfx](../formats/chameleon-gfx.md), [chameleon-vehicle](../formats/chameleon-vehicle.md)).

**Nothing from either game is in the repository or in a release.** The cars are made on your machine, in the background
while the game runs, from your own installs. Without a 2012 install the mod adds nothing and the game is unaffected.

## Using it

1. Build the mod (`cargo build --release -p nfsmw-2012-content`) and put `mods/nfsmw-2012-content/mod.toml` and the built
   library (`nfsmw_2012_content.dll`, `libnfsmw_2012_content.so` or `libnfsmw_2012_content.dylib` from `target/release/`)
   in one folder under the game's mods folder, for example `%APPDATA%\nfsmw\config\mods\nfsmw-2012-content\` (see
   [Using mods](README.md#using-mods)).
2. Start the game once. The mod writes a commented `config.toml` next to its `mod.toml`.
3. Put the folder of your 2012 install in it and start the game again:

   ```toml
   source = 'D:\Need For Speed Most Wanted 2012'
   ```

That is all. With a `source` set, the mod converts the cars **on a background thread while the game starts and runs**;
nothing waits for it. It takes about a second on a six-core PC once the files are in the disk cache, a few more seconds
when they are not, and writes 360 MB into the mod's own `files/` and `cars/` folders. When it is done the cars are in
the game: everything the game opens after that, the garage and the career among them, lists them, so by the time you
reach the career menu they are there. Later starts convert nothing unless the settings or the files of either install
changed (the mod keeps a fingerprint in `.import-stamp`), and the cars are in from the first frame. A change in
`config.toml` takes effect at the next start; when cars are already in the game, the new conversion is kept aside and
replaces them at that start. To remove the cars, delete the mod folder, or set `mods = off` / use `--no-mods`.

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

## How it works with the host

- **At load** the mod learns its own folder and the game's install from the host table (`ctx.mod_dir()`,
  `ctx.game_dir()`, see [api.md](api.md#lifecycle)). It promotes a finished conversion left by an earlier start, reads
  `config.toml`, and, unless the content on disk is still right, starts the conversion thread and returns.
- **The conversion** runs on a pool of all cores but one (at most six), one car per task, and builds into `.staging/`. It
  never writes where the game reads. The fingerprint is written last, so a run cut short is thrown away.
- **When it is done** (the mod sees it in its frame events: the worker cannot call the host) the mod moves `.staging/files`
  and `.staging/cars` into place and calls `bb.content.rescan()`. The host adds the mod's `files/` as a *late layer* of the
  install, which every copy of the install shares, including the ones the scenes already hold, and publishes the cars its
  `cars/` folder declares; the car tables list them from then on. See [api-content.md](api-content.md#content-registries-bbcontent).
- If content was already live at start (a rerun after a change), the finished conversion is promoted at the next start
  instead, because the game has indexed the folder.

## Limits

- **Looks only.** Handling, engine sound, damage and the HUD are the clone's. The 2012 damage model, headlight beams,
  brake discs (they are part of the wheel), spoiler and mirror animation, and the lamp locators are not converted.
- **Normal, specular and effect maps are dropped**: the 2005 car shader has its own lighting.
- **Names.** The mod gives every car a display name (`Porsche 911 Carrera S 2012`) and a brand. The garage uses the
  display name (`scenes/world/garage.rs`); I have not looked at the garage screen with an imported car, nor at the other
  screens that name cars.
- **Livery duplicates.** The 2012 game ships some cars several times as liveries (five Skyline R34 bundles, for
  example); all are imported, with a number added to the id.
- **Traffic and low-detail bundles** (`_TR`, `_LO`) are not imported.
- A few cars stretch visibly: the SWAT van and the Venom GT need 1.4 times along one axis on the nearest stock car.
- A screen the game has open when the conversion finishes keeps the car list it built; open it again to see the cars.

## Tests

`cargo test -p nfsmw-2012-content` runs the unit tests (settings, ids, fitting, textures, promotion and fingerprint). With
`MW2012_GAME_DIR=<2012 install>` and `NFSMW_GAME_DIR=<2005 install>` set, `cargo test -p nfsmw-2012-content -- --ignored`
converts a car through the background job, reads it back with the game's readers, and converts every car with the time it
takes. `NFSMW_GAME_DIR=<2005 install> cargo test -p nfsmw rescan` checks that content added after start-up reaches the
copies of the install that are already open. `CHAMELEON_GAME_DIR=<2012 install> cargo test -p chameleon-bundle
-p chameleon-vehicle -- --ignored` loads every car of the install. To look at a car without opening a window:

```
nfsmw view-car POR911CARR12 --screenshot out.png --muted
```
