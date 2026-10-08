# Dynamic-mixer maps (`SOUND/MIXMAPS/*.mxb`)

A mixer map tells the game how loud, how high and how filtered each sound object should be, given what the
game is doing: a graph of curves, envelopes, distance controls and sums, read once per frame. How the graph is
evaluated is in [specs/dynamic-mixer.md](../specs/dynamic-mixer.md); how the car sound feeds it is in
[specs/car-sound-mixer.md](../specs/car-sound-mixer.md). For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

Sources: the layout is read off the files and cross-checked against the loader of the decompilation
(`EAXSound/Dynamic_Mixer/NFSMixMap*.cpp`, **[decomp]**); the checks below were run on the four PC files with a
throwaway parser, and `blackbox-mixmap` keeps them as tests.

## Files **[verified]**

| File | Used for **[decomp]** |
|---|---|
| `MAPOUTPUT.mxb` | circuit races and free roam |
| `MAPOUTPUTDRG.mxb` | drag races |
| `MAPOUTPUT2CR.mxb` | two-player circuit |
| `MAPOUTPUT2DR.mxb` | two-player drag |

All four are 9,996 bytes of little-endian `u32`/`i32` with no magic, loaded whole into memory. They are the same
graph: against `MAPOUTPUT.mxb`, `MAPOUTPUT2DR.mxb` differs in 4 dwords, `MAPOUTPUT2CR.mxb` in 11 and
`MAPOUTPUTDRG.mxb` in 13 (a few levels and times). The `.dyn` twins (43 KB, with strings such as
`------MapTitle----`) look like the authoring files; nothing reads them.

## Layout **[verified]**

```
0x00  header, 4 x i32:  map type (0), state count (13), state table offset (0x10), dynamic-map offset (-1)
0x10  state table: 13 x i32 offsets from the start of the file, -1 = no such state
...   one block per state, in order, back to back
```

Each state block starts with a 32-byte header of 8 `i32`:

| Dword | Meaning |
|---|---|
| 0 | `0x001F0000 | state number` |
| 1 to 6 | offsets, **from the start of the state block**, of: controls, 3D controls, sub-mix channels, master channels, preset table, event controls; -1 = none |
| 7 | -1 |

The sections follow the header in the order controls, events, 3D controls, sub-mix channels, master channels,
preset table, with no gaps: in all four files each state ends exactly where the next begins and the last ends
at the end of the file. States 9 and 10 have no section at all.

The 13 states are the original's `eMAINMAPSTATES`: 0 main (volumes, music ducking), 1 music, 2 player car,
3 AI race car, 4 cop car, 5 traffic, 6 environment, 7 collision, 8 drive-by, 9 plane, 10 train, 11 helicopter,
12 truck **[decomp]**. A state is instanced once per object of its kind (several cop cars), see the spec.

Counts in `MAPOUTPUT.mxb`: 62 controls, 71 events, 30 3D controls, 51 sub-mix channels, 130 master channels;
the player-car state holds 31, 23, 8, 18 and 38.

## Source ids

Everything that points at something (a control's input, a channel's input, a trigger, a scale) is a `u32`:

| Bits | Meaning |
|---|---|
| 29 to 31 | kind: 0 control, 1 channel, 2 sound-object input, 3 sound-controller input, 4 3D control, 5 event |
| 28 | channel only: 1 sub-mix channel, 0 master channel |
| 24 to 27 | a control's own input id: the curve shape (0 to 9, spec §2); an event's own id: the envelope (0 AR, 1 ASR, 2 ATR, 3 LFO); a 3D control's own id: the number of camera states; otherwise ignored |
| 16 to 23 | state number |
| 11 to 15 | instance; **zero or an editor flag in the file** (the loader overwrites it), so readers must ignore it. The 3D and event ids of the file carry `0x3000` here in places |
| 4 to 10 | sound-object or sound-controller number (kinds 2 and 3 only) |
| 0 to 3 | input index 0 to 15 (kinds 2 and 3); for the other kinds the low byte is the index of the element in its state (control, channel, 3D control, event) |

