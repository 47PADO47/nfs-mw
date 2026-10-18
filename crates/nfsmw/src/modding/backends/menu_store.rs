//! The saved values of the `bb.menus` rows, kept per mod in the config file under `[mod_values.<id>]`.
//!
//! The table cannot be `[mods.<id>]`: the config's `mods` key is the mod mode (`off`, `allowlist`, `all`), and TOML does
//! not allow a key and a table of the same name. The table is written with the rest of the file, after its plain keys, so
//! the settings writer and the alias payloads keep their order.

use std::io::ErrorKind;
use std::path::Path;

use toml::{Table, Value};

use crate::settings::Settings;

/// The config table that holds every mod's values.
const TABLE: &str = "mod_values";

/// The value of row `name` of mod `owner` in a parsed config, if it is there.
fn row_value<'a>(table: &'a Table, owner: &str, name: &str) -> Option<&'a Value> {
    table.get(TABLE)?.as_table()?.get(owner)?.as_table()?.get(name)
}

/// The saved value of row `name` of mod `owner` in the config text, as a number. A boolean reads as 0 or 1.
pub fn saved_in(text: &str, owner: &str, name: &str) -> Option<f32> {
    let table: Table = text.parse().ok()?;
    match row_value(&table, owner, name)? {
        Value::Boolean(on) => Some(f32::from(u8::from(*on))),
        Value::Integer(number) => Some(*number as f32),
        Value::Float(number) => Some(*number as f32),
        _ => None,
    }
}

/// The saved text of row `name` of mod `owner` in the config text. Only a string reads back as text.
pub fn saved_text_in(text: &str, owner: &str, name: &str) -> Option<String> {
    let table: Table = text.parse().ok()?;
    match row_value(&table, owner, name)? {
        Value::String(text) => Some(text.clone()),
        _ => None,
    }
}

/// The config text with row `name` of mod `owner` set to `value`. Every other key and table stays as it was.
pub fn with_saved(text: &str, owner: &str, name: &str, value: Value) -> Result<String, String> {
    let mut table: Table = text.parse().map_err(|e| format!("the config file is not valid TOML: {e}"))?;
    let Value::Table(values) = table.entry(TABLE).or_insert_with(|| Value::Table(Table::new())) else {
        return Err(format!("{TABLE} in the config file is not a table"));
    };
    let Value::Table(mod_values) = values.entry(owner).or_insert_with(|| Value::Table(Table::new())) else {
        return Err(format!("{TABLE}.{owner} in the config file is not a table"));
    };
    mod_values.insert(name.to_owned(), value);
    toml::to_string(&table).map_err(|e| e.to_string())
}

/// Where the rows' values are kept: the config file in a game run, a memory store in the tests. The game's own text
/// values (the mod theme choice, `ui/theme`) are kept here too, under the owner `nfsmw`.
pub trait ValueStore {
    /// The saved value of row `name` of mod `owner` as a number, if any.
    fn saved(&self, owner: &str, name: &str) -> Option<f32>;
    /// The saved value of row `name` of mod `owner` as text, if it is text.
    fn saved_text(&self, owner: &str, name: &str) -> Option<String>;
    /// Save row `name` of mod `owner`. An error says why it could not be.
    fn save(&self, owner: &str, name: &str, value: Value) -> Result<(), String>;
}

/// The config file (`[mod_values.<id>]`), read and written as the other settings are.
pub struct ConfigFile;

impl ValueStore for ConfigFile {
    fn saved(&self, owner: &str, name: &str) -> Option<f32> {
        saved(owner, name)
    }

    fn saved_text(&self, owner: &str, name: &str) -> Option<String> {
        let path = Settings::config_path()?;
        let text = std::fs::read_to_string(path).ok()?;
        saved_text_in(&text, owner, name)
    }

    fn save(&self, owner: &str, name: &str, value: Value) -> Result<(), String> {
        save(owner, name, value)
    }
}

/// The saved value of row `name` of mod `owner`, from the config file. No file, or no value, is `None`.
fn saved(owner: &str, name: &str) -> Option<f32> {
    let path = Settings::config_path()?;
    let text = std::fs::read_to_string(path).ok()?;
    saved_in(&text, owner, name)
}

/// Save row `name` of mod `owner` in the config file, creating the file and its folder if needed.
fn save(owner: &str, name: &str, value: Value) -> Result<(), String> {
    let Some(path) = Settings::config_path() else {
        return Err("there is no config folder, so the value is kept for this run only".into());
    };
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("reading {}: {e}", path.display())),
    };
    let text = with_saved(&existing, owner, name, value)?;
    write_file(&path, &text)
}

fn write_file(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    }
    std::fs::write(path, text).map_err(|e| format!("writing {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_value_reads_back_as_a_number() {
        let text = with_saved("", "example.hello", "fast", Value::Boolean(true)).unwrap();
        assert_eq!(saved_in(&text, "example.hello", "fast"), Some(1.0));
        let text = with_saved(&text, "example.hello", "speed", Value::Float(0.25)).unwrap();
        assert_eq!(saved_in(&text, "example.hello", "speed"), Some(0.25));
        let text = with_saved(&text, "example.hello", "pick", Value::Integer(2)).unwrap();
        assert_eq!(saved_in(&text, "example.hello", "pick"), Some(2.0));
        assert_eq!(saved_in(&text, "other.mod", "speed"), None, "each mod has its own table");
        assert_eq!(saved_in(&text, "example.hello", "missing"), None);
    }

    #[test]
    fn a_saved_text_reads_back_as_text_and_not_as_a_number() {
        let text = with_saved("", "nfsmw", "ui_style_mod", Value::String("example.a/night".into())).unwrap();
        assert_eq!(saved_text_in(&text, "nfsmw", "ui_style_mod").as_deref(), Some("example.a/night"));
        assert_eq!(saved_in(&text, "nfsmw", "ui_style_mod"), None, "a string is not a number");
        let text = with_saved(&text, "nfsmw", "ui_style_mod", Value::String(String::new())).unwrap();
        assert_eq!(saved_text_in(&text, "nfsmw", "ui_style_mod").as_deref(), Some(""), "the built-in choice is empty");
        assert_eq!(saved_text_in(&text, "nfsmw", "missing"), None);
    }

    #[test]
    fn the_mode_key_and_other_tables_survive_a_save() {
        let before = "mods = \"allowlist\"\nmaster_volume = 70\n\n[hd_content]\nenabled = true\n";
        let after = with_saved(before, "example.hello", "fast", Value::Boolean(false)).unwrap();
        let table: Table = after.parse().unwrap();
        assert_eq!(table.get("mods").and_then(Value::as_str), Some("allowlist"));
        assert_eq!(table.get("master_volume").and_then(Value::as_integer), Some(70));
        assert!(table.get("hd_content").is_some(), "another table is kept");
        assert!(after.find("[mod_values").unwrap() > after.find("master_volume").unwrap(), "plain keys come first");
    }

    #[test]
    fn a_config_that_is_not_toml_is_refused_and_left_alone() {
        assert!(with_saved("not = [toml", "example.hello", "fast", Value::Boolean(true)).is_err());
    }

    #[test]
    fn a_scalar_where_the_table_goes_is_refused() {
        assert!(with_saved("mod_values = 3\n", "example.hello", "fast", Value::Boolean(true)).is_err());
    }
}
