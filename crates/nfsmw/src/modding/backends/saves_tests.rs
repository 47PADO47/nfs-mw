use std::fs;
use std::path::{Path, PathBuf};

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_CORRUPT, BB_ERR_NOT_FOUND, BB_ERR_NOT_READY};
use blackbox_modding::host::SavesBackend;

use super::saves::SavesPort;
use crate::career::ProfileDir;

/// A profile folder under the temp dir, removed when the test ends.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("nfsmw-saves-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Scratch { root }
    }

    fn profiles(&self) -> ProfileDir {
        ProfileDir::new(&self.root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A port over `root` with `alias` loaded.
fn port(root: &Path, alias: &str) -> SavesPort {
    let alias = alias.to_owned();
    SavesPort::with_loader(Some(ProfileDir::new(root)), Box::new(move || Some(alias.clone())))
}

#[test]
fn a_write_is_read_back_before_the_profile_is_saved_and_is_not_on_disk_yet() {
    let scratch = Scratch::new("staged-read");
    let port = port(&scratch.root, "staged1");
    port.write("example.hello", b"cars: 3").unwrap();
    assert_eq!(port.read("example.hello").unwrap(), b"cars: 3");
    assert!(!scratch.root.join("staged1").join("mods").join("example.hello.bin").exists());
}

#[test]
fn the_profile_save_writes_the_blob_and_then_the_profile() {
    let scratch = Scratch::new("save-writes");
    let port = port(&scratch.root, "saved1");
    port.write("example.hello", b"cars: 3").unwrap();
    scratch.profiles().save("saved1", b"profile bytes").unwrap();
    assert!(scratch.root.join("saved1").join("saved1").exists(), "the profile is written");
    assert_eq!(port.read("example.hello").unwrap(), b"cars: 3", "read from the file after the save");
}

#[test]
fn a_removal_is_staged_until_the_profile_save() {
    let scratch = Scratch::new("removal");
    let port = port(&scratch.root, "removed1");
    port.write("example.hello", b"cars: 3").unwrap();
    scratch.profiles().save("removed1", b"profile").unwrap();
    port.remove("example.hello").unwrap();
    assert_eq!(port.read("example.hello"), Err(BB_ERR_NOT_FOUND), "a staged removal reads as missing");
    assert!(scratch.root.join("removed1").join("mods").join("example.hello.bin").exists(), "still on disk");
    scratch.profiles().save("removed1", b"profile").unwrap();
    assert_eq!(port.read("example.hello"), Err(BB_ERR_NOT_FOUND));
    assert!(!scratch.root.join("removed1").join("mods").join("example.hello.bin").exists());
}

#[test]
fn a_mod_sees_only_its_own_blob() {
    let scratch = Scratch::new("own-only");
    let port = port(&scratch.root, "owner1");
    port.write("example.one", b"one").unwrap();
    assert_eq!(port.read("example.two"), Err(BB_ERR_NOT_FOUND));
    assert_eq!(port.read("example.one").unwrap(), b"one");
}

#[test]
fn with_no_profile_loaded_every_call_is_not_ready() {
    let scratch = Scratch::new("no-profile");
    let port = SavesPort::with_loader(Some(scratch.profiles()), Box::new(|| None));
    assert_eq!(port.write("example.hello", b"x"), Err(BB_ERR_NOT_READY));
    assert_eq!(port.read("example.hello"), Err(BB_ERR_NOT_READY));
    assert_eq!(port.remove("example.hello"), Err(BB_ERR_NOT_READY));
}

#[test]
fn a_profile_with_no_folder_is_not_ready_for_a_read() {
    let port = SavesPort::with_loader(None, Box::new(|| Some("nofolder".to_owned())));
    assert_eq!(port.read("example.hello"), Err(BB_ERR_NOT_READY));
}

#[test]
fn a_bad_owner_or_an_over_long_body_is_an_argument_error() {
    let scratch = Scratch::new("bad-args");
    let port = port(&scratch.root, "args1");
    assert_eq!(port.write("../escape", b"x"), Err(BB_ERR_ARG));
    let body = vec![0u8; blackbox_modding::abi::MAX_BLOB_BYTES as usize + 1];
    assert_eq!(port.write("example.hello", &body), Err(BB_ERR_ARG));
}

#[test]
fn a_corrupt_blob_is_reported_and_left_in_place() {
    let scratch = Scratch::new("corrupt");
    let port = port(&scratch.root, "corrupt1");
    let mods = scratch.root.join("corrupt1").join("mods");
    fs::create_dir_all(&mods).unwrap();
    fs::write(mods.join("example.hello.bin"), b"not a blob").unwrap();
    assert_eq!(port.read("example.hello"), Err(BB_ERR_CORRUPT));
    assert!(mods.join("example.hello.bin").exists());
}

#[test]
fn a_failed_blob_write_stops_the_profile_save() {
    let scratch = Scratch::new("failed-write");
    let port = port(&scratch.root, "failed1");
    // A file where the `mods` folder must go: the blob cannot be written.
    fs::create_dir_all(scratch.root.join("failed1")).unwrap();
    fs::write(scratch.root.join("failed1").join("mods"), b"in the way").unwrap();
    port.write("example.hello", b"x").unwrap();
    assert!(scratch.profiles().save("failed1", b"profile").is_err());
    assert!(!scratch.root.join("failed1").join("failed1").exists(), "the profile is not written");
    assert_eq!(port.read("example.hello").unwrap(), b"x", "the change stays staged for the next save");
}