Kinds 2 and 3 name the 16-integer input block that a sound object or sound controller of the game publishes
(`SetDMIX_Input(index, value)`); which number is which object is game data, not format.

## Controls **[verified]**

At the control offset: `{i32 count, i32 new data procs, i32 main procs, i32 main outputs}`, then `count` entries
`{u32 input id, u32 swing, u32 scale ids[n]}`. `n` is bits 16 to 20 of `swing` (at most 1 in the install). `swing`
bit 15 clear: low 15 bits are a positive offset (cents or hundredths of a dB); bit 15 set: the low 16 bits are a
negative signed value, the depth of the attenuation (-10000 is silence). The input id's bits 24 to 27 pick the
curve.

## Events

`{i32 count, 3 reserved}`, then entries of 6 dwords plus `n` scale ids: `{id, swing, trigger id, p0, p1, p2,
scales[n]}`. `swing` as for controls (bits 16 to 19 give `n`; the low 16 bits are a signed level, negative for a
fade). `p0` is the attack, `p1` the sustain, `p2` the release: bits 0 to 11 a time in frames of 1/60 s, bits 12 to
15 a curve shape. 71 events in the file: 6 AR, 11 ASR, 54 ATR.

## 3D controls

`{i32 count (low byte), 3 reserved}`, then per control `{u32 id}` followed by one 24-byte record per camera state
(bits 24 to 27 of the id give how many; 1 to 3 in the install): `{u32 info, u32 curves, u32 q0, q1, q2, q3}`.
`info` bits 24 to 27 are the camera state the record is for (0 default, 3 jump camera, 4 cut scene), bits 12 to 15
how the distance is measured (0 camera, 1 car), bits 8 to 11 the azimuth (0 camera, 1 car); `curves` holds the curve
shapes in its high half and a speed of sound in m/s in its low 16 bits (0 = no Doppler). The shape used for the
quadrants 0 to 3 of the azimuth is the nibble at bits 28, 16, 24 and 20 (the code reads a second nibble per
quadrant too, bits 16, 24, 20 and 28, and never uses it); `q0` to `q3` are the rolloff range of each quadrant of the azimuth: the near distance in
bits 0 to 14 and the far distance in bits 16 to 30, in metres.

## Sub-mix and master channels

Both start with `{i32 count, ...}` (master: also the number of distinct objects at dword 1, 15 for the player
map). A channel header word is `0xD0 | 0xC0` flags in bits 28 to 31 (`D` sub-mix, `C` master), a type nibble
(24 to 27), the number of inputs (16 to 23), the state (8 to 15) and the index (0 to 7).

- Sub-mix: `{u32 header, u32 limits, u32 input ids[n]}`; `limits` high 16 bits the upper limit (15 bits used),
  low 16 bits the lower limit (signed). All limits are within +-10000.
- Master: `{u32 header, u32 base, u32 object id, u32 input ids[n]}`; `object id` is a kind-2 id (state, object
  number, index 0). `base`: **the high 16 bits are the channel's starting level** (a signed value from -10200 to
  500 in the install, 31 of 130 are 0); the low 16 bits are 0xD8F0 (-10000) in every channel and are not used.
  The console loader reads the first short of the dword, which is the high half on a big-endian machine; on
  the PC the low half would make every channel silent, so the high half is the value **[verified by effect, see
  the spec]**.

## Preset table

One entry per master channel, in the order of the channels: `{u32 header, u32 words[count]}`; header bits 24 to 27
the kind of output (0 volume, 1 pitch in cents, 2 filter frequency, 4 depth), low 5 bits the word count. A word:
bit 31 set = this is the azimuth of the 3D control; bits 26 to 30 the output slot (0 to 29); bits 21 to 25 which
of the channel's 3D inputs it uses; low 16 bits a signed offset added to the channel's level. Counts per channel:
1 to 11.

## Open

- What the first dword of the state header (`0x1F`) means. It is not read by the game.
- The 16 bits above the offset in a preset word's bit 31 azimuth entries: not used.
