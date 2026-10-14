# XNFSMusicPlayer

XNFSMusicPlayer is a C++ ASI plugin for Need for Speed: Most Wanted (2005, PC) and Need for Speed: Carbon, written by xan1242. It replaces the games' built-in music playback with a BASS-based player. It can play custom folders, M3U playlists, and online radio streams, and it exposes per-track settings and a set of hooks into the game's sound, save-data, and jukebox code. It is a mod for the original game binaries, not a standalone player.

Status: documentation only. Nothing was built, run, or downloaded. The code is read from the repository, not executed.

## 1. Metadata

| Field | Value |
|---|---|
| Repository | https://github.com/xan1242/XNFSMusicPlayer |
| Author | xan1242 (repo owner). Credits: ThirteenAG (Ultimate ASI Loader, injector, IniReader), Un4seen Developments (BASS, BASSMIDI, BASSFLAC) [README] |
| Type | .asi plugin (ASI loader based, `DllMain` entry point) [source: `XNFSMusicPlayer/dllmain.cpp`] |
| Licence | MIT. Copyright (c) 2021 Lovro Pleše, per the LICENSE file [source: LICENSE]. Note the copyright holder differs from the repo owner. The bundled BASS, BASSMIDI and BASSFLAC libraries carry their own licences, which were not checked. |
| Language | C++ (Visual Studio solution `XNFSMusicPlayer.sln`, `.vcxproj`) [source: repo listing] |
| Target game builds | Need for Speed: Most Wanted (`GAME_MW`) and Need for Speed: Carbon (`GAME_CARBON`), selected by preprocessor define [source: `dllmain.cpp`, `XNFSMusicPlayer_MW.h`, `XNFSMusicPlayer_Carbon.h`] |
| Latest visible activity | Not confirmed. The repo page showed "29 Commits" and an empty Releases section. No commit or release date was visible to the fetch tool. |
| Stars, issues, open PRs | Not recorded (not visible in the fetched pages). |

## 2. What it does

Each feature below is backed by the README or by code. The README was read through a summarising fetch, so the wording is paraphrased.

- Replaces in-game music with a BASS-based stream player. [README] [source: `XNFSMusicPlayer_Common.h` (BASS handle creation), `dllmain.cpp`]
- Custom playlists from a folder and M3U playlists. The main playlist path is set by `PlaylistFile`. M3U files must be non-Unicode; paths with special characters may fail. [README] [source: `InitConfig` keys]
- Online radio streaming with SHOUTcast metadata. [README] [source: `MetaSyncProc` and stream functions listed in `XNFSMusicPlayer_Common.h`]
- Automatic metadata reading (title, artist, album) for tracks. [README] [source: `WriteModeToIni` writes `Title`, `Album`, `Artist` per track]
- Track looping, including loop start and loop end points. [README] [source: keys `Loop`, `LoopStart`, `LoopEnd`; functions `LoopCounterSync`, `StreamPositionLoopSync`]
- Per-track settings: amplitude (`ModuleAmpLevel`), loop points, and playback mode (default `AL`). [source: `XNFSMusicPlayer_MW.h`]
- Support for MP3, OGG, WAV, tracker formats, FLAC (via BASSFLAC), and MIDI with one SoundFont at a time (via BASSMIDI). [README] [source: `CreateBASSHandleByFileType`, `CheckIfSupportedFile`]
- Game volume follows in-game volume control, when enabled. [source: `BASSVolume`, `BASSVolumeCave`, `GameVolumeControl`]
- Track switching through a hotkey, default the media next-track key. [source: `TrackSwitchHotkeyThread`, key `TrackSwitchHotkey`]
- Interactive noding: music follows in-game events, such as ambience and pursuit music, through the game's own event IDs. Can be turned off with `EnableInteractiveNoding`. [source: `StartPursuitMusicCave`, `StartAmbienceCave`, `NodePaths.txt`, `Overrides.txt`]
- Fade-out on pause and on stop (default 5000 ms). [source: key `FadeoutTime`, `PauseBassChannel`]
- Installer music: a first-run installer and an install music file. [source: `PlayInstallerMusic`, `SetupInstallMusic`, keys `InstallerMusic`, `InstallMusicFile`]
- Random-mode RNG fix: the game's random track mode is made properly random. [source: `bRandom_Custom` hooked at `MW_RNGFIX_ADDRESS`, comment in `MW_MusicAddresses.h`]
- Track change allowed outside the front end, not only in the menus. [source: `TRACKCHANGE_NOP_ADDR` comment]
- Jukebox support: writes jukebox pointers and disables the game's own jukebox write path. [source: `WriteJukePointers`, `DISABLEJUKEWRITE1/2_ADDRESS`]
- Save-data pointer redirection, so the game's save path can reference the player's data. [source: `SetSaveDataPointer*`, `SaveDataWriter*`]
- Chyron (in-game caption) notifications from the player. The README lists NFSC chyron support as TODO for the Carbon build. [source: `NotifyChyronCave`; README TODO]
- Console window for logs and verbosity, and an optional log file. [source: `LogFileEnabled`, `LogFilePath`, `VerbosityLevel`, `separate_console` in `dllmain.cpp`]

