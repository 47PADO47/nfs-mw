//! The version numbers and build identities a peer presents: [`ProtoVersion`], [`DataId`] and [`SimBuild`].

use sha2::{Digest, Sha256};

use crate::message::macros::wire_struct;

wire_struct! {
    /// A protocol version: `major` must match, `minor` is negotiated down.
    #[derive(Copy, Eq, Hash)]
    pub struct ProtoVersion {
        pub major: u8,
        pub minor: u16,
    }
}

impl ProtoVersion {
    /// This build's protocol version.
    pub const CURRENT: Self = Self { major: super::PROTO_MAJOR, minor: super::PROTO_MINOR };
}

wire_struct! {
    /// The identified game data (protocol.md section 8.3): 16 bytes of a SHA-256 over the executable's hash and the list
    /// of required files with their sizes. Large files are not hashed at join.
    #[derive(Copy, Eq, Hash)]
    pub struct DataId {
        pub bytes: [u8; 16],
    }
}

impl DataId {
    /// The data id of an install whose executable hashes to `exe_sha256` and whose required files are `files`
    /// (name and size). The file list is sorted by name first, so the order the caller found them in does not matter.
    pub fn compute(exe_sha256: &[u8; 32], files: &[(&str, u64)]) -> Self {
        let mut sorted: Vec<&(&str, u64)> = files.iter().collect();
        sorted.sort();
        let mut hasher = Sha256::new();
        hasher.update(b"nfsmw-data/1\0");
        hasher.update(exe_sha256);
        hasher.update((sorted.len() as u64).to_le_bytes());
        for (name, size) in sorted {
            hasher.update((name.len() as u64).to_le_bytes());
            hasher.update(name.as_bytes());
            hasher.update(size.to_le_bytes());
        }
        let digest = hasher.finalize();
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        Self { bytes }
    }

    /// The first 4 bytes, as the discovery reply carries them for an early "incompatible" label.
    pub fn short(&self) -> [u8; 4] {
        [self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3]]
    }
}

wire_struct! {
    /// The simulation build: 8 bytes of a SHA-256 over the git hash and the target triple. The same build predicts
    /// exactly; another one may still play (MP-62).
    #[derive(Copy, Eq, Hash)]
    pub struct SimBuild {
        pub bytes: [u8; 8],
    }
}

impl SimBuild {
    /// The sim build of `git_hash` compiled for `target`.
    pub fn from_parts(git_hash: &str, target: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"nfsmw-sim/1\0");
        hasher.update(git_hash.as_bytes());
        hasher.update([0]);
        hasher.update(target.as_bytes());
        let digest = hasher.finalize();
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&digest[..8]);
        Self { bytes }
    }
}
