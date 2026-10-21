use super::*;

#[test]
#[ignore = "needs the base-game install selected by NFSMW_GAME_DIR"]
fn installed_prop_collision_channels_and_world_materials_preserve_linked_fields() {
    let dir = game_install::GameDir::open(std::env::var("NFSMW_GAME_DIR").unwrap()).unwrap();
    let db = crate::shared::attributes(&dir).unwrap();
    let mut count = 0;
    let mut object_disabled = 0;
    let mut world_disabled = 0;
    for collection in db.collections_of("smackable") {
        let Some(dynamics) = PropDynamics::read(collection) else { continue };
        let body = collection.follow("rigidbodyspecs").unwrap();
        assert_eq!(dynamics.no_object_collisions, body.get_bool("NO_OBJ_COLLISIONS").unwrap_or(false));
        assert_eq!(dynamics.no_world_collisions, body.get_bool("NO_WORLD_COLLISIONS").unwrap_or(false));
        let expected = |field| {
            let v = body.get_vector4(field).unwrap();
            Vec3::new(v[0], v[1], v[2])
        };
        assert_eq!(dynamics.walls.elasticity, expected("WALL_ELASTICITY"));
        assert_eq!(dynamics.walls.moment_scale, expected("WORLD_MOMENT_SCALE"));
        let friction = std::array::from_fn(|index| body.get_at("WALL_FRICTION", index).unwrap().as_f32().unwrap());
        assert_eq!(dynamics.walls.friction, friction);
        object_disabled += usize::from(dynamics.no_object_collisions);
        world_disabled += usize::from(dynamics.no_world_collisions);
        count += 1;
    }
    assert!(count > 0, "selected no linked prop collision records");
    let surfaces = crate::car::physics::SurfaceTable::from_database(&db);
    let mut surface_count = 0;
    for surface in db.collections_of("simsurface") {
        assert_eq!(surfaces.world_friction(surface.key()), surface.get_f32("WORLD_FRICTION").unwrap_or(1.0));
        surface_count += 1;
    }
    assert!(surface_count > 0, "selected no surface world-friction records");
    println!(
        "prop collision records={count}, object disabled={object_disabled}, world disabled={world_disabled}, world surfaces={surface_count}"
    );
}
