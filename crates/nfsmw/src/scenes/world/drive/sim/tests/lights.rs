use super::*;
use crate::scenes::world::drive::FixedClock;

#[test]
fn short_npc_brake_pulses_do_not_flash_lamps_or_filter_physical_brakes() {
    let mut car = sim();
    car.configure_traffic();
    let ground = FlatGround::new(0.0);
    for tick in 0..240 {
        let brake = match tick % 20 < 19 {
            true => 1.0,
            false => 0.0,
        };
        car.body_mut().wake();
        car.step(&DriveInput { brake, ..Default::default() }, &ground, None);
        assert_eq!(car.vehicle.controls().brake, brake);
        assert_eq!(car.lamp_brake(), 0.0);
    }
    for _ in 0..32 {
        car.step(&DriveInput { brake: 0.2, ..Default::default() }, &ground, None);
    }
    assert_eq!(car.lamp_brake(), 1.0);
    car.step(&DriveInput::default(), &ground, None);
    assert_eq!(car.lamp_brake(), 0.0);
    car.step(&DriveInput { brake: 1.0, ..Default::default() }, &ground, None);
    assert_eq!(car.lamp_brake(), 0.0, "released input resets the full admission interval");
}

#[test]
fn player_brake_lamps_are_immediate_and_handbrake_alone_is_not_regular_braking() {
    let mut car = sim();
    car.vehicle.config.auto_brake = false;
    car.vehicle.config.auto_reverse = false;
    let ground = FlatGround::new(0.0);
    car.step(&DriveInput { brake: 0.2, ..Default::default() }, &ground, None);
    assert_eq!(car.lamp_brake(), 1.0);
    car.step(&DriveInput { handbrake: true, ..Default::default() }, &ground, None);
    assert_eq!(car.lamp_brake(), 0.0);
}

#[test]
fn sleeping_npc_body_cannot_keep_a_released_brake_lamp_lit() {
    let mut car = sim();
    car.configure_traffic();
    let ground = FlatGround::new(0.0);
    for _ in 0..32 {
        car.step(&DriveInput { brake: 1.0, ..Default::default() }, &ground, None);
    }
    assert_eq!(car.lamp_brake(), 1.0);
    car.body_mut().state = blackbox_vehicle::rigid_body::BodyState::Asleep;
    car.step(&DriveInput::default(), &ground, None);
    assert_eq!(car.lamp_brake(), 0.0, "FX reads current input even when physical integration sleeps");
}

fn timeline(fps: usize) -> Vec<f32> {
    let mut car = sim();
    car.configure_traffic();
    let ground = FlatGround::new(0.0);
    let mut clock = FixedClock::default();
    let mut states = Vec::new();
    for _ in 0..fps * 4 {
        for _ in 0..clock.advance(1.0 / fps as f32) {
            let tick = states.len();
            let brake = match tick {
                0..60 | 121..140 => 1.0,
                _ => 0.0,
            };
            car.step(&DriveInput { brake, ..Default::default() }, &ground, None);
            states.push(car.lamp_brake());
        }
    }
    assert!(states.contains(&1.0));
    states
}

#[test]
fn npc_brake_lamp_admission_has_one_fixed_timeline_at_different_render_rates() {
    let expected = timeline(60);
    for fps in [30, 144, 360] {
        assert_eq!(timeline(fps), expected);
    }
}
