//! Connect tokens (security.md 4.8, online only). The account service mints a sealed token naming the player's static
//! key and the servers (with their static keys) the player may join; a server admits a connection only if the token
//! opens under one of its two active keys, has not expired, names the key the handshake authenticated and the server's
//! own key, and has not been used before. Wall-clock seconds appear here and nowhere else in the protocol.
//!
//! **Wire layout** ([`TOKEN_LEN`] bytes): `version` u8 = 1 and `kid` u32 in the clear (the associated data), a 24-byte
//! random nonce, then the sealed body and the 16-byte XChaCha20-Poly1305 tag. **Body** ([`BODY_LEN`] bytes):
//! `user_id` u64, `token_id` 16, `issued_at` u64, `expires_at` u64, `role` u8, `ban_epoch` u32, `player_key` 32,
//! `server_count` u8 (1 to [`MAX_SERVERS`]), then [`MAX_SERVERS`] entries of address (16, IPv4 mapped) + port u16 +
//! key 32, unused entries zero. All integers little-endian. Every token has the same size.

use std::net::{IpAddr, Ipv6Addr, SocketAddr};

use chacha20poly1305::aead::{AeadInPlace, KeyInit};
use chacha20poly1305::{Key, Tag, XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

use super::crypto::ct_eq;

/// Token format version.
pub const TOKEN_VERSION: u8 = 1;
/// Servers one token can name.
pub const MAX_SERVERS: usize = 4;
/// Bytes of one server entry.
const SERVER_LEN: usize = 18 + 32;
/// Bytes of the sealed body.
pub const BODY_LEN: usize = 8 + 16 + 8 + 8 + 1 + 4 + 32 + 1 + MAX_SERVERS * SERVER_LEN;
/// Bytes of the clear head: version and key id.
const HEAD_LEN: usize = 5;
/// Bytes of a token on the wire.
pub const TOKEN_LEN: usize = HEAD_LEN + 24 + BODY_LEN + 16;
/// The longest lifetime a token may claim.
pub const MAX_LIFETIME_S: u64 = 30;
/// Clock skew tolerated between the account service and the server.
pub const SKEW_S: u64 = 5;

/// What the token lets its holder be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenRole {
    /// A racer.
    Player = 0,
    /// A spectator.
    Spectator = 1,
}

/// The contents of a connect token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectToken {
    /// The account.
    pub user_id: u64,
    /// Random, unique per token; the replay key.
    pub token_id: [u8; 16],
    /// Unix seconds when minted.
    pub issued_at: u64,
    /// Unix seconds after which it is refused.
    pub expires_at: u64,
    /// Player or spectator.
    pub role: TokenRole,
    /// The account's ban epoch when minted; a server refuses tokens older than its current epoch.
    pub ban_epoch: u32,
    /// The player's static public key; must equal the key the handshake authenticated.
    pub player_key: [u8; 32],
    /// The servers the token is good for, each with its static public key (the `XK` pin).
    pub servers: Vec<(SocketAddr, [u8; 32])>,
}

/// Why a token was refused. Never sent on the wire (the connection gets a generic close); for counters only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenError {
    /// Wrong length or version.
    Malformed,
    /// The key id is not one of the active keys.
    UnknownKey,
    /// The tag did not verify.
    BadSeal,
    /// A field is out of range (role, server count, lifetime).
    BadField,
    /// Expired, or issued in the future.
    Expired,
    /// The ban epoch is older than the server's.
    Banned,
    /// The player key is not the key the handshake authenticated.
    WrongPlayer,
    /// This server's key is not in the token.
    WrongServer,
    /// The token was used before.
    Replayed,
    /// The replay table is full of live tokens; refused to stay bounded.
    Busy,
}

/// One token key and its id.
pub struct TokenKey {
    /// The key id written in the clear.
    pub kid: u32,
    key: Zeroizing<[u8; 32]>,
}

impl TokenKey {
    /// A key with id `kid`.
    pub fn new(kid: u32, key: [u8; 32]) -> Self {
        Self { kid, key: Zeroizing::new(key) }
    }
}

