# The dynamic mixer: evaluating a mixer map

How the original turns the numbers its sound objects publish ("the car's speed is 60 mph", "the nitrous is on")
into levels, pitches and filter settings, using a mixer map (`SOUND/MIXMAPS/*.mxb`, layout in
[formats/mixmap.md](../formats/mixmap.md)). This file is the engine-generic part: the evaluator. What the car
sound feeds it and which outputs it reads is in [car-sound-mixer.md](car-sound-mixer.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube
  build), under `src/Speed/Indep/Src/EAXSound/`: `Dynamic_Mixer/{NFSMixMap,NFSMixMapState,NFSMixMaster,
  NFSMixShape}.{hpp,cpp}`, `Dynamic_Mixer/NFSMixerDefines.hpp`, `EAXSound.cpp` (the callbacks that connect
  objects), `SndBase.cpp` (how an object reads its outputs), `CARSFX/SFXObj_Enums.hpp`. Read for
  understanding; no code copied (the two lookup tables of the original are formulas, §2).
- **Data inputs:** the four `.mxb` files.
- **Tags:** [docs README](../README.md#evidence-tags). **[decomp]** = from the sources above (GameCube build);
  **[verified]** = measured on the PC files.

All integers are `i32` and all shifts arithmetic, as in the original. Every number the original keeps in a
`float` is an `f32` or `f64` here; the places where it matters are named.

## 1. Units

- **Q15 level** 0 to 32767 (`0x7FFF` = unity). **Hundredths of a dB**: -10000 is silence, 0 is unity; a *level*
  of the graph is a hundredths-of-a-dB number, and it is added, not multiplied. **Cents**: pitch, 1200 per
  octave. Positive control offsets are cents (or dB for a boost), negative swings dB.
- The mixer is evaluated once per frame with the frame time `dt` in seconds.

## 2. Shapes and dB conversion

`dB(q)` and `q15(d)` convert between a Q15 level and hundredths of a dB. The original uses two tables; both
are the following functions **[decomp, checked against the tables' end points and 20 sampled entries]**:

```
q15(d):    if d == 0: return 0x7FFF
           s = d / -602   (C division, towards zero; for d < 0 a positive shift count)
           r = -(d % 602) (C remainder, 0 to 601)
           v = round(16384 * 10^((601 - r) / 2000)) >> s
           return 0 if v < 50 else v

dB(q):     if q == 0: return -10000
           n = 0 if q & 0x4000, else the least n in 1..=16 with (0x4000 >> n) & q != 0, none: return -10000
           m = q - (0x4000 >> n)
           m = (n > 4) ? (m << (n - 5)) | ((1 << (n - 5)) - 1) : m >> (5 - n)       # 9-bit mantissa, 0 to 511
           d = -602 * (n + 1) + floor(602 * log2(1 + m / 512))
           return -10000 if d < -10000 else d
```

(602 steps per halving of the amplitude: 6.02 dB. `dB(0x7FFF)` is -1, `q15(-1)` is 32690: the pair is only
accurate to about 0.01 dB.)

`cosq(i)` for `i` in 0..=513 is `floor(32767 * cos(i * pi / 1024))` (0 for 512 and 513). The **curve shapes**
take a Q15 input `x` (0 to 32767, clamped) and give a Q15 output; the *down* shapes fall with `x`, the *up*
shapes are the mirrored ones:

| Id | Name | Output |
|---|---|---|
| 0 | down, equal power | `eq(x)` |
| 1 | up, equal power | `eq(0x7FFF - x)` |
| 2 | down, equal power squared | `eq(x)^2 >> 15` |
| 3 | up, equal power squared | the same of `0x7FFF - x` |
| 4 | down, one minus equal power | `omep(x)` |
| 5 | up, one minus equal power | `omep(0x7FFF - x)` |
| 6 | down, one minus equal power squared | `omep(x)^2 >> 15` |
| 7 | up | the same of `0x7FFF - x` |
| 8 | down, linear | `0x7FFF - x` |
| 9 | up, linear | `x` |

```
eq(x):     i = x >> 6;  rem = ((x & 0x1F) << 9) | 0x3FF;  m = cosq(i)
           d = ((cosq(i + 1) - m) * rem) >> 15;  return m == 0 ? 0 : m + d
omep(x):   i = 0x1FF - (x >> 6);  rem as above;  m = 0x7FFF - cosq(i)
           return m + (((cosq(i) - cosq(i + 1)) * rem) >> 15)
```

(`rem` uses the low 5 bits of `x` although the index uses `x >> 6`: kept.) The engine control also asks for
the same shapes **in dB**: for shapes 0 and 4 it is `dB(eq or omep)` with values below -9600 read as -10000, for the
squared shapes twice that (same cut-off), for the up shapes the negative of the down shape's dB, for linear
`dB(0x7FFF - x)`.

`pitch_ratio(c)` for cents `c`: reduce `c` by whole octaves (1200 = `0x4B0`; the original steps while
`c > 0x4AF` or `c <= -0x4B0`) and return `2^(c / 1200) * 2^octaves` (the original uses semitone and cent
tables, the same numbers); a negative `c` gives the reciprocal. The objects read pitch as
`int(pitch_ratio(cents) * 4096)` (4096 = unity). A filter slot is `int(pitch_ratio(level) * 25000)`.

## 3. The graph

A map has up to 13 states (formats doc). The game instantiates a state once per object of that kind
(`refs[state]`, 0 to 32); each instance has its own copy of every control, event, 3D control, sub-mix and master
channel of the state. Cross-state references are expanded: a reference to state `s` from another state stands for
all `refs[s]` instances of `s` (a channel input then becomes that many inputs; a scale then that many factors;
with `refs[s] = 0` it vanishes). A reference within the same state means the instance of the referencing
element. Two exceptions: the **input of a control** and the **trigger of an event** always take the element's
own instance number, whatever state they name; and a control's or event's scales compare the scale's state with
the state field of **its own input id** (not the state it lives in) to decide between "same" and "expand".

Values that elements read:

| Source kind | As a curve input or a scale (Q15) | As a channel input (dB) |
|---|---|---|
| control | the control's curve output `cq` (before the swing) | its `db` |
| channel | its output | its output |
| object / controller input | the published integer | the published integer |
| 3D control | `q15` rolloff | `db` rolloff |
| event | `q` | `out` |

Objects that are not in the game (no such input published) read as 0. Kinds 2 and 3 name an input block of 16
integers per object; the game says which exist.

## 4. One frame, in this order **[decomp]**

Elements are visited state by state (0 to 12), instance by instance, in file order; a channel that reads a
channel visited later sees last frame's value.

1. **Curves.** For each control: `cq = shape(id.bits[24..28], input)` where `input` is the source value of
   its input id (instance replaced as in §3). Controls with equal ids share one curve in the original; same result.
2. **Controls.** `swing` bit 15 clear: `offset = swing & 0x7FFF`, `depth = -offset`; set: `offset = 0`, `depth =
   int16(swing)`. `ratio = 0x7FFF - q15(depth)`. `q = ((0x7FFF - cq) * ratio) >> 15`; `out = dB(0x7FFF - q) +
   offset`; `scale = 0x7FFF`, and for each scale `scale = (value * scale) >> 15`; `control.db = (scale * out) >> 15`.
   So a control gives `depth` (a cut, silence at -10000) when its curve output is 0 and 0 (or `offset`, a boost)
   when it is full.
3. **3D controls** (§6).
4. **Events** (§5).
5. **Sub-mix channels.** `out = clamp(sum of input dB values, lower, upper)`.
6. **Master channels.** If the object it belongs to is *attached* (the game says so): `out = base + sum of the
   non-3D input dB values`; else `out = -10000`. `base` is the high half of the data dword (formats doc).
7. **Outputs** (§7).

## 5. Events (envelopes)

Each event has a trigger (a source value, nonzero = on), an envelope kind (AR, ASR, ATR, LFO), a level `swing`
(signed 16 bit, negative = a fade) and times `t0`, `t1`, `t2` (attack, sustain, release; each `(p & 0xFFF) *
16.66667` ms, a zero time counts as 1 frame, `t1` only for ASR) with curve shapes `c0`, `c2` (bits 12 to 15 of
`p0`, `p2`). State per event: `elapsed` ms, `reset_time`, `reset`, `reset_level`, `out`, `q`. Start: `out = 0,
q = 0x7FFF`.

```
if elapsed == 0 and trigger == 0: out = 0; q = 0x7FFF; reset = 0; reset_level = -10000; (nothing else)
else: elapsed += dt * 1000; run the kind; then out = out * s1 >> 15 * s2 >> 15 ... for each scale (kind, Q15 values)
shape'(c, t) = swing < 0 ? shape(c, t) : 0x7FFF - shape(c, t)
frac = (32767 - q) / 32767          # f32
AR:  if elapsed < t0: q = shape'(c0, int(elapsed * 32767 / t0))
     elif elapsed - t0 < t2: q = shape'(c2, int(32767 - (elapsed - t0) * 32767 / t2))
     else elapsed = 0; q = 0x7FFF
     out = int(frac * swing)
ASR: elapsed < t0: attack as AR; elif elapsed - t0 > t1: if elapsed - (t0 + t1) < t2: q = shape'(c2, int(32767 -
     (elapsed - t0 - t1) * 32767 / t2)) else elapsed = 0, q = 0x7FFF;  else q = 0 (hold).  out = int(frac * swing)
ATR: (the release starts when the trigger drops)
     if elapsed < t0:
         if trigger == 0: reset = 1; reset_level = out; elapsed = t0; return
         q = shape'(c0, int(elapsed * 32767 / t0))
     else if trigger == 0:
         if reset_time == 0: reset_time = elapsed
         if elapsed - reset_time > t2: reset = 0; reset_level = 0; elapsed = 0; reset_time = 0; q = 0x7FFF; return
         q = shape'(c2, int(32767 - (elapsed - reset_time) * 32767 / t2))
     else:
         if trigger == 1 and reset_time != 0: reset = 1; elapsed = 0; reset_level = out; q = 0x7FFF; reset_time = 0; return
         reset = 0; reset_level = 0; reset_time = 0; elapsed = t0; q = 0        # held on
     if reset: out = trigger == 0 ? int(frac * reset_level) : int(frac * (swing - reset_level)) + reset_level
     else out = int(frac * swing)
LFO: not implemented in the original (does nothing).
```

(The ATR's `trigger == 1` test is literal: an object that publishes `0x7FFF` as "on" never takes that branch.)

## 6. 3D controls

A 3D control turns a distance and an azimuth into a rolloff. The game publishes in the control's object's input block
`[0]` distance to the car in cm, `[1]` distance to the camera in cm, `[2]` azimuth to the car and `[3]` to the
camera (16-bit angle, 65536 = full turn, 0 = straight ahead), `[13]`, `[14]` the closing speeds (cm/s) and `[15]`
flags (bit 0 = the object has a position). On a change of camera state (0 default, 1 bumper, 2 in-car, 3 jump,
4 cut scene, 5 collision) each control picks the record whose camera state matches, else the default one (0).

```
if not (flags & 1): db = -10000; q15 = 0; azimuth = 0; DopplerCents = 0     (the whole control is off)
d = dist type 0: [1] * 0.01 m; type 1: [0] * 0.01 m; else -1          (record.info bits 12..15)
a = azimuth type 0: [3]; type 1: [2]; else 0                            (bits 8..11)
quad = (a >> 14) & 3;  u = a - quad * 0x4000;  azimuth = a
(min0, max0) = the range of quadrant quad, (min1, max1) the range of quadrant (quad + 1) & 3
shape = the curve nibble of quadrant quad (formats doc)
if d > max0 and d > max1: db = -10000; q15 = 0; return
x0 = (clamp(d, min0, max0) - min0) / (max0 - min0);  x1 likewise with min1, max1     # f32
o0 = shape(shape, int(x0 * 32767));  o1 = u != 0 ? shape(shape, int(x1 * 32767)) : 0x7FFF
q15 = ((o0 * (0x7FFF - 2u)) >> 15) + ((o1 * 2u) >> 15);  db = dB(q15)
```

(The original passes two shapes and uses the first for both quadrants.) The Doppler cents are smoothed toward the
pitch ratio of the closing speed (`dCents = 0.2 * (target - dCents)`); this implementation leaves them at 0.

## 7. Outputs

Each master channel has a list of output words (formats doc). The channel's *kind* (`0` volume, `4` depth,
`1` pitch, `2` filter, else other) decides the conversion. For every word, with `L` the channel's level, `v` the
word's signed offset, and the channel's 3D inputs numbered in input order:

- If the channel is not attached, only its **first** word is written, with `-10000` converted for volume/depth, `0`
  for pitch, `25000` for filter, `-10000` otherwise (in the original: the dB value stays raw for "other").
- If the word names a 3D input `k` that exists: with the azimuth flag (bit 31) the output is the control's
  azimuth (low 16 bits). Otherwise, by kind: volume or depth: `q15(clamp(rolloff_dB + L + v, -10000, 0))`; pitch:
  `L + v` clamped to at most 2400 and, if below -4800, set to 0 (sic); filter: `clamp(L + v, -10000, 0)` raw; other: `L`.
- Otherwise (no such 3D input): `x = L + v`; volume or depth: `q15(clamp(x, -10000, 0))`; pitch: `clamp(x,
  -4800, 2400)`; filter: `int(pitch_ratio(clamp(x, -10000, 0)) * 25000)`; other: `clamp(x, 0, 25000)`.

The 16-bit result goes into slot `slot` of the object's output block (two slots per integer, low half for
even slots). The object reads slot `i` of its block as: a volume or filter `value & 0x7FFF`; a pitch
`int(pitch_ratio(int16(value)) * 4096)`; an azimuth `value & 0xFFFF`. So a pitch slot holds cents and a volume
slot a Q15 level.

## 8. What differs here **[decision]**

- The master channel's starting level is the **high** half of the data dword (formats doc); with the low half
  (-10000 everywhere) every master channel is below silence whatever its inputs are.
- Doppler is not computed (cents stay 0).
- The evaluator has no 32-instance limit, no memory pools and no callbacks; "attached" is a flag the caller sets.
- An id whose element does not exist (a state with no instances) reads 0.

## How to check it

- Level of an output at a known input: set the inputs (a car at 60 mph with the chase camera) and compare the
  output slots with the running game's `GetDMixOutput` values (a breakpoint on `SndBase::GetDMixOutput`, or the
  sound handle volumes: `SNDvol`). Not done; the checks below are against the data and the arithmetic.
- `dB(q15(d))` returns `d` within one unit for -1500 to 0 (tested); the shapes are monotone and end at the
  documented points (tested).
- All four maps parse and every element's inputs resolve (tested against the install).
- With no input published the player-car objects read silent or near it (`ctl` outputs are cuts at 0),
  with the idle inputs of [car-sound-mixer.md](car-sound-mixer.md) they read the values listed there.
