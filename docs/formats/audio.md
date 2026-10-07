# Audio (`SOUND/`)

MW's audio is **not** bChunk data. It uses EA's own audio formats: `SCHl` streams, `ABKC`/`BNKl`
sound banks, `Gnsu` granular engine sounds and a `PFDx` interactive-music map. Decoding is solved by
[vgmstream](https://github.com/vgmstream/vgmstream). The runtime side (which sound plays when, mixing,
reverb) is only in the [decomp](https://github.com/dbalatoni13/nfsmw). For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

## Folder map **[verified]**

| Folder | Files | Magic | What |
|---|---|---|---|
| `ENGINE/` | 232 `.abk` (`CAR_xx_ENG_MB_EE` / `_SPU` pairs), 160 `.gin` | `ABKC`, `Gnsu` | Engine banks and granular engine loops |
| `FE/`, `GLOBAL/` | `FE_MB.abk` (3.1 MB), `FE_COMMON_MB.abk` | `ABKC` | Front-end sounds |
| `IG_GLOBAL/` | 13 `.abk` | `ABKC` | In-game: sirens, traffic, wind, road noise, collisions (`Stich_Collision_MB`), helicopter, rain |
| `NOS/`, `SHIFTING/`, `SKIDS/`, `TURBO/` | 2 / 16 / 4 / 32 `.abk` | `ABKC` | Per-feature banks |
| `PFDATA/` | `MW_Music.mpf` (136 KB) + `MW_Music.mus` (533,877,888 B) | `xDFP` / `FA CE A5 8C` | Interactive music (PathFinder) |
| `SPEECH/` | `copspeech.big` (192,753,408 B), `.idx`, `.evt`, `.csi` | `SCHl`, —, `03 12 3C 07`, `MOIR` | Police radio / cop speech |
| `STREAMS/` | `NISAudio.big` (184,221,440 B), `.idx`, `.evt`, `.csi` | same as `SPEECH/` | Cut-scene (NIS) audio |
| `EVT_SYS/` | 7 `.csi` (64–348 B) | `MOIR` | Event-system descriptors (`MAIN_AEMS`, `ENGINES_AEMS2`, `ENVIRO_AEMS`, …) |
| `FXEDIT/` | 15 `.fx` (160–336 B) | `FX` `0B 00` | Reverb presets (`City_Dense`, `Alley`, `Garage`, `Simple_Tunnel`, …) |
| `MIXMAPS/` | 4 `.mxb` (9,996 B) + 4 `.dyn` (43 KB) | `00 00 00 00 0D 00 00 00 10 00 00 00 FF FF FF FF` | Dynamic-mixer maps |
| `NISREVDATA/` | 29 `.bin` | none | Engine-RPM curves for cut-scenes |

Also in bChunk files: `GLOBAL/InGameB.bun` holds 3 × `8003B500 SndStichBundle`, with 720
`0003B502 SndStichData` + 720 `0003B503 SndSampleRef` chunks **[verified]**. The decomp names them
`SND_STICHBUNDLE` / `SND_STICHDATA` / `SND_SAMPLEREF` **[decomp]**. Their layout is undocumented.

## Codecs actually used on PC **[verified]**

I parsed every stream header in the install (header tag meanings from vgmstream):

| Container | Streams | Header | Codec | Rate | Channels |
|---|---|---|---|---|---|
| `.abk` bank sounds | 2,577 | `PT\0\0`, platform 0 (PC), version 2, no codec tag | **EA-XA** (vgmstream's PC default) | 12,000–44,100 Hz (32,000 ×960, 36,000 ×214, 44,100 ×100, plus three odd 400 Hz entries). 1,204 entries have no rate tag (ENGINE 1,048, TURBO 148, NOS 6, IG_GLOBAL 2); vgmstream then assumes 22,050 **[community]** | mono, except 40 stereo (FE 38, GLOBAL 2) |
| `.gin` | 160 | `Gnsu` | **EA-XAS v0** (vgmstream `gin.c`) | 32,000 ×103, 24,000 ×41, 36,000 ×14, 34,000, 20,000 | mono |
| `MW_Music.mus` | 3,257 `SCHl` | `GSTR` (generic platform, big-endian header values), version 3 | **EA-XA** | 36,000 Hz | 3,243 stereo, 14 four-channel |
| `copspeech.big` | 13,562 `SCHl` | PC, version 2, **codec2 tag `0xA0` = 4** | **EA MicroTalk 10:1** (`EA_CODEC2_MT10`) | 24,000 Hz | 13,385 mono, 177 stereo |
| `NISAudio.big` | 142 `SCHl` | PC, version 2, no codec tag | **EA-XA** | 44,100 Hz | 80 × 6-ch, 33 stereo, 28 mono, 1 × 4-ch |
| `MOVIES/*.vp6` | see [video.md](video.md) | `GSTR`, version 3 | EA-XA | — | stereo |

No EALayer3, EA-MP3, `SNR`/`SNS` or other newer EA formats occur. `.abk` banks use the mono and stereo
EA-XA flavours, picked by platform/version defaults.

## `SCHl` stream layout **[community; tags verified]**

A stream is a run of blocks, each `char[4] tag, u32 size (LE, header included), payload`:
`SCHl` (header) → `SCCl` (block count) → `SCDl` ×N (audio data) → `SCEl` (end).

The `SCHl` payload starts with a platform marker, either `"PT"` + u16 platform (0 = PC) or `"GSTR"` +
4 extra bytes. After that comes a tag stream: `u8 tag`, `u8 len`, then `len` bytes as a **big-endian**
value. `0xFC`/`0xFD`/`0xFE` are bare markers, and `0xFF` ends the header.

| Tag | Meaning (vgmstream `ea_schl.c`) |
|---|---|
| `0x06` | priority |
| `0x80` | header version (2 or 3 in MW) |
| `0x82` | channel count (default 1) |
| `0x84` | sample rate (default per platform) |
| `0x85` | sample count |
| `0x86` / `0x87` | loop start / end sample |
| `0xA0` | codec2 (absent = platform default; 4 = MicroTalk 10:1) |
| `0x8A`, `0x8C`, … | padding / flags |

## `ABKC` + `BNKl` sound banks **[community; offsets verified]**

| Offset | Field | Notes |
|---|---|---|
| 0x00 | `ABKC` | |
| 0x04 | `01 01 01 00` | the same in all 301 files |
| 0x0A | u16 module count | |
| 0x1C | u32 module-table offset | modules → players → sample tables (vgmstream `ea_schl_abk.c`) |
| 0x20 | u32 offset of the embedded `BNKl` | lands on `BNKl` in all 301 files |

`BNKl`: 0x04 u8 version (**5** in every MW bank), 0x06 u16 sound count, then at 0x14 a table of u32
offsets, **each relative to its own table slot**, pointing at `PT` headers. Entry value 0 marks a
dummy entry. Totals: 2,878 entries = 301 dummies (always entry 0) + 2,577 sounds. **[verified]**

The bank also stores sample tables with a type per sound (RAM / streamed / streamed-looped), the
module/player tables, and the AEMS ("audio event") logic that picks sounds at runtime. vgmstream walks
these tables only far enough to list the sounds. The runtime is EA's SND9 library, partly decompiled
in `src/Speed/Indep/Libs/snd/9/` (`saems.c`, `sbanki.h`, …) **[decomp]**. The `_EE` / `_SPU` file
pairs probably match the bank memory types in the decomp's `eSNDDATATYPE`
(`SDT_AEMS_MAINMEM`, `SDT_AEMS_SYNCSPU`, …), i.e. PS2 EE vs SPU2 RAM. **[unconfirmed]**

## `Gnsu` granular engine sounds (`.gin`) **[verified on all 160 files]**

| Offset | Type | Field (names from the decomp's `GinsuSynthData`) |
|---|---|---|
| 0x00 | char[4] | `Gnsu` |
| 0x04 | char[4] | `"20\0\0"` (version; the loader checks `ver[0] == '2'`) **[decomp]** |
| 0x08 | f32 | min frequency |
| 0x0C | f32 | max frequency |
| 0x10 | u32 | segCount |
| 0x14 | u32 | cycleCount |
| 0x18 | u32 | sampleCount |
| 0x1C | u32 | sampleRate |
| 0x20 | u32[segCount+1] | frequency → sample positions |
| … | u32[cycleCount+1] | cycle → sample positions |
| … | | EA-XAS v0 data, mono: 0x13 bytes per 32 samples |

Check: in all 160 files the data size equals `ceil(sampleCount/32) × 0x13`. The synthesis (picking and
cross-fading grains by target frequency) is in the decomp's `EAXSound/Ginsu/ginsusynth.cpp`
**[decomp]**. File names: `GIN_<CAR>_<variant>.gin`, e.g. `GIN_240SX_Decel.gin`, `GIN_300ZX_DCL.gin`.

## Interactive music: `MW_Music.mpf` + `.mus` **[community; checked against the files]**

| Offset (`.mpf`) | Field | Value here |
|---|---|---|
| 0x00 | `xDFP` (`PFDx` read as LE) | |
| 0x04 / 0x05 | u8 version / sub-version | **5 / 1** (vgmstream: "Need for Speed: Most Wanted, Carbon, SSX on Tour") |
| 0x2C | u32 tracks table | 0x1ADF4 |
| 0x30 | u32 tracks data | 0x1ADF8 |
| 0x34 | u32 samples table (8 B per stream: offset/0x80 or bank index, duration in ms) | 0x1AE0C |
| 0x38 | u32 end of samples table | 0x213D4 → (0x213D4 − 0x1AE0C) / 8 = **3,257 streams**, the same as the `SCHl` count in `.mus` **[verified]** |

The track entry stores a big-endian checksum at +0x08. Its value `FA CE A5 8C` (at `.mpf` 0x1AE00) is
also the first 4 bytes of `MW_Music.mus` **[verified]**. In `.mus`, streams start at 0x100 and sample
offsets are multiplied by 0x80. The node graph (which segment follows which, driven by pursuit
intensity) belongs to EA's PathFinder 5.01.04, decompiled in `src/Speed/Indep/Libs/path/5.01.04/` and
driven by `EAXSound/sfxctl/SFXCTL_Pathfinder5.cpp` **[decomp]**. vgmstream only extracts the samples.
The AttribSys class `music` (5 fields, 27 collections) lists tracks ([attributes.md](attributes.md)).

## Speech and NIS streams (`.big` / `.idx` / `.evt` / `.csi`)

- `.big` is a plain concatenation of `SCHl` streams (counts above) **[verified]**.
- `.idx` starts `u32 1, u32 count`: 0x243 = 579 for copspeech, 4 for NISAudio. `.evt` starts
  `03 12 3C 07`. `.csi` starts `MOIR 00 02 00 01`. **Layouts undocumented. [verified bytes only]**
- Decomp: the speech system is `src/Speed/Indep/Src/Speech/` (`SoundAI`, `PursuitFlow`,
  `RoadblockFlow`, `SpeechCache`, …). It sits on EA's SPCH library (`Libs/spch/dev/include/spch/spch.h`,
  header only, version comment 3.20.5), with generated tables in `EAXSound/SND_GEN/COPSPEECH.cpp` and
  `NISAudio.cpp`. AttribSys classes `speech` (28 fields, 133 collections) and `speechtune`. **[decomp]**

## `EVT_SYS/*.csi` (`MOIR`)

Header `MOIR 00 02 00 02`, u32 0, then a small u32: 2, 6, 8, 4, 13, 3 and 1 for `COP_SIREN_AEMS`,
`ENGINES_AEMS2`, `ENVIRO_AEMS`, `FE_AEMS`, `MAIN_AEMS`, `STITCH_AEMS` and `TURBO`, probably an entry count
**[verified bytes; meaning unconfirmed]**. The runtime is EA's CSIS library (`Libs/csis`, header only)
plus generated tables in `EAXSound/SND_GEN/*_AEMS.cpp` **[decomp]**. vgmstream's ABK code notes that EA
ships `.abk` with `.csi` ("MOIR") files but does not parse them.

## `FXEDIT/*.fx` reverb presets

Magic `FX` `0B 00`, which is the same value as the `0x000B5846 "FX"` entry in the decomp's chunk list.
The game loads them by name from a table in `EAXSound/CARSFX/SFXObj_Reverb.cpp` (path
`SNDPATH_FXEDIT`); the parameter struct is in `EAXSound/Data/SND_REVERBFXPARAMS.hpp` **[decomp]**. The
byte layout is undocumented.

## `MIXMAPS/`

`MAPOUTPUT{,DRG,2CR,2DR}.mxb` are loaded from hard-coded paths in
`EAXSound/Dynamic_Mixer/NFSMixMap.cpp` (76 KB of decompiled code) **[decomp]**. The `.dyn` twins share
the header and contain strings such as `------MapTitle----`. They may be an authoring version and are
not referenced in the decomp. **[unconfirmed]**

## `NISREVDATA/*.bin` **[decomp + verified]**

The loader (`EAXSound/sfxctl/SFXCTL_NISReving.cpp`, `NIS_RevManager::OpenNISRevData`) opens
`sound\NISRevData\<anim name>.bin` and reads **16 data sets**:

```
repeat 16:  u32 NumPoints;  NumPoints × { f32 time; i32 RPM; i32 Trq }   // 12-byte EngRevDataPoint
```

All 29 files parse. 9 end exactly after set 16; the other 20 have trailing bytes that the loader
ignores. Most files use only set 0 (e.g. `GenericStart.bin`: 187 points); `OPMRivalIntro.bin` uses sets
0–3.

## References

| Source | What | License |
|---|---|---|
| [vgmstream](https://github.com/vgmstream/vgmstream) `src/meta/ea_schl.c`, `ea_schl_abk.c`, `ea_schl_map_mpf_mus.c`, `gin.c`; `src/coding/ea_xa_decoder.c`, `ea_xas_decoder.c`, `ea_mt_decoder.c` + `libs/utkdec.c` | Reference decoders for every container and codec above | ISC-style permissive (`COPYING`) |
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) `src/Speed/Indep/Src/EAXSound/`, `Src/Speech/`, `Libs/snd/9`, `Libs/path/5.01.04`, `Libs/csis`, `Libs/spch` | Runtime: AEMS, Ginsu synthesis, PathFinder, mixer, reverb, NIS revs, speech | CC0-1.0 |
| [FFmpeg](https://github.com/FFmpeg/FFmpeg) `libavformat/electronicarts.c` | `SCHl` parsing for movies (see [video.md](video.md)) | LGPL-2.1+ |
| [xan1242/XNFSMusicPlayer](https://github.com/xan1242/XNFSMusicPlayer) | Replacement music player (installed here as an ASI). Does not document MPF | MIT |
| [TsyVM/MWEncyclopedia](https://github.com/TsyVM/MWEncyclopedia) | Leads only: no license, and has verified errors (e.g. no mention of MicroTalk) | none |
