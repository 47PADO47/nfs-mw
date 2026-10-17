//! Activation diagnostics: a vehicle ceiling is not an observed population.
use super::{Focus, TrafficWorld};

#[derive(Clone, Copy, Debug)]
pub(super) enum Rejection {
    Lane,
    Budget,
    Collision,
    Projection,
    Spacing,
    Ground,
}

#[derive(Default, Debug)]
pub(super) struct PopulationStats {
    pub attempts: u64,
    pub spawned: u64,
    pub rejected: [u64; 6],
    pub waiting_collision: usize,
}

impl TrafficWorld {
    pub fn population_status(&self, focus: Focus) -> String {
        let near = |radius: f32| {
            self.cars
                .iter()
                .filter(|car| {
                    let p = car.position();
                    (p.x - focus.position[0]).hypot(p.y - focus.position[1]) <= radius
                })
                .count()
        };
        format!(
            "{} actors / {} occupied slots / {} budget; within 150/300/600 m: {}/{}/{}\n{} waiting for collision; {} activations / {} probes\nRejected lane/budget/collision/projection/spacing/ground: {:?}",
            self.cars.len(),
            self.population_slots(focus),
            self.target,
            near(150.0),
            near(300.0),
            near(600.0),
            self.population_stats.waiting_collision,
            self.population_stats.spawned,
            self.population_stats.attempts,
            self.population_stats.rejected
        )
    }
}
