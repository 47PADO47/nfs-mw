//! Cached driving assemblies. The canonical model remains the physics/attachment source.
use super::{CarTables, Corner, Placement, Slots, WheelSetup};
use blackbox_solid::Solid;
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct DrivingLods {
    pub traffic: bool,
    pub levels: Vec<LodAssembly>,
}
pub struct LodAssembly {
    pub level: usize,
    pub placements: Vec<Placement>,
}

pub(super) fn assemble(
    tables: &CarTables,
    slots: &Slots,
    solids: &HashMap<u32, Arc<Solid>>,
    setup: Option<&WheelSetup>,
) -> Vec<LodAssembly> {
    (1..5)
        .filter_map(|level| {
            let (placements, _): (_, Option<[Corner; 4]>) =
                super::assemble::assemble(tables, slots, solids, setup, level, true);
            // Never replace a car with only wheels or driver geometry from an incomplete LOD.
            let body = placements.iter().any(|p| p.slot == super::slot::BODY || p.slot == super::slot::BASE);
            body.then_some(LodAssembly { level, placements })
        })
        .collect()
}
