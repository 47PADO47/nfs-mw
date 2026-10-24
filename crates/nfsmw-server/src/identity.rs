//! An identity key on disk (security.md section 3): the 32 secret bytes of the X25519 key, made the first time and read
//! every time after. A server that keeps its key is the same server to every client that pinned it (trust on first
//! use); a player that keeps theirs is known again by resume, bans and the host's own seat across runs.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use blackbox_net_core::secure::IdentityKey;

/// The length of the key file: the secret alone.
const KEY_FILE_LEN: usize = 32;

/// Why a key file could not be used.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("cannot read the identity key {path}: {source}")]
    Read { path: PathBuf, source: io::Error },
    /// The file is there but is not a key. It is left alone: it may be someone's key.
    #[error("the identity key {path} is broken: {reason}")]
    Broken { path: PathBuf, reason: String },
    #[error("cannot write the identity key {path}: {source}")]
    Write { path: PathBuf, source: io::Error },
    #[error("no identity key: {0}")]
    Generate(String),
}

/// The key in `path`, or a new one written there when the file does not exist. A file of the wrong size is an error
/// and is not overwritten.
pub fn load_or_create(path: &Path) -> Result<IdentityKey, IdentityError> {
    match std::fs::read(path) {
        Ok(bytes) => read(path, &bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => create(path),
        Err(source) => Err(IdentityError::Read { path: path.to_path_buf(), source }),
    }
}

fn read(path: &Path, bytes: &[u8]) -> Result<IdentityKey, IdentityError> {
    let broken = |reason: String| IdentityError::Broken { path: path.to_path_buf(), reason };
    let Ok(secret) = <&[u8; KEY_FILE_LEN]>::try_from(bytes) else {
        return Err(broken(format!("{} bytes, not {KEY_FILE_LEN}", bytes.len())));
    };
    IdentityKey::from_secret(secret).map_err(|err| broken(format!("not a key: {err}")))
}

fn create(path: &Path) -> Result<IdentityKey, IdentityError> {
    let write_error = |source: io::Error| IdentityError::Write { path: path.to_path_buf(), source };
    let key = IdentityKey::generate().map_err(|err| IdentityError::Generate(err.to_string()))?;
    if let Some(dir) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).map_err(write_error)?;
    }
    let mut file = private_file(path).map_err(write_error)?;
    let written = restrict_to_owner(path)
        .and_then(|()| file.write_all(key.secret_bytes().as_slice()))
        .and_then(|()| file.sync_all());
    if let Err(source) = written {
        // A half-written file would read as a broken key on every later start and block it.
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(write_error(source));
    }
    log::info!("new identity {} in {}", key.key_id().fingerprint(), path.display());
    Ok(key)
}

/// A new file only the owner can read (the key is a secret). Fails if the file appeared meanwhile. On unix the mode is
/// 0600; on Windows [`restrict_to_owner`] follows.
fn private_file(path: &Path) -> io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path)
}

/// Windows: the file keeps the access of its folder, which can include other users. The std library has no ACL call, so
/// the system's `icacls` drops the inherited access and grants the current user alone. An error means the key must not
/// be written. Nothing to do elsewhere (the mode of [`private_file`] is enough).
#[cfg(windows)]
fn restrict_to_owner(path: &Path) -> io::Result<()> {
    let user = std::env::var("USERNAME").map_err(|_| io::Error::other("USERNAME is not set"))?;
    let account = match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.is_empty() => format!("{domain}\\{user}"),
        _ => user,
    };
    let output = std::process::Command::new("icacls")
        .arg(path)
        .args(["/inheritance:r", "/grant:r"])
        .arg(format!("{account}:(F)"))
        .output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(io::Error::other(format!("icacls failed: {}", String::from_utf8_lossy(&output.stdout).trim())))
}

#[cfg(not(windows))]
fn restrict_to_owner(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nfsmw-server-identity-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_key_is_made_once_and_read_back_after() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("keys").join("server.key");
        let made = load_or_create(&path).expect("made");
        let again = load_or_create(&path).expect("read");
        assert_eq!(made.key_id(), again.key_id());
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn a_broken_file_is_an_error_and_is_left_alone() {
        let dir = temp_dir("broken");
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("server.key");
        std::fs::write(&path, [1u8; 5]).expect("write");
        assert!(matches!(load_or_create(&path), Err(IdentityError::Broken { .. })));
        assert_eq!(std::fs::read(&path).expect("read"), [1u8; 5]);
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn the_key_file_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("private");
        let path = dir.join("server.key");
        load_or_create(&path).expect("made");
        let mode = std::fs::metadata(&path).expect("metadata").permissions().mode();
        assert_eq!(mode & 0o077, 0, "no access for group or others: {mode:o}");
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn the_key_file_is_for_the_current_user_alone() {
        let dir = temp_dir("acl");
        let path = dir.join("server.key");
        load_or_create(&path).expect("made");
        let listing = std::process::Command::new("icacls").arg(&path).output().expect("icacls");
        let listing = String::from_utf8_lossy(&listing.stdout).to_lowercase();
        for group in ["everyone", "builtin\\users", "authenticated users"] {
            assert!(!listing.contains(group), "{group} can reach the key: {listing}");
        }
        assert!(listing.contains(&std::env::var("USERNAME").expect("USERNAME").to_lowercase()), "{listing}");
        std::fs::remove_dir_all(&dir).expect("cleanup");
    }
}
