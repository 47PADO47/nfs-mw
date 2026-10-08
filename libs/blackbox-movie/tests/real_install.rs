//! Against the user's own install; ignored without `NFSMW_GAME_DIR`. Run with
//! `cargo test --release -p blackbox-movie -- --ignored --nocapture`.

use std::io::BufReader;
use std::path::PathBuf;
use std::time::Instant;

use blackbox_movie::{Demuxer, Packet, Timeline, Vp6Decoder};

fn movies() -> Option<Vec<PathBuf>> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    let dir = std::path::Path::new(&dir).join("MOVIES");
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "vp6"))
        .collect();
    files.sort();
    Some(files)
}

fn open(path: &PathBuf) -> Demuxer<BufReader<std::fs::File>> {
    Demuxer::new(BufReader::new(std::fs::File::open(path).unwrap())).unwrap()
}

/// The `0x85` sample-count tag of an `SCHl` payload (`GSTR`, 4 bytes, then `tag len value…`).
fn schl_sample_count(header: &[u8]) -> Option<u32> {
    let mut at = 8;
    while at + 2 <= header.len() {
        let tag = header[at];
        at += 1;
        if tag == 0xFF {
            return None;
        }
        if (0xFC..=0xFE).contains(&tag) {
            continue;
        }
        let len = usize::from(header[at]);
        let value = header.get(at + 1..at + 1 + len)?;
        if tag == 0x85 {
            return Some(value.iter().fold(0u32, |acc, byte| (acc << 8) | u32::from(*byte)));
        }
        at += 1 + len;
    }
    None
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn all_movies_demux_as_documented() {
    let Some(files) = movies() else { return };
    assert_eq!(files.len(), 32);
    let (mut frames_total, mut audio_total) = (0u64, 0u64);
    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let mut demuxer = open(path);
        let header = *demuxer.header();
        assert!(header.is_vp6(), "{name}");
        assert_eq!((header.width, header.height), (1024, 512), "{name}");
        assert_eq!((header.rate, header.scale), (982_047, 32_767), "{name}");
        assert!((header.fps() - 29.97).abs() < 0.001, "{name}");
        let audio_header = demuxer.audio_header().expect(&name).to_vec();
        assert_eq!((&audio_header[..4], audio_header.len()), (&b"GSTR"[..], 32), "{name}");

        let (mut frames, mut keys, mut audio, mut samples) = (0u32, 0u32, 0u32, 0u64);
        let (mut max_payload, mut declared_audio, mut ended) = (0usize, None, false);
        while let Some(packet) = demuxer.next_packet().unwrap_or_else(|e| panic!("{name}: {e}")) {
            match packet {
                Packet::Video(v) => {
                    assert_eq!(v.index, frames, "{name}");
                    assert!(frames > 0 || v.key, "{name}: first frame is not a key frame");
                    // The first bit of a VP6 frame is clear for key frames and set for inter frames.
                    assert_eq!(v.data[0] & 0x80 == 0, v.key, "{name} frame {frames}");
                    keys += u32::from(v.key);
                    frames += 1;
                    max_payload = max_payload.max(v.data.len());
                }
                Packet::Audio(a) => {
                    assert_eq!(a.index, audio, "{name}");
                    assert_eq!(a.first_sample, samples, "{name}");
                    audio += 1;
                    samples += u64::from(a.samples);
                }
                Packet::AudioCount(count) => declared_audio = Some(count),
                Packet::AudioEnd => ended = true,
                Packet::Unknown { tag, .. } => panic!("{name}: unknown block {tag:?}"),
            }
        }
        assert_eq!(frames, header.frame_count, "{name}");
        assert!(keys >= 1, "{name}");
        assert_eq!(declared_audio, Some(audio), "{name}: SCCl");
        assert!(ended, "{name}: no SCEl");
        assert_eq!(Some(samples), schl_sample_count(&audio_header).map(u64::from), "{name}");
        let declared_max = header.max_frame_size as usize;
        assert!((declared_max..=declared_max + 2).contains(&max_payload), "{name}: {max_payload}");
        frames_total += u64::from(frames);
        audio_total += u64::from(audio);
    }
    println!("{} movies, {frames_total} video frames, {audio_total} audio blocks", files.len());
    assert_eq!((frames_total, audio_total), (45_435, 45_502));
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_frame_of_every_movie_decodes() {
    let Some(files) = movies() else { return };
    let started = Instant::now();
    let frames: u64 = std::thread::scope(|scope| {
        let workers: Vec<_> = files.iter().map(|path| scope.spawn(|| decode_all(path))).collect();
        workers.into_iter().map(|worker| worker.join().unwrap()).sum()
    });
    let seconds = started.elapsed().as_secs_f64();
    println!("decoded {frames} frames in {seconds:.1} s on all cores ({:.0} frames/s)", frames as f64 / seconds);
    assert_eq!(frames, 45_435);
}

fn decode_all(path: &PathBuf) -> u64 {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let mut demuxer = open(path);
    let header = *demuxer.header();
    let mut decoder = Vp6Decoder::new(header.width, header.height).unwrap();
    let mut frame = blackbox_movie::YuvFrame::default();
    let mut count = 0;
    while let Some(packet) = demuxer.next_packet().unwrap() {
        let Packet::Video(video) = packet else { continue };
        decoder.decode_into(&video.data, &mut frame).unwrap_or_else(|e| panic!("{name} frame {}: {e}", video.index));
        assert_eq!((frame.width, frame.height), (1024, 512), "{name}");
        assert_eq!((frame.y.len(), frame.u.len(), frame.v.len()), (1024 * 512, 512 * 256, 512 * 256), "{name}");
        count += 1;
    }
    assert_eq!(count, u64::from(header.frame_count), "{name}");
    count
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn decode_and_convert_run_faster_than_real_time() {
    let Some(files) = movies() else { return };
    let path = files.iter().find(|p| p.to_string_lossy().contains("blacklist_01")).unwrap();
    let mut demuxer = open(path);
    let header = *demuxer.header();
    let mut decoder = Vp6Decoder::new(header.width, header.height).unwrap();
    let mut frame = blackbox_movie::YuvFrame::default();
    let mut rgba = vec![0u8; 1024 * 512 * 4];
    let (mut frames, mut decode_time, mut convert_time) = (0u32, 0.0, 0.0);
    while let Some(packet) = demuxer.next_packet().unwrap() {
        let Packet::Video(video) = packet else { continue };
        let t = Instant::now();
        decoder.decode_into(&video.data, &mut frame).unwrap();
        decode_time += t.elapsed().as_secs_f64();
        let t = Instant::now();
        frame.to_rgba(&mut rgba);
        convert_time += t.elapsed().as_secs_f64();
        frames += 1;
    }
    let decode_fps = f64::from(frames) / decode_time;
    let total_fps = f64::from(frames) / (decode_time + convert_time);
    println!(
        "blacklist_01: {frames} frames, decode {decode_fps:.0} fps, decode + RGBA {total_fps:.0} fps (movie: 29.97)"
    );
    // The converter writes every pixel, opaque.
    assert!(rgba.iter().skip(3).step_by(4).all(|&alpha| alpha == 255));
    if cfg!(not(debug_assertions)) {
        assert!(total_fps > 2.0 * header.fps(), "{total_fps} fps");
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn timeline_covers_the_movie_duration() {
    let Some(files) = movies() else { return };
    for path in &files {
        let header = *open(path).header();
        let timeline = Timeline::from_header(&header);
        let end = header.duration();
        assert_eq!(timeline.frame_at(end - 1e-6), Some(header.frame_count - 1));
        assert!(timeline.is_finished_at(end));
        assert!(!timeline.is_finished_at(end - 0.01));
    }
}