/// The two active token keys (rotation every 30 days keeps two `kid`s valid).
#[derive(Default)]
pub struct TokenKeyring {
    keys: [Option<TokenKey>; 2],
}

impl TokenKeyring {
    /// An empty keyring.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `key` as the newest; the oldest of the two is dropped.
    pub fn install(&mut self, key: TokenKey) {
        let [newest, _] = std::mem::take(&mut self.keys);
        self.keys = [Some(key), newest];
    }

    fn find(&self, kid: u32) -> Option<&TokenKey> {
        self.keys.iter().flatten().find(|key| key.kid == kid)
    }
}

/// Seals `token` under `key` with `nonce` (24 random bytes, never reused under one key). The minting side; in this
/// crate for tests and for a self-hosted account service.
pub fn seal_token(token: &ConnectToken, key: &TokenKey, nonce: &[u8; 24]) -> Result<[u8; TOKEN_LEN], TokenError> {
    let count = token.servers.len();
    if count == 0 || count > MAX_SERVERS {
        return Err(TokenError::BadField);
    }
    let mut out = [0u8; TOKEN_LEN];
    out[0] = TOKEN_VERSION;
    out[1..5].copy_from_slice(&key.kid.to_le_bytes());
    out[5..29].copy_from_slice(nonce);
    let body = &mut out[29..29 + BODY_LEN];
    body[0..8].copy_from_slice(&token.user_id.to_le_bytes());
    body[8..24].copy_from_slice(&token.token_id);
    body[24..32].copy_from_slice(&token.issued_at.to_le_bytes());
    body[32..40].copy_from_slice(&token.expires_at.to_le_bytes());
    body[40] = token.role as u8;
    body[41..45].copy_from_slice(&token.ban_epoch.to_le_bytes());
    body[45..77].copy_from_slice(&token.player_key);
    body[77] = count as u8;
    for (i, (addr, server_key)) in token.servers.iter().enumerate() {
        let at = 78 + i * SERVER_LEN;
        let ip = match addr.ip() {
            IpAddr::V4(v4) => v4.to_ipv6_mapped(),
            IpAddr::V6(v6) => v6,
        };
        body[at..at + 16].copy_from_slice(&ip.octets());
        body[at + 16..at + 18].copy_from_slice(&addr.port().to_le_bytes());
        body[at + 18..at + 50].copy_from_slice(server_key);
    }
    let (head, rest) = out.split_at_mut(HEAD_LEN);
    let (nonce_bytes, rest) = rest.split_at_mut(24);
    let (body, tag_slot) = rest.split_at_mut(BODY_LEN);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key.key[..]));
    let tag = cipher
        .encrypt_in_place_detached(XNonce::from_slice(nonce_bytes), head, body)
        .map_err(|_| TokenError::BadSeal)?;
    tag_slot.copy_from_slice(&tag);
    Ok(out)
}

/// Opens a token: version, key id and tag only (the cheap precheck before any allocation for the player).
pub fn open_token(keyring: &TokenKeyring, bytes: &[u8]) -> Result<ConnectToken, TokenError> {
    let wire: &[u8; TOKEN_LEN] = bytes.try_into().map_err(|_| TokenError::Malformed)?;
    if wire[0] != TOKEN_VERSION {
        return Err(TokenError::Malformed);
    }
    let kid = u32::from_le_bytes([wire[1], wire[2], wire[3], wire[4]]);
    let key = keyring.find(kid).ok_or(TokenError::UnknownKey)?;
    let mut copy = Zeroizing::new(*wire);
    let (head, rest) = copy.split_at_mut(HEAD_LEN);
    let (nonce, rest) = rest.split_at_mut(24);
    let (body, tag) = rest.split_at_mut(BODY_LEN);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&key.key[..]));
    cipher
        .decrypt_in_place_detached(XNonce::from_slice(nonce), head, body, Tag::from_slice(tag))
        .map_err(|_| TokenError::BadSeal)?;
    decode_body(body)
}

fn u64_at(body: &[u8], at: usize) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&body[at..at + 8]);
    u64::from_le_bytes(bytes)
}

