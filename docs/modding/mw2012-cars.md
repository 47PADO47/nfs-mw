# The NFS: Most Wanted (2012) cars mod

`nfsmw-import2012` turns the cars of **NFS: Most Wanted (2012)** into a content mod for this game: 93 more cars to
choose from, with their own bodies, interiors, glass, lamps, wheels and textures. The 2012 game runs on a different
engine (Criterion's Chameleon, not Black Box), so the cars are converted, not copied; the converter and the readers
behind it are [`crates/nfsmw-import2012`](../../crates/nfsmw-import2012) and the three `libs/chameleon-*` crates
(formats: [chameleon-bundle](../formats/chameleon-bundle.md), [chameleon-gfx](../formats/chameleon-gfx.md),
[chameleon-vehicle](../formats/chameleon-vehicle.md)).

**Nothing from either game is in the repository or in a release.** The mod is built on your machine from your own
installs. Without a 2012 install the importer has nothing to do, says so, and exits cleanly; the game is unaffected.

## Using it

You need your own installs of both games. Then:

```
nfsmw-import2012 --source "D:\Need For Speed Most Wanted 2012" --game-dir "D:\Need For Speed Most Wanted"
```

It writes the mod folder `<config dir>/mods/mw2012.cars` (`%APPDATA%\nfsmw\config\mods\mw2012.cars` on Windows;
see [Using mods](README.md#using-mods)), which the game loads at start-up like any content mod. Run it again to rebuild:
it only clears a folder it made itself (it leaves a marker file) and refuses to write into any other non-empty folder.
The 93 cars take about six seconds and 360 MB.

The cars are clones: each takes the physics, sound, parts and wheel layout of a stock racing car it fits best (the
importer picks the one whose wheelbase, track and tyre radius need the least stretching; its log names the clone of each
car). Paint is the game's own: the body paint surfaces use the placeholder texture and light material the stock cars
use, so the colour comes from the clone's paint system. Only the default paint was looked at; repainting in the garage
is untested.

### Settings

`mw2012-import.toml`, next to the game's `config.toml` (`nfsmw-import2012 --write-config` writes a commented copy).
Every key is optional and each has a command-line twin; the command line wins.

| Key | Flag | Meaning |
|---|---|---|
| `source` | `--source` | The 2012 install folder. Also read from `MW2012_GAME_DIR`. Not set: nothing is imported |
| `game_dir` | `--game-dir` | The 2005 install, when the game's own discovery does not find it |
| `output` | `--out` | Where the mod folder goes. Default `<config dir>/mods/mw2012.cars` |
| `cars` | `--cars a,b` | `"all"` (default) or a list of ids or name fragments (`["Porsche", "LAMGALLARD11"]`) |
| `exclude` | `--exclude a,b` | Ids or name fragments to leave out |
| `max_texture_size` | `--max-texture-size` | Longest texture side, a power of two from 16 to 4096 (default 1024); bigger maps lose their top mips |
| `role` | | `"player"` (default), `"cop"` or `"both"`: who can use the cars |

`--list` prints every car of the 2012 install with the id the importer gives it. To keep the cars out of a run, remove
the folder or use `--no-mods` / the `mods` setting (see [Using mods](README.md#using-mods)).

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

## Tests

`cargo test -p nfsmw-import2012` runs the unit tests (settings, ids, fitting, textures, folder safety).
`CHAMELEON_GAME_DIR=<2012 install> cargo test -p chameleon-bundle -p chameleon-vehicle -- --ignored` loads every car of
the install. To look at a result without opening a window:

```
nfsmw view-car POR911CARR12 --screenshot out.png --muted
```
