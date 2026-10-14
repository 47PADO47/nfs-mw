# ealayer3-nfsw

A command-line EALayer3 audio encoder and decoder, with a guide for adding custom music tracks to Need for Speed World (NFSW) through it. This is a standalone tool, not an ASI plugin.

## Metadata

| Field | Value |
|---|---|
| Repo | https://github.com/driftyz700/ealayer3-nfsw |
| Author | DriFtyZ (repo owner, per README credits). Original ealayer3 tool by Ben Moench; looping and multi-channel patch by Translu. |
| Type | Tool (CLI encoder/decoder) with a modding guide. Not an `.asi` plugin. |
| Licence | BSD-style, three-clause ("New" or "Modified" BSD). `license.txt` states copyright Ben Moench (c) 2010. |
| Language | C++ (CMake build). The GitHub language bar was not visible in the fetched page. |
| Target game build | Need for Speed World (NFSW). The README does not name a build version. This is not Need for Speed: Most Wanted (2005). |
| Latest visible activity | 5 commits on `main`. The last commit date was not visible in the fetched page. |

Summary: the repo contains ealayer3, a tool that reads and writes EA's EALayer3 audio format (MP3 to EALayer3 and back, to WAV or MP3). The README explains how to make NFSW accept custom freeroam music. NFSW expects codec version 6 audio, but the bundled tool only writes version 5, which the game still plays. The guide covers encoding the tracks and registering them in the game's vault files with VltEd.

Verification note: the README was read through a summarising fetch, not as verbatim text. Details below marked [README] come from that summary.

## What it does

Each feature is listed with its evidence.

- **Encode MP3 to EALayer3.** `-E` takes one or more MPEG input files and combines them into one EALayer3 output. [source: `src/Main.cpp` encode mode; README Part 1]
- **Output layouts.** Headerless stream (default), `--two-files` (stream plus a separate `<output>.header`), `--single-block`, and `--header-b`. [source: `src/Main.cpp`; README Part 1]
- **Looping.** `--loop` marks output as looping, for the single-block and two-files layouts. [source: `src/Main.cpp`] The README says looping and multi-channel support came from a patch by Translu. [README]
- **Decode EALayer3 to MP3 or WAV.** Default mode. Options for MP3 (`-m`), WAV (`-w`), and multi-channel WAV (`-mc`). [source: `src/Main.cpp`]
- **Stream selection and inspection.** `-s` selects one stream or all; `-n` prints file information; `-v` is verbose. [source: `src/Main.cpp`]
- **Parser version override.** `--parser5` and `--parser6` force a parser; by default the parser is chosen automatically. [source: `src/Main.cpp`]
- **Game install registration (guide only).** Edit three vault files in VltEd so the game loads the new tracks: add `rwac_music` nodes, enlarge the `audiosystem/mostwanted` array `0xab458147` from 17 to 20 entries, and point `blackboard/audio_freeroam` at the custom tracks and disable hero-track interruption. [README]
- **Sample modscript.** The guide includes a modscript that performs the vault edits and saves with Ctrl+S. [README] The script itself was not read.

## How it does it

This is a file-format tool with no runtime component.

- **No ASI loader, no hooks, no memory patches, no DirectX code.** The repo is a CMake project building two executables. [source: `CMakeLists.txt`] The README does not mention any game-process interaction. [README]
- **Build targets.** `ealayer3` (from the main sources) and `ealayer3testdriver` (the same sources with `src/TestDriver.cpp` in place of `Main.cpp`). [source: `CMakeLists.txt`]
- **Dependencies.** Boost 1.36.0 or later and mpg123 (library and headers). [source: `CMakeLists.txt`]
- **Source layout.** `src/` holds `AllFormats`, `BlockLoader`/`BlockWriter`, `FileDecoder`, `Generator`, `MpegGenerator`, `MpegParser`, `MpegOutputStream`, `OutputStream`, `Parser`, `PcmOutputStream`, `WaveWriter`, and a `Loaders/`, `Parsers/`, `Writers/` split. [source: `src/` listing] The contents of the subfolders were not read.
- **Flow.** Encode: parse each input MPEG, feed frames to a generator, emit EALayer3 blocks, and write them immediately or buffer them for single-block and two-files output. Decode: configure a file decoder with input, offset, parser, stream, and output format, then run it. [source: `src/Main.cpp` high-level flow]
- **Header format details.** The README says the first header byte is the codec version, following bytes hold codec settings, and a looping flag is 0x40 for non-looping or 0x60 for looping. It also says the last 8 header bytes, related to a loop point, are written as zeros, and that multi-channel looping tracks start simultaneously. [README] These were not checked against the source.
- **Version 6 gap.** NFSW expects v6 audio, which the bundled encoder does not produce. [README] The README suggests older ealayer3 versions could work with a manually split header. [README] Not verified.

## Configuration

