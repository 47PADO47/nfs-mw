# RIDE 6 audio: Wwise events, banks and media

How the RIDE 6 install (`D:\RIDE 6`, UE 5.6.1, Wwise integration) keeps the sound of its bikes, and what could and could
not be read. The behaviour built on it is in [specs/ride6-engine-sound.md](../specs/ride6-engine-sound.md), the mod
workflow in [modding/ride6-bikes-sound.md](../modding/ride6-bikes-sound.md). The data is the user's and is never
committed ([provenance](../provenance/ride6-audio.md)).

## Where the data is

| Layer | Where | Readable? |
|---|---|---|
| Event, parameter and bike-binding assets (`.uasset`) | the IoStore containers `ride6/Paks/pakchunk*-Windows.utoc/.ucas`, under `ride6/Content/assets/audio/` (1,969 packaged files have `audio` in their path) | Yes, without a key (CUE4Parse, UE5.6, with the game's `.usmap`) |
| Sound banks `WwiseAudio/*.bnk`, `WwiseAudio/Event/*.bnk` and the media `WwiseAudio/Media/<id>.wem` | not in the IoStore (the 134,905 packaged files are all `.uasset`, `.ubulk`, `.umap`, `.ushaderbytecode`); by the paths the events carry they are plain files, which the cook puts in a `.pak` | **No.** `pakchunk0-Windows.pak` (1.18 GB, the only large pak; the other 43 are under 4 KB) has pak version 11 and its footer says *encrypted index*. The encryption key is not in the install; FModel's settings on this machine hold none for RIDE 6 (`mainKey` is empty). No key was extracted, guessed or searched for. |
| Wwise authoring data (the blend containers, RTPC curves) | inside the banks | follows from the row above |

Everything below is therefore from the `.uasset`s, or from the executable, except where a user supplies exported banks.
A user who legitimately holds the key can export the banks with their own tooling and use the importer's `extract-bnk`
command (`sound::cli::extract_bnk`) ([modding](../modding/ride6-bikes-sound.md#decoding-the-recordings)); nothing in this repository needs the key.

## Assets

| Asset (under `ride6/Content/assets/audio/`) | Class | Holds |
|---|---|---|
| `Bindings/bikes/CHASSIS_AUDIO` | `DataTable` of `IgnitionDataAssetTableRow` | 634 rows, key `CH_<chassis id>$Stock` or `$Custom` (317 chassis), value the path of a bike audio data asset |
| `Bindings/bikes/DataAssets/<name>_Stock`, `_Custom` | `BikeAudioDataSource_C` | the bike's events and effect numbers (194 distinct assets over the 634 rows) |
| `Bindings/bikes/BikeAudioDataSource` | Blueprint class | the base values every data asset inherits (default gear events, backfire defaults) |
| `Events/bikes/<brand>/<model>/<name>_EN`, `_EX` | `AkAudioEvent` | the engine and exhaust loops (infinite events), both in the bank `<name>` |
| `Events/Common/Backfires/...`, `Events/Common/GearChanges/...` | `AkAudioEvent` | backfire bangs and gear changes; the event bank lists the recordings it plays |
| `Blueprints/BikeAudioComponent_BP`, `BikeAudioDataComponent_BP` | Blueprint classes | Doppler exponents, sockets, rpm noise per surface, limiter overrides, hysteresis |
| `Game_Parameters/Parameters/...` | `AkRtpc` | RTPCs for options, cameras, ambience and the bike speed (`P1_Speed`); the engine parameters are named in code |

`CHASSIS_AUDIO` shares: many chassis point at one asset (the RM and stock versions of a model, models of one family).
The merge rule is the engine's: an asset lists only the fields it changes; every other field has the base asset's value.

### The event record

An `AkAudioEvent` carries, for the `SFX` language, `EventCookedData.EventLanguageMap[0].Value`: `EventId`, a list of
`SoundBanks` (each: `SoundBankId`, `DebugName`, `bContainsMedia`, `SoundBankType` `User` or `Event`, `PackagedFile.PathName`
such as `WwiseAudio/KTM_EXC_450.bnk` or `WwiseAudio/Event/Backfire_KTM_EXC_450.bnk`) and `Media` (each: `MediaId`,
`DebugName` of the authoring recording such as `Backfire_KTM_EXC_450_02.wav`, `PackagedFile.PathName`
`WwiseAudio/Media/<id>.wem`). The audio-node lists are empty in this cook. Also `MinimumDuration`, `MaximumDuration`
and `IsInfinite` (engine and exhaust: infinite).

An engine event lists no media: its recordings are inside the *user* bank `WwiseAudio/<name>.bnk`
(`bContainsMedia = true`), which holds both `<name>_EN` and `<name>_EX`.

### The KTM 450 SMR 2022

Chassis `KTM_450-SMR-2022`; both variants use the engine bank `KTM_EXC_450` (event ids `3544254634` EN, `3544254652` EX,
bank id `3132996580`, file `WwiseAudio/KTM_EXC_450.bnk`).

| Event | Id | Bank | Recordings (media id: authoring name) |
|---|---|---|---|
| `Backfire_KTM_EXC_450` (Custom only) | 3367577182 | `Event/Backfire_KTM_EXC_450.bnk` | 277083137 `_01`, 18229670 `_02`, 134814594 `_03`, 76202780 `_04`, 963388660 `_05`; 23.8 to 51.9 ms |
| `GearDown_KTM_EXC_450` (Custom only) | 208625966 | `Event/GearDown_KTM_EXC_450.bnk` | the five backfire recordings above, and `GearDown_gsxr1000-001` (104321985, 884634499), `-002` (199543514, 551943984), `-003` (268459296, 348650929), `-004` (1015751896, 1049052940) |
| `GearUp` (both) | 602699311 | `VehicleCommon.bnk` (shared) | none listed |
| `GearDown` (Stock) | n/a | shared | none listed |

The event lists two media ids for each `GearDown_gsxr1000` clunk (same authoring name); what the pair is (two encodings,
or two perspectives) and which one plays is a bank decision that was not read.

The numbers of `AudioEffects` (backfire, down-shift spike, muffler filter) are in the spec, sections 3 and 5.

## Sound banks (`.bnk`)

Only the parts the importer reads. A bank is a sequence of chunks, each `tag[4]`, `size u32 LE`, body:

| Chunk | Body |
|---|---|
| `BKHD` | `version u32`, `bank id u32`, then fields that are not needed |
| `DIDX` | the media index: entries of 12 bytes `media id u32`, `offset u32`, `size u32` |
| `DATA` | the media bytes; an entry's `offset` is from the start of this body |
| `HIRC` | the object hierarchy (sounds, blend and switch containers, RTPC curves, events): **not parsed**, its layout depends on the bank version |

A media is a Wwise RIFF `.wem` (Vorbis, Opus or PCM). `vgmstream-cli` decodes it to WAV, with the loop points as a
`smpl` chunk when the file has them.

## Tools

All outside the repository, in the scratch folder (`C:\r6` junction):

| Tool | Use |
|---|---|
| `C:\r6\tools\CUE4Parse\R6Tool` (`json <package>`) | the JSON of an audio package |
| `C:\r6\snd\tools\resolve_bike_audio.py <chassis> <Stock\|Custom> <out.json>` | one bike's resolved audio data: events, banks, media names, effect numbers (merged by the importer's `sound::data`) |
| `C:\r6\snd\tools\all_bikes_audio.py` | `C:\r6\snd\all-bikes-audio.csv`: the 634 rows with their engine, exhaust, bang, flame, gear and supercharger events |
| `C:\r6\snd\tools\sdkgrep.py`, `idavt.py` | struct layouts and vtable addresses from the SDK dump (for the executable) |
| `C:\r6\tools\vgmstream\vgmstream-cli.exe` | `.wem` to WAV |
| `ride6-bike-import sound`, `ride6-bike-import extract-bnk` (`sound::cli::run`, `sound::cli::extract_bnk` of `mods/ride6-bikes`) | build the mod's sound set; split a bank into `.wem` files |
