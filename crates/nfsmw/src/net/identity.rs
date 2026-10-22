//! The player's identity key on disk (`net_identity_file`, security.md section 3): the 32 secret bytes of the X25519
//! key, made the first time and read every time after, so a server knows the player again (resume, bans, the host's
//! own seat) across runs.

use std::path::Path;

use anyhow::{Context, Result, bail};
use blackbox_net_core::secure::IdentityKey;

/// The length of the key file: the secret alone.
const KEY_FILE_LEN: usize = 32;

/// The key in `path`, or a new one written there when the file does not exist. A file of the wrong size is an error
/// (it is not overwritten: it may be someone's key).
pub fn load_or_create(path: &Path) -> Result<IdentityKey> {
    match std::fs::read(path) {
        Ok(bytes) => read(&bytes).with_context(|| format!("the identity key {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => create(path),
        Err(error) => Err(error).with_context(|| format!("cannot read the identity key {}", path.display())),
    }
}

fn read(bytes: &[u8]) -> Result<IdentityKey> {
    let Ok(secret) = <&[u8; KEY_FILE_LEN]>::try_from(bytes) else {
        bail!("{} bytes, not {KEY_FILE_LEN}", bytes.len());
    };
    IdentityKey::from_secret(secret).map_err(|err| anyhow::anyhow!("not a key: {err}"))
}

fn create(path: &Path) -> Result<IdentityKey> {
    let key = IdentityKey::generate().map_err(|err| anyhow::anyhow!("no identity key: {err}"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("cannot make {}", dir.display()))?;
    }
    std::fs::write(path, key.secret_bytes().as_slice())
        .with_context(|| format!("cannot write the identity key {}", path.display()))?;
    log::info!("net: new identity {} in {}", key.key_id().fingerprint(), path.display());
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_is_made_once_and_read_back_after() {
        let dir = std::env::temp_dir().join(format!("nfsmw-identity-{}", std::process::id()));
        let path = dir.join("keys").join("identity.key");
        let _ = std::fs::remove_dir_all(&dir);
        let made = load_or_create(&path).unwrap();
        let read = load_or_create(&path).unwrap();
        assert_eq!(made.key_id(), read.key_id());
        std::fs::write(&path, [1u8; 5]).unwrap();
        assert!(load_or_create(&path).is_err(), "a broken file is an error");
        assert_eq!(std::fs::read(&path).unwrap(), [1u8; 5], "and is left alone");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
