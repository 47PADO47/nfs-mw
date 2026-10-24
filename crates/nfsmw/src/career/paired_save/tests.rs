use super::*;

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "paired-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        )))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stage(profiles: &ProfileDir, old: &[u8], next: &[u8]) {
    let folder = profiles.mod_data_dir("PLAYER").unwrap();
    let pending = Pending {
        version: 1,
        extension: "garage.toml".into(),
        before_native: Some(fingerprint(old)),
        after_native: fingerprint(next),
        before: Some(b"old RGB".to_vec()),
        after: b"new RGB".to_vec(),
    };
    replace(&folder.join(JOURNAL), &serde_json::to_vec(&pending).unwrap()).unwrap();
    replace(&folder.join("garage.toml"), b"new RGB").unwrap();
}

#[test]
fn interrupted_rgb_purchase_restores_old_extension_before_any_other_save() {
    let scratch = Scratch::new();
    let profiles = ProfileDir::new(&scratch.0);
    profiles.write("PLAYER", b"same parts; cash 100").unwrap();
    stage(&profiles, b"same parts; cash 100", b"same parts; cash 80");
    profiles.write("PLAYER", b"unrelated autosave").unwrap();
    assert_eq!(fs::read(profiles.mod_data_dir("PLAYER").unwrap().join("garage.toml")).unwrap(), b"old RGB");
    assert_eq!(fs::read(profiles.path("PLAYER")).unwrap(), b"unrelated autosave");
}

#[test]
fn committed_rgb_purchase_recovers_new_extension_before_profile_read() {
    let scratch = Scratch::new();
    let profiles = ProfileDir::new(&scratch.0);
    profiles.write("PLAYER", b"cash 100").unwrap();
    stage(&profiles, b"cash 100", b"cash 80");
    profiles.write_native("PLAYER", b"cash 80").unwrap();
    let file = profiles.mod_data_dir("PLAYER").unwrap().join("garage.toml");
    fs::remove_file(&file).unwrap();
    // The fake native bytes fail load validation, but recovery must happen before validation.
    assert!(profiles.read("PLAYER").is_err());
    assert_eq!(fs::read(&file).unwrap(), b"new RGB");
    assert!(!file.parent().unwrap().join(JOURNAL).exists());
}

#[test]
fn failed_recovery_blocks_native_overwrite_and_can_retry() {
    let scratch = Scratch::new();
    let profiles = ProfileDir::new(&scratch.0);
    profiles.write("PLAYER", b"cash 100").unwrap();
    stage(&profiles, b"cash 100", b"cash 80");
    let file = profiles.mod_data_dir("PLAYER").unwrap().join("garage.toml");
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();
    assert!(profiles.write("PLAYER", b"unrelated autosave").is_err());
    assert_eq!(fs::read(profiles.path("PLAYER")).unwrap(), b"cash 100");
    assert!(file.parent().unwrap().join(JOURNAL).is_file());
    fs::remove_dir(&file).unwrap();
    recover(&profiles, "PLAYER").unwrap();
    assert_eq!(fs::read(&file).unwrap(), b"old RGB");
}

#[test]
fn paired_write_publishes_both_and_rejects_escaping_paths() {
    let scratch = Scratch::new();
    let profiles = ProfileDir::new(&scratch.0);
    profiles.write_with_extension("PLAYER", b"cash 80", "garage.toml", b"new RGB").unwrap();
    assert_eq!(fs::read(profiles.path("PLAYER")).unwrap(), b"cash 80");
    assert_eq!(fs::read(profiles.mod_data_dir("PLAYER").unwrap().join("garage.toml")).unwrap(), b"new RGB");
    for name in ["../garage.toml", "..\\garage.toml", "C:garage.toml", JOURNAL] {
        assert!(profiles.write_with_extension("PLAYER", b"bad", name, b"bad").is_err());
    }
}
