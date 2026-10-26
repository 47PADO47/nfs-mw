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
| [car-damage-parts.md](car-damage-parts.md) | `[[damage_parts]]`: bonnet, bumpers, mirrors and doors that turn or fall off as the car is damaged |
| [car-lamps.md](car-lamps.md) | the lamp markers and lamp textures: flares and lit surfaces of the head, brake and reverse lamps |

The [2012 cars mod](nfsmw-2012-content.md) is the worked example, and its settings are on its own page.

## The files of a car

A car is the folder `files/CARS/<ID>/` of a mod plus the manifest `cars/<id>.toml` ([api-content.md](api-content.md#cars)):

| File | Needed | Holds |
|---|---|---|
| `GEOMETRY.BIN` | yes | the body and wheel solids, in the install's format, **under the names below** |
| `TEXTURES.BIN` | yes | their textures |
| `BOUNDS.BIN` | yes, to drive | the car's own collision bounds ([car-collision.md](car-collision.md)); the garage leaves out a car whose physics cannot be built, and the game refuses it with the reason in the log |
| `VINYLS.BIN` | only for `vinyls` | the vinyl pictures, a texture pack (`PREVINYL.BIN` is read too) |
| `SOUND/...` WAV files | only for `[engine_sound]`, `[car_sound]` | [car-sound.md](car-sound.md) |

The game builds the car from its own solids, named after its folder (`<ID>` is the folder, in upper case, as in the
manifest's file name). A solid it does not find is not drawn, and a car with neither a body nor a wheel solid does not show:

| Solid | Is |
|---|---|
| `<ID>_KIT00_BODY_A` to `_D` | the body, with the glass, lamps and interior, the tail pipe markers ([Tail pipes](#tail-pipes)) and the lamp markers ([car-lamps.md](car-lamps.md)) |
| `<ID>_KIT00_FRONT_TIRE_A` to `_D` | one wheel solid: tyre, rim, brake disc and caliper, axle along y, rim face at y = 0, as the game's wheels are modelled; the car draws it at all four corners and the game turns it for the left side |

The letters are the levels of detail, `A` the finest; `E` is not looked for, and a level a car lacks draws nothing at that
distance, so A and B are the finest the player sees. The paint placeholder texture is `<ID>_SKIN1`, on the paint surfaces.
Solids are in car space (+x forward, +y left, +z up), centred on the wheelbase: the wheels sit at plus and minus half the
`wheelbase_m` of `[stats]` ([car-stats.md](car-stats.md)).

The game finds these through two parts it adds to its parts database for the car (`car/mods/parts.rs`): the body and the wheel,
each of the car's own type, with these solids as their models. Nothing else is added: the car has no brake part (the wheel
solid holds the discs), no hood, spoiler, roof, damage overlay, decal or window tint part, and its paint, rims and tints are
the choices of its `[customization]` table. The 2005 parts database allows 256 car and part-group names, about 106 of which
the game uses, so a game that publishes more than about 150 mod cars leaves the later ones without a body part (the log says
so). Hand-made cars must use the names above: the 2012 mod writes exactly these, and its real-install test loads them.

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
| **Liveries** | yes, listed with the vinyls | `liveries`: whole-body pictures in the car's own `TEXTURES.BIN` ([Liveries](#liveries)) |
| Performance upgrades (engine, transmission, tires, brakes, suspension, induction, nitrous) | yes, three levels each | steps of the car's own `[stats]` ([car-stats.md](car-stats.md#upgrades)) |
| Nitrous | yes: from the factory (`nitrous = true`), or bought as a kit | [car-stats.md](car-stats.md#nitrous) |
| Body kits, hoods, manufacturer decals, door numbers | no | a mod car has no such parts; a mod that wants a body variant adds another car |

A car that gives no `[customization]` table offers the shop nothing of its own: its paint is what its textures hold and the
shop lists no choice for it.

The garage lists the car under the display name and brand its manifest gives (`name`, `manufacturer`; without a name it shows
the car id, without a brand none). `nfsmw_data::car::mods::titled` is the one place that decides the name and brand a screen
shows for a car, and the screens built on the `Car_Select` package (the garage's car page, the car lot, the custom race's car
select, Quick Play) write a mod car's name into the package's two name strings, which the game fills with the model-name picture
of a stock car and the sample text `temp123`/`tempXXX` otherwise (`frontend/car_title.rs`).

**The developer tools.** The F12 `parts` command and the F6 garage panel list and apply a mod car's own choices (paints, rim
paints, tints, rims, spoilers, scoops, vinyls, and the liveries among the vinyls) from the same catalogue as the auto shop
(`Inventory::catalog`), for the car being driven (not for the car the panel has selected for browsing), and read the inventory
again when a mod car was published after it was loaded. In a world with a converted Nissan Skyline R34 GT-R, `parts vinyl 3`
and `view-car NISSKYLINE99 --part vinyl=3` put its third livery on the car.

To look at a car's paint without starting the world:

```
nfsmw view-car POR911CARR12 --part paint=3 --screenshot out.png --muted
```

The number after `=` counts from 1 in the order the `parts` console command lists (F12, then `parts` in a world with the
car; `parts paint 3` applies it there; both come from `garage::listing::arrange`, which puts each category in name order and
numbers choices that share a name, so a number means the same part in the viewer, the console and the F6 panel). The categories are `body`, `spoiler`, `hood`, `roof`, `rims`, `paint`,
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
liveries = [{ name = "Livery 1", texture = "FASTCOUPE_LIV1" }]       # a picture in the car's TEXTURES.BIN
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
| `liveries` | `texture` is a texture name in the car's own `TEXTURES.BIN`, in the layout of the car's paint surfaces. See [Liveries](#liveries). |

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

### Liveries

A livery is a whole-body picture, authored for the car's own paint texture coordinates, laid over the body paint **by its
alpha**: where the picture is opaque it replaces the paint, where it is transparent the paint shows (the rule of the 2012
game's livery shader, [chameleon-customization.md](../formats/chameleon-customization.md#liveries)). It differs from a vinyl
in three ways: the picture carries its own colours (there are no recolourable channels), its alpha is its mask (there is no
mask texture), and it lives in `TEXTURES.BIN`, not `VINYLS.BIN`. The picture may be DXT1 with one-bit alpha, DXT3, DXT5 or
a 32-bit texture; DXT1 is read with its one-bit alpha.

The shop lists the liveries among the **vinyls** (the vinyl category: a livery and a vinyl are both a picture over the paint),
after the vinyls, in the order of the list; choosing none keeps the plain paint. A livery is not offered, and the log says
why, when its texture is missing from the car's `TEXTURES.BIN` or cannot be decoded. A livery may not take the name of a
vinyl or of another livery. The choice is told apart like the others, by a hash of the folder, the kind (`LIVERY`) and the
name. A livery is drawn on every paint surface of the car (they all share one texture layout); the paint the player picked
shows only where the picture is transparent.

### The 2012 cars

The [2012 mod](nfsmw-2012-content.md) fills `paints`, with the colour records of each car's own bundle (1 to 15 per
car, 396 in the 93 bundles), and `liveries`: that is all the 2012 game offers for a car's looks (no vinyls, rims, spoilers,
roof scoops or tints to choose; [chameleon-customization.md](../formats/chameleon-customization.md)). The paint texture
coordinates of a converted car are the 2012 car's own. The 2012 game ships five vehicles (the BMW M3 GTR, Lamborghini Diablo SV,
Porsche 911 GT2, Nissan 350Z and Nissan Skyline R34) in five liveries each, a bundle per livery; the mod makes one car of each
and lists the five pictures as `Livery 1` to `Livery 5` (73 cars from 93 bundles).

### Tail pipes

The game starts the nitrous flames at the markers `EXHAUST`, `LEFT_EXHAUST` and `RIGHT_EXHAUST` of the car's body solid
(the marker's z axis points backwards), so a mod's own car has to carry them. A car without pipe markers boosts, but shows no
flame. The flame emitters themselves are the game's default `ecar` effect group. The 2012 files have tail pipe locators,
and the 2012 mod writes one marker for each of a car's pipes from them ([chameleon-exhaust.md](../formats/chameleon-exhaust.md)),
none for a car without a pipe (an electric car boosts with no flame). Only a car whose bundle has no effect records gets
pipes derived from its body's shape.

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
- **Livery blending.** A livery's picture is flattened over the paint into one texture per choice, without mip levels, as the
  vinyls are; the flat paint of a car with no livery is a 4 x 4 texture. Two liveries cost one texture of the picture's size
  each in memory while chosen. A livery changes only the paint surfaces' texture: lamps, glass and the rest of the car are
  unchanged.
- **Coverage.** The paints of the 2012 cars are checked against the real installs (the conversion of all 93 bundles, and the
  garage catalogue and paint of one). The other parts of the table (rim paints, tints, rims, spoilers, roof scoops, vinyls)
  are covered by unit tests only, because no 2012 car uses them; a mod that does has to check them on its car.
- **Not played.** Nobody has driven the converted cars or used their shop in a window: the checks are the conversion of
  all 93 cars and the tests named on [nfsmw-2012-content.md](nfsmw-2012-content.md#tests).
