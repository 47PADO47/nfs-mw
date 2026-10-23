//! The player's identity key on disk (`net_identity_file`, security.md section 3): the 32 secret bytes of the X25519
//! key, made the first time and read every time after, so a server knows the player again (resume, bans, the host's
//! own seat) across runs. The file handling is the server crate's ([`nfsmw_server::identity`]), the same a dedicated
//! server keeps its key with.

use std::path::Path;

use anyhow::Result;
use blackbox_net_core::secure::IdentityKey;

/// The key in `path`, or a new one written there when the file does not exist. A file of the wrong size is an error
/// (it is not overwritten: it may be someone's key).
pub fn load_or_create(path: &Path) -> Result<IdentityKey> {
    Ok(nfsmw_server::identity::load_or_create(path)?)
}
