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

    /// Resolve the settings: `cli` over the process environment over the config file over the defaults. A stock run
    /// (`--stock-rules`, `parity/stock.rs`) reads no mod-style ini file and forces the original's mechanics on the
    /// result; the config file is not written.
    pub fn load(cli: Partial) -> Self {
        use crate::parity::stock;
        let env = env::read(|name| std::env::var(name).ok());
        let config = game_install::config_file_path(&nfsmw_data::game::SPEC);
        let file = config.as_deref().map(file::read).unwrap_or_default();
        let ini = |name: &str| {
            let dir = config.as_deref().and_then(std::path::Path::parent)?;
            (!stock::ini_absent(name)).then(|| dir.join(name))
        };
        let extra = ini(extra_options::FILE_NAME).map(|path| extra_options::read(&path)).unwrap_or_default();
        let speedbreaker = ini(speedbreaker::FILE_NAME).map(|path| speedbreaker::read(&path)).unwrap_or_default();
        let mut settings: Self = cli.or(env).or(file).or(extra).into();
        settings.speedbreaker = speedbreaker;
        stock::effective(&settings)
    }
}
