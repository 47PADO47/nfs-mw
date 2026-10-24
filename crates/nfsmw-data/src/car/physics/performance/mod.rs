//! Installed performance package selection, in the original game's category order.

mod catalog;
pub(crate) mod fields;
mod frontend;
mod resolve;
pub use catalog::{CategoryOptions, PerformanceCatalog};
pub use frontend::{PackageDetails, PerformanceDetails};
pub use resolve::apply;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PerformanceCategory {
    Tires,
    Brakes,
    Chassis,
    Transmission,
    Engine,
    Induction,
    Nitrous,
}

impl PerformanceCategory {
    pub const ALL: [Self; 7] =
        [Self::Tires, Self::Brakes, Self::Chassis, Self::Transmission, Self::Engine, Self::Induction, Self::Nitrous];

    pub const fn field(self) -> &'static str {
        match self {
            Self::Tires => "tires",
            Self::Brakes => "brakes",
            Self::Chassis => "chassis",
            Self::Transmission => "transmission",
            Self::Engine => "engine",
            Self::Induction => "induction",
            Self::Nitrous => "nos",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Tires => "Tires",
            Self::Brakes => "Brakes",
            Self::Chassis => "Suspension",
            Self::Transmission => "Transmission",
            Self::Engine => "Engine",
            Self::Induction => "Turbo / supercharger",
            Self::Nitrous => "Nitrous",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PerformancePackage {
    pub levels: [u8; 7],
    pub junkman: u8,
}

impl PerformancePackage {
    pub fn get(self, category: PerformanceCategory) -> u8 {
        self.levels[category as usize]
    }
    pub fn has_junkman(self, category: PerformanceCategory) -> bool {
        self.junkman & (1 << category as u8) != 0
    }
}

#[cfg(test)]
mod tests;
