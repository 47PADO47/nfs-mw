//! A quick message (MP-44) is checked and relayed to every seated player with the sender's slot.

use nfsmw_net::input::InputCode;
use nfsmw_net::message::{Message, QuickMessage, QuickRelay};
use nfsmw_sim::toy::ToyConfig;

use super::harness::{Harness, test_config};

fn relayed(h: &Harness, index: usize) -> Vec<QuickRelay> {
    h.clients[index]
        .messages
        .iter()
        .filter_map(|message| match message {
            Message::QuickRelay(relay) => Some(relay.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_quick_message_reaches_everyone_with_its_sender() {
    let mut h = Harness::new(test_config(), ToyConfig::bare());
    let ann = h.connect(20);
    let bob = h.connect(21);
    h.join_all(&[ann, bob]);
    h.clients[ann].send(Message::QuickMessage(QuickMessage { id: 3 }));
    h.run(30, InputCode::NEUTRAL);
    let from_slot = h.clients[ann].slot().expect("seated");
    let expected = vec![QuickRelay { from_slot, id: 3 }];
    assert_eq!(relayed(&h, ann), expected, "the sender hears it too");
    assert_eq!(relayed(&h, bob), expected);
}

#[test]
fn an_id_outside_the_table_is_not_relayed() {
    let mut h = Harness::new(test_config(), ToyConfig::bare());
    let ann = h.connect(22);
    let bob = h.connect(23);
    h.join_all(&[ann, bob]);
    h.clients[ann].send(Message::QuickMessage(QuickMessage { id: 5_000 }));
    h.run(30, InputCode::NEUTRAL);
    assert!(relayed(&h, ann).is_empty() && relayed(&h, bob).is_empty());
}
