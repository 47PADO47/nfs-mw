# Video (`MOVIES/*.vp6`)

The FMVs use EA's VP6 movie container: a flat run of tagged blocks that interleaves On2 **VP6**
video with an EA `SCHl` audio stream. It is **not** bChunk data. For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

## Files **[verified]**

32 files named `<movie>_english_ntsc.vp6`: `attract_movie`, `ealogo`, `psa`, `blacklist_01`…`_15`, five
`*_tutorial`, and nine `storyfmv_*`. Subtitles are separate files in `SUBTITLES/<movie>` (see
[text.md](text.md#subtitles-decomp--verified)).

## Block stream **[verified on all 32 files]**

Every block is `char[4] tag, u32 size (LE, includes the 8-byte header), payload`. Every block size is a
multiple of 4 (payloads are zero-padded), and each file walks cleanly to its end with exactly these tags
(45,435 video frames: 589 `MV0K` + 44,846 `MV0F`; 45,502 `SCDl` blocks):

| Tag | What |
|---|---|
| `MVhd` | video header (first block) |
| `SCHl` | audio stream header (see [audio.md](audio.md#schl-stream-layout-community-tags-verified)) |
| `SCCl` | audio block count: u32 **big-endian**, equal to the number of `SCDl` blocks in all 32 files |
| `MV0K` | VP6 **key** frame |
| `MV0F` | VP6 inter frame |
| `SCDl` | audio data |
| `SCEl` | audio end (empty payload) |

### Order and interleaving **[verified on all 32 files]**

The blocks always start `MVhd`, `SCHl`, `MV0K`, `SCCl`; every file begins with a key frame. After that
the stream is mostly `video, audio, video, audio, …`: one `SCDl` (about 1,600 samples, one frame time)
follows each frame. Two things break the strict alternation, so a demuxer must not rely on it:

- When the audio runs longer than the video it ends with a run of `SCDl` blocks (23 in `ealogo`, 43 in
  `storyfmv_bla134`) and then `SCEl`. When the audio is shorter, `SCEl` sits before the last frames: only video
  frames follow it, in 5 files (`attract_movie` 12, `bounty_tutorial` 3, `psa` 16, `storyfmv_rac01` 27,
  `storyfmv_raz08` 13).
- A few places have two `SCDl` in a row (up to 2 in the `blacklist` files).

The presentation index of a video frame is its ordinal among `MV0K`/`MV0F` blocks, starting at 0: the
header frame count equals `MV0K + MV0F` in all 32 files. There is no timestamp in the stream. Key frames
come every 120 frames in most files (every 62 or 63 in `blacklist_*`, every 108 in `storyfmv_pin11` and
`storyfmv_rac01`), plus extra ones at scene cuts (`ealogo` has only the first, `storyfmv_cro06_coh06a` has 65).

### `MVhd` (block size 0x20)

| Offset (from block start) | Type | Field | Value in MW |
|---|---|---|---|
| 0x08 | char[4] | codec FourCC, stored as bytes `30 36 50 56` (`'VP60'` as LE u32) | all 32 |
| 0x0C | u16 | width | 1024 |
| 0x0E | u16 | height | 512 |
| 0x10 | u32 | frame count | e.g. 92 (`ealogo`), 932 (`blacklist_01`), 5,655 (`storyfmv_cro06_coh06a`) |
| 0x14 | u32 | largest frame payload in bytes: equals the biggest `MV0K`/`MV0F` payload, or is 2 below it (17 files exact, 15 files off by 2, from the padding) | 10,784 to 91,270 |
| 0x18 | u32 | rate | 982,047 |
| 0x1C | u32 | scale | 32,767 → **29.97 fps** |

fps is `rate / scale` = 29.97003. FFmpeg reads this the same way: it skips FourCC/width/height and reads
frame count, then `den`, then `num` **[community]**. The first `MV0K` of every movie codes **64 × 32
macroblocks = 1024 × 512**, matching `MVhd` **[verified]**.

### Video frames **[verified; bit layout from nihav-vp6]**

The payload of `MV0K`/`MV0F` is one raw VP6 frame, with no extra prefix (Flash's one-byte crop prefix is
not present). The first bit of byte 0 is clear for key frames (`MV0K`) and set for inter frames (`MV0F`)
in all 45,435 frames. Every `MV0K` has 6-bit quantiser, multistream flag set, version field **7**
(VP6.1; nihav numbers VP6.0 as 6), profile 0 (simple), progressive; a 16-bit offset to the second
partition follows, then macroblock rows and columns (32, 64 in the first frame) and the display size in macroblocks. There
is no alpha plane. Decoding needs the previous frame and the golden (last key) frame, so playback
starts at a key frame.

Decoded with nihav-vp6, **[verified on all 45,435 frames]**: every frame decodes to 1024 × 512 YUV 4:2:0.
The rows come out **bottom to top** (plain VP6 as opposed to Flash's VP6F; FFmpeg flips it too
**[community]**), so a player reverses them; seen on `blacklist_01` frame 200, which shows the Razor
portrait upright after the flip. Black is luma 16, so the picture is studio range (BT.601 coefficients
for RGB). Speed on one core of the dev machine, release build: 164 frames/s decode alone and 116
frames/s with the RGBA conversion on `blacklist_01` (a high-bitrate movie), against 29.97 needed.

Shown on screen: the 1024 × 512 picture carries a black letterbox (the film is about 2.4:1 inside it): over the
first 500 frames of all 32 movies the picture is only ever non-black in rows 64 up to 448, **except `ealogo`**, whose
canvas is all picture **[measured]**. The widescreen FEng packages put a movie in a 900 × 480 object (so the original
shows the baked bars). The Rust player crops rows 64..448 of every movie but the logo and stretches them over the
whole window, so no band is left at any window shape (at 16:9 the film is then about 40 % taller than at its own
shape); the logo keeps 900:480 with black around it, which cannot be seen on its black canvas. The display aspect is
not stored anywhere that has been found.

### Audio

`SCHl` has a 32-byte payload, identical in structure in all 32 files **[verified]**:

```
"GSTR" 01 00 00 00        platform marker + 4 bytes
00 04 1D F8 51 E0         tag 0x00, len 4 (same value in all 32 files, meaning unknown)
06 01 65                  tag 0x06 priority = 0x65
FD                        marker
80 01 03                  tag 0x80 header version = 3
85 03 xx xx xx            tag 0x85 sample count (24-bit big-endian, per file)
82 01 02                  tag 0x82 channels = 2
FF                        end, then 2 zero bytes of padding
```

No codec tag (`0xA0`) and no sample-rate tag (`0x84`). Both FFmpeg (`ADPCM_EA_R3`) and vgmstream
(`EA-XA`) decode it, and both default a missing rate to **48,000 Hz** for this header type
**[community]**. That rate fits the movies: `0x85` divided by the frame count is 1,600.9 samples per frame
in `blacklist_01` (48,000 / 29.97 = 1,601.6).

`SCDl` payload: a u32 **big-endian** sample count per block (the top byte is 0 in all 45,502 blocks),
followed by the EA-XA stereo data. **The sum of the per-block sample counts equals the `0x85` tag of
`SCHl` in all 32 files.** Splitting the block into per-channel streams belongs to the audio decoder
(`libs/ea-audio`), not the movie demuxer.

## Decoders and licensing

| Option | Language | License | Notes |
|---|---|---|---|
| [ruffle-rs/nihav-vp6](https://github.com/ruffle-rs/nihav-vp6) | **Rust** | **MIT** (`COPYING`: relicensed by Kostya Shishkov for Ruffle) | VP5/VP6 decoder only. **Used** as a pinned git dependency of [`blackbox-movie`](../../libs/blackbox-movie); the demuxer is our own. |
| [FFmpeg](https://github.com/FFmpeg/FFmpeg) `libavformat/electronicarts.c` + `libavcodec/vp6.c` | C | **LGPL-2.1+** (file headers) | Complete demux + decode + EA ADPCM. Read for facts only (field order of `MVhd`, default audio rate); no code taken. |
| [vgmstream](https://github.com/vgmstream/vgmstream) `ea_schl_video` | C | ISC-style | Audio track only. |

Sources and how the layout above was checked: [provenance/video.md](../provenance/video.md).

The decomp only has headers for EA's movie player (`src/Speed/Indep/Libs/rcmp/4.00.11/include/rcmp/av/avplayer.h`,
including the `AV_SUBTITLE` classes) **[decomp]**.
