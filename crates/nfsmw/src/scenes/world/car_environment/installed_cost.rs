//! Repeatable CPU cost against resident collision in the G9 road tunnel.
use super::CarEnvironment;
use blackbox_collision::{CollisionWorld, read_collision_packs};
use blackbox_gfx::{Instance, MeshHandle};
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};
use std::{hint::black_box, time::Instant};

#[test]
#[ignore = "needs NFSMW_GAME_DIR; CPU timing diagnostic"]
fn installed_car_environment_cost() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut world = CollisionWorld::new(index.collision_grid);
    let center = Vec3::new(1549.0, 4391.0, 210.0);
    for section in index.sections.iter().filter(|s| s.is_spatial() && s.distance_to(center.x, center.y) < 250.0) {
        let bytes = dir.read_range(&index.stream_file, section.file_offset.into(), section.size as usize).unwrap();
        for pack in read_collision_packs(&bytes).unwrap() {
            world.insert(pack);
        }
    }
    let mut environment = CarEnvironment::from_zones(index.track_zones.as_ref());
    environment.enabled = true;
    let mut parts = Vec::new();
    for car in 0..16 {
        let offset = car as f32 - 8.0;
        let at = center + Vec3::new(5.0 * offset, -1.5 * offset, 0.0);
        for part in 0..12 {
            parts.push(Instance::keyed(
                MeshHandle::from_raw(0),
                Mat4::from_translation(at),
                super::super::keys::car_part(car, part),
            ));
        }
    }
    let mut runs = Vec::new();
    for run in 0..12 {
        let start = Instant::now();
        for frame in 0..120 {
            environment.apply((120 * run + frame) as f32 / 60.0, &world, Some(center), black_box(&mut parts));
        }
        if run > 0 {
            runs.push(start.elapsed().as_secs_f64() * 1e6 / 120.0);
        }
    }
    assert!(parts[0].environment_occlusion > 0.0);
    runs.sort_by(f64::total_cmp);
    eprintln!("16 cars, 192 parts at 60 Hz: median {:.1} us/frame, range {:.1}–{:.1}", runs[5], runs[0], runs[10]);
}
