# blackbox-movie

Reader and player helpers for the movie container of EA Black Box games (`MOVIES/*.vp6` in Need for Speed:
Most Wanted): a flat run of tagged blocks that interleaves On2 VP6 video (`MV0K` key frames, `MV0F` inter
frames) with an EA `SCHl` audio stream.

```rust
use blackbox_movie::{Demuxer, Packet, Timeline, Vp6Decoder, YuvFrame};

let mut demuxer = Demuxer::new(std::io::BufReader::new(file))?;
let header = *demuxer.header();                   // size, frame count, fps = rate / scale
let audio_header = demuxer.audio_header();        // raw SCHl payload, for the audio decoder
let mut decoder = Vp6Decoder::new(header.width, header.height)?;
let mut timeline = Timeline::from_header(&header);
let (mut frame, mut rgba) = (YuvFrame::default(), vec![0u8; 1024 * 512 * 4]);

while let Some(packet) = demuxer.next_packet()? {
    match packet {
        Packet::Video(video) => {
            decoder.decode_into(&video.data, &mut frame)?; // every frame, in order
            let update = timeline.update(clock_seconds);    // you own the clock
            if update.present == Some(video.index) {
                frame.to_rgba(&mut rgba);                   // RGBA8, or upload the Y/U/V planes
            }
        }
        Packet::Audio(block) => { /* block.first_sample, block.samples, block.data */ }
        _ => {}
    }
}
```

- `Demuxer` takes any `Read` (a `&[u8]` works) and never opens files. Packets are owned and come in
  file order; the presentation index of a frame is its ordinal, its time `index * scale / rate`.
- `Vp6Decoder` (default feature `vp6`) decodes a frame to a `YuvFrame` (planar 4:2:0, upright). It wraps the
  MIT-licensed VP6 decoder of [ruffle-rs/nihav-vp6](https://github.com/ruffle-rs/nihav-vp6), pinned in the
  workspace `Cargo.toml`. Without the feature only the demuxer and the timeline are built.
- `Timeline` is pure arithmetic: elapsed seconds in, frame to present and frames to skip out. Skipped frames
  still have to be decoded, because inter frames predict from them. Feed it the audio clock to keep lip sync.
- `YuvFrame::to_rgba` converts with BT.601 studio-range coefficients (about 3 ms for a 1024x512 frame on one
  core). A GPU player can instead upload the Y, U and V planes as three textures.
- Audio: the `SCHl` payload and each `AudioPacket::data` are in the form the EA-XA stereo decoder of
  `libs/ea-audio` expects. This crate does not decode audio: parse `audio_header()` with
  `ea_audio::schl::header::parse` and feed each `AudioPacket::data` to an `ea_audio::schl::StreamDecoder` (the game
  does, in `crates/nfsmw/src/movie/player.rs`).

Format: `docs/formats/video.md`. Real-install checks: `NFSMW_GAME_DIR=... cargo test --release -p
blackbox-movie -- --ignored --nocapture`.

License: MIT OR Apache-2.0.
