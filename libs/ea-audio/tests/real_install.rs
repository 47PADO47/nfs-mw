//! Against the user's own install; every test is ignored without `NFSMW_GAME_DIR`.
//!
//! Run with `cargo test --release -p ea-audio -- --ignored --nocapture`. Nothing decoded is written anywhere.

use ea_audio::{Pcm, ReadAt, abk, big, gin, mus, speech};
use std::fs::File;
use std::path::{Path, PathBuf};

fn sound_dir() -> Option<PathBuf> {
    std::env::var_os("NFSMW_GAME_DIR").map(|d| PathBuf::from(d).join("SOUND"))
}

/// A file read with positioned reads, so nothing is loaded up front.
struct FileSource {
    file: File,
    len: u64,
}

impl FileSource {
    fn open(path: &Path) -> Self {
        let file = File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let len = file.metadata().unwrap().len();
        Self { file, len }
    }
}

impl ReadAt for FileSource {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> usize {
        let mut done = 0;
        while done < buf.len() {
            #[cfg(windows)]
            let n = std::os::windows::fs::FileExt::seek_read(&self.file, &mut buf[done..], offset + done as u64);
            #[cfg(unix)]
            let n = std::os::unix::fs::FileExt::read_at(&self.file, &mut buf[done..], offset + done as u64);
            match n {
                Ok(0) | Err(_) => break,
                Ok(n) => done += n,
            }
        }
        done
    }
}

/// Aggregated sanity numbers for a set of decoded sounds.
#[derive(Default, Debug)]
struct Stats {
    sounds: usize,
    frames: u64,
    silent: usize,
    clipped_samples: u64,
    total_samples: u64,
    /// Sounds whose mean absolute sample-to-sample step exceeds 0.9 times the mean absolute level, i.e. noise-like.
    noisy: usize,
    mismatched_length: usize,
}

impl Stats {
    /// Adds a sound; returns its step-to-level ratio (about 1.4 for white noise, well under 0.5 for most audio).
    fn add(&mut self, pcm: &Pcm, expected_frames: usize) -> f64 {
        self.sounds += 1;
        self.frames += pcm.frames() as u64;
        self.total_samples += pcm.samples.len() as u64;
        if pcm.frames() != expected_frames {
            self.mismatched_length += 1;
        }
        let peak = pcm.samples.iter().map(|&s| (s as i32).abs()).max().unwrap_or(0);
        if peak == 0 {
            self.silent += 1;
            return 0.0;
        }
        self.clipped_samples += pcm.samples.iter().filter(|&&s| s == i16::MAX || s == i16::MIN).count() as u64;
        let ch = pcm.channels as usize;
        let level: f64 = pcm.samples.iter().map(|&s| (s as f64).abs()).sum::<f64>() / pcm.samples.len() as f64;
        let step: f64 = pcm.samples.windows(ch + 1).map(|w| (w[ch] as f64 - w[0] as f64).abs()).sum::<f64>()
            / pcm.samples.len().max(1) as f64;
        if pcm.samples.len() > 2000 && step > 0.9 * level {
            self.noisy += 1;
        }
        step / level
    }

    /// `clip_per_mille`: how many samples in a thousand may sit at full scale.
    fn check(&self, clip_per_mille: u64, noisy_allowed: usize) {
        println!("{self:?}");
        assert_eq!(self.mismatched_length, 0, "decoded length differs from the header's sample count");
        assert!(self.noisy <= noisy_allowed, "{} noise-like sounds: the decoder is probably wrong", self.noisy);
        assert!(
            self.clipped_samples * 1000 < self.total_samples.max(1) * clip_per_mille,
            "more than {clip_per_mille} per mille of the samples clip: {} of {}",
            self.clipped_samples,
            self.total_samples
        );
    }
}