## 3. How it does it

### 3.1 Loading

- The plugin is an `.asi` DLL loaded by an ASI loader. The README credits ThirteenAG's Ultimate ASI Loader, but the loader is not in the repo. [README] [inferred: `DllMain` entry point]
- `DllMain` on `DLL_PROCESS_ATTACH`: detects whether the process has a separate console (cursor at 0,0), redirects stdio to `CON` if so, sets the working directory to the executable folder, then calls `Init()`. [source: `dllmain.cpp`]
- `Init()` checks the BASS version (`HIWORD(BASS_GetVersion()) != BASSVERSION` shows a message box and fails), then calls `InitConfig()` and `InjectCode()`. [source: `dllmain.cpp`]
- The version check is the only BASS check in this file. BASS itself is initialised elsewhere (`BASSCreate` and related paths in `XNFSMusicPlayer_Common.h`). [source: `dllmain.cpp`] [inferred for where BASS_Init runs]

### 3.2 Code injection

`InjectCode()` in `XNFSMusicPlayer_MW.h` patches the game binary at runtime. Mechanisms used:

- JMP caves: a jump is written at a game address to redirect into a plugin function. Used for channel pause/resume, pursuit and ambience music, time remaining, save data, chyron notification, path finder, and jukebox write. [source]
- CALL caves: a call is written at a game address to run a plugin function, for example `SoundUpdater` (only with `UseGameThread`), `GetNumberOfEATrax`, `BASSVolume` (only with `GameVolumeControl`), `QueueNodeInterceptor`, `BASSCreate`, `BASSDestroy`. [source]
- Vtable writes: function pointers in the game's sound channel vtable are overwritten to `StopBassChannel`, `PauseBassChannel`, `BASSPlayChannel`. [source: `STOPCHANNELVTABLE_ADDRESS`, `PAUSECHANNELVTABLE_ADDRESS`, `PLAYCHANNELVTABLE_ADDRESS`]
- NOP patches: instructions are replaced with NOPs to disable game write paths (jukebox, feDatabase, volume critical sections, track change check). [source]
- Literal patch points also exist, for example a JMP at `0x00462ACF` to `vsprintfdamnit`. [source]

Patch addresses are specific to one retail build of each game. The MW set is in `MW_MusicAddresses.h`. Example addresses (MW, as written in the header): `EAXSOUNDUPDATE_ADDRESS = 0x004D0610`, `QUEUENODE_ADDRESS = 0x0082FF80`, `THEGAMEFLOWMANAGER_ADDRESS = 0x00925E90`, `IS_GAME_PAUSED = *(bool*)0x0091CAE4`. [source] The Carbon address set is in `Carbon_MusicAddresses.h` and was not read.

### 3.3 Audio

- Playback uses BASS streams. `BASSPlayChannel`, `PauseBassChannel`, `StopBassChannel` and `BASSTimeRemaining` map the game's channel calls onto BASS. [source: function names in `XNFSMusicPlayer_Common.h`]
- Sync callbacks (`MetaSyncProc`, `LoopCounterSync`, `StreamPositionLoopSync`) handle metadata and looping. [source]
- Asynchronous file reads (`UseAsyncRead`, `AsyncBufferSize`) and an input buffer (`InputBufferSize`) feed the stream. [source: keys in `InitConfig`]
- Optional game-thread updates: `SoundUpdater` runs from the game's sound update call when `UseGameThread` is set. [source]

### 3.4 Playlists and event mapping

- Tracks are mapped from game event IDs to files via `NodePaths.txt` (generated by first-time setup), `Overrides.txt` (generated by setup, EventID overrides), and `PlaylistConfig.ini` (per-track settings). [README] [source: `DoFileNodePathsStruct`, `DoFileOverrides`, `AppendOverridesAndFree`]
- Event IDs for tracks and online streams are generated by `GenerateEventIDForTrack` and `GenerateEventIDForOnlineStream`. [source]
- `SearchPathByNodeID` and `SearchPathByID` look up tracks. [source]

### 3.5 Threads

- `TrackSwitchHotkeyThread` watches the hotkey. [source]
- Thread count and update period are configured (`NumOfThreads`, `UpdatePeriod`). The threading code was not read in detail. [inferred]

## 4. Configuration

