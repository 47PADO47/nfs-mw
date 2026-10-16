# Custom music: folders, playlists and the radio

Plays the player's own audio files on the radio, next to or instead of the EA Trax songs. Companion of
[music-graph.md](music-graph.md) (the EA Trax songs and play lists, sections 6 and 7) and
[interactive-music.md](interactive-music.md) (the pursuit music, which custom tracks do not replace).

- **Sources:** `docs/mods/xnfsmusicplayer.md` (a summary of the XNFSMusicPlayer mod's features, read from its
  public repository) for the list of features. Nothing of the mod's code was read or used. The rest is this
  project's design, marked **[design]**.
- **Not available:** a listening test of any output; a run against a real music folder. The tests use generated
  WAV files.
- **Evidence tags:** **[mod]** a feature the XNFSMusicPlayer mod has (behaviour only), **[design]** a choice
  made here, **[deferred]** not in this version.

## 1. What the player gets

- Their own tracks play on the radio, while driving and (with `radio next`) in the menus, with the same controls as
  the EA Trax songs: pause, resume, next, previous, the radio on and off. [design]
- Tracks come from **folders** (every supported file in the folder) and from **playlist files** (M3U or M3U8).
  [mod: custom folder and M3U playlists]
- The title and artist shown on the HUD caption come from the playlist entry, the file's tags or the file name.
  [mod: chyron captions, title and artist] [design for the order]
- Two modes: **replace** (custom tracks take the place of the EA Trax songs) and **mix** (one EA Trax song, then one
  custom track, and so on). [design; the mod has the replace idea only]
- Ordered or shuffled play, the same as the EA Trax lists: a shuffled round plays every track once before any
  repeats. [mod: random mode, with the random bug fixed]
- A fade-out of 5000 ms when a custom track is stopped before its end. [mod: `FadeoutTime` 5000]
- Per-context sources: the menus and free roam can each have their own folders and playlists. [design; the mod has
  one list]

Off by default. With the feature off, nothing is scanned or decoded and the radio behaves as before.

## 2. Behaviour

### 2.1 Sources

- A **folder source** is one folder. Its tracks are the files directly inside it (no subfolders), whose extension is
  one of `mp3`, `ogg`, `flac` or `wav`, compared without case. Files are sorted by file name, case-insensitively
  (ties by the exact name). A folder that cannot be read is logged and skipped. [design]
- A **playlist source** is an M3U or M3U8 file. Its text is read as UTF-8 (a byte-order mark is dropped); if that
  fails, each byte is read as one Latin-1 character. [mod: M3U files had to be non-Unicode, which this avoids]
  - Blank lines and lines starting with `#` are ignored, except `#EXTINF`.
  - `#EXTINF:<seconds>,<text>` gives the next entry its length and text. The length `-1` means unknown. The text is
    `Artist - Title` when it has a separator ` - `; otherwise the whole text is the title.
  - Every other line names a file. Trailing spaces and a carriage return are removed. A relative name is relative to
    the folder of the playlist file, with either slash. An absolute name is used as it is.
  - A named file that does not exist is logged and skipped.
- A context's list is its folders in the order the config gives them, then its playlists in order; each file is in the
  list once. The library holds each file once across all lists. When a file is named by a playlist entry with a
  caption and also found in a folder, the playlist entry's caption is kept. [design]

### 2.2 Contexts

The radio has two play lists today: the **front end** (menus) and **in game** (driving). The custom sources are
per context:

| Context (settings key) | Radio list it feeds | Status |
|---|---|---|
| `menu` | front end | played with `radio next` in the menus |
| `free_roam` | in game | played while driving |
| `race` | none yet | read from the config; not played |
| `pursuit` | none yet | read from the config; not played |

- A context uses its own folders and playlists when it names any. Otherwise it uses the global `folders` and
  `playlists`. [design]
- `race` and `pursuit` need the radio to know when a race or a pursuit starts; today a pursuit only holds the
  radio's place (`update_held`), and there is no race context. The keys are read so the config is stable when those
  contexts come. [deferred]

### 2.3 Replace and mix

