//! Per-user configuration access and layered loading.
use super::*;

impl Settings {
    /// The per-user config file (it may not exist yet).
    pub fn config_path() -> Option<std::path::PathBuf> {
        game_install::config_file_path(&nfsmw_data::game::SPEC)
    }

    /// Writes the settings a layer sets to the config file, keeping its other keys.
    pub fn save(changes: &Partial) -> anyhow::Result<std::path::PathBuf> {
        let path = Self::config_path().ok_or_else(|| anyhow::anyhow!("there is no per-user config folder"))?;
        write_file(&path, changes)?;
        Ok(path)
    }

    /// Resolve the settings: `cli` over the process environment over the config file over the defaults.
    pub fn load(cli: Partial) -> Self {
        let env = env::read(|name| std::env::var(name).ok());
        let config = game_install::config_file_path(&nfsmw_data::game::SPEC);
        let file = config.as_deref().map(file::read).unwrap_or_default();
        let extra = match config.as_deref().and_then(std::path::Path::parent) {
            Some(dir) => extra_options::read(&dir.join(extra_options::FILE_NAME)),
            None => Partial::default(),
        };
        let speedbreaker = match config.as_deref().and_then(std::path::Path::parent) {
            Some(dir) => speedbreaker::read(&dir.join(speedbreaker::FILE_NAME)),
            None => SpeedbreakerTuning::default(),
        };
        let mut settings: Self = cli.or(env).or(file).or(extra).into();
        settings.speedbreaker = speedbreaker;
        settings
    }
}
