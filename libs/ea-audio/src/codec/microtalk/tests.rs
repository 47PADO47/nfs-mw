use super::bits::BitReader;
use super::*;

#[test]
fn bit_reader_reads_low_bits_first_and_looks_one_byte_ahead() {
    let mut br = BitReader::new(&[0b1010_0110, 0xFF, 0x00]);
    br.init();
    assert_eq!(br.peek(3), 0b110);
    assert_eq!(br.read(3), 0b110);
    assert_eq!(br.read(5), 0b10100);
    assert_eq!(br.read(8), 0xFF);
    assert_eq!(br.read(8), 0x00);
    // Past the end reads as zero.
    assert_eq!(br.read(8), 0);
}

#[test]
fn release_lookahead_steps_back_one_byte() {
    let mut br = BitReader::new(&[1, 2, 3, 4]);
    br.init();
    br.read(8); // consumes byte 0, loads byte 1 as lookahead
    br.release_lookahead();
    assert_eq!(br.read_byte(), 2);
}

#[test]
fn silent_frames_decode_to_finite_samples() {
    let mut decoder = Decoder::new(false);
    let mut out = Vec::new();
    decoder.decode_channel(&[0u8; 200], 3 * FRAME_SAMPLES + 5, &mut out).unwrap();
    assert_eq!(out.len(), 3 * FRAME_SAMPLES + 5);
    assert!(decoder.samples().iter().all(|s| s.is_finite()));
}

/// Bytes the main part of an all-zero frame occupies once the one-byte lookahead is given back.
fn zero_frame_len() -> usize {
    let mut decoder = Decoder::new(true);
    let mut br = BitReader::new(&[0u8; 400]);
    decoder.decode_main(&mut br);
    br.release_lookahead();
    br.position()
}

#[test]
fn pcm_patch_overwrites_samples_and_the_next_frame_follows_it() {
    let len = zero_frame_len();
    // Frame 1: marker, main frame, patch (offset 10, two samples). Frame 2: plain marker and a zero frame.
    let mut data = vec![PCM_MARKER];
    data.extend(vec![0u8; len]);
    data.extend(10i16.to_be_bytes());
    data.extend(2i16.to_be_bytes());
    data.extend(1000i16.to_be_bytes());
    data.extend((-2000i16).to_be_bytes());
    data.push(0x00);
    data.extend(vec![0u8; len]);
    let mut decoder = Decoder::new(true);
    let mut br = BitReader::new(&data);
    decoder.decode_frame(&mut br).unwrap();
    assert_eq!(decoder.samples()[9], 0.0);
    assert_eq!(decoder.samples()[10], 1000.0);
    assert_eq!(decoder.samples()[11], -2000.0);
    assert_eq!(decoder.samples()[12], 0.0);
    decoder.decode_frame(&mut br).unwrap();
    // The second frame is silent apart from the tail of the first one's synthesis memory.
    assert!(decoder.samples().iter().all(|s| s.is_finite()));
    // The second frame carries no stream header (15 bits), so its main part is two bytes shorter.
    assert_eq!(br.position(), 1 + len + 8 + 1 + (len - 2), "both frames consumed exactly their bytes");
}

#[test]
fn pcm_patch_bounds_are_checked() {
    let len = zero_frame_len();
    let patch = |offset: i16, count: i16| {
        let mut data = vec![PCM_MARKER];
        data.extend(vec![0u8; len]);
        data.extend(offset.to_be_bytes());
        data.extend(count.to_be_bytes());
        data.extend(vec![0u8; 900]);
        Decoder::new(true).decode_frame(&mut BitReader::new(&data))
    };
    assert!(patch(0, 432).is_ok());
    assert!(patch(432, 0).is_ok());
    assert_eq!(patch(433, 0), Err(Error::Corrupt("MicroTalk PCM patch offset")));
    assert_eq!(patch(-1, 0), Err(Error::Corrupt("MicroTalk PCM patch offset")));
    assert_eq!(patch(400, 33), Err(Error::Corrupt("MicroTalk PCM patch length")));
    assert_eq!(patch(0, -1), Err(Error::Corrupt("MicroTalk PCM patch length")));
}

#[test]
fn the_original_revision_has_no_marker_byte() {
    // Without PCM blocks the frame starts at the first byte; an all-zero stream decodes the same either way.
    let mut with_marker = Decoder::new(true);
    let mut without = Decoder::new(false);
    let mut a = Vec::new();
    let mut b = Vec::new();
    with_marker.decode_channel(&[0u8; 300], 432, &mut a).unwrap();
    without.decode_channel(&[0u8; 300], 432, &mut b).unwrap();
    assert_eq!(a, b);
}

#[test]
fn nonzero_data_produces_a_signal_and_reset_forgets_it() {
    let data: Vec<u8> = (0..400u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
    let mut decoder = Decoder::new(false);
    let mut first = Vec::new();
    decoder.decode_channel(&data, 2 * FRAME_SAMPLES, &mut first).unwrap();
    assert!(first.iter().any(|&s| s != 0), "pseudo-random bits must not decode to silence");
    decoder.reset();
    let mut again = Vec::new();
    decoder.decode_channel(&data, 2 * FRAME_SAMPLES, &mut again).unwrap();
    assert_eq!(first, again, "a reset decoder is deterministic");
}

#[test]
fn rounding_is_half_away_from_zero_and_clamped() {
    assert_eq!(to_i16(0.4), 0);
    assert_eq!(to_i16(0.5), 1);
    assert_eq!(to_i16(-0.5), -1);
    assert_eq!(to_i16(-2.6), -3);
    assert_eq!(to_i16(1e9), i16::MAX);
    assert_eq!(to_i16(-1e9), i16::MIN);
}

#[test]
fn the_lpc_conversion_of_zero_coefficients_is_zero() {
    assert_eq!(super::synth::rc_to_lpc(&[0.0; 12]), [0.0; 12]);
    let lpc = super::synth::rc_to_lpc(&[0.5; 12]);
    assert!(lpc.iter().all(|c| c.is_finite()) && lpc.iter().any(|&c| c != 0.0));
}

#[test]
fn the_synthesis_filter_with_no_coefficients_passes_samples_through() {
    let mut history = [0.0; 12];
    let mut samples: Vec<f32> = (0..24).map(|i| i as f32).collect();
    super::synth::filter(&[0.0; 12], &mut history, &mut samples);
    assert_eq!(samples, (0..24).map(|i| i as f32).collect::<Vec<_>>());
    assert_eq!(history[0], 23.0, "the newest sample sits at the front of the history");
}
