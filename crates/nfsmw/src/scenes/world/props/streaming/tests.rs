use super::*;
use crate::scenes::world::props::tests::placement;
use glam::{Mat4, Vec3};

fn consumed() -> (PropWorld, crate::scenes::world::resident::Placed) {
    let mut placed = placement(41, Vec3::Y, 100.0, false, 0.0);
    let mut world = PropWorld::default();
    world.add_tile(7, std::slice::from_mut(&mut placed));
    world.colliders.get_mut(&placed.prop_id).unwrap().body.as_mut().unwrap().moved = true;
    (world, placed)
}

fn hidden(world: &PropWorld, id: u32) -> bool {
    let mut instances = Vec::new();
    world.model_instances(id, Mat4::IDENTITY, &mut instances)
}

#[test]
fn unloading_releases_resources_but_reloading_keeps_consumed_placements_hidden() {
    let (mut world, mut placed) = consumed();
    let id = placed.prop_id;
    assert_eq!(world.section_for(id), Some(7));
    world.remove_tile(7);
    assert!(world.colliders.is_empty() && world.scenery.models.is_empty() && world.grid.is_empty());
    assert_eq!(world.section_for(id), None);
    world.begin_native(6.0);
    world.add_tile(7, std::slice::from_mut(&mut placed));
    assert_eq!(placed.prop_id, id, "the original spawner ID survives allocator changes");
    assert!(hidden(&world, id), "streaming cannot regenerate a displaced prop after six seconds");
    assert!(!world.colliders[&id].can_collide_with_objects());
    assert!(!world.colliders[&id].body.as_ref().unwrap().lifetime.active);
    assert!(world.grid.is_empty());
}

#[test]
fn only_a_reload_after_strictly_more_than_three_minutes_restores_the_section() {
    let (mut world, mut placed) = consumed();
    world.remove_tile(7);
    world.begin_native(180.0);
    world.add_tile(7, std::slice::from_mut(&mut placed));
    assert!(hidden(&world, placed.prop_id), "equality does not expire the saved override");
    assert!(world.take_restored_sections().is_empty());
    world.remove_tile(7);
    world.begin_native(180.5);
    world.add_tile(7, std::slice::from_mut(&mut placed));
    assert!(!hidden(&world, placed.prop_id));
    assert!(world.colliders[&placed.prop_id].can_collide_with_objects());
    assert_eq!(world.take_restored_sections(), [7]);
    assert!(world.take_restored_sections().is_empty());
}

#[test]
fn actively_racing_suppresses_expiry_without_pausing_elapsed_time() {
    let (mut world, mut placed) = consumed();
    world.remove_tile(7);
    world.set_streaming_reset_locked(true);
    world.begin_native(240.0);
    world.set_streaming_reset_locked(false);
    world.add_tile(7, std::slice::from_mut(&mut placed));
    assert!(!hidden(&world, placed.prop_id), "time passed while racing still counts after the race");
    world.colliders.get_mut(&placed.prop_id).unwrap().body.as_mut().unwrap().moved = true;
    world.remove_tile(7);
    world.set_streaming_reset_locked(true);
    world.begin_native(240.0);
    world.add_tile(7, std::slice::from_mut(&mut placed));
    assert!(hidden(&world, placed.prop_id), "a reload while actively racing retains the override");
}

#[test]
fn pristine_sections_and_other_tiles_do_not_inherit_consumed_overrides() {
    let (mut world, mut consumed) = consumed();
    let mut pristine = placement(42, Vec3::X * 10.0, 100.0, false, 0.0);
    world.add_tile(8, std::slice::from_mut(&mut pristine));
    world.remove_tile(7);
    world.remove_tile(8);
    world.add_tile(8, std::slice::from_mut(&mut pristine));
    world.add_tile(7, std::slice::from_mut(&mut consumed));
    assert!(hidden(&world, consumed.prop_id));
    assert!(!hidden(&world, pristine.prop_id));
    assert_eq!(world.take_restored_sections(), [8]);
}
