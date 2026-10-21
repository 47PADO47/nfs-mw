//! Unit tests of the wire codec and the bookkeeping types: header, varints, frame prefix, sequence buffer, receive window,
//! RTT, fragment limits.

use crate::conn::acks::{RecvWindow, SequenceBuffer, seq_newer};
use crate::conn::fragment::{declared_total, fragment_count, fragment_span};
use crate::conn::header::{
    Header, frame_size, parse_header, put_varint, read_varint, varint_len, write_frame, write_header,
};
use crate::conn::rtt::Rtt;
use crate::conn::{ConnError, FRAME_SPACE, MAX_FRAGMENT, MAX_MESSAGE};

#[test]
fn header_round_trips_with_little_endian_fields() {
    let header = Header { flags: 1, conn_id: 0xBEEF, number: 0x0102_0304, ack: 77, ack_bits: 0xF0F0_0F0F };
    let mut out = vec![0u8; 1200];
    write_header(&mut out, &header).expect("header fits");
    assert_eq!(out[0], 1, "flags byte");
    assert_eq!(&out[1..3], &[0xEF, 0xBE], "connection id is little-endian");
    assert_eq!(&out[3..7], &[0x04, 0x03, 0x02, 0x01], "packet number is little-endian");
    assert_eq!(parse_header(&out).expect("parses"), header);
}

#[test]
fn header_needs_fifteen_bytes() {
    assert_eq!(parse_header(&[0u8; 14]), Err(ConnError::Truncated));
    let mut small = [0u8; 14];
    assert_eq!(
        write_header(&mut small, &Header { flags: 0, conn_id: 0, number: 0, ack: 0, ack_bits: 0 }),
        Err(ConnError::BufferTooSmall)
    );
}

#[test]
fn varint_round_trips_and_is_canonical() {
    for value in [0usize, 1, 127, 128, 16_383, 16_384, 2_097_151] {
        let mut out = [0u8; 3];
        let written = put_varint(&mut out, value).expect("fits");
        assert_eq!(written, varint_len(value), "length of {value}");
        assert_eq!(read_varint(&out[..written]), Ok((value, written)), "round trip of {value}");
    }
    assert_eq!(varint_len(127), 1);
    assert_eq!(varint_len(128), 2);
    let mut out = [0u8; 2];
    assert_eq!(put_varint(&mut out, 128), Some(2));
    assert_eq!(out, [0x80, 0x01], "low group first, top bit = more");
}

#[test]
fn varint_refuses_non_canonical_and_overlong_forms() {
    assert_eq!(read_varint(&[0x00]), Ok((0, 1)), "the single byte 0 is canonical");
    assert_eq!(read_varint(&[0x80, 0x00]), Err(ConnError::BadFrame), "trailing zero group");
    assert_eq!(read_varint(&[0x81, 0x80, 0x00]), Err(ConnError::BadFrame), "trailing zero group, three bytes");
    assert_eq!(read_varint(&[0x80, 0x80, 0x80, 0x01]), Err(ConnError::BadFrame), "fourth byte");
    assert_eq!(read_varint(&[0x80]), Err(ConnError::BadFrame), "cut short");
    assert_eq!(read_varint(&[]), Err(ConnError::BadFrame), "empty");
    assert_eq!(read_varint(&[0xFF, 0xFF, 0x7F]), Ok((2_097_151, 3)), "largest three-byte value");
}

#[test]
fn frame_is_type_length_then_parts() {
    let mut out = [0u8; 16];
    let end = write_frame(&mut out, 1, 0x33, &[&[1, 2], &[3]]).expect("fits");
    assert_eq!(end, 1 + frame_size(3));
    assert_eq!(&out[1..end], &[0x33, 3, 1, 2, 3]);
    assert_eq!(write_frame(&mut out[..3], 0, 0x33, &[&[1, 2, 3]]), None, "does not fit");
}

#[test]
fn sequence_buffer_keys_wrap_and_slots_are_checked() {
    let mut buf: SequenceBuffer<char> = SequenceBuffer::with_capacity(4);
    assert_eq!(buf.insert(u32::MAX - 1, 'a'), None);
    assert_eq!(buf.insert(u32::MAX, 'b'), None);
    assert_eq!(buf.insert(0, 'c'), None, "0 wraps after u32::MAX and takes its own slot");
    assert_eq!(buf.insert(4, 'd'), Some((0, 'c')), "4 shares slot 0 with 0 and replaces it");
    assert_eq!(buf.remove(0), None, "0 was replaced");
    assert_eq!(buf.remove(4), Some('d'));
    assert_eq!(buf.remove(4), None, "removed once");
    assert_eq!(buf.remove(u32::MAX), Some('b'));
    assert_eq!(buf.remove(u32::MAX - 1), Some('a'));
}

#[test]
fn sequence_buffer_drains_matching_entries_only() {
    let mut buf: SequenceBuffer<u32> = SequenceBuffer::with_capacity(8);
    for seq in 1..=3 {
        buf.insert(seq, seq * 10);
    }
    let mut taken = Vec::new();
    buf.drain_matching(|seq, _| seq >= 2, |seq, value| taken.push((seq, value)));
    assert_eq!(taken, vec![(2, 20), (3, 30)]);
    assert_eq!(buf.remove(1), Some(10), "the entry that did not match stays");
}

