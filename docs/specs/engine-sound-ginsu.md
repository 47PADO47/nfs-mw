# Ginsu: the granular engine synthesiser

How the original turns a `.gin` file (a recording of an engine sweeping through its RPM range, cut into
pitch cycles) and a target frequency into a continuous engine sound. Part of
[engine-sound.md](engine-sound.md); the file layout is in [formats/audio.md](../formats/audio.md#gnsu-granular-engine-sounds-gin).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube
  build), `src/Speed/Indep/Src/EAXSound/Ginsu/{ginsu.h,ginsudata.cpp,ginsusynth.cpp,ginsuhelper.h}`,
  `EAXSound/CARSFX/CARSFX_Engine.cpp` (how it is driven), `Libs/snd/9/include/snd/sndo.h` (packet player).
  Read for understanding; no code copied. The sample decoder (EA-XAS) is a codec, specified by vgmstream and
  implemented in `ea-audio`, not here.
- **Data inputs:** the `.gin` files of `SOUND/ENGINE/`; nothing from AttribSys.

Evidence tags as in the [docs README](../README.md#evidence-tags). **[verified]** = measured on the 160 `.gin`
files of the PC v1.3 install; **[decomp]** = from the sources above.

## 1. The data

A `Gnsu` file holds three things besides the audio **[decomp + verified layout]**:

| Table | Meaning |
|---|---|
| `min_frequency`, `max_frequency` (f32) | the frequency range of the recording, in "RPM units" |
| `freq_pos[0 ..= seg]` (`seg + 1` u32) | the sample index at which the recording's pitch equals `min + (max - min) * i / seg` |
| `cycle_pos[0 ..= cycles]` (`cycles + 1` u32) | the sample index of each cycle boundary; cycle `k` spans `cycle_pos[k] .. cycle_pos[k + 1]` |
| mono 16-bit samples, `sample_rate` Hz | EA-XAS in the file; decoded elsewhere |

A **cycle** is one period of the recording's fundamental. The **frequency unit** is such that
`period_in_samples = sample_rate * 120 / frequency`: at 7784 "RPM" and 24 kHz a cycle is 370 samples. Measured:
the cycle length at each of the 51 table knots matches `sample_rate * 120 / f` with a median error of 3.3 %
(up to 14 % in the worst knot of a file; the 120 is the engine's `Hz x 120 = RPM`) **[verified]**. The
game feeds the synth "RPM" values and reads back `pitch_Hz x 120`, so the unit is only a convention.

Measured on all 160 files **[verified]**: `seg = 50` in every file; 18 to 265 cycles; 9,932 to 223,531
samples; rates 32,000 (103 files), 24,000 (41), 36,000 (14), 34,000 and 20,000 (one each);
`cycle_pos[0] = 0` and `cycle_pos[cycles] = sample_count - 1` in every file; `cycle_pos` strictly increasing;
`min_frequency` 1,031 to 6,065 and `max_frequency` 2,011 to 9,241. **Accelerate files** (`GIN_<car>.gin`, 85) have a
rising `freq_pos` (the recording is a rev up) and **decelerate files** (`..._DCL.gin`, 75) a falling one (a rev
down): `freq_pos[0]` is near the end of the file for them. Both kinds work the same in the synth.

### 1.1 Lookups

All on the tables; `floor`, `ceil` as usual, `round(x)` = nearest integer, ties away from zero
(`x > 0 ? trunc(x + 0.5) : trunc(x - 0.5)`). All arithmetic is `f32` in the original **[decomp]**.

```
frequency_to_sample(f):                      # f in RPM units
    if seg < 1: return 0
    if f <= min:  return freq_pos[0]
    if f >= max:  return freq_pos[seg]
    x = seg * (f - min) / (max - min);  i = floor(x);  a = x - i
    return round(freq_pos[i] + a * (freq_pos[i+1] - freq_pos[i]))

cycle_to_sample(c):                          # c is a fractional cycle index
    if cycles < 1: return 0
    if c <= 0: return cycle_pos[0];   if c >= cycles: return cycle_pos[cycles]
    i = floor(c);  return round(cycle_pos[i] + (c - i) * (cycle_pos[i+1] - cycle_pos[i]))

sample_to_cycle(s):                          # the inverse; any monotone search gives the same answer
    if cycles <= 0 or s <= cycle_pos[0]: return 0
    if s >= cycle_pos[cycles]:           return cycles
    find g with cycle_pos[g] <= s < cycle_pos[g+1]
    return g + (s - cycle_pos[g]) / (cycle_pos[g+1] - cycle_pos[g])

cycle_period(c):                             # samples per cycle at fractional cycle c, smooth between boundaries
    if cycles < 1: return 0
    i = floor(c)
    if i < 1:                                # first cycle (negative c clamps to 0)
        if i < 0: c = 0, i = 0
        p0 = cycle_pos[1] - cycle_pos[0];            p1 = (cycle_pos[2] - cycle_pos[0]) / 2
    elif i >= cycles - 1:                    # last cycle (c beyond the end clamps to cycles)
        if i >= cycles: i = cycles - 1, c = cycles
        p0 = (cycle_pos[cycles] - cycle_pos[cycles-2]) / 2;  p1 = cycle_pos[cycles] - cycle_pos[cycles-1]
    else:
        p0 = (cycle_pos[i+1] - cycle_pos[i-1]) / 2;   p1 = (cycle_pos[i+2] - cycle_pos[i]) / 2
    return p0 + (c - i) * (p1 - p0)
```

`cycle_period` is the central-difference period at each boundary, interpolated linearly in between. With
fewer than 2 cycles the formulas for `cycle_pos[2]` do not exist; the original never meets such a file
(minimum 18 cycles), a reimplementation can return `cycle_pos[1] - cycle_pos[0]`.

## 2. Constants

Derived from the file's `sample_rate` in `f32`, `round` as above **[decomp]**:

```
packet_size  = round(sample_rate * 0.011)      # 11 ms: samples produced per packet
no_jump_size = round(sample_rate * 0.011)      # 11 ms: minimum time between two jumps
overlap_size = round(sample_rate * 0.0005)     # 0.5 ms: cross-fade length at a jump
```

At 32 kHz these are 352, 352 and 16; at 24 kHz 264, 264 and 12. The latency argument of the update is in
milliseconds (the game passes 60), and `packets_to_target = floor(latency / 11) + 1` (the original multiplies by
`0.09090909`, 1/11), so a latency of 60 ms is 6 packets.

## 3. State

```
playback_pos      # the sample being read from the recording (an integer)
current_pos       # where the synth "is" on the frequency axis, as a sample index of the recording (an integer)
target_pos        # frequency_to_sample(target frequency)
countdown         # packets left to reach target_pos (>= 1)
no_jump_remaining # samples until another jump is allowed
current_cycle     # sample_to_cycle(current_pos), refreshed at the start of every packet
```

`start(f0)`: `playback_pos = current_pos = target_pos = frequency_to_sample(f0)`, `countdown = 1`,
`no_jump_remaining = no_jump_size`, `current_cycle = sample_to_cycle(playback_pos)`.
The original also submits two silent packets so the player starts with 22 ms of silence; that is a
hardware queue detail and not reproduced.

`update_frequency(f, latency_ms)`: `target_pos = frequency_to_sample(f)`;
`countdown = floor(latency_ms / 11) + 1`. Called once per game update, usually many times per second; the
packets that follow chase the new target.

`current_pitch()` = `sample_rate / cycle_period(current_cycle + 0.5)` Hz. The game multiplies it by 120 (and
by its sub-minimum pitch ratio, effects spec) to get the RPM it reports back **[decomp]**.

## 4. Making one packet

The player asks for a packet whenever one finishes (every 11 ms). One packet is built as follows
**[decomp]**; this is the entire synthesis.

```
current_cycle = sample_to_cycle(current_pos)
change        = (target_pos - current_pos) / countdown          # f32: samples the target position moves this packet
jump_time = 0;  jump_dist = 0                                   # in packets and in whole cycles

if no_jump_remaining < packet_size:                             # a jump is allowed within this packet
    play_cycle    = sample_to_cycle(playback_pos)
    play_rate     = packet_size / cycle_period(play_cycle)      # cycles the playback advances per packet
    target_rate   = change / cycle_period(current_cycle)        # cycles the target moves per packet
    pos_diff      = current_cycle - play_cycle                  # how far the target is ahead, in cycles
    rate_diff     = target_rate - play_rate
    nj            = no_jump_remaining / packet_size             # fraction of the packet before a jump is allowed
    at_nojump     = pos_diff + rate_diff * nj                   # predicted lead when jumps become allowed
    at_end        = pos_diff + rate_diff                        # ... at the end of the packet
    if floor(at_nojump) != floor(at_end):                       # the lead crosses a whole number of cycles
        jump_dist = ceil(at_nojump)  if at_nojump < at_end  else floor(at_nojump)
        jump_time = (jump_dist - pos_diff) / rate_diff          # when exactly the lead equals jump_dist
    else:
        max_dist = ceil(|rate_diff| * 0.99999994)
        if |at_nojump| > max_dist:                              # the lead is already too large: catch up now
            jump_time = nj
            jump_dist = ceil(at_nojump) if at_nojump < 0 else floor(at_nojump)

if jump_dist == 0: jump_time = 1
n = round(jump_time * packet_size)                              # samples before the jump
out[0 .. n] = recording[playback_pos .. playback_pos + n];  playback_pos += n;  no_jump_remaining = max(0, no_jump_remaining - n)

if jump_dist != 0:
    a = recording[playback_pos .. + overlap_size]                # the old position, continued for the overlap
    c = sample_to_cycle(playback_pos)
    playback_pos = cycle_to_sample(c + jump_dist)                # jump by whole cycles
    b = recording[playback_pos .. + overlap_size]                # the new position
    out[n + i] = a[i] + (i / overlap_size) * (b[i] - a[i])       # linear cross-fade, i = 0 .. overlap_size - 1, rounded
    n += overlap_size;  playback_pos += overlap_size;  no_jump_remaining = no_jump_size - overlap_size
    rest = packet_size - n
    if rest > 0:  out[n .. n + rest] = recording[playback_pos .. + rest];  n = packet_size;  playback_pos += rest;  no_jump_remaining -= rest

current_pos = round(current_pos + change);  if countdown > 1: countdown -= 1
emit out[0 .. n]      # normally packet_size samples; more than packet_size if the jump left no room (n + overlap)
```

How it behaves: the playback runs forward through the recording at its natural rate, so the pitch it plays
is the pitch of the recording at that spot. The target position (`current_pos`) glides toward the requested
frequency's spot. The playback is kept within one cycle of the target by **whole-cycle jumps**, taken at the
instant the lead is an exact number of cycles, so a jump lands at the same phase of the waveform in another
cycle and the 0.5 ms cross-fade hides the rest. With a constant target the playback runs one cycle past it and
jumps back one cycle, over and over: the sound is one pitch cycle of the recording looped (a grain). When the
target moves, the cycles jump along the recording and the pitch follows the recording's own pitch curve, with
the recording's timbre at that RPM. At least `no_jump_size` samples (11 ms) lie between jumps, which caps the
rate of change of the pitch. A very large target step is closed over `packets_to_target` packets.

The output has the file's sample rate, one channel, 16-bit (the original converts the decoded float to
`i16` with saturation). Playback speed for the pitch below the minimum frequency and for the dynamic
mixer's pitch multiplier is applied by the player as a resampling ratio, not by the synth.

**Edge cases.** Reads outside `0 .. sample_count - 1` fail in the original and leave the destination
unchanged (stale buffer contents): that happens only if the playback runs off the end, which the targets
clamped to `freq_pos[seg]` avoid in the shipped data. A reimplementation fills such samples with silence.
The decoded block cache of the original (`DecodeBlock`) is an optimisation.

## 5. Driving it

The engine object (engine-sound.md §6) calls `update_frequency(freq, 60)` every game update with
`freq = max(ginsu_freq, min_frequency)`; when `ginsu_freq` is below the file's minimum the loop plays at the
minimum and the player's playback speed is multiplied by `ginsu_freq / min_frequency`. The volume of each
loop is set separately on the player (a 0 to 127 value from the mix), and the dual mode runs two
independent synths with the same `freq` (they do not stay in phase).

## How to check it

- Pitch accuracy: for a constant target `f` inside `[min, max]`, the fundamental of the output should be
  `sample_rate * 120 / cycle_period` at the target spot, which is within about 3 % of `f` for the shipped
  files (the data's own error, §1). Decode a `.gin`, hold `f`, and compare an autocorrelation period.
- Glide: sweep `f` from `min` to `max` over a second; the output should have no clicks (the cross-fade is
  0.5 ms) and the pitch should rise monotonically (accelerate files) without steps larger than one cycle's
  worth per 11 ms.
- Determinism: same data, same call sequence, bit-identical output.
- Compare against the PC game's engine at a fixed RPM (recording of the process's `SNDPKTPLAY_submit`
  buffers, if hookable) **[unconfirmed]**.

Done on the install with the Rust implementation (`crates/nfsmw-data/tests/real_install/ginsu.rs`,
**[verified]** for the BMWM3GTR loops and 13 other engines): at 25, 50 and 75 % of each file's range the output
repeats at the table's cycle length (within 3 %, autocorrelation above 0.8), the reported frequency is within 8 %
of the request, and a 4 s pull through the range has no level jump above a factor of 3 between 50 ms windows.
That checks the synthesis against the file's own tables; it does not compare with the original's audio.
