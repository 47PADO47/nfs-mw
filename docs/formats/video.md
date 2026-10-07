# Video (`MOVIES/*.vp6`)

The FMVs use EA's VP6 movie container: a flat run of tagged blocks that interleaves On2 **VP6**
video with an EA `SCHl` audio stream. It is **not** bChunk data. For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

## Files **[verified]**

32 files named `<movie>_english_ntsc.vp6`: `attract_movie`, `ealogo`, `psa`, `blacklist_01`…`_15`, five
`*_tutorial`, and nine `storyfmv_*`. Subtitles are separate files in `SUBTITLES/<movie>` (see
[text.md](text.md#subtitles-decomp--verified)).

## Block stream **[verified on all 32 files]**

Every block is `char[4] tag, u32 size (LE, includes the 8-byte header), payload`. Each file walks
cleanly to its end with exactly these tags:

| Tag | What |
|---|---|
| `MVhd` | video header (first block) |
| `SCHl` | audio stream header (see [audio.md](audio.md#schl-stream-layout-community-tags-verified)) |
| `SCCl` | audio block count |
| `MV0K` | VP6 **key** frame |
| `MV0F` | VP6 inter frame |
| `SCDl` | audio data |
| `SCEl` | audio end |

### `MVhd` (block size 0x20)

| Offset (from block start) | Type | Field | Value in MW |
|---|---|---|---|
| 0x08 | char[4] | codec FourCC, stored as bytes `30 36 50 56` (`'VP60'` as LE u32) | all 32 |
| 0x0C | u16 | width | 1024 |
| 0x0E | u16 | height | 512 |
| 0x10 | u32 | frame count | e.g. 92 (`ealogo`), 932 (`blacklist_01`) |
| 0x14 | u32 | largest frame size **[unconfirmed]**; FFmpeg skips it | |
| 0x18 | u32 | rate | 982,047 |
| 0x1C | u32 | scale | 32,767 → **29.97 fps** |

FFmpeg reads this the same way: it skips FourCC/width/height and reads frame count, then `den`, then
`num` **[community]**. The first `MV0K` of every movie codes **64 × 32 macroblocks = 1024 × 512**,
matching `MVhd` **[verified]**.

### Audio

`SCHl` with a `GSTR` platform marker, header version 3, 2 channels, no codec tag and no sample-rate
tag **[verified]**. Both FFmpeg (`ADPCM_EA_R3`) and vgmstream (`EA-XA`) decode it, and both default a
missing rate to **48,000 Hz** for this header type **[community]**.

## Decoders and licensing

| Option | Language | License | Notes |
|---|---|---|---|
| [ruffle-rs/nihav-vp6](https://github.com/ruffle-rs/nihav-vp6) | **Rust** | **MIT** (`COPYING`: relicensed by Kostya Shishkov for Ruffle) | VP5/VP6 decoder only. The `MVhd`/`SCHl` demuxer above is small enough to write yourself. Best fit for a Rust rewrite. |
| [FFmpeg](https://github.com/FFmpeg/FFmpeg) `libavformat/electronicarts.c` + `libavcodec/vp6.c` | C | **LGPL-2.1+** (file headers) | Complete demux + decode + EA ADPCM. Dynamic linking keeps LGPL obligations simple; static linking or porting is a copyleft concern. |
| [vgmstream](https://github.com/vgmstream/vgmstream) `ea_schl_video` | C | ISC-style | Audio track only. |

The decomp only has headers for EA's movie player (`src/Speed/Indep/Libs/rcmp/4.00.11/include/rcmp/av/avplayer.h`,
including the `AV_SUBTITLE` classes) **[decomp]**.
