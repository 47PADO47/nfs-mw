# ea-audio

Decoders for the audio containers and codecs of EA Black Box games (Need for Speed Underground 2, Most Wanted,
Carbon, …): bytes in, 16-bit PCM out. No game knowledge, no file access, no audio output.

| Module | What |
|---|---|
| `schl` | `SCHl` / `SCCl` / `SCDl` / `SCEl` block streams with the `PT` and `GSTR` header tags (channels, rate, sample count, loop, codec, per-channel offsets) |
| `abk` | `ABKC` sound banks: module / player / sample tables and the embedded `BNKl` sound list |
| `mus` | the `xDFP` v5 music map (`.mpf`): tracks and the stream table (offset, duration) for a `.mus` file; `mus::graph`, its PathFinder node graph (nodes, transitions, routers, events) and the walker that turns a track into a chain of streams |
| `big` | finding the streams inside `.big` containers |
| `gin` | the EA-XAS audio inside granular engine-loop files (`Gnsu`) |
| `codec` | EA-XA (revisions 1 and 2, mono and stereo flavours), EA-XAS v0, EA MicroTalk 10:1 (UTK) |

```rust
// A whole stream or engine loop in memory:
let pcm = ea_audio::decode(&bytes)?;          // Pcm { sample_rate, channels, samples: Vec<i16>, loop_range }

// A bank: list the sounds, decode one.
let bank = ea_audio::abk::Bank::parse(&bytes)?;
for sound in bank.sounds() {
    let pcm = bank.decode(sound.index)?;
}

// The 533 MB music file, one block at a time, through your own ReadAt (file, mmap, ...):
let mpf = ea_audio::mus::Mpf::parse(&mpf_bytes)?;
let mut reader = mpf.open(&mus_source, 42)?;  // stream 42
let mut pcm = Vec::new();
while let Some(frames) = reader.next_chunk(&mut pcm)? { /* feed the audio device */ }

// The node graph: which streams a song plays, one after the other.
let graph = ea_audio::mus::graph::Graph::parse(&mpf_bytes)?;
let start = graph.song_start(event_id).unwrap();       // node of a song's event (24 bits compared)
let chain = mpf.chain(&graph, start, 0)?;              // segments: node, stream, start_ms, duration_ms
for seg in &chain.segments { /* open stream seg.stream, queue it after the previous one */ }

// A .big container: find the streams, decode any of them.
for entry in ea_audio::big::scan(&big_source)? {
    let pcm = entry.decode(&big_source)?;
}
```

`ReadAt` is a two-method trait (`len`, `read_at`) implemented for `[u8]` and `Vec<u8>`; implement it over a
`File` with positioned reads to avoid loading large files. `schl::extent` measures a stream from its block
prefixes alone.

The decoder never loops: `Pcm::loop_range` holds the loop points in frames (end exclusive) and the player repeats.

## Verified on the PC v1.3 install of Need for Speed: Most Wanted

`cargo test --release -p ea-audio -- --ignored` (with `NFSMW_GAME_DIR` set; about 12 s) decodes everything:

| Data | Count | Result |
|---|---|---|
| `SOUND/**/*.abk` | 301 banks, 2,577 sounds (40 stereo, 1,237 with loop points) | every sound decodes to its header's sample count; the sample tables reference exactly these sounds |
| `PFDATA/MW_Music.mus` via `.mpf` | 3,257 streams, 203.9 minutes, 36,000 Hz | stored durations match the decoded length to 2 ms |
| `SPEECH/copspeech.big` | 13,562 MicroTalk streams (177 stereo), 24,000 Hz | all decode; speech is mastered at full scale and touches the rails in runs of at most 8 samples |
| `STREAMS/NISAudio.big` | 142 streams (up to 6 channels), 44,100 Hz | all decode |
| `ENGINE/*.gin` | 160 files | all decode; frame boundaries are as smooth as frame interiors |
| `PFDATA/MW_Music.mpf` graph | 3,681 nodes, 70 events, 123 routers | `cargo test -p ea-audio --test real_graph -- --ignored`: every audio node names an existing stream, all 26 song events resolve to chains of the expected stream counts and lengths |

EA-XA output is identical, sample for sample, to FFmpeg's `adpcm_ea_r3` / `adpcm_ea_r2` on music and cut-scene
streams. MicroTalk has no second implementation to compare with here; it is checked for smoothness, clipping and
bit-stream sync over all 13,562 streams. Unit tests use synthetic bytes only.

## Not implemented

EALayer3 / MPEG, PS-ADPCM, DSP, IMA and PCM sample formats (none occur in Most Wanted), EA-XAS version 1, the
CBX variant of MicroTalk, MicroTalk loop resets, streamed (`.ast`) bank entries, the PathFinder node graph in
`.mpf`, and the `.idx` / `.evt` / `.csi` companion files.

## Sources and licence

Format notes: [`docs/specs/audio-containers.md`](../../docs/specs/audio-containers.md) and
[`docs/formats/audio.md`](../../docs/formats/audio.md); provenance: [`docs/provenance/audio-codecs.md`](../../docs/provenance/audio-codecs.md).
The codecs are ported from [vgmstream](https://github.com/vgmstream/vgmstream) (ISC-style licence) and, for
MicroTalk, through it from [utkencode](https://github.com/daddesio/utkencode) (public domain); the notices are kept
in the ported files and listed in the repository's `NOTICE`.

License: MIT OR Apache-2.0.
