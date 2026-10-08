# EA audio containers and codecs (`SCHl`, `ABKC`/`BNKl`, `xDFP`, EA-XA, EA-XAS, MicroTalk)

- **Sources read:** vgmstream (ISC-style licence, <https://github.com/vgmstream/vgmstream>, commit
  `7dc938f`): `src/meta/ea_schl.c`, `ea_schl_abk.c`, `ea_schl_map_mpf_mus.c`, `gin.c`,
  `src/layout/blocked_ea_schl.c`, `src/coding/ea_xa_decoder.c`, `ea_xas_decoder.c`, `ea_mt_decoder.c`,
  `src/coding/libs/utkdec.c` (derived from Andrew D'Addesio's `utkencode`, Unlicense/public domain).
  The `.mpf` / `.big` / `.abk` layouts were additionally measured on the PC v1.3 install.
  dbalatoni13/nfsmw (CC0) was not needed for this spec: it holds the runtime, not the file layouts.
  Full record: [provenance/audio-codecs.md](../provenance/audio-codecs.md).
- **Data inputs:** `SOUND/**/*.abk`, `SOUND/ENGINE/*.gin` (sample data only), `SOUND/PFDATA/MW_Music.{mpf,mus}`,
  `SOUND/SPEECH/copspeech.big`, `SOUND/STREAMS/NISAudio.big`. Layout tables with evidence tags are in
  [formats/audio.md](../formats/audio.md).
- **Scope:** decoding to PCM. The PathFinder node graph, AEMS logic, mixing and the `.idx`/`.evt`/`.csi`
  companions are out of scope.

All numbers little-endian unless stated. "BE stream" means the header platform is `GSTR` (generic), where the
values inside `SCDl` blocks are big-endian; block sizes are always little-endian.

## 1. Block streams (`SCHl` / `SCCl` / `SCDl` / `SCEl`)

A stream is a run of blocks: `char tag[4]`, `u32 size` (LE, includes the 8 header bytes), payload.

| Tag | Payload |
|---|---|
| `SCHl` | Header: platform marker + tag stream (section 2) |
| `SCCl` | `u32` number of `SCDl` blocks (stream endianness) |
| `SCDl` | One audio block (section 3) |
| `SCEl` | End of stream (size 8 or more; the walker stops here) |

A walker reads the header block, then steps `size` bytes at a time until it sees `SCEl`. Blocks with another tag
are skipped (`SCLl` loop blocks and movie blocks exist in other games; none in MW). A zero or
`0xFFFFFFFF` tag means the data ended early and is an error. A block size below 8 is an error.

### Stream extent

The extent of a stream is `[start, end of SCEl block)`. In the `.big` containers the next stream starts at the
next multiple of `0x100` at or after that end whose 4 bytes are `SCHl`; between groups of streams there are
non-stream tables (see section 7).

## 2. Header tag stream

`SCHl` payload: either `'P' 'T' u16 platform` (platform 0 = PC) or `'G' 'S' 'T' 'R'` followed by 4 bytes
(`01 00 00 30` in MW, meaning unknown; skip). Then tags until `0xFF`:

```
tag  u8
0xFC, 0xFD, 0xFE : no value (0xFE ends the header: a sub-section follows; ignore the rest)
0xFF             : end of header (then zero padding)
other            : u8 length n, then n bytes. n <= 4: big-endian value. n == 0: value 0.
                   n == 0xFF: skip a u32 BE length plus that many bytes (user data).
                   n > 4: skip n bytes.
```

| Tag | Field | Notes |
|---|---|---|
| `0x80` | version | 0..3; default by platform (PC = 0, generic = 2) |
| `0x82` | channels | default 1, at most 6 |
| `0x83` | codec1 | legacy codec ids (7 = EA-XA, 9 = MicroTalk 10:1) |
| `0x84` | sample rate | default by platform (PC 22050, generic 48000) |
| `0x85` | sample count | per channel |
| `0x86` | loop start (sample) | |
| `0x87` | loop end (sample) | stored value + 1 is the exclusive end |
| `0x88` `0x89` `0x94` `0x95` `0xA2` `0xA3` | data offset of channel 1..6 | sound banks only, absolute from the `BNKl` start |
| `0x1A` `0x26`..`0x2A` | relative loop offset channel 1..6 | MicroTalk loops only |
| `0x8C` | flags | |
| `0xA0` | codec2 | see below |

Other tags (`0x00`..`0x25`, `0x81`, `0x8A`, `0x8B`, `0x8D`, `0x8E`, `0x92`, `0x93`, `0x98`, `0x99`,
`0x9C`..`0x9F`, `0xA1`, `0xA6`, `0xA7`, `0x8F`..`0x91`, `0xAB`..`0xAD`) carry values this library does not use;
they are parsed with the generic length rule. An unknown tag is not an error as long as the length rule applies.

