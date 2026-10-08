use crate::codec::xa::{self, Revision, XaState};
use crate::codec::xas;
use crate::error::Error;
use crate::tests::build::*;

#[test]
fn xa_zero_frame_is_silent_and_takes_15_bytes() {
    let mut out = [1i16; 28];
    let used = XaState::new().decode_frame(&[0u8; 15], Revision::V2, &mut out).unwrap();
    assert_eq!(used, 15);
    assert_eq!(out, [0; 28]);
}

#[test]
fn xa_decodes_nibbles_with_the_first_filter() {
    // info 0x10: coefficient pair (240, 0), shift 8. Nibble 7 then 0xF (-1).
    let mut frame = vec![0x10, 0x7F];
    frame.extend([0u8; 13]);
    let mut out = [0i16; 28];
    XaState::new().decode_frame(&frame, Revision::V2, &mut out).unwrap();
    assert_eq!(out[0], 28672); // (7 << 20) >> 8
    assert_eq!(out[1], 22784); // ((-1 << 20) + 240 * 28672) >> 8
}

#[test]
fn xa_revision_1_adds_a_rounding_term() {
    // History h1 = 1, pair 1 (240, 0), zero nibbles: 240 >> 8 = 0 without the term, (240 + 128) >> 8 = 1 with it.
    let prime = xa_pcm_frame(1, 0, &[0; 28]);
    let frame = xa_adpcm_frame(0x10, 0x0);
    let mut out = [0i16; 28];
    let mut v1 = XaState::new();
    let mut v2 = XaState::new();
    v1.decode_frame(&prime, Revision::V2, &mut out).unwrap();
    v2.decode_frame(&prime, Revision::V2, &mut out).unwrap();
    v1.decode_frame(&frame, Revision::V1, &mut out).unwrap();
    assert_eq!(out[0], 1);
    v2.decode_frame(&frame, Revision::V2, &mut out).unwrap();
    assert_eq!(out[0], 0);
}

#[test]
fn xa_pcm_frame_copies_samples_and_sets_the_history() {
    let samples = ramp(100);
    let mut data = xa_pcm_frame(500, 400, &samples);
    // Follow with an ADPCM frame using filter pair (240, 0) and a zero nibble: output = 240 * h1 >> 8.
    data.extend(xa_adpcm_frame(0x10, 0x0));
    let mut state = XaState::new();
    let mut out = [0i16; 28];
    let used = state.decode_frame(&data, Revision::V2, &mut out).unwrap();
    assert_eq!(used, 61);
    assert_eq!(out, samples);
    // The history comes from the frame header (500), not from the last sample.
    state.decode_frame(&data[61..], Revision::V2, &mut out).unwrap();
    assert_eq!(out[0] as i32, (240 * 500) >> 8);
}

#[test]
fn xa_revision_1_has_no_pcm_frames() {
    let data = xa_pcm_frame(0, 0, &ramp(0));
    let mut out = [0i16; 28];
    assert_eq!(XaState::new().decode_frame(&data, Revision::V1, &mut out).unwrap(), 15);
}

#[test]
fn xa_short_input_is_an_error() {
    let mut out = [0i16; 28];
    let adpcm = XaState::new().decode_frame(&[0x10; 14], Revision::V2, &mut out);
    assert!(matches!(adpcm, Err(Error::Truncated { .. })));
    let pcm = xa_pcm_frame(0, 0, &ramp(0));
    let cut = XaState::new().decode_frame(&pcm[..60], Revision::V2, &mut out);
    assert!(matches!(cut, Err(Error::Truncated { .. })));
    assert!(XaState::new().decode_frame(&[], Revision::V2, &mut out).is_err());
}

#[test]
fn xa_run_trims_the_last_frame_and_reports_bytes() {
    let mut data = xa_pcm_frame(0, 0, &ramp(0));
    data.extend(xa_adpcm_frame(0, 0));
    data.extend(xa_adpcm_frame(0, 0));
    let mut out = Vec::new();
    let used = XaState::new().decode_run(&data, Revision::V2, 28 + 28 + 5, &mut out).unwrap();
    assert_eq!(used, 61 + 15 + 15);
    assert_eq!(out.len(), 61);
    assert_eq!(&out[..28], &ramp(0));
    assert!(XaState::new().decode_run(&data[..70], Revision::V2, 84, &mut Vec::new()).is_err());
}

