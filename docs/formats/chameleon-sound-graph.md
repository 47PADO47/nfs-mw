# Chameleon sound graphs: sequences, curves and the Ginsu engine

How NFS: Most Wanted (2012) builds the sound of a car from the *sound graph* objects of its bundle, and how
[`libs/chameleon-sound-graph`](../../libs/chameleon-sound-graph) reads them. The container is in
[chameleon-bundle.md](chameleon-bundle.md), the object layout (Genesys types and objects) in
[chameleon-vehicle-stats.md](chameleon-vehicle-stats.md#genesys-types-and-objects-verified), the samples in
[chameleon-sound.md](chameleon-sound.md). Evidence tags: **[verified]** is checked on the install (v1.4.0.0) with the Rust
reader or a scratch script, **[exe]** is read from `NFS13.exe` (names and registrations only, no code is kept),
**[inferred]** is a reading that fits every value checked but is not proven, **[unknown]** is not decoded.

## Short answer

A car bundle's sound half is a set of **sequences** (class `Genesys.Gen.Sequence`, kind 0x15 objects). A sequence is driven
by one live value of the game (the audible engine speed, the engine load, the transmission speed, ...). It holds
**items** that are active in *windows* of that value, with fades at both ends: **wave items** (loops and one-shots),
**Ginsu items** (granular engine recordings) and **mixer items** (the gain of a mixer channel). Gain and pitch of an item
are **modulators**: a constant, or a live value mapped through a curve.

For the Porsche 911 Carrera S the engine you hear is therefore **not** the loops the earlier converter mixed: it is three
**Ginsu voices** (the same granular engine synthesis as the 2005 game, in a version-3 resource) crossfaded by the
**engine load**, an idle loop that hands over to them just above idle, and five **whine loops** driven by the car's speed.
The high-rev "harmonics" are not a synthesiser: they are mixer-channel DSP parameters (filters) that the graph drives
([below](#the-dsp-enginesound2)).

## The objects **[verified]**

Every sound graph object is a `GenesysObject` (0x15). Its import table lists, with bit 31 of the slot offset set, the
**class** of the object (slot offset `0x8000_0000`, the object's own type), the classes of every structure inline in it
(slot offset `0x8000_0000 | offset`) and the **handles** it points to (samples, Ginsu resources, mixer channels; slot
offset `0x8000_0000 | offset`, id not a type). A type is told from a handle by the id being a kind 0x14 resource. A
structure starts with 12 bytes (two zero words and the type hash); fields follow at the offsets of its type
([chameleon-vehicle-stats.md](chameleon-vehicle-stats.md)). Field names are hashed, so the offsets below were read from
the type records and the meaning inferred from values and from the expression strings the objects carry.

Strings (`StringBase`, 8 bytes): a u32 **offset relative to the field** and a u32 length that counts the zero. Arrays of
structures are a u32 absolute offset to a table of u32 absolute offsets (one per structure; the structures follow the
table; the count is where the table's entries stop naming a type import). Arrays of handles are an offset to 8-byte slots
(a handle slot is 8 bytes, the pointer in the first four).

| Class | Size | Fields (offset: meaning) |
|---|---|---|
| `Sequence` | 44 | 12 driver expression (string), 20 id, 24 **hi** (f32), 28 **lo** (f32), 32 items (table) |
| `SequenceItem` (base of every item) | 36 | 24 **hi**, 28 **lo**, 20 id, 12 name |
| `WaveSequenceItem` | 228 | 36 gain modulator, 92 pitch modulator, 148 fade-in, 168 fade-out, 204 mixer channel (handle), 212 waves (handles), 216 more modulators, 224 pitch tracks the driver (u8), 225 pick one wave at random (u8), 226 number of waves (u8) |
| `GinsuSequenceItem` | 92 | 36 fade-in, 56 fade-out, 76 Ginsu resource (handle), 84 mixer channel (handle) |
| `MixerChannelSequenceItem` | 60 | 44 mixer channel (handle), 52 values (inline, 36 bytes each) |
| `MixerChannelDoubleValue` | 36 | 12 and 16 two ids (both 0: the channel's gain; else a DSP parameter), 24 pointer to a modulator |
| `ModulatingDoubleValue` (modulator) | 56 | 12 expression (string), 20 scale, 24 input max, 28 input min, 32 output max, 36 output min, 40 base, 44 curve points (inline), 52 point count (u16), 48 and 50 two enums, 54 and 55 two bools |
| curve point | 36 | 12 x, 16 y, 20 and 24 left tangent, 28 and 32 right tangent |
| fade | 20 | 12 shape (f32), 16 span (f32) |
| `MixerChannel` | 96 | 48 gain (f32); 12 and 20 parent buses (handles) |

`Sequence` and item class names, the 352 type resources of a bundle, and the offsets were checked on all the sequences of
the Porsche (`VEH_1085007_HI.BNDL`) and the counts of the walk against the objects' imports (every item type import is
consumed; the point count equals the number of points whose header matches) **[verified]**. The 93 car bundles share the
types; only the Porsche's values were read in detail.

## What a sequence does **[inferred]**

- **Range and direction.** A sequence has two floats, written `hi` (offset 24) and `lo` (28); its live value is clamped
  into `lo..hi`. Items have their own `lo..hi` in the same unit. The sequence runs from `lo` to `hi`: an item is active
  while the value is inside its window, **fades in at its `lo`** and **out at its `hi`**. Reading it the other way round
  would make the Porsche's idle loop (window 1790 to 2290, a fade-out span of 500) play loudest *above* the engine speed
  where the Ginsu voices take over, and silent at idle; read this way it is loud at the bottom of the range and gone 500
  units up, as an idle loop is. Gear-change sequences (range 0 to 6, items 0 to 4) run from the time elapsed 0 upwards.
- **Fades.** A fade has a **span** in the unit of the live value (500 audible rpm, 20 mph, 100 rpm) and a **shape**. The
  gain over the fade is `(distance / span) ^ shape`; a span of 0 is a hard edge. Shapes found: 1 (linear), 0.5 (an
  equal-power crossfade, the idle loop out against the Ginsu voices in) and a few others (2.058, 1.0) on the Ginsu
  fade-outs. The exponent reading is the one that makes the 0.5 pairs equal power; it is not proven.
- **The audible engine speed.** The live value `GetPlayerAudibleRPM` is, by `NFS13.exe`'s accessor, a linear mix of two
  numbers of the engine sound object by a normalised engine speed **[exe]** (`lerp(a, b, normalised rpm)`, with `a` and
  `b` read from the object at 0x1C0 and 0x1C4). The two numbers are not in the car bundle. The Porsche's sequences span
  1790 to 7500, so this mapping puts idle at 1790 and the limiter at 7500 **[inferred]**: the converter treats the
  sequence's own range as idle to redline (the range's ends are the ends of the engine's speed), which is why a window is
  written as a fraction of the way from idle.
- **Ginsu items** have no gain of their own: their level is the gain of their **mixer channel**, which a
  `MixerChannelSequenceItem` of the engine-load sequence sets by a curve ([next](#mixer-channels-and-the-engine-load)),
  times the channel's base gain (0.7087 on the Porsche's two accelerating voices, 1 on the third).
- **Wave items** play one wave of the list (or the one wave). When byte 224 is set, the pitch modulator's **base** is not
  a pitch but the live value at which the sample plays at its own speed: the rate is `value / base`. The Porsche's
  transmission whine loops (windows 20 to 90, 70 to 140, 119 to 250, 20 to 100 and 80 to 250 mph, bases 40, 95, 125, 75
  and 150) are the five; all other wave items leave it clear and play at rate 1 (a modulator of base 1).

### Modulators and curves **[inferred]**

A modulator with **no expression and no curve** gives its `base` (the whine loops' gain 0.6, the idle loop's 0.5). One with
an expression or a curve maps the live value `v`: `t = clamp((v - in_min) / (in_max - in_min))`, `y = curve(t)` (or `t`
with no curve), result `scale * (out_min + (out_max - out_min) * y)`. An empty expression reads the sequence's own driver
value (for a transmission-speed item with a curve, the position in the item's window, [below](#the-electric-motor)). The Porsche's engine-load mixer curves span `out` 0 to 2 and 0 to 4, so a curve value above 1 is a gain boost.

A curve is a list of points with tangents, drawn as cubic Bezier segments. This reader takes a tangent as the **fraction
of the segment's width and height** at which the handle sits (`a + (tx * width, ty * height)` for the right handle of
`a`; `b - (tx * width, ty * height)` for the left handle of `b`). It keeps the engine-load curves between their points and
monotone where their points are, and the default tangent `(1, 0)` of the first and last point gives a flat start and end.
Another reading (absolute handle lengths) overshoots the points by several times; this one was chosen for that
reason **[inferred]**. The converter writes curves sampled to at most 32 piecewise linear points, so only the curve
*shape* matters, not the exact Bezier.

## Mixer channels and the engine load **[verified structure, inferred meaning]**

The sequence driven by `Players.LocalPlayer.GetPlayerEngineLoad` (range 0 to 1) holds twelve mixer items, one per
channel it sets. The Porsche's three Ginsu voices of the first set have these gain curves of the load (0 idle throttle,
1 full):

| Voice (channel) | Output range | Curve (load, y) | Reads as |
|---|---|---|---|
| 1 (`7343a`) | 0 to 2 | (0, 0) (0.21, 0.28) (0.33, 0.20) (0.41, 0.04) (0.58, 0) (0.94, 1) (1, 1) | on-throttle: silent below 0.6, full above 0.9 |
| 2 (`7345a`) | 0 to 4 | (0, 0) (0.09, 0.89) (0.60, 1) (0.85, 0.42) (1, 0.08) | part throttle: loud from 0.1 to 0.6, gone at full throttle |
| 3 (`7343e`) | 0 to 1 | (0, 1) (0.21, 1) (0.49, 0) (1, 0) | off-throttle: full below 0.2, gone by 0.5 |

so the engine crossfades between three recordings by the throttle: coast, part throttle, full throttle **[verified from
the curves; the names are readings]**. The second set (`73467` to `73469`) has the same structure with slightly different
curves.

## Two sets of Ginsu voices **[verified count, unknown meaning]**

The Porsche has two sequences of the local player driven by the audible engine speed, `170aa6` and `1d1c83`: each three
Ginsu items over the whole range and an idle loop (`1d1c7b` and `1d1c7a`), with their own mixer channels under two
different parent buses (`7347f` and `7343b`). Both are in the engine's list. One could be the interior mix and the other
the exterior, or two layers that both play; nothing in the data says. The converter takes **the first in the bundle**.
Two sequences driven by `RaceCars.EntityEngineSound["Id"].GetAudibleRPM` (opponents and traffic: two Ginsu voices and idle
loops, whose engine-load sequence sets DSP parameters, not gains) are not used.

## Whose sequences they are: `EngineSound2` and `TransmissionWhine` **[verified]**

A bundle holds the sequences of several sounds, and a car sounds like the player's, not like the sum of them. Two kinds
of object gather sequences by handle (`SoundGraphs::groups`, `libs/chameleon-sound-graph`):

| Class | Per bundle | Holds |
|---|---|---|
| `Genesys.Gen.EngineSound2` (260 bytes, handles at 0x0c to 0x1bc) | two | the engine sound of **the local player** (the one that holds a sequence driven by a `LocalPlayer.` value: the engine load, the speed, the audible engine speed, the gear change time, `GetNormalisedRPM`, ...), and of **the opponents and traffic** (`EntityEngineSound`, `GetDistanceToListener`, `GetSpeedMph`: two Ginsu voices and idle loops, the same sequences on every car) |
| `Genesys.Gen.Vehicles.Sound.TransmissionWhine` (112 bytes) | one | the **gas** sequence (`GetPlayerGas`), the **whine** sequence (`GetTransmissionSpeedMph`, five loops) and the **reverse** whine (`GetReverseTransSpeedMph`) |

All 93 car bundles have both kinds. The whine object's ids are the **same on every car** (`d672d`, `d6751`, `11f220`: a
template instance copied into each bundle), so the gearbox whine is one set of five loops that every car has. What differs
between cars is the player's `EngineSound2`: a combustion car holds two sets of audible-speed sequences (the Ginsu voices
and the idle loop) and the gear change sequences; the **Tesla Roadster** holds neither, and holds a transmission-speed
sequence of its own, the motor ([below](#the-electric-motor)). A sequence in the opponents' engine sound is never played
for the player: an older reading took every transmission-speed sequence of the bundle and so played the Tesla's motor twice
(the player's and the opponents' copy, whose channels differ).

## The Ginsu resource (kind 0x80) **[verified]**

A car bundle holds about eight, 330 to 350 KB each. The main block:

| Offset | Content |
|---|---|
| 0x00 | f32 volume (0.5) and a zero word |
| 0x08 | `"Gnsu30"`, two zero bytes: the magic and version 3 |
| 0x10 | f32 lowest frequency (1614.4), f32 highest (7823.8) |
| 0x18 | u32 `seg_count` (50), u32 `cycle_count` (505), u32 `sample_count` (583,130), u32 sample rate (48,000) |
| 0x28 | two words (0 and 721, meaning not known) |
| 0x30 | `seg_count` u32 frequency positions, then `cycle_count + 1` u32 cycle positions |
| ... | EA-XAS version 0 audio, 19 bytes per 32 samples |

This is the `.gin` layout of the 2005 game ([audio.md](audio.md#gnsu-granular-engine-sounds-gin)) with a version-3 header:
the same pitch-cycle tables (cycle 0 is 3,583 samples at 48 kHz, which is 1,607 in the file's frequency unit, the lowest
frequency; the last is 746 samples, 7,721), the same codec. The one difference found is that the frequency table has
`seg_count` positions, not `seg_count + 1`; the converter puts the first cycle's position in front so the 2005 readers
parse the result. **Verified:** with that, `ea_audio::gin::decode` decodes all 583,130 samples and the audio is smooth
(the energy of its first difference is 1.3 percent of its energy, which a misaligned payload would not give), and
`blackbox-ginsu` plays it. **Unverified:** that the missing first frequency position is the first cycle's, nothing was
listened to.

Ginsu frequencies are engine rpm: a cycle of the recording is one four-stroke cycle, so its frequency in the file's unit is
`120 / period`. That is why the audible engine speed (1790 to 7500) is the frequency a voice is played at.

## The transmission whine **[verified windows, inferred pitch rule]**

`RaceCars.Entities["Id"].GetTransmissionSpeedMph` (range 0 to 170, windows beyond it) holds five wave items, each of one
loop. The converter writes each as a layer whose volume follows its window in mph and whose rate is the car's speed over
the item's base, so the whine rises with speed and the loops crossfade over 20 to 40 mph. Over their bases the five loops'
strongest frequencies (a 16,384-sample spectrum of the middle of each, `graph_voices/real_tests.rs`) are:

| Window (mph) | Base | Strongest | Hz per mph of base |
|---|---|---|---|
| 20 to 90 | 40 | 508 Hz | 12.7 |
| 20 to 100 | 75 | 680 Hz | 9.1 |
| 70 to 140 | 95 | 1,018 Hz | 10.7 |
| 119 to 250 | 125 | 2,766 Hz | 22.1 (a harmonic is likely) |
| 80 to 250 | 150 | 854 Hz | 5.7 |

The three loops of the first three rows rise with their bases within a factor of 1.4, which supports "the base is the
speed the loop was recorded at"; the last two do not fit (a harmonic is stronger than the fundamental, or the base of that
item means something else). The rule is therefore **[inferred, weakly supported]**; nothing was listened to.

**The gas pedal crossfades the whine [verified structure].** The whine object's gas sequence (`GetPlayerGas`, 0 to 1) holds
two mixer items on the channels the loops play on: the three loops of the first channel (`d66f1`) follow (0, 0) (0.32, 0.03)
(0.75, 1) (1, 1) and the two of the second (`d672a`) the opposite, (0, 1) (0.32, 1) (0.75, 0) (1, 0). So the gearbox is heard
on the throttle with the first set and off it with the second, which the converter writes as `load` curves on the loops.
Each loop has a channel gain of 0.16 and an item gain of 0.6: against the Tesla's motor (gain 2, channel 1) the whine is 26 dB
under it.

## The electric motor **[verified structure, inferred reading of the curves]**

The Tesla Roadster (`VEH_1085764`) has no Ginsu resource (the bundle holds 16 samples), no audible-speed sequence, no gear
change, pop, backfire or turbo sequence. Its player engine sound holds one transmission-speed sequence (`17f7a6`, range 0
to 170 mph) with two **wave items** that are the whole sound:

| Item | Window (mph) | Loop | Gain | Pitch (playback rate) |
|---|---|---|---|---|
| 1 | 0 to 150, hard edges | `17f7a4` | 2 on a channel of 1 | 1 to 2.2 by a curve: (0, 0) (0.39, 0.8) (1, 0.99) |
| 2 | 5 to 200, hard edges | `17f868` | 2 on a channel of 1 | 0.8 to 1.2 by a curve: (0, 0) (0.54, 1) (1, 1) |

The pitch modulators have no expression and a curve: **the curve is read over the item's window**, `t = 0` at the window's
low end and 1 at its high end **[inferred]**, so loop 1 plays at its own speed standing, at 1.96 times it at 59 mph and at 2.2
times at 150 mph, and loop 2 at 0.8 up to 1.2 times by 111 mph. The reading is supported by a second car that uses the same
loop `17f7a4` (a hybrid, `VEH_1085830`, next to its Ginsu engine): its item, window 5 to 150 with a fade-out of 50 and a
pitch of 1.8 to 2.8 by the same kind of curve, is accompanied by a **mixer item on the loop's own channel** (window 5 to 150)
whose curve is (0, 0) (0.52, 0.985) (1, 0): over the window the motor swells from nothing at 5 mph to a peak near 80 mph and
falls to nothing at 150, which only makes sense with the curve read over the item's own window. The same reading is used for
the mixer curve (`speed_mph` points in the manifest).

The sequence also has mixer items for four channels (`b381c`, `73504`, `7504`, `87ee3`) whose curves are over the speed
(`b381c`: 0.4 at standstill, 1 at 55 mph, 0 from 120; `73504`: 0 to 0.96 at 80 mph and 0.38 from 140). No wave plays on them:
they are the channels of the engine's DSP chain (the tracked-harmonic and sub-harmoniser plug-ins, [below](#the-dsp-enginesound2)),
which are **not ported**. If those plug-ins synthesise a tone that rises with the speed, the Tesla in the 2012 game has it in
addition to the two loops; the data does not say.

## The DSP: `EngineSound2` **[partly decoded, not ported]**

`Genesys.Gen.EngineSound2` (260 bytes, `170aa5` for the Porsche) is the car's engine sound **container**, not a
synthesiser. It holds 12 handle arrays (the sequences listed above and the mixer channels), 40 floats of parameters and
27-element arrays of `DspParam` (mixer channel, parameter id, default value) and `GainParam`. Its classes name the
plug-ins that run on the mixer channels: `LowPassButterworth`, `HighPassButterworth`, `BandPass`, `Peaking`, `HighShelf`,
`Distortion`, `Delay`, `SubHarmoniser`, `Pan2D`, `Send` and `Submix`. The `TrackedHarmonic1Frequency`,
`TrackedHarmonic2Frequency`, `TrackedHarmonicTunnelFrequency`/`Gain`/`Q`, `SubharmoniserFrequency` and `CombRaspDelayTime`
names are **outputs** of the engine sound object that graph expressions set (`NFS13.exe` registers them as writable
properties of `RaceCarEngineSound` at 0x100 to 0x118 **[exe]**) and mixer values then read: they are the centre
frequencies and gains of filters that track the engine order, applied to the Ginsu voices' channels. The `b71b7` sequence
(driven by the normalised engine speed, 0 to 1) holds the curves that set them, and a few short grains (`f3468` to `f346f`).
There is **no additive harmonic oscillator**, so there is nothing to port as a synthesiser; reproducing the tracked
harmonics needs per-channel filters in the audio mixer. This is not implemented: the mod car plays the Ginsu voices and
loops unfiltered.

## What is not decoded **[unknown]**

- the second modulator array of a wave item (`216`: it holds `GetEngineHeat` on the Porsche's grain item), the two
  enums and the two bools of a modulator, the item enum at 32;
- the gear-change, pop, backfire, turbo, shunt and LFO sequences' behaviour (the mod still picks their samples by the live
  values they read, [chameleon-sound.md](chameleon-sound.md));
- the gear change, pop, backfire and turbo sequences of the opponents' engine sound (not played for the player);
- DSP parameter curves (the `GainParam` and `DspParam` values), the `EngineOn` and `RevLimitGain` gates, the entity
  (opponent) engine set;
- how the game maps the engine's own rpm onto the audible range (only the accessor's form is known).

## Evidence for the player-engine reading

| Claim | Evidence |
|---|---|
| Items fade in at `lo`, out at `hi` | idle loop window 1790 to 2290, fade-out span 500 shape 0.5, against Ginsu fade-in span 100 shape 0.5 at 1790: equal-power handover just above idle |
| Whine pitch is `speed / base` | five loops with the tracking byte set only on them and bases that rise with their windows; three of the five loops' strongest frequencies scale with the base, two do not (table above) |
| Ginsu gain by load | three mixer items on the Ginsu items' channels, curves above |
| Ginsu is the 2005 granular format | tables and audio decode with the 2005 readers |
| No additive synth | the `EngineSound2` class has no oscillator member; the named outputs are filter parameters |
