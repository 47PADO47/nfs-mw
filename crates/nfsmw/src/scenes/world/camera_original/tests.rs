use blackbox_collision::{Article, Barrier, CollisionPack, CollisionWorld, Instance, RayOptions};
use glam::Vec3;

use super::{space, wall_hit};

pub(super) fn world(group: u16) -> CollisionWorld {
    let mut world = CollisionWorld::default();
    world.insert(CollisionPack {
        section: 1,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [0.0; 3],
            half_width: 10.0,
            half_length: 10.0,
            half_height: 3.0,
            radius: 15.0,
            iter_stamp: 0,
            flags: 0,
            group,
            article: 0,
        }],
        articles: vec![Article {
            strips: vec![],
            barriers: vec![Barrier {
                p0: [-10.0, -1.0, -3.0],
                p1: [10.0, 2.0, -3.0],
                surface: 0,
                flags: 0,
                inv_xz_len: 0.05,
            }],
            surfaces: vec![],
            intermediate_object: 0,
            flags: 0,
        }],
        objects: vec![],
    });
    world
}

#[test]
fn road_height_changes_cannot_activate_an_inactive_camera_barrier() {
    let collision = world(9);
    let mut old_hits = 0;
    let mut old_misses = 0;
    for step in 0..120 {
        let height = 1.0 + (step as f32 * 0.1).sin() * 0.5;
        let car = Vec3::Z * height;
        let end = car + Vec3::new(-6.2, 0.0, 1.9);
        let old = collision.ray_cast(
            space::to_physics(car),
            space::to_physics(end),
            &RayOptions { faces: false, barriers: true, exclude: 0, skip_groups: false },
        );
        old_hits += usize::from(old.is_some());
        old_misses += usize::from(old.is_none());
        assert!(wall_hit(&collision, car, end).is_none());
    }
    assert!(old_hits > 0 && old_misses > 0, "height must reproduce intermittent false obstruction");
}

#[test]
fn ordinary_camera_walls_still_obstruct_from_both_directions() {
    let collision = world(0);
    let a = Vec3::new(0.0, 0.0, 0.5);
    let b = Vec3::new(-6.0, 0.0, 1.5);
    for (from, to) in [(a, b), (b, a)] {
        assert!(wall_hit(&collision, from, to).is_some());
    }
    assert!(wall_hit(&collision, a + Vec3::Z * 5.0, b + Vec3::Z * 5.0).is_none());
}

#[test]
#[ignore = "needs the game selected by NFSMW_GAME_DIR"]
fn installed_far_camera_skips_inactive_barriers_on_height_changing_roads() {
    use blackbox_roads::centre_line;
    use game_install::GameDir;
    use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let network = index.road_network.as_ref().expect("road network");
    let mut collision = CollisionWorld::new(index.collision_grid.clone());
    for section in &index.sections {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        for pack in blackbox_collision::read_collision_packs(&bytes).unwrap() {
            collision.insert(pack);
        }
    }
    let raw = RayOptions { faces: false, barriers: true, exclude: 0, skip_groups: false };
    let ground = RayOptions { faces: true, barriers: false, exclude: 0, skip_groups: true };
    let mut witnesses = 0;
    let mut slopes = 0;
    for segment in 0..network.segments.len() {
        let curve = centre_line(network, segment as u16, 1);
        for i in 1..20 {
            let t = i as f32 / 20.0;
            let p = curve.position(t);
            let forward = curve.tangent(t).normalize_or(Vec3::Z);
            if forward.y.abs() < 0.02 {
                continue;
            }
            let Some(road) =
                collision.ray_cast((p + Vec3::Y * 2.0).to_array(), (p - Vec3::Y * 4.0).to_array(), &ground)
            else {
                continue;
            };
            slopes += 1;
            let car = space::to_render(road.point) + Vec3::Z * 0.6;
            let ahead = space::to_render(forward.to_array());
            let eye = car - ahead * 5.44 + Vec3::Z * 1.7;
            let delta = car - eye;
            let width = (16382.0_f32 * 0.000_078_473_77).clamp(1.0, 1.5) * 0.44;
            let side = Vec3::Z.cross(delta.normalize()) * (delta.length() * width);
            for end in [eye - (delta + side).normalize() * 0.9, eye - (delta - side).normalize() * 0.9] {
                let Some(hit) = collision.ray_cast(space::to_physics(car), space::to_physics(end), &raw) else {
                    continue;
                };
                let instance = &collision.pack(hit.section).unwrap().instances[hit.instance];
                if instance.group == 0 {
                    continue;
                }
                let fixed = wall_hit(&collision, car, end);
                assert!(
                    fixed.is_none_or(|h| h.point.distance(car) + 0.001 >= space::to_render(hit.point).distance(car))
                );
                witnesses += 1;
                if witnesses <= 6 {
                    eprintln!(
                        "slope={:.3} car={car:?} false hit={}/{} group={} distance={:.2} fixed={fixed:?}",
                        forward.y,
                        hit.section,
                        hit.instance,
                        instance.group,
                        space::to_render(hit.point).distance(car)
                    );
                }
            }
        }
    }
    eprintln!("{slopes} installed road slope samples, {witnesses} inactive camera-barrier hits");
    assert!(slopes > 100 && witnesses > 0, "must exercise actual sloping roads and inactive barriers");
}
