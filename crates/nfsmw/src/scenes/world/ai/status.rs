//! The traffic status command's text output.
use super::{Focus, TrafficWorld};

impl TrafficWorld {
    /// One line per car for the `traffic status` command.
    pub fn status(&self, focus: Focus) -> String {
        let mut lines = vec![self.population_status(focus), self.pursuit_status()];
        lines.push(format!(
            "range {}-{} m; unseen <= {} m / {} s; density {}%; {} occupied vehicle slots",
            self.policy.spawn_min,
            self.policy.spawn_max,
            self.policy.retire_distance,
            self.policy.offscreen_seconds,
            self.policy.density_percent,
            self.population_slots(focus)
        ));
        for car in self.cars.iter().take(12) {
            let p = car.position();
            let c = car.controls();
            lines.push(format!(
                "{} at ({:.0}, {:.0}), {:.0} m away, {:.1} m/s, gas {:.0} brake {:.0} steer {:+.2}; {}",
                car.name,
                p.x,
                p.y,
                (p.x - focus.position[0]).hypot(p.y - focus.position[1]),
                car.speed(),
                c.gas,
                c.brake,
                c.steer,
                car.debug()
            ));
        }
        lines.join("\n")
    }
}
