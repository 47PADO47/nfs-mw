use super::*;
use crate::scenes::world::drive::FixedClock;
use crate::scenes::world::props::lifetime::{Change, Lifetime, Observation, track_lifetime};

fn spec(dropout: [f32; 2], kill: f32) -> Arc<PropDynamics> {
    let p = placement(1, Vec3::Y, 100.0, false, 0.0);
    let mut spec = p.prop.unwrap().dynamics.as_ref().unwrap().as_ref().clone();
    spec.dropout = dropout;
    spec.kill_off_screen = kill;
    Arc::new(spec)
}

fn observation(fragment: bool, visible: bool) -> Observation {
    Observation { fragment, vehicle_part: fragment, visible, in_world: true, resting: false }
}

#[test]
fn visibility_resets_each_original_clock_and_parts_expire_after_exact_equality() {
    let spec = spec([0.0; 2], 1.0);
    for fragment in [false, true] {
        let mut life = Lifetime::default();
        life.activate(&spec);
        assert!(matches!(life.tick(&spec, observation(fragment, false), 0.75), Change::Keep));
        assert!(matches!(life.tick(&spec, observation(fragment, true), 0.25), Change::Keep));
        assert!(matches!(life.tick(&spec, observation(fragment, false), 0.75), Change::Keep));
        assert!(matches!(life.tick(&spec, observation(fragment, false), 0.3), Change::Release));
    }
    let mut part = Lifetime::default();
    part.activate(&spec);
    assert!(matches!(part.tick(&spec, observation(true, false), 1.0), Change::Keep));
    assert!(matches!(part.tick(&spec, observation(true, false), 0.01), Change::Release));
    let mut scenery = Lifetime::default();
    scenery.activate(&spec);
    assert!(matches!(scenery.tick(&spec, observation(false, false), 1.0), Change::Keep));
    assert!(matches!(scenery.tick(&spec, observation(false, false), 0.1), Change::Keep));
}

#[test]
fn offworld_clock_resets_and_the_attribute_exception_survives() {
    let mut spec = spec([0.0; 2], 0.0);
    let mut life = Lifetime::default();
    life.activate(&spec);
    let mut view = observation(false, true);
    view.in_world = false;
    assert!(matches!(life.tick(&spec, view, 0.75), Change::Keep));
    view.in_world = true;
    assert!(matches!(life.tick(&spec, view, 0.1), Change::Keep));
    view.in_world = false;
    assert!(matches!(life.tick(&spec, view, 0.75), Change::Keep));
    assert!(matches!(life.tick(&spec, view, 0.25), Change::Release));
    Arc::make_mut(&mut spec).allow_off_world = true;
    assert!(matches!(life.tick(&spec, view, 10.0), Change::Keep));
}

#[test]
fn resting_without_dropout_keeps_the_final_pose_until_another_contact() {
    let (mut world, _, _, id) = hit(false, 0.0, 20.0);
    let b = world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap();
    b.rigid.place(Vec3::new(10.0, 0.5, 0.0), Quat::IDENTITY);
    b.rigid.state = BodyState::Asleep;
    for _ in 0..900 {
        world.step(&FlatGround::new(0.0), 1.0 / 60.0);
        world.update_lifetimes(&FlatGround::new(0.0), Mat4::IDENTITY, 1.0 / 60.0);
    }
    let b = world.colliders[&id].body.as_ref().unwrap();
    assert_eq!(b.rigid.position, Vec3::new(10.0, 0.5, 0.0));
    assert!(!b.lifetime.active);
    assert!(b.moved);
}

fn sink_trace(frames: &[f32]) -> Vec<(u32, u32, bool, bool)> {
    let mut world = PropWorld::default();
    let mut p = placement(9, Vec3::Y * 0.5, 100.0, false, 0.0);
    Arc::make_mut(Arc::make_mut(p.prop.as_mut().unwrap()).dynamics.as_mut().unwrap()).dropout = [0.5, 2.0];
    world.add_tile(1, std::slice::from_mut(&mut p));
    let id = p.prop_id;
    let body = world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap();
    body.moved = true;
    let spec = world.colliders[&id].shape.dynamics.clone().unwrap();
    world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap().lifetime.activate(&spec);
    track_lifetime(&mut world.live_lifetimes, id);
    // The view is off-screen, so the dropout clock may start.
    let view = Mat4::from_translation(Vec3::X * 1000.0);
    let mut clock = FixedClock::default();
    let mut trace = Vec::new();
    let mut frame = 0;
    while trace.len() < 120 {
        for _ in 0..clock.advance(frames[frame % frames.len()]) {
            if trace.len() >= 120 {
                break;
            }
            world.step(&FlatGround::new(0.0), 1.0 / 60.0);
            world.update_lifetimes(&FlatGround::new(0.0), view, 1.0 / 60.0);
            let c = &world.colliders[&id];
            let b = c.body.as_ref().unwrap();
            trace.push((b.rigid.position.y.to_bits(), c.cells.len() as u32, b.lifetime.sinking, b.moved));
        }
        frame += 1;
    }
    trace
}

