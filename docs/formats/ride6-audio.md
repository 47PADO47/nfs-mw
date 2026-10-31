# RIDE 6 audio: Wwise events, banks and media

How the RIDE 6 install (`D:\RIDE 6`, UE 5.6.1, Wwise integration) keeps the sound of its bikes, and what could and could
not be read. The behaviour built on it is in [specs/ride6-engine-sound.md](../specs/ride6-engine-sound.md), the mod
workflow in [modding/ride6-bikes-sound.md](../modding/ride6-bikes-sound.md). The data is the user's and is never
committed ([provenance](../provenance/ride6-audio.md)).

## Where the data is

| Layer | Where | Readable? |
|---|---|---|
| Event, parameter and bike-binding assets (`.uasset`) | the IoStore containers `ride6/Paks/pakchunk*-Windows.utoc/.ucas`, under `ride6/Content/assets/audio/` (1,969 packaged files have `audio` in their path) | Yes, without a key (CUE4Parse, UE5.6, with the game's `.usmap`) |
| Sound banks `WwiseAudio/*.bnk`, `WwiseAudio/Event/*.bnk`, `Init.bnk` and the media `WwiseAudio/Media/<id>.wem` | plain files in the `.pak` files; `pakchunk0-Windows.pak` has pak version 11 and an **encrypted index** | **Yes, with the pak key** (below): 1096 banks and 1372 loose media, 2,468 files, 879 MB |
| Wwise authoring data (blend containers, RTPC curves) | inside the banks | the object lists (`HIRC`) are read, see "Sound banks" and "REV engine models" |

### The pak key

The index of the `.pak` files is AES-256 encrypted. **The key was supplied by the user, who found it in the thread
<https://cs.rin.ru/forum/viewtopic.php?f=10&t=100672>**; this project did not break, guess or search for it. It was
**verified by mounting**: with it the reader mounts 88 paks instead of 44 and lists 142,907 files instead of 134,905, among them
the 1096 banks, and every bank extracted is a well formed Wwise bank (`BKHD` version 154, an object list that parses to its end).

The key is a constant of the RIDE 6 mod (`mods/ride6-bikes/src/sound/pak_key.rs`) and nowhere else; the environment variable
`R6_AES` (64 hexadecimal digits, `0x` allowed) or the `pak_key` setting of the mod's `config.toml` replaces it
([modding/ride6-bikes-sound.md](../modding/ride6-bikes-sound.md#extracting-the-banks)). The external pak reader takes the key
from `R6_AES`, which the `pak-sound` command sets for it.

Everything below is from the `.uasset`s, the banks and the executable.

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

A bank is a sequence of chunks, each `tag[4]`, `size u32 LE`, body (`sound::bnk`):

| Chunk | Body |
|---|---|
| `BKHD` | `version u32` (154 in every RIDE 6 bank), `bank id u32`, then fields that are not needed |
| `DIDX` | the media index: entries of 12 bytes `media id u32`, `offset u32`, `size u32` |
| `DATA` | the media bytes; an entry's `offset` is from the start of this body |
| `HIRC` | the object hierarchy: `count u32`, then objects `kind u8`, `size u32`, body (first four bytes: the id) |

A bank with `bContainsMedia` holds its recordings in `DIDX`/`DATA`; an *event bank* (`WwiseAudio/Event/*.bnk`, no media) names
media that are loose files `WwiseAudio/Media/<id>.wem`. The importer looks in the bank first, then in `Media/`.

### Objects read (`sound::hirc`)

| Kind | Body after the id |
|---|---|
| 2 sound | `plugin u32` (low nibble 1: a codec, 4 is Vorbis, 2 ADPCM; else a source plugin), `stream u8`, `source id u32` (the media id of a codec, the id of a plugin object for a source plugin), node parameters (parent id and others, **not parsed**) |
| 3 action | `type u16` (`0x04xx` plays, `0x1303` sets a game parameter), `target id u32` |
| 4 event | the number of actions as a 7-bit variable length number, then the action ids |
| 5, 6, 7, 9 containers | random/sequence, switch, actor-mixer, layer (blend) container; node parameters, then the child ids |
| 17 plugin object | `plugin u32`, `size u32`, the plugin's parameter block, `n u8` and `n` x (`index u8`, `media id u32`), `m u16` game parameter curves |

A curve of a plugin object is `game parameter id u32`, `type u8`, `accumulation u8`, `plugin parameter u8`, `curve id u32`,
`scaling u8`, `points u16`, then points of `x f32`, `y f32`, `interpolation u32` (4 linear, 9 constant). Ids of game parameters
are the 32-bit FNV-1 hash of the lower-case name; the names `RPM`, `Throttle`, `GEAR` and `GearDown` were found by hashing
the strings of the executable.

The parent and children of an object are not read from the node parameters (their layout changes with the bank version) but
from the references objects hold to each other: an object's parent is the first container whose id its body mentions, and a
container's children are the objects that name it as their parent. Ids are 32-bit hashes, so a chance match is practically
impossible, and the result was checked on every event of the 634 chassis rows.

The events of the KTM 450 SMR 2022 in this form:

| Event | Plays |
|---|---|
| `KTM_EXC_450_EN` (3544254634) | a layer container with one Vorbis recording (a 0.97 s sub-bass thump, 95 % of its energy under 100 Hz), the **REV engine sound**, and a sound of the plugin `0x1990002` (38-byte parameter block, not identified) |
| `KTM_EXC_450_EX` (3544254652) | the **REV exhaust sound**; a layer container whose random playlist is five ADPCM clips of 43 to 64 ms (the exhaust crackles) and a silent sound; a play action on an object of another bank |
| `Backfire_KTM_EXC_450` | a random playlist of the five bangs |
| `GearDown_KTM_EXC_450` | sets the game parameter `GearDown`; playlists of the five bangs and of eight clunks (`GearDown_gsxr1000-001..004`, 0.09 to 0.17 s); a sound of the plugin `0x1990002` |
| `GearUp` (`VehicleCommon`, shared) | sets `GearDown`; a playlist of six clunks of 0.12 to 0.15 s; the same plugin sound |

## REV engine models

**Every one of the 634 chassis rows (317 chassis, 122 engine banks, 204 distinct models) makes its engine and its exhaust with one
source plugin**: the Crankcase Audio **REV** engine synthesiser. Its sounds have the plugin number `0x01A01052`, its plugin
object holds a 212-byte parameter block (floats and flags, not interpreted) and the id of a media file whose bytes start with
`PLUG` and `ADM3` (`vgmstream` reads it as "Crankcase ADMx": 43 or 42 recordings, a 4-bit IMA ADPCM variant, 48 kHz mono).
The synthesiser itself is not reproduced; its recordings are (`sound::rev`):

- The `u32` at `0x40` is where the first recording's bytes start; the directory before it holds 48-byte records tagged `SMP2`:
  `hz f32` (engine cycle frequency), `cycles u32` (whole cycles in the clip), `rate u32`, `frames u32`, `mark u32`, `offset u32`.
- The records are always in this order: **20 loops under load**, one long **rev-up** sweep, **20 (19 in two models) loops off load**,
  a long **coast-down** sweep, a long **idle** recording. Loops are 0.57 to 0.60 s and hold whole cycles, so they loop seamlessly
  (their seam is within the spread of ordinary sample steps); the idle loop does not (a click of 3 to 4 times a normal step).
- A loop's pitch is `cycles * rate / frames` (the tag agrees except for the first loop of some models, which has a rounded
  nominal value; the autocorrelation of the audio confirms the first). A four-stroke engine has one cycle per two turns, so
  **rpm = 120 x cycles per second**. Check on the data: over 293 bikes the top loop is at 0.97 of the physics asset's `MaxRPM`
  in the median (quartiles about 0.90 and 1.03, range 0.71 to 2.05).
- The KTM 450 engine model has loops from 3174 to 11361 rpm (26.4 to 94.7 Hz); its idle recording is 4.2 s at 17.4 Hz (2088 rpm; the
  physics asset's idle is 2300). The exhaust model has the same layout at nearly the same pitches.
- Loops under load are much louder at the top of the range than at the bottom (rms 0.011 at the first loop, 0.33 at the last, for
  the KTM engine model); the loops off load are quieter (0.015 to 0.1).
- The idle recording has a clear period near the bike's own idle speed on most engines; on 32 of the 293 bikes it has none (an
  uneven V or two-stroke idle) and the autocorrelation peak is under 0.2. Searched over the whole range instead of a band around
  the idle speed, the period of several idles came out as a harmonic (6349 rpm for a bike that idles at 1000).

### What drives the plugin

The plugin object carries four game parameter curves, the same four in all 204 models: **`Throttle`** to plugin parameter 1,
**`RPM`** to 2, **`GEAR`** to 3 and **`GearDown`** to 5. `GEAR` is the identity above 1 (0 to 6); `GearDown` is 1 below 0.5 and 0
above; `Throttle` and `RPM` map 0 to 1 to 0 to 1, with 37 and 134 distinct shapes over the models (a dead zone of 0.1 to 0.15 for the
throttle in most; for the RPM a straight line in 49 models, otherwise a knee near 0.6 such as (0.6, 0.45)). What the plugin does with
them (the blend of the loops by throttle, the grain playback, the limiter) is inside the plugin and is not read.

## Tools

All outside the repository, in the scratch folder (`C:\r6` junction):

| Tool | Use |
|---|---|
| `C:\r6\tools\CUE4Parse\R6Tool` (`json <package>`) | the JSON of an audio package |
| `C:\r6\snd\tools\resolve_bike_audio.py <chassis> <Stock\|Custom> <out.json>` | one bike's resolved audio data: events, banks, media names, effect numbers (merged by the importer's `sound::data`) |
| `C:\r6\snd\tools\all_bikes_audio.py` | `C:\r6\snd\all-bikes-audio.csv`: the 634 rows with their engine, exhaust, bang, flame, gear and supercharger events |
| `C:\r6\snd\tools\sdkgrep.py`, `idavt.py` | struct layouts and vtable addresses from the SDK dump (for the executable) |
| `C:\r6\tools\vgmstream\vgmstream-cli.exe` | `.wem` to WAV; `-s <n>` picks the n-th recording of a REV model |
| the pak reader (a copy of the CUE4Parse console tool, modes `raw <regex> <folder>` and `audiomap <folder>`) | extracts files of the paks; writes one resolved audio JSON per chassis row; started by `pak-sound` |
| `ride6-bike-import sound`, `extract-bnk`, `pak-sound`, `sound-bikes` (`mods/ride6-bikes/src/sound`) | build one bike's sound from recordings; split a bank; extract the banks; build the sound of every bike from the banks |