Config is read from the game's ini files. The main file is `scripts\XNFSMusicPlayer.ini` [README]. Defaults below are from `InitConfig()` and `InitConfig_GameSpecific()` [source].

| Section | Key | Default |
|---|---|---|
| Installer | InstallerCompleted | 0 |
| Installer | InstallerMusic | 1 |
| Installer | InstallMusicFile | INSTALL_MUSIC_FILE (macro, value not read) |
| XNFSMusicPlayer | LogFileEnabled | 0 |
| XNFSMusicPlayer | LogFilePath | DEFAULT_LOG_PATH (read only if LogFileEnabled) |
| XNFSMusicPlayer | GlobalVolume | 10000 |
| XNFSMusicPlayer | SampleRate | 44100 |
| XNFSMusicPlayer | InputBufferSize | 500 |
| XNFSMusicPlayer | DeviceBufferSize | -1 |
| XNFSMusicPlayer | UseAsyncRead | 1 |
| XNFSMusicPlayer | AsyncBufferSize | 262144 (minimum 4096) |
| XNFSMusicPlayer | SlowHashing | 0 |
| XNFSMusicPlayer | VerbosityLevel | 0 |
| XNFSMusicPlayer | SoundFontFilePath | default.sf2 (only if BASS_USE_MIDI defined) |
| Playback | GameVolumeControl | 1 |
| Playback | EnableInteractiveNoding | 1 |
| Playback | GlobalTrackerAmp | 50.0 |
| Playback | VolumeMultiplier | 1.0 |
| Playback | FadeoutTime | 5000 |
| Playback | TrackSwitchHotkey | VK_MEDIA_NEXT_TRACK |
| GameThread | UseGameThread | 0 |
| GameThread | UpdateLength | 500 |
| Threading | NumOfThreads | 0 |
| Threading | UpdatePeriod | 0 |
| Playlists | PlaylistFile | DEFAULT_PLAYLIST_PATH (macro) |
| Playlists | PlaylistConfigFile | DEFAULT_PLAYLISTCONFIG_PATH (macro) |
| Playlists | OverridePathConfigFile | DEFAULT_OVERRIDES_PATH (macro) |
| Playlists | NodePathConfigFile | scripts\XNFSMusicPlayer\NodePaths.txt |

Per-track keys, in a section named by the six-digit hex PathfinderID: `ModuleAmpLevel` (50.0), `Loop` (0), `LoopStart` (0), `LoopEnd` (0), `Title`, `Album`, `Artist` (NULL), `PlaybackMode` ("AL"). [source: `XNFSMusicPlayer_MW.h`]

Files named in the README: `scripts\XNFSMusicPlayer.ini`, `scripts\XNFSMusicPlayer\PlaylistConfig.ini`, `scripts\XNFSMusicPlayer\Playlist.m3u`, `scripts\XNFSMusicPlayer\NodePaths.txt`, `scripts\XNFSMusicPlayer\Overrides.txt`. [README]

Note: the code's `PlaylistConfigFile` default and the README file names may differ in path form. Check the exact macro values before relying on them.

## 5. How we could implement it in nfs-mw

Rule: the repo is MIT licensed, so its code could be read and adapted under that licence, but this project's rule is spec-first. The behaviour below is written as facts and specs. No code is to be copied from the repo. Also note the `docs/licensing.md` rule for ports. Anything taken from the repo's code should be recorded as a spec before implementation.

Feature assessment:

| Feature | Applies to clean-room rewrite? | Notes |
|---|---|---|
| Custom folder and M3U playlists | Yes | Pure file and format handling. Belongs in a format or data layer. |
| Per-track settings (amp, loop points, playback mode) | Yes | A settings file keyed by track ID. |
| Online radio with stream metadata | Yes, optional | Needs a network stream decoder and ICY/SHOUTcast metadata parsing. Could be deferred. |
| Audio decode (MP3, OGG, WAV, FLAC, MIDI) | Yes | Decoder choice is an engine decision. MIDI needs a soundfont synth. |
| Event-to-track mapping (interactive noding) | Yes, but needs research | Requires knowing the rewrite's own event IDs. The original's IDs are game data, not something to copy. |
| Track switch hotkey | Yes | Input handling, in the game's own input layer. |
| Game volume follow | Yes | Mixer routing. |
| Jukebox and save-data pointer redirection | No, not as described | These are byte patches into the original binary. A rewrite has its own save and jukebox code. |
| RNG fix for random mode | Maybe | Behaviour could be specified after research; the fix itself is a patch. |
| Chyron notifications | Maybe | Depends on whether the rewrite has the chyron system. |
| ASI loader, DllMain, console redirect, in-binary JMP/CALL/NOP patches | No | Loader-specific hacks. A Rust rewrite does not inject into the original binary. |

Behaviours to specify (before any implementation):

