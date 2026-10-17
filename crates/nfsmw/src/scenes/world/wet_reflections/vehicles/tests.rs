use super::*;
use crate::scenes::world::keys;
use blackbox_gfx::MeshHandle;
use glam::{Quat, Vec3};

fn near(actual: Mat4, expected: Mat4) {
    let error = actual
        .to_cols_array()
        .into_iter()
        .zip(expected.to_cols_array())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f32, f32::max);
    assert!(error < 0.001, "root/child transform difference {error}");
}

#[test]
fn reflected_roots_preserve_part_locals_keys_and_existing_world_fragments() {
    let mesh = MeshHandle::from_raw(1);
    let old = Instance::keyed(mesh, Mat4::from_translation(Vec3::new(9.0, 8.0, 7.0)), keys::car_part(99, 0));
    let mut out = vec![old];
    for (car, at, angles) in [
        (0, Vec3::new(1945.0, 1850.0, 149.9), Vec3::new(0.23, -0.4, 1.2)),
        (8, Vec3::new(1940.0, 1843.0, 150.4), Vec3::new(-0.7, 0.13, -0.6)),
    ] {
        let root =
            Mat4::from_rotation_translation(Quat::from_euler(glam::EulerRot::XYZ, angles.x, angles.y, angles.z), at);
        let locals = [
            Mat4::IDENTITY,
            Mat4::from_translation(Vec3::new(1.2, -0.8, -0.28))
                * Mat4::from_rotation_z(0.3)
                * Mat4::from_rotation_y(2.4)
                * Mat4::from_scale(Vec3::new(1.3, 0.8, 1.3)),
        ];
        let start = out.len();
        append(root, &mut out, |out| {
            out.extend(
                locals
                    .into_iter()
                    .enumerate()
                    .map(|(part, local)| Instance::keyed(mesh, root * local, keys::car_part(car, part))),
            );
        });
        let mut reflected = root;
        reflected.z_axis = -reflected.z_axis;
        for (part, local) in locals.into_iter().enumerate() {
            assert_eq!(out[start + part].key, keys::car_part(car, part));
            assert_eq!(out[start + part].mesh, mesh);
            near(out[start + part].transform, reflected * local);
        }
    }
    assert_eq!(out[0].transform, old.transform, "previous fragment is not attached to either vehicle");
    assert_eq!(out[0].key, old.key);
}

#[test]
fn invalid_roots_do_not_append_or_poison_the_capture() {
    for root in [Mat4::ZERO, Mat4::from_translation(Vec3::new(f32::NAN, 0.0, 0.0))] {
        append(root, &mut Vec::new(), |_| panic!("invalid root must not draw"));
    }
}

#[test]
#[ignore = "requires installed player, cop, tractor and trailer models"]
fn installed_vehicle_capture_preserves_real_body_wheel_meshes_and_keys() {
    use crate::app::test_backend::NullBackend;
    use crate::scenes::world::drive::{CarPose, CarRig};
    use crate::settings::CarShading;
    use nfsmw_data::car::{LoadOptions, WheelPose};

    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking();
    for (car, name) in ["BMWM3GTR", "COPMIDSIZE", "SEMI", "TRAILERLOG"].into_iter().enumerate() {
        let model = nfsmw_data::car::load_driving(&dir, name, &options).unwrap();
        let rig = CarRig::upload(&mut renderer, car as u32, model, CarShading::Original, Vec3::Z);
        let pose = CarPose {
            position: Vec3::new(1945.0 - car as f32 * 3.0, 1850.0, 149.9),
            rotation: Quat::from_euler(glam::EulerRot::XYZ, 0.1, -0.2, 0.6),
            wheels: [WheelPose { steer: 0.25, spin: 1.3, travel: 0.06 }; 4],
        };
        let mut normal = Vec::new();
        rig.instances_for(car as u32, &pose, None, &Default::default(), &mut normal);
        assert!(!normal.is_empty(), "{name}: actual body/child feed");
        let root = pose.transform();
        let mut reflected = root;
        reflected.z_axis = -reflected.z_axis;
        let count = renderer.mesh_labels.len();
        let mut wet = Vec::new();
        append(root, &mut wet, |out| rig.instances_for(car as u32, &pose, None, &Default::default(), out));
        assert_eq!(wet.len(), normal.len());
        for (actual, original) in wet.iter().zip(normal) {
            assert_eq!((actual.key, actual.mesh), (original.key, original.mesh));
            near(actual.transform, reflected * (root.inverse() * original.transform));
        }
        assert_eq!(renderer.mesh_labels.len(), count, "reflection cannot upload meshes");
        rig.release(&mut renderer);
    }
}
