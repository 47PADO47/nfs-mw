use super::*;
use blackbox_vehicle::rigid_body::BodyState;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_resident_gas_station_hide_ends_the_parent_simulation_without_regenerating_it() {
    let Some(install) = installed() else { return };
    let target = install
        .placed
        .iter()
        .find(|p| {
            p.prop.as_ref().unwrap().dynamics.as_ref().unwrap().event_sequencer == vlt_hash("gaspumpexplosion_seq")
        })
        .unwrap();
    let centre = Vec3::from_array(target.spawner.unwrap().position);
    let (mut world, _) = world(&install, centre);
    strike(&mut world, target.spawner.unwrap().unique_id + 1);
    let ground = FlatGround::new(centre.y - 0.5);
    let mut zones = 0;
    for _ in 0..120 {
        world.step(&ground, STEP);
        zones += world.update_scenery(&install.data).len();
    }
    assert!(zones > 0);
    let hidden: Vec<_> = world
        .scenery
        .models
        .iter()
        .filter(|(_, model)| model.parent_owner.is_some() && model.hidden)
        .map(|(&id, _)| id)
        .collect();
    assert!(!hidden.is_empty(), "the installed collapse must hide placed parents");
    for &id in &hidden {
        let collider = &world.colliders[&id];
        let body = collider.body.as_ref().unwrap();
        assert!(!collider.collidable && !body.lifetime.active, "hidden parent {id} ended simulation");
        assert_eq!(body.rigid.state, BodyState::Frozen);
    }
    let offscreen = Mat4::from_translation(Vec3::splat(100000.0));
    for _ in 0..1200 {
        world.update_lifetimes(&ground, offscreen, STEP);
        world.update_scenery(&install.data);
    }
    for &id in &hidden {
        assert!(world.scenery.models[&id].hidden, "resident parent {id} must not regenerate on its old timer");
    }
    let id = hidden[0];
    let owner = world.scenery.models[&id].owner.clone();
    world.dispatch_scenery(id, nfsmw_data::world::sequences::WorldEvent::KillObject, &install.data);
    assert!(!world.scenery.models[&id].hidden, "explicit model release still restores the placement");
    assert!(world.colliders[&id].collidable);
    assert_eq!(owner.strong_count(), 0, "explicit release invalidates old children");
}
