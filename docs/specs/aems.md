# AEMS: running a module of a sound bank

How the original's event-sound system evaluates the dataflow graph of a module (layout:
[formats/aems.md](../formats/aems.md)). Used by the car sound for the engine's sample layer, the sputters, the
shift sweeteners and the gear whine ([engine-sound-aems.md](engine-sound-aems.md)); any other class of the banks
runs the same way.

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube
  build), `src/Speed/Indep/Libs/snd/9/`: `extern/aemsdef.h` (node structures), `source/library/cmn/saems.c` (node
  update functions, player, module instances, bank resolution), `saemstimupdt.c`, `sndcmn.h`. The PC functions
  with the numbers 0 to 3, 7, 11, 15 and 31 of the table below were read in `speed.exe` (disassembled) and match.
  Read for understanding; no code copied.
- **Data inputs:** the module banks (`.abk`) of `SOUND/ENGINE`, `SHIFTING`, `TURBO`, `SKIDS`, `NOS`,
  `IG_GLOBAL`.
- Tags as in the [docs README](../README.md#evidence-tags). **[verified]** = measured on the PC banks.

Integers are `i32` with wrap-around; floats are `f32`. "Word at +k" means the little-endian dword (or the stated
width) at byte `k` of a node.

## 1. Instances and the update

A module is instantiated when the game creates a Csis object of its class (`CAR`, `CAR_SWTN`, ...) and removed
when the graph's destroy node fires or the object is deleted. An instance has its own copy of the data, a
random-number state and a set of players. At most `max instances` of a module exist.

The game sets the class data (the object's parameters) whenever it changes them (`SetMemberData`), then once per
**update tick** calls the module's code, which runs every node once, in the order of the code, and routes the
results. The tick period `T` is in milliseconds and may vary; the original uses the sound system's variable timer.
This implementation uses the fixed 60 Hz tick of the car sound (`T = 16.6667`).

## 2. The node functions **[verified: numbering, sizes]**

The code calls node functions by number. `node(k)` is the word at +k. Outputs are the function's return value; the
code stores it where it is read.

| N | Node | Size | Behaviour |
|---|---|---|---|
| 0 | class destructor | 0x14 | returns the flag at +0x10 and clears it (the game sets it when the object is deleted) |
| 1 | class data | 0x14 + 4n | returns word +0x14 (the first parameter); the code reads the others at +0x14 + 4k |
| 2 | global variable | | returns +0x18 (not used by any bank) |
| 3 | create | 4 | returns +0 and clears it |
| 4 | destroy | 0x10 | if the flag at +0xC is set the instance is torn down: every player is stopped, the instance ends |
| 5 | call function | | calls a Csis function (not used) |
| 6 | counter | 0x18 | `if min <= override <= max: return override`; if `triggered > 0` step `value` by `direction`, wrapping from `max` to `min` and back; return `value`. Words: min +0, max +4, value +8, direction (i8) +0xC, triggered +0x10, override +0x14 |
| 7 | random | 0x10 | if `triggered` (+0xC) is 0 return the last output (+8); else output = `random % range (+4) + min (+0)`, remembered |
| 8 | random shuffle | var | `inputs` at `+inputoffset` (u16 at +0): its word is the trigger. Draws without repeat from a permutation: `swap = random % (range - index - avoid) + index`; output = set[swap] + min; set[swap] <-> set[index]; `index++`; at `range` the index returns to 0 and `avoid` becomes 1. Entries are u8 or u16 (+2); `avoid` (i8) +3, min +4, index u16 +8, range u16 +0xA, output +0xC, set at +0x10 |
| 9 | random weighted | 0x14 | trigger +0x10: `r = random % 100`; walk the u8 table (pointer +0, data at table +0x10) adding entries until the sum exceeds `r`; output = that index + min (+4); range +8, output +0xC |
| 10 | range trigger | 0x18 | input +0x14. If `trip_gte <= input <= trip_lte` (+0, +4) and not `tripped` (+0x10, i8): set `tripped`, return 1. Else if `reset_gte <= input <= reset_lte` (+8, +0xC): clear `tripped`. Otherwise 0 |
| 11 | delay trigger | 0x10 | `time` f32 +0; trigger +8 sets it to 0; delay (ms) +0xC. While `time >= 0`: if `time >= delay` return 1 and set time to -1, else add `T`. Else 0 |
| 12 | state generator | var | `inputs` at `+inputoffset` (u16 +0), count u8 +2: the first non-zero input i makes the output `value[i]` (words from +8); output +4 is remembered |
| 13 | merge | var | count u8 +0, triggers from +4: 1 if any is non-zero |
| 14 | envelope | var | see §4 |
| 15 | table | 0x10 | see §3 |
| 16 | delay line | var | input value and delay time (ms) at `+inputoffset` (u16 +0). A ring of `maxslots` (u16 +2) words from +0xC; the write slot is `read slot + round(delay / T)` (capped at `maxslots - 1`) when the delay changes; each tick writes the input and returns the word at the read slot, then both advance |
| 17 | mux | var | count u8 +0; word +4 selects `k` (1 to count), the output is the word at +4 + 4k; else 0 |
| 18 | demux | 0x14 | count u8 +0, previous select i16 +2, select +4, value +8, outputs from +0xC: clears the previous output, writes `value` to output `select` (1 to count), returns output 1 |
| 19, 20 | min, max | var | count u8 +0, inputs from +4 |
| 21 | scale | var | count u8 +0, `1/scale` f32 +4, inputs from +8: product of the inputs (as float) times `1/scale`, rounded |
| 22 | add | var | count u8 +0, inputs from +4: their sum |
| 23 to 26 | subtract, multiply, divide, modulo | 8 | `a` +0, `b` +4: `a - b`, `a * b`, `a / b` and `a % b` (0 if `b` is 0) |
| 27 | player | var | §5 |
| 28 | oscillator | 0x10 | waveform u8 +0 (0 sine, 1 square, 2 saw, else triangle), phase f32 +4 (0 to 1), period (ms) +8, amplitude +0xC. If `period <= 0` return 0. `phase` first loses whole cycles; output (sine: `sin(2 pi phase) * amplitude`; square: amplitude from phase 0.5, else 0; saw: `phase * amplitude`; triangle: up to 0.5 then down) is rounded; then `phase += T / period` |
| 29 | ramp | 0x1C | `current` f32 +0, `delta` f32 +4, previous target +8, previous duration +0xC, duration +0x10, scale +0x14, target +0x18. If `target == current` return the target. When the target or duration changed: duration <= 0 jumps to the target, else `delta = (target - current) * T / duration / 4096`. Then `current += delta * scale`, clamped to the target in the direction of travel; returns `current` rounded |
| 30 | add, capped | var | count u8 +0, max +4, inputs from +8: `min(sum, max)` |
| 31 | subtract, floored | 0xC | min +0, `a` +4, `b` +8: `max(a - b, min)` |
| 32 | multiply, capped | 0xC | max +0, `a` +4, `b` +8: `min(a * b, max)` |
| 33, 34 | min2, max2 | 8 | of `a` +0 and `b` +4 |
| 35 | scale2 | 0xC | `1/scale` f32 +0, `a` +4, `b` +8: `round(a * b * (1/scale))` in float |
| 36 | add2 | 8 | `a + b` |
| 37 | function | | a Csis function's arguments (not used) |
| 38 | class controller | var | §6 |
| 39 | set global variable | | (not used) |

**Float to integer** **[verified]**: the decompiled sources say "truncate" (`ftoifast`), but the PC functions of
the scale (21), scale2 (35), envelope (14), table (15), oscillator (28) and delay-line (16) nodes end in a plain
`fistp`, which rounds to nearest, ties to even (the FPU's default mode). "Rounded" below means that; this
implementation does the same (`round_ties_even`). Only the table's interpolation index is a floor (the PC
subtracts 0.5 before `fistp`). The oscillator's sine is the sound system's own table (`iSNDsin`, not in the
sources): this implementation computes it. The random generator (`iSNDrandom`) is not in the sources either; any
uniform generator will do, this one is a seeded xorshift so that equal runs give equal sound.

## 3. Table

`ptable` (+0) points at `{u8 entry size, u8, u16 count, i32 min, i32 max, f32 resolution, entries}` (entries from
+0x10; sizes 1 and 2 are signed). The node holds `previous input` (+4), `output` (+8) and `input` (+0xC). When
the input differs from the previous one: `i = clamp(input, min, max) - min`; with resolution 1 the output is
`entry[i]`; otherwise `x = i * resolution`, `a = floor(x)`, `b = min(a + 1, count - 1)` and the output is
`round(entry[a] + (x - a) * (entry[b] - entry[a]))`. The output is held while the input does not change.

## 4. Envelope

`inputs` at `+inputoffset` (u16 +0): the control word (0 stop, 1 start/run, 2 hold, 3 release). State: previous
control (i8 +2), segment (u8 +3), remaining time f32 +4, delta f32 +8, output f32 +0xC, segment count u8 +0x10,
release segment i16 +0x12, initial value f32 +0x14, segments `{duration f32, target f32}` from +0x18. To program
a segment: `remaining = duration`, `delta = (target - output) / duration * T`.

- control 1 and previous 0: output = initial value, segment = (previous != 0), program it.
- control 3, previous not 3, segment before the release segment: segment = release segment, program it.
- control 1 or 3 and segment < count: `remaining -= T`; when it reaches 0 the output becomes the segment's
  target, the next segment is programmed, and after the last the output becomes 0; otherwise `output += delta`.
- otherwise, unless the control is 2, the output is 0.

Then previous = control; the node returns the output rounded.

## 5. Player

A player turns a sample group, a select and a play control into a sound. State at the node: sample group pointer
+4, handle +8, previous play controls (i8 x2) +0xC, input count u8 +0xE, "has outputs" u8 +0xF, sample type i8
+0x10 (-1 none), `sampleselect` +0x14, `playcontrol` +0x18; then the **inputs**, 12 bytes each `{u8 type, 3
bytes, i32 previous, i32 value}` from +0x1C; then, if "has outputs", `{i32 time left, i32 time current}` (ms).
The input types: 0 pitch multiplier (0x1000 = 1.0), 1 time multiplier (0x800 to 0x2000), 2 volume (0 to 0x7FFF,
linear), 3 azimuth, 5 reverb send, 6 low-pass, 7 high-pass, 8 dry level; 4, 9, 10 and 11 do nothing.

Each tick, with `c = clamp(playcontrol, 0, 2)`:

- If `c` differs from the previous play control: **0** stops the sound if one is playing and clears the outputs;
  **1** resumes a paused sound, else starts one: `select = clamp(sampleselect, 0, count - 1)` picks the entry, the
  entry type is remembered and the sound starts with all inputs applied (`previous = value`); if starting fails the
  outputs are cleared; **2** pauses (volume 0 and pitch 0 on the voice). The previous control is then updated.
- If `c` is 1 and nothing plays, the node returns 0.
- If `c` is 1 and a sound plays: every input whose value differs from its previous one is applied to the voice
  (pitch, volume, ...) and remembered; if the voice has ended the outputs are cleared and the node returns 0, else
  it returns 1 and, with outputs, the elapsed time and (unless the sound is a sustained loop) the time left.
- Otherwise (`c` is 0 or 2) it returns `c` if the player holds a sound and 0 if it does not (a stopped or
  never-started player reports 0, a paused one 2).

A sound that ends by itself leaves the play control at 1; the graph restarts it by taking the control to 0 and
back to 1. The game side of this (voices, banks) is the host's: a host starts, stops, pauses, resumes and updates
voices, and says whether one is still playing.

## 6. Class controller

A class controller creates and feeds another Csis object (the sputter makes its `CAR_SputOutput`, which carries
the volume back to the game). Inputs: constructor trigger, destructor trigger, then the parameters (optionally
clamped to ranges stored after the node). A destructor trigger releases the object; a constructor trigger creates
it with the parameters if it does not exist; otherwise existing objects get their data updated. It returns the
object's reference count (0 if none). The host receives these calls.

## 7. Decisions for this implementation **[decision]**

- Update tick 60 Hz, `T = 1000 / 60`; all module times are in ms.
- Random and sine as above. The sine is `sin` of the 1024-step phase, in the node's integer scale `amplitude`.
- A module whose data does not fit its image, a node pointer outside the image or a `call` of an unknown number
  stops the instance with an error, never a panic.
- The instance is a copy of the module image (the resident part of the bank), so table and sample-group pointers
  are plain offsets into it.

## How to check it

- Every one of the 386 shipped modules runs on its authored state without error (tested).
- The engine module: with a steady class state, each of the eight players holds a stable sample and the volumes
  cross-fade smoothly over the RPM range; the pitch inputs rise in proportion to the RPM (tested on the M3's bank).
- Compare with the running game: break on the volume/pitch calls of the voices of an engine and list the player
  inputs against the class data. Not done.
