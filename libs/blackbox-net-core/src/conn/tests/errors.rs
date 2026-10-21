//! Connection-level behaviour: the refusals of `read_packet` and `send_message`, the frame rules, disconnect, and timers.

use crate::MAX_DATAGRAM;
use crate::conn::header::{Header, write_header};
use crate::conn::{ConnConfig, ConnError, ConnEvent, Connection, FRAME_SPACE, HEADER_LEN, MAX_IN_FLIGHT, MAX_MESSAGE};

/// Connection 1 talking to 2, and connection 2 talking to 1.
fn pair() -> (Connection, Connection) {
    let a = Connection::new(ConnConfig { local_id: 1, remote_id: 2, ..ConnConfig::default() });
    let b = Connection::new(ConnConfig { local_id: 2, remote_id: 1, ..ConnConfig::default() });
    (a, b)
}

/// Writes one packet from `conn` at `now` and returns the datagram.
fn packet(conn: &mut Connection, now: u64, frames: &[(u8, &[u8])]) -> Vec<u8> {
    let mut out = vec![0u8; MAX_DATAGRAM];
    let sent = conn.write_packet(now, frames, &mut out).expect("write_packet");
    out.truncate(sent.len);
    out
}

/// A datagram for connection 2 with flags 0, the given number and raw frame bytes after the header.
fn raw(number: u32, frames: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; HEADER_LEN];
    let header = Header { flags: 0, conn_id: 2, number, ack: 0, ack_bits: 0 };
    write_header(&mut out, &header).expect("header fits");
    out.extend_from_slice(frames);
    out
}

/// What one read produced: the packet number or the error, and the application frames handed out.
type Received = (Result<u32, ConnError>, Vec<(u8, Vec<u8>)>);

/// Reads `datagram` into `conn` and returns the result together with the application frames handed out.
fn read(conn: &mut Connection, now: u64, datagram: &[u8]) -> Received {
    let mut frames = Vec::new();
    let result = conn.read_packet(now, datagram, |ftype, payload| frames.push((ftype, payload.to_vec())));
    (result, frames)
}

#[test]
fn one_packet_each_way_delivers_frames_and_acks() {
    let (mut a, mut b) = pair();
    let datagram = packet(&mut a, 0, &[(0x40, b"hi")]);
    let (result, frames) = read(&mut b, 0, &datagram);
    assert_eq!(result, Ok(0));
    assert_eq!(frames, vec![(0x40, b"hi".to_vec())]);

    let reply = packet(&mut b, 16, &[]);
    assert_eq!(read(&mut a, 16, &reply).0, Ok(0));
    assert_eq!(a.poll_event(), Some(ConnEvent::Delivered(0)), "packet 0 is acknowledged");
    assert_eq!(a.poll_event(), None);
    assert!(a.bytes_sent() > 0 && a.bytes_received() > 0);
}

#[test]
fn packet_for_another_connection_is_refused() {
    let (mut a, _) = pair();
    let datagram = packet(&mut a, 0, &[]);
    let mut stranger = Connection::new(ConnConfig { local_id: 3, remote_id: 1, ..ConnConfig::default() });
    assert_eq!(read(&mut stranger, 0, &datagram).0, Err(ConnError::WrongConnection));
}

#[test]
fn short_datagrams_are_truncated() {
    let (_, mut b) = pair();
    assert_eq!(read(&mut b, 0, &[0u8; 14]).0, Err(ConnError::Truncated));
    assert_eq!(read(&mut b, 0, &[]).0, Err(ConnError::Truncated));
}

#[test]
fn reserved_flag_bits_are_refused() {
    for flags in [0x80u8, 0x02, 0x04, 0x10, 0x20, 0x40, 0x0E] {
        let mut datagram = raw(0, &[]);
        datagram[0] = flags;
        let (_, mut b) = pair();
        assert_eq!(read(&mut b, 0, &datagram).0, Err(ConnError::BadHeader), "flags {flags:#04x}");
    }
    let mut datagram = raw(0, &[]);
    datagram[0] = 0x01;
    let (_, mut b) = pair();
    assert_eq!(read(&mut b, 0, &datagram).0, Ok(0), "the ack-valid bit alone is fine");
}

#[test]
fn a_repeated_packet_is_a_duplicate_and_hands_out_nothing_twice() {
    let (mut a, mut b) = pair();
    let datagram = packet(&mut a, 0, &[(0x40, b"once")]);
    assert_eq!(read(&mut b, 0, &datagram).1.len(), 1);
    let (result, frames) = read(&mut b, 0, &datagram);
    assert_eq!(result, Err(ConnError::Duplicate));
    assert!(frames.is_empty());
}

#[test]
fn a_packet_more_than_1024_behind_is_too_old() {
    let (_, mut b) = pair();
    assert_eq!(read(&mut b, 0, &raw(5000, &[])).0, Ok(5000));
    assert_eq!(read(&mut b, 0, &raw(1, &[])).0, Err(ConnError::TooOld));
}

#[test]
fn channel_out_of_range_is_refused() {
    let (mut a, _) = pair();
    assert_eq!(a.send_message(4, b"x"), Err(ConnError::BadChannel));
    assert_eq!(a.send_message(3, b"x"), Ok(()));
}