For the current context, with `n` custom tracks and `m` EA Trax songs in the list:

- **Replace:** while `n > 0`, every next track is custom. When `n = 0` the EA Trax list plays, as before.
- **Mix:** with `n > 0` and `m > 0`, the source alternates: EA Trax, custom, EA Trax, and so on. The first source is
  EA Trax. When one side is empty, the other plays alone.
- The alternation is by source, not by time: a song that ends and a track that is skipped both count as one turn.
- The state of the alternation is kept with the history (below). Going back and forward through the history does
  not change the turn.

With the feature off (`n = 0` in every context) the choice is the EA Trax list alone, which is the behaviour
before this feature.

### 2.4 Order inside a list

- **Ordered:** the next track after the last one played, wrapping to the first. [design; same as the EA Trax list]
- **Shuffle:** a round is a random order of all the tracks of the list. A round ends when every track has played;
  the next round is shuffled again. The first track of a new round is not the last track of the old one when the list
  has more than one track. [design]
- The mode is the radio's mode (`radio shuffle` and `radio ordered` change it for EA Trax and custom alike). The
  config sets the mode the radio starts in: `shuffle`. [design]

### 2.5 Controls and history

- The radio's controls work the same for custom tracks: toggle, next, previous, pause, resume, `radio play`
  (EA Trax only: it takes a song number), status and list.
- The history holds EA Trax songs and custom tracks in one order, so previous and next walk across both.
- `radio status` and the caption show the custom track's title and artist. The HUD card and the EA Trax chyron show
  them without change.

### 2.6 Fade

- `fade_ms` (default 5000, 0 to 60000) is the fade-out of a custom track that is stopped before its end: by
  `radio next`, `radio previous` (from a track that is cut short), leaving the game, `radio off`, or the pursuit
  taking the radio's place. [mod: `FadeoutTime`]
- A custom track that ends by itself is not faded. An EA Trax song keeps its own short fade (80 ms).
- While the fade runs, the next track may start. The two overlap for the length of the fade. This is a deliberate
  crossfade; set `fade_ms = 0` for a cut. [design]
- Pausing keeps the existing pause fade (50 ms) for both kinds of track. [design]

### 2.7 Failures

- A track that cannot be opened (missing file, unknown format, damaged header) is logged, and the radio tries the
  next one. It tries at most once per track of the list in one start, so a folder of bad files cannot loop. If no
  track opens, the radio reports the last error once (as it does for an EA Trax song), and starts again on
  `radio next`. [design]
- A track that fails in its middle ends there and the radio goes on with the next one. The error is logged.
  [design]

### 2.8 When the config is read

- The config is read when the radio is first loaded (the first drive, or the first `radio` command). Changing the
  file while the game runs takes effect on the next start. [design]

## 3. The config: `[custom_music]` in the per-user config file

The file is the one that holds the other settings (`game_dir`, `radio`, and so on). The section is read by its own
reader; the other settings ignore it. A missing section is the defaults.

```toml
[custom_music]
enabled = false          # the feature is off by default
mode = "replace"         # "replace" or "mix"
shuffle = false          # start in shuffle order
fade_ms = 5000           # 0 to 60000
folders = ["D:/Music/NFS"]        # global sources, every context
playlists = ["D:/Music/nfs.m3u8"]

[custom_music.menu]      # per-context sources; empty or missing = use the global ones
folders = []
playlists = []

[custom_music.free_roam]
folders = []
playlists = []

[custom_music.race]
folders = []
playlists = []

[custom_music.pursuit]
folders = []
playlists = []
```

| Key | Type | Default | Rule |
|---|---|---|---|
| `enabled` | bool | `false` | Off means no scan and no decode. |
| `mode` | `"replace"` or `"mix"` | `"replace"` | Other text: logged, default. |
| `shuffle` | bool | `false` | The starting mode of the radio. |
| `fade_ms` | integer | `5000` | Clamped to 0 to 60000; a non-number is logged and the default is used. |
| `folders`, `playlists` | arrays of strings | empty | A non-string item is logged and skipped. |
| `[custom_music.<context>]` | tables | empty | Same two arrays. |

