//! Stock PC precipitation in the renderer's Z-up world. Spec: `rain-pc.md`.
use super::{drops::DropPool, lens::LensRain};
use blackbox_hash::bstring_hash;
use blackbox_paramap::Layer;
use glam::Vec3;

#[derive(Debug, Clone)]
pub struct Rain {
    zones: Option<blackbox_roads::TrackZones>,
    curtain: super::curtain::Curtain,
    layer: Option<Layer>,
    intensity: f32,
    pool: DropPool,
    lens: LensRain,
    always: bool,
    amount: f32,
    exposed: bool,
    previous: Option<Vec3>,
}
impl Default for Rain {
    fn default() -> Self {
        Self::new()
    }
}
impl Rain {
    pub fn new() -> Self {
        Self {
            zones: None,
            curtain: Default::default(),
            layer: None,
            intensity: 0.0,
            pool: DropPool::default(),
            lens: LensRain::default(),
            always: false,
            amount: 1.0,
            exposed: true,
            previous: None,
        }
    }
    pub fn read(metadata: &[u8]) -> anyhow::Result<Self> {
        let layer = Layer::read_all(metadata)?.into_iter().find(|l| l.name_hash == bstring_hash("Rain"));
        Ok(Self { layer, zones: blackbox_roads::TrackZones::read(metadata)?, ..Self::new() })
    }
    pub fn configure(&mut self, always: bool, amount: f32) {
        self.always = always;
        if amount.is_finite() {
            self.amount = amount.clamp(0.0, 1.0);
        }
    }
    pub fn target_at(&self, camera: Vec3) -> f32 {
        if self.amount == 0.0 {
            return 0.0;
        }
        if self.always {
            return 1.0;
        }
        let Some(layer) = &self.layer else { return 0.0 };
        let Some(set) = layer.set_at(camera.x, camera.y) else { return 0.0 };
        let (Some(value), Some(threshold)) = (layer.float(set, 0), layer.float(set, 1)) else { return 0.0 };
        if !value.is_finite() || !threshold.is_finite() || super::regional_gate(1.0, camera.x) > threshold {
            return 0.0;
        }
        value.clamp(0.0, 1.0)
    }
    pub fn target(&self) -> f32 {
        self.target_at(self.previous.unwrap_or(Vec3::ZERO))
    }
    pub fn intensity(&self) -> f32 {
        self.intensity
    }
    pub fn lens(&self) -> &[super::lens::Droplet] {
        self.lens.visible(self.exposed && self.intensity > 0.0 && self.amount > 0.0)
    }
    #[cfg(test)]
    pub fn update(&mut self, dt: f32, camera: Vec3) {
        self.update_view(dt, camera, Vec3::X, true);
    }
    pub fn update_view(&mut self, dt: f32, camera: Vec3, forward: Vec3, exposed: bool) {
        if !dt.is_finite() || dt <= 0.0 || !camera.is_finite() || !forward.is_finite() {
            return;
        }
        self.exposed = exposed;
        let target = self.target_at(camera);
        let alpha = 1.0 - (59.0_f32 / 60.0).powf(60.0 * dt);
        self.intensity += (target - self.intensity) * alpha;
        if target == 0.0 && self.intensity < 0.01 {
            self.intensity = 0.0;
        }
        let displacement = self.previous.map_or(Vec3::ZERO, |old| camera - old);
        self.previous = Some(camera);
        self.pool.update(dt, camera, forward, self.intensity * self.amount, exposed);
        let zone = self.zones.as_ref().and_then(|zones| {
            zones.zones.iter().find(|zone| {
                matches!(zone.kind, 3 | 4 | 5 | 7) && zone.contains(camera.truncate()) && zone.elevation > camera.z
            })
        });
        self.curtain.update(dt, camera, forward, zone.filter(|_| !exposed), self.intensity * self.amount);
        self.lens.update(dt, (displacement / dt).length(), exposed && self.intensity > 0.0 && self.amount > 0.0);
    }
    pub fn falling(&self) -> usize {
        self.pool.falling()
    }
    pub fn quads(&self, camera: Vec3, out: &mut Vec<([Vec3; 4], usize)>) {
        self.pool.quads(camera, out);
        self.curtain.quads(camera, out);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires the original global texture and track packs"]
    fn installed_rain_textures_and_regional_map_are_present() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let globals = nfsmw_data::world::load_global_textures(&dir).unwrap();
        for name in ["RAINDROP", "RAINDROPPC"] {
            let texture = globals.textures.iter().find(|t| t.name_hash == bstring_hash(name)).expect(name);
            assert!(texture.width > 0 && texture.height > 0);
        }
        let bytes = nfsmw_data::read_unwrapped(&dir, "TRACKS/L2RA.BUN").unwrap();
        let rain = Rain::read(&bytes).unwrap();
        let layer = rain.layer.expect("installed Rain parameter map");
        assert!(layer.set_count() > 0);
        assert!((0..layer.set_count()).any(|set| layer.float(set, 0).is_some_and(|v| v > 0.0)));
    }
    #[test]
    fn shelter_blocks_lens_water_without_drying_the_weather() {
        let mut rain = Rain::new();
        rain.configure(true, 1.0);
        rain.update_view(3.0, Vec3::ZERO, Vec3::X, false);
        assert!(rain.intensity() > 0.9);
        assert!(rain.lens().is_empty());
        assert_eq!(rain.falling(), 0);
        rain.update_view(1.0 / 60.0, Vec3::ZERO, Vec3::X, true);
        assert!(!rain.lens().is_empty());
        assert!(rain.falling() > 300);
    }
    #[test]
    fn pausing_holds_weather_and_droplets() {
        let mut rain = Rain::new();
        rain.configure(true, 1.0);
        rain.update(1.0, Vec3::ZERO);
        let before = rain.clone();
        for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            rain.update(dt, Vec3::splat(100.0));
        }
        assert_eq!(rain.intensity(), before.intensity());
        assert_eq!(rain.lens(), before.lens());
        assert_eq!(rain.pool, before.pool);
    }
    #[test]
    fn zero_amount_removes_precipitation() {
        let mut rain = Rain::new();
        rain.configure(true, 0.0);
        rain.update(5.0, Vec3::ZERO);
        assert_eq!(rain.falling(), 0);
        assert_eq!(rain.intensity(), 0.0);
        assert!(rain.lens().is_empty());
    }
}
