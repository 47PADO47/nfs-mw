use blackbox_net_core::endpoint::ClientEvent;
use blackbox_net_core::rng::SplitMix64;

use super::*;
use crate::client_session::{EndReason, Status};
use crate::frames::FRAME_PONG;
use crate::message::{ChatChannel, ChatSend, ChatText, SetReady};

fn noise(rng: &mut SplitMix64, max: u64) -> Vec<u8> {
    let len = rng.below(max) as usize;
    (0..len).map(|_| rng.next_u64() as u8).collect()
}

#[test]
fn random_frames_never_panic_or_end_the_session() {
    let mut h = Harness::synced();
    let mut rng = SplitMix64(0x0BAD_5EED);
    for round in 0..2000 {
        let frame_type = [FRAME_SNAPSHOT, FRAME_PONG, 0x05, 0x99][round % 4];
        h.unreliable(frame_type, noise(&mut rng, 200));
        if round % 50 == 0 {
            h.frame();
        }
    }
    h.frame();
    assert!(h.session.status().is_joined());
    assert!(h.session.counters().bad_frames > 1000);
}

#[test]
fn random_messages_never_panic() {
    for seed in 0..50u64 {
        let mut h = Harness::joined();
        let mut rng = SplitMix64(seed);
        for _ in 0..40 {
            let channel = rng.below(4) as u8;
            h.peer.push(ClientEvent::Reliable { channel, bytes: noise(&mut rng, 64) });
        }
        h.frame();
    }
}

#[test]
fn too_many_invalid_messages_end_the_session() {
    let mut h = Harness::joined();
    for _ in 0..8 {
        // Client-to-server messages and a message on the wrong channel are invalid from a server.
        h.reliable(SetReady { ready: true }.into());
        let chat = ChatSend { channel: ChatChannel::All, to_slot: None, text: ChatText::new("x").unwrap() };
        h.reliable(chat.into());
    }
    h.frame();
    assert!(h.session.status().is_joined(), "16 are tolerated");
    h.peer.push(ClientEvent::Reliable { channel: 2, bytes: vec![0x10, 0x00, 0x01] });
    h.frame();
    assert_eq!(*h.session.status(), Status::Ended(EndReason::Protocol));
}

#[test]
fn a_seat_offered_twice_is_invalid() {
    let mut h = Harness::joined();
    h.reliable(accepted(SLOT, b"x").into());
    h.frame();
    assert!(h.session.status().is_joined());
    assert_eq!(h.session.counters().bad_messages, 1);
}
