//! Source-authored heavy attachments and parked vehicles through production contacts.

use super::*;
use crate::scenes::world::object_geometry::{BodyGeometryFrame, object_shapes};
use blackbox_attrib::vlt_hash;
use blackbox_scenery::{layout::MOST_WANTED, read_object_spawners, read_scenery_sections};
use game_install::GameDir;
use nfsmw_data::{
    car::physics::PhysicsData,
    world::{DEFAULT_TRACK, PropCatalog, WorldIndex},
};

fn fixtures(dir: &GameDir, index: &WorldIndex, data: &PhysicsData) -> Vec<Placed> {
    let catalog = PropCatalog::new(index.prop_bounds.clone(), data.database());
    let mut found = std::collections::BTreeMap::new();
    // The source minivan profile exists but has no authored placement in this track.
    let wanted = ["tollbooth", "parked_car"];
    for section in &index.sections {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let scenery = read_scenery_sections(&bytes, &MOST_WANTED).unwrap();
        for spawn in read_object_spawners(&bytes, &MOST_WANTED.spawners).unwrap() {
            if !wanted.into_iter().any(|name| vlt_hash(name) == spawn.attributes)
                || found.contains_key(&spawn.attributes)
            {
                continue;
            }
            let (sc, hook) = scenery
                .iter()
                .find_map(|sc| {
                    sc.override_hooks
                        .iter()
                        .find(|h| u32::from(h.override_index) == spawn.override_index)
                        .map(|h| (sc, h))
                })
                .unwrap();
            let instance = &sc.instances[usize::from(hook.instance_index)];
            let shape = catalog.for_spawner(&sc.info_of(instance).unwrap().name, &spawn).unwrap();
            let mut placed = Placed::prop_only(Mat4::from_cols_array_2d(&instance.matrix_columns()), shape);
            placed.bounds = Aabb::new(instance.bbox_min, instance.bbox_max);
            placed.spawner = Some(spawn);
            found.insert(spawn.attributes, placed);
        }
    }
    assert_eq!(found.keys().copied().collect::<std::collections::BTreeSet<_>>(), wanted.map(vlt_hash).into());
    found.into_values().collect()
}

#[derive(Clone, Copy)]
struct StrikePose {
    position: Vec3,
    orientation: Quat,
    direction: Vec3,
}

fn fixture_pose(placed: &Placed, data: &PhysicsData) -> StrikePose {
    let mut placed = placed.clone();
    let mut world = PropWorld::default();
    world.add_tile(0, std::slice::from_mut(&mut placed));
    let body = world.colliders[&placed.prop_id].body.as_ref().unwrap();
    let physics = data.car("BMWM3GTR").unwrap();
    let geometry = &physics.geometry.as_ref().unwrap().body;
    let targets = object_shapes(&body.rigid, Some(body.geometry_frame()));
    for target in &targets {
        let basis = Quat::from_mat3(&target.bounds.axes);
        let direction = target.bounds.axes.z_axis;
        for primitive in geometry.primitives.iter().filter(|p| p.collides(flags::PRIM_VS_OBJECTS)) {
            for yaw in [0.12, -0.12, 0.3, -0.3, 0.0] {
                let orientation = basis * Quat::from_rotation_y(yaw) * primitive.rotation.inverse();
                let axes = glam::Mat3::from_quat(orientation * primitive.rotation);
                let reach = primitive.half.x * direction.dot(axes.x_axis).abs()
                    + primitive.half.y * direction.dot(axes.y_axis).abs()
                    + primitive.half.z * direction.dot(axes.z_axis).abs();
                for overlap in [0.15, 0.3, 0.5] {
                    let centre = target.bounds.centre - direction * (target.bounds.half.z + reach - overlap);
                    let position = centre - orientation * primitive.centre;
                    // Native clipping can reject degenerate almost-parallel corners.
                    // Select one actual approaching contact at both test speeds.
                    let valid = [0.01, 60.0].into_iter().all(|speed| {
                        let mut car = Vehicle::new(physics.spec.clone());
                        car.body_mut().place(position, orientation);
                        let frame = BodyGeometryFrame {
                            geometry,
                            previous: (position - direction * speed / 60.0, orientation),
                        };
                        object_shapes(car.body(), Some(frame)).iter().any(|a| {
                            targets.iter().any(|b| a.contact(b).is_some_and(|hit| -direction.dot(hit.normal) > 0.9))
                        })
                    });
                    if valid {
                        return StrikePose { position, orientation, direction };
                    }
                }
            }
        }
    }
    panic!("no real approaching source contact for {}", placed.prop.as_ref().unwrap().name);
}

