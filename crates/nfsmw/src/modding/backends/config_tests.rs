//! `bb.config` on the game side: the `[mods.<id>]` table of the config file reaches the host, in both shapes of `mods`, and a
//! missing or broken file is "no table".

use std::fs;

use blackbox_modding::host::ConfigBackend;

use super::config::ConfigPort;

fn file_with(name: &str, text: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!("nfsmw-config-port-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("settings.toml");
    fs::write(&path, text).unwrap();
    (dir, path)
}

fn parsed(text: Option<String>) -> toml::Table {
    text.expect("a table").parse().expect("valid TOML")
}

#[test]
fn a_mods_table_reaches_the_host_with_its_sub_tables() {
    let text = "master_volume = 70\n[mods]\nmode = \"all\"\n[mods.nfsmw-2012-content]\nsource = 'D:/x'\n[mods.nfsmw-2012-content.police]\nauto = false\n";
    let (dir, path) = file_with("table", text);
    let section = parsed(ConfigPort::at(&path).main_section("nfsmw-2012-content").unwrap());
    assert_eq!(section.get("source").and_then(toml::Value::as_str), Some("D:/x"));
    assert!(section.get("police").is_some_and(toml::Value::is_table));
    assert!(!section.contains_key("mode"), "the table holds the mod's keys, not the mode");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_dotted_id_is_found_when_it_is_quoted() {
    let (dir, path) = file_with("dotted", "[mods.\"my.mod\"]\nlevel = 3\n");
    let section = parsed(ConfigPort::at(&path).main_section("my.mod").unwrap());
    assert_eq!(section.get("level").and_then(toml::Value::as_integer), Some(3));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn the_legacy_string_shape_and_other_mods_have_no_table() {
    let (dir, path) = file_with("legacy", "mods = \"all\"\n");
    assert_eq!(ConfigPort::at(&path).main_section("a-mod").unwrap(), None);
    let _ = fs::remove_dir_all(dir);
    let (dir, path) = file_with("other", "[mods.b-mod]\nx = 1\n");
    assert_eq!(ConfigPort::at(&path).main_section("a-mod").unwrap(), None);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn an_empty_table_is_still_a_table() {
    let (dir, path) = file_with("empty", "[mods.a-mod]\n");
    assert_eq!(ConfigPort::at(&path).main_section("a-mod").unwrap().as_deref(), Some(""));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_missing_or_broken_file_is_no_table() {
    let (dir, path) = file_with("broken", "not = [toml");
    assert_eq!(ConfigPort::at(&path).main_section("a-mod").unwrap(), None);
    assert_eq!(ConfigPort::at(&dir.join("absent.toml")).main_section("a-mod").unwrap(), None);
    assert_eq!(ConfigPort { path: None }.main_section("a-mod").unwrap(), None);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_later_edit_of_the_file_is_seen_by_the_next_call() {
    let (dir, path) = file_with("reload", "[mods.a-mod]\nx = 1\n");
    let port = ConfigPort::at(&path);
    assert!(port.main_section("a-mod").unwrap().unwrap().contains("x = 1"));
    fs::write(&path, "[mods.a-mod]\nx = 2\n").unwrap();
    assert!(port.main_section("a-mod").unwrap().unwrap().contains("x = 2"));
    let _ = fs::remove_dir_all(dir);
}