fn find_banks(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            find_banks(&path, out);
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("abk")) {
            out.push(path);
        }
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn all_301_banks_list_and_decode_2577_sounds() {
    let Some(dir) = sound_dir() else { return };
    let mut banks = Vec::new();
    find_banks(&dir, &mut banks);
    assert_eq!(banks.len(), 301);
    let (mut stats, mut stereo, mut looped, mut referenced) = (Stats::default(), 0, 0, 0);
    for path in &banks {
        let bytes = std::fs::read(path).unwrap();
        let bank = abk::Bank::parse(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        for table in bank.sample_tables() {
            referenced += table.entries.iter().filter(|e| !e.is_dummy()).count();
        }
        for sound in bank.sounds() {
            let pcm = bank.decode(sound.index).unwrap_or_else(|e| panic!("{} #{}: {e}", path.display(), sound.index));
            stereo += (pcm.channels == 2) as usize;
            looped += pcm.loop_range.is_some() as usize;
            let ratio = stats.add(&pcm, sound.header.sample_count as usize);
            if ratio > 0.9 && pcm.samples.len() > 2000 {
                println!("noise-like {ratio:.2} {} #{}", path.file_name().unwrap().to_string_lossy(), sound.index);
            }
        }
    }
    println!("stereo {stereo}, looped {looped}, referenced by sample tables {referenced}");
    assert_eq!(stats.sounds, 2577);
    assert_eq!(referenced, 2577);
    // Collisions, turbo whooshes, helicopter and nitrous really are noise (177 of 2,577); everything else is smooth.
    assert_eq!(stereo, 40);
    stats.check(1, 200);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn music_has_3257_streams_and_all_decode() {
    let Some(dir) = sound_dir() else { return };
    let mpf_bytes = std::fs::read(dir.join("PFDATA/MW_Music.mpf")).unwrap();
    let mpf = mus::Mpf::parse(&mpf_bytes).unwrap();
    let source = FileSource::open(&dir.join("PFDATA/MW_Music.mus"));
    let mut head = [0u8; 4];
    source.read_at(0, &mut head);
    assert!(mpf.matches_mus(&head));
    assert_eq!(mpf.streams.len(), 3257);
    assert_eq!(mpf.tracks.len(), 1);
    println!("{:.1} minutes of music", mpf.total_secs() / 60.0);
    let mut stats = Stats::default();
    let mut previous_end = 0u64;
    for (i, stream) in mpf.streams.iter().enumerate() {
        let len = mpf.stream_len(&source, i).unwrap();
        assert!(stream.offset >= previous_end, "stream {i} overlaps the previous one");
        previous_end = stream.offset + len;
        let pcm = mpf.decode(&source, i).unwrap_or_else(|e| panic!("stream {i}: {e}"));
        assert_eq!(pcm.sample_rate, 36000);
        let ms = pcm.frames() as f64 * 1000.0 / pcm.sample_rate as f64;
        assert!((ms - stream.duration_ms as f64).abs() < 2.0, "stream {i}: {ms} ms vs {}", stream.duration_ms);
        stats.add(&pcm, pcm.frames());
    }
    stats.check(1, 0);
}

fn check_big(file: &str, expected_streams: usize, expected_rate_set: &[u32], clip_per_mille: u64) {
    let Some(dir) = sound_dir() else { return };
    let source = FileSource::open(&dir.join(file));
    let entries = big::scan(&source).unwrap();
    assert_eq!(entries.len(), expected_streams);
    let mut stats = Stats::default();
    for (i, entry) in entries.iter().enumerate() {
        let pcm = entry.decode(&source).unwrap_or_else(|e| panic!("{file} stream {i}: {e}"));
        assert!(expected_rate_set.contains(&pcm.sample_rate), "stream {i}: {} Hz", pcm.sample_rate);
        stats.add(&pcm, pcm.frames());
    }
    stats.check(clip_per_mille, 0);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn copspeech_has_13562_microtalk_streams() {
    // Radio speech is mastered at full scale: nearly every stream touches the rails in runs of at most a few samples.
    check_big("SPEECH/copspeech.big", 13562, &[24000], 5);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn copspeech_index_places_every_take_on_a_stream() {
    let Some(dir) = sound_dir() else { return };
    let source = FileSource::open(&dir.join("SPEECH/copspeech.big"));
    let idx = speech::SpeechIndex::parse(&std::fs::read(dir.join("SPEECH/copspeech.idx")).unwrap()).unwrap();
    assert_eq!(idx.banks().len(), 579);
    assert_eq!(idx.take_count(), 13562);
    let starts: std::collections::HashSet<u64> = big::scan(&source).unwrap().iter().map(|e| e.offset).collect();
    for (n, bank) in idx.banks().iter().enumerate() {
        assert_eq!(bank.number as usize, n);
        for take in 0..bank.header.takes() {
            let at = bank.take_offset(take).unwrap();
            assert!(starts.contains(&at), "bank {n} take {take} at {at:#x} is not a stream");
        }
    }
    // A take decodes to speech at the codec's rate.
    let pcm = idx.banks()[2].decode(&source, 0).unwrap();
    assert_eq!(pcm.sample_rate, 24000);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn nis_audio_has_142_streams() {
    check_big("STREAMS/NISAudio.big", 142, &[44100], 1);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn all_160_gin_files_decode() {
    let Some(dir) = sound_dir() else { return };
    let mut stats = Stats::default();
    let mut files = 0;
    for entry in std::fs::read_dir(dir.join("ENGINE")).unwrap() {
        let path = entry.unwrap().path();
        if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case("gin")) {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let declared = u32::from_le_bytes(bytes[0x18..0x1C].try_into().unwrap()) as usize;
        let pcm = gin::decode(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        stats.add(&pcm, declared);
        files += 1;
    }
    assert_eq!(files, 160);
    stats.check(1, 0);
}
