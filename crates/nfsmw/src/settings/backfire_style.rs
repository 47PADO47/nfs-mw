//! Shared, release-relative appearance for transient exhaust bursts.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BackfireStyle {
    pub root: [u8; 3],
    pub tail: [u8; 3],
    pub size: u16,
    pub brightness: u16,
    pub duration: u16,
    pub variation: u16,
}

impl Default for BackfireStyle {
    fn default() -> Self {
        Self { root: [130, 180, 255], tail: [255, 145, 45], size: 100, brightness: 100, duration: 100, variation: 100 }
    }
}

impl BackfireStyle {
    pub fn validate(&self) -> anyhow::Result<()> {
        for (name, value) in [
            ("size", self.size),
            ("brightness", self.brightness),
            ("duration", self.duration),
            ("variation", self.variation),
        ] {
            anyhow::ensure!(value <= 200, "Backfire {name} is outside 0..=200%");
        }
        Ok(())
    }
}

pub(super) fn parse(value: &toml::Value) -> Option<BackfireStyle> {
    let result = value.clone().try_into::<BackfireStyle>().map_err(anyhow::Error::from).and_then(|style| {
        style.validate()?;
        Ok(style)
    });
    match result {
        Ok(style) => Some(style),
        Err(error) => {
            log::warn!("ignoring backfire_style: {error}");
            None
        }
    }
}
