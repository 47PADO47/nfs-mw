//! Pursuit action memories affect future controls even when the current pose is identical.

use super::CopState;
use crate::scenes::world::StateHasher;

impl CopState {
    pub(in crate::scenes::world::ai) fn hash_into(&self, hash: &mut StateHasher) {
        self.close.hash_into(hash);
        self.road_nos.hash_into(hash);
        self.governor.visit_state(&mut |word| hash.u32(word));
        self.stuck.visit_state(&mut |word| hash.u32(word));
        hash.f32(self.repath);
        hash.vec3(self.last_find);
        hash.f32(self.last_speed);
        hash.bytes(format!("{:?}", self.path_state).as_bytes());
        hash.bool(self.ram_allowed);
        let environment = self.environment;
        hash.u32(environment.level);
        hash.option_f32(environment.reaction);
        hash.bool(environment.nitrous);
        hash.bool(environment.nitrous_input);
        hash.f32(environment.nitrous_boost);
        hash.bool(environment.jerk);
        hash.bool(environment.racing);
        hash.bool(environment.shared_target_road);
        hash.bool(environment.target_road.is_some());
        if let Some(road) = environment.target_road {
            road.hash_into(hash);
        }
        for value in [
            self.limits.top_speed,
            self.limits.start_grip,
            self.limits.end_grip,
            self.limits.top_speed_multiplier,
            self.limits.acceleration_multiplier,
            self.car.ai.max_speed_kmh,
            self.car.ai.top_speed_multiplier,
            self.car.ai.acceleration_multiplier,
        ] {
            hash.f32(value);
        }
        for performance in [self.car.performance, self.player] {
            for value in [performance.top_speed, performance.start_grip, performance.end_grip]
                .into_iter()
                .chain(performance.acceleration)
            {
                hash.f32(value);
            }
        }
    }
}
