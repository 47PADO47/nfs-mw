//! Connect tokens: the round trip, every check, single use and key rotation.

use std::net::SocketAddr;

use crate::secure::token::{
    ConnectToken, TOKEN_LEN, TokenCheck, TokenError, TokenGuard, TokenKey, TokenKeyring, TokenRole, open_token,
    seal_token,
};

const NOW: u64 = 1_800_000_000;

fn server_key() -> [u8; 32] {
    [0x5E; 32]
}

fn player_key() -> [u8; 32] {
    [0x9A; 32]
}

fn token(id: u8) -> ConnectToken {
    ConnectToken {
        user_id: 42,
        token_id: [id; 16],
        issued_at: NOW,
        expires_at: NOW + 30,
        role: TokenRole::Player,
        ban_epoch: 3,
        player_key: player_key(),
        servers: vec![
            (SocketAddr::from(([203, 0, 113, 7], 7000)), server_key()),
            ("[2001:db8::1]:7001".parse().expect("address"), [0x11; 32]),
        ],
    }
}

fn keyring() -> TokenKeyring {
    let mut ring = TokenKeyring::new();
    ring.install(TokenKey::new(1, [0xA1; 32]));
    ring
}

fn check(now: u64) -> TokenCheck<'static> {
    static SERVER: [u8; 32] = [0x5E; 32];
    static PLAYER: [u8; 32] = [0x9A; 32];
    TokenCheck { now_unix_s: now, server_key: &SERVER, handshake_static: &PLAYER, min_ban_epoch: 3 }
}

fn sealed(token: &ConnectToken) -> [u8; TOKEN_LEN] {
    seal_token(token, &TokenKey::new(1, [0xA1; 32]), &[0x24; 24]).expect("sealed")
}

#[test]
fn a_token_round_trips_and_hides_its_contents() {
    let bytes = sealed(&token(1));
    assert_eq!(bytes.len(), TOKEN_LEN);
    assert_eq!(bytes[0], 1);
    assert_eq!(&bytes[1..5], &1u32.to_le_bytes());
    assert!(!bytes.windows(8).any(|w| w == [0x9A; 8]), "the player key is sealed");
    assert_eq!(open_token(&keyring(), &bytes), Ok(token(1)));
}

#[test]
fn a_guard_admits_a_good_token_once() {
    let mut guard = TokenGuard::new(keyring(), 16);
    let bytes = sealed(&token(1));
    assert_eq!(guard.admit(&bytes, &check(NOW + 2)), Ok(token(1)));
    assert_eq!(guard.admit(&bytes, &check(NOW + 3)), Err(TokenError::Replayed));
    assert!(guard.admit(&sealed(&token(2)), &check(NOW + 3)).is_ok(), "another token id is fine");
}

#[test]
fn every_check_refuses_on_its_own() {
    let mut guard = TokenGuard::new(keyring(), 16);
    let bytes = sealed(&token(1));
    assert_eq!(guard.admit(&bytes, &check(NOW + 36)), Err(TokenError::Expired), "past expiry plus skew");
    assert_eq!(guard.admit(&bytes, &check(NOW - 6)), Err(TokenError::Expired), "issued in the future");
    assert!(guard.admit(&sealed(&token(9)), &check(NOW + 35)).is_ok(), "within the 5 s skew");

    let other_player = [0u8; 32];
    let wrong_player = TokenCheck { handshake_static: &other_player, ..check(NOW) };
    assert_eq!(guard.admit(&sealed(&token(2)), &wrong_player), Err(TokenError::WrongPlayer));
    let other_server = [1u8; 32];
    let wrong_server = TokenCheck { server_key: &other_server, ..check(NOW) };
    assert_eq!(guard.admit(&sealed(&token(3)), &wrong_server), Err(TokenError::WrongServer));
    let banned = TokenCheck { min_ban_epoch: 4, ..check(NOW) };
    assert_eq!(guard.admit(&sealed(&token(4)), &banned), Err(TokenError::Banned));

    let mut long = token(5);
    long.expires_at = NOW + 31;
    assert_eq!(guard.admit(&sealed(&long), &check(NOW)), Err(TokenError::BadField), "lifetime over 30 s");
    let mut backwards = token(6);
    backwards.expires_at = NOW - 1;
    assert_eq!(guard.admit(&sealed(&backwards), &check(NOW)), Err(TokenError::BadField));
    assert!(guard.admit(&sealed(&token(2)), &check(NOW)).is_ok(), "refused tokens are not recorded as used");
}

#[test]
fn a_tampered_or_foreign_token_does_not_open() {
    let bytes = sealed(&token(1));
    for at in [0usize, 3, 10, 100, TOKEN_LEN - 1] {
        let mut forged = bytes;
        forged[at] ^= 0x01;
        assert!(open_token(&keyring(), &forged).is_err(), "flip at {at}");
    }
    let foreign = seal_token(&token(1), &TokenKey::new(1, [0xB2; 32]), &[0; 24]).expect("sealed");
    assert_eq!(open_token(&keyring(), &foreign), Err(TokenError::BadSeal));
    let unknown_kid = seal_token(&token(1), &TokenKey::new(7, [0xA1; 32]), &[0; 24]).expect("sealed");
    assert_eq!(open_token(&keyring(), &unknown_kid), Err(TokenError::UnknownKey));
    assert_eq!(open_token(&keyring(), &bytes[..TOKEN_LEN - 1]), Err(TokenError::Malformed));
}

#[test]
fn two_keys_are_active_after_a_rotation_and_the_oldest_drops() {
    let mut ring = keyring();
    ring.install(TokenKey::new(2, [0xA2; 32]));
    let old = sealed(&token(1));
    let new = seal_token(&token(1), &TokenKey::new(2, [0xA2; 32]), &[1; 24]).expect("sealed");
    assert!(open_token(&ring, &old).is_ok());
    assert!(open_token(&ring, &new).is_ok());
    ring.install(TokenKey::new(3, [0xA3; 32]));
    assert_eq!(open_token(&ring, &old), Err(TokenError::UnknownKey));
    assert!(open_token(&ring, &new).is_ok());
}

#[test]
fn a_token_names_one_to_four_servers() {
    let mut none = token(1);
    none.servers.clear();
    assert!(seal_token(&none, &TokenKey::new(1, [0xA1; 32]), &[0; 24]).is_err());
    let mut five = token(1);
    five.servers = vec![(SocketAddr::from(([1, 2, 3, 4], 1)), [0; 32]); 5];
    assert!(seal_token(&five, &TokenKey::new(1, [0xA1; 32]), &[0; 24]).is_err());
}

#[test]
fn a_full_replay_table_refuses_instead_of_growing() {
    let mut guard = TokenGuard::new(keyring(), 2);
    assert!(guard.admit(&sealed(&token(1)), &check(NOW)).is_ok());
    assert!(guard.admit(&sealed(&token(2)), &check(NOW)).is_ok());
    assert_eq!(guard.admit(&sealed(&token(3)), &check(NOW)), Err(TokenError::Busy));
    let mut later = token(4);
    later.issued_at = NOW + 40;
    later.expires_at = NOW + 60;
    assert!(guard.admit(&sealed(&later), &check(NOW + 40)).is_ok(), "expired entries are forgotten");
}
