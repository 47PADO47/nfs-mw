//! Shared cosmetic flame shape; colour overrides remain in the car preset.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExhaustStyle {
    pub length: u16,
    pub width: u16,
    pub brightness: u16,
    pub density: u16,
    pub flow: u16,
    pub turbulence: u16,
    pub curl: u16,
    pub response: u16,
}

impl Default for ExhaustStyle {
    fn default() -> Self {
        Self {
            length: 100,
            width: 100,
            brightness: 100,
            density: 100,
            flow: 100,
            turbulence: 100,
            curl: 100,
            response: 100,
        }
    }
}

impl ExhaustStyle {
    pub fn validate(&self) -> anyhow::Result<()> {
        for (name, value, min, max) in [
            ("length", self.length, 0, 200),
            ("width", self.width, 0, 200),
            ("brightness", self.brightness, 0, 200),
            ("density", self.density, 0, 200),
            ("flow", self.flow, 0, 200),
            ("turbulence", self.turbulence, 0, 200),
            ("curl", self.curl, 0, 200),
            ("motion response", self.response, 0, 200),
        ] {
            anyhow::ensure!((min..=max).contains(&value), "Exhaust {name} is outside {min}..={max}%");
        }
        Ok(())
    }
}

pub(super) fn parse(value: &toml::Value) -> Option<ExhaustStyle> {
    let result = value.clone().try_into::<ExhaustStyle>().map_err(anyhow::Error::from).and_then(|style| {
        style.validate()?;
        Ok(style)
    });
    match result {
        Ok(style) => Some(style),
        Err(error) => {
            log::warn!("ignoring exhaust_style: {error}");
            None
        }
    }
}