- A relative path in the file is relative to the folder of the config file. [design]
- Each key is read on its own: one bad value does not discard the others (the rule of the other settings).
- A file that is not TOML is logged and the feature stays off.

## 4. Formats and the decoder

### 4.1 Choice

**symphonia** 0.6 (pure Rust, MPL-2.0, the decoder family that `kira` uses for its own file playback). [design]

- Features switched on: `mp3`, `ogg` (container), `vorbis`, `flac`, `wav`, `pcm`, and the tag readers `id3v1`, `id3v2`,
  `ape`. Everything else is off: AAC, ALAC, ADPCM, AIFF, CAF, MP4, MKV, and SIMD (`opt-simd`), so the crate builds
  without the optional native paths.
- **Licence:** MPL-2.0 is allowed by `deny.toml`. It is file-level copyleft: this project does not modify the crate's
  files, so no MPL file changes. Its dependencies are checked by `cargo deny check licenses`.
- **New dependency:** symphonia and its sub-crates (`symphonia-core`, `-common`, `-metadata`, `-bundle-mp3`,
  `-bundle-flac`, `-codec-vorbis`, `-codec-pcm`, `-format-ogg`, `-format-riff`) enter `Cargo.lock` for the first time.
  kira's own copy is not enabled in this build.

Rejected: writing WAV, MP3 or Ogg decoders by hand (bugs for no gain); `rodio` and `cpal` playback (a second output
path next to kira); decoding the whole file into memory (a 4-minute stereo track is about 85 MB of frames; the
decoder streams instead).

### 4.2 Formats supported in this version

| Extension | Container | Codec | Notes |
|---|---|---|---|
| `mp3` | MPEG audio | MP3 | Tags: ID3v2, ID3v1. |
| `ogg` | Ogg | Vorbis | Tags: Vorbis comments. |
| `flac` | native FLAC | FLAC | Tags: Vorbis comments. |
| `wav` | RIFF WAVE | PCM | 8, 16, 24, 32-bit integer and 32-bit float. |

The mod also plays tracker modules and MIDI (with a SoundFont); those are **[deferred]**. So is AAC in M4A and MP4.

### 4.3 Frames

The decoder gives interleaved samples of any channel count. The radio takes stereo frames: mono is duplicated, the
first two channels of a surround stream are kept (the same rule as the EA cut-scene audio). Blocks of about 0.2 s of
frames go to the radio's stream, which resamples to the device rate with the same code as the EA Trax songs.

## 5. Port design

### 5.1 Where the code goes

| Part | Place | Depends on |
|---|---|---|
| M3U/M3U8 parsing, the text rule, path resolution, folder selection and sorting | `libs/blackbox-playlist` (new, engine-generic, no I/O, no game names) | std |
| Reading the folders and playlist files, building the track list, the config reader | `crates/nfsmw/src/audio/radio/custom/library.rs`, `crates/nfsmw/src/settings/custom_music.rs` | `blackbox-playlist`, `std` |
| The decoder thread | `crates/nfsmw/src/audio/radio/custom/decode.rs` | `symphonia` |
| Order (queue), alternation (rotation), label | `crates/nfsmw/src/audio/radio/custom/{queue,rotation,label}.rs` | none (pure) |
| Fade of a stopped stream | `crates/nfsmw/src/audio/radio/stream.rs` | kira |

`libs/blackbox-playlist` could also hold the decoder later (an engine-generic audio file reader). It stays in
`crates/` for now because the file list and the radio are the only users. [design]

### 5.2 Types

