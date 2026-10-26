# Customizing mod cars: the shop, paints and parts

Part of the [modding catalogue](README.md). A car a mod adds ([api-content.md](api-content.md#cars)) is one more entry in the
car list, named by its folder, with files of its own. Nothing about it comes from a car of the 2005 game: not its
parts, not its physics, not its sound. This page says what the auto shop (the garage, the career shop and the F6 garage)
offers such a car, how a mod lists the paints and parts, and what the car's files must hold for the shop to work on it. The
other halves of a car are on their own pages:

| Page | Covers |
|---|---|
| [car-stats.md](car-stats.md) | `[stats]` (mass, engine, gearbox, tyres, brakes in SI units), the upgrade levels, the shop's bars, nitrous |
| [car-sound.md](car-sound.md) | `[engine_sound]`, `[car_sound]`, the built-in engine, the 2012 recordings |
| [car-collision.md](car-collision.md) | `BOUNDS.BIN`: the collision shape, the damage zones |

The [2012 cars mod](nfsmw-2012-content.md) is the worked example, and its settings are on its own page.

## The files of a car

A car is the folder `files/CARS/<ID>/` of a mod plus the manifest `cars/<id>.toml` ([api-content.md](api-content.md#cars)):

| File | Needed | Holds |
|---|---|---|
| `GEOMETRY.BIN` | yes | the body and wheel solids, in the install's format |
| `TEXTURES.BIN` | yes | their textures |
| `BOUNDS.BIN` | yes, to drive | the car's own collision bounds ([car-collision.md](car-collision.md)); the garage leaves out a car whose physics cannot be built, and the game refuses it with the reason in the log |
| `VINYLS.BIN` | only for `vinyls` | the vinyl pictures, a texture pack (`PREVINYL.BIN` is read too) |
| `SOUND/...` WAV files | only for `[engine_sound]`, `[car_sound]` | [car-sound.md](car-sound.md) |

The 2012 mod names the solids after the car's folder: `<ID>_KIT00_BODY_A` to `_D` (the body, with the glass, lamps and
interior), `<ID>_KIT00_FRONT_TIRE_A` to `_D` (one wheel solid: tyre, rim, brake disc and caliper, axle along y, rim face
at y = 0, as the game's wheels are modelled; the game turns it for the left side) and the placeholder texture
`<ID>_SKIN1` on the paint surfaces. Solids are in car space (+x forward, +y left, +z up), centred on the wheelbase: the
wheels sit at plus and minus half the `wheelbase_m` of `[stats]` ([car-stats.md](car-stats.md)). A level of detail a car
lacks draws nothing, so A and B are the finest. These are the names the 2012 mod writes and its real-install test loads; how
the game would find the body of a hand-made car under other names was not re-checked.

## What the shop offers a mod car

The auto shop offers a mod car **what the car's own `[customization]` table lists** and nothing from a stock car's parts
table. The table and the files in the car's folder are the whole source:

| Auto shop item | Mod car | Where it comes from |
|---|---|---|
| Paint finish, colours, custom RGB body colour | yes | `paints`: each has a name, a colour and a finish; one is the car's paint until the player picks another |
| Rim paint | yes | `rim_paints` |
| Window tint | yes | `tints`: a colour and an opacity |
| Rims | yes | `rims`: wheel models in the car's own `GEOMETRY.BIN` |
| Spoilers, roof scoops | yes | `spoilers`, `roof_scoops`: models in the car's own `GEOMETRY.BIN` |
| **Vinyls** and their colour channels | yes | `vinyls`: pictures in the car's own `VINYLS.BIN`; `vinyl_colors` is the palette |
| Performance upgrades (engine, transmission, tires, brakes, suspension, induction, nitrous) | yes, three levels each | steps of the car's own `[stats]` ([car-stats.md](car-stats.md#upgrades)) |
| Nitrous | yes: from the factory (`nitrous = true`), or bought as a kit | [car-stats.md](car-stats.md#nitrous) |
| Body kits, hoods, manufacturer decals, door numbers | no | a mod car has no such parts; a mod that wants a body variant adds another car |

A car that gives no `[customization]` table offers the shop nothing of its own: its paint is what its textures hold and the
shop lists no choice for it.

The garage lists the car under the display name and brand its manifest gives (`name`, `manufacturer`; without a name it shows
the car id, without a brand none).

To look at a car's paint without starting the world:

```
nfsmw view-car POR911CARR12 --part paint=3 --screenshot out.png --muted
```

The number after `=` counts from 1 in the order the `parts` console command lists (F12, then `parts` in a world with the
car; `parts paint 3` applies it there). The categories are `body`, `spoiler`, `hood`, `roof`, `rims`, `paint`,
`tint`, `rimpaint` and `vinyl`.

## Customization: `[customization]` in the car manifest

A mod car carries its own customization. Nothing in it names another car. The table is checked by
`libs/blackbox-modding/src/content/customization.rs` and read by `crates/nfsmw-data/src/car/mods/custom/`.

```toml
# cars/fastcoupe.toml
[customization]
default_paint = 0                   # position in paints (default 0)
paints = [
  { name = "Red", color = [173, 43, 44] },                          # finish: gloss (default), metallic or chrome
  { name = "Silver", color = [200, 204, 197], finish = "metallic" },
]
rim_paints = [{ name = "Black", color = [10, 10, 10] }]
tints = [{ name = "Smoke", color = [20, 30, 40], opacity = 0.7 }]   # opacity 0 (clear) to 1
rims = [{ name = "Five spoke", model = "FASTCOUPE_RIM1" }]
spoilers = [{ name = "Wing", model = "FASTCOUPE_WING" }]
roof_scoops = [{ name = "Scoop", model = "FASTCOUPE_SCOOP" }]
vinyls = [{ name = "Stripe", bitmap = "FASTCOUPE_STRIPE", mask = "FASTCOUPE_STRIPE_MASK", channels = 2 }]
vinyl_colors = [{ name = "White", color = [255, 255, 255] }]        # left out: a built-in palette of 12 colours
```

| Key | Meaning |
|---|---|
| `paints` | At least one, at most 64. `color` is sRGB (three numbers from 0 to 255). `finish` picks the shading: `gloss` is the plain paint material, `metallic` the metallic one, `chrome` the mirror one (the shared light materials `REGPAINTWHITE`, `METPAINTSILVER`, `CHROME`). |
| `default_paint` | The paint the car has until the player changes it; it is the car's stock paint, so it is not listed among the choices (picking nothing is picking it). |
| `rim_paints` | Colours for the metal of the rims. They work on the wheels the car ships with and on `rims`. |
| `tints` | A copy of the shared windshield material with the colour applied and the alpha set to `opacity`. |
| `rims` | `model` is a wheel solid of the car's own `GEOMETRY.BIN`, **without the level-of-detail letter**: the game looks for `<model>_A` to `<model>_E` and uses the nearest level the pack has. The solid replaces the whole wheel (tyre and rim) and is modelled as the car's own wheel solid is: axle along y, rim face at y = 0. |
| `spoilers`, `roof_scoops` | `model` as for rims. The solid is in car space (+x forward, +y left, +z up), already at its place on the body: it is drawn as it is, with no mount marker. |
| `vinyls` | `bitmap` and `mask` are texture names in the car's own `VINYLS.BIN` (a texture pack; `PREVINYL.BIN` is read too). The mask's red channel is the opacity. `channels` (0 to 3) says how many of the bitmap's red, green and blue weights are colour channels the player can recolour; 0 keeps the bitmap's own colours. |
| `vinyl_colors` | The colours the channels can take, for all three channels. |

- A name is 1 to 32 characters, with no control character, and unique within its list. Models and textures are letters,
  digits, `_` and `-`, up to 56 bytes. A list has at most 64 entries. A wrong value, or a key the table does not know,
  refuses the car and the log names the key.
- A choice is told apart by a hash of the car's folder, the kind and the name, so a saved car keeps its choice when the mod
  reorders its lists, and loses it when the mod renames it.
- A model that is missing from `GEOMETRY.BIN` (the `_A` level), or a vinyl whose pictures are missing, differ in size or
  cannot be decoded, is **not offered** and the log says why. The paint placeholder is the texture `<ID>_SKIN1` (the car's
  own folder) on the car's paint surfaces, with the light material `CARSKIN`; the game swaps it for the colour, and for a
  vinyl laid over it.
- The shop prices every choice of a mod car at the first tier of its category (the game's price table has no entry for the
  mod's parts).
- Decals, door numbers and the HUD gauge skins are not part of the table; the gauge skins of the install work as for any car.

### Vinyls

A vinyl is a picture authored in **the car's own paint texture layout**: the picture is drawn over the paint surfaces through
their texture coordinates, so the mod authors the picture for its own coordinates. The bitmap and the mask are texture-pack
entries of `VINYLS.BIN`; the shop composes the layer (`vinyl::compose`) and draws it over the paint colour. The 2012 cars give
none (see below).

### The 2012 cars

The [2012 mod](nfsmw-2012-content.md) fills only `paints`, with the colour records of each car's own bundle (1 to 15 per
car, 396 in the 93 cars), because that is all the 2012 game offers for a car's looks: it has no vinyls, rims, spoilers,
roof scoops or tints to choose, and its paint shader takes a flat colour and has no artwork layout
([chameleon-customization.md](../formats/chameleon-customization.md)). The paint texture coordinates of a converted car
are the 2012 car's own. The 2012 game ships some cars several times as liveries (five Skyline R34 bundles); each bundle is a
separate vehicle there, and the mod converts each as a car of its own, so a livery is never a vinyl.

### Tail pipes

The game starts the nitrous flames at the markers `EXHAUST`, `LEFT_EXHAUST` and `RIGHT_EXHAUST` of the car's body solid
(the marker's z axis points backwards), so a mod's own car has to carry them. A car without pipe markers boosts, but shows no
flame. The flame emitters themselves are the game's default `ecar` effect group. The 2012 files have tail pipe locators,
but their table is not decoded, so the 2012 mod places the pipes from the body's shape: at the rear face, at 22 percent of the
car's height, and at half the half width of the rear on either side. That is an approximation.

## Using it

1. Add the car (a mod with a `cars/<id>.toml`, or the [2012 mod](nfsmw-2012-content.md)).
2. Start the game, open the garage or the shop (a safe house or an auto shop in the world, My Cars in the main menu) and
   pick the car. The paints and parts its `[customization]` table lists, and the performance upgrades, are on the usual screens.
3. To change how the car drives, edit its `[stats]` table ([car-stats.md](car-stats.md); for the 2012 cars, the
   multipliers of `config.toml`) and restart.

## Limits

- **Body kits and hoods** are not offered: a mod car has none (a body variant is another car).
- **Saved careers.** My Cars slots remember the car's folder. The career's own garage record names a car by its car type,
  which for a mod car is the hash of its folder name; the 2005 database has no vehicle record of that name. A career save
  with a mod car has not been tested.
- **Part fit.** A rim, spoiler or roof scoop model is drawn as it is, at the place the mod modelled it. The game does not
  move it to fit the body.
- **Coverage.** The paints of the 2012 cars are checked against the real installs (the conversion of all 93 cars, and the
  garage catalogue and paint of one). The other parts of the table (rim paints, tints, rims, spoilers, roof scoops, vinyls)
  are covered by unit tests only, because no 2012 car uses them; a mod that does has to check them on its car.
- **Not played.** Nobody has driven the converted cars or used their shop in a window: the checks are the conversion of
  all 93 cars and the tests named on [nfsmw-2012-content.md](nfsmw-2012-content.md#tests).
