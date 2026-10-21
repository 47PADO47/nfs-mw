//! Cached driving assemblies. The canonical model remains the physics/attachment source.
use super::{CarTables, Corner, Placement, Slots, WheelSetup};
use blackbox_solid::Solid;
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct DrivingLods {
    pub traffic: bool,
    pub levels: Vec<LodAssembly>,
    pub damage: super::damage_parts::DamageParts,
    pub damage_fx: super::damage_effects::DamageFx,
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
    police: bool,
) -> Vec<LodAssembly> {
    (1..5)
        .filter_map(|level| {
            let (placements, _): (_, Option<[Corner; 4]>) =
                super::assemble::assemble(tables, slots, solids, setup, level, true);
            // Never replace a car with only wheels or driver geometry from an incomplete LOD.
            let body = placements.iter().any(|p| body_slot(p.slot, police));
            body.then_some(LodAssembly { level, placements })
        })
        .collect()
}

fn body_slot(slot: usize, police: bool) -> bool {
    matches!(slot, super::slot::BASE | super::slot::BODY) || (police && slot == super::slot::DAMAGE_BODY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_police_body_slot_does_not_loosen_the_ordinary_body_guard() {
        assert!(body_slot(super::super::slot::DAMAGE_BODY, true));
        assert!(!body_slot(super::super::slot::DAMAGE_BODY, false));
        assert!(!body_slot(super::super::slot::DRIVER, true));
        assert!(body_slot(super::super::slot::BODY, false));
    }
}
