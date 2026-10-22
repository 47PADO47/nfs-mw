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
        hash.f32(self.hit_timer);
        hash.bool(self.seen_by_cops);
        hash.bool(self.pursuit.is_some());
        hash.f32(self.heat_value());
        hash.f32(self.busted_meter());
        hash.bool(self.is_busted());
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
}