There is no config file. Configuration is entirely through command-line options.

| Option | Argument | Default | Purpose |
|---|---|---|---|
| (positional) | input file | required | Unrecognized arguments are treated as input |
| `-i`, `--offset` | byte offset | 0 | Start reading at this offset |
| `-o`, `--output` | file name | empty | Output name. In encode mode, derived from the first input with an `.ealayer3` extension |
| `-s`, `--stream` | stream number or `all` | first stream | Select a stream (1-based) |
| `-m`, `--mp3` | none | auto | MP3 output |
| `-w`, `--wave` | none | auto | WAV output |
| `-mc`, `--multi-wave` | none | auto | Multi-channel WAV output |
| `--parser5` / `--parser6` | none | auto | Force parser version |
| `-n`, `--info` | none | off | Print file information |
| `-v`, `--verbose` | none | off | Verbose output |
| `-b-`, `--no-banner` | none | banner shown | Suppress banner (stderr) |
| `-E` | none | off | Encode mode |
| `--single-block` | none | off | Single-block stream |
| `--header-b` | none | off | Header B stream |
| `--two-files` | none | off | Headerless stream plus `.header` file |
| `--loop` | none | off | Mark output as looping |

Source: `src/Main.cpp` [source]. Note that some decode options are parsed but ignored in encode mode, and the code does not show the decoder's behaviour when no output name is given. [source]

## How we could implement it in nfs-mw

Licence check: the repo is BSD-style, so its code may legally be reused with the notice kept. Our project rule (`docs/licensing.md`, spec-first) still applies, so the plan below describes behaviour and formats and does not port the C++ code. This feature applies to our rewrite, but the loader-specific parts do not.

Behaviours to specify (spec-first, in `docs/formats/` or a new EALayer3 page):

1. Header parsing for codec versions 5 and 6: version byte, codec settings, looping flag values, track length and sample count fields.
2. Block layout: headerless, two-files (header plus stream), single-block, and header-B variants.
3. Decode to PCM and WAV, including multi-channel and stream selection.
4. Encode from PCM or MP3 frames into v6 blocks, with a loop point. This is the part the upstream tool lacks, so the NFSW v6 requirement must be confirmed from our own install data first.
5. Round-trip tests: decode a shipped NFSW `.snr`/`.sns` pair and check sample count and length.

Data and formats needed:
- Real NFSW `.snr` and `.sns` samples from the user's install (read-only, never committed; `docs/` leak rules apply).
- The vault layout for `rwac_music`, `audiosystem/mostwanted`, and `blackboard/audio_freeroam`, documented from our own reading. The vault format belongs with existing format docs.

Natural homes:
- EALayer3 bitstream and block reader and writer: `libs/` as a format crate taking `&[u8]`, with no file I/O (AGENTS.md rule). EA's audio format is not NFS:MW-specific, so it fits `libs/` if the code stays free of NFS:MW names.
- Vault edits and track registration for freeroam music: `crates/`, since they depend on NFSW vault hashes and paths. `crates/` depends on `libs/`, never the reverse.
- Any CLI wrapper: a thin binary in `crates/` or `tools/`, not in `libs/`.

Risks:
- Format uncertainty. The header fields and the loop-point zeroing are reported only in the README. Each must be confirmed against real data before use.
- Vault and hash constants are game-build specific. Keep them in per-version tables, not in `libs/`.
- Audio legality: custom tracks are user-supplied content. We should ship no game audio.
- Loader-specific hacks: none needed. The repo has no ASI or memory patching, so nothing here conflicts with a clean-room rewrite.

## Open questions and what could not be verified

- The README was read as a summary, not verbatim. Header byte values and the zeroed loop-point bytes are unverified against the code or real files.
- The `src/Loaders`, `src/Parsers`, and `src/Writers` subfolders were not read, so codec internals are not documented here.
- The decoder's behaviour without an output name is not shown.
- The NFSW build version the guide targets is not stated.
- The modscript was not read.
- The GitHub language bar and the latest commit date were not visible.
- Whether v5 files play in NFSW as the README claims was not tested.
- Whether an older ealayer3 version with a manually split header works was not tested. The README suggests it, and we have not checked.

## Sources

- https://github.com/driftyz700/ealayer3-nfsw (repo page: file listing, commit count, license link)
- https://raw.githubusercontent.com/driftyz700/ealayer3-nfsw/HEAD/README.md (summary read, not verbatim)
- https://raw.githubusercontent.com/driftyz700/ealayer3-nfsw/HEAD/license.txt
- https://github.com/driftyz700/ealayer3-nfsw/tree/HEAD/src (file listing)
- https://raw.githubusercontent.com/driftyz700/ealayer3-nfsw/HEAD/CMakeLists.txt
- https://raw.githubusercontent.com/driftyz700/ealayer3-nfsw/HEAD/src/Main.cpp (summary read)
