# Customizing mod cars: paint, liveries, performance, nitrous and engine sound

Part of the [modding catalogue](README.md). A car a mod adds ([api-content.md](api-content.md#cars)) is a *clone*: it
keeps the record of a stock car, and with it the stock car's parts, physics and sound (the engine can be its own: [Engine sound](#engine-sound-engine_sound-in-the-car-manifest)). This page says what the auto shop
(the garage, the career shop and the F6 garage) does with such a car, how a mod sets how it drives, and what a mod that
brings its own car has to put in its files for the shop to work on it. The
[2012 cars mod](nfsmw-2012-content.md) is the worked example, and its settings are in its own page.

## What the shop offers a mod car

| Auto shop item | Mod car | Where it comes from |
|---|---|---|
| Paint finish, preset colours, custom RGB body colour | yes | the clone's paint parts; the body's paint surfaces take the colour |
| Rim paint, rims, window tint | yes | the clone's parts, the shared `WHEELS` models |
| **Vinyls** (liveries) and their colour channels | yes, the clone's | the clone's `VINYLS.BIN`, drawn over the paint (see [Vinyls](#vinyls)) |
| Spoilers and roof scoops | yes, the shared aftermarket models | marker `SPOILER` / `SPOILER2` / `ROOF_SCOOP` on the car's `<BASE>_BASE` solid |
| Performance upgrades (engine, transmission, tires, brakes, suspension, induction, nitrous) | yes, the clone's | the clone's `pvehicle` record, then the car's `[performance]` on top |
| Nitrous | yes: bought as a kit, or from the factory with `nitrous = true` | see [Nitrous](#nitrous) |
| Body kits, hoods | no | they are models in the clone's own `GEOMETRY.BIN`, drawn for its body shape |
| Manufacturer decals, door numbers | not checked | |

The garage lists the car under the display name and brand its manifest gives (`name`, `manufacturer`); without a brand
the clone's brand shows.

To look at a car's paint and liveries without starting the world:

```
nfsmw view-car POR911CARR12 --part paint=40 --part vinyl=30 --part spoiler=8 --yaw 55 --screenshot out.png --muted
```

The number after `=` counts from 1 in the order the `parts` console command lists (F12, then `parts` in a world with the
car; `parts paint 40 vinyl 30` applies them there). The categories are `body`, `spoiler`, `hood`, `roof`, `rims`, `paint`,
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
on the car's body solid, so a mod's own car has to carry them (the 2012 mod copies its clone's). A car without pipe
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

## What a mod's own car needs for the shop to work

The shop treats a mod car like its clone and reads the car's files for the rest. For a car with its own model:

1. **Body solids** named `<BASE>_KIT00_BODY_A` to `_D`, where `<BASE>` is the clone's `base_model_name`
   (`SL500` for the SL500 clone), with the clone's pipe markers on them.
2. **A base solid** `<BASE>_BASE_A` to `_D` that carries the markers of the spoiler and roof-scoop mounts and of the
   lamps. A stock car's base is the cabin shell; a mod car whose body is all in `KIT00_BODY` can make its base one
   triangle with no area, and put the clone's markers on it. Without it the shop offers no spoiler and no roof scoop,
   and the brake and head lights have no flare.
3. **Paint surfaces** that use the texture `<BASE>_SKIN1` and the light material `CARSKIN`. The game swaps that texture for
   the paint colour (and for a vinyl, below). Everything else (glass, trim, lamps, tyres) uses its own textures.
4. **Texture coordinates on the paint surfaces in the clone's layout** if the car is to take the clone's vinyls.

### Vinyls

A vinyl is a picture authored in the clone's paint texture layout: the hood is at one place of the picture, the doors at
another. For a mod car the shop reads the vinyls from the clone's folder (`VINYLS.BIN`, `PREVINYL.BIN`), after any the mod
car has in its own folder, so the car offers exactly the clone's list. The game draws the picture on the paint surfaces
through their texture coordinates, so a car whose coordinates are its own would show the stripes in the wrong places.

The 2012 mod therefore gives every point of the 2012 paint surface the coordinates of the nearest point of the clone's
paint surface (`mods/nfsmw-2012-content/src/liveries.rs`): a stripe over the clone's roof runs over the 2012 car's
roof. A triangle whose corners fall on two different parts of the clone's layout is given one texel, and a vertex is
split where its triangles disagree. It is a nearest-point match, not an unwrapping: it follows the large panels well and
is rougher at the bumpers and around the lamps.

A mod that makes its own car can do the same, or ship its own `VINYLS.BIN` in the car's folder with pictures in the car's
own layout (the bitmap `<BASE>_<PART>` and its mask `<BASE>_<PART>_MASK`, as the stock files name them).

## Using it

1. Add the car (a mod with a `cars/<id>.toml`, or the [2012 mod](nfsmw-2012-content.md)).
2. Start the game, open the garage or the shop (a safe house or an auto shop in the world, My Cars in the main menu) and
   pick the car. Paint, tint, rims, vinyls, spoilers and the performance upgrades are on the usual screens.
3. To change how the car drives, edit its `[performance]` table (for the 2012 cars, `config.toml`) and restart.

## Limits

- **Body kits and hoods** are not offered: they are models of the clone's own body.
- **The performance bars** show the clone's ratings (see above).
- **Saved careers.** My Cars slots remember the car's folder. The career's own garage record names a car by its car type,
  which for a mod car is its clone's, so a clone and the mod cars that copy it share one career record. I have not
  tested a career save with a mod car.
- **Spoiler and scoop fit.** The mounts are the clone's, moved up or back to the new body's surface by at most 35 cm. A
  body very different from its clone's can leave a wing a little high.
- The 2012 mod's cars are real-install checked for the items above; a car a mod makes by hand has to be checked by that
  mod's author.
