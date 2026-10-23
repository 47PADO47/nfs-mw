//! Surface-selected PC tire particles. Spec: `rain-pc.md`.
use super::super::drive::CarPose;
use super::pc_sparks::PcSparks;
use glam::Vec3;
use nfsmw_data::vehicle_effects::TireWaterData;

#[derive(Clone, Copy)]
pub(in crate::scenes::world) struct WaterContact {
    pub point: Vec3,
    pub surface: u32,
    pub slip: f32,
    pub slide: f32,
    pub flat: bool,
}
pub(in crate::scenes::world) struct WaterEffects {
    data: TireWaterData,
    groups: [PcSparks; 12],
    pub active: bool,
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
            active: false,
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
            "tire water {}: {} live, {} emitted",
            self.active,
            self.groups.iter().map(PcSparks::len).sum::<usize>(),
            self.groups.iter().map(|group| group.emitted).sum::<u64>(),
        )
    }
    pub fn step(&mut self, contacts: [Option<WaterContact>; 4], pose: CarPose, velocity: Vec3, dt: f32) {
        for (wheel, contact) in contacts.into_iter().enumerate() {
            for kind in 0..3 {
                let group = &mut self.groups[wheel * 3 + kind];
                let selected = contact.filter(|_| self.active).and_then(|c| {
                    let effect = self.data.wet(c.surface, kind, c.flat)?;
                    let speed = [velocity.length(), c.slip.abs(), c.slide.abs()][kind];
                    let intensity = ((speed - effect.min) / (effect.max - effect.min)).clamp(0.0, 1.0);
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
        let effect = data.wet(surface, 0, false).expect("installed asphalt driving spray");
        let mut water = WaterEffects::new(data);
        water.active = true;
        let pose = CarPose {
            position: Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            wheels: [nfsmw_data::car::WheelPose::default(); 4],
        };
        let contact = WaterContact { point: Vec3::ZERO, surface, slip: 0.0, slide: 0.0, flat: false };
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
}
