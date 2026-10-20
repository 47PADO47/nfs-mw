use super::*;
use blackbox_vehicle::rigid_body::RigidBodySpec;

#[test]
fn native_camera_explosions_keep_event_values_axes_and_ignore_physical_target_flags() {
    let mut world = PropWorld::default();
    let mut source = wave();
    source.position = Vec3::new(1.0, 2.0, 3.0);
    source.spec.include_players = false;
    source.spec.exclude_vehicles = true;
    source.spec.exclude_smackables = true;
    world.scenery.waves.push(source);
    let snapshots = world.camera_explosions();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].position, Vec3::new(3.0, -1.0, 2.0));
    assert_eq!(snapshots[0].expansion_speed, 4.0);
    assert_eq!(snapshots[0].maximum_radius, 16.0);
    world.scenery.waves[0].complete = true;
    assert!(world.camera_explosions().is_empty());
}

fn wave() -> Wave {
    Wave {
        source: 7,
        position: Vec3::ZERO,
        radius: 10.0,
        complete: false,
        bodies: BTreeSet::new(),
        vehicles: Vec::new(),
        spec: Explosion {
            position: 0,
            speed: 4.0,
            radius: 16.0,
            start_radius: 0.0,
            affect_this: false,
            damage: false,
            include_players: true,
            exclude_vehicles: true,
            exclude_smackables: false,
        },
    }
}

#[test]
fn player_target_is_independent_of_other_vehicle_target_and_a_wave_hits_once() {
    let mut world = PropWorld::default();
    world.scenery.waves.push(wave());
    let mut body = RigidBody::new(100.0, Vec3::ONE, Vec3::ONE, RigidBodySpec::default());
    body.place(Vec3::Z * 5.0, glam::Quat::IDENTITY);
    let owner = Rc::new(());
    let dt = 1.0 / 60.0;
    world.vehicle_explosions(&mut body, &owner, false, dt);
    assert_eq!(body.pending_force(), Vec3::ZERO);
    world.vehicle_explosions(&mut body, &owner, true, dt);
    let force = body.pending_force();
    assert!((force.z - 24000.0).abs() < 0.01);
    world.vehicle_explosions(&mut body, &owner, true, dt);
    assert_eq!(body.pending_force(), force);
    body.begin_frame(dt);
    assert!((body.linear_velocity.z - 4.0).abs() < 0.0001);
    body.linear_velocity = Vec3::Z * 20.0;
    let force_before = body.pending_force();
    world.vehicle_explosions(&mut body, &Rc::new(()), true, dt);
    assert_eq!(body.pending_force(), force_before, "a faster outward target must not receive extra force");
}

#[test]
fn player_exclusion_is_respected_when_other_vehicles_are_enabled() {
    let mut world = PropWorld::default();
    let mut wave = wave();
    wave.spec.exclude_vehicles = false;
    wave.spec.include_players = false;
    world.scenery.waves.push(wave);
    let mut body = RigidBody::new(100.0, Vec3::ONE, Vec3::ONE, RigidBodySpec::default());
    body.place(Vec3::Z * 5.0, glam::Quat::IDENTITY);
    let owner = Rc::new(());
    world.vehicle_explosions(&mut body, &owner, true, 1.0 / 60.0);
    assert_eq!(body.pending_force(), Vec3::ZERO);
    world.vehicle_explosions(&mut body, &owner, false, 1.0 / 60.0);
    assert!(body.pending_force().z > 0.0);
}
