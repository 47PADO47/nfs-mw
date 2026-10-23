//! Round trips and refusals of the handshake datagrams.

use crate::secure::Pattern;
use crate::secure::packet::{
    CHALLENGE_LEN, Challenge, HELLO_LEN, Hello, HelloCookie, HsType, NoiseMsg, PacketError, PacketKind, RejectReason,
    classify,
};

fn sample_hello() -> Hello {
    Hello { proto_major: 3, client_nonce: [0xAB; 16], client_conn_id: 0x1234 }
}

fn sample_cookie(noise: &[u8]) -> HelloCookie<'_> {
    HelloCookie {
        proto_major: 3,
        pattern: Pattern::Xk,
        client_nonce: [0x11; 16],
        client_conn_id: 0x0102,
        server_conn_id: 0x0304,
        bucket: 9,
        cookie: [0x22; 32],
        noise,
    }
}

#[test]
fn hello_round_trips_at_the_spec_offsets() {
    let bytes = sample_hello().encode();
    assert_eq!(bytes.len(), HELLO_LEN);
    assert_eq!(bytes[0], 0x80, "handshake bit, type 0, format 0");
    assert_eq!(bytes[1], 3);
    assert_eq!(bytes[2], 1, "format byte");
    assert_eq!(&bytes[3..19], &[0xAB; 16]);
    assert_eq!(&bytes[19..21], &[0x34, 0x12]);
    assert!(bytes[21..].iter().all(|&b| b == 0));
    assert_eq!(Hello::decode(&bytes), Ok(sample_hello()));
    assert_eq!(classify(&bytes), PacketKind::Handshake(HsType::Hello));
}

#[test]
fn hello_refuses_other_lengths_padding_and_formats() {
    let bytes = sample_hello().encode();
    assert_eq!(Hello::decode(&bytes[..511]), Err(PacketError::BadLength));
    let mut longer = bytes.to_vec();
    longer.push(0);
    assert_eq!(Hello::decode(&longer), Err(PacketError::BadLength));
    let mut padded = bytes;
    padded[300] = 1;
    assert_eq!(Hello::decode(&padded), Err(PacketError::BadPadding));
    let mut format = bytes;
    format[2] = 2;
    assert_eq!(Hello::decode(&format), Err(PacketError::BadVersion));
    let mut kind = bytes;
    kind[0] = HsType::HelloCookie.byte0();
    assert_eq!(Hello::decode(&kind), Err(PacketError::BadType));
}

#[test]
fn challenge_round_trips_and_is_never_larger_than_hello() {
    let challenge = Challenge { server_conn_id: 0xBEEF, bucket: 200, cookie: [5; 32] };
    let bytes = challenge.encode();
    assert_eq!(bytes.len(), CHALLENGE_LEN);
    const { assert!(CHALLENGE_LEN <= HELLO_LEN) };
    assert_eq!(bytes[0], 0x90);
    assert_eq!(&bytes[1..3], &[0xEF, 0xBE]);
    assert_eq!(bytes[3], 200);
    assert_eq!(Challenge::decode(&bytes), Ok(challenge));
    assert_eq!(Challenge::decode(&bytes[..35]), Err(PacketError::BadLength));
}

#[test]
fn hello_cookie_round_trips_with_its_noise_message() {
    let noise = [0x5A; 48];
    let bytes = sample_cookie(&noise).encode().expect("fits");
    assert_eq!(bytes.len(), HELLO_LEN);
    assert_eq!(bytes[0], 0xA0);
    assert_eq!(bytes[3], 1, "pattern XK");
    assert_eq!(&bytes[57..59], &[48, 0]);
    assert_eq!(&bytes[59..107], &noise);
    assert!(bytes[107..].iter().all(|&b| b == 0));
    let decoded = HelloCookie::decode(&bytes).expect("decodes");
    assert_eq!(decoded, sample_cookie(&noise));
}

#[test]
fn hello_cookie_refuses_bad_lengths_patterns_and_padding() {
    let noise = [1u8; 32];
    let bytes = sample_cookie(&noise).encode().expect("fits");
    let mut zero_len = bytes;
    zero_len[57] = 0;
    assert_eq!(HelloCookie::decode(&zero_len), Err(PacketError::BadField));
    let mut huge = bytes;
    huge[57..59].copy_from_slice(&500u16.to_le_bytes());
    assert_eq!(HelloCookie::decode(&huge), Err(PacketError::BadField));
    let mut pattern = bytes;
    pattern[3] = 2;
    assert_eq!(HelloCookie::decode(&pattern), Err(PacketError::BadVersion));
    let mut padding = bytes;
    padding[511] = 9;
    assert_eq!(HelloCookie::decode(&padding), Err(PacketError::BadPadding));
    assert_eq!(sample_cookie(&[]).encode(), Err(PacketError::BadField));
    assert_eq!(sample_cookie(&[0u8; 257]).encode(), Err(PacketError::BadField));
}

#[test]
fn noise_messages_round_trip_and_must_end_with_the_message() {
    let body = [0x77u8; 96];
    let mut out = [0u8; 200];
    let len = NoiseMsg { conn_id: 42, msg: &body }.encode(HsType::NoiseMsg2, &mut out).expect("fits");
    assert_eq!(len, 101);
    let decoded = NoiseMsg::decode(&out[..len], HsType::NoiseMsg2).expect("decodes");
    assert_eq!(decoded.conn_id, 42);
    assert_eq!(decoded.msg, &body);
    assert_eq!(NoiseMsg::decode(&out[..len], HsType::NoiseMsg3), Err(PacketError::BadType));
    assert_eq!(NoiseMsg::decode(&out[..len + 1], HsType::NoiseMsg2), Err(PacketError::BadLength));
    assert_eq!(NoiseMsg::decode(&out[..len - 1], HsType::NoiseMsg2), Err(PacketError::BadLength));
    assert_eq!(
        NoiseMsg { conn_id: 1, msg: &body }.encode(HsType::NoiseMsg3, &mut out[..50]),
        Err(PacketError::BadField)
    );
}

#[test]
fn reject_knows_three_reasons() {
    for reason in [RejectReason::Full, RejectReason::Version, RejectReason::ShuttingDown] {
        assert_eq!(RejectReason::decode(&reason.encode()), Ok(reason));
    }
    assert_eq!(RejectReason::decode(&[0xD0, 0]), Err(PacketError::BadField));
    assert_eq!(RejectReason::decode(&[0xD0, 4]), Err(PacketError::BadField));
    assert_eq!(RejectReason::decode(&[0xD0, 1, 0]), Err(PacketError::BadLength));
}

#[test]
fn classify_sorts_by_the_first_byte() {
    assert_eq!(classify(&[]), PacketKind::Invalid);
    assert_eq!(classify(&[0x01; 6]), PacketKind::Invalid, "too short for a data header");
    assert_eq!(classify(&[0x01; 7]), PacketKind::Data);
    assert_eq!(classify(&[0x81]), PacketKind::Invalid, "format bits must be 0");
    assert_eq!(classify(&[0xE0]), PacketKind::Invalid, "type 6 is unknown");
    assert_eq!(classify(&[0xF0]), PacketKind::Invalid, "type 7 is unknown");
    for (byte, kind) in [
        (0x80, HsType::Hello),
        (0x90, HsType::Challenge),
        (0xA0, HsType::HelloCookie),
        (0xB0, HsType::NoiseMsg2),
        (0xC0, HsType::NoiseMsg3),
        (0xD0, HsType::Reject),
    ] {
        assert_eq!(classify(&[byte]), PacketKind::Handshake(kind));
        assert_eq!(kind.byte0(), byte);
    }
}
