//! A detached trailer receives neither its former tractor's brakes nor an automatic full-brake command.

use super::*;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_released_trailer_rolls_like_a_brake_free_body_while_its_tractor_brakes() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to run installed tests");
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let (mut spawn, _, _) = highway(index.road_network.as_ref().unwrap());
    let collision = collision(&dir, &index, spawn);
    spawn.position.z =
        road::probe(&collision, spawn.position.x, spawn.position.y, spawn.position.z + 10.0).unwrap().height;
    let data = PhysicsData::load(&dir).unwrap();
    let ground = WorldGround { collision: &collision, surfaces: &data.surfaces };
    let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
    for name in ["semib", "semilog"] {
        let tractor_model = model(&dir, &data, name, &mut renderer);
        let trailer_model = model(&dir, &data, &data.trailer_of(name).unwrap(), &mut renderer);
        let mut tractor = CarSim::for_rig(tractor_model.physics.clone(), &tractor_model.rig);
        tractor.configure_traffic();
        assert!(tractor.place_traffic_moving(&ground, spawn, 15.0));
        let mut actual = TrailerCar::place(&trailer_model, &tractor, spawn, 15.0, &ground).unwrap();
        let mut reference = TrailerCar::place(&trailer_model, &tractor, spawn, 15.0, &ground).unwrap();
        let supported = Posture { up: Vec3::Y, wheels_on_ground: 4 };
        let tipped = Posture { up: Vec3::X, ..supported };
        assert!(!actual.coupling.hitch.update(STEP, supported, tipped));
        reference.sim.set_trailer_hitched(false);
        let mut actual_props = PropWorld::default();
        let mut reference_props = PropWorld::default();
        for tick in 0..120 {
            actual.step(
                &mut tractor,
                Brakes { brake: 1.0, handbrake: true },
                &collision,
                &mut actual_props,
                &data.surfaces,
            );
            reference.sim.step(&DriveInput::default(), &ground, Some((&collision, &mut reference_props)));
            actual.sync_contacts();
            assert!(!actual.is_hitched(), "{name} must stay released");
            let (a, b) = (actual.sim.body(), reference.sim.body());
            assert_eq!(a.position, b.position, "{name} tick {tick}: extra braking displaced the trailer");
            assert_eq!(a.orientation, b.orientation, "{name} tick {tick}: extra braking rotated the trailer");
            assert_eq!(a.linear_velocity, b.linear_velocity, "{name} tick {tick}: extra braking changed velocity");
            assert_eq!(a.angular_velocity, b.angular_velocity, "{name} tick {tick}: extra braking changed spin");
        }
    }
}
