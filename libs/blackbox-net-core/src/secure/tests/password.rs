//! The room password: matching and wrong passwords, the channel binding, and bad messages.

use crate::secure::password::{MAX_PASSWORD_BYTES, PAKE_MSG_LEN, PakeError, PakeRole, start};

/// Runs the exchange with the given passwords and hashes; returns whether each side accepts the other.
fn exchange(host_pw: &[u8], join_pw: &[u8], host_hash: [u8; 32], join_hash: [u8; 32]) -> (bool, bool) {
    let (host, host_msg) = start(host_pw, &host_hash, PakeRole::Host).expect("host starts");
    let (join, join_msg) = start(join_pw, &join_hash, PakeRole::Joiner).expect("joiner starts");
    let host_done = host.finish(&join_msg).expect("host finishes");
    let join_done = join.finish(&host_msg).expect("joiner finishes");
    (host_done.verify(&join_done.tag()), join_done.verify(&host_done.tag()))
}

#[test]
fn the_same_password_over_the_same_handshake_is_accepted() {
    assert_eq!(exchange(b"hunter22", b"hunter22", [1; 32], [1; 32]), (true, true));
}

#[test]
fn a_wrong_password_is_refused_by_both_sides() {
    assert_eq!(exchange(b"hunter22", b"hunter23", [1; 32], [1; 32]), (false, false));
}

#[test]
fn a_man_in_the_middle_with_two_handshakes_cannot_relay() {
    assert_eq!(exchange(b"hunter22", b"hunter22", [1; 32], [2; 32]), (false, false));
}

#[test]
fn the_two_tags_differ_so_a_reflected_tag_fails() {
    let (host, host_msg) = start(b"pw", &[3; 32], PakeRole::Host).expect("host");
    let (join, join_msg) = start(b"pw", &[3; 32], PakeRole::Joiner).expect("joiner");
    let host_done = host.finish(&join_msg).expect("host");
    let join_done = join.finish(&host_msg).expect("joiner");
    assert_ne!(host_done.tag(), join_done.tag());
    assert!(!host_done.verify(&host_done.tag()), "the host's own tag reflected back");
    assert!(!host_done.verify(&join_done.tag()[..31]));
}

#[test]
fn passwords_and_messages_are_checked() {
    assert_eq!(start(b"", &[0; 32], PakeRole::Host).err(), Some(PakeError::BadPassword));
    let long = vec![b'a'; MAX_PASSWORD_BYTES + 1];
    assert_eq!(start(&long, &[0; 32], PakeRole::Host).err(), Some(PakeError::BadPassword));
    assert!(start(&long[..MAX_PASSWORD_BYTES], &[0; 32], PakeRole::Host).is_ok());
    let (host, msg) = start(b"pw", &[0; 32], PakeRole::Host).expect("host");
    assert_eq!(msg.len(), PAKE_MSG_LEN);
    assert_eq!(host.finish(&msg[..32]).err(), Some(PakeError::BadMessage));
}