Codec2: `0x03` EA-XA stereo flavour, `0x04` MicroTalk 10:1, `0x16` MicroTalk 5:1 (same decoder), `0x0A` EA-XA.
When absent, PC and generic default to EA-XA (`0x0A`). Any other value is reported as unsupported.
Codec1 `0x07` is mapped to EA-XA, `0x09` to MicroTalk, when codec2 is absent.

**PCM-block revision.** EA-XA and MicroTalk streams use a later revision with PCM frames when
`version == 3`, or `version == 2` on platform PC or generic. All MW data is in this group.

## 3. `SCDl` block

```
u32 sample_count          // samples per channel in this block (stream endianness)
u32 channel_offset[ch]    // bytes from the end of this table to the channel's data (stream endianness)
data
```

For version 1 and later every channel has its own offset. For version 0 there are no offsets and the layout
depends on the codec (EA-XA stereo flavour: both channels start at `block + 0x0C + 8`).
Channel data therefore starts at `block + 0x0C + 4 * ch_count + channel_offset[ch]`.

A decoder produces exactly `sample_count` samples per channel per block, then moves to the next block. Frames that
are only partly needed are decoded and truncated. The sum of block sample counts equals the header's
sample count in every MW stream **[verified]**.

## 4. EA-XA

ADPCM with 28 samples per frame and two history samples per channel (`h1` newest, `h2` older). The history is
kept across blocks; it is set to zero at the start of the stream.

Coefficient table, 20 entries, indexed by the high nibble `i` (0..15) of the frame header as
`c1 = T[i]`, `c2 = T[i + 4]` (the two reads overlap for `i >= 4`):

```
T = 0, 240, 460, 392,  0, 0, -208, -220,  0, 1, 3, 4,  7, 8, 10, 11,  0, -1, -3, -4
```

The encoder in MW only emits `i` in 0..3 (`c1, c2` = (0,0), (240,0), (460,-208), (392,-220)), so the extra
entries are never selected **[verified: all MW frames use i < 4]**; they are kept so that a stray nibble
cannot index out of range.

### Frame, revision 2 (PCM-block revision; mono, one per channel) **[verified on all MW data]**

```
u8 info
if info == 0xEE:                              // PCM frame, 1 + 2*2 + 28*2 = 61 bytes
    h1 = s16be; h2 = s16be
    sample[0..28] = s16be
else:                                         // ADPCM frame, 15 bytes
    c1 = T[info >> 4]; c2 = T[(info >> 4) + 4]; shift = (info & 0x0F) + 8
    for i in 0..28:
        nib = (i even) ? byte[1 + i/2] >> 4 : byte[1 + i/2] & 0xF      // high nibble first
        s = ((nib << 28) >> shift)                                      // arithmetic shifts on i32
        s = (s + c1*h1 + c2*h2) >> 8
        s = clamp(s, -32768, 32767)
        output s; h2 = h1; h1 = s
```

The PCM frame sets `h1`/`h2` from its two history words and outputs its 28 samples unchanged.

### Frame, revision 1 (older streams; mono) 

Same as revision 2, except there is no PCM frame, and the sum gets `+ 128` before `>> 8`. Streams of this revision
(version 0 or 1 on non-generic platforms) have 4 bytes of per-channel ADPCM history at the start of each block's
channel data, which are skipped.

### Frame, stereo flavour (codec2 `0x03`) 

30 bytes shared by both channels: `info_l_r` (u8: left coefficient index in the high nibble, right in the low
nibble), `shift_l_r` (u8: same nibble scheme, `+ 8`), then 28 bytes, one per sample: left in the high nibble, right
in the low nibble. Arithmetic as revision 1 (with `+ 128`). Each channel has its own history. Not present in
the MW data **[community; not verified on MW]**.

### Sound banks (no blocks)

A bank sound has no `SCDl`. Channel `c` starts at `bnk_start + offset_tag[c]` and consists of
`ceil(sample_count / 28)` frames back to back (15 bytes, or 61 for `0xEE` frames in revision 2). Mono per
channel; stereo sounds have two offsets.

## 5. EA-XAS version 0 (`.gin`)

Mono, 0x13 bytes per frame, 32 samples per frame, no block structure:

