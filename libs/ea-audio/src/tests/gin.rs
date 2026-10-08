use crate::error::Error;
use crate::gin;

/// A `Gnsu` file with the given table sizes and sample count, followed by `frames` EA-XAS frames.
fn file(segments: u32, cycles: u32, samples: u32, frames: usize) -> Vec<u8> {
    let mut v = b"Gnsu20\0\0".to_vec();
    v.extend(1000f32.to_le_bytes());
    v.extend(5000f32.to_le_bytes());
    for word in [segments, cycles, samples, 32000] {
        v.extend(word.to_le_bytes());
    }
    v.resize(0x20 + 4 * (segments as usize + 1) + 4 * (cycles as usize + 1), 0xAB);
    for _ in 0..frames {
        let mut frame = [0u8; 0x13];
        frame[..4].copy_from_slice(&0x0640_0120u32.to_le_bytes());
        v.extend(frame);
    }
    v
}

#[test]
fn the_audio_starts_after_the_two_position_tables() {
    let bytes = file(2, 3, 64, 2);
    let pcm = gin::decode(&bytes).unwrap();
    assert_eq!((pcm.sample_rate, pcm.channels, pcm.frames()), (32000, 1, 64));
    assert_eq!((pcm.samples[0], pcm.samples[1]), (0x0120, 0x0640));
    assert_eq!(pcm.loop_range, None);
    assert_eq!(crate::decode(&bytes).unwrap(), pcm);
}

#[test]
fn damaged_files_are_errors() {
    assert!(matches!(gin::decode(b"Gnsx0000"), Err(Error::BadMagic { .. })));
    assert!(gin::decode(&file(2, 3, 64, 2)[..0x2F]).is_err());
    assert!(matches!(gin::decode(&file(2, 3, 64, 1)), Err(Error::Truncated { .. })));
    let mut huge = file(2, 3, 64, 2);
    huge[0x10..0x14].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(gin::decode(&huge).is_err());
}