1. Playlist load: input is a folder or an M3U file; output is an ordered list of tracks with file paths. Behaviour for non-Unicode M3U, missing files, and special characters.
2. Per-track settings: amp level, loop enable, loop start, loop end, playback mode. Defaults as in section 4.
3. Loop semantics: what happens at loop end, and how loop start and loop end are interpreted when both are 0.
4. Fade: time in ms, applied on pause and stop.
5. Volume: global volume, volume multiplier, and game volume control on or off.
6. Event mapping: the mapping from the game's event IDs to tracks, and the override file format. Requires the rewrite's own event data, not the original's.
7. Hotkey: default key and rebinding.
8. Metadata: title, artist, album, read from tags or set per track.
9. Radio stream: URL, reconnect behaviour, and metadata interval parsing (if in scope).
10. Installer music: a file played during first-run setup (if in scope).

Data and formats needed:

- An ini or TOML config with the sections in section 4, using the rewrite's own names where the original's names are not required.
- A per-track settings file keyed by a stable track ID.
- An M3U parser (format crate, takes `&[u8]`, per the repo rules).
- An event-to-track mapping file, specified from the rewrite's own data.

Natural home (per AGENTS.md rules):

- `libs/`, engine-generic and Bevy-free: M3U parsing, playlist model, per-track settings model, loop and fade logic, volume maths, metadata model. Format crates take `&[u8]` and never open files.
- `crates/`, NFS:MW-specific: event IDs and mapping for NFS:MW, default track set, mapping of game events to the player. `crates/` depends on `libs/`, never the reverse.
- The audio backend (decoder and output) goes in an optional renderer or audio backend crate under `libs/`, following ADR 0004 for swappable backends. The backend may depend on Bevy only if it is a leaf crate.
- Install discovery goes through `game-install`, which is the only crate that touches the install. [per AGENTS.md]

Risks:

- Licensing: BASS is a commercial library with its own licence. A Rust rewrite cannot bundle it without checking its terms. Use a permissively licensed decoder instead, and record the choice in the spec.
- MIDI and SoundFont support needs a synth, which is a large dependency. Consider deferring.
- Event mapping depends on game event IDs that have to be researched in the rewrite, not copied from the patch addresses.
- Metadata reading from tags adds format parsing work.
- Radio streaming adds network I/O and reconnection logic, which needs careful error handling.
- Relying on the original's jukebox and save behaviour would tie the rewrite to the original binary. Keep the rewrite independent.
- Audio thread timing: the original's game-thread option suggests thread-safety issues. Design the audio thread boundary up front.

## 6. Open questions and what could not be verified

- Latest commit date and release status: not visible. The repo page showed 29 commits and no releases.
- The README was read through a summarising fetch. The tool would not return it verbatim, so exact wording and the full TODO list could differ from the summary above.
- Carbon address set (`Carbon_MusicAddresses.h`) and the Carbon build: not read.
- `XNFSMusicPlayer_TagReader.h`, `XNFSMusicPlayer_StreamPusher.h`, and the BASS wrapper code: not read.
- `InjectCode()` body in full: only the hook table was extracted. Exact patch byte counts for non-JMP patches were not verified.
- `KNOWNISSUES.md` and `PREGIT-CHANGELOG.md`: listed, not read.
- The directory listing for `XNFSMusicPlayer/` was truncated ("View all files" link); some files may be missing from the list.
- Whether the repo's BASS, BASSMIDI and BASSFLAC folders carry their own licence texts: not checked.
- Whether the MIT licence in LICENSE covers all bundled code: not checked file by file.
- Which game build the addresses target (retail release, version, and region): not stated in the header.
- Whether the ASI loader used by the author is distributed in the repo: not in the top-level listing.

## 7. Sources

- https://github.com/xan1242/XNFSMusicPlayer (repo page, README summary, top-level and XNFSMusicPlayer/ directory listings)
- https://github.com/xan1242/XNFSMusicPlayer/tree/master/XNFSMusicPlayer (directory listing)
- https://raw.githubusercontent.com/xan1242/XNFSMusicPlayer/master/README.md (summarised, not verbatim)
- https://raw.githubusercontent.com/xan1242/XNFSMusicPlayer/master/LICENSE
- https://raw.githubusercontent.com/xan1242/XNFSMusicPlayer/master/dllmain.cpp
- https://raw.githubusercontent.com/xan1242/XNFSMusicPlayer/master/XNFSMusicPlayer/MW_MusicAddresses.h
- https://raw.githubusercontent.com/xan1242/XNFSMusicPlayer/master/XNFSMusicPlayer/XNFSMusicPlayer_MW.h
- https://raw.githubusercontent.com/xan1242/XNFSMusicPlayer/master/XNFSMusicPlayer/XNFSMusicPlayer_Common.h
