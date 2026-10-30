# Sound of the RIDE 6 bikes: the `sound` module of `ride6-bikes`

Part of the [RIDE 6 bikes mod](ride6-bikes.md) and the [car sound](car-sound.md) tables. Two ways in, one result: the
`[engine_sound]` / `[car_sound]` tables of a bike's manifest and the recordings they name.

- **From the banks (all bikes at once).** `pak-sound` extracts RIDE 6's Wwise banks with the pak key and resolves every
  chassis's audio data; `sound-bikes` builds the sound of every imported bike from them (the real engine recordings, the
  gear changes, bangs and crackles). This is the main path and needs nothing else from you but the install.
- **From your own recordings (one bike).** `sound` and `extract-bnk` (below) build a bike's sound from WAV files you decoded
  and named by engine speed; the original path, kept for experiments.

What RIDE 6 does and why is in [specs/ride6-engine-sound.md](../specs/ride6-engine-sound.md); where the data lives in
[formats/ride6-audio.md](../formats/ride6-audio.md). Nothing from RIDE 6 is in the repository: the banks and every recording
are made on your machine from your own install, under `C:\r6` or any folder you choose, never in the repository.

## The pak key

RIDE 6 keeps its sound banks in `.pak` files with an AES-256 encrypted index. The key was **supplied by the user** (found in
<https://cs.rin.ru/forum/viewtopic.php?f=10&t=100672>) and **verified by mounting** (1096 banks appear). The mod uses it by
default (`mods/ride6-bikes/src/sound/pak_key.rs`). To use another key:

- the environment variable `R6_AES` (64 hexadecimal digits, `0x` allowed), which wins; or
- `pak_key = '...'` in the mod's `config.toml` (read by `pak-sound --config <config.toml>`); or
- `pak-sound --pak-key <hex>`.

`pak-sound --print-key-source` says which one is in use (never the key). The pak reader is an external program; the commands
start it with `R6_AES` set to the key in use, so you do not export anything.

## Extracting the banks

The reader is a console tool built on CUE4Parse (Apache-2.0) with two modes, `raw <regex> <folder>` (save every file of the
paks whose path matches) and `audiomap <folder>` (one JSON per row of the chassis audio table). The commands call it:

```text
ride6-bike-import pak-sound --tool C:\r6\tools\CUE4Parse\R6ToolSnd\bin\Release\net10.0\R6ToolSnd.exe ^
  --usmap C:\r6\ride6.usmap --out C:\r6\sound-work\raw --resolved C:\r6\sound-work\resolved
```

`--out` receives `ride6/Content/WwiseAudio/...` (879 MB: 1096 banks, 1372 loose media); `--resolved` receives 634 files
`<chassis>__<Stock|Custom>.json` (events, banks, media names and backfire numbers of each chassis row). `--tool` and `--usmap`
default to `$R6_PAK_TOOL` and `$R6_USMAP`. Decoding needs `vgmstream-cli` (`--vgmstream` of `sound-bikes`, or `$VGMSTREAM_CLI`).

## Building every bike's sound

```text
ride6-bike-import sound-bikes --bikes C:\r6\bikes --physics C:\r6\physdump-all ^
  --resolved C:\r6\sound-work\resolved --wwise C:\r6\sound-work\raw\ride6\Content ^
  --out C:\r6\sound-work\out --vgmstream C:\r6\tools\vgmstream\vgmstream-cli.exe [--variant Stock] [--only <vehicle id>]
```

For each bike folder of `--bikes` it reads the engine's idle and limiter speed from the physics file, finds the bike's engine
and exhaust events in the banks, follows them to the REV engine models and builds the layers
([specs/ride6-engine-sound.md](../specs/ride6-engine-sound.md#8-the-engine-layers-from-the-rev-models)), and builds the
shift, bang and crackle lists from the gear-up, gear-down, backfire and exhaust events. It writes, in `--out`:

- `files/SOUND/RIDE6/ENG/<engine>-<hash>/L<nn>_<rpm*10>.WAV`: the engine layers, **one folder per engine** (the same banks
  serve many bikes: 293 bikes use 98 engines); 21 layers each (the idle and the model's 20 loops under load), about 1.5 MB;
- `files/SOUND/RIDE6/FX/<media id>.WAV`: every effect recording once (the gear-up clunks of the shared bank are decoded once for
  all bikes);
- `cars/<id>.sound.toml`: the tables of a bike, naming those paths (the paths are ordinary `SOUND/...` paths of the mod's
  `files/` folder, so many cars name the same files);
- `sound-report.md` (with `--report`): the bikes with their own sound, the bikes without and why.

`--variant` picks the audio variant of a chassis: `Custom` (the default: the aftermarket exhaust, the variant of the
reference photo and the only one with the bangs of many bikes) or `Stock`; a chassis with only the other variant uses it.
Set the mod's `sound` setting to `--out` (`sound = 'C:\r6\sound-work\out'` in `config.toml`): the importer appends each
bike's tables to its manifest and copies the files its tables name (`manifest_sound::copy_files`: the per-car folder and the
shared recordings, each only when the mod folder does not hold a file of that size).

Run it again after a change: an engine folder that exists is reused (delete `files/SOUND/RIDE6/ENG` to rebuild), effect
recordings that exist are not decoded again.

## By hand: decoding recordings of one bike

1. Extract the bank (`pak-sound`, above, or `extract-bnk KTM_EXC_450.bnk C:\r6\audio-ktm450\wem`, which writes the embedded media
   as `<media id>.wem`; the engine media of a bike are REV models, `PLUG` files that `vgmstream-cli -s <n>` reads one recording at
   a time).
2. Decode each file: `vgmstream-cli.exe -o out.wav in.wem`.
3. Tell `sound` the engine speed of each loop. A loop of the REV models carries it: engine cycles per second times 120
   (formats doc, "REV engine models"); for other recordings a pitch tracker gives the firing frequency, `rpm / 120` for a
   single-cylinder four-stroke.

Keep everything outside the repository and outside `D:\RIDE 6`.

## The resolved data

`C:\r6\snd\tools\resolve_bike_audio.py KTM_450-SMR-2022 Custom C:\r6\snd\ktm450-custom.json` writes the bike's events,
banks, media names and merged effect numbers (already done for the KTM 450, both variants). The module reads it with
`sound::data::BikeAudioData::from_json`; `overrun()` returns the backfire numbers, the bike's own over the base asset's.

## Building a bike's sound

From the command line (`sound::cli::run`, the `sound` word of `ride6-bike-import`):

```text
ride6-bike-import sound --out C:\r6\bike-mod --car-id KTM450 --idle 2300 --redline 11000 ^
  --layer "3000:en_3000.wav,ex_3000.wav*0.8" --layer "6000:en_6000.wav,ex_6000.wav*0.8" --layer "9500:en_9500.wav" ^
  --backfire bang1.wav --backfire bang2.wav --shift-down gsxr1.wav --pops pop.wav ^
  --data C:\r6\snd\ktm450-custom.json
```

It writes `<out>/files/CARS/KTM450/SOUND/ENG0.WAV`, `BACKFIRE0.WAV` ... and `<out>/cars/ktm450.sound.toml`, and prints
the tables, which are pasted after the car's own keys in `cars/ktm450.toml`:

```toml
[engine_sound]
idle_rpm = 2300.0
redline_rpm = 11000.0
layers = [
  { file = "CARS/KTM450/SOUND/ENG0.WAV", rpm = 3000.0 },
  { file = "CARS/KTM450/SOUND/ENG1.WAV", rpm = 6000.0 },
]

[car_sound]
backfire = ["CARS/KTM450/SOUND/BACKFIRE0.WAV", "CARS/KTM450/SOUND/BACKFIRE1.WAV"]
shift_down = ["CARS/KTM450/SOUND/SHIFTDN0.WAV"]
pops = ["CARS/KTM450/SOUND/POP0.WAV"]
shift_clunk = false
shift_trigger = "gear"

[car_sound.overrun]
accumulator_rate = 0.2
# ... the other ten numbers of the bike's data
```

As a library: `sound::plan::build(&SoundSources, &SoundOptions) -> BikeSound` (files as bytes plus both table texts) and
`sound::plan::write_files(&sound, files_dir)`.

What the build does to the recordings (modelling choices, [spec section 7](../specs/ride6-engine-sound.md#7-modelling-choices-not-facts)):
the loop body of a `smpl` loop is kept, a clip is closed with a 40 ms equal-power crossfade, files of one rpm
(`en,ex*gain`) are summed, the loudest engine loop peaks at 0.9 and the others keep their relative level, each one-shot
is normalised to 0.9, everything is written as 16-bit PCM WAV. It refuses: no layer or more than 12, two layers of one
rpm, an rpm outside 100 to 20000, idle not below redline, more than 16 files in a list, a bad car id.

## The overrun backfire table and the gear trigger

With `--data` the command adds `[car_sound.overrun]` with the eleven numbers of the bike's backfire (spec section 6) when the
bike backfires by the accumulator system (simulation on, new system; otherwise it prints a note and leaves the table out so
the core rule applies). `--no-overrun-table` leaves it out. It also writes `shift_trigger = "gear"` (every change of the gear
number plays `shift_up` / `shift_down`, spec section 3); `--mixer-shifts` leaves it out. Both are read by the core manifest
and played by the host ([car-sound.md](car-sound.md#the-overrun-backfire-car_soundoverrun)); the controller is
`blackbox_carsound::overrun::OverrunBackfire`. A bike with no recordings of its own has none of these lists and plays the
built-in engine; with the table it still lights its pipes.

## `extract-bnk`

`ride6-bike-import extract-bnk <bank.bnk> <folder>` writes the media of a sound bank as `<media id>.wem`
(`sound::cli::extract_bnk`); step 2 of the decoding above.

## Tests

`cargo test -p ride6-bikes sound:: manifest_sound:: config::` (WAV round trip, sample formats, loops, mixing, bank splitting and its
refusals, the `HIRC` objects of a synthetic bank, the REV directory, pitch measurement and resampling, the engine layers, the pak key
order, the shared files a manifest names, table text, the whole build with synthetic tones, the commands, refusals),
`cargo test -p blackbox-carsound overrun::`, `cargo test -p blackbox-modding overrun_sound engine_sound` and
`cargo test -p nfsmw --bin nfsmw own_sfx::`. They use synthetic data only. On a real install the converted bike was also
rendered offline by the game's mixer (`NFSMW_MEASURE_MOD=<mod folder> NFSMW_MEASURE_CARS=KTM450 cargo test -p nfsmw --bin nfsmw
measure_cars -- --ignored --nocapture`) and loaded by the game (`nfsmw view-world --drive KTM450 --mods-dir <mods> --muted
--radio --screenshot <png>`: `--radio` keeps the audio on in a screenshot run, and a missing layer then logs a warning from
`nfsmw::audio::car::own_engine`).

### Measuring a bike's level and effects

Three more ignored tests of `crates/nfsmw/src/audio/car/measure/curve.rs` render a bike offline through the game's mixer (set
`NFSMW_GAME_DIR`, `NFSMW_MEASURE_MOD=<mod folder>`, `NFSMW_MEASURE_CARS=<car ids, comma separated>`):

- `level_curve`: the engine's level at every `NFSMW_MEASURE_STEP` rpm (default 250) from idle to redline at full throttle
  (`NFSMW_MEASURE_THROTTLE` changes it), one line `car rpm dB` each; holes in the layer set show as dips
  ([specs/ride6-engine-sound.md](../specs/ride6-engine-sound.md#the-level-from-idle-to-redline)).
- `lift_offs`: `NFSMW_MEASURE_LIFTS` pulls and lifts; for each lift the peak of the effects (shifts, bangs, pops) in the second
  after it, so it shows which lifts pop.
- `gear_changes`: the effects' peak in the 0.4 s after each change of the gear number, against the 0.4 s before.

`NFSMW_AUDIO_TRACE=<file>` logs the one-shots the effects play (`own_sfx_shot`: the list, the variant, the volume).
