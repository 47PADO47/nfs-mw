//! Identity keys, ids and fingerprints, and trust on first use.

use super::harness::identity;
use crate::secure::{Admission, IdentityKey, JoinMode, KeyId, MemoryPeerStore, PeerStore, Trust, admit, assess};

#[test]
fn a_stored_secret_gives_back_the_same_key() {
    let key = IdentityKey::generate().expect("random key");
    let again = IdentityKey::from_secret(&key.secret_bytes()).expect("from secret");
    assert_eq!(key.public(), again.public());
    assert_eq!(key.key_id(), again.key_id());
    assert_ne!(IdentityKey::generate().expect("random key").public(), key.public(), "two installs, two keys");
}

#[test]
fn the_debug_form_never_shows_the_secret() {
    let key = identity(9);
    let text = format!("{key:?}");
    assert!(text.contains("IdentityKey"));
    assert!(!text.contains("secret"));
    assert!(!text.contains("09, 09"));
}

#[test]
fn the_key_id_is_the_first_16_bytes_of_sha256() {
    // SHA-256 of 32 zero bytes starts 66687aadf862bd776c8fc18b8e9f8e20.
    let id = KeyId::of(&[0; 32]);
    assert_eq!(id.fingerprint(), "6668 7aad f862 bd77 6c8f c18b 8e9f 8e20");
    assert_eq!(id.short(), [0x66, 0x68, 0x7a, 0xad, 0xf8, 0x62, 0xbd, 0x77]);
    assert_eq!(id.to_string(), id.fingerprint());
    let words = id.phrase_indices();
    assert!(words.iter().all(|&w| w < 2048));
    assert_eq!(words[0], 0x666 >> 1, "the first 11 bits");
}

#[test]
fn a_new_peer_is_new_until_pinned_then_known() {
    let mut store = MemoryPeerStore::new();
    let host = identity(1);
    assert_eq!(assess(&store, host.public(), "Razor"), Trust::New { name_clash: None });
    store.pin(host.key_id(), "Razor");
    assert_eq!(assess(&store, host.public(), "Razor"), Trust::Known);
    assert_eq!(assess(&store, host.public(), "Renamed"), Trust::Known, "pins are by key, not by name");
}

#[test]
fn a_new_key_with_a_pinned_name_is_flagged() {
    let mut store = MemoryPeerStore::new();
    store.pin(identity(1).key_id(), "Razor");
    let impostor = identity(2);
    assert_eq!(assess(&store, impostor.public(), "Razor"), Trust::New { name_clash: Some(identity(1).key_id()) });
}

#[test]
fn join_modes_and_bans_decide_admission() {
    let mut store = MemoryPeerStore::new();
    let friend = identity(1).key_id();
    let stranger = identity(2).key_id();
    store.pin(friend, "Friend");
    assert_eq!(admit(&store, JoinMode::Open, &stranger), Admission::Admit);
    assert_eq!(admit(&store, JoinMode::Ask, &stranger), Admission::AskHost);
    assert_eq!(admit(&store, JoinMode::Ask, &friend), Admission::Admit);
    assert_eq!(admit(&store, JoinMode::FriendsOnly, &stranger), Admission::Refuse);
    assert_eq!(admit(&store, JoinMode::FriendsOnly, &friend), Admission::Admit);
    store.ban(friend);
    assert!(store.is_banned(&friend));
    assert_eq!(admit(&store, JoinMode::Open, &friend), Admission::Refuse, "a ban beats a pin");
}
