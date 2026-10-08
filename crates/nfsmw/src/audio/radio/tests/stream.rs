use std::sync::mpsc::{Sender, channel};

use kira::Frame;
use kira::info::{Info, MockInfoBuilder};
use kira::sound::{Sound, SoundData};

use super::super::stream::{Block, StreamData, StreamHandle};

/// A stream of 100 Hz-ish ramp: frame `i` is `(i, -i) / 1000`, in blocks of `block` frames.
fn ramp(total: usize, block: usize) -> Vec<Block> {
    (0..total)
        .collect::<Vec<_>>()
        .chunks(block)
        .map(|c| c.iter().map(|&i| Frame::new(i as f32 / 1000.0, -(i as f32) / 1000.0)).collect())
        .collect()
}

fn open(rate: u32) -> (Sender<Block>, Box<dyn Sound>, StreamHandle) {
    let (tx, blocks) = channel();
    let (sound, handle) = StreamData { blocks, sample_rate: rate }.into_sound().unwrap();
    (tx, sound, handle)
}

fn render(sound: &mut dyn Sound, frames: usize, out_rate: f64) -> Vec<Frame> {
    let info: Info = MockInfoBuilder::new().build();
    let mut out = vec![Frame::ZERO; frames];
    sound.process(&mut out, 1.0 / out_rate, &info);
    out
}

#[test]
fn blocks_play_in_order_without_gaps_at_the_same_rate() {
    let (tx, mut sound, handle) = open(1000);
    for block in ramp(100, 30) {
        tx.send(block).unwrap();
    }
    drop(tx);
    let out = render(sound.as_mut(), 120, 1000.0);
    // The first output frame is the fade-in from silence; after that the ramp comes out one frame behind.
    let left: Vec<f32> = out.iter().map(|f| f.left).collect();
    assert!(left.iter().all(|v| v.is_finite()));
    let steps: Vec<f32> = left.windows(2).take(95).map(|w| w[1] - w[0]).collect();
    assert!(steps.iter().all(|s| (s - 0.001).abs() < 1e-5), "{steps:?}");
    assert!(sound.finished() && handle.finished());
    assert!((handle.position_secs() - 0.1).abs() < 1e-6, "{}", handle.position_secs());
}

#[test]
fn resampling_keeps_the_duration_and_the_ramp() {
    // 36 kHz into 48 kHz: 3600 source frames last 0.1 s and become 4800 output frames.
    let (tx, mut sound, handle) = open(36_000);
    for block in ramp(3600, 700) {
        tx.send(block).unwrap();
    }
    drop(tx);
    let out = render(sound.as_mut(), 6000, 48_000.0);
    let live = out.iter().rposition(|f| f.left != 0.0).unwrap() + 1;
    assert!((live as i32 - 4800).abs() <= 3, "{live} frames of sound");
    // Linear interpolation of a ramp is a ramp: a constant step of 0.75 source frames per output frame.
    let steps: Vec<f32> = out[2..4790].windows(2).map(|w| w[1].left - w[0].left).collect();
    let expected = 0.75 / 1000.0;
    assert!(steps.iter().all(|s| (s - expected).abs() < 1e-5), "steps are not constant");
    assert!(handle.finished());
}

#[test]
fn a_late_block_waits_instead_of_skipping_ahead() {
    let (tx, mut sound, handle) = open(1000);
    let mut blocks = ramp(40, 20).into_iter();
    tx.send(blocks.next().unwrap()).unwrap();
    // 50 frames wanted but only 20 exist: the rest is silence and the sound is not finished.
    let first = render(sound.as_mut(), 50, 1000.0);
    assert!(first[30..].iter().all(|f| *f == Frame::ZERO));
    assert!(!sound.finished());
    assert!(handle.underruns() > 0);
    let before = handle.position_secs();
    tx.send(blocks.next().unwrap()).unwrap();
    let second = render(sound.as_mut(), 30, 1000.0);
    // The stream continues with the frame after the 20th; nothing was skipped while waiting.
    assert!((second[1].left - second[0].left - 0.001).abs() < 1e-5);
    assert!(second[0].left > first[18].left, "continues where it stopped");
    assert!(handle.position_secs() > before);
    drop(tx);
    render(sound.as_mut(), 60, 1000.0);
    assert!(sound.finished());
}

#[test]
fn dropping_the_handle_fades_the_sound_out_and_ends_it() {
    let (tx, mut sound, handle) = open(1000);
    for _ in 0..50 {
        tx.send(vec![Frame::new(0.5, 0.5); 100]).unwrap();
    }
    let before = render(sound.as_mut(), 500, 1000.0);
    assert!((before[499].left - 0.5).abs() < 1e-6);
    drop(handle);
    let out = render(sound.as_mut(), 200, 1000.0);
    // About 80 ms to silence at 1000 Hz: the level falls every frame, then it is exactly zero and the sound ends.
    assert!(out.windows(2).take(75).all(|w| w[1].left < w[0].left));
    assert!(out[0].left < 0.5 && out[0].left > 0.4);
    assert!(out[85..].iter().all(|f| *f == Frame::ZERO));
    assert!(sound.finished());
    drop(tx);
}

#[test]
fn a_dropped_sender_with_no_blocks_ends_at_once() {
    let (tx, mut sound, handle) = open(36_000);
    drop(tx);
    let out = render(sound.as_mut(), 10, 48_000.0);
    assert!(out.iter().all(|f| *f == Frame::ZERO));
    assert!(sound.finished() && handle.finished());
    assert_eq!(handle.position_secs(), 0.0);
}
