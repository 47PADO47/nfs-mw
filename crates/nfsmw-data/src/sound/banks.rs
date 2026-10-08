//! The game-wide sound bank lists: the `mostwanted` collection of `audiosystem`.

use blackbox_attrib::Database;

use super::fields::Fields;
use super::{NOS_DIR, SKIDS_DIR};

/// Banks every car shares, by family.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GlobalBanks {
    /// `SOUND/SKIDS`: big tires, small tires, drift big, drift medium.
    pub skids: Vec<String>,
    /// `SOUND/NOS`: the nitrous banks by upgrade level.
    pub nitrous: Vec<String>,
}

impl GlobalBanks {
    /// Reads `audiosystem/mostwanted`; empty lists if the collection is missing.
    pub fn read(db: &Database) -> Self {
        let Some(system) = db.collection("audiosystem", "mostwanted") else { return Self::default() };
        let system = Fields(system);
        Self { skids: system.strings("AEMS_SkidBanks"), nitrous: system.strings("AEMS_NOSBanks") }
    }

    /// The nitrous bank for upgrade level 0, the one every car gets in the original (the upgrade level of the
    /// nitrous sound is never raised there).
    pub fn nitrous_bank(&self) -> Option<&str> {
        self.nitrous.first().map(String::as_str)
    }

    /// The skid bank for tire level 0 (`SKID_BIG_MB.abk`).
    pub fn skid_bank(&self) -> Option<&str> {
        self.skids.first().map(String::as_str)
    }

    /// Install-relative paths of the banks a car loads from these lists.
    pub fn files(&self) -> Vec<String> {
        let nitrous = self.nitrous_bank().map(|b| format!("{NOS_DIR}/{b}"));
        let skid = self.skid_bank().map(|b| format!("{SKIDS_DIR}/{b}"));
        nitrous.into_iter().chain(skid).collect()
    }
}
