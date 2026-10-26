# Customizing mod cars: paint, parts, performance, nitrous and engine sound

Part of the [modding catalogue](README.md). A car a mod adds ([api-content.md](api-content.md#cars)) is a *clone*: it
keeps the record of a stock car, and with it the stock car's physics and sound. Its looks are its own ([Customization](#customization-customization-in-the-car-manifest)), and so can its engine ([Engine sound](#engine-sound-engine_sound-in-the-car-manifest)). This page says what the auto shop
# Customizing mod cars: paint, liveries, performance, nitrous and sound

Part of the [modding catalogue](README.md). A car a mod adds ([api-content.md](api-content.md#cars)) is a *clone*: it
keeps the record of a stock car, and with it the stock car's parts and physics (its sound is the mod's own: [Sound](#sound-engine_sound-and-car_sound-in-the-car-manifest)). This page says what the auto shop
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
| Paint finish, preset colours, custom RGB body colour | yes | the clone's paint parts; the body's paint surfaces take the colour |
| Rim paint, rims, window tint | yes | the clone's parts, the shared `WHEELS` models |
| **Vinyls** (liveries) and their colour channels | yes, the clone's | the clone's `VINYLS.BIN`, drawn over the paint (see [Vinyls](#vinyls)) |
| Spoilers and roof scoops | yes, the shared aftermarket models | marker `SPOILER` / `SPOILER2` / `ROOF_SCOOP` on the car's `<BASE>_BASE` solid |
| Performance upgrades (engine, transmission, tires, brakes, suspension, induction, nitrous) | yes, three levels each | steps of the car's own `[stats]` ([Upgrades](#upgrades)) |
| Nitrous | yes: from the factory (`nitrous = true`), or bought as a kit | see [Nitrous](#nitrous) |
| Body kits, hoods | no | they are models in the clone's own `GEOMETRY.BIN`, drawn for its body shape |
| Manufacturer decals, door numbers | not checked | |

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
on the car's body solid, so a mod's own car has to carry them (the 2012 mod derives them from the body, see [Tail pipes](#tail-pipes)). A car without pipe
markers boosts, but shows no flame.

## Sound: `[engine_sound]` and `[car_sound]` in the car manifest

A mod car has no record in the game's sound database (`pvehicle`, `engineaudio`, `ShiftSND`, `TurboSND` all belong to the
2005 game's own cars), and it plays nothing of any other car. What it sounds like is made of three things, and the
manifest can replace the first two:

| Sound | The car's own | Without one |
|---|---|---|
| Engine | `[engine_sound]` loops | the **built-in engine**, below |
| Gear shifts, turbo whistle and blow-off, backfires, pops | `[car_sound]` recordings | **silent**, except the gear clunk below |
| Collisions, scrapes, skids, nitrous, road noise, wind | none: shared by every car | always plays, from the game-wide banks |

**The shared sounds.** Collision and scrape sounds are the links of `pvehicle/cars`, the record every car of the game
derives from, so a car gets what a car gets when it sets nothing of its own; skids and nitrous come from the game-wide
bank lists (`audiosystem/mostwanted`); road noise, wind and the scrape loops are shared banks of `SOUND/IG_GLOBAL`.
None of them is a particular car's. The rpm shape of a gear change (how far the engine drops and
rises) is `shiftpattern/default`, and with it the class-wide gear bank `GEAR_SML_Base.abk`, so a car without
`shift` recordings still clunks into gear. The database has no class-wide turbo, sweetener or Ginsu engine sound
(`turbosfx/default` switches the turbo off, `engineaudio/default` has tuning numbers and no loops), so those are silent
unless the car gives them.

### Engine: `[engine_sound]`

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
  The volume follows the throttle and load.
- A layer that cannot be read leaves the car with the built-in engine, and the log says why.
- Traffic and cops are cars of the game and keep their own engines; a mod car plays its own engine only as the car the
  player drives.

**The built-in engine.** A car with no `[engine_sound]` plays an engine the game makes itself: five loops of a
six-cylinder four-stroke at 900, 1700, 2900, 4400 and 6500 rpm (idle 900, limiter 7000), mixed exactly as recorded
layers are (`blackbox_carsound::synth`, `audio/car/own_engine.rs`). Each loop is a whole number of engine cycles
built from the firing harmonics of the cylinders, the uneven "lope" of the other cycle multiples and a little low-passed
noise, and brightens with the rpm. No sample is read and none is any car's, which is why it is the default: the database
holds no engine sound that belongs to no car. It is a plain, neutral engine, not a good one; a car that matters brings
its own recordings.

### Other sounds: `[car_sound]`

```toml
[car_sound]
shift = ["SOUND/MODENGINE/FASTCOUPE/SHIFT0.WAV", "SOUND/MODENGINE/FASTCOUPE/SHIFT1.WAV"]
turbo_whine = ["SOUND/MODENGINE/FASTCOUPE/WHINE0.WAV", "SOUND/MODENGINE/FASTCOUPE/WHINE1.WAV"]
turbo_blowoff = ["SOUND/MODENGINE/FASTCOUPE/BLOWOFF0.WAV"]
backfire = ["SOUND/MODENGINE/FASTCOUPE/BACKFIRE0.WAV"]
pops = ["SOUND/MODENGINE/FASTCOUPE/POP0.WAV", "SOUND/MODENGINE/FASTCOUPE/POP1.WAV"]
```

Every key is optional and a list left out is silence for that effect. Each is a list of up to sixteen 16-bit PCM WAV
files (mono or stereo, paths of the install's layout like the engine's). Several files in a list are variants: one is
picked at random each time. A file that cannot be read is left out, and the log says why. The keys:

| Key | Plays | Notes |
|---|---|---|
| `shift` | at every gear change | at the moment the engine mixer asks for the gear clunk, at its level; **replaces** the shared clunk |
| `shift_up`, `shift_down` | at an up or a down shift | each falls back to `shift`; one given alone leaves the other direction to `shift` |
| `turbo_whine` | loops, from the lowest spool to the highest | all play at once, silent until the turbo spools; mixed along the spool in equal power and sped up from 0.8 to 1.4 times as it spools |
| `turbo_blowoff` | when the throttle is lifted with the turbo spooled | louder with more spool; the spool drops to 40 percent |
| `backfire` | when the throttle is lifted at high engine speed | likelier with the revs: one in five at idle speed, seven in ten at the limiter |
| `pops` | after a backfire (two to four, a tenth of a second and more apart), or alone | a lift-off without a backfire sometimes has one or two |

**These rules are the game's own, not any recording game's.** The turbo spools towards the throttle times how hard the
engine is turning (0.9 seconds up, 0.6 down); a "lift-off" is the engine mixer's *engine off* event (the throttle
released more than 55 percent of the way from idle to the limiter, in a gear above first, with no shift under way). When a recording plays, and
how loud, is chosen here; the 2012 game decides it in sound graphs that are not decoded
([chameleon-sound.md](../formats/chameleon-sound.md)).

### The 2012 cars' recordings

The 2012 game builds its engine sound in a sound graph of loops and synthesised harmonics, which this game does not
rebuild. The 2012 mod takes **the looping recordings of each car's bundle** ([chameleon-sound.md](../formats/chameleon-sound.md)),
measures the pitch of each, calls the lowest one the idle, and gives every other loop the rpm that its pitch stands for
(the idle rpm times its pitch over the lowest pitch). Loops that would need an rpm far beyond the limiter are left out.
This is an **approximation**: the five loops of the transmission whine graph are mixed as if they were engine loops (the
engine alone has too few pitches), and the loops are not the 2012 game's own blend.

Its other recordings are found by the **name of the live value each sound graph reads**: the samples that the graph
reading `GetPlayerNormalisedGearChangeTimeElapsed` plays are `shift` (the direction is not known, so it is `shift`, not
`shift_up` and `shift_down`); `BackfireGain` (and the variants listed beside it) `backfire`; `GetPopProbability` and
`PopGain` `pops`; `NormalisedTurboRPM` and `TurboToneGain` `turbo_whine` (the car's own loops and the 2012 game's shared
high tone, `VEHICLES/WAVES.BNDL`); `TurboBlowoffGain` `turbo_blowoff` (the shared one-shot that dies away over about two
and a half seconds). The evidence for each is in [chameleon-sound.md](../formats/chameleon-sound.md). A car whose bundle has no such
graph has no such sound: a car without a turbo has no whistle, and one without `TurboBlowoffGain` no blow-off.

`own_engine_sound = false` in the mod's `config.toml` writes no recordings: the cars play the built-in engine and have
only the shared sounds.

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
   pick the car. Paint, tint, rims, vinyls, spoilers and the performance upgrades are on the usual screens.
3. To change how the car drives, edit its `[stats]` table (for the 2012 cars, the multipliers of `config.toml`) and restart.

## Limits

- **Body kits and hoods** are not offered: they are models of the clone's own body.
- **Saved careers.** My Cars slots remember the car's folder. The career's own garage record names a car by its car type,
  which for a mod car is its clone's, so a clone and the mod cars that copy it share one career record. I have not
  tested a career save with a mod car.
- **Part fit.** A rim, spoiler or roof scoop model is drawn as it is, at the place the mod modelled it. The game does not
  move it to fit the body.
- **Coverage.** The paints of the 2012 cars are checked against the real installs (the conversion of all 93 cars, and the
  garage catalogue and paint of one). The other parts of the table (rim paints, tints, rims, spoilers, roof scoops, vinyls)
  are covered by unit tests only, because no 2012 car uses them; a mod that does has to check them on its car.
