use super::*;
fn path(name: &str) -> std::path::PathBuf {
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("nfsmw-presets-{name}-{}-{stamp}", std::process::id())).join("presets.toml")
}

#[test]
fn presets_replace_only_the_named_entry_and_keep_other_data() {
    let path = path("replace");
    assert!(names(&path).unwrap().is_empty());
    save(&path, " Rain / HDR ", "first").unwrap();
    save(&path, "Track", "second").unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, format!("future=42\n{text}")).unwrap();
    save(&path, "Rain / HDR", "replacement").unwrap();
    assert_eq!(load(&path, "Rain / HDR").unwrap(), "replacement");
    assert_eq!(load(&path, "Track").unwrap(), "second");
    assert_eq!(names(&path).unwrap(), ["Rain / HDR", "Track"]);
    assert_eq!(read(&path).unwrap()["future"].as_integer(), Some(42));
    assert!(!path.with_extension("toml.tmp").exists());
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(path.parent().unwrap()).unwrap();
}

#[test]
fn malformed_and_future_files_and_invalid_names_are_never_overwritten() {
    let path = path("invalid");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    for text in ["bad = [", "version=99", "version=1\npresets=3"] {
        std::fs::write(&path, text).unwrap();
        assert!(save(&path, "name", "value").is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }
    for name in ["", "   ", "line\nbreak", &"x".repeat(65)] {
        assert!(save(&path, name, "value").is_err());
    }
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(path.parent().unwrap()).unwrap();
}

#[test]
fn rename_and_delete_preserve_other_saved_setups() {
    let dir = std::env::temp_dir().join(format!("nfsmw-preset-management-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("presets.toml");
    super::save(&path, "Supra", "one").unwrap();
    super::save(&path, "Porsche", "two").unwrap();
    assert!(super::rename(&path, "Supra", "Porsche").is_err());
    super::rename(&path, "Supra", "Toyota").unwrap();
    assert_eq!(super::load(&path, "Toyota").unwrap(), "one");
    super::delete(&path, "Toyota").unwrap();
    assert_eq!(super::names(&path).unwrap(), vec!["Porsche"]);
    assert_eq!(super::load(&path, "Porsche").unwrap(), "two");
    std::fs::remove_dir_all(dir).unwrap();
}
