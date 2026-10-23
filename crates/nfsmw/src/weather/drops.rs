//! Camera-relative primary PC rain points, separate from sheltered rain curtains.
use glam::Vec3;
pub const POOL_SIZE: usize = 350;
const TICK: f32 = 1.0 / 60.0;
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Drop {
    local: Vec3,
    motion: Vec3,
}
#[derive(Debug, Clone, PartialEq)]
pub struct DropPool {
    drops: Vec<Drop>,
    velocities: [Vec3; 10],
    seed: u32,
    credit: f32,
    camera: Option<Vec3>,
    forward: Vec3,
    count: usize,
    ready: bool,
    length: f32,
}
impl Default for DropPool {
    fn default() -> Self {
        Self {
            drops: vec![Drop::default(); POOL_SIZE],
            velocities: [Vec3::ZERO; 10],
            seed: 0x2545_F491,
            credit: 0.0,
            camera: None,
            forward: Vec3::X,
            count: 0,
            ready: false,
            length: 0.45,
        }
    }
}
pub(super) fn random(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*seed >> 8) as f32 / 16_777_216.0
}
fn spawn(seed: &mut u32) -> Vec3 {
    Vec3::new(8.0 + 20.0 * (random(seed) - 0.5), 7.0 * (random(seed) - 0.5), 6.0 * random(seed))
}
impl DropPool {
    pub fn falling(&self) -> usize {
        self.count
    }
    fn initialize(&mut self) {
        for velocity in &mut self.velocities {
            *velocity = Vec3::new(
                0.02 * random(&mut self.seed) - 0.01,
                0.02 * random(&mut self.seed) - 0.01,
                -0.35 - 0.03 * random(&mut self.seed),
            );
        }
        for drop in &mut self.drops {
            drop.local = spawn(&mut self.seed);
        }
        self.ready = true;
    }
    pub fn update(&mut self, dt: f32, camera: Vec3, forward: Vec3, intensity: f32, exposed: bool) {
        if !self.ready {
            self.initialize();
        }
        let heading = Vec3::new(forward.x, forward.y, 0.0).normalize_or(Vec3::X);
        let side = Vec3::Z.cross(heading);
        let delta = self.camera.map_or(Vec3::ZERO, |p| camera - p);
        let previous_side = Vec3::Z.cross(self.forward);
        let cut = delta.length_squared() > 400.0;
        let camera_tick = match cut {
            true => Vec3::ZERO,
            false => delta / (dt * 60.0),
        };
        let camera_tick = Vec3::new(camera_tick.dot(heading), camera_tick.dot(side), camera_tick.z);
        self.length = 0.45 + (camera_tick.length() * 16.0).clamp(0.0, 1.0);
        for drop in &mut self.drops {
            if cut {
                drop.local = spawn(&mut self.seed);
                continue;
            }
            let world = self.forward * drop.local.x + previous_side * drop.local.y + Vec3::Z * drop.local.z - delta;
            drop.local = Vec3::new(world.dot(heading), world.dot(side), world.z);
        }
        self.forward = heading;
        self.camera = Some(camera);
        for (i, drop) in self.drops.iter_mut().enumerate() {
            drop.motion = self.velocities[i % 10] - camera_tick;
        }
        self.count = (intensity.clamp(0.0, 1.0) * POOL_SIZE as f32) as usize;
        if !exposed {
            self.count = 0;
        }
        self.credit += dt.min(1.0);
        while self.credit >= TICK {
            self.credit -= TICK;
            for (i, drop) in self.drops.iter_mut().enumerate() {
                drop.local += self.velocities[i % 10];
                if drop.local.z < 0.0
                    || drop.local.z > 6.0
                    || !(-2.0..=18.0).contains(&drop.local.x)
                    || drop.local.y.abs() > 3.5
                {
                    drop.local = spawn(&mut self.seed);
                }
            }
        }
    }
    pub fn quads(&self, camera: Vec3, out: &mut Vec<([Vec3; 4], usize)>) {
        let side = Vec3::Z.cross(self.forward);
        for (i, drop) in self.drops.iter().take(self.count).enumerate() {
            let at = camera + self.forward * drop.local.x + side * drop.local.y + Vec3::Z * drop.local.z;
            let motion = self.forward * drop.motion.x + side * drop.motion.y + Vec3::Z * drop.motion.z;
            let axis = motion.normalize_or(-Vec3::Z) * (self.length * 0.5);
            let width = (camera - at).cross(axis).normalize_or(side) * 0.01;
            out.push(([at - axis - width, at - axis + width, at + axis + width, at + axis - width], i % 10));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rain_is_z_up_and_teleports_follow_the_camera() {
        let mut pool = DropPool::default();
        pool.update(TICK, Vec3::ZERO, Vec3::X, 1.0, true);
        assert_eq!(pool.falling(), 350);
        let mut vertices = Vec::new();
        pool.quads(Vec3::ZERO, &mut vertices);
        assert!(vertices.iter().all(|(q, _)| (q[0].z - q[3].z).abs() > 0.4));
        let camera = Vec3::splat(10000.0);
        pool.update(TICK, camera, Vec3::Y, 1.0, true);
        vertices.clear();
        pool.quads(camera, &mut vertices);
        assert!(vertices.iter().all(|(q, _)| q.iter().all(|p| p.distance(camera) < 22.0)));
    }
    #[test]
    fn population_and_motion_do_not_depend_on_refresh_rate() {
        let run = |hz| {
            let mut pool = DropPool::default();
            for _ in 0..hz * 3 {
                pool.update(1.0 / hz as f32, Vec3::ZERO, Vec3::X, 0.5, true);
            }
            pool
        };
        let (a, b) = (run(60), run(240));
        assert_eq!(a.falling(), 175);
        assert_eq!(a.drops, b.drops);
    }
}
