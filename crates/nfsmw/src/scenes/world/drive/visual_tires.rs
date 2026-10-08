//! Displayed tread geometry, independent of the physics wheel-arm layout.

use glam::{Mat4, Vec3};
use nfsmw_data::car::{CarModel, WheelPose};

#[derive(Clone, Copy)]
pub(super) struct VisualTire {
    base: Mat4,
    pivot: Vec3,
    bottom: Vec3,
    pub width: f32,
}

impl VisualTire {
    pub fn point(&self, pose: WheelPose) -> Vec3 {
        // A wheel spins, but the geometric bottom of its round tread does not.
        let steer = Mat4::from_translation(self.pivot)
            * Mat4::from_rotation_z(pose.steer)
            * Mat4::from_translation(-self.pivot);
        (Mat4::from_translation(Vec3::Z * pose.travel) * steer * self.base).transform_point3(self.bottom)
    }
}

pub(super) fn from_model(model: &CarModel) -> Option<[VisualTire; 4]> {
    let corners = model.corners.as_ref()?;
    let tires: [Option<VisualTire>; 4] = std::array::from_fn(|i| {
        let part = model.placements.iter().find(|p| p.corner == Some(i) && !p.is_brake())?;
        let solid = model.solids.get(&part.solid)?;
        let min = Vec3::from_array(solid.bounds_min);
        let max = Vec3::from_array(solid.bounds_max);
        let base = corners[i].wheel;
        let width = (max.y - min.y) * base.y_axis.truncate().length();
        if !width.is_finite() || width <= 0.0 {
            return None;
        }
        Some(VisualTire {
            base,
            pivot: corners[i].centre(),
            bottom: Vec3::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5, min.z),
            width,
        })
    });
    let [Some(a), Some(b), Some(c), Some(d)] = tires else { return None };
    Some([a, b, c, d])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tread_bottom_steers_and_lifts_but_does_not_orbit_with_spin() {
        let tire = VisualTire {
            base: Mat4::from_translation(Vec3::new(1.2, 0.8, 0.3)),
            pivot: Vec3::new(1.2, 0.8, 0.3),
            bottom: Vec3::new(0.0, 0.05, -0.3),
            width: 0.26,
        };
        let pose = WheelPose { steer: std::f32::consts::FRAC_PI_2, travel: 0.12, spin: 123.0 };
        assert!(tire.point(pose).abs_diff_eq(Vec3::new(1.15, 0.8, 0.12), 1e-5));
        assert_eq!(tire.point(pose), tire.point(WheelPose { spin: 0.0, ..pose }));
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn installed_bmw_treads_match_the_displayed_wheel_geometry() {
        use blackbox_vehicle::{FlatGround, Vehicle};
        use nfsmw_data::car::{LoadOptions, physics::PhysicsData};

        let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
        let model =
            nfsmw_data::car::load(&dir, "BMWM3GTR", &LoadOptions { lod: 'A', all_parts: false, preset: None }).unwrap();
        let physics = PhysicsData::load(&dir).unwrap().car("BMWM3GTR").unwrap();
        let tires = from_model(&model).expect("assembled BMW wheels");
        let mut vehicle = Vehicle::new(physics.spec.clone());
        let ground = FlatGround::new(0.0);
        assert!(vehicle.place_on_ground(&ground, 0.0, 0.0, 5.0, 0.0));
        for _ in 0..120 {
            vehicle.step(blackbox_vehicle::FIXED_STEP, &blackbox_vehicle::InputState::default(), &ground);
        }
        assert_eq!(vehicle.wheels_on_ground(), 4);
        let origin = vehicle.position() - vehicle.rotation() * physics.bounds.pivot;
        for (i, tire) in tires.iter().enumerate() {
            let w = vehicle.wheel([0, 1, 3, 2][i]);
            let local = vehicle.rotation().transpose() * (w.position - origin);
            let physical = Vec3::new(local.z, -local.x, local.y);
            let visual = tire.point(WheelPose::default());
            let gap = (physical.truncate() - visual.truncate()).length();
            eprintln!(
                "BMW wheel {i}: physical {physical:?}, displayed tread {visual:?}, planar gap {gap:.4} m, width {:.4} m",
                tire.width
            );
            let part = model.placements.iter().find(|p| p.corner == Some(i) && !p.is_brake()).unwrap();
            let solid = &model.solids[&part.solid];
            let pose = WheelPose { steer: 0.3, travel: 0.08, spin: 0.0 };
            let rendered = model.corners.unwrap()[i].posed(pose).0.transform_point3(Vec3::new(
                (solid.bounds_min[0] + solid.bounds_max[0]) * 0.5,
                (solid.bounds_min[1] + solid.bounds_max[1]) * 0.5,
                solid.bounds_min[2],
            ));
            assert!(tire.point(pose).abs_diff_eq(rendered, 1e-5));
        }
        // Exercise the complete contact path with a turned, suspended and spinning BMW.
        let mut sim = super::super::sim::CarSim::new(physics, model.corners.unwrap().map(|c| c.centre().z));
        sim.set_visual_tires(Some(tires));
        let ground = FlatGround::new(5.0);
        let spawn = crate::scenes::world::road::Spawn { position: Vec3::new(0.0, 0.0, 5.0), heading: 0.4 };
        assert!(sim.place(&ground, spawn));
        for _ in 0..120 {
            sim.step(&super::super::input::DriveInput::default(), &ground, None);
        }
        for _ in 0..30 {
            sim.step(
                &super::super::input::DriveInput { throttle: 0.3, steer: 0.8, ..Default::default() },
                &ground,
                None,
            );
        }
        let pose = sim.pose();
        let contacts = sim.tire_contacts(&road());
        assert_eq!(contacts.iter().flatten().count(), 4);
        for (i, tire) in tires.iter().enumerate() {
            let contact = contacts[[0, 1, 3, 2][i]].unwrap();
            let displayed = pose.transform().transform_point3(tire.point(pose.wheels[i]));
            assert!(contact.point.truncate().abs_diff_eq(displayed.truncate(), 1e-4));
            assert!((contact.point.z - 5.0).abs() < 1e-4);
            assert_eq!(contact.width, tire.width);
        }
    }

    fn road() -> blackbox_collision::CollisionWorld {
        use blackbox_collision::{Article, CollisionPack, CollisionWorld, Instance, PackedVert, Strip};
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
        let mut road = CollisionWorld::new(None);
        road.insert(CollisionPack { section: 7, instances: vec![instance], articles: vec![article], objects: vec![] });
        road
    }
}
