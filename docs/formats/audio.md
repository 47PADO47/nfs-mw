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
`SND_STICHBUNDLE` / `SND_STICHDATA` / `SND_SAMPLEREF` **[decomp]**. Layout: [Sound stitches](#sound-stitches).

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

Decoding results **[verified with `libs/ea-audio`]**: every bank sound, music stream, cut-scene stream, cop-speech
stream and `.gin` file decodes to its header's sample count. EA-XA output equals FFmpeg's `adpcm_ea_r3` (music) and
`adpcm_ea_r2` (cut scenes) sample for sample. EA-XA frame headers use coefficient indexes 0 to 3 only, and 61-byte PCM
frames (`0xEE`) are rare (about 0.02 % of cut-scene frames). Cop speech is mastered at full scale: 99 % of the streams
touch +/-32768, in runs of at most 8 samples. In a bank, each channel's data is followed by 4 zero bytes and
every sound starts at an offset that is 4 modulo 16 from the `BNKl`.

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

Per-file tag sets in the install **[verified]**: `copspeech.big` 32-byte headers with tags `06 80 84 85 A0`
(+ `82` for the 177 stereo streams); `NISAudio.big` `05 06 80 84 85` (+ `82` for 114 multichannel streams);
`MW_Music.mus` `06 80 82 84 85` (GSTR, version 3, 36,000 Hz). Bank sounds: `06 13 80 85 88 8A` (+ `82 89`
for stereo, `84` rate, `86 87 1A` loop). In every stream the sum of the `SCDl` sample counts equals tag `0x85`.

`SCCl` holds the number of `SCDl` blocks (`GSTR`: big-endian; PC: little-endian) **[verified]**.
`SCDl` payload: `u32 samples, u32 channel_offset[ch], data` with channel data at `block + 0x0C + 4*ch +
offset`; values are big-endian in `GSTR` streams and little-endian in `PT` streams, the block size is always
little-endian. `MW_Music.mus` example: `SCDl` with 7,224 samples, offsets `0, 0xFA8`, EA-XA frames (15 bytes, or
a 61-byte PCM frame starting `0xEE`). MicroTalk channel data starts with one flag byte (1 in the first block, 0
after) **[verified]**. The decoding rules are in [specs/audio-containers.md](../specs/audio-containers.md).

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

### Module, player and sample tables **[community; verified on all 301 banks]**

```
0x1C  module table; module = 0x3C bytes + 4*(players + class controllers) of u32 player offsets
      +0x24 u8 players, +0x27 u8 class controllers, +0x2C u32 module data offset, +0x3C u32 player_offset[]
player (module data + player_offset): +0x04 u32 offset of its sample table
sample table: u32 count, then 12-byte entries { u8 type, u8 priority, u16 0, u32 index, u32 loop offset }
```

In all 301 banks every entry has type 0 (RAM) and `index` selects a `BNKl` entry; entries whose type and index are
both 0 are dummies. Across a bank, the non-dummy entries of the distinct sample tables (306 tables, 1-3 modules
and 1-13 players per module; several players share a table) reference every non-dummy `BNKl` entry exactly once,
so the tables only group sounds, they do not add any. The total is 2,577 sounds. Types 1 and 2 (streamed from a
companion `.ast`) do not occur. A bank sound's `PT` header offsets (`0x88`, `0x89`) are relative to the start of
the `BNKl`, and the data is mono EA-XA per channel (stereo sounds have two offsets).

The AEMS ("audio event") logic that picks sounds at runtime is not in these tables. The runtime is EA's SND9
library, partly decompiled in `src/Speed/Indep/Libs/snd/9/` (`saems.c`, `sbanki.h`, …) **[decomp]**. The `_EE` /
`_SPU` file pairs probably match the bank memory types in the decomp's `eSNDDATATYPE`
(`SDT_AEMS_MAINMEM`, `SDT_AEMS_SYNCSPU`, …), i.e. PS2 EE vs SPU2 RAM. **[unconfirmed]**

## `Gnsu` granular engine sounds (`.gin`) **[verified on all 160 files]**

| Offset | Type | Field (names from the decomp's `GinsuSynthData`) |
|---|---|---|
| 0x00 | char[4] | `Gnsu` |
| 0x04 | char[2] | `"20"`: version 2 (the loader checks `ver[0] == '2'`) **[decomp]** |
| 0x06 | u16 | flags, 0 in every PC file; the console loader sets it to 1 after byte-swapping the tables **[decomp]** |
| 0x08 | f32 | `min_frequency` |
| 0x0C | f32 | `max_frequency` |
| 0x10 | u32 | `seg_count` (50 in all 160 files) |
| 0x14 | u32 | `cycle_count` (18 to 265) |
| 0x18 | u32 | `sample_count` (9,932 to 223,531) |
| 0x1C | u32 | `sample_rate` (32,000 x103, 24,000 x41, 36,000 x14, 34,000, 20,000) |
| 0x20 | u32[seg_count + 1] | `freq_pos`: the sample at which the recording's pitch equals `min + (max - min) * i / seg_count` |
| … | u32[cycle_count + 1] | `cycle_pos`: the first sample of each pitch cycle; the last entry is the last sample |
| … | | EA-XAS v0 data, mono: 0x13 bytes per 32 samples |

Checks, all 160 files **[verified]**: the data size equals `ceil(sample_count / 32) x 0x13`; `cycle_pos[0] = 0`,
`cycle_pos[cycle_count] = sample_count - 1` and `cycle_pos` is strictly increasing; every `freq_pos` entry is
below `sample_count`; `freq_pos` is non-decreasing in the 85 accelerate files (`GIN_<CAR>*.gin`: a recording of
a rev up) and non-increasing in the 75 decelerate files (`..._DCL.gin`, `GIN_240SX_Decel.gin`, ...: a rev
down); `min_frequency` is 1,031 to 6,065 and `max_frequency` 2,011 to 9,241. The frequency unit is chosen so
that a cycle is `sample_rate * 120 / frequency` samples long (median error 3.3 % over the 51 knots of a file).
The EA-XAS blocks are independent (each carries its two history samples), so any sample can be decoded alone.

The synthesis (picking and cross-fading whole pitch cycles by target frequency) is specified in
[specs/engine-sound-ginsu.md](../specs/engine-sound-ginsu.md) from the decomp's `EAXSound/Ginsu/ginsusynth.cpp`
**[decomp]**. File names: `GIN_<CAR>_<variant>.gin`, e.g. `GIN_240SX_Decel.gin`, `GIN_300ZX_DCL.gin`,
`GIN_TVR_Cerbera.gin`. 129 of the 160 files are named by an `engineaudio` collection; 31 belong to cut cars
(`GIN_BMW_450i*`, `GIN_Porsche_Cayenne*`, `GIN_TVR_Chimaera*`, ...).

### Tuning classes

Which `.gin`, bank and tuning a car uses is AttribSys data in `GLOBAL/ATTRIBUTES.BIN`; the rules that use it
are in [specs/engine-sound.md](../specs/engine-sound.md) and [specs/engine-sound-effects.md](../specs/engine-sound-effects.md).
Counts are for the PC install **[verified]**; field names and meanings are from the decomp's generated headers
**[decomp]**.

| Class (collections, fields) | What it holds |
|---|---|
| `pvehicle` (121, 66) | per car: `engineaudio[ ]` (RefSpecs, one per audio upgrade level), `ShiftSND[ ]` and `TurboSND[ ]` (`UpgradeSpecs` = `{RefSpec Item, u8 Level}`, 16 bytes), `TrafficEngType`, `TruckSndFX`, `OnHit*` / `OnScrape*` / `OnBottomOut` audio-event links |
| `engineaudio` (70: `default` + 69 sets, 40) | `Filename_GinsuAccel`, `Filename_GinsuDecel` (StringKey), `BankName_mainRAM` (`CAR_nn_ENG_MB_EE.abk`), `BankName_auxRAM[ ]` (`..._SPU.abk`), `SweetBank[ ]` (`SWTN_CAR_nn_MB.abk`, `CAR_WHINE_00.abk`), `acceltrans` (RefSpec), `CarID` (`nn`), `EngType` (0 V4, 1 V6, 2 V8), `Priority`, `MaybeV8`, `Tranny`, `MinRPM`, `MaxRPM`, `PhysicsRPM_Map` (Bezier y values), `Master_Vol`; mix levels `AEMSMix_S_RPM` / `_L_RPM`, `GINSUMix_S_RPM` / `_L_RPM`, `DECEL_AEMSMix_*`, `DECEL_GINSUMix_*`, `Ginsu_ACL_Neg_S_RPM` / `_L_RPM`, thresholds `AccelDeltaRPMThreshold` (75 to 150), `DecelDeltaRPMThreshold`; volumes `AEMSVol`, `DECEL_AEMSVol`, `GINSUAccelVol`, `GinsuDecelVol` (0 to 32767), `Vol_ShiftSweets`, `Vol_Sputters`; the decel window `GINSU_Decel_MinRPM`, `GINSU_Decel_MaxRPM`, `GINSU_DECEL_FADE_IN`, `GINSU_DECEL_FADE_OUT`; `GINSU_LowPassCutoff`; `DecelPitchOffset` (0 in all data) |
| `acceltrans` (28, 5) | `AccelFromIdle_PEAK_T`, `_RESUME_T`, `_INTERUPT_T` (ms), `_PEAK_RPM`, `_PEAK_VOL` |
| `shiftpattern` (25, 24) | `BankName` (`SOUND/SHIFTING/GEAR_{SML,MED,LRG,BIG}_{Base,Lev1..3}.abk`), sound delays and volumes, the up-shift curves (`Up_DisengageFall[ ]`, `Up_Engage`, with Bezier matrices), the down-shift stage times and RPMs, the post-shift LFO |
| `turbosfx` (18, 6) | `BankName` (`SOUND/TURBO/TURBO_*.abk`), `Vol_Spool`, `ChargeTime`, `Leak_Rate`, `Vol_Blowoff1`, `Vol_Blowoff2` |
| `audiosystem` (9, 20) | `mostwanted`: bank lists `AEMS_SkidBanks`, `AEMS_NOSBanks`, `AEMS_StitchBanks`, `AEMS_MiscBanks`, `AEMS_RNBanks`, `AEMS_WNBanks`, `AEMS_EnvBanks`, `AEMS_FEBanks`, `EvtSys` (`.csi` list); the others hold the speech paths of each language |
| `audioimpact` (131, 7), `audioscrape` (7, 2) | collision sounds: `STITCH_LEVEL_0..3`, `Volumes`, `StreamSweetner`, `DESCRIPTION[ ]`; scrape `CSIS_EFFECT` |

Bezier fields (`PhysicsRPM_Map`, `Up_Engage_Curve`, `Up_DisengageFall_Curve`) are `Attrib::Types::Matrix`
values: four rows `(x, y, 0, 0)` of `f32` are the control points. `stShiftPair` is `{i16 RPM, i16 Time}`.
Example, `pvehicle/bmwm3gtr` -> `engineaudio/tvr_cerb` (accelerate `GIN_TVR_Cerbera.gin`, decelerate
`GIN_TVR_Cerbera_DCL.gin`, bank `CAR_66_ENG_MB_EE.abk`, `MinRPM` 1500, `MaxRPM` 7784) and
`shiftpattern/0x6EB87040` (bank `GEAR_MED_Lev3.abk`). Bank contents: `CAR_66_ENG_MB_EE.abk` has 8 sounds
(112 KB), the `_SPU` twin 7, `SWTN_CAR_66_MB.abk` 12, a `SKID_*` bank 5, `Nitrous_00_MB.abk` 3,
`Stich_Collision_MB.abk` 171 **[verified]** (sound counts from the `BNKl` header, dummy entry excluded).

## Sound stitches

A collision sound is not one sample: the `audioimpact` lists (`STITCH_LEVEL_n`) hold ids of a *stitch*, a short
chain of pieces of `IG_GLOBAL/Stich_Collision_MB.abk`. `GLOBAL/InGameB.bun` has three bundles (8003B500) of
608, 90 and 22 stitches; the first is the collision one (its piece ids run 0 to 170 for the bank's 171
sounds, the other two refer to the static and whoosh banks). Each stitch is a `0003B502` record and a
`0003B503` record, in the same order (the order is the id) **[verified on the bytes]**:

| Chunk | Layout |
|---|---|
| `0003B502` (20 B) | `u32` hash (a name), `u16` volume (`0x7FFF` = full), `u16` id (equals the position), `u32` piece count, 8 bytes not understood |
| `0003B503` (16 B per piece) | `u16` sample (the bank's sound counted from **0**: `BNKl` entry = sample + 1), `u16` volume (`0x3FFF` is typical), `u16` A, `u16` B, 8 bytes not understood |

**[unconfirmed]** reading of the piece fields: A is the number of samples after which the next piece starts
(2,700 to 4,600 at 36,000 Hz, which is 75 to 130 ms, shorter than the 0.1 to 0.5 s pieces, so they overlap);
B (0 to 100) is a delay or random range and is not used. The Rust reader plays the pieces in order with the
cumulative A as start times.

How the car picks the stitch: `pvehicle` lists `OnHitGround`, `OnHitWorld`, `OnHitObject`, `OnBottomOut`,
`OnScrapeGround`, `OnScrapeWorld`, `OnScrapeObject`, `OnBottomScrape` as arrays of 32-byte records
`{u32 selector class, u32 selector key, u32 0, u32 wrapper class, u32 wrapper key, u32 0, f32, f32}`. The selector
is a `simsurface` (or `carbody` for car-to-car) the link applies to, with `default` as the fallback; the wrapper
(class `0xEBCEE74C`, collections `carhitwall`, `carhitgrass`, `carscrapepavement`, ...) holds the references to the
`audioimpact` / `audioscrape` collections itself: the first (in the layout) for a hit on the car's side and
others for a front hit (`DESCRIPTION` has `FRONT` or `SIDE`). The two floats look like a speed range
(1 and 5, 1 and 30, 0 and 1) and are not used **[verified: names and structure; floats unconfirmed]**.

## Which bank sound

The engine specs name the sounds by Csis ids (`FX_SHIFTING_01` sample 0, ...); the `.csi` files that map an id to a
bank sound are not decoded, so the Rust code picks by the order in the banks, checked on durations and loop
points (`BNKl` entry numbers, from 1) **[unconfirmed except where noted]**:

| Bank | Sounds |
|---|---|
| `SHIFTING/GEAR_*.abk` | 1 up shift (0.57 s), 2 down shift (0.68 s), 3 brake mash (0.34 s; the small banks have it, the big ones 2 sounds) |
| `ENGINE/SWTN_CAR_nn_MB.abk` (12) | 1 to 7 short pops (0.01 to 0.08 s, the sputters), 8 to 12 longer (0.14 to 0.36 s): 10 is used for sweetener 0 and 8 for sweetener 1 |
| `ENGINE/CAR_WHINE_00.abk` | the reverse whine loop |
| `TURBO/TURBO_*.abk` (5) | 1 spool loop, 2 blow-off 1, 3 and 4 blow-offs 2 and 3, 5 another loop (unused) |
| `NOS/Nitrous_00_MB.abk` (3) | 1 loop (2.4 s), 2 (0.55 s, unused), 3 purge (1.4 s) |
| `SKIDS/SKID_BIG_MB.abk` (5 loops) | 1 and 2 on asphalt (squeal, burnout), 3 and 4 on loose surfaces; 5 unused |
| `IG_GLOBAL/ROADNOISE_00_MB.abk` (15) | 1 to 7 are the road loops, numbered by `Aud_Roadnoise_LOOP` **[verified by the count]**; 8 to 15 are transition sounds (unused) |
| `IG_GLOBAL/WIND_00_MB.abk` (5) | 1 to 4 wind loops (28,000 Hz), 5 (12,000 Hz); only 1 is played |
| `IG_GLOBAL/FX_MAIN_MEM_MB.abk` (4) | four 3 to 5 s loops, used for the scrapes: ground, wall, car |

`simsurface` has `Aud_Skid_Type` (0 asphalt, concrete and the like; 1 grass, dirt, sand, gravel, snow, mud, golf) and
`Aud_Roadnoise_LOOP` (0 none for the loose surfaces, 1 concrete, stone, wood and roof tiles, 2 cobble, 3 water and
railroad, 5 everything else including asphalt, 6 a blown tire) **[verified]**.

## Interactive music: `MW_Music.mpf` + `.mus` **[community; checked against the files]**

| Offset (`.mpf`) | Field | Value here |
|---|---|---|
| 0x00 | `xDFP` (`PFDx` read as LE) | |
| 0x04 / 0x05 | u8 version / sub-version | **5 / 1** (vgmstream: "Need for Speed: Most Wanted, Carbon, SSX on Tour") |
| 0x2C | u32 tracks table | 0x1ADF4 |
| 0x30 | u32 tracks data | 0x1ADF8 |
| 0x34 | u32 samples table (8 B per stream: offset/0x80 or bank index, duration in ms) | 0x1AE0C |
| 0x38 | u32 end of samples table | 0x213D4 → (0x213D4 − 0x1AE0C) / 8 = **3,257 streams**, the same as the `SCHl` count in `.mus` **[verified]** |

The file has one track (`0x0D` = 1) with 7 sections, 70 events, 123 routers, 5 variables and 3,681 nodes. The
track entry at `0x1ADF8` is `{u32 first sample = 0, u16 sub-bank count = 0, …, u32 BE checksum at +0x08}`. All 3,257
sample-table offsets point at a `SCHl` in `.mus`, are strictly ascending (first stream at 0x100, last at
523,893,120) and no stream overlaps the next. The stored duration equals `samples * 1000 / 36000` within 1 ms for
every stream; the total is 203.9 minutes. **[verified]** The track entry stores a big-endian checksum at +0x08. Its
value `FA CE A5 8C` (at `.mpf` 0x1AE00) is also the first 4 bytes of `MW_Music.mus` **[verified]**. In `.mus`,
streams start at 0x100 and sample offsets are multiplied by 0x80.

### The node graph (PathFinder) **[verified on the bytes; behaviour in [specs/music-graph.md](../specs/music-graph.md)]**

The rest of the header (all `u8` counts at 0x0C to 0x11 and the `u16` node count at 0x12, then `u32` offsets):

| Offset | Field |
|---|---|
| 0x0C / 0x0D / 0x0E / 0x0F / 0x10 / 0x11 | project index 0 / tracks 1 / sections 7 / events 70 / routers 123 / variables 5 |
| 0x12 | `u16` nodes: 3,681 |
| 0x14 | node offset table, `u16` each, value `* 4` = byte offset (0x48) |
| 0x1C | event offset table, `u16` each, value `* 4` (0x178C0); events at 0x1794C |
| 0x24 | variable table (0x1A8AC): 5 x 20 bytes, a 16-byte name (`rapsheet`, `pursuitid`, `partnode`, `newnode`, `ambstate`) and a `u32` initial value |
| 0x28 | router offset table (0x1A910): 124 `u32`, indexes in `u32` units; router data from 0x1AB00 |

A **node** is `16 + 4 * n` bytes: `d0` (bits 0-15 id = stream index + 1, or 0 group head / 0xFFFF end / 0xFFFD fire
event; bits 21-26 section), `d1` (bits 0-11 router, 12-16 transition count `n`, 20-23 beats, 24-31 bars), `d2`
(bits 0-15 the group head), `d3` (the event id of a fire-event node) and `n` transitions `{i8 lo, i8 hi, i16
target}` over a 0 to 127 control value. 3,326 audio nodes, 89 heads, 89 ends and 177 fire-event nodes fill 0x1D0C
to 0x178C0 exactly. A **router** entry is a `u32` `key << 16 | value` that replaces a chosen target equal to
`key`. An **event** is 20 header bytes (`id24` and the action count in the top byte at +12) and 12-byte actions
`{mask, w1, w2}`; opcode = bits 8-14 of `w1` (18 opcodes), `BRANCHTO` is 4 with the node in the low 16 bits of
`w2`. Each of the 26 songs has an event of two actions: stop everything, then branch to the song's first node;
following the first transition of every node from there to the end node gives the song's streams (a gapless run
of the `.mus`, with the last bars repeated in most songs) that add up to the song's length. Sections: 1 to 4 pursuit music, 5 the songs and the
start-screen music, 6 ambience. The meaning of a few flag bits and of 26 condition properties is unknown.

The AttribSys class `music` (5 fields: `Artist`, `SongName`, `Album`, `PathEvent`, `Defplay`; 27 collections, one a
template) lists the songs; the `PFMapping` array of the `audiosystem` collection that `LicensedMusic` of
`audiosystem/0x7E4B0ED2` names is the song order ([attributes.md](attributes.md)). The game side is EA's
PathFinder 5.01.04 driven by `EAXSound/sfxctl/SFXCTL_Pathfinder5.cpp` and `SFXObj_Pathfinder.cpp` **[decomp]**.

## Speech and NIS streams (`.big` / `.idx` / `.evt` / `.csi`)

- `.big` holds `SCHl` streams (counts above), each starting on a `0x100` boundary, with the
  gap after a stream's `SCEl` zero-filled **[verified]**. Between groups of streams there are non-stream tables
  of unknown layout (579 in `copspeech.big`, 4 in `NISAudio.big`, which are the same counts as the `.idx` entry
  counts), so a stream scan walks `SCHl`..`SCEl`, then advances in `0x100` steps to the next `SCHl`.
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