- `CustomTrack { path, title, artist, album, length_secs }`: one file. `length_secs` is the M3U length when given, else
  0 until the track is opened (then the decoder's length replaces it).
- `CustomLibrary`: the tracks of all sources (each file once) and, per context, the list of track numbers in it.
- `CustomQueue`: the order state of one context: a permutation of its list, the position, the round.
- `Source { Trax, Custom }` and `rotation::choose(mode, has_trax, has_custom, last) -> Option<Source>`.
- `Entry { Song(usize), Custom(usize) }`: one history item. `Song` is an index in the EA Trax song list; `Custom`
  is a track number in the library.
- `NowPlaying.song` holds the EA Trax index, or `NO_SONG` (`usize::MAX`) for a custom track. [design]

### 5.3 Flow

1. The radio loads (first drive). The config is read, the library is built: folders are read, playlists parsed,
   tracks listed; nothing is decoded yet.
2. The radio asks `rotation::choose` for the next source of the current context, then the queue of that context
   (the EA Trax list for `Trax`) for the next track.
3. A custom track is opened: the decoder thread starts, the header (sample rate, tags, length) comes back before the
   call returns; the blocks follow on a channel. The stream is handed to the mixer's music track.
4. The label is built: M3U text, then tags, then the file stem. The `NowPlaying` is set, the serial moves, the HUD
   announces it.
5. Ends and skips work as for EA Trax songs, with the fade of section 2.6 for an early stop.

The decoder thread does the probing before the header is sent, and the header is awaited on the game thread (a few
milliseconds). Blocks are sent through a bounded channel, so the thread runs at most a few blocks ahead.

### 5.4 Threads

- Game thread: the library, the choice, the header wait.
- Decoder thread (one per track, as for the EA Trax songs): decode and send. Ends at the end of the file, on a
  decoding error (logged), or when the receiver is gone.
- Audio thread: the stream sound, unchanged except for the fade length it reads from the handle.

## 6. Tests

- `libs/blackbox-playlist`: M3U with `#EXTM3U`, `#EXTINF` with and without ` - `, unknown length `-1`, comments,
  blank lines, CRLF, a BOM, Latin-1 bytes, relative and absolute names, a `..` name; folder selection by extension
  in any case, sorted order, subfolders skipped.
- `custom/queue.rs`: ordered wraps; shuffle covers every track once per round; no immediate repeat across rounds
  (checked with a fixed random sequence); an empty list gives none; a single track repeats.
- `custom/rotation.rs`: replace with and without custom tracks; mix alternates, starts with EA Trax, and falls back
  to the other side when one is empty; no list gives none.
- `custom/label.rs`: the order M3U, tags, stem.
- `custom/decode.rs` (with a WAV written to the scratchpad): a 16-bit mono WAV and a 24-bit stereo one decode to the
  expected frame count and length; a non-audio file is an error at open; a truncated WAV ends with its frames
  delivered.
- `settings/custom_music.rs`: defaults; a missing section; each key on its own (a bad value keeps the others);
  clamping of `fade_ms`; relative paths against the config folder.
- `audio/radio/tests/custom.rs`: with fake sources (no decoder): replace and mix sequences through `Radio::update`;
  previous and next across both kinds; a failing custom track is skipped; off means the EA Trax behaviour is unchanged.

## 7. Decisions, and what is not done

- Replace is the default because it is what the mod does; mix is the option for people who want both. [design]
- Per-track amplitude and loop points: **[deferred]**. The mod has them; a loop point needs a decoder that seeks, and
  the amplitude needs a gain per track. The config has no keys for them yet.
- Internet radio streams: **[deferred]**. They need a network client, a new licence check, and reconnect handling.
- Installer music, the jukebox and save-data hooks of the mod: **not applicable**. They patch the original binary;
  this build does not.
- Interactive noding (music from game events, such as the pursuit): the EA Trax graph covers it; custom tracks do
  not follow events. The `race` and `pursuit` contexts wait for that. [deferred]
- The MIDI and tracker formats: **[deferred]**.
- Event IDs of the mod: not used. The rewrite has its own event IDs (music-graph.md section 4).
- The radio must have a map to start: if `MW_Music.mpf` is missing, no radio loads and custom tracks do not play
  either. Playing without the map would need the radio to accept an empty song list. [deferred]
- A decoder that cannot keep up (underruns) is only counted in `radio status`, as for EA Trax songs.
- Metadata written by a tool with a non-Latin-1 legacy code page in an M3U: the Latin-1 rule reads it as mojibake.
  Windows-1252 would read the 0x80 to 0x9F range correctly. [deferred]

## 8. Port status

What the port does now, against sections 1 to 7. The feature is off unless `[custom_music] enabled = true`. The
config reader is `settings/custom_music.rs`; the radio's custom side is `audio/radio/custom/`.

| Item | Status | Where |
|---|---|---|
| 2.6 fade of a stopped custom track | done: the track's stream is given `fade_ms` (default 5000) when it starts; dropping its handle fades it over that time. Every early stop goes through `Radio::stop`: next, previous, radio off, leaving the game, and a pursuit holding the radio. A track that ends by itself is not faded. EA Trax songs keep their 80 ms fade | `audio/radio/mod.rs` `start_custom`, `stream.rs` `set_fade_out`; tests `tests/custom.rs` (default 5 s), `tests/stream.rs` (`fade_ms = 0` cuts at once) |
| 2.5 controls | done, no new binding was needed: `RadioToggle` (N), `RadioNext` (`.`, media next, pad D-pad right), `RadioPrevious` (`,`, media previous, pad D-pad left) are in `input/bindings.rs` and change with `bind`. Note: `music-graph.md` section 7 still names M for the toggle; the bound key is N | `input/action.rs`, `input/bindings.rs`, `audio/radio/controls.rs` |
| 2.2 menu context | done: the menu sources feed the front-end list, which plays on `radio next` outside the game | `custom/library.rs` `Library::build` |
| 2.2 free roam context | done: the free-roam sources feed the in-game list | `custom/library.rs` |
| 2.3 replace and mix, 2.4 order | done, with the rules tested | `custom/rotation.rs`, `custom/queue.rs` |
| 2.5 history across both kinds | done | `audio/radio/mod.rs` `start_entry`, `record`; `controls.rs` |
| 2.7 failures | done: a custom track that will not open is skipped, at most once per track in one start | `audio/radio/mod.rs` `start_next` |
| 2.2 pursuit context | **not played (decision needed).** The radio already holds its place while a pursuit has the music (`update_held`, fed by `Scene::is_pursuing`), so no custom track plays in a chase. Playing the pursuit sources would put custom tracks over the pursuit music, which section 1 says they do not replace. Pick one: the pursuit sources play instead of the pursuit music (needs the music's gating changed), or they stay unplayed | `audio/radio/glue.rs` `update_radio`, `audio/plugin.rs` |
| 2.2 race context | **not played.** The port has no race mode: Quick Race stops at its options screen (`frontend/track_options.rs`) | none |
| 2.8 config read at the first drive | done | `audio/radio/glue.rs` `load_radio` |
| 4 formats (mp3, ogg, flac, wav) | done through symphonia; the decoder streams blocks | `custom/decode.rs` |
| EA Trax playlist screen (`frontend-menus.md` 2.2) | **done for stock songs**: the jukebox screen drives `EA_Trax_Jukebox.fng` with the four-state song mode, table order and random flag reversed from `speed.exe` ([eatrax-jukebox.md](eatrax-jukebox.md)). Custom tracks are not listed: the package has four fixed rows | `frontend/jukebox.rs`, `settings/eatrax.rs` |
| 7 per-track amplitude and loop points | **not done, and no source defines them.** The M3U `#EXTINF` line gives only the length and the text, and this spec has no sidecar format. Stays deferred | none |

Tests in this pass: `tests/custom.rs` (the 5 s default fade), `tests/stream.rs` (the zero fade). The earlier tests of
sections 6 cover the rest.

## 9. Next leads

- Per-track amplitude and loop points in the config (`[custom_music.tracks."<path>"]`), after a decoder that seeks.
- Pursuit context: decide the question in section 8 (play over the pursuit music or not). Race context waits for a race mode.
- EA Trax playlist screen: an R task on `EA_Trax_Jukebox.fng`'s game messages (what each reads and writes, the 28-bit
  mask and the per-song rows), then an I task to drive it and to add the custom rows.
- A prefetch of the next track's header on the decoder thread, so a track change never waits on the game thread.
- Windows-1252 for legacy M3U files.
- Streams: a network client with a licence check (ureq or reqwest, both MIT or Apache-2.0 with their own dependency
  trees), ICY metadata for the caption.
- Move the decoder into a `libs/` crate once a second game uses it.