#[test]
fn seq_newer_is_wrap_aware() {
    assert!(seq_newer(1, 0));
    assert!(!seq_newer(0, 1));
    assert!(!seq_newer(5, 5));
    assert!(seq_newer(0, u32::MAX), "0 is newer than u32::MAX");
    assert!(!seq_newer(u32::MAX, 0));
}

#[test]
fn receive_window_builds_ack_bits_and_refuses_duplicates() {
    let mut window = RecvWindow::default();
    assert!(!window.any());
    window.record(5).expect("first packet");
    assert_eq!((window.ack(), window.ack_bits()), (5, 0));
    window.record(3).expect("older packet inside the window");
    assert_eq!(window.ack_bits(), 0b10, "3 is 5 - 1 - 1");
    window.record(4).expect("another older packet");
    assert_eq!(window.ack_bits(), 0b11);
    assert_eq!(window.record(3), Err(ConnError::Duplicate));
    assert_eq!(window.record(5), Err(ConnError::Duplicate));
    window.record(9).expect("newer packet");
    assert_eq!((window.ack(), window.ack_bits()), (9, 0b11_1000), "5, 4 and 3 are 3, 4 and 5 below 9 - 1");
}

#[test]
fn receive_window_shifts_across_words_and_drops_old_numbers() {
    let mut window = RecvWindow::default();
    window.record(10).expect("first");
    window.record(10 + 1024).expect("jump by the whole window");
    assert_eq!(window.record(10), Err(ConnError::TooOld), "1024 behind the newest");
    window.record(11).expect("1023 behind is still inside");

    let mut window = RecvWindow::default();
    window.record(100).expect("first");
    window.record(164).expect("shift by one word");
    assert_eq!(window.record(100), Err(ConnError::Duplicate), "the mark moved with the window");
    window.record(99).expect("99 was never received");
    window.record(164 + 100).expect("shift by more than a word");
    assert_eq!(window.record(164), Err(ConnError::Duplicate));
}

#[test]
fn rtt_first_sample_sets_both_and_loss_moves_by_a_sixteenth() {
    let mut rtt = Rtt::default();
    assert_eq!(rtt.srtt_ms(), 100.0, "initial estimate");
    rtt.sample(40.0);
    assert_eq!((rtt.srtt_ms(), rtt.rttvar_ms()), (40.0, 20.0));
    rtt.sample(40.0);
    assert_eq!(rtt.srtt_ms(), 40.0, "equal samples keep the mean");
    assert!((rtt.rttvar_ms() - 15.0).abs() < 1e-9, "variance decays by 3/4");
    assert_eq!(rtt.resend_after_ms(), 60.0, "1.5 * srtt");
    let mut quick = Rtt::default();
    quick.sample(4.0);
    assert_eq!(quick.resend_after_ms(), 20.0, "never below 20 ms");

    assert_eq!(rtt.loss(), 0.0);
    rtt.record_outcome(true);
    assert!((rtt.loss() - 1.0 / 16.0).abs() < 1e-12);
    rtt.record_outcome(false);
    assert!((rtt.loss() - (1.0 / 16.0) * (15.0 / 16.0)).abs() < 1e-12);
}

#[test]
fn fragment_counts_and_spans_hold_the_limits() {
    assert_eq!(fragment_count(0), Some(1), "empty message is one fragment");
    assert_eq!(fragment_count(MAX_FRAGMENT), Some(1));
    assert_eq!(fragment_count(MAX_FRAGMENT + 1), Some(2));
    assert_eq!(fragment_count(16_382), Some(16), "the largest message that fits");
    assert_eq!(fragment_count(16_383), None, "needs 17 fragments");
    assert_eq!(fragment_count(MAX_MESSAGE), Some(16));
    assert_eq!(fragment_count(MAX_MESSAGE + 1), None);
    assert_eq!(fragment_span(1025, 0), (0, 1022), "first chunk leaves room for the length");
    assert_eq!(fragment_span(1025, 1), (1022, 1025));
    assert_eq!(fragment_span(16_382, 15), (15_358, 16_382));
    assert_eq!(fragment_span(10, 0), (0, 10), "single fragment is the whole message");
}

#[test]
fn declared_total_must_need_a_split() {
    assert_eq!(declared_total(&[0x00, 0x04]), Err(ConnError::BadFrame), "1024 would not be split");
    assert_eq!(declared_total(&[0x01, 0x04]), Ok(1025));
    assert_eq!(declared_total(&[0xFF, 0xFF]), Err(ConnError::BadFrame), "above the message limit");
    assert_eq!(declared_total(&[0x01]), Err(ConnError::BadFrame), "one byte is not a length");
}

#[test]
fn a_full_fragment_fits_the_frame_space() {
    // Type, two length bytes, three reliable header bytes, two total-length bytes and 1024 data bytes.
    assert!(frame_size(3 + 2 + MAX_FRAGMENT) <= FRAME_SPACE);
}
