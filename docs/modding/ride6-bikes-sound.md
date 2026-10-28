# Sound of the RIDE 6 bikes: the `sound` module of `ride6-bikes`

Part of the [RIDE 6 bikes mod](ride6-bikes.md) and the [car sound](car-sound.md) tables. Given the user's own decoded
recordings and a bike's resolved audio data, it writes the bike's `files/CARS/<ID>/SOUND/*.WAV` and the
`[engine_sound]` / `[car_sound]` tables of its manifest. What RIDE 6 does and why is in
[specs/ride6-engine-sound.md](../specs/ride6-engine-sound.md); where the data lives in
[formats/ride6-audio.md](../formats/ride6-audio.md).

## The state of the data, in one paragraph

RIDE 6 plays its engines through Wwise. The events, the effect numbers and the backfire logic are readable; the engine
loops, their rpm points and the crossfade curves are inside sound banks that the game stores in an encrypted `.pak`
index, which this repository does not break. So the module takes the recordings **from you**: decode them from your own
install with your own tools (below), tell the module which engine speed each loop was recorded at, and it does the rest.
Without recordings a bike plays the built-in engine of the host.

## Decoding the recordings

1. Export the bike's bank `WwiseAudio/KTM_EXC_450.bnk` (engine and exhaust loops) and, for effects, the `.wem` files named
   by the bike's events (`resolved.json`, below) with a tool that can read your install (FModel with the key you hold, or
   any equivalent). The repository neither holds nor looks for that key.
2. A bank with embedded media: `extract-bnk KTM_EXC_450.bnk C:\r6\audio-ktm450\wem` writes `<media id>.wem` files
   (`sound::cli::extract_bnk`).
3. Decode each file (cmd): `for %f in (C:\r6\audio-ktm450\wem\*.wem) do C:\r6\tools\vgmstream\vgmstream-cli.exe -o C:\r6\audio-ktm450\wav\%~nf.wav %f`.
   Loop points come out as a `smpl` chunk, which the module uses.
4. Listen, and note each engine loop's rpm. The recordings carry no rpm; for a single-cylinder four-stroke the firing
   frequency in Hz is `rpm / 120`, so a pitch tracker on the loop gives it (the physics asset says idle 2300 and limit
   11000 rpm for the KTM 450).

Keep everything under `C:\r6` (never in the repository, never in `D:\RIDE 6`).

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

`cargo test -p ride6-bikes sound::` (WAV round trip, sample formats, loops, mixing, bank splitting and its refusals, table text,
the whole build with synthetic tones, the two commands, refusals), `cargo test -p blackbox-carsound overrun::`,
`cargo test -p blackbox-modding overrun_sound engine_sound` and `cargo test -p nfsmw --bin nfsmw own_sfx::`. They use synthetic data only; nothing was
built or run when this page was written.
