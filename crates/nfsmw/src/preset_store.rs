//! Named overlay presets beside the user config. Names are keys, never paths.
mod picker;
#[cfg(test)]
mod tests;

use anyhow::{Context, Result, ensure};
pub use picker::{Action, Picker};
use std::path::Path;

fn read(path: &Path) -> Result<toml::Table> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(toml::Table::new()),
        Err(error) => return Err(error).context("Reading presets"),
    };
    let table: toml::Table = text.parse().context("Invalid preset file")?;
    ensure!(table.get("version").and_then(toml::Value::as_integer) == Some(1), "Unsupported preset schema");
    ensure!(table.get("presets").is_none_or(toml::Value::is_table), "Invalid preset list");
    Ok(table)
}

pub fn names(path: &Path) -> Result<Vec<String>> {
    Ok(read(path)?
        .get("presets")
        .and_then(toml::Value::as_table)
        .map(|v| v.keys().cloned().collect())
        .unwrap_or_default())
}

pub fn load(path: &Path, name: &str) -> Result<String> {
    read(path)?
        .get("presets")
        .and_then(|v| v.get(name))
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
        .context("Preset is missing or invalid")
}

pub fn save(path: &Path, name: &str, payload: &str) -> Result<()> {
    let text = updated(path, name, payload)?;
    let parent = path.parent().context("Missing preset folder")?;
    std::fs::create_dir_all(parent).context("Creating preset folder")?;
    // Stage the complete document before replacing the previous version.
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, text).context("Writing preset staging file")?;
    std::fs::rename(&temporary, path).context("Replacing presets")
}

/// Build the replacement without publishing it; paired native saves own their commit.
pub(crate) fn updated(path: &Path, name: &str, payload: &str) -> Result<String> {
    let name = name.trim();
    ensure!(
        !name.is_empty() && name.chars().count() <= 64 && !name.chars().any(char::is_control),
        "Use a preset name of 1 to 64 characters"
    );
    let mut table = read(path)?;
    table.insert("version".into(), 1.into());
    table
        .entry("presets")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .context("Invalid preset list")?
        .insert(name.into(), payload.into());
    toml::to_string(&table).context("Serializing presets")
}

/// Rename refuses to overwrite another named setup.
pub fn rename(path: &Path, old: &str, new: &str) -> Result<()> {
    let new = new.trim();
    let payload = load(path, old)?;
    ensure!(old == new || !names(path)?.iter().any(|n| n == new), "A preset with that name already exists");
    let mut table: toml::Table = updated(path, new, &payload)?.parse()?;
    if old != new {
        table.get_mut("presets").and_then(toml::Value::as_table_mut).context("No presets")?.remove(old);
    }
    replace(path, &table)
}
pub fn delete(path: &Path, name: &str) -> Result<()> {
    let mut table = read(path)?;
    ensure!(
        table.get_mut("presets").and_then(toml::Value::as_table_mut).and_then(|v| v.remove(name)).is_some(),
        "Preset is missing"
    );
    replace(path, &table)
}
fn replace(path: &Path, table: &toml::Table) -> Result<()> {
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, toml::to_string(table)?)?;
    std::fs::rename(temporary, path).context("Replacing presets")
}
