use super::TrafficWorld;
use crate::scenes::world::damage_effects::Source;

impl TrafficWorld {
    pub(in crate::scenes::world) fn damage_effect_sources(&self) -> impl Iterator<Item = Source<'_>> {
        self.cars.iter().filter_map(|car| car.damage_effect_source())
    }
}