#[test]
fn xa_stereo_frame_splits_nibbles_between_channels() {
    // Left: pair 1 (240, 0), shift 8. Right: pair 0, shift 8. Every byte 0x7F: left 7, right -1.
    let mut frame = vec![0x10, 0x00];
    frame.extend([0x7F; 28]);
    let (mut l, mut r) = (xa::XaState::new(), xa::XaState::new());
    let (mut ol, mut or) = ([0i16; 28], [0i16; 28]);
    let used = xa::decode_stereo_frame(&mut l, &mut r, &frame, &mut ol, &mut or).unwrap();
    assert_eq!(used, 30);
    assert_eq!(ol[0] as i32, (7 * (1i32 << 20) + 128) >> 8);
    assert_eq!(or[0] as i32, (-(1i32 << 20) + 128) >> 8);
    assert_eq!(or[1], or[0]);
    assert!(ol[1] > ol[0], "the left channel accumulates through its filter");
    assert!(xa::decode_stereo_frame(&mut l, &mut r, &frame[..29], &mut ol, &mut or).is_err());
}

fn xas_frame(header: u32, nibbles: u8) -> [u8; xas::FRAME_BYTES] {
    let mut frame = [nibbles; xas::FRAME_BYTES];
    frame[..4].copy_from_slice(&header.to_le_bytes());
    frame
}

#[test]
fn xas_frame_starts_with_the_two_history_samples() {
    // Coefficient pair 0, hist2 = 0x1230, hist1 = 0x4560, shift 2; nibbles 1 then 0xF.
    let mut frame = xas_frame(0x4562_1230, 0x00);
    frame[4] = 0x1F;
    let mut out = [0i16; 32];
    xas::decode_frame(&frame, &mut out);
    assert_eq!(out[0], 0x1230);
    assert_eq!(out[1], 0x4560);
    assert_eq!(out[2], 1024); // (1 << 12) >> 2
    assert_eq!(out[3], -1024); // (0xF << 12) as i16 >> 2
}

#[test]
fn xas_applies_the_cd_xa_filter() {
    // Pair 1: 0.9375 * h1 + 0 * h2, zero nibbles.
    let frame = xas_frame(0x4560_1231, 0x00);
    let mut out = [0i16; 32];
    xas::decode_frame(&frame, &mut out);
    assert_eq!(out[2], (0x4560 as f32 * 0.9375) as i16);
    assert_eq!(out[3], (out[2] as f32 * 0.9375) as i16);
    // Pair 2 uses both history samples.
    let frame = xas_frame(0x4560_1232, 0x00);
    xas::decode_frame(&frame, &mut out);
    assert_eq!(out[2], (0x4560 as f32 * 1.796875 + 0x1230 as f32 * -0.8125) as i16);
}

#[test]
fn xas_clamps_to_16_bits() {
    // Pair 3 (1.53125, -0.859375) on a loud history with the largest positive nibble.
    let frame = xas_frame(0x7FF0_7FF3, 0x77);
    let mut out = [0i16; 32];
    xas::decode_frame(&frame, &mut out);
    assert_eq!(out[2], i16::MAX);
}

#[test]
fn xas_unknown_filter_index_means_no_filter() {
    let frame = xas_frame(0x4560_123F, 0x00);
    let mut out = [0i16; 32];
    xas::decode_frame(&frame, &mut out);
    assert_eq!(out[2], 0);
}

#[test]
fn xas_run_decodes_consecutive_frames_and_trims() {
    let mut data = Vec::new();
    data.extend(xas_frame(0x0010_0020, 0x00));
    data.extend(xas_frame(0x0030_0040, 0x00));
    let pcm = xas::decode_v0(&data, 40).unwrap();
    assert_eq!(pcm.len(), 40);
    assert_eq!((pcm[0], pcm[1]), (0x20, 0x10));
    assert_eq!((pcm[32], pcm[33]), (0x40, 0x30));
    assert!(matches!(xas::decode_v0(&data[..30], 40), Err(Error::Truncated { .. })));
    assert!(xas::decode_v0(&[], 0).unwrap().is_empty());
}
