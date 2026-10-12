//! Rain as the world draws it: the weather model's rain eased toward a target, and a pool of falling
//! drops around the camera, each drawn as a short streak. Pure: no renderer, the caller turns the
//! quads into vertices. The rain target comes from the settings (`always_rain`); the zone rain map
//! that the original rolls is not wired yet.

use glam::Vec3;

use super::Weather;
use super::drops::{DropPool, Tuning};

/// New drops per second at full rain and the full amount.
const SPAWN_RATE: f32 = 120.0;
/// Fall speed in metres per second.
const FALL_SPEED: f32 = 14.0;
/// Half-size of the spawn box around the camera, metres.
const BOX_HALF: [f32; 3] = [25.0, 6.0, 25.0];
/// Drops are recycled this far below and above the camera's height, metres.
const BELOW: f32 = 8.0;
const ABOVE: f32 = 6.0;
/// Streak length and half-width, metres.
const STREAK_LENGTH: f32 = 0.9;
const STREAK_HALF_WIDTH: f32 = 0.015;
/// The random source's starting state (any non-zero value works; fixed for repeatable runs).
const SEED: u32 = 0x2545_F491;

/// The rain state of one scene.
#[derive(Debug, Clone)]
pub struct Rain {
    weather: Weather,
    pool: DropPool,
    always: bool,
    /// The rain amount, 0 to 1 of the original.
    amount: f32,
    seed: u32,
}

impl Default for Rain {
    fn default() -> Self {
        Self::new()
    }
}

impl Rain {
    /// Dry, with no rain forced on and the full amount.
    pub fn new() -> Self {
        Self { weather: Weather::new(0.0), pool: DropPool::new(), always: false, amount: 1.0, seed: SEED }
    }

    /// Set the settings: rain on every track, and the amount (0 to 1).
    pub fn configure(&mut self, always: bool, amount: f32) {
        self.always = always;
        self.amount = amount.clamp(0.0, 1.0);
    }

    /// The rain intensity the weather eases toward: full when forced on, otherwise dry.
    pub fn target(&self) -> f32 {
        if self.always && self.amount > 0.0 { 1.0 } else { 0.0 }
    }

    /// The eased rain intensity, 0 to 1.
    pub fn intensity(&self) -> f32 {
        self.weather.rain
    }

    /// Advance by `dt` seconds with the camera at `camera`.
    pub fn update(&mut self, dt: f32, camera: Vec3) {
        self.weather.step(dt, self.target(), 0.0);
        let tuning = Tuning {
            fall_speed: FALL_SPEED,
            box_half: BOX_HALF,
            ground_low: camera.y - BELOW,
            ground_high: camera.y + ABOVE,
            wind_scale: 0.0,
            spawn_rate: SPAWN_RATE * self.amount,
        };
        let seed = &mut self.seed;
        let mut rand = || {
            *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (*seed >> 8) as f32 / (1 << 23) as f32 - 1.0
        };
        self.pool.update(dt, camera.to_array(), [0.0, 0.0], self.weather.rain, &tuning, &mut rand);
    }

    /// How many drops are falling.
    pub fn falling(&self) -> usize {
        self.pool.falling()
    }

    /// One quad per falling drop: four corners of a vertical streak, facing the camera.
    pub fn streaks(&self, camera: Vec3, out: &mut Vec<[Vec3; 4]>) {
        let half = Vec3::Y * (STREAK_LENGTH * 0.5);
        for drop in self.pool.drops().iter().filter(|d| d.falling) {
            let at = Vec3::from_array(drop.pos);
            let side = (camera - at).cross(Vec3::Y).normalize_or_zero() * STREAK_HALF_WIDTH;
            out.push([at - half - side, at + half - side, at + half + side, at - half + side]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;
    const CAMERA: Vec3 = Vec3::new(10.0, 2.0, -4.0);

    fn run(rain: &mut Rain, seconds: f32) {
        for _ in 0..(seconds / DT) as usize {
            rain.update(DT, CAMERA);
        }
    }

    #[test]
    fn dry_without_the_setting() {
        let mut rain = Rain::new();
        run(&mut rain, 5.0);
        assert_eq!(rain.target(), 0.0);
        assert_eq!(rain.falling(), 0);
    }

    #[test]
    fn always_rain_eases_in_and_drops_fall() {
        let mut rain = Rain::new();
        rain.configure(true, 1.0);
        run(&mut rain, 3.0);
        assert!(rain.intensity() > 0.9, "the weather eases to full rain: {}", rain.intensity());
        assert!(rain.falling() > 50, "drops spawn while it rains: {}", rain.falling());
        assert!(rain.falling() <= crate::weather::drops::POOL_SIZE);
    }

    #[test]
    fn zero_amount_is_dry_even_when_forced() {
        let mut rain = Rain::new();
        rain.configure(true, 0.0);
        run(&mut rain, 3.0);
        assert_eq!(rain.falling(), 0);
        assert_eq!(rain.intensity(), 0.0);
    }

    #[test]
    fn a_smaller_amount_spawns_fewer_drops() {
        let (mut full, mut half) = (Rain::new(), Rain::new());
        full.configure(true, 1.0);
        half.configure(true, 0.25);
        run(&mut full, 1.0);
        run(&mut half, 1.0);
        assert!(half.falling() < full.falling(), "{} vs {}", half.falling(), full.falling());
    }

    #[test]
    fn drops_stay_around_the_camera() {
        let mut rain = Rain::new();
        rain.configure(true, 1.0);
        run(&mut rain, 4.0);
        for drop in rain.pool.drops().iter().filter(|d| d.falling) {
            let [x, y, z] = drop.pos;
            assert!((x - CAMERA.x).abs() <= BOX_HALF[0] + 0.01);
            assert!((z - CAMERA.z).abs() <= BOX_HALF[2] + 0.01);
            assert!((CAMERA.y - BELOW - 0.01..=CAMERA.y + ABOVE + 0.01).contains(&y), "height {y}");
        }
    }

    #[test]
    fn streaks_are_vertical_quads_one_per_falling_drop() {
        let mut rain = Rain::new();
        rain.configure(true, 1.0);
        run(&mut rain, 2.0);
        let mut quads = Vec::new();
        rain.streaks(CAMERA, &mut quads);
        assert_eq!(quads.len(), rain.falling());
        let quad = quads[0];
        let height = quad[1].y - quad[0].y;
        assert!((height - STREAK_LENGTH).abs() < 1e-4, "streak height {height}");
        let width = (quad[0] - quad[3]).length();
        assert!((width - 2.0 * STREAK_HALF_WIDTH).abs() < 1e-4, "streak width {width}");
    }

    #[test]
    fn the_same_run_gives_the_same_drops() {
        let (mut a, mut b) = (Rain::new(), Rain::new());
        a.configure(true, 1.0);
        b.configure(true, 1.0);
        run(&mut a, 2.0);
        run(&mut b, 2.0);
        assert_eq!(a.pool.drops(), b.pool.drops());
    }
}
