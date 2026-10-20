use super::*;
use std::collections::BTreeSet;

impl AiCar {
    pub(in crate::scenes::world::ai) fn glass_mask(&self) -> u16 {
        let Some(events) = &self.sim.events else { return 0 };
        if self.rig.model().driving_lods.damage.enabled {
            return nfsmw_data::car::damage_parts::glass_mask(events.source_levels());
        }
        (0..events.damage.zone_count())
            .filter(|&zone| events.damage.zone(zone).glass > 0)
            .fold(0, |mask, zone| mask | (1 << zone))
    }

    pub(in crate::scenes::world::ai) fn glass_rig(&self) -> Rc<CarRig> {
        self.rig.clone()
    }
}

impl super::super::TrafficWorld {
    pub fn update_damaged_windows(&self, renderer: &mut dyn blackbox_gfx::RenderBackend, sun: Vec3) {
        let mut requests = std::collections::BTreeMap::<usize, (Rc<CarRig>, BTreeSet<u16>)>::new();
        // Include loaded empty rigs too, so retired actors release their private variants.
        for rig in self.models.iter().map(|m| m.rig.clone()).chain(self.cop_models.iter().map(|m| m.model.rig.clone()))
        {
            requests.entry(Rc::as_ptr(&rig) as usize).or_insert((rig, BTreeSet::new()));
        }
        for car in &self.cars {
            let rig = car.glass_rig();
            requests
                .entry(Rc::as_ptr(&rig) as usize)
                .or_insert_with(|| (rig, BTreeSet::new()))
                .1
                .insert(car.glass_mask());
        }
        for (_, (rig, masks)) in requests {
            rig.prepare_glass(renderer, &masks, sun);
        }
    }
}
