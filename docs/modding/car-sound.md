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
| Gear shifts, turbo whistle and blow-off, backfires, pops | `[car_sound]` recordings | **silent**, except the gear clunk below |
| Collisions and scrapes | the mod's shared recordings, `[car_sound.shared]` | the game-wide banks |
| Skids, nitrous, road noise, wind | none: shared by every car | always plays, from the game-wide banks |

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
- One to eight layers, `rpm` from 100 to 20000, no two the same, `idle_rpm` below `redline_rpm` (both 100 to 20000). A wrong
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
| `turbo_whine` | loops, from the lowest spool to the highest | all play at once, silent until the turbo spools; mixed along the spool in equal power and sped up from 0.8 to 1.4 times as it spools |
| `turbo_blowoff` | when the throttle is lifted with the turbo spooled | louder with more spool; the spool drops to 40 percent |
| `backfire` | when the throttle is lifted at high engine speed | likelier with the revs: one in five at idle speed, seven in ten at the limiter |
| `pops` | after a backfire (two to four, a tenth of a second and more apart), or alone | a lift-off without a backfire sometimes has one or two |

**These rules are the game's own, not any recording game's.** The turbo spools towards the throttle times how hard the
engine is turning (0.9 seconds up, 0.6 down); a "lift-off" is the engine mixer's *engine off* event (the throttle
released more than 55 percent of the way from idle to the limiter, in a gear above first, with no shift under way). When a
recording plays, and how loud, is chosen here (`crates/nfsmw/src/audio/car/own_sfx.rs`); the 2012 game decides it in sound
graphs that are not decoded ([chameleon-sound.md](../formats/chameleon-sound.md)).


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

**What was not verified.** No recording has been listened to in the game: the checks are that the files are written, that the
manifests parse and that the mixing and trigger logic passes its unit tests. The body hits, scrapes and shared pops are the same: the graph names that pick them are text in the data, but which crash a mass or a strength should play, and the level of one against another, are the game's own simple rules here. Which 2012 sample is which (the engine layer
roles are not read, the pitch-to-rpm mapping is a guess, the shift direction is unknown) is a reading of the data, so a
converted car may sound wrong even when everything loads. The built-in engine has not been judged by ear either.
