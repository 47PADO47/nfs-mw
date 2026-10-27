//! The two shapes of the `mods` key: the legacy string and the `[mods]` table with the mode, the lists and one table per mod.
//! Both must read, and a rewrite of the file must keep the mods' own tables.

use toml::{Table, Value};

use super::mods::{ALLOWLIST_KEY, DISABLED_KEY, ModsMode, merge_list, mod_section, parse_list, read_mode};
use super::{Partial, file, write};

fn table(text: &str) -> Table {
    text.parse().expect("test TOML")
}

const TABLE_SHAPE: &str = r#"
master_volume = 70

[mods]
mode = "allowlist"
allowlist = ["a-mod"]
disabled = ["b-mod"]

[mods.a-mod]
source = 'D:/Game'
[mods.a-mod.police]
auto = false

[mods."my.mod"]
level = 3
"#;

#[test]
fn the_mode_reads_from_the_string_and_from_the_table() {
    assert_eq!(read_mode(&table("mods = \"off\""), "test"), Some(ModsMode::Off));
    assert_eq!(read_mode(&table(TABLE_SHAPE), "test"), Some(ModsMode::Allowlist));
    assert_eq!(read_mode(&table("[mods]\nmode = \"all\""), "test"), Some(ModsMode::All));
}

#[test]
fn a_table_without_a_mode_or_a_bad_mode_sets_none() {
    assert_eq!(read_mode(&table("[mods.a-mod]\nx = 1"), "test"), None, "only mod tables: no mode");
    assert_eq!(read_mode(&table("[mods]\nmode = \"some\""), "test"), None);
    assert_eq!(read_mode(&table("[mods]\nmode = 3"), "test"), None);
    assert_eq!(read_mode(&table("mods = 3"), "test"), None);
    assert_eq!(read_mode(&table("master_volume = 1"), "test"), None);
}

#[test]
fn the_file_layer_reads_the_mode_in_both_shapes() {
    assert_eq!(file::parse("mods = \"off\"\n", "test").mods, Some(ModsMode::Off));
    assert_eq!(file::parse(TABLE_SHAPE, "test").mods, Some(ModsMode::Allowlist));
    assert_eq!(file::parse(TABLE_SHAPE, "test").master_volume, Some(super::Percent(70)), "other keys still read");
}

#[test]
fn the_lists_read_from_the_table() {
    assert_eq!(parse_list(TABLE_SHAPE, ALLOWLIST_KEY), ["a-mod"]);
    assert_eq!(parse_list(TABLE_SHAPE, DISABLED_KEY), ["b-mod"]);
}

#[test]
fn the_tables_list_wins_over_the_legacy_key_and_the_legacy_key_is_the_fallback() {
    let text = "mods_allowlist = [\"legacy\"]\nmods_disabled = [\"legacy-off\"]\n[mods]\nallowlist = [\"table\"]\n";
    assert_eq!(parse_list(text, ALLOWLIST_KEY), ["table"], "the table wins");
    assert_eq!(parse_list(text, DISABLED_KEY), ["legacy-off"], "no `disabled` in the table: the legacy key is used");
    let empty = "mods_disabled = [\"legacy-off\"]\n[mods]\ndisabled = []\n";
    assert!(parse_list(empty, DISABLED_KEY).is_empty(), "an empty list in the table is still the table's answer");
    assert_eq!(
        parse_list("mods_allowlist = [\"x\"]\nmods = \"allowlist\"\n", ALLOWLIST_KEY),
        ["x"],
        "the string shape"
    );
}

#[test]
fn the_mod_manager_writes_into_the_table_when_the_table_is_there() {
    let text = merge_list(TABLE_SHAPE, DISABLED_KEY, &["c-mod".to_owned()]).unwrap();
    assert_eq!(parse_list(&text, DISABLED_KEY), ["c-mod"]);
    let parsed = table(&text);
    assert!(parsed.get("mods_disabled").is_none(), "no legacy key is added next to a [mods] table");
    assert_eq!(read_mode(&parsed, "test"), Some(ModsMode::Allowlist));
    assert!(mod_section(&text, "a-mod").is_some() && mod_section(&text, "my.mod").is_some(), "the mods' tables stay");
}

#[test]
fn the_mod_manager_writes_the_legacy_key_for_the_string_shape() {
    let text = merge_list("mods = \"all\"\n", DISABLED_KEY, &["c-mod".to_owned()]).unwrap();
    let parsed = table(&text);
    assert_eq!(parsed.get("mods").and_then(Value::as_str), Some("all"));
    assert_eq!(parse_list(&text, DISABLED_KEY), ["c-mod"]);
}

#[test]
fn a_mods_section_is_found_by_its_exact_id() {
    let a = mod_section(TABLE_SHAPE, "a-mod").expect("a-mod has a table");
    assert_eq!(a.get("source").and_then(Value::as_str), Some("D:/Game"));
    assert!(a.get("police").is_some_and(Value::is_table), "sub-tables come along");
    let dotted = mod_section(TABLE_SHAPE, "my.mod").expect("a quoted id with a dot");
    assert_eq!(dotted.get("level").and_then(Value::as_integer), Some(3));
    assert!(mod_section(TABLE_SHAPE, "my").is_none(), "an id is not split at its dots");
    assert!(mod_section(TABLE_SHAPE, "mode").is_none(), "a plain key is not a mod's table");
    assert!(mod_section("mods = \"all\"\n", "a-mod").is_none());
    assert!(mod_section("not = [toml", "a-mod").is_none());
}

#[test]
fn an_unquoted_dotted_id_nests_and_is_not_the_dotted_mod() {
    let text = "[mods.my.mod]\nlevel = 3\n";
    assert!(mod_section(text, "my.mod").is_none());
    assert!(mod_section(text, "my").is_some(), "TOML reads it as mod `my` with a table `mod`");
}

#[test]
fn saving_other_settings_keeps_the_mods_tables_in_both_shapes() {
    let changes = Partial { vsync: Some(false), ..Partial::default() };
    let text = write::merge(TABLE_SHAPE, &changes).unwrap();
    let kept = table(&text);
    assert_eq!(kept.get("vsync").and_then(Value::as_bool), Some(false));
    assert_eq!(kept.get("mods"), table(TABLE_SHAPE).get("mods"), "the whole [mods] table is unchanged");
    assert_eq!(file::parse(&text, "test").mods, Some(ModsMode::Allowlist));
    let legacy = write::merge("mods = \"off\"\nmods_allowlist = [\"x\"]\n", &changes).unwrap();
    let kept = table(&legacy);
    assert_eq!(kept.get("mods").and_then(Value::as_str), Some("off"));
    assert_eq!(parse_list(&legacy, ALLOWLIST_KEY), ["x"]);
}