#[test]
fn a_visible_resting_prop_with_dropout_does_not_sink_or_restore() {
    let (mut world, _, _, id) = hit(false, 0.0, 20.0);
    let spec = world.colliders[&id].shape.dynamics.clone().unwrap();
    let spec = Arc::new({
        let mut s = spec.as_ref().clone();
        s.dropout = [0.5, 2.0];
        s.kill_off_screen = 0.0;
        s
    });
    // The identity view frustum contains the origin, so this prop stays visible.
    let b = world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap();
    b.rigid.place(Vec3::new(0.0, 0.5, 0.0), Quat::IDENTITY);
    b.rigid.state = BodyState::Asleep;
    b.moved = true;
    b.lifetime.activate(&spec);
    track_lifetime(&mut world.live_lifetimes, id);
    for _ in 0..240 {
        world.step(&FlatGround::new(0.0), 1.0 / 60.0);
        world.update_lifetimes(&FlatGround::new(0.0), Mat4::IDENTITY, 1.0 / 60.0);
    }
    let b = world.colliders[&id].body.as_ref().unwrap();
    assert!(!b.lifetime.sinking);
    assert_eq!(b.rigid.position, Vec3::new(0.0, 0.5, 0.0));
    assert!(b.moved);
}

#[test]
fn sinking_disables_contacts_restores_the_placement_and_has_identical_fixed_tick_traces() {
    let expected = sink_trace(&[1.0 / 60.0]);
    assert!(expected.iter().any(|&(y, cells, sink, _)| sink && cells == 0 && f32::from_bits(y) < 0.0));
    let &(height, cells, sinking, moved) = expected.last().unwrap();
    assert_eq!(height, 0.5_f32.to_bits());
    assert!(cells > 0 && !sinking && !moved);
    for fps in [5.0, 10.0, 30.0, 120.0, 144.0, 240.0] {
        assert_eq!(sink_trace(&[1.0 / fps]), expected, "{fps} FPS");
    }
    assert_eq!(sink_trace(&[0.003, 0.31, 0.017, 0.007]), expected);
}

#[test]
fn expired_fragments_release_their_cells_and_do_not_restore_the_parent() {
    let p = placement(9, Vec3::Y, 100.0, false, 0.0);
    let shape = p.prop.unwrap();
    let spec = spec([0.0; 2], 0.0625);
    let body = PropBody::fragment(shape.bounds.as_ref().unwrap(), 0, &spec, (Vec3::Y, Quat::IDENTITY), Mat4::IDENTITY)
        .unwrap();
    let owner = std::rc::Rc::new(());
    let mut world = PropWorld::default();
    let id = world.insert_fragment(body, spec, Vec::new(), std::rc::Rc::downgrade(&owner)).unwrap();
    assert_eq!(world.fragment_count(), 1);
    let view = Mat4::from_translation(Vec3::X * 1000.0);
    for _ in 0..4 {
        world.step(&FlatGround::new(0.0), 1.0 / 60.0);
        world.update_lifetimes(&FlatGround::new(0.0), view, 1.0 / 60.0);
    }
    assert_eq!(world.fragment_count(), 0);
    assert!(world.pose(id).is_none());
    assert!(world.colliders.is_empty() && world.grid.is_empty());
    assert_eq!(std::rc::Rc::strong_count(&owner), 1);
}

#[test]
fn resting_scenery_fragments_stop_their_countdown_but_vehicle_parts_keep_their_model_clock() {
    let p = placement(9, Vec3::Y, 100.0, false, 0.0);
    let shape = p.prop.unwrap();
    let spec = spec([0.0; 2], 1.0);
    let owner = std::rc::Rc::new(());
    let mut world = PropWorld::default();
    let mut ids = Vec::new();
    for scenery in [false, true] {
        let mut body = PropBody::fragment(
            shape.bounds.as_ref().unwrap(),
            0,
            &spec,
            (Vec3::Y * 0.5, Quat::IDENTITY),
            Mat4::IDENTITY,
        )
        .unwrap();
        body.rigid.state = BodyState::Asleep;
        let id = world
            .insert_fragment_with(body, spec.clone(), Vec::new(), std::rc::Rc::downgrade(&owner), scenery)
            .unwrap();
        ids.push(id);
    }
    let ground = FlatGround::new(0.0);
    for _ in 0..12 {
        world.update_lifetimes(&ground, Mat4::IDENTITY, 1.0 / 60.0);
    }
    assert!(world.colliders[&ids[0]].body.as_ref().unwrap().lifetime.active);
    assert!(!world.colliders[&ids[1]].body.as_ref().unwrap().lifetime.active);
    let offscreen = Mat4::from_translation(Vec3::X * 1000.0);
    for _ in 0..120 {
        world.update_lifetimes(&ground, offscreen, 1.0 / 60.0);
    }
    assert!(!world.colliders.contains_key(&ids[0]), "vehicle-part clock still expires after rest");
    assert!(world.colliders.contains_key(&ids[1]), "scenery debris waits for reactivation or parent cleanup");
    drop(owner);
    world.remove_orphaned_fragments();
    assert!(world.colliders.is_empty() && world.grid.is_empty());
}

#[test]
fn visible_dropout_waits_for_looking_away_instead_of_disabling_cleanup_forever() {
    let spec = spec([0.5, 2.0], 0.0);
    let mut life = Lifetime::default();
    life.activate(&spec);
    let mut visible = observation(false, true);
    visible.resting = true;
    for _ in 0..120 {
        assert!(matches!(life.tick(&spec, visible, 1.0 / 60.0), Change::Keep));
    }
    assert!(life.active && !life.sinking);
    visible.visible = false;
    assert!(matches!(life.tick(&spec, visible, 1.0 / 60.0), Change::Keep));
    assert!(life.sinking, "the pending dropout starts when visibility permits it");
}
