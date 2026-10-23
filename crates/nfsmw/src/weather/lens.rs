//! Stock camera-water lifetimes; the renderer receives generic distortion regions.
use super::drops::random;
use glam::Vec2;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Droplet {
    pub position: Vec2,
    pub remaining: f32,
    pub lifetime: f32,
    pub size: f32,
    speed: f32,
}
#[derive(Debug, Clone)]
pub struct LensRain {
    droplets: Vec<Droplet>,
    seed: u32,
}
impl Default for LensRain {
    fn default() -> Self {
        let mut seed = 0x174B_A251;
        let droplets = (0..20)
            .map(|_| Droplet {
                position: Vec2::ZERO,
                remaining: 0.0,
                lifetime: 0.3 + 5.0 * random(&mut seed),
                speed: 0.02 + 0.1 * random(&mut seed),
                size: random(&mut seed),
            })
            .collect();
        Self { droplets, seed }
    }
}
impl LensRain {
    pub fn visible(&self, on: bool) -> &[Droplet] {
        if !on {
            return &[];
        }
        &self.droplets
    }
    pub fn update(&mut self, dt: f32, camera_speed: f32, exposed: bool) {
        if !exposed {
            return;
        }
        for drop in &mut self.droplets {
            drop.remaining -= dt;
            if drop.remaining <= 0.0 {
                drop.remaining = drop.lifetime;
                drop.position = Vec2::new(random(&mut self.seed), random(&mut self.seed));
                continue;
            }
            drop.position.y += 0.2 * drop.speed * dt;
            let outward = (drop.position - Vec2::new(0.5, 0.1)).normalize_or_zero();
            drop.position.x += outward.x * camera_speed.min(100.0) * 0.0001 * 60.0 * dt;
            if drop.position.y > 1.0 || !(0.0..=1.0).contains(&drop.position.x) {
                drop.remaining = 0.0;
            }
        }
    }
}
