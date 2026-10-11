//! Exhaustive installation-to-runtime validation, without rendering or name-based behavior guesses.

use blackbox_attrib::{Database, vlt_hash};
use blackbox_scenery::{layout::MOST_WANTED, read_object_spawners, read_scenery_sections};
use blackbox_vehicle::rigid_body::box_inertia;
use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, PropCatalog, WorldIndex};

use super::*;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_installed_placement_preserves_its_identity_dynamics_and_initial_visual_pose() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let catalog = PropCatalog::new(index.prop_bounds.clone(), &db);
    let mut total = 0;
    let mut missing = 0;
    let mut ground_clouds = 0;
    let mut large = 0;
    let mut classes = std::collections::HashSet::new();
    let mut shared = std::collections::HashMap::<_, String>::new();
    let mut distinct_names = 0;
    for section in &index.sections {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        let scenery = read_scenery_sections(&bytes, &MOST_WANTED).unwrap();
        let mut hooks = std::collections::HashMap::new();
        for sc in &scenery {
            for hook in &sc.override_hooks {
                let inst = &sc.instances[usize::from(hook.instance_index)];
                hooks.insert(u32::from(hook.override_index), (inst, sc.info_of(inst).unwrap().name.as_str()));
            }
        }
        let mut placed = Vec::new();
        for spawn in read_object_spawners(&bytes, &MOST_WANTED.spawners).unwrap() {
            total += 1;
            let (inst, name) = hooks[&spawn.override_index];
            let attributes = db.collection_by_key(vlt_hash("smackable"), spawn.attributes).unwrap();
            let Some(shape) = catalog.for_spawner(name, &spawn) else {
                missing += 1;
                assert!(matches!(attributes.name(), Some("lightwoodenpost" | "wood_slat_fence")));
                continue;
            };
            assert_eq!(shape.name, name, "shared geometry must not change visible model identity");
            assert_eq!(shape.bounds.as_ref().unwrap().name_hash, spawn.collision);
            assert_eq!(shape.dynamics.as_ref().unwrap().mass, attributes.get_f32("MASS").unwrap());
            let key = (spawn.collision, spawn.attributes);
            if let Some(previous) = shared.insert(key, name.to_owned()) {
                distinct_names += usize::from(previous != name);
            }
            let mut p = Placed::prop_only(Mat4::from_cols_array_2d(&inst.matrix_columns()), shape);
            p.bounds = Aabb::new(inst.bbox_min, inst.bbox_max);
            p.spawner = Some(spawn);
            placed.push(p);
        }
        let mut world = PropWorld::default();
        world.add_tile(1, &mut placed);
        assert_eq!(world.len(), placed.len(), "every resolved placement needs a runtime body");
        for p in &placed {
            let collider = &world.colliders[&p.prop_id];
            let body = collider.body.as_ref().unwrap();
            let spec = collider.shape.dynamics.as_ref().unwrap();
            let spawn = p.spawner.unwrap();
            classes.insert(spawn.attributes);
            assert_eq!(p.prop_id, spawn.unique_id + 1);
            assert_eq!(body.attached, spec.detach_impulse != 0.0);
            assert_eq!(body.rigid.state, BodyState::Asleep);
            assert_eq!(body.rigid.mass(), spec.mass);
            assert_eq!(body.rigid.cog(), body.geometry.centre_of_gravity());
            assert_eq!(body.rigid.inertia(), box_inertia(spec.mass, body.rigid.dimension()) * spec.tensor_scale);
            assert!(
                body.rigid.position.is_finite()
                    && body.rigid.orientation.is_finite()
                    && body.rigid.inertia().is_finite()
            );
            let delta = body.transform(1.0).to_cols_array().into_iter().zip(p.transform.to_cols_array());
            assert!(
                delta.map(|(a, b)| (a - b).abs()).fold(0.0, f32::max) < 0.002,
                "initial pose changed: {}",
                collider.shape.name
            );
            ground_clouds += body.geometry.ground_meshes.len();
            large += usize::from(body.rigid.dimension().max_element() > 3.0);
            let enabled = body.geometry.primitives.iter().filter(|p| p.collides(flags::PRIM_VS_OBJECTS)).count();
            assert_eq!(
                crate::scenes::world::object_geometry::object_shapes(&body.rigid, Some(body.geometry_frame())).len(),
                enabled
            );
            let bounds = collider.physics_bounds();
            assert!(bounds.min.is_finite() && bounds.max.is_finite());
            assert!(world.candidates(&bounds).contains(&p.prop_id));
        }
        world.remove_tile(1);
        assert!(world.colliders.is_empty() && world.grid.is_empty());
    }
    assert_eq!((total, missing), (13_484, 5));
    assert!(classes.len() > 40 && large > 100 && ground_clouds > 100 && distinct_names > 0);
    println!(
        "checked {} runtime bodies, {} attribute classes, {large} large bodies, {ground_clouds} ground clouds and {distinct_names} shared-geometry name changes",
        total - missing,
        classes.len()
    );
}