fn strike(
    placed: &Placed,
    data: &PhysicsData,
    pose: StrikePose,
    speed: f32,
    attached: bool,
) -> (PropWorld, Vehicle, Impact) {
    let mut world = PropWorld::default();
    let mut placed = placed.clone();
    world.add_tile(0, std::slice::from_mut(&mut placed));
    let body = world.colliders.get_mut(&placed.prop_id).unwrap().body.as_mut().unwrap();
    body.attached = attached;
    let physics = data.car("BMWM3GTR").unwrap();
    let geometry = &physics.geometry.as_ref().unwrap().body;
    let StrikePose { position, orientation, direction } = pose;
    let mut car = Vehicle::new(physics.spec);
    car.body_mut().place(position, orientation);
    car.body_mut().linear_velocity = direction * speed;
    let frame = BodyGeometryFrame { geometry, previous: (position - direction * speed / 60.0, orientation) };
    let mut impact = Impact::default();
    world.collide_vehicle(&mut car, Some(frame), &mut impact);
    assert!(
        !impact.contacts.is_empty(),
        "source geometry must actually be struck: {}",
        placed.prop.as_ref().unwrap().name
    );
    (world, car, impact)
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_native_tollbooth_and_parked_contacts_preserve_source_weight_and_breakaway() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR")).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    for placed in fixtures(&dir, &index, &data) {
        let spec = placed.prop.as_ref().unwrap().dynamics.as_ref().unwrap();
        let id = placed.spawner.unwrap().unique_id + 1;
        let pose = fixture_pose(&placed, &data);
        assert!(spec.detach_impulse > 0.0 && !spec.immobile);
        for (speed, expected_attached) in [(0.01, true), (60.0, false)] {
            let (world, car, impact) = strike(&placed, &data, pose, speed, true);
            let prop = world.colliders[&id].body.as_ref().unwrap();
            assert_eq!(prop.rigid.mass(), spec.mass);
            assert_eq!(prop.attached, expected_attached, "{} at {speed}", placed.prop.as_ref().unwrap().name);
            assert!(car.linear_velocity().is_finite() && prop.rigid.linear_velocity.is_finite());
            assert!(impact.impulse > 0.0);
            assert_eq!(impact.damage[0].other.smackable, Some((spec.mass, spec.immobile || prop.attached)));
            assert!(impact.damage.iter().all(|hit| hit.other.smackable.is_some_and(|(mass, _)| mass == spec.mass)));
        }
        let (world, car, impact) = strike(&placed, &data, pose, 30.0, false);
        let prop = world.colliders[&id].body.as_ref().unwrap();
        assert!(prop.rigid.linear_velocity.length() > 0.0);
        assert!(impact.sounds.iter().any(|hit| hit.delta_v > 0.0));
        match spec.no_car_effect {
            true => assert!((car.linear_velocity().length() - 30.0).abs() < 0.0001),
            false => assert!(car.linear_velocity().length() < 30.0, "parked vehicle must resist the car"),
        }
    }
}

#[test]
fn newly_detached_prop_receives_ground_support_without_second_integration() {
    let mut placed = placement(400, Vec3::new(0.0, 0.6, 0.0), 100.0, true, 500.0);
    let mut world = PropWorld::default();
    world.add_tile(0, std::slice::from_mut(&mut placed));
    let id = placed.prop_id;
    let before = world.colliders[&id].body.as_ref().unwrap().rigid.position;
    world.begin_native(1.0 / 60.0);
    let mut car = car(20.0);
    let mut impact = Impact::default();
    world.collide_vehicle(&mut car, None, &mut impact);
    let body = world.colliders[&id].body.as_ref().unwrap();
    assert!(!body.attached && body.object_contact);
    let contact_position = body.rigid.position;
    assert!(body.rigid.linear_velocity.z > 0.0);
    world.finish_native(&FlatGround::new(0.3), 1.0 / 60.0);
    let body = world.colliders[&id].body.as_ref().unwrap();
    assert_eq!(body.previous.0, before);
    assert_eq!(body.rigid.position.z, contact_position.z, "end frame must not integrate the collision velocity again");
    assert!(body.rigid.position.y > contact_position.y, "a newly detached body needs ground support in the same tick");
    let collider = &world.colliders[&id];
    assert!(cells(&collider.physics_bounds()).eq(collider.cells.iter().copied()));
    assert!(candidate_ids(&world, &collider.physics_bounds()).contains(&id));
}

#[test]
fn grazing_prop_callback_keeps_normal_closing_separate_from_sliding_and_shock() {
    let mut car = car(30.0);
    car.body_mut().linear_velocity.x = 0.25;
    let at = car.body().position + Vec3::X * (car.body().dimension().x + 0.4);
    let mut placed = placement(405, at, 500.0, false, 0.0);
    let mut world = PropWorld::default();
    world.add_tile(0, std::slice::from_mut(&mut placed));
    let mut impact = Impact::default();
    world.collide_vehicle(&mut car, None, &mut impact);
    assert_eq!(impact.damage.len(), 1, "the fixture must strike the prop's side");
    let hit = impact.damage[0];
    assert!(hit.normal.x.abs() > 0.99);
    assert!((hit.closing_speed - 0.25).abs() < 0.0001);
    assert_eq!(hit.before_velocity.z, 30.0, "incoming linear motion remains available separately");
    assert!(hit.shock_force > 0.0, "prop callbacks preserve normal force even below the damage gate");
}

#[test]
fn light_props_retain_cop_impact_eligibility_only_while_still_attached() {
    for (threshold, attached) in [(100_000.0, true), (1.0, false), (0.0, false)] {
        let (world, _, impact, id) = hit(false, threshold, 5.0);
        let prop = world.colliders[&id].body.as_ref().unwrap();
        assert_eq!(prop.attached, attached);
        assert!(!impact.damage.is_empty());
        for damage in impact.damage {
            assert_eq!(damage.other.smackable, Some((100.0, attached)));
            assert_eq!(damage.allows_cop(), attached, "a retained attachment is immobile to the damage callback");
            assert!(damage.shock_force > 0.0);
        }
    }
}