fn decode_body(body: &[u8]) -> Result<ConnectToken, TokenError> {
    if body.len() != BODY_LEN {
        return Err(TokenError::Malformed);
    }
    let mut token_id = [0u8; 16];
    token_id.copy_from_slice(&body[8..24]);
    let role = match body[40] {
        0 => TokenRole::Player,
        1 => TokenRole::Spectator,
        _ => return Err(TokenError::BadField),
    };
    let mut player_key = [0u8; 32];
    player_key.copy_from_slice(&body[45..77]);
    let count = usize::from(body[77]);
    if count == 0 || count > MAX_SERVERS {
        return Err(TokenError::BadField);
    }
    let mut servers = Vec::with_capacity(count);
    for i in 0..count {
        let at = 78 + i * SERVER_LEN;
        let mut ip = [0u8; 16];
        ip.copy_from_slice(&body[at..at + 16]);
        let port = u16::from_le_bytes([body[at + 16], body[at + 17]]);
        let ip = Ipv6Addr::from(ip);
        let ip = ip.to_ipv4_mapped().map_or(IpAddr::V6(ip), IpAddr::V4);
        let mut key = [0u8; 32];
        key.copy_from_slice(&body[at + 18..at + 50]);
        servers.push((SocketAddr::new(ip, port), key));
    }
    Ok(ConnectToken {
        user_id: u64_at(body, 0),
        token_id,
        issued_at: u64_at(body, 24),
        expires_at: u64_at(body, 32),
        role,
        ban_epoch: u32::from_le_bytes([body[41], body[42], body[43], body[44]]),
        player_key,
        servers,
    })
}

/// What a server checks a token against.
#[derive(Debug, Clone, Copy)]
pub struct TokenCheck<'a> {
    /// Unix seconds now.
    pub now_unix_s: u64,
    /// This server's static public key.
    pub server_key: &'a [u8; 32],
    /// The client's static key, as the handshake authenticated it.
    pub handshake_static: &'a [u8; 32],
    /// The current ban epoch; older tokens are refused.
    pub min_ban_epoch: u32,
}

/// The server side: opens, checks and records tokens so each is used once.
pub struct TokenGuard {
    keyring: TokenKeyring,
    used: Vec<([u8; 16], u64)>,
    capacity: usize,
}

impl TokenGuard {
    /// A guard over `keyring` that remembers up to `capacity` used tokens until they expire.
    pub fn new(keyring: TokenKeyring, capacity: usize) -> Self {
        Self { keyring, used: Vec::new(), capacity: capacity.max(1) }
    }

    /// The keyring, for rotation.
    pub fn keyring_mut(&mut self) -> &mut TokenKeyring {
        &mut self.keyring
    }

    /// Admits `bytes` as a token or says why not. Only a token that passes every check is recorded as used.
    pub fn admit(&mut self, bytes: &[u8], check: &TokenCheck<'_>) -> Result<ConnectToken, TokenError> {
        let token = open_token(&self.keyring, bytes)?;
        let lifetime = token.expires_at.checked_sub(token.issued_at).ok_or(TokenError::BadField)?;
        if lifetime > MAX_LIFETIME_S {
            return Err(TokenError::BadField);
        }
        let now = check.now_unix_s;
        if now > token.expires_at.saturating_add(SKEW_S) || now.saturating_add(SKEW_S) < token.issued_at {
            return Err(TokenError::Expired);
        }
        if token.ban_epoch < check.min_ban_epoch {
            return Err(TokenError::Banned);
        }
        if !ct_eq(&token.player_key, check.handshake_static) {
            return Err(TokenError::WrongPlayer);
        }
        if !token.servers.iter().any(|(_, key)| ct_eq(key, check.server_key)) {
            return Err(TokenError::WrongServer);
        }
        self.used.retain(|(_, expires)| expires.saturating_add(SKEW_S) >= now);
        if self.used.iter().any(|(id, _)| *id == token.token_id) {
            return Err(TokenError::Replayed);
        }
        if self.used.len() >= self.capacity {
            return Err(TokenError::Busy);
        }
        self.used.push((token.token_id, token.expires_at));
        Ok(token)
    }
}
