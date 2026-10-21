use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use nfsmw_career::save::ProfileSave;
use nfsmw_career::save::layout::PROFILE_LEN;

use super::*;

/// A temp folder under the system temp dir, unique to this process and test, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(test: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("nfsmw-career-sources-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Scratch(root)
    }

    fn dir(&self, name: &str) -> ProfileDir {
        ProfileDir::new(self.0.join(name))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A profile file the loader accepts, told apart by `tag` in the unused end of the buffer.
fn file(tag: u8) -> Vec<u8> {
    let mut buffer = vec![0u8; PROFILE_LEN];
    buffer[PROFILE_LEN - 1] = tag;
    ProfileSave::from_profile_buffer(&buffer).unwrap().to_bytes()
}

#[test]
fn both_folders_are_listed_once_each_and_sorted_without_case() {
    let scratch = Scratch::new("union");
    let (own, original) = (scratch.dir("own"), scratch.dir("original"));
    own.write("stock", &file(1)).unwrap();
    original.write("NAME", &file(2)).unwrap();
    original.write("mwo", &file(3)).unwrap();
    original.write("STOCK", &file(4)).unwrap();
    let aliases = Sources::new(Some(own), Some(original)).aliases();
    // The same alias in both folders is listed once.
    assert_eq!(aliases.iter().filter(|a| a.eq_ignore_ascii_case("stock")).count(), 1, "{aliases:?}");
    assert_eq!(aliases[0], "mwo");
    assert_eq!(aliases[1], "NAME");
}

#[test]
fn a_folder_with_no_profile_gives_an_empty_list() {
    let scratch = Scratch::new("empty");
    let sources = Sources::new(Some(scratch.dir("own")), Some(scratch.dir("original")));
    assert!(sources.is_empty());
    assert!(Sources::new(None, None).is_empty());
}

#[test]
fn an_alias_only_the_original_has_is_read_from_the_original() {
    let scratch = Scratch::new("original-read");
    let (own, original) = (scratch.dir("own"), scratch.dir("original"));
    let bytes = file(7);
    original.write("mwo", &bytes).unwrap();
    let sources = Sources::new(Some(own), Some(original));
    assert_eq!(sources.read("mwo").unwrap(), bytes);
}

#[test]
fn this_builds_copy_hides_the_originals() {
    let scratch = Scratch::new("own-wins");
    let (own, original) = (scratch.dir("own"), scratch.dir("original"));
    own.write("mwo", &file(1)).unwrap();
    original.write("mwo", &file(2)).unwrap();
    let sources = Sources::new(Some(own), Some(original));
    assert_eq!(sources.read("mwo").unwrap(), file(1));
}

#[test]
fn listing_and_reading_change_nothing_on_disk() {
    let scratch = Scratch::new("read-only");
    let (own, original) = (scratch.dir("own"), scratch.dir("original"));
    original.write("mwo", &file(2)).unwrap();
    let before = fs::read(original.path("mwo")).unwrap();
    let sources = Sources::new(Some(own.clone()), Some(original.clone()));
    sources.aliases();
    sources.read("mwo").unwrap();
    assert_eq!(fs::read(original.path("mwo")).unwrap(), before);
    assert!(!own.root().exists(), "nothing is created in this build's folder either");
}

#[test]
fn a_missing_alias_is_an_error() {
    let scratch = Scratch::new("missing");
    let sources = Sources::new(Some(scratch.dir("own")), Some(scratch.dir("original")));
    assert!(sources.read("nobody").is_err());
    assert!(Sources::new(None, None).read("nobody").is_err());
}

#[test]
fn the_original_folder_is_documents_nfs_most_wanted_unless_given() {
    let user = Some(OsString::from("C:/Users/me"));
    let dir = original_dir_from(None, user.clone()).unwrap();
    assert_eq!(dir.root(), PathBuf::from("C:/Users/me").join("Documents").join("NFS Most Wanted"));
    let given = original_dir_from(Some(OsString::from("D:/saves")), user).unwrap();
    assert_eq!(given.root(), PathBuf::from("D:/saves"));
}

#[test]
fn an_empty_override_turns_the_originals_off() {
    assert_eq!(original_dir_from(Some(OsString::new()), Some(OsString::from("C:/Users/me"))), None);
    assert_eq!(original_dir_from(None, None), None);
}
