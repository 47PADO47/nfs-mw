//! A hash of the world state that later ticks depend on (`docs/specs/multiplayer/determinism.md`, step 5): the players'
//! cars, the traffic and its random streams, the police and the pursuit, and the race. Two runs from the same start with
//! the same inputs must give the same hash after every tick; a server and a client can compare it to find a desync.
//!
//! The hash is FNV-1a over 64 bits (offset basis `0xcbf2_9ce4_8422_2325`, prime `0x0000_0100_0000_01b3`), fed little-endian
//! integers and the bit patterns of floats, so equal hashes mean bit-identical state. Everything is fed in a stable order:
//! the players in id order, the cars in entity-id order. Presentation (render keys, effects, sounds, cameras) is left out.

use glam::{Quat, Vec3};

use super::WorldScene;
use super::drive::CarSim;

/// FNV-1a, 64 bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateHasher(u64);

impl Default for StateHasher {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl StateHasher {
    pub fn bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    pub fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    pub fn bool(&mut self, value: bool) {
        self.bytes(&[u8::from(value)]);
    }

    /// A float by its bit pattern: `-0.0` and `0.0` differ, as they may in a later tick.
    pub fn f32(&mut self, value: f32) {
        self.u32(value.to_bits());
    }

    pub fn vec3(&mut self, value: Vec3) {
        value.to_array().into_iter().for_each(|v| self.f32(v));
    }

    pub fn quat(&mut self, value: Quat) {
        value.to_array().into_iter().for_each(|v| self.f32(v));
    }

    pub fn option_f32(&mut self, value: Option<f32>) {
        self.bool(value.is_some());
        self.f32(value.unwrap_or(0.0));
    }

    /// The rigid body of a car: where it is, how it is turned and how it moves.
    pub fn car(&mut self, sim: &CarSim) {
        let body = sim.body();
        self.vec3(body.position);
        self.quat(body.orientation);
        self.vec3(body.linear_velocity);
        self.vec3(body.angular_velocity);
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

impl WorldScene {
    /// The hash of everything that affects later ticks, in a stable order (see the module header).
    pub fn state_hash(&self) -> u64 {
        let mut hash = StateHasher::default();
        hash.bool(self.view == super::View::Chase);
        for player in self.players.iter() {
            hash.u32(u32::from(player.id.0));
            let Some(sim) = player.drive.sim() else {
                hash.bool(false);
                continue;
            };
            hash.bool(true);
            hash.car(sim);
        }
        hash.bool(self.traffic.is_some());
        if let Some(traffic) = &self.traffic {
            traffic.hash_into(&mut hash);
        }
        self.hash_race(&mut hash);
        hash.finish()
    }

    /// The running race: its stage and every racer's progress.
    fn hash_race(&self, hash: &mut StateHasher) {
        let Some(race) = &self.race else {
            hash.bool(false);
            return;
        };
        hash.bool(true);
        hash.bytes(race.stage.name().as_bytes());
        let Some(engine) = &race.engine else {
            hash.bool(false);
            return;
        };
        hash.bool(true);
        hash.f32(engine.elapsed());
        for racer in engine.racers() {
            hash.bool(racer.human);
            hash.vec3(racer.position);
            hash.f32(racer.percent);
            hash.f32(racer.speed);
            hash.option_f32(racer.finished_at);
            hash.option_f32(racer.out_at);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_is_fnv_1a() {
        let mut hash = StateHasher::default();
        hash.bytes(b"a");
        assert_eq!(hash.finish(), 0xaf63_dc4c_8601_ec8c, "the published FNV-1a 64 value of \"a\"");
        let mut empty = StateHasher::default();
        empty.bytes(b"");
        assert_eq!(empty.finish(), 0xcbf2_9ce4_8422_2325);
    }

    #[test]
    fn a_float_is_hashed_by_its_bits() {
        let (mut a, mut b) = (StateHasher::default(), StateHasher::default());
        a.f32(0.0);
        b.f32(-0.0);
        assert_ne!(a.finish(), b.finish());
    }
}
