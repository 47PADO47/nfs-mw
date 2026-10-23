//! What of one computer-driven car the world's state hash takes (`scenes/world/state_hash.rs`): its entity id, its body
//! and trailer, its driver's decisions and its road cursor. The render key and the lights are presentation and stay out.

use super::{AiCar, Role};
use crate::scenes::world::StateHasher;

impl AiCar {
    pub(in crate::scenes::world::ai) fn hash_into(&self, hash: &mut StateHasher) {
        hash.u32(self.entity);
        hash.car(&self.sim);
        hash.bool(self.trailer.is_some());
        if let Some(trailer) = &self.trailer {
            hash.bool(trailer.is_hitched());
            hash.vec3(trailer.physics_position());
            hash.vec3(trailer.velocity());
        }
        hash.u32(match self.role {
            Role::Traffic => 0,
            Role::Cop(_) => 1,
            Role::Racer(_) => 2,
        });
        hash.u32(self.steps);
        hash.vec3(self.target);
        hash.f32(self.speed);
        hash.f32(self.controls.gas);
        hash.f32(self.controls.brake);
        hash.f32(self.controls.steer);
        hash.f32(self.controls.handbrake);
        hash.u32(u32::from(self.nav.segment));
        hash.u64(self.nav.lane as u64);
        hash.f32(self.nav.t);
        hash.f32(self.idle);
        hash.f32(self.shock);
        hash.f32(self.offscreen);
        for flag in [
            self.blocked,
            self.waiting,
            self.patrol,
            self.searching,
            self.fleeing,
            self.engaged,
            self.arrested,
            self.sees_player,
            self.breaker_stop,
            self.arrived,
            self.roadblock,
        ] {
            hash.bool(flag);
        }
        hash.u32(self.roadblock_group);
        hash.option_f32(self.damage_health());
    }
}
