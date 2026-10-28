# Sound of a mod car: `[engine_sound]` and `[car_sound]`

Part of the [modding catalogue](README.md). A car a mod adds ([api-content.md](api-content.md#cars)) has no record in the
game's sound database (`pvehicle`, `engineaudio`, `ShiftSND`, `TurboSND` all belong to the 2005 game's own cars), and it plays
nothing of any other car. Its stats are in [car-stats.md](car-stats.md), its shop in
[car-customization.md](car-customization.md).

## Sound: `[engine_sound]` and `[car_sound]` in the car manifest

What a mod car sounds like is made of four things, and the manifest can replace the first three:

| Sound | The car's own | Without one |
|---|---|---|
| Engine | `[engine_sound]` loops | the **built-in engine**, below |
| Gear shifts, turbo whistle and blow-off, backfires, pops | `[car_sound]` recordings | **silent**, except the gear clunk below (which `shift_clunk = false` silences) |
| Collisions and scrapes | the mod's shared recordings, `[car_sound.shared]` | the game-wide banks |
| Skids, nitrous, road noise, wind | none: shared by every car | always plays, from the game-wide banks (the wind may be replaced by a mod's [wind variant](effect-variants.md#profile-format-of-the-wind)) |

**The game-wide sounds.** Collision and scrape sounds (for a car with no `[car_sound.shared]`) are the links of `pvehicle/cars`, the record every car of the game
derives from, so a car gets what a car gets when it sets nothing of its own (`generic_car_sound`,
`crates/nfsmw-data/src/sound/car.rs`); skids and nitrous come from the game-wide bank lists (`audiosystem/mostwanted`); road
noise, wind and the scrape loops are shared banks of `SOUND/IG_GLOBAL`. None of them is a particular car's. The rpm shape of
a gear change (how far the engine drops and rises) is `shiftpattern/default`, and with it the class-wide gear bank
`GEAR_SML_Base.abk`, so a car without `shift` recordings still clunks into gear. The database has no class-wide turbo,
sweetener or Ginsu engine sound (`turbosfx/default` switches the turbo off, `engineaudio/default` has tuning numbers and no
loops), so those are silent unless the car gives them ([specs/engine-sound.md](../specs/engine-sound.md) section 2.4).

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
  repeats over its whole length. The file is a path of the install's layout (letters, digits, `_`, `-`, `.` and `/`, up to 96
  characters, ending in `.WAV`), so it goes in the mod's `files/` folder.
- One to twelve layers, `rpm` from 100 to 20000 (a layer with `gain` curves has none, see [Curves](#curves-layers-that-play-by-the-cars-own-inputs)), no two the same, `idle_rpm` below `redline_rpm` (both 100 to 20000). A wrong
  table refuses the car.
- All layers play at once. Each is played at the engine's rpm over its own (so its pitch follows the engine) but never
  slower than 0.55 or faster than 1.8 times its recording, and the layers are mixed by how close the engine's rpm is
  to theirs, in equal power, on a log scale. Below the lowest and above the highest layer one layer carries the sound.
  The volume follows the throttle and load.
- A layer that cannot be read or played leaves the car with the built-in engine (not half an engine), and the log says why.
- A mod car plays its own engine as the car the player drives. Whether a mod cop car in a chase is heard with it was not
  checked.

**The built-in engine.** A car with no `[engine_sound]` plays an engine the game makes itself: five loops of a
six-cylinder four-stroke at 900, 1700, 2900, 4400 and 6500 rpm (idle 900, limiter 7000), mixed exactly as recorded
layers are (`blackbox_carsound::synth`, `crates/nfsmw/src/audio/car/own_engine.rs`). Each loop is a whole number of engine
cycles built from the firing harmonics of the cylinders, the uneven "lope" of the other cycle multiples and a little
low-passed noise, and brightens with the rpm. No sample is read and none is any car's, which is why it is the default: the
database holds no engine sound that belongs to no car. It is a plain, neutral engine, not a good one; a car that matters
brings its own recordings.

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
| `shift_trigger` | (`"mixer"` by default, or `"gear"`) | what sets the shifts off: `"mixer"` is the engine mixer's gear clunk, `"gear"` is every change of the gear number, see below |
| `shift_clunk` | (a flag, `true` by default) | `shift_clunk = false` makes a gear change **silent** when the car has no `shift` recording for it, instead of the class-wide clunk of the game's gear bank; an electric car has no gear to clunk into. It is a table of its own: a car may have it with no list |
| `turbo_whine` | loops, from the lowest spool to the highest | all play at once, silent until the turbo spools; mixed along the spool in equal power and sped up from 0.8 to 1.4 times as it spools |
| `turbo_blowoff` | when the throttle is lifted with the turbo spooled | louder with more spool; the spool drops to 40 percent |
| `backfire` | when the throttle is lifted at high engine speed | likelier with the revs: 3 percent of the lifts at idle speed, a quarter at the limiter (a car with a [`[car_sound.overrun]`](#the-overrun-backfire-car_soundoverrun) table backfires by that instead) |
| `pops` | after a backfire (two to four, a tenth of a second and more apart), or alone | a lift-off without a backfire sometimes has one or two |

**`shift_trigger = "gear"`.** By default a shift plays when the engine mixer asks for the 2005 cars' gear clunk, which waits a
little after the gear changes and ignores an up shift under 3000 rpm. With `"gear"` the shift recording plays at **every**
change of the gear number, in the frame it happens, whatever the engine and road speed: `shift_up` when the new gear is higher
than the last frame's, `shift_down` when it is lower (the first gear seen is not a change). The mixer's clunk is then dropped
for a direction that has recordings; a direction with none keeps the rules of `shift_clunk` and `shift`. The recording
plays at the full one-shot level.

**These rules are the game's own, not any recording game's.** The turbo spools towards the throttle times how hard the
engine is turning (0.9 seconds up, 0.6 down); a "lift-off" is the engine mixer's *engine off* event (the throttle
released more than 55 percent of the way from idle to the limiter, in a gear above first, with no shift under way). When a
recording plays, and how loud, is chosen here (`crates/nfsmw/src/audio/car/own_sfx.rs`); the 2012 game decides it in sound
graphs ([chameleon-sound-graph.md](../formats/chameleon-sound-graph.md)).

**The flames follow the same rules.** A car of the game lights its tail pipes at each pop of its sample layer's sputter; a mod
car has no sample layer, so each backfire and each pop decided here lights the pipes (`exhaust-flames.md`) at the moment its
sound would play, **whether or not the car has a recording for it**: a car with an empty `backfire` and `pops` list still
backfires in flames, and so does a car with no `[car_sound]` table at all.


### The overrun backfire: `[car_sound.overrun]`

```toml
[car_sound.overrun]
accumulator_rate = 0.2          # charge per second at the redline and full throttle (0 to 5)
throttle_arm = 0.9              # the throttle falling through this arms a bang (0 to 1)
throttle_fire = 0.1             # ... and falling through this fires it (0 to 1)
window_seconds = 2.0            # the longest time between the two for a bang (0 to 10)
flame_delay = 0.2               # seconds of a shut throttle before flames start (0 to 10)
flame_min_rpm = 0.1             # flames need the engine speed over the redline at or above this (0 to 1)
bang_audio_threshold = 0.95     # a bang needs a strength above this to be heard (0 to 1)
bang_effect_threshold = 0.8     # a bang above this strength also lights the tail pipes (0 to 1)
flame_effect_threshold = 1.0    # a flame above this strength also lights the tail pipes (0 to 1)
probability_closed = 0.8        # chance of a loud enough bang with the throttle shut (0 to 1)
probability_open = 0.3          # ... and with the throttle fully open (0 to 1)
```

A car with this table backfires by a **charge** instead of by the lift-off rule. All eleven keys are required numbers; a
missing or unknown key, or a number outside its range, refuses the car. The table works with or without a `backfire` list
(without one the car makes no bang sound and still lights its pipes); a car without the table keeps the lift-off rule.

- The charge (0 to 1) rises by `accumulator_rate * rpm_fraction * dt` per second while the throttle is open and falls by the
  same while it is shut; `rpm_fraction` is the engine speed over the **redline** (`rpm / redline_rpm`, so not idle based).
- The throttle falling through `throttle_arm` arms a bang and starts a timer. The frame it then falls through `throttle_fire`
  ends it: if the timer is within `window_seconds`, the bang's strength is `charge * (1 - timer / window_seconds)`. The bang
  happens if the strength is above `bang_audio_threshold` and a draw is at most the chance, which goes from `probability_closed`
  to `probability_open` with the driver's throttle. So a pull that fills the charge followed by a lift within a fraction of a
  second bangs, and a slow lift or a short pull does not.
- A bang plays one random file of `backfire` at the volume `(strength - audio threshold) / (1 - audio threshold)`. A bang above
  `bang_effect_threshold` also lights the tail pipes, and so do the one to three `pops` after it (the pops are the core ones and
  are not driven by this table); a bang at or under it, and its pops, are audio only.
- With the throttle shut for `flame_delay` seconds at an engine speed of `flame_min_rpm` or more, a flame burns, of strength
  `charge * rpm_fraction`. It plays no sound; above `flame_effect_threshold` it lights the pipes (a burst every 0.06 s, the
  length of the pipe flame of a pop) until the throttle opens.
- It is deterministic: the draws come from a generator seeded by the car's name. The throttle is the pedal (the game has no
  separate electronic throttle) and the engine speed is the host's `0 to 1` span of `[engine_sound]` (or of the built-in
  engine, 900 to 7000 rpm) turned into rpm, so a car with the table should have an `[engine_sound]` with its engine's real
  idle and redline; without one the built-in engine's 900 and 7000 rpm are assumed.
- The turbo blow-off of a lift-off is not changed by the table.

The numbers of a RIDE 6 bike are in [specs/ride6-engine-sound.md](../specs/ride6-engine-sound.md#5-backfire). The controller
is `blackbox_carsound::overrun` (a library, game-agnostic); the host side is `crates/nfsmw/src/audio/car/own_sfx/overrun.rs`.

### The shared sounds: `[car_sound.shared]`

```toml
[car_sound.shared]
impact_small  = ["SOUND/MODSHARED/IMPACT/B38F5.WAV", "SOUND/MODSHARED/IMPACT/B38F6.WAV"]
impact_medium = ["SOUND/MODSHARED/IMPACT/B3903.WAV"]
impact_big    = ["SOUND/MODSHARED/IMPACT/B3922.WAV"]
scrape        = ["SOUND/MODSHARED/SCRAPE/B38F2.WAV", "SOUND/MODSHARED/SCRAPE/B38F3.WAV"]
pops          = ["SOUND/MODSHARED/POP/17F571.WAV"]
```

A mod that adds many cars has recordings its cars share (the 2012 game keeps them in `METALCAR.BNDL` and `WAVES.BNDL`). The
mod writes them **once** in a folder of its own and every car's manifest names them by path, so the files are not copied
per car. Each list has up to sixteen 16-bit PCM WAV files (mono or stereo, paths like the engine's), several files being
variants of which one is picked at random. A wrong table refuses the car. The table is a part of `[car_sound]`: a car may
have it alone or beside its own lists.

| Key | Plays | Notes |
|---|---|---|
| `impact_small`, `impact_medium`, `impact_big` | when the car's body hits the world or another car | the class is picked from the hit's strength (0 to 1): up to 0.25 small, up to 0.75 medium, above it big; a class without files uses the nearest one that has some |
| `scrape` | while the body scrapes along a surface | the one-shots follow one another (the next starts when 85 percent of the last has played, never the same twice running) and fade out 0.15 s after the scrape stops |
| `pops` | after a backfire, like `[car_sound] pops` | used only when `[car_sound] pops` is empty |

**What is played and what stays with the game.** A car with impact files plays a **body hit** from them instead of from the
collision banks of its class-wide record: a hit on the world or on another car, not on a prop that falls over (a cone, a
sign: those are the `smackable` hits). A hit of another kind (the ground, a landing, a part coming off) has no recording
in the table and plays what a car without the table plays. A car with `scrape` files scrapes with them instead of the
scrape loop of the banks. **When** a hit is heard and **how hard** it is (its strength, and whether a scrape is loud enough)
is still decided by the collision rules of the class-wide record, so the recordings are the car's and the thresholds are the
game's; the volume of a hit is `0.55 + 0.45 * strength`, a scrape's `0.35 + 0.65 * strength`; two hits within 0.1 s are
one. A car without the table, and every car of the game, plays exactly what it did (`crates/nfsmw/src/audio/car/own_sfx/shared.rs`).
Skids, nitrous, road noise, wind and the landing sounds stay with the game-wide banks for every car.


### Curves: layers that play by the car's own inputs

A layer can carry curves instead of the engine speed it was recorded at. The 2012 mod writes them (below); a mod author can
too:

```toml
[engine_sound]
idle_rpm = 1000.0
redline_rpm = 7800.0
layers = [
  { file = "SOUND/MODENGINE/MYCAR/L0.GIN", kind = "ginsu", span = [1790.0, 7500.0], volume = 0.35,
    gain = [ { input = "rpm", points = [[0.0, 0.0], [0.0175, 1.0], [1.0, 1.0]] },
             { input = "load", points = [[0.0, 0.0], [0.6, 0.0], [0.94, 1.0], [1.0, 1.0]] } ] },
  { file = "SOUND/MODENGINE/MYCAR/L1.WAV", volume = 0.6,
    gain = [ { input = "speed_mph", points = [[20.0, 0.0], [40.0, 1.0], [90.0, 1.0], [110.0, 0.0]] } ],
    pitch = { input = "speed_mph", reference = 40.0 } },
]
```

- `gain` is a list of up to four curves of two to 32 points (`x` rising, `y` from 0 to 8), piecewise linear and flat
  outside their points. A layer's volume is its `volume` (0 to 8, default 1) times the value of each curve at the car's
  current input. The inputs are `rpm` (0 at idle to 1 at the limiter), `load` (the throttle, 0 to 1) and `speed_mph` (the
  car's speed). A layer with `gain` needs no `rpm` and takes no part in the mix by engine speed; layers without `gain` are
  mixed by `rpm` among themselves as before, so the two kinds can share a car.
- `pitch = { input, reference }` plays the layer at the input over `reference` (from 0.25 to 4 times its own speed). A layer
  without it plays at its own speed (a curve layer) or at the engine's rpm over its own (a layer mixed by `rpm`).
- `rate = { input, points }` is a curve of an input (`rpm`, `load` or `speed_mph`, 2 to 32 points like a `gain` curve, `y` from 0
  to 8) that **is** the layer's playback rate, 1 being the recording's own speed (the host clamps it to 0.25 to 4). It is for a
  pitch that is not proportional to an input, such as an electric motor's whine that rises quickly and levels off, and it takes
  the place of `pitch`. Only a layer with `gain` curves has one.
- `kind = "ginsu"` makes the file a `.GIN` granular engine recording (the format of the 2005 game's `.gin`), played by the
  game's Ginsu synthesiser at the frequency `span` gives the engine speed: `span[0]` at idle, `span[1]` at the limiter.
  It needs `gain` and `span`; the rest of the table is as for a loop.
- Up to 12 layers.

### The 2012 cars' engine

The 2012 game builds a car's engine in **sound graphs** ([chameleon-sound-graph.md](../formats/chameleon-sound-graph.md)):
three Ginsu voices (granular recordings, crossfaded by the engine load), an idle loop that hands over to them just above
idle, and the gearbox whine, five loops driven by the car's speed that the gas pedal crossfades. The 2012 mod reads those
sequences from each car's bundle, **those of the player's engine sound and the whine object only** (the bundle's second engine
sound is the opponents' and is never read), and writes them as layers with curves: the Ginsu resources become `.GIN` files, the
loops WAV files, the windows, the load curves and the gas curves `gain` curves, the whine's reference speed `pitch`.

**An electric or hybrid car** has a **motor** besides: a transmission-speed sequence in the player's engine sound whose loops
play at a rate that is a curve of the car's speed (`rate`) and, for the hybrid, swell and fade by the speed through a mixer
item. The **Tesla Roadster** has nothing else: no Ginsu voice, no idle loop, no gear change, pop, backfire or turbo sequence.
It is written as the two motor loops (rate 1 to 2.2 and 0.8 to 1.2 over their windows of 0 to 150 and 5 to 200 mph) at the
graph's gain, the gearbox whine 26 dB under them, and `shift_clunk = false`: no combustion sound, no idle, no gear shifts. A car whose bundle has no such sequence falls back to the older
**approximation**: it takes **the looping recordings of each car's bundle**
([chameleon-sound.md](../formats/chameleon-sound.md)), measures the pitch of each, calls the lowest one the idle, and gives
every other loop the rpm that its pitch stands for (the idle rpm times its pitch over the lowest pitch). Loops that would
need an rpm far beyond the limiter are left out, and the loops are not the 2012 game's own blend.

The tracked harmonics (`EngineSound2`) are filters on the mixer channels, not a synthesiser, and are **not ported**: the
engine plays unfiltered.

Its other recordings are found by the **name of the live value each sound graph reads**: the samples that the graph
reading `GetPlayerNormalisedGearChangeTimeElapsed` plays are `shift` (the direction is not known, so it is `shift`, not
`shift_up` and `shift_down`); `BackfireGain` (and the variants listed beside it) `backfire`; `GetPopProbability` and
`PopGain` `pops`; `NormalisedTurboRPM` and `TurboToneGain` `turbo_whine` (the car's own loops and the 2012 game's shared
high tone, `VEHICLES/WAVES.BNDL`); `TurboBlowoffGain` `turbo_blowoff` (the shared one-shot that dies away over about two
and a half seconds). The evidence for each is in [chameleon-sound.md](../formats/chameleon-sound.md). A car whose bundle has no
such graph has no such sound: a car without a turbo has no whistle, and one without `TurboBlowoffGain` no blow-off.

**The shared recordings of the 2012 cars.** The 2012 game's body hits live in `SOUND/METALCAR.BNDL` and are named by
the graphs `Player Vehicle Collision Small`, `Medium` and `Big`; the scrape one-shots by `Player Vehicle Scraping` (in
`HAWAII/SURFACESOUNDS.BNDL`); and the pops of 72 cars are not in the cars' bundles but in the shared `VEHICLES/WAVES.BNDL`,
named by the cars' own pop graphs. The mod converts these once to `files/SOUND/MODSHARED/` and names them in every car's
`[car_sound.shared]`. Only the **first selector** of each collision graph (the crash itself, 11 to 12 recordings per size) is
used: the layers above it (a bright layer, a thud, two tails) are mixed by graph nodes that are not decoded. The 2012 data
names no material, skid, glass break or tyre burst in the shared bundles, so those are not converted. The evidence is in
[chameleon-shared-sounds.md](../formats/chameleon-shared-sounds.md).

`own_engine_sound = false` in the mod's `config.toml` writes no recordings, the shared ones included: the cars play the
built-in engine and have only the game-wide sounds.

**What was not verified.** The Tesla's DSP chain (four mixer channels whose gains the motor sequence sets by the speed, and
the tracked-harmonic plug-ins) is not ported; if it synthesises a tone, the 2012 car has it besides the two loops. No recording has been listened to in the game: the checks are that the files are written, that the
manifests parse, that the Ginsu files decode and that the mixing and trigger logic passes its unit tests and a headless
render. The windows' direction, the audible-speed range taken as idle to redline, the curve tangents, the fade shapes, the
whine's reference speed and which of the two Ginsu sets plays are readings of the data
([chameleon-sound-graph.md](../formats/chameleon-sound-graph.md)); the shift direction is unknown. A converted car may sound
wrong even when everything loads. The built-in engine has not been judged by ear either.

## Measured against the cars of the game

Nobody can listen while the game is built, so the sound of a car is **measured**: `crates/nfsmw/src/audio/car/measure`
drives a car through the real mixer with the headless backend (`Audio::replay`, nothing is played) and reads numbers off the
render: loudness (root mean square, and a gated 400 ms loudness like LUFS without the K weighting), peak and clipping, the
spectral centroid and strongest partial against the engine speed, the largest level step between 100 ms windows, clicks,
holes and bangs. The scripts are steady engine speeds at 30 and 100 percent throttle, a sweep from idle to redline, a
coast, a standing start through six shifts, a nitrous pull, a hit and a scrape. The cars of the game are the reference
(BMW M3 GTR, Corvette, Audi TT, Lancer Evo, Cobalt SS). Run it:

```sh
# the converted cars: MW2012_GAME_DIR=<2012 install> MW2012_MOD_OUT=<folder>
cargo test --release -p nfsmw-2012-content --test sound_convert -- --ignored
# the numbers (NFSMW_MEASURE_CARS narrows them, NFSMW_MEASURE_OUT appends them to a file)
NFSMW_GAME_DIR=<2005 install> NFSMW_MEASURE_MOD=<folder> cargo test -p nfsmw --bin nfsmw measure_cars -- --ignored --nocapture
# the thresholds every converted car must keep
NFSMW_GAME_DIR=<2005 install> NFSMW_MEASURE_MOD=<folder> cargo test -p nfsmw --bin nfsmw every_converted_car -- --ignored
```

### What was wrong, and the numbers

| Problem | Before | After | Stock cars |
|---|---|---|---|
| Engine level of a converted car (Porsche 911 Carrera S, full throttle, idle to redline) | -46 to -41 dB | -19.1 to -19.2 dB | -21 to -16 dB |
| Engine level at idle over the 73 cars, full throttle (min, median, max) | -49.6, -23.7, -10.4 dB (after the first fix of the factor) | -31.6, -19.1, -16.4 dB | -33 (Cobalt), -17, -16 |
| Engine level at 50 percent revs (min, median, max) | -29.5, -20.9, -12.1 dB | -22.9, -19.2, -16.3 dB | -32 to -15 |
| Turbo whistle: spectral centroid at the limiter (median, max) | 1503, 5158 Hz | 708, 1539 Hz | 468 to 578 Hz |
| Peak of a rev sweep (median, max) | -4.9, -0.3 dB | -6.6, -3.5 dB | -10.6 to -6.6 dB |
| Peak of the shifts, backfires and pops of a six-shift drive | -2.4 dB | -12.2 dB | -16.9 to -12.8 dB |
| Loudness of those effects | -14.4 dB | -27.9 dB | -33.0 to -28.6 dB |
| Backfire chance at the limiter | 70 percent of lifts | 25 percent | |
| A wall scrape (average level over the scrape) | -17.5 dB | -26.6 dB (peak -13.5 dB) | -24.7 to -26.5 dB |

The causes, in the order they were found:

1. **Engine level.** The host scaled a mod car's engine by `engine_volume * accel_volume` of a car of the game, which is 0.035
   (24 dB under the stock cars: the factor scales a stock car's Ginsu loop next to its sample layer and means nothing for
   layers that have no sample layer). Dropped; the engine is now scaled by `own_engine/level.rs`: it predicts the level of
   the whole engine at the current engine speed, load and car speed from the root mean square of each recording (a Ginsu
   recording at the stretch its frequency plays) times the gain each layer has, and brings it to -19 dB at the nominal
   engine volume (-22 dB off the throttle). The correction stays within 4 times of the car's typical one, so a car whose
   graph idles quietly (two cars, as the stock Cobalt does) keeps a quiet idle.
2. **Recordings mastered apart.** The 2012 cars' Ginsu and idle recordings differ by up to 40 dB, and the three Ginsu voices
   of a few cars have no engine-load curve at all (they play together); both are absorbed by the level above.
3. **The turbo whistle was a second engine**: the loops were brought to a root mean square of 0.25, as loud as the whole
   engine, and made every turbocharged car a hiss (centroid 5 to 10 kHz). `WHINE_LEVEL` is now 0.08.
4. **The transmission whine** the graph gives (a gain of 0.6 on a channel at -16 dB) sits 6 dB under the engine at 1 to 3 kHz;
   the converter takes it down to 0.35 of that so it is a detail 15 dB under the engine.
5. **The shifts and pops** were normalised to a peak of 0.9 and played at the volume the stock mixer asks for the clunk of a
   car of the game (whose recording is quiet): `SHOT_LEVEL` is 0.3. A backfire followed a lift at the limiter seven times in
   ten and a lift at idle speed one in five, then two to four pops: now 3 to 25 percent, one to three pops.
6. **Silent idle:** for three cars (the Koenigsegg and two others) the first of the two engine sets of the bundle starts its
   items 250 rpm above the bottom of its range and is silent at idle (-55.8 dB); the converter takes the first set that
   sounds at the bottom of the range.
7. **The electric car** (Tesla Roadster) has no Ginsu voice: its old engine was the five whine loops spread by pitch over an
   idle of 500 rpm and a redline of 14,000. It was then played by the whine loops alone, at a fixed pitch (the converter
   read no pitch curve), with the opponents' copy of its motor on top, and the shared gear clunk at every one of its two
   gears. It now plays the two motor loops of its player engine sound by their pitch curves of the car's speed, the gearbox
   whine under them, and no clunk (`measure/electric.rs`: the pitch rises with the speed, the level holds, a drive through six
   shifts has no burst). The motor plays from standstill, as its graph's window does (0 mph, no fade).
8. **Scrape**: the mod's shared scrape was 9 dB louder than the same scrape of the cars of the game (average -17.5 dB against
   -26.5, peak -4.4 against -16.5 dB): `SCRAPE_LEVEL` is 0.35. A hit peaks as high as the hit of the cars of the game (-2.5 dB) and
   its 500 ms level is 3 to 5 dB higher (the recordings are longer); that is left as it is.

### Where the cars stand now

All 73 converted cars pass `every_converted_car_is_as_loud_and_as_smooth_as_the_cars_of_the_game`: their median loudness is
within 3 dB of the BMW M3 GTR (-19.2 against -18.8 dB), every rev from 10 percent to the limiter is between -25 and -14 dB,
a car's loudness spreads at most 9 dB over the revs, no sweep, coast or drive clips or peaks above -3 dB, there is no hole and
no step above 10 dB in 100 ms (the stock cars: up to 13.5 dB on a coast), the sound brightens with the revs (median Spearman
correlation of the centroid with the revs 0.94; the stock cars 0.71 to 0.98), and the effects stay under the engine.

The cars by how their engine is made (73 conversions; the livery bundles of a car are not cars):

| Class | Cars | Engine |
|---|---|---|
| Three Ginsu voices, an idle loop and five whine loops | 69 | the graphs' own engine, level set as above |
| The same plus a tenth layer (a second idle loop of the hybrid) | 2 (Porsche 918 Spyder, both) | as above |
| Two Ginsu voices | 1 (Mercedes SLS AMG 2010) | as above |
| No Ginsu voice, motor loops by speed | 1 (Tesla Roadster) | two motor loops with a rate curve of the speed, the whine under them, no clunk |
| Three Ginsu voices, an idle loop, a motor and the whine | the hybrid (the bundles of the Porsche 918 Spyder that have a motor sequence) | the graphs' engine, plus the motor loop with its speed curves |
| Pitch heuristic (the bundle has no engine graph) | 0 | kept for a car that has none |
| Built-in engine (`own_engine_sound = false`, or a car with no table) | 0 of the converted cars | six-cylinder loops, level set as above; between two of its loops the level dips up to 6 dB where they are out of phase |

### What is still different, and what cannot be checked

- **Brightness.** The 2012 Ginsu recordings are brighter than the 2005 ones (centroid at the limiter: median 708 Hz against 468
  to 578 Hz; the brightest cars, the Pagani Zonda and the Audi A1 family, 1.4 to 1.5 kHz). The 2012 game filters its voices
  with mixer-channel filters (low-pass, band-pass, peaking) that are not ported
  ([chameleon-sound-graph.md](../formats/chameleon-sound-graph.md)).
- **Pitch tracking** is read from the strongest partial and the centroid: the Audi A1 family (3 cars) and the Aston Martin
  DBS family (2) rise less than the others, because their dominant component is not the engine.
- **The loudness model** adds powers (incoherent); between two layers of the same harmonics the real level can dip.
- **Nothing was listened to.** The numbers say a converted car is as loud, as smooth and about as bright as the cars of the
  game within a few dB; they do not say it sounds like the 2012 car or like a stock car. The gains of the individual voices
  (the graph's relative levels, the two engine sets, the load crossfade) are the graph's own and unverified by ear.