```
u32 header (LE)
    coef_index = header & 0xF                // index into XA_K (below)
    hist2 = (header & 0xFFF0) as i16          // low half, low nibble masked
    hist1 = ((header >> 16) & 0xFFF0) as i16
    shift = (header >> 16) & 0xF
u8 nibbles[15]                                // 30 samples, high nibble first
output hist2, hist1, then for each nibble n:
    d = ((n << 12) as i16) >> shift           // sign-extended 4-bit value scaled by 16 - shift
    s = clamp16(d + hist1*K0[coef_index] + hist2*K1[coef_index])  // float math, truncated to int
    hist2 = hist1; hist1 = s
```

`K0 = [0.0, 0.9375, 1.796875, 1.53125]`, `K1 = [0.0, 0.0, -0.8125, -0.859375]` (the CD-XA filter pairs;
indexes above 3 select `0.0, 0.0`). The frame count is `ceil(samples / 32)`; a `.gin` file has
`Gnsu` header `0x20 + 4*(segCount+1) + 4*(cycleCount+1)` bytes long ([formats/audio.md](../formats/audio.md))
and the frames follow. Each frame is independent (the header carries the history), so frames can be decoded
in any order.

## 6. EA MicroTalk 10:1 (UTK)

CELP/RELP speech codec; every channel has its own decoder state. A frame yields 432 samples (4 sub-frames of
108). The bit reader is LSB-first: `bits` is a u32 accumulator, `count` the number of valid bits.

**Bit reader.** `init`: if `count == 0`, load one byte, `count = 8`. `peek(n)` returns `bits & ((1<<n)-1)` for n in
1..8. `read(n)`: take the low n bits, shift them out, `count -= n`; if `count < 8`, OR the next byte in at
position `count` and add 8. So the reader always holds at least 8 bits after a read and reads one byte ahead.
A missing byte reads as 0.

**PCM-block revision frame.** The frame starts with one byte `p` read with plain byte reads. `p == 0xEE`
means a PCM patch follows. Then the main frame is decoded from the bit reader (which starts reading at the
byte after `p`). Afterwards the reader's one-byte lookahead is given back (position `-= 1`) and the bit
accumulator cleared. If `p == 0xEE`: read `offset = s16be`, `count = s16be`; `offset` must be in 0..=432 and
`count` in 0..=432-offset, else the stream is corrupt; then `count` samples `s16be` overwrite
`samples[offset..offset+count]` (as floats, after synthesis). Older revision: no `p` byte, frames are
contiguous in the bit stream.

**Block change.** At the start of every `SCDl` block the channel data begins with one flag byte (1 on the first
block, 0 after), which is skipped; the bit reader is reset (`count = 0`) but the synthesis state is kept.
Decode `ceil(sample_count / 432)` frames and truncate to `sample_count`.

**Header** (parsed from the bit reader at the first frame after (re)initialisation, 15 bits):
`reduced_bandwidth = read(1)`, `thre = read(4)`, `gain = read(4)`, `mult = read(6)`.
`multipulse_threshold = 32 - thre`; `fixed_gain[0] = 8 * (1 + gain)`; `fixed_gain[i] = fixed_gain[i-1] *
(1.04 + mult * 0.001)` for i in 1..64.

**Frame:**

1. Reflection coefficients: `idx0 = read(6)`; `use_multipulse = idx0 < multipulse_threshold`. `idx1..3 = read(6)`.
   `idx4..11 = 16 + read(5)`. `rc_target[i] = RC[idx_i]`; `rc_delta[i] = (rc_target[i] - rc[i]) * 0.25`.
   `RC` is the 64-entry table of reflection coefficients in `tables.rs` (mirrored: `RC[64-i] = -RC[i]`).
2. Four sub-frames `i = 0..4`: `pitch_lag = read(8)`, `pitch_value = read(4)`, `gain_index = read(6)`;
   `pitch_gain = pitch_value / 15`; `fixed = fixed_gain[gain_index]`.
   - Not reduced bandwidth: decode 108 excitation values (below) into `ex[0..108]`.
   - Reduced bandwidth: `align = read(1)`, `zero_flag = read(1)`; decode 54 values into
     `ex[align], ex[align+2], ...`. If `zero_flag`, set the other 54 positions to 0. Else zero five values
     either side of the 108 (padding for the filter), interpolate the missing positions
     `ex[j] = 0.01803268*(ex[j-5]+ex[j+5]) - 0.11459156*(ex[j-3]+ex[j+3]) + 0.59738597*(ex[j-1]+ex[j+1])`
     and halve `fixed`.
   - `samples[108*i + j] = fixed * ex[j] + pitch_gain * adapt[108*i + 216 - pitch_lag + j]` for j in 0..108,
     where `adapt` is the 324-float adaptive codebook directly followed in memory by `samples` (index below 0
     clamps to 0).
