# blackbox-movie

Reader and player helpers for the movie container of EA Black Box games (`MOVIES/*.vp6` in Need for Speed:
Most Wanted): a flat run of tagged blocks that interleaves On2 VP6 video (`MV0K` key frames, `MV0F` inter
frames) with an EA `SCHl` audio stream.

```rust
let mut demuxer = blackbox_movie::Demuxer::new(std::io::BufReader::new(file))?;
let header = *demuxer.header();                  // size, frame count, fps = rate / scale
let audio_header = demuxer.audio_header();       // raw SCHl payload, for the audio decoder
for packet in demuxer {
    match packet? {
        blackbox_movie::Packet::Video(frame) => { /* frame.index, frame.key, frame.time, frame.data */ }
        blackbox_movie::Packet::Audio(block) => { /* block.first_sample, block.samples, block.data */ }
        _ => {}
    }
}
```

Takes any `Read` (a `&[u8]` works), never opens files. Format: `docs/formats/video.md`.

License: MIT OR Apache-2.0.
