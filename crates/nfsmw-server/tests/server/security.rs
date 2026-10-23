//! Hostile clients (security-gameplay.md sections 1.3 and 5): floods of malformed messages and frames, commands out
//! of order, spoofed host commands.

use blackbox_net_core::endpoint::{DisconnectReason, reason};
use nfsmw_net::input::{FRAME_INPUT, InputCode};
use nfsmw_net::message::{AdminCommand, HostCommand, KickReason, Message};
use nfsmw_sim::toy::ToyConfig;

use super::harness::{Harness, test_config};

#[test]
fn a_flood_of_malformed_messages_strikes_and_disconnects() {
    let mut h = Harness::new(test_config(), ToyConfig::bare());
    let good = h.connect(70);
    let bad = h.connect(71);
    h.join_all(&[good, bad]);
    for index in 0..20u8 {
        let garbage = vec![0x10, 0x00, 0x01, 0xFF, index, 0xFF, 0xFF];
        h.clients[bad].endpoint.send_reliable(0, garbage);
    }
    h.until("the kick", 600, InputCode::NEUTRAL, |h| h.clients[bad].disconnected.is_some());
    assert_eq!(h.clients[bad].kicked.as_ref().map(|kick| kick.reason), Some(KickReason::Violations));
    assert_eq!(h.clients[bad].disconnected, Some(DisconnectReason::Remote(reason::KICKED)));
    let stats = h.server.stats();
    assert!(stats.invalid_messages >= 12, "the bad messages counted: {}", stats.invalid_messages);
    assert!(stats.strikes >= 3);
    assert_eq!(h.server.lobby().members().count(), 1, "the slot is released");
    assert!(h.clients[good].disconnected.is_none(), "the honest client stays");
}

#[test]
fn a_flood_of_bad_input_frames_is_dropped_and_kicked() {
    let mut h = Harness::new(test_config(), ToyConfig::bare());
    let bad = h.connect(72);
    h.join_all(&[bad]);
    for index in 0..40u8 {
        h.clients[bad].endpoint.send_unreliable(FRAME_INPUT, vec![index, 0xFF, 0xFF], None);
    }
    h.until("the kick", 600, InputCode::NEUTRAL, |h| h.clients[bad].disconnected.is_some());
    assert_eq!(h.clients[bad].kicked.as_ref().map(|kick| kick.reason), Some(KickReason::Violations));
}

#[test]
fn spoofed_host_commands_add_up_to_a_kick() {
    let mut h = Harness::new(test_config(), ToyConfig::bare());
    let a = h.connect(73);
    let b = h.connect(74);
    h.join_all(&[a, b]);
    let target = h.clients[b].slot().expect("seated");
    for _ in 0..6 {
        h.clients[a].send(Message::HostCommand(HostCommand { command: AdminCommand::Kick { slot: target } }));
        h.run(2, InputCode::NEUTRAL);
    }
    h.until("the spoofer's kick", 300, InputCode::NEUTRAL, |h| h.clients[a].disconnected.is_some());
    assert!(h.clients[b].disconnected.is_none(), "the target stays");
    assert!(h.server.lobby().member(target).is_some());
}