3. `adapt[0..324] = samples[108..432]` (the last 324 samples).
4. For `i` in 0..4: `rc[j] += rc_delta[j]`, then run the 12th-order LPC synthesis filter over
   `samples[12*i ..]` for 1 block of 12 samples (`i < 3`) or 33 blocks (`i == 3`) = 396 samples. In total
   12 + 12 + 12 + 396 = 432.

**Excitation, multi-pulse** (`use_multipulse`): `model = 0`, `i = 0`; while `i < 108`: `code = peek(8)`;
`cmd = CODEBOOK[model][code]`; `model = CMD[cmd].next_model`; `read(CMD[cmd].code_size)`; then
- `cmd > 3`: `ex[i] = CMD[cmd].pulse; i += stride`
- `cmd in 2..=3`: `n = 7 + read(6)`; write `n` zeros (clipped to the end), `i += n * stride`
- `cmd in 0..=1`: `x = 7; while read(1): x += 1; if !read(1): x = -x`; `ex[i] = x; i += stride`

**Excitation, RELP** (otherwise): per value `code = peek(2)`; code 0 or 2: value 0, consume 1 bit; code 1:
value -2, consume 2 bits; code 3: value 2, consume 2 bits.

**`stride`** is 1 for full bandwidth and 2 for reduced bandwidth (positions start at `5 + align` in a buffer with
5 floats of padding at each end).

**LPC synthesis filter.** Convert `rc[0..12]` to 12 LPC coefficients with the step-up recursion used by the
original (the Rust code keeps the recursion as written in `synth.rs`), then for each block of 12 samples
and each `j` in 0..12: `x = sample + sum_k lpc[k] * hist[...]` where the history is a 12-float shift register,
`hist[11 - j] = x`, `sample = x`. The state `rc`, `synth_history`, `adapt`, `samples` persist from frame to
frame, and are zero after (re)initialisation.

**Output.** Each float sample is rounded half away from zero and clamped to i16.

**Tables.** `RC[64]`, the two 256-entry `CODEBOOK` tables and the 29-entry `CMD` table (next model, code
size, pulse value) are constants ported from the reference (see provenance).

## 7. `ABKC` bank

```
0x00 'ABKC'    0x04 01 01 01 00    0x0A u16 num_modules
0x1C u32 module table offset       0x20 u32 offset of the embedded 'BNKl' (0 = none)
module (at the table offset; the next module follows at 0x3C + 4*(players + class controllers)):
    +0x24 u8 players     +0x27 u8 class controllers     +0x2C u32 module data offset
    +0x3C u32 player_offset[players]
player at module_data + player_offset:  +0x04 u32 offset of a sample table
sample table:  u32 count, then 12-byte entries:
    u8 type (0 RAM, 1 streamed, 2 streamed+loop), u8 priority, u16 pad, u32 index/offset, u32 loop offset
```

A RAM entry's `index` selects an entry of the bank's `BNKl`. Entries of type 0 with index 0 are dummies.
Several players may share a sample table; read each distinct table once. Streamed types refer to a companion
`.ast` file that MW does not use (all 3,678 entries in the install are type 0).

`BNKl` (version 5): `'BNKl'`, u8 version at 4, u16 `num_sounds` at 6, entries from `0x14`: `u32 rel` per sound,
relative to the entry's own position; 0 marks a dummy. The target is a `PT` header (section 2) whose data
offsets (`0x88`...) are relative to the start of the `BNKl`.

## 8. `.mpf` and `.mus`

```
0x00 'xDFP' (reads as LE 'PFDx')   0x04 u8 version (5)   0x05 u8 sub-version (1 here)
0x0D u8 tracks   0x0E sections   0x0F events   0x10 routers   0x11 vars   0x12 u16 nodes
0x2C u32 tracks table    0x30 u32 tracks data    0x34 u32 samples table    0x38 u32 end of samples table
tracks table:  u32 entry offset / 4 per track
track entry:   +0x00 u32 first sample index, +0x04 u16 sub-bank count (0 = streamed), +0x08 u32 BE checksum
samples table: 8 bytes each: u32 offset (in 0x80 units; high 16 = bank index, low 16 = sound index for RAM
               tracks), u32 duration in ms
```

Stream `k` starts at `.mus` offset `entry.offset * 0x80`, with a `SCHl` there (GSTR, big-endian values).
`end of samples table - samples table` divided by 8 is the number of streams. A track covers sample indexes from its
first sample index up to the next track's. RAM tracks (sub-bank count nonzero) are not used by MW.

## 9. Output

PCM `i16`, interleaved, per-stream sample rate and channels. Loop points are exposed as `(start, end)` in
samples of the decoded stream (`end` exclusive). The library does not loop; the player does.
