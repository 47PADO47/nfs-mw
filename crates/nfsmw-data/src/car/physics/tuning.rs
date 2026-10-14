//! Original customization's three tuning profiles and eleven-position sliders.
//! Behavioral source: docs/specs/performance-tuning.md.

use anyhow::{Result, ensure};
use blackbox_vehicle::Tunings;

use super::{PerformanceCategory as Hardware, PerformancePackage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum TuningCategory {
    Steering,
    Handling,
    Brakes,
    RideHeight,
    Aerodynamics,
    Nitrous,
    Induction,
}

impl TuningCategory {
    pub const ALL: [Self; 7] = [
        Self::Steering,
        Self::Handling,
        Self::Brakes,
        Self::RideHeight,
        Self::Aerodynamics,
        Self::Nitrous,
        Self::Induction,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Steering => "Steering",
            Self::Handling => "Handling",
            Self::Brakes => "Brakes",
            Self::RideHeight => "Ride height",
            Self::Aerodynamics => "Aerodynamics",
            Self::Nitrous => "Nitrous",
            Self::Induction => "Turbo / supercharger",
        }
    }

    pub fn directions(self) -> (&'static str, &'static str) {
        match self {
            Self::Steering => ("Slower response", "Quicker response"),
            Self::Handling => ("More sliding", "Less sliding"),
            Self::Brakes => ("Rear bias", "Front bias"),
            Self::RideHeight => ("Lower", "Higher"),
            Self::Aerodynamics => ("Less downforce", "More downforce"),
            Self::Nitrous => ("Longer duration", "More power"),
            Self::Induction => ("Lower RPM boost", "Higher RPM boost"),
        }
    }

    /// Original ordinary-package/body-part gate. Stock equipment and Junkman do not unlock tuning.
    pub fn available(self, package: PerformancePackage, aftermarket_aero: bool) -> bool {
        let category = match self {
            Self::Steering | Self::Handling | Self::RideHeight => Hardware::Chassis,
            Self::Brakes => Hardware::Brakes,
            Self::Nitrous => Hardware::Nitrous,
            Self::Induction => Hardware::Induction,
            Self::Aerodynamics => return aftermarket_aero,
        };
        package.get(category) > 0
    }

    pub fn requirement(self) -> &'static str {
        match self {
            Self::Steering | Self::Handling | Self::RideHeight => "Requires a suspension upgrade.",
            Self::Brakes => "Requires a brake upgrade.",
            Self::Nitrous => "Requires a nitrous upgrade.",
            Self::Induction => "Requires a turbo or supercharger upgrade.",
            Self::Aerodynamics => "Requires an aftermarket body kit or spoiler.",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TuningSelection {
    /// Signed positions -5..=5, in TuningCategory order. Zero is the original neutral position.
    pub profiles: [[i8; 7]; 3],
    pub active: u8,
}

impl TuningSelection {
    pub fn validate(self) -> Result<()> {
        ensure!(self.active < 3, "unknown tuning profile");
        ensure!(
            self.profiles.iter().flatten().all(|value| (-5..=5).contains(value)),
            "tuning is outside its original range"
        );
        Ok(())
    }

    pub fn normalized(self) -> Result<Tunings> {
        self.validate()?;
        let [steering, handling, brakes, ride_height, aerodynamics, nos, induction] =
            self.profiles[self.active as usize].map(|value| f32::from(value) / 5.0);
        Ok(Tunings { steering, handling, brakes, ride_height, aerodynamics, nos, induction })
    }
}

#[cfg(test)]
mod tests;
