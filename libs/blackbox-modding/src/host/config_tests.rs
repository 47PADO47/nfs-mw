//! The merge rules and the file lookup of a mod's configuration: deep merge, replaced arrays, a scalar against a table,
//! the sources bits, the paths that must stay inside the mod folder.

use toml::Table;

use super::config::{ConfigError, DEFAULT_FILE, check_relative, effective, load, merge, resolve_file};
use super::test_support::Scratch;
use crate::abi::{BB_CONFIG_FILE, BB_CONFIG_MAIN};

fn table(text: &str) -> Table {
    text.parse().expect("test TOML")
}

#[test]
fn a_relative_path_inside_the_folder_is_accepted() {
    for ok in ["config.toml", "settings/mod.toml", "a/b/c.toml", "x.y.toml"] {
        assert_eq!(check_relative(ok), Ok(()), "{ok}");
    }
}

#[test]
fn paths_that_leave_the_folder_or_are_not_plain_are_refused() {
    for bad in [
        "",
        "../config.toml",
        "a/../../config.toml",
        "a/./b.toml",
        "./config.toml",
        "/etc/config.toml",
        "C:/config.toml",
        "C:config.toml",
        "\\\\server\\share\\config.toml",
        "sub\\config.toml",
        "a//b.toml",
        "dir/",
        "file:config.toml",
    ] {
        assert!(check_relative(bad).is_err(), "{bad:?} must be refused");
    }
}

#[test]
fn tables_merge_key_by_key_and_the_second_wins() {
    let mut base = table("a = 1\nb = 'keep'\n[t]\nx = 1\ny = 2\n[t.deep]\nk = 'file'\nj = 'file'\n");
    merge(&mut base, table("a = 2\nc = true\n[t]\ny = 3\n[t.deep]\nk = 'main'\n"));
    assert_eq!(base, table("a = 2\nb = 'keep'\nc = true\n[t]\nx = 1\ny = 3\n[t.deep]\nk = 'main'\nj = 'file'\n"));
}

#[test]
fn an_array_is_replaced_whole() {
    let mut base = table("cars = ['a', 'b', 'c']\n[police]\nadd = ['x']\n");
    merge(&mut base, table("cars = ['z']\n[police]\nadd = []\n"));
    assert_eq!(base, table("cars = ['z']\n[police]\nadd = []\n"));
}

#[test]
fn a_scalar_and_a_table_replace_each_other() {
    let mut base = table("a = 1\n[b]\nx = 1\n");
    merge(&mut base, table("[a]\nx = 2\n\n[b]\n"));
    assert_eq!(
        base,
        table("[a]\nx = 2\n\n[b]\nx = 1\n"),
        "a table over a scalar replaces it, an empty table adds nothing"
    );
    let mut base = table("[b]\nx = 1\n");
    merge(&mut base, table("b = 'text'\n"));
    assert_eq!(base, table("b = 'text'\n"), "a scalar over a table replaces the whole table");
}

#[test]
fn no_source_gives_an_empty_text_and_no_bits() {
    let none = effective(None, None).expect("nothing to merge");
    assert_eq!((none.text.as_str(), none.sources), ("", 0));
}

#[test]
fn the_bits_say_which_sources_exist() {
    assert_eq!(effective(Some("a = 1"), None).unwrap().sources, BB_CONFIG_FILE);
    assert_eq!(effective(None, Some("a = 1")).unwrap().sources, BB_CONFIG_MAIN);
    assert_eq!(effective(Some(""), Some("")).unwrap().sources, BB_CONFIG_FILE | BB_CONFIG_MAIN, "empty is not absent");
}

#[test]
fn the_main_config_wins_over_the_file_and_the_text_parses_back() {
    let merged = effective(
        Some("source = 'file'\nrole = 'player'\n[performance]\npower = 1.0\n"),
        Some("source = 'main'\n[performance]\ngrip = 2.0\n"),
    )
    .expect("merged");
    assert_eq!(
        table(&merged.text),
        table("source = 'main'\nrole = 'player'\n[performance]\npower = 1.0\ngrip = 2.0\n")
    );
}

#[test]
fn a_source_that_is_not_toml_names_itself() {
    let file = effective(Some("not = [toml"), None).unwrap_err();
    assert!(matches!(file, ConfigError::Parse { source_name, .. } if source_name.contains("config file")), "{file}");
    let main = effective(None, Some("not = [toml")).unwrap_err();
    assert!(matches!(main, ConfigError::Parse { source_name, .. } if source_name.contains("main config")), "{main}");
}

#[test]
fn the_default_file_is_config_toml_and_a_missing_one_is_none() {
    let scratch = Scratch::new("config-default");
    let dir = scratch.folder("mod");
    assert_eq!(resolve_file(&dir, None), Ok(None));
    scratch.write("mod/config.toml", b"a = 1\n");
    let found = resolve_file(&dir, None).unwrap().expect("config.toml is found");
    assert!(found.ends_with(DEFAULT_FILE));
}

#[test]
fn the_named_file_replaces_the_default() {
    let scratch = Scratch::new("config-named");
    let dir = scratch.folder("mod");
    scratch.folder("mod/settings");
    scratch.write("mod/config.toml", b"a = 'default'\n");
    scratch.write("mod/settings/mine.toml", b"a = 'named'\n");
    let both = load(&dir, Some("settings/mine.toml"), None).unwrap();
    assert_eq!(table(&both.text), table("a = 'named'\n"));
    assert_eq!(
        load(&dir, Some("settings/none.toml"), None).unwrap().sources,
        0,
        "a named file that is missing is absent"
    );
}

#[test]
fn a_path_that_steps_out_is_an_error_not_a_missing_file() {
    let scratch = Scratch::new("config-escape");
    let dir = scratch.folder("mod");
    scratch.write("secret.toml", b"a = 1\n");
    let error = resolve_file(&dir, Some("../secret.toml")).unwrap_err();
    assert!(matches!(error, ConfigError::BadPath { .. }), "{error}");
}

#[cfg(unix)]
#[test]
fn a_link_that_leads_out_of_the_folder_is_refused() {
    let scratch = Scratch::new("config-link");
    let dir = scratch.folder("mod");
    let secret = scratch.write("secret.toml", b"a = 1\n");
    std::os::unix::fs::symlink(&secret, dir.join("config.toml")).expect("symlink");
    let error = resolve_file(&dir, None).unwrap_err();
    assert!(matches!(error, ConfigError::Escapes(_)), "{error}");
}

#[test]
fn the_file_and_the_main_table_merge_end_to_end() {
    let scratch = Scratch::new("config-load");
    let dir = scratch.folder("mod");
    scratch.write("mod/config.toml", b"cars = 'all'\nmax_texture_size = 1024\n[police]\nauto = true\n");
    let merged = load(&dir, None, Some("max_texture_size = 512\n[police]\nadd = ['Camaro']\n")).unwrap();
    assert_eq!(merged.sources, BB_CONFIG_FILE | BB_CONFIG_MAIN);
    assert_eq!(
        table(&merged.text),
        table("cars = 'all'\nmax_texture_size = 512\n[police]\nauto = true\nadd = ['Camaro']\n")
    );
}

#[test]
fn a_broken_file_is_an_error_even_when_the_main_table_is_fine() {
    let scratch = Scratch::new("config-broken");
    let dir = scratch.folder("mod");
    scratch.write("mod/config.toml", b"not = [toml");
    assert!(load(&dir, None, Some("a = 1")).is_err());
}
