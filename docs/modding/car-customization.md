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
| Performance upgrades (engine, transmission, tires, brakes, suspension, induction, nitrous) | yes, three levels each | steps of the car's own `[stats]` ([Upgrades](#upgrades)) |
| Nitrous | yes: from the factory (`nitrous = true`), or bought as a kit | see [Nitrous](#nitrous) |
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

## Stats: `[stats]` in the car manifest

A mod car is a car of its own. What it weighs, how its engine pulls, how its gearbox is spaced, what grip its tyres have
and how hard it brakes is in its manifest, in absolute SI units, and the game builds the car's physics from that table and
from the engine's documented generic values for what the table does not say. Nothing is taken from a stock car.

```toml
# cars/fastcoupe.toml
[stats]
mass_kg = 1397.0
front_weight_pct = 38.8            # share of the weight on the front axle
cg_height_m = 0.44
wheelbase_m = 2.45
front_track_m = 1.54
rear_track_m = 1.52
front_tire_radius_m = 0.34
rear_tire_radius_m = 0.343
idle_rpm = 1000.0
redline_rpm = 7800.0
torque_curve = [[1000.0, 360.0], [3300.0, 419.0], [5600.0, 440.0], [7800.0, 390.0]]   # [rpm, N m]
gear_ratios = [3.91, 2.29, 1.55, 1.3, 1.08, 0.88, 0.71]
reverse_ratio = 2.68
final_drive = 3.44
drive = "rwd"                      # rwd, fwd or awd
top_speed_kph = 304.0              # the air drag is solved so that the car reaches this
front_tire_grip = 1.5              # peak friction coefficient
rear_tire_grip = 1.4
front_brake_torque_nm = 1300.0
rear_brake_torque_nm = 1350.0
nitrous = true
nitrous_boost = 0.9
```

Every key is optional. A key left out takes the engine's generic car, a 1450 kg rear-drive saloon of about 290 hp
(`CarStats::default`, the numbers of `VehicleSpec::example`). A key outside its range, a key that is not in the table, a torque
curve that does not rise in rpm and an idle above the redline refuse the car; the log names the key.

| Key | Unit, range | Default | Builds |
|---|---|---|---|
| `mass_kg` | kg, 200 to 40000 | 1450 | body mass |
| `front_weight_pct` | percent, 20 to 80 | 52 | where the centre of gravity is along the wheelbase |
| `cg_height_m` | m, 0.1 to 2 | 0.5 | centre-of-gravity height (the chassis' roll centre) |
| `wheelbase_m` | m, 1 to 6 | 2.7 | wheelbase; the axles are centred on the body box |
| `front_track_m`, `rear_track_m` | m, 0.8 to 3.5 | 1.55 | track between the tyre centres |
| `front_tire_radius_m`, `rear_tire_radius_m` | m, 0.15 to 0.8 | 0.33 | outer tyre radius (the rim takes what a sidewall of 45 percent of the width leaves, then the aspect ratio makes the radius exact) |
| `front_tire_width_m`, `rear_tire_width_m` | m, 0.1 to 0.5 | 0.235, 0.255 | tyre width |
| `ride_height_m` | m, 0.03 to 0.5 | 0.12 | gap under the body |
| `idle_rpm`, `redline_rpm` | rpm, 300 to 3000, 1500 to 20000 | 900, 7000 | idle and rev limiter |
| `torque_curve` | `[rpm, N m]`, 2 to 16 points, rpm rising | 8 points, peak 359 N m | the engine's torque, sampled to 16 evenly spaced values from idle to the last point |
| `engine_braking` | share of the torque, 0 to 1 | 0.15 | closed-throttle drag |
| `gear_ratios` | 1 to 10 ratios, 0.2 to 10, falling | 6 gears, 3.6 to 0.75 | forward gears |
| `reverse_ratio`, `final_drive` | 0.5 to 10 | 3.4, 3.7 | |
| `gear_efficiency` | 0.5 to 1 | 0.9 | power kept by the gearbox |
| `drive`, `front_torque_share` | rwd, fwd, awd; 0 to 1 | rwd, 0.4 | which axle gets the torque; with `awd`, the front's share |
| `front_diff_lock`, `rear_diff_lock` | 0 to 1 | 0, 0.3 | differential lock (an all-wheel drive's centre differential is 0.5) |
| `top_speed_kph` | km/h, 50 to 700 | none | the air drag is solved so that the car, at full throttle in its top gear, reaches it (see below) |
| `drag` | N per (m/s)², 0.02 to 3 | 0.35 | air drag, used when there is no `top_speed_kph` |
| `downforce` | N per (m/s)², 0 to 0.1 | 0.003 | downforce |
| `front_tire_grip`, `rear_tire_grip` | friction coefficient, 0.3 to 3 | 1.2 | tyre grip, see below |
| `front_brake_torque_nm`, `rear_brake_torque_nm` | N m per wheel, 100 to 20000 | 1400, 950 | brakes |
| `handbrake_torque_nm` | N m, 0 to 20000 | 1500 | handbrake at the rear wheels |
| `steering_scale` | 0.5 to 2 | 1 | steering range |
| `front_ride_frequency_hz`, `rear_ride_frequency_hz` | Hz, 0.5 to 6 | 3.0, 3.1 | spring rate per wheel, `k = m (2 pi f)²` from the mass on that wheel |
| `damping_ratio`, `travel_m` | 0.1 to 1; 0.03 to 0.4 m | 0.7, 0.1 | dampers (`c = 2 zeta sqrt(k m)`, rebound 1.33 times), wheel travel |
| `induction_boost` | share of the torque at the red line, 0 to 1.5 | 0 | a turbo (for a torque curve without one) |
| `nitrous`, `nitrous_boost`, `nitrous_seconds` | bool; 0 to 3; 0.5 to 30 s | false, 0.5, 5 | [Nitrous](#nitrous) |

What the physics reads that the table does not give (flywheel mass, gear-change times, the body's ground and object contact,
yaw assists, the damper and anti-roll shapes, the downforce centre, wall contact) takes the engine's generic value; the list is
at the top of `libs/blackbox-vehicle/src/stats/build.rs`.

- **Tyre grip.** `front_tire_grip` is a friction coefficient (1.0 an ordinary road tyre, 1.5 a sports tyre). The engine's
  tyre model is calibrated on a high-performance tyre whose `STATIC_GRIP` is 2.075 and `DYNAMIC_GRIP` 1.85
  (`docs/specs/vehicle-calibration.md`), which the builder calls a coefficient of 1.65: a car's grip and its lateral curve
  scale with `coefficient / 1.65`. That scale is a calibration, not a measurement of every car (**not yet checked by a skidpad
  run**): if the cars handle too stickily or too loosely, change `GRIP` constants in `build.rs`, not the cars.
- **Top speed.** At the top speed the drag force equals the wheel force of the engine in top gear: the engine speed at a road
  speed follows the game's drivetrain, and `drag = wheel force / speed²`. A top speed the car cannot reach before the rev
  limiter gives the speed at the limiter (a little under it). Upgrades keep the drag the stock car got.
- A car with no table is the generic car above.

## Upgrades

The shop offers every category three levels (no junkman parts). A level changes the car's own numbers by a share of
what it has, so a strong engine gains more newton metres than a weak one (`libs/blackbox-vehicle/src/stats/upgrades.rs`):

| Category | Per level |
|---|---|
| Tires | grip + 5 percent of the car's own, both axles |
| Brakes | brake torque + 10 percent |
| Suspension | spring frequency + 6 percent, damping ratio + 0.03, ride height - 8 mm, centre of gravity - 15 mm, downforce + 15 percent |
| Transmission | gear efficiency + 0.03 (at most 1) |
| Engine | torque + 6 percent at every engine speed |
| Induction | `induction_boost` + 0.08 |
| Nitrous | level 1 is the car's factory nitrous (a car without one gets a kit of 0.4 boost for 4 s); each further level + 15 percent boost, + 25 percent burn time |

The physics of an upgraded car are `physics_from_stats(stats, package, bounds)`; the garage preview, the world, split screen
and the AI use the same function.

### The shop's bars

The three bars of the shop (top speed, handling, acceleration) are computed from the car's stats. Top speed and acceleration
are the shop's longitudinal estimate (`docs/specs/garage-performance-bars.md`) of the car built from its stats, put on the
scale of the stock cars (the lowest and highest of the stock cars and their maximum builds are 0 and 1, and a faster car is
clamped to 1). Handling is the score of the car's grip, `0.75 x cornering + 0.25 x braking` on a scale from 1.0 g (0) to
1.9 g (1), where cornering is the mean tyre coefficient less up to 25 percent for a high centre of gravity on a narrow track
(`libs/blackbox-vehicle/src/stats/rating.rs`). The bars of the maximum build are the same estimate for the car with every
category at level 3.

## Nitrous

A stock car has no nitrous until its owner buys the kit. A mod car with `nitrous = true` has it from the factory: the extra
torque while it burns is `nitrous_boost` times the engine's (`0.9` is 90 percent more) and a full tank lasts
`nitrous_seconds`. When the player buys a nitrous level in the shop, level 1 is the factory nitrous, and the levels above
make it stronger and longer ([Upgrades](#upgrades)); a car without nitrous gets a kit of 0.4 boost for 4 seconds. The
tank refills by speed (20 s at 20 mph, 5 s at 100 mph).

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
3. To change how the car drives, edit its `[stats]` table (for the 2012 cars, the multipliers of `config.toml`) and restart.

## Limits

- **Body kits and hoods** are not offered: they are models of the clone's own body.
- **Saved careers.** My Cars slots remember the car's folder. The career's own garage record names a car by its car type,
  which for a mod car is its clone's, so a clone and the mod cars that copy it share one career record. I have not
  tested a career save with a mod car.
- **Spoiler and scoop fit.** The mounts are the clone's, moved up or back to the new body's surface by at most 35 cm. A
  body very different from its clone's can leave a wing a little high.
- The 2012 mod's cars are real-install checked for the items above; a car a mod makes by hand has to be checked by that
  mod's author.
