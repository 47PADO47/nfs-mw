//! Merge preferences into the shared config without replacing unrelated settings.
use super::{Bookmark, CameraPreferences, Location, State};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Stored {
    camera: CameraPreferences,
    locations: Vec<Location>,
    bookmarks: Vec<Bookmark>,
}
impl Stored {
    fn validate(&self) -> Result<()> {
        self.camera.validate()?;
        ensure!(self.locations.len() <= 128 && self.bookmarks.len() <= 128, "Too many saved locations or views");
        for v in &self.locations {
            ensure!(v.position.iter().all(|n| n.is_finite()) && v.heading.is_finite(), "Invalid location");
        }
        for v in &self.bookmarks {
            ensure!(
                v.eye.iter().chain(&v.forward).chain(&v.up).all(|n| n.is_finite())
                    && v.fov.is_finite()
                    && (1.0..=170.0).contains(&v.fov),
                "Invalid camera bookmark"
            );
            let f = glam::Vec3::from_array(v.forward);
            let up = glam::Vec3::from_array(v.up);
            ensure!(
                f.length_squared() > 0.5 && up.length_squared() > 0.5 && f.cross(up).length_squared() > 0.01,
                "Invalid bookmark orientation"
            );
        }
        Ok(())
    }
}
fn path() -> Result<std::path::PathBuf> {
    crate::settings::Settings::config_path().context("No per-user config folder")
}
fn read(path: &std::path::Path) -> Result<toml::Table> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    text.parse().context("Invalid config TOML; file was left unchanged")
}
pub fn load() -> Result<State> {
    let table = read(&path()?)?;
    let stored: Stored = table.get("extra_options").cloned().map(|v| v.try_into()).transpose()?.unwrap_or_default();
    stored.validate()?;
    Ok(State { camera: stored.camera, locations: stored.locations, bookmarks: stored.bookmarks, ..Default::default() })
}
pub fn save(state: &State) -> Result<()> {
    save_at(&path()?, state)
}
fn save_at(path: &std::path::Path, state: &State) -> Result<()> {
    ensure!(
        state.persistence_error.is_none(),
        "Config could not be loaded; fix it and reload before saving preferences"
    );
    let stored =
        Stored { camera: state.camera.clone(), locations: state.locations.clone(), bookmarks: state.bookmarks.clone() };
    stored.validate()?;
    let mut table = read(path)?;

    let replacement = toml::Value::try_from(stored)?;
    let extra = table
        .entry("extra_options")
        .or_insert_with(|| toml::Value::Table(Default::default()))
        .as_table_mut()
        .context("Invalid extra options table")?;
    for (key, value) in replacement.as_table().context("Invalid serialized preferences")? {
        if key == "camera"
            && let Some(camera) = extra.get_mut(key).and_then(toml::Value::as_table_mut)
        {
            for (name, value) in value.as_table().context("Invalid camera preferences")? {
                camera.insert(name.clone(), value.clone());
            }
            continue;
        }
        extra.insert(key.clone(), value.clone());
    }
    std::fs::create_dir_all(path.parent().context("Missing config parent")?)?;
    let temp = path.with_extension("extra.tmp");
    std::fs::write(&temp, toml::to_string(&table)?)?;
    std::fs::rename(temp, path).context("Saving extra options")
}
pub fn reload(state: &mut State) -> Result<()> {
    let loaded = match load() {
        Ok(loaded) => loaded,
        Err(error) => {
            state.persistence_error = Some(format!("{error:#}"));
            return Err(error);
        }
    };
    state.persistence_error = None;
    state.camera = loaded.camera;
    state.locations = loaded.locations;
    state.bookmarks = loaded.bookmarks;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn save_preserves_other_settings_and_never_persists_testing_overrides() {
        let dir = std::env::temp_dir().join(format!("nfsmw-extra-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "game_dir = 'example'\ntraffic = 25\n[extra_options.camera]\nfuture_option = 7\n")
            .unwrap();
        let mut state = State::default();
        state.camera.auto_center = false;
        state.testing.police_off = true;
        save_at(&path, &state).unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        state.persistence_error = Some("Invalid camera value".into());
        assert!(save_at(&path, &state).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            saved,
            "a failed config load must not erase saved locations or views"
        );
        state.persistence_error = None;
        let table = read(&path).unwrap();
        assert_eq!(table["traffic"].as_integer(), Some(25));
        assert_eq!(table["extra_options"]["camera"]["future_option"].as_integer(), Some(7));
        assert_eq!(table["game_dir"].as_str(), Some("example"));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.contains("police_off"));
        let stored: Stored = table["extra_options"].clone().try_into().unwrap();
        assert!(!stored.camera.auto_center);
        std::fs::write(&path, "broken = [").unwrap();
        assert!(save_at(&path, &state).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken = [");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
