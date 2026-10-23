//! Surface-selected PC tire particles, wet or dry. Spec: `rain-pc.md` and `tire-surface-dust.md`.
use super::super::drive::CarPose;
use super::pc_sparks::PcSparks;
use glam::Vec3;
use nfsmw_data::vehicle_effects::{AxleScale, TireWaterData};

#[derive(Clone, Copy)]
pub(in crate::scenes::world) struct WaterContact {
    pub point: Vec3,
    pub surface: u32,
    pub normal: Vec3,
    pub slip: f32,
    pub slide: f32,
    pub flat: bool,
}
pub(in crate::scenes::world) struct WaterEffects {
    data: TireWaterData,
    groups: [PcSparks; 12],
    /// Rain on the car: tires run their wet records, else their dry ones (dust, grass, leaves).
    pub wet: bool,
    /// The car's per-axle slip and slide factors (`AxleScale`).
    pub scale: AxleScale,
    pub surface_water: Option<(blackbox_gfx::SurfaceWater, blackbox_gfx::RenderExperiments)>,
}
impl WaterEffects {
    pub fn new(data: TireWaterData) -> Self {
        Self {
            data,
            groups: std::array::from_fn(|_| {
                let mut group = PcSparks::default();
                group.set_limit(128);
                group
            }),
            wet: false,
            scale: AxleScale::default(),
            surface_water: None,
        }
    }
    pub fn clear(&mut self) {
        for group in &mut self.groups {
            group.clear();
        }
    }
    pub fn disconnect(&mut self) {
        for group in &mut self.groups {
            group.disconnect();
        }
    }
    pub fn age(&mut self, dt: f32) {
        for group in &mut self.groups {
            group.age(dt);
        }
    }
    pub fn status(&self) -> String {
        format!(
            "tire surface wet={}: {} live, {} emitted",
            self.wet,
            self.groups.iter().map(PcSparks::len).sum::<usize>(),
            self.groups.iter().map(|group| group.emitted).sum::<u64>(),
        )
    }
    pub fn step(&mut self, contacts: [Option<WaterContact>; 4], pose: CarPose, velocity: Vec3, dt: f32) {
        for (wheel, contact) in contacts.into_iter().enumerate() {
            for kind in 0..3 {
                let group = &mut self.groups[wheel * 3 + kind];
                let selected = contact.and_then(|c| {
                    let effect = self.data.effect(c.surface, kind, self.wet, c.flat)?;
                    let speed = [velocity.length(), c.slip.abs(), c.slide.abs()][kind] * self.scale.factor(wheel, kind);
                    let amount = self.surface_water.filter(|_| self.wet).map_or(1.0, |(water, options)| {
                        let slope = ((c.normal.z - 0.90) / 0.095).clamp(0.0, 1.0);
                        let puddle =
                            water.puddle(c.point.truncate().to_array(), options.puddle_coverage, options.puddle_scale)
                                * slope
                                * slope
                                * (3.0 - 2.0 * slope);
                        (water.dampness * 0.35 + puddle * 0.65).clamp(0.0, 1.0)
                    });
                    let intensity = ((speed - effect.min) / (effect.max - effect.min)).clamp(0.0, 1.0) * amount;
                    (intensity > 0.0).then_some((effect, c.point, intensity))
                });
                let Some((effect, point, intensity)) = selected else {
                    group.stop_scrape();
                    continue;
                };
                group.surface_group(effect.emitters, point, pose.rotation, velocity, intensity);
                group.update(dt);
            }
        }
    }
    pub fn geometry(
        &self,
        forward: Vec3,
        textures: &std::collections::HashMap<u32, (blackbox_gfx::TextureHandle, blackbox_gfx::BlendMode)>,
        out: &mut Vec<blackbox_gfx::TexturedEffect>,
    ) {
        for group in &self.groups {
            group.geometry_with_brightness(forward, textures, out, 2.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires the original surface and emitter profiles"]
    fn installed_wet_tire_groups_emit_and_stop_when_contact_is_lost() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let db = blackbox_attrib::Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
        let data = TireWaterData::read(&db);
        let surface = db.collection("simsurface", "asphalt").expect("installed asphalt").key();
        let effect = data.effect(surface, 0, true, false).expect("installed asphalt driving spray");
        let mut water = WaterEffects::new(data);
        water.wet = true;
        let pose = CarPose {
            position: Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            wheels: [nfsmw_data::car::WheelPose::default(); 4],
        };
        let contact = WaterContact { point: Vec3::ZERO, normal: Vec3::Z, surface, slip: 0.0, slide: 0.0, flat: false };
        for _ in 0..120 {
            water.age(1.0 / 60.0);
            water.step([Some(contact); 4], pose, Vec3::X * effect.max, 1.0 / 60.0);
        }
        let emitted = water.groups.iter().map(|g| g.emitted).sum::<u64>();
        assert!(emitted > 0, "installed surface must actually emit particles");
        water.step([None; 4], pose, Vec3::X * effect.max, 1.0 / 60.0);
        assert!(water.groups.iter().all(|g| !g.pending()));
        assert_eq!(emitted, water.groups.iter().map(|g| g.emitted).sum::<u64>());
    }
    fn pose() -> CarPose {
        CarPose {
            position: Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            wheels: [nfsmw_data::car::WheelPose::default(); 4],
        }
    }
    fn contact(surface: u32, slide: f32) -> WaterContact {
        WaterContact { point: Vec3::ZERO, normal: Vec3::Z, surface, slip: 0.0, slide, flat: false }
    }
    fn emitted(water: &WaterEffects) -> u64 {
        water.groups.iter().map(|g| g.emitted).sum()
    }
    fn installed() -> (blackbox_attrib::Database, TireWaterData) {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let db = blackbox_attrib::Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
        let data = TireWaterData::read(&db);
        (db, data)
    }

    #[test]
    #[ignore = "requires the original surface and emitter profiles"]
    fn installed_dry_loose_ground_kicks_up_dust_and_grass_without_rain() {
        let (db, data) = installed();
        for name in ["grass", "golf_fairway", "sand", "dirt"] {
            let surface = db.collection("simsurface", name).expect("installed loose surface").key();
            let effect = data.effect(surface, 0, false, false).expect("dry driving record");
            let mut water = WaterEffects::new(data.clone());
            assert!(!water.wet, "dry weather");
            for _ in 0..120 {
                water.age(1.0 / 60.0);
                water.step([Some(contact(surface, 0.0)); 4], pose(), Vec3::X * effect.max, 1.0 / 60.0);
            }
            assert!(emitted(&water) > 0, "{name} must emit dry particles with no rain");
            water.step([None; 4], pose(), Vec3::X * effect.max, 1.0 / 60.0);
            assert!(water.groups.iter().all(|g| !g.pending()), "{name}: leaving the ground ends emission");
        }
    }

    #[test]
    #[ignore = "requires the original surface and emitter profiles"]
    fn installed_dry_asphalt_is_silent_until_it_slides_and_then_throws_leaves() {
        let (db, data) = installed();
        let asphalt = db.collection("simsurface", "asphalt").expect("installed asphalt").key();
        let mut water = WaterEffects::new(data);
        for _ in 0..120 {
            water.age(1.0 / 60.0);
            water.step([Some(contact(asphalt, 0.0)); 4], pose(), Vec3::X * 40.0, 1.0 / 60.0);
        }
        assert_eq!(emitted(&water), 0, "straight dry asphalt has no dust and the smoke stays procedural");
        for _ in 0..240 {
            water.age(1.0 / 60.0);
            water.step([Some(contact(asphalt, 3.0)); 4], pose(), Vec3::X * 40.0, 1.0 / 60.0);
        }
        assert!(emitted(&water) > 0, "a sliding tire throws the authored leaves");
    }
    #[test]
    #[ignore = "requires the original surface and emitter profiles"]
    fn installed_front_axle_slide_is_damped_so_only_the_rear_throws_leaves() {
        let (db, data) = installed();
        let asphalt = db.collection("simsurface", "asphalt").expect("installed asphalt").key();
        let scale = nfsmw_data::vehicle_effects::AxleScale::read(&db, "BMWM3GTR");
        let run = |wheels: [bool; 4]| {
            let mut water = WaterEffects::new(data.clone());
            water.scale = scale;
            for _ in 0..240 {
                water.age(1.0 / 60.0);
                let contacts = wheels.map(|on| on.then(|| contact(asphalt, 1.5)));
                water.step(contacts, pose(), Vec3::X * 40.0, 1.0 / 60.0);
            }
            emitted(&water)
        };
        assert_eq!(run([true, true, false, false]), 0, "1.5 m/s on the front axle scales to 0.15, below the ramp");
        assert!(run([false, false, true, true]) > 0, "the rear axle keeps the full slide");
    }
}
