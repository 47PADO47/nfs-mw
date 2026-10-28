# Chameleon sound: `.SPS` streams and the car sample resources

NFS: Most Wanted (2012) plays its audio through EA Audio Core 8.03 (strings in `NFS13.exe`), not the Black Box
sound system of the 2005 game ([audio.md](audio.md)). Reader: [`libs/ea-audio/src/sps`](../../libs/ea-audio/src/sps).
For the evidence tags see [README.md](../README.md#evidence-tags). Everything here was measured on the 2012 install
(v1.4.0.0); the layouts were read from the game's own decoder code and then checked against the files.

## `SOUND/STREAMS/*.SPS` **[verified on all 56 files]**

56 files, 108 MB. The file name is a decimal id. They hold ambience, music and speech, **not** car engines: the
ids are referenced from `HAWAII/AMBIENCES.BNDL` (the 6-channel 30 s and 60 s loops), `HAWAII/REVERBROADDATA.BNDL`, the
`TRK_UNIT*.BNDL` music units (`31189`, `31190`, `31235`) and the language bundles.

A stream is a list of big-endian chunks. The size field counts the whole chunk, header included.

| Chunk | Layout |
|---|---|
| `H` (0x48) | `H`, size (24 bits), then a bit string, most significant bit first (below) |
| `D` (0x44) | `D`, size (24 bits), samples of the chunk (u32), then frames |

The `H` bit string: version (4 bits, 1), codec (4), channels minus one (6), sample rate (18), kind (2), loop flag (1),
samples per channel (29); a set loop flag adds the loop start (32 bits); kind 2 adds 32 bits, and a loop flag with
kind 1 or 2 adds 32 bits more. A 12-byte `H` chunk is the plain form, a 20-byte one has loop data.

The codec numbers, from the game's table: 2 `P6B0` (PCM), 3 `EXm0` (EA-XMA), 4 `Xas1` (EA-XAS), 5 `EL31`
(EA Layer III v1), 6 `L32P`, 7 `L32S` (Layer III v2), 9 `Esp0` (Speex). **All 55 streams that decode are codec 5**
(one more is MPEG-2 Layer III, 16 kHz, and is not read).

### `EL31` frames **[verified: 55 streams decode with no error and to their declared length]**

A frame is one granule (576 samples per channel) of an MPEG-1 Layer III frame with the 4-byte MP3 header squeezed
into one byte and no bit reservoir. Bit by bit:

1. a flag byte: `0x00`, or `0xEE` when raw PCM follows the frame (a count, an offset and 16-bit big-endian samples
   that the game writes over the decoded samples, a splice for looping; the reader skips them);
2. a header byte: version (2 bits, 3 = MPEG-1), sample rate index (2), channel mode (2), mode extension (2);
3. the granule number (1 bit);
4. for granule 1, the four scalefactor-sharing bits of each channel;
5. per channel, the standard 59 bits of Layer III granule side information; the first 12 are `part2_3_length`;
6. the main data, `part2_3_length` bits for each channel in turn.

Then the frame ends at a byte boundary. Two granules (0, then 1) are one MPEG-1 frame, so repacking is: write the
MP3 header and side information (no reservoir, or a reservoir when a frame is bigger than 320 kbit/s allows) and the
two main data runs. `ea_audio::sps::to_mp3` does that and any MP3 decoder reads the result.

A stream of more than two channels is made of **cores**, one per stereo pair (a mono core for an odd last
channel). Inside a `D` chunk the cores' frames alternate, core 0, core 1, and so on. The first chunk of a stream has
about 47 extra samples; the decoded length is within five frames of the header's sample count on every stream.

## Car sample resources (kind `0x81`) **[verified on one car, `VEH_1085007_HI`]**

Each car bundle holds about 40 sample resources: block 0 is a 128-byte header and a whole `SPS` stream.

| Offset | Field |
|---|---|
| 0x00 | u32 `0x80` |
| 0x10 | f32 volume, 1.0 |
| 0x14 | f32 length in milliseconds |
| 0x18 | u32 size of the stream that follows the header |
| 0x24 | u32 flag, 1 |
| 0x80 | the `SPS` stream (`H` chunk first) |

They are mono, 48 kHz, codec 5, and decode with the same reader. Seven of the 40 loop (loop start 0), the rest are
one-shots. The two longest loops, 1.8 s each, have a harmonic series on a 41 Hz fundamental, which for a six-cylinder
engine with three firings per revolution is an idle of about 820 rpm. That car is the Porsche 911 Carrera S, a flat six
that idles near 800 rpm, so these are the car's own engine loops. Other loops have partials near 510, 680, 850 and
1,020 Hz (higher revs, or a whine).

**How the game uses them.** The sample resources are named by sound graphs, kind `0x15` objects in the same bundle whose
imports point at the samples. A graph is a tree of nodes with ranges (a pair like 7500 and 1790 is the span of the
engine speed it follows) and expression strings that read live values: `GetAudibleRPM`, `GetNormalisedRPM`,
`GetEngineLoad`, `GetTransmissionSpeedMph`, `GetNormalisedGearChangeTimeElapsed`. The upper range of the engine is
synthesised by a DSP object (`EngineSound2`, parameters `TrackedHarmonic1Frequency` and so on) that follows the rpm,
which no recording holds.

**The node format** is decoded for the engine's sequences (the windows, fades and curves between a live value and each
sample's volume or pitch, and the Ginsu engine resources): see [chameleon-sound-graph.md](chameleon-sound-graph.md). The
shift, turbo, backfire and pop graphs are still found by **which graph, named by the live value it reads, plays which
sample**, and that is how the mod finds them.

## Which samples play when **[measured on the 2012 install; the node format is not decoded]**

A graph's main block holds its expression strings (`RaceCars.EntityEngineSound[#].BackfireGain`) and its imports are the
resources it plays, in order: other graphs, shared node types (kind `0x14`), and samples. Following the imports from every
graph that has a given name in its strings gives the samples that graph plays. The names occur across the 93 car bundles
(`VEH_*_HI.BNDL`; the turbo values only in the turbocharged cars), and what they reach is listed here for two cars that show the whole set,
the Porsche 911 Carrera S (`VEH_1085007`, no turbo) and a turbocharged car (`VEH_1085698`); the mod's real-install tests
repeat the counts on a third and a fourth (`src/effects/real_tests.rs`):

| Live value the graph reads | Samples it plays | Evidence for the effect |
|---|---|---|
| `Players.LocalPlayer.GetPlayerAudibleRPM`, `...EntityEngineSound[..].GetAudibleRPM` | two 1.6 to 1.8 s loops (`17f5f7`, `17f5f8`; Porsche: `1d1c7a`, `1d1c7b`, 41 Hz fundamentals) | the engine's own idle loops |
| `RaceCars.Entities[..].GetTransmissionSpeedMph` | five loops, 0.4 to 3.5 s (the gearbox whine, the same on every car); the electric and hybrid cars have a second sequence of this value in the player's engine sound with one or two more loops, the motor ([chameleon-sound-graph.md](chameleon-sound-graph.md#the-electric-motor)) | transmission speed: by its name the transmission's whine |
| `RaceCars.Entities[..].GetReverseTransSpeedMph` | one 0.8 s loop at 24 kHz (`23104`) | reverse gear whine |
| `LocalPlayer.GetPlayerNormalisedGearChangeTimeElapsed` (five graphs, each with `EngineSound[#].Lfo`) | six one-shots of 0.14 to 0.36 s (`d4a9f` to `d4aa4`): two groups, of four and of two, each behind a selector node | played during a gear change, timed by the elapsed gear change; **up and down are not told apart** |
| `RaceCars.EngineSound[#].ShuntLfo` | twelve one-shots of 0.09 to 0.36 s | the engine shunt on throttle changes; not exported |
| `...EntityEngineSound[#].BackfireGain` | one 0.77 s one-shot (`17f5c9`), listed with two more (`17f5c7`, `17f5c8`, 0.70 and 0.72 s) by a graph with no expression of its own, a bag of variants | the backfire and its variants |
| `RaceCars.Entities[..].GetPopProbability` (Porsche) | nine one-shots of 0.02 to 0.10 s | the pops: the game's own name for the chance of a pop; `PopGain` and `GurgleGain` are other cars' versions (gurgles are not exported) |
| `...EntityEngineSound["Id"].NormalisedTurboRPM` (turbo cars) | two 1.0 s loops (`17f618`, `17f619`) | the turbo shaft's speed, 0 to 1: the whistle |
| `...EngineSound[#].TurboLfo` (turbo cars) | six 3 s one-shots and three 2 s ones | not identified further; not exported |
| `NormalisedTurboRPM` + `TurboToneGain` + `GetEngineLoad(LTEQ:0.1)` + `TurboBlowoffGain`, one shared graph (`1d1b53`) | two samples of the shared `VEHICLES/WAVES.BNDL` (also `SOUND/GLOBAL.BNDL`): `d6787` and `d4de7` | see below |
| `...GetEngineHeat`, `TrackedHarmonic*`, `SubharmoniserFrequency` | eight 0.13 to 0.25 s one-shots (`f3468` to `f346f`) | the grains of the synthesised engine; not exported |
| `RevLimitGain`, `...EngineOn` | not examined | the limiter and ignition; not exported |

The shared turbo graph imports two resources that no car bundle holds. They are samples of `WAVES.BNDL`: `d6787` is a
0.86 s **loop** with its energy at 3 to 7 kHz (a high tone, the turbo whistle at full spool), and `d4de7` a 2.57 s
**one-shot** whose level falls to under a quarter by its last third (broadband, centroid about 1.6 kHz: air let out, the
blow-off). The graph's string table lists `NormalisedTurboRPM` and `TurboToneGain` first, then
`GetEngineLoad(LTEQ:0.1)` (the throttle closed) and `TurboBlowoffGain`, and the measured samples fit that order: the
tone first, the blow-off second. A turbo car whose graphs lack `TurboBlowoffGain` (`VEH_2076399`) has a whistle and no
blow-off.

**What this does not say.** Which sample of a group plays at a moment, at what level and pitch, and why the Porsche's
two engine loops have the same pitch, are in the node data of those graphs, which is not read ([chameleon-sound-graph.md](chameleon-sound-graph.md#what-is-not-decoded-unknown)). The shifts could be a click, a blip or a burble: they are
what the graphs play for a gear change, not known to be a "clunk". The turbo tone's rpm tracking and the blow-off's
trigger are likewise the game's own rules here, not the 2012 game's (`docs/modding/car-sound.md`).

`WAVES.BNDL` and `METALCAR.BNDL` hold the shared one-shots in the same resource layout; which of them the named graphs play (body hits, scraping, the cars' pops) is in [chameleon-shared-sounds.md](chameleon-shared-sounds.md).
