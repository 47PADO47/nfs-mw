# blackbox-ginsu

Ginsu, the granular engine-sound synthesiser of EA Black Box games (Need for Speed: Underground 2 and Most
Wanted `Gnsu` files), and the parser of the tables it runs on.

A `.gin` file is a recording of an engine sweeping through its RPM range, cut into pitch cycles, plus tables
that say where in the recording each frequency and each cycle is. To follow a target frequency the synthesiser
plays the recording forward and jumps by whole cycles, with a half-millisecond cross-fade, so the pitch tracks
the target and the timbre is the recording's at that RPM.

- **Pure.** `f32`, no I/O, no audio output, no game knowledge, no dependency on a codec. The same data and the
  same calls always give the same samples.
- **Bytes and samples in.** `GinsuTables::parse(&[u8])` reads the header and the frequency and cycle tables of
  a `.gin` and returns where the EA-XAS payload starts. The caller decodes that payload (the `ea-audio` crate)
  and builds a `GinsuData` from the mono samples (`i16` or `f32`).
- **Block based.** `GinsuSynth::render(&SynthParams, &mut [f32])` fills any block size at the file's sample
  rate; packets (11 ms) are made as needed. The volume ramps across the block. Pitch changes that the game
  applies at the player (the sub-minimum ratio from `GinsuTables::clamp_frequency`, the mixer's pitch
  multiplier) are resampling ratios for the caller's output stage.
- **Written from the spec** in `docs/specs/engine-sound-ginsu.md`, not from the decompilation
  (`docs/provenance/engine-sound.md`).

```rust
use std::sync::Arc;
use blackbox_ginsu::{GinsuData, GinsuSynth, GinsuTables, SynthParams};

let (tables, payload_at) = GinsuTables::parse(&gin_bytes)?;
let pcm: Vec<i16> = decode_xas(&gin_bytes[payload_at..], tables.sample_count()); // ea-audio
let data = Arc::new(GinsuData::from_i16(tables, &pcm)?);

let mut synth = GinsuSynth::new(data, 1500.0)?;          // start at 1500 "RPM"
let mut block = [0.0f32; 512];
synth.render(&SynthParams::new(3200.0).with_volume(0.8), &mut block);
let rpm = synth.current_frequency();                      // the pitch it is playing, in file units
```

## Units

Frequencies are in the file's units, the game's "engine RPM": a pitch cycle is `sample_rate * 120 /
frequency` samples long (`FREQUENCY_PER_HZ = 120`). `current_pitch()` is in Hz, `current_frequency()` in file
units.

## Tests

`cargo test -p blackbox-ginsu`: lookups and the file parser on synthetic tables; the synthesiser on a
synthetic rev-up (a sine whose frequency rises, cut at whole turns): steady pitch within 3 % across the range
and at several sample rates, a glide that follows a rising target, no clicks at the joins (largest step below
1.5 times the sine's own), bit-identical output for equal call sequences and for any block size at a steady
target, volume ramps, the end of the recording, nonsense inputs (NaN, infinity, negative gain, empty blocks).

License: MIT OR Apache-2.0.
