//! Independent civilian suppression does not change F8 density or remove police/racers.
use super::{TrafficWorld, car::Role};
impl TrafficWorld {
    pub(in crate::scenes::world) fn set_extra_heat_once(&mut self, level: u8) -> bool {
        if !self.is_pursuing() {
            return false;
        }
        self.police.set_forced_pursuit(true);
        self.police.set_test_heat(Some(level));
        self.police.set_test_heat(None);
        true
    }

    pub(in crate::scenes::world) fn set_extra_traffic(&mut self, off: bool) {
        self.extra_traffic_off = off;
        if off {
            self.cars.retain(|car| !matches!(car.role, Role::Traffic) || car.is_police());
        }
    }
    pub(in crate::scenes::world) fn set_extra_heat(&mut self, level: Option<u8>) {
        self.police.set_test_heat(level);
    }
}
