use super::*;
use blackbox_vehicle::rigid_body::BodyState;

#[test]
fn destroyed_sleep_adapter_excludes_the_player_and_keeps_a_supported_live_npc_awake() {
    let ground = FlatGround::new(0.0);
    for player in [true, false] {
        let mut car = sim();
        let mut spec = car.vehicle.spec().clone();
        spec.body.sleep_velocity = 1.0;
        car.vehicle.set_spec(spec);
        if !player {
            car.configure_ai();
        }
        assert!(car.place(&ground, Spawn { position: Vec3::ZERO, heading: 0.0 }));
        for _ in 0..180 {
            car.step(&DriveInput::default(), &ground, None);
            car.finish_retirement_contacts();
        }
        assert!(car.telemetry().wheels_on_ground > 2);
        assert_eq!(car.body().state, BodyState::Awake);
        car.set_destroyed_non_player(true);
        for _ in 0..240 {
            car.step(&DriveInput::default(), &ground, None);
            car.finish_retirement_contacts();
        }
        let expected = match player {
            true => BodyState::Awake,
            false => BodyState::Asleep,
        };
        assert_eq!(car.body().state, expected);
    }
}
