use super::*;
use crate::scenes::world::drive::SurfaceDamage;
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

    fn surface_sources(&self) -> impl Iterator<Item = (Rc<CarRig>, SurfaceDamage)> + '_ {
        std::iter::once((self.rig.clone(), SurfaceDamage::new(self.glass_mask(), self.lamp_state().broken)))
            .chain(self.trailer.iter().map(|trailer| trailer.surface_source()))
    }
}

impl super::super::TrafficWorld {
    pub fn update_damaged_windows(&self, renderer: &mut dyn blackbox_gfx::RenderBackend, sun: Vec3) {
        let mut requests = std::collections::BTreeMap::<usize, (Rc<CarRig>, BTreeSet<SurfaceDamage>)>::new();
        // Include loaded empty rigs too, so retired actors release their private variants.
        for model in self.models.iter().chain(self.cop_models.iter().map(|m| &m.model)) {
            for rig in std::iter::once(model.rig.clone()).chain(model.trailer.iter().map(|trailer| trailer.rig.clone()))
            {
                requests.entry(Rc::as_ptr(&rig) as usize).or_insert((rig, BTreeSet::new()));
            }
        }
        for (rig, state) in self.cars.iter().flat_map(|car| car.surface_sources()) {
            requests.entry(Rc::as_ptr(&rig) as usize).or_insert_with(|| (rig, BTreeSet::new())).1.insert(state);
        }
        for (_, (rig, masks)) in requests {
            rig.prepare_surfaces(renderer, &masks, sun);
        }
    }
}
