# Customizing mod cars: paint, parts, performance, nitrous and engine sound

Part of the [modding catalogue](README.md). A car a mod adds ([api-content.md](api-content.md#cars)) is a *clone*: it
keeps the record of a stock car, and with it the stock car's physics and sound. Its looks are its own ([Customization](#customization-customization-in-the-car-manifest)), and so can its engine ([Engine sound](#engine-sound-engine_sound-in-the-car-manifest)). This page says what the auto shop
(the garage, the career shop and the F6 garage) does with such a car, how a mod sets how it drives, and what a mod that
brings its own car has to put in its files for the shop to work on it. The
[2012 cars mod](nfsmw-2012-content.md) is the worked example, and its settings are in its own page.

## What the shop offers a mod car

The auto shop (the garage, the career shop and the F6 garage) offers a mod car **what the car's own `[customization]` table
lists** and nothing from a stock car's parts table. The table and the files in the car's folder (`files/CARS/<ID>/`) are the
whole source:

| Auto shop item | Mod car | Where it comes from |
|---|---|---|
| Paint finish, colours, custom RGB body colour | yes | `paints`: each has a name, a colour and a finish; one is the car's paint until the player picks another |
| Rim paint | yes | `rim_paints` |
| Window tint | yes | `tints`: a colour and an opacity |
| Rims | yes | `rims`: wheel models in the car's own `GEOMETRY.BIN` |
| Spoilers, roof scoops | yes | `spoilers`, `roof_scoops`: models in the car's own `GEOMETRY.BIN` |
| **Vinyls** and their colour channels | yes | `vinyls`: pictures in the car's own `VINYLS.BIN`; `vinyl_colors` is the palette |
| Performance upgrades (engine, transmission, tires, brakes, suspension, induction, nitrous) | yes | see [Performance](#performance-performance-in-the-car-manifest) |
| Nitrous | yes: bought as a kit, or from the factory with `nitrous = true` | see [Nitrous](#nitrous) |
| Body kits, hoods, manufacturer decals, door numbers | no | a mod car has no such parts; a mod that wants a body variant adds another car |

A car that gives no table gives the shop nothing of its own to offer: its paint is what its files say and the shop lists no
choice for it. (While a car can still name a stock car in `clone`, the shop falls back to that car's parts for a car with no
table; that fallback is going away with `clone`.)

The garage lists the car under the display name and brand its manifest gives (`name`, `manufacturer`).

To look at a car's paint and liveries without starting the world:

```
nfsmw view-car POR911CARR12 --part paint=3 --screenshot out.png --muted
```

The number after `=` counts from 1 in the order the `parts` console command lists (F12, then `parts` in a world with the
car; `parts paint 3` applies it there). The categories are `body`, `spoiler`, `hood`, `roof`, `rims`, `paint`,
`tint`, `rimpaint` and `vinyl`.

## Performance: `[performance]` in the car manifest

A car's manifest may carry a `[performance]` table. Every number is a **multiplier of the clone's own value**, so the
table says how the car differs from its clone and leaving a key out (or `1.0`) keeps the clone.

```toml
# cars/fastcoupe.toml
clone = "CAMARO"
name = "Fast Coupe"

[performance]
power = 1.15          # engine torque at every rpm
mass = 0.95           # body mass
grip = 1.05           # tyre grip, lateral and longitudinal
brakes = 1.2          # brake torque, both axles
nitrous = true        # the car has nitrous from the factory
nitrous_power = 1.25  # torque the nitrous adds, times the kit's own
```

| Key | Range | Default | Changes |
|---|---|---|---|
| `power` | 0.25 to 4 | 1.0 | every point of the engine torque curve |
| `mass` | 0.25 to 4 | 1.0 | the body mass |
| `grip` | 0.25 to 3 | 1.0 | the tyre grip scale and the static and dynamic friction of both axles |
| `brakes` | 0.25 to 4 | 1.0 | the brake torque of both axles |
| `nitrous` | `true` or `false` | `false` | whether the car has nitrous without buying the kit |
| `nitrous_power` | 0 to 4 | 1.0 | the torque the nitrous adds; `0` switches a car's nitrous off |

- A number outside its range, or a key that is not in the table, refuses the car; the log names the key.
- The multipliers are applied **after the upgrades** the player has bought. A car with `power = 1.15` is 15 percent
  stronger at stock, with the engine kit and with the junkman part, because the upgrade picks the clone's engine record
  and the multiplier then scales it.
- They apply wherever the car's physics are built: the garage preview, the world, split screen, and the AI when the car
  is a cop car. A car that asks for nothing is built exactly as its clone.
- The performance bars in the shop (top speed, handling, acceleration) are the clone's: they come from the frontend's
  own rating table, which does not know the multipliers. The car does drive as the manifest says.

## Nitrous

A stock car has no nitrous until its owner buys the kit. A mod car with `nitrous = true` has the clone's **first
nitrous kit** from the factory. If the clone sells no kit, the car gets a typical one (a 5 second tank, half again the
torque, a recharge that depends on speed). When the player buys a nitrous level in the shop, that level replaces the
factory kit; `nitrous_power` is applied to whichever is fitted.

The flames come out of the tail pipes. The game finds a pipe by the markers `EXHAUST`, `LEFT_EXHAUST` and `RIGHT_EXHAUST`
on the car's body solid, so a mod's own car has to carry them (the 2012 mod derives them from the body, see [Tail pipes](#tail-pipes)). A car without pipe
markers boosts, but shows no flame.

## Engine sound: `[engine_sound]` in the car manifest

A mod car sounds like its clone: the engine, the shifts, the turbo and the collisions all come from the clone's sound
set. A mod car may give **its own engine loops**, which play instead of the clone's engine (the rest of the clone's
sounds stay):

```toml
[engine_sound]
idle_rpm = 820.0       # where the engine idles
redline_rpm = 7500.0   # where its limiter holds: the game's 0 to 1 engine speed spans idle to redline
layers = [
  { file = "SOUND/MODENGINE/FASTCOUPE/IDLE.WAV", rpm = 820.0 },
  { file = "SOUND/MODENGINE/FASTCOUPE/HIGH.WAV", rpm = 5200.0 },
]
```

- A layer is a looping recording of the engine made at the engine speed `rpm`: a 16-bit PCM WAV, mono or stereo, that
  repeats over its whole length. The file is a path of the install's layout, so it goes in the mod's `files/` folder.
- One to eight layers, `rpm` from 100 to 20000, no two the same, `idle_rpm` below `redline_rpm`. A wrong table refuses the car.
- All layers play at once. Each is played at the engine's rpm over its own (so its pitch follows the engine) but never
  slower than 0.55 or faster than 1.8 times its recording, and the layers are mixed by how close the engine's rpm is
  to theirs, in equal power, on a log scale. Below the lowest and above the highest layer one layer carries the sound.
  The volume is the clone's engine volume (throttle and load).
- A layer that cannot be read leaves the car with its clone's engine sound, and the log says why.
- Only the car the player drives uses its loops; traffic and cops keep the clone's engine.

The 2012 mod fills this table from the car's own bundle (see below). A real recording at the right rpm for every part of
the range gives the best sound; few layers stretched far sound thin.

### The 2012 cars' engine loops

The 2012 game builds its engine sound in a sound graph of loops and synthesised harmonics, which this game does not
rebuild. The 2012 mod takes **the looping recordings of each car's bundle** ([chameleon-sound.md](../formats/chameleon-sound.md)),
measures the pitch of each, calls the lowest one the idle, and gives every other loop the rpm that its pitch stands for
(the idle rpm times its pitch over the lowest pitch). Loops that would need an rpm far beyond the limiter are left out.
This is an **approximation**: a loop of the bundle that is not a recording of the engine at one speed (a whine, a
drone) is mixed as if it were, and the loops are not the 2012 game's own blend. It is the car's own recording, not its
clone's. `own_engine_sound = false` in the mod's `config.toml` keeps the clone's engine instead.

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
their texture coordinates, so the mod authors the picture for its own coordinates. The 2012 cars give none (see below). The
bitmap and the mask are texture packs entries of `VINYLS.BIN`; the shop composes the layer (`vinyl::compose`) and draws it
over the paint colour.

### The 2012 cars

The [2012 mod](nfsmw-2012-content.md) fills only `paints`, with the colour records of each car's own bundle (1 to 15 per
car, 396 in the 93 cars), because that is all the 2012 game offers for a car's looks: it has no vinyls, rims, spoilers,
roof scoops or tints to choose, and its paint shader takes a flat colour and has no artwork layout
([chameleon-customization.md](../formats/chameleon-customization.md)). The paint texture coordinates of a converted car
are the 2012 car's own. The mod names the solids and the placeholder after the car's folder (`<ID>_KIT00_BODY_A`,
`<ID>_SKIN1`) and puts the tail pipe markers (`LEFT_EXHAUST`, `RIGHT_EXHAUST`) on the body, derived from its shape.

What the conversion used to do and no longer does: it gave the 2012 paint surface the texture coordinates of the nearest
point of a stock car's paint surface so that stock car's vinyls would fit, named its solids after the stock car, and copied
the stock car's spoiler, roof-scoop and lamp markers. None of that is left.

### Tail pipes

The game starts the nitrous flames at the markers `EXHAUST`, `LEFT_EXHAUST` and `RIGHT_EXHAUST` of the car's body solid
(the marker's z axis points backwards). A car without pipe markers boosts, but shows no flame. The 2012 files have tail pipe
locators, but their table is not decoded, so the 2012 mod places the pipes from the body's shape: at the rear face, at 22
percent of the car's height, and at half the half width of the rear on either side. That is an approximation.

## Collision and impact geometry

A car's collision is its own file, `BOUNDS.BIN` in the car's `files/CARS/<ID>/` folder, whatever the car's other files take
from elsewhere: the root box its rigid body is built from, its contact boxes and spheres, the mesh it rolls on, and the zones its
damage is counted in (taken from the root box). The file is required for the car to drive. Its contents, the checks the game
makes, the damage numbers and how a mod writes one are in [car-collision.md](car-collision.md). The 2012 mod fits it to the
collision mesh of each 2012 car ([nfsmw-2012-content.md](nfsmw-2012-content.md)).

## Using it

1. Add the car (a mod with a `cars/<id>.toml`, or the [2012 mod](nfsmw-2012-content.md)).
2. Start the game, open the garage or the shop (a safe house or an auto shop in the world, My Cars in the main menu) and
   pick the car. The paints and parts its `[customization]` table lists, and the performance upgrades, are on the usual screens.
3. To change how the car drives, edit its `[performance]` table (for the 2012 cars, `config.toml`) and restart.

## Limits

- **Body kits and hoods** are not offered: a mod car has none (a body variant is another car).
- **The performance bars** show the clone's ratings (see above).
- **Saved careers.** My Cars slots remember the car's folder. The career's own garage record names a car by its car type,
  which for a mod car is its clone's, so a clone and the mod cars that copy it share one career record. I have not
  tested a career save with a mod car.
- **Part fit.** A rim, spoiler or roof scoop model is drawn as it is, at the place the mod modelled it. The game does not
  move it to fit the body.
- **Coverage.** The paints of the 2012 cars are checked against the real installs (the conversion of all 93 cars, and the
  garage catalogue and paint of one). The other parts of the table (rim paints, tints, rims, spoilers, roof scoops, vinyls)
  are covered by unit tests only, because no 2012 car uses them; a mod that does has to check them on its car.
