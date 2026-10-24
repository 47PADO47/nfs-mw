//! What of the traffic world the world's state hash takes (`scenes/world/state_hash.rs`): the random streams, the clocks
//! that time spawns and sight tests, the police and the pursuit, and every car in entity-id order.

use super::TrafficWorld;
use crate::scenes::world::StateHasher;

impl TrafficWorld {
    pub(in crate::scenes::world) fn hash_into(&self, hash: &mut StateHasher) {
        let rng = &self.rng;
        for stream in [&rng.nav, &rng.population, &rng.police, &rng.roadblock, &rng.air] {
            hash.u64(stream.0);
        }
        hash.u32(self.next_entity);
        hash.f32(self.signal_time);
        hash.f32(self.spawn_timer);
        hash.f32(self.sight_clock);
        hash.bool(self.world_moment);
        hash.f32(self.hit_timer);
        hash.bool(self.seen_by_cops);
        hash.bool(self.pursuit.is_some());
        self.target_road.hash_into(hash);
        if let Some(pursuit) = &self.pursuit {
            pursuit.hash_spawn_into(hash);
        }
        hash.f32(self.heat_value());
        self.formation_membership.borrow().hash_into(hash);
        self.police.hash_formations(hash);
        hash.f32(self.busted_meter());
        hash.bool(self.is_busted());
        self.police.hash_observation(hash);
        self.police.hash_costs(hash);
        hash.bytes(format!("{:?}", self.cooldown_status()).as_bytes());
        for site in self.roadblock_sites() {
            hash.f32(site[0]);
            hash.f32(site[1]);
        }
        hash.bool(self.helicopter_motion().is_some());
        if let Some((at, velocity, _)) = self.helicopter_motion() {
            hash.vec3(at);
            hash.vec3(velocity);
        }
        hash.u64(self.cars.len() as u64);
        for car in &self.cars {
            car.hash_into(hash);
        }
    }

    /// The parts of [`Self::hash_into`] by name, so a test that finds two runs apart can say where they part.
    #[cfg(test)]
    pub(in crate::scenes::world) fn hash_parts(&self) -> Vec<(String, u64)> {
        let one = |feed: &dyn Fn(&mut StateHasher)| {
            let mut hash = StateHasher::default();
            feed(&mut hash);
            hash.finish()
        };
        let mut parts = vec![
            (
                "streams".to_owned(),
                one(&|h| [&self.rng.nav, &self.rng.population, &self.rng.police].iter().for_each(|s| h.u64(s.0))),
            ),
            (
                "clocks".to_owned(),
                one(&|h| [self.signal_time, self.spawn_timer, self.sight_clock].iter().for_each(|&v| h.f32(v))),
            ),
            ("ids".to_owned(), u64::from(self.next_entity)),
            ("target road".to_owned(), one(&|h| self.target_road.hash_into(h))),
        ];
        for car in &self.cars {
            parts.push((format!("car {} {}", car.key(), car.name), one(&|h| car.hash_into(h))));
        }
        parts
    }
}
