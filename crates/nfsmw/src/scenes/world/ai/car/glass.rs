use super::*;
use crate::scenes::world::drive::SurfaceDamage;

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

/// The rigs to prepare this frame, each with the states asked of it. `live` counts the entries in use; the ones past it
/// are spare, so a frame refills the buffers of the last without allocating.
type Requests = Vec<(Rc<CarRig>, Vec<SurfaceDamage>)>;

fn note_rig(requests: &mut Requests, live: &mut usize, rig: &Rc<CarRig>, state: Option<SurfaceDamage>) {
    let at = match requests[..*live].iter().position(|(seen, _)| Rc::ptr_eq(seen, rig)) {
        Some(at) => at,
        None => {
            match requests.get_mut(*live) {
                Some((slot, states)) => {
                    *slot = Rc::clone(rig);
                    states.clear();
                }
                None => requests.push((Rc::clone(rig), Vec::new())),
            }
            *live += 1;
            *live - 1
        }
    };
    if let Some(state) = state {
        requests[at].1.push(state);
    }
}

impl super::super::TrafficWorld {
    pub fn update_damaged_windows(&self, renderer: &mut dyn blackbox_gfx::RenderBackend, sun: Vec3) {
        let mut requests = self.window_requests.take();
        let mut live = 0;
        // Include loaded empty rigs too, so retired actors release their private variants.
        for model in self.models.iter().chain(self.cop_models.iter().map(|m| &m.model)) {
            note_rig(&mut requests, &mut live, &model.rig, None);
            if let Some(trailer) = &model.trailer {
                note_rig(&mut requests, &mut live, &trailer.rig, None);
            }
        }
        for (rig, state) in self.cars.iter().flat_map(|car| car.surface_sources()) {
            note_rig(&mut requests, &mut live, &rig, Some(state));
        }
        for (rig, states) in &mut requests[..live] {
            states.sort_unstable();
            states.dedup();
            rig.prepare_surface_slice(renderer, states, sun);
        }
        // Rigs of retired models are not kept alive past this frame.
        requests.truncate(live);
        *self.window_requests.borrow_mut() = requests;
    }
}
