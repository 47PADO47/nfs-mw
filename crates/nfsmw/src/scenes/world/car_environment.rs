//! Stable vehicle keys share one shelter estimate across bodywork, wheels and views.
use blackbox_collision::{CollisionWorld, RayOptions};
use blackbox_gfx::Instance;
use glam::Vec3;
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct CarEnvironment {
    pub enabled: bool,
    cars: HashMap<u32, Sample>,
}

struct Sample {
    time: f32,
    occlusion: f32,
}

impl CarEnvironment {
    pub fn apply(&mut self, now: f32, world: &CollisionWorld, probe: Option<Vec3>, instances: &mut [Instance]) {
        if !self.enabled {
            self.cars.clear();
            return;
        }
        self.cars.retain(|_, sample| now - sample.time < 0.5);
        let lead = super::keys::car_part(super::keys::PLAYER_CAR, 0).parts().0;
        let probe_shelter = probe.map_or(0.0, |at| self.sample(now, world, lead, at));
        for instance in instances {
            let group = instance.key.parts().0;
            if !super::keys::is_car(group) {
                continue;
            }
            let blocked = self.sample(now, world, group, instance.transform.w_axis.truncate());
            // A sheltered capture under-lights remote cars that can see the sky.
            let exposure_gain = 0.6 * (probe_shelter - blocked).max(0.0);
            instance.environment_occlusion = blocked - exposure_gain;
        }
    }

    fn sample(&mut self, now: f32, world: &CollisionWorld, group: u32, at: Vec3) -> f32 {
        let origin = Vec3::from_array(super::space::to_physics(at));
        let sample = self.cars.entry(group).or_insert_with(|| Sample { time: now, occlusion: shelter(world, origin) });
        let dt = now - sample.time;
        if dt < 1.0 / 30.0 {
            return sample.occlusion;
        }
        let target = shelter(world, origin);
        let rate = match target > sample.occlusion {
            true => 12.0,
            false => 6.0,
        };
        sample.occlusion += (target - sample.occlusion) * (1.0 - (-rate * dt).exp());
        sample.time = now;
        sample.occlusion
    }
}

fn shelter(world: &CollisionWorld, origin: Vec3) -> f32 {
    let start = origin + Vec3::Y * 0.8;
    let opts = RayOptions { skip_groups: true, ..Default::default() };
    let mut blocked = 0.0;
    for (x, z) in [(0., 0.), (1., 0.), (-1., 0.), (0., 1.), (0., -1.), (1., 1.), (-1., 1.), (1., -1.), (-1., -1.)] {
        let end = start + Vec3::new(x, 1., z).normalize() * 22.;
        if let Some(hit) = world.ray_cast(start.to_array(), end.to_array(), &opts) {
            blocked += 1.0 - hit.t * hit.t;
        }
    }
    0.78 * blocked / 9.0
}

#[cfg(test)]
mod tests {
    use super::shelter;
    use blackbox_collision::{Article, CollisionPack, CollisionWorld, Instance, PackedVert, Strip};
    use glam::Vec3;
    fn roof() -> CollisionWorld {
        let verts = [(-1280, -1280), (1280, -1280), (-1280, 1280), (1280, 1280)]
            .into_iter()
            .map(|(x, z)| PackedVert { x, y: 0, z, surface: 0, flags: 0 })
            .collect();
        let article = Article {
            strips: vec![Strip { center: [0.0; 3], radius: 15.0, flags: 2, verts }],
            barriers: vec![],
            surfaces: vec![123],
            intermediate_object: 0,
            flags: 0,
        };
        let instance = Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            half_width: 10.0,
            half_length: 10.0,
            translation: [0.0, -5.0, 0.0],
            radius: 15.0,
            half_height: 1.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        };
        let mut world = CollisionWorld::new(None);
        world.insert(CollisionPack { section: 7, instances: vec![instance], articles: vec![article], objects: vec![] });
        world
    }

    #[test]
    fn roof_blocks_only_the_vehicle_below_it() {
        let world = roof();
        assert!(shelter(&world, Vec3::ZERO) > 0.5);
        assert_eq!(shelter(&world, Vec3::X * 50.0), 0.0);
        assert_eq!(shelter(&world, Vec3::Y * 6.0), 0.0);
    }
    #[test]
    fn cars_start_at_their_own_exposure_and_adapt_when_they_move() {
        use blackbox_gfx::{Instance, MeshHandle};
        use glam::Mat4;
        let world = roof();
        let outside = Vec3::X * 50.0;
        let car = |id, at| {
            Instance::keyed(MeshHandle::from_raw(0), Mat4::from_translation(at), super::super::keys::car_part(id, 0))
        };
        let mut cars = [car(0, outside), car(1, Vec3::ZERO)];
        let mut environment = super::CarEnvironment { enabled: true, ..Default::default() };
        environment.apply(0.0, &world, Some(outside), &mut cars);
        let enclosed = cars[1].environment_occlusion;
        assert!(enclosed > 0.5);
        assert_eq!(cars[0].environment_occlusion, 0.0);
        cars[1].transform = Mat4::from_translation(outside);
        environment.apply(0.05, &world, Some(outside), &mut cars);
        assert!((0.0..enclosed).contains(&cars[1].environment_occlusion));
        let mut swapped = [car(0, Vec3::ZERO), car(1, outside)];
        environment.cars.clear();
        environment.apply(1.0, &world, Some(Vec3::ZERO), &mut swapped);
        assert!(swapped[1].environment_occlusion < 0.0);
    }
}
