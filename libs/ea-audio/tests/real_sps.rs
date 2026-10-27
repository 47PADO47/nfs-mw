//! `.SPS` streams of the user's own NFS: Most Wanted (2012) install. Every test is ignored without `MW2012_GAME_DIR`.
//!
//! Run with `cargo test --release -p ea-audio --test real_sps -- --ignored --nocapture`. Nothing decoded is written.

use std::io::Cursor;
use std::path::PathBuf;

use ea_audio::sps;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;

fn streams() -> Option<Vec<PathBuf>> {
    let root = PathBuf::from(std::env::var_os("MW2012_GAME_DIR")?).join("SOUND/STREAMS");
    let mut files: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("sps")))
        .collect();
    files.sort();
    Some(files)
}

/// Decode an MP3 stream: its sample rate, the samples per channel and how many packets failed.
fn decode(bytes: Vec<u8>) -> (u32, usize, usize) {
    let mut hint = Hint::new();
    hint.with_extension("mp3");
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes)), MediaSourceStreamOptions::default());
    let mut format = symphonia::default::get_probe()
        .probe(&hint, source, FormatOptions::default(), MetadataOptions::default())
        .expect("the repacked stream is probed as MP3");
    let track = format.tracks().iter().find(|track| matches!(track.codec_params, Some(CodecParameters::Audio(_))));
    let track = track.expect("an audio track");
    let id = track.id;
    let Some(CodecParameters::Audio(params)) = track.codec_params.clone() else { unreachable!() };
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .expect("an MP3 decoder");
    let (mut rate, mut frames, mut failed) = (0, 0, 0);
    while let Ok(Some(packet)) = format.next_packet() {
        if packet.track_id != id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buffer) => {
                rate = buffer.spec().rate();
                frames += buffer.frames();
            }
            Err(_) => failed += 1,
        }
    }
    (rate, frames, failed)
}

#[test]
#[ignore = "reads the 2012 install (MW2012_GAME_DIR)"]
fn every_layer3_stream_repacks_and_decodes_to_its_declared_length() {
    let Some(files) = streams() else { return };
    let (mut decoded, mut skipped) = (0, Vec::new());
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let data = std::fs::read(path).unwrap();
        let mp3 = match sps::to_mp3(&data) {
            Ok(mp3) => mp3,
            Err(error) => {
                skipped.push(format!("{name}: {error}"));
                continue;
            }
        };
        for (at, core) in mp3.cores.iter().enumerate() {
            let (rate, frames, failed) = decode(core.bytes.clone());
            assert_eq!(failed, 0, "{name} core {at}: packets that did not decode");
            assert_eq!(rate, mp3.header.sample_rate, "{name} core {at}");
            let declared = mp3.header.samples as usize;
            let off = frames.abs_diff(declared);
            assert!(off <= 5 * 1152, "{name} core {at}: decoded {frames} samples, the header says {declared}");
        }
        decoded += 1;
    }
    println!("decoded {decoded} streams; skipped: {skipped:?}");
    assert!(decoded >= 50, "only {decoded} streams decoded");
    assert!(skipped.len() <= 3, "{skipped:?}");
}
