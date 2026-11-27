//! Optional families of visually related traffic cars provided by a content pack.
use game_install::GameDir;
use serde::Deserialize;

const FILE: &str = "GLOBAL/TRAFFIC_TRIMS.TOML";

#[derive(Default, Deserialize)]
pub(super) struct Families {
    families: Vec<Vec<String>>,
}

impl Families {
    pub(super) fn of(&self, name: &str) -> Option<&[String]> {
        self.families.iter().find(|ids| ids.iter().any(|id| id.eq_ignore_ascii_case(name))).map(Vec::as_slice)
    }
}

impl super::TrafficWorld {
    pub fn load_traffic_trims(&mut self, dir: &GameDir) {
        if !dir.exists(FILE) {
            return;
        }
        let load = || -> anyhow::Result<Families> {
            let bytes = dir.read(FILE)?;
            Ok(toml::from_str(std::str::from_utf8(&bytes)?)?)
        };
        match load() {
            Ok(families) => self.trims = families,
            Err(error) => log::warn!("traffic: could not read {FILE}: {error}"),
        }
    }
}
