//! Validated response settings. Defaults preserve the existing action layer's response.

use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DeadzoneMode {
    #[default]
    Rescaled,
    Cutoff,
}
impl FromStr for DeadzoneMode {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, String> {
        match text.trim().to_ascii_lowercase().as_str() {
            "rescaled" => Ok(Self::Rescaled),
            "cutoff" => Ok(Self::Cutoff),
            _ => Err("deadzone response is rescaled or cutoff".into()),
        }
    }
}
impl fmt::Display for DeadzoneMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Rescaled => "rescaled",
            Self::Cutoff => "cutoff",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deadzone(u8);

impl Deadzone {
    pub fn new(percent: u8) -> Result<Self, String> {
        if percent > 95 {
            return Err("a deadzone is 0 to 95 percent".into());
        }
        Ok(Self(percent))
    }

    pub fn percent(self) -> u8 {
        self.0
    }

    pub fn apply(self, input: f32) -> f32 {
        self.apply_mode(input, DeadzoneMode::Rescaled)
    }

    pub fn apply_mode(self, input: f32, mode: DeadzoneMode) -> f32 {
        if !input.is_finite() {
            return 0.0;
        }
        let input = input.clamp(-1.0, 1.0);
        let threshold = f32::from(self.0) / 100.0;
        if input.abs() < threshold {
            return 0.0;
        }
        if mode == DeadzoneMode::Cutoff {
            return input;
        }
        input.signum() * (input.abs() - threshold) / (1.0 - threshold)
    }

    pub fn step(self, forward: bool) -> Self {
        let percent = match forward {
            true => self.0.saturating_add(5).min(95),
            false => self.0.saturating_sub(5),
        };
        Self(percent)
    }
}

impl Default for Deadzone {
    fn default() -> Self {
        Self(15)
    }
}
impl FromStr for Deadzone {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, String> {
        let percent =
            text.trim().trim_end_matches('%').parse().map_err(|_| "a deadzone is 0 to 95 percent".to_owned())?;
        Self::new(percent)
    }
}
impl fmt::Display for Deadzone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sensitivity(u16);

impl Sensitivity {
    pub fn new(percent: u16) -> Result<Self, String> {
        if !(1..=400).contains(&percent) {
            return Err("sensitivity is 1 to 400 percent".into());
        }
        Ok(Self(percent))
    }
    pub fn percent(self) -> u16 {
        self.0
    }
    pub fn factor(self) -> f32 {
        f32::from(self.0) / 100.0
    }
    pub fn step(self, forward: bool) -> Self {
        Self(match forward {
            true => self.0.saturating_add(25).min(400),
            false => self.0.saturating_sub(25).max(1),
        })
    }
}
impl Default for Sensitivity {
    fn default() -> Self {
        Self(100)
    }
}
impl FromStr for Sensitivity {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, String> {
        let percent =
            text.trim().trim_end_matches('%').parse().map_err(|_| "sensitivity is 1 to 400 percent".to_owned())?;
        Self::new(percent)
    }
}
impl fmt::Display for Sensitivity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Controls {
    pub deadzone_mode: DeadzoneMode,
    pub steering_deadzone: Deadzone,
    pub camera_deadzone: Deadzone,
    pub trigger_deadzone: Deadzone,
    pub steering_sensitivity: Sensitivity,
    pub camera_sensitivity: Sensitivity,
    pub mouse_sensitivity: Sensitivity,
    pub invert_camera_y: bool,
}
impl Default for Controls {
    fn default() -> Self {
        Self {
            deadzone_mode: DeadzoneMode::default(),
            steering_deadzone: Deadzone::default(),
            camera_deadzone: Deadzone::default(),
            trigger_deadzone: Deadzone(0),
            steering_sensitivity: Sensitivity::default(),
            camera_sensitivity: Sensitivity::default(),
            mouse_sensitivity: Sensitivity::default(),
            invert_camera_y: false,
        }
    }
}
