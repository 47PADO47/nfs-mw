use super::*;
use nfsmw_career::stable::CustomizationRecord;

struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "garage-purchase-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn refused_native_write_restores_previous_rgb_extension_exactly() {
    let scratch = Scratch::new();
    let profiles = ProfileDir::new(&scratch.0);
    let record = CustomizationRecord::default();
    let mut selection = Selection { car: "CAR".into(), color: Some([10, 20, 30]), ..Default::default() };
    save_extension(&profiles, "PLAYER", 42, &record, &selection).unwrap();
    let file = path(&profiles, "PLAYER").unwrap();
    let before = std::fs::read(&file).unwrap();
    // A directory at the native file path makes its atomic replacement fail on every platform.
    std::fs::create_dir_all(profiles.path("PLAYER")).unwrap();
    selection.color = Some([99, 88, 77]);
    assert!(save_purchase(&profiles, "PLAYER", 42, &record, &selection, b"native").is_err());
    assert_eq!(std::fs::read(&file).unwrap(), before);
}

#[test]
fn refused_first_native_write_leaves_no_extension_and_success_writes_both() {
    let scratch = Scratch::new();
    let profiles = ProfileDir::new(&scratch.0);
    let record = CustomizationRecord::default();
    let selection = Selection { car: "CAR".into(), ..Default::default() };
    std::fs::create_dir_all(profiles.path("PLAYER")).unwrap();
    assert!(save_purchase(&profiles, "PLAYER", 42, &record, &selection, b"native").is_err());
    assert!(!path(&profiles, "PLAYER").unwrap().exists());
    std::fs::remove_dir(profiles.path("PLAYER")).unwrap();
    save_purchase(&profiles, "PLAYER", 42, &record, &selection, b"native").unwrap();
    assert_eq!(std::fs::read(profiles.path("PLAYER")).unwrap(), b"native");
    assert!(path(&profiles, "PLAYER").unwrap().is_file());
}
