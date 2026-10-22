use super::*;
use crate::career::io::ProfileDir;

/// A folder under the temp dir, removed when the test ends.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(test: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("nfsmw-mod-data-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        Scratch { root }
    }

    fn blobs(&self) -> ModData {
        ModData::new(self.root.join("mods"))
    }

    fn blob_path(&self, key: &str) -> PathBuf {
        self.root.join("mods").join(format!("{key}.bin"))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn a_blob_round_trips_and_a_missing_one_is_none() {
    let scratch = Scratch::new("round-trip");
    let blobs = scratch.blobs();
    assert_eq!(blobs.read("example.hello").unwrap(), None);
    blobs.write("example.hello", b"cars: 3").unwrap();
    assert_eq!(blobs.read("example.hello").unwrap().as_deref(), Some(&b"cars: 3"[..]));
}

#[test]
fn an_empty_body_is_kept() {
    let scratch = Scratch::new("empty");
    let blobs = scratch.blobs();
    blobs.write("a", b"").unwrap();
    assert_eq!(blobs.read("a").unwrap(), Some(Vec::new()));
}

#[test]
fn a_write_replaces_the_blob_and_leaves_no_temporary_file() {
    let scratch = Scratch::new("replace");
    let blobs = scratch.blobs();
    blobs.write("a", b"first").unwrap();
    blobs.write("a", b"second").unwrap();
    assert_eq!(blobs.read("a").unwrap(), Some(b"second".to_vec()));
    assert!(!scratch.root.join("mods").join("a.bin.tmp").exists());
}

#[test]
fn a_body_over_the_limit_is_refused_and_nothing_is_written() {
    let scratch = Scratch::new("too-large");
    let blobs = scratch.blobs();
    let body = vec![0u8; MAX_BODY + 1];
    assert!(matches!(blobs.write("a", &body), Err(ModDataError::TooLarge(len)) if len == MAX_BODY + 1));
    assert!(!scratch.blob_path("a").exists());
}

#[test]
fn a_body_at_the_limit_is_kept() {
    let scratch = Scratch::new("at-limit");
    let blobs = scratch.blobs();
    let body = vec![7u8; MAX_BODY];
    blobs.write("a", &body).unwrap();
    assert_eq!(blobs.read("a").unwrap(), Some(body));
}

#[test]
fn keys_must_be_mod_ids() {
    let scratch = Scratch::new("keys");
    let blobs = scratch.blobs();
    let too_long = "k".repeat(65);
    let longest = "k".repeat(64);
    for bad in ["", ".hidden", "../x", "a/b", "a\\b", "Upper", "a b", "x:y", too_long.as_str()] {
        assert!(matches!(blobs.write(bad, b"x"), Err(ModDataError::BadKey(_))), "{bad:?} must be refused");
    }
    for good in ["a", "example.hello", "mod_name-2", longest.as_str()] {
        blobs.write(good, b"x").unwrap();
        assert_eq!(blobs.read(good).unwrap(), Some(b"x".to_vec()));
    }
}

#[test]
fn a_flipped_byte_is_corrupt_and_the_file_is_left_in_place() {
    let scratch = Scratch::new("flipped");
    let blobs = scratch.blobs();
    blobs.write("a", b"payload").unwrap();
    let path = scratch.blob_path("a");
    let mut file = fs::read(&path).unwrap();
    let last = file.len() - 1;
    file[last] ^= 0xFF;
    fs::write(&path, &file).unwrap();
    assert!(matches!(blobs.read("a"), Err(ModDataError::Corrupt)));
    assert!(path.exists(), "a corrupt blob is not deleted");
}

#[test]
fn a_valid_digest_over_a_bad_header_is_corrupt() {
    let scratch = Scratch::new("bad-header");
    let blobs = scratch.blobs();
    fs::create_dir_all(scratch.root.join("mods")).unwrap();
    let cases: Vec<Vec<u8>> = vec![
        // Wrong magic.
        frame(b"NOPE", 1, 1, b"x"),
        // Wrong version.
        frame(b"MODD", 2, 1, b"x"),
        // Length longer than the body.
        frame(b"MODD", 1, 5, b"x"),
        // Too short for the header.
        b"MODD".to_vec(),
    ];
    for body in cases {
        fs::write(scratch.blob_path("a"), digest::with_prefix(&body)).unwrap();
        assert!(matches!(blobs.read("a"), Err(ModDataError::Corrupt)), "{body:?}");
    }
}

/// A blob body with the given header fields, for the corrupt-header cases.
fn frame(magic: &[u8], version: u32, length: u64, body: &[u8]) -> Vec<u8> {
    let mut out = magic.to_vec();
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(body);
    out
}

#[test]
fn a_file_without_a_digest_is_corrupt() {
    let scratch = Scratch::new("no-digest");
    let blobs = scratch.blobs();
    fs::create_dir_all(scratch.root.join("mods")).unwrap();
    fs::write(scratch.blob_path("a"), b"short").unwrap();
    assert!(matches!(blobs.read("a"), Err(ModDataError::Corrupt)));
}

#[test]
fn remove_deletes_the_blob_and_a_missing_one_is_fine() {
    let scratch = Scratch::new("remove");
    let blobs = scratch.blobs();
    blobs.write("a", b"x").unwrap();
    blobs.remove("a").unwrap();
    assert_eq!(blobs.read("a").unwrap(), None);
    blobs.remove("a").unwrap();
}

#[test]
fn deleting_a_profile_deletes_its_mod_data() {
    let scratch = Scratch::new("profile-delete");
    let profiles = ProfileDir::new(&scratch.root);
    profiles.write("alias", b"profile").unwrap();
    let mods = ModData::new(profiles.mod_data_dir("alias").unwrap());
    mods.write("example.hello", b"data").unwrap();
    profiles.remove("alias").unwrap();
    assert!(!scratch.root.join("alias").exists(), "the profile folder is gone");

    profiles.write("alias", b"profile").unwrap();
    assert_eq!(mods.read("example.hello").unwrap(), None, "a new profile with the same name starts empty");
}
