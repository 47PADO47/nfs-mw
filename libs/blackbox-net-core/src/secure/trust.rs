//! Trust on first use (security.md 4.5), as a pure API over keys and a store the caller provides. Pins are keyed by
//! [`KeyId`], never by address. A different key is a different id, so "the key changed" cannot happen for one id; what
//! can happen is a new id using the display name of a pinned one, and [`assess`] reports that.

use super::{KEY_LEN, KeyId};

/// Where pinned peers (and bans) live. The game backs it with a file; tests use [`MemoryPeerStore`].
pub trait PeerStore {
    /// Whether `id` is pinned.
    fn is_pinned(&self, id: &KeyId) -> bool;
    /// A pinned id that uses `name` as its display name, if any.
    fn pinned_with_name(&self, name: &str) -> Option<KeyId>;
    /// Pins `id` under `name` (the player confirmed "trust this peer").
    fn pin(&mut self, id: KeyId, name: &str);
    /// Whether `id` is banned. No bans by default.
    fn is_banned(&self, _id: &KeyId) -> bool {
        false
    }
}

/// What the lobby shows for a peer after the handshake proved it holds `public`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    /// Pinned before: connect silently, show a "known" mark.
    Known,
    /// Not pinned: show the fingerprint and ask once. `name_clash` is a pinned id that uses the same display name
    /// ("a new host with the name of a host you know; different key").
    New {
        /// The pinned id with the same name, if any.
        name_clash: Option<KeyId>,
    },
}

/// Decides how to present the peer with key `public` and display name `name`.
pub fn assess(store: &impl PeerStore, public: &[u8; KEY_LEN], name: &str) -> Trust {
    let id = KeyId::of(public);
    if store.is_pinned(&id) {
        return Trust::Known;
    }
    let name_clash = store.pinned_with_name(name).filter(|other| *other != id);
    Trust::New { name_clash }
}

/// The host's join mode (security.md 4.5, step 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinMode {
    /// Anyone not banned joins.
    Open,
    /// The host approves each id it has not pinned.
    Ask,
    /// Only pinned ids join.
    FriendsOnly,
}

/// What the host does with a joiner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    /// Let it in.
    Admit,
    /// Ask the host first.
    AskHost,
    /// Refuse with the generic "join refused".
    Refuse,
}

/// The host's decision for the joiner with id `id` under `mode`.
pub fn admit(store: &impl PeerStore, mode: JoinMode, id: &KeyId) -> Admission {
    if store.is_banned(id) {
        return Admission::Refuse;
    }
    if mode == JoinMode::Open || store.is_pinned(id) {
        return Admission::Admit;
    }
    match mode {
        JoinMode::Ask => Admission::AskHost,
        _ => Admission::Refuse,
    }
}

/// A [`PeerStore`] in memory: the pins and bans of one session, or a test.
#[derive(Debug, Clone, Default)]
pub struct MemoryPeerStore {
    pins: Vec<(KeyId, String)>,
    bans: Vec<KeyId>,
}

impl MemoryPeerStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bans `id`.
    pub fn ban(&mut self, id: KeyId) {
        if !self.bans.contains(&id) {
            self.bans.push(id);
        }
    }

    /// The pins, oldest first.
    pub fn pins(&self) -> &[(KeyId, String)] {
        &self.pins
    }
}

impl PeerStore for MemoryPeerStore {
    fn is_pinned(&self, id: &KeyId) -> bool {
        self.pins.iter().any(|(pinned, _)| pinned == id)
    }

    fn pinned_with_name(&self, name: &str) -> Option<KeyId> {
        self.pins.iter().find(|(_, pinned_name)| pinned_name == name).map(|(id, _)| *id)
    }

    fn pin(&mut self, id: KeyId, name: &str) {
        if let Some(entry) = self.pins.iter_mut().find(|(pinned, _)| *pinned == id) {
            entry.1 = name.to_owned();
            return;
        }
        self.pins.push((id, name.to_owned()));
    }

    fn is_banned(&self, id: &KeyId) -> bool {
        self.bans.contains(id)
    }
}
