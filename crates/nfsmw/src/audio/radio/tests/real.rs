//! Against the user's own install: songs from the attribute database, chains from the music map, and the stream
//! file, through the same decoder thread and player the game uses, rendered offline at 48 kHz.
//!
//! `NFSMW_GAME_DIR="D:/..." cargo test --release -p nfsmw radio::tests::real -- --ignored --nocapture`

use std::sync::mpsc::channel;

use blackbox_attrib::Database;
use game_install::GameDir;
use kira::Frame;
use kira::info::MockInfoBuilder;
use kira::sound::SoundData;
use nfsmw_data::music::{Song, songs};

use super::super::backend::{Backend, Files};
use super::super::stream::StreamData;

const OUT_RATE: f64 = 48_000.0;

fn install() -> Option<GameDir> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    Some(GameDir::open(std::path::PathBuf::from(dir)).expect("NFSMW_GAME_DIR cannot be indexed"))
}

fn load() -> Option<(Vec<Song>, Files)> {
    let dir = install()?;
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let files = Files::new(
        &dir.read("SOUND/PFDATA/MW_Music.mpf").unwrap(),
        dir.resolve("SOUND/PFDATA/MW_Music.mus").unwrap().to_path_buf(),
    )
    .unwrap();
    Some((songs(&db), files))
}

/// What the player output for a song, how long its chain says it is, how much of the stream it played, and where
/// the streams join (ms into the song).
type Rendered = (Vec<Frame>, f64, f64, Vec<u64>);

/// Everything the player would output for a song.
fn render(files: &Files, event: u32) -> Rendered {
    let chain = files.chain(event).unwrap();
    let started = files.open(&chain).unwrap();
    // Let the decoder finish first so the render is not paced by it, then play the blocks back.
    let blocks: Vec<_> = started.blocks.iter().collect();
    let (tx, rx) = channel();
    for block in blocks {
        tx.send(block).unwrap();
    }
    drop(tx);
    let (mut sound, handle) = StreamData { blocks: rx, sample_rate: started.sample_rate }.into_sound().unwrap();
    let info = MockInfoBuilder::new().build();
    let mut out = Vec::new();
    while !sound.finished() {
        let mut block = vec![Frame::ZERO; 4800];
        sound.process(&mut block, 1.0 / OUT_RATE, &info);
        out.extend(block);
    }
    assert_eq!(handle.underruns(), 0, "the blocks were all queued");
    let joins = chain.segments.iter().skip(1).map(|s| s.start_ms).collect();
    (out, chain.total_secs(), f64::from(handle.position_secs()), joins)
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_installs_songs_all_have_chains() {
    let Some((songs, files)) = load() else { return };
    assert_eq!(songs.len(), 26);
    for (n, song) in songs.iter().enumerate() {
        let chain = files.chain(song.event).unwrap_or_else(|e| panic!("song {n} {}: {e}", song.title));
        assert!(chain.segments.len() >= 39 && chain.total_secs() > 170.0, "song {n}");
    }
}

/// The shortest song, streamed through the player: its length, no NaN, nothing louder than full scale, and a join
/// between two streams no steeper than the music itself.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn a_whole_song_plays_to_its_end_without_a_break() {
    let Some((songs, files)) = load() else { return };
    let (n, song) = songs.iter().enumerate().find(|(_, s)| s.title == "Let's Move").unwrap();
    let (out, secs, played, joins) = render(&files, song.event);
    let audible = out.iter().rposition(|f| f.left != 0.0 || f.right != 0.0).unwrap() as f64 / OUT_RATE;
    println!(
        "song {n} {}: {played:.2} s played, {audible:.2} s until the last sound, chain {secs:.2} s, {} joins",
        song.title,
        joins.len()
    );
    // Every source frame was played (the stored lengths round to the millisecond per stream), and the song ends in
    // silence: the streams fade out, so the last sound comes up to a few seconds before the end.
    assert!((played - secs).abs() < 0.002 * joins.len() as f64 + 0.01, "{played} s against {secs} s");
    assert!(audible <= secs + 0.1 && audible > secs - 4.0, "{audible} s of sound against {secs} s");
    assert!(out.len() as f64 / OUT_RATE >= played - 0.01);
    assert!(
        out.iter().all(|f| f.left.is_finite() && f.right.is_finite() && f.left.abs() <= 1.0 && f.right.abs() <= 1.0)
    );
    let peak = out.iter().map(|f| f.left.abs().max(f.right.abs())).fold(0.0, f32::max);
    assert!(peak > 0.05, "peak {peak}: the song is silent");
    // The joins themselves are checked on the decoded samples in `ea-audio`'s `real_graph` test (the largest step
    // across a join is 96 of 32,768); here the stored lengths round to the millisecond, so a join cannot be found
    // in the output to the sample. Silence is allowed: the last bar of a song ends in exact zeros.
    let first = out.iter().position(|f| f.left != 0.0 || f.right != 0.0).unwrap();
    let last = out.iter().rposition(|f| f.left != 0.0 || f.right != 0.0).unwrap();
    let mut longest = (0, 0);
    let mut at = first;
    for run in out[first..last].chunk_by(|a, b| (*a == Frame::ZERO) == (*b == Frame::ZERO)) {
        if run[0] == Frame::ZERO && run.len() > longest.0 {
            longest = (run.len(), at);
        }
        at += run.len();
    }
    println!(
        "peak {peak:.3}; longest exact silence {:.2} s at {:.2} s",
        longest.0 as f64 / OUT_RATE,
        longest.1 as f64 / OUT_RATE
    );
    // Before the last bars there is no silence: a gap of 100 ms or more in the body of the song would be a defect.
    assert!(longest.1 as f64 / OUT_RATE > secs - 12.0 || longest.0 < 4800, "a gap in the body of the song");
}

/// Every song renders to its length. Slow without `--release`.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR); slow without --release"]
fn all_songs_play_to_their_length() {
    let Some((songs, files)) = load() else { return };
    for (n, song) in songs.iter().enumerate() {
        let (out, secs, played, joins) = render(&files, song.event);
        assert!((played - secs).abs() < 0.002 * joins.len() as f64 + 0.01, "song {n}: {played} s against {secs} s");
        assert!(out.iter().all(|f| f.left.is_finite() && f.right.is_finite()), "song {n}");
        println!("song {n:2} {:.1} s {} - {}", played, song.artist, song.title);
    }
}
