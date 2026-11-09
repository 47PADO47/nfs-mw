//! Which effect variants the mod's settings turn on (`trees`, `skid_marks`, `rain_splash`, `wet_road`).

use crate::config::Config;

/// The settings that say which effect variants the mod offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Features {
    pub trees: bool,
    pub skid_marks: bool,
    pub rain_splash: bool,
    pub wet_road: bool,
}

impl Config {
    /// Which effect variants the settings turn on.
    pub fn features(&self) -> Features {
        Features {
            trees: self.trees,
            skid_marks: self.skid_marks,
            rain_splash: self.rain_splash,
            wet_road: self.wet_road,
        }
    }
}
