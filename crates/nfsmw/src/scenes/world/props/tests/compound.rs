//! An installed compound contact must retain the later approaching primitive.

use blackbox_attrib::Database;
use blackbox_scenery::{layout::MOST_WANTED, read_object_spawners, read_scenery_sections};
use game_install::GameDir;
use nfsmw_data::{
    car::physics::PhysicsData,
    world::{DEFAULT_TRACK, PropCatalog, WorldIndex},
};

use super::*;
use crate::scenes::world::object_geometry::{BodyGeometryFrame, object_shapes};

#[test]
fn compound_prop_contacts_keep_later_breakaway_and_ignore_disabled_parts() {
    for enabled in [false, true] {
        let mut world = PropWorld::default();
        let mut compound = placement(1, Vec3::new(0.0, -4.25, 0.0), 1000.0, false, 0.0);
        let shape = Arc::make_mut(compound.prop.as_mut().unwrap());
        let bounds = Arc::make_mut(shape.bounds.as_mut().unwrap());
        let mut body = bounds.nodes[0].clone();
        body.half_dimensions = [3.0, 1.0, 3.0];
        let mut upper = body.clone();
        upper.position = [0.0, 6.0, 0.0];
        upper.half_dimensions = [0.75, 0.5, 0.75];
        if !enabled {
            upper.flags |= flags::DISABLED;
        }
        bounds.nodes[0].half_dimensions = [3.0, 6.5, 3.0];
        bounds.nodes[0].child_count = 2;
        bounds.nodes[0].first_child = 1;
        bounds.nodes.extend([body, upper]);
        let mut pole = placement(2, Vec3::ZERO, 100.0, false, 5000.0);
        let shape = Arc::make_mut(pole.prop.as_mut().unwrap());
        Arc::make_mut(shape.bounds.as_mut().unwrap()).nodes[0].half_dimensions = [0.25, 5.0, 0.25];
        let mut placed = [compound, pole];
        world.add_tile(1, &mut placed);
        let [a, b] = placed.map(|p| p.prop_id);
        let first = world.colliders.get_mut(&a).unwrap().body.as_mut().unwrap();
        first.rigid.linear_velocity = -Vec3::X * 20.0;
        first.previous.0 = first.rigid.position + Vec3::X / 6.0;
        let first = world.colliders[&a].body.as_ref().unwrap();
        let second = world.colliders[&b].body.as_ref().unwrap();
        let initial = object_shapes(&first.rigid, Some(first.geometry_frame()));
        let pole = &object_shapes(&second.rigid, Some(second.geometry_frame()))[0];
        let lower = initial[0].contact(pole).unwrap();
        assert_eq!(lower.normal, -Vec3::Y);
        assert_eq!(lower.overlap, 1.75);
        assert_eq!(first.rigid.linear_velocity.dot(lower.normal), 0.0);
        if enabled {
            assert!(initial[1].contact(pole).is_none(), "the next contact appears only after separation");
            let mut separated = first.rigid.clone();
            separated.position += lower.normal * lower.overlap;
            let prepared = object_shapes(&separated, Some(first.geometry_frame()));
            let upper = prepared[1].contact(pole).unwrap();
            assert_eq!(upper.normal, Vec3::X);
            assert_eq!(upper.overlap, 1.0);
            assert!(-separated.linear_velocity.dot(upper.normal) * separated.mass() > 5000.0);
        }
        world.collide_props(&[a]);
        let first = world.colliders[&a].body.as_ref().unwrap();
        let second = world.colliders[&b].body.as_ref().unwrap();
        assert_eq!(first.rigid.position.y, -6.0, "failed response still separates before the next pair");
        assert_eq!(second.attached, !enabled, "the later approaching primitive controls breakaway");
        assert!(first.rigid.position.is_finite() && first.rigid.linear_velocity.is_finite());
        assert!(second.rigid.position.is_finite() && second.rigid.linear_velocity.is_finite());
        if enabled {
            assert!(second.moved && second.rigid.linear_velocity.length() > 1.0);
        }
    }
}

fn installed_lamp(dir: &GameDir) -> Placed {
    let index = WorldIndex::open(dir, DEFAULT_TRACK).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let catalog = PropCatalog::new(index.prop_bounds.clone(), &db);
    for section in &index.sections {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let spawns = read_object_spawners(&bytes, &MOST_WANTED.spawners).unwrap();
        let Some(spawn) = spawns.into_iter().find(|s| s.unique_id == 8482) else { continue };
        for sc in read_scenery_sections(&bytes, &MOST_WANTED).unwrap() {
            let Some(hook) = sc.override_hooks.iter().find(|h| u32::from(h.override_index) == spawn.override_index)
            else {
                continue;
            };
            let instance = &sc.instances[usize::from(hook.instance_index)];
            let name = &sc.info_of(instance).unwrap().name;
            let shape = catalog.for_spawner(name, &spawn).unwrap();
            let mut p = Placed::prop_only(Mat4::from_cols_array_2d(&instance.matrix_columns()), shape);
            p.spawner = Some(spawn);
            return p;
        }
    }
    panic!("installed dynamic lamp placement is missing");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_compound_car_contact_reaches_later_breakaway_after_nonclosing_overlap() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let physics = PhysicsData::load(&dir).unwrap().car("BMWM3GTR").unwrap();
    let car_geometry = &physics.geometry.as_ref().unwrap().body;
    for speed in [1.0, 20.0] {
        let mut placed = [installed_lamp(&dir)];
        let mut world = PropWorld::default();
        world.add_tile(1, &mut placed);
        let id = placed[0].prop_id;
        let prop = world.colliders[&id].body.as_ref().unwrap();
        let primitive = prop.geometry.primitives.iter().find(|p| p.collides(flags::PRIM_VS_OBJECTS)).unwrap();
        let pole = prop.obb(primitive);
        let position = pole.centre + pole.axes * Vec3::new(-0.04, -4.616, 0.16);
        let orientation = prop.rigid.orientation;
        let velocity = -pole.axes.x_axis * speed;
        let mut car = Vehicle::new(physics.spec.clone());
        car.body_mut().place(position, orientation);
        car.body_mut().linear_velocity = velocity;
        let frame =
            Some(BodyGeometryFrame { geometry: car_geometry, previous: (position - velocity / 120.0, orientation) });
        if speed > 1.0 {
            let shapes = object_shapes(car.body(), frame);
            let poles = object_shapes(&prop.rigid, Some(prop.geometry_frame()));
            let contacts: Vec<_> = shapes.iter().filter_map(|a| a.contact(&poles[0])).collect();
            assert_eq!(contacts.len(), 2, "the fixture needs both intact car boxes");
            assert!(contacts[0].overlap > contacts[1].overlap);
            assert!(-velocity.dot(contacts[0].normal) < 0.001, "first pair is not closing");
            assert!(-velocity.dot(contacts[1].normal) * car.body().mass() > 7500.0);
        }
        let mut impact = Impact::default();
        world.collide_vehicle(&mut car, frame, &mut impact);
        let body = world.colliders[&id].body.as_ref().unwrap();
        assert_eq!(body.attached, speed <= 1.0, "later approaching pair must reach breakaway response");
        assert!(body.rigid.position.is_finite() && body.rigid.linear_velocity.is_finite());
        assert!(car.position().is_finite() && car.linear_velocity().is_finite());
        if speed > 1.0 {
            assert!(body.moved && body.rigid.linear_velocity.length() > 1.0);
            assert!(impact.contacts.iter().any(|c| c.kind == ContactKind::PropMovable));
        }
    }
}
