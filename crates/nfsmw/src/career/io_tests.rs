use super::*;
use nfsmw_career::save::ProfileSave;
use nfsmw_career::save::layout::PROFILE_LEN;
use std::time::Duration;

/// A temp folder under the system temp dir, unique to this process and test, removed on drop.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("nfsmw-career-io-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Scratch { root }
    }

    fn dir(&self) -> ProfileDir {
        ProfileDir::new(self.root.clone())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A profile file the loader accepts: a zeroed profile buffer wrapped in a fresh container.
fn valid_file() -> Vec<u8> {
    let buffer = vec![0u8; PROFILE_LEN];
    ProfileSave::from_profile_buffer(&buffer).unwrap().to_bytes()
}

#[test]
fn a_valid_profile_round_trips() {
    let scratch = Scratch::new("round-trip");
    let dir = scratch.dir();
    let bytes = valid_file();
    dir.write("mwo", &bytes).unwrap();
    assert_eq!(dir.read("mwo").unwrap(), bytes);
    assert_eq!(dir.path("mwo"), scratch.root.join("mwo").join("mwo"));
    assert!(dir.path("mwo").is_file());
    assert!(!scratch.root.join("mwo").join("mwo.tmp").exists(), "the temporary file is renamed away");
}

#[test]
fn a_second_write_replaces_the_first() {
    let scratch = Scratch::new("replace");
    let dir = scratch.dir();
    dir.write("mwo", b"first").unwrap();
    let bytes = valid_file();
    dir.write("mwo", &bytes).unwrap();
    assert_eq!(dir.read("mwo").unwrap(), bytes);
}

#[test]
fn a_refused_file_is_an_error_not_bytes() {
    let scratch = Scratch::new("garbage");
    let dir = scratch.dir();
    dir.write("mwo", b"not a profile").unwrap();
    let err = dir.read("mwo").unwrap_err();
    assert!(err.to_string().contains("refused"), "{err}");
    assert!(dir.read("missing").is_err(), "a missing profile is an error");
}

#[test]
fn aliases_list_only_listable_folders_with_a_profile() {
    let scratch = Scratch::new("aliases");
    let dir = scratch.dir();
    dir.write("stock", b"x").unwrap();
    dir.write("mwo", b"x").unwrap();
    dir.write("toolongname", b"x").unwrap();
    dir.write("my name", b"x").unwrap();
    fs::create_dir_all(scratch.root.join("empty")).unwrap();
    fs::write(scratch.root.join("Settings.ini"), b"x").unwrap();
    assert_eq!(dir.aliases(), ["mwo", "stock"]);
}

#[test]
fn aliases_of_a_missing_root_are_empty() {
    let scratch = Scratch::new("missing-root");
    assert!(scratch.dir().aliases().is_empty());
    assert_eq!(scratch.dir().latest(), None);
}

#[test]
fn latest_is_the_profile_written_last() {
    let scratch = Scratch::new("latest");
    let dir = scratch.dir();
    dir.write("bbb", b"x").unwrap();
    std::thread::sleep(Duration::from_millis(50));
    dir.write("aaa", b"x").unwrap();
    assert_eq!(dir.latest().as_deref(), Some("aaa"));
    std::thread::sleep(Duration::from_millis(50));
    dir.write("bbb", b"y").unwrap();
    assert_eq!(dir.latest().as_deref(), Some("bbb"));
}

#[test]
fn invalid_aliases_are_refused_and_nothing_leaves_the_root() {
    let scratch = Scratch::new("invalid");
    let dir = scratch.dir();
    let escape = scratch.root.parent().unwrap().join(format!("nfsmw-io-escape-{}", std::process::id()));
    let names = [
        String::new(),
        "a/b".to_string(),
        "a\\b".to_string(),
        "..".to_string(),
        "a..b".to_string(),
        "C:x".to_string(),
        format!("../nfsmw-io-escape-{}", std::process::id()),
        "a".repeat(32),
    ];
    for name in &names {
        assert!(dir.write(name, b"x").is_err(), "write refuses {name:?}");
        assert!(dir.read(name).is_err(), "read refuses {name:?}");
        assert!(dir.remove(name).is_err(), "remove refuses {name:?}");
    }
    assert!(!escape.exists(), "no file outside the root");
    assert!(!scratch.root.exists(), "a refused write creates no folder");
}

#[test]
fn the_longest_alias_is_31_characters() {
    let scratch = Scratch::new("length");
    let dir = scratch.dir();
    let longest = "b".repeat(ALIAS_MAX_CHARS);
    dir.write(&longest, b"x").unwrap();
    assert!(dir.path(&longest).is_file());
    assert!(dir.write(&"b".repeat(ALIAS_MAX_CHARS + 1), b"x").is_err());
}

#[test]
fn remove_deletes_the_profile_and_its_folder() {
    let scratch = Scratch::new("remove");
    let dir = scratch.dir();
    dir.write("mwo", b"x").unwrap();
    dir.remove("mwo").unwrap();
    assert!(!dir.path("mwo").exists());
    assert!(!scratch.root.join("mwo").exists(), "the empty folder goes too");
    assert!(dir.aliases().is_empty());
    assert!(dir.remove("mwo").is_err(), "removing twice is an error");
}