#[test]
fn message_size_limit_is_the_fragment_limit() {
    let (mut a, _) = pair();
    assert_eq!(a.send_message(0, &vec![7u8; 16_382]), Ok(()), "16 fragments exactly");
    assert_eq!(a.send_message(0, &vec![7u8; 16_383]), Err(ConnError::MessageTooLarge), "would need 17");
    assert_eq!(a.send_message(0, &vec![7u8; MAX_MESSAGE + 1]), Err(ConnError::MessageTooLarge));
    assert_eq!(a.pending_messages(), 1, "refused messages are not queued");
}

#[test]
fn a_channel_refuses_more_than_256_unacknowledged_messages() {
    let (mut a, _) = pair();
    for _ in 0..MAX_IN_FLIGHT {
        assert_eq!(a.send_message(1, b"m"), Ok(()));
    }
    assert_eq!(a.send_message(1, b"m"), Err(ConnError::QueueFull));
    assert_eq!(a.send_message(2, b"m"), Ok(()), "other channels are not affected");
}

#[test]
fn empty_messages_are_allowed() {
    let (mut a, mut b) = pair();
    a.send_message(0, b"").expect("empty message");
    let datagram = packet(&mut a, 0, &[]);
    read(&mut b, 0, &datagram).0.expect("packet");
    let mut out = vec![1u8];
    assert_eq!(b.recv_message(&mut out), Some(0));
    assert!(out.is_empty());
}

#[test]
fn app_frames_must_fit_the_frame_space() {
    let (mut a, _) = pair();
    let mut out = vec![0u8; MAX_DATAGRAM];
    let too_big = vec![0u8; FRAME_SPACE - 2];
    assert_eq!(a.write_packet(0, &[(0x40, &too_big)], &mut out), Err(ConnError::FrameTooLarge));
    let fits = vec![0u8; FRAME_SPACE - 3];
    let sent = a.write_packet(0, &[(0x40, &fits)], &mut out).expect("exactly the frame space");
    assert_eq!(sent.number, 0, "the refused write used no packet number");
    assert_eq!(sent.len, MAX_DATAGRAM - 16);
}

#[test]
fn output_must_hold_a_whole_datagram() {
    let (mut a, _) = pair();
    let mut out = vec![0u8; MAX_DATAGRAM - 1];
    assert_eq!(a.write_packet(0, &[], &mut out), Err(ConnError::BufferTooSmall));
}

#[test]
fn malformed_frames_are_refused() {
    let cases: [&[u8]; 5] = [
        &[0x40, 0x05, 0x01],
        &[0x40, 0x80, 0x00],
        &[0x10, 0x03, 0x0F, 0x00, 0x00],
        &[0x10, 0x06, 0x20, 0x00, 0x00, 0x00, 0x04, 0x00],
        &[0x11, 0x00],
    ];
    for frames in cases {
        let (_, mut b) = pair();
        assert_eq!(read(&mut b, 0, &raw(0, frames)).0, Err(ConnError::BadFrame), "frames {frames:02x?}");
    }
}

#[test]
fn frames_before_a_bad_one_are_still_handed_out() {
    let (_, mut b) = pair();
    let mut frames = vec![0x30, 0x02, b'o', b'k'];
    frames.extend_from_slice(&[0x31, 0x09, 0x00]);
    let (result, got) = read(&mut b, 0, &raw(0, &frames));
    assert_eq!(result, Err(ConnError::BadFrame));
    assert_eq!(got, vec![(0x30, b"ok".to_vec())]);
}

#[test]
fn padding_and_reserved_types_are_skipped() {
    let (_, mut b) = pair();
    let frames = [0x00, 0x12, 0x00, 0x13, 0x01, 0x07, 0x00, 0x00, 0x30, 0x01, 0x42, 0x00];
    let (result, got) = read(&mut b, 0, &raw(0, &frames));
    assert_eq!(result, Ok(0));
    assert_eq!(got, vec![(0x30, vec![0x42])]);
}

#[test]
fn disconnect_is_sent_once_and_reported() {
    let (mut a, mut b) = pair();
    a.close(7);
    let first = packet(&mut a, 0, &[]);
    let second = packet(&mut a, 16, &[]);
    assert_eq!(read(&mut b, 0, &first).0, Ok(0));
    assert_eq!(b.poll_event(), Some(ConnEvent::Disconnected(7)));
    assert_eq!(read(&mut b, 16, &second).0, Ok(1));
    assert_eq!(b.poll_event(), None, "the disconnect went out once");
}

#[test]
fn timeout_and_keepalive_count_from_the_first_call() {
    let (mut a, mut b) = pair();
    assert!(!a.is_timed_out(u64::MAX), "no clock before the first call");
    assert!(!a.wants_keepalive(u64::MAX));
    let datagram = packet(&mut a, 1_000, &[]);
    assert!(!a.is_timed_out(10_999));
    assert!(a.is_timed_out(11_000), "10 s after creation with nothing heard");
    assert!(!a.wants_keepalive(2_999));
    assert!(a.wants_keepalive(3_000), "2 s after the last send");

    read(&mut b, 5_000, &datagram).0.expect("packet");
    let reply = packet(&mut b, 5_000, &[]);
    assert_eq!(read(&mut a, 5_000, &reply).0, Ok(0));
    assert!(!a.is_timed_out(14_999));
    assert!(a.is_timed_out(15_000), "10 s after the last packet heard");
}
