//! The police on the minimap and the world map's moving cars (`docs/specs/hud-cop-blips.md` section 2): only during a
//! running pursuit (the world drops it on the frame the player gets away or is busted), and only the police cars that
//! came within the engagement radius in it. An engaged car stays on the map after it is wrecked, steady instead of
//! flashing, until it leaves the world; a car sent away leaves the map at once. The helicopter is shown whenever it flies in the pursuit.

use crate::hud::{COP, MapBlip, ROADBLOCK};

impl super::TrafficWorld {
    /// The blips of the police and the standing roadblocks, appended to `out`.
    pub fn map_blips(&self, out: &mut Vec<MapBlip>) {
        let pursuing = self.pursuit.is_some();
        if pursuing {
            self.helicopter_blips(out);
        }
        for at in self.roadblock_sites() {
            out.push(MapBlip { style: ROADBLOCK, position: at, active: false, heading: None });
        }
        if !pursuing {
            return;
        }
        // A cop sent away (`StartFlee`, the cool-down's lost cops) has left the chase: the original's icon pass hides the
        // cars that are not in it (only a roadblock's cars show outside it), so it is not drawn as a grey car.
        for car in self.cars.iter().filter(|c| c.is_police() && c.was_engaged() && !c.is_fleeing()) {
            let at = car.position();
            out.push(MapBlip {
                style: COP,
                position: [at.x, at.y],
                active: car.is_pursuing_on_map(),
                heading: Some(car.heading()),
            });
        }
    }
}
